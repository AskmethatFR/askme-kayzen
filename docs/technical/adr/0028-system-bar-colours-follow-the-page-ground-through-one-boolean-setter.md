---
id: "adr-0028-system-bar-colours-follow-the-page-ground-through-one-boolean-setter"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-11-27"
relations:
  supersedes:
    # FACET SUPERSESSION — [[adr-0024-safe-area-css-bridge]] stays `current`. Exactly
    # ONE facet is dead: the bridge surface being one interface with one method. The
    # seam now registers a second interface, `AskmeSystemBars`, with one method
    # `darkGround(Boolean)`. `AskmeSafeArea` itself (one 0-arg `insets(): String`, no
    # references, grammar-closed push) is untouched.
    - "adr-0024-safe-area-css-bridge"
  related:
    - "adr-0023-platform-behaviour-seam-patched-kotlin"
    - "adr-0027-android-back-goes-up-one-level-by-asking-the-page"
answers:
  - "How do the Android system bars follow the Ritual's dark ground?"
  - "What can a page script do with the second bridge, and why is that acceptable?"
  - "What brings the light bars back when the user leaves the Ritual or the app restarts?"
decided_in:
  - "S24 (FU7) — 2026-11-27, on the Ritual the system bars take the dark ground"
---

# ADR 0028 — System-bar colours follow the page ground through one boolean setter

> **One-liner**: The seam registers `AskmeSystemBars`, whose one `@JavascriptInterface` method `darkGround(Boolean)` switches both bars on the UI thread. The Ritual's practice screen calls it with `true` on mount and `false` on drop; `onCreate` still starts every Activity with light bars.
> **Links**: [[adr-0024-safe-area-css-bridge]] (one facet replaced: one interface, one method), [[adr-0023-platform-behaviour-seam-patched-kotlin]] (native behaviour lives in the seam patch), [[adr-0027-android-back-goes-up-one-level-by-asking-the-page]] (the other page-to-native link, a fixed script).

## Context

The seam hard-codes light system bars (`isAppearanceLight*Bars = true` in `onCreate`): dark icons, and on 3-button navigation a light contrast scrim. The Ritual paints a dark ground (`--color-ritual-ground`) up to `html` and `body`, so under it the nav bar showed a light band and the status icons sat dark on dark. Only the page knows when the Ritual is shown, so the page must tell the native shell, which needs a JS-to-native link. adr-0024 froze the bridge at one interface with one method and made any widening a new ADR plus a security review.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Native switch | `class AskmeSystemBars(activity)` with one member, `@JavascriptInterface fun darkGround(dark: Boolean)`, no return value. It posts to `activity.runOnUiThread` (bridge calls arrive on the JavaBridge thread) and sets `isAppearanceLightStatusBars` and `isAppearanceLightNavigationBars` to `!dark`. The system picks icon colour and scrim from these flags, so no deprecated `setNavigationBarColor` call is needed (adr-0023's dex scan still holds). Registered in `onWebViewCreate` under `"AskmeSystemBars"` | `scripts/android-release-lib.sh` seam block |
| Page side | Two constant scripts, `if (window.AskmeSystemBars) { window.AskmeSystemBars.darkGround(true|false); }`. `PracticeCounter` evaluates the `true` one on mount and the `false` one on drop, through the document it captured at mount. The guard makes both a no-op on web, iOS and the debug deploy | `app/src/system_bars.rs`, `app/src/views/ritual.rs` |
| Way back to light | Leaving the Ritual by any route change drops `PracticeCounter`, so `false` runs. A killed or recreated Activity runs `onCreate`, which sets light bars, and the page starts again at Aujourd'hui. Pause and resume keep the window's flags, which still match the screen shown, so no `onResume` hook is added | seam block, `app/src/views/ritual.rs` |
| Security | See below. The pins: two `@JavascriptInterface` members and two `addJavascriptInterface` calls in the seam; `"AskmeSystemBars"` named once; the class takes no `String` and holds no `WebView` | `app/src/safe_area.rs`, `app/src/system_bars.rs`, `scripts/test-shell-units.sh` |

**Threat analysis (security finding S1 re-accepted).** Any script running in the WebView can call `darkGround`. It passes one boolean (the WebView coerces any other JS value to a Java boolean) and gets nothing back: no string is read, no data leaves native code. At worst a script flips the icon contrast of the bars, which is cosmetic, visible to the user and reset by the next Ritual exit or app start. A burst of calls only queues idempotent UI-thread posts, no worse than any busy script. The class holds the Activity so it can reach the window. Page scripts cannot read that field, because only annotated methods are exposed, and the WebView lives inside that Activity, so nothing outlives it. The page origin is still the bundled assets. adr-0024's reopening trigger stands: content we do not own loading in the WebView reopens both bridges. Exported components and permissions do not change.

## Options considered

- **One boolean setter, page to native** — the smallest surface that lets the page say which ground it shows. Native code keeps the colour decision.
- **Turn off only the system contrast layer (`isNavigationBarContrastEnforced = false`)** — rejected: the light band goes, but the dark system icons stay on the dark ground.
- **Native reads the route (a fixed `evaluateJavascript` question on each change)** — rejected: native has no route-change signal, so it would poll or need a push, which brings back the same JS-to-native link with more parts.
- **A colour or string parameter (`setBars("#12303A")`)** — rejected: a string crosses the trust boundary for a two-state choice.

## Consequences

- Good: both bars match the Ritual's ground, and every other screen keeps the light bars it had.
- Bad: the bridge surface is now two interfaces. Every new member is still a new ADR and a security review.
- Bad: if the page reloaded inside a live Activity during a Ritual, the bars would stay dark on Aujourd'hui until the next Ritual exit. wry offers no reload gesture and a renderer crash ends the app, so no handler is added.
- Bad: the debug deploy (`android-deploy.sh`) does not apply the seam, so only the release bundle proves this on a device.
- Reopening trigger: a second method or a non-boolean parameter on `AskmeSystemBars`, a return value, or a third interface.
