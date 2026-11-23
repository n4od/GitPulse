# Release v0.8.2 — the badge tells the truth

Two fixes, one symptom: **the taskbar badge said a repository was failing when
nothing was.**

## What was wrong

The count behind the repository cards, the tray icon, the taskbar badge and the
**Failing** filter was _every failure the cache had ever seen_. It had no time
window, so a run that broke weeks ago and has passed several times since kept the
repository red forever.

On the machine this was found on, it reported **nine failing runs while all four
repositories' latest runs were green**. The badge was lit on history.

## What changed

**"Failing" now means failing now.** It counts the latest _completed_ run of each
workflow.

Two deliberate choices in that sentence:

- **Per workflow, not per branch.** A workflow that runs on tags would otherwise
  be red forever on whichever tag broke — that tag never runs again, so nothing
  can supersede it. This project's own `Release` workflow runs on thirteen tag
  branches, so the wrong choice here would have been permanent.
- **A fix in flight does not clear it.** A retry that is still running has not
  fixed anything; the badge clears when a green result lands, not when the intent
  to produce one appears.

**The badge is a badge.** The taskbar overlay reused the tray's full-bleed dot,
and Windows scales the whole image into a slot at the icon's corner — so it
rendered as a 14px disc spilling a pixel past the icon, reading as a blob. It is
now drawn with a margin: **10px on a 24px icon, 1px inside the corner.**

## What this means for you

- Repositories whose latest runs succeed no longer show a failing count, and the
  taskbar badge turns off when everything is green.
- If a workflow's most recent completed run failed, it is still shown as failing,
  exactly as before.
- **This is the first release whose update path nobody staged.** `0.8.1` was
  published purely to prove that an installed `0.8.0` could update itself. This
  one is the first that a _real_ installed copy will pick up, on all four targets
  at once, whether or not anyone is watching.

## What was verified before publishing

- With every latest run green, the taskbar button holds **zero red pixels** at
  12x, and the dashboard shows four `SUCCESS` cards with no failing badges.
- The overlay's geometry was photographed on a real taskbar, before and after.
- Six tests cover the new rule: a failure followed by a success counts as clear, a
  failure followed by an in-flight retry stays failing, each workflow counts at
  most once, a newer tagged run supersedes an older tag failure, and the
  single-repository count agrees with the dashboard.

## What this does _not_ change

- **The builds are still unsigned.** SmartScreen and Gatekeeper still warn on
  first launch. Auto-update is not code signing.
- **Showing the badge while a workflow is red is covered by tests, not by a
  photograph** — no repository here is currently failing.
- **macOS and Linux update installs remain unproven.** Both are built and signed
  the same way, but no update has been watched installing on either.

---

## 📅 Recent Version History

- **v0.8.2**: "Failing" counts the latest run, not every failure ever cached; the taskbar badge fits its icon.
- **v0.8.1**: A version bump, to give `0.8.0` something to update to.
- **v0.8.0**: Auto-update — signed artifacts, a launch check, install on request.
- **v0.7.0**: The webhook subsystem removed; GitPulse no longer listens on a socket.
- **v0.6.2**: The account switcher moved from Settings to the avatar in the header.
- **v0.6.1**: `url` 2.5.9 for RUSTSEC-2026-0592, which was never reachable from the app.
- **v0.6.0**: Multiple accounts, switchable without signing in again.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU verified.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
