# Pulse — Roadmap

Milestone plan for the local-first GitHub workflow monitor (Tauri v2 + Rust + React 19).

**How this works**

- This file is the single source of truth for _planned_ work. Shipped work is recorded in
  [CHANGELOG.md](./CHANGELOG.md).
- Tasks carry stable IDs (`M4-03`). Reference them in commits, PR titles, and changelog
  entries so progress is traceable in both directions.
- A milestone is done only when its **Exit criteria** are met — not when its checkboxes are
  ticked.
- Mark a task `**BLOCKED**` inline with the reason rather than silently leaving it open.

**Legend:** `[ ]` todo · `[x]` done · `**BLOCKED**` waiting on a decision or dependency
**Effort:** S ≤ 1 day · M = 2–4 days · L = 1–2 weeks (solo dev, unfamiliar codebase)

---

## Release map

| Version | Theme               | Milestones | Definition of done                                 |
| ------- | ------------------- | ---------- | -------------------------------------------------- |
| `0.1.0` | Read-only dashboard | M0 → M6    | Sign in, see live status for N repos, nothing else |
| `0.2.0` | Actions & insight   | M7         | Inspect runs, read logs, re-run and dispatch       |
| `0.3.0` | Live & native       | M8 → M9    | Webhooks, tray, notifications, autostart           |
| `0.4.0` | Polish & hardening  | M10 → M11  | Perf budgets measured, a11y audited, e2e green     |
| `1.0.0` | Release-ready       | M12        | Signed installers on three OSes, auto-update       |

**Critical path:** M0 → M1 → M2 → M3 → M4 → M5 → M6. Everything after M6 is parallelisable
except M8, which depends on M4 (client) and M3 (persistence).

---

## M0 — Foundation · S — **tasks complete, CI pending first push**

**Goal:** a Tauri window opens on Windows, macOS, and Linux, and CI proves it.

- [x] `M0-01` `git init`, `.gitignore` (Rust + Node + Tauri), `.editorconfig`, `.gitattributes`
- [ ] `M0-01b` `LICENSE` — **BLOCKED** on choosing a licence (see D5)
- [x] `M0-02` `src-tauri` crate, `#![forbid(unsafe_code)]`, release profile (`lto`, `codegen-units=1`, `strip`)
- [x] `M0-03` `package.json`, Vite 8, React 19, TypeScript 6 strict (`noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`)
- [x] `M0-04` Tailwind CSS 4 wired to `src/styles/tokens.css`
- [x] `M0-05` `tauri.conf.json`: identifier `dev.pulse.workflowmonitor`, CSP without `unsafe-eval`, bundle targets for all three OSes
- [x] `M0-06` `capabilities/default.json` — minimal permission set, no `shell`/`fs`/`http`
- [x] `M0-07` `error.rs`: `AppError` + `{ code, message, retryable }` serialization + redaction on serialize
- [x] `M0-08` `logging.rs`: `tracing` + daily rolling file + secret redaction layer
- [x] `M0-09` `ci.yml`: fmt, clippy, `tsc`, eslint, vitest, bundle budget, three-OS backend matrix
- [x] `M0-10` `ARCHITECTURE.md` with the decision log started

**Verified locally:** `cargo test` (7 passing), `clippy -D warnings`, `fmt --check`,
`tsc`, `eslint --max-warnings 0`, `vitest`, `vite build` (69.7 kB gzipped),
`tauri build --debug` produced `pulse.exe`, and the binary starts and writes a
redacted log to `%LOCALAPPDATA%\dev.pulse.workflowmonitor\logs`.

**Still open:** the macOS/Linux legs have never run — that needs the first CI
push. A human should confirm the window renders with `npm run tauri dev`.

---

## M1 — Design system & shell · M — **complete**

**Goal:** every visual primitive exists, is accessible, and the status vocabulary is frozen.

- [x] `M1-01` Token palette (`--bg-base` … `--st-neutral`) mapped into Tailwind theme
- [x] `M1-02` Primitives: `Button`, `Badge`, `Card`, `Spinner`, `Tooltip`, `Toast`, `Skeleton`
- [x] `M1-03` App shell: `TitleBar`, content region, `EmptyState`, `ErrorBoundary`
- [x] `M1-04` `useMotionAllowed()` hook — gates animation on `prefers-reduced-motion` **and** `visibilityState`
- [x] `M1-05` `mapRunStatus()` pure function + unit test covering every row of the status table
- [x] `M1-06` `StatusBadge` / `StatusPulse` (colour is never the only signal — icon or label always present)
- [x] `M1-07` Dev-only `/kitchen-sink` route rendering every primitive in every state
- [x] `M1-08` Focus-visible rings, 4.5:1 text contrast check, keyboard pass on primitives

**Verified:** Lighthouse accessibility **1.00** and best-practices **1.00** on the
gallery (the only remaining findings are `meta-description` / `robots.txt`, which
are meaningless in a desktop WebView). Status mapping has 100% branch coverage
across 14 table rows plus precedence and fallback cases. The gallery is
tree-shaken out of production builds — the `Component gallery` heading is absent
from `dist`.

Two palette tokens were adjusted to reach WCAG AA rather than relaxing the rule:
`--color-ink-muted` `#6B7480` → `#77808C` (was 4.12:1) and `--color-bad`
`#EF4444` → `#F05252` (was 4.48:1 on its own badge tint).

**Partial:** keyboard reachability is asserted by test for `Button` and provided
by `:focus-visible` / `group-focus-within` elsewhere, but there is no automated
keyboard-only sweep of every primitive yet. That belongs with the e2e work in
M11-04.

---

## M2 — Authentication · L — **complete**

**Goal:** real "Sign in with GitHub", tokens only in the OS keychain, refresh rotation that survives crashes.

Depends on M0, M1.

- [x] `M2-01` README section: registering a GitHub App or OAuth App with device flow enabled
- [x] `M2-02` `device_flow.rs` state machine `SignedOut → RequestingCode → AwaitingUser → Polling → Authorized | Denied | Expired | Failed`, cancellable
- [x] `M2-03` `slow_down` handling: +5s cumulative interval bump; `authorization_pending` continues; every terminal code maps to a distinct `AppError`
- [x] `M2-04` `opener` launches the verification URI; UI shows a letter-spaced code, copy button and live countdown
- [x] `M2-05` `token_store.rs`: `keyring` v3 behind a `TokenStore` trait, refresh token written **before** access token, `SecretString` throughout
- [x] `M2-06` `tokens.rs`: single-flight refresh behind an async gate with a re-check after acquiring it
- [x] `M2-07` Startup validation via `GET /user`; `401` clears the keychain and emits `AuthRequired`
- [x] `M2-08` `KeyringUnavailable` path: falls back to in-memory session-only storage, never plaintext on disk
- [x] `M2-09` Commands `auth_get_state` / `auth_start_device_flow` / `auth_cancel_device_flow` / `auth_sign_out` / `auth_refresh_session` + `auth://state` events
- [x] `M2-10` `AuthPanel` + `DeviceCodeCard` covering every phase, with retry and a config-specific hint
- [x] `M2-11` `wiremock` tests: every documented error code, `slow_down` growth, cancellation mid-poll, refresh single-flight
- [x] `M2-12` **Resolved** — the auth code is app-type agnostic. It branches on what the token response _contains_, so GitHub App (refresh tokens) and OAuth App (non-expiring) both work with no code fork. See ADR-011.

**Verified:** 46 backend tests and 77 frontend tests pass. The refresh test
drives 20 concurrent callers through one wiremock server and asserts exactly
one refresh request reaches it, with the rotated token persisted. `ts-rs`
exports `AuthState`/`AuthSession` to `src/lib/bindings/` and the UI imports
them, so the union cannot drift from the Rust enum.

**Not verified, and it matters:** no real sign-in has been performed. That needs
a registered GitHub App client id, which cannot be created on the user's behalf.
The flow is covered against a mock server, but the final hop — GitHub's device
page, a real token, a real keychain write — is untested end to end.

---

## M3 — Persistence · M — **complete**

**Goal:** normalized local cache with migrations, diffs, and retention.

Depends on M0.

- [x] `M3-01` `0001_init.sql` (repos, workflow_runs, jobs, deployments, pull_requests, meta) + indexes
- [x] `M3-02` `0002_webhook_deliveries.sql`
- [x] `M3-03` **Changed:** single `Mutex<Connection>` instead of a `deadpool-sqlite` pool, plus pragmas (`WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`). See ADR-002.
- [x] `M3-03b` Hand-rolled forward-only migrations with a `schema_migrations` ledger. See ADR-010.
- [x] `M3-04` DAOs per table with idempotent upserts (`ON CONFLICT DO UPDATE`)
- [x] `M3-05` Diff helper: given a fetched run set, return only inserted/changed rows
- [x] `M3-06` Retention job: runs > 90 days, deliveries > 30 days, monthly `VACUUM` off the hot path
- [x] `M3-07` Tests: migration up, round-trip, cascade deletes, 250-repo seed query timing

**Verified:** 24 backend tests pass, including idempotent migrations, a real
cascade delete across four child tables, diff behaviour for unchanged runs /
bare `updated_at` bumps / re-runs, delivery replay rejection, retention pruning
and `VACUUM` cadence. The binary was launched for real: it created
`%APPDATA%\dev.pulse.workflowmonitor\pulse.db`, applied both migrations, ran the
retention sweep and left `-wal` / `-shm` files behind, confirming WAL.

**Measured:** 250 repositories × 20 runs (5,000 rows) — "list all repos" plus
250 newest-run lookups completed in **27 ms**, i.e. ~108 µs per repository.
The roadmap's "under 5 ms" was an unmeasured guess; 27 ms for the full dashboard
refresh path is the honest number and leaves plenty of headroom.

**Deviation from the plan (M3-03):** the 4-reader/1-writer pool was dropped. A
pool exists to hide contention; at tens of statements per poll cycle there is
none, and `deadpool-sqlite 0.13` had dropped the `with_init` hook that is the
natural place for the per-connection `foreign_keys` pragma — without which every
`ON DELETE CASCADE` in the schema is silently inert. A single connection sets
pragmas once, at open. Revisit only with a measurement.

---

## M4 — GitHub client · L — **complete**

**Goal:** one client that never leaks a token, never exceeds budget, and handles 304s for free.

Depends on M2.

- [x] `M4-01` Shared `reqwest` client: rustls, gzip, `X-GitHub-Api-Version: 2022-11-28`, `redirect::Policy::none()`
- [x] `M4-02` `etag.rs`: LRU(512), `If-None-Match`, `304` serves the cached body and refunds the budget token
- [x] `M4-03` `budget.rs`: **hand-rolled** token bucket (3,500 req/hr, burst 30) instead of `governor`. See ADR-012.
- [x] `M4-04` Secondary-limit handling: `403` + `Retry-After` / `429` pauses the whole client and publishes `ratelimit://updated`
- [x] `M4-05` Retry policy: `5xx` and transport failures only, exponential backoff with full jitter, max 4 attempts
- [x] `M4-06` Endpoint functions: repos, repo, runs, run, jobs, workflows, dispatch, rerun-failed, rerun, cancel, deployments, deployment statuses, pulls, `/rate_limit`
- [x] `M4-07` Log download: read `Location`, re-issue **without** auth header, 5 MB streamed cap
- [x] `M4-08` Tests: 304 path, secondary-limit pause, 429, primary exhaustion, retry/no-retry, redirect token-stripping, truncation

**Verified:** 85 backend tests pass. The redirect test asserts the second
hop carries **no** `Authorization` header, and the budget test asserts a
burst is paced. `RateLimitSnapshot` is exported to
`src/lib/bindings/RateLimitSnapshot.ts` for the UI meter in M6.

**Measured cost:** the local bucket is the only thing standing between 250
repositories and the hourly ceiling, so it is deliberately naive: a `Mutex`, an
`Instant`, and a float. There is no background refill task and no timer.

**Deviation (M4-03):** `governor` was not used. It models a rate but not the
second constraint this client actually has — "GitHub told us to stop for 120
seconds" — so the pause would sit awkwardly alongside it. The hand-rolled
bucket is ~150 lines with its own tests, versus a dependency plus a wrapper.
The pure refill maths is unit-tested against a fixed clock.

---

## M5 — Poller · M — **complete**

**Goal:** keep 250 repos fresh without ever blowing the budget.

Depends on M3, M4.

- [x] `M5-01` Scheduler loop with a 30 s tick; the tick only wakes to scan, it does not poll on a fixed schedule
- [x] `M5-02` Budget-aware interval: `clamp(ceil(repos × 3600 / ceiling), 60s, 15min)`
- [x] `M5-03` Manual refresh supersedes the schedule, deduplicated so repeated clicks queue one poll
- [x] `M5-04` Active runs poll at the floor interval; idle repositories drift to the budget-derived one
- [x] `M5-05` Diff-only emission — `run://updated` carries only runs whose state actually changed
- [x] `M5-06` Sleep/resume needs no special handling: due times are wall-clock, so everything is overdue on wake
- [x] `M5-07` `poller://status` event (monitored count, target interval, budget available, last error)
- [x] `M5-08` Tests: interval math at 10 / 100 / 250 / 500 repos, budget deferral, rate-limit stop, quiet ticks

**Design change worth knowing:** the poller is **sequential** — one request at
a time — not a pool. The sweep of 250 repositories at a 257 s interval needs
about one request per second, and each request takes ~200 ms, so concurrency
would buy latency we do not need in exchange for a scheduler, a semaphore and
in-flight bookkeeping. Being sequential also makes "never fetch the same
repository twice" structurally true rather than something a test has to prove.

**Two mechanisms, two jobs:** [`schedule`] decides how often each repository
_should_ be revisited (the plan); the token bucket in `github::budget` is what
actually stops us (the enforcement). When many repositories have runs in
flight, the plan alone could overshoot, and the bucket caps it.

**Verified:** 108 backend tests pass, including a test that a rate limit stops
the sweep instead of retrying against every remaining repository, and one that
an identical response publishes nothing.

**Not verified:** no live poll has run against GitHub — that needs a real token.
The 24-hour/250-repository soak in the exit criterion remains untested; the
arithmetic that underpins it is unit-tested, but arithmetic is not a soak.

---

## M6 — Dashboard · L → **v0.1.0**

**Goal:** the read-only product. Ship it.

Depends on M1, M5.

- [x] `M6-01` Commands: `repos_list`, `repos_available`, `repos_track`, `repos_set_monitored`, `repos_refresh`. **Deviation:** no `repos_search` — filtering 250 cards in the webview is instant, and a round trip per keystroke would make search feel worse.
- [x] `M6-02` `DashboardGrid`: `repeat(auto-fill, minmax(320px, 1fr))`, row-virtualized
- [x] `M6-03` `RepoCard`: worst-status badge, latest run, sparkline of the last 10 runs, hover actions (refresh, copy run URL, mute, open on GitHub)
- [ ] `M6-03b` Right-click context menu — **not done**; the hover actions cover the same operations
- [x] `M6-04` `RepoCardSkeleton`, empty state, dashboard-level error state with retry
- [ ] `M6-04b` Per-card error state with retry — **not done**; a failed poll is reported on the card's status, and the dashboard-level retry covers a failed load
- [x] `M6-05` `FilterBar`: search, `All / Failing / Running`, Refresh all
- [ ] `M6-05b` Density toggle and the `Mine` filter — **not done**
- [x] `M6-06` `RateLimitMeter` + poller status, from GitHub's own headers
- [x] `M6-07` Surgical `setQueryData` per card; `invalidateQueries` only for structural change (track, mute)
- [x] `M6-08` React Profiler bench at `#/bench`, 250 repositories
- [x] `M6-09` Bundle budget check in CI (< 300 KB gzipped initial)
- [x] `M6-10` Tag `v0.1.0`, move `[Unreleased]` in CHANGELOG

**Measured, in a real browser (Vite dev build, React StrictMode on, which
double-renders):**

| Interaction                         | p95 commit    |
| ----------------------------------- | ------------- |
| One card updated (`run://updated`)  | **4.3 ms**    |
| Fifty cards updated (webhook burst) | **7.4 ms**    |
| Filter transition (250 → 111 cards) | **~19–23 ms** |
| Replace all 250 cards               | **~19–30 ms** |

So the budget holds for the steady state — which is what a dashboard does all
day — and is exceeded by a full-list replacement, which only happens on an
explicit refetch. Dev-build numbers are pessimistic; production has no
double-render.

**The measurement changed the design.** The grid originally switched to a plain
CSS grid below 120 cards. Crossing that boundary measured **272 ms**: a filter
dropping from 250 to 111 cards tore down every row and rebuilt them in a
different mode. Removing the threshold and virtualising always cut it to ~19 ms.
Without the profiler this would have shipped as "sometimes the search box
stutters".

**Not verified:** the exit criterion — sign in, watch live for an hour, no
memory growth — needs a real token, so it has not been run.

---

## M7 — Runs, jobs, logs, actions · L → **v0.2.0** — **complete**

- [x] `M7-01` `runs_list` (offset paging), `runs_jobs` (fetched on demand), `job_log`. **Deviation:** no `runs_get` — the run inspector works from the cached card, so a single-run fetch would be a round trip for data already on screen.
- [x] `M7-02` Run list in the inspector, newest first, with status per row
- [x] `M7-03` `LogViewer`: on-demand fetch, 20k line render cap, search with match count, jump-to-failure, truncation banner, virtualised rows
- [x] `M7-04` Re-run failed / re-run all / cancel, optimistic status with rollback on error
- [x] `M7-05` `workflows_list` + `DispatchDialog`. **Deviation:** inputs are a validated JSON object, not a generated form — reading each workflow's `workflow_dispatch` schema means fetching and parsing its YAML file, which this dialog does not do.
- [x] `M7-06` Copy the whole log, and copy a run URL
- [x] `M7-07` Deployment and pull-request panels behind tabs in the inspector
- [x] `M7-08` Tests for the optimistic helper, log parsing, the viewer and its states

**Two corrections to the plan, both from reading GitHub's API:**

- The run-level log endpoint returns a **ZIP archive**, not text. Reading a log
  has to go through the _job_ endpoint, so `job_log` replaces the planned
  `runs_fetch_log`.
- GitHub issues **new job ids on every re-run**, so upserting jobs would leave
  the previous attempt's jobs beside the new ones. `jobs::replace` deletes the
  run's jobs and writes the fresh set in one transaction.

**Verified:** 115 backend and 140 frontend tests pass; the app builds and starts.
The log viewer is exercised with a mocked virtualizer, since jsdom has no layout.

**Not verified:** none of these actions has been run against GitHub — that still
needs a real token. The exit criterion ("never silently fails, surfaces the
GitHub error verbatim") is implemented as a toast carrying the backend message,
but has only been exercised against mocks.

---

## M8 — Webhook ingestion · L → **v0.3.0**

**Goal:** sub-second updates without burning API budget.

Depends on M3, M4.

- [x] `M8-01` `axum` listener on `127.0.0.1` with `/{path_secret}/webhook` and `/{path_secret}/health`
- [x] `M8-02` `HMAC-SHA256` over **raw bytes**, compared with `subtle::ConstantTimeEq`, before anything is parsed
- [x] `M8-03` 25 MB body cap rejected by the extractor before buffering; `Content-Type` must be `application/json`
- [x] `M8-04` Delivery dedupe on `X-GitHub-Delivery`, persisted in `webhook_deliveries`
- [x] `M8-05` Immediate `202`, processing on a spawned task — GitHub retries anything slower than ten seconds
- [x] `M8-06` Event filter for the seven handled types; anything else is recorded and ignored rather than 500'd
- [x] `M8-07` Relay transport (SSE) with exponential reconnect and the same verifier
- [x] `M8-08` Local mode plus a reachability check for a public tunnel URL
- [x] `M8-09` Webhook panel: transport select, copyable endpoint, secret reveal/rotate, delivery count, tunnel test
- [x] `M8-10` Integration tests against a real socket: signed → 202, tampered → 401, wrong path → 404, replay deduped, non-JSON → 415, ping does nothing
- [x] `M8-11` README: why GitHub cannot reach loopback, and the three options

**Design decision (M8-06):** the router does **not** apply payloads to the
database. It records the delivery and asks the poller to refresh that one
repository. That costs one API request per event instead of zero, and buys
three things: the event-to-state mapping stays in the one place that is already
tested, there is no second writer racing the poller's transaction, and a
payload Pulse misreads cannot leave the cache wrong — the refresh fetches the
truth. See ADR-016.

**Verified:** 149 backend and 140 frontend tests pass. The webhook tests bind a
real loopback socket and drive it with real HTTP, so routing, headers, status
codes and dedupe are exercised end to end rather than mocked.

**Not verified:** no delivery from GitHub has ever arrived. The signature
algorithm is tested against known-good HMACs, but GitHub's actual delivery —
including whether a relay forwards the headers Pulse requires — is untested.

**Release note:** `v0.3.0` is _not_ cut here. The release map defines it as
M8–M9, and M9 (tray, notifications, autostart, single instance) is not done.

---

## M9 — Native integration · M → **v0.3.0** — **complete**

- [x] `M9-01` Tray icon with four states (idle / running / failure / auth required), generated at runtime rather than shipped as assets
- [x] `M9-02` Tray menu: Open, Pause/Resume polling, Mute 1 h, Quit; left click toggles the window, menu on right click
- [x] `M9-03` Notifications on `failure` / `timed_out` / `startup_failure`, deduped per `(run id, attempt)`, throttled to one per 10 s, with quiet hours
- [ ] `M9-04` Notification click deep-links to the run — **not done.** `tauri-plugin-notification` exposes no click callback, and a per-platform implementation is out of scope here.
- [x] `M9-05` `single-instance` focuses the existing window; `window-state` restores geometry
- [x] `M9-06` Opt-in autostart, off by default — verified: the Run key is untouched until the user asks
- [x] `M9-07` macOS dock badge from the failure count; tray tooltip on every platform. **Windows taskbar overlay icon not done.**
- [x] `M9-08` Close-to-tray: closing hides instead of quitting, and it is a setting

**Verified:** 166 backend tests pass. The tray's state function, the icon
generation and the quiet-hours window are unit-tested. The app launches with a
tray icon on Windows and does not register itself for autostart.

**The tray icon is drawn at runtime.** Four 16-pixel PNGs would be four binary
assets to keep in step with the palette; a loop that measures distance from the
centre produces an antialiased dot from the same hex values the UI uses, so the
tray and the dashboard cannot disagree about what "red" means.

**Not verified:** nothing here has run on macOS or Linux. The tray, the dock
badge and the notification permission prompt are all platform-specific, and
Windows is the only platform available.

**Exit criteria, honestly:** one of the three is unverified. "Exactly one
notification per failed run" is unit-tested through the dedupe key; "tray
reflects a completed run" cannot be observed without a live poll; "a second
launch does not spawn a process" is configured but untested.

---

## M10 — Settings & polish · M → **v0.4.0**

- [ ] `M10-01` `settings_get` / `settings_set` with optimistic store and rollback
- [ ] `M10-02` `SettingsPanel`: polling, notifications, webhook, autostart, tray, retention, sign out
- [ ] `M10-03` `CommandPalette` (`Cmd/Ctrl+K`) fuzzy jump to repo / run / action
- [ ] `M10-04` Metrics panel (`rss_mb`, `db_size_mb`, `last_poll_ms`, `requests_last_hour`) + clear cache
- [ ] `M10-05` Keyboard map documented and implemented
- [ ] `M10-06` a11y audit: live regions, roles, contrast, reduced motion
- [ ] `M10-07` Zero-repo onboarding: browse and pick repositories
- [ ] `M10-08` Loading / empty / error state for every surface

---

## M11 — Hardening · M → **v0.4.0**

- [ ] `M11-01` 8-hour soak script: RSS growth < 2 MB/h at 250 repos
- [ ] `M11-02` Idle CPU measurement: < 1% over 10 min with window hidden
- [ ] `M11-03` Perf bench wired into CI as a non-blocking report first, blocking later
- [ ] `M11-04` `msw` handler set + Playwright e2e against the built binary with a mock GitHub
- [ ] `M11-05` Coverage ≥ 80% lines on `lib/`, `stores/`, `components/status`, `components/dashboard`
- [ ] `M11-06` Security pass: capability audit, CSP check, grep for `gh[pousr]_`, no token in logs
- [ ] `M11-07` Dependency audit (`cargo audit`, `npm audit`) in CI

---

## M12 — Release engineering · M → **v1.0.0**

- [ ] `M12-01` `release.yml` with `tauri-action`, three-OS matrix, draft release
- [ ] `M12-02` Updater signing keys + `latest.json` generation
- [ ] `M12-03` macOS notarization, Windows code signing (secrets documented, never committed)
- [ ] `M12-04` `ARCHITECTURE.md` complete: threat model, data flow, decision log
- [ ] `M12-05` README: setup, GitHub App registration, tunnel trade-offs, troubleshooting
- [ ] `M12-06` Walk the 10 acceptance criteria and record evidence
- [ ] `M12-07` Tag `v1.0.0`

---

## Post-1.0 backlog

Unscheduled. Promote into a milestone when it becomes a real user complaint.

- Light theme (tokens are structured for it)
- Multi-account / multiple GitHub identities
- Organization and team filters
- GraphQL batching to cut request count
- Saved filter presets and pinned ordering
- Webhook-only mode (disable polling entirely)
- Full-text search index over cached logs
- Windows toast action buttons ("Re-run", "Open")
- Linux AppIndicator fallback when no tray is available
- i18n scaffolding
- GHES (self-hosted GitHub) base-URL support
- CLI companion that shares the same SQLite cache

---

## Decisions

Resolved decisions are recorded in `ARCHITECTURE.md` under "Decision log".
Open ones are below.

| #      | Decision                  | Options                                                                                                          | Default if unresolved                                    | Affects |
| ------ | ------------------------- | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------- |
| ~~D1~~ | ~~Auth model~~            | **Resolved in M2:** the code supports both — it branches on whether the token response contains a refresh token. | Either; GitHub App for refresh, OAuth App for simplicity | M2      |
| D2     | Webhook transport         | **Relay only** (smee-style, zero config) vs relay + tunnel + local server                                        | Relay + local server, tunnel documented                  | M8      |
| D3     | Polling default           | 60 s min / 15 min max vs user-set fixed interval                                                                 | Budget-aware adaptive                                    | M5, M10 |
| D4     | Log retention             | Keep log text on disk vs fetch-on-demand only                                                                    | Fetch-on-demand, cache nothing                           | M7, M11 |
| D5     | Product name / identifier | `Pulse` / `dev.pulse.workflowmonitor`                                                                            | As written                                               | M0      |

Record the outcome of each in `ARCHITECTURE.md` under "Decisions" and tick the corresponding
`**BLOCKED**` task.

---

## Maintaining this file

1. Tick a task in the same PR that implements it, and add the matching `CHANGELOG.md` entry.
2. Do not renumber task IDs after they have been referenced — append `-b` instead.
3. If a milestone's exit criteria change, note it in the changelog under `Changed`.
4. Review this file at every release tag: re-estimate the next two milestones, prune the
   backlog, and close any open decision that has been implicitly made.
