# M4 · Step 1 — ChordPro grid export

> Coarse step. Refine before starting.

## Goal
F-EX-2: export `.cho` using ChordPro 6 `{start_of_grid}` / `{end_of_grid}`.

## Spec refs
F-EX-2 (VERIFY grid support in target apps).

## Depends on
M1-11.

## Tasks
- Read the ChordPro 6 grid spec (chordpro.org, Context7). Header directives `{title}`, `{artist}`, `{key}`, `{tempo}`, `{time}`.
  One grid per section with a label (`{start_of_grid label="Verse" shape="..."}`), bars `|`, beats as `.` placeholders per the spec,
  repeats via the section `x2` comment (the spec's repeat bars are F-EX-6, later).
- Chord names in ChordPro's expected spelling (international style from M1-4, check `m7b5`/`ø` handling in the reference implementation).
- Pure `toChordPro(song): string` in `model/export/chordpro.ts` + a "Export ChordPro…" action.
- VERIFY: render the output with the ChordPro reference CLI **if it's installable as a package** (`cpanm` counts as a toolchain → ask
  instead); otherwise record that the user must check it.

## Tests
- Golden file: `app/src/model/export/__golden__/song1.cho` for a fixture song with 2 sections, a split bar, a repeat, a slash chord, `N`.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): export ChordPro grids`
