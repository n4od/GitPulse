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
# 1. Land the release on main: changelog and version bump. (The README needs no
#    edit - its download links are version-less and never change.)
# 2. Tag it.
git tag v0.5.6
git push origin v0.5.6            # this starts the release workflow
# 3. Wait for CI. It produces a *draft* release with thirteen artifacts attached.
# 4. Read the notes, check the artifacts, then publish the draft by hand.
```

One secret is required: the **updater signing key**. It is free to generate, and
it is _not_ code signing — see [The updater key](#the-updater-key). Code signing,
which is the thing that removes the operating system's warning, is still not
configured; see [Adding code signing](#adding-code-signing).

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
rights. That is **nine files a human downloads**, and three more that only the
app reads — two update artifacts and `latest.json`. **Thirteen in all**,
counting `SHA256SUMS.txt`.

| Platform            | Installer(s)          | Portable                                        |
| ------------------- | --------------------- | ----------------------------------------------- |
| macOS (arm64 / x64) | `.dmg`                | `-portable.tar.gz` — the `.app`, unpack and run |
| Linux               | `.deb`                | `.AppImage` — already portable                  |
| Windows             | `.exe` (NSIS), `.msi` | `-portable.zip` — one `.exe`, no install        |

#### Names

One file per platform and format, and the names carry **no version**:

```
GitPulse-windows-x64-setup.exe
GitPulse-windows-x64.msi
GitPulse-windows-x64-portable.zip
GitPulse-macos-arm64.dmg
GitPulse-macos-arm64-portable.tar.gz
GitPulse-macos-x64.dmg
GitPulse-macos-x64-portable.tar.gz
GitPulse-linux-x64.deb
GitPulse-linux-x64.AppImage
GitPulse-macos-arm64-updater.tar.gz
GitPulse-macos-x64-updater.tar.gz
latest.json
SHA256SUMS.txt
```

#### The update artifacts

Those three exist so the app can update itself, and nobody should download them
by hand. `latest.json` maps each platform to a URL and a **signature**; the app
fetches the manifest, and fetches the payload only if the user presses Install.

The shape differs per platform, and getting it wrong is a failed release rather
than a subtle bug — so it is worth knowing before editing the workflow:

- **Windows and Linux have no update artifact of their own.** Tauri signs the
  installer itself, so `GitPulse-windows-x64-setup.exe` and
  `GitPulse-linux-x64.AppImage` are each both the install download _and_ the
  update payload. There is no `.nsis.zip` and no `.AppImage.tar.gz`. That is not
  an assumption: `tauri-action` prints its own _"Looking for artifacts"_ next to
  _"Found artifacts"_, and the tarballs appear only in the first list.
- **macOS is the exception**, because an `.app` cannot be replaced in place. Tauri
  tars it, signs the tarball, and that is what the updater downloads. It is named
  `-updater.tar.gz` here so it is not confused with the portable build — and the
  two are **not** interchangeable: the updater verifies the bytes against the
  signature, so it must be the tarball Tauri signed, not the one
  `scripts/package-portable.sh` produced.

The manifest is **assembled in the `verify` job** from one fragment per platform,
because no single build job knows about the other three. A missing fragment fails
the run: a manifest without a platform is a platform whose users never get an
update.

**Why no version in the name.** GitHub resolves
`releases/latest/download/<file>` by _exact filename_, so a name can either carry
the version or stay stable across releases — never both. Up to `v0.5.5` every
artifact was published twice, in an attempt to have it both ways, which is why a
release held twenty assets and why the README's links resolved to whatever the
newest release happened to contain.

Now each artifact has one stable name and the README links through
`latest/download/`, so the links keep resolving as versions ship and **need no
edit per release**. The version is not lost: it is in the release title, inside
the package (Add/Remove Programs, `dpkg -I`, Get Info), and in each release's
`SHA256SUMS.txt`.

**The workflow names and uploads the artifacts itself.** It creates the draft,
builds with `tauri-action` — given no `tagName` or `releaseId`, so it uploads
nothing — then copies each bundle to its release name and uploads exactly those.
Tauri's own names cannot be used as-is: the architecture is `amd64` in a Debian
package, `x64` on Windows and `x86_64` in a Rust target triple.

> **Why the action is not allowed to upload.** On macOS it packages the `.app`
> into a tarball _unconditionally_ — its source says _"Other steps may benefit
> from this so we do this whether or not we want to upload it"_ — and then
> uploads what it made. That put two dead assets in every release from `v0.5.0` to
> `v0.5.5`. No input disables it, so the fix is to give the action no release to
> upload to.

#### What `verify` enforces

Not "an artifact matching this pattern exists" but **exactly these twelve names,
and nothing else**. An asset nobody asked for is a signal that the pipeline
changed underneath us — which is precisely how those two tarballs went unnoticed
for four releases — and a missing one fails the run rather than being discovered
by a user.

It also checks that the README links to the **nine a human downloads**, through
`latest/download/`. The update artifacts are excluded from that check on purpose:
they are machinery, and pointing a reader at a signed tarball they should never
fetch by hand would be worse than not mentioning it. That split is the coupling a
stable name buys — if a name drifts, every Download button 404s, and nothing else
would catch it.

The manifest is checked separately: it must cover all four platforms, or the run
fails before `latest.json` is uploaded. A manifest listing three platforms looks
perfectly valid and silently strands everyone on the fourth.

`verify` then re-downloads every asset, hashes what actually arrived, and
publishes `SHA256SUMS.txt`. These bundles are unsigned, so a hash is the only
integrity signal a user has.

Adding a bundle target to `src-tauri/tauri.conf.json` means adding it to the
collect step, to the expected list and to the README table, or the run fails —
which is the point.

The portable archives are built by `scripts/package-portable.ps1` (Windows) and
`scripts/package-portable.sh` (macOS), which `release.yml` calls after the
bundles exist. They are the same scripts a local `npm run tauri build` uses, so
a local release and a CI release cannot drift.

> **`app` must stay in `bundle.targets`.** Without it, Tauri builds the macOS
> `.app` only as a throwaway step on the way to the `.dmg` and then _deletes_
> it — so the portable step has nothing to pack. The first `v0.5.0` release run
> failed exactly this way.

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

An unsigned build is still published as a **normal release**, not a prerelease,
and it is not `v1.0.0`. Publishing it as a prerelease would keep it out of
`/releases/latest`, which is exactly where the README's download links point, so
a prerelease would break the buttons for everyone. The release notes say plainly
that the builds are unsigned. See [Why not v1.0.0](#why-not-v100).

---

## Cutting a version

1. **Move the changelog.** In `CHANGELOG.md`, move everything under
   `[Unreleased]` into a new `## [x.y.z] - YYYY-MM-DD` section. Keep the
   `[Unreleased]` heading.

   The notes that go into the GitHub release live in
   [`docs/release_notes/`](./docs/release_notes) — one file per tag, named after
   it. The workflow creates the release with a placeholder body, so the notes are
   pasted (or patched) in when the draft is published.

2. **Bump the version in three places** — they must agree, or the tag and the
   version inside the installers drift apart:
   - `src-tauri/tauri.conf.json` → `"version"`
   - `src-tauri/Cargo.toml` → `[package] version`
   - `package.json` → `"version"`

   `src-tauri/Cargo.lock` follows the manifest on the next `cargo` command.

   **The README needs no edit.** Its links go through
   `releases/latest/download/<name>` and the names never change; `verify` checks
   that the README still links to all nine, so a deliberate rename cannot slip
   through.

3. **Commit, push to `main`, and confirm CI is green.** Do not tag a red commit.
4. **Tag and push.**

   ```bash
   git tag v0.5.6
   git push origin v0.5.6
   ```

5. **Watch the run** under the repository's **Actions** tab. Four build jobs
   plus `verify`, on a cold cache taking roughly 20–30 minutes.
6. **Review the draft** under **Releases**. It should hold exactly **ten
   assets**: the nine named artifacts plus `SHA256SUMS.txt`. Check the notes,
   download an installer or two, verify one against the checksum, then publish.
   The builds are unsigned; that is a known state, not a reason to hold the
   release back — the notes say so plainly.

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

## The GitHub client id

The published app has to know which GitHub App to sign users in through. That
value is baked into the binary at compile time by `option_env!` in
`auth::configured_client_id`, and the release workflow supplies it from a
repository **variable**:

| Variable                    | Value                                  |
| --------------------------- | -------------------------------------- |
| `GITPULSE_GITHUB_CLIENT_ID` | the app's public client id (`Iv23li…`) |

Set it under **Settings → Secrets and variables → Actions → Variables**. It is a
variable rather than a secret on purpose: a client id identifies the app, not the
user, and the device flow never sends a client secret.

If the variable is missing the build still succeeds, but the installers cannot
sign anyone in — which is exactly what happened to `v0.5.0`. A runtime
`GITPULSE_GITHUB_CLIENT_ID` still overrides the compiled value, so a local
checkout can point at a different app without rebuilding.

---

## The updater key

The app verifies every update against a minisign public key compiled into the
binary, so a release **cannot be built without the matching private key**.
`createUpdaterArtifacts` is on, and Tauri fails the bundle with _"A public key has
been found, but no private key"_ rather than shipping an update nothing can
verify.

Generating one is free and needs no certificate authority:

```bash
npm run tauri signer generate -- -w <somewhere-outside-the-repo>/updater.key
```

That prints a public key and writes a private one. Then:

| Where it goes                                          | What                                              |
| ------------------------------------------------------ | ------------------------------------------------- |
| `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` | the **public** half. Committed; public by design. |
| `TAURI_SIGNING_PRIVATE_KEY` secret                     | the **private** half — the whole file's contents. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret            | its password, if it has one.                      |

Both are already declared in `release.yml` under the `tauri-action` step. **If you
generate a key with no password, delete the PASSWORD line** rather than leaving it
empty — an empty signing variable is exactly the failure mode described below.

The private key never enters the repository. Keep a copy somewhere that survives
this machine: **losing it means no future release can be signed**, and every
installed copy then has to be replaced by hand.

### The first updater release cannot test itself

No released build has an updater, so `0.8.0` is the first release anything can see
— installed by hand, from nothing. The check and the install can only be exercised
end to end by an installed `0.8.0` noticing `0.8.1`.

What _can_ be checked before then: that the manifest is well formed, that all four
platforms are listed with a signature each, and that a build pointed at a local
endpoint reports an update as available.

## Adding code signing

Code signing is **separate** from the updater key and is **not configured**. It is
what removes the operating system's warning on first launch, it needs a paid
certificate, and nothing else in this project depends on it.

One thing is needed here: the secrets, and an `env:` block in `release.yml` that
declares them. **That block is still not there on purpose.**

Declaring a signing variable _empty_ is not the same as leaving it unset. The
first release run proved it: mapping `APPLE_CERTIFICATE` to an unset secret
produced an empty string, Tauri read "present" as "sign this", and every macOS
build died trying to import a blank PKCS#12 — `failed codesign application:
failed to run command security import -f pkcs12`.

So the variables are omitted entirely. Turning signing on means adding the
secrets under **Settings → Secrets and variables → Actions** _and_ this block,
under the `tauri-action` step:

```yaml
env:
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  # The updater pair is already declared; these are the ones you would add.
  APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
  APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
  APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
  APPLE_ID: ${{ secrets.APPLE_ID }}
  APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
  APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
  WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
  WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
```

Add only the lines whose secrets you have set. Add them all and leave some
unset, and the empty-string failure comes straight back.

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

The updater key has its own section above. This one is only about the certificates
that silence the operating system.

---

## Why not v1.0.0

`ROADMAP.md` defines `v1.0.0` as **signed installers on three operating systems**.
Auto-update used to be part of that definition and shipped in `0.8.0`, so the only
condition left is **code signing** — paid certificates that are not bought, and
are not planned to be.

That is a spending decision rather than an engineering one, and it is the only
thing in the way. Until somebody makes it, releases carry `0.x`, which is the
honest number for "works, but the OS warns you the first time you open it".

---

## If a release run fails

The failures seen so far, and their causes:

| Symptom                                                            | Cause                                                                                            |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `verify` says an artifact is missing                               | a bundle target changed and the pattern list in `release.yml` was not updated                    |
| A build job fails on clippy                                        | the toolchain pin in the workflow drifted from the one the code was verified on                  |
| `Resource not accessible by integration` when creating the release | the workflow is missing `permissions: contents: write`                                           |
| macOS: `security import -f pkcs12` failed                          | a signing variable is declared but empty — remove the line, or set the secret                    |
| macOS: `no .app bundle under .../bundle/macos`                     | `app` was removed from `bundle.targets`; Tauri deletes the `.app` after building the `.dmg`      |
| `createUpdaterArtifacts` error                                     | the flag is on but `TAURI_SIGNING_PRIVATE_KEY` is not set — turn the flag off, or add the secret |
| `A public key has been found, but no private key`                  | the same missing key, seen from the bundler. Locally, export it; in CI, the secret is missing    |
| `no updater signature for <slug>`                                  | the collect step could not find the `.sig` — same cause, named by platform                       |
| `the manifest covers [...], expected [...]`                        | a build job produced no fragment, so that platform would never be offered the update             |
| macOS build fails to compile                                       | platform-specific code that has never been built on macOS; this is what CI exists to catch       |

Fix it on `main`, then **delete the tag and re-push it**:

```bash
git tag -d v0.5.0
git push origin :refs/tags/v0.5.0
git tag v0.5.0
git push origin v0.5.0
```

Re-tagging is safe while the release is still a draft. It is not safe once the
release is published — the tag is then a promise to anyone who downloaded it.
