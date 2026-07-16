---
id: "adr-0004-routing-flat-enum"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-16"
relations:
  related:
    - "architecture-overview"
    - "design-ecrans"
  depends-on:
    - "adr-0003-two-crate-workspace"
answers:
  - "How is routing structured in kayzen-app (flat enum vs nest/layout)?"
  - "Why are URL paths English while screen titles are French?"
  - "Why is the `:id` route segment a String and not a Uuid?"
  - "How do agents navigate between screens (explicit Link vs go_back)?"
  - "Does dioxus-router disambiguate sibling routes by declaration order?"
  - "What Android decisions were taken now, and what is deferred to the mobile-shell ticket?"
decided_in:
  - "LOCAL-4"
---

# ADR 0004 — Routing: flat `Route` enum mirroring the designer's screen map

> **One-liner**: `kayzen-app` gets a single flat `#[derive(Routable)]` `Route` enum — six variants mirroring the designer's six screens ([[design-ecrans]]) plus a `NotFound` catch-all — with English URL paths, `String` ids, a `views/` folder of seven stub screens, **no** `#[layout]`/`Outlet`, and navigation exclusively via explicit `Link { to: Route::X }`.
> **Links**: [[architecture-overview]] (where applied), [[adr-0003-two-crate-workspace]] (the app crate this lives in), [[design-ecrans]] (the screen map the enum mirrors).

## Context

Since [[adr-0003-two-crate-workspace]], `app/src/main.rs` was a single-component Dioxus placeholder. The designer's prototype defines six screens with transitions ([[design-ecrans]]); before any screen gets real content, the app shell needs a navigation skeleton so each future screen lands as its own vertical slice. **Target context (human-stated)**: the first shipping target is **Android**; all development happens on the web platform for speed. Routing choices must therefore behave identically under web history and a future Android WebView/native shell, and keep deep links possible.

Commits `2e4a148` (RED — failing precedence/display/parse tests) / `a3bbe28` (GREEN — Ritual path fix) / `7811375` (feat — Router wired, Today body moved, five screens stubbed). Core untouched (16/16 green); app carries 4 route tests.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Route shape | One **flat** `Route` enum, `#[derive(Routable, Clone, Debug, PartialEq)]`, six screen variants + `NotFound` catch-all (`/:..segments`) | `app/src/route.rs` |
| Paths | **English** URL paths: `/`, `/habit/:id`, `/habit/:id/ritual`, `/week`, `/anchored`, `/add` | `app/src/route.rs` |
| `:id` type | `String` — **human-approved (Q1)**: Uuid typing is deferred to the core-wiring boundary, where the id will be converted once at the edge | `app/src/route.rs` |
| Views | `views/` folder — 7 view components (one per variant) + `mod.rs`; screens are **stubs** carrying the designer's **French titles** — **human-approved (Q2)** | `app/src/views/` |
| Layout | **No** `#[layout]` / `Outlet` — the design has no shared chrome today | `app/src/route.rs` |
| Navigation | Explicit `Link { to: Route::X }` **only** — never `go_back()`. Explicit targets give **identical history behavior on web and mobile** (Android-motivated) | future views |
| Router placement | `Router::<Route> {}` inside `App` in `main.rs`; `document::Link` asset tags stay **above** the Router | `app/src/main.rs` |
| Deep links | URL-shaped, stable paths keep Android **App Links / deep links** possible later; no intent-filter work now | — |

**Human-validated contextual choices**: `String` id (Q1); `views/` jumpstart with French designer titles (Q2); Android-first target with web as dev platform.

**Explicitly DEFERRED to a future `mobile-shell` ticket** (do not build speculatively): Android hardware back-button wiring; intent-filters / App Links registration.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| `#[nest]` for the `/habit/:id` sub-tree | Ceremony for 2 routes; flat variants with full paths are simpler and equally testable |
| `#[layout]` + `Outlet` now | No shared chrome exists in the design — YAGNI; additive to introduce when chrome appears |
| Board index or `Uuid` as `:id` | Index is fragile under reordering; `Uuid` in the route couples the URL layer to core types before core wiring exists — convert at the boundary later (Q1) |
| French URL paths | Encoding friction (accents) and the code surface is English; French belongs to what the user *sees* (titles), not the URL |
| Real screen content in this cycle | Each screen is a future vertical slice per the designer's build order; stubs keep this slice bounded to navigation |

## Consequences / Constraints

- **MUST**: navigate with explicit `Link { to: Route::X }` — never `go_back()` — so web and Android history behave identically.
- **MUST**: keep URL paths English and stable — they are the future Android App-Links surface.
- **MUST**: convert `:id` `String` → typed id (`HabitId`/Uuid) **once, at the core-wiring boundary** — views never parse ids ad hoc.
- **MUST**: keep `document::Link` asset tags above `Router` in `App` (`app/src/main.rs`).
- **Router-macro precedence (verified fact — dev discovered, Dev-B independently confirmed)**: `dioxus-router-macro` 0.7.9 does **NOT** disambiguate two sibling routes by declaration order. Each generated parser rejects non-exhausted remainders, and `sort_ids` only orders Static < Route < CatchAll — so `/habit/:id/ritual` vs `/habit/:id` cannot shadow each other regardless of enum order, and catch-alls always sort last. Do not reorder variants "for precedence"; it is a no-op. Precedence is pinned by test (`precedence_ritual_over_habit_detail`).
- **Known debt / watch items**:
  - **LOW**: `not_found.rs` takes a `segments` prop it does not use — required by the catch-all macro contract; revisit if the macro relaxes.
  - **MEDIUM, fix at core-wiring time**: pre-existing stale-`done` closure bug in `today.rs` toggle — a double rapid click before re-render uses the stale captured value.
  - **LOW, recurring (Security)**: `cargo audit` / `cargo-deny` still absent from CI (carried from [[adr-0003-two-crate-workspace]]).
- **Out of scope**: screen content (each screen = its own future slice), core wiring, `mobile-shell` concerns (hardware back, intent-filters), shared layout chrome.
