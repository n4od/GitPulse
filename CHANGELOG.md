# Changelog

All notable changes to **Pulse** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Scope:** this file records what has actually shipped or landed on `main`.
Forward-looking work lives in [ROADMAP.md](./ROADMAP.md) — do not duplicate it here.
Every entry should be traceable to a roadmap task ID (`M2-05`, etc.) where one exists.

---

## [Unreleased]

Nothing implemented yet. The project is at milestone **M0** (Foundation) — see
[ROADMAP.md](./ROADMAP.md#m0--foundation).

### Added

- Project scaffolding and tracking documents:
  - `ROADMAP.md` — milestone plan, task IDs, exit criteria, open decisions.
  - `CHANGELOG.md` — this file.

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
