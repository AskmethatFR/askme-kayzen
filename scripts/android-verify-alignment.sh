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
# Entries are listed and read through Python's zipfile module, never `unzip`:
# `unzip` treats a member argument as a GLOB PATTERN, and entry names come
# from the archive's own central directory -- attacker-controlled in the
# "a .aab was downloaded from somewhere" model. A crafted entry name (e.g. a
# `[...]` character class) makes `unzip -p` silently return a DIFFERENT,
# benign member's bytes instead of its own, so the malicious member is never
# inspected while the script still reports success. zipfile.read(name) maps a
# name to exactly that member's bytes, nothing else -- the one property this
# check exists to rely on. This also means no file is ever extracted to disk,
# so zip-slip is structurally unreachable here, not merely guarded against.
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

# The pattern here is a fixed, trusted literal -- only the candidate names
# being tested against it come from the archive. That is the safe direction
# for glob matching; `unzip`'s bug was matching in the other one.
list_so_entries() {
    python3 -c '
import sys, zipfile, fnmatch
zf = zipfile.ZipFile(sys.argv[1])
for name in zf.namelist():
    if fnmatch.fnmatchcase(name, "base/lib/*/*.so"):
        print(name)
' "$AAB"
}

read_zip_entry() {
    python3 -c '
import sys, zipfile
sys.stdout.buffer.write(zipfile.ZipFile(sys.argv[1]).read(sys.argv[2]))
' "$AAB" "$1"
}

entries="$(list_so_entries || true)"
[ -n "$entries" ] || fail "no base/lib/*/*.so entries in $AAB"

count=0
while IFS= read -r entry; do
    align="$(read_zip_entry "$entry" | "$READELF" -l - | min_load_alignment)"
    [ "$align" -ge "$REQUIRED_PAGE_ALIGNMENT" ] \
        || fail "$entry is aligned to $align bytes, needs >= $REQUIRED_PAGE_ALIGNMENT"
    echo "  $entry: $align bytes"
    count=$((count + 1))
done <<< "$entries"

echo "android-verify-alignment: verified $count .so entries at >= $REQUIRED_PAGE_ALIGNMENT bytes"
