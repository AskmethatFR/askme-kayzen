---
id: "adr-0009-quality-gates"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-26"
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
  - "Where do the gate implementations live, and what happens on a machine that lacks them?"
  - "Can a test in the app crate materialize a scenario, or only a core test?"
decided_in:
  - "2026-07-25 gates cycle"
  - "2026-07-26 architect ratification (slice 3 adjust-goal cycle)"
---

# ADR 0009 — Quality gates: spec-only Gherkin, diff-scoped mutation testing, one runner

> **Ratified 2026-07-26.** Drafted by the orchestrator while wiring the gates,
> then reviewed against the code it describes (`scripts/check.sh`,
> `scripts/mutation-gate.sh`, `.cargo/mutants.toml`) and adopted by the Architect,
> who owns this node. Every decision below is retained; the review added the two
> facets the draft left implicit — **where the gate implementations live** and
> **which test roots the scenario gate scans** — and tightened the doc-anchor
> facet to what the check actually greps.

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
| **Both crates** carry scenarios | The gate scans `core/src` **and** `app/src` for anchors. A scenario about what the *user sees* — an affordance being offered, a screen rendering a value — is legitimately materialized by an SSR render test in `kayzen-app`; only behavior scenarios belong in the core. The scenario decides its own level, the crate does not | `scripts/check.sh` (`--tests-root core/src --tests-root app/src`) |
| Mutation testing is **diff-scoped** | The gate mutates the committed diff, not the whole tree, so a change is judged on the tests it brought rather than on accumulated debt. Blocking for `fix-bug` and `new-feature`, advisory for `quick-change` | `scripts/mutation-gate.sh` |
| Mutation measures **decisions**, not implementations | The perimeter is the domain, the command use cases and the read queries (plus `LocalDate`, a pure value object sitting in `shared/`). Out: the Dioxus views, the app shell, the in-memory port implementations, and the boundary adapters delegating to `uuid` and `chrono`. Stated as an **include list**, so what is measured is readable rather than inferred from a pile of exclusions | `.cargo/mutants.toml` |
| A survivor on an adapter is a **use-case** finding | Testing an adapter directly proves the adapter matches itself. When a mutant survives outside the perimeter, the question is which use case failed to cover the concern. The two `UuidGenerator` survivors were really saying that no test pinned "every accepted request gets a freshly generated id" — that test belongs to `RequestHabit` | `core/src/habit_management/use_cases/request_habit.rs` |
| The gate reads **committed** work | The diff is `git diff <base-ref>...`, and cargo-mutants refuses a tree whose lines have moved under the patch. Commit the slice, then gate it. Default base-ref is `HEAD~1`: development happens on `master`, there is no base branch | `scripts/mutation-gate.sh` |
| Code anchors are checked | Every backticked Rust source path cited in `docs/` — `` `core/src/…rs` ``, `` `app/src/…rs` ``, or a bare `` `src/…rs` `` — must exist. The knowledge graph's anchors are its only bridge from a domain term to the code; a dead one sends its reader nowhere, silently. A *planned* shape is therefore written unanchored (`domain/step_history.rs`, no `src/` prefix) until the file lands — the check deliberately guards resolvable anchors, not intentions | `scripts/check.sh` (doc anchors gate) |
| One runner, no fail-fast | `scripts/check.sh` runs every gate even after one fails, and never invokes a gate as an `if` condition — bash suspends errexit inside a function called that way, which is how a runner swallows the failures it exists to surface | `scripts/check.sh` |
| Gate implementations are **shared, not vendored** | Both gates are thin repo-local wrappers over the operator's shared tooling (`$CLAUDE_HOME/lib/scenario_audit.py`, `$CLAUDE_HOME/lib/mutation_gate.py`); this repo owns the *policy* — the perimeter, the test roots, the base ref — and not the instrument. A missing implementation exits **2** and is recorded as a **failed** gate, never skipped: a gate that cannot run must read as red, because a green run is a claim about what was measured | `scripts/check.sh`, `scripts/mutation-gate.sh` |

## Rejected alternatives

| Alternative | Why not |
|---|---|
| A real Cucumber/BDD runner | The whole value of an executable runner is bridging a non-technical stakeholder's spec to code. Nobody here collects that benefit, and the glue would have to track every wording change. The files stay valid Gherkin, so wiring a runner later costs only step definitions |
| Whole-tree mutation testing | Slow, and dominated by pre-existing survivors nobody will fix today. A slice would be judged on debt it did not create |
| Mutating the whole tree, adapters included | Tried first. It produces survivors whose only possible answer is a test written against an implementation — which proves nothing but that the implementation equals itself, and hides the fact that the concern was never handled where it belongs |
| Testing `UuidGenerator` directly | Two calls, two distinct non-empty ids: that test exercises the `uuid` crate, not this codebase. The real gap was one level up, in the use case |
| A pre-commit hook running everything | Every commit would pay for a mutation campaign. The gate belongs at the end of a slice, not on each commit of it |
| Fail-fast in the runner | One broken gate would hide the other four, turning one run into four |
| Vendoring the two Python gates into `scripts/` | They are general instruments, not project decisions: a fix to the anchor-attachment parser would then have to be copied back by hand, and this repo would slowly own a fork of a tool it did not write. What belongs here is the *policy* — perimeter, test roots, base ref — and that is exactly what the two wrappers and `.cargo/mutants.toml` hold |
| Skipping a gate whose implementation is missing | It would turn "not measured" into "green", which is the one failure mode a gate exists to prevent. Exit 2 is reported as a failure |

## Consequences

- A slice is not done when its tests pass; it is done when its scenarios are no longer `@wip` and its diff mutates clean.
- Specifying a slice before building it is now cheap and visible: 18 scenarios across five backlog slices are written and waived, waiting for their tests.
- Running the mutation gate mid-slice, on a dirty tree, will fail with a line-mismatch error rather than a verdict. That is a workflow constraint, not a bug.
- Code outside the perimeter is not "trusted", it is **out of this instrument's reach**. An adapter's correctness is established by the use case that drives it, or not at all.
- Full campaign at the perimeter's adoption: 32 mutants, 25 caught, 0 survivors, 7 unviable.
- The gates need the operator's toolbox on the machine (`CLAUDE_HOME`, default `~/.claude`). A fresh clone without it gets two red gates and a path in the message, not a false green.
- A scenario may be pinned from either crate, so a slice's user-observable half is provable: "both gestures are offered" is an assertion about a rendered screen, and it counts.
