#!/usr/bin/env bash
# Every gate this repo commits to, in one run.
#
# Usage: scripts/check.sh [work-class] [base-ref]
#   no argument : formatting, lints, tests, the scenario gate, the doc anchors
#   work-class  : quick-change | fix-bug | new-feature — also runs the mutation
#                 gate (scripts/mutation-gate.sh), which reads the COMMITTED diff
#   base-ref    : MANDATORY whenever work-class is given — the commit
#                 immediately before this slice's first RED commit. Passed
#                 through as a CLI argument, never defaulted or guessed here:
#                 it is a per-slice value (it moves every time you start a
#                 new TDD slice), so a fixed env-var default would go stale
#                 as often as it helped. Omitting it is not a silent skip —
#                 scripts/mutation-gate.sh itself refuses to run without a
#                 base-ref and reports "mutation" as failed below.
#
# Every gate runs even after an earlier one failed: one run tells you everything
# that is broken, not just the first thing. Exit 1 as soon as any gate failed.

# Deliberately NOT `set -e`: this script's whole job is to keep going after a
# failure and report the full picture. Each gate's status is captured explicitly
# instead, and no gate is ever invoked as an `if` condition — bash suspends
# errexit inside a function called that way, which is exactly how a runner ends
# up swallowing the failures it exists to surface.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 2

# shellcheck source=scripts/verify-instrument.sh
source "$ROOT/scripts/verify-instrument.sh"

SCENARIO_AUDIT="$ROOT/scripts/vendor/scenario_audit.py"
SHELL_UNIT_TESTS="$ROOT/scripts/test-shell-units.sh"

WORK_CLASS="${1:-}"
BASE_REF="${2:-}"

# A work-class with no base-ref is a malformed invocation, not a measurement
# to report — fail fast here, before any gate runs, instead of burning two to
# four minutes only to hand back scripts/mutation-gate.sh's own usage message
# at an unrelated absolute path. This does not contradict the no-`set -e`,
# run-every-gate design above: that design is about *gates* each reporting
# their own result so one run shows the full picture. A CLI argument error
# means there is nothing yet to measure, so failing fast and running every
# gate to completion are complementary, not in tension. `check.sh` with no
# arguments at all is unaffected — that is the pre-existing, deliberate
# mutation-skip path below, not an error.
if [ -n "$WORK_CLASS" ] && [ -z "$BASE_REF" ]; then
    echo "usage: scripts/check.sh [work-class] [base-ref]" >&2
    echo "base-ref is mandatory with a work-class: the commit immediately" >&2
    echo "before this slice's first RED commit (e.g. \$(git rev-parse <red-commit>^))." >&2
    exit 2
fi

FAILED=()

run_gate() {
    local name="$1"
    shift
    printf '\n=== %s ===\n' "$name"
    "$@"
    local status=$?
    if [ "$status" -ne 0 ]; then
        FAILED[${#FAILED[@]}]="$name"
    fi
}

# Every `path/to/file.rs` cited in the knowledge graph must still exist. The
# graph's code anchors are the only bridge from a domain term to the code that
# implements it; a dead one sends its reader nowhere, silently.
doc_anchors() {
    local missing=0
    local path
    while IFS= read -r path; do
        if [ ! -f "$path" ]; then
            echo "dead code anchor in docs/: $path"
            missing=1
        fi
    done < <(grep -rhoE '`((core|app)/)?src/[A-Za-z0-9_/.-]+\.rs`' docs/ \
             | tr -d '`' | sort -u)
    if [ "$missing" -eq 0 ]; then
        echo "every code anchor in docs/ resolves"
    fi
    return "$missing"
}

# Lints the Android arm the way `lints` above lints the default target: a
# typo inside `#[cfg(target_os = "android")]` code compiles clean today and
# is caught by nothing else in this file, because no gate here ever builds
# for that target.
#
# What this proves, and what it does not:
#   - compiles and lints cleanly;
#   - it never LINKS and never EXECUTES, so a missing NDK symbol or any
#     runtime failure is invisible to it;
#   - JNI method signature strings (e.g. "()Ljava/io/File;") are resolved
#     at runtime by the JVM, not by rustc or clippy — a typo in one of
#     those strings compiles clean here and only fails on a real device.
#     That is the largest residual risk on this platform arm, and no
#     compile-only gate can close it;
#   The bar this gate meets is "a clean cross-target build plus manual
#   verification, stated as such and never presented as coverage" — see
#   docs/technical/architecture.md's own wording for the web arm.
#
# `--no-default-features` is mandatory: the default `web` feature pulls in
# `dioxus/web`, which does not compile for Android. `clippy`/`check` stop
# before linking, so no NDK and no `WRY_ANDROID_*` env var is required —
# wry's Kotlin-generation build step is gated behind
# `WRY_ANDROID_KOTLIN_FILES_OUT_DIR`, which stays unset here.
android_cross_target() {
    if ! rustup target list --installed | grep -qx aarch64-linux-android; then
        echo "missing Rust target: rustup target add aarch64-linux-android" >&2
        return 2
    fi
    cargo clippy --target aarch64-linux-android -p kayzen-app \
        --no-default-features --features mobile --all-targets --quiet \
        -- -D warnings
}

# Same verdict-line contract as scenario_gate below: a house harness that
# is missing, not executable, or exits 0 without its own "shell-units:"
# line is never trusted as a pass.
shell_unit_gate() {
    if [ ! -f "$SHELL_UNIT_TESTS" ] || [ ! -x "$SHELL_UNIT_TESTS" ]; then
        echo "shell unit harness not found or not executable at $SHELL_UNIT_TESTS" >&2
        return 2
    fi

    local tmp_output
    tmp_output="$(mktemp)"
    "$SHELL_UNIT_TESTS" | tee "$tmp_output"
    local status=${PIPESTATUS[0]}
    if [ "$status" -ne 0 ]; then
        rm -f "$tmp_output"
        return "$status"
    fi
    if ! grep -q 'shell-units:' "$tmp_output"; then
        echo "shell unit gate exited 0 but produced no verdict line -- refusing to trust it" >&2
        rm -f "$tmp_output"
        return 1
    fi
    rm -f "$tmp_output"
    return 0
}

scenario_gate() {
    if [ ! -f "$SCENARIO_AUDIT" ]; then
        echo "scenario gate not found at $SCENARIO_AUDIT (vendored copy missing)" >&2
        return 2
    fi
    if ! verify_vendored_instrument "$ROOT/scripts/vendor" "scenario_audit.py"; then
        echo "scenario gate failed provenance verification, refusing to run" >&2
        return 2
    fi

    local tmp_output
    tmp_output="$(mktemp)"
    python3 "$SCENARIO_AUDIT" --root "$ROOT" \
        --tests-root core/src --tests-root app/src | tee "$tmp_output"
    local status=${PIPESTATUS[0]}
    if [ "$status" -ne 0 ]; then
        rm -f "$tmp_output"
        return "$status"
    fi
    # A truncated/empty instrument exits 0 and prints nothing -- that must
    # not read as "pass". Require the verdict line it is contracted to emit.
    if ! grep -q 'scenario-audit:' "$tmp_output"; then
        echo "scenario gate exited 0 but produced no verdict line -- refusing to trust it" >&2
        rm -f "$tmp_output"
        return 1
    fi
    rm -f "$tmp_output"
    return 0
}

# TN-1b — cargo-mutants 27.1.0 hard-skips any function literally named `new`
# (its src/visit.rs excludes `i.sig.ident == "new"` before mutants are even
# generated — no CLI flag or config key overrides it). Every domain invariant
# in this repo lives in exactly such a constructor, so a regression in any
# one of them would still pass the mutation gate as a clean zero-survivor
# "pass" — this lists the blind spot so it stays visible until the rename
# (to e.g. `parse`) is decided, which is a public-API change out of this
# check's scope. ADVISORY ONLY: this never appends to FAILED and never
# affects the exit code.
#
# Heuristic, not a parser: a single-line `fn new(...) -> Type {` grep across
# every core/src/**/*.rs, filtered down to paths containing `/domain/`,
# excluding a bare `-> Self` return (a trivial passthrough constructor
# computes/validates nothing, so it loses nothing by being skipped). The
# pattern is visibility-agnostic — matching cargo-mutants' own skip, which
# doesn't check visibility either — so a private `fn new` is caught too.
# This WILL miss a `new` whose signature wraps onto multiple lines and
# cannot see through a type alias — read the domain files directly for
# anything this shape doesn't fit. Honest-heuristic over false-parser is a
# deliberate choice, not an oversight.
new_constructor_advisory() {
    local domain_dirs
    domain_dirs="$(find core/src -type d -name domain 2>/dev/null)"
    printf 'domain constructors invisible to mutation testing (cargo-mutants hard-skips any fn literally named "new"):\n'
    if [ -z "$domain_dirs" ]; then
        printf '  no domain/ directory found under core/src\n'
        return 0
    fi
    # Belt-and-braces, checked explicitly rather than inferred from
    # domain_dirs above: BSD grep (macOS) silently exits 1 — the same code
    # as "no matches" — when `-r --include` targets a directory that does
    # not exist, with nothing on stderr either. That is exactly the failure
    # this function must never mistake for a clean result, so the
    # precondition is tested directly instead of trusted to grep's exit code.
    if [ ! -d core/src ]; then
        printf '  search FAILED: core/src does not exist — cannot report "none found" from an unscanned target\n'
        return 1
    fi
    local raw_hits grep_status hits
    raw_hits="$(grep -rnE '(pub )?fn new\([^)]*\)[[:space:]]*->[[:space:]]*[A-Za-z_][A-Za-z0-9_:<, >]*[[:space:]]*\{' \
        core/src --include='*.rs' 2>/dev/null)"
    grep_status=$?
    if [ "$grep_status" -gt 1 ]; then
        printf '  search FAILED (grep exit %d scanning core/src) — this is "could not look", not "none found"; the list below is NOT reliable\n' "$grep_status"
        return 1
    fi
    hits="$(printf '%s\n' "$raw_hits" | grep -E '/domain/' | grep -vE '\->[[:space:]]*Self[[:space:]]*\{')"
    if [ -z "$hits" ]; then
        printf '  none found\n'
    else
        printf '%s\n' "$hits" | sed 's/^/  /'
    fi
    printf '  (heuristic grep, single-line signatures only — verify by reading the file if in doubt)\n'
    return 0
}

run_gate "formatting" cargo fmt --check
run_gate "lints" cargo clippy --all-targets --quiet -- -D warnings
run_gate "android cross-target" android_cross_target
run_gate "tests" cargo test --quiet
run_gate "shell units" shell_unit_gate
run_gate "scenarios" scenario_gate
run_gate "doc anchors" doc_anchors

printf '\n=== new-constructor blind spot (advisory) ===\n'
new_constructor_advisory

if [ -n "$WORK_CLASS" ]; then
    run_gate "mutation" "$ROOT/scripts/mutation-gate.sh" "$WORK_CLASS" "$BASE_REF"
else
    printf '\n=== mutation ===\nskipped: pass a work-class to run it\n'
fi

printf '\n=== summary ===\n'
if [ "${#FAILED[@]}" -eq 0 ]; then
    echo "all gates green"
    exit 0
fi

echo "failed: ${FAILED[*]}"
exit 1
