# M3 · Step 3 — Downbeat phase nudge

## Goal
F-PB-3: shift the bar phase by ±1 beat in one undoable action, for the whole song **or from the cursor bar onward** (after an
undetected 2-beat break bar the phase is wrong only from that bar on, see `BACKLOG.md`).

## Spec refs
F-PB-3, §2.1 known limits, §8 M3 "done when".

## Depends on
M3-1.

## Decisions
- Pure `shiftPhase(song, delta: 1 | -1, from?: BarRef)` in `commands.ts`. `+1` = the downbeats come one beat later. A beat is a
  slot beat (an eighth in 6/8). The range runs from `from` (default: the first bar) to the end of the song; earlier bars are
  the same objects.
- The range is cut into a beat sequence and re-cut into bars of the song meter. **Edges:** a leading partial bar becomes a
  pickup and a trailing one a short last bar, both with a `meter` override (nothing is padded or dropped). The pickup's
  length is the current phase, so −1 then +1 (and +1 then −1) gives the original song back, and **pressing +1 (or −1)
  twice from the bar after a missed 2-beat break turns it into the 2-beat break bar** (4/4). No separate "insert a 2-beat
  bar" command: this already does it.
- **Meter overrides:** refused (`the phase can't be shifted across a bar with its own meter`) when a bar inside the range
  other than the first (pickup) and the last is not in the song meter, or when the first/last is longer. `ponytail:` comment.
  Shift from the bar after the override instead.
- **Slots:** consecutive beats of the same chord make one slot; merged slots keep the lowest confidence.
- **Times:** beat times are interpolated from each bar's `startSec` to the next bar's; when the next bar has no time (or for
  the last bar), the last known period carries on. A bar whose first beat has no time (inside a bar without `startSec`)
  gets no `startSec`.
- **Sections:** each new bar goes to the section of the beat it started on before the shift, so each section's first bar
  is its old first bar line, moved. Repeats are not taken into account (the beat sequence is the written one).
- **UI:** "Bar lines" toolbar group: "Whole song −1/+1 beat", "From this bar −1/+1 beat". Keys `<` / `>`: −1 / +1 from the
  cursor bar (on the first bar = whole song). `<` is a plain key on AZERTY and Shift+, on QWERTY; not chord syntax. A refused
  shift says why in the status notice. One history entry per action.

## Tests
- Chords changing on beat 2 of every bar → +1 puts each chord on a bar; −1/+1 round-trips.
- From a bar: earlier bars untouched, the next section starts on its moved bar line, an untimed bar's beats stay untimed;
  +1 twice makes a 2-beat bar.
- An override in the range is refused; after it, accepted.
- "Done when": the first 12 bars of GuitarSet `00_Rock1-130-A_comp` (its reference, inlined), shifted by +1 to simulate the
  error, are restored by −1 (chords and times). Also checked once by hand on all GuitarSet references (±1 round trip).
- `keyToCommand`: `<` / `>`; `Editor`: toolbar buttons, one undo step, refusal message.

## Check by hand
- `pnpm tauri dev`, a transcribed song with a wrong phase: "Whole song +1 beat", then Mod+Z. On a bar after a break, `>` twice.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): fix the downbeat phase in one action`
