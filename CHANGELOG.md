# Changelog

All notable changes to **Pulse** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Scope:** this file records what has actually shipped or landed on `main`.
Forward-looking work lives in [ROADMAP.md](./ROADMAP.md) — do not duplicate it here.
Every entry should be traceable to a roadmap task ID (`M2-05`, etc.) where one exists.

---

## [Unreleased]

Milestone **M0 (Foundation)** is complete apart from the licence choice and the
first CI run on macOS/Linux — see [ROADMAP.md](./ROADMAP.md).

### Added

- **Project scaffold:** Tauri 2 desktop shell with React 19, TypeScript 6 and
  Tailwind CSS 4, plus a native window that starts hidden and shows on ready.
  (`M0-02`, `M0-03`, `M0-04`)
- **Design tokens:** obsidian-charcoal palette, status colours, radii and
  reduced-motion guards in `src/styles/tokens.css`. (`M0-04`)
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

### Security

- CSP without `unsafe-eval`, no wildcard origins, `script-src 'self'`.
- Capability set limited to `core:default` and `opener:allow-open-url`; no
  `shell`, `fs` or `http` permissions granted.
- `#![forbid(unsafe_code)]` on both crate roots.
- Log directory is created with the platform default ACLs; no telemetry.

### Known issues

- `npm ci` requires the `allowScripts` entry for `esbuild@0.28.2`; upgrading
  esbuild without bumping that pin breaks the Vite build.
- The macOS and Linux CI legs have not run yet.

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

| Version | Date | Theme | Milestones |
| ------- | ---- | ----- | ---------- |
| _unreleased_ | — | — | M0 |
| `0.1.0` | planned | Read-only dashboard | M0–M6 |
| `0.2.0` | planned | Actions & insight | M7 |
| `0.3.0` | planned | Live & native | M8–M9 |
| `0.4.0` | planned | Polish & hardening | M10–M11 |
| `1.0.0` | planned | Release-ready | M12 |

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

| Bump | When |
| ---- | ---- |
| **Major** | Local DB schema breaks without an automatic migration, or the update path requires user action. |
| **Minor** | New user-facing capability (new event types, new transports, new panels). |
| **Patch** | Fixes, perf work, dependency security bumps, copy changes. |

---

[Unreleased]: https://github.com/OWNER/pulse/commits/main
