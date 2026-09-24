---
id: "android-play-release-execution-guide"
type: "technical"
owner: "operator"
status: "current"
updated: "2026-09-24"
relations:
  supersedes: []
  extends:
    - "android-play-release-runbook"
  related:
    - "adr-0022-tag-derived-release-version"
answers:
  - "What exact commands do I run to build, sign, and upload the first bundle?"
  - "How do I verify the signature is by the correct alias?"
  - "What do I paste back into the issue after uploading?"
---

# Execution guide — the first upload (steps 3-6)

> **Prerequisites**: The runbook at [[android-play-release-runbook]] documents **why** the procedure is ordered this way and **what** each step protects. This guide documents **how** to execute steps 3-6 on the current machine, with the exact commands and expected outputs. Steps 1-2 are already done: the upload keystore exists at `~/.kayzen/upload.jks`, and the release version comes from the release tag ([[adr-0022-tag-derived-release-version]]) — never from the workspace.

## Before you start

**Verify the keystore exists and you know the passwords:**

```bash
ls -l ~/.kayzen/upload.jks
keytool -list -keystore ~/.kayzen/upload.jks -alias upload
```

The second command prompts for the store password. If it prints the alias `upload` with its certificate chain, you have the keystore and the password. If you do not know the passwords, **stop** — there is no recovery. Generate a new keystore (runbook step 1) and update this guide.

**Verify the NDK is installed and `NDK_HOME` is set:**

```bash
echo "$NDK_HOME"
ls "$NDK_HOME/toolchains/llvm/prebuilt/"*/bin/llvm-readelf
```

If `NDK_HOME` is empty or the `llvm-readelf` path does not resolve, install the NDK (r25c per [[adr-0022-tag-derived-release-version]], which carries adr-0019's toolchain-pin rule forward) and export `NDK_HOME` before proceeding.

**Verify `jarsigner`, `keytool`, and `python3` are on PATH:**

```bash
jarsigner -help >/dev/null && echo "jarsigner ok"
keytool -help >/dev/null && echo "keytool ok"
python3 -c 'import zlib, zipfile, io; buf = io.BytesIO(); \
  __import__("zipfile").ZipFile(buf, "w", __import__("zipfile").ZIP_DEFLATED).writestr("p", b"p"); \
  buf.seek(0); __import__("zipfile").ZipFile(buf).read("p")' && echo "python3 ok"
```

All three must print `ok`. If any fails, install the JDK or fix the PATH before proceeding.

## Step 3 — Build the unsigned bundle, then sign it

**Build the unsigned, aligned, versioned bundle** — the script takes the version as a mandatory argument; pass the one the release tag names (`v<version>` at the commit being released). With no argument it refuses with a usage line ([[adr-0022-tag-derived-release-version]]):

```bash
scripts/android-bundle.sh "<version>"
```

Expected output (last two lines):

```
==> <path-to-unsigned-aab>
<path-to-unsigned-aab>
```

The script prints the unsigned AAB's path on stdout. Capture it:

```bash
UNSIGNED_AAB="$(scripts/android-bundle.sh "<version>" | tail -1)"
echo "$UNSIGNED_AAB"
```

**Sign it with the upload key:**

```bash
export ANDROID_SIGN_KEYSTORE="$HOME/.kayzen/upload.jks"
export ANDROID_SIGN_KEY_ALIAS="upload"
export ANDROID_SIGN_STORE_PASSWORD="<store-password>"
export ANDROID_SIGN_KEY_PASSWORD="<key-password>"

scripts/android-sign.sh "$UNSIGNED_AAB"
```

Expected output (last two lines on stderr, then stdout):

```
==> <path-to-signed-aab>
<path-to-signed-aab>
```

The signed bundle is at `<unsigned-aab-path-with--signed.aab-suffix>`. For example, if the unsigned was `app/build/outputs/bundle/release/kayzen-app-unsigned.aab`, the signed is `app/build/outputs/bundle/release/kayzen-app-signed.aab`.

**Unset the passwords immediately after signing:**

```bash
unset ANDROID_SIGN_STORE_PASSWORD ANDROID_SIGN_KEY_PASSWORD
```

## Step 4 — Confirm the signature is by the intended alias

The signing script already verifies the signature **and** the alias (runbook standing rule: `jarsigner -verify` exits 0 for the wrong alias). If the script succeeded, step 4 is done.

To verify by hand (optional, for confidence):

```bash
SIGNED_AAB="<path-from-step-3>"

# Verify the signature
jarsigner -verify -verbose -certs "$SIGNED_AAB" | head -20

# Read the alias's certificate fingerprint
keytool -list -v -keystore "$HOME/.kayzen/upload.jks" -alias upload \
  -storepass "<store-password>" | grep "SHA256:"

# Read the signed bundle's signer fingerprint
jarsigner -verify -verbose -certs "$SIGNED_AAB" | grep "SHA256:"
```

The two SHA256 fingerprints must match. If they do not, the bundle is signed by the wrong alias — **do not upload it**. Rebuild and re-sign, checking the alias name.

## Step 5 — Upload by hand to internal-testing, enable Play App Signing

**In the Play Console:**

1. Navigate to the app listing for package id `com.askmethat.kayzen`.
2. In the left sidebar, click **Testing → Internal testing**.
3. Click **Create new release**.
4. Upload the signed AAB from step 3.
5. **When offered, accept Play App Signing.** This is the irreversible step: Google takes custody of the app signing key, and the repository holds only the upload key.
6. Before clicking **Next** or **Save**, confirm the release shows `versionCode: 1` and `versionName: 0.0.1`.
7. Add release notes (optional for internal testing): "First upload — shakedown rail."
8. Click **Save** or **Start rollout to Internal testing**.

**Confirm the release is visible:**

In the Internal testing page, the release should appear with status **Draft** (per the S4 spec — the workflow also publishes as `draft`, so the first manual upload matches the automated path). The `versionCode` column shows `1`.

If the Console refuses the upload with a `versionCode` collision, **stop** — the floor is already set, and it is not at 1. Inspect the Console's error message, record it, and escalate. Do not retry with a different bundle.

## Step 6 — Attest it

Paste the following into a comment on issue #28, in plain text:

```
## Manual upload attestation

- **Version uploaded**: 0.0.1
- **versionCode**: 1
- **Track**: internal testing
- **Release status**: draft
- **Play App Signing**: enabled
- **Date**: <YYYY-MM-DD>
- **Play Console URL**: <link-to-the-release>
```

This attestation closes AC 12 and unblocks S4 (the automated publish workflow). Until this comment exists, S4 is not started — the two-leg rule (runbook standing rule) requires both machine-provable behaviour and a human-executed step.

## If anything fails

| Failure | Recovery |
|---|---|
| Keystore password unknown | Generate a new keystore (runbook step 1), update this guide |
| `android-bundle.sh` fails | Read the script's error message; all failure modes are named — a usage refusal means the version argument is missing (the version the release tag names). Fix the cause, re-run |
| `android-sign.sh` fails | Read the script's error message; all failure modes are named. Fix the cause, re-run |
| Signature fingerprint mismatch | Do not upload. Rebuild and re-sign, checking the alias name |
| Play Console refuses the upload | Record the error message verbatim in the issue. Do not retry with a different bundle |
| `versionCode` collision | The floor is already set. Inspect the Console, record the state, escalate |

**Never re-run a step that reached Play.** The publish step is not idempotent; a half-failed upload burns the `versionCode`. Recovery is a new version plus a new tag, never a retry.
