---
id: "adr-0006-cqrs-light"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-27"
relations:
  related:
    - "architecture-overview"
    - "habit-progression-study"
    - "adr-0008-goal-based-dose-user-paced-progression"
    - "adr-0010-crate-boundary-trust-boundary"
    - "adr-0011-one-public-method-per-use-case"
  depends-on:
    - "adr-0003-two-crate-workspace"
    - "adr-0008-goal-based-dose-user-paced-progression"
answers:
  - "How is the read side of habit_management structured — CQRS or shared use cases?"
  - "Do domain types cross the crate boundary into kayzen-app, or do queries return DTOs?"
  - "Do queries get their own ReadModel port, projections, or a separate read store?"
  - "Where will the future per-habit statistics board get its data from?"
  - "What would justify escalating from CQRS-light to full CQRS with projections?"
  - "May a Dioxus view name the DTO its query returns?"
  - "Why does app/src/services/add_habit.rs import a domain error type?"
decided_in:
  - "LOCAL-6"
  - "2026-07-27 slice 3 adjust-goal cycle (DTO-naming scope, HabitBoardError tension recorded)"
---

# ADR 0006 — CQRS-light for the read side of habit_management (query handlers + per-screen DTOs, no projections)

> **One-liner**: The read side is **CQRS-light** — dedicated query use cases (one flat `snake_case` module per query under `core/src/habit_management/queries/`) returning **per-screen DTOs** that are the crate-boundary contract; queries read through the **existing domain ports** against the same store, and all derived display data (stats, minutes gained, messages) is **computed on read** by stateless policies. No projections, no read store, no ReadModel port.
> **Links**: [[architecture-overview]] (the workspace shape this applies to), [[adr-0003-two-crate-workspace]] (the compiler-enforced boundary the DTOs serve), [[adr-0008-goal-based-dose-user-paced-progression]] (the user-paced progression model — supersedes ADR-0005; removes the suggestion DTO fields the read side once carried), [[habit-progression-study]] (evidence base behind the derived-data model).
>
> **Timing note — decision capture ahead of implementation**: the read query use cases are **not yet built** (the first read ticket is next). This ADR pins the shape *before* implementation so the tech spec inherits it instead of re-deciding; anchors below are planned shapes, not existing files. Human-approved 2026-07-16.
>
> **Amended 2026-07-23 by [[adr-0008-goal-based-dose-user-paced-progression]]**: the read side loses its progression-**suggestion** DTO fields (`growth_suggested` / `anchor_suggested`) — the `StabilityPolicy` that produced them is removed (progression is user-paced, never suggested). The CQRS-light shape is **otherwise unchanged**: query use cases + per-screen DTOs as the crate-boundary contract, reads through existing ports, all *other* derived display data (stats, minutes gained, messages) still computed on read.

## Context

The lifecycle backlog ([[habit-progression-study]], `lifecycle-backlog`) requires screens that *read*: the board listing, later a habit detail, and a **per-habit statistics board** (days done, empty days — never framed "failed" —, grow/lighten counts, minutes gained, adaptive motivational non-guilt messages). The open fork was how to structure that read side: shared use cases (no CQRS), CQRS-light, or full CQRS with projections. (The progression *suggestion* fields this originally also carried are withdrawn by [[adr-0008-goal-based-dose-user-paced-progression]] — progression is user-paced, so nothing is suggested on read.)

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Handler split | **Query handlers (read use cases) separate from command use cases.** Commands live under `use_cases/`, queries under a dedicated `queries/` module — one flat `snake_case` module per query: `list_board_habits.rs`, later `get_habit_detail.rs`, `get_habit_stats.rs`. Module/file names are `snake_case` per Rust RFC 430 — flat modules (no `mod.rs`-inception folders), no `#[path]` remapping. The physical `use_cases/` (commands) vs `queries/` split was adopted early rather than deferred | `core/src/habit_management/queries/list_board_habits.rs` |
| Read models | **Per-screen DTOs owned by their query use case** (`HabitSummary`, `HabitDetail`, later `HabitStats`). Duplication across DTOs is acceptable — one screen = one read model, no god read model | planned: DTO next to its query |
| Crate boundary | DTOs are the **crate-boundary contract** — domain types NEVER cross into `kayzen-app` (extends [[adr-0003-two-crate-workspace]]'s dependency rule to data shapes) | — |
| Data access | Queries read through the **EXISTING domain ports** (`HabitRepository`, `HabitBoardRepository`) against the same store. **NO dedicated ReadModel port** — a single trivial implementation would be YAGNI | — |
| Derived data | ALL derived display data (stats, minutes gained, motivational messages) is **computed on read** by pure stateless domain services — nothing stored. Progression *suggestions* are **no longer among them**: the `StabilityPolicy` is removed ([[adr-0008-goal-based-dose-user-paced-progression]]) | — |
| Statistics board | The future per-habit statistics board is **MORE QUERIES over the two dated histories** already in the aggregate (completions + step history) — days done, empty days, grow/lighten counts, minutes gained are *derivations, not projections*. The designer's own "tout le récap est dérivé" principle generalizes to every stat. Adaptive motivational messages = another stateless read-side policy | planned: `get-habit-stats/` |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| No CQRS (shared use cases, domain types returned to views) | Domain types would leak into Dioxus views across the crate boundary; per-screen DTOs are the required boundary contract |
| Full CQRS (projections + separate read store) | Requires the deferred production outbox dispatcher; eventual consistency = stale screens for a single local user; volume physically cannot justify it — ~20k dates after 10 years, microsecond recompute |
| Dedicated ReadModel port | Single trivial implementation over the same store — abstraction with no second implementation (YAGNI) |
| Physical `commands/` / `queries/` split now | Premature structure for a handful of use cases; deferred until the folder crowds |

## Escalation triggers (verbatim)

A MEASURED read-latency problem on device once real persistence + years of history exist, or multi-device sync entering the product. Escalation is additive: projections land behind the existing query handlers; UI contract unchanged; outbox already exists per [[adr-0001-validation-by-construction]].

## Consequences / Constraints

- **MUST**: every screen reads through a query use case returning its own DTO; `kayzen-app` never imports a domain type.
  - **The stakes of that MUST changed on 2026-07-26.** [[adr-0010-crate-boundary-trust-boundary]] makes this rule the reason the **crate boundary is the system's trust boundary**: because the app crate cannot fabricate a domain type, a query's entry point (primitives in, DTO out) is structurally the anticorruption layer, and parsing belongs there rather than in the view. Violating this MUST is therefore no longer only an architectural regression — it opens a second, unaudited door into the domain.
  - **Scope clarified 2026-07-27**: the MUST is about **domain types**. A view **naming** its query's output DTO (`HabitDetail` in `app/src/views/habit_detail.rs`) is the *prescribed* shape of this ADR, not an exception to it — DTOs exist precisely so the app needs no domain type. The boundary holds on what the core **accepts as input**, and no core entry point accepts a DTO. Full reasoning and the one-question test: [[adr-0010-crate-boundary-trust-boundary]]'s 2026-07-27 amendment.
- **Known tension, pre-existing and undecided (recorded 2026-07-27)**: `app/src/services/add_habit.rs` imports `HabitBoardError`, a **domain error type**, in production code — a literal tension with the MUST above. It predates the `adjust-goal` slice and was untouched by it; the slice's reviewers surfaced it rather than fixing it out of scope. **Neither a defect of that slice nor a settled exception** — the two candidate resolutions (a DTO-side error contract for the app service, or an explicit carve-out for error types crossing the boundary) both need a decision. Recorded here so it is not rediscovered as a fresh finding on every subsequent review.
- **MUST**: query use cases consume the existing ports (`HabitRepository`, `HabitBoardRepository`) — no new port for reads.
- **MUST NOT**: store any derived value (suggestion, stat, message) — recompute on read.
- **MAY**: duplicate fields across per-screen DTOs; introduce the physical `commands/`/`queries/` folder split when the use-case folder crowds (no new ADR needed).
- **Out of scope**: the query implementations themselves (next ticket), the statistics-board content/wording (functional lane), persistence technology.
