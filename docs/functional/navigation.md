---
id: "navigation"
type: "functional"
owner: "pm"
status: "current"
updated: "2026-09-19"
relations:
  related:
    - "design-ecrans"
    - "design-style-galets"
    - "feature-catalog"
    - "glossary"
    - "adr-0021-router-layout-persistent-chrome"
answers:
  - "Which screens carry the bottom bar, and which keep the whole screen?"
  - "What are the bar's destinations, and in what order?"
  - "How is the current destination marked?"
  - "Do the in-page links stay, now that a bar exists?"
  - "Does the bar count as a gesture of the daily life?"
---

# Navigation — the bar at the bottom of the three main screens

> **One-liner**: the three main screens carry a bar at the bottom offering Aujourd'hui, Semaine and Ancrées; the three focus screens keep the whole screen; the in-page links that already existed stay exactly where they were.
> **Links**: [[design-ecrans]] (the screen map), [[design-style-galets]] (the visual world), [[adr-0021-router-layout-persistent-chrome]] (how it is mounted).

## What it is

The bar is **app chrome**, not a gesture of the daily life. It is a first slice of the « Galets » redesign (issue #55): it changes how the user moves between screens and nothing else.

- **With a bar** — Aujourd'hui `/`, Cette semaine `/week`, Ancrées `/anchored`.
- **Without a bar** — a habit's detail `/habit/:id`, a ritual `/habit/:id/ritual`, adding a habit `/add`.

Three destinations, in this order: **Aujourd'hui**, **Semaine**, **Ancrées**. Each carries its icon (sun, curve, pebble) and its label.

## Rules

- The current destination is marked **twice over**: the router announces it as the current page to assistive technology, and the bar carries that mark in weight and colour — never colour alone (design rule #32, carried by [[design-style-galets]]).
- The bar speaks the user's language: the three labels come from the same catalogue as the rest of the app.
- The bar never hides content: the screen keeps enough room at the bottom to scroll its last element clear of the bar.
- The bar is **not** a gesture: « the add-habit gesture is the only interactive element » on an empty board is read as the **content's** only interactive element (owner ruling, issue #55).

## What this slice does not change

The in-page links stay: Today's footer link, Today's add call-to-action (both on an empty board and under a list), and Ancrées' « ← Aujourd'hui ». They are what the bar will eventually make redundant; that removal is not this slice.

## Acceptance

The five scenarios live in `docs/functional/features/navigation/bottom-nav.feature` (`@feature:bottom-nav`): the bar shows on the three main screens · the current screen is marked by more than colour · a destination opens its screen · focus screens keep the whole screen · the bar speaks the user's language.

## Open questions / Gaps

- [ ] Whether the in-page links retire once the bar has proven itself (a later slice, with the rest of Galets).
- [ ] Hover and motion styling of the bar.
- [ ] Notched-device safe-area behaviour and the on-screen keyboard — unverifiable in the suite, unverified on device.
