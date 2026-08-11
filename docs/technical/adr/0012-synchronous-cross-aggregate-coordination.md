---
id: "adr-0012-synchronous-cross-aggregate-coordination"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-08-11"
relations:
  related:
    - "architecture-overview"
    - "adr-0006-cqrs-light"
    - "adr-0011-one-public-method-per-use-case"
  depends-on:
    - "adr-0002-habitboard-stateful-aggregate"
    - "adr-0007-habit-lifecycle-aggregate"
answers:
  - "One gesture must change two aggregates — is that an event, a saga, or a synchronous orchestration?"
  - "Why is no HabitAnchored event published when anchoring frees a board seat?"
  - "In which order are the two aggregates saved, and what breaks if the order is inverted?"
  - "What happens if the process dies between the two saves?"
  - "What would force this decision to be reopened?"
  - "Why can a new habit retake an anchored habit's title, and what does that impose on readmission?"
decided_in:
  - "2026-08-11 slice 6 anchor-habit cycle"
---

# ADR 0012 — Cross-aggregate coordination is a synchronous application-service orchestration, not an event

> **One-liner**: When one user gesture must change two aggregates, the **command use case orchestrates them synchronously in one call and publishes nothing** — `AnchorHabit::execute` moves the `Habit` to `Anchored`, saves it, then removes its entry from the `HabitBoard` and saves that. Both transitions are **idempotent**, so a replay converges from any partial state; the save order (**habit first, board second**) is deliberate and chosen for its failure mode.
> **Links**: [[adr-0007-habit-lifecycle-aggregate]] (d3 — lifecycle mutations are internal transitions, never published; this node is where that rule meets a second aggregate), [[adr-0002-habitboard-stateful-aggregate]] (the board's load → mutate → save discipline and its `release` method, added in the same cycle), [[adr-0011-one-public-method-per-use-case]] (the use case that does the orchestrating still exposes exactly one public method), [[adr-0006-cqrs-light]] (why there is no projection to feed and therefore no subscriber), [[architecture-overview]] (where this is applied).

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
| How the seat is freed | Through a **board method**, `HabitBoard::release(&mut self, id)`, not by mutating the registry from the use case. The invariant stays inside the aggregate that owns it ([[adr-0002-habitboard-stateful-aggregate]] amendment, same cycle) | `core/src/habit_management/domain/habit_board.rs` |
| Save order | **Habit first, board second** — deliberately, for the failure mode it produces (see below). Not an accident of writing order | `core/src/habit_management/use_cases/anchor_habit.rs` |
| Recovery model | **Idempotence instead of a transaction.** `anchor()` assigns a state, `release()` retains over the registry and is a silent no-op when the entry is absent. Replaying the gesture from any partial state converges to the same result; nothing accumulates, nothing double-counts | `core/src/habit_management/domain/habit.rs`, `core/src/habit_management/domain/habit_board.rs` |
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

## Consequences / Constraints

- **MUST**: keep the save order **habit, then board**, in `AnchorHabit::execute`. The order is a decision with a stated failure model, not a formatting choice.
- **MUST**: keep both steps idempotent — `anchor()` assigns (it does not toggle or accumulate) and `release()` is a **silent no-op when the entry is absent**. Convergence-by-replay is the only recovery mechanism this design has; a guard that turns a missing entry into an error would remove it.
- **MUST**: free the seat through `HabitBoard::release`, never by mutating the registry from a use case ([[adr-0002-habitboard-stateful-aggregate]]).
- **MUST NOT**: publish any lifecycle event, or introduce a `HabitEvent` enum, to coordinate two aggregates ([[adr-0007-habit-lifecycle-aggregate]] d3 stands unamended).
- **MUST NOT**: teach `HabitBoard` to filter its own registry on a habit's lifecycle state.
- **Constraint carried to slice 7**: readmission re-admits an **existing** habit id — it must not republish `HabitRequested`, which would create a second `Habit` for the same identity. The title released by anchoring may legitimately have been retaken in the meantime, so readmission has a real rejection path to design.
- **Escalation triggers** — any one of these reopens this node: (1) the **persistence slice**, where a partial write survives a restart and "replay converges" stops being free; (2) **concurrency** of any kind, which makes the interval between the two saves observable; (3) a **third** cross-aggregate coordination, at which point the shape is a pattern worth naming rather than a decision taken twice.
- **Out of scope**: any transaction/saga/outbox machinery; readmission itself (slice 7); a retry path — nothing in the app replays the gesture today, and slice 6's UI removes every gesture from an anchored habit.
