---
id: "adr-0006-cqrs-light"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-16"
relations:
  related:
    - "architecture-overview"
    - "habit-progression-study"
  depends-on:
    - "adr-0003-two-crate-workspace"
    - "adr-0005-progression-suggestion-policy"
answers:
  - "How is the read side of habit_management structured — CQRS or shared use cases?"
  - "Do domain types cross the crate boundary into kayzen-app, or do queries return DTOs?"
  - "Do queries get their own ReadModel port, projections, or a separate read store?"
  - "Where will the future per-habit statistics board get its data from?"
  - "What would justify escalating from CQRS-light to full CQRS with projections?"
decided_in:
  - "LOCAL-6"
---

# ADR 0006 — CQRS-light for the read side of habit_management (query handlers + per-screen DTOs, no projections)

> **One-liner**: The read side is **CQRS-light** — dedicated query use cases (one `snake_case` module folder per query under `core/src/habit_management/use_cases/`) returning **per-screen DTOs** that are the crate-boundary contract; queries read through the **existing domain ports** against the same store, and all derived display data (suggestions, stats, messages) is **computed on read** by stateless policies. No projections, no read store, no ReadModel port.
> **Links**: [[architecture-overview]] (the workspace shape this applies to), [[adr-0003-two-crate-workspace]] (the compiler-enforced boundary the DTOs serve), [[adr-0005-progression-suggestion-policy]] (the recompute-never-store principle this generalizes), [[habit-progression-study]] (evidence base behind the derived-data model).
>
> **Timing note — decision capture ahead of implementation**: the read query use cases are **not yet built** (the first read ticket is next). This ADR pins the shape *before* implementation so the tech spec inherits it instead of re-deciding; anchors below are planned shapes, not existing files. Human-approved 2026-07-16.

## Context

The lifecycle backlog ([[habit-progression-study]], `lifecycle-backlog`) requires screens that *read*: the board listing, later a habit detail, and a **per-habit statistics board** (days done, empty days — never framed "failed" —, grow/lighten counts, minutes gained, adaptive motivational non-guilt messages). ADR-0005 already fixed that suggestions are DTO fields recomputed on every read. The open fork was how to structure that read side: shared use cases (no CQRS), CQRS-light, or full CQRS with projections.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Handler split | **Query handlers (read use cases) separate from command use cases.** One `snake_case` module folder per query in the existing `core/src/habit_management/use_cases/` convention: `list_board_habits/`, later `get_habit_detail/`, `get_habit_stats/`. Module/file names are `snake_case` per Rust RFC 430 — no `#[path]` remapping. A physical `commands/` / `queries/` split is **deferred** until the folder crowds | planned: `core/src/habit_management/use_cases/list_board_habits/` |
| Read models | **Per-screen DTOs owned by their query use case** (`HabitSummary`, `HabitDetail`, later `HabitStats`). Duplication across DTOs is acceptable — one screen = one read model, no god read model | planned: DTO next to its query |
| Crate boundary | DTOs are the **crate-boundary contract** — domain types NEVER cross into `kayzen-app` (extends [[adr-0003-two-crate-workspace]]'s dependency rule to data shapes; [[adr-0005-progression-suggestion-policy]]'s DTO fields already presuppose it) | — |
| Data access | Queries read through the **EXISTING domain ports** (`HabitRepository`, `HabitBoardRepository`) against the same store. **NO dedicated ReadModel port** — a single trivial implementation would be YAGNI | — |
| Derived data | Suggestions and ALL derived display data are **computed on read** by pure stateless domain services (`StabilityPolicy` et al.) — nothing stored, per [[adr-0005-progression-suggestion-policy]] | — |
| Statistics board | The future per-habit statistics board is **MORE QUERIES over the two dated histories** already in the aggregate (completions + step history) — days done, empty days, grow/lighten counts, minutes gained are *derivations, not projections*. The designer's own "tout le récap est dérivé" principle generalizes to every stat. Adaptive motivational messages = another stateless read-side policy | planned: `get-habit-stats/` |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| No CQRS (shared use cases, domain types returned to views) | Domain types would leak into Dioxus views across the crate boundary; [[adr-0005-progression-suggestion-policy]]'s DTO fields (`growth_suggested` / `anchor_suggested`) already foreclose it |
| Full CQRS (projections + separate read store) | Requires the deferred production outbox dispatcher; eventual consistency = stale screens for a single local user; volume physically cannot justify it — ~20k dates after 10 years, microsecond recompute |
| Dedicated ReadModel port | Single trivial implementation over the same store — abstraction with no second implementation (YAGNI) |
| Physical `commands/` / `queries/` split now | Premature structure for a handful of use cases; deferred until the folder crowds |

## Escalation triggers (verbatim)

A MEASURED read-latency problem on device once real persistence + years of history exist, or multi-device sync entering the product. Escalation is additive: projections land behind the existing query handlers; UI contract unchanged; outbox already exists per [[adr-0001-validation-by-construction]].

## Consequences / Constraints

- **MUST**: every screen reads through a query use case returning its own DTO; `kayzen-app` never imports a domain type.
- **MUST**: query use cases consume the existing ports (`HabitRepository`, `HabitBoardRepository`) — no new port for reads.
- **MUST NOT**: store any derived value (suggestion, stat, message) — recompute on read.
- **MAY**: duplicate fields across per-screen DTOs; introduce the physical `commands/`/`queries/` folder split when the use-case folder crowds (no new ADR needed).
- **Out of scope**: the query implementations themselves (next ticket), the statistics-board content/wording (functional lane), persistence technology.
