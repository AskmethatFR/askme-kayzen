#!/usr/bin/env bash
# Pure helpers for the Android release build, sourced by
# scripts/android-verify-alignment.sh, scripts/android-bundle.sh and
# scripts/android-sign.sh.
# Sourceable, side-effect-free: nothing runs until a function below is
# called, matching scripts/verify-instrument.sh's own shape.
#
# workspace_version reads [workspace.package].version out of a Cargo.toml
# path given as its one argument. It is the bundle's READ-BACK oracle: the
# release version no longer comes from Cargo.toml (it is derived from the tag
# at the commit being released, in scripts/android-preflight.sh), so this
# reader exists to prove the bytes the bundle wrote are the bytes it meant to
# write.
#
# set_workspace_version rewrites that same [workspace.package].version line
# with the version the bundle was handed, through a mktemp+mv so a failed
# write can never leave a half-written Cargo.toml behind. It refuses an empty
# version and any version carrying a quote, a backslash or a control
# character: a quote ends the TOML string early and a control character is
# illegal inside one, while the backslash is the subtle one -- the value is
# handed to `awk -v`, which escape-processes it, so a literal `\n` would
# otherwise reach the file as a real newline with the writer still exiting 0.
# It refuses unless the section holds EXACTLY one version line, so an
# ambiguous anchor is never guessed at. It validates nothing about the
# version's SHAPE: that is version_code_from_semver's job, and its caller
# runs it first.
#
# version_code_from_semver refuses a "v" prefix and any -pre/+build suffix:
# its input is always a bare major.minor.patch, with any tag prefix already
# stripped by the caller. It also refuses any component over 999 and a result
# of 0. The Play Store
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
# version_codes_from_track_json reads a Play `tracks/<track>` response body
# on stdin and prints one versionCode per line -- the set the collision
# check tests membership against. An absent `releases` key and an empty
# `releases` list both mean "the track carries no codes", which is NOT an
# error; anything whose SHAPE is unexpected (a body that does not parse, a
# `releases` that is not a list, a release entry that is not an object, a
# `versionCodes` that is not a list) is refused with the offending field
# named. A caller that read a broken response as an empty track would
# upload straight into a collision, which is the one failure this whole
# preflight exists to prevent.
#
# track_contains_version_code reads the same one-per-line set on stdin and
# answers set membership for the versionCode in its first argument. Exact
# equality, deliberately: a substring match would let versionCode 1 collide
# with a stored 10.
#
# env_var_is_nonempty answers "is the environment variable NAMED by its
# argument set to a non-empty value?" -- by name, so that a caller's `set -x`
# trace shows the name and never the value.
#
# write_play_auth_header_file writes `Authorization: Bearer <token>` into
# the file named by its first argument, reading the token from the
# environment variable NAMED by its second -- the same
# name-never-value discipline keystore_alias_fingerprint applies to the
# store password.
#
# fingerprint_matches_expected answers "is this the upload key we
# configured?". It exists because no check inside the keystore can answer
# it: a keystore's own alias always matches its own certificate, so an
# operator who stored the WRONG keystore in the secret passes every alias
# comparison there is. An expected fingerprint that comes from outside the
# keystore is the only thing that discriminates the two, and it refuses an
# empty value on EITHER side rather than comparing them -- two empty
# strings compare equal, which would turn a secret that decoded to nothing
# into a silent pass.
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
readonly PLAY_PACKAGE_NAME="com.askmethat.kayzen"

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

set_workspace_version() {
    local cargo_toml="$1" version="$2"
    if [ -z "$version" ] \
        || [[ "$version" == *'"'* ]] \
        || [[ "$version" == *'\'* ]] \
        || [[ "$version" == *[[:cntrl:]]* ]]; then
        echo "set_workspace_version: version $(printf '%q' "$version") is empty or carries a quote, a backslash or a control character -- refusing to write it into $cargo_toml" >&2
        return 1
    fi
    if [ ! -f "$cargo_toml" ]; then
        echo "set_workspace_version: no Cargo.toml at $cargo_toml" >&2
        return 1
    fi

    local occurrences
    occurrences="$(awk '
        /^\[workspace\.package\]/ { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && /^version[[:space:]]*=/ { count++ }
        END { print count + 0 }
    ' "$cargo_toml")"
    if [ "$occurrences" -ne 1 ]; then
        echo "set_workspace_version: $cargo_toml has $occurrences '[workspace.package].version' line(s), expected exactly 1" >&2
        return 1
    fi

    local tmp
    tmp="$(mktemp)"
    awk -v version="$version" '
        /^\[workspace\.package\]/ { in_section = 1; print; next }
        /^\[/ { in_section = 0 }
        in_section && /^version[[:space:]]*=/ { print "version = \"" version "\""; next }
        { print }
    ' "$cargo_toml" > "$tmp"

    mv "$tmp" "$cargo_toml"
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

_patch_gradle_once() {
    local who="$1" target="$2" start_ere="$3" end_ere="$4" replacement="$5"
    local marker="__ANDROID_GRADLE_PATCH_MARKER__"
    local one="no"
    [ "$start_ere" = "$end_ere" ] && one="yes"

    # @law: `grep -c` exits 1, not 0, on zero matches -- `|| true` keeps the
    # explicit occurrences check below the sole arbiter of pass/fail.
    # @law: the anchor EREs travel through ENVIRON, never `awk -v`: a -v
    # assignment runs its value through the interpreter's escape conversion,
    # which eats the backslashes that make `\(`/`\.` literal -- the anchor
    # then silently matches nothing (verified: -v received
    # `classpath("com.android.tools.build:gradle:8.7.0")`, unescaped).
    local ranges
    ranges="$(ONE="$one" ANCHOR_START="$start_ere" ANCHOR_END="$end_ere" awk '
        BEGIN { done = 0 }
        ENVIRON["ONE"] == "yes" && $0 ~ ("^[[:space:]]*" ENVIRON["ANCHOR_START"] "$") { done++; next }
        ENVIRON["ONE"] == "no" && !opened && $0 ~ ("^[[:space:]]*" ENVIRON["ANCHOR_START"] "$") { opened = 1; next }
        ENVIRON["ONE"] == "no" && opened && $0 ~ ("^[[:space:]]*" ENVIRON["ANCHOR_END"] "$") { done++; opened = 0; next }
        ENVIRON["ONE"] == "no" && opened { next }
        { print > "/dev/null" }
        END { print done + 0 }
    ' "$target" 2>/dev/null || true)"
    if [ -z "$ranges" ]; then
        echo "$who: could not read $target" >&2
        return 1
    fi
    if [ "$ranges" -ne 1 ]; then
        echo "$who: $target has $ranges occurrence(s) of the anchor, expected exactly 1" >&2
        return 1
    fi

    local tmp_marked
    tmp_marked="$(mktemp)"
    ONE="$one" ANCHOR_START="$start_ere" ANCHOR_END="$end_ere" MARKER="$marker" awk '
        BEGIN { opened = 0; done = 0 }
        {
            match($0, /^[[:space:]]*/)
            indent = substr($0, RSTART, RLENGTH)
        }
        ENVIRON["ONE"] == "yes" && !done && $0 ~ ("^[[:space:]]*" ENVIRON["ANCHOR_START"] "$") {
            print indent ENVIRON["MARKER"]; done = 1; next
        }
        ENVIRON["ONE"] == "no" && !opened && $0 ~ ("^[[:space:]]*" ENVIRON["ANCHOR_START"] "$") { opened = 1; next }
        ENVIRON["ONE"] == "no" && opened && $0 ~ ("^[[:space:]]*" ENVIRON["ANCHOR_END"] "$") {
            print indent ENVIRON["MARKER"]; done = 1; opened = 0; next
        }
        ENVIRON["ONE"] == "no" && opened { next }
        { print }
    ' "$target" > "$tmp_marked"

    local marked
    marked="$(grep -cF "$marker" "$tmp_marked" || true)"
    if [ "$marked" -ne 1 ]; then
        rm -f "$tmp_marked"
        echo "$who: the anchor did not turn into the internal patch marker ($marked line(s) marked) -- the substitution did not run" >&2
        return 1
    fi
    if grep -qE "^[[:space:]]*$start_ere\$" "$tmp_marked"; then
        rm -f "$tmp_marked"
        echo "$who: the anchor line survived the first substitution in $target" >&2
        return 1
    fi

    local tmp_final
    tmp_final="$(mktemp)"
    if [ -n "$replacement" ]; then
        sed -E "s|^([[:space:]]*)$marker\$|\1$replacement|" "$tmp_marked" > "$tmp_final"
        rm -f "$tmp_marked"
        if grep -qF "$marker" "$tmp_final"; then
            rm -f "$tmp_final"
            echo "$who: the internal patch marker survived the second substitution in $target" >&2
            return 1
        fi
        local read_back
        read_back="$(awk -v want="$replacement" '
            {
                line = $0
                sub(/^[[:space:]]+/, "", line)
                if (line == want) { count++ }
            }
            END { print count + 0 }
        ' "$tmp_final")"
        if [ "$read_back" -ne 1 ]; then
            rm -f "$tmp_final"
            echo "$who: $replacement not found in $target after patching" >&2
            return 1
        fi
    else
        sed -E "/^[[:space:]]*$marker\$/d" "$tmp_marked" > "$tmp_final"
        rm -f "$tmp_marked"
        if grep -qF "$marker" "$tmp_final"; then
            rm -f "$tmp_final"
            echo "$who: the internal patch marker survived the deletion in $target" >&2
            return 1
        fi
        local leftovers
        leftovers="$(grep -cE "^[[:space:]]*$start_ere\$" "$tmp_final" || true)"
        if [ "$leftovers" -ne 0 ]; then
            rm -f "$tmp_final"
            echo "$who: the anchor line survived the deletion in $target" >&2
            return 1
        fi
    fi

    mv "$tmp_final" "$target"
}

patch_agp_classpath() {
    local build_gradle="$1" agp_version="$2"
    if [[ ! "$agp_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "patch_agp_classpath: AGP version '$agp_version' is not an exact major.minor.patch (floating or partial ranges make the release build non-reproducible)" >&2
        return 1
    fi
    local anchor='classpath\("com\.android\.tools\.build:gradle:8\.7\.0"\)'
    local replacement="classpath(\"com.android.tools.build:gradle:$agp_version\")"
    _patch_gradle_once "patch_agp_classpath" "$build_gradle" "$anchor" "$anchor" "$replacement"
}

patch_material_dependency() {
    local build_gradle="$1" material_version="$2"
    if [[ ! "$material_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "patch_material_dependency: Material version '$material_version' is not an exact major.minor.patch (floating or partial ranges make the release build non-reproducible)" >&2
        return 1
    fi
    local anchor='implementation\("com\.google\.android\.material:material:1\.13\.0"\)'
    local replacement="implementation(\"com.google.android.material:material:$material_version\")"
    _patch_gradle_once "patch_material_dependency" "$build_gradle" "$anchor" "$anchor" "$replacement"
}

patch_built_in_kotlin() {
    local build_gradle="$1"
    local plugin_anchor='id\("org\.jetbrains\.kotlin\.android"\)'
    _patch_gradle_once "patch_built_in_kotlin" "$build_gradle" \
        "$plugin_anchor" "$plugin_anchor" "" || return 1
    local block_start='kotlinOptions \{'
    local block_end='\}'
    _patch_gradle_once "patch_built_in_kotlin" "$build_gradle" \
        "$block_start" "$block_end" ""
}

patch_dead_buildconfig_default() {
    local gradle_properties="$1"
    local anchor='android\.defaults\.buildfeatures\.buildconfig=true'
    _patch_gradle_once "patch_dead_buildconfig_default" "$gradle_properties" \
        "$anchor" "$anchor" ""
}

patch_main_activity_edge_to_edge() {
    local main_activity="$1"
    if [ ! -f "$main_activity" ]; then
        echo "patch_main_activity_edge_to_edge: no MainActivity.kt at $main_activity" >&2
        return 1
    fi
    local anchor_re='class MainActivity : WryActivity\(\)'
    local marker="__ANDROID_MAIN_ACTIVITY_EDGE_TO_EDGE_MARKER__"
    local block
    block="$(cat <<'KOTLIN'
class MainActivity : WryActivity() {
    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)
        androidx.core.view.ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { view, insets ->
            val bars = insets.getInsets(
                androidx.core.view.WindowInsetsCompat.Type.systemBars() or
                    androidx.core.view.WindowInsetsCompat.Type.displayCutout()
            )
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            insets
        }
    }
}
KOTLIN
)"

    # @law: `grep -c` exits 1, not 0, on zero matches -- `|| true` keeps the
    # explicit occurrences check below the sole arbiter of pass/fail.
    local occurrences
    occurrences="$(grep -cE "^[[:space:]]*$anchor_re\$" "$main_activity" || true)"
    if [ "$occurrences" -ne 1 ]; then
        echo "patch_main_activity_edge_to_edge: $main_activity has $occurrences occurrence(s) of the MainActivity anchor, expected exactly 1" >&2
        return 1
    fi

    local tmp_marked
    tmp_marked="$(mktemp)"
    ANCHOR="$anchor_re" MARKER="$marker" awk '
        {
            match($0, /^[[:space:]]*/)
            indent = substr($0, RSTART, RLENGTH)
        }
        $0 ~ ("^[[:space:]]*" ENVIRON["ANCHOR"] "$") {
            print indent ENVIRON["MARKER"]
            next
        }
        { print }
    ' "$main_activity" > "$tmp_marked"

    local marked
    marked="$(grep -cF "$marker" "$tmp_marked" || true)"
    if [ "$marked" -ne 1 ]; then
        rm -f "$tmp_marked"
        echo "patch_main_activity_edge_to_edge: the anchor did not turn into the internal patch marker ($marked line(s) marked) -- the substitution did not run" >&2
        return 1
    fi
    if grep -qE "^[[:space:]]*$anchor_re\$" "$tmp_marked"; then
        rm -f "$tmp_marked"
        echo "patch_main_activity_edge_to_edge: the anchor line survived the first substitution in $main_activity" >&2
        return 1
    fi

    # @law: the replacement is a whole Kotlin class BODY, so the marker swap
    # runs in awk over ENVIRON (a multi-line value survives intact); sed's s
    # command cannot carry embedded newlines into its replacement text.
    local tmp_final swap_status=0
    tmp_final="$(mktemp)"
    MARKER="$marker" BLOCK="$block" awk '
        $0 ~ ENVIRON["MARKER"] {
            printf "%s\n", ENVIRON["BLOCK"]
            replaced++
            next
        }
        { print }
        END { if (replaced != 1) exit 1 }
    ' "$tmp_marked" > "$tmp_final" || swap_status=$?
    rm -f "$tmp_marked"
    if [ "$swap_status" -ne 0 ]; then
        rm -f "$tmp_final"
        echo "patch_main_activity_edge_to_edge: the marker swap failed in $main_activity (awk exited $swap_status)" >&2
        return 1
    fi
    if grep -qF "$marker" "$tmp_final"; then
        rm -f "$tmp_final"
        echo "patch_main_activity_edge_to_edge: the internal patch marker survived the swap in $main_activity" >&2
        return 1
    fi

    local read_back
    read_back="$(grep -cF 'WindowCompat.setDecorFitsSystemWindows(window, false)' "$tmp_final" || true)"
    if [ "$read_back" -ne 1 ]; then
        rm -f "$tmp_final"
        echo "patch_main_activity_edge_to_edge: the edge-to-edge call was not read back exactly once in $main_activity" >&2
        return 1
    fi
    if grep -qE '^[[:space:]]*class MainActivity : WryActivity\(\)$' "$tmp_final"; then
        rm -f "$tmp_final"
        echo "patch_main_activity_edge_to_edge: the bare anchor line is still present in $main_activity after patching" >&2
        return 1
    fi

    mv "$tmp_final" "$main_activity"
}

deprecated_bar_api_refs() {
    # Reads one dex file on stdin. Prints one `Landroid/view/Window;-><name>`
    # line per method_id whose class is android.view.Window AND whose name is
    # one of the two deprecated setters; exits 0 = none found, 1 = at least
    # one found, 2 = the input is not a dex file (cannot verify).
    #
    # @law: the check is on the method_id TABLE's (class, name) pair, never
    # on a substring of the bytes: a dex string pool stores the class
    # descriptor and the method name as SEPARATE entries, so the
    # concatenated `Landroid/view/Window;->setStatusBarColor` form a byte
    # scan would look for does not occur in any real dex -- verified against
    # this repo's own AAB, which carries both method_ids while the
    # concatenated needle is absent (a substring scan is a silent false
    # green here). The clean test fixture deliberately carries the bare
    # strings too, so only a pair scan passes it.
    python3 -c '
import struct
import sys


def refuse(message):
    sys.stderr.write("deprecated_bar_api_refs: " + message + "\n")
    sys.exit(2)


data = sys.stdin.buffer.read()
if len(data) < 0x70 or data[:4] != b"dex\n":
    refuse("input is not a dex file (bad magic)")


def u4(offset):
    if offset + 4 > len(data):
        refuse("input is truncated (header out of bounds)")
    return struct.unpack_from("<I", data, offset)[0]


string_ids_size = u4(56)
string_ids_off = u4(60)
type_ids_size = u4(64)
type_ids_off = u4(68)
method_ids_size = u4(88)
method_ids_off = u4(92)
if (
    string_ids_off + 4 * string_ids_size > len(data)
    or type_ids_off + 4 * type_ids_size > len(data)
    or method_ids_off + 8 * method_ids_size > len(data)
):
    refuse("input is truncated (table out of bounds)")


def uleb(offset):
    result = 0
    shift = 0
    while True:
        if offset >= len(data):
            refuse("input is truncated (string data out of bounds)")
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return result, offset
        shift += 7


def string_at(index):
    if index >= string_ids_size:
        refuse("string index out of bounds")
    offset = u4(string_ids_off + 4 * index)
    length, start = uleb(offset)
    if start + length > len(data):
        refuse("string data out of bounds")
    return data[start:start + length]


window_types = set()
for type_index in range(type_ids_size):
    if string_at(u4(type_ids_off + 4 * type_index)) == b"Landroid/view/Window;":
        window_types.add(type_index)

target_names = set()
for index in range(string_ids_size):
    value = string_at(index)
    if value in (b"setStatusBarColor", b"setNavigationBarColor"):
        target_names.add(index)

found = set()
for method_index in range(method_ids_size):
    base = method_ids_off + 8 * method_index
    class_index = struct.unpack_from("<H", data, base)[0]
    name_index = struct.unpack_from("<I", data, base + 4)[0]
    if class_index in window_types and name_index in target_names:
        found.add(string_at(name_index))

for name in (b"setStatusBarColor", b"setNavigationBarColor"):
    if name in found:
        print("Landroid/view/Window;->" + name.decode("ascii"))
sys.exit(1 if found else 0)
'
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
    if [ "$signer_count" -ne 1 ]; then
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

version_codes_from_track_json() {
    python3 -c '
import json
import sys


def refuse(message):
    sys.stderr.write("version_codes_from_track_json: " + message + "\n")
    sys.exit(1)


try:
    document = json.load(sys.stdin)
except ValueError as error:
    refuse("the track response is not valid JSON: %s" % error)

if not isinstance(document, dict):
    refuse("expected a JSON object for the track response, got %s" % type(document).__name__)

releases = document.get("releases", [])
if not isinstance(releases, list):
    refuse("\"releases\" is not a list in the track response")

for release in releases:
    if not isinstance(release, dict):
        refuse("a release entry in \"releases\" is not a JSON object")
    codes = release.get("versionCodes", [])
    if not isinstance(codes, list):
        refuse("\"versionCodes\" is not a list in a release entry")
    for code in codes:
        print(code)
'
}

track_contains_version_code() {
    local version_code="$1" line
    while IFS= read -r line; do
        if [ "$line" = "$version_code" ]; then
            return 0
        fi
    done
    return 1
}

env_var_is_nonempty() {
    local name="$1"
    # @law: the test runs inside `set +x`, and takes the variable's NAME --
    # `[ -n "$TOKEN" ]` expanded under `set -x` prints the token itself into
    # the trace, which is the leak AC 17 forbids and the trace test watches
    # for. The subshell keeps the suppression local to this check.
    ( set +x
      [ -n "${!name:-}" ] )
}

write_play_auth_header_file() {
    local header_file="$1" token_var="$2"
    # @law: `set +x` inside a SUBSHELL, never in this one: a `set -x` run of
    # a caller would otherwise print the printf's expansion -- and with it
    # the token, verbatim, into the job log. The subshell keeps the
    # suppression local, so the caller's own tracing resumes untouched.
    ( set +x
      printf 'Authorization: Bearer %s\n' "${!token_var}" > "$header_file" )
}

fingerprint_matches_expected() {
    local actual="$1" expected="$2"
    if [ -z "$actual" ] || [ -z "$expected" ]; then
        echo "fingerprint_matches_expected: an empty fingerprint never matches (keystore alias '$actual', configured expected '$expected')" >&2
        return 1
    fi
    if [ "$actual" != "$expected" ]; then
        echo "fingerprint_matches_expected: keystore alias fingerprint $actual does not match the configured upload-key fingerprint $expected" >&2
        return 1
    fi
    return 0
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
