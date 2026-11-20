# Architecture

Pulse is a local-first desktop client for GitHub Actions. There is no Pulse
server: the app talks to the GitHub API and (optionally) receives webhooks
through a relay, and everything it learns is cached in a local SQLite file.

This document records **why** the system is shaped the way it is. Task-level
planning lives in [ROADMAP.md](./ROADMAP.md); user-visible changes live in
[CHANGELOG.md](./CHANGELOG.md).

---

## 1. Process model

```
┌──────────────────────────── Tauri process ─────────────────────────────┐
│                                                                        │
│  ┌──────────────── WebView (React 19) ────────────────┐                │
│  │  TanStack Query  ← server-derived state            │                │
│  │  Zustand         ← UI-only state                   │                │
│  │  lib/ipc.ts      ← the only way out                │                │
│  └────────────────────────┬───────────────────────────┘                │
│                           │ invoke() / emit()                          │
│  ┌────────────────────────┴───────────────────────────┐                │
│  │  Rust core                                         │                │
│  │   commands/  ── IPC surface, validates input       │                │
│  │   auth/      ── device flow, keyring, refresh      │                │
│  │   github/    ── HTTP client, ETag, rate budget     │                │
│  │   poller/    ── scheduler, adaptive intervals      │                │
│  │   webhook/   ── local server, relay, HMAC verify   │                │
│  │   db/        ── SQLite, migrations, diffs          │                │
│  │   tray/ notify/ ── native surface                  │                │
│  └────────────────────────────────────────────────────┘                │
└────────────────────────────────────────────────────────────────────────┘
```

**Rule:** the WebView never talks to the network. All HTTP happens in Rust so
tokens never enter JavaScript, CSP can stay closed, and rate limiting has a
single owner.

## 2. Module map

| Path                            | Responsibility                                            | Milestone |
| ------------------------------- | --------------------------------------------------------- | --------- |
| `src-tauri/src/lib.rs`          | Builder, plugin registration, window lifecycle            | M0        |
| `src-tauri/src/error.rs`        | `AppError` wire contract                                  | M0        |
| `src-tauri/src/redact.rs`       | Secret scrubbing for logs and errors                      | M0        |
| `src-tauri/src/logging.rs`      | Rolling file log + redacting writer                       | M0        |
| `src-tauri/src/state.rs`        | Shared process state                                      | M0        |
| `src-tauri/src/commands/`       | IPC surface, input validation                             | M0+       |
| `src-tauri/src/auth/`           | Device flow, token custody, single-flight refresh         | M2        |
| `src-tauri/src/db/`             | Schema, migrations, DAOs, diffing, retention              | M3        |
| `src-tauri/migrations/`         | Append-only `.sql` schema versions                        | M3        |
| `src-tauri/src/github/`         | HTTP client and endpoints (ETag + budget from M4)         | M2+       |
| `src-tauri/src/poller/`         | Scheduler, budget math, backoff                           | M5        |
| `src-tauri/src/webhook/`        | Local server, relay, HMAC verification                    | M8        |
| `src/lib/ipc.ts`                | Typed `invoke` wrapper, error normalisation               | M0        |
| `src/lib/repo-selectors.ts`     | Filtering, sorting, surgical card updates, formatting     | M6        |
| `src/components/runs/`          | Run inspector, job list, log viewer, dispatch dialog      | M7        |
| `src/lib/run-selectors.ts`      | Optimistic status, log line parsing and search            | M7        |
| `src/hooks/useLiveUpdates.ts`   | Bridges push events into the query cache and live store   | M6        |
| `src/components/dashboard/`     | Grid, card, sparkline, filter bar, repository picker      | M6        |
| `src/pages/Bench.tsx`           | Dev-only render profiler (compiled out of production)     | M6        |
| `src/lib/status.ts`             | Canonical status vocabulary (mapping table, worst-of)     | M1        |
| `src/components/status/`        | `StatusBadge` / `StatusPulse` — the only status renderers | M1        |
| `src/components/primitives/`    | Button, Badge, Card, Spinner, Skeleton, Tooltip, Toast    | M1        |
| `src/hooks/useMotionAllowed.ts` | Single gate for every looping animation                   | M1        |
| `src/stores/`                   | Zustand stores (UI-only state)                            | M1+       |
| `src/lib/bindings/`             | `ts-rs`-generated types (checked in)                      | M1+       |

## 3. Decision log

### ADR-001 — Tauri v2 over Electron

Required by the brief, but the reasoning holds: a ~10 MB binary and a shared
OS WebView instead of a bundled Chromium. The cost is per-OS WebView
divergence, which is why `vite.config.ts` targets the actual engine
(`chrome105` on Windows, `safari15` on macOS) rather than a lowest-common
denominator.

### ADR-002 — rusqlite, a single connection, no pool (revised in M3)

`sqlx`'s compile-time macros need a live `DATABASE_URL` or a checked-in
`.sqlx` offline cache, which makes a fresh clone fail to build. `rusqlite` with
the `bundled` feature embeds SQLite and has no build-time coupling.

The original plan added `deadpool-sqlite` with a 4-reader/1-writer pool. That
was dropped while building it, for two reasons:

1. **It had no pragma hook.** `deadpool-sqlite 0.13` removed `with_init`, so
   there is no place to run `PRAGMA foreign_keys = ON` when a connection is
   created. That pragma is per-connection and defaults to OFF, which means
   every `ON DELETE CASCADE` in the schema is silently inert — deleting a
   repository would orphan its runs, jobs and pull requests. Silent data
   corruption is not an acceptable price for concurrency we do not need.
2. **There is nothing to hide.** A pool exists to smooth contention. A poll
   cycle issues tens of statements over a few thousand indexed rows; measured
   at 250 repositories, the dashboard's whole read path takes 27 ms. A pool
   would add a checkout, a semaphore and a background thread pool to save
   microseconds.

The design does need one property a pool would have provided — that no two
writers race for the file lock. A single `Mutex<Connection>` gives it for free,
and WAL means readers never block on a writer anyway. Blocking work runs on
`spawn_blocking` so a slow query cannot stall the async runtime.

If profiling ever shows real contention, add readers _then_, with the
measurement in hand.

### ADR-010 — Migrations are hand-rolled and append-only

`rusqlite_migration` was tried first and rejected: it pins a different
`rusqlite` than `deadpool-sqlite` did, and Rust permits only one crate in a
dependency graph to declare `links = "sqlite3"`, so the two cannot coexist.

What replaced it is ~40 lines: each migration is a `.sql` file embedded with
`include_str!`, run inside `unchecked_transaction`, and recorded in a
`schema_migrations` ledger. Two properties matter more than the library would
have given us:

- **Idempotent.** `Db::open` runs migrations on every launch; already-applied
  versions are skipped.
- **Append-only.** A shipped migration has already executed on someone's
  machine. Editing one would leave those databases permanently inconsistent
  with a fresh install. Corrections go in a new file.

`VACUUM` is deliberately excluded from migrations: SQLite refuses to run it
inside a transaction, so it lives in the retention sweep on its own cadence.

### ADR-013 — A sequential poller, and a plan separated from its enforcement

**Why one request at a time.** The schedule needs roughly one request per second
for 250 repositories; each request takes about 200 ms. A pool would therefore
buy latency the product does not feel, in exchange for a semaphore, a task set
and a "don't poll the same repository twice" invariant that has to be proven.
Being sequential makes that invariant structural: there is one loop, so two
fetches for one repository cannot be in flight.

The cost is honest and bounded: a full sweep of 250 repositories takes about a
minute, which is well inside the 257-second interval it is meant to fit.

**Why the interval and the bucket are separate.** They answer different
questions. `target_interval` answers _"how often should each repository be
revisited"_ — it spreads the sweep evenly and is what the status event reports.
The token bucket answers _"may I send this request right now"_ — it is the hard
limit. When many repositories have runs in flight, the plan alone can overshoot;
the bucket caps it. Conflating the two would mean either a plan that cannot
express priority or a limit that depends on the plan being correct.

**Why the scheduler checks `available()` instead of taking a token.** Taking a
token to decide whether to take a token would double-charge, and a 304 refunds
its token anyway. The check is advisory — two polls may slip through on a
one-token bucket — but the client's own `acquire()` is the real gate, so the
overshoot costs a brief internal wait, not an extra request.

**Why sleep/resume needs no code.** Due times are compared against wall-clock
`last_polled_at` values persisted in SQLite, not against monotonic instants held
in memory. After the machine sleeps, every repository is simply overdue, which
is exactly the desired behaviour. The one case that needed handling was the
opposite: a clock that moves _backwards_ (NTP correction, timezone change) must
not park a repository forever, so a future timestamp is treated as overdue.

### ADR-014 — The dashboard renders through one path, and the profiler decided that

The grid was built with two rendering modes: a plain CSS grid for short lists,
and row-virtualised rendering above 120 cards. It is an obvious optimisation —
virtualisation has overhead, so why pay it for six cards?

The profiler said otherwise. Filtering 250 repositories down to 111 **measured
272 ms** for a single commit, against a 16 ms budget. The cause was not the
filter: it was the _boundary_. Crossing from virtualised to plain means React
unmounts every virtual row and mounts a differently-shaped tree, and it does
that on a keystroke, because the number of matches decides the mode.

Removing the second mode cut it to about 19 ms. Virtualising a three-card list
costs nothing measurable; the discontinuity cost a quarter of a second.

The lesson generalises: a performance optimisation that changes _representation_
introduces a cliff at its own threshold, and the cliff is usually worse than the
cost it was avoiding. Two rendering paths also means two behaviours to keep in
step, which is why the simpler version is the one that survived.

**Where the numbers come from.** `#/bench` mounts 250 synthetic repositories
wrapped in a React `Profiler` and reports p50/p95/max of the commit durations it
observes. It records only commits caused by an interaction: an earlier version
recorded every commit, including the one caused by _reporting the results_,
which produced a render loop that measured itself. Dev-build numbers are
pessimistic because React StrictMode double-renders.

**What the budget actually is.** Steady state is one card per `run://updated`
(4.3 ms) and the occasional burst (7.4 ms for fifty). A full 250-card
replacement is 19–30 ms, over budget, and only happens on an explicit refetch.
The honest statement is "the budget holds for what a dashboard does all day",
not "every frame is under 16 ms".

### ADR-015 — Read job logs, and replace a run's jobs wholesale

Two things about GitHub's Actions API only became clear while building the run
inspector, and both changed the design.

**The run-level log endpoint returns a ZIP.** `GET
/repos/{o}/{r}/actions/runs/{id}/logs` is an archive of every job's log. Only
`/actions/jobs/{id}/logs` returns text. A log _viewer_ therefore reads job logs,
which is why the command is `job_log` rather than the planned `runs_fetch_log`.
Unzipping in the backend would mean adding a zip dependency, a decompression
budget and an extraction path to defend, to produce something the API already
offers as plain text.

**Job ids change on every re-run.** They are not stable across attempts, so
upserting a run's jobs leaves the previous attempt's rows in the table next to
the new ones. `jobs::replace` deletes the run's jobs and writes the fresh set
inside one transaction. The alternative — keying on job _name_ — would be wrong
for a matrix build, where several jobs legitimately share a name.

Both are the same lesson: when an API's shape disagrees with the plan, change
the plan, and write down why, because the next person will otherwise "fix" it
back.

### ADR-016 — A webhook refreshes; it does not patch

The obvious design for webhook ingestion is to apply the payload: a
`workflow_run.completed` event says the run finished, so write that to the run's
row. It is fewer requests and it is what "real-time" usually means.

It was rejected, for three reasons that compound:

1. **The mapping would exist twice.** The poller already maps GitHub's run shape
   into cache rows, with tests for status drift, re-runs and partial payloads. A
   webhook path would need the same mapping again, for seven event types, and the
   two would drift.
2. **Two writers.** A webhook arriving mid-poll would race the poller's
   transaction. SQLite serialises them, but "which one wins" becomes a function
   of timing.
3. **A misread payload leaves the cache wrong.** Webhook bodies are partial and
   versioned; a field that moved would be written as `null` and stay wrong until
   the next poll silently repaired it. A refresh cannot do that: it replaces what
   is there with what GitHub says now.

So a delivery is recorded, deduplicated, and turned into
`poller.request_refresh(repo_id)`. The poller already wakes immediately when
asked, so the update arrives in about one request's latency.

**The cost is one API request per event**, which is real and worth stating.
With the local budget at 3,500/hour and a busy CI sending tens of events an
hour, this is nowhere near the ceiling — and every one of those requests is one
the poller would have made later anyway.

### ADR-017 — What a relay must forward

A relay is not a transparent pipe, and that is the whole difficulty. Pulse
verifies every payload against the signing secret, and the signature lives in a
header. A relay that forwards only the body produces frames that must be
rejected.

Rather than weaken verification for the convenience of one transport, the relay
path accepts an envelope carrying both:

```json
{ "headers": { "x-hub-signature-256": "sha256=…", … }, "body": "<raw json>" }
```

A frame without headers, without a signature, or with a signature that does not
match is logged and dropped. That means **a relay which strips headers simply
will not work with Pulse**, and the log says so explicitly rather than failing
somewhere downstream. The alternative — trusting anything that reaches the relay
URL — would make the URL itself the only secret, and relay URLs are pasted into
configuration files and chat windows.

### ADR-018 — Draw the tray icon, and earn each notification

**The tray icon is generated, not shipped.** Four states need four icons, and
four 16-pixel PNGs are four binary assets that can drift from the palette the
rest of the app uses. Instead the icon is a small RGBA buffer: distance from the
centre gives an antialiased dot, coloured from the same hex values as
`tokens.css`. The tray and the dashboard cannot disagree about what "red" means
because there is only one definition of red.

**Notifications have three rules, and the third is the important one.**

- Only failures. Success is expected; announcing it is noise.
- Once per `(run id, attempt)`. A re-run is a new attempt and deserves its own
  notification; the same attempt seen twice does not.
- At most one every ten seconds.

That last rule exists because of matrix builds. A workflow that fails across
twelve jobs emits twelve events in a few seconds, and without a throttle the
user gets a wall of identical notifications about one build. A monitor that
cries wolf gets muted, and a muted monitor is worse than no monitor.

**Quiet hours wrap midnight.** `22-07` is the common case and a naive
`start < hour < end` comparison returns "never quiet" for it, which fails
exactly when the feature is needed. The window is tested for both the wrapping
and non-wrapping cases, and the end hour is exclusive so `22-07` does not fire
at 07:00.

**Autostart is opt-in, and applied immediately.** An app that adds itself to
startup uninvited is behaving like malware regardless of intent. The setting
also takes effect when it is toggled, not at the next launch — a checkbox that
only works after a restart is a bug report waiting to happen.

### ADR-012 — One request pipeline, and a budget we own

Every GitHub call goes through `GithubClient::execute`, which applies the five
rules in a fixed order: local budget, token, conditional header, send,
classify. Putting them in one function is what makes the guarantees auditable —
there is no second path that could forget to attach the version header, or to
refund the token on a `304`.

**Why the budget is hand-rolled.** `governor` models a rate. This client needs a
rate _and_ a pause ("GitHub said stop for 120 seconds"), and two mechanisms for
one constraint is worse than one. A `Mutex`, an `Instant` and a float do the
job, the refill maths is unit-tested against a fixed clock, and there is no
background task.

**Why a `304` refunds.** The client must spend a token before it knows the
response will be a `304`. GitHub does not charge for conditional requests, so
the token is given back. Without this, the cheapest possible poll would cost the
same as the most expensive one, which would defeat the point of polling with
`ETag`s at all.

**Why long waits are errors, not sleeps.** A primary limit can last an hour.
Sleeping inside a request would be indistinguishable from a hang and would
ignore cancellation, so anything past 30 seconds surfaces as
`AppError::RateLimited { reset_at }` and the caller schedules the retry.

**Why the CDN request drops the token.** GitHub redirects log downloads to
`objects.githubusercontent.com` with a pre-signed URL. Replaying `Authorization`
to a different origin is the classic way desktop apps leak tokens into a CDN's
logs. The redirect is resolved by hand and the second request is anonymous; a
test asserts the header is absent.

**Why inputs are validated before the URL is built.** `full_name` and the git
ref are interpolated into a path and a body. A `..` that survived validation
would let a caller address an endpoint they were never granted, so both are
checked against a character class _and_ against the two traversal spellings
(`.`/`..`) that pass one.

### ADR-011 — Device flow, and one auth path for both app types

**Why the device flow.** A desktop binary cannot keep a client secret, so the
authorization-code flow would be a fiction. The device flow needs only a public
client id, has no redirect URI to register or hijack, and keeps the user's
password inside their browser rather than an embedded webview. The cost is a
code the user must copy — which is why the UI treats that code as the primary
element of the screen, not a detail.

**Why the code does not care which app type you register.** The plan framed
this as a fork: GitHub App for refresh tokens, OAuth App for simplicity. It is
not a fork. The flow, the storage and the refresh logic are identical; the only
difference is whether the token response contains `refresh_token` and
`expires_in`. So the implementation branches on the response:

- refresh token present: proactive refresh 10 minutes before expiry,
  single-flight, rotation persisted
- refresh token absent: the token is treated as long-lived and used until
  GitHub rejects it

The decision therefore stays a deployment choice rather than a rewrite, and an
OAuth App does not get 8-hour expiry handling bolted on that it cannot support.

**Why the refresh token is written before the access token.** GitHub invalidates
a refresh token as soon as it issues a replacement. If the process died after
persisting the new access token but before the new refresh token, the stored
refresh token would already be dead and the user would be signed out for good.
Writing refresh-first means the worst case is a stale access token, which is
exactly the case a refresh fixes.

**Why refresh is single-flight.** Two concurrent refreshes with the same
refresh token leave one caller holding a token the other has invalidated. The
manager serialises refresh behind an async gate and **re-checks after
acquiring it**, so 20 simultaneous requests collapse into one network call. A
test asserts that, rather than assuming it.

**Why keychain failure degrades to memory rather than failing closed.**
"Fail closed" here would mean refusing to run on a headless Linux box. The
property that matters is _never write a token to disk in plaintext_, and
in-memory storage satisfies it: the user signs in again after a restart (logged
as a warning), and no secret ever leaves the keychain abstraction.

**Why redirects are disabled on the HTTP client.** GitHub serves log archives
as a redirect to a signed CDN URL. `reqwest`'s default policy can replay
headers across hosts; with `Policy::none()` every redirect is resolved by hand,
and the follow-up request carries no `Authorization` header.

### ADR-011 — Device flow, and one auth path for both app types

**Why the device flow.** A desktop binary cannot keep a client secret, so the
authorization-code flow would be a fiction. The device flow needs only a public
client id, has no redirect URI to register or hijack, and keeps the user's
password inside their browser rather than an embedded webview. The cost is a
code the user must copy — which is why the UI treats that code as the primary
element of the screen, not a detail.

**Why the code does not care which app type you register.** The plan framed
this as a fork: GitHub App for refresh tokens, OAuth App for simplicity. It is
not a fork. The flow, the storage and the refresh logic are identical; the only
difference is whether the token response contains `refresh_token` and
`expires_in`. So the implementation branches on the response:

- refresh token present → proactive refresh 10 minutes before expiry,
  single-flight, rotation persisted
- refresh token absent → the token is treated as long-lived and used until
  GitHub rejects it

This means the decision stays a deployment choice, not a rewrite. It also means
an OAuth App does not silently get 8-hour expiry handling bolted on that it
cannot support.

**Why the refresh token is written before the access token.** GitHub invalidates
a refresh token as soon as it issues a replacement. If the process died after
persisting the new access token but before the new refresh token, the stored
refresh token would already be dead and the user would be signed out for good.
Writing refresh-first means the worst case is a stale access token, which is
exactly the case a refresh fixes.

**Why refresh is single-flight.** Two concurrent refreshes with the same
refresh token leave one caller holding a token the other has invalidated. The
manager therefore serialises refresh behind an async gate and **re-checks after
acquiring it**, so 20 simultaneous requests collapse into 1 network call. This
is asserted by a test rather than assumed.

**Why keychain failure degrades to memory rather than failing closed.** "Fail
closed" here would mean refusing to run on a headless Linux box. The property
that matters is _never write a token to disk in plaintext_, and in-memory
storage satisfies it: the user signs in again after a restart (logged as a
warning), and no secret ever leaves the keychain abstraction.

**Why redirects are disabled on the HTTP client.** GitHub serves log archives as
a redirect to a signed CDN URL. `reqwest`'s default policy can replay headers
across hosts; with `Policy::none()` every redirect is resolved by hand, and the
follow-up request carries no `Authorization` header.

### ADR-003 — Boxed error causes

`AppError` stores `Box<dyn Error + Send + Sync>` rather than concrete
`reqwest::Error` / `rusqlite::Error`. This keeps the public error surface
stable while milestones add dependencies, and it prevents crate-specific
detail from leaking into the wire contract. The trade-off is that callers
cannot `match` on the cause — they match on `AppError::code()`, which is the
contract the frontend already uses.

### ADR-004 — Redaction at the boundary, not at the call site

Every string leaving the process (log line, serialized error) passes through
`redact`. Relying on developers to remember not to log a token is a losing
strategy; a single choke point is auditable and testable.

`RedactingWriter` buffers a formatted record before writing because `io::Write`
hands out arbitrary byte chunks — a token can be split across two `write`
calls, and a naive streaming regex would miss it.

### ADR-005 — No `withGlobalTauri`

The frontend imports `@tauri-apps/api` explicitly. No `window.__TAURI__`
global, so the IPC surface is not reachable from injected content and the
bundle can be tree-shaken.

### ADR-006 — `allowScripts` pins esbuild

npm 11 refuses to run install scripts unless they are approved. `esbuild`
needs its postinstall to fetch the platform binary, so `package.json` carries:

```json
"allowScripts": { "esbuild@0.28.2": true }
```

**This pin must be bumped when esbuild is upgraded**, otherwise `npm ci`
installs a package without its binary and the Vite build fails with
`Cannot find package 'esbuild'`. CI runs `npm ci` and would catch it.

### ADR-007 — Two TypeScript projects, no project references

TypeScript 6 rejects `noEmit` in a referenced composite project and deprecated
`baseUrl`. The app config (`tsconfig.json`) and the tooling config
(`tsconfig.node.json`) are therefore independent, and `npm run typecheck`
checks both.

### ADR-008 — One status vocabulary, one motion gate

Two cross-cutting concerns are centralised rather than left to components:

- **Status.** `mapRunStatus()` in `src/lib/status.ts` is the only place that
  knows what GitHub's `status`/`conclusion` strings mean, and `StatusBadge` is
  the only component that renders them. The repository card, the tray icon and
  the notification rules must agree; duplicating a `switch` in each would let
  them drift, and a wrong badge is worse than a missing one.
- **Motion.** `useMotionAllowed()` is the only place that reads
  `prefers-reduced-motion` and `document.visibilityState`. An animation that
  loops in a hidden window is the most likely way to blow the <1% idle CPU
  budget, so the check lives in one hook that every animated component calls
  rather than in each component's CSS.

`conclusion` is treated as authoritative over `status`, because webhook
payloads can carry a conclusion while the status field is still stale. A
`completed` run with no conclusion maps to `unknown` rather than `success` —
never invent a green badge.

### ADR-009 — Coverage gate scoped to logic, not markup

Vitest coverage is measured over `lib/`, `stores/` and `components/status`.
Chasing 80% across pure presentational wrappers would produce tests that assert
class names rather than behaviour. The correct instrument for those is the
component gallery (visual) and the e2e sweep (M11-04).

## 4. Threat model

### Assets

| Asset                  | Where it lives | Impact if leaked                              |
| ---------------------- | -------------- | --------------------------------------------- |
| GitHub access token    | OS keychain    | Full repo access as the user                  |
| GitHub refresh token   | OS keychain    | Token minting after access expiry             |
| Webhook signing secret | OS keychain    | Forged build events                           |
| Cached repo data       | Local SQLite   | Private repo names, branches, commit messages |
| Log files              | App log dir    | Anything logged, if redaction fails           |

### Trust boundaries

1. **WebView ↔ Rust.** The WebView is the untrusted side. Every command
   validates its input; `system_open_url` allowlists `https://*.github.com`
   so the webview cannot be used as a generic protocol launcher.
2. **Network ↔ Rust.** All GitHub responses are untrusted input: parsed into
   typed models, never `eval`'d, never rendered as HTML.
3. **Relay ↔ Rust.** Webhook payloads arrive from a public relay. HMAC
   verification over raw bytes happens _before_ deserialization, and the local
   listener is bound to `127.0.0.1` behind a secret path segment.

### Mitigations in place (M0–M2)

- CSP with no `unsafe-eval`, no wildcard origins, `script-src 'self'`.
- Capability file grants `core:default` + `opener:allow-open-url` only — no
  `shell`, `fs` or `http` permissions.
- `#![forbid(unsafe_code)]` on both crate roots.
- Tokens are never serialized into a command response.
- Log redaction covers `gh[pousr]_`, `github_pat_`, bearer values, OAuth query
  parameters and signature headers.

### Explicit non-goals

- Defending against a compromised local OS account. Anything the app can read,
  that account can read.
- Hiding which repositories you monitor from GitHub. The API sees every request.

## 5. Coding rules

- No `unwrap`/`expect`/`panic!` in production paths; `?` and typed errors.
- No `unsafe`. If a dependency needs it, that is the dependency's business.
- No token, secret or full request URL in a log line, even at `trace`.
- Commands validate inputs before doing work (`limit`, `repo_id`, `ref`, URL).
- Every fallible user action surfaces an `AppError` with a stable `code`.
- Frontend never parses error strings; it branches on `code`.

## 6. Open decisions

Carried from the roadmap; each one changes the shape of a milestone.

| #   | Decision                                                       | Default if unresolved                 | Affects |
| --- | -------------------------------------------------------------- | ------------------------------------- | ------- |
| D1  | GitHub App (refresh tokens) vs OAuth App (non-expiring token)  | GitHub App                            | M2      |
| D2  | Webhook transport: relay only vs relay + tunnel + local server | Relay + local                         | M8      |
| D3  | Polling: budget-aware adaptive vs user-fixed interval          | Adaptive                              | M5      |
| D4  | Log retention: cache text vs fetch on demand                   | Fetch on demand                       | M7      |
| D5  | Product name / bundle identifier                               | `Pulse` / `dev.pulse.workflowmonitor` | shipped |

## 7. Known constraints of the current toolchain

| Constraint                   | Consequence                                                                                                                                                                                                                                                                              |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TypeScript 6                 | `baseUrl` removed; `noEmit` not allowed in composite references                                                                                                                                                                                                                          |
| Vite 8 (rolldown)            | `esbuild` is a separate install; `safari13` target cannot down-level destructuring                                                                                                                                                                                                       |
| npm 11                       | Install scripts require explicit approval (`allowScripts`)                                                                                                                                                                                                                               |
| Windows PowerShell policy    | `npm.ps1` is blocked on this machine — use `npm.cmd`                                                                                                                                                                                                                                     |
| One `sqlite3` link per graph | Only one crate in the graph may depend on `libsqlite3-sys`, so `rusqlite_migration` and a differently-versioned `rusqlite` cannot coexist. Cargo's MSRV-aware resolver also silently selected an ancient `deadpool-sqlite` until `rust-version` was raised to the real toolchain (1.85). |
