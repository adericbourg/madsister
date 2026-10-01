# M4 · Step 3 — MIDI export (nice to have)

## Goal
F-EX-4: a Standard MIDI File with chord pads aligned to the audio tempo, for DAW import.

## Spec refs
F-EX-4, §4.

## Depends on
M1-11.

## Mapping
- Pure `toMidi(song): Uint8Array` in `app/src/model/export/midi.ts`, bytes written by hand (no dependency): SMF type 0,
  one track, 480 ticks per quarter, channel 1, velocity 80, explicit note-offs, no program change (the DAW picks the sound).
- Bars as in MusicXML (M4-2), through the shared `playedBars(song)` (`export/bars.ts`): repeated sections written out,
  a lone `%` → the previous bar's chords (same beat count); a `%` slot inside a split bar and `N` are silence.
- Slot length: `beats × 480 × 4 / unit` ticks (6/8 slots count eighths).
- Time signature (`FF 58`): at the start, then on every bar whose meter differs from the previous one (overrides and
  the return to the song meter).
- Tempo (`FF 51`, emitted only when it changes):
  - a bar with a `startSec` followed by a bar with a later `startSec` gets the tempo of its real duration
    (`duration / quarters in the bar`), so the MIDI follows the recording;
  - any other bar (untimed, last bar, the jump back to a written-out repeat) keeps the previous tempo;
  - before the first timed bar: `meta.tempoBpm`, or 120, counted in the pulse (quarters; dotted quarters in x/8, spec §4).
- Leading offset: when the first bar's `startSec` > 0, a rest bar (first bar's meter) is inserted, its tempo stretched so
  it lasts exactly `startSec`. Importing the MIDI and the audio both at the DAW's 0 then lines them up, and the DAW's
  bar 2 is the song's bar 1 (the bar grid stays intact, no pickup bar). Tempos are clamped to MIDI's 24-bit limit
  (16.7 s per quarter, i.e. a leading rest over ~67 s in 4/4).
- Voicing: root-position close voicing from the root in C4–B4 (quality → semitones table; add2 = 1 2 3 5, add4 = 1 3 4 5,
  9 adds the 9th, 11 drops the 3rd, 13 drops the 5th and 11th), plus a bass note in C3–B3: the root, or the slash degree.
- UI: "Export → MIDI…" in the File nav, `exportSong(song, "MIDI", "mid", toMidi)`; `exportSong` now writes a
  `Uint8Array` render with the fs plugin's `writeFile`. Permission: `fs:allow-write-file` (no static scope; the save
  dialog adds the picked path to the runtime scope, as for the text exports).

## Tests
- Byte-level golden for a 2-bar song (`C`, then `G/B` + `N`), hex-compared in `midi.test.ts`.
- A timed song (leading 1 s, a 2/4 override, a repeat): header chunk, track length field = file length − 22,
  end-of-track, tempo events `[250000, 500000]`, time signatures 4/4 → 2/4 → 4/4 on each pass.
- `App.test.tsx`: the button opens a `.mid` save dialog named after the title and writes bytes starting with `MThd`.
- Sanity check (by hand, not in CI): the golden bytes parse with `mido` (engine venv) as type 0, 480 PPQ, 4.0 s.

## Verify
`cd app && pnpm test && pnpm build`

### Check by hand
- In the desktop app: Export → MIDI… writes a `.mid` file (to a folder outside `$HOME/**/*.madsister.json` too).
- Import the `.mid` and the source audio in a DAW (Logic, Reaper, Ableton) both at 0: with the DAW following the MIDI
  tempo map, chord changes land on the recording's bar starts, also late in the song; the leading rest bar covers the
  audio's intro offset; meter changes show in the DAW's time signature track.
- Open it in MuseScore 4: the chords read as stacked notes over a bass, `N` bars are rests, written-out repeats.

## Commit message
`feat(app): export chord pads as MIDI`
