---
id: "adr-0003-two-crate-workspace"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-16"
relations:
  related:
    - "architecture-overview"
  depends-on:
    - "adr-0001-validation-by-construction"
    - "adr-0002-habitboard-stateful-aggregate"
answers:
  - "Why is the repository a two-crate Cargo workspace (kayzen-core / kayzen-app)?"
  - "How is the dependency rule (domain never depends on UI) enforced?"
  - "Where do Dioxus, platform features, and assets live — and why not in core?"
  - "Why is uuid's `js` feature target-scoped to wasm32?"
  - "What known debt did the restructure leave (Cargo.lock closure, CI audit, public test doubles)?"
decided_in:
  - "LOCAL-3"
---

# ADR 0003 — Two-crate workspace: `kayzen-core` (pure domain) / `kayzen-app` (Dioxus shell)

> **One-liner**: The repository is a Cargo workspace of two crates — `kayzen-core`, a pure domain library with zero UI/wasm dependencies, and `kayzen-app`, the Dioxus 0.7 binary with feature-per-platform — so the dependency rule `app → core` (one-way) is **compiler-enforced**, not conventional.
> **Links**: [[architecture-overview]] (where applied), [[adr-0001-validation-by-construction]], [[adr-0002-habitboard-stateful-aggregate]] (the domain this crate boundary protects).

## Context

Until LOCAL-3 the project was a single crate: the pure domain (`habit_management`, `shared`) and the future UI entry point (`main.rs`) shared one `Cargo.toml`. The dependency rule "domain never depends on application/infrastructure/UI" ([[architecture-overview]]) was enforced only by review discipline — nothing stopped a `use dioxus::…` from landing in a domain file. With UI wiring approaching (Dioxus 0.7, targets web/desktop/mobile per [[design-setup-rust-dioxus]]), the boundary needed to become structural before the first UI code arrives.

Commit `40635ae` — a pure move (`git mv`, R100 on every core file), zero behavior change, 16/16 tests green.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| Workspace | Cargo workspace (`resolver = "3"`), members `core` + `app`, shared `version`/`edition` via `[workspace.package]` | `Cargo.toml` |
| Core crate | `kayzen-core`, **lib**, contains `habit_management/` + `shared/`; sole dependency `uuid` — zero dioxus/web-sys/wasm-bindgen | `core/Cargo.toml`, `core/src/lib.rs` |
| App crate | `kayzen-app`, **bin**, Dioxus 0.7 shape: `Dioxus.toml`, `assets/`, `main.rs` | `app/Cargo.toml`, `app/Dioxus.toml`, `app/src/main.rs` |
| Dependency rule | One-way edge `kayzen-app → kayzen-core` (path dependency). Core cannot reference app — the **compiler** rejects it; verified rule grep-clean by review | `app/Cargo.toml` |
| Platform features | Feature-per-platform on the app crate only: `default = ["web"]`, `web`/`desktop`/`mobile` mapping to `dioxus/<platform>` | `app/Cargo.toml` |
| uuid `js` feature | Target-scoped: `[target.'cfg(target_arch = "wasm32")'.dependencies]` adds `js`; native builds never pull the wasm shim | `core/Cargo.toml` |
| `main.rs` normalization | Idiomatic Dioxus 0.7: `#[component] fn App() -> Element` + `dioxus::launch(App)` + `asset!` links (favicon, main.css). Still a **placeholder** — calls no use case (constraint unchanged, see [[architecture-overview]]) | `app/src/main.rs` |

**Human-validated contextual choices**: crate names `kayzen-core`/`kayzen-app`; keep the existing hyphenated `use-cases/` directory layout; normalize `main.rs` to the idiomatic App shape.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Single crate + feature gate (`ui` feature isolating dioxus) | The dependency rule stays conventional — a feature flag does not stop domain code from importing UI types; only a crate boundary makes the compiler the enforcer |
| `dx` workspace template, crate-per-platform (`web/`, `desktop/`, `mobile/` crates) | Gold-plating — nothing is platform-divergent today; three shell crates would carry identical content. Feature-per-platform on one app crate covers the same targets |
| Third crate for `shared/` | One file (`guid_generator.rs`); a crate for it is pure overhead. Revisit only if a second bounded context needs the kernel without `habit_management` |
| Unconditional uuid `js` feature | Leaks the wasm/js shim into native desktop/mobile builds; target-scoping keeps native builds clean |
| Renaming `use-cases/` → `use_cases/` in the same move | Separate decision (naming convention, not structure); bundling it would break R100 pure-move traceability. **Deferred** — graph is silent on it |

## Consequences / Constraints

- **MUST**: keep `kayzen-core` free of UI/platform dependencies — anything dioxus/web-sys/wasm-bindgen belongs to `kayzen-app`. Adding such a dependency to `core/Cargo.toml` is a violation of this ADR, not a judgment call.
- **MUST**: keep the edge one-way — `core` never gains a path/dep on `app`.
- **MUST**: add any new platform target as a feature on `kayzen-app` mapping to `dioxus/<platform>`, not as a new crate (until something is genuinely platform-divergent).
- **MUST**: keep wasm-only dependency features target-scoped (`cfg(target_arch = "wasm32")`), never unconditional.
- **Known debt (from Security review — follow-ups, not blockers)**:
  - **MEDIUM, fast-follow**: the committed `Cargo.lock` carries the full desktop/mobile dependency closure (~280 crates: wry, webkit2gtk, openssl, jni, image codecs) because verification ran a bare `dx build` (defaults to desktop). Remediation: regenerate the lockfile scoped `--platform web`, or explicitly commit to supporting desktop/mobile and own that closure.
  - **LOW**: no `cargo audit` / `cargo-deny` in CI — the dependency surface just grew; supply-chain checks should follow.
  - **LOW/info**: the in-memory infrastructure test-doubles now sit on `kayzen-core`'s public API surface; consider `pub(crate)` (or a test-support feature) when a real repository adapter lands.
- **Out of scope**: UI wiring of the use cases (unchanged human constraint), any real persistence adapter, CI pipeline design.
