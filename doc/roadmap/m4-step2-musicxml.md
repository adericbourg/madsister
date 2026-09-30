# M4 · Step 2 — MusicXML export

> Coarse step. Refine before starting.

## Goal
F-EX-3: MusicXML with `<harmony>` elements and slash/empty measures, opening in MuseScore.

## Spec refs
F-EX-3.

## Depends on
M1-11.

## Tasks
- Pure `toMusicXml(song): string` (string templates, no XML library): `score-partwise`, one part, `<attributes>` with time signature,
  `<direction>` rehearsal marks for sections, one `<measure>` per bar (repeat sections are written out N times, or with repeat barlines —
  choose writing out for v1), `<harmony>` with `<root>`, `<kind>` (map each §4.1 quality to MusicXML `kind` values: `major`, `minor`,
  `augmented`, `diminished`, `dominant`, `major-seventh`, `minor-seventh`, `diminished-seventh`, `half-diminished`, `major-minor`,
  `major-sixth`, `minor-sixth`, `suspended-second`, `suspended-fourth`; add2/add4 = `major` + `<degree>` add 9/11), `<bass>` for slash
  chords, and a whole-bar rest or slash notes (`<notehead>slash</notehead>`) with `<offset>`/durations per slot.
- Per-bar meter overrides → `<time>` changes. XML-escape the title/labels.

## Tests
- Golden file; plus an XSD validation if the MusicXML 4.0 XSD can be fetched and validated with a package (`xmllint` is present on
  macOS — check with `which xmllint`).

## Verify
`cd app && pnpm test`; the user checks MuseScore (record it in the hand-off).

## Commit message
`feat(app): export MusicXML chord charts`
