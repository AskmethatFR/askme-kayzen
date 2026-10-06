---
id: "adr-0024-safe-area-css-bridge"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-10-06"
relations:
  supersedes:
    # FACET SUPERSESSION — [[adr-0023-platform-behaviour-seam-patched-kotlin]] stays
    # `current`. Its DECISION spine is untouched and remains binding, re-anchored in
    # this node's Consequences: platform window behaviour lives in a seam patch of the
    # generated Kotlin (no owned Kotlin/Java source), the patch house shape
    # (exactly-once anchor / marker swap / loud refusal / read-back), and the
    # artifact-is-oracle dex scan. Exactly ONE facet is dead: « How the content learns
    # the safe areas » = decor-view padding — and its own escape clause (« MUST NOT
    # freeze the insets application *form*; `safe-area-css-bridge` may replace the
    # padding ») is what fires here. Scope also recorded in that node's
    # docs/INDEX.md row and in its frontmatter.
    - "adr-0023-platform-behaviour-seam-patched-kotlin"
  related:
    - "adr-0021-router-layout-persistent-chrome"
    - "adr-0019-android-release-bundle-seam"
    - "android-play-release-runbook"
answers:
  - "How do the device insets reach the web layer, in what units, and when do they refresh?"
  - "Why a one-method JS bridge instead of decor-view padding?"
  - "What is the bridge's security surface, and what reopens it?"
  - "What keeps the web/iOS env() arm alive when no bridge exists?"
decided_in:
  - "#79 — 2026-10-06, safe-area-css-bridge (feat/safe-area-css-bridge @ bd5f984)"
---

# ADR 0024 — The insets reach the web layer as `--safe-area-inset-*` CSS custom properties written by a one-method `AskmeSafeArea` JS bridge

> **One-liner**: the insets reach the web layer as `--safe-area-inset-*` CSS custom properties written by a one-method `AskmeSafeArea` JS bridge; the decor-view padding form is retired. Units are **CSS px**, density-corrected at the seam; timing is a load **pull** plus an insets-change **push**; when the bridge is absent the variables stay **unset**, so the web/iOS `env()` arm keeps deciding.
> **Links**: [[adr-0023-platform-behaviour-seam-patched-kotlin]] (one facet retired here — its seam, patch-shape and artifact-oracle rulings survive unamended, re-anchored below), [[adr-0021-router-layout-persistent-chrome]] (`env(safe-area-inset-*)` measured `0` in Chromium — the fact that puts a bridge between the runtimes and kills the CSS-only arm on Android), [[adr-0019-android-release-bundle-seam]] (the two-pass seam the patch lives in; its patch rules applied, not restated).

## Context

[[adr-0021-router-layout-persistent-chrome]] measured `env(safe-area-inset-*)` at **0** in the Android WebView under `viewport-fit=cover`, so the CSS chain alone left content under the system bars and the camera cutout. [[adr-0023-platform-behaviour-seam-patched-kotlin]] compensated by **padding the decor view**, and explicitly did *not* freeze that form — naming `safe-area-css-bridge` as the slice free to replace it. Padding kept the inset bands out of the page's reach entirely: the page background could never paint under the bars (`safe-area-bleed` S1 fails by construction), and the web layer still knew nothing of the geometry it must respect. Issue #79 is that named replacement — shipped `feat/safe-area-css-bridge` @ `bd5f984`, Dev-B / QA / Security green, device-attested on a notched device (QA legs 1–7).

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| **Bridge form** | One object `AskmeSafeArea`, **exactly one** `@android.webkit.JavascriptInterface` member — `fun insets(): String`, 0-arg, read-only — registered `webView.addJavascriptInterface(safeArea, "AskmeSafeArea")` inside `onWebViewCreate` (before page load). The class holds one `@Volatile` cached string and **no** Activity/WebView reference | `scripts/android-release-lib.sh` seam block |
| **Wire format + units** | `"T R B L"` — four `%.2f` slots, `Locale.US`, **CSS px** (physical px ÷ `resources.displayMetrics.density` at the seam). `env()` is specified in CSS px, so the custom property and the fallback arm are commensurable; raw px would over-pad a ~2.75× density device, and `Locale.US` keeps the JS parse locale-proof | seam block, `app/src/safe_area.rs` |
| **Push + pull timing** | **Pull** at load: `document::eval(BOOTSTRAP_JS)` from the app shell — the script defines the single writer `window.__kzApplySafeAreaInsets(t, r, b, l)`, then pulls `bridge.insets()` once. **Push** on every insets dispatch: the listener refreshes the cache and posts `webView.evaluateJavascript(applyScript())` on the WebView thread (the same `post` shape wry's own `evalScript` uses); `onWebViewCreate` re-dispatches `requestApplyInsets()`. The cache is the source of truth: a pull taken late reads the current snapshot, a lost push re-converges at the next dispatch | `app/src/safe_area.rs`, `app/src/main.rs`, seam block |
| **Absent-bridge guard** | The pull and the writer call sit inside `if (bridge)`. No bridge ⇒ the four variables stay **unset** — never written as `0px`, which would shadow the `env()` fallback while looking like one | `app/src/safe_area.rs` |
| **Chain form** | Every safe-area site reads `var(--safe-area-inset-*, env(safe-area-inset-*, 0px))`: Android arm = the bridge's custom property, web/iOS arm = `env()`. Exactly **15** sites, form and count pinned | `app/assets/main.css` |
| **Inset mask** | `systemBars() ∪ displayCutout() ∪ ime()` — rotation **and** the on-screen keyboard refresh the same four variables (bars ∪ cutout alone would leave the keyboard a no-op) | seam block |
| **R8 keep** | `-keepclassmembers class * { @android.webkit.JavascriptInterface <methods>; }` joins `PRO_RULES` at the seam. A rename of `insets()` would break the JS call at runtime, invisible to every gate — artifact-oracle discipline ([[adr-0023-platform-behaviour-seam-patched-kotlin]]'s dex scan untouched) | `scripts/android-bundle.sh` |
| **Wire contract moves together** | Seam format ↔ push's space→comma conversion ↔ pull's split ↔ four `parseFloat` reads ↔ the writer's four parameters ↔ its four `setProperty` pairs are pinned to change together: alter one and the cross-artifact pins redden the others | `app/src/safe_area.rs`, `scripts/test-shell-units.sh` |
| **Security surface (appendix A)** | One interface, one method, **no references** into native (a plain four-number string; the class caches a string and nothing else), **grammar-closed push** (`window.__kzApplySafeAreaInsets(<t>,<r>,<b>,<l>)`, built from four `%.2f` numbers — no external data concatenated). Threat model: any page script may read four geometry numbers; the page origin is the bundled assets. Anti-widening pins hold the shape: exactly one `@JavascriptInterface` in the seam block; the `AskmeSafeArea` name exactly once in the block and once in `BOOTSTRAP_JS` | #79 spec appendix A; Security review appendix A **7/7** |
| **S1 accepted by design** | Security's S1 — the bridge is a new inter-runtime surface, callable by any script running in the WebView — is **accepted by design** on the appendix-A shape above (read-only geometry, no references, grammar-closed push, bundled-origin page). **Reopening trigger**: any widening of that shape (a second method, a parameter, a returned object reference, a push payload outside the four-number grammar) **or** content we do not own ever loading in the WebView — the flagged, deliberately unchanged `RustWebView.shouldOverride` navigation policy is exactly that door | Security review, #79 |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Keep the decor-view padding form | The bands never enter the page's reach: the background cannot paint under the bars (`safe-area-bleed` S1 fails by construction) and the web layer cannot position content against the geometry. This is the facet [[adr-0023-platform-behaviour-seam-patched-kotlin]] deliberately left unfrozen — and the reason it was not frozen is this slice |
| Rust/JNI window code | Method-signature strings resolved at runtime on the one arm of the stack no gate here verifies — [[adr-0023-platform-behaviour-seam-patched-kotlin]] rejects it on the same ground; `bundleRelease` compiles the seam, not a JNI string |
| Push-only (no load pull) | A push that lands before the page defines the writer is lost; the pull is what gives the first paint its values |
| Polling the bridge | A timer to observe four numbers the platform already dispatches — latency and battery for nothing; `OnApplyWindowInsets` *is* the change signal |
| wry `with_initialization_script` | The builder is unreachable through `dioxus::launch`; a second launch path to save one `eval` is the wrong trade |
| Per-side methods or a JSON/object return | Every extra member is another `@JavascriptInterface` — the reviewed surface grows to carry a fixed four-slot payload the space grammar already carries |
| A bridge holding the Activity or WebView | A leak, and a reference the reader never needs — `insets()` reads a string cache |
| Write `0px` when the bridge is absent | A set variable shadows the `env()` arm: the fallback dies while looking present (the failure shape [[adr-0020-signer-alias-verification-text-parse]] names — a form that cannot fail is a claim) |

## Consequences / Constraints

- **MUST**: keep the bridge surface exactly as frozen here — one object, one 0-arg `insets(): String`, no returned references, grammar-closed push, name `AskmeSafeArea`. Any widening reopens Security's S1 **and** is a new ADR. The anti-widening pins are part of the decision, not merely its tests.
- **MUST**: keep the absent-bridge guard's semantics — variables **unset** when the bridge is missing, never `0px`; the `var(--X, env(X, 0px))` chain and its 15-site count stay pinned (`safe-area-bleed` S3).
- **MUST**: keep units CSS px at the seam (density division, `Locale.US`) and keep the wire contract's members moving together — the format changes only as one act across seam, push and pull.
- **MUST**: keep the R8 `@JavascriptInterface` keep rule in `PRO_RULES`, and keep proving required properties on the artifact's own bytes.
- **Re-anchored survivors of [[adr-0023-platform-behaviour-seam-patched-kotlin]] — unamended, still binding**: platform window behaviour lives in a seam patch of the generated `MainActivity.kt`, never an owned Kotlin/Java source; the patch house shape (exactly-once anchor, marker swap, loud refusal, read-back); edge-to-edge stays on (`setDecorFitsSystemWindows(window, false)`); the artifact is the oracle. What changes is only the insets application *form* — and that form is now **frozen**, its readers being a second runtime.
- **Scenarios**: `safe-area-bleed` S1–S3 hold — S1 full-bleed now reachable *by the page*, S2 content inside the reported insets (keyboard included, via the `ime` arm), S3 the 15-site fallback chain — device-attested (QA legs 1–7).
- The open notched-device / on-screen-keyboard question of [[navigation]] (`docs/functional/navigation.md`) is **device-verified** by those legs — a functional-node update the PM owns (see `gaps_left` of the cycle's `tech-doc-update`).
