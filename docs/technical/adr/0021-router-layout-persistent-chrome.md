---
id: "adr-0021-router-layout-persistent-chrome"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-09-19"
relations:
  supersedes:
    # FACET SUPERSESSION — [[adr-0004-routing-flat-enum]] stays `current`. Its DECISION
    # (one flat Route enum, English paths, String id, explicit Link navigation) is
    # untouched and every node still depends on it. Exactly ONE facet is dead:
    # « Layout: **No** `#[layout]` / `Outlet` — the design has no shared chrome today »,
    # whose own escape clause (« additive to introduce when chrome appears ») is what
    # fires here. Scope also recorded in that node's docs/INDEX.md row.
    - "adr-0004-routing-flat-enum"
  related:
    - "architecture-overview"
    - "design-ecrans"
    - "design-style-galets"
    - "adr-0014-view-wiring-click-dispatch-harness"
    - "adr-0018-i18n-fluent-catalogue"
  depends-on:
    - "adr-0003-two-crate-workspace"
decided_in:
  - "issue-55"
answers:
  - "How is chrome that outlives a screen mounted, and why not once per view?"
  - "Why is the bottom bar a sibling of `.screen` rather than a child?"
  - "Why does the layout enumerate its screens instead of testing the route?"
  - "How is the current destination marked, and why is there no route matching of our own?"
  - "Why do the bar's label keys carry no `-label` suffix?"
  - "Why does the bar not participate in the daily-life gesture count?"
---

# ADR 0021 — Persistent chrome is mounted by a router layout, not by each view

> **One-liner**: the Galets bottom bar (issue #55) is mounted **once**, by `#[layout(MainScreenFrame)]` wrapping the three main screens, so a screen that must *not* show it is a focus route left **outside** the layout — the absence is a property of the route table, not of each view.
> **Links**: [[adr-0004-routing-flat-enum]] (one facet revoked here), [[architecture-overview]] (where applied), [[design-style-galets]] (the visual decision), [[adr-0014-view-wiring-click-dispatch-harness]] (the `aria-label` convention reused as the test handle).

## Context

The « Galets » redesign (issue #55, slice 1) gives the three main screens — Aujourd'hui `/`, Cette semaine `/week`, Ancrées `/anchored` — a navigation bar fixed at the bottom, while a habit's detail (`/habit/:id`), a ritual (`/habit/:id/ritual`) and adding a habit (`/add`) keep the whole screen to themselves.

[[adr-0004-routing-flat-enum]] settled a **flat** `Route` enum with no `#[layout]`/`Outlet`, on the ground that no shared chrome existed — with the explicit escape clause « additive to introduce when chrome appears ». Chrome appears here, and with it a second question adr-0004 did not have to answer: three routes must carry the bar and three must not.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Mounting | `#[layout(MainScreenFrame)] … #[end_layout]` wraps **Today, Week, Anchored only**; the three focus variants and `NotFound` stay outside | `app/src/route.rs` |
| Variant order | Layout variants are **contiguous** (the macro requires it) — the enum is reordered; paths, variant names and `#[rustfmt::skip]` unchanged | `app/src/route.rs` |
| Precedence | Unaffected by the reorder — route disambiguation is by static/route/catch-all kind, not declaration order (adr-0004, guarded by `precedence_ritual_over_habit_detail`) | `app/src/route.rs` tests |
| Composition | `MainScreenFrame` renders `div.main-screen-frame { Outlet::<Route> {}, BottomNav {} }` — **content first, bar second**: reading and tab order, and the bar paints above without a `z-index` | `app/src/views/main_screen_frame.rs` |
| Ancestor | The bar is a **sibling** of `.screen`, never a descendant: `.screen` animates `transform` (`kzUp`), and a transformed ancestor captures `position: fixed` | `app/assets/main.css` |
| Current page | The router `Link`'s own `aria-current="page"` (emitted when its href equals the current URL) plus a CSS weight cue — **no** `active_class`, `use_route` or route `match` of our own | `app/src/views/bottom_nav.rs` |
| Labels | Seven new Fluent keys, label keys **without** a `-label` suffix — the pair invariant in `app/src/i18n/mod.rs` binds `X-label` to an `X-aria` reading `"{label} · {title}"`, and these arias carry no `$title` | `app/src/i18n/{fr,en}.ftl` |
| Tests | `aria_label: "<visible text> · <subject>"` per [[adr-0014-view-wiring-click-dispatch-harness]], doubling as the dispatched-click handle; S1 pinning is on the **visible nodes**, by href, never on attribute text | `app/src/views/bottom_nav.rs` tests |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Mount the bar in each of the three views | Three edits that can drift, and the absence on focus screens becomes a per-view discipline instead of a route-table property. `S4` holds by construction only in the layout form |
| Test the route inside the layout (`match` / `use_route`) | The router `Link` already publishes the current page and re-renders on route change; a second source of truth about « where am I » is exactly the state that drifts |
| `active_class` on `Link` | One cue, colour-shaped, and it would not announce anything. The weight cue is the non-colour half, `aria-current` the announced one |
| Fluent message references (`{ bottom-nav-today } · navigation`) | The catalogue's own pairing invariant demands the literal shape; references would have to be special-cased out of it |
| A `z-index` on the bar | Unnecessary once the bar is a sibling of the transformed element — reaching for stacking order would hide the ancestor problem rather than avoid it |
| A CSS-reading test | Owner ruling: CSS carries no `include_str!` test. The `--bottom-nav-height` arithmetic that keeps content clear of the bar is carried by a comment above the rule |

## Consequences / Constraints

- **MUST**: a new screen that should show the bar goes **inside** the layout; a focus screen stays **outside** it. The route table is the single place where that is decided.
- **MUST NOT**: nest the bar inside any ancestor carrying a `transform` (`.screen` today) — `position: fixed` is captured silently, with no test able to see it.
- **MUST**: keep the bar out of the daily-life gesture count. `today-habit-list/S4` (« the add-habit gesture is the only interactive element ») is **scoped to the screen's content** by owner ruling: the bar is app chrome, not a board gesture, and the assertion counts only what renders before the bar.
- **MUST**: pin visible text by its node (`>{label}</span>`), never by a first occurrence of the label string — the string also occurs in the `aria-label` attribute, which precedes it in the markup.
- **Known limits**: `env(safe-area-inset-*)` resolves to `0` in Chromium, so the notched-device path is unverified; the on-screen-keyboard interaction is unverified. Hover styles and the rest of the Galets restyling are out of this slice.
- **Facet revoked**: [[adr-0004-routing-flat-enum]]'s « no `#[layout]`/`Outlet` » — its escape clause fires; that node stays `current`.

## Open questions / Gaps

- [ ] Where the bar's own restyling (hover, motion) lands — deferred with the rest of Galets.
- [ ] Whether a fourth destination is ever added: the S1 assertions enumerate exactly three, and a fourth would have to update them deliberately.
