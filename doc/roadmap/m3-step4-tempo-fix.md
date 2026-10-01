# M3 · Step 4 — Double/half tempo fix

## Goal
F-PB-4: merge pairs of bars (tracker found double tempo) or split each bar in two (half tempo), in one undoable action, for
the whole song or from the cursor bar onward (same scope as M3-3). M0 saw madmom at double tempo on 9/72 GuitarSet takes.

## Spec refs
F-PB-4, §8 M3 "done when".

## Depends on
M3-3 (reuses its beat sequence: interpolated beat times, `toSlots`, splicing the bars back into their sections).

## Decisions
- Pure `halveTempo(song, from?)` / `doubleTempo(song, from?)` in `commands.ts`. The range runs from `from` (default: the first
  bar) to the end of the song; earlier bars are the same objects.
- **Halve** (double tempo found: 2 tracked bars = 1 real bar, 2 tracked beats = 1 real beat): bars are merged two by two
  **within each section** (a section boundary is kept), and every two beats of the pair make one beat, taking the chord,
  confidence and time of the first. Bars keep the meter's beat count, so a 4/4 chart stays 4/4. **Rounding:** a chord change
  on an odd tracked beat moves to the next beat (a 3+1 bar becomes 2 beats + the next bar's chord); a 1-beat chord there is
  dropped. A section's **odd last bar** (or a lone bar) becomes a half bar with a meter override (2/4). A pair with a meter
  override gives a bar of half their total beats (rounded up), overridden if not the song meter.
- **Double** (half tempo found): each bar becomes two bars of its own beat count (meter overrides kept), every beat becoming
  two; the second bar starts at the midpoint (the last timed period carries on, as in M3-3); untimed bars stay untimed.
- **Tempo:** `meta.tempoBpm` is halved / doubled only when the range is the whole song (from the first bar); from a later bar
  it is left alone (one song tempo). No refusal: every song can be halved or doubled.
- **Round trip:** double then halve is the identity (bars, times, confidences, overrides, tempo), except where adjacent slots
  carry the same chord (merged, as in M3-3). Halve then double is not (rounding).
- **UI:** "Tempo" toolbar group: "Whole song half/double tempo", "From this bar half/double tempo". Keys `-` / `+`: half /
  double from the cursor bar (on the first bar = whole song). One history entry per action.

## Tests
- Halve: rounding of an odd-beat change, odd section, sections kept, from a bar (earlier bars untouched, tempo kept).
- Double: midpoints, untimed bars, a 3/4 override bar, tempo; double then halve = identity (whole song and from a bar).
- "Done when": the first 12 bars of GuitarSet `00_Rock1-130-A_comp` (inlined), tracked at 260 BPM (each bar cut in two),
  restored by halving (chords, times, tempo). Also checked once by hand on all 72 GuitarSet references: double then halve
  gives each reference back.
- `keyToCommand`: `-` / `+`; `Editor`: toolbar buttons, one undo step.

## Check by hand
- `pnpm tauri dev`, a song transcribed at double tempo: "Whole song half tempo", then Mod+Z.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): fix double or half tempo in one action`
