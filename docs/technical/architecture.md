---
id: "architecture-overview"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-20"
relations:
  related:
    - "adr-0001-validation-by-construction"
    - "adr-0002-habitboard-stateful-aggregate"
    - "adr-0003-two-crate-workspace"
    - "adr-0004-routing-flat-enum"
    - "adr-0007-habit-lifecycle-aggregate"
answers:
  - "What bounded contexts exist and how are they layered?"
  - "How is the repository laid out (workspace, crates) and who may depend on whom?"
  - "How does habit creation flow through the system (board-driven, event-mediated)?"
  - "Is HabitBoard stateful? What state does it hold and who persists it?"
  - "Why is there no outbox dispatcher / main wiring / handler idempotence yet?"
  - "Why was HabitDescription renamed to HabitTitle, and why does matches() differ from PartialEq?"
  - "How is the app shell structured (Router, route enum, views) and what platform is targeted first?"
decided_in:
  - "LOCAL-1"
  - "LOCAL-2"
  - "LOCAL-3"
  - "LOCAL-4"
---

# Habit Management — Architecture Overview

> **One-liner**: Two-crate Cargo workspace — pure domain lib `kayzen-core` / Dioxus shell `kayzen-app` — single bounded context `habit_management`, hexagonal layering, where habit creation goes through the **stateful `HabitBoard` aggregate** (cross-habit invariants) and is event-mediated through a transactional outbox.
> **Links**: [[adr-0001-validation-by-construction]] (invariant model), [[adr-0002-habitboard-stateful-aggregate]] (aggregate boundary & persistence), [[adr-0003-two-crate-workspace]] (workspace split & dependency rule enforcement), [[adr-0004-routing-flat-enum]] (app-shell routing skeleton), [[adr-0007-habit-lifecycle-aggregate]] (`Habit` promoted to the lifecycle aggregate root).

## Workspace layout (since LOCAL-3 — settled in [[adr-0003-two-crate-workspace]])

| Crate | Kind | Contents | Dependencies |
|---|---|---|---|
| `kayzen-core` (`core/`) | lib | `habit_management/` + `shared/` — the whole domain, use cases, in-memory infra | `uuid` only (`js` feature target-scoped to wasm32); **zero** dioxus/web-sys/wasm-bindgen |
| `kayzen-app` (`app/`) | bin | Dioxus 0.7 shell: `Dioxus.toml`, `assets/`, `main.rs` (`App` hosts `Router::<Route>`; `document::Link` assets above it), `route.rs` (flat route enum, 4 tests), `views/` (7 screen stubs + `mod.rs`) | `kayzen-core` + `dioxus`; feature-per-platform `default = ["web"]`, `web`/`desktop`/`mobile` |

Dependency edge is **one-way `app → core`, compiler-enforced** — core cannot reference app or any UI crate. Do not re-decide the split, crate names, or feature-per-platform shape.

## Bounded context & layers

One bounded context: **`habit_management`** (`core/src/habit_management/`), plus a small `core/src/shared/` kernel (`guid_generator.rs`; a `Clock` port + library-free `LocalDate` VO join it as the lifecycle aggregate lands — [[adr-0007-habit-lifecycle-aggregate]]).

| Layer | Contents | Anchors |
|---|---|---|
| Domain | `Habit` (**promoted to the lifecycle aggregate root** — [[adr-0007-habit-lifecycle-aggregate]]), `HabitBoard` (stateful aggregate), `HabitBoardEvent`, `HabitBoardError`, VOs (`HabitTitle`, `InitialDuration`, `HabitId`; planned lifecycle VOs `CompletionHistory`, `StepHistory`, `Dose`, `LifecycleState`, `LocalDate`), ports (`HabitRepository`, `HabitBoardRepository`, `DomainEventPublisher`; planned `Clock`) | `core/src/habit_management/domain/`, `core/src/shared/` |
| Application (use cases) | `RequestHabit` (command side), `CreateHabitOnRequest` (event handler) | `core/src/habit_management/use-cases/request-habit/request-habit.rs`, `core/src/habit_management/use-cases/create-habit-on-request/create-habit-on-request.rs` |
| Infrastructure | `InMemoryOutbox`, `InMemoryHabitRepository`, `InMemoryHabitBoardRepository` | `core/src/habit_management/infrastructure/` |
| Presentation (shell only) | Dioxus `App` hosting `Router::<Route>`; flat `Route` enum mirroring the designer's 6 screens + NotFound catch-all ([[adr-0004-routing-flat-enum]]); `views/` = 7 screen **stubs** (French designer titles, placeholder data) — **wires no use case yet** (unchanged human constraint). Target: **Android-first**, all dev on the web platform for speed | `app/src/main.rs`, `app/src/route.rs`, `app/src/views/` |

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

## Habit lifecycle write side (planned — settled in [[adr-0007-habit-lifecycle-aggregate]])

Beyond creation, `Habit` is **promoted to the lifecycle aggregate root** (one aggregate, keyed by `HabitId`) and grows behavior vertically across slices 2/3/5/6/7 of [[lifecycle-backlog]]:

- **Two dated histories inside `Habit`** — `CompletionHistory` (ordered set of `LocalDate`, one completion/day structurally, kept forever; `toggle_done(today)` insert/remove) and `StepHistory` (dated `Vec<StepChange{on, dose}>`, seeded at creation; `current_dose()` = last step, never stored separately). `grow()` / `lighten()` push steps; the **floor at 1 min** is a true aggregate invariant (`Dose` construction + `lighten() = max(1, current-1)`).
- **`LifecycleState {Active, Paused, Anchored}`** enum (illegal combos unrepresentable); `toggle_done` never inspects it (paused/anchored habits stay markable-done). Board↔habit anchoring coordination is deferred to slice 6.
- **Time enters through a `Clock` port** (`today() -> LocalDate`) in `shared/`; the domain owns a **library-free `LocalDate` VO** (epoch-day integer, **no `chrono` in its public API**), and `chrono` is confined to the infra `SystemClock` adapter. Aggregate methods take `today: LocalDate` as a **plain parameter** — the aggregate stays a pure function, no clock stub in domain tests.
- **Lifecycle mutations are internal state transitions** (load → method → save), **NOT** outbox events — there is no subscriber and [[adr-0006-cqrs-light]] has no projections to feed. Only `HabitRequested` is published; `HabitBoardEvent` + outbox untouched, no `HabitEvent` enum.
- **`HabitRepository`** gains `get(&HabitId) -> Option<Habit>` + upsert-by-id `save` (slice 2).

This cycle wrote the ADR + docs only — **zero production code** (approved decision d4).

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

- Production outbox-draining dispatcher (`drain()` is test-only, `core/src/habit_management/infrastructure/in_memory_outbox.rs`)
- UI wiring of the use cases — the app shell now has a Router + 7 view stubs (LOCAL-4, [[adr-0004-routing-flat-enum]]) but still calls **no** use case; views render placeholder data. The route `:id` stays `String` until this boundary is wired
- Android `mobile-shell` concerns — hardware back-button wiring and intent-filters/App Links registration are **explicitly deferred** to a future `mobile-shell` ticket ([[adr-0004-routing-flat-enum]])
- Idempotence in `CreateHabitOnRequest`
- Entry **removal** from the board — the future "ancrée" (anchored) rule; `BoardEntry.id` is the reserved hook
- Unicode handling in `HabitTitle`: NFC normalization in `matches` + grapheme-based length — both deferred, flagged by Security review as low/UX; **handle together in one future ticket**

Do not build these speculatively; each requires a new decision cycle.

## Consequences / Constraints

- **MUST**: route habit creation through `RequestHabit` = load → `HabitBoard::request_habit` → save → publish; never create a `Habit` directly from a use case.
- **MUST**: treat the board's registry as the **source of truth** for the habit count and the duplicate check — never re-seed or re-derive it from `HabitRepository`.
- **MUST**: keep rejection non-destructive — on `Err`, neither `save` nor `publish` runs.
- **MUST**: respect the dependency rule — domain has no dependency on application or infrastructure; ports live in the domain.
- **MUST**: respect the crate boundary — `kayzen-core` stays free of UI/platform dependencies; the `app → core` edge is one-way (see [[adr-0003-two-crate-workspace]], incl. its known-debt follow-ups: Cargo.lock platform closure, `cargo audit` in CI, public in-memory test doubles).
- **MUST**: in the app shell, navigate via explicit `Link { to: Route::X }` only (never `go_back()`), keep URL paths English and stable, and convert the `:id` `String` to a typed id once at the core-wiring boundary (see [[adr-0004-routing-flat-enum]], incl. its watch items: unused `segments` prop in `not_found.rs`, stale-`done` closure bug in `today.rs` to fix at core-wiring time).
- **MUST NOT**: reintroduce a direct-creation command use case, or trait abstractions without a second consumer.
- **MUST NOT**: make `PartialEq` on `HabitTitle` case-insensitive — `matches` is the business comparison.
- **Out of scope**: persistence beyond in-memory adapters; any UI.

## Open questions / Gaps

- [ ] Production event dispatching strategy (poll vs push, error handling, retries) — graph is silent.
- [ ] Idempotence / dedup strategy for `CreateHabitOnRequest` once a real dispatcher exists.
- [ ] "Ancrée" (anchored) rule: when/how an entry leaves the board (`BoardEntry.id` is the hook) — future cycle.
- [ ] Unicode ticket: NFC normalization in `HabitTitle::matches` + grapheme-based length validation (deferred together).
