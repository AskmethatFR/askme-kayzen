---
id: "adr-0014-view-wiring-click-dispatch-harness"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-19"
relations:
  supersedes:
    # FACET SUPERSESSION — [[adr-0009-quality-gates]] stays `current`. This node annuls
    # exactly ONE of its rationales (L1-bis's « the residue too thin to hide a defect »).
    # Its other live decisions — the scenario gate, the mutation perimeter, the mandatory
    # base-ref, L1–L5, three-cell triangulation — are untouched. Scope also recorded in
    # that node's docs/INDEX.md row.
    - "adr-0009-quality-gates"
  related:
    - "architecture-overview"
  depends-on:
    - "adr-0003-two-crate-workspace"
answers:
  - "Can mutation testing reach the wiring inside a Dioxus view?"
  - "Why is « the onclick residue is too thin to hide a defect » false?"
  - "How does a test drive a real click through a VirtualDom in this repo?"
  - "Which view facts must be pinned by a dispatched click, and which may rest on a render assertion?"
  - "Why does every gesture button carry an aria-label, and how is it composed?"
  - "Why does the harness panic on a second click, and on a duplicated label?"
  - "Does the click harness add anything to the production dependency graph?"
  - "Is the extracted-free-function gesture pattern still the standing answer?"
decided_in:
  - "2026-08-19 slice 7 readmit fix cycle (F2/F3 wiring defects, and the mutation-operator measurement that explains them)"
---

# ADR 0014 — View wiring is pinned by a dispatched click, not by a lint or a render assertion

> **⚠️ Annuls one rationale of [[adr-0009-quality-gates]]** — L1-bis's « the residue too thin to
> hide a defect ». That node stays `current`; only this facet is dead. Its expiry was written
> into it (*« the standing answer until a click-dispatch harness lands »*). It has landed.

> **One-liner**: the statements inside an `onclick` closure, and the per-instance conditionals in
> the render tree, are invisible to **all three** instruments this repo owns — `#[must_use]`, SSR
> render assertions, and the mutation gate. The only instrument that reaches them is a test that
> **dispatches a real click** into a `VirtualDom` and asserts on the re-rendered HTML.
> **Links**: [[adr-0009-quality-gates]] (the gate whose blind spot this measures and closes),
> [[architecture-overview]] (where the pattern is applied).

## Context — the rationale that was refuted by construction

[[adr-0009-quality-gates]] L1-bis established that a gesture written inline in an `onclick` is
unreachable by every gate, and prescribed the fix that still stands: extract it into a
`#[must_use]` free function, where it is ordinary Rust a test calls directly. It then justified
leaving the remaining `onclick` body — a call plus a signal assignment — unpinned: *the residue
too thin to hide a defect*.

**That sentence was refuted by construction.** QA mutated exactly that residue, in
`app/src/views/anchored.rs`, reusing a read model captured *before* the gesture instead of the one
it returned:

```rust
let stale = services.list_anchored_habits.handle();
let (_reloaded, message) = readmit_row(&services, &habit_id);
screen.set(stale);
```

`#[must_use]` is satisfied — the result *is* bound. The HTML is well-formed. `cargo clippy
-D warnings` passed and **all 130 tests passed**, while the screen silently showed a stale Ancrées
list after a readmit. A naiver variant — deleting the `set` outright — was caught only
*incidentally*, by an unused-`mut` warning, which is luck, not coverage.

Two more defects of the same family were then found in production code by the harness this node
records: a refusal message assignment collapsed to `set(None)` (the refusal copy could never
appear — a dead button), and the row-anchoring predicate mutated to `|| true` (the refusal note
leaking onto every anchored row). **Both left the full suite green before a dispatched-click test
existed.** Three defects in one residue is the measure of how thin it was not.

## The measured fact: mutation testing structurally cannot reach view wiring

The blind spot is not a perimeter setting. Dev-B widened `examine_globs` to `app/src/views/**` and
enumerated what cargo-mutants generates there; QA reproduced it independently with `--no-config`.

**All 21 generated mutants are whole-`fn` return-value replacements. Zero target a statement. Zero
descend into a closure.**

An `onclick` body is a closure inside a macro-expanded render tree — it is neither a `fn` the tool
enumerates nor a statement it rewrites. So the defects above are outside cargo-mutants' operator
set **at any perimeter width**: widening the gate to `app/src/**` would generate more mutants and
still not one that touches the wiring. This is the load-bearing fact of the whole node, and it is
what makes a dispatched-click test the *only* available instrument rather than merely a nicer one.

> Do not re-litigate the mutation perimeter on the strength of this. Including `app/src/**` buys
> function-level mutants and no wiring coverage; the exclusion recorded in [[adr-0009-quality-gates]]
> stands on its own reasons.

## Decision

### 1. The harness

`app/src/views/click_harness.rs`, `#[cfg(test)]`-only, exposes a `Screen` that drives a real click:

```
VirtualDom::new(root)
  → rebuild_to_vec()                                       capture the first render's Mutations
  → scan for Mutation::SetAttribute { name: "aria-label", value: Text(t), id }
                                                           t == the wanted label  →  ElementId
  → runtime().handle_event("click", Event::new(data, true), id)
  → render_immediate(&mut NoOpMutations)
  → dioxus_ssr::render(&vdom)                              assert on the re-rendered HTML
```

Event data requires a converter installed in global process state: `dioxus_html`'s
`SerializedHtmlEventConverter`, installed once behind a `std::sync::Once`. It arrives as the
dev-dependency `dioxus-html = { version = "0.7", features = ["serialize"] }`; under `resolver = "3"`
a dev-dependency's features do not unify into the production graph, so **the harness adds nothing to
the shipped binary** — the `app → core` edge and the crate split of [[adr-0003-two-crate-workspace]]
are untouched.

### 2. What must be pinned by a dispatched click

A view fact needs a dispatched click — a render assertion will not do — whenever it is one of:

1. **which** free function an `onclick` calls, and **with what argument**. `#[must_use]` sees that a
   result is bound, never that it is the right result of the right call;
2. **which signal receives that result**. A dropped, misdirected or stale-valued `set` re-renders
   valid HTML over the wrong data;
3. any **per-instance conditional** in the render tree. An assertion over the whole document cannot
   distinguish *« on the row that was acted on »* from *« on every row »*.

Everything a render assertion genuinely discriminates — a button's presence per state, a zone's
absence when empty, a rendered value — stays a render assertion. The dispatched click is for the
wiring, not for re-testing what the free function's own test already covers.

### 3. The `aria-label` convention

Every element a test drives carries `aria_label: "<visible text> · <subject>"`.

**One artifact, two jobs, and that is the point.** It is the element's accessible name: the
accessible name contains the visible label text (WCAG 2.5.3 *Label in Name*, so voice control can
address it), and on a button with no visible text it is the *only* accessible name (WCAG 4.1.2). It
is simultaneously the harness's handle. A handle that exists only for tests rots the moment the
product forgets it; a handle that is also the accessible name is maintained by an accessibility
requirement.

The `· <subject>` half is not decoration — it is what makes the label unique among sibling rows.
Adding it closed a real defect: repeated gesture buttons in a list were previously indistinguishable
to a screen reader, all announcing the same name. The accessibility fix and the harness's
*locate* precondition are the same change.

### 4. The harness's three refusals

The handle stays honest because `Screen` panics rather than degrade:

| It panics on | Because |
|---|---|
| an **unknown** label (listing the available ones) | a renamed label must fail loudly, not silently pass by asserting nothing |
| an **ambiguous** label — two elements sharing one | two elements with one accessible name is an a11y defect *and* an untargetable test; picking the first would hide both |
| a **second click** on the same `Screen` | `ElementId`s are reassigned on every diff, so ids captured at the first render go stale — one `Screen`, one click, then read the HTML |

## Rejected alternatives

| Rejected | Why |
|---|---|
| Widen the mutation perimeter to `app/src/**` and rely on the gate | Measured, not assumed: all 21 mutants there are whole-`fn` return replacements. It cannot reach a closure statement at any width |
| Keep the extraction pattern as the whole answer (status quo) | Three defects shipped through the residue it declared too thin, in one cycle |
| Locate elements by a test-only attribute (`data-testid`) | A handle with no product purpose rots. The `aria-label` is maintained by an accessibility requirement and fixed a real screen-reader defect on the way in |
| Locate by CSS selector / DOM position | Couples every test to markup structure; breaks on a wrapper element, and cannot distinguish sibling rows any better than the render assertion that already failed to |
| A full browser / WebDriver end-to-end layer | A dependency and a runtime this repo has no other use for, to reach facts a `VirtualDom` in-process reaches in milliseconds |

## Consequences / Constraints

- **MUST**: the extracted-free-function pattern **stands** — a gesture is application logic that
  happens to be triggered by a click, and belongs where a test can call it directly. This node
  removes its *justification for stopping there*, not the pattern.
- **MUST**: any element a test drives carries the `<visible text> · <subject>` `aria-label`, and
  no two elements on a screen share one. The ambiguity panic enforces it at test time.
- **MUST**: treat a statement inside an `onclick`, or a per-instance conditional in the render tree,
  as **unmeasured until a dispatched click pins it**. No gate will announce the gap.
- **MUST NOT**: read a green mutation campaign as a statement about view wiring. It is silent there
  by operator set, not by perimeter — [[adr-0009-quality-gates]] L1-bis's remaining limits, plus
  this one.
- A view-only diff therefore generates zero mutants. That is the expected shape here, not a signal
  about the change's quality.
