---
id: "adr-0023-platform-behaviour-seam-patched-kotlin"
type: "technical"
owner: "architect"
# FACET SUPERSESSION 2026-10-06 — [[adr-0024-safe-area-css-bridge]] retires exactly
# ONE facet of this node: « How the content learns the safe areas » = decor-view
# padding (and this title's « the safe areas reach the content as decor-view padding »
# clause with it). This node stays `current` — its seam placement, patch house shape
# and artifact-is-oracle rulings are untouched and re-anchored in adr-0024's
# consequences. The escape clause that fires is this node's own: « MUST NOT freeze
# the insets application form; safe-area-css-bridge may replace the padding ».
# Scope also recorded in this node's docs/INDEX.md row (0019 frontmatter-comment
# precedent; body untouched, no `## Amendment`).
status: "current"
updated: "2026-10-06"
relations:
  related:
    - "adr-0019-android-release-bundle-seam"
    - "adr-0022-tag-derived-release-version"
    - "adr-0021-router-layout-persistent-chrome"
    - "android-play-release-runbook"
answers:
  - "Where does Android platform behaviour live when the app shell is generated and rewritten on every build?"
  - "How does the web content learn the safe areas on Android, and what is env(safe-area-inset-*) still for?"
decided_in:
  - "#75 — 2026-10-01, Play Console 0.0.3 edge-to-edge / deprecated window APIs / AGP 9"
---

# ADR 0023 — Platform behaviour lives in a seam patch of the generated Kotlin; the safe areas reach the content as decor-view padding

> **One-liner**: The Android shell is generated and rewritten in full on every build, so **we own no Kotlin source**: edge-to-edge and inset handling are a seam patch of the generated `MainActivity.kt`, in the same exactly-once-anchor / marker-swap / read-back shape every seam patch already takes. System-bar insets are compensated by **padding the decor view** from a `WindowInsets` listener — never by trusting `env(safe-area-inset-*)` inside the WebView, which measures `0` in Chromium ([[adr-0021-router-layout-persistent-chrome]]). The CSS `env()` chain is demoted to a **web/iOS fallback**, pinned per site by test.
> **Links**: [[adr-0019-android-release-bundle-seam]] (the two-pass seam this extends — applied here, not restated: patch between the passes, assert the anchor, verify on the produced artifact), [[adr-0022-tag-derived-release-version]] (which of adr-0019's clauses survive — the seam and verify-on-produced-bytes among them), [[adr-0021-router-layout-persistent-chrome]] (the measured `env() = 0` fact that kills the CSS-only option, and the chrome/layout surface the padding protects), [[android-play-release-runbook]] (the patch-target roster a `dx` upgrade must re-diff).

## Context

Google Play flagged release 0.0.3 twice on window display: edge-to-edge "may not display for all users" (insets unhandled where the platform draws edge-to-edge), and two deprecated calls — `Window.setStatusBarColor` / `Window.setNavigationBarColor` — originating inside the Material **library**, not our code. The app ships **no Kotlin/Java of its own**: `dx` generates the whole Android project (`MainActivity : WryActivity()`, `Theme.AppCompat.Light.NoActionBar`, AGP, dependencies) and rewrites it on every invocation, and our only hand-owned Android file is the manifest fork. Meanwhile the web layer already opts into `viewport-fit=cover` and pads with `env(safe-area-inset-*)` in fifteen places — a mechanism measured at **0** in Chromium, so on Android the content sat under the system bars while every CSS test stayed green.

Two placement questions follow, and neither is settled by the existing graph: where platform window behaviour may live at all, given that every generated file dies on the next `dx` run; and how the content is told about the safe areas once `env()` is known dead on that platform.

## Decision

| Facet | Decision |
|---|---|
| **Where platform behaviour lives** | In a **seam patch of the generated `MainActivity.kt`**, between `dx` pass 1 and `gradlew bundleRelease` — `WindowCompat.setDecorFitsSystemWindows(this.window, false)` plus a `ViewCompat.setOnApplyWindowInsetsListener` on the decor view. The patch takes the house shape: one exactly-once anchor (`class MainActivity : WryActivity()`), whole-class replacement, impossible-marker swap, read-back ([[adr-0019-android-release-bundle-seam]]'s patch rules, applied) |
| Why not an owned Kotlin/Java source | The generator rewrites the project in full on every run ([[adr-0019-android-release-bundle-seam]]'s *generated build project is patched at a seam, not configured*). A source file we own would either live outside the generated tree and need plumbing `dx` does not offer, or live inside it and be destroyed on the next build. The patch keeps *the manifest is our only hand-owned Android file* true, and every patched line is compile-checked at `bundleRelease` — a loud build failure, unlike a runtime-resolved method-signature string |
| **How the content learns the safe areas** | **Decor-view padding**: the insets listener pads the decor view (`systemBars ∪ displayCutout`), so the WebView never draws under the bars and needs no knowledge of them. The CSS `env(safe-area-inset-*, 0px)` chain stays as the web/iOS fallback and is pinned **per site** by test (exact count + fallback form) |
| Why not `env()` on the Android arm | Measured `0` in Chromium with `viewport-fit=cover` ([[adr-0021-router-layout-persistent-chrome]]): Android WebView does not populate it. Compensating through `env()` there is a green suite over an obscured UI |
| Why padding, not a bridge | Padding is the smallest mechanism that satisfies "content not obscured" — no WebView handle, no JS bridge, no custom-property contract. The padding form is explicitly **not frozen**: a native→JS bridge is a named future slice (`safe-area-css-bridge`) that may replace it without breaking any external reader |
| **The artifact is the oracle** | AC "no deprecated system-bar calls in the shipped app" is proven by scanning the produced AAB's own **dex bytes** (`scripts/android-verify-no-deprecated-bar-apis.sh`, ordinal entry addressing, exit 1 = defect / 2 = missing prerequisite / mandatory verdict line), and re-verified on the signed bytes before publish — never by a dependency version number ([[adr-0019-android-release-bundle-seam]]: *a property required of the artifact is verified on the artifact*) |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Trust `env(safe-area-inset-*)` + `viewport-fit=cover` alone | Measured `0` in Chromium ([[adr-0021-router-layout-persistent-chrome]]) — the exact failure this ticket exists to fix, invisible to every test |
| A native→JS bridge writing `--safe-area-inset-*` custom properties | Same outcome as padding with more moving parts (a JS seam, a property contract, two runtimes to attest). Deferred as the **named** slice `safe-area-css-bridge`, which is free to replace the padding precisely because this node does not freeze the form |
| `WindowCompat`-free platform opt-outs (`windowOptOutEdgeToEdgeEnforcement`) | API 35 only, ignored where we target (SDK 36 / Android 16): a dead end that delays the same work while fighting the platform Play tells us to cooperate with |
| `EdgeToEdge.enable()` (the ticket's Kotlin example) | Lives in `androidx.activity`, whose presence on the `dx` template's classpath is unverified; `WindowCompat`/`ViewCompat` come from `androidx.core`, provably pulled in by appcompat/webkit/material. Taking an unverifiable dependency to match a doc example is not required — the ticket itself says "or equivalent" |
| Rust/JNI window code (edge-to-edge + insets listener) | Method-signature strings resolved at runtime, on exactly the arm of the stack no gate here can verify; the whole gain of the patch form is that `bundleRelease` compiles it |
| Fork `res/values/styles.xml` instead of patching `MainActivity.kt` | The icon step already copies `app/android/res/` beside the generated `values/styles.xml`, and AAPT2 refuses duplicate resources; a theme cannot pad the decor view anyway |
| Keep `env()` as the Android mechanism "for consistency" | Consistency with a function that returns 0 is not consistency — it is a documented no-op wearing a mechanism's clothes (cf. the same-shape family of [[adr-0020-signer-alias-verification-text-parse]]: a check that cannot fail is a claim) |

## Consequences / Constraints

- **MUST**: keep all Android platform window behaviour in seam patches of generated files — **MUST NOT** add a Kotlin/Java source under `app/android/` or any other hand-owned build input the generator would overwrite or duplicate.
- **MUST**: compensate Android system-bar insets at the decor view; **MUST NOT** rely on `env(safe-area-inset-*)` for the Android arm. The CSS `env()` sites remain as the web/iOS fallback and stay pinned per site (exact count + `env(safe-area-inset-*, 0px)` form).
- **MUST**: extend the patch-target roster (root `build.gradle.kts`, `app/build.gradle.kts`, `MainActivity.kt`, `*.pro`, and any further generated file a patch touches) in the runbook's re-diff list — a `dx` upgrade re-derives every anchor, and a silently moved anchor must refuse loudly ([[adr-0019-android-release-bundle-seam]]'s patch rule).
- **MUST**: keep the deprecated-API absence proven on the artifact's own bytes (dex scan pre-sign, re-verify on signed bytes) — a version bump of Material is a belt, never the proof.
- **MUST NOT**: freeze the insets application *form* here. `safe-area-css-bridge` (or any later slice) may replace decor-view padding; what is frozen is only where the behaviour lives (the seam) and that the verifier contract keeps its exit taxonomy and verdict line.
- The padding form having no reader outside the seam's own patch is what makes that non-freeze cheap; if a second consumer of the insets ever appears, the freeze question reopens *before* that consumer lands.
