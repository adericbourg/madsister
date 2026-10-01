# M4 · Step 2 — MusicXML export

## Goal
F-EX-3: MusicXML with `<harmony>` elements and slash/empty measures, opening in MuseScore.

## Spec refs
F-EX-3, §4.

## Depends on
M1-11.

## Mapping (MusicXML 4.0, w3.org/2021/06/musicxml40)
- Pure `toMusicXml(song)` in `app/src/model/export/musicxml.ts`, string templates (no XML library): `score-partwise`
  version 4.0, `<work-title>` = title, `<creator type="composer">` = artist when set, one part "Chords" (treble clef).
  Title and labels are XML-escaped.
- `<divisions>2</divisions>` (per quarter): a 4-unit beat lasts 2, an 8-unit beat (6/8) lasts 1.
- One `<measure>` per bar, filled with one slash note per beat (B4, `<stem>none</stem>`, `<notehead>slash</notehead>`).
  Each slot's `<harmony>` sits right before its first beat's note, so no `<offset>` is needed.
- Section label → `<rehearsal>` mark on the section's first bar.
- Section `repeat` → the section is written out N times, each pass under its own rehearsal mark (not repeat barlines:
  they are F-EX-6, and a written-out score also plays right in MuseScore). ChordPro uses `x2` instead (M4-1).
- `<harmony>`: `<root>` (`root-step` + `root-alter` ±1), `<kind>` per quality — maj major, min minor, aug augmented,
  dim diminished, sus2 suspended-second, sus4 suspended-fourth, 7 dominant, maj7 major-seventh, min7 minor-seventh,
  minmaj7 major-minor, maj6 major-sixth, min6 minor-sixth, dim7 diminished-seventh, hdim7 half-diminished,
  9 dominant-ninth, maj9 major-ninth, min9 minor-ninth, 11 dominant-11th, 13 dominant-13th.
  add2/add4 → `major` + `<degree>` add 2/4 (degree-alter 0), to read like the app (`Bbadd2`) rather than add9/add11.
  Slash chords → `<bass>`, spelled like the display (`G/B`, M1-4).
- `N` → `<kind text="N.C.">none</kind>` with a dummy `<root-step text="">C</root-step>` (the schema requires a root;
  the empty `text` hides it, as the MusicXML `kind` docs recommend).
- `%` alone in a bar → the previous bar's chords, repeated (if both bars have the same beat count). A `%` slot inside a
  split bar gets no harmony (`ponytail:` comment in the code).
- Song meter → `<time>` in the first measure. A per-bar meter override → `<attributes><time>` on that measure, and
  again on the next bar whose meter differs (back to the song meter).
- No key signature (slash notation has no pitches to spell) and no tempo marking.
- UI: "Export → MusicXML…" in the File nav (`App.tsx`), `exportSong(song, "MusicXML", "musicxml", toMusicXml)`.
  Permissions unchanged (see M4-1).

## Tests
- Golden file `app/src/model/export/__golden__/song1.musicxml`: 2 sections, a split bar, a repeat, a slash chord,
  `%`, `N`, add2, a 2-beat override bar, a title to escape.
- `App.test.tsx`: the button opens a `.musicxml` save dialog named after the title and writes the score.
- XSD: the golden file validates against the MusicXML 4.0 XSD (`xmllint --noout --nonet --schema musicxml.xsd`, schema
  from `github.com/w3c/musicxml` tag v4.0 with its `xml.xsd`/`xlink.xsd` imports pointed at local copies). Done by hand,
  not in CI (no network in tests).

## Verify
`cd app && pnpm test && pnpm build`

### Check by hand
- Open the exported `.musicxml` in MuseScore 4: no import warning; chord symbols on the right beats (`Am` beat 1,
  `G/B` beat 3), `N.C.`, `Bbadd2`, `Em(maj7)`, `Bø7` (or MuseScore's spelling); rehearsal marks; the 2/4 bar and the
  return to 4/4; slash noteheads.
- Play it in MuseScore with "Play chord symbols" on: chords sound on the beats they're written on.
- In the desktop app: Export → MusicXML… to a folder outside `$HOME/**/*.madsister.json` writes the file.

## Commit message
`feat(app): export MusicXML chord charts`
