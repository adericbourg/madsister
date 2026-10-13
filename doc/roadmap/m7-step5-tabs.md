# M7 · Step 5 — Tabs

> Written on 2026-10-13 with the user.

## Goal
Several songs open at once, one tab each.

## Spec refs
F-UI-7, F-ED-10, F-PB-1.

## Depends on
M7-4.

## Tasks
- Move the per-song state out of `App.tsx` into `ui/SongTab.tsx`: `useHistory`, `savedSong`, `path`, `mode`, `audioNotice`,
  `error`, autosave, `locateAudio`, the start screen (new grid / import), and the editor with its sidebar.
  - `App.tsx` keeps the tab list (`{ id, path, title, isDirty }`), the active tab id, settings, recent files and the menu.
  - Menu and ribbon actions (save, export, print, undo, mode…) go to the active tab. Use a ref handle set by each tab, or lift
    only what the menu needs. Choose the smaller diff and say which one you chose in this file.
- Inactive tabs stay mounted with `hidden`. That keeps the undo stacks and a running import.
- `ui/TabBar.tsx`, between the ribbon and the content: one tab per song, labelled with the title, "•" when dirty, and a close ×
  button. The active tab uses `--accent`. ARIA `tablist`/`tab`/`tabpanel`, and ⌃Tab / ⌃⇧Tab cycle through the tabs. The bar is
  shown as soon as one tab exists.
- New tab (⌘N) adds a tab on the start screen. Creating the grid or finishing the import turns that same tab into the song.
- Open: if the path is already open, focus its tab. Otherwise open a new tab. Exception: if the active tab is an untouched start
  screen, open the song in it.
- Close tab (⌘W, ×): a dirty tab asks `confirmDiscard`. Closing the last tab shows the empty start screen.
- Window close (`getCurrentWindow().onCloseRequested`): if a tab is dirty, confirm. Otherwise close.
- Playback: only the active tab plays. Switching tabs pauses the player of the tab you leave.
- The window title is the active tab's title.

## Tests
vitest (`App.test.tsx`, `TabBar.test.tsx`), with the fs/dialog mocks already used:
- Given one open song, when the user opens a second file, then there are two tabs and the second is active.
- Given a file already open in a tab, when the user opens it again, then no tab is added and its tab is active.
- Given a dirty tab, when the user closes it and declines the confirm, then the tab stays.
- Given two tabs, when the user edits tab 1, switches to tab 2 and back, then tab 1 still has its edit and its undo.
- Given New tab, when the user creates a grid, then the same tab shows the grid (the tab count doesn't change).
- Given two tabs, when the user presses ⌃Tab, then the other tab becomes active.

## Verify
`cd app && pnpm test && pnpm build`.

## Check by hand
- [ ] Open two songs, edit both, and switch: each keeps its own state, and the "•" follows the dirty state.
- [ ] Start an audio import in a new tab, switch to another tab, then come back: the import is still running, then shows the grid.
- [ ] Play in tab 1, then switch to tab 2 → playback pauses.
- [ ] Quit with an unsaved untitled tab → confirm, and Cancel keeps the window open.
- [ ] VoiceOver announces the tabs as "tab, 1 of 2, selected".

## Commit message
`feat(app): open several songs in tabs`
