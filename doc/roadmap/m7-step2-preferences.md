# M7 · Step 2 — Preferences dialog

> Written on 2026-10-13 with the user.

## Goal
The user settings leave the sidebar for a dedicated dialog.

## Spec refs
F-UI-3, F-DS-1, F-DS-2, F-ED-1, F-ED-8.

## Depends on
M7-1.

## Tasks
- `ui/Preferences.tsx`: a `<dialog>` opened with `showModal()`, following the `About.tsx` pattern. It has every `Settings` field:
  chord style, font, Nashville minor convention, bars per row, low-confidence threshold and compact print.
  - Changes apply at once through `onSettingsChange`. The dialog has a Close button and Escape closes it.
  - The low-confidence threshold is always shown, not only for songs with confidence data.
- `Toolbar.tsx`: remove the Display fieldset and move its controls into `Preferences.tsx`. `Settings`, `parseSettings`,
  `mergeSettings` and `settings.json` don't change.
- Until M7-4, open the dialog from a "Preferences" button in the current `<nav>`.

## Tests
vitest: the Display cases already in `App.test.tsx`, enriched (no separate test file):
- Given the dialog is open, when the user changes the font, then `onSettingsChange` gets the new font.
- Given a song without confidence data, when the dialog opens, then the threshold field is shown.
- Given the sidebar is open, then it has no chord-style select.

## Verify
`cd app && pnpm test && pnpm build`.

## Check by hand
- [ ] Preferences → change the chord style and the font → the chart updates behind the dialog. After a relaunch, they are kept.
- [ ] Escape closes the dialog, and focus goes back to the button that opened it.

## Commit message
`feat(app): move the user settings into a Preferences dialog`
