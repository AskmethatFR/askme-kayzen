#!/usr/bin/env bash
# Signs the unsigned release AAB scripts/android-bundle.sh produces with the
# real upload key, then re-verifies both the signature and the 16 KB
# page-size alignment on the SIGNED bytes.
#
# Usage: scripts/android-sign.sh <aab-path>
# Progress goes to stderr; the ONLY line on stdout is the signed AAB's
# path, mirroring scripts/android-bundle.sh:177-178.
#
# Reads four values from the environment, never from argv:
#   ANDROID_SIGN_KEYSTORE        path to the upload keystore (outside the repo)
#   ANDROID_SIGN_KEY_ALIAS       the alias to sign with
#   ANDROID_SIGN_STORE_PASSWORD  the keystore's store password
#   ANDROID_SIGN_KEY_PASSWORD    the key's own password
#
# @law: jarsigner's `-storepass:env NAME` / `-keypass:env NAME` read the
# NAMED environment variable's value themselves -- this script puts only
# the variable NAME on jarsigner's argv and never expands
# ANDROID_SIGN_STORE_PASSWORD / ANDROID_SIGN_KEY_PASSWORD itself, anywhere,
# for anything. A password on argv is readable in /proc/*/cmdline by any
# process on the machine and is echoed verbatim by `set -x`; keytool has no
# `:env` modifier, which is why signing never shells out to it.
#
# @algo: the alignment re-check below (scripts/android-verify-alignment.sh,
# unchanged, as a second call site) measures the SIGNED bundle's own bytes,
# per ADR-0019's "a property required of the artifact is verified on the
# artifact". Said honestly: jarsigner rewrites the archive's central
# directory but never alters a member's own content, so a native library's
# ELF LOAD alignment cannot regress from signing alone -- this is a
# pipeline guard against a future repackaging step, not a check that
# signing itself could ever fail.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    echo "android-sign: $1" >&2
    exit 1
}

# A missing tool or a missing input is never a skip (verify-instrument.sh's
# own doctrine, mirrored by android-verify-alignment.sh's own
# preflight_fail): exit 2 keeps it distinguishable from an actual signing
# or verification defect (exit 1, above).
preflight_fail() {
    echo "android-sign: $1" >&2
    exit 2
}

[ $# -eq 1 ] || preflight_fail "usage: scripts/android-sign.sh <aab-path>"
AAB="$1"

command -v jarsigner >/dev/null 2>&1 || preflight_fail "jarsigner not found on PATH (needs a JDK)"

[ -n "${ANDROID_SIGN_KEYSTORE:-}" ] || preflight_fail "ANDROID_SIGN_KEYSTORE is not set"
[ -n "${ANDROID_SIGN_KEY_ALIAS:-}" ] || preflight_fail "ANDROID_SIGN_KEY_ALIAS is not set"

[ -f "$AAB" ] || preflight_fail "no AAB at $AAB"
[ -f "$ANDROID_SIGN_KEYSTORE" ] || preflight_fail "no keystore at $ANDROID_SIGN_KEYSTORE"

SIGNED_AAB="${AAB%.aab}-signed.aab"

tmp_signed="$(mktemp)"
trap 'rm -f "$tmp_signed"' EXIT

echo "==> signing $AAB with alias '$ANDROID_SIGN_KEY_ALIAS'" >&2
# @algo: jarsigner prints its own diagnostics -- including the ones
# classified below -- to STDOUT even on a hard failure, never stderr; only
# `2>&1` (never `1>/dev/null`) captures them. This never leaks into this
# script's own stdout: command substitution consumes the whole thing into
# sign_out, nothing escapes to the caller regardless of sign_status.
sign_out="$(jarsigner \
    -keystore "$ANDROID_SIGN_KEYSTORE" \
    -storepass:env ANDROID_SIGN_STORE_PASSWORD \
    -keypass:env ANDROID_SIGN_KEY_PASSWORD \
    -signedjar "$tmp_signed" \
    "$AAB" "$ANDROID_SIGN_KEY_ALIAS" 2>&1)" && sign_status=0 || sign_status=$?

if [ "$sign_status" -ne 0 ]; then
    case "$sign_out" in
        *"password was incorrect"*)
            fail "wrong store password for keystore $ANDROID_SIGN_KEYSTORE"
            ;;
        *"not a private key"*)
            fail "wrong key password for alias '$ANDROID_SIGN_KEY_ALIAS' in keystore $ANDROID_SIGN_KEYSTORE"
            ;;
        *"Certificate chain not found for"*)
            fail "alias '$ANDROID_SIGN_KEY_ALIAS' not found in keystore $ANDROID_SIGN_KEYSTORE"
            ;;
        *)
            fail "jarsigner failed to sign $AAB: $sign_out"
            ;;
    esac
fi

echo "==> verifying the signature by alias '$ANDROID_SIGN_KEY_ALIAS'" >&2
verify_out="$(jarsigner -verify \
    -keystore "$ANDROID_SIGN_KEYSTORE" \
    -storepass:env ANDROID_SIGN_STORE_PASSWORD \
    "$tmp_signed" "$ANDROID_SIGN_KEY_ALIAS" 2>&1)" && verify_status=0 || verify_status=$?

[ "$verify_status" -eq 0 ] || fail "jarsigner could not verify the signed bundle: $verify_out"
case "$verify_out" in
    *"jar verified"*) : ;;
    *) fail "the signed bundle did not verify: $verify_out" ;;
esac
case "$verify_out" in
    *"not signed by the specified alias"*)
        fail "the signed bundle is not signed by alias '$ANDROID_SIGN_KEY_ALIAS'"
        ;;
esac

echo "==> re-verifying 16 KB page-size alignment on the signed bundle" >&2
"$ROOT/scripts/android-verify-alignment.sh" "$tmp_signed" >&2

mv "$tmp_signed" "$SIGNED_AAB"

echo "==> $SIGNED_AAB" >&2
printf '%s\n' "$SIGNED_AAB"
