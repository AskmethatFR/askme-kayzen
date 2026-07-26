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
    - "adr-0001-validation-by-construction"
    - "adr-0010-crate-boundary-trust-boundary"
  depends-on:
    - "adr-0003-two-crate-workspace"
answers:
  - "What proves a slice is done here, beyond a green test run?"
  - "Where do the Gherkin scenarios live and what executes them?"
  - "How does a scenario connect to the test that materializes it?"
  - "What does the mutation gate cover, and what is deliberately excluded from it?"
  - "Which base-ref does the mutation gate measure from, and why is it never defaulted?"
  - "Why does the mutation gate need committed work and a clean tree?"
  - "What can the mutation gate structurally NOT measure in this codebase?"
  - "Why are the domain's validating constructors invisible to mutation testing?"
  - "Why has renaming `new` to `parse` not been done, and who decides it?"
  - "Where do the gate implementations live, and what happens on a machine that lacks them?"
  - "Can a test in the app crate materialize a scenario, or only a core test?"
decided_in:
  - "2026-07-25 gates cycle"
  - "2026-07-26 architect ratification (slice 3 adjust-goal cycle)"
  - "2026-07-26 base-ref + new()-blind-spot cycle (amendment below)"
---

# ADR 0009 — Quality gates: spec-only Gherkin, diff-scoped mutation testing, one runner

> **Ratified 2026-07-26.** Drafted by the orchestrator while wiring the gates,
> then reviewed against the code it describes (`scripts/check.sh`,
> `scripts/mutation-gate.sh`, `.cargo/mutants.toml`) and adopted by the Architect,
> who owns this node. Every decision below is retained; the review added the two
> facets the draft left implicit — **where the gate implementations live** and
> **which test roots the scenario gate scans** — and tightened the doc-anchor
> facet to what the check actually greps.

> **Amended 2026-07-26, the same day it was ratified.** Two facts this node
> asserted were wrong or missing, and both were corrected in code before the
> node: (1) the mutation gate's **default base-ref `HEAD~1` was deleted** — it
> silently under-measured every properly split TDD slice, which is to say every
> slice this protocol permits; (2) cargo-mutants **hard-skips any function
> literally named `new`**, so the domain's validating constructors — where
> [[adr-0001-validation-by-construction]] puts every invariant — generate zero
> mutants, and this node presented the gate as a discrimination guarantee over
> exactly those constructors. Amended in place rather than superseded: both
> corrections are facets of the same settled question ("what does the mutation
> gate measure"), and splitting them into a second ADR would force a reader to
> reassemble the gate's real coverage from two nodes. The audit trail a
> supersession would have given is carried by the dated rows below.

> **One-liner**: Two gates guard a slice beyond `cargo test` — a **spec-only Gherkin gate** proving every specified behavior is materialized by a real test, and a **diff-scoped mutation gate** proving those tests discriminate *within the reach of the instrument*. `scripts/check.sh` runs them alongside formatting, lints, a doc-anchor check and one advisory blind-spot listing.
> **Links**: [[architecture-overview]] (the two-crate shape both gates run over), [[lifecycle-backlog]] (the slices whose scenarios feed the first gate), [[feature-catalog]] (the delivered behavior the scenarios mirror), [[adr-0001-validation-by-construction]] (the decision the `fn new` blind spot leaves unmeasured), [[adr-0010-crate-boundary-trust-boundary]] (a boundary constructor written to the exact boundary *because* the gate cannot see it).

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
| **Blind spot: cargo-mutants cannot see any `fn new`** (recorded 2026-07-26) | cargo-mutants 27.1.0 **hard-skips any method literally named `new`**, before mutants are generated. Read from the crate's own source (visit.rs, upstream — deliberately left unanchored: it is not a file of this repo, and the doc-anchor gate resolves repo paths only): `if fn_sig_excluded(&i.sig) \|\| attrs_excluded(&i.attrs) \|\| i.sig.ident == "new" \|\| block_is_empty(&i.block) { return; }` — *"Don't look inside constructors (called `new`) because there's often no good alternative."* The skip is **name-based, visibility-agnostic and non-configurable**: no CLI flag or config key overrides it (`skip_calls` filters *calls to* a function, not its definition, and cannot resurrect mutants that were never generated) | `.cargo/mutants.toml` |
| Why that blind spot is an **ADR-level** fact, not a tooling footnote | [[adr-0001-validation-by-construction]] requires **every** domain invariant to live in a value-object constructor, and those constructors are named `new`. So the gate this node presents as the project's discrimination guarantee **structurally cannot measure the project's most load-bearing decision**. Six such constructors exist today: `Habit::new`, `CompletionHistory::new`, `Goal::new`, `HabitId::new`, `HabitTitle::new`, and the private `StepChange::new` | `core/src/habit_management/domain/habit.rs`, `core/src/habit_management/domain/habit_id.rs`, `core/src/habit_management/domain/habit_title.rs` |
| Measured, not inferred | Dev-B proved the effect by controlled experiment: renaming `HabitId::new` → `HabitId::parse`, changing nothing else, made **7 mutants appear** on the boundary comparison where there had been zero; applied workspace-wide it yielded **48 mutants, 34 caught, 14 unviable, 0 missed**. Verified contrast: `LocalDate::from_epoch_day` — a fallible constructor of identical shape but a different name — **does** get mutants | `core/src/shared/local_date.rs` |
| Mitigation: **surface it, advisory only** | `scripts/check.sh` lists every `fn new` under `core/src/**/domain/**` whose return type is not a bare `Self`, naming the cause inline. It is **never appended to `FAILED` and never affects the exit code** — both reviewers verified this by breaking it deliberately. It is an explicitly-labelled **single-line-signature grep heuristic** that prints its own limits on every run (it will miss a wrapped signature and cannot see through a type alias), and it distinguishes "could not look" from "none found" rather than reporting an unscanned target as clean | `scripts/check.sh` |
| A survivor on an adapter is a **use-case** finding | Testing an adapter directly proves the adapter matches itself. When a mutant survives outside the perimeter, the question is which use case failed to cover the concern. The two `UuidGenerator` survivors were really saying that no test pinned "every accepted request gets a freshly generated id" — that test belongs to `RequestHabit` | `core/src/habit_management/use_cases/request_habit.rs` |
| The gate reads **committed** work | The diff is `git diff <base-ref>...`, and cargo-mutants refuses a tree whose lines have moved under the patch. Commit the slice, then gate it | `scripts/mutation-gate.sh` |
| **base-ref is mandatory, never defaulted** (corrected 2026-07-26) | The base-ref is a CLI argument with **no default and no guess**: *the commit immediately before this slice's first RED commit*. `scripts/mutation-gate.sh` exits **2** without one, and `scripts/check.sh` exits 2 before any gate runs when a work-class is given without one. Only the caller knows which commit that is — it moves on every new slice, so any fixed default goes stale as often as it helps | `scripts/mutation-gate.sh`, `scripts/check.sh` |
| Why the old `HEAD~1` default was a defect, not a convenience (2026-07-26) | GATE 2 of the operator protocol **mandates** every TDD slice be split into ≥ 2 commits — failing tests alone, then implementation, a hook hard-denying a mixed commit. So the protocol's *only* permitted shape is exactly what `HEAD~1` under-scoped: the RED commit's test additions fell outside the measured diff. **The more disciplined the developer, the more the gate under-measured.** Measured on the `98a1d13`→`5693d01` pair: `HEAD~1` → `killed=2`; correct ref `70e3d70` → `killed=3` — and **both reported `verdict: "pass"`**. Nothing in the output distinguished "correctly scoped" from "missed a commit". Reproduced independently by QA, the CTO and Dev-B | `scripts/mutation-gate.sh` |
| Code anchors are checked | Every backticked Rust source path cited in `docs/` — `` `core/src/…rs` ``, `` `app/src/…rs` ``, or a bare `` `src/…rs` `` — must exist. The knowledge graph's anchors are its only bridge from a domain term to the code; a dead one sends its reader nowhere, silently. A *planned* shape is therefore written unanchored (`domain/step_history.rs`, no `src/` prefix) until the file lands — the check deliberately guards resolvable anchors, not intentions | `scripts/check.sh` (doc anchors gate) |
| One runner, no fail-fast **between gates** | `scripts/check.sh` runs every gate even after one fails, and never invokes a gate as an `if` condition — bash suspends errexit inside a function called that way, which is how a runner swallows the failures it exists to surface | `scripts/check.sh` |
| …but **fail fast on a malformed invocation** (added 2026-07-26) | A work-class with no base-ref is an *argument error*, not a measurement to report: the runner prints its own usage and exits 2 **before any gate runs**, instead of burning two to four minutes only to hand back `scripts/mutation-gate.sh`'s usage message at an unrelated absolute path. Not a contradiction of the row above — that row is about each *gate* reporting its own result so one run shows the full picture; when the arguments are wrong there is nothing yet to measure. `check.sh` with **no arguments at all** is unaffected: that stays the deliberate, loud mutation-skip path, exit 0 | `scripts/check.sh` |
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
| Keeping a `HEAD~1` base-ref default "for convenience" (2026-07-26) | It is not a convenience, it is a silent under-measurement of the *only* commit shape the protocol permits, and it reports `pass` while doing so. A wrong measurement presented as a verdict is worse than a refusal to measure |
| Defaulting the base-ref from an env var or a `merge-base` guess | The base-ref is a **per-slice** value — it moves every time a slice starts. A fixed default would go stale as often as it helped, and a `merge-base` guess has no base branch to work from (development happens on `master`) |
| **Renaming the six domain `new` constructors to `parse` now** (2026-07-26) | It is the real fix — it lifts the cargo-mutants skip and aligns with [[adr-0001-validation-by-construction]]'s own "Parse, don't validate" wording, with the effect measured above. But it is a **public-API-changing decision awaiting the human**: deliberately deferred, recorded here as a known option, **not decided** |
| Making the `new`-constructor listing a blocking gate | It reports a property of the *instrument*, not a defect in the change under review. Failing a slice for a limitation of cargo-mutants would train the team to ignore a red gate — the fastest way to lose a gate's meaning |
| Writing a real Rust parser instead of the grep heuristic | The listing exists to keep a known blind spot visible until the rename is decided, not to be an authority. An honest heuristic that prints its own limits beats a "parser" that quietly mis-parses; the fix is the rename, not a better lister |
| Suppressing the listing once the team "knows about it" | Institutional memory is what this graph exists to replace. The listing is printed on every run precisely so the blind spot cannot go back to being invisible |

## Consequences

- A slice is not done when its tests pass; it is done when its scenarios are no longer `@wip` and its diff mutates clean.
- Specifying a slice before building it is now cheap and visible: 18 scenarios across five backlog slices are written and waived, waiting for their tests.
- Running the mutation gate mid-slice, on a dirty tree, will fail with a line-mismatch error rather than a verdict. That is a workflow constraint, not a bug.
- Code outside the perimeter is not "trusted", it is **out of this instrument's reach**. An adapter's correctness is established by the use case that drives it, or not at all.
- Full campaign at the perimeter's adoption: 32 mutants, 25 caught, 0 survivors, 7 unviable. **Read that figure with the `fn new` skip in mind** — it counts only what cargo-mutants agreed to generate, and the same tree yields 48 mutants once one constructor is renamed. A zero-survivor campaign here means "clean within reach", never "clean".
- **Running the gate requires knowing the slice's first RED commit.** That is a real cost carried by the caller on every run, accepted deliberately: the alternative measured a subset and called it a pass.
- **A green mutation run says nothing about the domain's invariants.** Every rule that [[adr-0001-validation-by-construction]] places in a VO constructor is invisible to the gate, so a regression in any one of them still passes as a clean zero-survivor run. Until the rename is decided, the countermeasure is human: boundary tests written to the **exact** boundary (`MIN_LEN`, `MAX_LEN`, `MIN_LEN - 1`, `MAX_LEN + 1`), because no instrument will catch a sloppy one — see [[adr-0010-crate-boundary-trust-boundary]], whose `HabitId::new` tests were written under exactly this constraint.
- The blind-spot listing is **advisory noise by design**: it prints on every run, green or red, and never changes the exit code. A reader who sees it and shrugs has still been told.
- **Open, deferred, and owned by the human**: renaming the domain's `new` constructors to `parse`. Not decided here; the option and its measured payoff are recorded so the decision can be taken with evidence rather than rediscovered.
- The gates need the operator's toolbox on the machine (`CLAUDE_HOME`, default `~/.claude`). A fresh clone without it gets two red gates and a path in the message, not a false green.
- A scenario may be pinned from either crate, so a slice's user-observable half is provable: "both gestures are offered" is an assertion about a rendered screen, and it counts.
