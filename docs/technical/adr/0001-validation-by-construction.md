---
id: "adr-0001-validation-by-construction"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-27"
relations:
  related:
    - "architecture-overview"
    - "adr-0009-quality-gates"
    - "adr-0010-crate-boundary-trust-boundary"
    - "adr-0008-goal-based-dose-user-paced-progression"
answers:
  - "Where do the habit business rules (title 1..=50 chars, initial duration <= 5 min) live?"
  - "Why does HabitBoardEvent carry value objects instead of primitives?"
  - "Why is Habit::new infallible and why does the event handler not re-validate?"
  - "Why is CreateHabitOnRequest an application handler and not a domain service?"
  - "Why is HabitId::new fallible when Habit::new must stay infallible?"
  - "Are the invariants this ADR mandates covered by the mutation gate?"
  - "May a VO method build a new instance without going through the validating constructor?"
  - "Why does Goal::grown/lightened saturate, and why can no test tell the difference?"
decided_in:
  - "LOCAL-1"
  - "LOCAL-2"
  - "2026-07-26 crate-boundary parsing cycle (notes below)"
  - "2026-07-27 slice 3 adjust-goal cycle (SEC-1: in-module derivations, human arbitrated for Security)"
---

# ADR 0001 — Validation by construction: VOs as single source of truth, events valid end-to-end

> **One-liner**: All habit invariants live in self-validating value objects; events carry those VOs, so anything published — and anything consumed — is valid by construction, and no layer ever re-validates.
> **Links**: [[architecture-overview]] — where this decision is applied.

## Context

The board-driven creation flow (LOCAL-1) splits habit creation into an emitting side (`HabitBoard::request_habit`) and a consuming side (`CreateHabitOnRequest`). Both sides touch the same business rules (`HabitTitle` 1..=50 chars, `InitialDuration` ≤ 5 min). Duplicating validation, or validating after emission, would either drift or let invalid events into the outbox.

*(LOCAL-2 renamed `HabitDescription` → `HabitTitle` and made the board return `HabitBoardError` — VO failures wrapped as `InvalidHabit(HabitError)`. The decision below is unchanged; anchors updated. See [[architecture-overview]] for the rename rationale.)*

## Decision

> **Anchor note (2026-07-25)**: the second VO was named `InitialDuration` when this
> decision was taken; `[[adr-0008-goal-based-dose-user-paced-progression]]` replaced it
> with `Goal`. The decision below is unchanged — only the VO's name and file moved,
> and the anchors point at the current file.

| Facet | Decision | Anchor |
|---|---|---|
| Single source of truth | The two business rules live ONLY in the VO constructors (`HabitTitle`, `InitialDuration`), moved out of `Habit::new` | `core/src/habit_management/domain/habit_title.rs`, `core/src/habit_management/domain/goal.rs` |
| Validate before emission | `HabitBoard::request_habit` builds the VOs and returns `Result<HabitBoardEvent, HabitBoardError>` — an event cannot exist unless validation passed | `core/src/habit_management/domain/habit_board.rs` |
| Events carry VOs | `HabitBoardEvent::HabitRequested` holds the VOs, not primitives — validity is enforced by the type system end-to-end | `core/src/habit_management/domain/habit_board_event.rs` |
| Parse, don't validate | `Habit::new(HabitId, HabitTitle, InitialDuration) -> Habit` is infallible; the domain speaks `HabitId`, never raw `String` | `core/src/habit_management/domain/habit.rs` |
| Event is a fact | `CreateHabitOnRequest` is an **application** event handler (not a domain service): it consumes the event without re-validation and calls `HabitRepository::save` | `core/src/habit_management/use_cases/create_habit_on_request.rs` |

> **Note (2026-07-26) — the second half of the "Parse, don't validate" row was aspirational until now.**
> `HabitId` was obtained through an infallible `impl From<&str>`, so a raw URL
> segment *did* enter the domain as an unbounded `String`.
> [[adr-0010-crate-boundary-trust-boundary]] closes that: `From` is deleted and
> `HabitId::new(&str) -> Result<HabitId, HabitError>` is the single door, parsing
> at the crate boundary. This **confirms** the decision above rather than
> amending it — the aggregate root `Habit::new` stays infallible (the MUST NOT
> below is intact); it is the **VO** constructor that became fallible, which is
> exactly where this ADR says an invariant belongs. `HabitError::IdLength { min,
> max }` joins `TitleLength` in the same error family.

> **Warning (2026-07-26) — this ADR's invariants are invisible to the mutation
> gate.** cargo-mutants hard-skips any function literally named `new`, and this
> ADR mandates that *every* invariant live in a VO constructor — all six of which
> are named `new`. A regression in any of them still passes the gate as a clean
> zero-survivor run. Until the deferred `new` → `parse` rename is decided by the
> human, the invariants defended here are held by **hand-written boundary tests
> alone**. Full evidence, measurements and the deferred option:
> [[adr-0009-quality-gates]].

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Shared validation function called by both board and `Habit::new` | Rule written once but *executed* twice — the consumer still distrusts the event; type system enforces nothing |
| Board constructs a throwaway `Habit` to validate | Hack; couples the board to `Habit`'s lifecycle for a side effect |
| `CreateHabitOnRequest` as a domain service | Injecting a repository port into a domain service is a smell; with all invariants in the VOs, the handler has no domain logic left — it is orchestration |

## Consequences / Constraints

- **MUST**: any new habit invariant goes into a VO constructor (or a new VO) — never into `Habit::new`, never into a handler.
- **MUST**: any new domain event carries VOs/domain types, never raw primitives.
- **MUST NOT**: re-validate event payloads downstream; a consumed event is a fact.
- **MUST NOT**: make `Habit::new` fallible again. (A *VO* constructor being fallible is the opposite case — that is precisely where an invariant belongs; see the 2026-07-26 note above.)
- **MUST NOT**: give any VO a second, infallible construction path (`From`, `Deserialize`, a public field) that bypasses its validating constructor — [[adr-0010-crate-boundary-trust-boundary]]. **Refined 2026-07-27**: an *in-module derivation* is the one admitted shape — see the amendment below for the three conditions it must meet.
- **Out of scope**: cross-habit invariants — settled since LOCAL-2 by [[adr-0002-habitboard-stateful-aggregate]].

---

## Amendment — 2026-07-27, slice 3 `adjust-goal`: **in-module derivations**, and why they do not bypass the door

Slice 3 introduced `Goal::grown()` and `Goal::lightened()` — methods returning a new `Goal`
built with the tuple constructor `Goal(...)`, **not** routed through the fallible
`Goal::new`. Security (T1) correctly read that as a second infallible construction path and
raised it against the MUST NOT above. This amendment records the arbitration and the general
rule it produced, because the shape will recur on every future VO with arithmetic.

### The finding and its arbitration

Security's T1: `Goal(self.0 + 1)` bypasses the validating constructor, and with no
`[profile.release] overflow-checks`, a release build would wrap `u32::MAX` to `Goal(0)` —
**breaking the floor invariant this ADR exists to hold**. Dev-B argued against changing it:
the risk is unreachable in practice, and the guard retires two measurable mutants.
**The human arbitrated for Security.** `grown()` became `saturating_add(1)`, and the decision
was then **extended by symmetry** to `lightened()` — which Security's T2 pass showed was not
symbolic: on the pathological `Goal(0)`, a bare `- 1` wraps to `u32::MAX`, which `.max(MIN)`
passes straight through. A floor violation converted into a ceiling jump. The guard is
genuinely load-bearing.

### The rule this settles

A method may construct its own VO **without** going through the validating constructor only
when all three hold:

| Condition | For `Goal::grown` / `Goal::lightened` |
|---|---|
| It lives **inside the VO's own module** — the privacy boundary that owns the field | Both are in `core/src/habit_management/domain/goal.rs` |
| It is a **total function of an already-valid instance** — no external input | Both take only `&self` |
| It **preserves the invariant by construction**, on every input including pathological ones | `saturating_add` cannot wrap; `saturating_sub(1).max(MIN)` cannot go below `MIN` |

Fail any one, and it is a bypass door, not a derivation.

### The resulting property — **every reachable `Goal` is `>= MIN`, by induction**

- **Base case**: `Goal::new` is the only entry from outside, and it rejects `< MIN`.
- **Inductive step**: the only other two construction sites are `grown()` and `lightened()`,
  both of which take an already-valid `Goal` and preserve `>= MIN`.
- **Closure**: the tuple field is private, all three construction sites are in `goal.rs`, and
  there is **no `Default`, no `From`, no `Deserialize`**. There is no fourth site.

Adding any of those three derives, or a fourth construction site outside `goal.rs`, breaks
the induction and reopens this amendment.

### Recorded honestly: `saturating_sub` is proof-carrying, not tested

No test can discriminate `saturating_sub(1)` from `- 1` in `lightened()`, because reaching
the difference requires a `Goal(0)` that the induction above proves cannot exist. It is a
**proof-carrying line**, held by the argument in this node rather than by an assertion —
the same category as the invariants [[adr-0009-quality-gates]] records as invisible to the
mutation gate. Do not "clean it up" for lack of a covering test; the absence of the test is
the point.
