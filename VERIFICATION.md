# Verification status

What has been proven, what has only been proven against a mock, and what has not
been proven at all. This file exists because "it has tests" is not the same
claim as "it works", and the difference matters more here than usual: GitPulse
talks to a service that requires credentials, on platforms that are not all
available on the machine it was built on.

Last updated: 2026-11-20, after M10.

---

## Verified, with real evidence

| **A real device flow completed** | A human authorised the app in a browser; the access token, refresh token and metadata landed in Windows Credential Manager |
| **The stored token works** | `GET /user` with the keychain token returned the account that authorised the app |
| **The endpoint layer works live** | `GET /user/repos` returned 2 real repositories; `GET /actions/runs` returned 2 real runs with statuses and conclusions |
| **Polling writes real data** | The running app polled both repos; SQLite holds 2 repos, 2 runs and fresh `last_polled_at` timestamps |
| **Transient network blips are survivable** | Earlier live polling failed repeatedly; after the retry fix, 75 seconds of live polling produced zero failures |

| Claim                                                              | Evidence                                                                                                                               |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| The app builds and starts                                          | Launched repeatedly; `gitpulse.exe` created, migrations applied, retention swept                                                       |
| Rust compiles clean                                                | `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`                                                                       |
| TypeScript compiles clean                                          | `tsc --noEmit` on two projects                                                                                                         |
| Linting clean                                                      | `eslint --max-warnings 0`                                                                                                              |
| 175 backend tests pass                                             | `cargo test --all-targets`                                                                                                             |
| 156 frontend tests pass                                            | `vitest run`                                                                                                                           |
| Coverage on logic-bearing code                                     | 91.5% lines, 84.3% branches (gate: 80/75)                                                                                              |
| Bundle under budget                                                | 115 KB gzipped shipped, against a 300 KB gate                                                                                          |
| No npm advisories                                                  | `npm audit` — 0 vulnerabilities, prod and dev                                                                                          |
| No token-shaped strings outside tests                              | `grep` over `src/` and `src-tauri/src/`                                                                                                |
| Only one command returns a secret                                  | `webhook_signing_secret`, the documented setup path                                                                                    |
| CSP blocks `eval` and wildcards                                    | `tauri.conf.json` reviewed                                                                                                             |
| Capabilities grant only `core:default` and `opener:allow-open-url` | `capabilities/default.json`                                                                                                            |
| Webhooks work end to end                                           | Real loopback socket, real HTTP, signed and tampered payloads                                                                          |
| The webhook verifier rejects forgeries                             | HMAC tests including a single flipped hex digit                                                                                        |
| Refresh is single-flight                                           | 20 concurrent callers produce exactly 1 request, asserted                                                                              |
| A `304` costs no budget                                            | Asserted against a mock server                                                                                                         |
| The log redirect drops the token                                   | Asserted: second request carries no `Authorization`                                                                                    |
| Cascades actually delete                                           | Four child tables cleared by one repo deletion                                                                                         |
| A delivery is deduplicated                                         | Three identical deliveries, one row                                                                                                    |
| Render budget at 250 cards                                         | Real browser: 4.3 ms per card update, 7.4 ms for fifty                                                                                 |
| Path traversal is rejected                                         | Zero requests reach the network for `../../admin`                                                                                      |
| Accessibility of the primitives                                    | Lighthouse 1.00 on the component gallery                                                                                               |
| **The device-code request works against real GitHub**              | `examples/live_device_flow` with a real client id returned a user code, a verification URI, a 5-second interval and a 15-minute expiry |
| **GitHub accepted our polling shape**                              | The same run's first `POST /login/oauth/access_token` returned `authorization_pending`                                                 |
| **Device flow is enabled on the app registration**                 | Implied by the two rows above — GitHub refuses `device_flow_disabled` apps at the first call                                           |
| **An unknown client id produces a usable error**                   | Live: GitHub answers `404`, and GitPulse now reports "GitHub does not recognise this client id" instead of "resource not found"        |
| **A rejected device code produces a usable error**                 | Live: `incorrect_device_code` is reported as "the sign-in has to be restarted"                                                         |
| **A transient network failure no longer aborts sign-in**           | Found live — a real blip aborted a real sign-in — then fixed with a bounded retry budget and unit tests                                |
| **A new migration applies to a live database**                     | `0004_repo_error_detail` applied on launch to the real `gitpulse.db`; `last_error_code` and `last_error_at` are queryable afterwards   |
| **The right-click menu opens at the pointer**                      | Confirmed in the running app at several scroll positions, after the virtualised-row transform was ruled out as the containing block    |

## Verified only against a mock

| Claim                               | Why the mock is a limitation                                                                                                                                                                   |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Re-run / cancel / dispatch          | Request shapes are asserted; GitHub's acceptance is not                                                                                                                                        |
| Job logs are readable               | The ZIP-vs-text discovery was made from documentation, not a live response                                                                                                                     |
| Rate-limit handling                 | `403`/`429` are simulated. Secondary-limit behaviour in practice is unobserved                                                                                                                 |
| Notifications fire once per failure | The dedupe key is unit-tested; no notification has ever been shown for a real build                                                                                                            |
| The per-repository error state      | Rendered from a failure written straight into the database. A real API failure has not yet been observed end to end, so the wording and the clearing behaviour are unexercised by real traffic |

## Not verified at all

| Gap                                                 | What it needs                                                                                                                                                                                              |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A real webhook delivery**                         | A public relay or tunnel, plus the same                                                                                                                                                                    |
| **macOS and Linux runtimes**                        | Hardware. CI now builds and tests both, but the tray, dock badge, keychain backend and notification permission prompt only exist at run time                                                               |
| ~~24-hour soak, 250 repositories~~ **out of scope** | **Deliberately not scheduled.** `scripts/soak.ps1` was run for 5 minutes on a live 2-repo session and RSS held at ~33 MB; no per-hour rate is claimed, and the 250-repository figure is not being pursued. |
| ~~Idle CPU under 1%~~ **measured**                  | **Verified** on the release build with a live session: average **0.04%**, peak **0.26%** of total CPU over 10 minutes against a 1% budget.                                                                 |
| **Notification click deep-link**                    | Not implemented; the plugin exposes no click callback                                                                                                                                                      |
| **Windows taskbar overlay icon**                    | Implemented with Tauri's `set_overlay_icon`; nobody has seen it on a real taskbar                                                                                                                          |
| **Settings panel and palette accessibility**        | Lighthouse has not been run against them                                                                                                                                                                   |

**No longer on this list:** the release pipeline. It has now run four times. Each
`v0.5.x` tag built installers and portable archives on `ubuntu-22.04`,
`windows-latest` and `macos-latest`, and the `verify` job confirmed every
expected artifact was present before anything was published.

The bundles are still **unsigned** - no certificate exists - so this is "the
pipeline works", not "the app is release-ready". The releases are **published**,
not drafts, and each one's download links were followed through GitHub's
redirect to the CDN and confirmed to return bytes rather than merely being
listed in the API.

**`0.5.6` reshaped a release, and that shape has now run once.** Each artifact is
published once, under a stable name with no version in it
(`GitPulse-windows-x64-setup.exe`), and the _workflow_ names and uploads them
rather than `tauri-action` — because the action attaches a macOS `.app.tar.gz`
that nothing reads and no input disables. `verify` asserts the exact set of nine
instead of pattern-matching, checks that the README links to all nine, and
publishes a `SHA256SUMS.txt` built from a re-download of every asset.

The `v0.5.6` run was its first exercise and passed: six jobs green, a draft of
exactly ten assets and no phantom tarballs, and the published release's Windows
installer was downloaded through `releases/latest/download/` and its SHA-256
compared against the published manifest — a match. The version resource in that
same installer reads `0.5.6`, which is what makes dropping the version from the
filename safe: the name stays stable for the link, the version travels inside the
package.

The first attempt at `0.5.6` failed, informatively: `verify` refused to publish —
twelve assets instead of nine. The cause was the workflow pinning `tauri-action@v0`
while setting two inputs that only exist in its v1 line, which the action ignored
silently. The exact-set assertion and the checksum step are what made that visible
rather than shipping twenty wrong assets.

## How to close the largest gaps

1. **Register the app** (README, "GitHub authentication"), set
   `GITPULSE_GITHUB_CLIENT_ID`, and sign in. That converts roughly eight rows from
   "mock" to "real" in one step.
   _In progress:_ the client id is configured and the device-code half of the
   flow is verified against GitHub. Completing an authorisation converts the
   token, refresh and `/user` rows as well.
2. **Idle CPU: done.** `scripts/idle-cpu.ps1` has been run — 0.04% average
   against a 1% budget. The overnight soak is deliberately out of scope and is
   no longer listed as a next step.
3. ~~**Push to a remote.**~~ **Done.** The three-OS matrix runs green on every
   push to `main`, and it immediately found a compile error on macOS that no
   amount of testing on Windows could have surfaced. That closed the CI row and
   the build half of the platform row in one step.
4. **Point a tunnel at it** and let GitHub deliver one real webhook.

## Why this file is checked in

Because the alternative is a README that says "production-ready" and a user who
discovers otherwise. Every row above that says "not verified" is a claim this
project has not earned yet, and the honest version of a status page is the one
that admits it.

---

## Acceptance criteria

Ten criteria were written at the start of the project. This is the honest score.

| #   | Criterion                                                                                        | Result                                                                                                                                                                                                                                                                                                                                      |
| --- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | A clean clone builds on all three operating systems                                              | **Verified in CI.** `ubuntu-22.04`, `windows-latest` and `macos-latest` all green: clippy `-D warnings`, 171 tests and a format check on a clean checkout. The first CI run failed on all three for different reasons, and the macOS leg could not even compile `#[cfg(target_os = "macos")]` code that had never been built on its target. |
| 2   | Device flow completes; the token appears in no file, log or frontend payload                     | **Verified.** Real sign-in. Grep of `dist/assets/*.js` and the log files finds no token; the keychain holds it.                                                                                                                                                                                                                             |
| 3   | A crash mid-refresh never leaves a stale refresh token                                           | **Partial.** The write ordering (refresh before access) is unit-tested and no crash was induced. The refresh path itself is now verified live: the app restarted, reloaded its tokens from the OS keychain, and rotated the access token with no re-sign-in.                                                                                |
| 4   | A tampered webhook returns 401; a valid one returns 202 and updates without a manual refresh     | **Partial.** Both verified against a real socket. No delivery from GitHub has arrived, because that needs a public tunnel.                                                                                                                                                                                                                  |
| 5   | 250 repositories stay under 3,500 requests/hour                                                  | **Partial; the soak is out of scope.** The interval arithmetic and the token bucket are tested, and a live 2-repo session polled without exceeding the local ceiling. The 24-hour/250-repository soak is deliberately not planned, so the 250-repository claim remains unproven.                                                            |
| 6   | Every row of the status table is unit-tested                                                     | **Verified.** 14 rows, plus precedence and fallback cases.                                                                                                                                                                                                                                                                                  |
| 7   | Tray reflects a completed run within 2s; exactly one notification per failed run                 | **Partial.** The state function and the dedupe key are unit-tested; neither has been observed against a real build.                                                                                                                                                                                                                         |
| 8   | Every performance budget is measured by a script and reported in CI                              | **Partial.** `scripts/idle-cpu.ps1` has been executed against the release build — 0.04% average against a 1% budget. `scripts/soak.ps1` exists but the overnight soak is out of scope, and neither script is wired into CI. CI verifies the bundle budget and the dependency audit.                                                         |
| 9   | Architecture, registration and trade-offs are documented                                         | **Verified.** README, `ARCHITECTURE.md` (18 ADRs), `RELEASING.md` and this file.                                                                                                                                                                                                                                                            |
| 10  | Zero `unwrap`, `panic!`, `TODO`, `any`, `dangerouslySetInnerHTML` or `unsafe` in production code | **Two exceptions.** `redact.rs` calls `.expect()` on two compile-time-constant regexes. Deliberate: a bad pattern is a build-time bug, and the alternatives are a silent fallback that leaks secrets or an unreadable `match`. Everything else is clean; 214 further occurrences are all inside test modules.                               |

**Score: 3 verified, 5 partial, 1 not met, 1 with documented exceptions.**

Nothing here is a surprise. The partials cluster on one cause — no tunnel, no
second machine, no signing certificates — and they would collapse quickly with
those three things. The remote, which was on that list, is now in place: CI runs
green on all three platforms and caught a real macOS-only compile error the
moment it first ran.
