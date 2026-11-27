---
id: "adr-0027-android-back-goes-up-one-level-by-asking-the-page"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-11-27"
relations:
  supersedes:
    # FACET SUPERSESSION — [[adr-0024-safe-area-css-bridge]] stays `current`. Exactly
    # ONE facet is dead: the seam evaluating a single script (the grammar-closed
    # insets push). The seam now also evaluates one fixed Back question. The
    # `AskmeSafeArea` bridge itself (one object, one 0-arg method, no references)
    # is untouched.
    - "adr-0024-safe-area-css-bridge"
  related:
    - "adr-0021-router-layout-persistent-chrome"
    - "adr-0023-platform-behaviour-seam-patched-kotlin"
    - "adr-0004-routing-flat-enum"
answers:
  - "What does Android Back do inside the app, and why not go back through history?"
  - "How does the native shell learn whether the page handled Back?"
  - "What happens when the page does not answer, or answers anything but true?"
decided_in:
  - "S17 (FU1) — 2026-11-27, Android Back goes up one level"
---

# ADR 0027 — Android Back goes up one level by asking the page

> **One-liner**: An `OnBackPressedCallback` in the `MainActivity` seam runs the fixed script `window.__kzBack()` and reads a true/false answer. The page answers from the route table: a ritual goes up to its habit's detail, every other screen goes up to Aujourd'hui, and Aujourd'hui has no parent, so it answers false and Back goes to the system, which leaves the app.
> **Links**: [[adr-0024-safe-area-css-bridge]] (one facet replaced: the seam evaluated one script only), [[adr-0023-platform-behaviour-seam-patched-kotlin]] (native behaviour lives in the seam patch, applied here), [[adr-0021-router-layout-persistent-chrome]] (the route table decides chrome; it now decides the level up too).

## Context

Android Back left the app from every screen. wry's `WryActivity.onKeyDown` goes back only when `WebView.canGoBack()`, but Dioxus routes on a `MemoryHistory`, so the WebView history is always empty. With `targetSdk = 36`, Android 16 no longer sends `KEYCODE_BACK` to `onKeyDown` at all. The app's own way-back links push new entries (`BackLink`, the ritual's « Arrêter »), so router history Back would bring a stopped ritual back. Android's navigation principles say Up and Back match inside the app.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Level up | `back_target(route)`: `Today` → none; `Ritual { id }` → `HabitDetail { id }`; every other variant → `Today`. An exhaustive `match`, so a new route must pick its parent | `app/src/route.rs` |
| Page side | The outer router layout `BackKey` evaluates `BACK_JS` once. The script defines `window.__kzBack()`, keeps `canGoUp` (false until told), and never returns, so its channel stays open. `BackKey` tells it `back_target(current).is_some()` on every route render; a press sends one message and the layout pushes the level up | `app/src/back_key.rs` |
| Native side | In `onWebViewCreate`, `onBackPressedDispatcher.addCallback(this, OnBackPressedCallback(true))`. The callback runs `webView.evaluateJavascript("window.__kzBack()")` | `scripts/android-release-lib.sh` seam block |
| Fail closed | Any answer but the JSON `true` (false, `null` when the page is not ready, an error) disables the callback, re-dispatches Back to the system with `onBackPressedDispatcher.onBackPressed()`, then re-arms it for when the user returns. The re-dispatch is synchronous and skips the disabled callback, so it cannot loop | seam block |
| Security | The script is a constant string with no interpolation; the only value read back is compared to `"true"`; no `@JavascriptInterface` is added. Any page script can call `__kzBack()`: at worst it moves the app up one level, which the user can do themselves. An overwritten `__kzBack` function would swallow Back (a script could make it always answer `true`), so it is defined once, non-writable and non-configurable, by `Object.defineProperty` behind a has-own guard (`BackKey` mounts once as the outer layout; a second eval keeps the first function); the loop sits in `try`/`finally` so a rejected or ended `dioxus.recv()` resets `canGoUp` to false. Back then fails closed and exits the app from every screen until the page reloads, because nothing re-arms the first closure's `canGoUp`; `BackKey` mounts exactly once, in the outer layout. `BACK_JS` is pinned as one exact string. The pins: two `evaluateJavascript(` calls in the seam, `__kzBack` named once, one `addCallback(` | `app/src/back_key.rs`, `app/src/safe_area.rs`, `scripts/test-shell-units.sh` tests |

## Options considered

- **Ask the page, which goes up one level** — Back and Up match, a ritual cannot come back after « Arrêter », and Aujourd'hui leaves the app as users expect.
- **Back through router history** — rejected: the back-style links push entries, so history Back replays screens the user already left.
- **`enableOnBackInvokedCallback=false` plus wry's `onKeyDown`** — rejected: the WebView history is empty, so `canGoBack()` is always false.
- **JNI from Rust to the dispatcher** — rejected for the reason [[adr-0023-platform-behaviour-seam-patched-kotlin]] gives: runtime-resolved signatures no gate compiles.

## Consequences

- Good: one pure function decides the hierarchy and is tested on every route variant (`system-back` S1, S2).
- Good: a page that is not ready, or a broken script, still lets Back leave the app.
- Bad: the native side waits for an async script answer on each press; device testing is the only proof that a Dioxus eval channel keeps working after its script is set up.
- Bad: the debug deploy (`android-deploy.sh`) does not apply the seam, so Back works only in the release bundle.
- Bad: the callback stays enabled, so Android's predictive back-to-home animation never previews on Aujourd'hui; Back still leaves the app.
- Reopening trigger (with [[adr-0024-safe-area-css-bridge]]'s): a third script in the seam, a script built from data, or an answer read as anything but a boolean.
