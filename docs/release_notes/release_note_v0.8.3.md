# Release v0.8.3 — an install that shows its work

**Pressing "Install and restart" now shows what it is doing.** Before this, the
only reaction was one sentence of small text changing and the button vanishing —
and then the app was gone. That is indistinguishable from a click that did
nothing.

## What you get

A progress bar with a percentage and a byte count, in three stages:

| Stage           | When                                              |
| --------------- | ------------------------------------------------- |
| **Preparing**   | the manifest is re-read, before a byte is fetched |
| **Downloading** | `2.0 MB of 4.3 MB — 47%`                          |
| **Installing**  | the bytes are in and the installer has taken over |

The **Install** button stays on screen in a busy state instead of being removed
under the cursor. If the server never declares a size, no bar is drawn at all —
an empty bar and a full bar are both claims about progress, and neither would be
true.

## Why three stages, and not two

An install re-reads the update manifest before it downloads anything. That is
deliberate: it means a stale window installs the _newest_ release rather than
failing on a signature that no longer matches.

But that request can fail with **nothing downloaded**, and on the machine this was
reported from, it did. The log is unambiguous:

```
failed to check for updates: error sending request for url (.../latest.json)
```

The app afterwards was still `0.8.1`. So the click died before a single byte
arrived — which is exactly why it looked like nothing happened. Calling that
moment "downloading" would be a lie about a download that never started, so it
now says **preparing**.

## One thing to expect

**Installing this release will not show the new progress bar.** The build that
displays progress is the older one doing the updating, and `0.8.1` does not have
this code. The bar appears the next time you update — from `0.8.3` onwards.

## What was verified before publishing

- 183 backend and 203 frontend tests, including: the bar is seeded by the click
  rather than by the first byte; a late event cannot resurrect the bar over a
  finished or failed install; a returned install is treated as one that did not
  happen; and the three stage strings, which the frontend writes by hand, match
  the ones the backend emits.
- The bar is measured as a real `progressbar` with an `aria-valuenow`, so it is
  legible to a screen reader and not only to an eye.

## What was _not_ verified

**No live download has been watched.** That is recorded in
[`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md)
rather than glossed over: the bar, the stages and the byte counts are unit-tested,
and nothing has yet observed them against a real transfer, because it cannot be
observed until a release containing it is itself being updated.

## Security

**`cookie` 0.18.2 → 0.18.3**, for RUSTSEC-2026-0594 — private cookie nonces could
repeat across a `fork()`. GitPulse never forks, never uses private cookies, and the
advisory states outright that applications on Windows are unaffected, so it was not
reachable here. `cookie` arrives through `tauri` itself rather than by choice, and
the patch was published, so it is taken rather than exempted.

## What this does not change

- **The builds are still unsigned.** SmartScreen and Gatekeeper still warn on
  first launch. Auto-update is not code signing.
- **macOS and Linux update installs remain unproven.** Both are built and signed
  the same way; no update has been watched installing on either.
- **The download is unchanged** — same installer, same signature check, same
  restart. This release only reports on it.

---

## 📅 Recent Version History

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
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU verified.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
