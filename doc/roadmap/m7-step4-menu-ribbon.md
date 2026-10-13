# M7 · Step 4 — Native menu and ribbon

> Written on 2026-10-13 with the user.

## Goal
File actions move to a native menu that follows platform conventions, and a short ribbon holds the most frequent ones. The
`<nav>` goes away.

## Spec refs
F-UI-5, F-UI-6, F-ED-7, F-ED-10, F-EX-1..4.

## Depends on
M7-2 (Preferences), M7-3 (Show sidebar).

## Tasks
- Before coding, check with Context7 (Tauri v2, `@tauri-apps/api/menu`):
  - that a menu built from JS can use predefined items (Quit, Cut/Copy/Paste/Select all, About, Services/Hide on macOS);
  - how `Menu.setAsAppMenu()` behaves on Linux (a GTK menu bar inside the window);
  - which `core:menu:*` permissions `capabilities/default.json` needs.
  If something needs Rust, keep the Rust change to that part only and say so in this file.
- `ui/menu.ts`: build the menu from a small state object (has a song, recent paths, export lock, mode, sidebar shown) and rebuild
  it when that state changes.
  - **macOS**: an app menu "madsister" (About madsister, Settings… ⌘,, Services, Hide, Quit ⌘Q), then File, Edit, View, Window.
  - **Linux/Windows**: File (… Quit), Edit, View, Settings (Preferences…), Help (About madsister).
  - File: New tab ⌘N, Open… ⌘O, Open recent ▸ (shortest unique labels from `shortestUniquePaths`, then Clear recent),
    Close tab ⌘W, Save ⌘S, Save as… ⇧⌘S, Export ▸ ChordPro / MusicXML / MIDI (disabled when `isExportLocked`), Print… ⌘P.
    Until M7-5, New tab and Close tab both do today's New (back to the start screen).
  - Edit: Undo ⌘Z, Redo ⇧⌘Z, then predefined Cut, Copy, Paste, Select all. Undo/Redo call the song history *only when focus
    isn't in a text input*. Otherwise they let the input undo its text, as `keymap.ts` does today. Check that the grid's own
    Mod+Z handler doesn't then run twice.
  - View: Edit mode ⌘E (checkable), Show sidebar (checkable).
- Remove the window `keydown` handler in `App.tsx`: the accelerators replace it. The grid keymap keeps letting Mod+N/O/P/S bubble.
- About: the dialog stays in `About.tsx`, opened from the menu (no button any more).
- `ui/Ribbon.tsx`: left: New, Open, Save (`.primary`), Print. Right: the View/Edit switch, Preferences. Lucide icons with visible
  text labels.
- Remove the `<nav>` from `App.tsx`, together with the Recent `<details>`, the Export group and the About button.

## Tests
vitest, with `@tauri-apps/api/menu` mocked:
- `menu.ts`: given macOS, then the first submenu is the app menu with About and Quit, and File has no Quit. Given Linux, then File
  ends with Quit and there is a Help menu.
- Given an export-locked song, then the three Export items are disabled. Given no song, then Save, Save as, Export, Print and
  Close tab are disabled.
- Given two recent paths, then Open recent has two items plus Clear recent, and clicking one opens it.
- Ribbon: clicking Save calls the save action. The mode switch toggles the mode.

## Verify
`cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`.

## Check by hand
- [ ] macOS: madsister ▸ About / Settings… / Quit. Each shortcut in the menu works (⌘N/O/S/⇧S/P/E/W).
- [ ] macOS: ⌘C / ⌘V / ⌘Z work in the Title field, and ⌘Z in the grid undoes one chord edit (not two).
- [ ] Open recent lists the files, opens them, and Clear recent empties it.
- [ ] Export is greyed out for a key-less Nashville song.
- [ ] Linux build (CI artefact or VM): the menu bar is inside the window, with Quit in File and About in Help.

## Commit message
`feat(app): native menu and ribbon`
