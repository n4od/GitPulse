# Release v0.6.2 — The Account Menu

The account switcher moved to where the account is. Nothing else about the
application changed.

## 🚀 Key Improvements

### 👤 Switching accounts, from the avatar

The account list used to be a section in **Settings**, sitting beside
notifications and polling. That made "which account am I on?" look like a
behaviour preference rather than an identity, and it split the three account
actions across two places — switching in Settings, adding an account in Settings,
signing out at the bottom of Settings.

Clicking the **avatar in the header** now opens a menu:

```
  (you) n4od          ✓
  (you) birukboss
  ─────────────────────
  +     Add another account...
  ⇥     Sign out of n4od
```

Every account shows **its own circular picture**, and the active one is ticked.
`Ctrl+K` still offers each account by name, for the keyboard.

### ⚙️ Settings is about behaviour again

Notifications, quiet hours, window, polling, diagnostics, webhooks — and nothing
about who you are. The account fieldset and the sign-out button are gone; both
live in the menu.

### ⌨️ Menus behave like menus

The menu now moves between rows with **↑ / ↓ / Home / End**, closes on **Esc**,
and **returns focus to whatever opened it**. The repository cards' right-click
menu gets all of that for free, since it is the same primitive.

## 🐛 Fixed

- **The menu could not be dismissed by the button that opened it.** A click
  outside closes it, and that handler runs before the trigger's own click — so
  pressing the avatar closed the menu and immediately reopened it. It now treats
  a click on its own trigger as a toggle.

## ⚠️ What did not change

- **Nothing about polling, storage or authentication.** This is a change of place, not of behaviour.
- **The builds are still unsigned**, and there is still no auto-update.
- **Still unproven**: a real webhook delivery from GitHub, the notification click deep-link, and runtime behaviour on macOS and Linux. [`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md) records exactly which claims are measured and which are not.

---

## 📅 Recent Version History

- **v0.6.2**: The account switcher moved from Settings to the avatar in the header, with pictures and a tick.
- **v0.6.1**: `url` 2.5.9 for RUSTSEC-2026-0592, which was never reachable from the app; the audit now names what it finds.
- **v0.6.0**: Multiple accounts, switchable without signing in again; per-account tokens and cache; the tray and notifications name the account.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, `Mine` filter, a way back into the picker, and the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU and token-refresh verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems, green CI.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
