# Release v0.8.4 — the update that can be watched

**There are no functional changes in this release.** Every byte of the app is
identical to `0.8.3` apart from the version string. It exists so that the install
feedback added in `0.8.3` can actually be seen.

## Why this exists

`0.8.3` reports what an install is doing: a progress bar, a percentage, a byte
count, and three stages — preparing, downloading, installing.

The catch is unavoidable: **the build that shows that feedback is the older one
doing the updating.** `0.8.2` has no such code. So the release that adds it can
never display it. It takes one more release for the feature to be observable, and
this is that release.

## How to see it

1. **Be on `0.8.3`.** If you are not already, install it directly —
   [`GitPulse-windows-x64-setup.exe`](https://github.com/n4od/GitPulse/releases/download/v0.8.3/GitPulse-windows-x64-setup.exe)
   (or the macOS or Linux build from the [`v0.8.3` release](https://github.com/n4od/GitPulse/releases/tag/v0.8.3)).
2. Open GitPulse and go to **Settings → Updates**. It will offer `0.8.4`.
3. Press **Install 0.8.4 and restart** — and watch the Updates section.

You should see, in order:

```
Preparing to install GitPulse 0.8.4...        (the manifest is re-read)
Downloading GitPulse 0.8.4 - 47%              (with a bar)
2.0 MB of 4.3 MB
Installing GitPulse 0.8.4 - GitPulse will restart by itself.
```

and then the app goes away and comes back as `0.8.4`.

**Updating straight from `0.8.1` or `0.8.2` will not show any of this.** Those
builds predate the feature; you would get the old silent install. This is the one
time the order matters.

## What was verified before publishing

- `0.8.3` and this release are the same code. Nothing can regress that was not
  already shipped.
- The manifest names four platforms, each with a signature, and points at
  tag-pinned URLs.
- The signature over each update artifact matches the public key compiled into
  `0.8.3`.

## What happened

It worked, and it was watched.

Updating from `0.8.3` to `0.8.4` showed the stages in order — **preparing**,
**downloading** with its percentage and byte count, **installing** — and the app
came back as `0.8.4`. Confirmed on disk afterwards: the binary and the uninstall
entry both read `0.8.4`, and the log records `starting version="0.8.4"`.

That closes the last part of the install path that was only unit-tested.
[`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md)
now records it as verified rather than assumed.

## What this does not change

- **The builds are still unsigned.** SmartScreen and Gatekeeper still warn on
  first launch. Auto-update is not code signing.
- **macOS and Linux update installs remain unproven.** Both are built and signed
  the same way; no update has been watched installing on either.
- **Nothing else changed** — polling, storage, auth, the tray and the dashboard
  are `0.8.3`.

---

## 📅 Recent Version History

- **v0.8.4**: A version bump, so the install progress added in 0.8.3 can be watched.
- **v0.8.3**: An install reports preparing, downloading and installing as it goes.
- **v0.8.2**: "Failing" counts the latest run, not every failure ever cached; the taskbar badge fits its icon.
- **v0.8.1**: A version bump, to give `0.8.0` something to update to.
- **v0.8.0**: Auto-update — signed artifacts, a launch check, install on request.
- **v0.7.0**: The webhook subsystem removed; GitPulse no longer listens on a socket.
- **v0.6.2**: The account switcher moved from Settings to the avatar in the header.
- **v0.6.1**: `url` 2.5.9 for RUSTSEC-2026-0592, which was never reachable from the app.
- **v0.6.0**: Multiple accounts, switchable without signing in again.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, the MIT licence.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
