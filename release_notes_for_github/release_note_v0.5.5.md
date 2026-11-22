# Release v0.5.5 — The Dashboard Pass

This release is one pass over the dashboard, plus two fixes that were made
underneath it. Nothing about how GitPulse polls or stores changed.

It also carries the **account-isolation fix** that was staged for v0.5.3 but
never published — if you signed in as a different GitHub account, the old
account's repositories could still be on screen. That is fixed here.

## 🚀 Key Improvements

### 🗂️ Acting on a repository

- **Right-click menu on a card**: refresh, copy the run URL, mute notifications and open on GitHub. These were only visible on hover, which meant you had to already know they were there. The menu closes on Escape, on a click outside and on scroll, and it stays inside the window when you open it near an edge.
- **Add repositories from the header**: the repository picker could previously only be opened from the empty state, so once you had tracked one repository there was no way to add another. There is now an **Add repositories** button next to the watch count, and a **Track repositories** command in the palette.

### 🔴 Knowing why something failed

- **A per-repository error state**: a repository that cannot be read shows the failure on its own card, with a **Retry** that re-queues just that repository instead of all of them.
- **Enough detail to act on**: the run inspector shows what kind of failure it was (`No access`, `Rate limited`, `Repository not found`, …), a hint, when it happened, and the raw message underneath. The card itself stays a single line on purpose.
- **It clears itself**: the message is written on a failed poll and cleared on the next successful one, so it can never outlive the problem it describes.

### 📐 Fitting more on screen

- **Compact card size**: a density toggle in the header fits roughly a third more repositories on a screen. Nothing is dropped from the card — it is the same card, tightened — and the choice is remembered across restarts.
- **A `Mine` filter**: shows only the repositories the signed-in account owns, alongside `All` / `Failing` / `Running`, so a watch list full of organisation repositories can be narrowed to your own. The two combine, so `Mine + Failing` works.

### 🔐 Project licence

- **GitPulse is now MIT.** The repository had no licence at all, which is not a neutral state — without one the default is _all rights reserved_, while these pages hand out installers. `Cargo.toml` and `package.json` declare the same licence, so they agree with the file rather than merely pointing at it.

## ⚠️ What did not change

- **The builds are still unsigned.** Windows SmartScreen and macOS Gatekeeper ask for confirmation on first launch; right-click → **Open** on macOS, or _More info → Run anyway_ on Windows.
- **There is still no auto-update.** The updater plugin is not installed, so upgrading means downloading the new build.
- **Still unproven**: a real webhook delivery from GitHub, the notification click deep-link, and runtime behaviour on macOS and Linux — CI builds and tests both platforms, but nobody has launched the app on either. [`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md) tracks which claims are measured and which are not.

---

## 📅 Recent Version History

- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, `Mine` filter, a way back into the picker, and the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU and token-refresh verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems, green CI.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
