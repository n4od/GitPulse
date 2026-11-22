# Release v0.5.6 — Leaner Releases

**No application code changed.** GitPulse is functionally identical to v0.5.5
apart from its version string. This release is about the packaging, which had
quietly accumulated three problems.

If you already run v0.5.5, there is no functional reason to reinstall.

## 🚀 Key Improvements

### 📦 One file per artifact

Every release used to publish each artifact **twice** — a versioned copy and a
version-less one — which is why a release held twenty assets. GitHub resolves
`releases/latest/download/<file>` by exact filename, so a name can either carry
the version or stay stable across releases, never both, and the pipeline had been
trying to do both.

There is now exactly one file per platform and format:

```
GitPulse-windows-x64-setup.exe
GitPulse-windows-x64.msi
GitPulse-windows-x64-portable.zip
GitPulse-macos-arm64.dmg
GitPulse-macos-arm64-portable.tar.gz
GitPulse-macos-x64.dmg
GitPulse-macos-x64-portable.tar.gz
GitPulse-linux-x64.deb
GitPulse-linux-x64.AppImage
```

Ten files instead of twenty. The names carry no version, deliberately: that is
what keeps the download links working as versions ship. The version you
downloaded is in the release title, and inside the package — Add/Remove Programs
on Windows, Get Info on macOS, `dpkg -I` on Linux.

### 🧹 Two dead files are gone from every release

Each release since v0.5.0 carried `GitPulse_aarch64.app.tar.gz` and
`GitPulse_x64.app.tar.gz` — about 12 MB per release, with nothing to read them.
The release action packaged the macOS `.app` into a tarball unconditionally while
preparing for an updater that does not exist, and uploaded it. The workflow no
longer lets the action upload anything: it builds, and the workflow itself names
and uploads the nine files.

### ✅ Downloads are now checkable

Each release ships a **`SHA256SUMS.txt`**. The release job re-downloads all nine
artifacts and hashes what actually arrived, rather than trusting the build
directory. These bundles are **unsigned**, so a checksum is the only integrity
signal you have:

```bash
sha256sum --check SHA256SUMS.txt
```

The job also refuses to publish a set of artifacts that is not _exactly_ the nine
expected ones. An unexpected file is a sign the pipeline changed underneath us —
which is how those two macOS tarballs went unnoticed for four releases — and a
missing one fails the run instead of reaching a download button.

## ⚠️ What did not change

- **The application is identical to v0.5.5.** Only the version string differs.
- **The builds are still unsigned.** Windows SmartScreen and macOS Gatekeeper ask for confirmation on first launch; right-click → **Open** on macOS, or _More info → Run anyway_ on Windows.
- **There is still no auto-update.** Upgrading means downloading the new build.
- **Still unproven**: a real webhook delivery from GitHub, the notification click deep-link, and runtime behaviour on macOS and Linux — CI builds and tests both, but nobody has launched the app on either. [`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md) tracks which claims are measured and which are not.

---

## 📅 Recent Version History

- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, `Mine` filter, a way back into the picker, and the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU and token-refresh verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems, green CI.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
