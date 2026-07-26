---
id: "adr-0010-crate-boundary-trust-boundary"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-26"
relations:
  related:
    - "architecture-overview"
    - "adr-0004-routing-flat-enum"
    - "adr-0009-quality-gates"
  depends-on:
    - "adr-0001-validation-by-construction"
    - "adr-0003-two-crate-workspace"
    - "adr-0006-cqrs-light"
answers:
  - "Where is this application's trust boundary — the view, or the crate?"
  - "Why does a raw URL segment reach a core use case as a `&str` instead of being parsed in the view?"
  - "Why was `impl From<&str> for HabitId` deleted rather than made length-bounded?"
  - "Why is `HabitId::new` fallible, why 1..=64, and why does it not trim?"
  - "Why is there no charset restriction on a habit id, and what forces one to be added?"
  - "Why do a malformed id and an unknown id produce the same result?"
  - "Which routes are NOT covered by the id length bound, and why is that accepted?"
decided_in:
  - "2026-07-26 HabitId parse-at-the-boundary cycle (GATE 1.5 human approval)"
---

# ADR 0010 — The crate boundary is the trust boundary: `HabitId` parsed once, at the core's entry points

> **One-liner**: The anticorruption layer of this system is the **`kayzen-core` crate boundary** — the entry point of a use case or a query, primitives in and DTOs out — **not** the Dioxus view. A raw URL segment therefore travels as `&str` up to that entry point, where the single fallible constructor `HabitId::new(&str) -> Result<HabitId, HabitError>` parses it; the infallible `impl From<&str> for HabitId` is deleted, so there is exactly **one door**.
> **Links**: [[adr-0006-cqrs-light]] (the MUST that makes the crate boundary an ACL: `kayzen-app` never imports a domain type), [[adr-0003-two-crate-workspace]] (the compiler-enforced one-way edge that makes it structural rather than conventional), [[adr-0001-validation-by-construction]] ("the domain speaks `HabitId`, never a raw `String`" — this ADR is what makes that sentence true), [[adr-0004-routing-flat-enum]] (which said the `:id` `String` is converted to a typed id "once at the core-wiring boundary" — this ADR settles *where* that once is), [[adr-0009-quality-gates]] (the gate that cannot measure this constructor — see its `fn new` blind-spot facet).

## Context

Security audited the `GetHabitDetail` read path and filed a **low CWE-20** finding: `handle(&str)` took a raw URL path segment (`/habit/:id`) and converted it with an infallible `impl From<&str> for HabitId`, so an unvalidated, unbounded `String` entered the domain — flatly contrary to [[adr-0001-validation-by-construction]]'s "the domain speaks `HabitId`, never raw `String`".

Security's own remedy was to **parse in the Dioxus view**, at the URL boundary, on the classic "validate as early as possible" reflex. That remedy was **rejected by the Architect and the rejection approved by the human** (GATE 1.5, 2026-07-26). The reasoning, which is the substance of this ADR:

> The trust boundary of this application is **the crate boundary, not the view component**. [[adr-0006-cqrs-light]] states a MUST — "`kayzen-app` never imports a domain type" — and [[adr-0003-two-crate-workspace]] makes it compiler-enforced. A query's or use case's entry point, primitives in and DTOs out, is therefore *structurally* the system's anticorruption layer. Parsing in the view would create a **second** place that fabricates `HabitId`s, inside the crate that must not import the type.

Security **re-reviewed the delivered code and retracted its own recommendation**, concluding it would have *weakened* the property gained: it found no attack path the crate-boundary placement leaves open, and recorded that *"la porte unique est une propriété de sécurité plus forte que la proximité du contrôle avec l'URL"*.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Where the trust boundary is | The **`kayzen-core` crate boundary** — every use case's and query's entry point (`&str`/`String`/`u32` in, DTO or domain error out). That entry point is the anticorruption layer; nothing upstream of it is trusted, nothing downstream of it re-validates | `core/src/habit_management/queries/get_habit_detail.rs` |
| One door | `HabitId::new(&str) -> Result<HabitId, HabitError>` is the **only** way to obtain a `HabitId`. `impl From<&str> for HabitId` is **deleted** — an infallible constructor is a second door that structurally cannot refuse | `core/src/habit_management/domain/habit_id.rs` |
| The bound | `MIN_LEN = 1`, `MAX_LEN = 64` (`1..=64`). 64 confirmed by the human (Q1) — comfortably above any generated id shape, low enough to bound memory | `core/src/habit_management/domain/habit_id.rs` |
| **No trim** | Deliberate human decision (Q2): an id padded with spaces **is a different id**. Normalising would silently change repository lookup semantics — unlike `HabitTitle`, where trimming serves a human-typed value | `core/src/habit_management/domain/habit_id.rs` |
| Error shape | `HabitError::IdLength { min, max }`, mirroring the existing `TitleLength`. One error family per aggregate — no separate `HabitIdError` | `core/src/habit_management/domain/habit.rs` |
| Refusal rides the existing failure path | Each of the three production sites absorbs the refusal through the failure it already owned: `None` (read), `MarkDoneError::HabitNotFound`, `HabitBoardError::InvalidHabit`. **No public signature changed** | `core/src/habit_management/queries/get_habit_detail.rs`, `core/src/habit_management/use_cases/mark_done.rs`, `core/src/habit_management/use_cases/request_habit.rs` |
| Malformed ≡ unknown | A malformed id and an unknown id collapse onto the **same** fallback. Deliberate: the UI never discloses which id shapes are valid, so the failure path leaks nothing about the id space | `core/src/habit_management/queries/get_habit_detail.rs` |
| Views never parse | `kayzen-app` production code must never construct a domain type — including `HabitId`. The view forwards the raw route parameter; the core refuses it. (Test fixtures under `#[cfg(test)]` in the app crate *do* build domain objects to seed a store — that is fixture wiring, not a production door) | `app/src/views/habit_detail.rs` |
| The generated id is parsed too | `RequestHabit` parses the `GuidGenerator` output through the same door rather than trusting it. The generator is infrastructure; the boundary does not make exceptions for friendly callers | `core/src/habit_management/use_cases/request_habit.rs` |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| **Parse in the Dioxus view** (Security's original remedy) | Creates a second `HabitId` fabrication site inside the crate forbidden by [[adr-0006-cqrs-light]] from importing the type. Security re-reviewed the delivered code and **withdrew this recommendation itself**: the single door is a stronger security property than proximity of the check to the URL |
| **Bound the length inside `impl From<&str>`** (keep the infallible conversion, just clamp it) | `From` is **infallible by Rust contract**, so bounding without the ability to refuse forces one of two failures: **truncation** — two distinct ids silently aliasing, which in a repository keyed on id equality means habit A's URL resolving to habit B, an IDOR handed over ready-made the day multi-user arrives — or a **panic**, a trivial DoS. Security went further than the Architect on this point and it is the decisive argument for deletion over repair |
| A charset restriction (`[A-Za-z0-9_-]`) now | YAGNI while ids are generator-produced and never used as a key into a sink. Recorded as escalation triggers 1, 5 and 7 below rather than built — *recording the triggers is what makes it defensible not to build it today* |
| Trimming the id like `HabitTitle` does | Would change lookup semantics silently: `" h-1 "` and `"h-1"` would become the same id in the repository. The human ruled they are different ids (Q2) |
| A dedicated `HabitIdError` type | Splits one aggregate's error family for no consumer benefit; `IdLength` next to `TitleLength` keeps the `Display` surface and the match sites uniform |
| Distinguishing "malformed id" from "unknown id" in the UI | Discloses the shape of the id space for zero user value — the user's remedy is identical in both cases |

## Escalation triggers (verbatim — the conditions that reopen this decision)

Three were recorded by the Architect; Security confirmed them and added four. All seven are load-bearing: they are the reason the charset restriction and the deeper hardening are *deferred* rather than *forgotten*.

| # | Trigger | What it forces |
|---|---|---|
| 1 | **Persistence arrives** — the id becomes a storage key or a path component | The charset restriction goes from YAGNI to necessary (path traversal, key injection) |
| 2 | **A multi-user context arrives** — the id becomes an authorisation boundary | The ownership check must land **in the use case, at the same place as the parse**, on pain of direct IDOR by URL |
| 3 | An id generator produces more than 64 characters | `MAX_LEN` must be revisited before the generator ships |
| 4 | **SSR moves into production dependencies** (`dioxus-ssr` leaves `dev-dependencies`, or a backend appears) | **Security rates this the most important**: it converts a self-inflicted DoS into a *remote* one, and makes the `Ritual` route — which never crosses into the core and is therefore **not** covered by the bound — reachable by a third party via a forged link |
| 5 | The id becomes a key into a **sink** — file path, SQL, `HashMap` (hash-collision DoS), or an outbound URL | The charset restriction becomes **required immediately** |
| 6 | **`#[derive(Deserialize)]` is added to `HabitId`** (import/export, sync, local save) | Would reintroduce an infallible construction door bypassing `new` — the exact defect this ADR removes. The parade must be demanded at that moment: `#[serde(try_from = "String")]`, **never a bare derive**. (Security verified the workspace currently has no `serde` dependency at all) |
| 7 | Ids become **user-supplied** rather than generator-produced (import, restore, share) | The length bound stops being defence-in-depth and becomes a first-line control; the charset restriction returns to the foreground |

## Consequences / Constraints

- **MUST**: obtain every `HabitId` through `HabitId::new`. Reintroducing any infallible construction path (`From`, `Deserialize`, a `pub` tuple-struct field) reopens the defect this ADR closed.
- **MUST**: parse a primitive into its domain type at the **use case / query entry point**, never in `kayzen-app`, and never deeper than the entry point.
- **MUST NOT**: add `#[derive(Deserialize)]` to `HabitId` without `#[serde(try_from = "String")]` (trigger 6).
- **MUST NOT**: surface a distinct error for "malformed id" versus "unknown id".
- **Accepted consequence — the `Ritual` route is not covered.** `/habit/:id/ritual` never reaches the core: its view re-injects the raw route parameter into a `Link` and nothing parses it. Harmless in the current client-side WASM deployment, where the only author of a URL is the user themself; **trigger 4 changes that** — `app/src/views/ritual.rs`.
- **Known limitation**: `HabitId::new` is a constructor literally named `new`, so the mutation gate **cannot see it** — its `MIN_LEN`/`MAX_LEN` comparisons generate zero mutants. The boundary tests in `core/src/habit_management/domain/habit_id.rs` were written to the exact boundary anyway, because the instrument will not catch a regression here. See [[adr-0009-quality-gates]].
- **Verification**: the delivered change was approved by Dev-B (review), QA, and Security — the latter after explicitly re-testing its own rejected proposal against the code and retracting it.
