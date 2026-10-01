# M4 · Step 1 — ChordPro grid export

## Goal
F-EX-2: export `.cho` using ChordPro 6 `{start_of_grid}` / `{end_of_grid}`.

## Spec refs
F-EX-2 (VERIFY grid support in target apps), §4, §5.5.

## Depends on
M1-11.

## Mapping (ChordPro 6 grid spec, chordpro.org `directives-env_grid` and `chordpro-chords`)
- Header: `{title}`, `{artist}` and `{key}` when set, `{tempo}` (BPM rounded), `{time: beats/unit}`.
- One grid per section: `{start_of_grid shape="1+4x<song beats>+1" label="<section label>"}`, 4 bars per line,
  one cell per beat. A slot is its chord in its first cell, then `.` for its remaining beats (`| Am . G/B . |`).
  The section's last line ends on `||`.
- Chord spelling: the international display style (M1-4), except the two names missing from ChordPro's built-in
  extension list: `m(maj7)` → `mmaj7`, `ø7` → `m7b5`. Slash chords as `G/B`.
- `N` → `N.C.` (not in ChordPro's chord list; the reference implementation has "NC chord" handling). `%` stays `%`
  (the grid's "repeat previous measure").
- Section `repeat` → `x2` text in the right margin of the section's last line (repeat signs are F-EX-6, later).
- Per-bar meter overrides: ChordPro grids have a fixed beats-per-measure shape and no per-measure time signature.
  The bar gets as many cells as its own beats (`| Emmaj7 . |` for a 2/4 bar in 4/4), so its length is right but it
  renders narrower than the shape and the next bars shift left on that line.
- Files: `app/src/model/export/chordpro.ts` (pure `toChordPro(song)`), "Export → ChordPro…" button in the File nav
  (`App.tsx`) via `exportSong(song, name, extension, render)` in `ui/fileActions.ts` — the same group/helper is
  meant for M4-2 (MusicXML) and M4-3 (MIDI; binary, so it needs a `writeFile` variant).
- Permissions: none added. The dialog plugin adds the saved path to the fs runtime scope (`tauri-plugin-dialog`
  2.8.0 `save` → `allow_file`), which `tauri-plugin-fs` `resolve_path` accepts on top of the static scope.

## Tests
- Golden file: `app/src/model/export/__golden__/song1.cho` for a fixture with 2 sections, a split bar, a repeat,
  a slash chord, `N`, `%`, a 2-beat override bar.
- `App.test.tsx`: the button opens a `.cho` save dialog named after the title and writes the grid.

## Verify
`cd app && pnpm test && pnpm build`

Not done automatically: the ChordPro reference CLI is Perl (cpanm = toolchain install, not allowed).

### Check by hand
- Render an exported `.cho` with the ChordPro reference implementation (`chordpro song.cho` → PDF): grids show with
  their labels, `x2` in the right margin, no "Too few cells" warning; `N.C.`, `Bm7b5`, `Emmaj7` print without
  unknown-chord warnings.
- Open it in an OnSong-style reader and see whether grids are supported at all.
- In the desktop app: Export → ChordPro… to a folder outside `$HOME/**/*.madsister.json` (e.g. Desktop) writes the file.

## Commit message
`feat(app): export ChordPro grids`
