# Changelog

All notable changes to **Pulse** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Scope:** this file records what has actually shipped or landed on `main`.
Forward-looking work lives in [ROADMAP.md](./ROADMAP.md) — do not duplicate it here.
Every entry should be traceable to a roadmap task ID (`M2-05`, etc.) where one exists.

---

## [Unreleased]

### Verified against GitHub

- **A real sign-in completed.** The device flow was authorised in a browser; the
  access token, refresh token and metadata are in the OS keychain, and a real
  `GET /user` returned the account.
- **Live polling works end to end.** `GET /user/repos` returned the account's
  repositories and `GET /actions/runs` returned real runs; the running app cached
  both with fresh poll timestamps. This was previously mock-only.

### Fixed

- **A network blip aborted sign-in.** Found by running the device flow against
  real GitHub, not against a mock: the first poll succeeded, the next hit a
  transport error, and the whole sign-in failed even though the device code was
  still valid for another ten minutes. Transient failures during polling are now
  retried with a bounded budget that resets on any successful poll.
- **An unknown client id reported "resource not found".** GitHub answers a 404
  for an unrecognised client id rather than an OAuth error body, so the message
  pointed at the wrong problem. It now names the setting to check and the
  registration requirement.
- **A rejected device code reported "this is a bug in Pulse".** It is not: it
  means the code was not one GitHub issued, or it was mangled. The message now
  says the sign-in has to be restarted.

### Added

- **`examples/live_device_flow`:** a repeatable check against real GitHub, with
  `--wait` to complete an authorisation by hand and `--errors` to verify the
  error paths. It prints the login and the token's shape, never the token.

---

## [0.4.0] — 2026-11-20

Polish and hardening: a settings panel, a command palette, and an honest account
of what is actually verified.

### Added

**Settings and polish (M10)**

- **Settings panel:** notifications, quiet hours, close-to-tray, autostart, a
  plain-language explanation of the polling interval, diagnostics, cache
  clearing, a route into the webhook panel, and sign out. (`M10-02`)
- **Optimistic settings:** a checkbox moves immediately and is restored, with an
  explanation, if the backend refuses the change. (`M10-01`)
- **Command palette:** `Ctrl`/`Cmd`+`K` opens a subsequence search over
  repositories and actions; arrows and Enter navigate, Escape dismisses.
  (`M10-03`)
- **Diagnostics:** cache size, monitored repositories, cached runs, webhook
  deliveries, remaining local budget and uptime, plus a "clear cache" that keeps
  repositories and settings. (`M10-04`)
- **Keyboard map:** the palette and its navigation, alongside the focus-visible
  treatment every control already had. (`M10-05`)

**Hardening (M11)**

- **Resource measurement scripts:** `scripts/soak.ps1` reports memory growth per
  hour against a 2 MB/h budget, and `scripts/idle-cpu.ps1` reports average CPU
  against 1%. Both are written and neither has been run. (`M11-01`, `M11-02`)
- **`VERIFICATION.md`:** every claim in the repository classified as verified
  with evidence, verified only against a mock, or not verified — with the four
  gaps that need credentials, a second platform, or a remote called out
  explicitly.
- **Dependency audit in CI:** `npm audit --audit-level=high` and
  `rustsec/audit-check`. (`M11-07`)

### Changed

- **Sign out moved into the settings panel and the palette.** It used to be a
  title-bar button beside the account name, one mis-click from logging you out.

### Security

- **Audited, not assumed:** no token-shaped strings outside test modules, exactly
  one command returns a secret (the documented webhook setup path), CSP has no
  `unsafe-eval` and no wildcards, capabilities grant only `core:default` and
  `opener:allow-open-url`, and `npm audit` reports zero vulnerabilities.
  (`M11-06`)

### Known issues

- **The settings panel and palette have not been through Lighthouse.** The
  primitives were audited at 1.00 in M1 and the new surfaces reuse the same
  tokens and roles, but "same components" is not a measurement. (`M10-06`)
- **The perf bench is not in CI** and there is no Playwright e2e suite;
  `M11-03` and `M11-04` were left open rather than half-built. The bench remains
  a manual `#/bench` run.
- **`cargo audit` could not run on the build machine** — the advisory database
  is fetched over git and that network path is blocked. It is wired into CI,
  where it will run.
- **The soak and idle-CPU scripts have not been executed,** so the 2 MB/h and
  1% claims in the roadmap are budgets, not measurements.

---

## [0.3.0] — 2026-11-20

Pulse becomes a real desktop citizen: a tray icon that reports status at a
glance, notifications that respect attention, and webhook ingestion.

### Added

**Native integration (M9)**

- **Tray icon** with four states — idle, running, failure, sign-in required —
  drawn at runtime from the same palette the UI uses, with a matching tooltip.
  (`M9-01`)
- **Tray menu:** open, pause or resume polling, mute notifications for an hour,
  and quit. Left click toggles the window; the menu is the right-click action.
  (`M9-02`)
- **Notifications** on failure, timeout and startup failure, deduplicated per
  `(run id, attempt)` and throttled to one every ten seconds, with an optional
  quiet-hours window that may wrap midnight. (`M9-03`)
- **Single instance and window state:** a second launch focuses the existing
  window; size and position survive a restart. (`M9-05`)
- **Opt-in autostart**, off by default, applied immediately rather than at the
  next launch. (`M9-06`)
- **Close to tray:** closing the window hides it instead of exiting, which is
  what a monitor has to do to be useful. (`M9-08`)
- **macOS dock badge** showing the failure count. (`M9-07`)
- **Poller pause** that keeps the schedule intact, so resuming does not rebuild
  it.

**Webhook ingestion (M8)**

- **Local listener:** an `axum` server bound to `127.0.0.1`, addressed through an
  unguessable path segment so a stray local POST gets a 404 rather than reaching
  the verifier. (`M8-01`)
- **Signature verification:** HMAC-SHA256 over the raw bytes, compared in
  constant time, before a single byte is parsed. Malformed headers, wrong
  secrets and tampered bodies are all rejected with `401`. (`M8-02`)
- **Body limits:** GitHub's own 25 MB ceiling, enforced by the extractor before
  the body is buffered; non-JSON content types get `415`. (`M8-03`)
- **Idempotent delivery:** `X-GitHub-Delivery` is recorded, so a GitHub retry or
  a relay replay is acknowledged and ignored rather than reprocessed. (`M8-04`)
- **Fast acknowledgement:** `202` immediately, work on a spawned task, because
  GitHub retries anything slower than ten seconds. (`M8-05`)
- **Relay transport:** a long-lived SSE connection with exponential backoff,
  using the same verifier. A relay that does not forward headers produces frames
  Pulse refuses — deliberately. (`M8-07`)
- **Tunnel support:** local mode plus a reachability check that asks a public
  URL whether it reaches this listener. (`M8-08`)
- **Webhook panel:** transport selection, copyable endpoint, secret reveal and
  rotation, delivery count and last-delivery time, tunnel test. (`M8-09`)
- **Shared keychain access:** one module now owns keychain error mapping, used by
  both token storage and the webhook secret.

### Changed

- **A delivery refreshes; it does not patch.** The router records the delivery
  and asks the poller to refresh that repository instead of applying the payload
  to the cache. One extra API request buys a single tested mapping, no second
  writer racing the poller, and a cache that cannot be left wrong by a payload
  Pulse misreads. (`M8-06`)

### Known issues

- **No delivery from GitHub has arrived.** The verifier is tested against
  known-good HMACs and the listener against real HTTP, but GitHub's actual
  delivery — and whether a given relay forwards the headers Pulse needs — is
  unverified.
- **Notification click does not deep-link to the run.** The notification plugin
  has no click callback, and a per-platform implementation was not written.
- **No Windows taskbar overlay icon.** The macOS dock badge is wired; the
  Windows equivalent needs Win32 calls that were not written.
- **macOS and Linux are untested.** The tray, dock badge and notification
  permission prompt are all platform-specific.

---

## [0.2.0] — 2026-11-20

Inspect runs, read their logs, and act on them.

### Added

**Runs, logs and actions (M7)**

- **Run inspector:** selecting a repository opens a panel with its recent runs,
  the jobs of the selected run, and a log viewer. Jobs and logs are fetched on
  demand — they are only interesting while someone is looking. (`M7-01`,
  `M7-02`)
- **Log viewer:** virtualised rows, a search box with a live match count,
  jump-to-first-failure, and a banner when the download hit the 5 MB cap.
  Rows cap at 20,000 to keep the viewer responsive on pathological logs.
  (`M7-03`)
- **Actions:** re-run failed, re-run all, cancel — each updating the status
  optimistically and restoring the previous card if GitHub refuses, with the
  GitHub error surfaced verbatim. (`M7-04`)
- **Dispatch:** trigger a `workflow_dispatch` with a chosen workflow and ref.
  Inputs are a validated JSON object. (`M7-05`)
- **Copy:** the whole log, or a run's URL. (`M7-06`)
- **Deployments and pull requests:** tabs in the inspector, refreshed from
  GitHub with the deployment's latest status. (`M7-07`)

### Changed

- **Job logs, not run archives.** GitHub's run-level log endpoint returns a ZIP;
  only the job endpoint returns text, so the viewer reads job logs. (`M7-01`)
- **Jobs are replaced per run.** GitHub issues new job ids on every re-run, so
  an upsert would leave the old attempt's jobs in the list; the run's jobs are
  deleted and rewritten in one transaction. (`M7-01`)

### Known issues

- None of the actions has been exercised against GitHub; they are covered
  against mocks only.

---

## [0.1.0] — 2026-11-20

The first release: sign in, choose repositories, and watch workflow status
update live.

**This is a read-only dashboard.** Inspecting runs, reading logs, re-running and
dispatching workflows arrive in `0.2.0`. A live sign-in against GitHub has not
been performed — see Known issues — so treat this as a build milestone rather
than something to install and rely on.

### Added

**Dashboard (M6)**

- **Repository grid:** a card per watched repository showing the worst status of
  its recent runs, the latest run's workflow, branch, short SHA, age and
  duration, plus a sparkline of the last ten runs coloured by outcome. (`M6-02`,
  `M6-03`)
- **Live updates:** `run://updated` carries a whole recomputed card, so the
  cache is patched in place. A build finishing for one repository does not
  refetch the other 249. (`M6-07`)
- **Filtering:** search by name and filter to failing or running, applied in the
  webview so typing is instant. (`M6-05`)
- **Repository picker:** browse what the signed-in account can see and start
  watching individual repositories, with an immediate first poll on tracking.
  (`M6-01`)
- **Rate-limit meter:** remaining hourly quota and poller state in the title bar,
  read from GitHub's own response headers rather than a local estimate. (`M6-06`)
- **Generated bindings:** `Repo`, `RepoCard`, `WorkflowRun`, `AvailableRepo`,
  `PollerStatus` and `RateLimitSnapshot` are exported from Rust with `ts-rs`;
  the UI imports them rather than redeclaring them. (`M6-01`)
- **Dev-only render bench** at `#/bench`: 250 synthetic repositories, a React
  `Profiler`, and p50/p95/max commit timings. Compiled out of production builds.
  (`M6-08`)

**Poller (M5)**

- **Scheduler:** one loop that wakes every 30 seconds, picks the repositories
  that are due, and polls them in priority order — manual refresh first, then
  repositories with a run in flight, then alphabetically. (`M5-01`)
- **Budget-aware interval:** `clamp(ceil(repos × 3600 / ceiling), 60s, 15min)`
  keeps the sweep's plan inside the hourly ceiling. (`M5-02`)
- **Manual refresh:** `repos_refresh` queues a repository ahead of the schedule;
  repeated requests while one is pending collapse into a single poll. (`M5-03`)
- **Adaptive intervals:** a repository with a run in flight polls at the floor
  interval; an idle one drifts to the budget-derived interval. (`M5-04`)
- **Diff-only events:** `run://updated` carries only runs whose `status`,
  `conclusion` or `run_attempt` changed, so a quiet poll produces no traffic to
  the UI at all. (`M5-05`)
- **Status events:** `poller://status` reports the monitored count, target
  interval, remaining local budget and the last error. (`M5-07`)
- **Graceful stops:** a rate limit, a revoked token or a signed-out session ends
  the sweep instead of retrying against every remaining repository. (`M5-03`)

**GitHub client (M4)**

- **Typed REST surface:** repositories, a repository, runs, a run, jobs,
  workflows, dispatch, re-run, re-run-failed, cancel, deployments, deployment
  statuses, open pull requests and `/rate_limit`. (`M4-06`)
- **Conditional requests:** an LRU(512) `ETag` cache sends `If-None-Match`, and
  a `304` is served from the cache with its budget token refunded — GitHub does
  not charge for a `304`, so neither do we. (`M4-02`)
- **Local request budget:** a token bucket at 70% of the hourly ceiling
  (3,500/hour, burst 30) so polling cannot starve the reserve. Waits longer
  than 30 seconds surface as `rate_limited` instead of blocking. (`M4-03`)
- **Rate-limit handling:** primary exhaustion pauses until GitHub's reset time;
  a secondary limit (`429` or `403` + `Retry-After`) pauses the _whole_ client,
  not just the request that hit it. Snapshots are pushed to the UI on
  `ratelimit://updated`. (`M4-04`)
- **Retry policy:** only `5xx` and transport failures, at most four attempts,
  exponential backoff with full jitter. A `4xx` is never retried. (`M4-05`)
- **Log download:** follows the pre-signed CDN redirect by hand and re-issues
  the request **without** the `Authorization` header, streaming the body against
  a 5 MB cap. (`M4-07`)
- **Input validation:** repository names and git refs are rejected before a URL
  is built, so a `..` or a stray `/` cannot address a different endpoint.
- **Generated types:** `RateLimitSnapshot` is exported to
  `src/lib/bindings/RateLimitSnapshot.ts` for the rate-limit meter.

**Authentication (M2)**

- **Sign in with GitHub** via the OAuth 2.0 device flow: the app requests a
  device code, opens the verification page in the default browser, and shows
  the user code with a live countdown and a copy button. No client secret ships
  and no personal access token is ever requested. (`M2-02`, `M2-04`)
- **One flow, both app types.** The token handling branches on what GitHub
  actually returns, so a GitHub App (expiring tokens plus rotating refresh
  tokens) and an OAuth App (non-expiring token, no refresh) both work with no
  code fork. (`M2-12`)
- **Token custody:** access and refresh tokens live in the OS keychain behind a
  `TokenStore` trait, held as `SecretString` in memory and never serialised to
  the frontend, the database or a log. (`M2-05`)
- **Crash-safe rotation:** the refresh token is written before the access token,
  because GitHub invalidates the old refresh token the moment it issues a new
  one — the reverse order would strand a user mid-refresh. (`M2-05`)
- **Single-flight refresh:** concurrent callers queue behind one in-flight
  refresh and re-check after acquiring the gate, so twenty simultaneous requests
  produce one network call. (`M2-06`)
- **Session validation:** `GET /user` on startup; a `401` clears the keychain
  and returns to the sign-in screen instead of looping on a dead token.
  (`M2-07`)
- **Keychain failure handling:** when the OS keychain is unavailable (a Linux
  session with no Secret Service, typically) Pulse degrades to session-only
  in-memory storage rather than writing secrets to disk. (`M2-08`)
- **Auth UI:** a presentational `AuthPanel` covering every phase — signing out,
  requesting a code, awaiting approval, polling, denied, expired and failed —
  plus a `DeviceCodeCard` with countdown, copy and reopen-browser actions.
  (`M2-10`)
- **Generated types:** `AuthState` and `AuthSession` are exported from Rust to
  `src/lib/bindings/` with `ts-rs` and imported by the UI, so the TypeScript
  union cannot drift from the Rust enum.

**Persistence (M3)**

- **Local cache:** normalized `SQLite` schema for repositories, workflow runs,
  jobs, deployments, pull requests and webhook deliveries, with indexes on the
  dashboard's hot paths and `ON DELETE CASCADE` from a repository to everything
  it owns. (`M3-01`, `M3-02`)
- **Migrations:** forward-only `.sql` versions embedded at compile time and
  recorded in a `schema_migrations` ledger, applied automatically and
  idempotently on every launch. (`M3-03b`)
- **DAOs** for every table, all writes expressed as idempotent
  `ON CONFLICT DO UPDATE` upserts keyed on the GitHub numeric id. (`M3-04`)
- **Change detection:** `diff_runs()` fingerprints `status`, `conclusion` and
  `run_attempt` and writes only the runs that actually moved, so an unchanged
  poll produces no writes and no UI events. (`M3-05`)
- **Retention:** runs older than 90 days and deliveries older than 30 days are
  pruned on startup, with `VACUUM` throttled to once every 30 days because it
  rewrites the whole file. (`M3-06`)
- **Delivery ledger:** `X-GitHub-Delivery` ids are recorded so a replayed
  webhook is rejected rather than reprocessed. (`M3-02`)
- **Pull-request sweeping:** open pull requests not reported by the latest sync
  are removed, so a PR merged upstream stops showing as open locally. (`M3-04`)

**Design system (M1)**

- **Primitives:** `Button` (4 variants × 2 sizes, loading and disabled states),
  `Badge`, `Card` / `CardHeader` / `CardBody`, `Spinner`, `Skeleton`, `Tooltip`,
  and a toast region with a capped queue. (`M1-02`)
- **Status vocabulary:** `mapRunStatus()` maps every GitHub `status`/`conclusion`
  pair to a label, tone and pulse behaviour, plus `worstStatus()` to collapse a
  repository's runs into a single badge. `StatusBadge` and `StatusPulse` are the
  only components that render status, so the dashboard, tray and notifications
  cannot drift apart. (`M1-05`, `M1-06`)
- **App shell:** title bar, routed content area, empty state and error boundary.
  (`M1-03`)
- **Routing:** dependency-free hash router with a dev-only component gallery at
  `#/kitchen-sink` that renders every primitive in every state. (`M1-07`)
- **Motion policy:** `useMotionAllowed()` gates every looping animation on both
  `prefers-reduced-motion` and window visibility, so a hidden window does no
  animation work. (`M1-04`)
- **Dark-first palette** with status colours and radii published as Tailwind
  theme tokens. (`M1-01`)

**Foundation (M0)**

- **Project scaffold:** Tauri 2 desktop shell with React 19, TypeScript 6 and
  Tailwind CSS 4, plus a native window that starts hidden and shows on ready.
  (`M0-02`, `M0-03`, `M0-04`)
- **Typed IPC boundary:** `src/lib/ipc.ts` + `src/lib/errors.ts` normalise every
  backend failure into a `PulseError` with a stable `code`. (`M0-07`)
- **Backend error contract:** `AppError` serialises to `{ code, message,
retryable }` and redacts secrets on the way out. (`M0-07`)
- **Logging:** daily-rotating file log in the platform log directory, console
  output in debug builds, `RUST_LOG` override. (`M0-08`)
- **Secret redaction:** classic and fine-grained GitHub tokens, bearer values,
  OAuth query parameters and signature headers are scrubbed before a log line
  or error payload leaves the process. (`M0-08`)
- **System commands:** `system_info` and `system_open_url`, the latter
  allowlisted to `https://*.github.com`. (`M0-07`)
- **CI:** frontend job (typecheck, lint, tests with coverage, build, 300 KB
  gzipped budget) and a three-OS backend matrix (fmt, clippy, tests). (`M0-09`)
- **Docs:** `ARCHITECTURE.md` (decision log, threat model, module map) and
  `README.md` (setup, scripts, troubleshooting). (`M0-10`)

### Changed

- **Dashboard rendering is always virtualised.** The grid previously switched to
  a plain CSS grid below 120 cards; crossing that boundary cost **272 ms** in the
  profiler because the whole list is torn down and rebuilt in a different mode.
  One rendering path removes the cliff. (`M6-02`)
- **`RepoCard` carries its recent runs** rather than only the latest, so the
  sparkline and (in `0.2.0`) the run list need no second fetch. (`M6-03`)
- **Rate limiting** is a hand-rolled token bucket rather than `governor`, which
  models a rate but not the "GitHub said stop for 120 seconds" pause that this
  client needs. (`M4-03`)
- **Persistence architecture:** the planned 4-reader/1-writer `deadpool-sqlite`
  pool was replaced by a single `Mutex<Connection>`. `deadpool-sqlite 0.13` has
  no connection-init hook, which makes the per-connection `PRAGMA
foreign_keys = ON` impossible to set reliably — and without it every
  `ON DELETE CASCADE` in the schema is silently inert. Reasoning in
  `ARCHITECTURE.md` ADR-002. (`M3-03`)
- **Migration tooling:** `rusqlite_migration` was dropped in favour of ~40 lines
  of hand-rolled, append-only migrations, because it cannot coexist with another
  `rusqlite` version in the same dependency graph (`libsqlite3-sys` may only be
  linked once). (`M3-03b`)
- **Rust MSRV** raised from 1.77 to 1.85. Cargo's MSRV-aware resolver had been
  silently selecting a five-year-old `deadpool-sqlite` to satisfy the lower
  bound.
- **Colour tokens** adjusted to meet WCAG AA as measured, rather than by eye:
  `--color-ink-muted` `#6B7480` → `#77808C` (4.12:1 → 4.89:1) and `--color-bad`
  `#EF4444` → `#F05252` (4.48:1 → 4.80:1 on its own badge tint). (`M1-08`)
- **Test coverage scope** narrowed to logic-bearing modules (`lib/`, `stores/`,
  `components/status`), matching the roadmap gate. Coverage is 100% lines and
  branches on that scope. (`M1-08`)

### Fixed

- **Deployment column mismatch:** the first schema declared the deployments
  column as `ref` while the DAO wrote `git_ref`, which failed at runtime rather
  than at compile time. Since nothing has shipped, migration `0001_init` was
  corrected in place instead of adding a corrective migration. Caught by the
  idempotency test. (`M3-01`)

### Security

- CSP without `unsafe-eval`, no wildcard origins, `script-src 'self'`.
- Capability set limited to `core:default` and `opener:allow-open-url`; no
  `shell`, `fs` or `http` permissions granted.
- `#![forbid(unsafe_code)]` on both crate roots.
- Log directory is created with the platform default ACLs; no telemetry.
- `PRAGMA foreign_keys = ON` is applied to every connection, so cascade deletes
  cannot silently leave orphaned rows behind. (`M3-03`)
- GitHub tokens are held in the OS keychain as `SecretString` and never cross
  the IPC boundary; the webview receives only the login name and expiry.
  (`M2-05`)
- Redirects are not followed automatically, so an `Authorization` header can
  never be replayed to a third-party host such as the log CDN. (`M2-02`,
  `M4-07`)
- The repository picker re-fetches a repository from GitHub before storing it,
  so a name supplied by the webview cannot become a row that misrepresents what
  it is. (`M6-01`)
- Exactly one command returns a secret — `webhook_signing_secret` — because the
  user has to paste it into GitHub's form. No event carries one. (`M8-09`)
- Every webhook transport verifies the same HMAC over the raw body; a relay that
  does not forward headers produces frames Pulse refuses rather than trusts.
  (`M8-07`)
- `npm audit` reports zero vulnerabilities; `cargo audit` is wired into CI.
  (`M11-06`, `M11-07`)

### Known issues

- `npm ci` requires the `allowScripts` entry for `esbuild@0.28.2`; upgrading
  esbuild without bumping that pin breaks the Vite build.
- `rusqlite` is pinned to `0.38` to match the graph's single `libsqlite3-sys`
  link. Bumping it requires bumping every other crate that links SQLite.
- The macOS and Linux CI legs have not run yet.
- No automated keyboard-only sweep of the primitives yet (deferred to M11-04).
- **No real sign-in has been performed.** The device flow is covered end to end
  against a mock GitHub, but completing it needs a registered client id, so the
  final hop (GitHub's device page, a real token, a real keychain write) is
  untested.
- **No live poll has run against GitHub,** for the same reason. The 24-hour
  soak across 250 repositories has not been performed; only the underlying
  arithmetic is tested.
- **Read-only.** Re-running, cancelling and dispatching workflows, and reading
  logs, are not implemented yet. The card's actions are refresh, copy run URL,
  mute and open on GitHub.
- Not implemented from the dashboard plan: a right-click context menu, a
  per-card error state with retry, a density toggle and the `Mine` filter.

---

<!--
ENTRY TEMPLATE — copy this block into [Unreleased] and fill it in.
Delete any heading that has no entries. Keep entries user-facing.

### Added
- **Area:** what a user can now do. (`M4-03`)

### Changed
- **Area:** what behaves differently now.

### Fixed
- **Area:** the symptom that no longer happens.

### Removed
- **Area:** what is gone and what replaces it.

### Security
- **Area:** the exposure that is closed. (CVE-style wording where applicable.)
-->

---

## Release history

| Version | Date       | Theme               | Milestones |
| ------- | ---------- | ------------------- | ---------- |
| `0.1.0` | 2026-11-20 | Read-only dashboard | M0–M6      |
| `0.2.0` | 2026-11-20 | Actions & insight   | M7         |
| `0.3.0` | 2026-11-20 | Live & native       | M8–M9      |
| `0.4.0` | 2026-11-20 | Polish & hardening  | M10–M11    |
| `0.3.0` | planned    | Live & native       | M8–M9      |
| `0.4.0` | planned    | Polish & hardening  | M10–M11    |
| `1.0.0` | planned    | Release-ready       | M12        |

---

## How to maintain this file

1. **During work:** add entries to `[Unreleased]` in the same PR that changes behaviour.
2. **On release:** rename `[Unreleased]` to the version and date, then open a fresh empty
   `[Unreleased]` section above it.
3. **Tie it back:** reference the roadmap task ID in parentheses so the roadmap checkbox and
   the changelog entry can be reconciled in one pass.
4. **Never** log internal refactors with no user-visible effect under `Added`/`Changed`;
   those belong in the commit history.
5. **Never** let a released section change after publication. Corrections go in a new patch
   release.

### Semver policy for a desktop app

| Bump      | When                                                                                            |
| --------- | ----------------------------------------------------------------------------------------------- |
| **Major** | Local DB schema breaks without an automatic migration, or the update path requires user action. |
| **Minor** | New user-facing capability (new event types, new transports, new panels).                       |
| **Patch** | Fixes, perf work, dependency security bumps, copy changes.                                      |

---

[Unreleased]: https://github.com/OWNER/pulse/commits/main
