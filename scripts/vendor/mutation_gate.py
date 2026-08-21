#!/usr/bin/env python3
"""mutation_gate.py — stack-detecting, diff-scoped mutation-testing gate.

Downstream project repos (Rust and C# only) run mutation testing as a
blocking gate on NEW code, scoped to the diff against a base ref. Threshold:
0 surviving mutants on the diff, strict. The sole escape is a survivor marked
`equivalent-accepted` — and THIS TOOL NEVER DECIDES THAT: it emits survivors
by name with `disposition: null`; a human/reviewer fills the disposition in
later (Dev B, at review time), never the tool and never the implementer (who
has a structural incentive to wave survivors through).

Two independent tools drive mutation testing on the two supported stacks:
  - Rust:  cargo-mutants, reading `mutants.out/outcomes.json`.
  - C#:    Stryker.NET, reading the mutation-testing-elements JSON report
           (`StrykerOutput/<timestamp>/reports/mutation-report.json`).
Both are normalised into ONE `mutation-report/v1` shape (see `decide()`'s
docstring and the `mutation-testing` neurone) so a consumer (Dev B, QA, the
TDD-provenance relaxation) never has to know which tool produced a report.

Pure core / imperative shell (ca-ports-adapters in miniature): every
decision-bearing function below — stack detection, nextest detection, Blazor
detection, command construction, the two report parsers, `decide()`, and the
new `_tool_diagnostic()` helper — is pure: given the same inputs, same
outputs, no subprocess, no network, no mutable global state. The two report
parsers are ADAPTERS over each tool's native JSON shape; `decide()` is the
PORT's decision logic, and it does not know or care which adapter produced
its input. This is what makes the whole verdict surface unit-testable
without ever invoking a real cargo-mutants/Stryker campaign — see
`test_mutation_gate.py`, whose fixtures include a REAL captured
cargo-mutants report (a genuine baseline crash) alongside hand-built ones,
precisely so the pure-function tests stay honest about the tools' actual
JSON shape even though they never shell out to them. `run_gate()` is the
ONLY impure function: it is the sole caller of `subprocess`, and it is
deliberately thin and NOT unit-tested directly (see `test_mutation_gate.py`'s
module docstring for how its behaviour is instead exercised through the real
CLI, subprocess-to-subprocess, using a scripted stand-in `cargo` executable —
exactly the way `test_scenario_audit.py`'s `TestCli` exercises `main()`).
`--parse-only <report-path>` is the seam that lets a REAL downstream
cargo-mutants/Stryker report validate the two adapters against reality,
closing the fixture-drift risk the pure/impure split otherwise creates.

AC-T3.7 — the gate MUST NEVER SILENTLY PASS. When the mutation tool is
absent, fails to launch, or produces no parseable report, the report says
`ran: false`, `verdict: "error"`, a populated `error` string, and the CLI
exits non-zero. There is no `--allow-missing-tool` escape hatch: unlike a
downstream repo legitimately having zero Gherkin scenarios yet (the
`scenario_audit.py` analogue), a repo invoking a BLOCKING mutation gate
without the mutation tool installed is a CI misconfiguration, not a
legitimate adoption phase — see the module's Open Questions in the
Developer's implementation report for the full argument.

Pure functions + a thin argparse CLI (house style, see lib/tdd_audit.py and
lib/scenario_audit.py). Stdlib-only. `main()` never raises; the exit code
carries the verdict, which is one of "pass" / "advisory-survivors" (a
non-blocking run with real survivors — fold-in, distinct from a genuine
pass) / "empty" (a BLOCKING run that genuinely had nothing to mutate and the
tool exited cleanly — retry BLOCKING 2, never a silent pass) / "error" (tool
absent/unparseable/CRASHED — see below — never a silent "pass") / "fail" /
"not-applicable" (a deliberate, non-failing skip: a Blazor project, excluded
by design, or a diff with no mutable line in scope — see
`_classify_missing_report`).

Issue #121 — "empty"/"pass" vs "error" on a crashed tool. `subprocess.run()`'s
result used to be discarded entirely: a mutation tool that crashed at the
BASELINE (never ran a single mutant, e.g. a misconfigured test-tool flag)
still leaves behind a report file with all-zero counts, which `decide()`
could not tell apart from a genuinely empty diff — both read as "empty" on a
BLOCKING run, or, worse, silently as "pass" on an ADVISORY run (nobody
scrutinizes an advisory result in the first place). `decide()` now takes the
tool's own exit code and reports "error" (with a populated diagnostic naming
the cause) whenever nothing was validly exercised AND the tool exited
non-zero — checked UNCONDITIONALLY, before the blocking/advisory split:
`blocking` decides whether survivors stop the delivery, not whether a
crashed run gets reported honestly. "empty" is reserved for a genuinely
clean, vacuous, BLOCKING exit; a genuinely clean, vacuous, ADVISORY exit
still reports "pass", unchanged. A genuine "fail" (real survivors) is
unaffected either way — cargo-mutants legitimately exits non-zero when
survivors are found, and that branch is checked independently of the exit
code (see `decide()`'s docstring). The report also now carries
`mutants_generated` (the campaign's own mutant count) and, on the Rust path,
`diff_lines` (the scoped diff's size) — both additive fields, so a consumer
can sanity-check a campaign's scope without knowing the codebase.

Issue #138 — the crash from #121's own motivating example
(`additional_cargo_test_args` incompatible with an auto-detected nextest) is
now avoided rather than only diagnosed: `detect_nextest`/`_resolve_nextest`
read `.cargo/mutants.toml` and stand nextest down when the repo pinned
runner-coupled args without an explicit opt-in — see `_resolve_nextest`'s
docstring for the full priority order, including its BLOCKING-2 retry
addition (an explicit `test_tool` key, cargo-mutants' own "which runner"
field). The report gains a third additive field on the Rust path,
`test_runner_diagnostic` — the reason the runner was picked, "making the
choice visible" so a consumer never has to re-derive it.

Issue #130 — a broken environment used to buy a false "pass" instead of a
loud failure. `test_workspace = true` in `.cargo/mutants.toml` widens the
mutants' own test scope to the whole workspace, but cargo-mutants' BASELINE
(upstream `src/lab.rs:180-190`) stays hard-coded to the mutated packages only
and never reads `test_workspace`/`test_package` — a test failing OUTSIDE the
mutated packages is invisible to the baseline, so the campaign never aborts,
and `cargo test` stopping at the first failing binary turns every UNDETECTED
mutant into a false `CaughtMutant` (the truly-killed ones fail early and
never reach the broken binary; the survivors reach it and fail on IT
instead, for the wrong reason). `_resolve_mutant_scope` makes the asymmetry
visible in the report (`mutants_scope`/`baseline_scope`, two additive
fields) and drives a PRE-GATE: before any mutant is tried, `run_gate()` runs
the mutants' own scope for real (`build_pre_gate_command`, a constant argv —
`cargo test --workspace` / `cargo nextest run --workspace`, reusing the SAME
runner `_resolve_nextest` already picked, never recalculated) and refuses to
proceed (`verdict: "error"`, `pre_gate_diagnostic` populated, the third
additive field) if that suite is not already green. Scoped to when the
asymmetry structurally exists (`needs_pre_gate`, cargo-mutants' DEFAULT scope
already has the baseline cover every mutant, no pre-gate needed there) and
applied uniformly across every work class, including advisory `quick-change`
runs (an advisory result that lies about the environment is still a lie).

This module requires Python 3.11+ (the stdlib `tomllib` used to parse
`.cargo/mutants.toml`) — the only file under `shared/lib/` with that floor;
every other file here runs on whatever `python3` the deploy target has.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path


# ---------------------------------------------------------------------------
# Per-work-class depth table (AC-T3.4) — data, not scattered conditionals.
# `scope` is descriptive metadata carried in the normalised report; `blocking`
# is the one field the verdict computation actually branches on.
#
# retry BLOCKING 4 — `new-feature`'s `scope` was `"in-diff+module"`, but
# `build_command` never implements the full-module campaign over the new
# bounded context (no bounded-context-path input exists for it to build from
# — see `build_command`'s docstring). The report must describe what actually
# ran, not what the AC aspired to: `"in-diff"`, same as `fix-bug`, is the
# truthful label. The full-module run is a genuine future feature, deferred
# — see ADR-2026-14's retry addendum.
# ---------------------------------------------------------------------------
_DEPTH_BY_WORK_CLASS = {
    "quick-change": {"scope": "touched-files-only", "blocking": False},
    "fix-bug": {"scope": "in-diff", "blocking": True},
    "new-feature": {"scope": "in-diff", "blocking": True},
}

# Canonical, specific markers only — never a blanket "blazor" substring (see
# is_blazor_project's docstring and the non_blazor.csproj fixture's decoy
# PackageReference).
_BLAZOR_MARKERS = (
    'sdk="microsoft.net.sdk.blazorwebassembly"',
    '<packagereference include="microsoft.aspnetcore.components.webassembly"',
    '<packagereference include="microsoft.aspnetcore.components.webassembly.',
    "<useblazorwebassembly>true</useblazorwebassembly>",
)

# Stryker.NET mutation-testing-elements status values (mutation-testing-report-
# schema.json): "Killed", "Survived", "NoCoverage", "CompileError",
# "RuntimeError", "Timeout", "Ignored", "Pending". A mutant with NO test
# covering it is exactly as undetected as one a test ran against and missed —
# both count as survived here (neither was caught).
_STRYKER_SURVIVOR_STATUSES = ("Survived", "NoCoverage")
_STRYKER_UNVIABLE_STATUSES = ("CompileError", "RuntimeError")


def detect_stack(root):
    """Detect which mutation-testing stack applies under `root`.

    Returns "rust" (a `Cargo.toml` exists at `root`), "dotnet" (a `*.csproj`
    or `*.sln` exists anywhere under `root`), or "unknown" (neither). Rust
    takes priority when both are present — a Rust workspace embedding a
    generated/vendored `*.csproj` is far more plausible than the reverse.
    """
    root_path = Path(root)
    if (root_path / "Cargo.toml").is_file():
        return "rust"
    if _find_all_files(root_path, (".csproj", ".sln")):
        return "dotnet"
    return "unknown"


def _find_all_files(root_path, suffixes):
    """Every file under `root_path` (any depth) whose name ends with one of
    `suffixes`, walked without following symlinks (same defensive posture as
    `scenario_audit.py`'s `_walk_files`: a hostile symlink cycle in a
    downstream repo must not hang the walk). Empty list if nothing matches,
    INCLUDING when `root_path` does not exist — `os.walk` on a nonexistent
    path yields nothing and never raises (verified; retry fold-in, QA: the
    previous explicit `is_dir()` pre-check was dead code, not a guard)."""
    matches = []
    for dirpath, _dirnames, filenames in os.walk(str(root_path), followlinks=False):
        for name in filenames:
            if name.endswith(suffixes):
                matches.append(Path(dirpath) / name)
    return matches


def _read_mutants_toml(root_path):
    """Parse `<root_path>/.cargo/mutants.toml`, returning `{}` for every
    failure mode — missing file, not a file, unreadable, not valid UTF-8,
    or any TOML parse failure (including pathological input, see below).

    INVARIANT, not a maintained list (retry-2 fold-in 7, Security
    FINDING-2): every reader of `.cargo/mutants.toml`, present and future,
    goes through THIS function — never a direct `open()`/`Path.read_text()`/
    `tomllib.load` elsewhere in the module — so the parsing/failure-handling
    logic never drifts between callers even though each call is its own
    independent read (BLOCKING 3 retry, Security FINDING-5: two separate
    calls from inside one resolution could observe two different states of
    a file that changed in between — that is about NOT caching the read
    across a single resolution, orthogonal to this invariant, which is
    about not re-implementing the read elsewhere).

    Deliberately NOT an enumeration of caller names: this paragraph was
    already rewritten once, in the #138 retry, specifically to replace an
    earlier over-claim — and it drifted stale again the moment
    `_resolve_mutant_scope` was added in #130 without anyone updating the
    list. A list of callers is a claim that rots on the next new caller,
    every time; an invariant phrased as a rule the reader can check against
    new code does not.

    `tomllib.loads` always returns a `dict` on success or raises
    `tomllib.TOMLDecodeError` on ordinary malformed input (verified: `''`,
    `'a=1'`, `'[[x]]'` all parse to a dict; only genuinely invalid syntax
    raises) — so there is no "valid TOML, non-dict top level" case for a
    caller to guard against separately.

    One failure `TOMLDecodeError` does NOT cover: pathologically deep
    nesting (`RecursionError` on ~5000 nested `[` — reproduced and
    confirmed uncaught by the narrower `except` this function used to
    have). Caught here with a deliberate broad `except Exception`, not a
    narrower catalogue, because fail-open is genuinely safe at this call
    site: every consumer of this dict only ever uses it to ADD a
    constraint (stand nextest down, or pin an explicit `test_tool`) — an
    empty dict is indistinguishable from "no constraints configured",
    which is the same posture a normal, absent `.cargo/mutants.toml`
    already produces. Worst case, a malformed downstream config makes the
    ambient PATH-based nextest detection run unconstrained; if that is
    genuinely incompatible, the baseline crashes loudly and `decide()`
    reports `verdict: "error"` — which can never satisfy the `ran &&
    blocking && verdict == "pass"` TDD-provenance relaxation triple. A
    malformed repo config can degrade this gate's precision, never turn it
    into a silent false pass.
    """
    config_path = root_path / ".cargo" / "mutants.toml"
    try:
        raw = config_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return {}
    try:
        return tomllib.loads(raw)
    except Exception:
        return {}


def _normalise_test_tool(value):
    """Normalise a raw `test_tool` config value to `"nextest"`, `"cargo"`,
    or `None` (issue #138 BLOCKING 3 retry) — the recognition rule
    `_repo_declared_test_tool` and `_resolve_nextest` both need, factored
    out so `_resolve_nextest` can apply it to an already-parsed dict
    instead of re-parsing `.cargo/mutants.toml` a second time (Security
    FINDING-5). `None` covers every case that is not an exact match: the
    value absent, or present as anything else (typo, unknown string,
    wrong type, a future variant this module does not know about) — never
    guessed at.
    """
    return value if value in ("nextest", "cargo") else None


def _repo_declares_additional_test_args(root_path):
    """True if `<root_path>/.cargo/mutants.toml` sets
    `additional_cargo_test_args` (issue #138's stand-down signal).

    The general, defensible invariant checked here is NOT "does the repo use
    flag X" (issue #121's rejected, unbounded flag catalogue) but "has the
    repo pinned test-runner-coupled args at all". `additional_cargo_test_args`
    is forwarded verbatim to whichever test runner ends up executing it, so
    ANY value under that key was necessarily written against ONE runner's
    argument grammar — the repo's own config never says which, which is
    exactly what makes the KEY's presence (not its contents) the signal
    here (contrast `_repo_declared_test_tool`, the one place this module
    reads a VALUE rather than a mere presence).

    A façade over `_read_mutants_toml`, with NO production caller since
    the BLOCKING 3 single-read refactor: `_resolve_nextest` reads
    `.cargo/mutants.toml` once itself and applies the same `in` check
    inline, never through this function (issue #138 BLOCKING 3 retry,
    Security FINDING-5: this function and `_repo_declared_test_tool` used
    to each call `_read_mutants_toml` independently from inside
    `_resolve_nextest`, two reads of a file that could in principle change
    between them — fixed by inlining, not by keeping this as a check on
    the inline path).

    Retained anyway as an independently-tested, stable module surface —
    NOT as a drift guard over `_resolve_nextest`'s inline logic. It is
    not one: issue #138 retry 3's mutation campaign (Dev-B D5, D7) proved
    the façade and the inline check are two separately-tested,
    separately-mutable paths — a mutant that breaks this function is
    caught only by this function's own tests, never by `_resolve_nextest`'s
    (which is pinned independently by Dev-B's D1-D6). Removing this
    function would require no change to `_resolve_nextest` or its tests.
    """
    return "additional_cargo_test_args" in _read_mutants_toml(root_path)


def _repo_declared_test_tool(root_path):
    """The value of `<root_path>/.cargo/mutants.toml`'s `test_tool` key,
    normalised via `_normalise_test_tool` (issue #138 BLOCKING 2 retry).

    This is the ONE place this module reads a VALUE out of
    `.cargo/mutants.toml` rather than merely a key's presence. `test_tool`
    is cargo-mutants' own explicit "which runner" field (`src/options.rs`'s
    `Common::test_tool`, `#[serde(flatten)]`-ed into `Config`, no
    `serde(skip)` — confirmed against upstream source and its own
    `from_config` test), so its value is the most direct, most
    authoritative signal this module can read: a repo that sets it has
    told cargo-mutants itself which runner to use, via cargo-mutants' own
    `self.test_tool.or(other.test_tool)` precedence (CLI first, config
    otherwise) — a fact that holds regardless of whether `build_command`
    ever adds an explicit `--test-tool` flag on top of it.

    A façade over `_read_mutants_toml` + `_normalise_test_tool`, with NO
    production caller since the BLOCKING 3 single-read refactor — see
    `_repo_declares_additional_test_args`'s docstring for why
    `_resolve_nextest` reads the dict inline instead.

    Retained anyway as an independently-tested, stable module surface —
    NOT as a drift guard over `_resolve_nextest`'s inline logic, for the
    same reason given there (Dev-B D5/D7/D8, retry 3): this façade and
    the inline `test_tool` handling are two separately-tested,
    separately-mutable paths, not one guarding the other.
    """
    return _normalise_test_tool(_read_mutants_toml(root_path).get("test_tool"))


def _resolve_nextest(root):
    """Pure decision-plus-reason core of `detect_nextest` (issue #138):
    decide whether `cargo nextest` should run, AND explain the decision —
    "make the choice visible" per the ticket's design constraint, so a
    consumer of the mutation report never has to re-derive why a given run
    picked the runner it did (three developer spawns already rediscovered
    the underlying crash by hand once — see issue #121).

    Returns (selected: bool, reason: str). Checked in this priority order:

    1. `.cargo/mutants.toml`'s `test_tool` key, if it is exactly `"nextest"`
       or `"cargo"` (issue #138 BLOCKING 2 retry, verified against
       cargo-mutants upstream `src/options.rs`) — cargo-mutants' OWN
       explicit "which runner" field, `#[serde(flatten)]`-ed into `Config`
       with no `serde(skip)`, cascading through cargo-mutants' own
       `self.test_tool.or(other.test_tool)` (CLI first, config otherwise)
       regardless of what flag `build_command` does or doesn't add. This
       is the most direct, most authoritative signal available — checked
       FIRST, ahead of `.config/nextest.toml`, because a repo that names
       the runner explicitly, in the one file cargo-mutants itself reads
       for it, has said more than a mere adjacent config file's presence
       ever could. Also ahead of the PATH/binary check below: an explicit
       `test_tool = "nextest"` selects nextest whether or not the binary
       happens to be reachable on THIS machine right now — cargo-mutants
       will pick it up from its own config regardless, so reporting
       anything else here would itself be the false diagnostic this retry
       exists to close.
    2. `.config/nextest.toml` at `root` — a DELIBERATE, versioned repo
       signal ("we use nextest"), distinct from a mere binary on the host's
       PATH (a machine artifact carrying zero repo intent). A repo that
       commits this file has taken on responsibility for keeping its own
       `.cargo/mutants.toml` nextest-compatible; the gate trusts that
       explicit signal over its own PATH-triggered heuristic and NEVER
       stands it down on account of `.cargo/mutants.toml` alone — checked
       here, once case 1 has already deferred (no recognised `test_tool`).
    3. No `cargo-nextest` binary on PATH — nothing to select, cargo test.
    4. `cargo-nextest` on PATH, but `_repo_declares_additional_test_args`
       is true and case 1 found no explicit `test_tool` — the repo pinned
       runner-coupled args without ever saying which runner. Standing
       down is the SAFE choice, not a failure: `cargo test` (libtest) is
       the runner cargo-mutants uses by default absent `--test-tool`, and
       therefore the one the repo's own args were necessarily written
       against.
    5. Otherwise — `cargo-nextest` on PATH, no conflicting signal found;
       unchanged from issue #121's original auto-detection.
    """
    root_path = Path(root)
    # Single read (issue #138 BLOCKING 3 retry, Security FINDING-5): both
    # signals below come from this ONE parsed dict, never from a second,
    # independent call to `_read_mutants_toml` -- see its docstring and
    # `_repo_declares_additional_test_args`'s for why the façades exist
    # but are not called from here.
    mutants_toml = _read_mutants_toml(root_path)
    declared_test_tool = _normalise_test_tool(mutants_toml.get("test_tool"))
    if declared_test_tool == "nextest":
        return True, (
            "nextest selected: .cargo/mutants.toml sets "
            'test_tool = "nextest" -- cargo-mutants\' own explicit runner '
            "choice, highest-priority signal, wins regardless of "
            "additional_cargo_test_args or cargo-nextest's PATH reachability")
    if declared_test_tool == "cargo":
        return False, (
            "cargo test selected: .cargo/mutants.toml sets "
            'test_tool = "cargo" -- explicit repo choice, not an inferred '
            "stand-down")
    config_path = root_path / ".config" / "nextest.toml"
    if config_path.is_file():
        return True, (
            "nextest selected: .config/nextest.toml present (explicit "
            "repo opt-in, wins over PATH-only detection and over "
            ".cargo/mutants.toml's additional_cargo_test_args)")
    if shutil.which("cargo-nextest") is None:
        return False, "cargo test selected: no cargo-nextest binary on PATH"
    if "additional_cargo_test_args" in mutants_toml:
        return False, (
            "cargo test selected: cargo-nextest is on PATH, but "
            ".cargo/mutants.toml sets additional_cargo_test_args without "
            "an explicit test_tool -- those args are coupled to whichever "
            "runner receives them, so standing down to cargo test "
            "(cargo-mutants' default runner, the one the repo's own args "
            "were written against) avoids a baseline crash")
    return True, ("nextest selected: cargo-nextest binary on PATH, no "
                   "repo constraint found")


def _resolve_mutant_scope(root):
    """Pure decision-plus-reason core for issue #130's scope report (AC-2):
    tell apart the scope MUTANTS actually run at from the scope the
    BASELINE runs at, so the report can finally say where each one runs
    without opening `mutants.out/log/`.

    Returns (mutants_scope: str, baseline_scope: str, needs_pre_gate: bool,
    reason: str).

    `baseline_scope` is always `"mutated-packages"` on Rust -- a
    STRUCTURAL fact of cargo-mutants 27.1.0, not a repo-configurable one:
    `run_baseline()` (upstream `src/lab.rs:180-190`) hard-codes
    `PackageSelection::Explicit(all_mutated_packages)` and never consults
    `options.test_workspace` or `options.test_package`. No config this
    module could read would change it.

    `mutants_scope` (and `needs_pre_gate`) reads `.cargo/mutants.toml`
    (via `_read_mutants_toml`, so malformed config degrades to "no
    signal" exactly like `_resolve_nextest`'s reads do -- never raises),
    checked in this priority order:

    1. `test_workspace` is the TOML boolean `true` (`is True`, not merely
       truthy -- a string `"true"` or an integer `1` is a different repo
       config value and must not be treated the same) -- `run_queue()`
       (upstream `src/lab.rs:245`) widens to `PackageSelection::All` via
       `TestsForMutant::TestsForMutant::Workspace`, so mutants run against
       the WHOLE workspace while the baseline above still covers only the
       mutated packages. Structural asymmetry, pre-gate required.
    2. A non-empty `test_package` list (and case 1 did not already match)
       -- mutants run against an explicit package set the baseline does
       not necessarily cover either. Pre-gate required.
    3. Otherwise -- cargo-mutants' default `TestsForMutant::Mutated`
       already covers the union of mutated packages, i.e. exactly what
       the baseline covers. No asymmetry, no pre-gate.
    """
    mutants_toml = _read_mutants_toml(Path(root))
    baseline_scope = "mutated-packages"
    if mutants_toml.get("test_workspace") is True:
        return ("workspace", baseline_scope, True,
                "pre-gate required: .cargo/mutants.toml sets "
                "test_workspace = true -- mutants run against the whole "
                "workspace (`cargo test --workspace` / `TestsForMutant::"
                "Workspace`, upstream src/lab.rs:245) while cargo-mutants' "
                "own baseline (src/lab.rs:180-190) is hard-coded to "
                "PackageSelection::Explicit(mutated packages) and never "
                "reads test_workspace/test_package -- a test failing "
                "outside the mutated packages would never abort the "
                "baseline, only the (wider) mutant runs")
    test_package = mutants_toml.get("test_package")
    if isinstance(test_package, list) and test_package:
        return ("explicit-packages", baseline_scope, True,
                "pre-gate required: .cargo/mutants.toml sets test_package "
                "-- mutants run against an explicit package set that the "
                "baseline (mutated packages only) does not necessarily "
                "cover")
    return ("mutated-packages", baseline_scope, False,
            "no pre-gate needed: mutants scope (mutated packages, "
            "cargo-mutants' default TestsForMutant::Mutated) already "
            "matches the baseline scope -- no asymmetry to guard against")


def detect_nextest(root):
    """True if `cargo nextest` should be used for the Rust test-tool
    (AC-T3.3). Delegates to `_resolve_nextest` and discards the reason —
    see ITS docstring for the priority order; this is the one normative
    copy, not restated here (retry-class lesson: five copies of the same
    priority order across this docstring, `_resolve_nextest`'s own, the
    module header, and two knowledge-base docs is exactly the drift
    surface that produced issue #138's own BLOCKING-2 retry).

    Kept as a public, directly testable boolean entry point
    (`TestDetectNextest` exercises it) even though `run_gate` calls
    `_resolve_nextest` directly for the reason string.
    """
    return _resolve_nextest(root)[0]


def is_blazor_project(csproj_text):
    """True if `csproj_text` is a Blazor WebAssembly project file.

    Matches specific, canonical markers only (an `Sdk="..."` attribute value,
    or a `PackageReference` to a `Microsoft.AspNetCore.Components.WebAssembly*`
    package) — never a blanket substring search for "blazor", which would
    false-positive on an unrelated third-party package merely named
    `BlazorFormManager.Client`.
    """
    lowered = csproj_text.lower()
    return any(marker in lowered for marker in _BLAZOR_MARKERS)


def build_command(stack, work_class, base_ref, nextest, timeout, root,
                   copy_fidelity_args=()):
    """Construct the mutation-tool argv for `stack` (AC-T3.2).

    Rust: `cargo mutants --in-diff <root>/.mutation-gate/changes.patch
    --timeout <timeout>`, plus `--test-tool nextest` when `nextest` is true,
    plus `copy_fidelity_args` appended VERBATIM, LAST (issue #130 third
    dimension, tree-content fidelity — see `_resolve_copy_fidelity_args`).
    The diff patch itself is produced by `run_gate()` (impure — a `git diff`
    subprocess call); `build_command` only ever emits the path, deterministically
    derived from `root`, never touches the filesystem.

    C# (non-Blazor): `dotnet stryker --since:<base_ref>`, plus
    `--config-file stryker-config.json` when that file exists at `root`.
    `copy_fidelity_args` is a Rust-only concept (cargo-mutants' own tree-copy
    mechanism has no dotnet analogue) — the dotnet branch never reads the
    parameter, so its argv is byte-identical whether or not a caller passes
    one.

    `--timeout` is unconditional on the Rust command (AC-T3.2: "mandatory").
    `work_class` is validated against the depth table (fail fast on a typo)
    but does not currently vary the argv — the per-work-class SCOPE
    difference is carried as descriptive metadata in the normalised report
    (see `decide()`), not as a distinct CLI invocation; see the Developer's
    Open Questions for why `--in-diff` covers all three classes as an
    over-approximation of "touched files only" for `quick-change`, and why
    "in-diff+module" for `new-feature` cannot be built without a
    bounded-context path this signature does not carry.
    """
    if work_class not in _DEPTH_BY_WORK_CLASS:
        raise ValueError(f"unknown work_class: {work_class!r}")
    if stack == "rust":
        cmd = ["cargo", "mutants", "--in-diff", diff_patch_path(root),
               "--timeout", str(timeout)]
        if nextest:
            cmd += ["--test-tool", "nextest"]
        cmd += list(copy_fidelity_args)
        return cmd
    if stack == "dotnet":
        cmd = ["dotnet", "stryker", f"--since:{base_ref}"]
        if (Path(root) / "stryker-config.json").is_file():
            cmd += ["--config-file", "stryker-config.json"]
        return cmd
    raise ValueError(f"no mutation command for stack {stack!r}")


def build_pre_gate_command(nextest, extra_args=()):
    """Construct the pre-gate's argv (issue #130 slice 2, AC-1/AC-3;
    retry-2 BLOCKING 1).

    CONSTANT prefix — `cargo test --workspace` (or `cargo nextest run
    --workspace` when `nextest` — the SAME bool `run_gate()` already
    resolved via `_resolve_nextest` for the real mutation command, reused
    here rather than recalculated, so the pre-gate validates the exact
    same environment it claims to guard) — plus `extra_args` appended
    VERBATIM, IN ORDER, never joined, never shell-interpolated (no
    `shell=True` anywhere in this module; keep it that way).

    Retry-2 BLOCKING 1 (Dev-B, reproduced end-to-end): the ORIGINAL
    over-approximation argument — "a green workspace implies every subset
    green" — holds only in the PACKAGE dimension. It silently assumed the
    same FEATURE set too. `additional_cargo_test_args` (e.g.
    `--all-features`, `--features x`, `--no-default-features`) is appended
    by cargo-mutants itself to EVERY command it runs, baseline and mutants
    alike (verified against a real campaign) — a pre-gate that omits it
    validates a DIFFERENT, STRICTLY WEAKER command than the one the
    mutants actually execute: an under-approximation, not the accepted
    over-approximation. A test gated behind a feature the pre-gate never
    enabled can be red under the mutants' real invocation while the
    pre-gate reports green — the exact silent false-pass this ticket
    exists to close, on a config shape the fix had not yet covered.

    The rejected alternative (refuse whenever `additional_cargo_test_args`
    is present) was deliberately NOT taken: codeimpact carries
    `additional_cargo_test_args = ["--", "--test-threads=4"]` (mandated by
    ADR-0025/ADR-0028, not removable) TOGETHER WITH `test_workspace =
    true` — refusing would make the gate decline to run on the very repo
    it guards, over args that change neither which tests run nor their
    outcome, only parallelism. An unusable guard is worse than a narrowed
    property. Passing the args through grants the repo NO new capability:
    it already has total control over what cargo-mutants executes via that
    SAME config file — this aligns the pre-gate's check with reality,
    exactly as #138 aligned the crash diagnostic with reality.

    Still deliberately NOT a copy of the repo's PACKAGE/glob scoping
    (`test_package`, `--file`): the pre-gate still over-approximates to
    `--workspace` in that dimension.

    retry-3 BLOCKING-2 correction, in place: the paragraph above used to
    claim "only the FEATURE dimension needed aligning" — FALSIFIED by
    Dev-B reproducing the exact same false green a second time, on
    `all_features = true` (the CANONICAL spelling of the feature
    dimension; `additional_cargo_test_args = ["--all-features"]` is a
    detour nobody writes when the first-class key exists) and on
    `additional_cargo_args`. `additional_cargo_test_args` was never the
    whole feature dimension — it was one of six `.cargo/mutants.toml` keys
    (verified against `cargo mutants --emit-schema=config`, 27.1.0) that
    reach every test invocation cargo-mutants makes: `all_features`,
    `no_default_features`, `features`, `profile`, and `additional_cargo_args`
    join it. `extra_args` is now built by `_resolve_pre_gate_extra_args`,
    which translates all six and REFUSES (rather than silently omitting)
    on any OTHER `.cargo/mutants.toml` key it cannot classify as either
    reproducible or verified-inert — see that function's docstring for the
    full accounting. `build_pre_gate_command` itself stays a pure,
    mechanical "prefix + verbatim append"; the classification lives
    upstream, in the caller.

    SECURITY (retry-3 fold-in 4, Security FINDING-6): `extra_args` execute
    in the pre-gate's subprocess, which runs cargo-mutants' TARGET repo's
    build in the SOURCE tree directly — unlike cargo-mutants' own mutant
    runs, which build in a COPIED tree. An argument that writes to a path
    (e.g. a hypothetical relative `--logfile`) would land in the source
    tree here but in the copy there — a real capability delta Security
    measured, not merely a theoretical one. It stays informational, not a
    blocker: running this gate on a repo already means executing that
    repo's `build.rs`, proc macros, and test bodies directly in the source
    tree — arbitrary code execution the gate already grants by design, of
    which a relative-path write is a strictly weaker instance. This
    matters operationally because the gate guards PRs, including branches
    nobody on this team wrote. No flag filtering by catalogue is applied
    here for this reason or any other — an open-ended "dangerous flags"
    list is the exact anti-pattern issue #121 already rejected; the
    reproducible/inert classification above is a FIDELITY mechanism (does
    the pre-gate run the same command as the mutants?), never a security
    filter, and the two must not be conflated.
    """
    if nextest:
        cmd = ["cargo", "nextest", "run", "--workspace"]
    else:
        cmd = ["cargo", "test", "--workspace"]
    cmd += list(extra_args)
    return cmd


def _additional_test_args(root):
    """Return `.cargo/mutants.toml`'s `additional_cargo_test_args` list,
    VERBATIM, IN ORDER (issue #130 retry-2 BLOCKING 1) — the same list
    cargo-mutants itself appends to every test invocation it makes, so
    `build_pre_gate_command` can append the identical tokens instead of
    validating a different command than the one the mutants run.

    Read through `_read_mutants_toml` (never a new parse — see its
    docstring's invariant). Degrades to `[]`, never raises, on anything
    this module cannot confidently treat as a real args list: missing/
    malformed config (already `_read_mutants_toml`'s own fail-open
    posture), the key absent, a non-list value, or a list containing any
    non-string element (a non-string token would crash `subprocess.run`'s
    argv, which must never receive anything but strings). `[]` is
    indistinguishable from "no extra args configured" — the same
    direction `_read_mutants_toml`'s own docstring already argues is safe:
    every consumer of this value only ever uses it to ADD tokens to the
    pre-gate's argv, so degrading to empty narrows the pre-gate back
    toward the (accepted, over-approximating) plain `--workspace` case —
    never the dangerous direction of guessing at a partial or malformed
    list.
    """
    value = _read_mutants_toml(Path(root)).get("additional_cargo_test_args")
    if not isinstance(value, list):
        return []
    if not all(isinstance(token, str) for token in value):
        return []
    return list(value)


def _translate_all_features(value):
    """`all_features = true` -> `--all-features` (issue #130 retry-3
    BLOCKING-2). Strict `is True`, not merely truthy -- same discipline as
    `test_workspace` elsewhere in this module: a non-bool value is a
    different repo config value, not the same signal."""
    return ["--all-features"] if value is True else []


def _translate_no_default_features(value):
    """`no_default_features = true` -> `--no-default-features`."""
    return ["--no-default-features"] if value is True else []


def _translate_features(value):
    """`features = ["a", "b"]` -> `["--features", "a,b"]` (comma-joined,
    matching the schema's own "space or comma separated list" wording for
    the single-token form cargo actually accepts). An empty list or a
    malformed VALUE (non-list, or containing a non-string element)
    contributes nothing -- a malformed value for a KNOWN key fails open,
    same posture as `_additional_test_args`; this is a different question
    from an UNKNOWN KEY, which refuses (see `_resolve_pre_gate_extra_args`).
    """
    if isinstance(value, list) and value and all(
            isinstance(feature, str) for feature in value):
        return ["--features", ",".join(value)]
    return []


def _translate_profile(value):
    """`profile = "release"` -> `["--profile", "release"]`. `profile`
    changes `debug_assertions` (a `debug_assert!` that only fires in the
    debug profile is a real fidelity gap the pre-gate must not silently
    ignore) -- reproducible, not merely inert timing/tooling metadata."""
    return ["--profile", value] if isinstance(value, str) and value else []


def _translate_cargo_args_list(value):
    """`additional_cargo_args = [...]` appended VERBATIM -- the repo's own
    "extra args to every cargo invocation" key (schema: "Pass extra args to
    every cargo invocation"), the same shape as `additional_cargo_test_args`
    but scoped to cargo itself rather than only `cargo test`. Malformed
    value (non-list, or a non-string element) contributes nothing, same
    fail-open posture as every other translator here."""
    if isinstance(value, list) and all(isinstance(token, str) for token in value):
        return list(value)
    return []


# issue #130 retry-3 BLOCKING-2 (Dev-B, reproduced twice: round-1 via
# `additional_cargo_test_args = ["--all-features"]`, round-2 via the
# CANONICAL `all_features = true` / `additional_cargo_args =
# ["--all-features"]` spellings). BLOCKING-1 closed exactly one of the six
# `.cargo/mutants.toml` keys that reach cargo-mutants' actual test
# invocations (verified against `cargo mutants --emit-schema=config`,
# 27.1.0 -- 25 keys total, run the command yourself rather than trusting
# this comment). This dict is the REPRODUCIBLE half: every key whose value
# changes what tests exist or their pass/fail outcome once compiled, each
# mapped to a pure `value -> extra argv tokens` translator. Iteration
# order is PYTHON DICT INSERTION order, deliberately fixed here rather
# than left to depend on the order keys happen to appear in the repo's own
# TOML file -- `additional_cargo_test_args` (handled separately, appended
# LAST by `_resolve_pre_gate_extra_args`, not part of this dict) commonly
# opens with its own `--` libtest separator, so every cargo-level flag
# here must land BEFORE it or risk being swallowed as a libtest argument
# instead of a cargo one.
_PRE_GATE_REPRODUCIBLE_TRANSLATORS = {
    "all_features": _translate_all_features,
    "no_default_features": _translate_no_default_features,
    "features": _translate_features,
    "profile": _translate_profile,
    "additional_cargo_args": _translate_cargo_args_list,
}

# The INERT half: real schema keys (same 27.1.0 source) that are NOT
# REPRODUCIBLE in the pre-gate's argv -- none of them has a corresponding
# CLI token this module could faithfully append. "Inert" describes their
# relationship to `build_pre_gate_command`'s argv, not a blanket claim
# that omitting them is always harmless.
#
# retry-5 correction (Architect, reading and measuring the ACTUAL upstream
# copy walker -- the retry-4 correction directly above this one got the
# MECHANISM backwards on two of its three claims, even though its
# conclusion was right; see `_resolve_copy_fidelity_args`'s docstring for
# the full, re-verified account this paragraph summarises):
#
# HEADLINE, corrected: the copy cargo-mutants builds each mutant in is a
# SUBSET of the source tree BY DEFAULT -- `copy_tree.rs:111-112`:
#     && (copy_target || !is_top_level_target)
#     && (copy_vcs   || !VCS_DIRS.contains(&name.as_ref()))
# with `copy_vcs`, `copy_target`, AND `gitignore` all DEFAULTING TO FALSE
# (`options.rs:346,359-363`; upstream's own test is literally named
# `gitignore_off_by_default`). So by default: no `.git`/VCS dirs, no
# top-level `target/`, but gitignored files ARE STILL COPIED (the
# `retry-4` claim that "default gitignore = true excludes gitignored
# paths" was backwards -- the default is `false`, and `gitignore = true`
# is an explicit repo OPT-IN that narrows the copy further, not the
# default state). `copy_vcs = true` / `copy_target = true` are the
# MITIGATION a repo can opt into, not the trigger of a divergence -- the
# divergence exists on EVERY repo's default config, unconditionally.
# The retry-4 claim that "a .git-dependent test is absent from .git in
# the pre-gate's own source-tree run" was also backwards: the SOURCE tree
# (where the pre-gate runs) always HAS `.git`; it is the COPY (where
# mutants run) that lacks it by default. The conclusion — pre-gate
# strictly more permissive than the campaign in this dimension — was
# right; it was reached through the wrong mechanism.
#
# Reproduced, one-key control, on /private/tmp/claude-502/r3-copyvcs2
# (left exactly so, evidence for this ticket):
#     test_workspace = true                     -> verdict: "pass",  killed: 5, survived: 0
#     test_workspace = true, copy_vcs = true     -> verdict: "fail",  killed: 0, survived: 5
# `copy_target` is now ALSO independently reproduced (retry-4 left it
# "named as the same class, not independently reproduced" -- no longer
# true), on /private/tmp/claude-502/r4-copytarget (a test reading
# `target/marker.txt`): default -> `5 caught`; `--copy-target=true` ->
# `5 missed`.
#
# `copy_vcs` and `gitignore` are now handled -- see
# `_resolve_copy_fidelity_args` (issue #130, third dimension): the
# CAMPAIGN's own argv is forced to `--copy-vcs=true` whenever
# `needs_pre_gate` is true (moving the copy toward the tree the pre-gate
# already validated), and the gate REFUSES outright on `gitignore = true`
# (forcing `--gitignore=false` could mean re-copying an unbounded amount
# of repo-excluded content). `copy_target` remains open — an
# unconditional force was ruled out on measured cost (8.2 GB on
# codeimpact, 6.3 GB on askme-kayzen) — see
# `_resolve_copy_fidelity_args`'s docstring and the `copy_fidelity_diagnostic`
# report field for the full, current accounting; this trio therefore
# stays in the INERT-FOR-ARGV set below (no CLI token fixes a tree-content
# question), but is no longer treated as "cannot cause a false PASS" —
# the campaign-argv force above is what actually closes two of the three.
#
# The reasoning below for the remaining 16 keys is unaffected by the
# correction above -- each is inert for reasons that hold regardless of
# tree contents (timing, mutant-selection scope, or handled elsewhere):
#   - build_timeout_multiplier / minimum_test_timeout / timeout_multiplier
#     -- timing only, never changes which test passes or fails
#   - cap_lints -- caps RUSTC LINTS at a level, at most changing whether a
#     BUILD fails on a lint that would otherwise be denied; the pre-gate
#     omitting it can only make the pre-gate MORE conservative (a spurious
#     build failure, "verdict: error") -- the safe, fail-closed direction,
#     never a silent pass. Also not a simple `cargo test` CLI flag to
#     reproduce faithfully (rustc-level, not documented as a stable cargo
#     argv token) -- reproducing it approximately would risk being subtly
#     WRONG, which is worse than the current safe-fails-loud behaviour.
#   - output -- relocates cargo-mutants' OWN report directory (what
#     `_locate_report` searches for), not a build-tree key; a relocated
#     report the pre-gate never touches yields "no mutation report found"
#     downstream on the real campaign, not a false pass here
#   - error_values / examine_globs / examine_re / exclude_globs /
#     exclude_re / skip_calls / skip_calls_defaults -- MUTANT SELECTION
#     scope (which mutants get generated/tried), not test execution; the
#     pre-gate over-approximates in this dimension exactly as it already
#     does for `test_package` (see below) and `--file`
#   - sharding -- how cargo-mutants splits ITS OWN campaign across
#     `--shards`; irrelevant to whether the suite itself is green
#   - test_package / test_workspace -- read by `_resolve_mutant_scope`
#     elsewhere (these two ARE what determines `needs_pre_gate` in the
#     first place); not this function's concern
#   - test_tool -- read by `_resolve_nextest` elsewhere; not this
#     function's concern
_PRE_GATE_INERT_KEYS = frozenset({
    "build_timeout_multiplier", "cap_lints", "copy_target", "copy_vcs",
    "error_values", "examine_globs", "examine_re", "exclude_globs",
    "exclude_re", "gitignore", "minimum_test_timeout", "output",
    "sharding", "skip_calls", "skip_calls_defaults", "test_package",
    "test_tool", "test_workspace", "timeout_multiplier",
})


def _resolve_pre_gate_extra_args(root):
    """Classify EVERY key actually present in `.cargo/mutants.toml`
    against the two sets above and return `(extra_args: list[str],
    unknown_keys: list[str])` (issue #130 retry-3 BLOCKING-2).

    `extra_args` is the concatenation of every reproducible key's
    translated tokens, in `_PRE_GATE_REPRODUCIBLE_TRANSLATORS`'s FIXED
    dict-insertion order, with `_additional_test_args`'s result appended
    LAST (it commonly opens with its own `--` libtest separator, so
    nothing cargo-level may follow it).

    `unknown_keys` (sorted, for a deterministic diagnostic) lists every
    key that is in NEITHER set -- a key this module has never classified
    at all. THIS is the invariant that stops the defect class reappearing
    a fourth time: a NEW upstream cargo-mutants key defaults to REFUSE
    (the caller must check `unknown_keys` and abort rather than run the
    pre-gate — see `run_gate`), never to silently running a pre-gate that
    diverges from the mutants' real command. This is a FIDELITY mechanism,
    not a security filter — the reproducible/inert sets exist to keep the
    pre-gate's check truthful, never to block or allow flags on a
    trust/danger basis (there is deliberately no flag blacklist/whitelist
    for security purposes anywhere in this module; see #121's rejected
    open-ended flag catalogue).

    Malformed TOML or a missing file degrades to `([], [])` via
    `_read_mutants_toml`'s own fail-open posture (indistinguishable from
    "no config at all") — never treated as an unknown key.
    """
    mutants_toml = _read_mutants_toml(Path(root))
    extra_args = []
    for key, translate in _PRE_GATE_REPRODUCIBLE_TRANSLATORS.items():
        if key in mutants_toml:
            extra_args += translate(mutants_toml[key])
    extra_args += _additional_test_args(root)
    unknown_keys = sorted(
        key for key in mutants_toml
        if key not in _PRE_GATE_REPRODUCIBLE_TRANSLATORS
        and key != "additional_cargo_test_args"
        and key not in _PRE_GATE_INERT_KEYS)
    return extra_args, unknown_keys


def _resolve_copy_fidelity_args(root):
    """Resolve the CAMPAIGN-side (not pre-gate-side) tree-fidelity argv and
    any refusal (issue #130, third dimension: tree-content fidelity, not
    the argv fidelity `_resolve_pre_gate_extra_args` already covers).

    Returns `(campaign_args: list[str], refusal: str | None)`.

    **Slice 1 — force `--copy-vcs=true`, unconditionally.** cargo-mutants
    copies each mutant's build tree WITHOUT VCS directories (`.git`, `.hg`,
    `.bzr`, `.svn`, `_darcs`, `.jj`, `.pijul`) by DEFAULT — `copy_vcs`
    defaults to `false` (upstream `options.rs:346`), and the copy walker
    (`copy_tree.rs:111-112`) excludes any VCS dir unless `copy_vcs` is
    true. This is the DEFAULT state of every repo, not something a repo
    opts into — a prior version of this docstring/comment got the
    direction backwards (retry-4's own correction), and the operator's own
    re-measurement corrected it again: `/private/tmp/claude-502/
    r3-copyvcs2`, `test_workspace = true` alone -> `verdict: "pass",
    killed: 5, survived: 0` (a `.git`-dependent test failing in every
    mutant's VCS-less copy, misread as "caught" regardless of the
    mutation); adding `copy_vcs = true` on the identical diff ->
    `verdict: "fail", survived: 5` (the same test now passes in the copy,
    same as the source tree, and the real survivors are revealed).
    Forcing `--copy-vcs=true` on the CAMPAIGN's own argv moves the copy
    toward the tree the pre-gate already validated (the pre-gate runs in
    the SOURCE tree, which always has `.git`). CLI beats config upstream
    (`args.copy_vcs.or(config.copy_vcs)`, `options.rs:346`), so this
    OVERRIDES a repo that set `copy_vcs = false` — toward fidelity, the
    safe direction; the repo already had zero ability to stop this via
    that key once the pre-gate is in play. One token covers all seven VCS
    dirs at once. Measured cost: 34 MB on codeimpact.

    **The clean-worktree door — a NEW path this force creates, not a
    pre-existing one left open, and it changes DIRECTION, not merely
    "residual".** A test asserting `git status --porcelain` is empty
    (Dev-B, reproduced on `/private/tmp/claude-502/r5-cleanworktree`, left
    exactly so):
    - BEFORE this fix: `.git` absent from the copy -> the assertion hits
      "not a git repository", which fails in BOTH the baseline's copy and
      every mutant's copy -> the baseline itself aborts -> fail-loud
      `verdict: "error"`. Never a false pass.
    - AFTER this fix: `.git` now present -> the BASELINE's copy is
      genuinely clean (untouched source, no mutation applied yet) so the
      assertion PASSES there; every MUTANT's copy is, by construction,
      dirty (the mutation itself is an uncommitted change) so the
      assertion FAILS there, regardless of what the mutation actually
      did -> every mutant misreads as "caught" -> `verdict: "pass",
      killed: 5, survived: 0`. Reproduced: with the clean-worktree
      assertion in place, `verdict: "pass"`; neutralizing ONLY that
      assertion (no other change) flips it to `verdict: "fail",
      survived: 5`.
    This fix therefore CONVERTS a fail-loud shape into a false-pass shape
    for this one test shape — stated plainly, not softened. It is
    structurally UNAVOIDABLE: a mutant copy cannot simultaneously contain
    `.git` (needed for VCS-dependent tests generally — build-metadata/SHA
    embedding via `vergen` and similar are common) AND present a clean
    worktree while the mutation itself is the uncommitted change that
    makes it dirty; there is no third option. The trade is NET POSITIVE
    on the operator's own call: tests that need `.git` to *exist* are
    common in real repos, while tests asserting a *clean worktree* are
    rare and, where they exist, typically live in a separate CI lint
    stage rather than inside `cargo test` itself. Not closed by this
    ticket; the operator holds the trade, not this function.

    Read through `_read_mutants_toml` (never a new parse), same fail-open
    posture as every other reader here — malformed config never blocks
    the force, it simply has no repo-config signal to read.

    **Slice 2 — refuse on `gitignore = true`, strict `is True`** (same
    boundary discipline as `test_workspace`/`all_features` elsewhere).
    `gitignore` also defaults to `false` upstream (`options.rs`, verified
    against the real `gitignore_off_by_default` test) — DEFAULT already
    copies gitignored paths, so the default case needs no mitigation. Only
    a repo that EXPLICITLY sets `gitignore = true` narrows its own copy.
    Unlike `copy_vcs` (bounded: seven well-known directory names), forcing
    `--gitignore=false` would mean re-copying whatever the repo's own
    `.gitignore` excludes — potentially the entire `target/` directory,
    dependency caches, or generated data of unbounded size. The gate
    refuses outright rather than silently deciding that trade-off on the
    repo's behalf. Verified to break neither `r3-copyvcs2` nor any other
    fixture in this suite (none sets `gitignore = true`).

    **`copy_target` stays a documented, open residual** — see
    `_PRE_GATE_INERT_KEYS`'s docstring and `copy_fidelity_diagnostic` in
    `_report`. Reproduced (`/private/tmp/claude-502/r4-copytarget`,
    default -> `5 caught`, `--copy-target=true` -> `5 missed`) but NOT
    forced here: 8.2 GB measured on codeimpact, 6.3 GB on askme-kayzen —
    an unconditional force would make the gate itself the resource
    problem it exists to prevent. This function does not read `copy_target`
    at all; it is handled by inert-classification + documentation, not by
    an argv decision.
    """
    mutants_toml = _read_mutants_toml(Path(root))
    if mutants_toml.get("gitignore") is True:
        return [], (
            "pre-gate refused: .cargo/mutants.toml sets gitignore = true "
            "-- cargo-mutants' copied build tree excludes gitignored paths "
            "in that case, and forcing a re-copy of everything the repo "
            "explicitly excluded could be unboundedly large (unlike the "
            "bounded, seven-directory --copy-vcs=true force); the gate "
            "refuses rather than deciding that trade-off silently")
    return ["--copy-vcs=true"], None


# issue #130, third dimension: the diagnostic text `_report`'s
# `copy_fidelity_diagnostic` field carries whenever `needs_pre_gate` is
# true and the gate did not refuse — essentially constant (it does not
# vary per repo beyond the fact that it ran), so a module-level string
# rather than a function with nothing to branch on.
_COPY_FIDELITY_DIAGNOSTIC = (
    "campaign argv forced --copy-vcs=true (issue #130, third dimension): "
    "cargo-mutants copies each mutant's build tree WITHOUT .git/.hg/etc by "
    "default (copy_tree.rs:111-112, copy_vcs defaults to false, "
    "options.rs:346) -- a VCS-dependent test fails in that copy for every "
    "mutant regardless of the mutation, producing a false 'caught' for all "
    "of them (reproduced: test_workspace=true alone -> verdict:'pass', "
    "killed:5, survived:0; +copy_vcs=true -> verdict:'fail', survived:5). "
    "Forcing it moves the copy toward the tree the pre-gate already "
    "validated. NOT closed: copy_target (default false, top-level target/ "
    "excluded from the copy -- reproduced, a target/-dependent test is the "
    "same live false-pass shape; 8.2 GB / 6.3 GB measured cost on real "
    "repos ruled out forcing it) and gitignore=true repos (refused "
    "outright, see pre_gate_diagnostic, rather than force-copying a tree "
    "the repo explicitly excluded). NEW DOOR OPENED BY THIS FIX, DIRECTION "
    "CHANGED, NOT MERELY RESIDUAL: a clean-worktree test (e.g. `git status "
    "--porcelain` empty) used to fail-loud (no .git in the copy -> baseline "
    "aborts -> verdict:'error'); now the baseline's copy is clean but every "
    "mutant's copy is dirty by construction (the mutation IS the "
    "uncommitted change) -> false 'caught' for all mutants -> verdict:'pass' "
    "(reproduced on r5-cleanworktree: pass with the assertion in place, "
    "fail once neutralized). Structurally unavoidable -- .git presence and "
    "a clean worktree cannot coexist while mutated -- and judged net "
    "positive: tests needing .git to exist are common, tests asserting a "
    "clean worktree are rare and usually live outside `cargo test`. Also "
    "note: `.mutation-gate/changes.patch` is written into the repo before "
    "the pre-gate runs, so a worktree-cleanliness test will also see that "
    "artifact -- same fail-loud direction, gitignoring the directory "
    "avoids it.")


def diff_patch_path(root):
    """Deterministic path (never created here — see `run_gate()`) of the
    `git diff` patch file `build_command`'s Rust invocation reads via
    `--in-diff`. A plain function of `root`, so `build_command` stays pure."""
    return str(Path(root) / ".mutation-gate" / "changes.patch")


def parse_cargo_mutants(outcomes_json_text):
    """Parse cargo-mutants' `mutants.out/outcomes.json` into the normalised
    shape (minus the work-class-dependent fields `decide()` owns).

    Returns {"stack": "rust", "tool": ..., "killed", "survived", "timeout",
    "unviable", "mutants_generated", "survivors": [{"file","line","function",
    "description", "disposition": None, "reason": None}]}. `killed`/
    `survived`/`timeout`/`unviable` are read directly from the LabOutcome's
    own top-level `caught`/`missed`/`timeout`/`unviable` counters
    (cargo-mutants already computes these; re-deriving them from `outcomes`
    would be a second, possibly-divergent computation of the same count).
    `mutants_generated` (issue #121) is cargo-mutants' own `total_mutants`
    counter — the campaign's size, so a consumer can judge scope (e.g. "5
    mutants for an 18-file diff" is visibly wrong) without knowing the
    codebase. Only `summary == "MissedMutant"` scenarios contribute to
    `survivors` — `disposition` and `reason` are ALWAYS null here (AC-T3.5:
    this function never marks a survivor `equivalent-accepted`).
    """
    data = json.loads(outcomes_json_text)
    survivors = []
    for outcome in data.get("outcomes", []):
        if outcome.get("summary") != "MissedMutant":
            continue
        scenario = outcome.get("scenario")
        if not isinstance(scenario, dict) or "Mutant" not in scenario:
            continue
        mutant = scenario["Mutant"]
        function_info = mutant.get("function")
        function_name = function_info.get("function_name") if function_info else None
        line = mutant.get("span", {}).get("start", {}).get("line")
        survivors.append({
            "file": mutant.get("file"),
            "line": line,
            "function": function_name,
            "description": (f"replace {function_name or '(module-level)'} "
                             f"with {mutant.get('replacement')}"),
            "disposition": None,
            "reason": None,
        })
    version = data.get("cargo_mutants_version")
    tool = f"cargo-mutants {version}" if version else "cargo-mutants"
    return {
        "stack": "rust",
        "tool": tool,
        "killed": data.get("caught", 0),
        "survived": data.get("missed", 0),
        "timeout": data.get("timeout", 0),
        "unviable": data.get("unviable", 0),
        "mutants_generated": data.get("total_mutants", 0),
        "survivors": survivors,
    }


def parse_stryker(mutation_report_json_text):
    """Parse a Stryker.NET mutation-testing-elements report into the
    normalised shape (minus the work-class-dependent fields `decide()` owns).

    Returns the same shape as `parse_cargo_mutants`, with `stack: "dotnet"`.
    `Killed` -> killed. `Survived` and `NoCoverage` both -> survived (neither
    was actually caught by a test, which is the property this gate cares
    about) and both populate `survivors`. `Timeout` -> timeout.
    `CompileError`/`RuntimeError` -> unviable (the Rust-side analogue of a
    mutant that never validly built). `Ignored`/`Pending` are excluded from
    every count — they represent no completed verdict either way, but they
    DO still count toward `mutants_generated` (issue #121): they were
    genuinely produced by the campaign, they just carry no verdict.
    """
    data = json.loads(mutation_report_json_text)
    killed = survived = timeout = unviable = 0
    mutants_generated = 0
    survivors = []
    for file_path, file_entry in data.get("files", {}).items():
        for mutant in file_entry.get("mutants", []):
            mutants_generated += 1
            status = mutant.get("status")
            if status == "Killed":
                killed += 1
            elif status in _STRYKER_SURVIVOR_STATUSES:
                survived += 1
                line = mutant.get("location", {}).get("start", {}).get("line")
                survivors.append({
                    "file": file_path,
                    "line": line,
                    "function": None,
                    "description": f"{mutant.get('mutatorName')}: replace with {mutant.get('replacement')}",
                    "disposition": None,
                    "reason": None,
                })
            elif status == "Timeout":
                timeout += 1
            elif status in _STRYKER_UNVIABLE_STATUSES:
                unviable += 1
    schema_version = data.get("schemaVersion")
    tool = f"Stryker.NET (schema {schema_version})" if schema_version else "Stryker.NET"
    return {
        "stack": "dotnet",
        "tool": tool,
        "killed": killed,
        "survived": survived,
        "timeout": timeout,
        "unviable": unviable,
        "mutants_generated": mutants_generated,
        "survivors": survivors,
    }


def decide(normalized, work_class, tool_exit_code=0):
    """Compute the verdict-relevant fields for `normalized` under `work_class`
    (AC-T3.4 / AC-T3.5). Returns {"scope", "blocking", "verdict"} — the ONE
    place `scope`, `blocking`, and `verdict` are computed (retry-class lesson
    from the sibling T2 tool: a duplicated verdict computed twice, once
    right and once wrong, is worse than one computed once).

    `tool_exit_code` (issue #121, follow-up): the mutation tool's own process
    exit code, when known. It is what tells apart two shapes that otherwise
    look identical at the report layer — `killed + survived + timeout == 0`
    — but mean opposite things: a genuinely empty diff (tool exited 0,
    nothing to mutate) versus a CRASHED run (tool exited non-zero, typically
    because the Rust baseline itself failed to build/run before a single
    mutant could be tried — see `run_gate()`'s nextest-incompatibility note).
    Defaults to 0 so every existing caller that has no live subprocess to
    report an exit code for (`--parse-only`, validating an already-real
    downstream report) keeps the original "empty" classification, unchanged.

    Five verdicts, checked in this priority order:
    - `"error"` — `killed + survived + timeout == 0` AND `tool_exit_code` is
      non-zero: the tool crashed before validly exercising anything.
      Checked UNCONDITIONALLY, before the `blocking` split — `blocking`
      decides whether SURVIVORS stop the delivery, it says nothing about
      whether a CRASHED run should be reported honestly. Reporting a crash
      as "pass" (the old advisory-scope behavior) or "empty" (the old
      blocking-scope behavior) both read as an inert non-event and hide a
      real tool-launch defect — exactly the silent pass AC-T3.7 forbids, one
      layer earlier than "no report found at all". Arguably worse under an
      advisory scope: nobody scrutinizes an advisory result in the first
      place, so a crashed `quick-change` run silently reporting "pass" is
      the more dangerous lie of the two.
    - `"empty"` — retry BLOCKING 2 (Security/Dev-B/QA converged
      independently): same `killed + survived + timeout == 0` condition, but
      the tool is known to have exited cleanly (or its exit code is unset/
      unknown) — i.e. genuinely nothing to mutate, not a crash. A report in
      this shape is indistinguishable at the verdict layer from "every
      mutant was killed" unless called out explicitly; reported "pass" here
      is the exact vacuous-pass class this gate exists to prevent. Scoped to
      BLOCKING runs only — an advisory (`quick-change`) run with a genuinely
      clean, vacuous, non-crashed exit stays `"pass"` (see below): it never
      gated anything, so "nothing was exercised" carries no false confidence
      to correct.
    - `"fail"` — `blocking` is true AND `survived > 0`, REGARDLESS of
      `tool_exit_code`. cargo-mutants legitimately exits non-zero when real
      survivors are found; that must never be reclassified as `"error"`.
      This branch is only reachable once `killed + survived + timeout > 0`
      (`survived` is one of its three addends), so it can never be shadowed
      by the `"error"`/`"empty"` branches above — the exit code is only ever
      diagnostic of a crash when NOTHING was validly exercised.
    - `"advisory-survivors"` — fold-in (Dev B/QA): `blocking` is false AND
      `survived > 0`. Distinct from a genuine `"pass"` so an exit-code-only
      or verdict-only consumer cannot mistake "ran, found real survivors,
      chose not to block" for "ran, found nothing."
    - `"pass"` — otherwise: something was validly exercised (or the run is
      advisory with a genuinely clean, vacuous, non-crashed exit) and no
      survivors were found.

    This function never inspects or mutates `disposition` — it does not and
    cannot know which survivors a reviewer will later accept as equivalent.
    """
    depth = _DEPTH_BY_WORK_CLASS.get(work_class)
    if depth is None:
        raise ValueError(f"unknown work_class: {work_class!r}")
    blocking = depth["blocking"]
    killed = normalized.get("killed", 0)
    survived = normalized.get("survived", 0)
    timeout_count = normalized.get("timeout", 0)
    validly_tested = killed + survived + timeout_count

    if validly_tested == 0 and tool_exit_code:
        verdict = "error"
    elif blocking and validly_tested == 0:
        verdict = "empty"
    elif blocking and survived > 0:
        verdict = "fail"
    elif not blocking and survived > 0:
        verdict = "advisory-survivors"
    else:
        verdict = "pass"
    return {"scope": depth["scope"], "blocking": blocking, "verdict": verdict}


def _tail(text, max_chars):
    """Trim `text` to its last `max_chars` characters (issue #121; factored
    to module level in issue #130 slice 2 so `_pre_gate_diagnostic` can
    reuse the exact same shape instead of re-deriving it — cc-kiss applies
    to test-support helpers too, and this one is genuinely shared, not a
    coincidental duplicate). The TAIL, not the head: the causal line in a
    crashed tool's own transcript lives at the end.

    `max_chars` is REQUIRED, not defaulted (retry-2 fold-in 4, M30): the
    pre-#138 closure this was extracted from never had a default either,
    and both current callers (`_tool_diagnostic`, `_pre_gate_diagnostic`)
    already pass it explicitly — an unused default is an unreachable
    branch, not a convenience. Requiring it makes that mutant structurally
    impossible instead of needing a test to catch it."""
    text = (text or "").strip()
    return text[-max_chars:] if len(text) > max_chars else text


def _tool_diagnostic(returncode, stdout, stderr, max_chars=2000):
    """Build a short, human-readable diagnostic string from a crashed
    mutation tool's own stdout/stderr (issue #121). Used only when `decide()`
    classifies a run as `"error"` because the tool exited non-zero with
    nothing validly exercised — this is what makes that verdict actionable
    instead of a bare label. Pure: no I/O, just string slicing.

    Each stream is trimmed independently (`_tail`, where the causal line
    usually lives — see the real cargo-mutants/nextest transcript this was
    modeled on) to `max_chars`, so one huge stream never crowds out a short
    one on the other side.
    """
    parts = [f"mutation tool exited {returncode} without validly exercising "
             "any mutants (baseline likely failed to build/run)"]
    stderr_tail = _tail(stderr, max_chars)
    if stderr_tail:
        parts.append(f"stderr: {stderr_tail}")
    stdout_tail = _tail(stdout, max_chars)
    if stdout_tail:
        parts.append(f"stdout: {stdout_tail}")
    return "\n".join(parts)


def _pre_gate_diagnostic(returncode, stdout, stderr, max_chars=500):
    """Build a short, human-readable diagnostic string from a RED pre-gate
    test suite (issue #130 slice 2, AC-1/AC-3) — the environment-health
    check that runs, at the mutants' own scope, before any mutant is tried.
    Reuses `_tail`'s exact shape (same trimming behaviour as
    `_tool_diagnostic`, deliberately: the failure mode this reports is a
    sibling of a crashed mutation tool, not a different kind of event).

    Distinct wording from `_tool_diagnostic` on purpose: this is not the
    mutation tool crashing, it is the pre-gate refusing to even start the
    campaign because the suite was already red at the scope the mutants
    would run at.

    SECURITY (retry-2 fold-in 8, Security FINDING-1): this field
    REPUBLISHES up to `max_chars` characters of the AUDITED REPO'S OWN test
    output verbatim, into a report that agents routinely paste into GitHub
    issues. The realistic risk is not an attacker crafting a payload — it
    is a private repo's failing-test output (a token, an API key, a
    production fixture excerpt printed by an assertion) landing in a public
    issue. `max_chars` is deliberately smaller than `_tool_diagnostic`'s
    (500, not 2000): a red test's causal line fits comfortably within it,
    and a smaller window is a smaller republished surface — not a security
    boundary on its own (the repo's own tests can still print whatever they
    want within it), but the cheapest available reduction. No pattern-based
    secret masking is applied here: fragile, unbounded maintenance for a
    problem this field does not claim to solve. `_tool_diagnostic` carries
    the identical exposure since issue #121; this is not new to #130."""
    parts = [f"pre-gate test suite exited {returncode} -- the mutation gate "
             "refuses to run the mutants' own (wider) scope against a suite "
             "that is already red outside the baseline's scope (see this "
             "report's mutant_scope_diagnostic for why the pre-gate ran)"]
    stderr_tail = _tail(stderr, max_chars)
    if stderr_tail:
        parts.append(f"stderr: {stderr_tail}")
    stdout_tail = _tail(stdout, max_chars)
    if stdout_tail:
        parts.append(f"stdout: {stdout_tail}")
    return "\n".join(parts)


def run_gate(root, work_class, base_ref, timeout):
    """IMPURE. Detect the stack, build and run the diff + mutation-tool
    subprocess pipeline, locate and parse the resulting report, and return
    the full normalised `mutation-report/v1` dict (AC-T3.6).

    Never raises (AC-T3.7): every subprocess/filesystem failure — the tool
    binary missing, `git diff` failing, no report file appearing where the
    tool was expected to write one, an unparseable report — collapses to
    `{"ran": False, "verdict": "error", "error": <message>}`. This is
    deliberately NOT unit-tested (it is the one function in this module that
    touches `subprocess`); it is instead exercised through the real CLI,
    subprocess-to-subprocess, in `test_mutation_gate.py`'s `TestCli`, against
    a real temporary git repo with a scripted `cargo` stand-in on PATH (never
    the real cargo-mutants) — deterministic regardless of what mutation
    tooling happens to be installed on the machine running the tests.
    """
    depth = _DEPTH_BY_WORK_CLASS.get(work_class)
    if depth is None:
        return _report(work_class, base_ref, stack=None, tool=None,
                        blocking=False, scope=None, ran=False, verdict="error",
                        error=f"unknown work_class: {work_class!r}", skipped_reason=None)
    scope, blocking = depth["scope"], depth["blocking"]
    try:
        _reject_unsafe_ref(base_ref)
        stack = detect_stack(root)
        if stack == "unknown":
            return _report(work_class, base_ref, stack, None, blocking, scope,
                            False, "error",
                            "no Rust (Cargo.toml) or .NET (*.csproj/*.sln) "
                            "project found under --root", None)
        if stack == "dotnet" and _all_dotnet_projects_are_blazor(root):
            return _report(work_class, base_ref, stack, None, blocking, scope,
                            False, "not-applicable", None, "blazor-excluded")

        nextest, test_runner_diagnostic = (
            _resolve_nextest(root) if stack == "rust" else (False, None))
        mutants_scope, baseline_scope, needs_pre_gate, mutant_scope_diagnostic = (
            _resolve_mutant_scope(root) if stack == "rust"
            else (None, None, False, None))
        diff_lines = None
        if stack == "rust":
            diff_lines = _write_diff_patch(root, base_ref)

        # `campaign_args` / `copy_fidelity_diagnostic` (issue #130, third
        # dimension) stay at their "no pre-gate needed" defaults unless the
        # `needs_pre_gate` block below overrides them: when the pre-gate
        # was never required, baseline and mutant scopes are identical, so
        # an environmental tree-content failure fails the BASELINE too and
        # cargo-mutants aborts on its own -- no false pass reachable
        # through that path, argued from upstream structure (not measured
        # the way the `needs_pre_gate == True` case below is).
        campaign_args = ()
        copy_fidelity_diagnostic = None

        if needs_pre_gate:
            def _pre_gate_error_report(diagnostic, copy_fidelity_diagnostic=None):
                return _report(work_class, base_ref, stack, None, blocking,
                                scope, False, "error", diagnostic, None,
                                diff_lines=diff_lines,
                                test_runner_diagnostic=test_runner_diagnostic,
                                baseline_scope=baseline_scope,
                                mutants_scope=mutants_scope,
                                mutant_scope_diagnostic=mutant_scope_diagnostic,
                                pre_gate_diagnostic=diagnostic,
                                copy_fidelity_diagnostic=copy_fidelity_diagnostic)

            # No `if stack == "rust"` guard: `needs_pre_gate` is only ever
            # True when stack is "rust" (the dotnet branch of
            # `_resolve_mutant_scope`'s tuple hard-codes it False) -- inside
            # this block that condition is already established, so a
            # second check here would be unreachable, untestable code.
            extra_args, unknown_config_keys = _resolve_pre_gate_extra_args(root)
            if unknown_config_keys:
                # retry-3 BLOCKING-2: refuse rather than run a pre-gate
                # that knowingly diverges from the mutants' real command --
                # the invariant that stops the defect class reappearing a
                # fourth time when cargo-mutants adds a key.
                return _pre_gate_error_report(
                    "pre-gate refused: .cargo/mutants.toml sets "
                    + ", ".join(unknown_config_keys) + " -- neither "
                    "reproducible in the pre-gate's argv nor known-inert "
                    "for fidelity purposes (verify against `cargo mutants "
                    "--emit-schema=config`); running a knowingly different "
                    "command than the mutants would risk a silent false "
                    "pass, so the gate refuses instead of guessing")

            # issue #130, third dimension: resolved AFTER the unknown-key
            # refusal (operator-approved ordering) -- a repo whose config
            # is already being refused for the argv dimension does not
            # also need a tree-fidelity verdict computed for it.
            campaign_args, copy_fidelity_refusal = _resolve_copy_fidelity_args(root)
            if copy_fidelity_refusal:
                return _pre_gate_error_report(copy_fidelity_refusal)
            copy_fidelity_diagnostic = _COPY_FIDELITY_DIAGNOSTIC

            pre_gate_cmd = build_pre_gate_command(nextest, extra_args)
            # `timeout` reuses run_gate's own --timeout value (retry-1,
            # operator-mandated open question 2): the pre-gate IS one test-
            # suite run, the same kind of wait --timeout already bounds for
            # the real campaign, so a second, separate bound would be a
            # distinct config knob for materially the same wait -- YAGNI
            # absent a stated need for a different one. A hanging pre-gate
            # must not hang the whole gate: `TimeoutExpired` is reported as
            # `verdict:"error"`, never left to hang, never silently "pass".
            try:
                pre_gate_proc = subprocess.run(
                    pre_gate_cmd, cwd=root, capture_output=True, text=True,
                    timeout=timeout)
            except subprocess.TimeoutExpired:
                return _pre_gate_error_report(
                    f"pre-gate test suite timed out after {timeout}s "
                    "(reusing run_gate's own --timeout) -- the mutation "
                    "gate refuses to run the mutants' own (wider) scope "
                    "against a suite that never finished",
                    copy_fidelity_diagnostic=copy_fidelity_diagnostic)
            if pre_gate_proc.returncode != 0:
                diagnostic = _pre_gate_diagnostic(
                    pre_gate_proc.returncode, pre_gate_proc.stdout,
                    pre_gate_proc.stderr)
                return _pre_gate_error_report(
                    diagnostic, copy_fidelity_diagnostic=copy_fidelity_diagnostic)

        cmd = build_command(stack, work_class, base_ref, nextest, timeout, root,
                             copy_fidelity_args=campaign_args)
        run_started_at = time.time()
        proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)

        report_path = _locate_report(stack, root, not_before=run_started_at)
        if report_path is None:
            no_report_verdict, no_report_error, skipped = _classify_missing_report(
                cmd, proc.returncode, diff_lines)
            return _report(work_class, base_ref, stack, None, blocking, scope,
                            False, no_report_verdict, no_report_error, skipped,
                            diff_lines=diff_lines,
                            test_runner_diagnostic=test_runner_diagnostic,
                            baseline_scope=baseline_scope,
                            mutants_scope=mutants_scope,
                            mutant_scope_diagnostic=mutant_scope_diagnostic,
                            copy_fidelity_diagnostic=copy_fidelity_diagnostic)

        text = Path(report_path).read_text(encoding="utf-8")
        normalized = (parse_cargo_mutants(text) if stack == "rust"
                      else parse_stryker(text))
        verdict_info = decide(normalized, work_class, tool_exit_code=proc.returncode)
        error = None
        if verdict_info["verdict"] == "error":
            error = _tool_diagnostic(proc.returncode, proc.stdout, proc.stderr)
        return _report(work_class, base_ref, normalized["stack"], normalized["tool"],
                        verdict_info["blocking"], verdict_info["scope"], True,
                        verdict_info["verdict"], error, None,
                        killed=normalized["killed"], survived=normalized["survived"],
                        timeout_count=normalized["timeout"], unviable=normalized["unviable"],
                        survivors=normalized["survivors"],
                        mutants_generated=normalized.get("mutants_generated", 0),
                        diff_lines=diff_lines,
                        test_runner_diagnostic=test_runner_diagnostic,
                        baseline_scope=baseline_scope,
                        mutants_scope=mutants_scope,
                        mutant_scope_diagnostic=mutant_scope_diagnostic,
                        copy_fidelity_diagnostic=copy_fidelity_diagnostic)
    except Exception as exc:
        return _report(work_class, base_ref, None, None, blocking, scope,
                        False, "error", str(exc), None)


def _classify_missing_report(cmd, tool_exit_code, diff_lines):
    """Decide what "the mutation tool wrote no report" MEANS, returning
    `(verdict, error, skipped_reason)`.

    cargo-mutants exits 0 and creates no `mutants.out/` at all on every
    zero-mutant early exit -- `Diff changes no Rust source files`, `No
    mutants to filter`, `Diff file is empty` (verified against cargo-mutants
    27.1.0). Reporting all of those as `verdict: "error", "is the mutation
    tool installed?"` was a false diagnostic on a machine where the tool IS
    installed, and it exited the CLI 1, failing every cycle whose diff
    happens to land entirely outside the crate.

    Two facts already in hand separate the skip from the failure, with no
    extra subprocess and no matching on the tool's log wording (which is not
    a stable interface):

    - `tool_exit_code` -- a MISSING tool makes `cargo` itself exit non-zero
      (`no such command: mutants`), and so does a crash. Only a clean exit
      can be a skip. This is the load-bearing half: it is what keeps the
      fail-closed direction intact.
    - `diff_lines` -- an EMPTY diff (0 lines, or `None` on the .NET path
      where no patch is written) is not proof that nothing was mutable; it
      is equally the signature of a base-ref that resolved to the wrong
      thing, so it stays `"error"`. Only a genuinely non-empty diff that
      the tool then found nothing to mutate in is a real skip.

    The skip is reported the same way the Blazor exclusion already is:
    `ran: False`, `verdict: "not-applicable"`, a named `skipped_reason`, and
    CLI exit 0 -- a deliberate non-failing skip, never a pass. It can never
    satisfy the TDD-provenance relaxation triple (`ran && blocking &&
    verdict == "pass"`), so a cycle that skipped the gate still owes the
    full provenance ceremony.

    A `.cargo/mutants.toml` that redirects `output` elsewhere would also
    reach here with a clean exit and a real diff, and would be labelled
    `not-applicable` rather than the pass it actually was. That mislabels a
    green run as a skip -- never the reverse -- and the PR-time hook, which
    locates the report itself, stays fail-closed on it either way.
    """
    if tool_exit_code == 0 and diff_lines:
        return "not-applicable", None, "no-mutable-lines-in-scope"
    if tool_exit_code == 0:
        return "error", (
            f"{cmd[0]!r} exited 0 but wrote no mutation report, and the diff "
            f"against the base ref is empty ({diff_lines} lines) -- nothing "
            "was ever in scope to mutate, which is a base-ref that resolved "
            "to the wrong thing far more often than it is a real no-op"), None
    return "error", (
        f"no mutation report found after running {cmd[0]!r}, which exited "
        f"{tool_exit_code} -- is the mutation tool installed?"), None


def _report(work_class, base_ref, stack, tool, blocking, scope, ran, verdict,
            error, skipped_reason, killed=0, survived=0, timeout_count=0,
            unviable=0, survivors=None, mutants_generated=0, diff_lines=None,
            test_runner_diagnostic=None, baseline_scope=None,
            mutants_scope=None, pre_gate_diagnostic=None,
            mutant_scope_diagnostic=None, copy_fidelity_diagnostic=None):
    """Assemble the ONE `mutation-report/v1` shape (AC-T3.6) — every return
    path in `run_gate()` and `main()`'s `--parse-only` branch funnels through
    this, so the report's field set never drifts between the success, skip,
    and error paths.

    `mutants_generated` (issue #121) and `diff_lines` are additive,
    backward-compatible fields: existing consumers reading `killed`/
    `survived`/`timeout`/`unviable`/`verdict` are unaffected. `diff_lines` is
    only ever populated on the Rust path (only `_write_diff_patch` produces
    it) — it stays `None` for dotnet (Stryker scopes itself via `--since`,
    no diff patch is written) and for any path that returns before a real
    run was attempted.

    `test_runner_diagnostic` (issue #138) is the third additive field: the
    `_resolve_nextest` reason string, "making the choice visible" so a
    report consumer never has to re-derive which Rust test runner ran and
    why. `None` for dotnet (no such choice exists) and for any path that
    returns before `_resolve_nextest` runs.

    `baseline_scope` / `mutants_scope` (issue #130 slice 1, AC-2) are the
    `_resolve_mutant_scope` outputs — where the baseline runs vs. where the
    mutants actually run, so a consumer can see the #130 asymmetry without
    opening `mutants.out/log/`. `mutant_scope_diagnostic` (retry-2 fold-in 6)
    is the THIRD element of that same `_resolve_mutant_scope` return tuple —
    the reason string explaining WHY `mutants_scope`/`baseline_scope` came
    out the way they did (e.g. naming `test_workspace = true` and the
    upstream `lab.rs` line it read the asymmetry from). Without it,
    `_pre_gate_diagnostic`'s own text ("see this report's mutants_scope/
    baseline_scope for why the pre-gate ran") pointed a reader at two bare
    labels that do not themselves explain anything — this field is what
    makes that cross-reference true. `pre_gate_diagnostic` (slice 2,
    AC-1/AC-3) is populated only when the pre-gate ran AND found the
    environment red; `None` when no pre-gate was needed, when it passed, or
    on any path that returns before scope resolution runs.

    `copy_fidelity_diagnostic` (issue #130, third dimension — tree-content
    fidelity, distinct from `mutant_scope_diagnostic`'s argv-scope
    fidelity) is populated whenever `needs_pre_gate` was true AND the gate
    did not refuse on either the unknown-argv-key or `gitignore=true`
    branch — i.e. whenever `run_gate` forced `--copy-vcs=true` onto the
    campaign's own argv (see `_resolve_copy_fidelity_args`). It stays
    populated on EVERY subsequent return path once that force happened
    (pre-gate timeout, pre-gate red, report-not-found, and the final
    success report) — the tree-fidelity measures taken are the same
    regardless of what happens afterward, so a consumer reading any of
    those reports learns them. `None` when no pre-gate was needed at all,
    when the gate refused before reaching the force (unknown key or
    `gitignore=true` — that refusal's own reason lives in
    `pre_gate_diagnostic`/`error` instead), on the dotnet path (no tree-copy
    analogue), and for `--parse-only`.

    All five diagnostic-shaped fields above stay `None` on the dotnet path
    (Stryker has no package scoping; it frames via `--since`) and for
    `--parse-only` (no repo to resolve scope against), exactly like
    `test_runner_diagnostic` today.
    """
    return {
        "schema": "mutation-report/v1",
        "ran": ran,
        "stack": stack,
        "tool": tool,
        "work_class": work_class,
        "scope": scope,
        "base_ref": base_ref,
        "blocking": blocking,
        "killed": killed,
        "survived": survived,
        "timeout": timeout_count,
        "unviable": unviable,
        "mutants_generated": mutants_generated,
        "diff_lines": diff_lines,
        "test_runner_diagnostic": test_runner_diagnostic,
        "baseline_scope": baseline_scope,
        "mutants_scope": mutants_scope,
        "mutant_scope_diagnostic": mutant_scope_diagnostic,
        "pre_gate_diagnostic": pre_gate_diagnostic,
        "copy_fidelity_diagnostic": copy_fidelity_diagnostic,
        "survivors": survivors or [],
        "verdict": verdict,
        "error": error,
        "skipped_reason": skipped_reason,
    }


def _reject_unsafe_ref(base_ref):
    """Refuse a `--base-ref` that looks like a flag (retry BLOCKING 1,
    Security HIGH). Reproduced for real on git 2.50.1: on a dirty tree,
    `base_ref = "--output=/tmp/PWNED"` turns `_write_diff_patch`'s
    `f"{base_ref}..."` into the single argv token `--output=/tmp/PWNED...`,
    which git happily parses as `--output=<path>` and writes the diff to an
    ARBITRARY file, exiting 0 -- silently producing an empty patch (the
    redirected stdout is empty) that would feed cargo-mutants zero mutants,
    chaining straight into the vacuous-pass hole BLOCKING 2 closes.
    `--end-of-options` on the `git diff` invocation is the primary fix; this
    is the belt-and-suspenders half, checked BEFORE any subprocess runs so a
    malformed `--base-ref` never reaches git at all, for either stack (the
    dotnet `--since:<base_ref>` token is a single argv element today and not
    separately injectable, but a future refactor that splits it silently
    reopens this -- checking here, once, protects both)."""
    if base_ref.startswith("-"):
        raise ValueError(
            f"unsafe --base-ref (looks like a flag, refusing): {base_ref!r}")


def _refuse_if_symlink(path):
    """Raise `ValueError` if `path` is itself a symlink (`Path.is_symlink()`
    does NOT follow the link -- it inspects the path entry itself). Fold-in,
    Security LOW: without this, a malicious PR branch could plant a
    symlinked `.mutation-gate` directory or `changes.patch` file and redirect
    `_write_diff_patch`'s write to an arbitrary target the CI process can
    reach."""
    if Path(path).is_symlink():
        raise ValueError(f"refusing to write through a symlink: {path}")


def _write_diff_patch(root, base_ref):
    """Write `git diff --end-of-options <base_ref>...` (AC-T3.2's invocation,
    hardened per `_reject_unsafe_ref`'s docstring) to the deterministic patch
    path `build_command`'s `--in-diff` reads.

    Returns the line count of the diff just written (issue #121: "the patch
    is already written here, so the line count is free") — `run_gate()`
    surfaces it in the report as `diff_lines`, so a consumer can sanity-check
    a campaign's scope (a 5-mutant run against an 18-file, +1376-line diff is
    visibly wrong) without a second git invocation or knowledge of the repo.
    """
    patch_path = Path(diff_patch_path(root))
    _refuse_if_symlink(patch_path.parent)
    patch_path.parent.mkdir(parents=True, exist_ok=True)
    _refuse_if_symlink(patch_path)
    result = subprocess.run(
        ["git", "diff", "--end-of-options", f"{base_ref}..."], cwd=str(root),
        capture_output=True, text=True, check=True)
    patch_path.write_text(result.stdout, encoding="utf-8")
    return len(result.stdout.splitlines())


def _locate_report(stack, root, not_before=None):
    """Find the mutation tool's own report file, or `None` if it never
    appeared (AC-T3.7's actual proof of "did the tool really run": a report
    file existing is stronger evidence than a subprocess exit code, because
    cargo-mutants legitimately exits non-zero when mutants are found missed).

    `not_before` (fold-in, Security MEDIUM): when given, a candidate whose
    mtime predates it is treated as not found. Without this, a crashed run
    on a reused/cached workspace could silently re-report a STALE report
    left over from a prior invocation — possibly a clean one — as this run's
    verdict. `run_gate()` always passes the timestamp captured just before
    the subprocess launch; `--parse-only` and any other direct caller that
    omits it gets the unfiltered original behaviour (there is no "this run"
    to be stale relative to)."""
    if stack == "rust":
        candidate = Path(root) / "mutants.out" / "outcomes.json"
        if not candidate.is_file():
            return None
        if not_before is not None and candidate.stat().st_mtime < not_before:
            return None
        return candidate
    if stack == "dotnet":
        matches = sorted(Path(root).glob("StrykerOutput/*/reports/mutation-report.json"))
        if not_before is not None:
            matches = [m for m in matches if m.stat().st_mtime >= not_before]
        return matches[-1] if matches else None
    return None


def _all_dotnet_projects_are_blazor(root):
    """True only when at least one `*.csproj` exists under `root` AND every
    one of them is a Blazor project (AC-T3.2's Blazor skip). A mixed
    solution (some Blazor, some not) is NOT skipped — Stryker scopes itself
    via `--since`, and skipping the whole solution would hide real,
    gate-able C# alongside the excluded Blazor project."""
    csproj_files = [Path(p) for p in _find_all_files(Path(root), (".csproj",))]
    if not csproj_files:
        return False
    return all(is_blazor_project(p.read_text(encoding="utf-8")) for p in csproj_files)


def main():
    """Thin argparse CLI. Never raises; the exit code carries the verdict.

    `--root` is mandatory for a real run (never inferred from cwd — C11: the
    tool lives in this config repo but always executes against a downstream
    project repo passed explicitly). `--parse-only <path>` skips the
    subprocess pipeline entirely and runs the two adapters + `decide()` over
    an existing report file — the seam that lets a REAL downstream report
    validate the parsers against reality (AC-T3.8).

    Exit code: 0 for verdict "pass", "advisory-survivors" (fold-in — a
    non-blocking run with real survivors is still non-blocking), or
    "not-applicable" (a deliberate, non-failing skip); 1 for "fail", "empty"
    (retry BLOCKING 2 — a blocking run that validly exercised zero mutants
    is never a silent pass), or "error". There is no `--allow-missing-tool`
    — see the module docstring.
    """
    parser = argparse.ArgumentParser(
        description="Stack-detecting, diff-scoped mutation-testing gate.")
    parser.add_argument("--root", default=None,
                         help="downstream project repo to audit (mandatory "
                              "for a real run, unused with --parse-only)")
    parser.add_argument("--work-class", required=True,
                         choices=sorted(_DEPTH_BY_WORK_CLASS),
                         help="drives scope + blocking depth (AC-T3.4)")
    parser.add_argument("--base-ref", default="origin/main",
                         help="git ref the diff/--since is computed against")
    parser.add_argument("--timeout", type=int, default=300,
                         help="cargo-mutants --timeout, in seconds (always "
                              "emitted on the Rust command) -- ALSO reused, "
                              "unmodified, as the pre-gate's own whole-suite "
                              "wall-clock bound (issue #130 retry-1): there "
                              "cargo-mutants' per-mutant budget becomes a "
                              "single, one-shot timeout on the pre-gate's "
                              "entire `cargo test --workspace` run")
    parser.add_argument("--parse-only", metavar="PATH", default=None,
                         help="skip the subprocess pipeline; run the parser "
                              "+ decide() over an existing report file")
    parser.add_argument("--stack", choices=("auto", "rust", "dotnet"), default="auto",
                         help="disambiguate --parse-only's report format; "
                              "ignored on a real run (stack is detected)")
    parser.add_argument("--json", action="store_true",
                         help="print the machine-readable report")
    args = parser.parse_args()

    if args.parse_only:
        try:
            text = Path(args.parse_only).read_text(encoding="utf-8")
            stack = args.stack
            if stack == "auto":
                stack = _detect_report_stack(text)
            if stack == "rust":
                normalized = parse_cargo_mutants(text)
            elif stack == "dotnet":
                normalized = parse_stryker(text)
            else:
                raise ValueError(f"unknown --stack: {stack!r}")
            # No live subprocess here (this is the seam that validates the
            # parsers against an already-real downstream report -- see the
            # module docstring), so `decide()` gets no `tool_exit_code` and
            # keeps its default (0): a vacuous report is classified "empty",
            # never "error" -- there is no crash to report on this path.
            verdict_info = decide(normalized, args.work_class)
            report = _report(
                args.work_class, args.base_ref, normalized["stack"], normalized["tool"],
                verdict_info["blocking"], verdict_info["scope"], True,
                verdict_info["verdict"], None, None,
                killed=normalized["killed"], survived=normalized["survived"],
                timeout_count=normalized["timeout"], unviable=normalized["unviable"],
                survivors=normalized["survivors"],
                mutants_generated=normalized.get("mutants_generated", 0))
        except Exception as exc:
            report = _report(args.work_class, args.base_ref, None, None,
                              False, None, False, "error", str(exc), None)
    else:
        if not args.root:
            parser.error("--root is required unless --parse-only is given")
        report = run_gate(args.root, args.work_class, args.base_ref, args.timeout)

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print(f"mutation-gate: {report['verdict']} "
              f"(killed={report['killed']} survived={report['survived']} "
              f"timeout={report['timeout']} unviable={report['unviable']} "
              f"blocking={report['blocking']})")

    sys.exit(0 if report["verdict"] in ("pass", "advisory-survivors", "not-applicable") else 1)


def _detect_report_stack(report_text):
    """Sniff whether a raw report file's JSON shape is cargo-mutants'
    `outcomes.json` or Stryker's mutation-testing-elements report, so
    `--parse-only` does not require an explicit `--stack` for the common
    case. Raises `ValueError` on anything else — `--parse-only` must never
    guess silently on an unrecognized shape."""
    data = json.loads(report_text)
    if "outcomes" in data and "total_mutants" in data:
        return "rust"
    if "files" in data and "schemaVersion" in data:
        return "dotnet"
    raise ValueError("could not detect report format: expected either a "
                      "cargo-mutants outcomes.json (\"outcomes\"/\"total_mutants\") "
                      "or a Stryker mutation-report.json (\"files\"/\"schemaVersion\") "
                      "shape -- pass --stack explicitly")


if __name__ == "__main__":
    main()
