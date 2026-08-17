---
id: "architecture-overview"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-08-17"
relations:
  related:
    - "adr-0011-one-public-method-per-use-case"
    - "adr-0012-synchronous-cross-aggregate-coordination"
    - "adr-0013-set-based-validation-outside-aggregates"
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
  - "How does habit creation flow through the system (one use case, one write)?"
  - "~~Is HabitBoard stateful?~~ HabitBoard is DELETED — where do the cross-habit rules live now?"
  - "~~Why is there no outbox dispatcher / handler idempotence yet?~~ There is no outbox and nothing is published"
  - "Why was HabitDescription renamed to HabitTitle, and why does matches() differ from PartialEq?"
  - "How is the app shell structured (Router, route enum, views) and what platform is targeted first?"
  - "Why are there two use cases for the goal gestures instead of one AdjustGoal?"
  - "Which lifecycle behavior is built today, and which is still planned?"
  - "Which use cases hold a Clock, and why do pause/resume not?"
  - "What does the Today query return now that a screen has two zones?"
  - "Why is a gesture never written inline in an onclick here?"
  - "~~How does an entry leave the board, and what frees a seat?~~ There is no board; a seat is a live count"
  - "~~How does one gesture change two aggregates here?~~ No gesture does — there is one aggregate"
  - "Which rules are aggregate invariants here, and which are set-based validation?"
decided_in:
  - "LOCAL-1"
  - "LOCAL-2"
  - "LOCAL-3"
  - "LOCAL-4"
  - "2026-07-27 slice 3 adjust-goal cycle"
  - "2026-08-06 slice 5 pause-resume cycle"
  - "2026-08-11 slice 6 anchor-habit cycle"
  - "2026-08-17 drop-habit-board refactor (HabitBoard + outbox deleted; adr-0013)"
---

# Habit Management — Architecture Overview

> **One-liner**: Two-crate Cargo workspace — pure domain lib `kayzen-core` / Dioxus shell `kayzen-app` — single bounded context `habit_management`, hexagonal layering, **one aggregate (`Habit`)**, where habit creation is **one use case performing one write** and the cross-habit rules are **set-based validation read live from `HabitRepository`**, not aggregate invariants ([[adr-0013-set-based-validation-outside-aggregates]]). Nothing is published: there is no event, no outbox, no dispatcher.
> **Links**: [[adr-0013-set-based-validation-outside-aggregates]] (**the shape of the write side** — invariant vs set-based validation, and why `HabitBoard` was deleted), [[adr-0001-validation-by-construction]] (invariant model), [[adr-0002-habitboard-stateful-aggregate]] (**superseded** — the deleted board), [[adr-0003-two-crate-workspace]] (workspace split & dependency rule enforcement), [[adr-0004-routing-flat-enum]] (app-shell routing skeleton), [[adr-0007-habit-lifecycle-aggregate]] (`Habit` promoted to the lifecycle aggregate root), [[adr-0008-goal-based-dose-user-paced-progression]] (single `Goal` VO + user-paced progression, no suggestion), [[adr-0010-crate-boundary-trust-boundary]] (the crate boundary is the trust boundary — where a primitive becomes a domain type), [[adr-0009-quality-gates]] (what the two gates prove, and the five named blind spots that limit them — `fn new`, L1-bis views, L2, L3, L4 `match` arms), [[adr-0011-one-public-method-per-use-case]] (the application layer's unit of responsibility — one public method, and the duplication that is deliberately kept), [[adr-0012-synchronous-cross-aggregate-coordination]] (**superseded / void** — the two-aggregate coordination the board forced).

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
| Domain | **`Habit` — the one and only aggregate root** ([[adr-0007-habit-lifecycle-aggregate]]; `toggle_done` / `is_done_on` / `grow` / `lighten` / `pause` / `resume` / `anchor` / `state`, plus the published bound `MAX_IN_DAILY_LIFE = 5`), VOs (`HabitTitle`, `HabitId`, `Goal`, `CompletionHistory`, `StepHistory`, `LocalDate`), `HabitError`, `LifecycleState` enum (`Active`, `Paused`, `Anchored`), ports (`HabitRepository`, `Clock`). **`HabitBoard`, `HabitBoardEvent`, `HabitBoardError`, `HabitBoardRepository` and `DomainEventPublisher` are deleted** (2026-08-17) — the rules they held are set-based validation, not invariants ([[adr-0013-set-based-validation-outside-aggregates]]) | `core/src/habit_management/domain/`, `core/src/shared/` |
| Application — commands | `AddHabit`, `MarkDone`, `GrowGoal`, `LightenGoal`, `PauseHabit`, `ResumeHabit`, `AnchorHabit`. **Each takes primitives and parses them** — this is the trust boundary (see below). **Each exposes exactly one public method** ([[adr-0011-one-public-method-per-use-case]]) — which is why the two goal gestures, like the two lifecycle gestures, are two types each. **A use case holds a `Clock` iff its aggregate method takes `today`**: `PauseHabit` / `ResumeHabit` / `AnchorHabit` take **none** — nothing dates those transitions ([[adr-0007-habit-lifecycle-aggregate]] AD-3); `AddHabit` does, because `Habit::new` stamps `created_on`. **Every command touches exactly one aggregate and performs exactly one write.** `AddHabit` additionally *reads* the habit set to run the two set-based guards before that write ([[adr-0013-set-based-validation-outside-aggregates]]) | `core/src/habit_management/use_cases/add_habit.rs`, `core/src/habit_management/use_cases/mark_done.rs`, `core/src/habit_management/use_cases/grow_goal.rs`, `core/src/habit_management/use_cases/lighten_goal.rs`, `core/src/habit_management/use_cases/pause_habit.rs`, `core/src/habit_management/use_cases/resume_habit.rs`, `core/src/habit_management/use_cases/anchor_habit.rs` |
| Application — queries | `ListBoardHabits`, `GetHabitDetail`, `ListAnchoredHabits` — flat `snake_case` modules under `queries/`, returning per-screen DTOs ([[adr-0006-cqrs-light]]). `ListBoardHabits` returns **`TodayHabits { active, paused, anchored_count }`** — one query, one DTO, two lists **plus a derived tally**: the partitioning *is* the business rule (« une habitude en pause quitte la liste du jour », and an anchored one leaves both lists) so it lives in the query, not the view; the « X sur Y » tally is `active.len()`, correct by construction, and `anchored_count` is computed in the same pass, never stored. `HabitDetail` carries `next_goal_up` / `next_goal_down`, **derived on read** — the floor is a business rule, so the view renders the number but never computes it ([[adr-0008-goal-based-dose-user-paced-progression]]) — and `state: HabitState`, a **DTO-side enum** mapped from the domain's `LifecycleState` by an exhaustive `match` (never the domain type, never a `bool` — [[adr-0007-habit-lifecycle-aggregate]] AD-2). `ListAnchoredHabits` is the **Ancrées screen's own** read model (`Vec<AnchoredHabit { title }>`, no `Clock`): a sibling *screen* gets a sibling query, where a *zone* of one screen would not | `core/src/habit_management/queries/list_board_habits.rs`, `core/src/habit_management/queries/get_habit_detail.rs`, `core/src/habit_management/queries/list_anchored_habits.rs` |
| Infrastructure | `InMemoryHabitRepository` — **the only adapter left** (`InMemoryOutbox` and `InMemoryHabitBoardRepository` deleted 2026-08-17), plus the `SystemClock` and `UuidGenerator` adapters in `shared/` | `core/src/habit_management/infrastructure/` |
| Presentation (shell + wired screens) | Dioxus `App` hosting `Router::<Route>`; flat `Route` enum mirroring the designer's 6 screens + NotFound catch-all ([[adr-0004-routing-flat-enum]]); composition root = `Services`, a pure DI registry provided once at the app root — where `habit_repository` is **one shared `Rc`** every use case and query receives ([[adr-0009-quality-gates]]; the board-specific guard test went with the board, see that node's 2026-08-17 amendment). `STARTING_GOAL` lives here too, so the views depend on the composition root and not the reverse. **Today, Add, Detail and Ancrées are wired** to their use cases/queries — Detail carries the « Ajuster, à votre rythme » zone with **two unconditional buttons** (grow / lighten, no precondition, never disabled) driving `GrowGoal` / `LightenGoal`, plus the pause/resume and anchor gestures and a **sober `Anchored` branch offering no gesture at all**; Today carries a **paused zone rendered only when non-empty** (« En pause · aucune pression » over an empty region would be a silent reproach) and a « Mes habitudes ancrées · N » link **rendered only when `N >= 1`**, for the same reason. Every gesture is a **`#[must_use]` mutate-then-reload free function** (`anchor_and_reload` joins `pause_and_reload` / `resume_and_reload` / `grow_and_reload` / `lighten_and_reload`), never logic inline in an `onclick` — see the Local decisions table. Ritual and Week remain stubs. **The `app/src/services/` layer is deleted** — the Add screen calls `AddHabit` directly through `Services`, so no app-side orchestration remains. Target: **Android-first**, all dev on the web platform for speed | `app/src/main.rs`, `app/src/route.rs`, `app/src/composition.rs`, `app/src/views/habit_detail.rs`, `app/src/views/today.rs`, `app/src/views/anchored.rs`, `app/src/views/add_habit.rs` |

## Trust boundary (settled 2026-07-26 in [[adr-0010-crate-boundary-trust-boundary]])

The system's **anticorruption layer is the `kayzen-core` crate boundary** — not the Dioxus view. Because [[adr-0006-cqrs-light]] forbids `kayzen-app` from importing a domain type and [[adr-0003-two-crate-workspace]] makes that edge compiler-enforced, every use case's and query's entry point (**primitives in, DTO or domain error out**) is *structurally* the only place a primitive can become a domain type.

```
URL /habit/:id                                    ← untrusted
  → app view forwards the raw String              ← never parses, cannot: it may not import HabitId
  ‖ ══════════════ crate boundary = trust boundary ══════════════
  → GetHabitDetail::handle(&str) / MarkDone::execute(&str) / AddHabit::execute(String, u32)
      → HabitId::new(&str) -> Result<HabitId, HabitError>   ← THE single door (1..=64, no trim)
  → domain, where nothing re-validates
```

| Rule | Where |
|---|---|
| One door per VO: no `From`, no `Deserialize`, no public field bypassing the validating constructor | `core/src/habit_management/domain/habit_id.rs` |
| Parse at the entry point, never deeper, never in `kayzen-app` | `core/src/habit_management/queries/get_habit_detail.rs`, `core/src/habit_management/use_cases/mark_done.rs`, `core/src/habit_management/use_cases/add_habit.rs` |
| Refusal rides each site's existing failure path — no public signature changed | idem |
| **Not covered**: the `Ritual` route never crosses into the core (its view re-injects the raw parameter into a `Link`) | `app/src/views/ritual.rs` |

Seven escalation triggers reopen this decision — persistence, multi-user, id length, **SSR in production deps**, id-as-sink-key, `Deserialize` on `HabitId`, user-supplied ids. They are listed verbatim in [[adr-0010-crate-boundary-trust-boundary]]; do not re-derive them.

## Habit creation flow (one use case, one write — since 2026-08-17)

```
AddHabit::execute(title: String, goal: u32) -> Result<(), AddHabitError>
  → HabitId::new(GuidGenerator::generate())          # the generated id is parsed too (adr-0010)
  → HabitTitle::new(title)                           # per-habit invariants, in the VOs (adr-0001)
  → Goal::new(goal)
  → HabitRepository::all() |> filter(state != Anchored)      # THE SET, read live
  → any(|h| h.title().matches(&title)) -> DuplicateHabit     # duplicate BEFORE capacity
  → len() >= Habit::MAX_IN_DAILY_LIFE -> DailyLifeFull{max}  # capacity
  → HabitRepository::save(&Habit::new(id, title, goal, clock.today()))   # THE ONLY WRITE
```

- Per-habit invariants (title length, goal floor) → VOs: settled in [[adr-0001-validation-by-construction]], **unchanged**. `Habit::new` takes three already-parsed types, so no unvalidated value can reach it.
- **Cross-habit rules (at most 5 in the daily life, no duplicate title) are set-based validation, not aggregate invariants** — they live in `AddHabit`, read through the existing `HabitRepository` port: [[adr-0013-set-based-validation-outside-aggregates]]. Do not re-decide, and do not reintroduce a "set aggregate" for the next such rule.
- Check precedence: **VOs → duplicate → capacity**. Duplicate wins on a full daily life — the one facet of [[adr-0002-habitboard-stateful-aggregate]] that survived it.
- **No event, no outbox, no handler.** `HabitRequested`, `HabitBoardEvent`, `DomainEventPublisher`, `InMemoryOutbox`, `RequestHabit` and `CreateHabitOnRequest` are deleted; the codebase publishes nothing at all.
- **The read-then-write window has zero width in this runtime, enforced by the type system** (`Rc<dyn HabitRepository>` is `!Send`/`!Sync`, the adapter uses `RefCell`, and no `async`/`.await`/thread/`Arc`/`Mutex` exists in `core/src` or `app/src`). Security's three reopening conditions — async, `Arc`/`Send`, or storage shared across processes/tabs — are recorded verbatim in [[adr-0013-set-based-validation-outside-aggregates]]. When one fires, the fix is to move the cap **into the write**, never to re-read.

## Habit lifecycle write side (**goal + pause + anchor facets built**; readmission still planned — settled in [[adr-0007-habit-lifecycle-aggregate]])

Beyond creation, `Habit` is **promoted to the lifecycle aggregate root** (one aggregate, keyed by `HabitId`) and grows behavior vertically across slices 2/3/5/6/7 of [[lifecycle-backlog]]. **Slices 2, 3, 5 and 6 have landed**: mark-done, the two goal gestures, pause/resume, and anchoring.

- **The dose is a single `Goal` VO** (default 5, floor 1, **no upper ceiling**; the ≤5-min creation guard dropped) — a **soft daily target, not a commitment**; completion stays **binary**. Progression is **user-paced**: `grow()` / `lighten()` (±1) are **always-available gestures the system NEVER suggests** — there is **no `StabilityPolicy`, no stability detection, no growth/anchor suggestion** ([[adr-0008-goal-based-dose-user-paced-progression]], superseding ADR-0005).
- **Two dated histories inside `Habit`** — `CompletionHistory` (ordered set of `LocalDate`, one completion/day structurally, kept forever; `toggle_done(today)` insert/remove) and `StepHistory` (the **self-paced staircase**, `{ first: StepChange, rest: Vec<StepChange> }` so non-emptiness is **structural** and `current()` is total — no `Option`, no `unwrap`, no panic path). `grow()` / `lighten()` **append** steps via `record`; the history never removes, pops or merges, and two changes on the same day stay two dated steps.
- **The floor at 1 min is a business rule, not a clamp** — below one minute there is no shorter habit, there is no habit, and the exit is deletion (named, not yet designed). It is enforced by `Goal` construction and by `Habit::lighten`, which **infers** the floor (`lightened() == *current()` → silent no-op) rather than restating `Goal::MIN` — the constant stays owned by `Goal`. Lightening at the floor is a **silent no-op**: no error, no UI signal; feedback is by state, not by reproach ([[adr-0007-habit-lifecycle-aggregate]] d2, settled 2026-07-27).
- **`LifecycleState` enum — complete since slice 6: `Active`, `Paused`, `Anchored`** (illegal combos unrepresentable; no `Default` derive — `Habit::new` names `Active` explicitly). One variant landed per slice, each with the use case that transitions into it, and the bet paid: slice 6's new variant broke every exhaustive `match` and handed the developer the exhaustive worklist. `Habit::pause()` / `resume()` / `anchor()` are the three transitions, driven by `PauseHabit` / `ResumeHabit` / `AnchorHabit`; **none of them refuses** — no precondition, no `Result`; the screens decide what to offer ([[adr-0007-habit-lifecycle-aggregate]] AD-4), which is why the domain would let a paused habit be anchored and no screen offers it. `toggle_done` never inspects the state (paused *and* anchored habits stay markable-done; `MarkDone` is byte-for-byte unchanged since slice 5).
- **A paused habit keeps its seat; an anchored one gives it back — and that is now a filter, not a registry.** `AddHabit` counts habits whose `state() != Anchored`, live, at the moment of the gesture (`core/src/habit_management/use_cases/add_habit.rs`). Pause keeps the seat because `Paused != Anchored`, pinned by the wired test in `core/src/habit_management/use_cases/pause_habit.rs` (`// @scenario: pause-resume/S3`); anchoring frees the seat *and* the title in the same act, pinned in `core/src/habit_management/use_cases/anchor_habit.rs` (`// @scenario: anchor-habit/S1`). Nothing is recorded, released or mirrored ([[adr-0013-set-based-validation-outside-aggregates]]).
- **No gesture touches two aggregates.** `AnchorHabit` holds `HabitRepository` alone and is the same 8-line load → mutate → save shape as `PauseHabit` / `ResumeHabit`. The save-order doctrine, the idempotence-instead-of-a-transaction argument and the partial-anchor failure mode of [[adr-0012-synchronous-cross-aggregate-coordination]] are **void with it** — there is one write, so there is no order and no partial state.
- **The paused-habit gating (« ni grandie, ni allégée, ni pratiquée ») is enforced by the view only** — `GrowGoal` / `LightenGoal` accept a paused habit's id, and `/habit/:id/ritual` is reachable by URL. A non-issue under the current single-user local threat model, and a **deliberate deferral**: it must land at the use-case entry point together with the ownership check the day the multi-user trigger fires ([[adr-0010-crate-boundary-trust-boundary]] trigger 2, amplified 2026-08-06).
- **Time enters through a `Clock` port** (`today() -> LocalDate`) in `shared/`; the domain owns a **library-free `LocalDate` VO** (epoch-day integer, **no `chrono` in its public API**), and `chrono` is confined to the infra `SystemClock` adapter. Aggregate methods take `today: LocalDate` as a **plain parameter** — the aggregate stays a pure function, no clock stub in domain tests.
- **Every mutation is an internal state transition** (load → method → save), **NOT** an event — there is no subscriber and [[adr-0006-cqrs-light]] has no projections to feed ([[adr-0007-habit-lifecycle-aggregate]] d3, now unconditional: with the outbox and `DomainEventPublisher` deleted, **nothing is published anywhere**).
- **A latent lifecycle defect, scheduled for PR 2.** `Habit::resume()` (`core/src/habit_management/domain/habit.rs`) has no precondition ([[adr-0007-habit-lifecycle-aggregate]] AD-4), so a caller resuming an `Anchored` habit produces **6 non-anchored habits against the 5-seat cap**. Unreachable today — `app/src/views/today.rs` renders « Reprendre » only inside `today_habits.paused`, and `app/src/views/habit_detail.rs`'s `Anchored` branch offers no action at all — and reachable exactly at **slice 7**. Security rated it LOW and pre-existing. Transition-table legality **is** a genuine `Habit` invariant (one instance, no other aggregate consulted), so it belongs in the aggregate — the counter-example [[adr-0013-set-based-validation-outside-aggregates]] closes on.
- **`HabitRepository`** gains `get(&HabitId) -> Option<Habit>` + upsert-by-id `save` (slice 2).

Still planned: **readmission** (slice 7). It is now a **lifecycle transition on one aggregate**, the mirror of anchoring — not a re-admission onto anything. Its rejection path is real and is **set-based validation**: a readmitted habit re-enters the daily life, so it must pass the same two guards `AddHabit` runs (the title may have been retaken while it was anchored — `readmit-habit` S3 — and the daily life may be full). PR 2's transition-table guard lands first, because slice 7 is the cycle that makes the resume-an-anchored-habit path reachable ([[adr-0013-set-based-validation-outside-aggregates]], and the 2026-08-17 amendment of [[adr-0012-synchronous-cross-aggregate-coordination]]).

## Local decisions (non-ADR — settled here)

| Decision | Rationale | Rejected |
|---|---|---|
| `AddHabit` is a concrete struct, no trait | YAGNI — one implementation, no consumer needing abstraction. (Inherited from `RequestHabit`, which it replaced) | Trait + impl pair |
| `Habit::MAX_IN_DAILY_LIFE = 5` is a **constant on the domain type**, while the rule reading it lives in `AddHabit` (2026-08-17) | The number is domain vocabulary — « cinq » is a word the product says — so it belongs with the type it bounds, exactly like `HabitTitle::MAX_LEN` and `Goal::MIN`. **Publishing a constant is not hosting an invariant**; the mistake corrected by [[adr-0013-set-based-validation-outside-aggregates]] was giving an aggregate a *behaviour* it could not honour, not giving a type a *name* for a number | The literal `5` in the use case (loses the vocabulary); a `Habit::can_be_added_to(...)` method (re-creates the mistake with a smaller surface) |
| `CreateHabitCommand` trait + `CreateHabit` impl **deleted** (LOCAL-1) | The *trait* was dropped for having no second consumer — that half stands. Its replacement, the board-driven flow, is itself now deleted (2026-08-17); creation is direct again, through the concrete `AddHabit` | Keeping the trait "for later" |
| ~~`HabitBoardError` resurrected (LOCAL-2)~~ → **`AddHabitError { InvalidHabit(HabitError), DuplicateHabit, DailyLifeFull { max } }`** (2026-08-17) | Same three rejection reasons, now owned by the use case that decides them rather than by a deleted aggregate. The LOCAL-2 point still holds in its new home: these are **not** `Habit` rules, so they do not join `HabitError` | Flattening the rejections into `HabitError` (wrong owner — no single habit can produce them) |
| Rename `HabitDescription` → `HabitTitle`, `HabitError::DescriptionLength` → `TitleLength` (LOCAL-2) | Ubiquitous language — the human calls it "un titre simple" | Keeping "description" as a technical synonym |
| `HabitTitle::new` trims before validating | Stored value is trimmed (case preserved); the length rule applies to the meaningful text | Validating the raw input |
| Duplicate matching via explicit `HabitTitle::matches` (trim + case-insensitive); `PartialEq` stays strict | Equality remains value equality; business-rule matching is an explicit, named domain operation | Overloading `PartialEq` with case-insensitive semantics (silently changes set/map behavior) |
| ~~`HabitBoardRepository` is mono-board~~ — **deleted (2026-08-17)** | The port existed only to persist the board. There is one repository port left, `HabitRepository` | — |
| ~~`BoardEntry.id` stored but not yet read / the hook is spent (slice 6)~~ — **deleted with `BoardEntry` (2026-08-17)** | The "hook" was the board's way of simulating a predicate (`state != Anchored`) it was forbidden to read directly. `AddHabit` now reads the predicate itself, and the whole entry — id and the **duplicated `HabitTitle`** — is gone. The duplication was the tell: a fact `Habit` already owned, written twice | — |
| ~~Outbox drained by a synchronous in-process dispatcher~~ — **deleted (2026-08-17)** | The dispatcher, the outbox, the event and the handler all existed to defer habit creation past the board's request step. With one write there is nothing to defer and nothing to drain. `app/src/services/` is gone; the Add screen calls `AddHabit` through `Services` directly | — |
| View helpers are `#[must_use]` (slice 3) | `grow_and_reload` / `lighten_and_reload` in `app/src/views/habit_detail.rs` return the refreshed detail; discarding it silently drops the screen refresh. `#[must_use]` makes that a compile error — the one class of view defect a lint can hold (it does **not** reach a helper *swap*, see [[adr-0009-quality-gates]] L1) | Trusting the render test to catch a dropped refresh (it cannot — the HTML is identical) |
| **Every gesture is an extracted free function, never logic inline in an `onclick`** (slice 5 — standing pattern) | Logic inside an `onclick` closure is **unreachable by every gate this repo owns**: no click dispatch exists in the suite, the mutation gate excludes `app/src/**`, and a render assertion cannot tell two identically-rendered buttons apart. Slice 5's review proved it by hand — a resume button rewired to *pause*, a whole button block deleted, and the paused-zone guard forced to `if true` each left the full suite green. Extracted into `fn(&Services, &str) -> T`, the same logic is ordinary Rust a test calls directly. Shape: `pause_and_reload` / `resume_and_reload` (`app/src/views/habit_detail.rs`), `resume_and_relist` / `mark_done_and_relist` (`app/src/views/today.rs`). Standing until a `VirtualDom` click-dispatch harness lands — [[adr-0009-quality-gates]] L1-bis | Programmatic navigation (`navigator().push`) after a gesture — no precedent in this repo, and it puts the gesture's only logic back inside the untested closure. Owner ruling: pausing keeps the user on the detail, which re-reads |
| `FixedClock` is **duplicated per test module in `app`** (accepted) | Core's `FixedClock` is `#[cfg(test)] pub(crate)` and genuinely unreachable cross-crate, so each app test module defines its own: `app/src/views/today.rs`, `app/src/views/habit_detail.rs` (the third copy went with `app/src/services/`, 2026-08-17). **Not a defect, not yet a decision** — sweeping it was ruled out of slice 3's surface. The real fork (a `test-support` feature on `kayzen-core` vs. a small app-side test helper module) is undecided | Exporting core's test double unconditionally (would put a test-only type in the production API) |

## Deliberately does NOT exist yet (human constraint — manual dev resumes after these cycles)

- Any **persistence**: every store is in-memory, so nothing survives a restart, and `Services::new` seeds three demo habits at startup
- UI wiring **beyond Today, Add, Detail and Ancrées** — those four screens call `ListBoardHabits`, `MarkDone`, `AddHabit`, `GetHabitDetail`, the lifecycle gestures and `ListAnchoredHabits` through the `Services` registry provided at the app root (`app/src/composition.rs`); **Ritual and Week stay stubs**. The route `:id` stays a `String` — deliberately, and now permanently: it is parsed once at the core's entry point, never in the view ([[adr-0010-crate-boundary-trust-boundary]])
- Android `mobile-shell` concerns — hardware back-button wiring and intent-filters/App Links registration are **explicitly deferred** to a future `mobile-shell` ticket ([[adr-0004-routing-flat-enum]])
- ~~Idempotence in `CreateHabitOnRequest`~~ — **moot**: the handler, the event and the outbox are deleted (2026-08-17)
- **Readmission** — putting an anchored habit back into the daily life (slice 7); the anchoring half exists, the return path does not
- **A lifecycle transition table on `Habit`** — no transition has a precondition today ([[adr-0007-habit-lifecycle-aggregate]] AD-4), which leaves the latent `resume()`-an-anchored-habit defect above. **PR 2**, security-motivated, not merely a purity correction
- Unicode handling in `HabitTitle`: NFC normalization in `matches` + grapheme-based length — both deferred, flagged by Security review as low/UX; **handle together in one future ticket**

Do not build these speculatively; each requires a new decision cycle.

## Consequences / Constraints

- **MUST**: route habit creation through `AddHabit` = parse VOs → read the set → duplicate guard → capacity guard → **one** `HabitRepository::save`. (This **replaces** the LOCAL-2 MUST *"route creation through `RequestHabit` = load → `HabitBoard::request_habit` → save → publish"*, which named five deleted things.)
- **MUST**: treat `HabitRepository` as the **single source of truth** for the habit count and the duplicate check, read live and filtered on `state != Anchored`. Never mirror the predicate into a stored flag, counter or registry — that mirroring was the deleted design ([[adr-0013-set-based-validation-outside-aggregates]]).
- **MUST**: keep rejection non-destructive — on `Err`, nothing is written. With a single write at the end of `execute`, this is now structural rather than a discipline.
- **MUST**: apply [[adr-0013-set-based-validation-outside-aggregates]]'s three checks before hosting any new cross-entity rule, and put set-based validation in the use case performing the gesture. **MUST NOT** reintroduce `HabitBoard` or any equivalent "set aggregate".
- **MUST**: respect the dependency rule — domain has no dependency on application or infrastructure; ports live in the domain.
- **MUST**: respect the crate boundary — `kayzen-core` stays free of UI/platform dependencies; the `app → core` edge is one-way (see [[adr-0003-two-crate-workspace]], incl. its known-debt follow-ups: Cargo.lock platform closure, `cargo audit` in CI, public in-memory test doubles).
- **MUST**: in the app shell, navigate via explicit `Link { to: Route::X }` only (never `go_back()`), keep URL paths English and stable, and convert the `:id` `String` to a typed id **once, inside the core, at the use-case/query entry point** — [[adr-0004-routing-flat-enum]] said "once at the core-wiring boundary"; [[adr-0010-crate-boundary-trust-boundary]] settles *where* that once is, and it is **not** the view. (adr-0004's watch items still stand: unused `segments` prop in `not_found.rs`, stale-`done` closure bug in `today.rs`.)
- **MUST**: obtain every domain type through its single validating constructor; no `From`/`Deserialize`/public-field bypass ([[adr-0001-validation-by-construction]], [[adr-0010-crate-boundary-trust-boundary]]).
- **MUST NOT**: introduce trait abstractions without a second consumer. *(The LOCAL-1 half of this MUST NOT — "no direct-creation command use case" — is **withdrawn**: a direct-creation use case is exactly what `AddHabit` is, and the board-driven indirection it forbade is the mistake [[adr-0013-set-based-validation-outside-aggregates]] corrects.)*
- **MUST NOT**: make `PartialEq` on `HabitTitle` case-insensitive — `matches` is the business comparison.
- **Out of scope**: persistence beyond in-memory adapters; any UI.

## Open questions / Gaps

- [x] ~~Production event dispatching strategy (poll vs push, error handling, retries).~~ **Moot 2026-08-17** — there are no events. Should one ever be needed, it starts from an empty page, not from the deleted outbox.
- [x] ~~Idempotence / dedup strategy for `CreateHabitOnRequest` once a real dispatcher exists.~~ **Moot 2026-08-17** — the handler is deleted.
- [x] ~~"Ancrée" (anchored) rule: when/how an entry leaves the board (`BoardEntry.id` is the hook).~~ **Settled 2026-08-11 (slice 6), then RE-SETTLED 2026-08-17.** The slice-6 answer (`HabitBoard::release`, called synchronously by `AnchorHabit`) is void with the board. There is no entry and nothing leaves: anchoring sets `state = Anchored` on one aggregate, and `AddHabit`'s filter stops counting it — [[adr-0013-set-based-validation-outside-aggregates]].
- [ ] Unicode ticket: NFC normalization in `HabitTitle::matches` + grapheme-based length validation (deferred together).
- [ ] **Habit deletion** — the affordance the floor-of-1 business rule implies (*« sinon on doit la supprimer »*). Named in [[adr-0008-goal-based-dose-user-paced-progression]]'s 2026-07-27 amendment, deliberately not designed. The next cycle meeting "the user wants less than one minute" starts here, not from a sub-minute goal.
- [ ] **Bounding or compacting `StepHistory` — a constraint on the persistence slice, not a free choice.** Alternating grow / lighten above the floor returns the goal to its starting value while appending **2 steps per round trip**, indefinitely, because [[adr-0007-habit-lifecycle-aggregate]] d6 forbids same-day fusion. Harmless in memory; unbounded storage once history survives a reload. Compaction is a *storage* decision — it must not be smuggled in as domain-level fusion, which would silently reverse d6.
- [x] ~~**app/src/services/add_habit.rs imports `HabitBoardError`**, a domain error type, in production code — a tension with [[adr-0006-cqrs-light]]'s "never imports a domain type" MUST.~~ **CLOSED 2026-08-17 by deletion.** The whole `app/src/services/` layer went with the outbox it drained. `app/` production code now imports **no domain error at all**: the Add screen calls `services.add_habit.execute(...)` and consumes the result with `.is_ok()` (`app/src/views/add_habit.rs`), every other gesture with `.ok()`. The MUST holds **unqualified on the write side** — the only domain items `app/src/composition.rs` names are the `HabitRepository` *port* and its in-memory adapter, which is what a composition root is for.
- [ ] **`FixedClock` triplication in the `app` test modules** (3 copies) — see the Local decisions table; the `test-support` feature vs. app-side helper fork is open.
- [x] ~~**Slice 5 will silently delete slice 3's view-mutant coverage.**~~ **Resolved and corrected 2026-08-06 — the premise was wrong.** The diff-scoped gate never generated a view mutant at all (`.cargo/mutants.toml` excludes `app/src/**` by design); the four mutants came from an unscoped campaign. The exposure was real and was *demonstrated* — three hand-run mutations left the full suite green — and is now mitigated by the extracted-free-function pattern (Local decisions table). [[adr-0009-quality-gates]] L1-bis.
- [ ] **No `VirtualDom` click dispatch exists in the test suite.** The extracted-free-function pattern shrinks the `onclick` to a call plus a signal assignment, but the residue is still unmeasured by construction. A click-dispatch harness remains the only thing that would close it — a cycle of its own, still with no precedent in this repo.
- [ ] **`match`-based rules are invisible to the mutation gate** ([[adr-0009-quality-gates]] L4). The Today partition and the `LifecycleState → HabitState` mapping each received zero viable mutants. The compiler guarantees exhaustiveness; only a deliberate test guarantees each arm is right. Applies to every state-driven branch slices 6–8 will add.
- [ ] **The paused-habit gating lives in the view only** — `GrowGoal` / `LightenGoal` / the Ritual route accept a paused habit. Slice 6 extends the same shape to anchored habits: the domain refuses nothing ([[adr-0007-habit-lifecycle-aggregate]] AD-4), so an anchored habit's id is still accepted by every gesture use case and only the screens withhold them. Deliberate under the current threat model; it must land at the use-case entry point *together with* the ownership check when the multi-user trigger fires ([[adr-0010-crate-boundary-trust-boundary]] trigger 2).
- [x] ~~**A partial anchor is not recoverable by the user, only by replay** — if the process dies between `AnchorHabit`'s two saves…~~ **VOID 2026-08-17**: `AnchorHabit::execute` performs one write, so there is no interval and no partial state. The persistence question it deferred is inherited, sharpened, by [[adr-0013-set-based-validation-outside-aggregates]]'s escalation trigger.
- [ ] **The cap must move into the write when one of Security's three conditions fires** — async on `HabitRepository`, `Arc`/`Send` on the port, or a store shared across processes/tabs. Conditional write, a uniqueness constraint on a normalised title, or a serialisable transaction; **not** a re-read. Verbatim trigger in [[adr-0013-set-based-validation-outside-aggregates]].
