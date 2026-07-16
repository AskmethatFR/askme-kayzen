---
id: "architecture-overview"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-16"
relations:
  related:
    - "adr-0001-validation-by-construction"
    - "adr-0002-habitboard-stateful-aggregate"
answers:
  - "What bounded contexts exist and how are they layered?"
  - "How does habit creation flow through the system (board-driven, event-mediated)?"
  - "Is HabitBoard stateful? What state does it hold and who persists it?"
  - "Why is there no outbox dispatcher / main wiring / handler idempotence yet?"
  - "Why was HabitDescription renamed to HabitTitle, and why does matches() differ from PartialEq?"
decided_in:
  - "LOCAL-1"
  - "LOCAL-2"
---

# Habit Management — Architecture Overview

> **One-liner**: Single bounded context `habit_management`, hexagonal layering, where habit creation goes through the **stateful `HabitBoard` aggregate** (cross-habit invariants) and is event-mediated through a transactional outbox.
> **Links**: [[adr-0001-validation-by-construction]] (invariant model), [[adr-0002-habitboard-stateful-aggregate]] (aggregate boundary & persistence).

## Bounded context & layers

One bounded context: **`habit_management`** (`src/habit_management/`), plus a small `src/shared/` kernel (`guid_generator.rs`).

| Layer | Contents | Anchors |
|---|---|---|
| Domain | `Habit`, `HabitBoard` (stateful aggregate), `HabitBoardEvent`, `HabitBoardError`, VOs (`HabitTitle`, `InitialDuration`, `HabitId`), ports (`HabitRepository`, `HabitBoardRepository`, `DomainEventPublisher`) | `src/habit_management/domain/` |
| Application (use cases) | `RequestHabit` (command side), `CreateHabitOnRequest` (event handler) | `src/habit_management/use-cases/request-habit/request-habit.rs`, `src/habit_management/use-cases/create-habit-on-request/create-habit-on-request.rs` |
| Infrastructure | `InMemoryOutbox`, `InMemoryHabitRepository`, `InMemoryHabitBoardRepository` | `src/habit_management/infrastructure/` |

## Habit creation flow (board-driven; stateful since LOCAL-2)

```
RequestHabit::execute
  → HabitBoardRepository::load() → HabitBoard        # mono-board port
  → GuidGenerator → HabitId
  → board.request_habit(&mut self, ...)              # VOs → duplicate → capacity → record entry → emit
      → Result<HabitBoardEvent::HabitRequested(VOs), HabitBoardError>
  → `?` short-circuits on rejection                  # nothing saved/published — rejection is
                                                     # structurally non-destructive
  → HabitBoardRepository::save(&board)
  → DomainEventPublisher (outbox port)               # event persisted transactionally
  → returns Result<HabitId, HabitBoardError>

CreateHabitOnRequest (application event handler)
  → handle(HabitBoardEvent)                          # event is a fact — no re-validation
  → Habit::new(HabitId, HabitTitle, InitialDuration) # infallible
  → HabitRepository::save
```

- Per-habit invariants (title length, duration) → VOs: settled in [[adr-0001-validation-by-construction]].
- Cross-habit invariants (max 5 in parallel, no duplicate title), the board's registry, record-at-request-time soundness, and the load → mutate → save → publish shape: settled in [[adr-0002-habitboard-stateful-aggregate]]. Do not re-decide either.
- Check precedence inside `request_habit`: **VOs → duplicate → capacity** (duplicate wins on a full board — pinned by test).

## Local decisions (non-ADR — settled here)

| Decision | Rationale | Rejected |
|---|---|---|
| `RequestHabit` is a concrete struct, no trait | YAGNI — one implementation, no consumer needing abstraction | Trait + impl pair |
| `CreateHabitCommand` trait + `CreateHabit` impl **deleted** (LOCAL-1) | Old direct-creation path replaced by the board-driven flow; abstraction had no consumer | Keeping the trait "for later" |
| `HabitBoardError` **resurrected** (LOCAL-2): `{ InvalidHabit(HabitError), DuplicateHabit, BoardFull { max } }` | LOCAL-1 deleted it as a *vacuous* enum; that condition expired the moment the board gained its own rejection reasons. Supersession, not contradiction | Flattening board rejections into `HabitError` (wrong owner: these are board rules, not habit rules) |
| Rename `HabitDescription` → `HabitTitle`, `HabitError::DescriptionLength` → `TitleLength` (LOCAL-2) | Ubiquitous language — the human calls it "un titre simple" | Keeping "description" as a technical synonym |
| `HabitTitle::new` trims before validating | Stored value is trimmed (case preserved); the length rule applies to the meaningful text | Validating the raw input |
| Duplicate matching via explicit `HabitTitle::matches` (trim + case-insensitive); `PartialEq` stays strict | Equality remains value equality; business-rule matching is an explicit, named domain operation | Overloading `PartialEq` with case-insensitive semantics (silently changes set/map behavior) |
| `HabitBoardRepository` is mono-board (`load() -> HabitBoard`, `save(&HabitBoard)`) | Exactly one board exists today; no identity parameter until a second board is a requirement | `load(BoardId)` speculative API |
| `BoardEntry.id` stored but not yet read | Reserved for the future "ancrée" (anchored) rule that removes an entry from the board. Deliberate, human-validated | Dropping the field and re-adding it later (would churn the persisted shape) |
| Outbox **without** production dispatcher | Transactional boundary proven by one end-to-end test draining the outbox inside the test; reversal is additive, so no ADR | Building a minimal production dispatcher now |

## Deliberately does NOT exist yet (human constraint — manual dev resumes after these cycles)

- Production outbox-draining dispatcher (`drain()` is test-only, `src/habit_management/infrastructure/in_memory_outbox.rs`)
- `main.rs` / UI wiring of the use cases
- Idempotence in `CreateHabitOnRequest`
- Entry **removal** from the board — the future "ancrée" (anchored) rule; `BoardEntry.id` is the reserved hook
- Unicode handling in `HabitTitle`: NFC normalization in `matches` + grapheme-based length — both deferred, flagged by Security review as low/UX; **handle together in one future ticket**

Do not build these speculatively; each requires a new decision cycle.

## Consequences / Constraints

- **MUST**: route habit creation through `RequestHabit` = load → `HabitBoard::request_habit` → save → publish; never create a `Habit` directly from a use case.
- **MUST**: treat the board's registry as the **source of truth** for the habit count and the duplicate check — never re-seed or re-derive it from `HabitRepository`.
- **MUST**: keep rejection non-destructive — on `Err`, neither `save` nor `publish` runs.
- **MUST**: respect the dependency rule — domain has no dependency on application or infrastructure; ports live in the domain.
- **MUST NOT**: reintroduce a direct-creation command use case, or trait abstractions without a second consumer.
- **MUST NOT**: make `PartialEq` on `HabitTitle` case-insensitive — `matches` is the business comparison.
- **Out of scope**: persistence beyond in-memory adapters; any UI.

## Open questions / Gaps

- [ ] Production event dispatching strategy (poll vs push, error handling, retries) — graph is silent.
- [ ] Idempotence / dedup strategy for `CreateHabitOnRequest` once a real dispatcher exists.
- [ ] "Ancrée" (anchored) rule: when/how an entry leaves the board (`BoardEntry.id` is the hook) — future cycle.
- [ ] Unicode ticket: NFC normalization in `HabitTitle::matches` + grapheme-based length validation (deferred together).
