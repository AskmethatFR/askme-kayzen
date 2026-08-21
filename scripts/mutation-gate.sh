#!/usr/bin/env bash
# Diff-scoped mutation gate for this workspace.
#
# Usage: scripts/mutation-gate.sh <work-class> <base-ref>
#   work-class : quick-change | fix-bug | new-feature
#   base-ref   : MANDATORY, no default. The calling contract: base-ref is the
#                commit immediately before this slice's first RED commit —
#                never a guess, never a fallback like HEAD~1. A TDD slice
#                following this repo's GATE 2 discipline (RED test commit,
#                then GREEN implementation commit, never squashed) needs the
#                diff to start at the parent of the RED commit, or the RED
#                commit's own test additions are silently excluded from what
#                the gate measures. Only the caller (scripts/check.sh, or a
#                human) knows which commit that is for the slice at hand.
#
# The diff is `git diff <base-ref>...`, so it covers COMMITTED work only:
# commit the slice before running the gate on it.
#
# The gate itself lives in scripts/vendor/mutation_gate.py, a pinned copy of
# the operator's own instrument (see scripts/vendor/PROVENANCE): it produces
# the diff patch, runs cargo-mutants against it, and normalises the outcome
# into a mutation-report/v1 JSON. quick-change is advisory, fix-bug and
# new-feature block on a survivor.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE="$ROOT/scripts/vendor/mutation_gate.py"

WORK_CLASS="${1:-}"
BASE_REF="${2:-}"

if [[ -z "$WORK_CLASS" ]]; then
    echo "usage: $0 <quick-change|fix-bug|new-feature> <base-ref>" >&2
    exit 2
fi

if [[ -z "$BASE_REF" ]]; then
    echo "usage: $0 <quick-change|fix-bug|new-feature> <base-ref>" >&2
    echo "base-ref is mandatory — no default is guessed (was silently" >&2
    echo "HEAD~1, which under-scopes a properly split TDD RED+GREEN slice)." >&2
    echo "base-ref = the commit immediately before this slice's first RED commit." >&2
    exit 2
fi

if [[ ! -f "$GATE" ]]; then
    echo "mutation gate not found at $GATE (vendored copy missing)" >&2
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
