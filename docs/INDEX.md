# Project Documentation Graph — INDEX

> **How to use this file**
> - **Human / AI looking for an answer**: scan the table, find the node whose Title/Type matches, open its `Path`, then follow its `[[links]]` edges.
> - **Agent about to implement / decide**: this index + the relevant node(s) are the **source of truth**. Do not re-decide what a node already settles. If no node covers your question, the graph is *silent* — decide, then add/extend a node (Architect for technical, PM for functional) and a row here in the same cycle.
> - **Owners**: `architect` owns every `technical` row, `pm` owns every `functional` row. Nobody edits the other lane's nodes without routing through that owner.

## Conventions

- **ID**: kebab-case, unique, stable (rename = breaking the graph). Referenced elsewhere as `[[id]]`.
- **Type**: `technical` (Architect-owned) or `functional` (PM-owned).
- **Status**: `draft` | `current` | `superseded` | `deprecated`.
- **Updated**: ISO date `YYYY-MM-DD` of last substantive change.
- **Links**: `[[id]]` of every node this one points to (dependencies, related decisions, feature↔ADR). Bidirectional links are encouraged but only one direction is required to keep the graph connected.

## Technical nodes (owner: architect)

| ID | Title | Status | Updated | Links | Path |
|---|---|---|---|---|---|
| `adr-0001-validation-by-construction` | Validation by construction: VOs as single source of truth, events valid end-to-end | current | 2026-07-16 | `[[architecture-overview]]`, `[[adr-0002-habitboard-stateful-aggregate]]` | `docs/technical/adr/0001-validation-by-construction.md` |
| `adr-0002-habitboard-stateful-aggregate` | HabitBoard becomes a persisted stateful aggregate (capacity + uniqueness invariants) | current | 2026-07-16 | `[[architecture-overview]]`, `[[adr-0001-validation-by-construction]]` | `docs/technical/adr/0002-habitboard-stateful-aggregate.md` |
| `adr-0003-two-crate-workspace` | Two-crate workspace: kayzen-core (pure domain) / kayzen-app (Dioxus shell), compiler-enforced dependency rule | current | 2026-07-16 | `[[architecture-overview]]`, `[[adr-0001-validation-by-construction]]`, `[[adr-0002-habitboard-stateful-aggregate]]` | `docs/technical/adr/0003-two-crate-workspace.md` |
| `architecture-overview` | Habit Management — workspace split (core/app), bounded context, layers, board-driven creation flow (stateful board) | current | 2026-07-16 | `[[adr-0001-validation-by-construction]]`, `[[adr-0002-habitboard-stateful-aggregate]]`, `[[adr-0003-two-crate-workspace]]` | `docs/technical/architecture.md` |
| `design-modele-donnees` | Kaizen design — data model (Rust structs: Habit, stage, gestures, journal) | current | 2026-07-16 | `[[design-overview]]`, `[[design-gestes-kaizen]]`, `[[architecture-overview]]` | `docs/technical/design/02-modele-donnees.md` |
| `design-setup-rust-dioxus` | Kaizen design — Rust project setup tutorial (Dioxus, targets web/desktop/mobile) | current | 2026-07-16 | `[[design-overview]]`, `[[design-modele-donnees]]` | `docs/technical/design/06-mise-en-place-rust.md` |

## Functional nodes (owner: pm)

| ID | Title | Status | Updated | Links | Path |
|---|---|---|---|---|---|
| `feature-catalog` | Habit Management — features, business rules (max 5, no duplicate), acceptance | current | 2026-07-16 | `[[glossary]]`, `[[architecture-overview]]`, `[[adr-0001-validation-by-construction]]`, `[[design-overview]]` | `docs/functional/feature-catalog.md` |
| `glossary` | Ubiquitous language — Habit, Habit Board, Title, Duplicate, HabitRequested, Outbox | current | 2026-07-16 | `[[feature-catalog]]` | `docs/functional/glossary.md` |
| `design-overview` | Kaizen design — product overview from designer (habit lifecycle, scope, doc map) | current | 2026-07-16 | `[[design-principes-kaizen]]`, `[[design-ecrans]]`, `[[design-gestes-kaizen]]`, `[[design-style-graphique]]`, `[[design-modele-donnees]]`, `[[design-setup-rust-dioxus]]`, `[[feature-catalog]]` | `docs/functional/design/README.md` |
| `design-principes-kaizen` | Kaizen design — non-negotiable Kaizen rules (small steps, no streak, no guilt) | current | 2026-07-16 | `[[design-overview]]` | `docs/functional/design/01-principes.md` |
| `design-ecrans` | Kaizen design — screens, gestures and transitions (6 prototype screens) | current | 2026-07-16 | `[[design-overview]]`, `[[design-style-graphique]]` | `docs/functional/design/03-ecrans.md` |
| `design-gestes-kaizen` | Kaizen design — the seven Kaizen gestures + the loop | current | 2026-07-16 | `[[design-overview]]`, `[[design-ecrans]]` | `docs/functional/design/04-gestes-kaizen.md` |
| `design-style-graphique` | Kaizen design — colors, typography, icons, components + screen captures | current | 2026-07-16 | `[[design-overview]]` | `docs/functional/design/05-style-graphique.md` |

## Graph health (maintained by the owners at end of each cycle)

- [x] Every node file has a row here; every row points to an existing file.
- [x] No dangling `[[id]]` edge (every referenced ID exists as a row).
- [x] No orphan node (every node is reachable from `architecture-overview` or `feature-catalog`).
- [x] `Updated` reflects the last cycle that touched the node.
