# M1 · Step 10 — Keyboard editing

## Goal
Keyboard-first editing of the whole chart (F-ED-3) with every command from M1-6/7/8 reachable from the keyboard.

## Spec refs
F-ED-3, F-ED-4, F-ED-5, F-ED-6 (keyboard parts), F-ED-7, NF-5. §8 M1 "done when": type a full chart by keyboard.

## Depends on
M1-3, M1-6, M1-7, M1-8, M1-9.

## Tasks
- Cursor = `SlotRef` in App state. Arrow keys: ←/→ previous/next slot (crossing bars and sections), ↑/↓ same column in the previous/next
  row (clamped), Home/End start/end of section.
- Typing a printable character opens an inline input on the cell prefilled with that character; Enter → `parseChord`: ok →
  `setChord` via history + move to the next slot (fast entry: `C Enter G Enter Am Enter …`); error → the input stays open, shows the
  message (`aria-invalid`, `aria-describedby`), nothing changes. Escape cancels. Tab from the input = Enter + next.
- Shortcuts (document them in a help overlay on `?`; use Mod = Cmd on macOS / Ctrl elsewhere):
  `/` split slot · `Backspace` on a slot = merge with the previous · `Alt+←/→` resize slot · `Mod+Enter` insert bar after ·
  `Mod+Shift+Enter` insert bar before · `Mod+Backspace` delete bar · `Mod+D` duplicate bar · `Shift+←/→` extend a bar selection ·
  `Mod+C` / `Mod+V` copy/paste bars · `Mod+Z` / `Mod+Shift+Z` undo/redo · `Mod+K` split section at the cursor bar
  (`Mod+N/O/S/Shift+S/P` are reserved for M1-11/M1-13).
  Avoid shortcuts that conflict with the webview/OS (check `Mod+D`/`Mod+K` in `pnpm tauri dev`; change them if they don't reach the page).
- Keep the key → command mapping in one pure function `keyToCommand(event, state)` so it's testable without the DOM.

## Tests
- Unit: `keyToCommand` for each shortcut.
- Component test (user-event), the M1 "done when" proxy: starting from an empty song, type a 2-section chart (Verse: `C G Am F`, with
  one bar split 2+2; Chorus ×2: `F C G G`), including one invalid input that is rejected then corrected, one undo/redo. Assert the
  resulting Song equals the expected fixture.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): keyboard-first grid editing`

## Out of scope
Mouse editing beyond click-to-place-cursor (add the click handler, it's one line).
