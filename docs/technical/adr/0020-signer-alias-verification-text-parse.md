---
id: "adr-0020-signer-alias-verification-text-parse"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-28"
relations:
  related:
    - "adr-0019-android-release-bundle-seam"
    - "android-play-release-runbook"
    - "adr-0009-quality-gates"
answers:
  - "Why does the signing script not establish the signer's identity by reading keytool's output?"
  - "What was tried, and why did each attempt fail?"
  - "Four different parsers were defeated — is there one reason, or four?"
  - "The identity check is known-bypassable. Why is that accepted rather than fixed?"
  - "Which single check carries the protection today, and what happens if it changes?"
  - "Under what conditions does this acceptance stop holding?"
  - "If someone picks this up, what is the known-good direction — and which trap does a naive version fall into?"
decided_in:
  - "#28 — 2026-08-28, slice S2b: signing the release AAB with the upload key (PR #53)"
---

# ADR 0020 — The signer's identity is not established by parsing keytool's rendered output; the residual bypass is accepted, and rests entirely on the signer-count check

> **One-liner**: A signed bundle's signer certificate is **attacker data rendered into the same text channel as the fields we trust**, so no read of `keytool`'s human-readable output can establish signer identity — four implementations were defeated by forged certificate Distinguished Names, one invariant behind all four. The owner has **accepted the residual risk** on an open-source project rather than pay for a sound implementation. What makes that safe in practice is not the identity check at all: it is that the script signs **our own build output** with **our own key**, so any pre-signed input carries two signers, and a **signer-count check that demands exactly one** refuses it. That check is the whole protection.
> **Links**: [[adr-0019-android-release-bundle-seam]] (its *verify the property on the artifact's own bytes* rule, and its *an archive entry is addressed by ordinal, never by its attacker-influenced name* rule — this node is the same family of defect, one layer down, and is where that family was **not** closed), [[android-play-release-runbook]] (its standing rule *a verified signature is not a verified alias* — this node explains why the alias check that rule demands cannot be made sound by parsing), [[adr-0009-quality-gates]] (*a gate that cannot fail is a claim, not a measurement* — applied to a gate deliberately left as a claim, with the claim written down).

## Context

`jarsigner -verify` exits **0** on a bundle signed by the wrong key ([[android-play-release-runbook]]). Distinguishing "signed" from "signed by us" therefore needs a separate check: read the SHA-256 fingerprint of the certificate that actually signed the bundle, and compare it against the fingerprint of our upload key.

Both halves of that comparison were implemented by reading `keytool`'s **human-readable output** as text. The two halves are not symmetric, and the asymmetry is the whole problem:

| The read | Whose bytes it renders | Trustworthy? |
|---|---|---|
| The expected fingerprint, taken from our keystore alias | ours | yes — an attacker who can plant a certificate in our keystore already holds the machine |
| The actual fingerprint, taken from the signed bundle | **the signer's**, i.e. whoever produced the bundle | **no** |

The second read prints a certificate we do not control. That certificate carries a Distinguished Name — a free-text field, chosen by whoever generated the key, rendered by `keytool` into the same stream as the fields the parser trusts. **The data being judged is interleaved with the verdict.**

## What was tried, and how each was defeated

Recorded because the sequence is the evidence for the decision, and because each attempt looks correct until the next one is broken.

| Attempt | How identity was decided | How it was defeated |
|---|---|---|
| 1 — tool-reported alias | `jarsigner -verify` was passed the alias, and its *"not signed by the specified alias"* marker was read as the verdict | **Did not discriminate at all.** That marker is emitted identically for the right alias and a wrong one, because a self-signed certificate fails PKIX chain building. Every Android upload key is self-signed, so the check could never pass, for anyone |
| 2 — first fingerprint-shaped token | take the last field of the first line matching `SHA256:` | A DN containing the text `SHA256:` followed by the victim's fingerprint wins the match, because the DN is printed **before** the certificate's own fingerprint block |
| 3 — anchored state machine | ignore everything until an exactly-matching `Certificate fingerprints:` header line, then read the `SHA256:` field under it | `keytool` renders a **raw LF inside a DN** without escaping it — it merely quotes the value. A DN carrying embedded newlines therefore forges the entire header block, ahead of the real one. **Reproduced end to end: a bundle signed exclusively by an attacker key verified as the victim, exit 0** |
| 4 — re-encode, then hash | `keytool -printcert -rfc` piped into `openssl x509 -fingerprint -sha256`, so the fingerprint is computed rather than scraped | A complete victim PEM injected into a long DN field (~1000 chars) makes `keytool` emit **two** `BEGIN CERTIFICATE` blocks; `openssl` reads the injected one first |

**One invariant behind all four**: the attacker's certificate is part of the output being parsed. Attempts 2 and 3 differ only in how tightly they anchor; attempt 4 changes the encoding but not the channel. Anchoring harder is not a direction — it is the same move again, and the next DN carries whatever the next anchor keys on. **A rendered-text read of a certificate we did not produce cannot establish that certificate's identity, in any parser.**

## Decision

| Facet | Decision |
|---|---|
| **The identity check is not made sound** | No further attempt is made to establish signer identity by parsing rendered output. The residual bypass is **accepted, knowingly**, with this node as its record |
| Why | The project is open source. The threat model — an attacker who must already be able to substitute the bundle handed to a local signing script — does not justify the cost of a sound implementation. This is an **informed acceptance by the owner**, made *after* the bypass was reproduced and explained, not an oversight or an unexamined default |
| **The protection is the signer-count check, and nothing else** | The script signs **our own build output** with **our own key**, then verifies. A pre-signed bundle handed to it therefore ends up carrying **two** signers. A check that refuses anything not signed by **exactly one** key rejects it before the identity comparison is ever reached. The identity check is a second opinion; the count check is the defence |
| Why the count check survives the same injection channel | It parses the same untrusted stream, but injection can only **add** an apparent signer — the real signer's line is emitted by `keytool` itself and cannot be removed by anything inside a certificate. The check demands *exactly one*, so every forgery in the only available direction makes it **refuse**. That asymmetry is why this one check can carry a load the fingerprint read cannot |
| **Exactly one, never "more than one"** | The count is compared against one for **inequality**, not for "greater than one". A count of **zero** — output from which no signer line could be read at all — passed the "greater than one" form. That was fail-**open** on an ORDINARY UNSIGNED JAR under the real keytool, not only on some crafted archive: `scripts/test-shell-units.sh:820-825` reaches it with no attacker and no wording drift at all. The rule: **a count guard states the count it accepts, never the counts it rejects** |
| **Scope of the acceptance** | Local signing, on a developer machine, of an artifact this repository just built. That premise is load-bearing and is not a property of the script — it is a property of how the script is invoked |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Anchor the parse harder — stricter regex, more context lines, stricter field position | Attempt 3 already was the strict version of attempt 2, and was broken by a newline. The next anchor is broken by whatever the next DN contains. This is the failing move repeated, not a fix |
| Reject certificates whose DN contains newlines, `SHA256:`, or PEM markers | A denylist over free-form attacker text, re-derived for every parser and every tool version. It also refuses legitimate certificates for reasons we invented, and proves nothing about the ones it lets through |
| Re-encode via `-rfc` and hash with `openssl` (attempt 4) | Broken as recorded above: the injected PEM is emitted first. Changing the encoding does not separate the channels |
| Compare the signature block's DER bytes now | The sound direction (see below), and genuinely more work than this slice's value justifies. **Deliberately deferred, not rejected on merit** — recorded here so a future pick-up starts from the answer rather than from attempt 5 |
| Drop the identity check entirely, since the count check carries the protection | It costs nothing to keep, it is correct on every non-adversarial input, and it is the check that catches the realistic mistake — signing with the wrong alias from our own keystore. Removing it would also erase the only place the reader learns the protection is elsewhere |
| Treat this as closed and silent | The bypass is real and reproduced. An accepted risk that is not written down is indistinguishable from one nobody noticed, and the next reader would re-derive attempts 2 through 4 |

## Consequences / Constraints

- **MUST**: refuse a bundle unless it is signed by **exactly one** key. This check is the entire protection; **MUST NOT** weaken it to a "more than one" comparison, and **MUST NOT** remove it on the grounds that the identity check exists.
- **MUST**: treat any change to that count check as a change to a security boundary. **It re-opens the accepted risk**, and belongs in a review that says so.
- **MUST NOT**: read this node as "the identity check works". It does not, against a crafted certificate. It works against mistakes, which is a different claim.
- **MUST NOT**: derive trust from a certificate field parsed out of rendered tool output, anywhere in this repository — the same family as [[adr-0019-android-release-bundle-seam]]'s *a name is display text, never a key*.

### What voids this acceptance

Each of these removes a premise the decision rests on. Any one of them makes the residual **unacceptable**, and the sound direction below becomes required work:

| Condition | Why it voids the acceptance |
|---|---|
| The signing step runs anywhere **other than a local machine, on an artifact this repository just built** — in particular on CI against an artifact whose provenance the runner does not control | The protection is *we built the input, so a pre-signed one has two signers*. A runner handed a bundle it did not build has no such premise, and the identity check — the only remaining discriminator — is the bypassable one. This is exactly the shape of the not-yet-existing release-path work |
| The signer-count check is relaxed, removed, or its comparison changes | It is the whole defence. See above |
| The script is ever pointed at a bundle it did not produce — a re-sign, a re-verify of a downloaded artifact, a verification-only entry point | Same lost premise, reached from inside the repository rather than from CI |
| Verification is reused to decide something beyond "our key signed this" — a trust level, an ownership claim, a publication decision | The check's strength was sized against the consequence of getting it wrong on a local build. A larger consequence needs a larger check |

### The known-good direction, if this is picked up

A **direction, not a design** — nothing here has been implemented or reviewed.

Compare **bytes, not rendered text**. The signer's certificate is present in the archive's own signature block (`META-INF/*.{RSA,DSA,EC}`, a PKCS#7 `SignedData`) as DER. Extract that DER and hash it; the fingerprint is then computed from the certificate's actual encoding, and nothing inside the certificate can influence how it is read. Address the signature-block entry by **ordinal**, per [[adr-0019-android-release-bundle-seam]] — the entry's name is attacker-influenced too.

**The trap, hit during this cycle**: a pipeline that computes a hash over a stream can fail silently and hash **nothing**. SHA-256 of the empty input is the constant `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`, and it compares **equal** to any other value derived the same broken way — so a comparison of two empty-derived hashes *succeeds*, in the most convincing way possible. Any implementation of this direction must establish that it read a non-empty certificate before it is allowed to compare one.
