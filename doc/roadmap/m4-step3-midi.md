# M4 · Step 3 — MIDI export (nice to have)

> Coarse step. Refine before starting.

## Goal
F-EX-4: a Standard MIDI File with chord pads aligned to the audio tempo, for DAW import.

## Spec refs
F-EX-4.

## Depends on
M1-11.

## Tasks
- Pure `toMidi(song): Uint8Array` written by hand (SMF type 0: header + one track, a few dozen lines) — no dependency.
- Tempo: if bars have `startSec`, emit tempo events per bar (from the bar durations) so the MIDI follows the recording; otherwise
  `meta.tempoBpm` or 120. Chord voicing: root position close voicing around middle C + bass note an octave below (use the Harte degrees).
- `N` = silence.

## Tests
- Byte-level golden for a 2-bar song; a parser-free sanity check (header, track length field consistent).

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): export chord pads as MIDI`
