---
id: "adr-0002-habitboard-stateful-aggregate"
type: "technical"
owner: "architect"
status: "superseded"
updated: "2026-08-17"
relations:
  # No hand-written `superseded-by` here: it is an INVERSE edge, derived at query
  # time from adr-0013's own `supersedes`.
  related:
    - "architecture-overview"
    - "adr-0012-synchronous-cross-aggregate-coordination"
    - "adr-0013-set-based-validation-outside-aggregates"
  depends-on:
    - "adr-0001-validation-by-construction"
answers:
  - "~~Where do cross-habit invariants (max 5 in parallel, no duplicate title) live and why?~~ SUPERSEDED by adr-0013 — they are not invariants; they live in AddHabit"
  - "~~Why is HabitBoard a persisted stateful aggregate instead of a pure validator?~~ SUPERSEDED — HabitBoard is deleted"
  - "Why was HabitBoard a mistake rather than a design that outlived its usefulness? (historical premise recorded here, verdict in adr-0013)"
  - "What did adr-0002 object to about counting the habits, and did each objection hold?"
decided_in:
  - "LOCAL-2"
  - "2026-08-11 slice 6 anchor-habit cycle (entry removal built — `release`)"
  - "2026-08-17 drop-habit-board refactor (SUPERSEDED)"
---

# ADR 0002 — HabitBoard becomes a persisted stateful aggregate

> **⚠️ SUPERSEDED (2026-08-17) by [[adr-0013-set-based-validation-outside-aggregates]].** The premise of this node — that capacity ≤ 5 and title-uniqueness are **aggregate invariants defining a boundary** — is the error adr-0013 corrects. They are **set-based validation**: the real predicate (`state != Anchored`) depends on another aggregate's *mutable* state, so no aggregate can host it. `HabitBoard`, `HabitBoardRepository`, `HabitBoardEvent`, `BoardEntry`, `release`, the outbox and the request/handler split are **all deleted**; the two rules now live in `core/src/habit_management/use_cases/add_habit.rs`. This document is retained as a **historical record** — every code anchor below points at a file that no longer exists. Its three objections to "count the habits instead" are re-judged in the 2026-08-17 amendment at the end.

> **One-liner** *(historical)*: The cross-habit invariants (capacity ≤ 5, no duplicate title) define an aggregate boundary — `HabitBoard` holds a private registry, records each accepted request *at request time*, and is persisted through its own mono-board repository port. Since slice 6 it also **releases** an entry (the anchoring rule), which frees both the seat and the title.
> **Links**: [[adr-0013-set-based-validation-outside-aggregates]] (what replaced this), [[architecture-overview]] (where it was applied), [[adr-0001-validation-by-construction]] (per-habit invariants — **unaffected**, they stay in the VO constructors), [[adr-0012-synchronous-cross-aggregate-coordination]] (the coordination this design forced, void with it).

## Context

LOCAL-2 introduced two rules that no single habit can enforce: **max 5 habits in parallel** and **no duplicate title** (trim + case-insensitive). A rule that spans several habits needs one object that sees them all and mutates atomically — the textbook definition of an aggregate boundary. LOCAL-1's board was a stateless validator/emitter; that shape cannot answer "how many habits exist?" or "is this title taken?".

## Decision

> Every anchor in the two tables below names a **file deleted on 2026-08-17**; they are written
> without code formatting so the doc-anchor gate does not resolve them. Nothing here describes
> the current code.

| Facet | Decision *(historical)* | Anchor *(deleted)* |
|---|---|---|
| Aggregate boundary | `HabitBoard` owns the cross-habit invariants; it is a stateful aggregate with a **private registry** `Vec<BoardEntry { id, title }>` | core/src/habit_management/domain/habit_board.rs |
| Record at request time | `request_habit(&mut self)` checks VOs → duplicate → capacity, **records the entry**, then emits. Soundness: a 2nd identical request sees the 1st **before any `Habit` exists** (habit creation is deferred to the event handler) | core/src/habit_management/domain/habit_board.rs |
| Registry = source of truth | The count and duplicate check read ONLY the registry — never re-seeded or re-derived from `HabitRepository`. **This is the facet adr-0013 identifies as the error**: it forced a second copy of a fact `Habit` already owned | — |
| Persistence port | `HabitBoardRepository { load() -> HabitBoard, save(&HabitBoard) }`, mono-board | core/src/habit_management/domain/habit_board_repository.rs, core/src/habit_management/infrastructure/in_memory_habit_board_repository.rs |
| Use case shape | `RequestHabit::execute` = **load → mutate → save → publish**; the `?` on `request_habit` short-circuits before save/publish — rejection is structurally non-destructive | core/src/habit_management/use_cases/request_habit.rs |
| Check precedence | VOs → duplicate → capacity; **duplicate wins on a full board** (pinned by test) | tests in habit_board.rs |

**The one facet that survived**: check precedence. `AddHabit::execute` still runs the duplicate
guard before the capacity guard, for the reason recorded here — on a full daily life, a
re-submitted title must be told it is a duplicate ([[adr-0013-set-based-validation-outside-aggregates]]).

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Counting / dedup via `HabitRepository` queries in the use case | Invariant leaks out of the domain into orchestration; non-atomic — two concurrent requests both pass the query; and unsound: the habit only exists after the event handler runs, so the query cannot see a just-accepted request |
| Event-sourced board (rebuild registry from event stream) | Over-engineering for two invariants and one event type; nothing today needs the history |
| Shared `Rc<RefCell<HabitBoard>>` held by the use case, no port | Welds the aggregate's lifecycle to the use case instance; no seam for persistence; hides the load → mutate → save transaction shape |

## Consequences / Constraints

- **MUST**: put any future cross-habit invariant inside `HabitBoard` — never in a use case, never as a repository query.
- **MUST**: keep the registry the sole source of truth for board state; `HabitRepository` is downstream, not an input to board decisions.
- **MUST**: preserve the load → mutate → save → publish order and the non-destructive rejection (`Err` ⇒ no save, no publish).
- **MUST NOT**: expose the registry publicly or let another aggregate mutate it.
- **MUST**: keep every cross-habit rule computed over the registry alone — including after slice 6's `release` (see the amendment: the board must never filter its own entries on a habit's lifecycle state).
- **~~Out of scope: entry removal~~ — BUILT 2026-08-11 (see amendment)**. Still out of scope: multi-board identity, Unicode normalization in `matches` (see [[architecture-overview]] gaps).

---

## Amendment — 2026-08-11, slice 6 `anchor-habit`: the board releases an entry, and the reserved hook is spent

Slice 6 delivers the "ancrée" rule this node reserved `BoardEntry.id` for. The question is
the one this node already owns — *how does the board maintain its two invariants* — so it
is amended in place rather than superseded. Who *calls* `release`, in what order, and why
nothing is published is a different question, owned by
[[adr-0012-synchronous-cross-aggregate-coordination]].

### The seat is freed by **removing the entry**, not by filtering the count

| Facet | Decision | Anchor |
|---|---|---|
| Removal | `release(&mut self, id: &HabitId)` retains every entry whose id differs. The **first read of `BoardEntry.id`** since LOCAL-2 | core/src/habit_management/domain/habit_board.rs |
| Absent entry | **Silent no-op** — no `Result`, no error variant. Releasing something that is not there has already achieved its purpose | idem |
| Capacity | Unchanged: still `requests.len()`, still with no state filter | idem |

The alternative was to leave the entry in place and teach the capacity check to count only
*non-anchored* habits. It was rejected on the ADR's own terms: both invariants are computed
over `requests`, so counting non-anchored habits would force the board to learn a fact that
lives in **another aggregate** — the habit's `LifecycleState` — on every single request.
The invariant would stop being local, and the registry would stop being the sole source of
truth, which is this node's second **MUST**. Removing the entry keeps everything where it
already was; it simply spends the hook.

**Why the no-op rather than an error.** `release` is one half of a two-aggregate
orchestration whose only recovery mechanism is **replay** ([[adr-0012-synchronous-cross-aggregate-coordination]]):
a second run must converge, not fail. An `EntryNotFound` variant would turn the recovery
path into the failure path, and it would give the caller nothing to do with the
information.

### The released title is released too — deliberately

Both invariants read the same registry, so dropping an entry frees **the seat and the
title**: a new habit may take an anchored habit's title. That is what `readmit-habit` S3
already specifies. The consequence it places on slice 7's readmission is recorded in
[[adr-0012-synchronous-cross-aggregate-coordination]], which owns the coordination.

---

## Amendment — 2026-08-17, `drop-habit-board`: SUPERSEDED, and the three objections judged

This node's premise is withdrawn by [[adr-0013-set-based-validation-outside-aggregates]]: the
two rules are **set-based validation**, not invariants, so no aggregate — this one or a better
one — could have hosted them. Read that node for the test, the current home of the rules, and
why this was a mistake rather than a design that aged out.

What is recorded **here** is the fate of this node's own argument, because it was a good
argument and its three objections to "count the habits in the use case" deserve verdicts
rather than silence.

| This node's objection to counting habits | Verdict, measured on the delivered code |
|---|---|
| *"Invariant leaks out of the domain into orchestration"* | **TRUE — a purity objection, upheld and paid.** The rule is now five readable lines in `core/src/habit_management/use_cases/add_habit.rs` instead of a type that guards itself. That is the accepted price of not pretending a set-based rule is an invariant, and it is cheaper than the two-writes design it replaces. adr-0013 records it openly so it is not rediscovered as a defect |
| *"Non-atomic — two concurrent requests both pass the query"* | **UNREACHABLE in this runtime, and enforced by the type system rather than by convention.** `Rc<dyn HabitRepository>` is unconditionally `!Send`/`!Sync`, the in-memory adapter uses `RefCell`, and there is no suspension point between the read and the write (zero `async`/`.await`/`thread::spawn`/`Arc`/`Mutex` in `core/src` + `app/src`). Security verified each leg and wrote the three conditions that reopen it — verbatim in [[adr-0013-set-based-validation-outside-aggregates]] |
| *"Unsound — the habit only exists after the event handler runs"* | **DIES WITH THE HANDLER.** The unsoundness was manufactured by the event indirection this very design required. `AddHabit::execute` reads the set and writes the habit in one call, so there is no interval in which an accepted request is invisible |

One of three still stands, and it is the cheapest of the three. That ratio is the measurement
this amendment exists to record.

### What this node's deletion removed, beyond the rule's relocation

Security credited the refactor with **removing a defect class**, not merely relocating a
check: the old non-transactional double write (board entry, then habit) could consume a seat
without ever creating the habit — a self-inflicted business denial of service by seat leak.
`AddHabit::execute` performs exactly **one** write, so the partial state is not representable.
