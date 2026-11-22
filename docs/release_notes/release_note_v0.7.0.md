# Release v0.7.0 — Local Only

**The webhook feature is gone.** This release removes it completely, and that is
the only thing in it.

If you never turned webhooks on, nothing about your day changes except that the
app no longer contains a listener you were not using.

## 🔒 Why it went

It worked. That is what decided it — a real GitHub delivery over an `ngrok`
tunnel, signature verified, refresh queued, eight deliveries accepted. Verifying
it turned up three things that reasoning had not:

- **The relay transport could never work.** `smee.io` re-encodes the JSON body,
  so the bytes GitHub signed no longer exist by the time a listener sees them.
  Every delivery was refused — not misconfigured, impossible.
- **The local transport needed a public URL, a tunnel, and a port that survived a
  restart.** The port did not: it was chosen by the OS and recorded as `0`, so the
  URL pasted into GitHub broke on the next launch.
- **What it bought was latency, and only latency.** The poller is the only writer
  by design, so a webhook could not patch the cache — it queued a refresh and
  spent the same API call sooner. The rate-limit saving people expect needs the
  poll interval to stretch when pushes cover the gaps, and that was never built.

Against that: it was the only thing in GitPulse that listened on a socket. The
README says everything stays on your machine; now that is literally true.

## 🧹 What was removed

|                  |                                                                                                                                                 |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **Code**         | The local server, the relay client, the signature verifier, the transport panel, five commands — about 1,900 lines                              |
| **Dependencies** | `axum`, `hmac`, `sha2`, `subtle`, `hex` — all five existed only for this                                                                        |
| **Database**     | Migration `0005` drops `webhook_deliveries`; `0006` deletes the leftover settings, including a path secret for a listener that no longer exists |
| **A secret**     | No command returns a secret any more. `webhook_signing_secret` was the only one                                                                 |

## ⚠️ What you will notice

- **Updates arrive at the poll interval** — every 60 seconds at the fastest, longer
  for a large watchlist — instead of within a second of a run finishing.
- **Settings no longer has a Webhooks button**, and the palette no longer offers
  "Configure webhooks".
- **Your existing cache is untouched** apart from the two migrations. If you had
  webhooks configured, their settings matter no longer and are deleted.
- An account you do not open again keeps an empty `webhook_deliveries` table in
  its file: migrations run per account, when that file is opened. Nothing reaches
  it, and it resolves the next time you use that account.

## ⚠️ What did not change

- **Polling, storage, authentication and the account switcher are untouched.**
- **The builds are still unsigned**, and there is still no auto-update.
- **Still unproven**: the notification click deep-link, the Windows taskbar overlay icon on a real taskbar, the settings/palette accessibility audit, and runtime behaviour on macOS and Linux. [`VERIFICATION.md`](https://github.com/n4od/GitPulse/blob/main/VERIFICATION.md) records exactly which claims are measured and which are not — including the webhook delivery that was verified before this release removed it.

---

## 📅 Recent Version History

- **v0.7.0**: The webhook subsystem removed; GitPulse no longer listens on a socket.
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
