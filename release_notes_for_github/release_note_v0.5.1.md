# Release v0.5.1 — Sign-In Fixed & Dashboard Refresh

The installers work on a fresh machine now, and the dashboard has been rebuilt to match the design. v0.5.0 shipped without a GitHub client id, which meant a new user could not sign in at all — this release fixes that at the build level, so nobody has to set anything.

## 🚀 Key Improvements

### 🐛 Sign-In

- **The installers can sign in**: the release workflow never supplied a client id, so the compile-time fallback resolved to nothing and every v0.5.0 installer asked for a client id it could not have. The value is now compiled into the binary at build time.
- **Verified in the shipped artifact**: the published Windows portable build was downloaded and its executable searched for the client id — found, at a real offset. This is checked against the binary, not inferred from the config.
- **No user setup**: a packaged build signs in on first launch; the runtime `GITPULSE_GITHUB_CLIENT_ID` override still wins for development.
- **Documented**: `RELEASING.md` now explains the repository variable, why a client id is a variable and not a secret, and that a missing one silently breaks sign-in.

### 🎨 Dashboard

- **One header row**: title, watch count, state filters, search and Refresh all share a single row, so the grid is never pushed down by a second toolbar.
- **Larger repository cards**: bold repository name, an uppercase status pill, `workflow · branch · sha`, and the run count beside the age and duration.
- **Line sparkline**: replaced the bar chart with a line whose height is still run duration — a rising line means builds are getting slower, and the stroke takes the newest run's status colour.
- **Status bar**: a new footer reports what the poller is doing and what the cache holds (`N repositories · M runs · X MB`).
- **API meter as a bar**: the quota readout is now a progress bar (`4,993 / 5,000`) that turns amber and then red as it drains.
- **Palette refresh**: emerald success, coral failure and amber running, with a larger card radius.

### 🔧 Stability & Measurement

- **Idle CPU measured, not assumed**: 0.04% average and 0.26% peak against a 1% budget, on the release build with a live session.
- **Token refresh verified across a restart**: the app reloaded its tokens from the OS keychain and rotated the access token with no re-sign-in.
- **The soak script no longer invents a leak**: 0.4 MB of allocator jitter over five minutes extrapolated to 4.27 MB/h; below a 30-minute window it now prints INCONCLUSIVE instead of failing.
- **Measurement scripts survive a cleanup**: both defaulted to a debug binary that a build cleanup deletes; they now prefer a release build and fall back to the newest kept binary.
- **`soak.ps1 -Minutes`**: short sanity runs no longer require a whole `-Hours`.
- **Idle-CPU label fixed**: it claimed "% of one core" while computing a percentage of total CPU.

### 🛠️ Developer & Docs

- **README banner and dashboard screenshot**, generated from checked-in SVG and HTML sources so they can be regenerated rather than becoming uneditable binaries.
- **`releases/` stays out of git**: built installers are kept on disk for convenience but never committed.
- **CHANGELOG cleanups**: duplicate release-history rows removed and the dead `[Unreleased]` link fixed.
- **The overnight soak is explicitly out of scope**, recorded as a decision rather than an open gap.

## ⚠️ Before You Upgrade

- **v0.5.0 cannot sign in.** If you installed v0.5.0, replace it with v0.5.1 — this is the release that fixes it.
- **These builds are unsigned.** macOS Gatekeeper and Windows SmartScreen will warn on first launch; right-click → Open on macOS, or More info → Run anyway on Windows.
- **Still unproven**: a real webhook delivery from GitHub, and runtime behaviour on macOS and Linux. `VERIFICATION.md` says which claims are measured and which are not.

---

## 📅 Recent Version History

- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU and token-refresh verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems, green CI.
- **v0.4.0**: Settings panel, command palette, diagnostics, and a hardening pass that produced `VERIFICATION.md`.
- **v0.3.0**: Webhook ingestion, tray icon, notifications, single instance, opt-in autostart.
- **v0.2.0**: Run inspector with job logs, re-run/cancel/dispatch, deployments and PR tabs.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
