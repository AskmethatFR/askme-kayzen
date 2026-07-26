---
id: "adr-0005-progression-suggestion-policy"
type: "technical"
owner: "architect"
status: "superseded"
updated: "2026-07-23"
relations:
  # No hand-written `superseded-by` here: it is an INVERSE edge, derived at
  # query time from adr-0008's own `supersedes`. Authoring it by hand made the
  # indexer store an unknown edge type verbatim and warn on every run.
  related:
    - "architecture-overview"
    - "habit-progression-study"
  depends-on:
    - "adr-0002-habitboard-stateful-aggregate"
answers:
  - "Where do progression rules live — aggregate invariant or policy?"
  - "Does the app ever change a habit's dose automatically?"
  - "Where do the 10-of-14 / step-held-14-days thresholds live, and can they be tuned without a new ADR?"
  - "Why is there no stored GrowthProposal entity, decline tracking, or suggestion expiry?"
  - "What is the researched fallback if suggestion-based progression underperforms?"
decided_in:
  - "LOCAL-5"
---

# ADR 0005 — Progression modeled as a read-side stability policy (suggestion, never mutation)

> **⚠️ SUPERSEDED (2026-07-23) by [[adr-0008-goal-based-dose-user-paced-progression]]** — an owner product decision. The suggestion/detection model below (the `StabilityPolicy`, stability detection, growth/anchor suggestion, and the 10-of-14 / step-held-14d thresholds) is **withdrawn**: progression is now **user-paced** (`grow`/`lighten` always available, never system-suggested), the dose is a single `Goal` VO, and no `StabilityPolicy` exists. This document is retained as a **historical record** of the prior evidence-derived decision. See ADR-0008 for the current model and the supersession provenance.

> **One-liner**: Habit progression is a **stateless read-side policy** — the app automatically *detects* stability from completion history and *suggests* growth/anchoring; the dose mutates **only** through explicit aggregate methods (`grow()` / `lighten()`) invoked by a user gesture. Thresholds are tunable policy values, not invariants.
> **Links**: [[architecture-overview]] (where this will apply), [[habit-progression-study]] (the verified evidence base — this ADR cites it, it does not duplicate it), [[adr-0002-habitboard-stateful-aggregate]] (the aggregate-mutation discipline this extends to the dose).
>
> **Timing note — decision capture ahead of implementation**: the habit-lifecycle aggregate that will carry `grow()`/`lighten()` and the read side that will carry the policy are **not yet built**. This ADR pins the model *before* implementation so the future tech spec inherits it instead of re-deciding; anchors below are therefore planned shapes, not existing files.

## Context

The progression fork — automatic vs suggested vs manual difficulty increase — was settled by the human on 2026-07-16 after an adversarially verified deep-research run. The full evidence (Lally 2010, Singh 2024, Adams RCTs, industry survey) lives in [[habit-progression-study]]; the settled functional rule is: **progression = automatically detected stability + suggestion; the dose changes only via explicit user gesture** (grow / lighten), preserving the designer's "proposé, jamais imposé" non-negotiable.

This ADR captures the *technical modeling* of that rule and its DDD grounding, so the implementation spec starts from a settled shape.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| DDD category | Progression rules are a **Policy** (Event Storming: Event → Policy → Command; Brandolini), **not** an aggregate invariant. Vernon: aggregates protect only *true transactional invariants* — "suggest growth after stability" is reactive logic, not a consistency rule | — |
| Automation level | **Detection automatic, application manual.** Both automatic and manual policies are first-class in the Policy pattern — the automation level is a policy *implementation choice*, not a domain-model change. Our policy emits a *suggestion*; the *command* (`grow`/`lighten`) stays with the user | — |
| Shape | Pure **stateless domain service** `StabilityPolicy`, consumed by the **READ side**; suggestions surface as DTO fields `growth_suggested` / `anchor_suggested` | planned: lifecycle read model |
| Recompute, never store | Suggestions are recomputed from completion history **on every read** — no stored proposal entity, no decline tracking, no expiry, no re-nag (anti-guilt by design) | — |
| Dose mutation | The dose changes **only** via aggregate methods `grow()` / `lighten()`, each triggered by an explicit user command. **True invariants** (these DO belong to the aggregate): dose never changes without a user command; floor at 1 min | planned: lifecycle aggregate |
| Thresholds | **Policy values — tunable, NOT invariants.** Starting values (conservative): growth suggested = done ≥ 10 of last 14 days AND current step held ≥ 14 days; anchor suggested = done ≥ 10 of last 14 days (designer's rule). Tuning them is a config-level change, no new ADR needed; changing the *suggestion-never-mutation* model would require superseding this ADR | [[habit-progression-study]] |
| Window shape | Rolling **X-of-last-Y** windows — validated by evidence (single-day misses harmless, Lally 2010); never consecutive-day streaks | — |

**Open implementation point (defer to the implementation spec, do not resolve here)**: growth and anchor suggestions share the 10-of-14 completion clause — the step-held clause differentiates them, but their interplay (can both fire at once? which wins in the UI?) is unresolved.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Literal automatic dose increase | Product grounds: violates the designer's "proposé, jamais imposé"; industry is unanimous (no major habit app auto-escalates demand; documented backlash against imposed pressure). Modeling grounds: it would smuggle reactive policy logic into the aggregate as if it were an invariant — the wrong DDD category |
| Stored `GrowthProposal` entity (with decline tracking / expiry) | State with no owner-question to answer: recomputing from history on every read yields the same answer with zero lifecycle to manage. Decline tracking + expiry are re-nag mechanics — anti-guilt by design excludes them (YAGNI + product) |
| Per-individual automaticity-asymptote detection | Refuted in the research: the model fails ~half of individuals and needs self-report data the app lacks (Keller 2021 critique) — see [[habit-progression-study]] §6 |
| Consecutive-day streak thresholds | Scientifically unjustified: single-day misses do not harm formation (Lally 2010, verified verbatim); streaks contradict the designer's no-streak/no-guilt principles |

## Minority path (recorded, not adopted)

RCTs (Adams 2013/2017) show **fully automatic *bidirectional* adaptation** (goals can decrease as well as increase) outperformed static goals for adherence — in step-count interventions, not tiny-habit durations. "Science forbids automatic" is therefore false; what the evidence rejects is forced *monotonic escalation*. If suggestion-based progression ever measurably underperforms, evidence-backed bidirectional auto-adaptation is the researched fallback — adopting it would supersede this ADR **and** amend the designer docs.

## Consequences / Constraints

- **MUST**: keep `StabilityPolicy` a pure stateless domain service on the read side — no persistence, no side effects, input = completion history, output = suggestion flags.
- **MUST**: route every dose change through an aggregate method (`grow()`/`lighten()`) behind an explicit user command; enforce the floor at 1 min inside the aggregate (per [[adr-0002-habitboard-stateful-aggregate]]'s discipline: invariants live in the aggregate, never in a use case).
- **MUST NOT**: persist suggestions, track declines, or add expiry/re-nag mechanics.
- **MAY**: tune the threshold values (10-of-14, step-held-14d) as policy configuration without a new ADR.
- **Out of scope**: the lifecycle aggregate itself (future implementation cycle), the growth/anchor interplay resolution (implementation spec), any UI treatment of the suggestion.
