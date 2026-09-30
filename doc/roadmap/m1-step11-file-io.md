# M1 · Step 11 — Open, save, autosave, recent files

## Goal
F-ED-10 and F-IN-6 end to end in the app.

## Spec refs
F-ED-10, F-IN-6, NF-7.

## Depends on
M1-2, M1-8.

## Tasks
- Tauri plugins `dialog` and `fs` (v2; check the setup and the **capabilities/permissions** files with Context7: v2 denies fs access by
  default). Scope fs to user-chosen paths + the app config dir.
- Actions: New (`emptySong`, F-IN-6), Open (`*.madsister.json` filter → `parseSong`; errors shown in an alert region, the current song is
  kept), Save (to the current path or Save As), Save As (default name `<title>.madsister.json`).
  Shortcuts `Mod+N/O/S/Shift+S` (reserved in M1-10); add them to the help overlay.
- Dirty tracking: `present !== savedSong` (reference equality works thanks to immutability). Title bar shows `•` when dirty.
- Autosave: when the song has a path and is dirty, save 2 s after the last edit (debounce). Songs without a path aren't autosaved
  (`ponytail:` comment: no temp/recovery file; add it if the user loses work).
- Recent files: last 10 paths in `<appConfigDir>/recent.json`; a "Recent" list in the menu/start area; a missing file is removed from the list
  when opening fails.
- Keep the Tauri calls in `ui/fileActions.ts`, separate from `model/` (pure logic like the recent-list update is a pure function, tested).

## Tests
- Pure: `pushRecent(list, path)` (dedupe, move to front, cap at 10). Dirty detection.
- Component: mock `@tauri-apps/plugin-dialog`/`plugin-fs` (vi.mock) → Open with an invalid file shows an error and keeps the song;
  Save writes `serializeSong` output to the chosen path.

## Verify
`cd app && pnpm test && pnpm build && (cd src-tauri && cargo check && cargo clippy -- -D warnings)`

## Commit message
`feat(app): open, save, autosave and recent files`

## Out of scope
Native menus (in-app toolbar is enough for M1), file associations (M6).
