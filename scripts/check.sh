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

CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"
SCENARIO_AUDIT="$CLAUDE_HOME/lib/scenario_audit.py"

WORK_CLASS="${1:-}"
BASE_REF="${2:-}"
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

scenario_gate() {
    if [ ! -f "$SCENARIO_AUDIT" ]; then
        echo "scenario gate not found at $SCENARIO_AUDIT (set CLAUDE_HOME)" >&2
        return 2
    fi
    python3 "$SCENARIO_AUDIT" --root "$ROOT" \
        --tests-root core/src --tests-root app/src
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
# Heuristic, not a parser: a single-line `pub fn new(...) -> Type {` grep
# across every core/src/**/domain/**/*.rs, excluding a bare `-> Self` return
# (a trivial passthrough constructor computes/validates nothing, so it loses
# nothing by being skipped). This WILL miss a `new` whose signature wraps
# onto multiple lines and cannot see through a type alias — read the domain
# files directly for anything this shape doesn't fit. Honest-heuristic over
# false-parser is a deliberate choice, not an oversight.
new_constructor_advisory() {
    local domain_dirs
    domain_dirs="$(find core/src -type d -name domain 2>/dev/null)"
    printf 'constructors invisible to mutation testing (cargo-mutants hard-skips any fn literally named "new"):\n'
    if [ -z "$domain_dirs" ]; then
        printf '  no domain/ directory found under core/src\n'
        return 0
    fi
    local hits
    # shellcheck disable=SC2086 # deliberate word-splitting: domain_dirs is a
    # newline-separated list of directories, one grep argument each.
    hits="$(grep -rnE 'pub fn new\([^)]*\)[[:space:]]*->[[:space:]]*[A-Za-z_][A-Za-z0-9_:<, >]*[[:space:]]*\{' \
        $domain_dirs --include='*.rs' 2>/dev/null | grep -vE '\->[[:space:]]*Self[[:space:]]*\{')"
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
run_gate "tests" cargo test --quiet
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
