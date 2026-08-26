#!/usr/bin/env bash
# Verifies every native library packaged into an Android App Bundle sits at
# the 16 KB page-size alignment Play requires on Android 15+ devices
# (REQUIRED_PAGE_ALIGNMENT in scripts/android-release-lib.sh). Reads the
# artifact's own bytes, never the build's intention: a stale cargo cache, an
# --rustc-args that silently didn't apply, or a Gradle no-op can each leave
# a 4 KB .so behind a build that otherwise "ran fine", and none of those
# show up anywhere except here.
#
# Every base/lib/*/*.so entry is checked, not just libmain.so: a future
# native dependency ships its own .so in the same bundle and would regress
# this exact property invisibly to a check that only looked at one name.
#
# @law: entry names in a zip's central directory are attacker-controlled,
# unconstrained bytes -- duplicate, NUL- or newline-bearing names are all
# legal, and Python's zipfile.NameToInfo is a dict keyed by name (the LAST
# duplicate wins, silently). Entries below are therefore addressed by their
# ordinal position in infolist(), never by name -- the one identifier a
# crafted name cannot forge. No file is ever extracted to disk, so zip-slip
# is structurally unreachable here, not merely guarded against.
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

command -v python3 >/dev/null 2>&1 || preflight_fail "python3 not found"

READELF=""
for candidate in "${NDK_HOME:-/nonexistent}"/toolchains/llvm/prebuilt/*/bin/llvm-readelf; do
    [ -x "$candidate" ] && READELF="$candidate" && break
done
[ -n "$READELF" ] || preflight_fail "no llvm-readelf under \$NDK_HOME/toolchains/llvm/prebuilt/*/bin (NDK_HOME=${NDK_HOME:-<unset>})"

[ -f "$AAB" ] || preflight_fail "no AAB at $AAB"

list_so_indices() {
    python3 -c '
import sys, zipfile, fnmatch
zf = zipfile.ZipFile(sys.argv[1])
for i, info in enumerate(zf.infolist()):
    if fnmatch.fnmatchcase(info.filename, "base/lib/*/*.so"):
        print(i)
' "$AAB"
}

entry_name() {
    python3 -c '
import sys, zipfile
print(zipfile.ZipFile(sys.argv[1]).infolist()[int(sys.argv[2])].filename)
' "$AAB" "$1"
}

read_zip_entry() {
    python3 -c '
import sys, shutil, zipfile
zf = zipfile.ZipFile(sys.argv[1])
info = zf.infolist()[int(sys.argv[2])]
with zf.open(info) as f:
    shutil.copyfileobj(f, sys.stdout.buffer)
' "$AAB" "$1"
}

indices="$(list_so_indices)"
[ -n "$indices" ] || fail "no base/lib/*/*.so entries in $AAB"

count=0
while IFS= read -r index; do
    entry="$(entry_name "$index")"
    align="$(read_zip_entry "$index" | "$READELF" -l - | min_load_alignment)"
    [ "$align" -ge "$REQUIRED_PAGE_ALIGNMENT" ] \
        || fail "$entry is aligned to $align bytes, needs >= $REQUIRED_PAGE_ALIGNMENT"
    echo "  $entry: $align bytes"
    count=$((count + 1))
done <<< "$indices"

echo "android-verify-alignment: verified $count .so entries at >= $REQUIRED_PAGE_ALIGNMENT bytes"
