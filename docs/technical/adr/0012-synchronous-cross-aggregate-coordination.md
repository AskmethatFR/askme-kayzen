---
id: "adr-0012-synchronous-cross-aggregate-coordination"
type: "technical"
owner: "architect"
status: "superseded"
updated: "2026-08-17"
relations:
  # No hand-written `superseded-by` here: it is an INVERSE edge, derived at query
  # time from adr-0013's own `supersedes`.
  related:
    - "architecture-overview"
    - "adr-0006-cqrs-light"
    - "adr-0011-one-public-method-per-use-case"
    - "adr-0013-set-based-validation-outside-aggregates"
  depends-on:
    - "adr-0002-habitboard-stateful-aggregate"
    - "adr-0007-habit-lifecycle-aggregate"
answers:
  - "~~One gesture must change two aggregates — event, saga, or synchronous orchestration?~~ VOID — no gesture touches two aggregates any more"
  - "~~In which order are the two aggregates saved?~~ VOID — there is one write"
  - "~~What happens if the process dies between the two saves?~~ VOID — there is no interval"
  - "What did the cross-aggregate coordination cost, and what made the whole question disappear?"
  - "What does slice 7 (readmission) actually face now?"
decided_in:
  - "2026-08-11 slice 6 anchor-habit cycle"
  - "2026-08-17 drop-habit-board refactor (SUPERSEDED / VOID)"
---

# ADR 0012 — Cross-aggregate coordination is a synchronous application-service orchestration, not an event

> **⚠️ SUPERSEDED AND VOID (2026-08-17) by [[adr-0013-set-based-validation-outside-aggregates]].** This node answered *"one gesture must change two aggregates — how?"*. With `HabitBoard` deleted there is **one aggregate left**, and **no gesture touches two**. Every decision below has lost its subject: the save-order doctrine, the idempotence-instead-of-a-transaction argument, the partial-anchor failure mode, and the constraint it carried to slice 7. `AnchorHabit::execute` is now the same 8-line load → mutate → save shape as `PauseHabit` and `ResumeHabit` (`core/src/habit_management/use_cases/anchor_habit.rs`). **Nothing below is current.** In particular the MUST *"free the seat through `HabitBoard::release`, never by mutating the registry from a use case"* names a method, a type and a registry that no longer exist — see the 2026-08-17 amendment for what replaces the whole node, and what slice 7 actually faces.

> **One-liner** *(historical)*: When one user gesture must change two aggregates, the **command use case orchestrates them synchronously in one call and publishes nothing** — `AnchorHabit::execute` moves the `Habit` to `Anchored`, saves it, then removes its entry from the `HabitBoard` and saves that. Both transitions are **idempotent**, so a replay converges from any partial state; the save order (**habit first, board second**) is deliberate and chosen for its failure mode.
> **Links**: [[adr-0013-set-based-validation-outside-aggregates]] (what voided this), [[adr-0007-habit-lifecycle-aggregate]] (d3 — lifecycle mutations are internal transitions, never published; **still current**, and the only part of this node's reasoning that survives), [[adr-0002-habitboard-stateful-aggregate]] (the board this coordinated with, also superseded), [[adr-0011-one-public-method-per-use-case]] (still current), [[adr-0006-cqrs-light]] (why there is no projection to feed and therefore no subscriber), [[architecture-overview]] (where this was applied).

## Context

Slice 6 (`anchor-habit`) is the first gesture in this codebase whose single user intent
must change **two aggregates**. Anchoring a habit that has become natural moves the
`Habit` to `LifecycleState::Anchored` — it leaves the day's list — **and** frees its seat
on the `HabitBoard`, so a sixth habit becomes requestable. Every gesture before it (mark
done, grow, lighten, pause, resume) touched exactly one aggregate; [[adr-0002-habitboard-stateful-aggregate]]
and [[adr-0007-habit-lifecycle-aggregate]] each settle their own aggregate's discipline
and neither answers what happens when one intent spans both.

Two facts frame the fork. [[adr-0007-habit-lifecycle-aggregate]] d3 forbids publishing any
lifecycle event and forbids the `HabitEvent` enum that would carry one — there is no
subscriber and [[adr-0006-cqrs-light]] has no projection to feed. And the runtime is one
process, one user, two in-memory stores, with no transaction manager and no concurrency.
The classic answer to "two aggregates, one intent" — publish a domain event, accept
eventual consistency — is a solution to a problem this system does not have.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Coordination shape | The **command use case orchestrates synchronously**: `AnchorHabit::execute` = load habit → `anchor()` → save habit → load board → `release(&id)` → save board → `Ok(())`. One call, one stack frame, no intermediate state visible to anything | `core/src/habit_management/use_cases/anchor_habit.rs` |
| Publication | **Nothing is published.** No `HabitAnchored`, no `HabitEvent` enum, `HabitBoardEvent` and the outbox untouched — [[adr-0007-habit-lifecycle-aggregate]] d3 is preserved intact, not amended | — |
| How the seat is freed | Through a **board method**, `HabitBoard::release(&mut self, id)`, not by mutating the registry from the use case. The invariant stays inside the aggregate that owns it ([[adr-0002-habitboard-stateful-aggregate]] amendment, same cycle) | core/src/habit_management/domain/habit_board.rs *(deleted)* |
| Save order | **Habit first, board second** — deliberately, for the failure mode it produces (see below). Not an accident of writing order | `core/src/habit_management/use_cases/anchor_habit.rs` |
| Recovery model | **Idempotence instead of a transaction.** `anchor()` assigns a state, `release()` retains over the registry and is a silent no-op when the entry is absent. Replaying the gesture from any partial state converges to the same result; nothing accumulates, nothing double-counts | `core/src/habit_management/domain/habit.rs`, core/src/habit_management/domain/habit_board.rs *(deleted)* |
| No transaction manager | Nothing to enlist: two in-memory maps in one process. A unit of work spanning both repositories would be ceremony over `HashMap`s | — |
| No `Clock` | Nothing about anchoring is dated — [[adr-0007-habit-lifecycle-aggregate]] AD-3's rule applied, not re-derived | `core/src/habit_management/use_cases/anchor_habit.rs` |

### Why habit first, then board

The two orders fail differently, and only one of the two failure modes is safe.

| Order | If the process dies between the two saves | Verdict |
|---|---|---|
| **Habit, then board** (chosen) | The habit is `Anchored` — it has left the day's list — but its entry is still on the board. **A seat is lost**: the user can hold four habits instead of five until the state is repaired | Degraded, visible, non-exploitable. The cap is *stricter* than the rule, never looser |
| Board, then habit | The entry is gone — a seat is free — while the habit is still `Active` and still in the day's list. **The 5-habit cap is bypassed**: six active habits | A capacity invariant silently broken by a crash |

Security's audit of this cycle confirmed both halves: the inverse order is the one that
would be exploitable, and a capacity bypass is **not reachable today** (the window
requires a crash between two synchronous in-memory writes, and no code path retries).
Dev-B and Security each said it explicitly — **do not change this order**. It is recorded
here so a later refactor tidying the method does not swap the two blocks for symmetry.

### Anchoring releases the title, and that is correct

The board's two invariants — capacity and title-uniqueness — are both computed over its
registry. Removing the entry therefore releases **both**: a new habit may take an anchored
habit's title. This is not a side effect that slipped through; it is what the functional
spec already asks for (`readmit-habit` S3).

**It constrains slice 7.** Readmission must put an **existing habit id** back on the board
*without* republishing `HabitRequested` — that event is what creates a `Habit`
([[architecture-overview]], creation flow), so republishing it would build a second
`Habit` for an identity that already exists, and the readmitted habit would lose its
histories. Slice 7's spec starts from this constraint rather than rediscovering it.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Publish `HabitAnchored`, have the board react | Requires the `HabitEvent` enum [[adr-0007-habit-lifecycle-aggregate]] d3 **explicitly forbids**, plus drain plumbing at the composition root and in the view — all of it to serve **one synchronous in-process subscriber** reached three lines later. It buys eventual consistency for a consistency window that does not exist, and it reopens a settled decision in order to solve nothing |
| Make the capacity count *non-anchored habits* instead of removing the entry | Forces `HabitBoard` to learn, on **every** request, a fact that lives in another aggregate — the habit's `LifecycleState`. The invariant stops being local, the registry stops being the sole source of truth ([[adr-0002-habitboard-stateful-aggregate]] MUST), and the board gains a dependency on `HabitRepository` that ADR explicitly refused |
| A transaction / unit of work spanning both repositories | Nothing to enlist — two in-memory maps in one process. Real transactionality is a question for the persistence slice, and it is listed below as an escalation trigger, not pre-built |
| A saga / compensating action | Same answer, one order of magnitude more machinery: idempotence already gives convergence, and there is no distributed step to compensate |
| A domain service holding both aggregates | The coordination is **orchestration, not a domain rule**: it encodes *what a gesture does*, not *what is always true*. It belongs in the application layer, which is where [[adr-0011-one-public-method-per-use-case]] already puts one gesture = one use case |
| Board saved first, habit second | Inverts the failure mode into the exploitable one — see the table above |

## Consequences / Constraints — **VOID since 2026-08-17, retained verbatim as history**

> ⚠️ **None of the constraints below is in force.** They govern a two-aggregate gesture that no
> longer exists. Read the amendment at the end of this node before acting on any of them; the
> third bullet in particular names a method (`HabitBoard::release`), a type and a registry that
> were deleted. The **only** rule in this list that survives is the `MUST NOT` on publishing —
> and it survives because it belongs to [[adr-0007-habit-lifecycle-aggregate]] d3, not to this node.

- ~~**MUST**: keep the save order **habit, then board**, in `AnchorHabit::execute`.~~ **VOID** — there is one save.
- ~~**MUST**: keep both steps idempotent — `anchor()` assigns and `release()` is a silent no-op when the entry is absent.~~ **VOID** — `release` is deleted. (`anchor()` still assigns rather than toggles, but that is [[adr-0007-habit-lifecycle-aggregate]]'s rule, not this node's.)
- ~~**MUST**: free the seat through `HabitBoard::release`, never by mutating the registry from a use case.~~ **VOID — and actively misleading.** There is no seat to free, no `release`, and no registry. A seat is free when no non-anchored habit occupies it, computed live at the gesture ([[adr-0013-set-based-validation-outside-aggregates]]).
- **MUST NOT**: publish any lifecycle event, or introduce a `HabitEvent` enum ([[adr-0007-habit-lifecycle-aggregate]] d3). **Still in force** — and now unconditional: the outbox and `DomainEventPublisher` are deleted, so the codebase publishes **nothing at all**.
- ~~**MUST NOT**: teach `HabitBoard` to filter its own registry on a habit's lifecycle state.~~ **VOID** — the filter on `LifecycleState` is now exactly what the rule *is*, and it lives in `core/src/habit_management/use_cases/add_habit.rs`. The prohibition was a consequence of the mistaken boundary.
- ~~**Constraint carried to slice 7**: readmission must not republish `HabitRequested`.~~ **VOID** — see "What slice 7 actually faces" in the amendment below.
- ~~**Escalation triggers** (1) persistence, (2) concurrency, (3) a third coordination.~~ **VOID as written.** Triggers (1) and (2) did not disappear — they were **inherited and sharpened** by [[adr-0013-set-based-validation-outside-aggregates]], whose Security escalation trigger is the one to read. Trigger (3) is gone: there is no coordination to repeat.
- ~~**Out of scope**: transaction/saga/outbox machinery; readmission; a retry path.~~ **VOID**.

---

## Amendment — 2026-08-17, `drop-habit-board`: the question itself disappeared

This node was not wrong about *how to coordinate two aggregates*; it was reasoning correctly
about a coordination that should never have existed. The second aggregate was
`HabitBoard`, invented in [[adr-0002-habitboard-stateful-aggregate]] to host a rule that is
not an aggregate invariant. Delete it and the entire question evaporates — which is why this
node is **void**, not merely superseded in its answer.

### What each of this node's decisions cost, and what replaced it

| This node decided | Status |
|---|---|
| Save order habit → board, chosen for its failure mode | **Gone.** `AnchorHabit::execute` performs one write. There is no order and no failure mode to choose |
| Idempotence instead of a transaction, so replay converges | **Gone.** There is no partial state to converge from |
| A crash between the two saves loses a seat (cap stricter than the rule) | **Gone.** Security recorded the mirror-image finding on the creation path: the *old* double write could consume a seat without creating the habit — a self-inflicted business DoS by seat leak. The single write removes that defect class outright |
| `AnchorHabit` holds two repositories and orchestrates them | **Gone.** It holds `HabitRepository` alone and is byte-for-byte the same shape as `PauseHabit` / `ResumeHabit` (`core/src/habit_management/use_cases/anchor_habit.rs`) |
| Nothing is published ([[adr-0007-habit-lifecycle-aggregate]] d3 preserved) | **Survives, and hardens.** d3 was always the other node's rule; with the outbox and `DomainEventPublisher` deleted, nothing in the codebase can publish anything |

### What slice 7 (readmission) actually faces

The constraint this node carried forward — *"re-admit an existing habit id without republishing
`HabitRequested`"* — is meaningless: there is no `HabitRequested`, no board to re-admit onto, and
no event that creates a `Habit`. Readmission is now a **lifecycle transition on one aggregate**,
the exact mirror of anchoring:

1. `Habit::readmit()` moves the state out of `Anchored` — one aggregate, one method, one write,
   the shape of `PauseHabit` / `ResumeHabit` / `AnchorHabit`. ~~(or `resume()` reused)~~ —
   **struck 2026-08-17**: reusing `resume()` would mean widening its guard to admit `Anchored`,
   which re-opens the closed security finding verbatim. `Habit::resume()`'s guard must stay
   exactly `!= Paused` ([[adr-0007-habit-lifecycle-aggregate]] AD-9).
2. **The rejection path is real and it is set-based validation, not an aggregate rule.** A
   readmitted habit re-enters the daily life, so it must pass the *same* two guards `AddHabit`
   runs — the title may have been retaken while the habit was anchored (`readmit-habit` S3), and
   the daily life may be full. That check belongs in the `ReadmitHabit` use case, over
   `repository.all()` filtered on `state != Anchored`, per
   [[adr-0013-set-based-validation-outside-aggregates]].
3. **The lifecycle guard landed first — 2026-08-17, PR 2.** `Habit` has its transition table (an
   anchored habit cannot be *resumed*), a genuine invariant of one instance — see
   [[adr-0013-set-based-validation-outside-aggregates]]'s counter-example and
   [[adr-0007-habit-lifecycle-aggregate]] **AD-9**. Slice 7 is the first cycle where that guard is
   reachable from a screen, which is why PR 2 preceded it. **`ReadmitHabit` is not covered by
   PR 2's induction** — it is the first transition since `AddHabit` that increases the
   non-anchored count, so point 2's set check is mandatory, and it runs **before** the
   transition, with exactly one save after both.
