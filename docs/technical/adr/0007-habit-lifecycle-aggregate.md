---
id: "adr-0007-habit-lifecycle-aggregate"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-08-06"
relations:
  related:
    - "architecture-overview"
    - "lifecycle-backlog"
    - "adr-0008-goal-based-dose-user-paced-progression"
    - "adr-0011-one-public-method-per-use-case"
    - "adr-0009-quality-gates"
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
  - "What happens when the user lightens a habit whose goal is already at the floor?"
  - "Can StepHistory be empty, and why does current() need no Option or unwrap?"
  - "Do two goal changes on the same day merge into one step?"
  - "Can StepHistory grow without bound, and where does that become a problem?"
  - "Does every use case take a Clock, or only the ones whose transition is dated?"
  - "How does a habit's lifecycle state cross the crate boundary into a view?"
  - "Why does LifecycleState carry only Active and Paused, with no Anchored?"
  - "Who guarantees that a paused habit keeps its seat on the board?"
decided_in:
  - "LOCAL-lifecycle-aggregate"
  - "2026-07-27 slice 3 adjust-goal cycle (d2 settled, StepHistory append-only)"
  - "2026-08-06 slice 5 pause-resume cycle (LifecycleState built, AD-2/AD-3/AD-5)"
---

# ADR 0007 — Habit promoted to the lifecycle aggregate root (dated histories, library-free LocalDate, internal transitions)

> **⚠️ Open point d2 SETTLED (2026-07-27, slice 3 `adjust-goal`)**: `lighten()` at the floor is a **silent no-op** — see the amendment block at the end of this node. `StepHistory` is now built and **append-only**; `Habit::grow` / `Habit::lighten` exist. The "planned" anchors below have landed for the goal facets; every other planned shape (pause/anchor, board coordination) still is.

> **⚠️ Pause facets BUILT (2026-08-06, slice 5 `pause-resume`)**: `LifecycleState` exists — **with `Active` and `Paused` only, no `Anchored`** — and `Habit::pause` / `Habit::resume` are the two transitions. Three decisions are recorded in the second amendment block at the end of this node: **AD-2** (the state crosses the crate boundary as a DTO-side enum, never a `bool`), **AD-3** (the "aggregate methods take `today`" facet is **scoped**, not blanket — neither pause nor resume takes a `Clock`), **AD-5** (the board seat a paused habit keeps is pinned by a wired test, not by a comment). Still planned: `Anchored`, anchor/readmit, and the board↔habit coordination (slices 6/7).

> **⚠️ Dose facets AMENDED (2026-07-23) by [[adr-0008-goal-based-dose-user-paced-progression]]**: the two-VO dose model below (`InitialDuration` with its ≤5-min creation guard + a running `Dose`) is **collapsed into a single `Goal` VO** (default 5, floor 1, **no upper ceiling**, guard dropped), the `StabilityPolicy` dependency is **withdrawn** (progression is user-paced, never suggested), and `StepHistory` now records dated **Goal** changes. **Every other facet of this ADR stands** (aggregate root, `CompletionHistory`, `LocalDate`/`Clock`, `LifecycleState`, internal transitions, repo `get`+upsert). This document is amended in place below; ADR-0008 is the source of truth for the dose/progression facets.

> **One-liner**: `Habit` becomes the **behavioral lifecycle aggregate root** (one aggregate, keyed by `HabitId`) carrying two **dated histories** — `CompletionHistory` (one completion/day, kept forever) and `StepHistory` (dated `Goal` changes, current goal = last step — the dose is a single `Goal` VO per the [[adr-0008-goal-based-dose-user-paced-progression]] amendment) — plus a `LifecycleState` enum (planned as `{Active, Paused, Anchored}`; **built at slice 5 as `{Active, Paused}`** — `Anchored` lands with slice 6's use case). The domain owns a **library-free `LocalDate` VO** (zero `chrono` in its public API); `chrono` is confined to an infra `Clock` adapter. Lifecycle mutations are **internal state transitions** (load → method → save), **not** published events — only `HabitRequested` stays on the outbox.
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
| Clock passed as parameter (**scope amended 2026-08-06, AD-3**) | Aggregate methods **that need a date** take `today: LocalDate` as a **plain parameter**; the **use case** holds the `Clock` and passes `clock.today()`. The aggregate stays a **pure function** — no clock stub in domain unit tests. **This is not a blanket template**: a method that dates nothing takes no `today`, and its use case holds no `Clock` — see the AD-3 amendment below | — |
| Lifecycle state (**built 2026-08-06, partially**) | `LifecycleState` **enum** on `Habit` (not two bools — illegal combinations unrepresentable). Transitions hub through `Active` (resume/readmit → `Active`). **`toggle_done` never inspects `LifecycleState`** — a paused or anchored habit stays markable-done (no guard). Pause keeps the board seat; anchor frees it (the cross-aggregate board coordination is **deferred to slice 6**). **Slice 5 built the enum with `Active` and `Paused` only** — `Anchored` is deliberately absent until slice 6 introduces the use case that needs it (use-case-driven discipline: no Domain variant without its calling use case) | `core/src/habit_management/domain/lifecycle_state.rs` |
| Events (d3) | Lifecycle mutations are **internal state transitions** (load aggregate → method → save) — **NOT published**. Event Storming names the moments (mark done / toggle off, grow / lighten, pause / resume, anchor / readmit) but publishes **none**: there is no subscriber and [[adr-0006-cqrs-light]] has no projections to feed. Only **`HabitRequested`** stays published (it crosses the aggregate boundary via the outbox). **No `HabitEvent` enum; `HabitBoardEvent` + outbox untouched** | — |
| Repository (d5) | `HabitRepository` gains `get(&HabitId) -> Option<Habit>` and **upsert-by-id `save`** semantics (save an existing id overwrites). Introduced in **slice 2** | planned: `domain/habit_repository.rs` |
| Read-side compatibility | Slice-1 reads stay stable: `id`/`title` unchanged; minutes read via a `current_goal()` accessor **now** (returns the initial Goal until step history lands → zero rework in slice 3). `done_today` source changes from the `false` default to `completion_history.contains(clock.today())` in slice 2 — the Today query must accept an **injected `Clock`** in slice 2 | — |

**~~Open implementation point (d2 — deferred to slice 3, NOT final)~~ — SETTLED 2026-07-27**: `lighten()` at the floor is a **silent no-op**. The provisional default became the decision; the two alternatives listed here (an error, a UI "already-at-floor" signal) were both rejected. Reasoning, rejections and the shape actually built are in the **amendment block below** — that block, not this paragraph, is the source of truth.

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
- **MUST**: pass `today: LocalDate` to aggregate methods **that need a date** as a parameter; the use case holds the `Clock`. The aggregate stays a pure function (no clock stub in domain unit tests). **MUST NOT** inject a `Clock` into a use case whose transition dates nothing (AD-3, 2026-08-06).
- **MUST**: model pause/anchor with the `LifecycleState` enum (never two bools); `toggle_done` must NOT inspect `LifecycleState` (paused/anchored habits stay markable-done). **MUST**: keep the enum's variants use-case-driven — a variant lands with the use case that transitions into it, not ahead of it (`Anchored` is slice 6's).
- **MUST**: cross the crate boundary with a **DTO-side enum**, never the domain `LifecycleState` and never a `bool` (AD-2, 2026-08-06).
- **MUST NOT**: publish any lifecycle event, introduce a `HabitEvent` enum, or touch `HabitBoardEvent` / the outbox — only `HabitRequested` is published (d3).
- **MUST NOT**: create a second aggregate for the lifecycle, or reintroduce a separate `InitialDuration`/`Dose` split — the dose is the single `Goal` VO ([[adr-0008-goal-based-dose-user-paced-progression]]).
- **MAY**: introduce `HabitRepository::get(&HabitId)` + upsert-by-id `save` in slice 2 (d5); add the epoch-day arithmetic `LocalDate` needs incrementally as slices require it.
- **Out of scope (this cycle — d4)**: any production code; the aggregate grows vertically inside slices 2/3/5/6/7 of [[lifecycle-backlog]]. Also out of scope: ~~the `lighten()`-at-floor resolution (d2, slice 3)~~ **— settled 2026-07-27, see below**; the board↔habit anchoring coordination (slice 6). The `StabilityPolicy` read-side computation is **removed entirely** ([[adr-0008-goal-based-dose-user-paced-progression]]) — no detection, no suggestion.

---

## Amendment — 2026-07-27, slice 3 `adjust-goal` (d2 settled; `StepHistory` built and append-only)

Slice 3 is the first cycle to write production code against this ADR. Three facets
move from *planned* to *settled*, and one future constraint is recorded. Amended in
place rather than superseded: all four are facets of the same settled question this
node owns — how the lifecycle aggregate stores its dated histories and mutates them.

### d2 — `lighten()` at the floor is a **silent no-op** (human decision)

`Habit::lighten(today)` at a goal of 1 appends nothing and returns; `LightenGoal::execute`
returns `Ok(())`. Nothing is signalled, nothing fails.

| Rejected | Why |
|---|---|
| An `AlreadyAtFloor` error variant | Contradicts the product's register — *« alléger n'est pas reculer, c'est enlever ce qui freine »*. An error turns a permitted gesture into a reproach |
| Any UI signal (disabled button, toast, badge) | Contradicts [[adr-0008-goal-based-dose-user-paced-progression]]'s "always available, no precondition" and the `adjust-goal` S4 scenario; and a signal returned from a command would breach [[adr-0006-cqrs-light]]'s commands-return-`()` shape |

**Feedback is by state, not by reproach.** Nothing tells the user "you cannot"; the
re-queried screen simply keeps reading « chaque jour · 1 min » while the button reads
« Alléger à 1 min ». The user sees where they are, not what they are refused.

**Where the floor lives.** `Habit::lighten` infers the floor from the domain rather than
restating it: it computes `lightened()` and compares it to the current goal —
`lightened() == *current()` means the floor was reached. The constant `Goal::MIN` stays
owned by `Goal` alone; the aggregate duplicates no knowledge of its value.

### d6 — `StepHistory` is **append-only**, with no same-day fusion (human decision)

Human ruling: *« oui on empile et depile on laisse la liberté sans fusion plus simple »* —
where *« dépiler »* means the **staircase may go back down**, not that a step is removed.

| Facet | Decision | Anchor |
|---|---|---|
| Shape | `StepHistory { first: StepChange, rest: Vec<StepChange> }`. Non-emptiness stays **structural**: `seeded` is the only constructor, so `current()` is **total** — `rest.last()` falling back to `first`. **No `Option`, no `unwrap`, no panic path** | `core/src/habit_management/domain/step_history.rs` |
| Mutation | `record(&mut self, on, goal)` **appends only**. No removal, no pop, no undo | idem |
| No same-day fusion | Three taps of « grandir » in one day yield **three dated steps**, not one merged step | idem |

**Why append-only wins.** Appending preserves information; fusing destroys it
irreversibly. A noisy staircase is a **rendering** question — solvable read-side, later,
without loss — while a fused history can never be un-fused. Deferring the question costs
storage; deciding it early costs the data.

### FUT-1 — the oscillation path, and the constraint it places on the persistence slice

Above the floor, alternating grow / lighten returns the goal to its starting value while
adding **2 `StepChange` per round trip**, indefinitely — precisely because d6 forbids
same-day fusion. It is not merely a doubled append rate: it is an **unbounded history
with zero net effect**, reachable by a user tapping two buttons.

Harmless today — the store is in-memory, local, single-user, and the history dies on
reload. **It stops being harmless at the persistence slice**, where the history survives
reloads and unbounded growth becomes unbounded storage.

**Constraint carried forward**: the persistence slice's spec MUST weigh bounding or
compacting `StepHistory`, explicitly in the knowledge of d6. Compaction is a *storage*
decision and must not be smuggled in as a domain-level fusion — that would silently
reverse d6.

### What is now built (was "planned")

`domain/step_history.rs`, `domain/goal.rs`, and `Habit::grow` / `Habit::lighten` exist —
`core/src/habit_management/domain/step_history.rs`, `core/src/habit_management/domain/goal.rs`,
`core/src/habit_management/domain/habit.rs`. The two gestures are driven by **two separate
use cases**, one public method each, per [[adr-0011-one-public-method-per-use-case]].
`CompletionHistory`, `LocalDate`, the `Clock` port and the repository `get`+upsert landed
in slice 2. Still planned: `LifecycleState` and the board↔habit anchoring coordination
(slices 5/6).

---

## Amendment — 2026-08-06, slice 5 `pause-resume` (`LifecycleState` built; AD-2, AD-3, AD-5)

Slice 5 is the cycle that builds the lifecycle *state* this ADR planned. Three decisions
were taken and validated; all three are facets of the question this node already owns —
how the lifecycle aggregate models and exposes its state — so they are amendments, not a
new node.

### What is now built (was "planned")

`core/src/habit_management/domain/lifecycle_state.rs` exists, carrying **`Active` and
`Paused` only**. `Habit` gains a `state: LifecycleState` field (named explicitly as
`Active` in `Habit::new` — the enum derives **no `Default`**, so the initial state is a
statement rather than a fallback), the accessor `state()`, and the two transitions
`pause()` / `resume()`. Two command use cases drive them, one public method each per
[[adr-0011-one-public-method-per-use-case]] —
`core/src/habit_management/use_cases/pause_habit.rs`,
`core/src/habit_management/use_cases/resume_habit.rs`.

**`Anchored` is deliberately absent.** The enum planned three variants; slice 5 built two.
A variant with no use case to transition into it is a Domain type arriving without its
caller — exactly what the use-case-driven discipline forbids. `Anchored` lands in slice 6,
with `AnchorHabit`. The cost is one more line in two `match` sites; the benefit is that
those `match` sites **fail to compile** in slice 6 and hand that developer the exhaustive
list of places to update.

### AD-2 — the state crosses the crate boundary as a **DTO-side enum**, never a `bool`

`GetHabitDetail`'s DTO carries `state: HabitState`, a **second, DTO-side enum** declared
next to its query in `core/src/habit_management/queries/get_habit_detail.rs`, mapped from
the domain's `LifecycleState` by an exhaustive `match`.

Two halves, and only the second is a new decision:

| Half | Status |
|---|---|
| Not the domain `LifecycleState` | **Already settled** — [[adr-0006-cqrs-light]]'s MUST ("`kayzen-app` never imports a domain type"), reinforced by [[adr-0010-crate-boundary-trust-boundary]]. Reused, not re-decided |
| Not a `bool` either | **The decision.** An enum on the DTO side too |

`paused: bool` would have worked today and broken next slice. This node's own rejected
alternatives already refuse two booleans **in the domain**, because they make
`paused && anchored` representable — and the view is precisely where an impossible
combination becomes a rendering bug rather than a compile error. Slice 6 would have put
`anchored: bool` beside `paused: bool` one slice from now. `Habit::state() ->
LifecycleState` follows the same reasoning one layer down, with a concrete payoff: the
exhaustive `match` in **both** queries stops compiling when `Anchored` is added, instead
of silently evaluating to `false`.

| Rejected | Why |
|---|---|
| `paused: bool` on the DTO | Makes the impossible combination representable exactly where it renders; slice 6 turns one bool into two |
| Exposing the domain `LifecycleState` to the app crate | Breaches [[adr-0006-cqrs-light]]'s MUST — the DTO exists so the app needs no domain type |
| A pre-rendered `String` ("actif"/"en pause") | Puts the vocabulary in the core and leaves the view unable to branch on the state |

### AD-3 — the "aggregate methods take `today`" facet is **scoped**, not a blanket template

Neither `PauseHabit` nor `ResumeHabit` takes a `Clock`, unlike `GrowGoal`, `LightenGoal`
and `MarkDone`. The Decision-table row above ("Clock passed as parameter") read as a
blanket template; it is hereby **scoped to methods that need a date**.

Nothing about pausing or resuming is dated. Injecting a `Clock` for template symmetry
would mean a `FixedClock` in every test of both use cases, buying **no assertion** — a
constructor parameter nobody reads is not consistency, it is ceremony, and it invites the
next reader to date the transition because the clock is right there.

Recorded because it was flagged: both Dev-B and QA read the asymmetry as a decision rather
than an omission. It should read that way to future readers too.

**The rule going forward**: a use case holds a `Clock` **iff** the aggregate method it
calls takes `today`. `pause()` and `resume()` take nothing; `PauseHabit::new` and
`ResumeHabit::new` take only the repository. If a later slice needs a *paused-since* date,
that is a new field on the aggregate and a new decision — not a clock quietly added back.

### AD-5 — the board seat is pinned by a **wired test**, not by a comment

Scenario `pause-resume/S3` — a paused habit keeps its seat, so a sixth request is still
rejected — is **true by construction today**: `HabitBoard::request_habit` counts
`requests.len()` with no state filter, and knows nothing of `LifecycleState`. That is
precisely why it is fragile: nothing in the code would object to a future filter, and the
property is invisible.

The test in `core/src/habit_management/use_cases/pause_habit.rs` (`// @scenario:
pause-resume/S3`) wires `RequestHabit` + `CreateHabitOnRequest` + `PauseHabit` over shared
in-memory stores: five habits requested and created, one paused, a sixth request still
rejected, and the paused habit still paused afterwards. **`HabitBoard` itself is
untouched** — the board↔habit coordination stays deferred to slice 6, as this ADR's
Decision table says. What the test buys is that the *absence* of coordination is now
asserted rather than assumed.

The alternative — a comment saying "the board does not filter on state" — is exactly the
class of invariant this graph exists to stop restating in prose.

### AD-8 — the paused zone renders only when non-empty (consequence, recorded for the reader)

`app/src/views/today.rs` guards the whole paused region on `!paused.is_empty()`. A heading
« En pause · aucune pression » standing over an empty region, in a product whose first
non-negotiable is *pause sans culpabilité*, is a silent reproach — it names an absence the
user did not create. Not an architectural fork; recorded so a later refactor does not
"simplify" the guard away. Pinned by a test (`app/src/views/today.rs`, the paused-zone
absence assertion) after a hand-run mutation showed `if true` passing the whole suite —
see [[adr-0009-quality-gates]]'s 2026-08-06 amendment.

### Owner ruling this cycle inherited (functional, but it shapes the code)

Pausing **keeps the user on the detail screen**, which re-reads and becomes the rest
screen. No programmatic navigation: `navigator().push` has no precedent in this repo, and
it would put the gesture's only logic inside an untested `onclick`. The shape is
**mutate-then-reload free functions** — `pause_and_reload` / `resume_and_reload` in
`app/src/views/habit_detail.rs`, `resume_and_relist` / `mark_done_and_relist` in
`app/src/views/today.rs`. That shape is also the mitigation for the view blind spot;
[[adr-0009-quality-gates]] owns the reasoning.
