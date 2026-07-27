---
id: "adr-0008-goal-based-dose-user-paced-progression"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-27"
relations:
  supersedes:
    - "adr-0005-progression-suggestion-policy"
  related:
    - "architecture-overview"
    - "habit-progression-study"
    - "adr-0006-cqrs-light"
    - "adr-0001-validation-by-construction"
    - "adr-0009-quality-gates"
    - "adr-0011-one-public-method-per-use-case"
  depends-on:
    - "adr-0007-habit-lifecycle-aggregate"
    - "adr-0002-habitboard-stateful-aggregate"
answers:
  - "Why is the floor of 1 minute a business rule rather than a display clamp?"
  - "Where is next_goal_up / next_goal_down computed, and why not in the view?"
  - "What is the intended exit for a user who wants less than one minute?"
  - "Is there one dose concept or two (InitialDuration + Dose)?"
  - "What are the Goal's bounds — default, floor, ceiling — and is the ≤5-min creation guard kept?"
  - "Is a Goal a commitment or a soft target, and does it change completion semantics?"
  - "Does the app ever detect stability or suggest growth/anchoring?"
  - "How does the dose change — user gesture only, or system-paced?"
  - "Is StepHistory kept, and what does it now record?"
  - "Why does this override the ADR-0005 / habit-progression-study evidence conclusion?"
decided_in:
  - "LOCAL-goal-based-dose"
  - "2026-07-27 slice 3 adjust-goal cycle (floor = business rule, lighten-at-floor settled)"
---

# ADR 0008 — Goal-based dose, user-paced progression (single `Goal` VO, no stability detection, no suggestion)

> **One-liner**: The two dose concepts collapse into **one `Goal` value object** — a *soft daily target* (**default 5 min, floor 1, no upper ceiling**, the ≤5-min creation guard dropped). A Goal is a target, **not a commitment**: completion stays **binary**. Progression is **user-paced** — `grow()` / `lighten()` (±1 on the Goal) survive as **always-available gestures that the system NEVER suggests**; the read-side `StabilityPolicy` (detection + suggestion) is **removed entirely**. `StepHistory` (dated Goal changes) is **kept** as the self-paced staircase.
> **Links**: [[adr-0005-progression-suggestion-policy]] (**SUPERSEDED by this ADR** — the whole suggestion/detection model is withdrawn), [[adr-0007-habit-lifecycle-aggregate]] (the lifecycle aggregate whose `Dose`/`InitialDuration` facets this collapses into `Goal`; every other facet stands), [[adr-0006-cqrs-light]] (the read side loses its `growth_suggested`/`anchor_suggested` DTO fields but is otherwise unchanged), [[architecture-overview]] (write/read shape updated), [[habit-progression-study]] (the evidence base whose *conclusion* this owner decision overrides — see Provenance), [[adr-0002-habitboard-stateful-aggregate]] (the aggregate-mutation discipline still holds).
>
> **Provenance — owner product decision, research explicitly declined**: this is an **owner product decision**, not an evidence-derived one. Deep research was offered and **explicitly declined** by the owner. It **supersedes the [[adr-0005-progression-suggestion-policy]] conclusion** (automatic stability detection + growth/anchor suggestion) and the matching read of [[habit-progression-study]]. Note this is a **shift of conclusion, not a denial of evidence**: the study's own autonomy finding (Singh 2024 — self-selected behaviours form stronger habits) actually *supports* a user-paced model. The evidence base stays valid; the product bet on top of it changes.

## Context

[[adr-0005-progression-suggestion-policy]] modeled progression as a read-side `StabilityPolicy` that *detects* stability from completion history and *suggests* growth/anchoring, with the dose mutating only via user-triggered `grow()`/`lighten()`. [[adr-0007-habit-lifecycle-aggregate]] then modeled the write side with **two distinct VOs** — `InitialDuration` (≤5-min creation guard) and a running `Dose` (floor 1, no ceiling) seeded into `StepHistory`.

The owner has since decided to **simplify the mental model**: there is no "initial duration" distinct from an evolving "dose" — there is a single **daily Goal** the user sets and moves at their own pace, and the app never nudges it. This removes an entire read-side policy and one of the two dose VOs. Because the lifecycle aggregate is **still ADR + docs only, zero production code** ([[adr-0007-habit-lifecycle-aggregate]] d4), this change lands in the docs before any code is written — the future slice specs inherit the collapsed shape instead of building the two-VO / suggestion model and then unwinding it.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Single dose VO | **One `Goal` VO replaces both `InitialDuration` and the planned running `Dose`.** No two-VO split: a habit has exactly one dose concept — its current daily Goal. `StepHistory` seeds at creation with `(creation_date, initial_goal)`; `current_goal() = steps.last().goal` — never stored separately | planned: `core/src/habit_management/domain/goal.rs`, `domain/step_history.rs` |
| Goal bounds | **Default 5 min, floor 1, NO upper ceiling.** The ADR-0007 `InitialDuration` **≤5-min creation guard is dropped** — the Goal is a flexible target the user owns, not a rule the app polices upward. Floor at 1 is the one true invariant, enforced twice (`Goal` construction cannot build `< 1`, and `lighten()` computes `max(1, current-1)`) | — |
| Soft target, not commitment | A Goal is a **soft daily target, not a commitment**. It does **not** change completion semantics: **completion stays binary** (`toggle_done(today)` per [[adr-0007-habit-lifecycle-aggregate]]) — a day is done or not, independent of whether the Goal was "met" in minutes | — |
| Progression = user-paced | The dose changes **only** via `grow()` (push step +1) / `lighten()` (push step `max(1, current-1)`), each triggered by an **explicit user gesture**. These gestures are **always available** (no precondition, no stability gate) and the system **NEVER suggests** them | planned: `domain/habit.rs` |
| StabilityPolicy REMOVED | The read-side `StabilityPolicy` of [[adr-0005-progression-suggestion-policy]] — stability *detection* + growth/anchor *suggestion* — is **removed entirely**. No detection, no suggestion, no thresholds (the 10-of-14 / step-held-14d values are withdrawn) | — |
| StepHistory kept | `StepHistory` (dated `Vec<StepChange { on: LocalDate, goal: Goal }>`) is **kept** — it is the **self-paced staircase**: the dated record of the user's own Goal changes, and the source of "minutes gagnées" (Σ Goal active on each completed day). This is the surviving reason the change must stay dated | planned: `domain/step_history.rs` |
| Everything else unchanged | Binary mark-done, `CompletionHistory` (ordered set of `LocalDate`, kept forever), library-free `LocalDate` VO + infra `Clock` port, `LifecycleState {Active, Paused, Anchored}`, CQRS-light, two-crate dependency rule, `HabitBoard` aggregate + board-driven creation, validation-by-construction — **all stand** exactly as settled in ADR-0007/0006/0003/0002/0001 | — |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| **Two distinct VOs** (`InitialDuration` + running `Dose`) — the [[adr-0007-habit-lifecycle-aggregate]] shape | The owner's model has a single dose concept: a flexible daily Goal. Two VOs with different rules (creation guard vs. running floor) encode a distinction the product no longer makes — needless conceptual weight. Collapsing to one `Goal` is the simpler true shape (KISS) |
| **≤5-min creation guard, or any upper ceiling** | The Goal is a soft target the user owns, not a demand the app caps. A ceiling (whether the old ≤5 creation guard or a new high ceiling) would police a value that is explicitly the user's to set; only the floor-at-1 is a real invariant |
| **Any auto-pace / stability-detection rule** (the ADR-0005 suggestion model) | Owner product decision: progression is user-paced, the app never nudges. Keeping detection + suggestion would preserve a whole read-side policy the product has withdrawn. Supported, not just permitted, by the evidence: autonomy strengthens habit formation (Singh 2024) |
| **Dropping `StepHistory`** (store only the current Goal) | Loses the dated staircase: "minutes gagnées" needs each Goal **dated** to the day it was active, and the self-paced history is itself a product artifact. Undated storage cannot reconstruct it — same ground as ADR-0007's rejection of `Vec<u32>` steps |

## Provenance / supersession (recorded honestly)

- **Type of decision**: owner **product** decision — not evidence-derived. Deep research was offered and **explicitly declined**.
- **What it overrides**: the [[adr-0005-progression-suggestion-policy]] *conclusion* (automatic stability detection + growth/anchor suggestion) and the matching product read of [[habit-progression-study]]. ADR-0005 is marked **superseded**; ADR-0007's two-VO dose facet is **amended into `Goal`**.
- **Evidence is not denied**: [[habit-progression-study]] stays a valid evidence base. Its autonomy finding (Singh 2024: self-selected behaviours form stronger habits) *supports* a user-paced model. This is a **shift of conclusion built on the same evidence**, not a rejection of it. The study's minority path (Adams RCTs, bidirectional auto-adaptation) remains the researched fallback if user-paced progression ever measurably underperforms — adopting it would supersede this ADR.

## Consequences / Constraints

- **MUST**: model the dose as a single `Goal` VO — default 5, floor 1, **no upper ceiling**; enforce the floor at 1 both in `Goal` construction and in `lighten()` (`max(1, current-1)`).
- **MUST**: seed `StepHistory` at creation with `(creation_date, initial_goal)`; derive `current_goal()` from `StepHistory.last()` — never store the current Goal separately.
- **MUST**: keep the Goal a **soft target** — completion stays **binary**; `toggle_done` must not gate on whether the Goal was met in minutes.
- **MUST**: change the dose **only** via user-triggered `grow()`/`lighten()`, always available, no stability precondition.
- **MUST NOT**: implement any `StabilityPolicy`, stability detection, growth/anchor suggestion, threshold (10-of-14 / step-held-14d), or `growth_suggested`/`anchor_suggested` DTO field.
- **MUST NOT**: reintroduce a separate `InitialDuration` VO, the ≤5-min creation guard, or any upper ceiling on the Goal.
- **MUST NOT**: drop or undate `StepHistory` — the dated staircase is required for "minutes gagnées" and is a product artifact.
- ~~**MAY**: choose the `lighten()`-at-floor behavior (silent no-op vs. error vs. UI "already-at-floor" signal) at implementation time — still open per [[adr-0007-habit-lifecycle-aggregate]] d2.~~ **SETTLED 2026-07-27 — silent no-op**; see the amendment below and [[adr-0007-habit-lifecycle-aggregate]]'s d2 amendment.
- **Out of scope**: any production code (the aggregate still grows vertically inside the lifecycle slices); ~~the `lighten()`-at-floor resolution~~ **(settled 2026-07-27)**; the board↔habit anchoring coordination; all functional/UI wording (PM lane).

---

## Amendment — 2026-07-27, slice 3 `adjust-goal` (the goal facets are now built)

Slice 3 implements the progression gestures this ADR specified. Three points are settled
or sharpened; the ADR's substance is unchanged.

### The `lighten()`-at-floor behavior is a **silent no-op**

The `MAY` above is spent. `Habit::lighten` at a goal of 1 appends nothing and returns;
`LightenGoal::execute` returns `Ok(())`. Both alternatives were rejected on this ADR's own
grounds: an error contradicts *« alléger n'est pas reculer, c'est enlever ce qui freine »*,
and a UI signal contradicts the "always available, no precondition" rule stated in the
Decision table above. The full reasoning is recorded once, in
[[adr-0007-habit-lifecycle-aggregate]]'s d2 amendment — not duplicated here.

### The floor of 1 is a **business rule**, not a display convenience

Human ruling: *« oui règle business forcément 1 minutes sinon on doit la supprimer »*.
Below one minute there is no shorter habit — **there is no habit**, and the only legitimate
exit is deleting it. The floor is therefore a domain invariant with a product meaning, not
a clamp applied for rendering.

**Consequence on placement**: `next_goal_down` is computed in the query
(`core/src/habit_management/queries/get_habit_detail.rs`), **not** in the Dioxus view. A
rule of that nature must not live in a crate excluded from the mutation gate's perimeter
([[adr-0009-quality-gates]]) — the view can render the number, it may not decide it. The
same holds for `next_goal_up`.

**Implied future affordance — named, not designed**: the floor's rationale points at
**habit deletion** as the real exit below one minute. This slice does not build it and does
not specify it. It is named here so the next cycle that meets "the user wants less than one
minute" finds the answer already framed, rather than inventing a sub-minute goal.

### `grown()` / `lightened()` are derivations, not construction paths

`Goal::grown()` = `Goal(self.0.saturating_add(1))` and
`Goal::lightened()` = `Goal(self.0.saturating_sub(1).max(Self::MIN))` are pure derivations
built **inside `core/src/habit_management/domain/goal.rs`**, deliberately not routed through
the fallible `new`. That they do not bypass [[adr-0001-validation-by-construction]] is a
non-obvious property with its own proof — recorded in that ADR's 2026-07-27 amendment, where
the invariant lives.
