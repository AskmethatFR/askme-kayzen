---
id: "adr-0022-tag-derived-release-version"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-09-24"
relations:
  supersedes:
    # FULL-status supersession — adr-0019 flips to `superseded`. The reversal is
    # surgical though: the tables below name which of its facets survive verbatim
    # and which die. Its frozen versionCode function is NOT re-decided here; this
    # node inherits it and re-points only its input.
    - "adr-0019-android-release-bundle-seam"
  related:
    - "android-play-release-runbook"
    - "adr-0009-quality-gates"
    - "adr-0020-signer-alias-verification-text-parse"
answers:
  - "Where does a Play release's version come from now, and why did Cargo.toml stop being it?"
  - "What must be true of the tags at the commit being released?"
  - "Why was adr-0019's rejection of tags as a version source overruled, and what still bounds a moved tag?"
  - "Which facets of adr-0019 survive this reversal, and which die?"
  - "How does the version reach the bundle once the workspace file is release-irrelevant?"
  - "Why does the PR-side AAB smoke job pass a placeholder version instead of deriving one?"
decided_in:
  - "#73 — owner ruling 2026-09-23; landed on `feat/73-tag-derived-release-version` 2026-09-24"
---

# ADR 0022 — The release version is the tag: one `vX.Y.Z` at the dispatched commit, stripped and fed to the frozen reader

> **One-liner**: The Play release version is **derived from the release tag at the commit being released** — exactly one tag matching `^v[0-9]` must point at that commit; its name minus the leading `v` is handed to the frozen `version_code_from_semver`, whose refusal message surfaces verbatim. `Cargo.toml` leaves the release path entirely: the committed file is release-irrelevant, and the bundle **injects** the tag's version into it at build time. [[adr-0019-android-release-bundle-seam]]'s `versionCode` arithmetic, two-pass seam, artifact verification and secret boundary survive verbatim; its version source and the bump-then-tag ritual die.
> **Links**: [[adr-0019-android-release-bundle-seam]] (superseded here — the surviving/dying split is tabled below), [[adr-0009-quality-gates]] (the gate doctrine), [[adr-0020-signer-alias-verification-text-parse]] (the release path's other accepted residual), [[android-play-release-runbook]] (the ritual this rewrites: tag → push tag → dispatch).

## Context

The ritual [[adr-0019-android-release-bundle-seam]] and the runbook prescribed was a two-act: bump `[workspace.package].version` in `Cargo.toml`, tag `v<version>` at that commit, then dispatch. Two sources of one version, kept equal by human discipline — with the workflow reduced to checking that the two acts had agreed. Discipline of that shape drifts, and it drifted: **the 2026-09-23 attempt to release `0.0.3` failed at preflight** because `Cargo.toml` still read `0.0.2` — the tag had moved ahead of the file. The drift was already visible before that: the shell harness's own pins asserted the committed version `0.0.2` while the repo's shipped tag said `v0.0.3`, and no gate existed that would have caught the disagreement *before* a dispatch, because the disagreement was between two facts the pipeline did not own.

The tag was already a de facto gate — the run refused unless `v<version>` pointed at the dispatched commit. The owner's ruling of 2026-09-23 collapses the two acts: **the tag is the source**, the file leaves the release path, and the ritual becomes tag → push tag → dispatch.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| **The version's source** | `git tag --points-at` at the commit being released, filtered by the **loose** `^v[0-9]` anchor | `scripts/android-preflight.sh:78` |
| **The exactly-one contract** (newly frozen) | 0 candidates → refusal naming the SHA (`carries no release tag vX.Y.Z`); >1 → refusal listing them raw (`carries several release tags (v0.1.2 v0.1.3)`); 1 → version = the tag name stripped of its leading `v` | `scripts/android-preflight.sh:79-89` |
| **Why the raw names in the several-candidates refusal are safe** | `git check-ref-format` bars control characters, spaces and backslashes from a refname, so an existing tag can neither forge a log line nor carry an escape; the sink is a plain stderr `echo`, never a format string or `eval` | Security verification, #73 (D7) |
| **Why the filter stays loose** | A semantically pre-filtered regex (`^v[0-9]+(\.[0-9]+){2}$`) would silently drop a malformed candidate, and the frozen reader — the one component that refuses malformed versions *with its own words* — would never run. The loose filter preserves the verbatim-refusal contract: a malformed tag like `v1.0.1000` or `v0.0.3-rc1` reaches the frozen function and its message surfaces **verbatim and unprefixed** | `scripts/android-preflight.sh:73-95`, `scripts/android-release-lib.sh:197-208` |
| **How the version reaches the build** | `android-bundle.sh` takes a **mandatory positional `<version>`** and refuses with a usage line otherwise; the frozen reader validates it **before the first file touch** | `scripts/android-bundle.sh:73-79` |
| **The injection** | `set_workspace_version` writes the version into `[workspace.package].version` before `dx` pass 1 (so `dx` stamps `versionName` from it), and the value is read back and compared — verify-on-written-bytes, as [[adr-0019-android-release-bundle-seam]] prescribed for the seam | `scripts/android-bundle.sh:121-123` |
| **One owner per rule in the writer pair** | The writer guards **bytes** (refuses empty, quote, backslash, control characters — everything that could escape a TOML string), and never judges shape; shape stays solely with the frozen reader. Two refusals, two messages, no duplicated rule | `scripts/android-release-lib.sh:25-28,159-181` |
| **The committed `Cargo.toml` is release-irrelevant** | Preflight is pure-read and structurally pinned never to call the workspace reader; the only production caller of the write/read-back pair is the bundle, at build time, on an ephemeral runner | `scripts/test-shell-units.sh:1041-1044` |
| **The tag stays a gate's input, never a trigger** | `release.yml` keeps `workflow_dispatch` as its **only** trigger, with its `@law` block untouched; the run derives the version from the tag, a human dispatches | `.github/workflows/release.yml:3-11,170` |
| **The PR-side AAB smoke job** | `ci.yml`'s `android-aab` job is a build smoke test: nothing is dispatched, no release tag exists at a PR, so it hands the bundle a **fixed placeholder**. The placeholder cannot reach a release — `release.yml` passes only `${{ steps.preflight.outputs.version }}`, and the two workflows never share a runner | `.github/workflows/ci.yml:230`, `.github/workflows/release.yml:170` |

### What survives [[adr-0019-android-release-bundle-seam]] verbatim, and what dies

| Facet of adr-0019 | Verdict |
|---|---|
| The frozen `versionCode` function (`major × 1000000 + minor × 1000 + patch`, components `0..=999`, result `> 0`) and its verbatim refusal messages | **Survives unchanged** — this node re-points its input, never its arithmetic |
| Range-guard-**before**-arithmetic | **Survives verbatim** — the rule is about the guard and the operation, independent of where the version string comes from |
| The two-pass build seam (inject between `dx` pass 1 and Gradle, assert the anchor) | **Survives verbatim** — the injected value's *provenance* changes, the seam does not |
| Verify-on-produced-bytes (manifest read-back, alignment on the artifact) | **Survives verbatim** |
| Ordinal addressing inside an archive | **Survives verbatim** |
| No-secret-in-a-PR-job | **Survives verbatim** — the smoke job still receives no secret, and the placeholder keeps it that way without deriving anything |
| **`Cargo.toml` as the version source** | **Dies.** The workspace file is written at build time and read back; it is never consulted as a source |
| **The bump-then-tag ritual** (bump the file, then tag `v<version>`) | **Dies** — it is the discipline that drifted; there is nothing left to keep in agreement |

### The tag-move trade-off, and what still bounds it

[[adr-0019-android-release-bundle-seam]] rejected tags as a version source because a tag is a ref a contributor can move: derive from a movable ref and the released number need not match the code. The reversal accepts that risk deliberately, because the alternative (two sources kept equal by hand) demonstrably failed first. The risk is not ignored — it is bounded, and the bounds are the ones that already existed:

| Bound | What it stops |
|---|---|
| The dispatch is a **human act** on `refs/heads/main` — a tag push starts nothing | A moved tag does nothing by itself; a release still requires a person to press the button at that commit |
| The `play-release` Environment requires a reviewer's approval | The person is not sufficient either — a second human sees the run before any credential is reachable |
| The `versionCode`-collision refusal | A tag moved to dodge the floor is refused the moment the derived code is already on the internal track |
| The workflow's concurrency group | Two dispatches cannot race the same store state |

Residual, accepted: a maintainer can move a tag and re-dispatch between preflight and publish, and no gate in this repository sees it. That is narrower than the two-act drift it replaces (which no gate caught either, at every release rather than one), and the publish step still refuses a code the track already carries.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Keep `Cargo.toml` as the source, behind stricter gates | The drift is not a missing gate, it is the two-source shape itself; a gate that watches the file agree with the tag re-proves a human ritual on every release instead of removing it. The 2026-09-23 failure is the demonstration |
| Make the tag push the trigger | Forbidden since adr-0019 and unchanged here: `workflow_dispatch` only (`release.yml:3-11` `@law` untouched). A trigger a contributor can move is a different, larger decision than a source |
| Inject the version through an environment variable | A parallel path that is silent when unset; the positional argument refuses loudly instead |
| Have the bundle derive the version from the tag itself | Duplicates the derivation in an adapter that cannot execute locally, and breaks the explicit-input-validated-before-write contract |
| Semantically pre-filter the candidates (full `vX.Y.Z` regex before counting) | A filtered-out malformed tag never reaches the frozen reader, whose verbatim refusal is the contract — the loose anchor is what keeps one owner per message |
| Amend adr-0019 | Graph rule: a changed decision is a new ADR with `supersedes:`, the old one flipped — never an amendment |

## Consequences / Constraints

- **MUST**: tag the commit `vX.Y.Z` and push the tag **before** dispatching; the run refuses a commit that carries zero release tags and refuses one that carries several.
- **MUST NOT**: read a release version from `[workspace.package].version` — the preflight's structural pin (`scripts/test-shell-units.sh:1041-1044`) holds the dependency shut; only the bundle's write/read-back pair may touch that line.
- **MUST**: surface the frozen reader's refusal message verbatim and unprefixed — a paraphrase anywhere in the chain is the same defect as a paraphrased error at any trust boundary.
- **MUST**: keep the writer guarding bytes and the frozen reader owning shape — one rule, one owner, one message.
- **MUST NOT**: add a trigger to `release.yml`; `workflow_dispatch` remains the only one.
- **MUST**: keep the PR-side smoke job free of both secrets and tag-derived inputs — a fixed placeholder is the honest input for a job that proves only that the build works.
- **What this node does not establish**: that a real dispatch publishes (owner-attested, as every store-reaching step in [[android-play-release-runbook]]), and that the AAB's manifest end-to-end carries the derived values on a real toolchain — the manifest read-back assertions are kept, but no SDK/NDK machine verified them this cycle.
