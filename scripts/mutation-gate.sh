#!/usr/bin/env bash
# Diff-scoped mutation gate for this workspace.
#
# Usage: scripts/mutation-gate.sh <work-class> [base-ref]
#   work-class : quick-change | fix-bug | new-feature
#   base-ref   : git ref the diff is computed against (default: HEAD~1, the
#                last commit — development happens on master here, there is no
#                base branch to diff against)
#
# The diff is `git diff <base-ref>...`, so it covers COMMITTED work only:
# commit the slice before running the gate on it.
#
# The gate itself lives in ~/.claude/lib/mutation_gate.py: it produces the diff
# patch, runs cargo-mutants against it, and normalises the outcome into a
# mutation-report/v1 JSON. quick-change is advisory, fix-bug and new-feature
# block on a survivor.

set -euo pipefail

CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"
GATE="$CLAUDE_HOME/lib/mutation_gate.py"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

WORK_CLASS="${1:-}"
BASE_REF="${2:-HEAD~1}"

if [[ -z "$WORK_CLASS" ]]; then
    echo "usage: $0 <quick-change|fix-bug|new-feature> [base-ref]" >&2
    exit 2
fi

if [[ ! -f "$GATE" ]]; then
    echo "mutation gate not found at $GATE (set CLAUDE_HOME)" >&2
    exit 2
fi

if ! cargo mutants --version >/dev/null 2>&1; then
    echo "cargo-mutants is not installed: cargo install cargo-mutants --locked" >&2
    exit 2
fi

exec python3 "$GATE" \
    --root "$ROOT" \
    --work-class "$WORK_CLASS" \
    --base-ref "$BASE_REF" \
    --json
