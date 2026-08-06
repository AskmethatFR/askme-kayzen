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
| 3b | `practice-staircase` | The detail's staircase is **redrawn on practice, not on intent**: one bar per calendar day, full when the day was done, faint when it was not. Owner correction 2026-07-27 — see below | M | **done** (7-day window, faint missed days, per-day heights, `steps` off the contract) |
| ~~4~~ | ~~`growth-suggestion`~~ | **DELETED** — StabilityPolicy removed (ADR-0008); progression is user-paced, nothing is suggested | — | ✂️ removed |
| 5 | `pause-resume` | "Mettre en pause" / paused zone / one-tap resume | S | **done** (4 scenarios; paused detail is a rest screen — resume + staircase only; seat kept) |
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

## Slice 3b `practice-staircase` — owner correction, 2026-07-27

> Raised by the owner on reviewing the delivered slice 3: *« le graph grandit à
> l'ajout d'une minute alors qu'elle devrait ajouter dans le graph quand un jour
> est complété »*. This is a **functional** correction, not a bug in slice 3's code.

**What the staircase is for.** Two questions, one drawing:

1. **Am I keeping it up?** — read in the run of bars.
2. **Across those days, am I raising, easing, or holding my effort?** — read in
   their profile.

**How it draws.** One bar per **calendar day**, not per goal change.

| The day was | The bar |
|---|---|
| done | full, height = the goal that was active that day |
| not done | **the same bar at low opacity** — present, quiet, never a hole and never red |

The owner chose the faint bar over both alternatives offered: hiding the missed
day (loses the regularity signal) and breaking the drawing into a streak (would
contradict a non-negotiable). *« au lieu de ne pas le voir, on fait une opacité,
moins culpabilisant. »* Nothing counts, nothing accuses — `[[design-principes-kaizen]]`
holds unamended, and there is still **no streak**.

**Why it was wrong before.** Slice 3 draws one bar per `StepChange`, per
`[[design-ecrans]]` (« une barre par étape de `steps` »). So tapping *grandir*
five times without practising once drew five bars — the app credited the
intention. Meanwhile « minutes gagnées » was already specified **per completed
day**. Two readings of the same screen counted different things; the owner's
correction settles both on practice.

**Shape (functional, the Architect owns the technical form).** Nothing new is
stored. The drawing is a **projection of the two histories already kept**:
completion history × step history, the goal active on day D being the last dated
step ≤ D. `[[adr-0006-cqrs-light]]` already requires everything derived on read.

**Welcome side effect.** The oscillation concern recorded as FUT-1 in
`[[adr-0007-habit-lifecycle-aggregate]]` stops reaching the screen: tapping
grandir/alléger twenty times without practising adds no bar at all.

**Settled by the owner, 2026-07-27:**

| Point | Decision |
|---|---|
| Window | **The last 7 days.** Aligned with the week recap's own rhythm. Accepted trade-off: seven bars show the effort trend more faintly than a longer window would — legibility won. |
| The detail's dot calendar | **Removed.** The staircase already carries done / not-done per day, plus the effort height the dots never had. Two drawings of one fact contradict the screen's sobriety. Amends `[[design-ecrans]]`. |
| The decisions staircase (one bar per goal change) | **Off the screen.** The step history stays as *data* — it gives each bar its height and feeds « minutes gagnées » — but stops being a drawing. One staircase on the detail: practice. |

**Settled at spec time:** 3b **stands alone**. It ships the detail's one drawing;
slice 8 `stats-board` keeps the per-habit numbers it always owned.

**Settled during delivery, 2026-08-06** — the backlog was silent on the days that
precede a habit's own creation (S6 needs seven bars, and those days have no step
at or before them). They stand at **the goal the habit started on**. The bar is
faint there anyway, and a zero-height bar would punch exactly the hole the faint
bar exists to avoid.

**Raised for the owner, not settled:** S3's prose says *« no bar changes »*, while
the mechanism this node specifies — *« the goal active on day D being the last
dated step ≤ D »* — makes **today's own bar rise when the goal is grown today**.
It stays faint (an unpractised day is never filled by deciding) and no bar is
added, so the correction itself holds; but the height of the current, unlived day
does follow the new goal. The delivered tests assert the days already lived are
untouched and stop short of today. Both readings are defensible — today's faint
bar at the new goal reads as *« today I aim at 6, not done yet »* — so the choice
is the owner's.

**Specified as** `[[practice-staircase]]` — 6 scenarios, `@wip`. S3 is the one
that pins the correction itself: adjusting the goal draws nothing until a day is
done. S6 pins that a brand-new habit already shows seven faint bars — *an empty
start is still a start*, per `[[design-principes-kaizen]]`.

**Gherkin debt this exposes.** `[[adjust-goal]]` S1 and S2 both assert *« the
change is recorded in the step history with today's local date »* — that is the
technical model leaking into a functional spec; `StepHistory` is a code name, not
a word the domain speaks. And **no scenario describes the staircase at all**,
which is why the drawing could be wrong while every gate stayed green. Both the
Architect and the reviewing Developer flagged the missing staircase scenarios
during slice 3; the gap was recorded and not acted on. Rewrite S1/S2 in the
user's language and add the staircase scenarios when 3b is refined.

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
| 5 `pause-resume` | `[[pause-resume]]` | S1–S4, covered (S3 pins Q1 — a paused habit keeps its seat; S4 pins the rest screen, added this cycle) |
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
| 5 `pause-resume` | **done.** `LifecycleState::{Active, Paused}` landed as an enum on `Habit` (illegal combos unrepresentable; `Anchored` deliberately absent until its use case exists). Two use cases, `PauseHabit` and `ResumeHabit`, one public method each, **neither taking a `Clock`** — nothing dates these transitions. `HabitBoard` untouched: paused keeps the board seat (Q1), pinned by a wired test rather than a comment. Read side reshaped: `ListBoardHabits` now returns the per-screen DTO `TodayHabits {active, paused}`, so "a paused habit leaves the day's list" is a rule in the core, not in a view. |
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

Landed with slice 5 (now in `[[glossary]]`): Mettre en pause (*pause*), La reprendre
(*resume*), État du cycle (*Active / En pause* — `LifecycleState`; *Ancrée* joins in
slice 6), Zone « En pause · aucune pression ».

Still to add as their slices land: Anchor/Readmit (*ancrer / remettre dans le
quotidien* — slices 6-7), Minutes gagnées (slice 8).

## Slice 5 `pause-resume` — settled during delivery, 2026-08-06

| Point | Decision |
|---|---|
| The detail of a **paused** habit | A **rest screen**: its practice staircase, and « La reprendre ». Nothing else — no ritual, no *grandir*, no *alléger*. *« Une pause est un vrai repos : rien à pratiquer, rien à ajuster. »* The domain still forbids nothing (Q3 holds unamended); the screen stops offering, the rule never starts refusing. Specified as `[[pause-resume]]` S4, promoted before implementation so the decision could not ship unspecified |
| Where pausing lands you | **On the detail**, which re-reads itself into the rest screen. Amends the designer's `pause(id) → screen = Today` (`[[design-ecrans]]`): that line predates the rest screen, and returning to Aujourd'hui would hide the screen just drawn and move the user away from their undo. Also the testable shape — programmatic navigation has no precedent here and would bury the gesture's only logic in an untested handler |
| The day's tally « X sur Y » | Counts **active habits only** — a habit at rest is not a habit missed. Structural, not a discipline: the query hands the screen two separate lists |
| The paused zone when nothing is paused | **Not rendered.** A heading over an empty region, in a product whose first principle is the absence of guilt, is a silent reproach |

**Left to the owner, not settled:** when *every* habit is paused, Aujourd'hui reads
« 0 sur 0 · c'est déjà quelque chose. » above an empty list. Deliberately putting
everything to rest is not the same as a day gone by unlived, and the copy does not
yet tell them apart. No scenario names this state.

**Gherkin debt from slice 3b, still open**: `[[adjust-goal]]` S1 and S2 assert *« the
change is recorded in the step history »* — `StepHistory` is a code name, not a word
the domain speaks. To rewrite in the user's language; independent of any slice.
