# M1 · Step 5 — Transposition

## Goal
`transposeSong(song, semitones, spelling: "sharp" | "flat"): Song` (F-ED-9).

## Spec refs
F-ED-9, §4 (`meta.key`).

## Depends on
M1-2, M1-3.

## Tasks
- `app/src/model/transpose.ts`: `transposeChord(harte, n, spelling)`. Root → pitch class + n mod 12 → name from the sharp or flat table.
  Quality and bass **degree** stay unchanged (the Harte bass is an interval, so it transposes for free). `N`, `%` unchanged.
- `transposeSong`: every slot chord, and `meta.key` if present (key strings are `C`, `Am`, `F#m`, …: transpose the root the same way).
  Keeps `confidence` (transposing isn't a correction of the chord).

## Tests
- `C:maj7` +2 sharp → `D:maj7`; `B:min` +1 flat → `C:min`; `F#:7/3` −1 flat → `F:7/3`; `N` unchanged; 0 semitones with the other spelling
  respells (`C#` → `Db`).
- `transposeSong` +12 then with the original spelling = original song; `meta.key` `Am` +3 flat → `Cm`.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): transpose songs with sharp or flat spelling`

## Out of scope
Automatic spelling from the key signature.
