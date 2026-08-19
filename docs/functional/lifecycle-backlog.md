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
| 6 | `anchor` | Anchor button (**user-initiated, no 10-of-14 suggestion**), board frees the slot, Ancrées screen counts | L | **done** (4 scenarios; detail re-renders as a sober anchored screen — no gesture at all; Ancrées screen ships the list + the count, nothing more) |
| 7 | `readmit` | "La remettre dans mon quotidien" — refusable (board full / duplicate title) | M | **done** (4 scenarios; Ancrées screen: per-row readmit button, quiet refusals, parallel-count footer shipped) |
| 8 | `stats-board` | Per-habit stats: days done, empty days (never "failed"), grow/lighten counts, minutes gained (reframe wording — nominal, anti-guilt) — plus adaptive (never guilt-inducing) messages | M | **done** (5 scenarios; the recap is a zone of the detail screen — see "settled during delivery" below) |

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

**Gherkin debt this exposed — both halves now closed.** `[[adjust-goal]]` S1 and S2
used to assert *« the change is recorded in the step history with today's local
date »* — the technical model leaking into a functional spec, `StepHistory` being a
code name rather than a word the domain speaks. And **no scenario described the
staircase at all**, which is why the drawing could be wrong while every gate stayed
green; both the Architect and the reviewing Developer flagged that gap during slice 3
and it was recorded without being acted on.

Both are settled. S1 and S2 now read *« the change is added to the habit's record
with today's date »* — rewritten in slice 3 itself (`cd66d22`), which is why no
later cycle found anything to fix. The staircase gained its own scenarios with 3b
(`[[practice-staircase]]`, S1–S6). The debt survived here only as an unretracted
note.

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
| 6 `anchor` | `[[anchor-habit]]` | S1–S4, covered (S3 pins Q3, S4 pins "no suggestion") |
| 7 `readmit` | `[[readmit-habit]]` | S1–S4, covered (S2 pins the full-life refusal, S3 the retaken-title refusal, S4 the parallel-count footer with a paused habit in the fixture) |
| 8 `stats-board` | `[[habit-stats]]` | S1–S5, covered |

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
| 6 `anchor` | **done.** `LifecycleState::Anchored` lands, completing the enum (`Habit::anchor()` — since PR 2 it requires an `Active` habit and refuses anything else). Resolves the deferred board↔habit coordination as a **synchronous application-service orchestration**: `AnchorHabit` moves the habit to `Anchored`, saves it, then **releases its board entry** (`HabitBoard::release`) and saves the board — publishing nothing. The cap does not "count non-anchored habits" as a query-time filter; the entry is **removed**, which frees the seat **and** the title in the same act (see `[[adr-0012-synchronous-cross-aggregate-coordination]]`). Read side gains `ListAnchoredHabits -> Vec<AnchoredHabit { title }>` and `TodayHabits.anchored_count`, both derived on read. **Debt paid**: the aggregate-boundary debt is closed — the invariant that a single `Habit` cannot violate (max 5 non-anchored) is now enforced at its rightful layer, the `AddHabit` use case, not hosted on an aggregate boundary object. |
| 7 `readmit` | **done.** `Habit::readmit()` lands, completing the transition table (`Anchored → Active`, guard `if state != Anchored` → `TransitionError::NotAnchored`, written `if`-not-`match` per adr-0009 L4); `TransitionError` gains its third variant with its caller. `ReadmitHabit`, one public method, no `Clock` (AD-3), is the **first transition since `AddHabit` that increases the non-anchored count** — it re-applies the set check itself (ADR-0013, AD-9): load → duplicate-before-capacity over `state() != Anchored` read live → `readmit()` → **one** save; a refusal leaves nothing behind. **The trap held**: `resume()`'s guard stays exactly `!= Paused`, readmission is a new method + new use case, `resuming_an_anchored_habit_is_refused` untouched. Read side: `ListAnchoredHabits` returns `AnchoredScreen { habits: Vec<AnchoredHabit { id, title }>, in_daily_life }` — the Ancrées screen now acts (per-row « La remettre », quiet refusals, always-rendered footer « Vous suivez N / 5 habitudes en parallèle » fed by `in_daily_life`, same predicate as the cap). |
| 8 `stats-board` | **done.** No aggregate growth — the recap is a **CQRS-light read** over the two dated histories, computed in `GetHabitDetail::handle` (a zone of one screen ⇒ a field of its DTO, per adr-0006). Two read-only accessors added: `Habit::created_on()` / `StepHistory::started_on()` — the recap's inclusive span anchors on the day the habit was seeded. `CompletionHistory` untouched. |

Lifecycle mutations are **internal state transitions** (load aggregate → method → save),
**not** published events (see `[[adr-0007-habit-lifecycle-aggregate]]`).

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

Landed with slice 6 (now in `[[glossary]]`): **Le quotidien / Daily life** (the set of non-anchored habits, max 5), **Ajouter une habitude / Add a habit** (one gesture, one write, validates and creates synchronously).

~~Habit Board~~ (`HabitBoard` aggregate), ~~Request (a habit)~~ (`request_habit` use case), ~~HabitRequested~~ (domain event), ~~Outbox~~ (event store), and ~~DomainEventPublisher~~ (event port) are **void** — the multi-step *request* → *handle* flow is replaced by the single `AddHabit` use case (slice 6), and the `HabitBoard` itself is deleted in the same slice because its only responsibility — guarding the invariant *"max 5 in the daily life"* — is a single-habit predicate that belongs in `AddHabit`, not a boundary object. They are named here only so nobody re-adds them from an older reading.

Landed with slice 5 (now in `[[glossary]]`): Mettre en pause (*pause*), La reprendre
(*resume*), État du cycle (*Active / En pause* — `LifecycleState`; *Ancrée* joins in
slice 6), Zone « En pause · aucune pression ».

Landed with slice 6 (now in `[[glossary]]`): Ancrer / Ancrée (*anchor / anchored*
— `LifecycleState::Anchored`, user-initiated only), Habitudes ancrées (the
Ancrées screen).

Still to add as its slice lands: ~~Readmettre / Remettre dans le quotidien (*readmit* — slice 7)~~ **landed**. ~~Minutes gagnées (slice 8)~~ **landed as « Minutes de pratique accumulées »** — the `current − steps[0]` sense is void (see the glossary), the recap sums each completed day against the goal in force that day.

## Slice 7 precondition — blocking issue (Security PR 2)

**Issue: `Habit::resume()` has no guard on the daily-life-full invariant.**

Today (slice 6), resuming an `Anchored` habit is unreachable — no screen offers it. At slice 7, readmission will make it reachable. A caller could:
1. `AddHabit` with 5 habits → 5 non-anchored, 0 anchored
2. `Anchor` one → 4 non-anchored, 1 anchored
3. `Resume` the anchored one (today: unreachable; slice 7: offered by *« La remettre »*) → 5 non-anchored again, no check applied, still within the cap by accident
4. But a second `Resume` on another anchored habit would produce **6 non-anchored**, violating the cap.

**Half closed by PR 2, half still owed by slice 7.**

**Closed — the transition itself.** `Habit::resume()` now requires a `Paused` habit and refuses an anchored one outright, so step 3 above is no longer a route back into the daily life. Security's induction: production writes a habit's state in exactly four places, and after PR 2 **no lifecycle transition can grow the daily life** — only `AddHabit` can, and it counts first. See `[[adr-0007-habit-lifecycle-aggregate]]` AD-9.

**Still owed — the count — CLOSED 2026-08-18 (slice 7).** Readmission is `Anchored → Active`: the first transition since adding a habit that grows the daily life. `ReadmitHabit` re-applies the count itself, in the mandated order (read the current non-anchored count → refuse at the cap → *then* move the habit → *then* one single write); the refusal paths write nothing. Security re-verified the induction with the new write site: `readmit()` is the only other +1 and it checks before saving.

**The trap slice 7 must not fall into.** Widening `Habit::resume()` so it can serve readmission re-opens this issue **word for word** — and viciously, the existing tests would still pass, because whoever widened it would delete « resuming an anchored habit is refused » as newly false. `resume()`'s guard stays exactly « requires paused ». Readmission is a **new** gesture with its own method and its own use case, never a loosened one. The full technical constraint is in `[[adr-0007-habit-lifecycle-aggregate]]` AD-9.

**Note on `readmit-habit.feature` scenarios — SUPERSEDED 2026-08-18 (slice 7).** S1 and S2 were rewritten in quotidien vocabulary and S4 (the parallel-count footer) added as part of slice 7's specification pass; the file ships `@wip`-free, all four anchored.

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

**Gherkin debt from slice 3b: closed, and it was never open.** The rewrite it asked
for had already shipped inside slice 3 — see the 3b section above for the full
account. No scenario carries a code name today.

## Slice 6 `anchor` — settled during delivery, 2026-08-11

| Point | Decision |
|---|---|
| Where anchoring lands you | **On the detail**, which re-renders as a sober "ancrée" screen: title, goal, practice staircase — **no gesture at all**. Same reasoning as pause in slice 5: returning to Aujourd'hui would hide the screen just drawn. Amends the designer's `anchor(id) → habit.anchored = true ; screen = Today` (`[[design-ecrans]]`) |
| How the freed seat works | The board's cap counts **entries**, not "non-anchored habits" as a filter — anchoring **removes** the habit's entry from the board (`HabitBoard::release`). The seat and the title are freed by the same act, not two. Corrects the wording carried since slice 5 (see the `[[design-ecrans]]` and `[[feature-catalog]]` amendments below) without reopening Q1, which stands exactly as approved |
| The Ancrées screen's scope | **The list and the count, nothing more.** The designer's node also draws each habit's last 7 days as dots and a footer « Vous suivez N / 5 habitudes en parallèle. » — the dots remain **deferred**; the **footer shipped in slice 7** (2026-08-18), where board-full refusal is the actual subject. No scenario asks for the dots (S2 says only "listed and counted"), and as long as no screen offers to mark an anchored habit done, the dots would freeze at the day of anchoring and replay a pre-anchor history forever. |
| The approved copy | Detail button « L'ancrer · elle est devenue naturelle » ; anchored banner « ancrée · {N} min » ; Today link « Mes habitudes ancrées · {N} », shown only when N ≥ 1 ; Ancrées screen: the titles + « {N} · devenues naturelles » |
| Scenario sufficiency | The four scenarios were judged sufficient; none changed. S3 ("an anchored habit can still be marked done") names **no screen** this slice — it is pinned at the rule level, on `MarkDone`, not in any UI |

**Handed to slice 7:** readmission must handle a title **retaken while the habit
was anchored** — a real rejection path, not a formality. `[[readmit-habit]]` S3
already specifies it (refused as duplicate); `[[adr-0012-synchronous-cross-aggregate-coordination]]`
now cites that scenario as the technical constraint on slice 7's design, since
anchoring frees the title for reuse.

**Gherkin: no debt.** `[[anchor-habit]]` shipped its four scenarios byte-for-byte
as specified, `@wip`-free since `fb71a8d`.

## Slice 8 `stats-board` — settled during delivery, 2026-08-19

The recap is the **8th and last slice**: the detail screen tells a habit's whole
life without guilt. All 8 decisions below were arbitrated by the owner before
implementation and shipped locked.

| # | Decision |
|---|---|
| D1 | The recap is a **zone of the detail screen**, not a 7th screen ⇒ one more field `recap: HabitRecap` on the `HabitDetail` DTO, computed in `GetHabitDetail::handle`. No `GetHabitStats`, no route, no `Services` field. The planned `get-habit-stats/` anchor of [[adr-0006-cqrs-light]] is stale |
| D2 | **Minutes = Σ of the goals of the done days** (total practised). Never Σ(`current − steps[0]`) — the old formula would read "0" to a regular practitioner |
| D3 | **Inclusive span**: every day from creation to today counts, done or not. `days_done + empty_days = age in days`. A habit created today and not done reads « 0 réalisé · 1 autre jour » — the `FreshStart` message carries the gentleness |
| D4 | The recap shows in **all 3 states** (Active / Paused / Anchored) — it is a reading, not a gesture; the rest screens forbid gestures only |
| D5 | **3 messages, none congratulating**. Rest threshold = **7 days** without practice — the only rhythm this app speaks (`WINDOW_DAYS`). No streak, ever |
| D6 | Copy: « réalisés » / « autres jours » / « minutes de pratique accumulées » — "autres jours" avoids the word *empty*; "pratique accumulée" lifts the total-vs-delta ambiguity |
| D7 | **No ADR** — everything applies [[adr-0006-cqrs-light]]. An ADR is never amended; a genuinely changed decision would be a new ADR with `supersedes:` |
| D8 | `CompletionHistory` stays **closed** (`new / toggle / contains` only) — the day-by-day walk needs only `contains`, `minus_days` and `Ord` |

**Handed to the recap, not reopened:** the 7-day window (slice 3b), the soft goal
(ADR-0008), the anti-guilt first principle. The detail screen goes from O(7) to
O(age) on this slice — **accepted, not optimised** (adr-0006 fixes the escalation
trigger: a measured latency problem, none observed).
