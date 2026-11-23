# Release v0.8.6 — no borders

Every hairline outline in the interface is gone. Nothing else about the layout
changed.

## What changed

The app had a one-pixel border on nearly everything: cards, panels, the filter
group, inputs, buttons, status pills, the dividers between settings sections. That
is roughly eighty borders doing work that colour can do better, and on a dark
palette they mostly read as noise.

Separation now comes from **surface fills** instead:

| Layer                             | Fill                    |
| :-------------------------------- | :---------------------- |
| page                              | `surface` — the darkest |
| cards                             | `raised`                |
| raised panels (Settings, dialogs) | `overlay`               |
| controls that sit on any of them  | `field` — new           |

Only the border **colour** was removed. `border-width` is still there, so
components keep their geometry and nothing moved by a pixel.

## The three controls that needed more than that

Removing the hairlines exposed three places where the border had been doing the
entire job of saying "this is a control":

- **The search field** drew a blue ring _and_ a blue outline when clicked.
  Browsers treat a text field as keyboard-focusable, so it fired on an ordinary
  mouse click too. Both are gone; a text field now shows focus with the caret and
  its own fill.
- **The quiet-hours field and the Settings buttons** sat on the Settings card,
  which is `raised` — the exact fill they used. With the border gone they
  disappeared into the panel. They use the new `field` fill, which is one clear
  step above `raised`.

## What was checked

This is a visual change, and it was judged by looking at it rather than by tests.
The tests are unchanged and still pass; typecheck, lint and the formatting gate
are clean. No screenshots are committed — the app is the source of truth for how
it looks.

## What this does not change

- **Nothing moves.** Border width is untouched, so every layout is identical.
- **Status colour is untouched.** Pills, error banners and toasts keep their
  tinted fills; only their outlines went.
- **Focus is still visible** for everything except text fields, which opt out of
  the blue outline and rely on the caret. That is a deliberate reduction and the
  one accessibility trade in this release.
- **The unsigned-build position is unchanged.** SmartScreen and Gatekeeper still
  warn on first launch.

## One thing to know for future work

The rule that removes borders is a single unlayered block in `global.css`, which
beats Tailwind's utility layer without `!important`. The consequence: a
`border-*` class added to a component is inert until that block is removed. If a
real border is ever wanted, give that element an explicit rule rather than
fighting the block.

---

## 📅 Recent Version History

- **v0.8.6**: No borders. Separation comes from surface fills.
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
