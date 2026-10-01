#!/usr/bin/env bash
# Verifies the produced Android App Bundle carries no reference to the
# deprecated Window.setStatusBarColor / Window.setNavigationBarColor setters
# in any packaged dex. Reads the artifact's own bytes (adr-0019/adr-0022:
# a property required of the artifact is verified on the artifact), never a
# version number or a build-time intention.
#
# @law: the dex entries are addressed by their ordinal position in the zip's
# infolist(), never by name -- entry names are attacker-controlled bytes and
# a name-keyed reader silently resolves duplicates to the LAST entry (the
# same law scripts/android-verify-alignment.sh:14-20 records for .so
# entries). No file is ever extracted to disk.
#
# Exit contract: 0 = verified clean (one verdict line on stdout), 1 = defect,
# 2 = missing prerequisite or unreadable input. Progress and failures go to
# stderr; an exit 0 without the verdict line is refused as untrusted by the
# same rule scripts/check.sh applies to its own gates.
#
# Usage: scripts/android-verify-no-deprecated-bar-apis.sh <aab-path>

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$ROOT/scripts/android-release-lib.sh"

fail() {
    echo "android-verify-no-deprecated-bar-apis: $1" >&2
    exit 1
}

preflight_fail() {
    echo "android-verify-no-deprecated-bar-apis: $1" >&2
    exit 2
}

[ $# -eq 1 ] || preflight_fail "usage: scripts/android-verify-no-deprecated-bar-apis.sh <aab-path>"
AAB="$1"

command -v python3 >/dev/null 2>&1 || preflight_fail "python3 not found"
[ -f "$AAB" ] || preflight_fail "no AAB at $AAB"

list_dex_indices() {
    python3 -c '
import sys, zipfile, fnmatch
try:
    zf = zipfile.ZipFile(sys.argv[1])
except zipfile.BadZipFile as e:
    print(str(e), file=sys.stderr)
    sys.exit(2)
for i, info in enumerate(zf.infolist()):
    if fnmatch.fnmatchcase(info.filename, "base/dex/classes*.dex"):
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

indices="$(list_dex_indices)" && indices_status=0 || indices_status=$?
[ "$indices_status" -eq 2 ] && preflight_fail "$AAB is not a valid zip archive"
[ "$indices_status" -eq 0 ] || preflight_fail "could not list entries in $AAB (python exited $indices_status)"
[ -n "$indices" ] || fail "no base/dex/classes*.dex entries in $AAB"

count=0
while IFS= read -r index; do
    entry="$(entry_name "$index")"
    scan="$(read_zip_entry "$index" | deprecated_bar_api_refs 2>&1)" && scan_status=0 || scan_status=$?
    case "$scan_status" in
        0)
            count=$((count + 1))
            ;;
        1)
            refs="$(printf '%s\n' "$scan" | paste -sd ' ' -)"
            fail "$entry carries a deprecated system-bar API reference: $refs"
            ;;
        2)
            fail "$entry could not be read as a dex file: $scan"
            ;;
        *)
            preflight_fail "the dex scan of $entry failed unexpectedly (exit $scan_status)"
            ;;
    esac
done <<< "$indices"

echo "android-verify-no-deprecated-bar-apis: verified $count dex entries carry no Window.setStatusBarColor/setNavigationBarColor reference"
