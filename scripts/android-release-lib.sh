#!/usr/bin/env bash
# Pure helpers for the Android release build, sourced by
# scripts/android-verify-alignment.sh and scripts/android-bundle.sh.
# Sourceable, side-effect-free: nothing runs until a function below is
# called, matching scripts/verify-instrument.sh's own shape.
#
# workspace_version reads [workspace.package].version out of a Cargo.toml
# path given as its one argument. scripts/android-bundle.sh is its only
# caller: it feeds the result straight into version_code_from_semver below,
# which is what makes the Cargo.toml -> versionCode binding provable by
# this file's own test harness instead of living in untestable inline awk.
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
