#!/usr/bin/env bash
# Reads the internal-testing track's versionCodes out of the Google Play
# Android Publisher API, so the release preflight can refuse a versionCode
# the store already knows.
#
# Usage: scripts/android-play-track.sh
# One versionCode per line on stdout (no output = the track carries no codes
# yet); every diagnostic goes to stderr, mirroring scripts/android-bundle.sh's
# single-line-stdout contract.
#
# Reads PLAY_ACCESS_TOKEN from the environment, never from argv: the token is
# written into a header file under ${RUNNER_TEMP:-$TMPDIR} and handed to curl
# as `-H @<file>`. That file is removed on every exit path.
#
# The Android Publisher API is edit-scoped: even reading a track needs an
# open edit. This script opens one, reads, and DELETES it before exiting --
# on the success path and on every failure path. An edit left open would be
# picked up by whatever publishes next.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$ROOT/scripts/android-release-lib.sh"

fail() {
    echo "android-play-track: $1" >&2
    exit 1
}

preflight_fail() {
    echo "android-play-track: $1" >&2
    exit 2
}

[ $# -eq 0 ] || preflight_fail "usage: scripts/android-play-track.sh"

# @law: both probes run with the token removed from the environment via
# `env -u` rather than trusting that a subprocess inherits nothing it was
# not handed -- `environ` outlives the argv of the command that set it.
env -u PLAY_ACCESS_TOKEN curl --version >/dev/null 2>&1 \
    || preflight_fail "curl not found or not invocable on PATH"
env -u PLAY_ACCESS_TOKEN python3 -c 'import json' >/dev/null 2>&1 \
    || preflight_fail "python3 not found, not invocable, or without the json module on PATH"
env_var_is_nonempty PLAY_ACCESS_TOKEN || preflight_fail "PLAY_ACCESS_TOKEN is not set"

API_ROOT="https://androidpublisher.googleapis.com/androidpublisher/v3/applications/$PLAY_PACKAGE_NAME"
WORK_DIR="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
header_file="$(mktemp "$WORK_DIR/play-auth-header-XXXXXX")"
insert_body="$(mktemp "$WORK_DIR/play-edit-insert-XXXXXX")"
track_body="$(mktemp "$WORK_DIR/play-track-XXXXXX")"
edit_id=""
edit_deleted="no"

# @law: the delete is best-effort and must never mask the failure that is
# already unwinding -- it reports itself and returns 0 unconditionally, so
# the exit status the trap then propagates is the original one.
delete_edit() {
    if [ -z "$edit_id" ] || [ "$edit_deleted" = "yes" ]; then
        return 0
    fi
    edit_deleted="yes"
    local delete_status
    delete_status="$(curl -sS -o /dev/null -w '%{http_code}' -X DELETE -H @"$header_file" \
        "$API_ROOT/edits/$edit_id" 2>/dev/null)" || delete_status="transport"
    case "$delete_status" in
        2*) : ;;
        *) echo "android-play-track: could not delete the Play edit $edit_id (HTTP $delete_status) -- it will expire on its own" >&2 ;;
    esac
    return 0
}

cleanup() {
    delete_edit || true
    rm -f "$header_file" "$insert_body" "$track_body"
}
trap cleanup EXIT

write_play_auth_header_file "$header_file" PLAY_ACCESS_TOKEN

echo "==> opening a Play edit" >&2
insert_status="$(curl -sS -o "$insert_body" -w '%{http_code}' \
    -X POST -H @"$header_file" -H 'Content-Type: application/json' \
    --data '{}' "$API_ROOT/edits" 2>/dev/null)" \
    || fail "opening a Play edit failed: curl could not complete the request (transport failure)"

case "$insert_status" in
    2*) : ;;
    403) fail "opening a Play edit was refused (HTTP 403) -- the service account is not authorized: check its invitation in the Play Console, not only the JSON key" ;;
    *) fail "opening a Play edit was refused (HTTP $insert_status): $(cat "$insert_body")" ;;
esac

edit_id="$(python3 -c '
import json
import sys

try:
    print(json.load(sys.stdin)["id"])
except (ValueError, KeyError, TypeError) as error:
    sys.stderr.write("android-play-track: the edit-insert response carries no edit id: %s\n" % error)
    sys.exit(1)
' < "$insert_body")" || fail "the edit-insert response did not carry an edit id"

echo "==> reading the internal track" >&2
track_status="$(curl -sS -o "$track_body" -w '%{http_code}' \
    -X GET -H @"$header_file" \
    "$API_ROOT/edits/$edit_id/tracks/internal" 2>/dev/null)" \
    || fail "reading the internal track failed: curl could not complete the request (transport failure)"

case "$track_status" in
    2*) : ;;
    403) fail "reading the internal track was refused (HTTP 403) -- the service account is not authorized: check its invitation in the Play Console, not only the JSON key" ;;
    *) fail "reading the internal track was refused (HTTP $track_status): $(cat "$track_body")" ;;
esac

version_codes_from_track_json < "$track_body"

echo "==> read the internal track" >&2
