---
id: "adr-0002-habitboard-stateful-aggregate"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-16"
relations:
  related:
    - "architecture-overview"
  depends-on:
    - "adr-0001-validation-by-construction"
answers:
  - "Where do cross-habit invariants (max 5 in parallel, no duplicate title) live and why?"
  - "Why is HabitBoard a persisted stateful aggregate instead of a pure validator?"
  - "Why does the board record the entry at request time, before any Habit exists?"
  - "Why is the board loaded/saved through a dedicated HabitBoardRepository port?"
decided_in:
  - "LOCAL-2"
---

# ADR 0002 — HabitBoard becomes a persisted stateful aggregate

> **One-liner**: The cross-habit invariants (capacity ≤ 5, no duplicate title) define an aggregate boundary — `HabitBoard` holds a private registry, records each accepted request *at request time*, and is persisted through its own mono-board repository port.
> **Links**: [[architecture-overview]] (where applied), [[adr-0001-validation-by-construction]] (per-habit invariants this builds on).

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
- **Out of scope**: entry removal (the future "ancrée" rule — `BoardEntry.id` is the reserved hook), multi-board identity, Unicode normalization in `matches` (see [[architecture-overview]] gaps).
