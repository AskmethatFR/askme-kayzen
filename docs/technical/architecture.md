---
id: "architecture-overview"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-27"
relations:
  related:
    - "adr-0011-one-public-method-per-use-case"
    - "adr-0001-validation-by-construction"
    - "adr-0002-habitboard-stateful-aggregate"
    - "adr-0003-two-crate-workspace"
    - "adr-0004-routing-flat-enum"
    - "adr-0006-cqrs-light"
    - "adr-0007-habit-lifecycle-aggregate"
    - "adr-0008-goal-based-dose-user-paced-progression"
    - "adr-0009-quality-gates"
    - "adr-0010-crate-boundary-trust-boundary"
answers:
  - "Where is the trust boundary, and where does a raw URL segment become a domain type?"
  - "Which screens are wired to a use case today, and which are still stubs?"
  - "What bounded contexts exist and how are they layered?"
  - "How is the repository laid out (workspace, crates) and who may depend on whom?"
  - "How does habit creation flow through the system (board-driven, event-mediated)?"
  - "Is HabitBoard stateful? What state does it hold and who persists it?"
  - "Why is there no outbox dispatcher / main wiring / handler idempotence yet?"
  - "Why was HabitDescription renamed to HabitTitle, and why does matches() differ from PartialEq?"
  - "How is the app shell structured (Router, route enum, views) and what platform is targeted first?"
  - "Why are there two use cases for the goal gestures instead of one AdjustGoal?"
  - "Which lifecycle behavior is built today, and which is still planned?"
decided_in:
  - "LOCAL-1"
  - "LOCAL-2"
  - "LOCAL-3"
  - "LOCAL-4"
  - "2026-07-27 slice 3 adjust-goal cycle"
---

# Habit Management — Architecture Overview

> **One-liner**: Two-crate Cargo workspace — pure domain lib `kayzen-core` / Dioxus shell `kayzen-app` — single bounded context `habit_management`, hexagonal layering, where habit creation goes through the **stateful `HabitBoard` aggregate** (cross-habit invariants) and is event-mediated through a transactional outbox.
> **Links**: [[adr-0001-validation-by-construction]] (invariant model), [[adr-0002-habitboard-stateful-aggregate]] (aggregate boundary & persistence), [[adr-0003-two-crate-workspace]] (workspace split & dependency rule enforcement), [[adr-0004-routing-flat-enum]] (app-shell routing skeleton), [[adr-0007-habit-lifecycle-aggregate]] (`Habit` promoted to the lifecycle aggregate root), [[adr-0008-goal-based-dose-user-paced-progression]] (single `Goal` VO + user-paced progression, no suggestion), [[adr-0010-crate-boundary-trust-boundary]] (the crate boundary is the trust boundary — where a primitive becomes a domain type), [[adr-0009-quality-gates]] (what the two gates prove, and the four named blind spots that limit them), [[adr-0011-one-public-method-per-use-case]] (the application layer's unit of responsibility — one public method, and the duplication that is deliberately kept).

## Workspace layout (since LOCAL-3 — settled in [[adr-0003-two-crate-workspace]])

| Crate | Kind | Contents | Dependencies |
|---|---|---|---|
| `kayzen-core` (`core/`) | lib | `habit_management/` + `shared/` — the whole domain, use cases, in-memory infra | `uuid` only (`js` feature target-scoped to wasm32); **zero** dioxus/web-sys/wasm-bindgen |
| `kayzen-app` (`app/`) | bin | Dioxus 0.7 shell: `Dioxus.toml`, `assets/`, `main.rs` (`App` hosts `Router::<Route>`; `document::Link` assets above it), `route.rs` (flat route enum, 4 tests), `views/` (7 screen stubs + `mod.rs`) | `kayzen-core` + `dioxus`; feature-per-platform `default = ["web"]`, `web`/`desktop`/`mobile` |

Dependency edge is **one-way `app → core`, compiler-enforced** — core cannot reference app or any UI crate. Do not re-decide the split, crate names, or feature-per-platform shape.

## Bounded context & layers

One bounded context: **`habit_management`** (`core/src/habit_management/`), plus a small `core/src/shared/` kernel (`core/src/shared/guid_generator.rs`, the `Clock` port `core/src/shared/clock.rs`, and the library-free `LocalDate` VO `core/src/shared/local_date.rs` — [[adr-0007-habit-lifecycle-aggregate]]).

| Layer | Contents | Anchors |
|---|---|---|
| Domain | `Habit` (**the lifecycle aggregate root** — [[adr-0007-habit-lifecycle-aggregate]]; `toggle_done` / `is_done_on` / `grow` / `lighten`), `HabitBoard` (stateful aggregate), `HabitBoardEvent`, `HabitBoardError`, VOs (`HabitTitle`, `HabitId`, `Goal`, `CompletionHistory`, `StepHistory`, `LocalDate`; **planned**: `LifecycleState`), ports (`HabitRepository`, `HabitBoardRepository`, `DomainEventPublisher`, `Clock`) | `core/src/habit_management/domain/`, `core/src/shared/` |
| Application — commands | `RequestHabit`, `MarkDone`, `GrowGoal`, `LightenGoal`, `CreateHabitOnRequest` (event handler). **Each takes primitives and parses them** — this is the trust boundary (see below). **Each exposes exactly one public method** ([[adr-0011-one-public-method-per-use-case]]) — which is why the two goal gestures are two types, not one `AdjustGoal` with two methods | `core/src/habit_management/use_cases/request_habit.rs`, `core/src/habit_management/use_cases/mark_done.rs`, `core/src/habit_management/use_cases/grow_goal.rs`, `core/src/habit_management/use_cases/lighten_goal.rs`, `core/src/habit_management/use_cases/create_habit_on_request.rs` |
| Application — queries | `ListBoardHabits`, `GetHabitDetail` — flat `snake_case` modules under `queries/`, returning per-screen DTOs ([[adr-0006-cqrs-light]]). `HabitDetail` carries `next_goal_up` / `next_goal_down`, **derived on read** — the floor is a business rule, so the view renders the number but never computes it ([[adr-0008-goal-based-dose-user-paced-progression]]) | `core/src/habit_management/queries/list_board_habits.rs`, `core/src/habit_management/queries/get_habit_detail.rs` |
| Infrastructure | `InMemoryOutbox`, `InMemoryHabitRepository`, `InMemoryHabitBoardRepository` | `core/src/habit_management/infrastructure/` |
| Presentation (shell + wired screens) | Dioxus `App` hosting `Router::<Route>`; flat `Route` enum mirroring the designer's 6 screens + NotFound catch-all ([[adr-0004-routing-flat-enum]]); composition root = `Services`, a pure DI registry provided once at the app root. **Today, Add and Detail are wired** to their use cases/queries — Detail carries the « Ajuster, à votre rythme » zone with **two unconditional buttons** (grow / lighten, no precondition, never disabled) driving `GrowGoal` / `LightenGoal`; Ritual, Week and Ancrées remain stubs. Target: **Android-first**, all dev on the web platform for speed | `app/src/main.rs`, `app/src/route.rs`, `app/src/composition.rs`, `app/src/views/habit_detail.rs`, `app/src/services/add_habit.rs` |

## Trust boundary (settled 2026-07-26 in [[adr-0010-crate-boundary-trust-boundary]])

The system's **anticorruption layer is the `kayzen-core` crate boundary** — not the Dioxus view. Because [[adr-0006-cqrs-light]] forbids `kayzen-app` from importing a domain type and [[adr-0003-two-crate-workspace]] makes that edge compiler-enforced, every use case's and query's entry point (**primitives in, DTO or domain error out**) is *structurally* the only place a primitive can become a domain type.

```
URL /habit/:id                                    ← untrusted
  → app view forwards the raw String              ← never parses, cannot: it may not import HabitId
  ‖ ══════════════ crate boundary = trust boundary ══════════════
  → GetHabitDetail::handle(&str) / MarkDone::execute(&str) / RequestHabit::execute(String, u32)
      → HabitId::new(&str) -> Result<HabitId, HabitError>   ← THE single door (1..=64, no trim)
  → domain, where nothing re-validates
```

| Rule | Where |
|---|---|
| One door per VO: no `From`, no `Deserialize`, no public field bypassing the validating constructor | `core/src/habit_management/domain/habit_id.rs` |
| Parse at the entry point, never deeper, never in `kayzen-app` | `core/src/habit_management/queries/get_habit_detail.rs`, `core/src/habit_management/use_cases/mark_done.rs`, `core/src/habit_management/use_cases/request_habit.rs` |
| Refusal rides each site's existing failure path — no public signature changed | idem |
| **Not covered**: the `Ritual` route never crosses into the core (its view re-injects the raw parameter into a `Link`) | `app/src/views/ritual.rs` |

Seven escalation triggers reopen this decision — persistence, multi-user, id length, **SSR in production deps**, id-as-sink-key, `Deserialize` on `HabitId`, user-supplied ids. They are listed verbatim in [[adr-0010-crate-boundary-trust-boundary]]; do not re-derive them.

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

## Habit lifecycle write side (**goal facets built since slice 3**; pause/anchor still planned — settled in [[adr-0007-habit-lifecycle-aggregate]])

Beyond creation, `Habit` is **promoted to the lifecycle aggregate root** (one aggregate, keyed by `HabitId`) and grows behavior vertically across slices 2/3/5/6/7 of [[lifecycle-backlog]]. **Slices 2 and 3 have landed**: mark-done, and the two goal gestures.

- **The dose is a single `Goal` VO** (default 5, floor 1, **no upper ceiling**; the ≤5-min creation guard dropped) — a **soft daily target, not a commitment**; completion stays **binary**. Progression is **user-paced**: `grow()` / `lighten()` (±1) are **always-available gestures the system NEVER suggests** — there is **no `StabilityPolicy`, no stability detection, no growth/anchor suggestion** ([[adr-0008-goal-based-dose-user-paced-progression]], superseding ADR-0005).
- **Two dated histories inside `Habit`** — `CompletionHistory` (ordered set of `LocalDate`, one completion/day structurally, kept forever; `toggle_done(today)` insert/remove) and `StepHistory` (the **self-paced staircase**, `{ first: StepChange, rest: Vec<StepChange> }` so non-emptiness is **structural** and `current()` is total — no `Option`, no `unwrap`, no panic path). `grow()` / `lighten()` **append** steps via `record`; the history never removes, pops or merges, and two changes on the same day stay two dated steps.
- **The floor at 1 min is a business rule, not a clamp** — below one minute there is no shorter habit, there is no habit, and the exit is deletion (named, not yet designed). It is enforced by `Goal` construction and by `Habit::lighten`, which **infers** the floor (`lightened() == *current()` → silent no-op) rather than restating `Goal::MIN` — the constant stays owned by `Goal`. Lightening at the floor is a **silent no-op**: no error, no UI signal; feedback is by state, not by reproach ([[adr-0007-habit-lifecycle-aggregate]] d2, settled 2026-07-27).
- **`LifecycleState {Active, Paused, Anchored}`** enum (illegal combos unrepresentable); `toggle_done` never inspects it (paused/anchored habits stay markable-done). Board↔habit anchoring coordination is deferred to slice 6.
- **Time enters through a `Clock` port** (`today() -> LocalDate`) in `shared/`; the domain owns a **library-free `LocalDate` VO** (epoch-day integer, **no `chrono` in its public API**), and `chrono` is confined to the infra `SystemClock` adapter. Aggregate methods take `today: LocalDate` as a **plain parameter** — the aggregate stays a pure function, no clock stub in domain tests.
- **Lifecycle mutations are internal state transitions** (load → method → save), **NOT** outbox events — there is no subscriber and [[adr-0006-cqrs-light]] has no projections to feed. Only `HabitRequested` is published; `HabitBoardEvent` + outbox untouched, no `HabitEvent` enum.
- **`HabitRepository`** gains `get(&HabitId) -> Option<Habit>` + upsert-by-id `save` (slice 2).

Still planned: `LifecycleState`, pause/resume, anchor/readmit and the board↔habit anchoring coordination (slices 5/6/7).

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
| Outbox drained by a **synchronous in-process dispatcher** | `AddHabit` (app service) requests on the board, then drains the outbox and hands each event to `CreateHabitOnRequest` in the same call. The dispatcher lives at the composition root, not in the domain, so making it asynchronous later stays additive | A background/async dispatcher before anything needs one |
| View helpers are `#[must_use]` (slice 3) | `grow_and_reload` / `lighten_and_reload` in `app/src/views/habit_detail.rs` return the refreshed detail; discarding it silently drops the screen refresh. `#[must_use]` makes that a compile error — the one class of view defect a lint can hold (it does **not** reach a helper *swap*, see [[adr-0009-quality-gates]] L1) | Trusting the render test to catch a dropped refresh (it cannot — the HTML is identical) |
| `FixedClock` is **duplicated per test module in `app`** (accepted, 3 copies) | Core's `FixedClock` is `#[cfg(test)] pub(crate)` and genuinely unreachable cross-crate, so each app test module defines its own: `app/src/services/add_habit.rs`, `app/src/views/today.rs`, `app/src/views/habit_detail.rs`. **Not a defect, not yet a decision** — sweeping it was ruled out of slice 3's surface. The real fork (a `test-support` feature on `kayzen-core` vs. a small app-side test helper module) is undecided | Exporting core's test double unconditionally (would put a test-only type in the production API) |

## Deliberately does NOT exist yet (human constraint — manual dev resumes after these cycles)

- Any **persistence**: every store is in-memory, so nothing survives a restart, and `Services::new` seeds three demo habits at startup
- UI wiring **beyond Today, Add and Detail** — those three screens call `ListBoardHabits`, `MarkDone`, `AddHabit` and `GetHabitDetail` through the `Services` registry provided at the app root (`app/src/composition.rs`); **Ritual, Week and Ancrées stay stubs**. The route `:id` stays a `String` — deliberately, and now permanently: it is parsed once at the core's entry point, never in the view ([[adr-0010-crate-boundary-trust-boundary]])
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
- **MUST**: in the app shell, navigate via explicit `Link { to: Route::X }` only (never `go_back()`), keep URL paths English and stable, and convert the `:id` `String` to a typed id **once, inside the core, at the use-case/query entry point** — [[adr-0004-routing-flat-enum]] said "once at the core-wiring boundary"; [[adr-0010-crate-boundary-trust-boundary]] settles *where* that once is, and it is **not** the view. (adr-0004's watch items still stand: unused `segments` prop in `not_found.rs`, stale-`done` closure bug in `today.rs`.)
- **MUST**: obtain every domain type through its single validating constructor; no `From`/`Deserialize`/public-field bypass ([[adr-0001-validation-by-construction]], [[adr-0010-crate-boundary-trust-boundary]]).
- **MUST NOT**: reintroduce a direct-creation command use case, or trait abstractions without a second consumer.
- **MUST NOT**: make `PartialEq` on `HabitTitle` case-insensitive — `matches` is the business comparison.
- **Out of scope**: persistence beyond in-memory adapters; any UI.

## Open questions / Gaps

- [ ] Production event dispatching strategy (poll vs push, error handling, retries) — graph is silent.
- [ ] Idempotence / dedup strategy for `CreateHabitOnRequest` once a real dispatcher exists.
- [ ] "Ancrée" (anchored) rule: when/how an entry leaves the board (`BoardEntry.id` is the hook) — future cycle.
- [ ] Unicode ticket: NFC normalization in `HabitTitle::matches` + grapheme-based length validation (deferred together).
- [ ] **Habit deletion** — the affordance the floor-of-1 business rule implies (*« sinon on doit la supprimer »*). Named in [[adr-0008-goal-based-dose-user-paced-progression]]'s 2026-07-27 amendment, deliberately not designed. The next cycle meeting "the user wants less than one minute" starts here, not from a sub-minute goal.
- [ ] **Bounding or compacting `StepHistory` — a constraint on the persistence slice, not a free choice.** Alternating grow / lighten above the floor returns the goal to its starting value while appending **2 steps per round trip**, indefinitely, because [[adr-0007-habit-lifecycle-aggregate]] d6 forbids same-day fusion. Harmless in memory; unbounded storage once history survives a reload. Compaction is a *storage* decision — it must not be smuggled in as domain-level fusion, which would silently reverse d6.
- [ ] **`app/src/services/add_habit.rs` imports `HabitBoardError`**, a domain error type, in production code — a tension with [[adr-0006-cqrs-light]]'s "never imports a domain type" MUST. Pre-existing, untouched by slice 3, recorded there rather than fixed out of scope.
- [ ] **`FixedClock` triplication in the `app` test modules** (3 copies) — see the Local decisions table; the `test-support` feature vs. app-side helper fork is open.
- [ ] **Slice 5 will silently delete slice 3's view-mutant coverage.** Four `app/src/views/habit_detail.rs` mutants are held only by a `dead_code` lint that fires because each use case has exactly one caller; a second caller silences it. Closing it needs `VirtualDom` click dispatch, for which this repo has no precedent — [[adr-0009-quality-gates]] L1. Weigh in the slices 5/6 specs.
