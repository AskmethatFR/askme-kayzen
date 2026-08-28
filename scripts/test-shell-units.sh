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

# --- patch_version_code (B1) ------------------------------------------------
# The dx-generated fixture always carries the sentinel `versionCode = 1`.
# Cargo.toml's real version is 0.0.1, whose version_code_from_semver output
# is ALSO 1 -- the exact collision that made the OLD "did the old value
# survive the patch" check fire on its own success. patch_version_code must
# tell "the substitution ran and produced 1" apart from "nothing ran and 1
# was merely left over", which a text-only before/after comparison cannot.
PVC_ROOT="$(mktemp -d)"

write_gradle_fixture() {
    printf '%s' "$2" > "$1"
}

write_gradle_fixture "$PVC_ROOT/distinct.kts" 'android {
    defaultConfig {
        versionCode = 1
        versionName = "0.1.0"
    }
}
'
patch_version_code "$PVC_ROOT/distinct.kts" "1000"
assert_eq "0" "$?" "patch_version_code: 0.1.0 -> 1000 returns success"
assert_eq "yes" "$(grep -qE '^[[:space:]]*versionCode = 1000$' "$PVC_ROOT/distinct.kts" && echo yes || echo no)" \
    "patch_version_code: 0.1.0 -> 1000 lands in the file"
assert_eq "no" "$(grep -qE '^[[:space:]]*versionCode = 1$' "$PVC_ROOT/distinct.kts" && echo yes || echo no)" \
    "patch_version_code: 0.1.0 -> 1000 leaves no trace of the old sentinel"

# The critical regression case for B1: target == sentinel == 1 (Cargo.toml
# at 0.0.x). A correct patch must still report success even though the
# file's text is unchanged by the round trip.
write_gradle_fixture "$PVC_ROOT/collision.kts" 'android {
    defaultConfig {
        versionCode = 1
        versionName = "0.0.1"
    }
}
'
patch_version_code "$PVC_ROOT/collision.kts" "1"
assert_eq "0" "$?" \
    "patch_version_code: 0.0.1 -> 1 (sentinel == target) still reports success (B1)"
assert_eq "yes" "$(grep -qE '^[[:space:]]*versionCode = 1$' "$PVC_ROOT/collision.kts" && echo yes || echo no)" \
    "patch_version_code: 0.0.1 -> 1 leaves the correct value in the file"

write_gradle_fixture "$PVC_ROOT/zero-sentinel.kts" 'android {
    defaultConfig {
        versionName = "0.1.0"
    }
}
'
assert_refuses "patch_version_code: zero occurrences of the sentinel refuses" \
    -- patch_version_code "$PVC_ROOT/zero-sentinel.kts" "1000"

write_gradle_fixture "$PVC_ROOT/double-sentinel.kts" 'android {
    defaultConfig {
        versionCode = 1
    }
}
other {
    versionCode = 1
}
'
assert_refuses "patch_version_code: two occurrences of the sentinel refuses" \
    -- patch_version_code "$PVC_ROOT/double-sentinel.kts" "1000"

# R2 (retry 2): the two-phase marker guard `[ "$marked" -ne 1 ]` is the
# entire mechanism the B1 fix relies on to prove the substitution actually
# ran -- QA hand-mutated it and the mutant survived. A stub `sed` that
# silently no-ops (streams its last argument through unchanged) simulates
# exactly the failure this guard exists to catch: the occurrences pre-check
# above uses `grep`, not `sed`, so it still passes while the substitution
# itself never happens.
write_gradle_fixture "$PVC_ROOT/noop-sed.kts" 'android {
    defaultConfig {
        versionCode = 1
        versionName = "0.1.0"
    }
}
'
SED_NOOP_DIR="$(mktemp -d)"
cat > "$SED_NOOP_DIR/sed" <<'EOF'
#!/usr/bin/env bash
shift
cat "$@"
EOF
chmod +x "$SED_NOOP_DIR/sed"

err_r2="$(PATH="$SED_NOOP_DIR:$PATH" patch_version_code "$PVC_ROOT/noop-sed.kts" "1000" 2>&1 1>/dev/null)"
status_r2=$?
assert_eq "1" "$status_r2" \
    "patch_version_code: a no-op marker substitution is refused, not silently accepted (R2)"
case "$err_r2" in
    *"the substitution did not run"*) msg_r2="yes" ;;
    *) msg_r2="no" ;;
esac
assert_eq "yes" "$msg_r2" \
    "patch_version_code: a no-op marker substitution names the cause (R2, kills the marked-guard mutant)"
rm -rf "$SED_NOOP_DIR"

# R4 (retry 2): version_code must be validated as a bare non-negative
# integer BEFORE it ever reaches sed as a substitution replacement. Each
# fixture is FRESH and carries an untouched sentinel, so without
# validation the call would otherwise run to completion (verified: today
# "abc", "12a", "-5", "1.0" and "" all currently return 0 and inject the
# literal garbage string as versionCode). The message assertion, not just
# the exit code, is what discriminates real validation from "1000/evil"
# coincidentally tripping sed's own delimiter-count error.
vcode_case=0
for bad_version_code in "" "abc" "12a" "-5" "1.0" "1000/evil"; do
    vcode_case=$((vcode_case + 1))
    vcode_fixture="$PVC_ROOT/vcode-$vcode_case.kts"
    write_gradle_fixture "$vcode_fixture" 'android {
    defaultConfig {
        versionCode = 1
        versionName = "0.1.0"
    }
}
'
    err_vcode="$(patch_version_code "$vcode_fixture" "$bad_version_code" 2>&1 1>/dev/null)"
    status_vcode=$?
    assert_eq "1" "$status_vcode" \
        "patch_version_code: version_code '$bad_version_code' is refused (R4)"
    case "$err_vcode" in
        *"is not a bare non-negative integer"*) msg_vcode="yes" ;;
        *) msg_vcode="no" ;;
    esac
    assert_eq "yes" "$msg_vcode" \
        "patch_version_code: version_code '$bad_version_code' names the validation cause, not a downstream accident (R4)"
    unchanged_vcode="$(grep -qE '^[[:space:]]*versionCode = 1$' "$vcode_fixture" && echo yes || echo no)"
    assert_eq "yes" "$unchanged_vcode" \
        "patch_version_code: version_code '$bad_version_code' leaves the file untouched (R4)"
done

# R4: phase-2's substitution must be anchored to the `versionCode = ` line
# shape, the same way phase-1 already is. A decoy line that happens to
# already contain the marker's own literal text (planted here to stand in
# for any future collision) is the only input that can tell an anchored
# substitution apart from an unanchored one: unanchored, sed's first-match-
# per-line semantics silently rewrite the decoy too (verified pre-fix: the
# call returned 0 with the decoy corrupted -- exactly the "silent collateral
# substitution" Security flagged). Anchored, the decoy no longer matches the
# substitution pattern, so it survives -- but it still trips the pre-
# existing, out-of-scope "did the marker fully disappear" invariant
# (B1, unrelated), which is the CORRECT fail-safe outcome here: refuse
# rather than either corrupt the decoy or silently ignore it. Never touch
# that invariant; assert the refusal it produces instead.
MARKER_LITERAL="__ANDROID_BUNDLE_VERSION_CODE_MARKER__"
DECOY_LINE="// unrelated: $MARKER_LITERAL must never be rewritten by patch_version_code"
write_gradle_fixture "$PVC_ROOT/anchor.kts" "android {
    defaultConfig {
        versionCode = 1
        versionName = \"0.1.0\"
    }
}
$DECOY_LINE
"
err_anchor="$(patch_version_code "$PVC_ROOT/anchor.kts" "777" 2>&1 1>/dev/null)"
status_anchor=$?
assert_eq "1" "$status_anchor" \
    "patch_version_code: a decoy line matching the marker literal is refused, never silently corrupted (R4, phase-2 anchor)"
case "$err_anchor" in
    *"survived the second substitution"*) msg_anchor="yes" ;;
    *) msg_anchor="no" ;;
esac
assert_eq "yes" "$msg_anchor" \
    "patch_version_code: the refusal is the marker-survival guard, not some other cause (R4)"
assert_eq "yes" "$(grep -qE '^[[:space:]]*versionCode = 1$' "$PVC_ROOT/anchor.kts" && echo yes || echo no)" \
    "patch_version_code: a refused patch leaves the original sentinel completely untouched (R4)"
assert_eq "yes" "$(grep -qF "$DECOY_LINE" "$PVC_ROOT/anchor.kts" && echo yes || echo no)" \
    "patch_version_code: a refused patch leaves the decoy line completely untouched (R4)"

rm -rf "$PVC_ROOT"

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

    AAB_4K="$SYNTH_ROOT/four-k.aab"
    build_aab "$AAB_4K" "base/lib/arm64-v8a/libstock.so" "$SYNTH_STOCK_SO"
    err_4k="$(va "$AAB_4K" 2>&1 1>/dev/null)"; status_4k=$?
    assert_eq "1" "$status_4k" "android-verify-alignment.sh: 4 KB .so exits 1"
    case "$err_4k" in
        *"aligned to 4096 bytes, needs >= 16384"*) msg_4k="yes" ;;
        *) msg_4k="no" ;;
    esac
    assert_eq "yes" "$msg_4k" "android-verify-alignment.sh: 4 KB .so names the cause"

    AAB_NOSO="$SYNTH_ROOT/no-so.aab"
    build_aab "$AAB_NOSO" "META-INF/MANIFEST.MF" "$SYNTH_STOCK_SO"
    err_noso="$(va "$AAB_NOSO" 2>&1 1>/dev/null)"; status_noso=$?
    assert_eq "1" "$status_noso" "android-verify-alignment.sh: no .so entries exits 1"
    case "$err_noso" in
        *"no base/lib/"*".so entries in"*) msg_noso="yes" ;;
        *) msg_noso="no" ;;
    esac
    assert_eq "yes" "$msg_noso" "android-verify-alignment.sh: no .so entries names the cause"

    err_missing="$(va "$SYNTH_ROOT/does-not-exist.aab" 2>&1 1>/dev/null)"; status_missing=$?
    assert_eq "2" "$status_missing" "android-verify-alignment.sh: missing AAB exits 2"
    case "$err_missing" in
        *"no AAB at"*) msg_missing="yes" ;;
        *) msg_missing="no" ;;
    esac
    assert_eq "yes" "$msg_missing" "android-verify-alignment.sh: missing AAB names the cause"

    err_nondk="$(env -u NDK_HOME "$VERIFY_ALIGNMENT" "$AAB_4K" 2>&1 1>/dev/null)"; status_nondk=$?
    assert_eq "2" "$status_nondk" "android-verify-alignment.sh: NDK_HOME unset exits 2"
    case "$err_nondk" in
        *"no llvm-readelf under"*"NDK_HOME=<unset>"*) msg_nondk="yes" ;;
        *) msg_nondk="no" ;;
    esac
    assert_eq "yes" "$msg_nondk" "android-verify-alignment.sh: NDK_HOME unset names the cause"

    err_usage="$("$VERIFY_ALIGNMENT" 2>&1 1>/dev/null)"; status_usage=$?
    assert_eq "2" "$status_usage" "android-verify-alignment.sh: no args exits 2"
    case "$err_usage" in
        *"usage: scripts/android-verify-alignment.sh <aab-path>"*) msg_usage="yes" ;;
        *) msg_usage="no" ;;
    esac
    assert_eq "yes" "$msg_usage" "android-verify-alignment.sh: no args names the cause"

    # A real 16 KB-aligned .so must PASS. Without this case, the `>=`
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

    # A forged AAB carrying an entry name shaped like a glob character
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

    # A REAL duplicate entry name: the malicious 4 KB library first, a
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

    # An entry name with an embedded newline: fnmatch's DOTALL semantics
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

    # The malicious 4 KB library under a SECOND ABI directory
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

    # A file that is not a zip archive at all, passed as the AAB.
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

    # A base/lib/*/*.so entry that is not an ELF object at all (a plain
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
    REAL_JARSIGNER="$(command -v jarsigner)"
    REAL_KEYTOOL="$(command -v keytool)"

    # JKS, not the PKCS12 default: `keytool -genkeypair -storetype PKCS12`
    # silently COERCES the key's own password to the store password
    # (verified: a distinct -keypass at signing time then fails with "not
    # a private key" even when it is the value keytool itself was given at
    # creation) -- so a PKCS12 fixture cannot hold two genuinely
    # independent store/key passwords, which is exactly what keeps the
    # wrong-store-password and wrong-key-password cases below distinct
    # from each other (mandatory hand-mutant f). JKS is chosen for that
    # independence, not because PKCS12 makes either case unreachable --
    # a wrong -keypass fails under PKCS12 too, just never independently of
    # the store password.
    SIGN_KEYSTORE="$SIGN_ROOT/upload.jks"
    SIGN_ALIAS="upload"
    keytool -genkeypair -storetype JKS -keystore "$SIGN_KEYSTORE" \
        -storepass "rightstorepw" -keypass "rightkeypw" \
        -alias "$SIGN_ALIAS" -dname "CN=Test,OU=Test,O=Test,L=Test,S=Test,C=US" \
        -keyalg RSA -keysize 2048 -validity 3650 >/dev/null 2>&1

    # A SECOND keystore, PKCS12 -- the format `keytool -genkeypair` has
    # defaulted to since JDK 9, and the format Security flagged as what a
    # real production upload keystore is more likely to use. Confirmed
    # directly against this JVM: a self-signed PKCS12 entry's certificate
    # fails PKIX chain validation at `jarsigner -verify` time
    # UNCONDITIONALLY -- jarsigner prints the identical "not signed by the
    # specified alias(es)" text whether the alias handed to it is the
    # bundle's REAL signer or a completely different one, because that
    # text comes from the chain-validation warning path, not from an
    # actual signer-identity check. This is the exact production defect
    # (issue #28 follow-up): a real upload keystore's own alias verified
    # clean and jarsigner still reported "not signed by alias 'upload'".
    # The JKS keystore above cannot reproduce it -- confirmed this JVM does
    # not raise the same warning against a JKS entry's self-signed
    # certificate at verify time -- so PKCS12 is the format this fixture
    # must use to prove the fix actually fixes the reported bug, not a
    # coincidentally-clean case.
    SIGN_KEYSTORE_P12="$SIGN_ROOT/upload.p12"
    SIGN_ALIAS_OTHER="otheralias"
    keytool -genkeypair -storetype PKCS12 -keystore "$SIGN_KEYSTORE_P12" \
        -storepass "rightstorepw" -keypass "rightstorepw" \
        -alias "$SIGN_ALIAS" -dname "CN=Test,OU=Test,O=Test,L=Test,S=Test,C=US" \
        -keyalg RSA -keysize 2048 -validity 3650 >/dev/null 2>&1
    keytool -genkeypair -storetype PKCS12 -keystore "$SIGN_KEYSTORE_P12" \
        -storepass "rightstorepw" -keypass "rightstorepw" \
        -alias "$SIGN_ALIAS_OTHER" -dname "CN=Other,OU=Test,O=Test,L=Test,S=Test,C=US" \
        -keyalg RSA -keysize 2048 -validity 3650 >/dev/null 2>&1

    # --- jar_signer_fingerprint / keystore_alias_fingerprint / -------------
    # verify_jar_signature (AC 4, mandatory hand-mutants g+h) ---------------
    VJS_FIXTURE_SRC="$SIGN_ROOT/verify-fixture-unsigned.aab"
    build_aab "$VJS_FIXTURE_SRC" "base/lib/arm64-v8a/lib16k.so" "$SYNTH_ROOT/lib16k.so"
    VJS_FIXTURE_SIGNED="$SIGN_ROOT/verify-fixture-signed.aab"
    jarsigner -keystore "$SIGN_KEYSTORE_P12" -storepass "rightstorepw" -keypass "rightstorepw" \
        -signedjar "$VJS_FIXTURE_SIGNED" "$VJS_FIXTURE_SRC" "$SIGN_ALIAS" >/dev/null 2>&1

    # Fixture sanity: prove this fixture actually reproduces the reported
    # production symptom before trusting anything built on it -- jarsigner's
    # own alias-mismatch text must fire even against the CORRECT alias. If
    # this ever stops reproducing (a JDK upgrade changes the warning), the
    # fixture is no longer proving what it claims to and everything below
    # is testing nothing.
    raw_verify_correct="$(jarsigner -verify -keystore "$SIGN_KEYSTORE_P12" "$VJS_FIXTURE_SIGNED" "$SIGN_ALIAS" 2>&1)"
    fixture_reproduces_bug="no"
    case "$raw_verify_correct" in
        *"not signed by the specified alias"*) fixture_reproduces_bug="yes" ;;
    esac
    assert_eq "yes" "$fixture_reproduces_bug" \
        "test fixture sanity: jarsigner's alias-mismatch text fires even for the CORRECT alias (the production false positive this ticket fixes)"

    export SIGN_STOREPASS_ENV="rightstorepw"

    fp_jar="$(jar_signer_fingerprint "$VJS_FIXTURE_SIGNED")"; status_fp_jar=$?
    assert_eq "0" "$status_fp_jar" \
        "jar_signer_fingerprint: reads a SHA-256 fingerprint from a real signed jar"
    fp_jar_looks_like_fingerprint="no"
    case "$fp_jar" in
        [0-9A-Fa-f][0-9A-Fa-f]:*[0-9A-Fa-f][0-9A-Fa-f]) fp_jar_looks_like_fingerprint="yes" ;;
    esac
    assert_eq "yes" "$fp_jar_looks_like_fingerprint" \
        "jar_signer_fingerprint: the result is colon-separated hex, not raw keytool prose"

    fp_alias_correct="$(keystore_alias_fingerprint "$SIGN_KEYSTORE_P12" "$SIGN_ALIAS" SIGN_STOREPASS_ENV)"; status_fp_alias_correct=$?
    assert_eq "0" "$status_fp_alias_correct" \
        "keystore_alias_fingerprint: reads a SHA-256 fingerprint for an alias that exists"
    assert_eq "$fp_jar" "$fp_alias_correct" \
        "keystore_alias_fingerprint: a jar signed by alias A reports the SAME fingerprint jar_signer_fingerprint reads off that jar"

    fp_alias_other="$(keystore_alias_fingerprint "$SIGN_KEYSTORE_P12" "$SIGN_ALIAS_OTHER" SIGN_STOREPASS_ENV)"; status_fp_alias_other=$?
    assert_eq "0" "$status_fp_alias_other" \
        "keystore_alias_fingerprint: reads a SHA-256 fingerprint for the second alias"
    fp_aliases_differ="no"
    [ "$fp_jar" != "$fp_alias_other" ] && fp_aliases_differ="yes"
    assert_eq "yes" "$fp_aliases_differ" \
        "keystore_alias_fingerprint: alias A and alias B have distinct certificate fingerprints (fixture sanity)"

    err_fp_noalias="$(keystore_alias_fingerprint "$SIGN_KEYSTORE_P12" "nosuchalias" SIGN_STOREPASS_ENV 2>&1 1>/dev/null)"
    status_fp_noalias=$?
    assert_eq "1" "$status_fp_noalias" \
        "keystore_alias_fingerprint: a nonexistent alias fails"
    msg_fp_noalias="no"
    case "$err_fp_noalias" in *"nosuchalias"*) msg_fp_noalias="yes" ;; esac
    assert_eq "yes" "$msg_fp_noalias" \
        "keystore_alias_fingerprint: a nonexistent alias names the cause"

    err_fp_unsigned="$(jar_signer_fingerprint "$VJS_FIXTURE_SRC" 2>&1 1>/dev/null)"
    status_fp_unsigned=$?
    assert_eq "1" "$status_fp_unsigned" \
        "jar_signer_fingerprint: an UNSIGNED jar (no signer certificate at all) fails rather than returning an empty fingerprint"
    msg_fp_unsigned="no"
    case "$err_fp_unsigned" in *"is signed by 0 signers, expected exactly 1"*) msg_fp_unsigned="yes" ;; esac
    assert_eq "yes" "$msg_fp_unsigned" \
        "jar_signer_fingerprint: an unsigned jar is refused by the signer-count guard (F1), naming the zero count"

    # THE regression test (AC 4): a jar signed by alias A, verified against
    # alias A's OWN fingerprint, must report a clean match -- even though
    # the fixture sanity check above proves jarsigner's own text says
    # otherwise for this exact jar. This is the false positive the owner
    # hit on the first real signing run, reproduced from scratch.
    verify_jar_signature "$SIGN_KEYSTORE_P12" "$VJS_FIXTURE_SIGNED" "$fp_alias_correct" >/dev/null
    status_vjs_match=$?
    assert_eq "0" "$status_vjs_match" \
        "verify_jar_signature: a jar signed by alias A verifies clean against alias A's fingerprint (kills the production false positive)"

    out_vjs_mismatch="$(verify_jar_signature "$SIGN_KEYSTORE_P12" "$VJS_FIXTURE_SIGNED" "$fp_alias_other")"
    status_vjs_mismatch=$?
    assert_eq "3" "$status_vjs_mismatch" \
        "verify_jar_signature: a jar signed by alias A reports mismatch against alias B's fingerprint (kills mutants g+h)"
    msg_vjs_mismatch="no"
    case "$out_vjs_mismatch" in
        *"does not match"*) msg_vjs_mismatch="yes" ;;
    esac
    assert_eq "yes" "$msg_vjs_mismatch" \
        "verify_jar_signature: the fingerprint-mismatch classification names both fingerprints"

    # B1 (retry 1): a forged, self-signed certificate whose Owner: DN
    # CONTAINS the victim alias's own fingerprint text (with the interior
    # space that lands a naive `awk '/SHA256:/ {print $NF; exit}'` on the
    # Owner: line instead of the real, later Certificate fingerprints:
    # block) must never be read as that victim's fingerprint. Real
    # self-signed cert, real jarsigner signature, not a synthetic string.
    B1_ROOT="$(mktemp -d)"
    openssl req -x509 -newkey rsa:2048 -keyout "$B1_ROOT/evil.key" -out "$B1_ROOT/evil.cert" \
        -days 365 -nodes -subj "/emailAddress=x $fp_alias_correct/CN=SHA256:" >/dev/null 2>&1
    openssl pkcs12 -export -in "$B1_ROOT/evil.cert" -inkey "$B1_ROOT/evil.key" -name evil \
        -out "$B1_ROOT/evil.p12" -passout pass:evilpw >/dev/null 2>&1
    B1_JAR="$B1_ROOT/evil-signed.aab"
    jarsigner -keystore "$B1_ROOT/evil.p12" -storepass evilpw -keypass evilpw \
        -signedjar "$B1_JAR" "$VJS_FIXTURE_SRC" evil >/dev/null 2>&1

    b1_naive_scan="$(keytool -J-Duser.language=en -J-Duser.country=US -printcert -jarfile "$B1_JAR" 2>&1 \
        | awk '/SHA256:/ { print $NF; exit }')"
    b1_fixture_reproduces_bug="no"
    [ "$b1_naive_scan" = "$fp_alias_correct" ] && b1_fixture_reproduces_bug="yes"
    assert_eq "yes" "$b1_fixture_reproduces_bug" \
        "test fixture sanity: B1 -- the forged Owner: DN fools a naive substring SHA256 scan (the production authentication bypass this ticket fixes)"

    b1_fp="$(jar_signer_fingerprint "$B1_JAR")"; b1_fp_status=$?
    assert_eq "0" "$b1_fp_status" \
        "jar_signer_fingerprint: B1 -- reads the forged jar's own signer fingerprint without erroring"
    b1_matches_forged_owner="no"
    [ "$b1_fp" = "$fp_alias_correct" ] && b1_matches_forged_owner="yes"
    assert_eq "no" "$b1_matches_forged_owner" \
        "jar_signer_fingerprint: B1 -- a forged Owner: DN containing the victim's fingerprint text is not read as the signer fingerprint"

    verify_jar_signature "$B1_ROOT/evil.p12" "$B1_JAR" "$fp_alias_correct" >/dev/null
    b1_verify_status=$?
    assert_eq "3" "$b1_verify_status" \
        "verify_jar_signature: B1 -- a bundle signed ONLY by an attacker's key is rejected against the victim alias's fingerprint (kills the authentication bypass)"
    rm -rf "$B1_ROOT"

    # B3 (retry 1, Dev-B's ruled-killed-required mutation survivor): a
    # certificate CHAIN of length 2 (leaf signed by a CA) is the only
    # fixture shape that discriminates the awk `exit` above -- every other
    # fixture in this file is self-signed (chain length 1), where the
    # first and only SHA256 line IS the answer regardless of `exit`.
    # Independent oracle: the leaf's own fingerprint, read by `openssl
    # x509 -fingerprint`, off the SAME exported certificate bytes.
    B3_ROOT="$(mktemp -d)"
    B3_KEYSTORE="$B3_ROOT/chain.p12"
    keytool -genkeypair -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -keypass chainpw \
        -alias ca -dname "CN=TestCA,OU=Test,O=Test,L=Test,S=Test,C=US" -keyalg RSA -keysize 2048 \
        -validity 3650 -ext bc:c=ca:true >/dev/null 2>&1
    keytool -genkeypair -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -keypass chainpw \
        -alias leaf -dname "CN=Test,OU=Test,O=Test,L=Test,S=Test,C=US" -keyalg RSA -keysize 2048 \
        -validity 3650 >/dev/null 2>&1
    keytool -certreq -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -alias leaf \
        -file "$B3_ROOT/leaf.csr" >/dev/null 2>&1
    keytool -gencert -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -alias ca \
        -infile "$B3_ROOT/leaf.csr" -outfile "$B3_ROOT/leaf.cert" -validity 3650 >/dev/null 2>&1
    keytool -exportcert -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -alias ca \
        -rfc -file "$B3_ROOT/ca.cert" >/dev/null 2>&1
    keytool -importcert -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -alias ca \
        -file "$B3_ROOT/ca.cert" -noprompt >/dev/null 2>&1
    cat "$B3_ROOT/leaf.cert" "$B3_ROOT/ca.cert" > "$B3_ROOT/fullchain.cert"
    keytool -importcert -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -alias leaf \
        -file "$B3_ROOT/fullchain.cert" -noprompt >/dev/null 2>&1
    keytool -exportcert -storetype PKCS12 -keystore "$B3_KEYSTORE" -storepass chainpw -alias leaf \
        -rfc -file "$B3_ROOT/leaf-exported.cert" >/dev/null 2>&1
    b3_expected_fp="$(openssl x509 -in "$B3_ROOT/leaf-exported.cert" -noout -fingerprint -sha256 | sed 's/^.*=//')"

    export B3_STOREPASS_ENV="chainpw"
    b3_fp="$(keystore_alias_fingerprint "$B3_KEYSTORE" leaf B3_STOREPASS_ENV)"; b3_status=$?
    assert_eq "0" "$b3_status" \
        "keystore_alias_fingerprint: B3 -- a chain-length-2 alias still reads a single fingerprint"
    b3_single_line="yes"
    case "$b3_fp" in *$'\n'*) b3_single_line="no" ;; esac
    assert_eq "yes" "$b3_single_line" \
        "keystore_alias_fingerprint: B3 -- the result is a single line, not both chain entries concatenated (kills the awk-exit mutant)"
    assert_eq "$b3_expected_fp" "$b3_fp" \
        "keystore_alias_fingerprint: B3 -- the result is the LEAF certificate's own fingerprint (openssl-derived oracle), not the CA's"
    rm -rf "$B3_ROOT"

    # B2 (retry 1): a jar carrying signatures from TWO different aliases
    # must never verify -- jar_signer_fingerprint's job is to answer "is
    # this artifact signed by our key, and only our key", and jarsigner
    # itself happily reports "jar verified" on a jar signed by alias A
    # AND alias B. Double-signed by pre-signing outside the pipeline
    # (android-sign.sh itself only ever produces a single fresh
    # signature), same as Dev-B's own reproduction.
    B2_ROOT="$(mktemp -d)"
    B2_STEP1="$B2_ROOT/step1.aab"
    cp "$VJS_FIXTURE_SRC" "$B2_STEP1"
    jarsigner -keystore "$SIGN_KEYSTORE_P12" -storepass rightstorepw -keypass rightstorepw \
        -signedjar "$B2_STEP1" "$B2_STEP1" "$SIGN_ALIAS_OTHER" >/dev/null 2>&1
    B2_MULTI="$B2_ROOT/multi-signed.aab"
    jarsigner -keystore "$SIGN_KEYSTORE_P12" -storepass rightstorepw -keypass rightstorepw \
        -signedjar "$B2_MULTI" "$B2_STEP1" "$SIGN_ALIAS" >/dev/null 2>&1

    b2_signer_count="$(keytool -J-Duser.language=en -J-Duser.country=US -printcert -jarfile "$B2_MULTI" 2>&1 \
        | grep -cE '^Signer #[0-9]+:' || true)"
    assert_eq "2" "$b2_signer_count" \
        "test fixture sanity: B2 -- the double-signed bundle carries two Signer # blocks"

    b2_raw_verify="$(jarsigner -J-Duser.language=en -J-Duser.country=US -verify -keystore "$SIGN_KEYSTORE_P12" "$B2_MULTI" 2>&1)"
    b2_raw_verify_status=$?
    b2_fixture_verifies="no"
    if [ "$b2_raw_verify_status" -eq 0 ]; then
        case "$b2_raw_verify" in *"jar verified"*) b2_fixture_verifies="yes" ;; esac
    fi
    assert_eq "yes" "$b2_fixture_verifies" \
        "test fixture sanity: B2 -- jarsigner itself verifies a multi-signer bundle clean (the production hole this ticket closes)"

    err_b2_fp="$(jar_signer_fingerprint "$B2_MULTI" 2>&1 1>/dev/null)"; status_b2_fp=$?
    assert_eq "1" "$status_b2_fp" \
        "jar_signer_fingerprint: B2 -- a bundle carrying more than one signer is refused"
    msg_b2_fp="no"
    case "$err_b2_fp" in *"2 signers"*) msg_b2_fp="yes" ;; esac
    assert_eq "yes" "$msg_b2_fp" \
        "jar_signer_fingerprint: B2 -- the multi-signer refusal names the signer count"

    verify_jar_signature "$SIGN_KEYSTORE_P12" "$B2_MULTI" "$fp_alias_correct" >/dev/null
    status_b2_verify=$?
    assert_eq "4" "$status_b2_verify" \
        "verify_jar_signature: B2 -- a multi-signer bundle is rejected even though one of its signers matches the expected alias (kills the multi-signer bypass)"
    rm -rf "$B2_ROOT"

    # F1 (security-barrier finding): the multi-signer guard is fail-open --
    # `signer_count -gt 1` accepts a count of 0. This is the ONLY thing
    # that makes the DN-forgery bypass (B1 above) unreachable in
    # production: android-sign.sh always signs with our own key, so a
    # pre-signed attacker bundle carries a SECOND signer and must be
    # refused right here. A forged DN (B1) can only ever RAISE the count
    # -- it fails closed already. The zero count is NOT an exotic case:
    # an ordinary unsigned jar under the real keytool already produces it
    # (:820-825), no attacker and no crafted archive involved. The shim
    # below covers the second route to the same count -- a keytool build
    # whose "Signer #" wording differs and prints no such line at all.
    F1_ROOT="$(mktemp -d)"
    cat > "$F1_ROOT/keytool" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "Certificate #1:"
printf '%s\n' "Certificate fingerprints:"
printf '\t SHA256: %s\n' "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99"
SHIM
    chmod +x "$F1_ROOT/keytool"

    err_f1="$(PATH="$F1_ROOT:$PATH" jar_signer_fingerprint "$VJS_FIXTURE_SIGNED" 2>&1 1>/dev/null)"
    status_f1=$?
    assert_eq "1" "$status_f1" \
        "jar_signer_fingerprint: F1 -- a keytool build printing no 'Signer #' line at all is refused, not read as a single signer"
    msg_f1="no"
    case "$err_f1" in *"expected exactly 1"*) msg_f1="yes" ;; esac
    assert_eq "yes" "$msg_f1" \
        "jar_signer_fingerprint: F1 -- the zero-signer refusal names the expectation"
    rm -rf "$F1_ROOT"

    # Belt-and-suspenders (retry 1): two empty strings must never compare
    # equal inside verify_jar_signature's own fingerprint check -- an
    # explicit invariant, independent of the fact that
    # _sha256_fingerprint_from_keytool_output already refuses to hand back
    # an empty value. jar_signer_fingerprint is overridden in a subshell to
    # simulate a caller-contract violation (empty stdout, exit 0) that no
    # real implementation of it can currently produce, so the guard inside
    # verify_jar_signature itself is what this test actually exercises.
    empty_guard_status="$(
        # shellcheck disable=SC2329 # called indirectly, by verify_jar_signature below
        jar_signer_fingerprint() { printf ''; return 0; }
        verify_jar_signature "$SIGN_KEYSTORE_P12" "$VJS_FIXTURE_SIGNED" "" >/dev/null
        echo $?
    )"
    empty_guard_reads_verified="no"
    [ "$empty_guard_status" = "0" ] && empty_guard_reads_verified="yes"
    assert_eq "no" "$empty_guard_reads_verified" \
        "verify_jar_signature: an empty actual fingerprint against an empty expected fingerprint never reads as verified"

    # Locale forcing (hand-mutant b): a shim keytool that only succeeds
    # when invoked with the English-forcing -J flags -- portable across any
    # host locale, unlike relying on this development machine's own
    # (French) AppleLocale, which is what actually crashed in production
    # (`erreur keytool : java.util.MissingFormatArgumentException: Format
    # specifier '%2$s'`, confirmed reproducible on this exact JDK, and
    # confirmed NOT fixed by LC_ALL=C -- a macOS JVM reads its locale from
    # native CFLocale APIs, never from shell environment variables).
    LOCALE_SHIM_ROOT="$(mktemp -d)"
    cat > "$LOCALE_SHIM_ROOT/keytool" <<'SHIM'
#!/usr/bin/env bash
has_lang="no"
has_country="no"
for a in "$@"; do
    case "$a" in
        -J-Duser.language=en) has_lang="yes" ;;
        -J-Duser.country=US) has_country="yes" ;;
    esac
done
if [ "$has_lang" != "yes" ] || [ "$has_country" != "yes" ]; then
    printf '%s\n' "keytool-locale-shim: refusing without forced English locale flags" >&2
    exit 1
fi
printf '%s\n' "Signer #1:"
printf '%s\n' "Certificate #1:"
printf '%s\n' "Certificate fingerprints:"
printf '\t SHA256: %s\n' "DE:AD:BE:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB"
SHIM
    chmod +x "$LOCALE_SHIM_ROOT/keytool"

    out_locale_jar="$(PATH="$LOCALE_SHIM_ROOT:$PATH" jar_signer_fingerprint "$VJS_FIXTURE_SIGNED")"
    status_locale_jar=$?
    assert_eq "0" "$status_locale_jar" \
        "jar_signer_fingerprint: forces English locale on keytool -printcert (kills hand-mutant b)"
    assert_eq "DE:AD:BE:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB" "$out_locale_jar" \
        "jar_signer_fingerprint: the shim's fingerprint reaches the caller once locale forcing lets it run"

    out_locale_alias="$(PATH="$LOCALE_SHIM_ROOT:$PATH" keystore_alias_fingerprint "$SIGN_KEYSTORE_P12" "$SIGN_ALIAS" SIGN_STOREPASS_ENV)"
    status_locale_alias=$?
    assert_eq "0" "$status_locale_alias" \
        "keystore_alias_fingerprint: forces English locale on keytool -list (kills hand-mutant b)"
    assert_eq "DE:AD:BE:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB" "$out_locale_alias" \
        "keystore_alias_fingerprint: the shim's fingerprint reaches the caller once locale forcing lets it run"
    rm -rf "$LOCALE_SHIM_ROOT"

    # Shape validation (retry 1): a value that does not look like a real
    # SHA256 fingerprint (32 uppercase hex pairs, colon-separated) must be
    # an ERROR, never a comparison input -- an anchored field read is not
    # by itself proof the field's own content is well-formed. A shim
    # keytool prints a truncated fingerprint (16 pairs, still under the
    # anchored "Certificate fingerprints:" header) to exercise this
    # independently of B1's anchor fix.
    SHAPE_SHIM_ROOT="$(mktemp -d)"
    cat > "$SHAPE_SHIM_ROOT/keytool" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "Signer #1:"
printf '%s\n' "Certificate #1:"
printf '%s\n' "Certificate fingerprints:"
printf '\t SHA256: %s\n' "AB:CD:EF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC"
SHIM
    chmod +x "$SHAPE_SHIM_ROOT/keytool"

    err_shape="$(PATH="$SHAPE_SHIM_ROOT:$PATH" jar_signer_fingerprint "$VJS_FIXTURE_SIGNED" 2>&1 1>/dev/null)"
    status_shape=$?
    assert_eq "1" "$status_shape" \
        "jar_signer_fingerprint: a truncated (non-32-pair) SHA256 value under the fingerprints header is refused, not returned"
    msg_shape="no"
    case "$err_shape" in *"no SHA256 fingerprint found"*) msg_shape="yes" ;; esac
    assert_eq "yes" "$msg_shape" \
        "jar_signer_fingerprint: the shape-rejection names the cause"
    rm -rf "$SHAPE_SHIM_ROOT"

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

    SIGNED_16K_EXPECTED="$SIGN_ROOT/unsigned-16k-signed.aab"
    out_sign_ok="$(sign_ok "$UNSIGNED_AAB_16K" 2>/dev/null)"; status_sign_ok=$?
    assert_eq "0" "$status_sign_ok" "android-sign.sh: S1 happy path exits 0"
    assert_eq "$SIGNED_16K_EXPECTED" "$out_sign_ok" \
        "android-sign.sh: S1 happy path prints exactly the signed path on stdout"
    signed_16k_exists="no"
    [ -f "$SIGNED_16K_EXPECTED" ] && signed_16k_exists="yes"
    assert_eq "yes" "$signed_16k_exists" "android-sign.sh: S1 happy path leaves the signed AAB on disk"
    rm -f "$SIGNED_16K_EXPECTED"

    # Alignment regression on the SIGNED bundle (AC 5, mandatory
    # hand-mutant e).
    # @law: jarsigner rewrites the archive's central directory but never
    # touches a member's own content, so a 4 KB .so stays 4 KB after
    # signing -- the second call site to android-verify-alignment.sh must
    # catch it and no signed artifact must be left behind.
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

    # S2 / AC 3: wrong store password -- a distinct, actionable message,
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

    # AC 3: wrong key password -- a DIFFERENT message than the wrong-store-
    # password case above (mandatory hand-mutant f: two of the four classes
    # must never collapse into one diagnostic).
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

    # AC 3: alias not present in the keystore -- a THIRD distinct message.
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

    # F2 (security-barrier finding): the SIGNING jarsigner call's own -J
    # locale-forcing flags were untested -- the classification right below
    # it (S2/AC3 tests above) matches ENGLISH substrings only
    # ("password was incorrect", "not a private key", "Certificate chain
    # not found for"), so under a non-English JVM (the owner's own
    # machine is French) every one of them falls through to the generic
    # catch-all and every specific diagnostic is lost. A shim jarsigner
    # emits the classifiable English text ONLY when both -J flags are
    # present on argv, and a French one otherwise -- this is what makes
    # the assertion below independent of THIS test machine's own locale.
    F2_ROOT="$(mktemp -d)"
    cat > "$F2_ROOT/jarsigner" <<EOF
#!/usr/bin/env bash
is_sign="no"
has_lang="no"
has_country="no"
for a in "\$@"; do
    case "\$a" in
        -signedjar) is_sign="yes" ;;
        -J-Duser.language=en) has_lang="yes" ;;
        -J-Duser.country=US) has_country="yes" ;;
    esac
done
if [ "\$is_sign" = "yes" ]; then
    if [ "\$has_lang" = "yes" ] && [ "\$has_country" = "yes" ]; then
        printf '%s\n' "jarsigner: password was incorrect for keystore"
    else
        printf '%s\n' "jarsigner : le mot de passe du fichier de cles est incorrect"
    fi
    exit 1
fi
exec "$REAL_JARSIGNER" "\$@"
EOF
    chmod +x "$F2_ROOT/jarsigner"

    err_f2="$(PATH="$F2_ROOT:$PATH" env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_f2=$?
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    assert_eq "1" "$status_f2" \
        "android-sign.sh: F2 -- a French-diagnostic signing failure still exits 1"
    msg_f2="no"
    case "$err_f2" in *"wrong store password"*) msg_f2="yes" ;; esac
    assert_eq "yes" "$msg_f2" \
        "android-sign.sh: F2 -- the signing call's locale flags let the classification match, instead of falling to the generic catch-all"
    rm -rf "$F2_ROOT"

    # AC 3: keystore file absent -- our own preflight check, exit 2
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

    err_usage_sign="$("$SIGN" 2>&1 1>/dev/null)"; status_usage_sign=$?
    assert_eq "2" "$status_usage_sign" "android-sign.sh: no args exits 2"
    case "$err_usage_sign" in
        *"usage: scripts/android-sign.sh <aab-path>"*) msg_usage_sign="yes" ;;
        *) msg_usage_sign="no" ;;
    esac
    assert_eq "yes" "$msg_usage_sign" "android-sign.sh: no args names the cause"

    # AC 2 / D-3 / mandatory hand-mutant c: a REAL `bash -x` run, with
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

    # B3 / mandatory hand-mutant k: an AAB path starting with '-' must be
    # refused before it ever reaches jarsigner's argv, where it could be
    # read as an option instead of a filename.
    err_dashaab="$(sign_ok "-Jsomething.aab" 2>&1 1>/dev/null)"; status_dashaab=$?
    assert_eq "2" "$status_dashaab" "android-sign.sh: AAB path starting with '-' is refused (B3)"
    case "$err_dashaab" in
        *"AAB path must not start with"*) msg_dashaab="yes" ;;
        *) msg_dashaab="no" ;;
    esac
    assert_eq "yes" "$msg_dashaab" "android-sign.sh: leading-dash AAB path names the cause"

    # B3 / mandatory hand-mutant k: an alias starting with '-' must be
    # refused too -- jarsigner has no `--` end-of-options marker, so a
    # value like `-J-javaagent:...` reaches the JVM that holds both
    # passwords.
    err_dashalias="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="-J-javaagent:/tmp/evil.jar" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_dashalias=$?
    assert_eq "2" "$status_dashalias" "android-sign.sh: alias starting with '-' is refused (B3)"
    case "$err_dashalias" in
        *"ANDROID_SIGN_KEY_ALIAS must not start with"*) msg_dashalias="yes" ;;
        *) msg_dashalias="no" ;;
    esac
    assert_eq "yes" "$msg_dashalias" "android-sign.sh: leading-dash alias names the cause"

    # B4 (retry 1) / mandatory hand-mutant d: a keystore path starting
    # with '-' must be refused too, for the same reason the AAB path and
    # the alias already are -- ANDROID_SIGN_KEYSTORE had no such guard,
    # and unlike the AAB path it never reaches a `[ -f ]` check before
    # landing on jarsigner's/keytool's own argv.
    err_dashkeystore="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="-J-javaagent:/tmp/evil.jar" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_dashkeystore=$?
    assert_eq "2" "$status_dashkeystore" "android-sign.sh: keystore path starting with '-' is refused (B4)"
    case "$err_dashkeystore" in
        *"ANDROID_SIGN_KEYSTORE must not start with"*) msg_dashkeystore="yes" ;;
        *) msg_dashkeystore="no" ;;
    esac
    assert_eq "yes" "$msg_dashkeystore" "android-sign.sh: leading-dash keystore path names the cause"

    # B6 / mandatory hand-mutant l: an UNSET store password must be
    # caught by its own preflight (exit 2), not fall into jarsigner's
    # catch-all (which happens at exit 1, with a usage banner as noise --
    # empirically confirmed, D8 corrected).
    err_nostorepw="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_nostorepw=$?
    assert_eq "2" "$status_nostorepw" "android-sign.sh: ANDROID_SIGN_STORE_PASSWORD unset exits 2 (preflight, B6)"
    case "$err_nostorepw" in
        *"ANDROID_SIGN_STORE_PASSWORD is not set"*) msg_nostorepw="yes" ;;
        *) msg_nostorepw="no" ;;
    esac
    assert_eq "yes" "$msg_nostorepw" "android-sign.sh: unset store password preflight names the cause"

    # B6 / mandatory hand-mutant l: same for the key password.
    err_nokeypw="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_nokeypw=$?
    assert_eq "2" "$status_nokeypw" "android-sign.sh: ANDROID_SIGN_KEY_PASSWORD unset exits 2 (preflight, B6)"
    case "$err_nokeypw" in
        *"ANDROID_SIGN_KEY_PASSWORD is not set"*) msg_nokeypw="yes" ;;
        *) msg_nokeypw="no" ;;
    esac
    assert_eq "yes" "$msg_nokeypw" "android-sign.sh: unset key password preflight names the cause"

    # B10: a failed run must not leave a STALE previous signed AAB
    # behind at the expected output path -- a caller that globs for
    # *-signed.aab after a failure must never pick up an old build.
    STALE_SIGNED="$SIGN_ROOT/unsigned-16k-signed.aab"
    printf 'stale bytes from a previous run' > "$STALE_SIGNED"
    err_stale="$(env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="wrongstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"; status_stale=$?
    assert_eq "1" "$status_stale" "android-sign.sh: B10 fixture -- a failing run still exits 1"
    case "$err_stale" in
        *"wrong store password"*) msg_stale="yes" ;;
        *) msg_stale="no" ;;
    esac
    assert_eq "yes" "$msg_stale" "android-sign.sh: B10 fixture fails for the expected reason"
    stale_survived="no"
    [ -f "$STALE_SIGNED" ] && stale_survived="yes"
    assert_eq "no" "$stale_survived" \
        "android-sign.sh: a failed run removes a stale previous signed AAB (B10)"

    # B11 (same-shape sweep): the intermediate temp file must be
    # created as a SIBLING of the AAB (same directory), never a bare
    # `mktemp` landing in $TMPDIR -- a spy `mktemp` on PATH records the
    # template every real mktemp call was given.
    MKTEMP_SPY_DIR="$(mktemp -d)"
    REAL_MKTEMP="$(command -v mktemp)"
    MKTEMP_SPY_LOG="$(mktemp)"
    cat > "$MKTEMP_SPY_DIR/mktemp" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$@" >> "$MKTEMP_SPY_LOG"
exec "$REAL_MKTEMP" "\$@"
EOF
    chmod +x "$MKTEMP_SPY_DIR/mktemp"
    PATH="$MKTEMP_SPY_DIR:$PATH" sign_ok "$UNSIGNED_AAB_16K" >/dev/null 2>&1
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    sibling_template="yes"
    while IFS= read -r logged_arg; do
        case "$logged_arg" in
            "$SIGN_ROOT"/.*) : ;;
            *) sibling_template="no" ;;
        esac
    done < "$MKTEMP_SPY_LOG"
    [ -s "$MKTEMP_SPY_LOG" ] || sibling_template="no"
    assert_eq "yes" "$sibling_template" \
        "android-sign.sh: the signing temp file is created as a sibling of the AAB, not in \$TMPDIR (B11)"
    rm -rf "$MKTEMP_SPY_DIR" "$MKTEMP_SPY_LOG"

    # B2 / mandatory hand-mutant j: both passwords must be scrubbed
    # from the environment of every descendant process -- argv silence
    # (already proven above) is not enough, since environ outlives argv. A
    # stub llvm-readelf dumps its own environment; the alignment re-check
    # is what invokes it.
    ENV_SPY_NDK="$(mktemp -d)"
    ENV_SPY_READELF_DIR="$ENV_SPY_NDK/toolchains/llvm/prebuilt/spy-host/bin"
    mkdir -p "$ENV_SPY_READELF_DIR"
    ENV_SPY_CAPTURE="$(mktemp)"
    cat > "$ENV_SPY_READELF_DIR/llvm-readelf" <<EOF
#!/usr/bin/env bash
env > "$ENV_SPY_CAPTURE"
cat >/dev/null
cat <<'DUMP'
Program Headers:
  Type           Offset   VirtAddr           PhysAddr           FileSiz  MemSiz   Flg Align
  LOAD           0x000000 0x0000000000000000 0x0000000000000000 0x001000 0x001000 R   0x4000
  LOAD           0x001000 0x0000000000001000 0x0000000000001000 0x001000 0x001000 R E 0x4000
DUMP
EOF
    chmod +x "$ENV_SPY_READELF_DIR/llvm-readelf"

    env NDK_HOME="$ENV_SPY_NDK" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" >/dev/null 2>&1
    status_envspy=$?
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"

    assert_eq "0" "$status_envspy" \
        "android-sign.sh: B2 fixture -- the run reaching the stubbed alignment check still succeeds"
    store_pw_in_child_env="no"
    grep -q '^ANDROID_SIGN_STORE_PASSWORD=' "$ENV_SPY_CAPTURE" && store_pw_in_child_env="yes"
    assert_eq "no" "$store_pw_in_child_env" \
        "android-sign.sh: the alignment re-check's child process never sees the store password (B2)"
    key_pw_in_child_env="no"
    grep -q '^ANDROID_SIGN_KEY_PASSWORD=' "$ENV_SPY_CAPTURE" && key_pw_in_child_env="yes"
    assert_eq "no" "$key_pw_in_child_env" \
        "android-sign.sh: the alignment re-check's child process never sees the key password (B2)"
    alias_in_child_env="no"
    grep -q "^ANDROID_SIGN_KEY_ALIAS=$SIGN_ALIAS\$" "$ENV_SPY_CAPTURE" && alias_in_child_env="yes"
    assert_eq "yes" "$alias_in_child_env" \
        "android-sign.sh: the scrub is narrow -- non-secret env vars still reach the child (B2)"
    rm -rf "$ENV_SPY_NDK" "$ENV_SPY_CAPTURE"

    # R1 (retry 2): a verification failure must never be swallowed by
    # `set -e` killing the script before the `case "$verify_status"`
    # dispatch gets to classify it -- this is a SCRIPT-level regression
    # that no lib-level test on verify_jar_signature can catch (this
    # harness itself runs under `set -uo pipefail`, deliberately without
    # `-e`). A shim jarsigner reports success on -verify but omits the
    # "jar verified" marker text, reproducing the exact drift
    # verify_jar_signature classifies as status 2 -- the case that also
    # collides with preflight_fail's own reserved exit code when the
    # dispatch is dead code.
    R1_ROOT="$(mktemp -d)"
    cat > "$R1_ROOT/jarsigner" <<EOF
#!/usr/bin/env bash
for a in "\$@"; do
    if [ "\$a" = "-verify" ]; then
        printf '%s\n' "jarsigner: verification succeeded but without the usual marker text"
        exit 0
    fi
done
exec "$REAL_JARSIGNER" "\$@"
EOF
    chmod +x "$R1_ROOT/jarsigner"

    err_r1="$(PATH="$R1_ROOT:$PATH" env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"
    status_r1=$?
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    assert_eq "1" "$status_r1" \
        "android-sign.sh: a verification drift (missing 'jar verified' marker) exits 1, not 2 (R1)"
    case "$err_r1" in
        *"did not verify"*) msg_r1="yes" ;;
        *) msg_r1="no" ;;
    esac
    assert_eq "yes" "$msg_r1" \
        "android-sign.sh: a verification drift prints a real diagnostic, not a silent exit (R1)"
    rm -rf "$R1_ROOT"

    # W1 (same-shape sweep, round 3): the `case "$verify_status"` dispatch
    # has no `*)` arm -- a status outside {0,1,2,3} falls through as a
    # silent no-op and the bundle ships as verified. verify_jar_signature's
    # own `-ne 0` check collapses ANY nonzero jarsigner exit into a plain
    # `return 1`, so killing the `jarsigner -verify` CHILD only ever
    # reaches the existing `1)` branch -- the only way android-sign.sh's
    # own `verify_status` can land outside {0,1,2,3} is the SUBSHELL that
    # runs `verify_jar_signature` itself dying from a signal before any of
    # its `return` statements execute, which is two fork levels above the
    # `jarsigner -verify` child (one subshell for the outer
    # `$(verify_jar_signature ...)`, one more for its own inner
    # `$(jarsigner -verify ...)`) -- confirmed by walking the live process
    # tree with `ps` while this exact shim ran. A shim jarsigner, invoked
    # with -verify, looks up its own grandparent PID via `ps -o ppid=` and
    # SIGKILLs THAT instead of exiting normally, so android-sign.sh
    # observes verify_status = 128+9 = 137.
    W1_ROOT="$(mktemp -d)"
    cat > "$W1_ROOT/jarsigner" <<EOF
#!/usr/bin/env bash
for a in "\$@"; do
    if [ "\$a" = "-verify" ]; then
        grandparent="\$(ps -o ppid= -p "\$PPID" 2>/dev/null | tr -d ' ')"
        [ -n "\$grandparent" ] && kill -KILL "\$grandparent"
        sleep 2
        exit 0
    fi
done
exec "$REAL_JARSIGNER" "\$@"
EOF
    chmod +x "$W1_ROOT/jarsigner"

    err_w1="$(PATH="$W1_ROOT:$PATH" env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"
    status_w1=$?
    signed_w1_exists="no"
    [ -f "$SIGN_ROOT/unsigned-16k-signed.aab" ] && signed_w1_exists="yes"
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    assert_eq "1" "$status_w1" \
        "android-sign.sh: an out-of-range verify status fails closed, not through (W1)"
    case "$err_w1" in
        *"unexpected verify status"*) msg_w1="yes" ;;
        *) msg_w1="no" ;;
    esac
    assert_eq "yes" "$msg_w1" \
        "android-sign.sh: an out-of-range verify status names the cause instead of shipping silently (W1)"
    assert_eq "no" "$signed_w1_exists" \
        "android-sign.sh: no signed artifact is left behind after an out-of-range verify status (W1)"
    rm -rf "$W1_ROOT"

    # W2 (same-shape sweep, round 3): branches `1)` and `3)` of the same
    # dispatch are pinned above only at lib level, on the
    # verify_jar_signature (AC 4, mandatory hand-mutants g+h) assertions'
    # RETURN VALUE -- never on the dispatch that CONSUMES it. Deleting
    # either `fail` arm would ship a
    # bundle jarsigner could not verify, or one signed by the wrong alias,
    # and nothing existing would redden. Two shim variants drive both
    # statuses through the real dispatch end-to-end, the way R1 already
    # does for status 2.
    W2_STATUS1_ROOT="$(mktemp -d)"
    cat > "$W2_STATUS1_ROOT/jarsigner" <<EOF
#!/usr/bin/env bash
for a in "\$@"; do
    if [ "\$a" = "-verify" ]; then
        printf '%s\n' "jarsigner: shim forces a verify failure (W2 status 1)"
        exit 1
    fi
done
exec "$REAL_JARSIGNER" "\$@"
EOF
    chmod +x "$W2_STATUS1_ROOT/jarsigner"

    err_w2_status1="$(PATH="$W2_STATUS1_ROOT:$PATH" env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"
    status_w2_status1=$?
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    assert_eq "1" "$status_w2_status1" \
        "android-sign.sh: verify status 1 (jarsigner could not verify) fails at the dispatch, script-level (W2)"
    case "$err_w2_status1" in
        *"jarsigner could not verify the signed bundle"*) msg_w2_status1="yes" ;;
        *) msg_w2_status1="no" ;;
    esac
    assert_eq "yes" "$msg_w2_status1" \
        "android-sign.sh: verify status 1 names the dispatch's own message, script-level (W2)"
    rm -rf "$W2_STATUS1_ROOT"

    # W2 status 3 is now driven by a FINGERPRINT mismatch, not by
    # jarsigner's own (unreliable, see the fixture-sanity check above)
    # alias-mismatch text -- so the shim that reaches it fakes `keytool
    # -printcert`, not `jarsigner -verify`. `keystore_alias_fingerprint`
    # (called before this shim's PATH entry matters, but shadowed all the
    # same) falls through to the REAL keytool for its `-list` call, so the
    # expected fingerprint is the real alias's; the shimmed `-printcert`
    # call inside verify_jar_signature returns a fingerprint that can
    # never match it.
    W2_STATUS3_ROOT="$(mktemp -d)"
    cat > "$W2_STATUS3_ROOT/keytool" <<EOF
#!/usr/bin/env bash
for a in "\$@"; do
    if [ "\$a" = "-printcert" ]; then
        printf '%s\n' "Signer #1:"
        printf '%s\n' "Certificate #1:"
        printf '%s\n' "Certificate fingerprints:"
        printf '\t SHA256: %s\n' "00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF"
        exit 0
    fi
done
exec "$REAL_KEYTOOL" "\$@"
EOF
    chmod +x "$W2_STATUS3_ROOT/keytool"

    err_w2_status3="$(PATH="$W2_STATUS3_ROOT:$PATH" env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" 2>&1 1>/dev/null)"
    status_w2_status3=$?
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    assert_eq "1" "$status_w2_status3" \
        "android-sign.sh: verify status 3 (fingerprint mismatch) fails at the dispatch, script-level (W2)"
    case "$err_w2_status3" in
        *"is not signed by alias"*) msg_w2_status3="yes" ;;
        *) msg_w2_status3="no" ;;
    esac
    assert_eq "yes" "$msg_w2_status3" \
        "android-sign.sh: verify status 3 names the dispatch's own message, script-level (W2)"
    rm -rf "$W2_STATUS3_ROOT"

    # R3 (retry 2): the SAME scrub that already protects the alignment
    # re-check's child (B2, above) must cover the jarsigner -verify child
    # too -- the `@law:` on `-storepass:env` at the top of the script
    # claims BOTH descendants are covered, and until now only one was.
    # A shim jarsigner dumps its own environment when invoked with
    # -verify and otherwise execs the real binary
    # transparently, so both the signing call and the verify call still
    # succeed and only the verify child's environment is captured.
    ENV_SPY_R3_DIR="$(mktemp -d)"
    ENV_SPY_R3_CAPTURE="$(mktemp)"
    cat > "$ENV_SPY_R3_DIR/jarsigner" <<EOF
#!/usr/bin/env bash
for a in "\$@"; do
    if [ "\$a" = "-verify" ]; then
        env > "$ENV_SPY_R3_CAPTURE"
    fi
done
exec "$REAL_JARSIGNER" "\$@"
EOF
    chmod +x "$ENV_SPY_R3_DIR/jarsigner"

    PATH="$ENV_SPY_R3_DIR:$PATH" env NDK_HOME="$SYNTH_NDK_HOME" \
        ANDROID_SIGN_KEYSTORE="$SIGN_KEYSTORE" ANDROID_SIGN_KEY_ALIAS="$SIGN_ALIAS" \
        ANDROID_SIGN_STORE_PASSWORD="rightstorepw" ANDROID_SIGN_KEY_PASSWORD="rightkeypw" \
        "$SIGN" "$UNSIGNED_AAB_16K" >/dev/null 2>&1
    status_r3=$?
    rm -f "$SIGN_ROOT/unsigned-16k-signed.aab"
    assert_eq "0" "$status_r3" \
        "android-sign.sh: R3 fixture -- the run reaching the verify shim still succeeds"
    store_pw_in_verify_env="no"
    grep -q '^ANDROID_SIGN_STORE_PASSWORD=' "$ENV_SPY_R3_CAPTURE" && store_pw_in_verify_env="yes"
    assert_eq "no" "$store_pw_in_verify_env" \
        "android-sign.sh: the jarsigner -verify child never sees the store password (R3)"
    key_pw_in_verify_env="no"
    grep -q '^ANDROID_SIGN_KEY_PASSWORD=' "$ENV_SPY_R3_CAPTURE" && key_pw_in_verify_env="yes"
    assert_eq "no" "$key_pw_in_verify_env" \
        "android-sign.sh: the jarsigner -verify child never sees the key password (R3)"
    verify_shim_ran="no"
    [ -s "$ENV_SPY_R3_CAPTURE" ] && verify_shim_ran="yes"
    assert_eq "yes" "$verify_shim_ran" \
        "android-sign.sh: the verify shim actually ran and captured an environment (R3 fixture sanity)"
    rm -rf "$ENV_SPY_R3_DIR" "$ENV_SPY_R3_CAPTURE"

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
assert_eq "0" "$(git_is_ignored "upload.pfx")" "gitignore: *.pfx is ignored at repo root (B4)"
assert_eq "0" "$(git_is_ignored "upload.pepk")" "gitignore: *.pepk is ignored at repo root (B4)"
assert_eq "0" "$(git_is_ignored "keystore.properties")" "gitignore: keystore.properties is ignored at repo root (B4)"
assert_eq "0" "$(git_is_ignored "some/nested/dir/keystore.properties")" \
    "gitignore: keystore.properties is ignored at a nested depth (unrooted pattern, B4)"

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
