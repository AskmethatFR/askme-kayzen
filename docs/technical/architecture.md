---
id: "architecture-overview"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-16"
relations:
  related:
    - "adr-0001-validation-by-construction"
answers:
  - "What bounded contexts exist and how are they layered?"
  - "How does habit creation flow through the system (board-driven, event-mediated)?"
  - "Why is there no outbox dispatcher / main wiring / board state yet?"
  - "Why were HabitBoardError and the CreateHabitCommand trait deleted?"
decided_in:
  - "LOCAL-1"
---

# Habit Management — Architecture Overview

> **One-liner**: Single bounded context `habit_management`, hexagonal layering, where habit creation is board-driven and event-mediated through a transactional outbox.
> **Links**: [[adr-0001-validation-by-construction]] — the invariant model behind this flow.

## Bounded context & layers

One bounded context: **`habit_management`** (`src/habit_management/`), plus a small `src/shared/` kernel (`guid_generator.rs`).

| Layer | Contents | Anchors |
|---|---|---|
| Domain | `Habit`, `HabitBoard`, `HabitBoardEvent`, VOs (`HabitDescription`, `InitialDuration`, `HabitId`), ports (`HabitRepository`, `DomainEventPublisher`) | `src/habit_management/domain/` |
| Application (use cases) | `RequestHabit` (command side), `CreateHabitOnRequest` (event handler) | `src/habit_management/use-cases/request-habit/request-habit.rs`, `src/habit_management/use-cases/create-habit-on-request/create-habit-on-request.rs` |
| Infrastructure | `InMemoryOutbox`, `InMemoryHabitRepository` | `src/habit_management/infrastructure/` |

## Habit creation flow (board-driven)

```
RequestHabit (use case)
  → GuidGenerator → HabitId
  → HabitBoard::request_habit(...)          # builds VOs = validates BEFORE emission
      → Result<HabitBoardEvent::HabitRequested(VOs), HabitError>
  → DomainEventPublisher (outbox port)       # event persisted transactionally
  → returns Result<HabitId, HabitError>

CreateHabitOnRequest (application event handler)
  → handle(HabitBoardEvent)                  # event is a fact — no re-validation
  → Habit::new(HabitId, HabitDescription, InitialDuration)   # infallible
  → HabitRepository::save
```

Why the pieces have this shape — invariants in VOs, events carrying VOs, infallible `Habit::new`, handler in the application layer — is settled in [[adr-0001-validation-by-construction]]. Do not re-decide it.

## Local decisions (non-ADR — settled here)

| Decision | Rationale | Rejected |
|---|---|---|
| `RequestHabit` is a concrete struct, no trait | YAGNI — one implementation, no consumer needing abstraction | Trait + impl pair |
| `CreateHabitCommand` trait + `CreateHabit` impl **deleted** | Old direct-creation path replaced by the board-driven flow; abstraction had no consumer. Behavior coverage migrated to the new flow's tests | Keeping the trait "for later" |
| `HabitBoardError` **deleted** | Vacuous enum; `HabitBoard::request_habit` returns `HabitError` directly | Wrapping `HabitError` in a board-specific error enum |
| Outbox **without** production dispatcher | Transactional boundary proven by one end-to-end test that drains the outbox inside the test; draining is deferrable infrastructure. Reversal is additive (add a dispatcher later), so no ADR | Building a minimal production dispatcher now (speculative infra, against the human constraint) |

## Deliberately does NOT exist yet (human constraint — manual dev resumes after LOCAL-1)

- Production outbox-draining dispatcher (`drain()` is test-only, `src/habit_management/infrastructure/in_memory_outbox.rs`)
- `main.rs` / UI wiring of the use cases
- `HabitBoard` state, identity, and repository (the board is currently a pure validator/emitter)
- Idempotence in `CreateHabitOnRequest`

Do not build these speculatively; each requires a new decision cycle.

## Consequences / Constraints

- **MUST**: route habit creation through `HabitBoard::request_habit` + the outbox event; never create a `Habit` directly from a use case.
- **MUST**: respect the dependency rule — domain has no dependency on application or infrastructure; ports live in the domain.
- **MUST NOT**: reintroduce a direct-creation command use case, a board-specific error enum, or trait abstractions without a second consumer.
- **Out of scope**: persistence beyond in-memory adapters; any UI.

## Open questions / Gaps

- [ ] Production event dispatching strategy (poll vs push, error handling, retries) — graph is silent.
- [ ] `HabitBoard` lifecycle: does the board become a persisted aggregate with identity? — graph is silent.
- [ ] Idempotence / dedup strategy for `CreateHabitOnRequest` once a real dispatcher exists.
