#!/usr/bin/env bash
# Builds the unsigned release Android App Bundle (.aab): 16 KB page-size
# aligned, real launcher icon, correct versionCode/versionName. Nothing is
# signed here -- that is scripts/android-sign.sh, a later slice.
#
# Usage: scripts/android-bundle.sh
# Progress goes to stderr; the ONLY line on stdout is the produced AAB's
# path, so `aab="$(scripts/android-bundle.sh)"` composes directly.
#
# Two passes, not one, and in this order:
#   1. `dx bundle` with --rustc-args carrying the 16 KB alignment flags
#      (see the comment on ALIGN_RUSTC_ARGS below), producing the Gradle
#      project AND compiling libmain.so.
#   2. the launcher icon is applied, the generated build.gradle.kts is
#      patched with the real versionCode, and Gradle alone re-packages the
#      bundle around the SAME .so `dx` already linked. It does not recompile
#      Rust: Gradle just re-zips what pass 1 deposited into jniLibs, so pass
#      2 is what actually produces the .aab and the icon and version patch
#      survive into it.
# A single `dx bundle` cannot do both: `dx` owns the whole generated Gradle
# project and rewrites it on every run, so anything hand-patched into it
# before a `dx bundle` call is gone by the time that call finishes.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$REPO_ROOT/scripts/android-release-lib.sh"

ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
NDK_HOME="${NDK_HOME:-$ANDROID_HOME/ndk/25.2.9519653}"
JAVA_HOME="${JAVA_HOME:-$HOME/Library/Java/JavaVirtualMachines/corretto-17.0.13/Contents/Home}"
export ANDROID_HOME NDK_HOME JAVA_HOME

GRADLE_PROJECT="$REPO_ROOT/target/dx/kayzen-app/release/android/app"
MODULE="$GRADLE_PROJECT/app"
GENERATED_RES="$MODULE/src/main/res"
BUILD_GRADLE="$MODULE/build.gradle.kts"
AAB="$MODULE/build/outputs/bundle/release/app-release.aab"
# NOT outputs/bundle/release/output-metadata.json: AGP only writes a
# versionCode/versionName-carrying output-metadata.json for an APK variant
# (artifactType "APK"). The bundle task's own IDE listing file
# (produceReleaseBundleIdeListingFile/output-metadata.json) exists but
# carries no version fields at all -- verified against a real local build,
# not assumed. The manifest AGP actually folds into the packaged .aab DOES
# carry both, applied fresh by processApplicationManifestReleaseForBundle
# every time build.gradle.kts's versionCode/versionName change, which is
# exactly the read-the-artifact signal the manifest read-back below needs.
META="$MODULE/build/intermediates/bundle_manifest/release/processApplicationManifestReleaseForBundle/AndroidManifest.xml"

# `dx` sets RUSTFLAGS itself and blanks
# CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS, so a .cargo/config.toml
# entry for this flag is dead, and silently -- and exporting RUSTFLAGS
# before calling `dx` is not a safe substitute, because it is not
# established that `dx` appends to what it inherits rather than replacing
# it. `dx build|bundle --rustc-args "<args>"` becomes
# `cargo rustc -- <args>`, applied to the final compilation unit -- the one
# that links libmain.so -- which is the only place this flag can land.
readonly ALIGN_RUSTC_ARGS="-Clink-arg=-Wl,-z,max-page-size=16384 -Clink-arg=-Wl,-z,common-page-size=16384"

fail() {
    echo "android-bundle: $1" >&2
    exit 1
}

[ -d "$ANDROID_HOME" ] || fail "no Android SDK at $ANDROID_HOME (set ANDROID_HOME)"
[ -d "$NDK_HOME" ] || fail "no NDK at $NDK_HOME (set NDK_HOME)"
[ -x "$JAVA_HOME/bin/java" ] || fail "no JDK 17 at $JAVA_HOME (set JAVA_HOME)"

rustup target list --installed | grep -qx aarch64-linux-android \
    || fail "missing Rust target (rustup target add aarch64-linux-android)"

command -v dx >/dev/null || fail "dx not found (cargo install dioxus-cli)"

# The generated module's own recursive fileTree(".") already picks up
# proguard-wry.pro, which covers the JNI/reflective surface R8 can strip. A
# [bundle.android] section overriding the template's own generation would go
# untested by that mitigation -- see the comment this refers to in
# app/Dioxus.toml. Checked via tomllib, not a `grep` line match: TOML allows
# the same table under several different literal spellings (leading
# whitespace, a tab, quoted-key form, dotted inline form, ...), and a
# `grep -qE '^\[bundle\.android\]'` misses all of them.
if ! python3 -c '
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    data = tomllib.load(f)
sys.exit(1 if "android" in data.get("bundle", {}) else 0)
' "$REPO_ROOT/app/Dioxus.toml"; then
    fail "app/Dioxus.toml carries a [bundle.android] section -- see its own comment, remove it"
fi

echo "==> cleaning generated resources" >&2
# >&2 on both: android-icon.sh's own "apply" success line goes to its
# stdout, and this script's own stdout contract (see the header above)
# allows exactly one line, the final AAB path.
"$REPO_ROOT/scripts/android-icon.sh" clean "$GENERATED_RES" >&2

echo "==> pass 1: dx bundle (release, aab, 16 KB page-size aligned)" >&2
# --rustc-args="..." (the = form), never a space before the value: clap
# reads a space-separated value starting with "-C" as a new flag of `dx
# bundle` itself ("unexpected argument '-C' found"), not as this flag's
# value.
(cd "$REPO_ROOT/app" && dx bundle --platform android --release --package-types aab --rustc-args="$ALIGN_RUSTC_ARGS") >&2

echo "==> applying the launcher icon" >&2
"$REPO_ROOT/scripts/android-icon.sh" apply "$GENERATED_RES" >&2

[ -f "$BUILD_GRADLE" ] || fail "dx did not generate $BUILD_GRADLE"

echo "==> checking the generated build.gradle.kts for a cleartext signing config" >&2
# Defence-in-depth (the [bundle.android] preflight above is the load-bearing
# control, see app/Dioxus.toml's own comment): matches both the property
# form (storePassword = ...) and the setter form
# (signingConfig.setStorePassword(...) / signingConfigs.getByName(...)) --
# `dx` could plausibly emit either.
if grep -qE 'storePassword|keyPassword|keyAlias|storeFile|signingConfig' "$BUILD_GRADLE"; then
    # The refusal is fail-closed either way, but a real keystore password
    # left interpolated on disk in $REPO_ROOT/target/ after a failed build
    # is exactly the shape a future signed build (scripts/android-sign.sh)
    # would make load-bearing -- delete it now while it is still harmless.
    rm -f "$BUILD_GRADLE"
    fail "$BUILD_GRADLE carries a cleartext signing config -- refusing to bundle"
fi

echo "==> reading the workspace version" >&2
VERSION="$(awk '
    /^\[workspace\.package\]/ { in_section = 1; next }
    /^\[/ { in_section = 0 }
    in_section && /^version[[:space:]]*=/ {
        match($0, /"[^"]*"/)
        print substr($0, RSTART + 1, RLENGTH - 2)
        exit
    }
' "$REPO_ROOT/Cargo.toml")"
[ -n "$VERSION" ] || fail "could not read [workspace.package].version from Cargo.toml"
VERSION_CODE="$(version_code_from_semver "$VERSION")"

echo "==> patching versionCode ($VERSION -> $VERSION_CODE)" >&2
# `grep -c` exits 1 (not 0) on zero matches -- under `set -e`/pipefail that
# would abort this assignment BEFORE the "expected exactly 1" message below
# ever runs, turning the exact regression this check exists to catch (dx's
# template stops emitting the literal) into total silence instead of a
# refusal. `|| true` lets the explicit occurrences check below be the one
# thing that decides pass/fail.
occurrences="$(grep -cE '^[[:space:]]*versionCode = 1$' "$BUILD_GRADLE" || true)"
[ "$occurrences" -eq 1 ] \
    || fail "$BUILD_GRADLE has $occurrences occurrence(s) of 'versionCode = 1', expected exactly 1"

tmp_gradle="$(mktemp)"
trap 'rm -f "$tmp_gradle"' EXIT
sed "s/^\([[:space:]]*\)versionCode = 1\$/\1versionCode = $VERSION_CODE/" "$BUILD_GRADLE" > "$tmp_gradle"
mv "$tmp_gradle" "$BUILD_GRADLE"

grep -qE "^[[:space:]]*versionCode = $VERSION_CODE\$" "$BUILD_GRADLE" \
    || fail "versionCode patch did not land in $BUILD_GRADLE"
grep -qE '^[[:space:]]*versionCode = 1$' "$BUILD_GRADLE" \
    && fail "the old versionCode = 1 survived the patch in $BUILD_GRADLE"
grep -qE "^[[:space:]]*versionName = \"$VERSION\"\$" "$BUILD_GRADLE" \
    || fail "versionName \"$VERSION\" not found in $BUILD_GRADLE"

# $META is the load-bearing one: its absence at the check below is the ONLY
# signal that AGP did not regenerate the bundle manifest this run. Without
# clearing it first, a stale $META from a previous run would satisfy that
# check and get read back further down, passing while proving nothing about
# the build that just ran. $AAB is defence-in-depth: on the AGP/Gradle
# pairing this repo pins, output-hash tracking already forces a rebuild on
# any versionCode/versionName change (a deliberately-stale $AAB survived
# unchanged through a real rebuild when tried) -- but that is a Gradle
# implementation detail, not a contract this script can rely on, hence
# clearing it too rather than trusting it.
echo "==> clearing any stale bundle output" >&2
rm -f "$AAB" "$META"

echo "==> pass 2: gradlew bundleRelease" >&2
(cd "$GRADLE_PROJECT" && ./gradlew --quiet bundleRelease) >&2

[ -f "$AAB" ] || fail "gradlew bundleRelease produced no AAB at $AAB"
[ -f "$META" ] || fail "gradlew bundleRelease produced no bundle manifest at $META"

echo "==> verifying page-size alignment" >&2
"$REPO_ROOT/scripts/android-verify-alignment.sh" "$AAB" >&2

echo "==> reading back the manifest AGP folded into the bundle" >&2
# Same shape as the occurrences check above: a `grep -oE` with no match
# exits 1 and, under pipefail, would abort the assignment before the "no
# ...found" refusals below ever ran -- silent on exactly the case (AGP
# stops emitting the attribute, or reshapes the manifest) those refusals
# exist to name. `|| true` on each pipeline; the explicit checks below
# decide pass/fail.
produced_version_code="$(grep -oE 'android:versionCode="[0-9]+"' "$META" | head -1 | grep -oE '[0-9]+' || true)"
produced_version_name="$(grep -oE 'android:versionName="[^"]*"' "$META" | head -1 | sed -E 's/^android:versionName="(.*)"$/\1/' || true)"

[ -n "$produced_version_code" ] || fail "no android:versionCode found in $META"
[ "$produced_version_code" = "$VERSION_CODE" ] \
    || fail "$META versionCode is $produced_version_code, expected $VERSION_CODE"
[ -n "$produced_version_name" ] || fail "no android:versionName found in $META"
[ "$produced_version_name" = "$VERSION" ] \
    || fail "$META versionName is '$produced_version_name', expected '$VERSION'"

echo "==> $AAB" >&2
printf '%s\n' "$AAB"
