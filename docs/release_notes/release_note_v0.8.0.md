# Release v0.8.0 — Self-updating

**GitPulse can now update itself.** A newer release is noticed when the app
starts, and installed when you say so.

## 🆕 What is new

- **The app tells you when there is a new version.** At launch it asks GitHub for
  one small file, and if a newer release exists you get a notice and a line in
  Settings.
- **You decide when to install.** Nothing is downloaded until you press
  **Install**, and nothing is installed unless the download matches a signature
  compiled into the app. There is no silent replacement, and no background
  download.
- **Settings → Updates** shows the version you are running, the check, and the
  release notes for anything it finds. You can turn the launch check off; that
  switch controls the _check_, never an install.
- **The palette gained "Check for updates"** (Ctrl/Cmd+K).

## 🔐 How the update is verified

Every update artifact is signed with a minisign key. The public half ships inside
the app; the private half exists only as a GitHub Actions secret. A download that
does not match is refused before anything runs.

This is **not** code signing, and the difference matters:

|          | Updater signature                  | Code signing                       |
| :------- | :--------------------------------- | :--------------------------------- |
| Verifies | that the _app_ trusts the download | that the _OS_ trusts the installer |
| Needs    | a key you generate                 | a paid certificate                 |
| Status   | **on, from this release**          | still off                          |

So Windows SmartScreen and macOS Gatekeeper will still warn you the first time you
open GitPulse. That has not changed, and this release does not pretend otherwise.

## ⚠️ The honest part

**This is the first release anything can update from, and it cannot update
itself.** No earlier build has an updater, so `0.8.0` is installed by hand from
nothing. The whole path — notice, download, verify, install, restart — can only be
exercised end to end by an installed `0.8.0` noticing a `0.8.1`.

What was verified before shipping:

- the signing key produces a signature over the actual installer, and the
  signature is a real minisign blob;
- the update manifest is assembled from per-platform fragments, and a manifest
  missing a platform is refused rather than published;
- the collect step was executed against a real 8.2 MB Windows installer and
  produced the right asset name, a tag-pinned URL and the byte-exact signature;
- the app's own version string comes from the same source the updater compares
  against, so the number in Settings cannot drift.

**Not verified:** an update actually installing, on any platform. That is the next
release's job.

## 🔧 What else changed

- **Stored settings survive new preferences.** The settings blob now tolerates a
  field it does not know about. Before this, adding one preference in a future
  version would have failed to parse the blob on disk and silently reset
  everything — notifications, quiet hours, autostart, close-to-tray.
- **The release workflow signs every updater artifact and publishes
  `latest.json`.** Fourteen files per release now: the nine you download, three
  update artifacts, `latest.json`, and the checksums.

## ⚠️ What did not change

- **The builds are still unsigned**, and macOS users still need
  right-click → Open the first time.
- **`v1.0.0` is not scheduled.** It was reserved for signed installers _and_
  auto-update; auto-update is now here, so the only thing left is the certificate
  spend, which is not planned.
- **Everything else is untouched** — polling, storage, auth, the account switcher.

---

## 📅 Recent Version History

- **v0.8.0**: Auto-update — signed artifacts, a launch check, install on request.
- **v0.7.0**: The webhook subsystem removed; GitPulse no longer listens on a socket.
- **v0.6.2**: The account switcher moved from Settings to the avatar in the header.
- **v0.6.1**: `url` 2.5.9 for RUSTSEC-2026-0592, which was never reachable from the app.
- **v0.6.0**: Multiple accounts, switchable without signing in again; per-account tokens and cache.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
