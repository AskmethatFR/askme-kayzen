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

# shellcheck source=scripts/verify-instrument.sh
source "$ROOT/scripts/verify-instrument.sh"

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

if ! verify_vendored_instrument "$ROOT/scripts/vendor" "mutation_gate.py"; then
    echo "mutation gate failed provenance verification, refusing to run" >&2
    exit 2
fi

if ! cargo mutants --version >/dev/null 2>&1; then
    echo "cargo-mutants is not installed: cargo install cargo-mutants --locked" >&2
    exit 2
fi

# Not `exec`: a truncated/empty instrument would exit 0 with no output and
# tail-calling it would hand that straight back as a silent pass. Captured
# and checked for the mutation-report/v1 payload it is contracted to emit,
# same defense as the scenario gate's verdict-line check.
tmp_output="$(mktemp)"
set +e
python3 "$GATE" \
    --root "$ROOT" \
    --work-class "$WORK_CLASS" \
    --base-ref "$BASE_REF" \
    --json | tee "$tmp_output"
status=${PIPESTATUS[0]}
set -e

if [[ "$status" -ne 0 ]]; then
    rm -f "$tmp_output"
    exit "$status"
fi

if ! grep -q '"schema": "mutation-report/v1"' "$tmp_output"; then
    echo "mutation gate exited 0 but produced no mutation-report/v1 payload -- refusing to trust it" >&2
    rm -f "$tmp_output"
    exit 1
fi

rm -f "$tmp_output"
