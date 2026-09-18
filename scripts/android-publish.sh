#!/usr/bin/env bash
# Publishes ONE signed release bundle to the Google Play internal-testing
# track as a DRAFT release, inside a single Play edit:
#
#   edit-insert -> bundle-upload -> track-update -> edit-commit
#
# Usage: scripts/android-publish.sh <signed-aab> <version> <versionCode>
# One line on stdout on success; every diagnostic on stderr.
#
# Reads PLAY_ACCESS_TOKEN from the environment, never from argv: the token is
# written into a header file under ${RUNNER_TEMP:-$TMPDIR} and handed to curl
# as `-H @<file>`, which is removed on every exit path.
#
# @law: this is the one irreversible step of the release. Play burns a
# versionCode permanently once it accepts it, so the upload is never
# retried -- no `--retry` on any curl call, no loop, exactly one request per
# step -- and a failure at any step ends the run non-zero, naming the step
# and its HTTP status. A caller that wants another attempt must dispatch a
# NEW version.
#
# @law: the track update is a READ-MODIFY-WRITE inside the same edit, never
# a wholesale replacement of the track's release list: Play's track resource
# carries every release it has ever kept, and writing back only the new one
# would unlist the earlier internal releases.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$ROOT/scripts/android-release-lib.sh"

fail() {
    echo "android-publish: $1" >&2
    exit 1
}

preflight_fail() {
    echo "android-publish: $1" >&2
    exit 2
}

[ $# -eq 3 ] || preflight_fail "usage: scripts/android-publish.sh <signed-aab> <version> <versionCode>"
AAB="$1"
VERSION="$2"
VERSION_CODE="$3"

case "$AAB" in
    -*) preflight_fail "the AAB path must not start with '-': $AAB" ;;
esac
[ -f "$AAB" ] || preflight_fail "no AAB at $AAB"

# @law: both probes run with the token removed from the environment via
# `env -u`, the same discipline android-sign.sh applies to the signing
# passwords: `environ` outlives the argv of the command that set it.
env -u PLAY_ACCESS_TOKEN curl --version >/dev/null 2>&1 \
    || preflight_fail "curl not found or not invocable on PATH"
env -u PLAY_ACCESS_TOKEN python3 -c 'import json' >/dev/null 2>&1 \
    || preflight_fail "python3 not found, not invocable, or without the json module on PATH"
env_var_is_nonempty PLAY_ACCESS_TOKEN || preflight_fail "PLAY_ACCESS_TOKEN is not set"

# @law: the versionCode is re-derived here from the version rather than
# trusted from the argument -- it is a line in a JSON body Play acts on
# irreversibly, and this is the last step before that body leaves the
# machine. The comparison also makes an injection impossible by
# construction: only the frozen function's own numeric output can equal it.
derived_status=0
derived_code="$(version_code_from_semver "$VERSION" 2>&1)" || derived_status=$?
if [ "$derived_status" -ne 0 ]; then
    preflight_fail "$derived_code"
fi
[ "$derived_code" = "$VERSION_CODE" ] \
    || preflight_fail "version '$VERSION' has versionCode $derived_code, but $VERSION_CODE was given -- refusing to publish a code the version does not derive"

API_ROOT="https://androidpublisher.googleapis.com/androidpublisher/v3/applications/$PLAY_PACKAGE_NAME"
PLAY_TRACK_ID="internal"
WORK_DIR="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
header_file="$(mktemp "$WORK_DIR/play-auth-header-XXXXXX")"
insert_body="$(mktemp "$WORK_DIR/play-edit-insert-XXXXXX")"
upload_body="$(mktemp "$WORK_DIR/play-bundle-upload-XXXXXX")"
track_get_body="$(mktemp "$WORK_DIR/play-track-get-XXXXXX")"
track_put_body="$(mktemp "$WORK_DIR/play-track-put-XXXXXX")"
commit_body="$(mktemp "$WORK_DIR/play-edit-commit-XXXXXX")"
edit_id=""
edit_finished="no"

summarize() {
    if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
        printf -- '- %s\n' "$1" >> "$GITHUB_STEP_SUMMARY"
    fi
}

fail_step() {
    summarize "FAILED at $1: $2"
    fail "$1: $2"
}

cleanup() {
    # A publish that did not reach the commit leaves an open edit behind; it
    # would expire on its own, but leaving it is a half-applied change an
    # operator cannot see. Best-effort, and it must never mask the failure
    # already unwinding -- hence the unconditional `return 0` at the end.
    if [ -n "$edit_id" ] && [ "$edit_finished" != "yes" ]; then
        edit_finished="yes"
        local delete_status
        delete_status="$(curl -sS -o /dev/null -w '%{http_code}' -X DELETE -H @"$header_file" \
            "$API_ROOT/edits/$edit_id" 2>/dev/null)" || delete_status="transport"
        case "$delete_status" in
            2*) : ;;
            *) echo "android-publish: could not delete the Play edit $edit_id (HTTP $delete_status) -- it will expire on its own" >&2 ;;
        esac
    fi
    rm -f "$header_file" "$insert_body" "$upload_body" \
        "$track_get_body" "$track_put_body" "$commit_body"
    return 0
}
trap cleanup EXIT

write_play_auth_header_file "$header_file" PLAY_ACCESS_TOKEN

echo "==> edit-insert" >&2
insert_status="$(curl -sS -o "$insert_body" -w '%{http_code}' \
    -X POST -H @"$header_file" -H 'Content-Type: application/json' \
    --data '{}' "$API_ROOT/edits" 2>/dev/null)" || insert_status="transport"
case "$insert_status" in
    transport) fail_step "edit-insert" "curl could not complete the request (transport failure)" ;;
    2*) : ;;
    *) fail_step "edit-insert" "HTTP $insert_status: $(cat "$insert_body")" ;;
esac

edit_id="$(python3 -c '
import json
import sys

try:
    print(json.load(sys.stdin)["id"])
except (ValueError, KeyError, TypeError) as error:
    sys.stderr.write("android-publish: the edit-insert response carries no edit id: %s\n" % error)
    sys.exit(1)
' < "$insert_body")" || fail_step "edit-insert" "the edit-insert response did not carry an edit id"

echo "==> bundle-upload" >&2
upload_status="$(curl -sS -o "$upload_body" -w '%{http_code}' \
    -X POST -H @"$header_file" -H 'Content-Type: application/octet-stream' \
    --data-binary @"$AAB" \
    "$API_ROOT/edits/$edit_id/bundles?uploadType=media" 2>/dev/null)" || upload_status="transport"
case "$upload_status" in
    transport) fail_step "bundle-upload" "curl could not complete the request (transport failure)" ;;
    2*) : ;;
    *) fail_step "bundle-upload" "HTTP $upload_status: $(cat "$upload_body")" ;;
esac

echo "==> track-update (read, then write back with the new draft release)" >&2
track_get_status="$(curl -sS -o "$track_get_body" -w '%{http_code}' \
    -X GET -H @"$header_file" \
    "$API_ROOT/edits/$edit_id/tracks/$PLAY_TRACK_ID" 2>/dev/null)" || track_get_status="transport"
case "$track_get_status" in
    transport) fail_step "track-update" "reading the $PLAY_TRACK_ID track: curl could not complete the request (transport failure)" ;;
    2*) : ;;
    403) fail_step "track-update" "reading the $PLAY_TRACK_ID track was refused (HTTP 403) -- the service account is not authorized: check its invitation in the Play Console, not only the JSON key" ;;
    *) fail_step "track-update" "reading the $PLAY_TRACK_ID track: HTTP $track_get_status: $(cat "$track_get_body")" ;;
esac

track_body="$(python3 - "$track_get_body" "$VERSION_CODE" "$PLAY_TRACK_ID" <<'PY'
import json
import sys

track_path, version_code, track_id = sys.argv[1], sys.argv[2], sys.argv[3]
with open(track_path) as handle:
    track = json.load(handle)

if not isinstance(track, dict):
    sys.stderr.write("android-publish: the track response is not a JSON object\n")
    sys.exit(1)

releases = track.get("releases", [])
if not isinstance(releases, list):
    sys.stderr.write("android-publish: the track response's \"releases\" is not a list\n")
    sys.exit(1)

releases = releases + [{"versionCodes": [version_code], "status": "draft"}]
json.dump({"track": track_id, "releases": releases}, sys.stdout)
PY
)" || fail_step "track-update" "the $PLAY_TRACK_ID track response could not be read as a track"

track_put_status="$(curl -sS -o "$track_put_body" -w '%{http_code}' \
    -X PUT -H @"$header_file" -H 'Content-Type: application/json' \
    --data-binary "$track_body" \
    "$API_ROOT/edits/$edit_id/tracks/$PLAY_TRACK_ID" 2>/dev/null)" || track_put_status="transport"
case "$track_put_status" in
    transport) fail_step "track-update" "writing the $PLAY_TRACK_ID track: curl could not complete the request (transport failure)" ;;
    2*) : ;;
    *) fail_step "track-update" "writing the $PLAY_TRACK_ID track: HTTP $track_put_status: $(cat "$track_put_body")" ;;
esac

echo "==> edit-commit" >&2
commit_status="$(curl -sS -o "$commit_body" -w '%{http_code}' \
    -X POST -H @"$header_file" \
    "$API_ROOT/edits/$edit_id:commit" 2>/dev/null)" || commit_status="transport"
case "$commit_status" in
    transport) fail_step "edit-commit" "curl could not complete the request (transport failure)" ;;
    2*) : ;;
    *) fail_step "edit-commit" "HTTP $commit_status: $(cat "$commit_body")" ;;
esac

edit_finished="yes"

summarize "Upload: versionCode $VERSION_CODE committed to the $PLAY_TRACK_ID track as a draft"
printf 'android-publish: versionCode %s published to the %s track as a draft\n' "$VERSION_CODE" "$PLAY_TRACK_ID"
