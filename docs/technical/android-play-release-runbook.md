---
id: "android-play-release-runbook"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-09-17"
relations:
  related:
    - "adr-0019-android-release-bundle-seam"
    - "adr-0009-quality-gates"
answers:
  - "Why must a human perform the first Play upload, and why can no workflow replace it?"
  - "Which step of a release is irreversible, and what does getting it wrong cost?"
  - "Why is the app shipping from a 0.0.x rail rather than 0.1.0?"
  - "Where does the signing key live, who holds which half, and what happens if we lose ours?"
  - "A signature verification exited 0 — under what condition is the bundle still wrong?"
  - "What must exist on the Play side before an automated publish can work at all?"
  - "When is a slice that depends on the store DONE, if no test can reach the store?"
  - "A publish half-failed — what is the recovery?"
  - "How is the automated publish started, what gates it, and what does it refuse?"
---

# Runbook — the first Google Play upload, and the release path it unlocks

> **One-liner**: The store's `versionCode` floor is set **permanently by the first upload a human performs**, and the Play API cannot create the first release of a new app. Everything below is therefore an **ordered human procedure**, not a workflow: its order is forced by irreversibility, not by preference, and two of its steps cannot be undone by anyone — including Google.
> **Links**: [[adr-0019-android-release-bundle-seam]] (the frozen `versionCode` function, the two-pass build seam, and the *no secret in a pull-request job* boundary — applied here, not restated), [[adr-0009-quality-gates]] (the gate doctrine — *a gate that cannot run must read as red*; a human step that cannot be gated is held to the attestation rule below instead).

## Why this is a runbook and not a script

Three properties of this procedure put it outside anything the repository can prove about itself:

| Property | Consequence |
|---|---|
| The `versionCode` floor is **permanent from the first accepted upload**. Play never accepts a value at or below one it already knows | A wrong first upload cannot be corrected, retried, or appealed. It burns version space forever |
| The Play API **cannot create the first release of a new app**, and Play refuses an unsigned bundle | No automated path can bootstrap itself. The first upload is a human one, by construction |
| **Play App Signing is enabled at that same first upload** | The key-custody arrangement every later release depends on is decided by a checkbox a human ticks once |

None of this is expressible in code, and none of it is recoverable. That is the whole reason this node exists.

## Standing rules

These hold for every release, not only the first. They are rulings already made — this node applies them, it does not re-open them.

| Rule | Why |
|---|---|
| **The version floor is chosen before the first upload, never after** | `versionCode` is a frozen arithmetic function of the workspace version ([[adr-0019-android-release-bundle-seam]] — see it for the function and its rationale). Choosing the version *is* choosing the floor |
| **No pull-request job ever receives a secret** | A credential reachable from a fork is a credential every contributor holds. The pull-request gate proves an unsigned bundle builds; that is all it needs to prove ([[adr-0019-android-release-bundle-seam]]) |
| **The keystore lives outside the repository**, under `$HOME`, and never enters it | `.gitignore` carries the keystore and secrets-file extensions as a **tripwire**, not as the defence. The defence is that the file was never inside the tree |
| **A password reaches a signing tool by variable *name*, never by value** | A password on `argv` is readable in `/proc/*/cmdline` by any process on the machine, is echoed verbatim by `set -x`, and lands in a CI log in plain text. The signing script passes names; it never expands a password itself, anywhere |
| **`jarsigner`, not `apksigner`** | An `.aab` is a JAR. `apksigner` implements the APK v2/v3 signing schemes and does not apply to a bundle |
| **A verified signature is not a verified *alias*** | `jarsigner -verify` exits **0** and prints `jar verified.` on a bundle signed by the **wrong alias**. Only an explicit alias check distinguishes the two. This is a property of the tool, not of our code, which is why it is recorded here rather than left to be rediscovered |
| **A store-dependent slice is DONE only on two legs** | (a) machine-provable behaviour green, **and** (b) a runbook step actually executed and its outcome attested in plain text on the issue. Never on "the YAML looks right" — [[adr-0009-quality-gates]] applied to a step no gate can reach |
| **The publish step is not idempotent and is never re-run on the same tag** | Play burns a `versionCode` permanently once it accepts it. Recovery from a half-failed publish is a **new tag**. There is no automated rollback, and none is wanted |

## Key custody, and what a lost key costs

With Play App Signing enabled, **Google holds the app signing key** — the one every installed device validates against. This repository's owner holds only the **upload key**, whose sole power is to prove to Play that an upload came from us.

The consequence is the point of enabling it: **a lost or compromised upload key does not make the app unshippable.** It is reset through Google, and shipping continues on the same app signing key, so already-installed devices update normally. There is no rotation automation and no escrow here, deliberately — the recovery path is Google's, and building one of our own would add a second thing to lose.

Losing the *app signing* key would be terminal. We do not hold it.

## The procedure

The numbering is the order. Each step's precondition is the previous step's outcome.

**1 — Generate the upload keystore.** Created locally, outside the repository, and never copied into it:

```bash
mkdir -p ~/.kayzen && keytool -genkeypair -v \
  -keystore ~/.kayzen/upload.jks -alias upload \
  -keyalg RSA -keysize 4096 -validity 10000
```

Record the alias and both passwords in a password manager at this moment. There is no way to recover them from the keystore, and the alias is what step 4's verification checks against.

**2 — Fix the workspace version. ⚠ IRREVERSIBLE ONCE STEP 5 COMPLETES.** The version in `Cargo.toml` is what the frozen function turns into the `versionCode` the store will remember forever. It must be set **before** building the bundle that gets uploaded — a bundle built at the wrong version is not patchable, it is rebuilt.

The current version is **`0.0.1` → `versionCode 1`**, chosen deliberately as an error-shakedown rail: `0.0.2`, `0.0.3` and so on stay available for the round of releases whose purpose is to find out what breaks. `0.1.0` comes when the app is functional — a jump *upward* is always legal, only a decrease is refused.

**What the alternative would have cost:** under the frozen function `0.1.0` yields `versionCode 1000`. Uploading it first would have set the floor at 1000 and made **every** `0.0.x` version permanently un-uploadable — the entire shakedown rail, gone before the first bug was found. This was caught by hand during the cycle that wrote this node, one step before the upload. It is the reason this document is a runbook and not a paragraph in a README.

**3 — Build the unsigned bundle, then sign it.** Two invocations, in that order: the release build produces the unsigned, aligned, correctly-versioned bundle, and the signing step signs it with the key from step 1 and re-verifies both the signature and the alignment on the **signed** bytes. Read each script's own header for its interface — this node does not restate it, and a restatement here would be wrong the first time a flag changes.

The signing script takes the keystore path, alias and both passwords from the environment, and nothing from the command line. Set them in the shell for the duration of the step; do not persist them in a shell profile.

**4 — Confirm the signature is by the intended alias**, not merely that the bundle verifies. See the standing rule above: exit 0 and `jar verified.` are printed for a bundle signed by the *wrong* alias. The signing script performs this check; if you verify by hand, verify the alias by hand too.

**5 — Upload by hand to the internal-testing track, and enable Play App Signing at that upload. ⚠ IRREVERSIBLE.** In the Play Console, on the app listing for the package id, create an internal-testing release and upload the signed bundle. Accept Play App Signing when offered — this is the moment the custody arrangement described above is established.

Confirm the release shows the expected `versionCode` before finalising. After this step, that number is the floor, permanently.

**6 — Attest it.** Paste the outcome back into the issue in plain text: the version uploaded, the `versionCode` the Console shows, the track, and the date. Until that record exists, the automated publish slice has not started — see the two-leg rule above.

## The Play-side prerequisites (one-time, outside this repository)

Four prerequisites, each independently checkable, all outside this repository:

1. A GCP project.
2. The Play Android Developer API enabled on it.
3. A service account **created in GCP and invited in the Play Console**, scoped to *release to testing tracks* only.
4. Its JSON key stored as a repository secret, reachable only from a job no pull request can trigger.

**A service account that exists in GCP but was never invited in the Play Console can publish nothing.** It authenticates successfully and is refused at the store, which reads as a credential problem and is not one. Check the invitation, not only the key.

## S4 — the automated publish

The publish path is one workflow, `.github/workflows/release.yml`, and three scripts that carry every decision the workflow must not make. It is **not** a replacement for the procedure above on a *new app*: the Play API cannot create an app's first release, so the first upload stays human (see the two-leg rule).

**How it is started.** Manually, from the Actions tab: *Play release* → *Run workflow*, on `main`. Two gates stand in front of it, and neither is a formality:

- **`workflow_dispatch` only, and only on `main`.** A tag push cannot start it; a tag is a ref a contributor can move, a dispatch is an explicit act on a named ref.
- **The `play-release` Environment requires a reviewer's approval.** The signing key and the Play credential are reachable only from that job, and no job a pull request can trigger references it.

**What the run does, in order** — everything refusal-shaped happens before the first build:

```
preflight (ref → version → tag → versionCode collision)
  → build the unsigned, 16 KB-aligned AAB
  → sign it with the upload key, and verify the keystore is the CONFIGURED one
  → attest the signed bundle (actions/attest-build-provenance)
  → publish it to internal testing as a DRAFT release
```

Nothing writes to the store before the commit at the end of that publish, and the release is left as a **draft**: a human starts the rollout in the Console. That is the only undo this pipeline has.

**The tag is a consistency gate, not a trigger.** The run refuses unless a tag named exactly `v<version>` exists *and* points at the commit being released. The version itself comes from `Cargo.toml` through the frozen function ([[adr-0019-android-release-bundle-seam]]) — never from the tag.

| run fails at | human action | recovery |
|---|---|---|
| ref refusal (dispatch not on `main`) | dispatch from `main` | none needed |
| refused version | fix `Cargo.toml` | new version + tag |
| tag missing / misplaced | create or retag `v<version>` at the intended commit — **the version is not yet burned**, nothing was built | re-tag, or new version + tag |
| `versionCode` collision | read the colliding code from the message; if the code is genuinely wanted, the version must increase | new version + tag |
| signing failure | the message names the cause (wrong password / alias / keystore, or a keystore that is not the configured upload key) — Play was never reached | fix, re-dispatch the same version |
| credential rejection | check the service-account **invitation** in the Play Console, not only the key | fix, re-dispatch the same version |
| Play rejection (the store refused the bundle) | inspect the Console; the `versionCode` **may** now be burned even though the run failed | treat as half-failed: new version + tag, unless the Console confirms nothing arrived |
| quota / transport failure after the upload started | inspect the Console for the `versionCode`; re-dispatch the same version **only** if the Console confirms nothing arrived | otherwise new version + tag |
| attestation missing but the upload green | provenance is recoverable by re-attesting locally; Play state is unaffected | re-run the attestation only, never the upload |

**Recovery from a failed or partial publish is a new version and a new tag — never a re-dispatch on the same version**, with the sole exception of a case where the Console confirms nothing arrived. Play burns a `versionCode` permanently once it accepts it, and the run itself refuses a code the internal track already carries.

**If the run failed at or after signing**, the signed AAB and its attestation are published as a workflow artifact (`kayzen-signed-aab-<version>`), so "a bad bundle" can be told from "the transport died" without rebuilding something that must not be rebuilt.

**What the run summary states**, without opening a log: the version and derived `versionCode`, the tag and the commit it was verified at, the configured upload-key SHA-256 fingerprint (public — exempt from redaction), and the upload outcome or the step that failed.

**Attest the run.** Paste this on issue #28 once the release is visible on the internal track:

```
## Automated publish attestation

- **Run URL**: <link to the workflow run>
- **Version**: <version> (versionCode <code>)
- **Tag**: v<version> at <commit>
- **Track**: internal testing, release status draft
- **Internal-track release**: <link to the release in the Play Console>
- **Upload-key fingerprint**: <as printed in the run summary>
- **Date**: <YYYY-MM-DD>
```

Until that record exists, the slice that built this workflow is not DONE — the two-leg rule above applies to a dispatch exactly as it applies to a hand upload.

## What is true now, and what still rests on a human record

Stated plainly, so nothing here is mistaken for routine:

- **Steps 1 and 2 have landed.** The upload keystore exists outside the repository, and the workspace version sits at `0.0.1` — ahead of the upload it exists to protect.
- **The automated publish path now exists**: one workflow and three scripts, with every refusal above checked before the first build step and exercised by `scripts/test-shell-units.sh`. What no local test can reach — that a real dispatch actually publishes, that the Environment gate really stops a non-reviewer, that Play accepts the bundle — is runbook-attested on issue #28 rather than dressed up as covered.
- **The `versionCode` floor and the Play App Signing custody arrangement come from the first upload Play accepts** (steps 5 and 6 of the procedure). The Console is the only place those facts are visible; this repository cannot read them, which is why the workflow refuses a code the internal track already carries instead of trusting a number written down here.
