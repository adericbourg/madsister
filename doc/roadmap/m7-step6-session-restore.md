# M7 · Step 6 — Reopen tabs at launch

> Written on 2026-10-13 with the user.

## Goal
The app reopens the tabs that had a file, as they were when it was last closed.

## Spec refs
F-UI-8.

## Depends on
M7-5.

## Tasks
- `session.json` in the config dir: `{ "paths": string[], "active": number }`. It is written with `writeConfigJson` whenever the
  tab list or the active tab changes, and read with `readConfigJson` at startup, like `recent.json`.
- Only tabs with a path are stored. Untitled tabs and start-screen tabs are not.
- At startup, open each path in order. Files that are missing or can't be read are skipped silently, and no error is shown. If
  `active` is out of range, the first tab is active.
- Invalid JSON is treated as an empty session.

## Tests
vitest, with the config and fs mocks:
- Given `session.json` with two existing paths and `active: 1`, when the app starts, then two tabs open and the second is active.
- Given one of them missing, then one tab opens and no alert is shown.
- Given an untitled tab and a saved one, when the tabs change, then `session.json` holds only the saved path.

## Verify
`cd app && pnpm test && pnpm build`.

## Check by hand
- [ ] Open three songs, make the second one active, and quit → relaunch: the same three tabs open, with the second active.
- [ ] Delete one of the files, then relaunch → the other two open, and no error is shown.

## Commit message
`feat(app): reopen the open tabs at launch`
