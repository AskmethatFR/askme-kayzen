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
# section) of this repo's own debug libmain.so, arm64-v8a, on NDK r25c --
# the exact command is in T2's preflight. Every LOAD segment sits at
# Align 0x1000 (F3): this is the regression the whole ticket exists to fix.
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
# alignment flag (T2) is supposed to produce.
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
