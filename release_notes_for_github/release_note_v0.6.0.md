# Release v0.6.0 — Multiple Accounts

GitPulse can now stay signed in to **more than one GitHub account** and move
between them without going through a browser again.

## 🚀 Key Improvements

### 👥 More than one account, one at a time

**One account is active, not all of them at once.** That is the decision the
whole feature rests on, and it is deliberate: polling every account
simultaneously is a different product. The dashboard would have to disambiguate
`acme/api` belonging to two accounts, a notification could not say _whose_ build
failed, and the webhook listener is one socket on one port while the signing
secret is per account. The reasoning, and what was rejected, is recorded as
[ADR-020](https://github.com/n4od/GitPulse/blob/main/ARCHITECTURE.md#adr-020--one-active-account-and-one-database-file-per-account).

- **Switching is instant.** Every account's tokens were stored when it signed in, so a switch is a keychain read rather than a round trip. Tokens that need renewing are renewed for the account you are switching _to_, and only that one.
- **Add another account** from **Settings → Accounts**. It runs the normal sign-in flow and leaves the account you are already in untouched.
- **The command palette offers each account by name** — `Ctrl+K`, type a login, Enter.
- **Signing out names the account it applies to**, because it now signs out of one account rather than the machine.

### 🔐 Isolation that is structural, not a filter

Two things are now kept per account, so that one account's data cannot reach the
other:

- **Tokens**, as `github-access-token:<login>` in the OS keychain, alongside an entry recording which accounts this machine knows and which one is active. Previously the keys were fixed strings, so a second account had nowhere to put its tokens without overwriting the first.
- **The cache**, as `gitpulse-<login>.db` rather than one `gitpulse.db` with an `account` column on every table. Another account's rows are not in the database to leak, which matters because the previous version of this cache is exactly how one account's repositories once appeared under another.

### 🗂️ Upgrading from 0.5.x keeps what you had

- **Your sign-in carries over**: a keychain written by `0.5.6` or earlier is adopted on the first launch, so you are not asked to sign in again.
- **Your cache carries over**: the old `gitpulse.db` is filed under the account recorded inside it — a rename, not a migration, after a write-ahead-log checkpoint so nothing un-checkpointed is left behind.

### 🔔 The tray and notifications say whose build it was

The tooltip reads **"GitPulse — n4od — build failures"** and a failure
notification is titled **"Build failed — n4od"**. With two accounts able to watch
a repository called `api`, a message without an owner is ambiguous.

### 🐛 Fixed: cancelling an add-account flow signed you out

Adding a second account works by signing in again, so cancelling that flow used
to publish "signed out" — even though you were still signed in to the first
account. It now returns you to the session that was already there. A genuine
failure is still reported as one.

## ⚠️ What did not change

- **The builds are still unsigned.** Windows SmartScreen and macOS Gatekeeper ask for confirmation on first launch; right-click → **Open** on macOS, or _More info → Run anyway_ on Windows.
- **There is still no auto-update.** Upgrading means downloading the new build.
- **Still unproven**: a real webhook delivery from GitHub, the notification click deep-link, and runtime behaviour on macOS and Linux — CI builds and tests both, but nobody has launched the app on either. [`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md) records exactly which claims are measured and which are not.

---

## 📅 Recent Version History

- **v0.6.0**: Multiple accounts, switchable without signing in again; per-account tokens and cache; the tray and notifications name the account.
- **v0.5.6**: One file per artifact under stable names, dead assets removed, published checksums.
- **v0.5.5**: Context menu, per-repository errors with detail, compact cards, `Mine` filter, a way back into the picker, and the MIT licence.
- **v0.5.2**: One-click downloads and permanent, version-less download links.
- **v0.5.1**: Installers that can sign in, dashboard restyle, idle-CPU and token-refresh verified.
- **v0.5.0**: First installable release — installers and portable builds on three operating systems, green CI.

---

**Full Changelog**: [CHANGELOG.md](https://github.com/n4od/GitPulse/blob/main/CHANGELOG.md)
