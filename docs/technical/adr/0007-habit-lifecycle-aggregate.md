---
id: "adr-0007-habit-lifecycle-aggregate"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-23"
relations:
  related:
    - "architecture-overview"
    - "lifecycle-backlog"
    - "adr-0008-goal-based-dose-user-paced-progression"
  depends-on:
    - "adr-0002-habitboard-stateful-aggregate"
    - "adr-0008-goal-based-dose-user-paced-progression"
    - "adr-0006-cqrs-light"
answers:
  - "Is there a second aggregate for the habit lifecycle, or is Habit promoted to the aggregate root?"
  - "How are completions stored, and what is the one-completion-per-day guarantee?"
  - "How is the running dose modeled, and where does the floor-of-1 invariant live?"
  - "How does the domain get 'today' without depending on chrono/NaiveDate?"
  - "Are lifecycle changes (mark done, grow/lighten, pause/anchor) published as domain events?"
  - "How is pause vs anchor modeled, and can a paused/anchored habit still be marked done?"
decided_in:
  - "LOCAL-lifecycle-aggregate"
---

# ADR 0007 — Habit promoted to the lifecycle aggregate root (dated histories, library-free LocalDate, internal transitions)

> **⚠️ Dose facets AMENDED (2026-07-23) by [[adr-0008-goal-based-dose-user-paced-progression]]**: the two-VO dose model below (`InitialDuration` with its ≤5-min creation guard + a running `Dose`) is **collapsed into a single `Goal` VO** (default 5, floor 1, **no upper ceiling**, guard dropped), the `StabilityPolicy` dependency is **withdrawn** (progression is user-paced, never suggested), and `StepHistory` now records dated **Goal** changes. **Every other facet of this ADR stands** (aggregate root, `CompletionHistory`, `LocalDate`/`Clock`, `LifecycleState`, internal transitions, repo `get`+upsert). This document is amended in place below; ADR-0008 is the source of truth for the dose/progression facets.

> **One-liner**: `Habit` becomes the **behavioral lifecycle aggregate root** (one aggregate, keyed by `HabitId`) carrying two **dated histories** — `CompletionHistory` (one completion/day, kept forever) and `StepHistory` (dated `Goal` changes, current goal = last step — the dose is a single `Goal` VO per the [[adr-0008-goal-based-dose-user-paced-progression]] amendment) — plus a `LifecycleState {Active, Paused, Anchored}` enum. The domain owns a **library-free `LocalDate` VO** (zero `chrono` in its public API); `chrono` is confined to an infra `Clock` adapter. Lifecycle mutations are **internal state transitions** (load → method → save), **not** published events — only `HabitRequested` stays on the outbox.
> **Links**: [[architecture-overview]] (the write-side shape this grows), [[adr-0002-habitboard-stateful-aggregate]] (the load→mutate→save aggregate discipline this extends to `Habit`), [[adr-0008-goal-based-dose-user-paced-progression]] (the single `Goal` VO + user-paced `grow()`/`lighten()` + floor-1 invariant this realizes — supersedes the earlier ADR-0005 suggestion model), [[adr-0006-cqrs-light]] (histories ARE the stored source of truth screens derive from — the reason no lifecycle events are published), [[lifecycle-backlog]] (the functional slices that grow the aggregate).
>
> **Timing note — decision capture ahead of implementation**: this cycle writes **ADR + docs ONLY, zero production code** (approved decision d4). The aggregate grows **vertically inside slices 2/3/5/6/7** of [[lifecycle-backlog]]; the anchors below are planned shapes, not existing files. Human-approved 2026-07-20 (lifecycle-aggregate design cycle), with one adjustment (d1, the `LocalDate` abstraction below).

## Context

The lifecycle backlog ([[lifecycle-backlog]]) requires `Habit` to gain behavior: mark done / toggle off, grow / lighten the dose, pause / resume, anchor / readmit, and a derived stats view. Progression is settled by [[adr-0008-goal-based-dose-user-paced-progression]] (superseding ADR-0005): the dose is a single `Goal` VO that changes only via user-paced `grow()`/`lighten()` with a floor at 1 min and **no suggestion**. ADR-0006 already fixed that the read side derives everything on read from the stored histories (no projections). The open technical fork this ADR settles: **is the lifecycle a second aggregate or a promotion of `Habit`, how are the two histories and the running dose modeled, how does the pure domain obtain a calendar date, and are lifecycle changes published?**

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Aggregate shape | **Promote `Habit` to the lifecycle aggregate root** — one aggregate, keyed by `HabitId`. No second aggregate: completions and step changes share `Habit`'s identity, transaction, and store, and hold **no cross-habit invariant** (the only cross-habit invariant — board capacity — stays on `HabitBoard`). Vernon: one true consistency boundary, one aggregate | planned: `core/src/habit_management/domain/habit.rs` |
| Completions | `CompletionHistory` VO over an **ordered set of `LocalDate`** (`BTreeSet<LocalDate>`): one-completion-per-day is **structural** (set membership, no guard), ordered for the dot calendar, kept forever. `toggle_done(today)` inserts/removes `today`; the only writable date is today (passed in), so no future-date invariant is needed (YAGNI) | planned: `domain/completion_history.rs` |
| Step history & goal (amended by [[adr-0008-goal-based-dose-user-paced-progression]]) | Dated `StepHistory` VO = ordered `Vec<StepChange { on: LocalDate, goal: Goal }>`, **seeded** at creation with `(creation_date, initial_goal)`. `current_goal() = steps.last().goal` — **never stored separately**. `grow()` pushes a step `+1`; `lighten()` pushes `max(1, current-1)`. The dose is a **single `Goal` VO** (default 5, floor 1, **no upper ceiling**) — the earlier `InitialDuration`/`Dose` two-VO split and its ≤5-min creation guard are **withdrawn** (ADR-0008) | planned: `domain/step_history.rs`, `domain/goal.rs` |
| Floor-of-1 invariant | The floor at 1 min is a **true aggregate invariant**, enforced **twice**: by `Goal` construction (cannot build a `Goal < 1`) AND by `lighten()` computing `max(1, current-1)`. Realizes the floor now settled in [[adr-0008-goal-based-dose-user-paced-progression]] | — |
| `LocalDate` (d1) | The domain **owns a pure, library-free `LocalDate` VO** in `kayzen-core` — **zero `chrono` dependency, no `chrono` type in its public API**. It carries the arithmetic the domain needs: comparison/ordering, day-difference, and "N days ago" windows (for the future `StabilityPolicy` 10-of-14). **Internal representation: an epoch-day integer** (signed days since a fixed epoch) — window arithmetic ("last 14 days") is pure integer subtraction, no hand-rolled calendar/leap-year math in the domain, and trivially testable | planned: `core/src/shared/local_date.rs` |
| `Clock` port | A `Clock` port `today() -> LocalDate` lives in `core/src/shared/` next to `GuidGenerator`. `chrono` (or any calendar lib) is confined to the **infra `SystemClock` adapter**, which produces "today" and converts to/from `LocalDate` at the boundary. **The port returns the domain `LocalDate`, never a `chrono` type** — exactly the dependency-rule discipline of [[adr-0003-two-crate-workspace]] (domain pure, infra at the edge) | planned: `core/src/shared/clock.rs`, infra `SystemClock` |
| Clock passed as parameter | Aggregate methods take `today: LocalDate` as a **plain parameter**; the **use case** holds the `Clock` and passes `clock.today()`. The aggregate stays a **pure function** — no clock stub in domain unit tests | — |
| Lifecycle state | `LifecycleState { Active, Paused, Anchored }` **enum** on `Habit` (not two bools — illegal combinations unrepresentable). Transitions hub through `Active` (resume/readmit → `Active`). **`toggle_done` never inspects `LifecycleState`** — a paused or anchored habit stays markable-done (no guard). Pause keeps the board seat; anchor frees it (the cross-aggregate board coordination is **deferred to slice 6**) | planned: `domain/lifecycle_state.rs` |
| Events (d3) | Lifecycle mutations are **internal state transitions** (load aggregate → method → save) — **NOT published**. Event Storming names the moments (mark done / toggle off, grow / lighten, pause / resume, anchor / readmit) but publishes **none**: there is no subscriber and [[adr-0006-cqrs-light]] has no projections to feed. Only **`HabitRequested`** stays published (it crosses the aggregate boundary via the outbox). **No `HabitEvent` enum; `HabitBoardEvent` + outbox untouched** | — |
| Repository (d5) | `HabitRepository` gains `get(&HabitId) -> Option<Habit>` and **upsert-by-id `save`** semantics (save an existing id overwrites). Introduced in **slice 2** | planned: `domain/habit_repository.rs` |
| Read-side compatibility | Slice-1 reads stay stable: `id`/`title` unchanged; minutes read via a `current_goal()` accessor **now** (returns the initial Goal until step history lands → zero rework in slice 3). `done_today` source changes from the `false` default to `completion_history.contains(clock.today())` in slice 2 — the Today query must accept an **injected `Clock`** in slice 2 | — |

**Open implementation point (d2 — deferred to slice 3, NOT final)**: `lighten()` at the floor (current dose already = 1). The **provisional default** is a **silent no-op** (push nothing / stay at 1, no error). This is revisable when slice 3 is implemented — the human has no opinion yet, so it is not locked. Alternatives to weigh at slice 3: reject with an error vs. return an "already-at-floor" signal for the UI.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Separate `HabitLifecycle` aggregate | Two aggregates over one identity → forced eventual consistency for data that shares one transaction and store; no cross-habit invariant justifies the split (Vernon: aggregate = one true consistency boundary) |
| `chrono::NaiveDate` wrapped in a newtype **inside the domain** (the Architect's original recommendation) | **Explicitly rejected by the human** — keeps `chrono` out of the domain entirely for a library-free, maximally testable core. The domain owns `LocalDate`; `chrono` lives only in the infra `Clock` adapter (d1) |
| Published `HabitEvent` enum / outbox for lifecycle changes | No consumer (YAGNI); [[adr-0006-cqrs-light]] has no projections to feed; lifecycle changes never cross the aggregate boundary. Only `HabitRequested` (which does cross it) stays published |
| Full event sourcing for the histories | Volume physically cannot justify it (~20k dates after 10 years, microsecond recompute) — same ground as [[adr-0006-cqrs-light]] |
| Designer's calendar as `Vec<bool>` (~21 days) | Loses the dates (needed for "minutes gagnées" = Σ dose active on each completed day) and contradicts the kept-forever history |
| Designer's undated steps as `Vec<u32>` | "Minutes gagnées" needs the dose **dated** to the day it was active — undated steps cannot reconstruct it |
| Two bools for pause/anchor | Allows the contradictory `paused && anchored` state; the `LifecycleState` enum makes illegal states unrepresentable |
| Injecting the `Clock` into the aggregate | Would force a clock stub in every domain unit test; passing `today: LocalDate` as a parameter keeps the aggregate a pure function |
| ~~Two distinct dose VOs (`InitialDuration` + running `Dose`)~~ — **withdrawn by [[adr-0008-goal-based-dose-user-paced-progression]]** | This ADR originally split the creation duration (≤5-min guard) from the running dose (floor-1 / no-ceiling). ADR-0008 removes that distinction: there is one dose concept, a single `Goal` VO (default 5, floor 1, no ceiling, guard dropped). The two-VO justification no longer applies |

## Consequences / Constraints

- **MUST**: keep `Habit` the single lifecycle aggregate root — completions, step history, and lifecycle state live inside it, keyed by `HabitId`; load → method → save (per [[adr-0002-habitboard-stateful-aggregate]]).
- **MUST**: store completions in `CompletionHistory` as an ordered set of `LocalDate` — one-completion-per-day is structural, never a guard.
- **MUST**: derive the current dose from `StepHistory.last()` via `current_goal()` — never store the current Goal separately.
- **MUST**: enforce the floor at 1 both in `Goal` construction and in `lighten()` (`max(1, current-1)`).
- **MUST**: keep `LocalDate` library-free — **no `chrono` type in `kayzen-core`'s public API**; `chrono` is confined to the infra `SystemClock` adapter, and the `Clock` port returns `LocalDate`.
- **MUST**: pass `today: LocalDate` to aggregate methods as a parameter; the use case holds the `Clock`. The aggregate stays a pure function (no clock stub in domain unit tests).
- **MUST**: model pause/anchor with the `LifecycleState` enum (never two bools); `toggle_done` must NOT inspect `LifecycleState` (paused/anchored habits stay markable-done).
- **MUST NOT**: publish any lifecycle event, introduce a `HabitEvent` enum, or touch `HabitBoardEvent` / the outbox — only `HabitRequested` is published (d3).
- **MUST NOT**: create a second aggregate for the lifecycle, or reintroduce a separate `InitialDuration`/`Dose` split — the dose is the single `Goal` VO ([[adr-0008-goal-based-dose-user-paced-progression]]).
- **MAY**: introduce `HabitRepository::get(&HabitId)` + upsert-by-id `save` in slice 2 (d5); add the epoch-day arithmetic `LocalDate` needs incrementally as slices require it.
- **Out of scope (this cycle — d4)**: any production code; the aggregate grows vertically inside slices 2/3/5/6/7 of [[lifecycle-backlog]]. Also out of scope: the `lighten()`-at-floor resolution (d2, slice 3); the board↔habit anchoring coordination (slice 6). The `StabilityPolicy` read-side computation is **removed entirely** ([[adr-0008-goal-based-dose-user-paced-progression]]) — no detection, no suggestion.
