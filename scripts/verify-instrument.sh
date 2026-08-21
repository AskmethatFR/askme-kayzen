#!/usr/bin/env bash
# Verifies a vendored instrument's sha256 against scripts/vendor/PROVENANCE
# before check.sh or mutation-gate.sh trusts it. Existence alone (the old
# `[ ! -f ]` guard) lets a truncated or hand-edited file read as present and
# therefore green -- a PROVENANCE row nobody checks is prose, not a control
# (AD-2). Sourced by both callers rather than duplicated, so the two direct
# vendored dependencies (scenario_audit.py, mutation_gate.py) are checked by
# one piece of logic, not two that can drift apart.
#
# Usage: verify_vendored_instrument <vendor-dir> <filename>
# Returns 0 and prints nothing on a match. Returns 1 and prints a reason to
# stderr on a missing PROVENANCE, a missing row, or a mismatch.
verify_vendored_instrument() {
    local vendor_dir="$1" filename="$2"
    local provenance="$vendor_dir/PROVENANCE"

    if [ ! -f "$provenance" ]; then
        echo "PROVENANCE not found at $provenance" >&2
        return 1
    fi

    local expected
    expected="$(awk -v f="$filename" '$2 == f { print $1; exit }' "$provenance")"
    if [ -z "$expected" ]; then
        echo "$filename has no row in $provenance" >&2
        return 1
    fi

    local actual
    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$vendor_dir/$filename" 2>/dev/null | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "$vendor_dir/$filename" 2>/dev/null | awk '{print $1}')"
    else
        echo "neither sha256sum nor shasum found -- cannot verify $filename" >&2
        return 1
    fi

    if [ "$actual" != "$expected" ]; then
        echo "$filename checksum mismatch: PROVENANCE says $expected, on disk is ${actual:-<unreadable>}" >&2
        return 1
    fi

    return 0
}
