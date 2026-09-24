#!/usr/bin/env bash
# Refuses a Play release BEFORE the first build step, in this order:
#
#   1. the ref being dispatched is not refs/heads/main            (AC 8)
#   2. the commit being released does not carry exactly one tag
#      vX.Y.Z: none, several, or one the frozen
#      version_code_from_semver refuses                           (AC 1/2/3/4)
#   3. the derived versionCode is already on the internal-testing
#      track, which Play would refuse anyway                      (AC 8)
#
# The order is the design, not an accident of writing: each refusal is
# cheaper and more fundamental than the next, and a release that fails two
# of them must report the FIRST one.
#
# The version is DERIVED FROM THE TAG at the commit being released, never
# read out of Cargo.toml: a tag is what an operator creates deliberately at
# the commit they mean to ship, whereas a Cargo.toml version is a working-tree
# value that nothing ties to the commit being released (owner ruling on issue
# #73, reversing the version-source half of adr-0019).
#
# Usage: scripts/android-preflight.sh [repo-root]
#   repo-root defaults to the repository this script lives in. It is the
#   checkout the ref and the tags are read from -- not the location of the
#   Play scripts, which always come from this script's own tree.
#
# Reads GITHUB_REF (required), GITHUB_SHA (defaults to HEAD off CI) and
# PLAY_ACCESS_TOKEN (for the collision query, via scripts/android-play-track.sh)
# from the environment. Optionally reads PLAY_UPLOAD_KEY_FINGERPRINT purely
# to state it in the run summary -- it prints a public value Google already
# publishes.
#
# One line on stdout on success; every diagnostic on stderr. Appends the run
# summary to $GITHUB_STEP_SUMMARY and version/version_code to $GITHUB_OUTPUT
# when those are set, and is silent about both when they are not (a local
# dry run).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$ROOT/scripts/android-release-lib.sh"

fail() {
    echo "android-preflight: $1" >&2
    exit 1
}

preflight_fail() {
    echo "android-preflight: $1" >&2
    exit 2
}

[ $# -le 1 ] || preflight_fail "usage: scripts/android-preflight.sh [repo-root]"
REPO_ROOT="${1:-$ROOT}"
[ -d "$REPO_ROOT" ] || preflight_fail "no directory at $REPO_ROOT"

env -u PLAY_ACCESS_TOKEN git --version >/dev/null 2>&1 \
    || preflight_fail "git not found or not invocable on PATH"

# @law: AC 2's job-level `if: github.ref == 'refs/heads/main'` in the
# workflow is the belt; this is the braces. It refuses an unset GITHUB_REF
# outright (exit 2) rather than reading "no ref" as "some ref that is not
# main" -- an environment that cannot name its own ref must not resolve to a
# pass, and must not resolve to a misleading refusal either.
[ -n "${GITHUB_REF:-}" ] || preflight_fail "GITHUB_REF is not set -- this preflight cannot verify which ref is being released"
[ "$GITHUB_REF" = "refs/heads/main" ] \
    || fail "GITHUB_REF is '$GITHUB_REF' -- the release must be dispatched from 'refs/heads/main'"

TARGET_SHA="$(git -C "$REPO_ROOT" rev-parse --verify "${GITHUB_SHA:-HEAD}^{commit}" 2>/dev/null)" \
    || preflight_fail "GITHUB_SHA '${GITHUB_SHA:-HEAD}' does not resolve to a commit in $REPO_ROOT"

# @law: the candidate filter is LOOSE (^v[0-9]) deliberately. A semantic
# pre-filter would refuse a malformed tag here, in this script's own words --
# the frozen version_code_from_semver must instead be the one that refuses it,
# so the message an operator reads is the one the runbook documents
# (adr-0019:42-45). The filter answers exactly one question: could this tag
# name a release version? What that version IS remains the frozen function's
# business, and so does every reason to reject it.
candidate_tags="$(git -C "$REPO_ROOT" tag --points-at "$TARGET_SHA" | grep -E '^v[0-9]' || true)"
candidate_count="$(printf '%s\n' "$candidate_tags" | grep -c . || true)"

if [ "$candidate_count" -eq 0 ]; then
    fail "the commit being released ($TARGET_SHA) carries no release tag vX.Y.Z -- create it there before dispatching"
fi
if [ "$candidate_count" -gt 1 ]; then
    candidate_list="$(printf '%s\n' "$candidate_tags" | paste -sd ' ' -)"
    fail "the commit being released ($TARGET_SHA) carries several release tags ($candidate_list) -- exactly one vX.Y.Z is required"
fi

VERSION="${candidate_tags#v}"
# @law: the frozen reader's stderr is captured and re-printed UNCHANGED, with
# no prefix and no rewording -- the frozen library owns this message, the
# runbook's failure table quotes it, and a wrapper that paraphrased it would
# make the table and the log disagree at the moment an operator is reading
# both.
if ! VERSION_CODE="$(version_code_from_semver "$VERSION" 2>&1)"; then
    printf '%s\n' "$VERSION_CODE" >&2
    exit 1
fi

# @law: the query's stderr is deliberately NOT redirected: an operator whose
# service-account invitation is missing must see android-play-track.sh's own
# 403 diagnostic, not a bare "the query failed" from here.
track_status=0
version_codes="$("$ROOT/scripts/android-play-track.sh")" || track_status=$?
case "$track_status" in
    0) : ;;
    2) preflight_fail "the internal-track query could not run (exit 2) -- see its diagnostic above" ;;
    *) fail "the internal-track query failed (exit $track_status) -- see its diagnostic above" ;;
esac

if printf '%s\n' "$version_codes" | track_contains_version_code "$VERSION_CODE"; then
    fail "versionCode $VERSION_CODE is already on the internal testing track for $PLAY_PACKAGE_NAME -- Play never accepts a code at or below one it already knows; release a higher version"
fi

if [ -n "${GITHUB_OUTPUT:-}" ]; then
    {
        printf 'version=%s\n' "$VERSION"
        printf 'version_code=%s\n' "$VERSION_CODE"
    } >> "$GITHUB_OUTPUT"
fi

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        printf '### Play release preflight\n'
        printf -- '- Version: %s (versionCode %s)\n' "$VERSION" "$VERSION_CODE"
        printf -- '- Tag: v%s at %s\n' "$VERSION" "$TARGET_SHA"
        printf -- '- Upload-key fingerprint: %s\n' "${PLAY_UPLOAD_KEY_FINGERPRINT:-(not configured)}"
    } >> "$GITHUB_STEP_SUMMARY"
fi

printf 'android-preflight: %s (versionCode %s), tag v%s at %s\n' "$VERSION" "$VERSION_CODE" "$VERSION" "$TARGET_SHA"
