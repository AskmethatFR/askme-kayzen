#!/usr/bin/env bash
# Pure helpers for the Android release build, sourced by
# scripts/android-verify-alignment.sh, scripts/android-bundle.sh and
# scripts/android-sign.sh.
# Sourceable, side-effect-free: nothing runs until a function below is
# called, matching scripts/verify-instrument.sh's own shape.
#
# workspace_version reads [workspace.package].version out of a Cargo.toml
# path given as its one argument. Its result feeds version_code_from_semver
# below, which is what makes the Cargo.toml -> versionCode binding provable
# by this file's own test harness instead of living in untestable inline awk.
#
# version_code_from_semver refuses a "v" prefix and any -pre/+build suffix:
# its input is always Cargo.toml's bare version string, never a git tag --
# tag<->version alignment is a separate decision, out of this file's scope.
# It also refuses any component over 999 and a result of 0. The Play Store
# ceiling (2100000000) is never checked here: major/minor/patch each capped
# at 999 puts the largest possible versionCode at 999999999, so the ceiling
# is structurally unreachable and a second guard for it would be a dead
# branch no test or mutant could ever discriminate.
#
# min_load_alignment reads an `llvm-readelf -l` dump on stdin and reports
# the SMALLEST LOAD segment Align it finds, in decimal bytes -- the 16 KB
# page-size regression this whole build exists to catch can land in any
# LOAD segment, not only the one carrying .text, so the caller must never
# assume which one to look at. It only reports; the caller decides whether
# the value it gets back is acceptable.
#
# patch_version_code rewrites a generated build.gradle.kts's ONE
# `versionCode = 1` sentinel line to the real versionCode. It proves the
# substitution actually RAN via a two-phase marker swap (sentinel -> a
# marker token that can never coincide with any versionCode -> the real
# value) rather than by comparing the before/after text: when the intended
# versionCode is itself 1 (a bare 0.0.x release), a text-only comparison
# cannot tell "the substitution ran and produced 1" from "the substitution
# never ran and the dx-generated 1 was merely left in place" -- the two
# read as byte-identical. The marker makes the two provably different
# events again.
#
# @law: jar_signer_fingerprint and keystore_alias_fingerprint both force
# `keytool -printcert`/`-list` to English (`-J-Duser.language=en
# -J-Duser.country=US`) -- a non-English JVM locale can crash keytool
# outright, and the forcing must happen on argv: a JVM can read its
# locale from native OS APIs rather than shell environment variables, so
# LC_ALL/LANG alone is not a reliable substitute. jar_signer_fingerprint
# reads the fingerprint off an already-SIGNED jar's own signer
# certificate and needs no password: `keytool -printcert -jarfile` only
# reads public certificate data. keystore_alias_fingerprint reads the
# fingerprint off a keystore alias's certificate and needs the store
# password (`-storepass:env NAME`, the variable NAME only, never its
# value -- the same discipline scripts/android-sign.sh's own `@law:`
# block holds jarsigner to); its caller must call it before the store
# password leaves scope.
#
# verify_jar_signature classifies a `jarsigner -verify` run into exactly
# one of: 0 (verified, and its signer certificate fingerprint matches the
# caller-supplied expected fingerprint), 1 (jarsigner itself could not
# verify the jar), 2 (verified without failing, but the "jar verified"
# marker text is absent), 3 (verified, exactly one signer certificate was
# read back, but its fingerprint does not match the expected one), 4
# (verified, but a single signer certificate could not be established --
# either keytool could not read one back at all, or the jar carries more
# than one signer; jar_signer_fingerprint refuses the latter outright
# rather than silently taking the first). It takes an expected FINGERPRINT, not an alias
# name: jarsigner's own alias check (passing an alias to `-verify`) prints
# "not signed by the specified alias(es)" for ANY self-signed certificate
# regardless of whether the named alias is in fact the signer -- every
# Android upload key IS self-signed, and this was a real false positive
# in production on a correctly-signed bundle (confirmed directly against
# this JVM with a two-alias PKCS12 keystore: the warning fires identically
# whether the alias asked for is the true signer or a different one). The
# text does not discriminate; a fingerprint comparison does, which is what
# makes it directly testable against a fixture jar signed by a DIFFERENT
# alias than the one whose fingerprint it is compared against -- a state
# scripts/android-sign.sh's own sign-then-verify contract can never reach
# on its own, since it always signs and verifies with the same single
# alias.

readonly REQUIRED_PAGE_ALIGNMENT=16384

workspace_version() {
    local cargo_toml="$1"
    if [ ! -f "$cargo_toml" ]; then
        echo "workspace_version: no Cargo.toml at $cargo_toml" >&2
        return 1
    fi

    local version
    version="$(awk '
        /^\[workspace\.package\]/ { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && /^version[[:space:]]*=/ {
            match($0, /"[^"]*"/)
            print substr($0, RSTART + 1, RLENGTH - 2)
            exit
        }
    ' "$cargo_toml")"

    if [ -z "$version" ]; then
        echo "workspace_version: could not read [workspace.package].version from $cargo_toml" >&2
        return 1
    fi

    printf '%s\n' "$version"
}

version_code_from_semver() {
    local version="$1"
    if [[ ! "$version" =~ ^(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})$ ]]; then
        echo "version_code_from_semver: '$version' is not a bare major.minor.patch, each component 0-999 (no v prefix, no -pre/+build suffix)" >&2
        return 1
    fi

    local major="${BASH_REMATCH[1]}" minor="${BASH_REMATCH[2]}" patch="${BASH_REMATCH[3]}"

    local code=$((major * 1000000 + minor * 1000 + patch))
    if [ "$code" -eq 0 ]; then
        echo "version_code_from_semver: '$version' yields versionCode 0" >&2
        return 1
    fi

    printf '%d\n' "$code"
}

min_load_alignment() {
    local aligns
    aligns="$(awk '$1 == "LOAD" { print $NF }')"
    if [ -z "$aligns" ]; then
        echo "min_load_alignment: no LOAD segments in input" >&2
        return 1
    fi

    local numeric_re='^(0x[0-9A-Fa-f]+|[0-9]+)$'
    local raw dec min=""
    while IFS= read -r raw; do
        if [[ ! "$raw" =~ $numeric_re ]]; then
            echo "min_load_alignment: LOAD Align '$raw' does not convert to a number" >&2
            return 1
        fi
        dec=$((raw))
        if [ -z "$min" ] || [ "$dec" -lt "$min" ]; then
            min="$dec"
        fi
    done <<< "$aligns"

    printf '%d\n' "$min"
}

patch_version_code() {
    local build_gradle="$1" version_code="$2"
    case "$version_code" in
        ''|*[!0-9]*)
            echo "patch_version_code: version_code '$version_code' is not a bare non-negative integer" >&2
            return 1
            ;;
    esac
    local marker="__ANDROID_BUNDLE_VERSION_CODE_MARKER__"
    local sentinel_re='^[[:space:]]*versionCode = 1$'

    # @law: `grep -c` exits 1, not 0, on zero matches -- `|| true` keeps
    # the explicit occurrences check below the sole arbiter of pass/fail.
    local occurrences
    occurrences="$(grep -cE "$sentinel_re" "$build_gradle" || true)"
    if [ "$occurrences" -ne 1 ]; then
        echo "patch_version_code: $build_gradle has $occurrences occurrence(s) of 'versionCode = 1', expected exactly 1" >&2
        return 1
    fi

    local tmp_marked
    tmp_marked="$(mktemp)"
    sed "s/^\([[:space:]]*\)versionCode = 1\$/\1versionCode = $marker/" "$build_gradle" > "$tmp_marked"

    local marked
    marked="$(grep -cF "versionCode = $marker" "$tmp_marked" || true)"
    if [ "$marked" -ne 1 ]; then
        rm -f "$tmp_marked"
        echo "patch_version_code: the versionCode sentinel did not turn into the internal patch marker ($marked line(s) marked) -- the substitution did not run" >&2
        return 1
    fi

    local tmp_final
    tmp_final="$(mktemp)"
    sed "s/^\([[:space:]]*\)versionCode = $marker\$/\1versionCode = $version_code/" "$tmp_marked" > "$tmp_final"
    rm -f "$tmp_marked"

    if grep -qF "$marker" "$tmp_final"; then
        rm -f "$tmp_final"
        echo "patch_version_code: the internal patch marker survived the second substitution in $build_gradle" >&2
        return 1
    fi
    if ! grep -qE "^[[:space:]]*versionCode = $version_code\$" "$tmp_final"; then
        rm -f "$tmp_final"
        echo "patch_version_code: versionCode = $version_code not found in $build_gradle after patching" >&2
        return 1
    fi

    mv "$tmp_final" "$build_gradle"
}

_looks_like_sha256_fingerprint() {
    [[ "$1" =~ ^([0-9A-F]{2}:){31}[0-9A-F]{2}$ ]]
}

_sha256_fingerprint_from_keytool_output() {
    local out="$1" context="$2"
    local fp
    fp="$(printf '%s\n' "$out" | awk '
        /^[[:space:]]*Certificate fingerprints:[[:space:]]*$/ { in_fp = 1; next }
        in_fp && /^[[:space:]]*SHA256:/ {
            sub(/^[[:space:]]*SHA256:[[:space:]]*/, "")
            print
            exit
        }
        /^[[:space:]]*$/ { in_fp = 0 }
    ')"
    if ! _looks_like_sha256_fingerprint "$fp"; then
        echo "$context: no SHA256 fingerprint found in keytool output" >&2
        return 1
    fi
    printf '%s\n' "$fp"
}

jar_signer_fingerprint() {
    local jar="$1"
    local out status
    out="$(keytool -J-Duser.language=en -J-Duser.country=US -printcert -jarfile "$jar" 2>&1)" && status=0 || status=$?
    if [ "$status" -ne 0 ]; then
        echo "jar_signer_fingerprint: keytool could not read a signer certificate from $jar: $out" >&2
        return 1
    fi
    local signer_count
    signer_count="$(printf '%s\n' "$out" | grep -cE '^Signer #[0-9]+:' || true)"
    if [ "$signer_count" -gt 1 ]; then
        echo "jar_signer_fingerprint: $jar is signed by $signer_count signers, expected exactly 1" >&2
        return 1
    fi
    _sha256_fingerprint_from_keytool_output "$out" "jar_signer_fingerprint: $jar"
}

keystore_alias_fingerprint() {
    local keystore="$1" alias="$2" store_password_var="$3"
    local out status
    out="$(keytool -J-Duser.language=en -J-Duser.country=US -list -v -alias "$alias" -keystore "$keystore" -storepass:env "$store_password_var" 2>&1)" && status=0 || status=$?
    if [ "$status" -ne 0 ]; then
        echo "keystore_alias_fingerprint: keytool could not read alias '$alias' from $keystore: $out" >&2
        return 1
    fi
    _sha256_fingerprint_from_keytool_output "$out" "keystore_alias_fingerprint: alias '$alias' in $keystore"
}

verify_jar_signature() {
    local keystore="$1" jar="$2" expected_fingerprint="$3"
    local verify_out verify_status
    verify_out="$(jarsigner -J-Duser.language=en -J-Duser.country=US -verify -keystore "$keystore" "$jar" 2>&1)" && verify_status=0 || verify_status=$?

    printf '%s' "$verify_out"

    if [ "$verify_status" -ne 0 ]; then
        return 1
    fi
    case "$verify_out" in
        *"jar verified"*) : ;;
        *) return 2 ;;
    esac

    local actual_fingerprint
    if ! actual_fingerprint="$(jar_signer_fingerprint "$jar")"; then
        printf '\nverify_jar_signature: could not establish a single signer certificate for %s\n' "$jar"
        return 4
    fi

    if [ -z "$actual_fingerprint" ] || [ -z "$expected_fingerprint" ] || [ "$actual_fingerprint" != "$expected_fingerprint" ]; then
        printf '\nverify_jar_signature: signer fingerprint %s does not match the expected alias fingerprint %s\n' \
            "$actual_fingerprint" "$expected_fingerprint"
        return 3
    fi

    return 0
}
