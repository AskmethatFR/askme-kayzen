#!/usr/bin/env bash
# Verifies every native library packaged into an Android App Bundle sits at
# the 16 KB page-size alignment Play requires on Android 15+ devices (F3,
# REQUIRED_PAGE_ALIGNMENT in scripts/android-release-lib.sh). Reads the
# artifact's own bytes, never the build's intention: a stale cargo cache, an
# --rustc-args that silently didn't apply, or a Gradle no-op can each leave
# a 4 KB .so behind a build that otherwise "ran fine", and none of those
# show up anywhere except here.
#
# Every base/lib/*/*.so entry is checked, not just libmain.so: a future
# native dependency ships its own .so in the same bundle and would regress
# this exact property invisibly to a check that only looked at one name.
#
# Usage: scripts/android-verify-alignment.sh <aab-path>

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$ROOT/scripts/android-release-lib.sh"

fail() {
    echo "android-verify-alignment: $1" >&2
    exit 1
}

# A missing tool or a missing input is never a skip (verify-instrument.sh's
# own doctrine): exit 2 keeps it distinguishable in scripts/check.sh's
# summary from an actual alignment defect (exit 1, below).
preflight_fail() {
    echo "android-verify-alignment: $1" >&2
    exit 2
}

[ $# -eq 1 ] || preflight_fail "usage: scripts/android-verify-alignment.sh <aab-path>"
AAB="$1"

command -v unzip >/dev/null 2>&1 || preflight_fail "unzip not found"

READELF=""
for candidate in "${NDK_HOME:-/nonexistent}"/toolchains/llvm/prebuilt/*/bin/llvm-readelf; do
    [ -x "$candidate" ] && READELF="$candidate" && break
done
[ -n "$READELF" ] || preflight_fail "no llvm-readelf under \$NDK_HOME/toolchains/llvm/prebuilt/*/bin (NDK_HOME=${NDK_HOME:-<unset>})"

[ -f "$AAB" ] || preflight_fail "no AAB at $AAB"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

entries="$(unzip -Z1 "$AAB" 'base/lib/*/*.so' 2>/dev/null || true)"
[ -n "$entries" ] || fail "no base/lib/*/*.so entries in $AAB"

echo "android-verify-alignment: verified"
while IFS= read -r entry; do
    align="$(unzip -p "$AAB" "$entry" | "$READELF" -l - | min_load_alignment)"
    [ "$align" -ge "$REQUIRED_PAGE_ALIGNMENT" ] \
        || fail "$entry is aligned to $align bytes, needs >= $REQUIRED_PAGE_ALIGNMENT"
    echo "  $entry: $align bytes"
done <<< "$entries"
