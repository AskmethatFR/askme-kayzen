#!/usr/bin/env bash
# Puts the real launcher icon into a generated Android project, and takes it
# back out again.
#
# Usage: scripts/android-icon.sh apply|clean <generated-res-dir>
#
# dx 0.7.9 does not wire [android].icon, so app/android/res/ cannot reach the
# APK on its own — something has to copy it in after dx has generated the
# Gradle project and before Gradle assembles. That is `apply`.
#
# `clean` exists because dx rewrites res/ on every build but only ever ADDS its
# own files back: it restores mipmap-*/ic_launcher.webp without removing the
# mipmap-*/ic_launcher.png a previous `apply` left there. Android resolves a
# resource by name and not by extension, so the two become one
# @mipmap/ic_launcher with two definitions and mergeDebugResources fails with
# "Duplicate resources". Running `clean` before dx hands it back the tree it
# expects to own.
#
# Both modes derive their file list from app/android/res/ itself, so adding a
# density or a layer needs no edit here.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_RES="$REPO_ROOT/app/android/res"

fail() {
  echo "android-icon: $1" >&2
  exit 1
}

[ $# -eq 2 ] || fail "usage: scripts/android-icon.sh apply|clean <generated-res-dir>"
MODE="$1"
TARGET_RES="$2"

[ -d "$SOURCE_RES" ] || fail "no icon sources at $SOURCE_RES"

owned_files() {
  (cd "$SOURCE_RES" && find . -type f -print | sed 's|^\./||')
}

case "$MODE" in
  clean)
    # A missing target is the normal case on a first build, not an error.
    [ -d "$TARGET_RES" ] || exit 0
    while IFS= read -r relative; do
      rm -f "$TARGET_RES/$relative"
    done < <(owned_files)
    ;;

  apply)
    [ -d "$TARGET_RES" ] || fail "no generated res at $TARGET_RES (run dx build first)"

    find "$TARGET_RES" -name 'ic_launcher*.webp' -delete
    rm -f "$TARGET_RES/drawable-v24/ic_launcher_foreground.xml"
    cp -R "$SOURCE_RES/." "$TARGET_RES/"

    # The copy is the whole point, so its result is asserted rather than
    # assumed: a silent no-op here ships the template robot to a device, and
    # the only symptom is a wrong icon nobody connects back to the build.
    survivor="$(find "$TARGET_RES" -name 'ic_launcher*.webp' | head -1)"
    [ -z "$survivor" ] || fail "a template icon survived the copy: $survivor"

    while IFS= read -r relative; do
      [ -f "$TARGET_RES/$relative" ] || fail "$relative did not land in $TARGET_RES"
    done < <(owned_files)

    echo "android-icon: launcher icon applied to $TARGET_RES"
    ;;

  *)
    fail "unknown mode '$MODE' (expected apply or clean)"
    ;;
esac
