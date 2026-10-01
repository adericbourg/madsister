# M2 · Step 5 — Per-bar meter override

## Goal
F-ED-11: set a single bar to another meter (e.g. one 2/4 bar in a 4/4 song).

## Spec refs
F-ED-11, §4 `Bar.meter`, §11.3.

## Depends on
M1-6, M1-10.

## Decisions
- Slot beats count in the bar's meter unit (as in M2-3's 6/8 songs). The UI keeps the song's unit; other units are only
  reachable by editing the file.
- **Mod+B** toggles a short break bar: 2/4 in a x/4 song, 3/8 (half a bar) in 6/8; on an overridden bar it goes back to the
  song meter. No +1/−1 beat shortcut: the "Beats in this bar" control covers arbitrary meters.
- The meter label is printed at the start of the bar (like a time signature), `aria-hidden`; the slots' accessible name says
  `bar 4 in 2/4`.

## Tasks
- Command `setBarMeter(song, barRef, meter | null)`: resizes the bar's slots to the new beat count (truncate from the end, or extend the
  last slot); `null` or a meter equal to the song's removes the override.
- Keymap: Mod+B (above), listed in the help dialog. Refused on an empty section.
- Editor: a "Bar" toolbar fieldset with a labelled number input "Beats in this bar" (+ the unit), disabled on an empty section.
- BarCell shows `2/4` when overridden, on screen and in print.

## Tests
- Command tests (shrink, grow, reset to the song meter or null, 6/8 song in eighths).
- Keymap: Mod+B toggles, 3/8 in 6/8, refused in an empty section. Grid: label + accessible name. Editor: Mod+B, then the control.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): per-bar meter override`
