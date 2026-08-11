---
id: "adr-0006-cqrs-light"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-08-11"
relations:
  related:
    - "architecture-overview"
    - "habit-progression-study"
    - "adr-0008-goal-based-dose-user-paced-progression"
    - "adr-0010-crate-boundary-trust-boundary"
    - "adr-0011-one-public-method-per-use-case"
    - "adr-0007-habit-lifecycle-aggregate"
    - "adr-0009-quality-gates"
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
  - "One screen shows two lists — one query returning both, or two queries?"
  - "Where does a rule that partitions a screen's content live: the query or the view?"
  - "How does a domain enum reach a view — as itself, as a bool, or as a DTO-side enum?"
  - "When does a new screen get its own query rather than a field on an existing DTO?"
  - "Is a count shown on one screen over data owned by another screen stored, or derived?"
decided_in:
  - "LOCAL-6"
  - "2026-07-27 slice 3 adjust-goal cycle (DTO-naming scope, HabitBoardError tension recorded)"
  - "2026-08-06 slice 5 pause-resume cycle (TodayHabits two-list DTO, DTO-side enums)"
  - "2026-08-11 slice 6 anchor-habit cycle (sibling screen ⇒ sibling query; anchored_count derived)"
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
| Read models | **Per-screen DTOs owned by their query use case** (`HabitSummary`, `HabitDetail`, later `HabitStats`). Duplication across DTOs is acceptable — one screen = one read model, no god read model. **Amended 2026-08-06**: "per-screen" is literal — a screen showing two zones gets **one DTO carrying both lists** (`TodayHabits { active, paused }`), not two queries; and a domain enum reaching the view crosses as a **DTO-side enum**, never a `bool`. See the amendment below | `core/src/habit_management/queries/list_board_habits.rs`, `core/src/habit_management/queries/get_habit_detail.rs` |
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
- **MUST**: keep a rule that decides *what a screen shows* inside the query, never in the view — the view arranges what it is handed (2026-08-06).
- **MUST**: expose a domain enum to the app as a **DTO-side enum** declared next to its query, mapped by an exhaustive `match` — never the domain type, never a `bool` (2026-08-06; the `bool` half is [[adr-0007-habit-lifecycle-aggregate]] AD-2).
- **MAY**: duplicate fields across per-screen DTOs; introduce the physical `commands/`/`queries/` folder split when the use-case folder crowds (no new ADR needed).
- **Out of scope**: the query implementations themselves (next ticket), the statistics-board content/wording (functional lane), persistence technology.

---

## Amendment — 2026-08-06, slice 5 `pause-resume`: the two-list per-screen DTO, and DTO-side enums

Slice 5 gave the Today screen a second zone (« En pause · aucune pression »). That is the
first time one screen carries two lists, and the first time a domain enum needs to reach a
view. Both are answered by this node's existing shape rather than beside it — hence an
amendment.

### The prescribed shape: one query, one DTO, two lists

`ListBoardHabits::handle()` no longer returns a flat `Vec<HabitSummary>`. It returns

```rust
pub struct TodayHabits {
    pub active: Vec<HabitSummary>,
    pub paused: Vec<PausedHabit>,
}
```

built in a single pass over `repository.all()` with an **exhaustive `match` on
`habit.state()`**. `PausedHabit { id, title }` deliberately carries neither goal nor
completion status: a pause carries no daily pressure, so the DTO carries none either.

**This is the first DTO in the codebase to carry two lists for one screen. It is the
prescribed shape — slices 6, 7 and 8 must not re-decide it.** When a screen grows a zone,
the query grows a field; it does not spawn a sibling query.

Two arguments carry it:

1. **The partitioning IS the business rule.** *« Une habitude en pause quitte la liste du
   jour »* is not a layout preference — it is the rule the slice exists to deliver. Rules
   belong where they can be measured: `queries/**` is inside `.cargo/mutants.toml`'s
   `examine_globs`; `app/src/views/**` is excluded **by design** ([[adr-0009-quality-gates]]).
   A rule placed in the view is a rule no instrument in this repo can reach.
2. **The tally becomes correct by construction.** Today's « X sur Y » counts active habits.
   With the split in the query, `total = active.len()` — it cannot drift. With a
   `paused: bool` flag and view-side filtering, the tally is correct only as long as every
   future reader remembers to exclude the paused ones. Discipline versus arithmetic; the
   arithmetic wins.

| Rejected | Why |
|---|---|
| `paused: bool` on `HabitSummary`, view filters | Relocates a business rule into the one layer the mutation gate cannot see, and makes the « X sur Y » tally silently wrong the day someone forgets the filter |
| A filtered `ListBoardHabits` + a separate `ListPausedHabits` | Two `repository.all()` passes to render one screen; a query no other screen wants; and the composition of the two zones falls back into the view — the very thing point 1 refuses. Sibling queries are for **sibling screens**, not for zones of one screen |
| A `Vec<enum HabitRow { Active(..), Paused(..) }>` | Preserves interleaving nobody wants and forces the view to re-partition — the view work returns, plus a match |

### DTO-side enums, and how they are mapped

`HabitDetail` gains `state: HabitState`, an enum declared next to its query
(`core/src/habit_management/queries/get_habit_detail.rs`) and mapped from the domain's
`LifecycleState` by an exhaustive `match`. The "never the domain type" half is this node's
existing MUST, reinforced by [[adr-0010-crate-boundary-trust-boundary]]; the "never a
`bool`" half is the decision, and its reasoning lives in
[[adr-0007-habit-lifecycle-aggregate]] AD-2 (a second bool arrives in slice 6 and makes an
impossible combination representable, right where it renders).

The mapping `match` is the point, not overhead: it is what makes slice 6's new variant a
**compile error at every site** instead of a silent `false`.

### The reach-of-the-gate corollary

Both mutants generated for this cycle's read side were classed `unviable`, so the
partition and the mapping received **zero viable mutants** — the gate proved nothing about
either. Each is covered by one deliberate test instead. Recorded in full as L4 of
[[adr-0009-quality-gates]]. The lesson for this node: *placing a rule inside the gate's
perimeter is necessary, not sufficient* — the perimeter is where the instrument is
allowed to look, not where it is guaranteed to see.

---

## Amendment — 2026-08-11, slice 6 `anchor-habit`: the other side of "per-screen", and a derived count

Slice 5 established that a screen's **zones** stay inside one query. Slice 6 exercises the
complementary half — a genuinely **new screen** — and confirms rather than qualifies the
rule. Recorded so the two are read together and neither is over-applied.

### A sibling **screen** gets a sibling query

*Ancrées* is a screen of the designer's six, not a zone of *Aujourd'hui*. It therefore gets
its own query and its own DTO: `ListAnchoredHabits -> Vec<AnchoredHabit { title }>`
(`core/src/habit_management/queries/list_anchored_habits.rs`). No `Clock` — nothing dated
is shown — and deliberately **just the title**: the screen names what has become natural,
and carries no daily pressure, so the DTO carries no goal and no completion status (the
same reasoning that emptied `PausedHabit`).

This is the sentence slice 5's amendment ended on, applied: *sibling queries are for
sibling screens, not for zones of one screen*. The rule reads in both directions, and
"per-screen" is the criterion in both.

| Rejected | Why |
|---|---|
| A third list on `TodayHabits` (`anchored: Vec<...>`) | Anchored habits are not part of the day — the whole slice exists to remove them from it. It would make every *Aujourd'hui* read pay for a list that screen never renders |
| Reusing `HabitSummary` for the anchored rows | Carries a goal and a done-today flag onto a screen where neither has meaning; the DTO would describe a pressure the product deliberately removed |

### `anchored_count` is **derived on read**, and it lives in the query

*Aujourd'hui* offers « Mes habitudes ancrées · N » only when `N >= 1`, so it needs a count
over data it does not display. `TodayHabits` gains `anchored_count: usize`, tallied in the
**same single pass** over `repository.all()` that already partitions active from paused
(`core/src/habit_management/queries/list_board_habits.rs`) — one extra arm on the `match`
that the new variant forced open anyway.

No stored counter, no second pass, no second query for a number: this node's founding rule
(*all derived display data is computed on read*) covers it, and it is recorded here only
because a counter is the classic place a projection quietly appears.

The placement matters for the same reason the slice-5 partition did. The condition
*« N >= 1 »* is a rendering choice and stays in the view; the **tally itself** is state, and
state that lives in `queries/**` is inside `.cargo/mutants.toml`'s perimeter while the view
is excluded by design ([[adr-0009-quality-gates]]). Filtering anchored habits out of both
`active` and `paused` in that same pass is what keeps *Aujourd'hui*'s « X sur Y » — which
is `active.len()` — correct by construction rather than by discipline.
