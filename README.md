# Pulse

A local-first desktop monitor for GitHub Actions workflows, deployments and
pull requests. Tauri 2 + Rust core, React 19 frontend, everything cached on
your machine.

> **Status:** M0 (foundation) complete — the window, toolchain and CI exist.
> Authentication lands in M2. See [ROADMAP.md](./ROADMAP.md).

## Requirements

| Tool     | Version | Notes                                                             |
| -------- | ------- | ----------------------------------------------------------------- |
| Rust     | 1.77+   | MSVC toolchain on Windows                                         |
| Node     | 22+     |                                                                   |
| npm      | 11+     |                                                                   |
| WebView2 | any     | Preinstalled on Windows 11; the installer bootstraps it otherwise |

Linux also needs `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`,
`librsvg2-dev`, `patchelf` and `libsecret-1-dev`.

## Getting started

```bash
npm ci
npm run tauri dev
```

On Windows, if `npm` is blocked by the PowerShell execution policy, use
`npm.cmd` instead — the `.ps1` shim cannot load under `Restricted`.

## Scripts

| Command               | What it does                                            |
| --------------------- | ------------------------------------------------------- |
| `npm run dev`         | Vite dev server on `:1420` (Tauri's `beforeDevCommand`) |
| `npm run tauri dev`   | Full desktop app with hot reload                        |
| `npm run build`       | Typecheck + production frontend bundle                  |
| `npm run tauri build` | Signed-less installers for the current OS               |
| `npm run typecheck`   | `tsc --noEmit` over app and tooling configs             |
| `npm run lint`        | ESLint, zero warnings tolerated                         |
| `npm run test`        | Vitest                                                  |
| `npm run format`      | Prettier                                                |

Rust side:

```bash
cargo fmt   --manifest-path src-tauri/Cargo.toml --all
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml --all-targets
```

## Project layout

```
src/              React application
src/lib/ipc.ts    the only bridge to Rust
src-tauri/        Rust core (commands, auth, db, github, poller, webhook)
src-tauri/migrations/   SQLite migrations
ARCHITECTURE.md   decisions, threat model, module map
ROADMAP.md        milestones and task IDs
CHANGELOG.md      what shipped
```

## GitHub authentication

Pulse uses the OAuth 2.0 **device flow**, so there is no client secret in the
binary and no personal access token to paste. Registration steps are
documented in M2 of the roadmap and will land with the auth milestone.

## Troubleshooting

**`Cannot find package 'esbuild'` during `npm run build`**
npm 11 skipped esbuild's postinstall. `package.json` pins the approved version
in `allowScripts`; run `npm install-scripts approve esbuild` after upgrading
esbuild.

**`npm : File npm.ps1 cannot be loaded`**
PowerShell execution policy. Use `npm.cmd`.

**Blank window on Linux**
Missing `WEBKIT_DISABLE_COMPOSITING_MODE=1` on some drivers:
`WEBKIT_DISABLE_COMPOSITING_MODE=1 npm run tauri dev`.

## License

Not yet chosen — see `ROADMAP.md` M0-01.
