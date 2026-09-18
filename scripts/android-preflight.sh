#!/usr/bin/env bash
# Refuses a Play release BEFORE the first build step, in this order:
#
#   1. the ref being dispatched is not refs/heads/main            (AC 2)
#   2. [workspace.package].version is rejected by the frozen
#      version_code_from_semver, its message surfaced verbatim    (AC 4/5)
#   3. the tag v<version> exists AND points at the commit being
#      released -- missing and misplaced are distinct refusals    (AC 6)
#   4. the derived versionCode is already on the internal-testing
#      track, which Play would refuse anyway                      (AC 7)
#
# The order is the design, not an accident of writing: each refusal is
# cheaper and more fundamental than the next, and a release that fails two
# of them must report the FIRST one.
#
# Usage: scripts/android-preflight.sh [repo-root]
#   repo-root defaults to the repository this script lives in. It is the
#   checkout the ref, the Cargo.toml and the tag are read from -- not the
#   location of the Play scripts, which always come from this script's own
#   tree.
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

# @law: both readers' stderr is captured and re-printed UNCHANGED, with no
# prefix and no rewording -- the frozen library owns these messages, the
# runbook's failure table quotes them, and a wrapper that paraphrased one
# would make the table and the log disagree at the moment an operator is
# reading both.
if ! VERSION="$(workspace_version "$REPO_ROOT/Cargo.toml" 2>&1)"; then
    printf '%s\n' "$VERSION" >&2
    exit 1
fi
if ! VERSION_CODE="$(version_code_from_semver "$VERSION" 2>&1)"; then
    printf '%s\n' "$VERSION_CODE" >&2
    exit 1
fi

tag_status=0
tag_sha="$(git -C "$REPO_ROOT" rev-parse -q --verify "refs/tags/v$VERSION^{commit}" 2>/dev/null)" \
    || tag_status=$?
if [ "$tag_status" -ne 0 ]; then
    fail "no tag 'v$VERSION' exists in $REPO_ROOT -- create it at the commit being released ($TARGET_SHA) before dispatching"
fi
if [ "$tag_sha" != "$TARGET_SHA" ]; then
    fail "tag 'v$VERSION' points at $tag_sha, not at the commit being released ($TARGET_SHA)"
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
