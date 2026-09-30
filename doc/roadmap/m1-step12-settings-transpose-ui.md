# M1 · Step 12 — Toolbar: settings, transpose, sections, metadata

## Goal
Expose everything that isn't a grid keystroke: display style, bars per row, transposition, section operations, song metadata.

## Spec refs
F-ED-1 (2/4/8 per row), F-ED-6 (rename, reorder, delete, repeat), F-ED-9, F-DS-1, §4 `meta`, NF-5.

## Depends on
M1-5, M1-7, M1-10, M1-11.

## Tasks
- Toolbar with native controls (`<select>`, `<input type="number">`, buttons, all labelled):
  display style (FR default / international), bars per row (2/4/8), transpose (−/+ buttons + sharp/flat select → one history entry per click).
- Metadata form: title, artist, key (free text validated as a note + optional `m`), tempo (number), meter for new songs.
  Changing the song meter on an existing song is **not** offered (it would invalidate every bar; `ponytail:` comment).
- Section header controls: rename (inline input), move up/down, delete (confirm when the section has non-`N` bars), repeat count.
- Preferences (style, bars per row) persisted in `<appConfigDir>/settings.json` (reuse the fs helpers from M1-11). Not in the song file.

## Tests
- Component: switching the style re-renders `C:maj7` as `C7M` → `Cmaj7`; transpose +2 changes the chords and is undoable in one step;
  moving a section down changes the order; delete asks for confirmation when needed.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): toolbar for display, transposition, sections and metadata`

## Out of scope
Latin note names (F-DS-2).
