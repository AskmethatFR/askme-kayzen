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
# Also reads NDK_HOME (no default -- must already be set, same contract as
# scripts/android-verify-alignment.sh).
#
# @law: jarsigner's and keytool's `-storepass:env NAME` / `-keypass:env
# NAME` read the NAMED environment variable's value themselves -- this
# script puts only the variable NAME on their argv and never expands
# ANDROID_SIGN_STORE_PASSWORD / ANDROID_SIGN_KEY_PASSWORD itself, anywhere,
# for anything. A password on argv is readable in /proc/*/cmdline by any
# process on the machine and is echoed verbatim by `set -x`. Signing and
# reading the signing alias's certificate fingerprint (verify_jar_signature's
# expected value, read via keystore_alias_fingerprint) are the ONLY two
# steps that need the store password, and both run before either password
# variable is unset from this script's own environment: every descendant
# process spawned after that point (the jarsigner -verify call, the
# signed jar's OWN fingerprint read inside verify_jar_signature, and the
# alignment re-check below) never sees them -- argv is not the only
# channel a password leaks through, and `environ` outlives the argv of the
# command that set it. Verifying a signature needs no store password at
# all -- jarsigner -verify and `keytool -printcert -jarfile` only read
# public certificate data -- so :env is never passed to either. The
# python3 preflight probe below runs inside that same pre-unset window and
# also needs neither password, so -- unlike jarsigner and keytool, which
# only ever see the variable NAME -- it is invoked via `env -u` rather
# than trusting that a subprocess inherits nothing it wasn't handed.
#
# @law: the alignment re-check below (scripts/android-verify-alignment.sh,
# unchanged, as a second call site) measures the SIGNED bundle's own bytes,
# per ADR-0019's "a property required of the artifact is verified on the
# artifact". Said honestly: jarsigner rewrites the archive's central
# directory but never alters a member's own content, so a native library's
# ELF LOAD alignment cannot regress from signing alone -- this is a
# pipeline guard against a future repackaging step, not a check that
# signing itself could ever fail.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$ROOT/scripts/android-release-lib.sh"

fail() {
    echo "android-sign: $1" >&2
    exit 1
}

preflight_fail() {
    echo "android-sign: $1" >&2
    exit 2
}

[ $# -eq 1 ] || preflight_fail "usage: scripts/android-sign.sh <aab-path>"
AAB="$1"
case "$AAB" in
    -*) preflight_fail "the AAB path must not start with '-': $AAB" ;;
esac

command -v jarsigner >/dev/null 2>&1 || preflight_fail "jarsigner not found on PATH (needs a JDK)"
keytool -help >/dev/null 2>&1 || preflight_fail "keytool not found or not invocable on PATH (needs a JDK)"
# @algo: the post-signing alignment re-check decompresses each
# base/lib/*/*.so entry (android-verify-alignment.sh's read_zip_entry), and
# a real signed bundle's .so entries are DEFLATE-compressed -- `import
# zipfile` alone succeeds without zlib, so it cannot catch a python3
# missing zlib support. Round-tripping a small DEFLATE-compressed entry
# through zipfile is what actually needs zlib, the same way the check
# above actually invokes keytool instead of merely locating it.
env -u ANDROID_SIGN_STORE_PASSWORD -u ANDROID_SIGN_KEY_PASSWORD python3 -c '
import zipfile, io
buf = io.BytesIO()
with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
    zf.writestr("probe", b"probe")
buf.seek(0)
with zipfile.ZipFile(buf) as zf:
    zf.read("probe")
' >/dev/null 2>&1 || preflight_fail "python3 not found, not invocable, or unable to decompress a zip entry on PATH -- needed for the post-signing alignment re-check"

[ -n "${ANDROID_SIGN_KEYSTORE:-}" ] || preflight_fail "ANDROID_SIGN_KEYSTORE is not set"
case "${ANDROID_SIGN_KEYSTORE:-}" in
    -*) preflight_fail "ANDROID_SIGN_KEYSTORE must not start with '-': $ANDROID_SIGN_KEYSTORE" ;;
esac
[ -n "${ANDROID_SIGN_KEY_ALIAS:-}" ] || preflight_fail "ANDROID_SIGN_KEY_ALIAS is not set"
case "${ANDROID_SIGN_KEY_ALIAS:-}" in
    -*) preflight_fail "ANDROID_SIGN_KEY_ALIAS must not start with '-': $ANDROID_SIGN_KEY_ALIAS" ;;
esac
[ -n "${ANDROID_SIGN_STORE_PASSWORD+x}" ] || preflight_fail "ANDROID_SIGN_STORE_PASSWORD is not set"
[ -n "${ANDROID_SIGN_KEY_PASSWORD+x}" ] || preflight_fail "ANDROID_SIGN_KEY_PASSWORD is not set"

[ -f "$AAB" ] || preflight_fail "no AAB at $AAB"
[ -f "$ANDROID_SIGN_KEYSTORE" ] || preflight_fail "no keystore at $ANDROID_SIGN_KEYSTORE"

# @law: ADR-0019 byte-freezes scripts/android-verify-alignment.sh, so its
# resolution of llvm-readelf under NDK_HOME is deliberately duplicated
# here rather than shared -- do not refactor this into android-release-lib.sh.
ndk_readelf_found="no"
for candidate in "${NDK_HOME:-/nonexistent}"/toolchains/llvm/prebuilt/*/bin/llvm-readelf; do
    [ -x "$candidate" ] && ndk_readelf_found="yes" && break
done
[ "$ndk_readelf_found" = "yes" ] \
    || preflight_fail "no llvm-readelf under \$NDK_HOME/toolchains/llvm/prebuilt/*/bin (NDK_HOME=${NDK_HOME:-<unset>}) -- needed for the post-signing alignment re-check"

SIGNED_AAB="${AAB%.aab}-signed.aab"
rm -f "$SIGNED_AAB"

# @law: `mv` across filesystems silently degrades to copy+unlink, losing
# the atomicity the temp-then-rename pattern exists to provide. Created as
# a sibling of $AAB (same directory), never a bare `mktemp` (which lands in
# $TMPDIR, possibly a different filesystem), so the final `mv` below stays
# a same-filesystem rename and a partially-written signed bundle can never
# be observed.
tmp_signed="$(mktemp "$(dirname "$AAB")/.android-sign-XXXXXX")"
trap 'rm -f "$tmp_signed"' EXIT

echo "==> signing $AAB with alias '$ANDROID_SIGN_KEY_ALIAS'" >&2
# @algo: jarsigner prints its own diagnostics -- including the ones
# classified below -- to STDOUT even on a hard failure, never stderr; only
# `2>&1` (never `1>/dev/null`) captures them. This never leaks into this
# script's own stdout: command substitution consumes the whole thing into
# sign_out, nothing escapes to the caller regardless of sign_status.
sign_out="$(jarsigner \
    -J-Duser.language=en -J-Duser.country=US \
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

echo "==> reading the certificate fingerprint for alias '$ANDROID_SIGN_KEY_ALIAS'" >&2
expected_fingerprint="$(keystore_alias_fingerprint "$ANDROID_SIGN_KEYSTORE" "$ANDROID_SIGN_KEY_ALIAS" ANDROID_SIGN_STORE_PASSWORD 2>&1)" \
    || fail "could not read the certificate fingerprint for alias '$ANDROID_SIGN_KEY_ALIAS' in $ANDROID_SIGN_KEYSTORE: $expected_fingerprint"

# @law: signing and the fingerprint read above are the only steps that
# need the two passwords -- every process spawned after this point
# (jarsigner -verify next, the signed jar's OWN fingerprint read inside
# verify_jar_signature, and the alignment re-check further down) must
# never see them: argv is not the only channel a password leaks through,
# and `environ` outlives the argv of the command that set it.
unset ANDROID_SIGN_STORE_PASSWORD ANDROID_SIGN_KEY_PASSWORD

echo "==> verifying the signature by alias '$ANDROID_SIGN_KEY_ALIAS'" >&2
verify_out="$(verify_jar_signature "$ANDROID_SIGN_KEYSTORE" "$tmp_signed" "$expected_fingerprint")" \
    && verify_status=0 || verify_status=$?
case "$verify_status" in
    0) : ;;
    1) fail "jarsigner could not verify the signed bundle: $verify_out" ;;
    2) fail "the signed bundle did not verify: $verify_out" ;;
    3) fail "the signed bundle is not signed by alias '$ANDROID_SIGN_KEY_ALIAS': $verify_out" ;;
    4) fail "could not establish a single signer certificate for the signed bundle: $verify_out" ;;
    *) fail "unexpected verify status $verify_status: $verify_out" ;;
esac

echo "==> re-verifying 16 KB page-size alignment on the signed bundle" >&2
"$ROOT/scripts/android-verify-alignment.sh" "$tmp_signed" >&2

mv "$tmp_signed" "$SIGNED_AAB"

echo "==> $SIGNED_AAB" >&2
printf '%s\n' "$SIGNED_AAB"
