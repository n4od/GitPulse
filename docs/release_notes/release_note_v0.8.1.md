# Release v0.8.1 — the first update

**There are no functional changes in this release.** It is a version bump, and
that is the whole point.

## Why this exists

`0.8.0` was the first release with an updater, and it could not update itself: no
earlier build had one, so there was nothing to notice it. That means the one claim
this project actually cares about — _the app updates itself_ — was verified in
every part except the part that installs.

This release is what an installed `0.8.0` should find. It exists so that claim
stops being an assertion.

## What it should do

On a machine running `0.8.0`:

1. Launching the app asks GitHub for the update manifest.
2. The manifest now says `0.8.1`, so a notice appears and **Settings → Updates**
   reads _"0.8.1 is available"_.
3. Pressing **Install 0.8.1 and restart** downloads one installer, checks it
   against the minisign signature compiled into the app, installs it, and
   restarts.
4. The version in Settings reads `0.8.1`.

Steps 2 to 4 are the entire point of this release. If any of them does not
happen, the updater is broken and this file will say so in a later release.

## What happened

It worked, on Windows, the first time it was tried.

1. **Settings → Updates** in an installed `0.8.0` read _0.8.1 is available._
2. **Install 0.8.1 and restart** downloaded the installer, checked it against
   the signature compiled into the app, and installed it.
3. Ten seconds later the app was running again as `0.8.1` - a new process, the
   binary and the uninstall entry both reading `0.8.1`, and
   `starting version="0.8.1"` in the log.
4. Settings then read `0.8.1`, and offered no update.

macOS and Linux are built and signed the same way, but no update has been
watched installing on either. That claim is still open.

## What was verified before publishing

- The other artifacts are the same code as `0.8.0`, so nothing can regress
  that was not already shipped.
- `latest.json` names four platforms, each with a signature, and points at
  tag-pinned URLs.
- The signature over each update artifact matches the public key compiled into
  `0.8.0`.

## What this does _not_ change

- **The builds are still unsigned.** SmartScreen and Gatekeeper still warn on
  first launch. Auto-update is not code signing.
- **Nothing else in the app changed** — polling, storage, auth, the account
  switcher, and the updater itself are byte-identical to `0.8.0` apart from the
  version string.
- **Updating from `0.8.1` to `0.8.2`** will be the first time the path runs
  without being deliberately staged, and the first time it runs on macOS and
  Linux whether or not anyone is watching.

---

## 📅 Recent Version History

- **v0.8.1**: A version bump, to give `0.8.0` something to update to.
- **v0.8.0**: Auto-update — signed artifacts, a launch check, install on request.
- **v0.7.0**: The webhook subsystem removed; GitPulse no longer listens on a socket.
- **v0.6.2**: The account switcher moved from Settings to the avatar in the header.
- **v0.6.1**: `url` 2.5.9 for RUSTSEC-2026-0592, which was never reachable from the app.
- **v0.6.0**: Multiple accounts, switchable without signing in again.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU verified.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
