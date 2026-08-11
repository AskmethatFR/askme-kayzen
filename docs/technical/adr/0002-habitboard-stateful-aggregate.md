---
id: "adr-0002-habitboard-stateful-aggregate"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-08-11"
relations:
  related:
    - "architecture-overview"
    - "adr-0012-synchronous-cross-aggregate-coordination"
  depends-on:
    - "adr-0001-validation-by-construction"
answers:
  - "Where do cross-habit invariants (max 5 in parallel, no duplicate title) live and why?"
  - "Why is HabitBoard a persisted stateful aggregate instead of a pure validator?"
  - "Why does the board record the entry at request time, before any Habit exists?"
  - "Why is the board loaded/saved through a dedicated HabitBoardRepository port?"
  - "How does an entry leave the board, and what is BoardEntry.id finally used for?"
  - "Why is releasing an absent entry a silent no-op rather than an error?"
decided_in:
  - "LOCAL-2"
  - "2026-08-11 slice 6 anchor-habit cycle (entry removal built — `release`)"
---

# ADR 0002 — HabitBoard becomes a persisted stateful aggregate

> **⚠️ Entry removal BUILT (2026-08-11, slice 6 `anchor-habit`)**: the "out of scope" line at the end of this node is **no longer true**. `HabitBoard::release` exists, `BoardEntry.id`'s reserved hook is now read, and the anchoring rule that motivated the reservation is settled. See the amendment block at the end of this node.

> **One-liner**: The cross-habit invariants (capacity ≤ 5, no duplicate title) define an aggregate boundary — `HabitBoard` holds a private registry, records each accepted request *at request time*, and is persisted through its own mono-board repository port. Since slice 6 it also **releases** an entry (the anchoring rule), which frees both the seat and the title.
> **Links**: [[architecture-overview]] (where applied), [[adr-0001-validation-by-construction]] (per-habit invariants this builds on), [[adr-0012-synchronous-cross-aggregate-coordination]] (who calls `release`, in what order, and why nothing is published).

## Context

LOCAL-2 introduced two rules that no single habit can enforce: **max 5 habits in parallel** and **no duplicate title** (trim + case-insensitive). A rule that spans several habits needs one object that sees them all and mutates atomically — the textbook definition of an aggregate boundary. LOCAL-1's board was a stateless validator/emitter; that shape cannot answer "how many habits exist?" or "is this title taken?".

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Aggregate boundary | `HabitBoard` owns the cross-habit invariants; it is a stateful aggregate with a **private registry** `Vec<BoardEntry { id, title }>` | `core/src/habit_management/domain/habit_board.rs` |
| Record at request time | `request_habit(&mut self)` checks VOs → duplicate → capacity, **records the entry**, then emits. Soundness: a 2nd identical request sees the 1st **before any `Habit` exists** (habit creation is deferred to the event handler) | `core/src/habit_management/domain/habit_board.rs` |
| Registry = source of truth | The count and duplicate check read ONLY the registry — never re-seeded or re-derived from `HabitRepository` | — |
| Persistence port | `HabitBoardRepository { load() -> HabitBoard, save(&HabitBoard) }`, mono-board | `core/src/habit_management/domain/habit_board_repository.rs`, `core/src/habit_management/infrastructure/in_memory_habit_board_repository.rs` |
| Use case shape | `RequestHabit::execute` = **load → mutate → save → publish**; the `?` on `request_habit` short-circuits before save/publish — rejection is structurally non-destructive | `core/src/habit_management/use_cases/request_habit.rs` |
| Check precedence | VOs → duplicate → capacity; **duplicate wins on a full board** (pinned by test) | tests in `habit_board.rs` |

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
| Removal | `release(&mut self, id: &HabitId)` retains every entry whose id differs. The **first read of `BoardEntry.id`** since LOCAL-2 | `core/src/habit_management/domain/habit_board.rs` |
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
