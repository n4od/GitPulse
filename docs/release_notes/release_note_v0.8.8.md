# Release v0.8.8 — signing out of one account

Signing out used to take you to a login page, even with another account still
signed in on the same machine. It now offers the accounts that are still there.

## What was wrong

The backend was already correct, and deliberately so. `sign_out` removes **only
the active account** from the keychain and leaves the others alone — with one
entry per account, signing out is a decision about an account rather than about
the machine. `auth_accounts` lists what remains, and `auth_switch_account` moves
to one without a browser or a device code.

The screen never asked. Three separate mistakes, all pointing the same way:

- **`useAccounts` was gated on being signed in**, with a doc comment claiming a
  signed-out window has no accounts. That is exactly backwards: signing out
  removes one account and leaves the rest, so signed-out is when the list
  matters most.
- **`AuthPanel`'s signed-out branch hardcoded a single button.** It rendered
  whatever `AuthState` it was handed and never asked which accounts existed.
- **The accounts query cached forever** (`staleTime: Infinity`), so even a fetch
  would have kept the account that had just left. Sign-out now invalidates the
  list _after_ the backend call resolves, not before.

## What it does now

```
Sign in to GitHub
These accounts are still signed in on this machine.
Pick one to carry on, or sign in with a different account.

[ Continue as n4od                 ]
[ Continue as birukboss            ]
[ Sign in with a different account ]
```

Each row switches to that account directly. With no other account stored, nothing
changes — it is the same single sign-in button as before.

## What was checked

- Unit tests pin the three states: no accounts (unchanged), several (one button
  each, calling back with the right login), and the click-through.
- **In a real run**, signing out of one account offered the other, and switching
  to it landed on that account's dashboard with no browser trip.
- 206 frontend tests, 190 backend, typecheck, lint and formatting clean.

## What this does not change

- **Nothing about what a token can reach.** This is a screen, not a permission;
  `0.8.7` is where the private-repository work landed.
- **The interface is untouched** apart from the signed-out screen.
- **macOS and Linux update installs remain unproven.** CI builds and signs both;
  nobody has watched one install.

---

## 📅 Recent Version History

- **v0.8.8**: Signing out of one account offers the others instead of a bare login page.
- **v0.8.7**: Private repositories without an install step; a client-id change forces a sign-in.
- **v0.8.6**: No borders, and a Settings panel rebuilt around its groups.
- **v0.8.5**: One status per repository, quiet hours read your clock, notifications are logged.
- **v0.8.4**: A version bump, so the install progress added in 0.8.3 can be watched.
- **v0.8.3**: An install reports preparing, downloading and installing as it goes.
- **v0.8.2**: "Failing" counts the latest run, not every failure ever cached; the taskbar badge fits its icon.
- **v0.8.1**: A version bump, to give `0.8.0` something to update to.
- **v0.8.0**: Auto-update — signed artifacts, a launch check, install on request.
- **v0.7.0**: The webhook subsystem removed; GitPulse no longer listens on a socket.
- **v0.6.2**: The account switcher moved from Settings to the avatar in the header.
- **v0.6.1**: `url` 2.5.9 for RUSTSEC-2026-0592, which was never reachable from the app.
- **v0.6.0**: Multiple accounts, switchable without signing in again.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, the MIT licence.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
