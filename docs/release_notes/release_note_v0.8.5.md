# Release v0.8.5 — one status, and a clock that agrees

Three fixes. Two of them came from looking at something nobody had looked at
before.

## A repository has one status again

A card could read **`SUCCESS`** while the line under it said **"1 failing run"**,
and while the tray said failing.

The badge described the single newest run of _any_ workflow. The failing count
describes **each workflow's** latest completed run. So a repository whose
`Release` succeeded after its `CI` failed badged the success and counted the
failure — three surfaces, two answers.

The badge, the line under it and the count now come from one workflow set, so
they agree by construction. The consequence worth knowing: when a repository is
failing, the line under the badge names the **failing** workflow, which may be
older than the newest run. That is the point — it is telling you what is red, not
what ran last.

## Quiet hours meant the wrong hours

Quiet hours are configured by a person: `22-07` means _your_ evening. They were
compared against **UTC**. On this machine (UTC+3) that turned the window into
01:00–10:00 local — silent through the morning, and chatty through the evening it
was meant to cover. It now reads the local clock.

This was found by verifying the notification path end to end for the first time,
which also confirmed it works:

> **Build failed — n4od**
> `n4od/GitPulse · Notify probe failed`

A deliberately failing workflow on a scratch branch produced exactly one toast,
1.4 s after launch, one log line, and still one after a second poll cycle. A run
that succeeded produced none. The branch and both runs were deleted afterwards.

## Notifications are now visible in the log

One line per send, with the run id and attempt:

```
INFO gitpulse_lib::native::notifier: notification sent
     run=45199668294 attempt=1 workflow="Notify probe"
```

Without it, the only evidence a notification happened is a toast that has already
gone — which makes "exactly once" impossible to check, by you or by me.

## What was verified

- Notification fires for a real failed run, once, and not for a success.
- The failing count returned to zero when a passing run became the newest one.
- 187 backend and 204 frontend tests, including four backend tests and one
  frontend regression test for the card status. That frontend test was confirmed
  to fail with the fix reverted and pass with it restored.

## What is still not verified

- **A re-run arriving as a new attempt**, and the throttle collapsing a matrix
  build into one notification. Both remain unit-tested only.
- **macOS and Linux.** No update has been watched installing on either, and the
  tray, keychain and notification permission prompt only exist at run time there.
- **The in-app accessibility audit** has still not been run.

## What this does not change

- **The builds are still unsigned.** SmartScreen and Gatekeeper still warn on
  first launch. Auto-update is not code signing.
- **Nothing about how you get notified changed** — same conclusions, same
  dedupe, same throttle. Only the clock the quiet window is measured against.

---

## 📅 Recent Version History

- **v0.8.5**: One status per repository, quiet hours read your clock, notifications are logged.
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
