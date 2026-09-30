# M1 · Step 4 — Chord display styles

## Goal
`displayChord(harte, style: "fr" | "intl"): string` following the §4.1 display column (FR default) and F-DS-1 (international).

## Spec refs
F-DS-1, §4.1 display column. F-DS-2 (Latin names) is a MAY: skip it.

## Depends on
M1-3 (`parseHarte`).

## Tasks
- `app/src/model/display.ts`: two quality → suffix tables.
  FR: `maj ""`, `min m`, `aug +`, `dim °`, `sus2 sus2`, `sus4 sus4`, `add2 add2`, `add4 add4`, `7 7`, `maj7 7M`, `min7 m7`,
  `minmaj7 m7M`, `maj6 6`, `min6 m6`, `dim7 °7`, `hdim7 m7b5`, `9 9`, `maj9 7M9`, `min9 m9`, `11 11`, `13 13`.
  Intl: same except `maj7 maj7`, `minmaj7 m(maj7)`, `dim dim`, `dim7 dim7`, `hdim7 ø7`, `maj9 maj9`.
- Bass: degree → note name from the root (inverse of the parser's table); spelling follows the root's accidental (flat roots → flat
  bass, otherwise sharp; `b7` → flat). `N` → `N.C.`, `%` → `%`.

## Tests
- Table-driven over every quality × both styles, e.g. `C:maj7` → `C7M` / `Cmaj7`, `B:hdim7` → `Bm7b5` / `Bø7`.
- `C:maj/3` → `C/E`; `A:min/b7` → `Am/G`; `Eb:maj/5` → `Eb/Bb`.
- Property: `parseChord(displayChord(h, "fr")).harte === h` and the same for `"intl"`, for every row (the display is re-typable).

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): display chords in French or international style`

## Out of scope
Latin note names.
