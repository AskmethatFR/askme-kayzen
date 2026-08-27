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
# verify_jar_signature classifies a `jarsigner -verify` run into exactly
# one of: 0 (verified, and by the named alias), 1 (jarsigner itself could
# not verify), 2 (verified without failing, but the "jar verified" marker
# text is absent), 3 (verified, but not by the alias asked for). The
# caller owns the resulting message; this function owns only the
# classification, which is what makes it directly testable against a
# fixture jar signed by a DIFFERENT alias than the one it is asked to
# verify against -- a state scripts/android-sign.sh's own sign-then-verify
# contract can never reach on its own, since it always signs and verifies
# with the same single alias.

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
    sed "s/$marker/$version_code/" "$tmp_marked" > "$tmp_final"
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

verify_jar_signature() {
    local keystore="$1" jar="$2" alias="$3"
    local verify_out verify_status
    verify_out="$(jarsigner -verify -keystore "$keystore" "$jar" "$alias" 2>&1)" && verify_status=0 || verify_status=$?

    printf '%s' "$verify_out"

    if [ "$verify_status" -ne 0 ]; then
        return 1
    fi
    case "$verify_out" in
        *"jar verified"*) : ;;
        *) return 2 ;;
    esac
    case "$verify_out" in
        *"not signed by the specified alias"*) return 3 ;;
    esac
    return 0
}
