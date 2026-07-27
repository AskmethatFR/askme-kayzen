# Habit Lifecycle — approved model & slice backlog

> **Status**: approved by the human on 2026-07-16 (GATE 1.5 go on the Architect's
> `implementation-proposal`). This node records the *functional* decisions and the
> delivery backlog. The technical model (aggregate shape, Clock port, StabilityPolicy)
> belongs to the Architect's lane — see `[[architecture-overview]]` and the upcoming
> lifecycle ADR.

> ⚠️ **AMENDED 2026-07-23 by `[[adr-0008-goal-based-dose-user-paced-progression]]`** (owner
> product decision, supersedes `[[habit-progression-study]]`). The dose is now a soft
> **goal** (default 5 min, floor 1, no ceiling); progression is **user-paced** (grow/lighten
> ±1 whenever the user chooses) with **no system detection or suggestion**. Consequences below:
> the growth-suggestion model + Q2/Q5 thresholds are **void**, and slice 4 is **deleted**.

## Approved lifecycle model (functional view)

- A habit's **goal** (minutes) is a **soft daily target** (default 5, floor 1, no ceiling).
  It changes **only through a user gesture** — *grandir* (+1) or *alléger* (−1, floor 1),
  **always available, whenever the user decides**. No automatic mutation, and **no system
  detection or suggestion** (per `[[design-principes-kaizen]]` and `[[design-gestes-kaizen]]`
  gesture 2, as amended by ADR-0008).
- **~~Growth suggestion~~** — **removed.** There is no stability detection and no "Passer à
  N+1" system prompt. Anchoring is likewise **user-initiated** (no 10-of-14 suggestion).
- **Completion** = one local date per day, toggleable the same day. No timestamps,
  no multi-completion. Full completion history is kept forever (feeds "minutes
  gagnées depuis les débuts").
- Everything the screens show (`done_today`, 21-day dots, 7-day rhythm, minutes
  gained) is **derived from the histories**, never stored.

## Approved answers to the open questions (Q1–Q5)

| # | Question | Decision |
|---|---|---|
| Q1 | Does pausing free one of the 5 slots? | **No** — a paused habit keeps its seat, so resume can never fail (anti-guilt). Amends the designer's literal `active = !paused && !anchored` cap formula: the cap counts non-anchored habits. |
| ~~Q2~~ | ~~Growth-suggestion thresholds~~ | **VOID** — there is no growth suggestion to threshold (ADR-0008). The 10-of-14 rule survives nowhere, not even as a tunable. |
| Q3 | Can an anchored habit still be marked done? | **Yes** — Ancrées dots stay alive; no domain guard on paused either (UI simply doesn't offer the target). |
| Q4 | Completion granularity | One per day, local date only, toggleable. |
| ~~Q5~~ | ~~Remember a dismissed growth suggestion?~~ | **VOID** — nothing is ever suggested, so there is nothing to dismiss (ADR-0008). |

## Slice backlog (approved order — each slice user-observable)

| # | Ticket | Slice (user sees) | Size | Status |
|---|---|---|---|---|
| R1 | `goal-default-5` | New habits start on a 5-min **goal** (`Goal` VO, floor 1, no ceiling; ≤5 guard dropped; Add copy "5 min") | S–M | **done** |
| 1 | `read-habits-query` | Today screen lists *real* board habits via the final DTO shape (honest defaults) | M | done |
| 2 | `mark-done` | Tapping the target fills the ink; toggle; calendar dots appear | M | done (core + Today toggle; calendar dots deferred to stats-board) |
| 3 | `adjust-goal` (was `grow-lighten`) | Detail's "Ajuster" zone: **user-paced** N+1 / N−1 on the goal, staircase renders, floor 1. **No suggestion driving it** | M | **done** (both gestures, floor silent no-op, staircase fixed so only the last step is current) |
| ~~4~~ | ~~`growth-suggestion`~~ | **DELETED** — StabilityPolicy removed (ADR-0008); progression is user-paced, nothing is suggested | — | ✂️ removed |
| 5 | `pause-resume` | "Mettre en pause" / paused zone / one-tap resume | S | todo |
| 6 | `anchor` | Anchor button (**user-initiated, no 10-of-14 suggestion**), board frees the slot, Ancrées screen counts | L | todo |
| 7 | `readmit` | "La remettre dans mon quotidien" — refusable (board full / duplicate title) | M | todo |
| 8 | `stats-board` | Per-habit stats: days done, empty days (never "failed"), grow/lighten counts, minutes gained (reframe wording — nominal, anti-guilt) — plus adaptive (never guilt-inducing) messages | M | todo |

Order rationale: completions (2) precede progression (4); grow/lighten (3) before
suggestion (4) so the suggestion highlights an existing affordance; anchor last
of the core loop because it is the only cross-aggregate slice; stats-board (8)
comes after the histories it derives from exist (2, 3) — everything it shows is
derived from the completion and step histories (CQRS-light queries, no stored
stats), per the designer's own "tout le récap est dérivé" principle and
`[[adr-0006-cqrs-light]]`. Adaptive messages are a read-side policy (same
pattern as the stability policy), stateless, anti-guilt wording only.

**8-slice order holds — no reorder, no inserted foundation slice.** The
lifecycle-aggregate ADR (`[[adr-0007-habit-lifecycle-aggregate]]`) pins the shape;
each slice grows the aggregate *vertically* by only the piece it needs.

## Gherkin per slice (spec-only, see `docs/functional/features/habit-management/`)

Each remaining slice is specified as scenarios before it is built; every scenario
is `@wip` until a test carries its `// @scenario: <feature-id>/<Sn>` anchor. The
Developer's first failing test of a slice is the scenario, and dropping the
`@wip` tag is part of that slice's Definition of Done.

| Slice | Feature file | Scenarios |
|---|---|---|
| 1 `read-habits-query` | `[[today-habit-list]]` | S1–S3, covered |
| 2 `mark-done` | `[[mark-done]]` | S1–S3, covered |
| 3 `adjust-goal` | `[[adjust-goal]]` | S1–S4, covered (S3 pins the floor no-op — d2 now settled) |
| 5 `pause-resume` | `[[pause-resume]]` | S1–S3, `@wip` (S3 pins Q1 — a paused habit keeps its seat) |
| 6 `anchor` | `[[anchor-habit]]` | S1–S4, `@wip` (S3 pins Q3, S4 pins "no suggestion") |
| 7 `readmit` | `[[readmit-habit]]` | S1–S3, `@wip` |
| 8 `stats-board` | `[[habit-stats]]` | S1–S4, `@wip` |

## Per-slice aggregate growth (so nothing is forgotten — technical shape in `[[adr-0007-habit-lifecycle-aggregate]]`)

> `Habit` is promoted to the lifecycle aggregate root; it stops being anemic at slice 2.
> Read side (slice 1) is unaffected — `HabitSummary` DTO is stable (read `minutes` via a
> `current_dose()` accessor now, keep `done_today = false` honest default).

| Slice | Aggregate / port growth |
|---|---|
| 1 `read-habits-query` | none on the aggregate. Read `minutes` via `current_dose()` (returns `initial_duration` until step history exists); `done_today = false`. Do NOT bake a Today-query signature that cannot later receive an injected `Clock`. |
| 2 `mark-done` | adds `CompletionHistory` VO + `toggle_done(today)`; adds the `Clock` port (`shared/`, returns domain `LocalDate`, chrono confined to infra `SystemClock` adapter); `HabitRepository` gains `get(&HabitId)` + upsert-by-id `save`; Today query gains injected `Clock` (`done_today` source flips to `completion_history.contains(today)`). |
| 3 `adjust-goal` | **done.** `StepHistory` grew to a dated, **append-only** staircase (`{first, rest}` — non-emptiness structural, no `unwrap`); `Goal::grown()` / `lightened()` carry the ±1 and the floor; `Habit::grow()` / `lighten()` append a dated step. **d2 SETTLED — silent no-op at the floor**: nothing appended, `Ok(())` returned, nothing signalled to the screen (an error would contradict *« alléger n'est pas reculer »*; a UI signal would contradict S4). Two use cases, `GrowGoal` and `LightenGoal`, one public method each — see `[[adr-0011-one-public-method-per-use-case]]`. |
| ~~4 `growth-suggestion`~~ | **removed** — no `StabilityPolicy`, no stability detection, no suggestion (ADR-0008). |
| 5 `pause-resume` | adds `LifecycleState::{Active, Paused}` transitions (enum on `Habit`, illegal combos unrepresentable). Paused keeps the board seat (Q1). |
| 6 `anchor` | adds `LifecycleState::Anchored`; resolves the deferred board↔habit anchoring coordination (how the board frees the slot); `HabitBoard` cap counts non-anchored. |
| 7 `readmit` | `Anchored → Active` + board re-admission (reuse the duplicate / board-full guards). |
| 8 `stats-board` | no aggregate growth — more CQRS-light queries over the two dated histories. |

Lifecycle mutations are **internal state transitions** (load aggregate → method → save),
**not** published events — only `HabitRequested` crosses the outbox
(`[[adr-0007-habit-lifecycle-aggregate]]`).

## Glossary impact

Landed with slice 3 (now in `[[glossary]]`): Step (*marche*), Step history
(*croissance en escalier* — dated, append-only), Grandir (*grow*), Alléger
(*lighten*), Goal floor (the business rule — below one minute there is no habit,
only deletion, which is **not built yet**).

Already landed in earlier slices: Completion (*fait*), Completion history,
Mark done, Local date, Clock, Goal.

~~Dose~~ / ~~Durée initiale~~ (`InitialDuration`) and ~~Growth suggestion~~ are
**void** — collapsed into the single `Goal` VO and removed outright by
`[[adr-0008-goal-based-dose-user-paced-progression]]`. They are named here only so
nobody re-adds them from an older reading.

Still to add as their slices land: Pause/Resume (slice 5), Anchor/Readmit
(*ancrer / remettre dans le quotidien* — slices 6-7), État du cycle
(*Active / En pause / Ancrée* — `LifecycleState`, slice 5), Minutes gagnées
(slice 8).
