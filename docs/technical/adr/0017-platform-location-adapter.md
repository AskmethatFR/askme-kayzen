---
id: "adr-0017-platform-location-adapter"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-24"
relations:
  related:
    - "architecture-overview"
    - "adr-0009-quality-gates"
  depends-on:
    - "adr-0016-snapshot-store-persistence"
    - "adr-0003-two-crate-workspace"
answers:
  - "The path-lookup crate has no answer on a platform — where does the data directory come from then?"
  - "How much of the rule about a location may live in a platform adapter?"
  - "A call into the platform's own runtime fails — does the app fall back, panic, or refuse?"
  - "Why does absorbing that failure not extend adr-0016's quiet-death hazard?"
  - "Where are a foreign runtime's hazards written down, and why not in this graph?"
  - "Is cross-target compilation a gate class here, and what does a green run of it prove?"
  - "What is the largest residual risk on a platform arm that no gate executes?"
decided_in:
  - "#40 — 2026-08-24, Android's data directory (PR #41)"
---

# ADR 0017 — A platform-specific location comes from an adapter that holds no policy, and the arm it opens is guarded by a gate that never runs it

> **One-liner**: When the path-lookup crate has no answer for a platform, the location comes from a **`#[cfg(target_os)]` adapter that answers `Option<PathBuf>` and decides nothing** — every rule about that value stays in the composition root, host-testable and shared by every arm. Failure on the platform call is **absorbed into `None`**, which is admissible only because `None` terminates in [[adr-0016-snapshot-store-persistence]]'s refusal screen and never in a write. The arm is guarded by a **cross-target compile gate that never links and never executes**, and says so where it is defined.
> **Links**: [[adr-0016-snapshot-store-persistence]] (durability as an adapter concern, *Degrade loudly, or not at all*, and the absorption cost this node bounds — **that node is untouched; this one extends it into territory it did not cover**), [[adr-0009-quality-gates]] (one runner, and a gate that cannot run must read as red — both applied unchanged), [[architecture-overview]] (the bar for a platform arm with no test runner, and the single home for what stays open).

## Context

[[adr-0016-snapshot-store-persistence]] settled that durability is an adapter concern, that the substrate is chosen at compile time, and that no durable location means **refuse at startup**. It did not settle where a platform-specific *location* comes from when the path-lookup crate has none.

On Android, `dirs::data_dir()` answers `None`: the crate routes that platform through its Linux/XDG lookup, and an app process has neither `HOME` nor `XDG_DATA_HOME`. adr-0016's refusal therefore fired at **every launch** — correctly, on a platform that does have a private per-app directory. The decision was never wrong; the directory was missing.

## Decision

| Facet | Decision |
|---|---|
| Where a platform location comes from | A **`#[cfg(target_os = "…")]` adapter** answering `Option<PathBuf>`. It asks the platform the one question a host cannot answer for itself, and returns |
| What that adapter may decide | **Nothing.** It holds no policy: no shape check, no name appended, no substitution. An adapter that decides is a decision reachable only on a device, and a decision reachable only on a device is one nobody exercises |
| Where the rules live instead | In `resolve_data_dir`, in the composition root (`app/src/composition.rs`) — must be absolute, gets the app's own name appended, `None` refuses. **One implementation for every arm**, run on a host by ordinary tests, so a rule added for one platform holds on all of them by construction |
| Failure on the platform call | **Absorbed into `None`.** Never a fallback, never a panic, never an `unwrap` — a location the OS can clear or another app can read is not a lesser outcome than a refusal ([[adr-0016-snapshot-store-persistence]], *Degrade loudly, or not at all*) |
| Why absorption is admissible here, when adr-0016 calls it a hazard | Because of **where it terminates**. adr-0016's standing price — *each absorption is a place data can die quietly* — is a property of absorptions on the **write** path, which cannot report failure. Every absorption on this path resolves to `None`, and `None` resolves to the refusal screen: the outcome is **loud**, the user is told, and nothing reaches `SnapshotStore::save`. The hazard class is not extended. **An absorption on this path that resolved to anything other than the refusal is a defect**, not a variant |
| Where a foreign runtime's hazards are written down | **At the call site, never in this graph.** The JVM/JNI facts this arm depends on — which errors leave state pending, which handle must never be released, which convenience API drops a safety check — are pinned as `@law:` blocks in `app/src/infrastructure/android_files_dir.rs`, each citing the upstream file and line it was read from. They are versioned facts about someone else's crate: copied here, they would go stale in the one place nobody opens when bumping a dependency |
| Cross-target compilation | A **gate class**, in the single runner ([[adr-0009-quality-gates]], applied unchanged): a real lint pass against the arm's target, a missing toolchain target returning **2** and reading as **red**, never skipped. CI installs the target on the existing pinned toolchain rather than the gate installing it |
| What a green cross-target run claims | Exactly [[architecture-overview]]'s bar for an arm with no test runner — *a clean cross-target build plus manual verification, stated as such and never presented as coverage* — and **no more**. It compiles and lints; it never links and never executes. **The gate states its own limits at its own definition** (`scripts/check.sh`), which is where anyone reading a green line is standing |
| Why this shape is also a measurement property | The mutation perimeter excludes `app/src/**` by design ([[adr-0009-quality-gates]]). Keeping every decision out of the platform adapter makes that exclusion **cost nothing on this arm**: what sits inside the excluded, unmeasurable file is a pipe, and what is worth measuring sits where the gate can reach it |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Let the platform adapter apply the rules (check the shape, append the app name) | Duplicates them per arm and puts them where only a device can run them. The composition root already had the one implementation; giving the adapter a second one is how two platforms start disagreeing about what a valid location is |
| Fall back to another writable directory when the platform call *fails* | adr-0016 already rejected a fallback for the *missing*-location case, and measured what it would have shipped. A location lookup that **failed** is not better evidence than one that was **absent** — the same silence, one step later |
| Panic (or `unwrap`) on the platform path | Turns a refusal screen into a crash. So does absorbing a foreign-runtime error while leaving that runtime's state dirty — which is exactly why the clearing is a `@law:` at the call site and not an implementation detail |
| Restate the JNI/JVM hazards in this node | They are pinned to crate versions. In an ADR they rot silently at the next dependency bump; at the call site they sit in the diff of the file that would break |
| Build (or link) the arm in the gate, with an NDK | Requires an NDK on every machine and in CI to catch a typo under a `#[cfg]`, which lint-only already catches. **Running** the arm is a device task, and pretending a gate does it is the failure this node's honesty clause exists to prevent |
| A gate that skips when the toolchain target is absent | "Not measured" presented as green — the one failure mode a gate exists to prevent ([[adr-0009-quality-gates]]) |

## Consequences / Constraints

- **MUST**: obtain a platform-specific location from a `#[cfg(target_os)]` adapter that answers `Option<PathBuf>` and **holds no policy**.
- **MUST**: keep every rule about a resolved location in the composition root, in the one implementation every arm shares, exercised by host tests.
- **MUST**: absorb a platform-call failure into `None` — and **MUST NOT** let an absorbed failure resolve anywhere but the refusal. Absorption on the **write** path is still governed by [[adr-0016-snapshot-store-persistence]]'s escalation trigger, which this node does not touch: the answer there remains a fallible `save`, not one more absorption.
- **MUST**: pin a foreign runtime's hazards at the call site, with the upstream source they were read from. **MUST NOT** copy them into this graph.
- **MUST**: guard every new platform arm with a cross-target gate in the single runner, red on a missing target.
- **MUST NOT**: present a green cross-target run as coverage of that arm — in a report, a PR body, or a doc.
- **Deferred work and residual risk** arising from this decision live in **one place** — [[architecture-overview]]'s *Open questions / Gaps*. They are not restated here.
