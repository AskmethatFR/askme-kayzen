# Kayzen

A habit app built on one rule: the gentlest option always wins.

A habit starts at a soft daily goal of 5 minutes. It grows only when *you* decide to
grow it — the system never detects "stability", never suggests a step up, and never
tells you that you failed. A day without a completion is an empty day, not a broken
streak. The board holds at most five habits in parallel, on purpose.

The product decisions behind those sentences are written down, not folklore — see
[`docs/INDEX.md`](docs/INDEX.md).

## Workspace

Two crates, one dependency direction.

| Crate | Contents |
|---|---|
| `core` (`kayzen-core`) | The domain, the use cases, the read-side queries, the in-memory adapters. No UI framework. |
| `app` (`kayzen-app`) | The Dioxus front end: routes, screens, composition root. Depends on `core`; `core` never depends on it. |

Rust 2024 edition, Dioxus 0.7 (`web` by default, `desktop` and `mobile` behind features).

Inside `core`, the layout follows Clean Architecture and DDD: `domain/` holds the
aggregate and its value objects, `use_cases/` the commands, `queries/` the read side
(CQRS-light — commands mutate and return nothing, screens re-query), `infrastructure/`
the adapters. Domain types never cross into `app`; DTOs do.

## Run it

```bash
cargo test --workspace   # every test, both crates
cd app && dx serve       # the web app (cargo install dioxus-cli)
```

## Android (local, unsigned)

```bash
scripts/android-deploy.sh   # build, install and launch on the device plugged in over USB
```

The script is the whole procedure: it checks the toolchain, builds the debug app from
the `mobile` feature, applies the launcher icon, installs it and launches it. Nothing is
signed and no keystore is involved — this is the "see it on my own screen" path, not a
release one.

The icon step is `scripts/android-icon.sh`, called between the two. `dx` 0.7.9 does not
wire `[android].icon` and rewrites `res/` under `target/` on every build, so
`app/android/res/` cannot reach the APK on its own: the icon is copied in once `dx` has
generated the Gradle project, and Gradle is then run alone — a second `dx build` would
put the template icons straight back. The script lives on its own so the release pipeline
can call it too, and it deletes the template's `ic_launcher*.webp` rather than copying
over them: Android resolves a resource by name and not by extension, so a leftover
`.webp` beside our `.png` gives `@mipmap/ic_launcher` two definitions and AAPT2 fails.

The one-time setup it expects, and why each version is pinned where it is:

| Piece | Value | Why |
|---|---|---|
| Android SDK | `~/Library/Android/sdk` | `ANDROID_HOME`; `sdkmanager` comes from the `android-commandlinetools` cask |
| SDK platform | `platforms;android-36` | `compile_sdk`/`target_sdk` in `app/Dioxus.toml`, aimed straight at 36 per the compileSdk-ladder decision |
| NDK | `25.2.9519653` (r25c) | the version Dioxus 0.7 documents, and the one Rust targets for `aarch64-linux-android` |
| CMake | `3.22.1` | installed alongside the NDK, from the SDK |
| JDK | 17 | the Android Gradle Plugin runs on 17; a newer JDK on `PATH` is not a substitute |
| Rust target | `aarch64-linux-android` | every current device; add the other three for emulators |

```bash
brew install --cask android-commandlinetools
sdkmanager --sdk_root="$ANDROID_HOME" --install "platforms;android-36" "ndk;25.2.9519653" "cmake;3.22.1"
rustup target add aarch64-linux-android
```

On an Apple-silicon Mac the r25 toolchain binaries are x86_64, so Rosetta 2 runs them.

The app id is `com.askmethat.kayzen` (`app/Dioxus.toml`'s `[android] identifier`). If
a device still carries an earlier build under the old `com.example.KayzenApp` id,
uninstall it first — Android treats a changed applicationId as a different app, so the
two coexist with separate `/data/data/.../files` directories instead of one upgrading
the other.

The manifest at `app/android/AndroidManifest.xml` is a hand-owned fork of the one `dx`
generates — `dx` rewrites the whole Gradle project on every build, so this is the one
file it is told to copy in verbatim instead (`[application].android_manifest`). Its own
leading comment carries the exact list of edits and their rationale, including
`android:allowBackup="false"` (owner-ruled: strict local, no Auto Backup / device
transfer of habit data, which is otherwise the platform default and would otherwise
include `getFilesDir()/kayzen/` — real backup/restore is a separate, later ticket).
**Because `[application].android_manifest` replaces rather than merges, every
manifest-shaped `Dioxus.toml` key is inert while this fork is in place** — see the
fork's own comment for the full list. Re-diff both this file *and* the generated
`res/xml/network_security_config.xml` against a fresh debug build whenever `dx` is
upgraded — the config isn't forked (nothing to fork yet: no `<base-config>`, cleartext
stays denied except to `127.0.0.1` for hot-reload), but a future `dx` changing that
default would ship silently otherwise.

The build prints `WARNING: We recommend using a newer Android Gradle plugin to use
compileSdk = 36` (AGP 8.7.0 was tested up to 35) and still succeeds — expected until
the Gradle project's pinned AGP moves. `dx` exposes no gradle-args passthrough to add
`android.suppressUnsupportedCompileSdk=36` from outside the generated project, so this
is silenced only by an AGP bump, tracked separately from this repo's `compileSdk`.

The device needs USB debugging on, and *Install via USB* on Xiaomi/HyperOS. Even then
the phone asks to confirm the first install of each build: `adb` reports
`INSTALL_FAILED_USER_RESTRICTED` if the prompt is not accepted in time, and re-running
the script is the fix.

### Android (release bundle)

```bash
scripts/android-bundle.sh   # build an unsigned, 16 KB-aligned, correctly-versioned .aab
```

Play requires every native library in an app targeting Android 15+ to align its `LOAD`
segments to 16 KB, not the historical 4 KB — `scripts/android-verify-alignment.sh`
checks this on the bundle's own bytes, and `scripts/android-release-lib.sh` holds the
two pure functions (`min_load_alignment`, `version_code_from_semver`) both scripts share,
pinned by `scripts/test-shell-units.sh` (the "shell units" gate, see below).

The script is two seams around one `dx bundle` call, in this order, because `dx` owns
the whole generated Gradle project and rewrites it on every run:

1. **`dx bundle --platform android --release --package-types aab
   --rustc-args="-Clink-arg=-Wl,-z,max-page-size=16384 -Clink-arg=-Wl,-z,common-page-size=16384"`**
   — the `=` before the value is mandatory: a space-separated value starting with `-C` is
   read by `dx`'s own argument parser as one of *its* flags, not as `--rustc-args`'s value.
   `dx` sets `RUSTFLAGS` itself and blanks the target-specific override, so a
   `.cargo/config.toml` entry for this flag is dead, silently — `--rustc-args` is the only
   place that reaches the final `cargo rustc` invocation linking `libmain.so`.
2. Gradle alone, patched first: the launcher icon is applied
   (`scripts/android-icon.sh`), the generated `build.gradle.kts`'s `versionCode` is
   patched from `Cargo.toml`'s `[workspace.package].version`, and only then does
   `./gradlew bundleRelease` run. It does not recompile Rust — it re-packages the
   `.so` step 1 already linked — so the icon and version survive into the bundle
   step 1 alone would not have produced.

Every negative control that backs this (a broken alignment flag, a missing patch, a
stale bundle left in place by a Gradle no-op, a leaked cleartext signing config, a
forbidden `[bundle.android]` section) is run and recorded slice by slice in the
ticket, not restated here. One is worth calling out: removing `--rustc-args` and
forcing a clean rebuild makes the bundle fail the alignment check — the control that
proves the flag is load-bearing, not decorative.

Same one-time toolchain setup as the local debug path above, plus `NDK_HOME` pointing
at the same NDK. `scripts/android-verify-alignment.sh` reads `NDK_HOME` directly with
no `ANDROID_HOME`-derived default, so it works unchanged against whatever NDK version
CI installs (see `.github/workflows/ci.yml`'s `android-aab` and `gates` jobs);
`scripts/android-bundle.sh` itself falls back to `$ANDROID_HOME/ndk/25.2.9519653` when
`NDK_HOME` is unset.

**What this does not prove**: that R8 stripped nothing load-bearing. `isMinifyEnabled`
stays `true`, and `proguard-wry.pro` (already generated by the `dx` template, already
picked up by the module's own `build.gradle.kts`) covers the JNI/reflective surface —
but only a real run on a real device proves nothing broke, and that is a later slice,
not this one.

## Gates

```bash
scripts/check.sh                          # fmt, clippy, workflow lint, tests, shell units, scenario gate, doc anchors
scripts/check.sh new-feature <base-ref>   # the above, plus the mutation gate on the committed diff
```

Requires `actionlint` on `PATH` for the workflow-lint gate (`brew install actionlint`),
alongside the toolchain prerequisites above — a missing `actionlint` refuses (exit 2)
rather than silently skipping the gate, same doctrine as every other gate here.

The shell-units gate runs `scripts/test-shell-units.sh`, a house harness for
`scripts/android-release-lib.sh` — the pure functions the Android release build
(`scripts/android-bundle.sh`) relies on and cannot exercise through `cargo test`.

The workflow-lint gate runs bare `actionlint` (auto-discovering every
`.github/workflows/*.{yml,yaml}`, not a hand-rolled `*.yml`-only glob): a
workflow referencing an unavailable context (e.g. `env` inside `jobs.<id>.env`) fails
to *load*, taking every job in the file down with it — this is the one gate that
guards the gates themselves.

The scenario gate is bidirectional: every Gherkin scenario in
`docs/functional/features/` resolves to a test through a `// @scenario: <feature>/<Sn>`
anchor, and every anchor resolves back to a scenario. A scenario still tagged `@wip`
is spec-only and waives coverage until its slice lands.

The mutation gate runs on the **committed** diff, so each slice is committed before it
is measured. It blocks on `fix-bug` and `new-feature`, and is advisory on
`quick-change`. Rationale and known blind spots:
[`docs/technical/adr/0009-quality-gates.md`](docs/technical/adr/0009-quality-gates.md).

CI runs `scripts/check.sh` on every PR and on `main`. The PR run additionally computes
the merge-base against `main` and uses it as the mutation gate's base-ref. The gate
implementations are pinned under `scripts/vendor/`, refreshed only by an explicit commit.

## Documentation

`docs/` is a graph, not a folder: [`docs/INDEX.md`](docs/INDEX.md) is the table, and the
`[[links]]` inside each node are its edges. Technical nodes (architecture, ADRs) and
functional nodes (feature catalog, glossary, backlog, Gherkin) are separate lanes.
Start at the index. The functional and design nodes are written in French.

## License

[PolyForm Noncommercial 1.0.0](LICENSE) — free to read, use, modify and share **for any
noncommercial purpose**. Commercial use is not granted. This is a source-available
license, not an OSI-approved open-source one.
