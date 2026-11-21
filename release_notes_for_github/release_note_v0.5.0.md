# Release v0.5.0 — Installers, Portable Builds & Green CI

The first release you can actually install. GitPulse now builds installers **and** portable archives for Windows, macOS and Linux from a single tag, with no certificates or secrets required — and this is also the first release where the app has been verified against real GitHub.

## 🚀 Key Improvements

### 📦 Distribution & Release Pipeline

- **Installers on three operating systems**: NSIS `.exe` and `.msi` on Windows, `.dmg` on macOS (arm64 and x64), and `.deb` on Linux, all produced by a tag-triggered pipeline.
- **Portable builds alongside the installers**: a `.zip` containing the single Windows `.exe`, a `.tar.gz` of the macOS `.app`, and the Linux `.AppImage` — run GitPulse without installing anything or granting admin rights.
- **Releases need no secrets**: the pipeline produces installers with zero configuration. Signing is a documented upgrade, not a prerequisite.
- **Self-checking releases**: a `verify` job refuses to publish if any expected artifact is missing, so a silently incomplete release cannot happen.
- **Draft by default**: a release is never published without a human reading the notes and checking the artifacts first.
- **`RELEASING.md`**: how a tag becomes installers, every signing secret and where it comes from, and exactly what an unsigned build means for the person installing it.

### 🚀 Dashboard & Monitoring

- **Repository grid**: a card per watched repository showing the worst status of its recent runs, the latest run's workflow, branch, short SHA, age and duration, plus a sparkline of the last ten runs coloured by outcome.
- **Live updates**: `run://updated` carries a whole recomputed card, so a build finishing for one repository never refetches the other 249.
- **Instant filtering**: search by name and filter to failing or running, applied in the webview so typing never waits on a round trip.
- **Repository picker**: browse what your account can see and start watching repositories, with an immediate first poll when you track one.
- **Rate-limit meter**: remaining hourly quota read straight from GitHub's own response headers, not a local estimate.
- **Budget-aware poller**: one sequential loop that adapts its interval to the number of repositories you watch, so polling cannot exceed your hourly ceiling.
- **Always-virtualised grid**: the dashboard uses a single rendering path, removing the 272 ms stall that occurred when crossing between grid modes.

### 🔍 Runs, Logs & Actions

- **Run inspector**: select a repository to see its recent runs, the jobs of the selected run, and a log viewer — jobs and logs fetched only while someone is looking.
- **Log viewer**: virtualised rows, a search box with a live match count, jump-to-first-failure, and a banner when a download hits the 5 MB cap.
- **Actions**: re-run failed, re-run all, and cancel — each updating the card optimistically and rolling back with GitHub's own error message if it refuses.
- **Workflow dispatch**: trigger a `workflow_dispatch` with a chosen workflow and ref, with inputs as a validated JSON object.
- **Deployments and pull requests**: tabs in the inspector, refreshed from GitHub with each deployment's latest status.
- **Copy**: the whole log, or a run's URL, in one click.

### 🔔 Webhooks & Native Integration

- **Local webhook listener**: an `axum` server bound to `127.0.0.1` behind an unguessable path segment, so a stray local request gets a `404` rather than reaching the verifier.
- **Verified signatures**: HMAC-SHA256 over the raw bytes, compared in constant time, before a single byte is parsed — tampered bodies are rejected with `401`.
- **Idempotent delivery**: GitHub's `X-GitHub-Delivery` id is recorded, so a retry or relay replay is acknowledged and ignored rather than reprocessed.
- **Three transports**: a local listener, an SSE relay, or a public tunnel, with a reachability check that asks the URL whether it reaches this machine.
- **Tray icon**: four states — idle, running, failure, sign-in required — drawn at runtime from the same palette the UI uses.
- **Notifications**: on failure, timeout and startup failure, deduplicated per run attempt, throttled to one every ten seconds, with optional quiet hours that may wrap midnight.
- **Native behaviour**: a second launch focuses the existing window, window size and position survive a restart, closing hides to tray, and autostart is opt-in and off by default.
- **Dock badge**: macOS shows the current failure count on the dock icon.

### ⚙️ Settings & Polish

- **Settings panel**: notifications, quiet hours, close-to-tray, autostart, a plain-language explanation of the polling interval, diagnostics, cache clearing, webhooks and sign out.
- **Optimistic settings**: a checkbox moves immediately and is restored, with an explanation, if the backend refuses the change.
- **Command palette**: `Ctrl`/`Cmd`+`K` opens a subsequence search over repositories and actions; arrows and Enter navigate, Escape dismisses.
- **Diagnostics**: cache size, monitored repositories, cached runs, webhook deliveries, remaining budget and uptime, plus a "clear cache" that keeps your repositories and settings.
- **Sign out moved** into the settings panel and palette, away from the one mis-click next to the account name.

### 🛡️ Security & Privacy

- **Sign in with GitHub** via the OAuth 2.0 device flow: no client secret ships in the binary and no personal access token is ever requested.
- **Tokens live in the OS keychain**, held as secrets in memory and never written to the database, a log, or the frontend. The webview receives only your login name and expiry.
- **Crash-safe token rotation**: the refresh token is written before the access token, because GitHub invalidates the old one the moment it issues a new one.
- **Single-flight refresh**: twenty simultaneous requests produce exactly one network call.
- **Secret redaction**: classic and fine-grained GitHub tokens, bearer values, OAuth query parameters and signature headers are scrubbed before they can reach a log line or an error payload.
- **Locked-down webview**: CSP with no `unsafe-eval` and no wildcards, capabilities limited to the minimum, and `#![forbid(unsafe_code)]` on both crate roots.
- **No telemetry.** GitPulse talks to GitHub and nothing else.

### 🔧 Reliability

- **Sign-in survives a network blip**: a transient transport error during polling is retried within a bounded budget instead of aborting the whole sign-in.
- **Clearer failures**: an unknown client id now names the setting to check, and a rejected device code says the sign-in must be restarted instead of claiming it found a bug.
- **Relay stays connected**: the SSE transport no longer dies and reconnects every twenty seconds because of a shared HTTP timeout meant for API calls.
- **A relay that re-encodes the body is refused with a reason** — the bytes GitHub signed are gone, so the signature cannot match, and GitPulse says so rather than blaming an unknown frame.
- **Windows taskbar overlay icon** is implemented using Tauri's own API, closing a gap previously recorded as needing raw Win32 calls.
- **Silent-corruption guards**: `foreign_keys` is enabled on every connection so cascade deletes cannot leave orphaned rows, and deployment rows are written against the column the schema actually declares.

### 🛠️ Developer & Core

- **CI is green on all three operating systems** — clippy at `-D warnings`, the full test suite, and a format check on a clean checkout of `ubuntu-22.04`, `windows-latest` and `macos-latest`.
- **The macOS leg earned its keep immediately**: it caught `#[cfg(target_os = "macos")]` code that had never once been compiled on its target platform.
- **Pinned toolchain in CI**, so a new clippy release cannot break the build without a code change.
- **Dependency audit in CI**: each Rust advisory is reported as a readable annotation, and the npm audit runs at high/critical only.
- **Cleared a real CVE**: `rusqlite` 0.38 → 0.43 moves `libsqlite3-sys` to 0.40.0 and clears **CVE-2026-119375** in the bundled SQLite, with no source changes.
- **`webpki-root-certs` 1.0.7 → 1.0.12**, clearing the distrusted-intermediate advisory.
- **Generated bindings**: Rust types are exported with `ts-rs` and imported by the UI, so a TypeScript type cannot drift from the Rust one it mirrors.
- **Live verification harness**: `examples/live_device_flow` runs the device flow against real GitHub, printing your login and the token's shape but never the token.

## ⚠️ Before You Install

- **These builds are unsigned.** On macOS, Gatekeeper will block the app on first launch — right-click → **Open**, or run `xattr -dr com.apple.quarantine /Applications/GitPulse.app`. On Windows, SmartScreen will warn; choose **More info → Run anyway**. Removing this warning needs paid code-signing certificates.
- **`v1.0.0` is deliberately reserved.** The project defines it as signed installers with auto-update; neither holds yet, so releases carry `0.x` rather than assert otherwise.
- **Some things are verified and some are not**, and `VERIFICATION.md` says which is which. A real sign-in and live polling have been confirmed against GitHub; a real webhook delivery has not, and the 24-hour soak and idle-CPU scripts have been written but not yet executed.

---

## 📅 Recent Version History

- **v0.5.0**: First installable release — installers and portable builds on three OSes, green CI, a SQLite CVE cleared, and live GitHub verification.
- **v0.4.0**: Settings panel, command palette, diagnostics, and a hardening pass that produced `VERIFICATION.md`.
- **v0.3.0**: Webhook ingestion, tray icon, notifications, single instance, and opt-in autostart.
- **v0.2.0**: Run inspector with job logs, re-run/cancel/dispatch, and deployments/PR tabs.
- **v0.1.0**: The read-only dashboard — sign in, pick repositories, watch status update live.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
