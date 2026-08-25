---
id: "adr-0018-i18n-fluent-catalogue"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-25"
relations:
  related:
    - "architecture-overview"
    - "adr-0009-quality-gates"
    - "adr-0014-view-wiring-click-dispatch-harness"
    - "adr-0017-platform-location-adapter"
  depends-on:
    - "adr-0010-crate-boundary-trust-boundary"
answers:
  - "Why a Fluent catalogue and not the far more popular alternative?"
  - "Which crate owns user-facing language, and why does the core stay out of it?"
  - "Why are catalogues embedded in the binary rather than loaded from a directory?"
  - "The same count reads plural in one language and singular in the other — bug or decision?"
  - "Why does the seam rewrite what the formatter returns before any view sees it?"
  - "A key is missing or unresolvable at runtime — does the screen degrade or die?"
  - "What actually guards a translation, given that a wrong-language key renders green?"
  - "Where does the device's locale come from, and which part of that path no test reaches?"
decided_in:
  - "#10 — 2026-08-25, French and English catalogues (PR #42)"
---

# ADR 0018 — User-facing language is an embedded Fluent catalogue owned by the app crate, reached through a seam that normalises what the formatter returns

> **One-liner**: Copy moves out of the views into **Fluent catalogues embedded at compile time** and read through a **single seam** (`app/src/i18n/mod.rs`). Fluent is chosen over the more popular alternative on two acceptance criteria — **real CLDR plural rules** and a **reactive locale** — because a language switcher that does not re-render is not a switcher. The catalogue is a **presentation concern and stays in `kayzen-app`**: [[adr-0010-crate-boundary-trust-boundary]] holds unchanged, the core remains language-free. The seam exists because the formatter's raw output is not what a view wants: **bidi isolates are stripped once**, and an unresolvable key **panics rather than degrading** — a property of the dependency that dictates the guard strategy.
> **Links**: [[adr-0010-crate-boundary-trust-boundary]] (the boundary this decision respects rather than moves — that node is untouched), [[adr-0014-view-wiring-click-dispatch-harness]] (whose `aria-label` lookups the raw formatter output would have broken — applied unchanged), [[adr-0009-quality-gates]] (the mutation perimeter that does not reach this layer, which is *why* the guards below are hand-built invariants), [[adr-0017-platform-location-adapter]] (the adapter-holds-no-policy rule, applied here to locale detection at zero cost), [[architecture-overview]] (the single home for what stays open).

## Context

Every screen carried its French copy inline. The requirement was a second language now and an in-app language switcher later, under an acceptance criterion that the **French copy stay byte-identical** to what already shipped.

The graph settled where a presentation concern lives ([[adr-0010-crate-boundary-trust-boundary]]) but was silent on two questions: which catalogue technology, and what a view is allowed to see of it.

## Decision

| Facet | Decision |
|---|---|
| Catalogue technology | **`dioxus-i18n` (Fluent / `fluent-bundle`)**. The alternative weighed — `rust-i18n`, roughly ten times the adoption — fails two acceptance criteria: its `count` is an **ordinary format variable, not a CLDR plural selector**, and its global locale is **not reactive**, so a locale change would not re-render. The second is decisive only because an in-app switcher is a planned follow-up: popularity does not substitute for the two properties actually required |
| Where the layer lives | **`kayzen-app` only.** Language is presentation; the core stays language-free and keeps returning identifiers and values, never sentences. [[adr-0010-crate-boundary-trust-boundary]] is **applied, not extended** — a catalogue behind the boundary would put copy where nothing renders it |
| How catalogues reach the binary | **Embedded with `include_str!`** — a **forced constraint, not a preference**: the library's directory loader is `#[cfg(not(target_arch = "wasm32"))]` and this app targets wasm. Favourable side effect on the Android arm: a catalogue that is part of the binary is nothing extra to bundle, and nothing extra that can be absent at runtime |
| What a view is allowed to see | **Only the seam's output** (`app/src/i18n/mod.rs`). No view calls the formatter directly. The seam is what makes the two corrections below single-site facts instead of per-call-site discipline |
| Bidi isolates | **Stripped once, at the seam.** The bundle never disables `use_isolating`, so every interpolated placeable comes back wrapped in U+2066..U+2069 — correct for a catalogue mixing scripts, noise for an `fr`/`en` app. Left in place it would silently break **every interpolated assertion** in the suite and [[adr-0014-view-wiring-click-dispatch-harness]]'s `aria-label` lookups, which locate an element by the exact text a user reads |
| A missing or unresolvable key | **Panics; it does not degrade.** The library discards the `{$var}` fallback as soon as resolution produces errors, so there is no placeholder render to fall back on. This is a **property of the dependency, established by reading its source** — not a policy this project chose and could relax. Its consequence is the guard strategy, not a runtime handler |
| What guards a translation | A **layered rule, because the interesting failure is not a missing key** — a missing one panics loudly on the first render that reaches it. The failure that ships is a **present key holding the wrong language**: it renders, it reads green, and no assertion about *French* copy notices. Hence: **static invariants over the catalogues themselves** — parsed at test time, so they hold for every message including the ones no fixture reaches (the two catalogues define the same ids; each id references the same variables in both; no message is byte-identical across them) — **plus per-screen render assertions in the second language**, which cover only the states a fixture actually reaches and are therefore the *narrower* of the two layers, never the exhaustive one |
| Why those invariants are hand-built | The mutation perimeter deliberately excludes this crate ([[adr-0009-quality-gates]], applied unchanged). Nothing measures whether a catalogue guard discriminates, so the guards are **structural properties of the catalogue files** rather than assertions about rendered values — a class of check that cannot pass vacuously |
| Locale detection | **A pure decision plus an adapter**, exactly [[adr-0017-platform-location-adapter]]'s shape applied at no cost: the *choice* is a pure function over the reported tag (`app/src/i18n/locale_choice.rs`) with one fallback rule and no platform call, host-tested across the tag shapes a device can report; **reading** the device is the adapter, and is deliberately **not** wrapped in a seam of its own |
| What that leaves untested, stated plainly | The single line that asks the platform for its tag is **verified on-device, not by the suite** — wrapping it would buy a test of the wrapper, not of the platform. The wasm arm additionally depends on a platform-conditional feature flag to read the browser's locale at all; that is a build fact, not a decision |
| Plural at zero | **The two languages diverge, deliberately.** CLDR classes zero as `one` in French and `other` in English. The pre-i18n screens already read as plural at zero in French, and byte-identical French copy was an acceptance criterion — so a French plural message that must read plural at zero **carries an explicit `[0]` variant**, and its English counterpart does not. **Blindly matching CLDR here would have changed shipped copy**: a catalogue is bound by the copy that shipped, not only by the rules of the language |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| `rust-i18n`, on adoption | Adoption is not one of the acceptance criteria. It has no real CLDR plural rules and no reactive locale — the two that were |
| Hand-rolled `match locale { … }` over string constants | Re-implements plural category selection, which is exactly the part that is hard and language-specific, and pushes it into every call site |
| Load catalogues from a directory at startup | Not available on the wasm target at all (`#[cfg]`-gated out), and on the other targets it would make a translation a runtime asset that can go missing — a second failure mode for zero benefit |
| Move the catalogue into the core crate so both crates can phrase messages | Puts copy where nothing renders it and makes the core's outputs language-bound, reopening [[adr-0010-crate-boundary-trust-boundary]] to buy nothing |
| Strip isolates at each call site (or assert against them in tests) | Spreads a single library fact across every view and every assertion, and makes the next call site a fresh chance to forget. It is one property of the formatter, so it is corrected in one place |
| Render a placeholder for an unresolvable key instead of panicking | Not reachable: the library discards its own fallback once resolution errors. A placeholder would have to be built by pre-checking every key before formatting — paying at every render for what a static catalogue invariant proves once |
| Rely on per-screen render assertions alone | They only cover the states a fixture reaches, which is a subset nobody can enumerate reliably — the exhaustive guard has to read the catalogues, not the screens |
| Match CLDR at zero in both languages | Would have rewritten shipped French copy to satisfy a rule the product never asked for, against an explicit acceptance criterion |
| Wrap the platform locale call in a port to make it testable | Tests the wrapper, not the platform. [[adr-0017-platform-location-adapter]] already settled that the policy — not the platform call — is what belongs in host-tested code, and the policy here is already pure |

## Consequences / Constraints

- **MUST**: keep every user-facing string in a catalogue and reach it only through the seam. A literal sentence in a view is a defect, not a shortcut.
- **MUST**: keep catalogues embedded at compile time — the wasm target admits nothing else.
- **MUST**: keep the language layer inside `kayzen-app`. **MUST NOT** let a core type carry a translated sentence.
- **MUST**: hold the catalogues to the static invariants above whenever a message is added, changed, or removed — they are the only exhaustive guard, because the mutation gate does not reach this crate.
- **MUST**: keep the locale *decision* pure and host-tested, and keep the platform call free of policy.
- **MUST NOT**: present a green suite as evidence that the device-locale read works — that arm is verified on a device ([[architecture-overview]]'s bar for an arm no runner executes).
- **MUST NOT**: normalise the zero case to CLDR without a copy decision. A divergence between catalogues at a plural boundary is a product fact, and changing it changes what shipped.
- **Deferred work and residual risk** arising from this decision live in **one place** — [[architecture-overview]]'s *Open questions / Gaps*. They are not restated here.
