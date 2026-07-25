---
id: "adr-0001-validation-by-construction"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-16"
relations:
  related:
    - "architecture-overview"
answers:
  - "Where do the habit business rules (title 1..=50 chars, initial duration <= 5 min) live?"
  - "Why does HabitBoardEvent carry value objects instead of primitives?"
  - "Why is Habit::new infallible and why does the event handler not re-validate?"
  - "Why is CreateHabitOnRequest an application handler and not a domain service?"
decided_in:
  - "LOCAL-1"
  - "LOCAL-2"
---

# ADR 0001 — Validation by construction: VOs as single source of truth, events valid end-to-end

> **One-liner**: All habit invariants live in self-validating value objects; events carry those VOs, so anything published — and anything consumed — is valid by construction, and no layer ever re-validates.
> **Links**: [[architecture-overview]] — where this decision is applied.

## Context

The board-driven creation flow (LOCAL-1) splits habit creation into an emitting side (`HabitBoard::request_habit`) and a consuming side (`CreateHabitOnRequest`). Both sides touch the same business rules (`HabitTitle` 1..=50 chars, `InitialDuration` ≤ 5 min). Duplicating validation, or validating after emission, would either drift or let invalid events into the outbox.

*(LOCAL-2 renamed `HabitDescription` → `HabitTitle` and made the board return `HabitBoardError` — VO failures wrapped as `InvalidHabit(HabitError)`. The decision below is unchanged; anchors updated. See [[architecture-overview]] for the rename rationale.)*

## Decision

> **Anchor note (2026-07-25)**: the second VO was named `InitialDuration` when this
> decision was taken; `[[adr-0008-goal-based-dose-user-paced-progression]]` replaced it
> with `Goal`. The decision below is unchanged — only the VO's name and file moved,
> and the anchors point at the current file.

| Facet | Decision | Anchor |
|---|---|---|
| Single source of truth | The two business rules live ONLY in the VO constructors (`HabitTitle`, `InitialDuration`), moved out of `Habit::new` | `core/src/habit_management/domain/habit_title.rs`, `core/src/habit_management/domain/goal.rs` |
| Validate before emission | `HabitBoard::request_habit` builds the VOs and returns `Result<HabitBoardEvent, HabitBoardError>` — an event cannot exist unless validation passed | `core/src/habit_management/domain/habit_board.rs` |
| Events carry VOs | `HabitBoardEvent::HabitRequested` holds the VOs, not primitives — validity is enforced by the type system end-to-end | `core/src/habit_management/domain/habit_board_event.rs` |
| Parse, don't validate | `Habit::new(HabitId, HabitTitle, InitialDuration) -> Habit` is infallible; the domain speaks `HabitId`, never raw `String` | `core/src/habit_management/domain/habit.rs` |
| Event is a fact | `CreateHabitOnRequest` is an **application** event handler (not a domain service): it consumes the event without re-validation and calls `HabitRepository::save` | `core/src/habit_management/use_cases/create_habit_on_request.rs` |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Shared validation function called by both board and `Habit::new` | Rule written once but *executed* twice — the consumer still distrusts the event; type system enforces nothing |
| Board constructs a throwaway `Habit` to validate | Hack; couples the board to `Habit`'s lifecycle for a side effect |
| `CreateHabitOnRequest` as a domain service | Injecting a repository port into a domain service is a smell; with all invariants in the VOs, the handler has no domain logic left — it is orchestration |

## Consequences / Constraints

- **MUST**: any new habit invariant goes into a VO constructor (or a new VO) — never into `Habit::new`, never into a handler.
- **MUST**: any new domain event carries VOs/domain types, never raw primitives.
- **MUST NOT**: re-validate event payloads downstream; a consumed event is a fact.
- **MUST NOT**: make `Habit::new` fallible again.
- **Out of scope**: cross-habit invariants — settled since LOCAL-2 by [[adr-0002-habitboard-stateful-aggregate]].
