---
id: "adr-0019-android-release-bundle-seam"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-27"
relations:
  related:
    - "adr-0009-quality-gates"
    - "adr-0017-platform-location-adapter"
answers:
  - "What number does a release carry to the store, and who may change how it is computed?"
  - "Why is the range check written *before* the arithmetic rather than after it?"
  - "A generator owns the build project and rewrites it on every run — where does a value we own get injected?"
  - "Why does a build-system property not reach a generated project, and why is that the worst failure shape?"
  - "What proves the shipped native libraries carry the page alignment the store requires?"
  - "Why is the NDK pinned *below* the version the runner offers?"
  - "How is an entry inside an archive identified, when its name is attacker-influenced?"
  - "A gate ran and exited 0 — under what condition is that still not a pass?"
  - "Which CI job is allowed to hold a signing credential?"
decided_in:
  - "#28 — 2026-08-27, slice S2a: the unsigned release AAB (PR #52)"
---

# ADR 0019 — The release bundle is produced through a two-pass seam, and every property it must carry is verified on the artifact's own bytes

> **One-liner**: The store's version number is a **frozen arithmetic function of the workspace version**, range-checked *before* any arithmetic runs. The generated Android build project is owned by a generator that rewrites it on every invocation, so the values we own are injected at a **seam between two passes**, never through a build-system property that the generator would accept and silently ignore. Every property the artifact must carry is then **verified on the produced bytes**, with the toolchain pinned to the version where that verification still discriminates. Inside an archive, an entry is addressed by its **ordinal**, never by its name. And **no pull-request job ever receives a secret**.
> **Links**: [[adr-0009-quality-gates]] (the gate doctrine — *a gate that cannot run must read as red*; applied unchanged, extended here by one case it did not cover), [[adr-0017-platform-location-adapter]] (the Android arm whose cross-target gate compiles but never links; this node is what finally links it and inspects the result).

## Context

Until this slice the Android arm was compiled and linted, never packaged ([[adr-0017-platform-location-adapter]]). Producing a release artifact introduces properties that are not decidable by the compiler and not observable in the source tree: a version number the store interprets, a page alignment the loader enforces, and a generated build project this repository does not author.

Two of those properties are **irreversible or externally enforced**. The store never accepts a version number lower than one already uploaded, so the mapping is frozen from the first upload onward. The page alignment requirement is the platform's, not ours, and is checked at install time on the device — after the artifact has left every gate we own.

## Decision

| Facet | Decision |
|---|---|
| **The store version number** | `versionCode = major × 1000000 + minor × 1000 + patch`, computed from the workspace version alone. Each component is bounded to `0..=999`, the result must be `> 0`. `0.1.0 → 1000` |
| Why the formula is **frozen** | The store refuses any upload whose version number is not strictly greater than the last one accepted. The function is therefore **irreversible as of the first upload**: revising it can only burn version space, never reclaim it. It is a human-owned decision, approved at the proposal gate, and changing it is a new ADR — not an edit to this one |
| Why no second guard on the store's own ceiling | Three components bounded at 999 put the largest reachable value at `999999999`, an order of magnitude under the store's ceiling. A ceiling guard would be a branch no test and no mutant could ever discriminate — a dead branch is not a defence, it is a claim nobody can check |
| **The bound is enforced before the arithmetic, never after** | The accepted shape is validated by pattern **first**; arithmetic runs only on input already known to be in range. This is not style. A numeric comparison in the shell **fails** on an operand outside the machine integer range — it returns an error status, which a conditional reads as *false*, so a guard placed after the arithmetic skips itself exactly on the inputs it exists to reject, and the wrapped, negative result then satisfies every downstream check. **The rule: a range guard is only a guard while it precedes the operation that can overflow** |
| **The generated build project is patched at a seam, not configured** | The build project is regenerated in full from templates on every invocation of the generator, and its template hardcodes the version number. The build therefore runs the generator, **patches the generated project between the two passes**, and then runs the platform build tool alone against what the first pass already produced. This is the same seam the launcher-icon step already occupies |
| Why not a build-system property | Because that path is **accepted and silently inert**. The property is set, nothing errors, and the artifact ships the template's default. An inert configuration is worse than an unsupported one: it produces a green build carrying the wrong value, with no signal anywhere |
| **The patch asserts its own anchor** | It refuses unless the literal it replaces appears **exactly once**, and it re-reads the value back out of the manifest the build tool actually folded into the package. A generator template bump therefore **refuses loudly** instead of shipping the template's default version to the store. The same seam is where the regenerated project is inspected for a credential before it is trusted — see the last row |
| **Page alignment is verified on the artifact's own bytes** | The alignment flags are applied at link time, but the **check reads every packaged native library out of the produced bundle** and measures its smallest loadable-segment alignment. A stale build cache, a link flag that did not apply, or a no-op repackaging each leave a mis-aligned library behind a build that otherwise "ran fine", and none of them are visible anywhere except in the artifact. **The rule: a property required of the artifact is verified on the artifact** |
| Why the toolchain is pinned **below** what the runner offers | Newer NDKs align to the required page size **by default**. On one of those, the check passes whether or not the flags applied — it stays green while measuring nothing, on precisely the regression it exists to catch. The pin is not conservatism about toolchains; it is the condition under which the gate still **discriminates**. A toolchain bump is a change to what this gate proves, and must be reviewed as one |
| **Archive entries are addressed by ordinal, never by name** | An entry is identified by its **position in the archive's own listing**. The name is display text, never a key. This closes a class, not a bug: a name is attacker-influenced, unconstrained bytes — it may carry glob metacharacters, a newline, a NUL, or simply be **duplicated**, and every name-keyed lookup resolves such an archive to *some* entry while leaving another unread. An integer index survives all four |
| Why that rule is written as a rule | It was violated twice, in two different technologies, on the same check — once by a shell tool that treats a member argument as a *pattern*, once by a library whose name→entry map keeps only the **last** duplicate while its listing returns every entry. Both were demonstrated against real crafted archives, both reported success while never reading the malicious member. Two independent implementations converged on the same defect, which is what makes it a rule rather than a fix |
| **A gate that measures only part of itself is not a pass** | [[adr-0009-quality-gates]] settled that a gate whose *instrument* is missing must read as red. This slice produced the case that node did not cover: the instrument was present, the gate **ran and exited 0**, and a block inside it silently skipped — reporting a green verdict over 26 of 40 assertions, the missing 14 including the security regression and the only one that discriminated the alignment threshold. **A partial measurement reported as a pass is worse than no gate, because it carries a gate's authority.** A harness therefore refuses (exit 2) when a prerequisite it needs is unreachable, and a runner refuses to trust an exit-0 that produced no verdict line of its own |
| **No pull-request job ever receives a secret** | The pull-request gate proves exactly one thing: *an unsigned, correctly-versioned, correctly-aligned bundle builds*. Signing is proven locally first and wired into the release path later, where the credential is scoped to a job no contributor can trigger. The boundary is the rule; which workflow file expresses it is not |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| A monotonic counter, a build number, or a commit-count as the store version | Untraceable back to a released version, and it makes the released number depend on CI history rather than on the source. The store's constraint is monotonicity, which the arithmetic mapping already satisfies |
| Bound the components *after* computing the code | The guard skips itself on exactly the inputs that overflow, and the wrapped result then agrees with every downstream check. This is not a hypothetical: it is the defect found in review |
| Configure the version through a build-system property or a properties file | The generator's template ignores it. Accepted, silent, and wrong — the worst of the three |
| One pass: patch the project, then invoke the generator | The generator rewrites the whole project; anything patched before it is gone by the time it returns |
| Fork or vendor the generator's templates | Buys control over one literal at the price of owning a template tree that must be re-merged on every upstream release. The seam costs one assertion and stays on the upstream path |
| Assert the alignment from the build flags, or from the build tool's own metadata | That verifies our intention, not the artifact. The failure modes worth catching are exactly the ones where the intention was recorded and did not apply |
| Let the runner supply whichever NDK it preinstalls | The check would go green on a toolchain that aligns by default, on a codebase that no longer needs the flag applied to pass. A gate that cannot fail is a claim, not a measurement — [[adr-0009-quality-gates]] |
| Check only the one native library we ship today | The next native dependency arrives in the same bundle and regresses the property invisibly to a name-specific check |
| Keep addressing archive entries by name, with escaping or validation | Escaping fixes one implementation's metacharacter set; it does not touch duplicates, and it must be re-derived for every tool that reads the archive. The ordinal is the one identifier a crafted name cannot forge |
| Let a harness skip a block when a prerequisite is missing, and report the rest | That is the partial-measurement failure above, and it shipped once already in this very slice |
| Give the pull-request job a signing key so it can produce a store-ready artifact | Every contributor's branch would then run with a release credential in reach. The unsigned artifact proves everything the pull request needs to prove |

## Consequences / Constraints

- **MUST**: derive the store version number from the workspace version by the frozen arithmetic above, and **MUST NOT** change that function without a new ADR superseding this one. Widening a component's bound is the same decision.
- **MUST**: place a range guard **before** the operation it protects, wherever an out-of-range operand can make the comparison itself fail rather than answer.
- **MUST**: inject values into the generated build project at the seam between the two passes, and **MUST** assert the anchor being patched — a silently missing anchor is how the template's default reaches the store.
- **MUST NOT**: express a value we own as a property the generator accepts and ignores.
- **MUST**: verify on the produced artifact every property the artifact is required to carry, and **MUST** keep the toolchain pinned to a version on which that verification can still fail. A toolchain bump that removes the failure mode removes the gate.
- **MUST**: address archive members by ordinal. A name is display text; **MUST NOT** use it as a lookup key, in any language.
- **MUST**: refuse, never skip, when a prerequisite is unreachable — [[adr-0009-quality-gates]] applied, extended here to the *inside* of a gate that runs. A harness that measured a subset **MUST NOT** exit 0, and a runner **MUST NOT** trust an exit-0 that carries no verdict of its own.
- **MUST NOT**: expose a signing credential to any job a pull request can trigger.
- **What this slice does not establish**: that the artifact installs, launches, or is accepted by the store. A verified bundle is a verified *file*. Device verification and the release path are separate work, and their state lives in the issue tracker — not here.
