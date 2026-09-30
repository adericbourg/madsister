# M1 · Step 7 — Section commands and bar clipboard

## Goal
Pure commands for sections (F-ED-6) and copy/paste of bar ranges across sections (F-ED-5).

## Spec refs
F-ED-5, F-ED-6, §11.1 (repeat count).

## Depends on
M1-6.

## Tasks
In `commands.ts` (same file while it stays readable; split into `sectionCommands.ts` if it passes ~300 lines):
- `addSection(song, index, label)` (one empty `N` bar), `renameSection`, `moveSection(song, from, to)`, `deleteSection`
  (deleting the last section leaves one empty "Song" section: a song always has ≥ 1 section), `splitSection(song, barRef)`
  (bars from `barRef` onward go to a new section labelled `<label> (2)`, inserted right after), `setRepeat(song, section, n)`
  (`n <= 1` removes the field).
- Clipboard as data, not global state: `copyBars(song, from: BarRef, to: BarRef): Bar[]` (same section; a range across sections isn't
  needed: YAGNI) and `pasteBars(song, at: BarRef, bars, position: "before" | "after")`. Pasted bars lose `startSec`.
  Pasting a bar whose beat sum doesn't match the target song meter keeps it with a `meter` override (the model allows it).

## Tests
- Each section command once; `deleteSection` of the only section; `splitSection` at bar 0 (whole section moves) and at the last bar.
- Copy 2 bars from section 0, paste after bar 1 of section 1: order correct, `startSec` stripped, source unchanged.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): add section commands and bar copy/paste`

## Out of scope
OS clipboard integration (the internal clipboard is enough for v1).
