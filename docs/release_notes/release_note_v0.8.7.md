# Release v0.8.7 — private repositories, without an install step

GitPulse listed only public repositories for one user, and nothing was broken.
The app is a GitHub App that had been _authorized_ but never _installed_: two
separate grants, where authorizing only says who you are and installing is what
grants repository access. A user token can only reach the intersection of what
the user and the app can access, so an uninstalled app reaches public
repositories alone. No error said so, which is what made it expensive.

## What changed

**GitPulse is now pointed at an OAuth App.** An OAuth App has no installation
and no granted-repositories list: authorize once, and the token reaches every
repository you can. That removes the concept rather than documenting it better.

Two bugs had to be fixed first, both of which would have made the switch look
broken:

- **Scopes were read only at runtime.** The client id already fell back to a
  compile-time value through `option_env!`; the scopes did not. A packaged build
  never sees a shell environment, so an OAuth App release would have asked for
  **no scopes**, received a token that works, and shown public repositories only —
  the same failure arriving by a different road. Both now resolve runtime-first,
  compile-time second, and the release workflow supplies each from a repository
  variable.
- **A token's reach belongs to the app that minted it.** Repointing the client id
  does not upgrade the token already on the machine: it keeps validating, with
  the old app's reach. Tokens now record which app minted them, and a mismatch is
  discarded with a request to sign in again. **Your first launch will sign you
  out. That is the fix working, not a regression.**

The startup line now names the scopes, which is the one field that tells an OAuth
App from a GitHub App from the outside:

```
gitpulse starting version="0.8.7" ... client_id_configured=true scopes=repo workflow read:user
```

## The trade, stated plainly

An OAuth App has no read-only scope. `repo` grants **read and write** over every
private repository the account can reach, and the token does not expire, so
there is nothing to refresh. A GitHub App can be held to `Actions: Read` and
`Contents: Read`, at the cost of one installation click.

This release takes the no-install side of that trade. The GitHub App path is
still supported and still documented — now including the install step the README
had been missing, which is the documentation gap that caused the problem above.

## What was checked

- The client id answers GitHub's device-code endpoint: `200`, with a user code, a
  verification URI and a 15-minute expiry.
- A build compiled with `GITPULSE_GITHUB_SCOPES` and launched with that variable
  **unset** still reported `scopes=repo workflow read:user`. Before the change the
  same line read `scopes=`.
- A token minted by another app is dropped at startup: a unit test covers the
  truth table, and a real run logged `the stored token was minted by a different
GitHub app; asking for a sign-in`.
- **A real sign-in, end to end:** after authorizing the OAuth App, a private
  repository (`is_private = 1`) appeared in the picker and reached the cache.

Not proven: the same flow on macOS or Linux. CI builds and signs both, but nobody
has run them.

## What this does not change

- **Nothing about the interface.** This release is authentication only; `0.8.6`
  owns the borderless look and the rebuilt Settings panel.
- **Builds remain unsigned.** SmartScreen and Gatekeeper still warn on first
  launch.
- **Tokens still live only in the OS keychain**, and a keychain that cannot be
  reached still degrades to session-only storage rather than writing a secret to
  disk.

---

## 📅 Recent Version History

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
