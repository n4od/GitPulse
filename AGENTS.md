# AGENTS.md — GitPulse

Tauri v2 desktop app: Rust core in `src-tauri/`, React 19 + TypeScript in `src/`.
Types crossing the IPC boundary are generated from Rust into `src/lib/bindings/` —
**never edit those by hand**. `npm.cmd run bindings` regenerates them (ts-rs).

Read `D:\CODEs\AGENTS.md` for the shell quirks that apply to every project here
(`npm.cmd`, ASCII commit messages, no `gh`, PowerShell 5.1).

## Commands

|                                                                 |                                                       |
| :-------------------------------------------------------------- | :---------------------------------------------------- |
| `npm.cmd run dev`                                               | Vite only. For the app, `npm.cmd run tauri dev`       |
| `npm.cmd run tauri build -- --debug --no-bundle`                | debug binary at `src-tauri/target/debug/gitpulse.exe` |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` | Rust tests                                            |
| `cargo test --manifest-path src-tauri/Cargo.toml <name>`        | one Rust test by name                                 |
| `npm.cmd run test`                                              | vitest, one pass                                      |
| `npm.cmd run test -- src/lib/repo-selectors.test.ts`            | one frontend file                                     |

## The order that matters

`cargo test` **regenerates `src/lib/bindings/*.ts` in ts-rs's raw style**, so
`npm.cmd run format` must run _after_ it. Skip that and the tree looks dirty for
no reason — bindings differ only in line breaks, not content.

Before committing, from the repo root:

```
cargo fmt   --manifest-path src-tauri/Cargo.toml --all
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml --all-targets
npm.cmd run format      # after cargo test — see above
npm.cmd run typecheck
npm.cmd run lint
npm.cmd run test
```

CI (`.github/workflows/ci.yml`) runs the same across three OSes plus a dependency
audit. **That audit reads the live advisory database**, so it can go red with
nobody having changed a dependency — that is not a broken tree, and the log now
names the crate, the advisory and the fix.

## Load-bearing, easy to break

- **Migrations are append-only.** Never edit a shipped `.sql`. Add a file under
  `src-tauri/migrations/`, register it in `src-tauri/src/db/migrate.rs`, and bump
  the two assertions in `src-tauri/src/db/tests.rs` (latest version, applied
  count).
- **One database per account**, `gitpulse-<login>.db` under
  `%APPDATA%\dev.gitpulse.workflowmonitor`. Tokens are in the OS keychain as
  `github-access-token:<login>`, with `github-accounts` and
  `github-active-account` alongside. `meta.account_login` is a **cross-check**, not
  the record of ownership.
- **Exactly one account is active, by design** (ADR-020). Simultaneous polling is
  excluded on purpose; the ADR says what it would cost.
- **The poller is the only writer** (ADR-016). Nothing else patches the cache.
- **A version bump touches four files**: `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`, `src-tauri/Cargo.lock`.
- **The client id is compiled in.** CI passes the repository variable
  `GITPULSE_GITHUB_CLIENT_ID` through `option_env!`; a packaged app never reads
  `.env`. For a local run, set that variable in the environment before launching.

## Read before changing behaviour

- `ARCHITECTURE.md` — the ADRs, with the alternatives that were rejected. ADR-020
  (one active account) and ADR-021 (why webhooks were removed) define the current
  shape.
- `VERIFICATION.md` — the honesty ledger: measured, mocked, unproven. **Update it
  when you verify or remove something.** It is the reason the project's claims are
  worth reading.
- `ROADMAP.md`, `CHANGELOG.md`, `RELEASING.md` — plan, history, release procedure.

## Releases

A `v*` tag builds a **draft** with nine version-less artifacts plus
`SHA256SUMS.txt`, so `/releases/latest/download/<name>` keeps working across
releases. The `verify` job asserts the exact set _and_ that `README.md` still
links to all nine. Publishing is a manual step; notes live in `docs/release_notes/`.
