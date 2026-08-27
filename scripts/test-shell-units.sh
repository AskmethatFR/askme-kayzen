#!/usr/bin/env bash
# House test harness for the shell-only Android release library
# (scripts/android-release-lib.sh), run by check.sh's "shell units" gate the
# same way scenario_gate wraps scenario_audit.py: this file's own verdict
# line is what the gate trusts, an exit 0 with no verdict line is not.
#
# Deliberately NOT `set -e` -- same doctrine as check.sh itself (see its own
# header): this harness's whole job is to keep going after one assertion
# fails and report everything that is broken in one run, not just the first
# thing it trips over.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/android-release-lib.sh
source "$ROOT/scripts/android-release-lib.sh"

PASS=0
FAIL=0

assert_eq() {
    local expected="$1" actual="$2" label="$3"
    if [ "$expected" = "$actual" ]; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        printf 'FAIL %s: expected %q, got %q\n' "$label" "$expected" "$actual" >&2
    fi
}

# Requires BOTH a non-zero exit AND a message on stderr -- a command that
# fails silently, or one whose stderr is populated but exits 0, is not a
# refusal, and neither on its own is allowed to read as one.
assert_refuses() {
    local label="$1"
    shift
    if [ "${1:-}" != "--" ]; then
        FAIL=$((FAIL + 1))
        printf 'FAIL %s: assert_refuses called without a -- separator\n' "$label" >&2
        return
    fi
    shift

    local stderr_out status
    stderr_out="$("$@" 2>&1 1>/dev/null)"
    status=$?
    if [ "$status" -ne 0 ] && [ -n "$stderr_out" ]; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        printf 'FAIL %s: expected a refusal (nonzero exit + stderr message), got status=%s stderr=%q\n' \
            "$label" "$status" "$stderr_out" >&2
    fi
}

# min_load_alignment reads its dump on stdin; assert_refuses only forwards
# argv, so a fixture that needs to reach stdin goes through this instead of
# being called directly.
min_load_alignment_stdin() {
    printf '%s' "$1" | min_load_alignment
}

# --- fixtures -----------------------------------------------------------
# DUMP_REAL_4K is a verbatim `llvm-readelf -l` capture (Program Headers
# section) of this repo's own debug libmain.so, arm64-v8a, on NDK r25c.
# Every LOAD segment sits at Align 0x1000: this is the regression the whole
# ticket exists to fix.
DUMP_REAL_4K="$(cat <<'EOF'
Program Headers:
  Type           Offset   VirtAddr           PhysAddr           FileSiz  MemSiz   Flg Align
  PHDR           0x000040 0x0000000000000040 0x0000000000000040 0x000268 0x000268 R   0x8
  INTERP         0x0002a8 0x00000000000002a8 0x00000000000002a8 0x000015 0x000015 R   0x1
      [Requesting program interpreter: /system/bin/linker64]
  LOAD           0x000000 0x0000000000000000 0x0000000000000000 0x230309c 0x230309c R   0x1000
  LOAD           0x230309c 0x000000000230409c 0x000000000230409c 0x1c9dec4 0x1c9dec4 R E 0x1000
  LOAD           0x3fa0f60 0x0000000003fa2f60 0x0000000003fa2f60 0x199658 0x199658 RW  0x1000
  LOAD           0x413a5c0 0x000000000413d5c0 0x000000000413d5c0 0x044f88 0x047cf8 RW  0x1000
  DYNAMIC        0x4136910 0x0000000004138910 0x0000000004138910 0x000200 0x000200 RW  0x8
  GNU_RELRO      0x3fa0f60 0x0000000003fa2f60 0x0000000003fa2f60 0x199658 0x19a0a0 R   0x1
  GNU_EH_FRAME   0x1ca69b0 0x0000000001ca69b0 0x0000000001ca69b0 0x10c44c 0x10c44c R   0x4
  GNU_STACK      0x000000 0x0000000000000000 0x0000000000000000 0x000000 0x000000 RW  0x0
  NOTE           0x0002c0 0x00000000000002c0 0x00000000000002c0 0x0000bc 0x0000bc R   0x4
EOF
)"

# Same shape, every LOAD Align raised to 0x4000 (16384) -- what the
# alignment flag applied in scripts/android-bundle.sh is supposed to produce.
DUMP_ALIGNED_16K="$(cat <<'EOF'
Program Headers:
  Type           Offset   VirtAddr           PhysAddr           FileSiz  MemSiz   Flg Align
  PHDR           0x000040 0x0000000000000040 0x0000000000000040 0x000268 0x000268 R   0x8
  INTERP         0x0002a8 0x00000000000002a8 0x00000000000002a8 0x000015 0x000015 R   0x1
      [Requesting program interpreter: /system/bin/linker64]
  LOAD           0x000000 0x0000000000000000 0x0000000000000000 0x230309c 0x230309c R   0x4000
  LOAD           0x230309c 0x000000000230409c 0x000000000230409c 0x1c9dec4 0x1c9dec4 R E 0x4000
  LOAD           0x3fa0f60 0x0000000003fa2f60 0x0000000003fa2f60 0x199658 0x199658 RW  0x4000
  LOAD           0x413a5c0 0x000000000413d5c0 0x000000000413d5c0 0x044f88 0x047cf8 RW  0x4000
  DYNAMIC        0x4136910 0x0000000004138910 0x0000000004138910 0x000200 0x000200 RW  0x8
  GNU_RELRO      0x3fa0f60 0x0000000003fa2f60 0x0000000003fa2f60 0x199658 0x19a0a0 R   0x1
  GNU_STACK      0x000000 0x0000000000000000 0x0000000000000000 0x000000 0x000000 RW  0x0
EOF
)"

# The minimum sits on the THIRD line, neither first nor last, and is not the
# most common value -- proves the function scans every LOAD line rather than
# reading the first one or the mode.
DUMP_MIXED_ALIGN="$(cat <<'EOF'
Program Headers:
  Type           Offset   VirtAddr           PhysAddr           FileSiz  MemSiz   Flg Align
  LOAD           0x000000 0x0000000000000000 0x0000000000000000 0x230309c 0x230309c R   0x4000
  LOAD           0x230309c 0x000000000230409c 0x000000000230409c 0x1c9dec4 0x1c9dec4 R E 0x4000
  LOAD           0x3fa0f60 0x0000000003fa2f60 0x0000000003fa2f60 0x199658 0x199658 RW  0x1000
  LOAD           0x413a5c0 0x000000000413d5c0 0x000000000413d5c0 0x044f88 0x047cf8 RW  0x4000
  GNU_STACK      0x000000 0x0000000000000000 0x0000000000000000 0x000000 0x000000 RW  0x0
EOF
)"

# Every non-LOAD segment type the real dump carries, with every LOAD line
# removed -- proves the function distinguishes "read fine, nothing matched"
# from a truly empty input.
DUMP_NO_LOAD="$(cat <<'EOF'
Program Headers:
  Type           Offset   VirtAddr           PhysAddr           FileSiz  MemSiz   Flg Align
  PHDR           0x000040 0x0000000000000040 0x0000000000000040 0x000268 0x000268 R   0x8
  DYNAMIC        0x4136910 0x0000000004138910 0x0000000004138910 0x000200 0x000200 RW  0x8
  GNU_RELRO      0x3fa0f60 0x0000000003fa2f60 0x0000000003fa2f60 0x199658 0x19a0a0 R   0x1
  GNU_STACK      0x000000 0x0000000000000000 0x0000000000000000 0x000000 0x000000 RW  0x0
EOF
)"

# One LOAD line's Align field is not a number at all.
DUMP_NONNUMERIC_ALIGN="$(cat <<'EOF'
Program Headers:
  Type           Offset   VirtAddr           PhysAddr           FileSiz  MemSiz   Flg Align
  LOAD           0x000000 0x0000000000000000 0x0000000000000000 0x230309c 0x230309c R   0x1000
  LOAD           0x230309c 0x000000000230409c 0x000000000230409c 0x1c9dec4 0x1c9dec4 R E bogus
  GNU_STACK      0x000000 0x0000000000000000 0x0000000000000000 0x000000 0x000000 RW  0x0
EOF
)"

# --- REQUIRED_PAGE_ALIGNMENT ------------------------------------------------
# The one number this whole ticket exists to enforce. Pinned directly: no
# fixture round-trip (DUMP_ALIGNED_16K below) constrains the POLICY value,
# only the parser reading whatever value is baked into it.
assert_eq "16384" "$REQUIRED_PAGE_ALIGNMENT" \
    "REQUIRED_PAGE_ALIGNMENT is the 16 KB Play floor"

# --- version_code_from_semver --------------------------------------------

assert_eq "1000" "$(version_code_from_semver "0.1.0")" \
    "version_code_from_semver: 0.1.0 -> 1000"
assert_eq "1000000" "$(version_code_from_semver "1.0.0")" \
    "version_code_from_semver: 1.0.0 -> 1000000"
assert_eq "1" "$(version_code_from_semver "0.0.1")" \
    "version_code_from_semver: 0.0.1 -> 1"
assert_eq "999999999" "$(version_code_from_semver "999.999.999")" \
    "version_code_from_semver: 999.999.999 -> 999999999"

patch_carry="no"
a="$(version_code_from_semver "0.0.999")"
b="$(version_code_from_semver "0.1.0")"
[ "$a" -lt "$b" ] && patch_carry="yes"
assert_eq "yes" "$patch_carry" "version_code_from_semver: 0.0.999 < 0.1.0 (patch carry)"

minor_carry="no"
c="$(version_code_from_semver "0.999.999")"
d="$(version_code_from_semver "1.0.0")"
[ "$c" -lt "$d" ] && minor_carry="yes"
assert_eq "yes" "$minor_carry" "version_code_from_semver: 0.999.999 < 1.0.0 (minor carry)"

assert_refuses "version_code_from_semver: empty string" -- version_code_from_semver ""
assert_refuses "version_code_from_semver: two components" -- version_code_from_semver "0.1"
assert_refuses "version_code_from_semver: four components" -- version_code_from_semver "0.1.0.0"
assert_refuses "version_code_from_semver: v prefix" -- version_code_from_semver "v0.1.0"
assert_refuses "version_code_from_semver: pre-release suffix" -- version_code_from_semver "0.1.0-rc1"
assert_refuses "version_code_from_semver: build metadata suffix" -- version_code_from_semver "0.1.0+build3"
assert_refuses "version_code_from_semver: patch over 999" -- version_code_from_semver "1.0.1000"
assert_refuses "version_code_from_semver: all-zero version" -- version_code_from_semver "0.0.0"
assert_refuses "version_code_from_semver: leading zero" -- version_code_from_semver "01.2.3"

# @law: test(1) exits 2 (read by `if` as false) on an out-of-int64
# operand -- the regex above rejects a >3-digit component before any
# arithmetic can hit that.
assert_refuses "version_code_from_semver: major far beyond int64" \
    -- version_code_from_semver "10000000000000000000.0.0"
assert_refuses "version_code_from_semver: patch far beyond int64" \
    -- version_code_from_semver "0.0.9223372036854775808"

# --- workspace_version ------------------------------------------------------
# AC 8b: the real Cargo.toml -> versionCode binding must be provable by this
# harness, which is exactly what extracting the reader out of
# android-bundle.sh's inline awk (D-1) makes possible.
assert_eq "0.0.1" "$(workspace_version "$ROOT/Cargo.toml")" \
    "workspace_version: the real Cargo.toml -> 0.0.1"
assert_eq "1" "$(version_code_from_semver "$(workspace_version "$ROOT/Cargo.toml")")" \
    "workspace_version -> version_code_from_semver: the real Cargo.toml -> versionCode 1 (AC 8b)"

write_cargo_toml_fixture() {
    printf '%s' "$2" > "$1"
}

WSV_ROOT="$(mktemp -d)"

write_cargo_toml_fixture "$WSV_ROOT/plain.toml" '[workspace]
resolver = "3"
members = ["core", "app"]

[workspace.package]
version = "2.5.7"
edition = "2024"
'
assert_eq "2.5.7" "$(workspace_version "$WSV_ROOT/plain.toml")" \
    "workspace_version: quote-stripping on a synthetic fixture"

# Kills the "wrong section" mutant (mandatory hand-mutant a): a [package]
# section carrying its OWN version line, ahead of [workspace.package], must
# never be the one picked.
write_cargo_toml_fixture "$WSV_ROOT/wrong-section.toml" '[package]
version = "9.9.9"
name = "not-the-workspace"

[workspace.package]
version = "0.0.1"
edition = "2024"
'
assert_eq "0.0.1" "$(workspace_version "$WSV_ROOT/wrong-section.toml")" \
    "workspace_version: picks [workspace.package], never an earlier [package] (kills the wrong-section mutant)"

assert_refuses "workspace_version: nonexistent path" \
    -- workspace_version "$WSV_ROOT/does-not-exist.toml"

write_cargo_toml_fixture "$WSV_ROOT/no-workspace-package.toml" '[package]
version = "1.0.0"
name = "not-a-workspace"
'
assert_refuses "workspace_version: no [workspace.package] section" \
    -- workspace_version "$WSV_ROOT/no-workspace-package.toml"

write_cargo_toml_fixture "$WSV_ROOT/no-version-key.toml" '[workspace.package]
edition = "2024"
'
assert_refuses "workspace_version: [workspace.package] present but no version key" \
    -- workspace_version "$WSV_ROOT/no-version-key.toml"

rm -rf "$WSV_ROOT"

# --- min_load_alignment ---------------------------------------------------

assert_eq "4096" "$(printf '%s' "$DUMP_REAL_4K" | min_load_alignment)" \
    "min_load_alignment: real 4 KB dump -> 4096"
assert_eq "16384" "$(printf '%s' "$DUMP_ALIGNED_16K" | min_load_alignment)" \
    "min_load_alignment: 16 KB dump -> 16384"
assert_eq "4096" "$(printf '%s' "$DUMP_MIXED_ALIGN" | min_load_alignment)" \
    "min_load_alignment: mixed 0x4000/0x1000 dump -> 4096 (the min)"

assert_refuses "min_load_alignment: empty input" -- min_load_alignment_stdin ""
assert_refuses "min_load_alignment: no LOAD lines" -- min_load_alignment_stdin "$DUMP_NO_LOAD"
assert_refuses "min_load_alignment: non-numeric Align" -- min_load_alignment_stdin "$DUMP_NONNUMERIC_ALIGN"

# "Could not look" (nothing to measure) and "an Align did not convert" are
# different failures and must say so differently -- a caller reading only
# the exit code cannot tell "no .so shipped a LOAD segment" from "this
# dump is corrupt", so the two paths are pinned to distinct stderr text.
no_load_says_no_load="no"
case "$(min_load_alignment_stdin "$DUMP_NO_LOAD" 2>&1 1>/dev/null)" in
    *"no LOAD segments"*) no_load_says_no_load="yes" ;;
esac
assert_eq "yes" "$no_load_says_no_load" \
    "min_load_alignment: no-LOAD refusal names the right cause"

nonnumeric_says_nonnumeric="no"
case "$(min_load_alignment_stdin "$DUMP_NONNUMERIC_ALIGN" 2>&1 1>/dev/null)" in
    *"does not convert to a number"*) nonnumeric_says_nonnumeric="yes" ;;
esac
assert_eq "yes" "$nonnumeric_says_nonnumeric" \
    "min_load_alignment: non-numeric-Align refusal names the right cause"

# --- android-verify-alignment.sh (synthetic-AAB integration) --------------
# assert_eq/assert_refuses above pin the pure functions in isolation; they
# cannot reach this script's own file/zip handling, preflight refusals, exit
# codes, or the `>=` threshold comparison it applies. This whole harness
# REFUSES (exit 2), never silently skips, when no local NDK r25c toolchain
# is found -- same doctrine as check.sh's own android_cross_target, which
# refuses rather than tolerating a missing Rust target.
VERIFY_ALIGNMENT="$ROOT/scripts/android-verify-alignment.sh"

locate_ndk_home() {
    if [ -n "${NDK_HOME:-}" ] && [ -d "${NDK_HOME:-}" ]; then
        printf '%s' "$NDK_HOME"
        return
    fi
    local candidate="$HOME/Library/Android/sdk/ndk/25.2.9519653"
    [ -d "$candidate" ] && printf '%s' "$candidate"
}

SYNTH_NDK_HOME="$(locate_ndk_home)"
SYNTH_READELF=""
SYNTH_CLANG=""
SYNTH_STOCK_SO=""
if [ -n "$SYNTH_NDK_HOME" ]; then
    for candidate in "$SYNTH_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-readelf; do
        [ -x "$candidate" ] && SYNTH_READELF="$candidate" && break
    done
    for candidate in "$SYNTH_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/aarch64-linux-android24-clang; do
        [ -x "$candidate" ] && SYNTH_CLANG="$candidate" && break
    done
    SYNTH_STOCK_SO="$(find "$SYNTH_NDK_HOME" -path '*/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so' 2>/dev/null | head -1)"
fi

if [ -z "$SYNTH_NDK_HOME" ] || [ -z "$SYNTH_READELF" ] || [ -z "$SYNTH_CLANG" ] || [ -z "$SYNTH_STOCK_SO" ]; then
    echo "test-shell-units: no local NDK r25c toolchain found (set NDK_HOME) -- refusing rather than reporting partial coverage as a pass" >&2
    exit 2
else
    SYNTH_ROOT="$(mktemp -d)"

    # Packs $2, $4, ... into an AAB-shaped zip at $1, at literal entry names
    # $1, $3, ... -- literal, so a bracket/glob-shaped name (see the forged
    # case below) lands in the archive exactly as written, never expanded.
    build_aab() {
        local aab="$1" work
        shift
        work="$(mktemp -d)"
        while [ "$#" -ge 2 ]; do
            mkdir -p "$work/$(dirname "$1")"
            cp "$2" "$work/$1"
            shift 2
        done
        (cd "$work" && zip -q -r "$aab" .)
        rm -rf "$work"
    }

    # @law: a filesystem path cannot represent two entries sharing one
    # literal name, or a name containing a raw newline, both of which a
    # real .aab's central directory permits.
    build_aab_raw() {
        local aab="$1"
        shift
        python3 -c '
import sys, zipfile
aab = sys.argv[1]
args = sys.argv[2:]
with zipfile.ZipFile(aab, "w") as zf:
    for i in range(0, len(args), 2):
        with open(args[i + 1], "rb") as f:
            zf.writestr(args[i], f.read())
' "$aab" "$@" 2>/dev/null
    }

    va() { env NDK_HOME="$SYNTH_NDK_HOME" "$VERIFY_ALIGNMENT" "$@"; }

    # A real 16 KB-aligned .so, compiled fresh with the exact link flags
    # scripts/android-bundle.sh applies to libmain.so (ALIGN_RUSTC_ARGS).
    "$SYNTH_CLANG" -shared -o "$SYNTH_ROOT/lib16k.so" -x c - \
        -Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384 \
        <<< 'int f(void) { return 0; }' 2>/dev/null

    # 1. a real 4 KB-aligned .so (the NDK's own stock libc++_shared.so) must
    # be refused, and the refusal must name the actual cause.
    AAB_4K="$SYNTH_ROOT/four-k.aab"
    build_aab "$AAB_4K" "base/lib/arm64-v8a/libstock.so" "$SYNTH_STOCK_SO"
    err_4k="$(va "$AAB_4K" 2>&1 1>/dev/null)"; status_4k=$?
    assert_eq "1" "$status_4k" "android-verify-alignment.sh: 4 KB .so exits 1"
    case "$err_4k" in
        *"aligned to 4096 bytes, needs >= 16384"*) msg_4k="yes" ;;
        *) msg_4k="no" ;;
    esac
    assert_eq "yes" "$msg_4k" "android-verify-alignment.sh: 4 KB .so names the cause"

    # 2. an AAB with no base/lib/*/*.so entries at all.
    AAB_NOSO="$SYNTH_ROOT/no-so.aab"
    build_aab "$AAB_NOSO" "META-INF/MANIFEST.MF" "$SYNTH_STOCK_SO"
    err_noso="$(va "$AAB_NOSO" 2>&1 1>/dev/null)"; status_noso=$?
    assert_eq "1" "$status_noso" "android-verify-alignment.sh: no .so entries exits 1"
    case "$err_noso" in
        *"no base/lib/"*".so entries in"*) msg_noso="yes" ;;
        *) msg_noso="no" ;;
    esac
    assert_eq "yes" "$msg_noso" "android-verify-alignment.sh: no .so entries names the cause"

    # 3. a missing AAB path.
    err_missing="$(va "$SYNTH_ROOT/does-not-exist.aab" 2>&1 1>/dev/null)"; status_missing=$?
    assert_eq "2" "$status_missing" "android-verify-alignment.sh: missing AAB exits 2"
    case "$err_missing" in
        *"no AAB at"*) msg_missing="yes" ;;
        *) msg_missing="no" ;;
    esac
    assert_eq "yes" "$msg_missing" "android-verify-alignment.sh: missing AAB names the cause"

    # 4. NDK_HOME unset.
    err_nondk="$(env -u NDK_HOME "$VERIFY_ALIGNMENT" "$AAB_4K" 2>&1 1>/dev/null)"; status_nondk=$?
    assert_eq "2" "$status_nondk" "android-verify-alignment.sh: NDK_HOME unset exits 2"
    case "$err_nondk" in
        *"no llvm-readelf under"*"NDK_HOME=<unset>"*) msg_nondk="yes" ;;
        *) msg_nondk="no" ;;
    esac
    assert_eq "yes" "$msg_nondk" "android-verify-alignment.sh: NDK_HOME unset names the cause"

    # 5. wrong argument count.
    err_usage="$("$VERIFY_ALIGNMENT" 2>&1 1>/dev/null)"; status_usage=$?
    assert_eq "2" "$status_usage" "android-verify-alignment.sh: no args exits 2"
    case "$err_usage" in
        *"usage: scripts/android-verify-alignment.sh <aab-path>"*) msg_usage="yes" ;;
        *) msg_usage="no" ;;
    esac
    assert_eq "yes" "$msg_usage" "android-verify-alignment.sh: no args names the cause"

    # 6. a real 16 KB-aligned .so must PASS. Without this case, the `>=`
    # threshold itself could be mutated away (e.g. to `-ge 0`) and every
    # refusal case above would still "correctly" refuse, for the wrong
    # reason -- this is what actually discriminates that mutant.
    AAB_16K="$SYNTH_ROOT/sixteen-k.aab"
    build_aab "$AAB_16K" "base/lib/arm64-v8a/lib16k.so" "$SYNTH_ROOT/lib16k.so"
    out_16k="$(va "$AAB_16K" 2>/dev/null)"; status_16k=$?
    assert_eq "0" "$status_16k" "android-verify-alignment.sh: 16 KB .so exits 0"
    case "$out_16k" in
        *"16384 bytes"*) msg_16k="yes" ;;
        *) msg_16k="no" ;;
    esac
    assert_eq "yes" "$msg_16k" "android-verify-alignment.sh: 16 KB .so reports the alignment"
    case "$out_16k" in
        *"verified 1 .so entries"*) msg_16k_count="yes" ;;
        *) msg_16k_count="no" ;;
    esac
    assert_eq "yes" "$msg_16k_count" \
        "android-verify-alignment.sh: 16 KB .so reports the entry count (kills the mutant deleting the summary line)"

    # 7. a forged AAB carrying an entry name shaped like a glob character
    # class (base/lib/arm64-v8a/libgoo[d].so, 4 KB, malicious) alongside a
    # benign 16 KB one (libgood.so) must have BOTH entries actually read.
    # `unzip -p "$AAB" "$entry"` treats the entry name as a pattern and
    # silently returns the benign entry's bytes for the bracket-named one
    # instead -- the malicious 4 KB library would never be inspected and
    # the script would report success.
    AAB_FORGED="$SYNTH_ROOT/forged.aab"
    build_aab "$AAB_FORGED" \
        "base/lib/arm64-v8a/libgood.so" "$SYNTH_ROOT/lib16k.so" \
        "base/lib/arm64-v8a/libgoo[d].so" "$SYNTH_STOCK_SO"
    err_forged="$(va "$AAB_FORGED" 2>&1 1>/dev/null)"; status_forged=$?
    assert_eq "1" "$status_forged" "android-verify-alignment.sh: forged glob-named entry exits 1"
    case "$err_forged" in
        *"libgoo[d].so"*"aligned to 4096"*) msg_forged="yes" ;;
        *) msg_forged="no" ;;
    esac
    assert_eq "yes" "$msg_forged" "android-verify-alignment.sh: forged glob-named entry is actually inspected"

    # 8. a REAL duplicate entry name: the malicious 4 KB library first, a
    # benign 16 KB one second, both at the identical literal name.
    # zipfile.NameToInfo is a dict keyed by name -- a name-keyed reader
    # resolves that key to the LAST entry, so it would verify the benign
    # second entry and never inspect the malicious first one, while still
    # reporting success.
    AAB_DUP="$SYNTH_ROOT/dup-name.aab"
    build_aab_raw "$AAB_DUP" \
        "base/lib/arm64-v8a/libmain.so" "$SYNTH_STOCK_SO" \
        "base/lib/arm64-v8a/libmain.so" "$SYNTH_ROOT/lib16k.so"
    err_dup="$(va "$AAB_DUP" 2>&1 1>/dev/null)"; status_dup=$?
    assert_eq "1" "$status_dup" "android-verify-alignment.sh: duplicate-name entry exits 1"
    case "$err_dup" in
        *"libmain.so"*"aligned to 4096"*) msg_dup="yes" ;;
        *) msg_dup="no" ;;
    esac
    assert_eq "yes" "$msg_dup" "android-verify-alignment.sh: duplicate-name entry names the malicious cause"

    # 9. an entry name with an embedded newline: fnmatch's DOTALL semantics
    # let `*` cross it, so the printed name splits into two lines each equal
    # to a second, genuinely-benign entry's plain name. A name-keyed reader
    # resolves both printed lines to the benign entry and never inspects the
    # malicious newline-named one, while still reporting success.
    AAB_NL="$SYNTH_ROOT/newline-name.aab"
    build_aab_raw "$AAB_NL" \
        "$(printf 'base/lib/arm64-v8a/libgood.so\nbase/lib/arm64-v8a/libgood.so')" "$SYNTH_STOCK_SO" \
        "base/lib/arm64-v8a/libgood.so" "$SYNTH_ROOT/lib16k.so"
    err_nl="$(va "$AAB_NL" 2>&1 1>/dev/null)"; status_nl=$?
    assert_eq "1" "$status_nl" "android-verify-alignment.sh: newline-embedded entry name exits 1"
    case "$err_nl" in
        *"aligned to 4096"*) msg_nl="yes" ;;
        *) msg_nl="no" ;;
    esac
    assert_eq "yes" "$msg_nl" "android-verify-alignment.sh: newline-embedded entry name is actually inspected"

    # 10. the malicious 4 KB library under a SECOND ABI directory
    # (armeabi-v7a), alongside a benign 16 KB arm64-v8a entry -- pins that
    # every base/lib/<abi>/ directory is scanned, not only arm64-v8a. Kills
    # a mutant narrowing the glob to "base/lib/arm64-v8a/*.so": every other
    # case above builds arm64-v8a-only AABs and would keep passing under
    # that narrower pattern.
    AAB_MULTIABI="$SYNTH_ROOT/multi-abi.aab"
    build_aab "$AAB_MULTIABI" \
        "base/lib/arm64-v8a/lib16k.so" "$SYNTH_ROOT/lib16k.so" \
        "base/lib/armeabi-v7a/libstock.so" "$SYNTH_STOCK_SO"
    err_multiabi="$(va "$AAB_MULTIABI" 2>&1 1>/dev/null)"; status_multiabi=$?
    assert_eq "1" "$status_multiabi" "android-verify-alignment.sh: second-ABI 4 KB entry exits 1"
    case "$err_multiabi" in
        *"armeabi-v7a/libstock.so"*"aligned to 4096"*) msg_multiabi="yes" ;;
        *) msg_multiabi="no" ;;
    esac
    assert_eq "yes" "$msg_multiabi" "android-verify-alignment.sh: second-ABI 4 KB entry is scanned and named"

    # 11. a file that is not a zip archive at all, passed as the AAB.
    # zipfile.ZipFile raises BadZipFile -- python exits 2 for that, and the
    # script names it "is not a valid zip archive". preflight_fail's OTHER
    # exit-2 message ("could not list entries ... python exited N") is the
    # fallback for an UNEXPECTED status, not for this known one; a fixture
    # is the only thing that discriminates "the BadZipFile branch ran" from
    # "it was deleted and execution fell through to the generic fallback",
    # since both still exit 2.
    AAB_NOTZIP="$SYNTH_ROOT/not-a-zip.aab"
    printf 'this is not a zip file, just garbage bytes\n' > "$AAB_NOTZIP"
    err_notzip="$(va "$AAB_NOTZIP" 2>&1 1>/dev/null)"; status_notzip=$?
    assert_eq "2" "$status_notzip" "android-verify-alignment.sh: non-zip AAB exits 2"
    case "$err_notzip" in
        *"is not a valid zip archive"*) msg_notzip="yes" ;;
        *) msg_notzip="no" ;;
    esac
    assert_eq "yes" "$msg_notzip" \
        "android-verify-alignment.sh: non-zip AAB names the exact cause (kills the BadZipFile-branch-deletion mutant)"

    # 12. a base/lib/*/*.so entry that is not an ELF object at all (a plain
    # text file). This must be refused with "could not be read as an ELF
    # object" -- a DIFFERENT cause than "aligned to N bytes" -- and the
    # stderr must be EXACTLY that one line: llvm-readelf's own diagnostic on
    # the malformed input is piped to /dev/null at the call site, so an
    # exact match (not a substring) is what discriminates "suppressed" from
    # "leaked onto this script's stderr", which is what deleting that
    # redirection would do without changing the exit code at all.
    printf 'not an elf file at all\n' > "$SYNTH_ROOT/notelf.txt"
    AAB_NOTELF="$SYNTH_ROOT/not-elf.aab"
    build_aab "$AAB_NOTELF" "base/lib/arm64-v8a/libnotelf.so" "$SYNTH_ROOT/notelf.txt"
    err_notelf="$(va "$AAB_NOTELF" 2>&1 1>/dev/null)"; status_notelf=$?
    assert_eq "1" "$status_notelf" "android-verify-alignment.sh: non-ELF .so exits 1"
    assert_eq "android-verify-alignment: base/lib/arm64-v8a/libnotelf.so could not be read as an ELF object" \
        "$err_notelf" \
        "android-verify-alignment.sh: non-ELF .so names the exact cause, no readelf diagnostic leaks through"

    # --- android-sign.sh (synthetic-keystore integration) -----------------
    # A missing jarsigner/keytool is a distinct prerequisite from the NDK
    # toolchain checked above, and refuses (exit 2) rather than silently
    # skipping -- same doctrine as the NDK-unreachable guard.
    if ! command -v jarsigner >/dev/null 2>&1 || ! command -v keytool >/dev/null 2>&1; then
        echo "test-shell-units: no local jarsigner/keytool found on PATH -- refusing rather than reporting partial coverage as a pass" >&2
        rm -rf "$SYNTH_ROOT"
        exit 2
    fi

    SIGN="$ROOT/scripts/android-sign.sh"
    SIGN_ROOT="$(mktemp -d)"

    # JKS, not the PKCS12 default: PKCS12 keystores silently ignore a
    # distinct -keypass and always use the store password as the key
    # password, which would make the "wrong key password" case below
    # unreachable. A real Play upload keystore may be either format; JKS is
    # what makes the two passwords genuinely independent for this fixture.
    SIGN_KEYSTORE="$SIGN_ROOT/upload.jks"
    SIGN_ALIAS="upload"
    keytool -genkeypair -storetype JKS -keystore "$SIGN_KEYSTORE" \
        -storepass "rightstorepw" -keypass "rightkeypw" \
        -alias "$SIGN_ALIAS" -dname "CN=Test,OU=Test,O=Test,L=Test,S=Test,C=US" \
        -keyalg RSA -keysize 2048 -validity 3650 >/dev/null 2>&1

    # A 16 KB-aligned unsigned AAB (S1's fixture) and a 4 KB-misaligned one
    # (for the alignment-regression case) -- both signed with the SAME
    # correct keystore, so only the alignment differs between them.
    UNSIGNED_AAB_16K="$SIGN_ROOT/unsigned-16k.aab"
    build_aab "$UNSIGNED_AAB_16K" "base/lib/arm64-v8a/lib16k.so" "$SYNTH_ROOT/lib16k.so"
    UNSIGNED_AAB_4K="$SIGN_ROOT/unsigned-4k.aab"
    build_aab "$UNSIGNED_AAB_4K" "base/lib/arm64-v8a/libstock.so" "$SYNTH_STOCK_SO"

    sign_ok() {
        env NDK_HOME="$SYNTH_NDK_HOME" \
            ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" \
            ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
            ANDROID_SIGN_STORE_PASSWORD="rightstorepw" \
            ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
            "$SIGN" "$@"
    }

    # 1. S1 happy path: correct keystore, correct passwords, 16 KB-aligned
    # .so -- must exit 0, print exactly the signed path on stdout, and the
    # signed file must actually exist.
    SIGNED_16K_EXPECTED="$SIGN_ROOT/unsigned-16k-signed.aab"
    out_sign_ok="$(sign_ok "$UNSIGNED_AAB_16K" 2>/dev/null)"; status_sign_ok=$?
    assert_eq "0" "$status_sign_ok" "android-sign.sh: S1 happy path exits 0"
    assert_eq "$SIGNED_16K_EXPECTED" "$out_sign_ok" \
        "android-sign.sh: S1 happy path prints exactly the signed path on stdout"
    signed_16k_exists="no"
    [ -f "$SIGNED_16K_EXPECTED" ] && signed_16k_exists="yes"
    assert_eq "yes" "$signed_16k_exists" "android-sign.sh: S1 happy path leaves the signed AAB on disk"
    rm -f "$SIGNED_16K_EXPECTED"

    # 2. Alignment regression on the SIGNED bundle (AC 5, mandatory
    # hand-mutant e): jarsigner rewrites the archive but never touches
    # member content, so a 4 KB .so stays 4 KB after signing -- the second
    # call site to android-verify-alignment.sh must catch it and no signed
    # artifact must be left behind.
    SIGNED_4K_EXPECTED="$SIGN_ROOT/unsigned-4k-signed.aab"
    err_sign_misaligned="$(sign_ok "$UNSIGNED_AAB_4K" 2>&1 1>/dev/null)"; status_sign_misaligned=$?
    assert_eq "1" "$status_sign_misaligned" \
        "android-sign.sh: a 4 KB .so fails the post-signing alignment re-check"
    case "$err_sign_misaligned" in
        *"aligned to 4096"*) msg_misaligned="yes" ;;
        *) msg_misaligned="no" ;;
    esac
    assert_eq "yes" "$msg_misaligned" \
        "android-sign.sh: the alignment-regression refusal names the cause"
    signed_4k_exists="no"
    [ -f "$SIGNED_4K_EXPECTED" ] && signed_4k_exists="yes"
    assert_eq "no" "$signed_4k_exists" \
        "android-sign.sh: no signed artifact is left behind after an alignment regression"

    # 3. S2 / AC 3: wrong store password -- a distinct, actionable message,
    # never a stack trace, and nothing signed left behind.
    SIGNED_WRONGSTORE_EXPECTED="$SIGN_ROOT/unsigned-16k-signed.aab"
    err_wrongstore="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="wrongstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_wrongstore=$?
    assert_eq "1" "$status_wrongstore" "android-sign.sh: S2 wrong store password exits 1"
    case "$err_wrongstore" in
        *"wrong store password"*) msg_wrongstore="yes" ;;
        *) msg_wrongstore="no" ;;
    esac
    assert_eq "yes" "$msg_wrongstore" "android-sign.sh: S2 wrong store password names the exact cause"
    case "$err_wrongstore" in
        *"	at "*|*"Exception in thread"*) msg_wrongstore_notrace="no" ;;
        *) msg_wrongstore_notrace="yes" ;;
    esac
    assert_eq "yes" "$msg_wrongstore_notrace" "android-sign.sh: S2 wrong store password never prints a stack trace"
    wrongstore_leftover="no"
    [ -f "$SIGNED_WRONGSTORE_EXPECTED" ] && wrongstore_leftover="yes"
    assert_eq "no" "$wrongstore_leftover" "android-sign.sh: S2 no signed bundle is left behind after a wrong store password"

    # 4. AC 3: wrong key password -- a DIFFERENT message than #3 above
    # (mandatory hand-mutant f: two of the four classes must never collapse
    # into one diagnostic).
    err_wrongkey="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="wrongkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_wrongkey=$?
    assert_eq "1" "$status_wrongkey" "android-sign.sh: wrong key password exits 1"
    case "$err_wrongkey" in
        *"wrong key password"*) msg_wrongkey="yes" ;;
        *) msg_wrongkey="no" ;;
    esac
    assert_eq "yes" "$msg_wrongkey" "android-sign.sh: wrong key password names the exact cause"

    # 5. AC 3: alias not present in the keystore -- a THIRD distinct message.
    err_noalias="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="nosuchalias" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_noalias=$?
    assert_eq "1" "$status_noalias" "android-sign.sh: alias not present exits 1"
    case "$err_noalias" in
        *"nosuchalias"*"not found"*) msg_noalias="yes" ;;
        *) msg_noalias="no" ;;
    esac
    assert_eq "yes" "$msg_noalias" "android-sign.sh: alias-not-present names the exact cause"

    # 6. AC 3: keystore file absent -- our own preflight check, exit 2
    # (matching android-verify-alignment.sh's `[ -f ]` -> preflight_fail
    # precedent), a FOURTH distinct message.
    err_nokeystore="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_ROOT/does-not-exist.jks" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_nokeystore=$?
    assert_eq "2" "$status_nokeystore" "android-sign.sh: keystore file absent exits 2 (preflight)"
    case "$err_nokeystore" in
        *"no keystore at"*) msg_nokeystore="yes" ;;
        *) msg_nokeystore="no" ;;
    esac
    assert_eq "yes" "$msg_nokeystore" "android-sign.sh: keystore-absent names the exact cause"

    # 7. Usage: wrong argument count.
    err_usage_sign="$("$SIGN" 2>&1 1>/dev/null)"; status_usage_sign=$?
    assert_eq "2" "$status_usage_sign" "android-sign.sh: no args exits 2"
    case "$err_usage_sign" in
        *"usage: scripts/android-sign.sh <aab-path>"*) msg_usage_sign="yes" ;;
        *) msg_usage_sign="no" ;;
    esac
    assert_eq "yes" "$msg_usage_sign" "android-sign.sh: no args names the cause"

    # 8. AC 2 / D-3 / mandatory hand-mutant c: a REAL `bash -x` run, with
    # both real password VALUES in the environment, must never print either
    # value anywhere in the combined output -- only the env-var NAMES may
    # appear (as :env arguments), never the values.
    trace_out="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        bash -x "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    store_pw_leaked="no"
    case "$trace_out" in *"rightstorepw"*) store_pw_leaked="yes" ;; esac
    assert_eq "no" "$store_pw_leaked" \
        "android-sign.sh: set -x never prints the store password (AC 2)"
    key_pw_leaked="no"
    case "$trace_out" in *"rightkeypw"*) key_pw_leaked="yes" ;; esac
    assert_eq "no" "$key_pw_leaked" \
        "android-sign.sh: set -x never prints the key password (AC 2)"

    rm -rf "$SIGN_ROOT"

    rm -rf "$SYNTH_ROOT"
fi

# --- .gitignore tripwire (AC 6) --------------------------------------------
# A real keystore lives at $HOME/.kayzen/, never in the repo; *.jks,
# *.keystore and *.p12 are refused by .gitignore as a tripwire against ever
# committing one by accident. `git check-ignore` is the real collaborator --
# no mocking, the actual .gitignore rules are exercised.
git_is_ignored() {
    git -C "$ROOT" check-ignore -q "$1" >/dev/null 2>&1
    printf '%d' "$?"
}
assert_eq "0" "$(git_is_ignored "upload.jks")" "gitignore: *.jks is ignored at repo root"
assert_eq "0" "$(git_is_ignored "upload.keystore")" "gitignore: *.keystore is ignored at repo root"
assert_eq "0" "$(git_is_ignored "upload.p12")" "gitignore: *.p12 is ignored at repo root"
assert_eq "0" "$(git_is_ignored "some/nested/dir/upload.jks")" \
    "gitignore: *.jks is ignored at a nested depth (unrooted pattern)"

# --- refuses, never silently skips, when no local NDK is reachable --------
# Subprocess invocation with NDK_HOME unset and a fabricated HOME, so
# locate_ndk_home's own fallback path can't accidentally resolve.
# TEST_SHELL_UNITS_NO_RECURSE stops it from spawning a third one.
if [ -z "${TEST_SHELL_UNITS_NO_RECURSE:-}" ]; then
    FAKE_HOME="$(mktemp -d)"
    norefusal_out="$(env -u NDK_HOME HOME="$FAKE_HOME" TEST_SHELL_UNITS_NO_RECURSE=1 \
        "${BASH_SOURCE[0]}" 2>&1 1>/dev/null)"
    norefusal_status=$?
    rm -rf "$FAKE_HOME"
    assert_eq "2" "$norefusal_status" \
        "test-shell-units.sh: refuses (does not silently skip) when no local NDK is reachable"
    case "$norefusal_out" in
        *"no local NDK r25c toolchain found"*) norefusal_msg="yes" ;;
        *) norefusal_msg="no" ;;
    esac
    assert_eq "yes" "$norefusal_msg" \
        "test-shell-units.sh: NDK-unreachable refusal names the cause"

    # A PARTIAL toolchain: NDK_HOME resolves to a real directory (so
    # SYNTH_NDK_HOME is non-empty), but nothing under it matches the
    # llvm-readelf/clang/libc++_shared.so globs (so the other three stay
    # empty). The four-way guard is `||` -- ANY of the four being empty
    # refuses -- specifically so this half-present case still refuses. A
    # mutant narrowing that `||` to `&&` (only refuse when ALL four are
    # empty) would let this exact case fall through into the toolchain
    # setup below, which then breaks in whatever way an empty $SYNTH_CLANG
    # or $SYNTH_READELF happens to break, rather than in the one clearly-
    # named way this harness commits to.
    FAKE_NDK_ROOT="$(mktemp -d)"
    FAKE_NDK_HOME="$FAKE_NDK_ROOT/ndk/25.2.9519653"
    mkdir -p "$FAKE_NDK_HOME"
    partial_out="$(env NDK_HOME="$FAKE_NDK_HOME" TEST_SHELL_UNITS_NO_RECURSE=1 \
        "${BASH_SOURCE[0]}" 2>&1 1>/dev/null)"
    partial_status=$?
    rm -rf "$FAKE_NDK_ROOT"
    assert_eq "2" "$partial_status" \
        "test-shell-units.sh: refuses (does not silently skip) with a partial NDK toolchain"
    case "$partial_out" in
        *"no local NDK r25c toolchain found"*) partial_msg="yes" ;;
        *) partial_msg="no" ;;
    esac
    assert_eq "yes" "$partial_msg" \
        "test-shell-units.sh: partial-NDK refusal names the cause (kills the guard's || -> && mutant)"
fi

# --- verdict --------------------------------------------------------------
# A harness with every case deleted still exits 0 with "0 passed, 0 failed"
# unless this is checked explicitly -- that reads as a clean gate while
# measuring nothing.
TOTAL=$((PASS + FAIL))
if [ "$TOTAL" -eq 0 ]; then
    echo "shell-units: 0 assertions ran -- refusing to report a pass for measuring nothing" >&2
    exit 1
fi

printf 'shell-units: %d passed, %d failed\n' "$PASS" "$FAIL"
if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
exit 0
