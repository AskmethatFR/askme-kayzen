---
id: "adr-0009-quality-gates"
type: "technical"
owner: "architect"
status: "draft"
updated: "2026-07-25"
relations:
  related:
    - "architecture-overview"
    - "lifecycle-backlog"
    - "feature-catalog"
  depends-on:
    - "adr-0003-two-crate-workspace"
answers:
  - "What proves a slice is done here, beyond a green test run?"
  - "Where do the Gherkin scenarios live and what executes them?"
  - "How does a scenario connect to the test that materializes it?"
  - "What does the mutation gate cover, and what is deliberately excluded from it?"
  - "Why does the mutation gate need committed work and a clean tree?"
decided_in:
  - "2026-07-25 gates cycle"
---

# ADR 0009 — Quality gates: spec-only Gherkin, diff-scoped mutation testing, one runner

> **Status `draft` — written by the orchestrator, awaiting Architect review.** The
> decisions below were taken while wiring the gates; they are recorded here rather
> than left in two script headers. The Architect owns this node and may reshape it.

> **One-liner**: Two gates guard a slice beyond `cargo test` — a **spec-only Gherkin gate** proving every specified behavior is materialized by a real test, and a **diff-scoped mutation gate** proving those tests discriminate. `scripts/check.sh` runs them alongside formatting, lints and a doc-anchor check.
> **Links**: [[architecture-overview]] (the two-crate shape both gates run over), [[lifecycle-backlog]] (the slices whose scenarios feed the first gate), [[feature-catalog]] (the delivered behavior the scenarios mirror).

## Context

A green `cargo test` says the tests pass. It says nothing about whether the tests
cover what was specified, nor whether they would notice if the production code
were wrong. Both questions had silently drifted: four delivered slices carried no
executable statement of intent, and `Display for HabitError` could be replaced by
`Ok(Default::default())` without a single test turning red.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Gherkin is **spec-only** | `.feature` files are valid Gherkin, parsed for structure and identity, **never executed**. No Cucumber-family runner, no step definitions, no glue. The reader is an agent or a developer, and both get the same comprehension from reading the file as from watching it run | `docs/functional/features/habit-management/` |
| Scenario ↔ test binding | A scenario is materialized by a real test carrying `// @scenario: <feature-id>/<Sn>` in the contiguous comment block directly above `#[test]`. The gate checks **both directions**: every non-`@wip` scenario has a test, and every anchor resolves to a live scenario | `scripts/check.sh` (scenarios gate) |
| `@wip` waives coverage only | A scenario specified before it is built carries `@wip`, exempting it from the coverage direction — never from the orphan or structural checks. Dropping the tag is part of a slice's Definition of Done | `docs/functional/lifecycle-backlog.md` |
| Mutation testing is **diff-scoped** | The gate mutates the committed diff, not the whole tree, so a change is judged on the tests it brought rather than on accumulated debt. Blocking for `fix-bug` and `new-feature`, advisory for `quick-change` | `scripts/mutation-gate.sh` |
| Mutation measures **decisions**, not implementations | The perimeter is the domain, the command use cases and the read queries (plus `LocalDate`, a pure value object sitting in `shared/`). Out: the Dioxus views, the app shell, the in-memory port implementations, and the boundary adapters delegating to `uuid` and `chrono`. Stated as an **include list**, so what is measured is readable rather than inferred from a pile of exclusions | `.cargo/mutants.toml` |
| A survivor on an adapter is a **use-case** finding | Testing an adapter directly proves the adapter matches itself. When a mutant survives outside the perimeter, the question is which use case failed to cover the concern. The two `UuidGenerator` survivors were really saying that no test pinned "every accepted request gets a freshly generated id" — that test belongs to `RequestHabit` | `core/src/habit_management/use_cases/request_habit.rs` |
| The gate reads **committed** work | The diff is `git diff <base-ref>...`, and cargo-mutants refuses a tree whose lines have moved under the patch. Commit the slice, then gate it. Default base-ref is `HEAD~1`: development happens on `master`, there is no base branch | `scripts/mutation-gate.sh` |
| Code anchors are checked | Every `path/to/file.rs` cited in `docs/` must exist. The knowledge graph's anchors are its only bridge from a domain term to the code; a dead one sends its reader nowhere, silently | `scripts/check.sh` (doc anchors gate) |
| One runner, no fail-fast | `scripts/check.sh` runs every gate even after one fails, and never invokes a gate as an `if` condition — bash suspends errexit inside a function called that way, which is how a runner swallows the failures it exists to surface | `scripts/check.sh` |

## Rejected alternatives

| Alternative | Why not |
|---|---|
| A real Cucumber/BDD runner | The whole value of an executable runner is bridging a non-technical stakeholder's spec to code. Nobody here collects that benefit, and the glue would have to track every wording change. The files stay valid Gherkin, so wiring a runner later costs only step definitions |
| Whole-tree mutation testing | Slow, and dominated by pre-existing survivors nobody will fix today. A slice would be judged on debt it did not create |
| Mutating the whole tree, adapters included | Tried first. It produces survivors whose only possible answer is a test written against an implementation — which proves nothing but that the implementation equals itself, and hides the fact that the concern was never handled where it belongs |
| Testing `UuidGenerator` directly | Two calls, two distinct non-empty ids: that test exercises the `uuid` crate, not this codebase. The real gap was one level up, in the use case |
| A pre-commit hook running everything | Every commit would pay for a mutation campaign. The gate belongs at the end of a slice, not on each commit of it |
| Fail-fast in the runner | One broken gate would hide the other four, turning one run into four |

## Consequences

- A slice is not done when its tests pass; it is done when its scenarios are no longer `@wip` and its diff mutates clean.
- Specifying a slice before building it is now cheap and visible: 18 scenarios across five backlog slices are written and waived, waiting for their tests.
- Running the mutation gate mid-slice, on a dirty tree, will fail with a line-mismatch error rather than a verdict. That is a workflow constraint, not a bug.
- Code outside the perimeter is not "trusted", it is **out of this instrument's reach**. An adapter's correctness is established by the use case that drives it, or not at all.
- Full campaign at the perimeter's adoption: 32 mutants, 25 caught, 0 survivors, 7 unviable.
