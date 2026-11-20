# Verification status

What has been proven, what has only been proven against a mock, and what has not
been proven at all. This file exists because "it has tests" is not the same
claim as "it works", and the difference matters more here than usual: Pulse
talks to a service that requires credentials, on platforms that are not all
available on the machine it was built on.

Last updated: 2026-11-20, after M10.

---

## Verified, with real evidence

| Claim                                                              | Evidence                                                                      |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| The app builds and starts                                          | Launched repeatedly; `pulse.exe` created, migrations applied, retention swept |
| Rust compiles clean                                                | `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`              |
| TypeScript compiles clean                                          | `tsc --noEmit` on two projects                                                |
| Linting clean                                                      | `eslint --max-warnings 0`                                                     |
| 168 backend tests pass                                             | `cargo test --all-targets`                                                    |
| 156 frontend tests pass                                            | `vitest run`                                                                  |
| Coverage on logic-bearing code                                     | 91.5% lines, 84.3% branches (gate: 80/75)                                     |
| Bundle under budget                                                | 115 KB gzipped shipped, against a 300 KB gate                                 |
| No npm advisories                                                  | `npm audit` — 0 vulnerabilities, prod and dev                                 |
| No token-shaped strings outside tests                              | `grep` over `src/` and `src-tauri/src/`                                       |
| Only one command returns a secret                                  | `webhook_signing_secret`, the documented setup path                           |
| CSP blocks `eval` and wildcards                                    | `tauri.conf.json` reviewed                                                    |
| Capabilities grant only `core:default` and `opener:allow-open-url` | `capabilities/default.json`                                                   |
| Webhooks work end to end                                           | Real loopback socket, real HTTP, signed and tampered payloads                 |
| The webhook verifier rejects forgeries                             | HMAC tests including a single flipped hex digit                               |
| Refresh is single-flight                                           | 20 concurrent callers produce exactly 1 request, asserted                     |
| A `304` costs no budget                                            | Asserted against a mock server                                                |
| The log redirect drops the token                                   | Asserted: second request carries no `Authorization`                           |
| Cascades actually delete                                           | Four child tables cleared by one repo deletion                                |
| A delivery is deduplicated                                         | Three identical deliveries, one row                                           |
| Render budget at 250 cards                                         | Real browser: 4.3 ms per card update, 7.4 ms for fifty                        |
| Path traversal is rejected                                         | Zero requests reach the network for `../../admin`                             |
| Accessibility of the primitives                                    | Lighthouse 1.00 on the component gallery                                      |

## Verified only against a mock

| Claim                               | Why the mock is a limitation                                                                                                        |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Sign-in completes                   | The device flow is exercised against `wiremock`; the real GitHub device page, token exchange and keychain write have never happened |
| Polling works                       | Run payloads are fixtures. GitHub's actual shapes, pagination and edge cases are unproven                                           |
| Re-run / cancel / dispatch          | Request shapes are asserted; GitHub's acceptance is not                                                                             |
| Job logs are readable               | The ZIP-vs-text discovery was made from documentation, not a live response                                                          |
| Rate-limit handling                 | `403`/`429` are simulated. Secondary-limit behaviour in practice is unobserved                                                      |
| Notifications fire once per failure | The dedupe key is unit-tested; no notification has ever been shown for a real build                                                 |

## Not verified at all

| Gap                                          | What it needs                                                                                                              |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **A real sign-in**                           | A registered GitHub App client id, which cannot be created on the user's behalf                                            |
| **A real poll**                              | The same                                                                                                                   |
| **A real webhook delivery**                  | A public relay or tunnel, plus the same                                                                                    |
| **macOS and Linux**                          | Hardware. The tray, dock badge, keychain backend and notification permission prompt are all platform-specific              |
| **CI on macOS and Linux**                    | A remote. No remote is configured for this repository                                                                      |
| **24-hour soak, 250 repositories**           | `scripts/soak.ps1` exists and is unexecuted                                                                                |
| **Idle CPU under 1%**                        | `scripts/idle-cpu.ps1` exists and is unexecuted                                                                            |
| **`cargo audit`**                            | Blocked here: the advisory database is fetched over git and this machine cannot reach it. Wired into CI, where it will run |
| **Notification click deep-link**             | Not implemented; the plugin exposes no click callback                                                                      |
| **Windows taskbar overlay icon**             | Not implemented; needs Win32 calls                                                                                         |
| **Settings panel and palette accessibility** | Lighthouse has not been run against them                                                                                   |

## How to close the largest gaps

1. **Register the app** (README, "GitHub authentication"), set
   `PULSE_GITHUB_CLIENT_ID`, and sign in. That converts roughly eight rows from
   "mock" to "real" in one step.
2. **Let it run overnight.** `scripts/soak.ps1 -Hours 8` turns the soak row into
   evidence, and the idle check takes ten minutes after that.
3. **Push to a remote.** The three-OS matrix has never executed; a single push
   would tell you more about macOS and Linux than every test in this repository.
4. **Point a tunnel at it** and let GitHub deliver one real webhook.

## Why this file is checked in

Because the alternative is a README that says "production-ready" and a user who
discovers otherwise. Every row above that says "not verified" is a claim this
project has not earned yet, and the honest version of a status page is the one
that admits it.
