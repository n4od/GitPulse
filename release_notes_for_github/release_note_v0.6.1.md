# Release v0.6.1 — Dependency Hygiene

**The application is unchanged from v0.6.0.** This release exists so that the
installers carry a dependency the advisory database is content with.

If you already run v0.6.0, there is no functional reason to reinstall.

## 🔐 What changed, and why it mattered

`url` is now **2.5.9**, which patches
[RUSTSEC-2026-0592](https://github.com/servo/rust-url/pull/1186) — a stack
overflow in `Url::origin()` when parsing deeply nested `blob:` URLs.

**The affected function was never reachable from GitPulse.** The app calls
`Url::parse` twice: once to validate a URL before handing it to the OS browser
opener, once to build links in the log viewer. `Url::origin()` is not called
anywhere, so v0.6.0 could not trigger the flaw. This release is for whoever scans
a downloaded installer next and would otherwise have to work that out.

Worth recording for the future: the advisory was published on **2026-11-20**, and
CI went red the following day against a repository nobody had changed, because
the audit reads the advisory database on every run. **A red dependency audit does
not mean someone edited a dependency.**

## 🛠️ The audit now says what it found

The failing job's log used to read:

```
1 vulnerabilities
##[error]Process completed with exit code 1.
```

...and nothing else. Each advisory was emitted only as a `::error` annotation,
which GitHub's runner consumes into the UI, so the crate, the advisory and the
fix were visible only through the checks API. The step now prints the finding as
plain text as well, and ends by naming the remedy.

## ⚠️ What did not change

- **The application is identical to v0.6.0.** Only the dependency and the version string differ.
- **The builds are still unsigned.** Windows SmartScreen and macOS Gatekeeper ask for confirmation on first launch.
- **There is still no auto-update.**
- **Still unproven**: a real webhook delivery from GitHub, the notification click deep-link, and runtime behaviour on macOS and Linux. [`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md) records exactly which claims are measured and which are not.

---

## 📅 Recent Version History

- **v0.6.1**: `url` 2.5.9 for RUSTSEC-2026-0592, which was never reachable from the app; the audit now names what it finds.
- **v0.6.0**: Multiple accounts, switchable without signing in again; per-account tokens and cache; the tray and notifications name the account.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, `Mine` filter, a way back into the picker, and the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU and token-refresh verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems, green CI.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
