# M2 · Step 4 — Low-confidence flags

## Goal
F-ED-8 end to end: engine confidences flagged in the grid, cleared on edit, and quick navigation between flags.

## Spec refs
F-ED-8, §3.3 step 3 (confidence), D6 (heuristic caps confidence).

## Depends on
M2-2 (M1-9 already styles flags; M1-6 already clears them on `setChord`).

## Tasks
- Settings: `lowConfidenceThreshold` in `settings.json` (default 0.5; missing or outside [0, 1] → 0.5). Toolbar → Display:
  labelled number input "Review chords below confidence" (0–1, step 0.05); out-of-range input is ignored.
- `F8` / `Shift+F8` (pure, in `keymap.ts`): move the cursor to the next / previous slot with `confidence < threshold`, in
  reading order, wrapping around the song. Nothing flagged → refused ("Not possible here"). Listed in the help dialog.
- Status line under the grid, a polite live region: "N chords to review" / "1 chord to review" / "No chords to review".
  Hidden in print, like the flags.
- Flags cleared on edit: typing a chord (`setChord`, already), and pasting bars (`pasteBars` drops `confidence`: a pasted chord
  is the user's decision). Split / resize / duplicate / transpose keep it: they don't decide the chord.

## Tests
- `keymap.test.ts`: F8 / Shift+F8 forward, backward, wrap-around, threshold, refusal.
- `commands.test.ts`: pasted chords lose `confidence`.
- `Editor.test.tsx`: F8 focuses the flagged cell, correcting it updates the counter, Shift+F8, refusal when none are left.
- `App.test.tsx`: the threshold setting is persisted, invalid values ignored.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): navigate and review low-confidence chords`
