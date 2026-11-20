# Pulse — Roadmap

Milestone plan for the local-first GitHub workflow monitor (Tauri v2 + Rust + React 19).

**How this works**

- This file is the single source of truth for *planned* work. Shipped work is recorded in
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

| Version | Theme | Milestones | Definition of done |
| ------- | ----- | ---------- | ------------------ |
| `0.1.0` | Read-only dashboard | M0 → M6 | Sign in, see live status for N repos, nothing else |
| `0.2.0` | Actions & insight | M7 | Inspect runs, read logs, re-run and dispatch |
| `0.3.0` | Live & native | M8 → M9 | Webhooks, tray, notifications, autostart |
| `0.4.0` | Polish & hardening | M10 → M11 | Perf budgets measured, a11y audited, e2e green |
| `1.0.0` | Release-ready | M12 | Signed installers on three OSes, auto-update |

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

## M1 — Design system & shell · M

**Goal:** every visual primitive exists, is accessible, and the status vocabulary is frozen.

- [ ] `M1-01` Token palette (`--bg-base` … `--st-neutral`) mapped into Tailwind theme
- [ ] `M1-02` Primitives: `Button`, `Badge`, `Card`, `Spinner`, `Tooltip`, `Toast`, `Skeleton`
- [ ] `M1-03` App shell: `TitleBar`, content region, `EmptyState`, `ErrorBoundary`
- [ ] `M1-04` `useMotionAllowed()` hook — gates animation on `prefers-reduced-motion` **and** `visibilityState`
- [ ] `M1-05` `mapRunStatus()` pure function + unit test covering every row of the status table
- [ ] `M1-06` `StatusBadge` / `StatusPulse` (colour is never the only signal — icon or label always present)
- [ ] `M1-07` Dev-only `/kitchen-sink` route rendering every primitive in every state
- [ ] `M1-08` Focus-visible rings, 4.5:1 text contrast check, keyboard pass on all primitives

**Exit:** kitchen-sink route renders all states; status mapping has 100% branch coverage; no primitive fails keyboard-only navigation.

---

## M2 — Authentication · L

**Goal:** real "Sign in with GitHub", tokens only in the OS keychain, refresh rotation that survives crashes.

Depends on M0, M1.

- [ ] `M2-01` README section: how to register the GitHub App (device flow enabled, token expiry enabled)
- [ ] `M2-02` `device_flow.rs` state machine `Idle → RequestingCode → AwaitingUser → Polling → Authorized | Denied | Expired | Error`, cancellable
- [ ] `M2-03` `slow_down` handling: +5s permanent interval bump; `authorization_pending` continues; terminal codes map to distinct `AppError`
- [ ] `M2-04` `opener` launches verification URI; UI shows letter-spaced code, copy button, 15-min countdown
- [ ] `M2-05` `token_store.rs`: `keyring` v3, refresh token written **before** access token, `SecretString` in memory
- [ ] `M2-06` `refresh.rs`: single-flight refresh under concurrent 401s, persist rotated refresh token before use
- [ ] `M2-07` Startup validation via `GET /user`; `bad_refresh_token` purges keychain and emits `AuthRequired`
- [ ] `M2-08` `KeyringUnavailable` path: fail closed, offer session-only in-memory opt-in, never plaintext on disk
- [ ] `M2-09` Commands `auth_start_device_flow` / `auth_cancel_device_flow` / `auth_get_session` / `auth_sign_out` + `auth://state` events
- [ ] `M2-10` `AuthPanel` + `DeviceCodeCard` with all error states and retry affordances
- [ ] `M2-11` `wiremock` tests: all five error codes, interval growth, cancellation mid-poll
- [ ] `M2-12` **BLOCKED** — confirm auth model: GitHub App (refresh tokens) vs OAuth App (non-expiring, simpler). See Open decisions.

**Exit:** sign-in against a real GitHub App works; grep of bundle, logs, and SQLite proves no token is present anywhere except the keychain; refresh rotation test passes under 20 concurrent callers.

---

## M3 — Persistence · M

**Goal:** normalized local cache with migrations, diffs, and retention.

Depends on M0.

- [ ] `M3-01` `0001_init.sql` (repos, workflow_runs, jobs, deployments, pull_requests, meta) + indexes
- [ ] `M3-02` `0002_webhook_deliveries.sql`
- [ ] `M3-03` `deadpool-sqlite` pool (4 readers / 1 writer) + pragmas (`WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`)
- [ ] `M3-04` DAOs per table with idempotent upserts (`ON CONFLICT DO UPDATE`)
- [ ] `M3-05` Diff helper: given a fetched run set, return only inserted/changed rows
- [ ] `M3-06` Retention job: runs > 90 days, deliveries > 30 days, monthly `VACUUM` off the hot path
- [ ] `M3-07` Tests: migration up, round-trip, cascade deletes, 250-repo seed query timing

**Exit:** 250-repo seed; every dashboard query under 5 ms; deleting a repo removes all dependent rows.

---

## M4 — GitHub client · L

**Goal:** one client that never leaks a token, never exceeds budget, and handles 304s for free.

Depends on M2.

- [ ] `M4-01` Shared `reqwest` client: rustls, gzip, `X-GitHub-Api-Version: 2022-11-28`, `redirect::Policy::none()`
- [ ] `M4-02` `etag_cache.rs`: LRU(512), `If-None-Match`, `304` serves cached body and costs zero budget
- [ ] `M4-03` `rate_limit.rs`: `governor` token bucket, 3,500 req/hr ceiling, burst 30
- [ ] `M4-04` Secondary-limit handling: `403` + `retry-after` / `429` pauses the whole client, emits `ratelimit://updated`
- [ ] `M4-05` Retry policy: `5xx` and timeouts only, exponential backoff with full jitter, max 4 attempts
- [ ] `M4-06` Endpoint functions: user, repos, runs, jobs, workflows, dispatches, reruns, cancel, deployments, pulls, `/rate_limit`
- [ ] `M4-07` Log download: read `Location`, re-issue **without** auth header, 5 MB streamed cap
- [ ] `M4-08` Tests: 304 path, secondary-limit pause, 429 recovery, redirect token-stripping

**Exit:** simulated 5,000-req/hr exhaustion pauses and recovers unattended; a redirect to `objects.githubusercontent.com` carries no `Authorization` header.

---

## M5 — Poller · M

**Goal:** keep 250 repos fresh without ever blowing the budget.

Depends on M3, M4.

- [ ] `M5-01` Scheduler task with a 30 s tick and a due-repo queue
- [ ] `M5-02` Budget-aware interval: `clamp(ceil(active_repos × 3600 / budget_remaining), 60s, 15min)`
- [ ] `M5-03` Per-repo single-flight so manual refresh supersedes the scheduled one
- [ ] `M5-04` Adaptive backoff: active runs at min interval, idle repos drift to max
- [ ] `M5-05` Diff-only emission — `run://updated` fires per changed run, never a full array
- [ ] `M5-06` Sleep/resume detection: mark all cache stale on wake
- [ ] `M5-07` `poller://status` event (state, next tick, budget remaining)
- [ ] `M5-08` Tests: interval math at 10 / 100 / 500 repos, no duplicate in-flight requests

**Exit:** 250 monitored repos stay under 3,500 req/hr for 24 h with zero duplicate fetches.

---

## M6 — Dashboard · L → **v0.1.0**

**Goal:** the read-only product. Ship it.

Depends on M1, M5.

- [ ] `M6-01` Commands: `repos_list`, `repos_search`, `repos_available`, `repos_set_monitored`, `repos_refresh`
- [ ] `M6-02` `DashboardGrid`: `repeat(auto-fill, minmax(320px, 1fr))`, virtualized above 120 cards
- [ ] `M6-03` `RepoCard`: worst-status pill, latest run, mini sparkline, hover actions, right-click menu
- [ ] `M6-04` `RepoCardSkeleton`, empty state, per-card error state with retry
- [ ] `M6-05` `FilterBar`: search, `All / Failing / Running / Mine`, density toggle
- [ ] `M6-06` `RateLimitMeter` + poller status indicator
- [ ] `M6-07` Event wiring: surgical `setQueryData` per card, `invalidateQueries` only for structural change
- [ ] `M6-08` React Profiler bench proving p95 commit < 16 ms at 250 repos
- [ ] `M6-09` Bundle budget check in CI (< 300 KB gzipped initial)
- [ ] `M6-10` Tag `v0.1.0`, move `[Unreleased]` in CHANGELOG

**Exit:** a user signs in, selects repos, and watches status update live for an hour with no manual refresh and no memory growth.

---

## M7 — Runs, jobs, logs, actions · L → **v0.2.0**

- [ ] `M7-01` `runs_list` with cursor pagination; `runs_get`, `runs_jobs`
- [ ] `M7-02` `RunList` / `RunRow` virtualized
- [ ] `M7-03` `LogViewer`: on-demand fetch, 20k line cap, search, jump-to-failure, truncation banner
- [ ] `M7-04` Re-run failed / re-run all / cancel with optimistic UI and rollback on error
- [ ] `M7-05` `workflows_list` + `DispatchDialog` with dynamic inputs form and JSON fallback
- [ ] `M7-06` Copy logs / copy run URL via clipboard plugin
- [ ] `M7-07` Pull request and deployment panels per repo
- [ ] `M7-08` Toast + rollback tests for every action

**Exit:** every action is undoable-by-retry, never silently fails, and surfaces the GitHub error verbatim.

---

## M8 — Webhook ingestion · L → **v0.3.0**

**Goal:** sub-second updates without burning API budget.

Depends on M3, M4.

- [ ] `M8-01` `axum` server on `127.0.0.1` with `/{path_secret}/webhook` + `/health`
- [ ] `M8-02` `HMAC-SHA256` over **raw bytes** with `subtle::ConstantTimeEq`, before deserialization
- [ ] `M8-03` 25 MB body cap rejected before buffering; `Content-Type` check
- [ ] `M8-04` Delivery dedupe on `X-GitHub-Delivery` (persisted, last 5,000)
- [ ] `M8-05` Fast `202` ack, processing on a spawned task
- [ ] `M8-06` Event router: `ping`, `workflow_run`, `workflow_job`, `deployment`, `deployment_status`, `pull_request`, `check_run`
- [ ] `M8-07` Relay transport (smee-style SSE) with reconnect + backoff, same verifier
- [ ] `M8-08` Tunnel mode with a signed reachability ping
- [ ] `M8-09` `WebhookConfig` UI: transport select, copyable URL, secret rotation, live status
- [ ] `M8-10` Integration tests: signed fixtures, tampered payload → `401`, replay → deduped
- [ ] `M8-11` README: why GitHub cannot reach `localhost`, and the three options

**Exit:** a real push-triggered run appears in the UI within 2 s; tampered payloads are rejected and logged without the secret appearing anywhere.

---

## M9 — Native integration · M → **v0.3.0**

- [ ] `M9-01` Tray icon states (`all_green` / `running` / `failure` / `auth_required`) + macOS template variant
- [ ] `M9-02` Tray menu: Open, Pause/Resume, Mute 1 h, Settings, Quit; left-click toggles window
- [ ] `M9-03` Notifications on `failure` / `timed_out` / `cancelled`, deduped per `(run_id, attempt)`, 1 per 10 s, quiet hours
- [ ] `M9-04` Notification click deep-links to the run
- [ ] `M9-05` `single-instance` focuses the existing window; `window-state` restores geometry
- [ ] `M9-06` Opt-in autostart (off by default)
- [ ] `M9-07` macOS badge count / Windows overlay icon / Linux tray tooltip
- [ ] `M9-08` Close-to-tray behaviour setting

**Exit:** tray state reflects a completed run within 2 s; exactly one notification per failed run; second launch does not spawn a second process.

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

## Open decisions

These block or reshape the milestones marked `**BLOCKED**`. Resolve before M2 ships.

| # | Decision | Options | Default if unresolved | Affects |
| - | -------- | ------- | --------------------- | ------- |
| D1 | Auth model | **GitHub App** (refresh tokens, installation step, fine-grained perms) vs **OAuth App** (non-expiring token, no refresh, `repo workflow read:user`, simpler) | GitHub App — required for the refresh-token requirement | M2, README |
| D2 | Webhook transport | **Relay only** (smee-style, zero config) vs relay + tunnel + local server | Relay + local server, tunnel documented | M8 |
| D3 | Polling default | 60 s min / 15 min max vs user-set fixed interval | Budget-aware adaptive | M5, M10 |
| D4 | Log retention | Keep log text on disk vs fetch-on-demand only | Fetch-on-demand, cache nothing | M7, M11 |
| D5 | Product name / identifier | `Pulse` / `dev.pulse.workflowmonitor` | As written | M0 |

Record the outcome of each in `ARCHITECTURE.md` under "Decisions" and tick the corresponding
`**BLOCKED**` task.

---

## Maintaining this file

1. Tick a task in the same PR that implements it, and add the matching `CHANGELOG.md` entry.
2. Do not renumber task IDs after they have been referenced — append `-b` instead.
3. If a milestone's exit criteria change, note it in the changelog under `Changed`.
4. Review this file at every release tag: re-estimate the next two milestones, prune the
   backlog, and close any open decision that has been implicitly made.
