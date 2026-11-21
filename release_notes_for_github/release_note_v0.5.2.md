# Release v0.5.2 — One-Click Downloads

No application code changed in this release. It is about getting the download right: the README's buttons now fetch the file directly, and the pipeline publishes version-less copies so those links survive every future release.

## 🚀 Key Improvements

### 📦 Downloads

- **The buttons download the file**: Windows, macOS on Apple Silicon, macOS on Intel and Linux each have their own button that hands the browser the installer straight away, instead of opening the releases page.
- **Permanent links**: every artifact is now published twice — the versioned bundle (`GitPulse_0.5.2_x64-setup.exe`) and a version-less copy (`GitPulse-windows-x64-setup.exe`). A link can point at `/releases/latest/download/GitPulse-windows-x64-setup.exe` and keep working across releases.
- **Two macOS buttons**: Apple Silicon and Intel are separate, because a single button cannot know which Mac it is running on.
- **Everything is linked, not just the buttons**: the `.msi`, the portable `.zip` and `.tar.gz`, and the `.deb` are one click from the table.
- **A guard against dead links**: the verify job refuses to publish a draft that is missing any stable-named asset, so a broken download button fails the release instead of reaching a user.
- **Verified through the real redirect chain**: all nine stable links were resolved through GitHub's redirect to the release CDN and returned 200 — the presence of the asset in the API was not treated as proof.

### 🎨 README

- **Status badges**: release version, total downloads, supported platforms and the Tauri version at the top of the page.
- **A download section** that says what each platform gets, and why the builds are unsigned.
- **The stale status line is gone**: it still claimed "M0 (foundation) complete — authentication lands in M2", two milestones out of date.

### 🛠️ Tooling

- **Stable names live in the release workflow**, beside the versioned upload, so nothing has to be renamed by hand after a release.
- **Release notes for v0.5.1 and v0.5.2** are checked in and written against the previous note as the baseline, so they describe the delta rather than repeating everything.

## ⚠️ What did not change

- **The application is identical to v0.5.1** apart from its version string. If you already run v0.5.1, there is no functional reason to reinstall.
- **The builds are still unsigned.** Windows SmartScreen and macOS Gatekeeper ask for confirmation on first launch; right-click → **Open** on macOS, or _More info → Run anyway_ on Windows.
- **Still unproven**: a real webhook delivery from GitHub, and runtime behaviour on macOS and Linux. [`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md) tracks which claims are measured and which are not.

---

## 📅 Recent Version History

- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU and token-refresh verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems, green CI.
- **v0.4.0**: Settings panel, command palette, diagnostics, and a hardening pass that produced `VERIFICATION.md`.
- **v0.3.0**: Webhook ingestion, tray icon, notifications, single instance, opt-in autostart.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
