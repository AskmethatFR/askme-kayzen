#!/usr/bin/env bash
# Build the unsigned debug Android app and put it on the device plugged in over USB.
#
# Usage: scripts/android-deploy.sh
#
# One-time toolchain setup — and why each pin is what it is — lives in the
# README, section "Android (local, unsigned)".

set -euo pipefail

ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
NDK_HOME="${NDK_HOME:-$ANDROID_HOME/ndk/25.2.9519653}"
JAVA_HOME="${JAVA_HOME:-$HOME/Library/Java/JavaVirtualMachines/corretto-17.0.13/Contents/Home}"
export ANDROID_HOME NDK_HOME JAVA_HOME
export PATH="$PATH:$ANDROID_HOME/platform-tools"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APK="$REPO_ROOT/target/dx/kayzen-app/debug/android/app/app/build/outputs/apk/debug/app-debug.apk"
PACKAGE="com.example.KayzenApp"
ACTIVITY="dev.dioxus.main.MainActivity"

fail() {
  echo "android-deploy: $1" >&2
  exit 1
}

[ -d "$ANDROID_HOME" ] || fail "no Android SDK at $ANDROID_HOME (set ANDROID_HOME)"
[ -d "$NDK_HOME" ] || fail "no NDK at $NDK_HOME (set NDK_HOME, or: sdkmanager --install 'ndk;25.2.9519653')"
[ -x "$JAVA_HOME/bin/java" ] || fail "no JDK 17 at $JAVA_HOME (set JAVA_HOME)"

rustup target list --installed | grep -qx aarch64-linux-android \
  || fail "missing Rust target (rustup target add aarch64-linux-android)"

command -v dx >/dev/null || fail "dx not found (cargo install dioxus-cli)"

DEVICES="$(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
[ -n "$DEVICES" ] || fail "no authorised device over USB (check 'USB debugging' and the RSA prompt)"

echo "==> building (unsigned debug, arm64-v8a)"
(cd "$REPO_ROOT/app" && dx build --platform android)

[ -f "$APK" ] || fail "build produced no APK at $APK"

echo "==> installing $PACKAGE"
adb install -r "$APK"

echo "==> launching $ACTIVITY"
adb shell am start -n "$PACKAGE/$ACTIVITY" >/dev/null

sleep 3
adb shell pidof "$PACKAGE" >/dev/null || fail "$PACKAGE died on launch (adb logcat)"

echo "==> $PACKAGE is running"
