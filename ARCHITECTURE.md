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

| Path | Responsibility | Milestone |
| ---- | -------------- | --------- |
| `src-tauri/src/lib.rs` | Builder, plugin registration, window lifecycle | M0 |
| `src-tauri/src/error.rs` | `AppError` wire contract | M0 |
| `src-tauri/src/redact.rs` | Secret scrubbing for logs and errors | M0 |
| `src-tauri/src/logging.rs` | Rolling file log + redacting writer | M0 |
| `src-tauri/src/state.rs` | Shared process state | M0 |
| `src-tauri/src/commands/` | IPC surface, input validation | M0+ |
| `src-tauri/src/auth/` | Device flow, token store, refresh | M2 |
| `src-tauri/src/db/` | Schema, migrations, DAOs, retention | M3 |
| `src-tauri/src/github/` | Client, ETag cache, rate budget, endpoints | M4 |
| `src-tauri/src/poller/` | Scheduler, budget math, backoff | M5 |
| `src-tauri/src/webhook/` | Local server, relay, HMAC verification | M8 |
| `src/lib/ipc.ts` | Typed `invoke` wrapper, error normalisation | M0 |
| `src/lib/bindings/` | `ts-rs`-generated types (checked in) | M1+ |

## 3. Decision log

### ADR-001 — Tauri v2 over Electron

Required by the brief, but the reasoning holds: a ~10 MB binary and a shared
OS WebView instead of a bundled Chromium. The cost is per-OS WebView
divergence, which is why `vite.config.ts` targets the actual engine
(`chrome105` on Windows, `safari15` on macOS) rather than a lowest-common
denominator.

### ADR-002 — rusqlite over sqlx (planned, M3)

`sqlx`'s compile-time macros need a live `DATABASE_URL` or a checked-in
`.sqlx` offline cache, which makes a fresh clone fail to build. `rusqlite`
with `deadpool-sqlite` gives the same async ergonomics without build-time
coupling, and SQLite is single-writer anyway so the async advantage is small.

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

## 4. Threat model

### Assets

| Asset | Where it lives | Impact if leaked |
| ----- | -------------- | ---------------- |
| GitHub access token | OS keychain | Full repo access as the user |
| GitHub refresh token | OS keychain | Token minting after access expiry |
| Webhook signing secret | OS keychain | Forged build events |
| Cached repo data | Local SQLite | Private repo names, branches, commit messages |
| Log files | App log dir | Anything logged, if redaction fails |

### Trust boundaries

1. **WebView ↔ Rust.** The WebView is the untrusted side. Every command
   validates its input; `system_open_url` allowlists `https://*.github.com`
   so the webview cannot be used as a generic protocol launcher.
2. **Network ↔ Rust.** All GitHub responses are untrusted input: parsed into
   typed models, never `eval`'d, never rendered as HTML.
3. **Relay ↔ Rust.** Webhook payloads arrive from a public relay. HMAC
   verification over raw bytes happens *before* deserialization, and the local
   listener is bound to `127.0.0.1` behind a secret path segment.

### Mitigations in place (M0)

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

| # | Decision | Default if unresolved | Affects |
| - | -------- | --------------------- | ------- |
| D1 | GitHub App (refresh tokens) vs OAuth App (non-expiring token) | GitHub App | M2 |
| D2 | Webhook transport: relay only vs relay + tunnel + local server | Relay + local | M8 |
| D3 | Polling: budget-aware adaptive vs user-fixed interval | Adaptive | M5 |
| D4 | Log retention: cache text vs fetch on demand | Fetch on demand | M7 |
| D5 | Product name / bundle identifier | `Pulse` / `dev.pulse.workflowmonitor` | shipped |

## 7. Known constraints of the current toolchain

| Constraint | Consequence |
| ---------- | ----------- |
| TypeScript 6 | `baseUrl` removed; `noEmit` not allowed in composite references |
| Vite 8 (rolldown) | `esbuild` is a separate install; `safari13` target cannot down-level destructuring |
| npm 11 | Install scripts require explicit approval (`allowScripts`) |
| Windows PowerShell policy | `npm.ps1` is blocked on this machine — use `npm.cmd` |
