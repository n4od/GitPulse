# Releasing GitPulse

How a version tag becomes installers people can download. Read this before
pushing a tag — a tag is not a label, it starts a build that creates a public
draft release.

**Companion documents:** [ROADMAP.md](./ROADMAP.md) (what is planned) ·
[CHANGELOG.md](./CHANGELOG.md) (what shipped) ·
[VERIFICATION.md](./VERIFICATION.md) (what is proven).

---

## The short version

```bash
# 1. Land the release on main, changelog and all.
# 2. Tag it.
git tag v0.5.0
git push origin v0.5.0            # this starts the release workflow
# 3. Wait for CI. It produces a *draft* release with installers attached.
# 4. Read the notes, check the artifacts, then publish the draft by hand.
```

No secrets are required for an unsigned release. See
[Adding signing](#adding-signing) to turn signing on later.

---

## What a tag actually does

`.github/workflows/release.yml` runs on `push: tags: ["v*"]`, and can also be
started by hand from **Actions → Release → Run workflow** (it takes an existing
tag name as input, because `github.ref_name` is unreliable for a manual run).

It does four things:

1. Builds on **four targets across three operating systems** — macOS arm64,
   macOS x64, Linux x64, Windows x64.
2. Re-runs the test suites on the tagged commit. The tests already ran on the
   merge to `main`; this is the last chance to catch a tag cut from a broken
   commit.
3. Uploads the installers to a **draft** release. Nothing is public until a
   human publishes it.
4. A `verify` job then checks that **every expected bundle is present** and
   fails the run if any is missing. A silently incomplete release is worse than
   a failed one.

**The draft is not optional.** A desktop release that publishes itself is a
release nobody reviewed.

### Artifacts, by platform

Every release ships two ways to run GitPulse: an **installer**, and a
**portable** build that unpacks and runs with nothing installed and no admin
rights.

| Platform            | Installer(s)          | Portable                                        |
| ------------------- | --------------------- | ----------------------------------------------- |
| macOS (arm64 / x64) | `.dmg`                | `_portable.tar.gz` — the `.app`, unpack and run |
| Linux               | `.deb`                | `.AppImage` — already portable                  |
| Windows             | `.exe` (NSIS), `.msi` | `_portable.zip` — one `.exe`, no install        |

The `verify` job requires one of each. Adding a bundle target to
`src-tauri/tauri.conf.json` means adding the matching pattern to that list, or
the run will fail on an artifact nobody asked for.

The portable archives are built by `scripts/package-portable.ps1` (Windows) and
`scripts/package-portable.sh` (macOS), which `release.yml` calls after the
bundles exist. They are the same scripts a local `npm run tauri build` uses, so
a local release and a CI release cannot drift.

---

## Unsigned builds, and what that means for users

Signing is **not configured**, so released installers are **unsigned**. That is
a deliberate state, not an oversight, and it has real consequences:

- **macOS** — Gatekeeper blocks the app on first launch ("Apple could not verify
  … is free of malware"). The user must right-click → **Open**, or run
  `xattr -dr com.apple.quarantine /Applications/GitPulse.app`. Notarization
  removes this, and notarization requires a paid Apple Developer account.
- **Windows** — SmartScreen shows "Windows protected your PC" and hides the run
  button behind **More info → Run anyway**. A code-signing certificate removes
  this.

Because of that, an unsigned build is a **prerelease**, and it is not
`v1.0.0`. See [Why not v1.0.0 yet](#why-not-v100-yet).

---

## Cutting a version

1. **Move the changelog.** In `CHANGELOG.md`, move everything under
   `[Unreleased]` into a new `## [x.y.z] - YYYY-MM-DD` section. Keep the
   `[Unreleased]` heading.
2. **Bump the version in three places** — they must agree, or the installer
   filenames and the release tag drift apart:
   - `src-tauri/tauri.conf.json` → `"version"`
   - `src-tauri/Cargo.toml` → `[package] version`
   - `package.json` → `"version"`
3. **Commit, push to `main`, and confirm CI is green.** Do not tag a red commit.
4. **Tag and push.**

   ```bash
   git tag v0.5.0
   git push origin v0.5.0
   ```

5. **Watch the run** under the repository's **Actions** tab. Four build jobs
   plus `verify`, on a cold cache taking roughly 20–30 minutes.
6. **Review the draft** under **Releases**. Check the notes, download an
   installer or two, and mark it a **prerelease** if it is unsigned. Publish
   when satisfied.

### Tag conventions

| Tag           | Meaning                                                        |
| ------------- | -------------------------------------------------------------- |
| `v0.5.0`      | A normal release.                                              |
| `v0.5.0-rc.1` | A release candidate. Everything after the `-` is a prerelease. |
| `v1.0.0`      | **Reserved.** See below.                                       |

**Do not push the historical `v0.1.0`–`v0.4.0` tags.** They point at commits
from before the project was renamed to GitPulse, so their releases would be
built from a binary and identifier that no longer exist. Those versions live on
in `CHANGELOG.md`, which is the durable record.

---

## Adding signing

Signing is a pure "add secrets" upgrade — no workflow edit is required, because
`release.yml` already passes every variable through. Set the relevant secrets
under **Settings → Secrets and variables → Actions** and the next tag is signed.

| Secret                               | What it is                                             | Where it comes from                        |
| ------------------------------------ | ------------------------------------------------------ | ------------------------------------------ |
| `TAURI_SIGNING_PRIVATE_KEY`          | Updater signing key                                    | `npm run tauri signer generate` — **free** |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password, if it has one                            | you choose it at generation                |
| `APPLE_CERTIFICATE`                  | Base64 `.p12` of the Developer ID cert                 | Apple Developer Program — **$99/yr**       |
| `APPLE_CERTIFICATE_PASSWORD`         | Password for that `.p12`                               | set when exporting it                      |
| `APPLE_SIGNING_IDENTITY`             | e.g. `Developer ID Application: Name (TEAMID)`         | `security find-identity -v -p codesigning` |
| `APPLE_ID`                           | Apple account email                                    | your account                               |
| `APPLE_PASSWORD`                     | An **app-specific** password, not the account password | appleid.apple.com                          |
| `APPLE_TEAM_ID`                      | 10-character team identifier                           | Apple Developer portal                     |
| `WINDOWS_CERTIFICATE`                | Base64 of the code-signing cert                        | a CA, or Azure Trusted Signing             |
| `WINDOWS_CERTIFICATE_PASSWORD`       | Its password                                           | the CA                                     |

`TAURI_SIGNING_PRIVATE_KEY` alone is enough to produce updater artifacts — but
those only matter once the updater plugin is installed, which it currently is
not. Re-enable `createUpdaterArtifacts` in `src-tauri/tauri.conf.json` **and**
install the plugin before adding that secret, or the app will keep shipping a
`latest.json` nothing reads.

Signing keys are secrets. Generate the updater key to a file outside the
repository, paste its contents into the secret, and delete the file. Never
commit it.

---

## Why not v1.0.0 yet

`ROADMAP.md` defines `v1.0.0` as **signed installers on three operating systems
with auto-update**. None of those three conditions holds today:

- the installers are unsigned (no certificates),
- there is no auto-update (the updater plugin is not installed),
- and the pipeline is correct on inspection but has only just started being
  exercised for real.

Tagging `v1.0.0` now would assert all three. The project's whole position is
that unearned claims are the expensive kind, so the version stays reserved and
releases carry `0.x` until the conditions are actually met.

---

## If a release run fails

The failures seen so far, and their causes:

| Symptom                              | Cause                                                                                            |
| ------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `verify` says an artifact is missing | a bundle target changed and the pattern list in `release.yml` was not updated                    |
| A build job fails on clippy          | the toolchain pin in the workflow drifted from the one the code was verified on                  |
| `createUpdaterArtifacts` error       | the flag is on but `TAURI_SIGNING_PRIVATE_KEY` is not set — turn the flag off, or add the secret |
| macOS build fails to compile         | platform-specific code that has never been built on macOS; this is what CI exists to catch       |

Fix it on `main`, then **delete the tag and re-push it**:

```bash
git tag -d v0.5.0
git push origin :refs/tags/v0.5.0
git tag v0.5.0
git push origin v0.5.0
```

Re-tagging is safe while the release is still a draft. It is not safe once the
release is published — the tag is then a promise to anyone who downloaded it.
