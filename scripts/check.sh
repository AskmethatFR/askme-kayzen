#!/usr/bin/env bash
# Every gate this repo commits to, in one run.
#
# Usage: scripts/check.sh [work-class]
#   no argument : formatting, lints, tests, the scenario gate, the doc anchors
#   work-class  : quick-change | fix-bug | new-feature — also runs the mutation
#                 gate (scripts/mutation-gate.sh), which reads the COMMITTED diff
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

run_gate "formatting" cargo fmt --check
run_gate "lints" cargo clippy --all-targets --quiet -- -D warnings
run_gate "tests" cargo test --quiet
run_gate "scenarios" scenario_gate
run_gate "doc anchors" doc_anchors

if [ -n "$WORK_CLASS" ]; then
    run_gate "mutation" "$ROOT/scripts/mutation-gate.sh" "$WORK_CLASS"
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
