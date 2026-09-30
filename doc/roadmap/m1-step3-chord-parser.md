# M1 · Step 3 — Chord parser / normalizer

## Goal
`parseChord(input: string): {ok: true, harte: string} | {ok: false, error: string}` that accepts the usual spellings and returns
canonical Harte. Invalid input is rejected with a message (F-ED-3), never guessed.

## Spec refs
F-ED-3, §4 rules, §4.1 table, §1.3 (jazz extensions: enterable manually).

## Depends on
M1-1.

## Tasks
- `app/src/model/chord.ts`. Grammar (case-sensitive for the root, trimmed input):
  `root [accidental] [quality] [/bass]` | `N` / `N.C.` / `NC` | `%` (repeat previous).
  - Root `A`–`G`, accidentals `#`, `b`, `♯`, `♭`.
  - Quality aliases → Harte quality:
    `""`/`maj`/`M` → `maj`; `m`/`min`/`-` → `min`; `+`/`aug` → `aug`; `°`/`o`/`dim` → `dim`;
    `sus`/`sus4` → `sus4`; `sus2` → `sus2`; `add2`/`add9` → `add2`; `add4`/`add11` → `add4`;
    `7` → `7`; `7M`/`maj7`/`M7`/`Δ`/`Δ7`/`ma7` → `maj7`; `m7`/`min7`/`-7` → `min7`;
    `m7M`/`mM7`/`mmaj7`/`minmaj7`/`-Δ` → `minmaj7`; `6` → `maj6`; `m6`/`min6`/`-6` → `min6`;
    `°7`/`o7`/`dim7` → `dim7`; `m7b5`/`ø`/`ø7`/`-7b5`/`hdim7` → `hdim7`;
    extensions (manual only): `9` → `9`, `maj9`/`7M9` → `maj9`, `m9` → `min9`, `11` → `11`, `13` → `13`
    (Harte shorthands `9, maj9, min9, 11, 13` exist and are mir_eval-compatible).
  - Bass: `/` + note → Harte interval relative to the root: `C/E` → `C:maj/3`, `C/G` → `C:maj/5`, `C/Bb` → `C:maj/b7`,
    `Am/G` → `A:min/b7`. Compute the interval from the semitone distance with a fixed table
    (`1 b2 2 b3 3 4 b5 5 b6 6 b7 7`); prefer `b3`/`3` according to the quality's third when it's ambiguous — keep it simple:
    the table maps semitones → one fixed degree name.
  - Output format: `Root:quality[/bass]`, with `Root` spelled as typed (`Db` stays `Db`; `♭` → `b`, `♯` → `#`). `N` and `%` stay as is.
- Also export `parseHarte(harte)` → `{root, quality, bass?}` (used by display and transpose), and `QUALITIES` (the list).

## Tests (table-driven, `it.each`)
- Every F-ED-3 example: `Am7`, `A-7`, `Amin7` → `A:min7`; `C7M`, `Cmaj7`, `CΔ` → `C:maj7`; `C/E` → `C:maj/3`; `Csus` → `C:sus4`; `C°` → `C:dim`.
- One row per §4.1 quality (Q1–Q11), including `add2`/`add4`, `N.C.` → `N`, and accidentals `F#m`, `Bbmaj7`, `E♭`.
- Rejections: `H`, `Cx7`, `C/`, `C/H`, empty string, `Cmaj7/E/G` — each with a non-empty error.
- `parseHarte` round-trip on every accepted output.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): add chord parser normalizing to Harte`

## Out of scope
Latin note names input (F-DS-2 is display only, and a MAY).
