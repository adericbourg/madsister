# M1 · Step 9 — Grid view

## Goal
Render a Song as a musician's chart: titled section blocks, bars in rows, chords split by beats (F-ED-1, F-ED-2).

## Spec refs
F-ED-1, F-ED-2, F-ED-8 (styling only), F-DS-1, NF-5, §1.2 (must look like a chart, not a beat grid).

## Depends on
M1-2, M1-4.

## Tasks
- Use the `frontend-design` skill for the visual direction; keep it sober and printable (black on white, a legible chord font).
- `ui/Grid.tsx`, `ui/SectionBlock.tsx`, `ui/BarCell.tsx`: props-driven, no editing logic yet. Props: `song`, `barsPerRow: 2 | 4 | 8`,
  `style`, `cursor: SlotRef | null`, `lowConfidenceThreshold = 0.5`.
- CSS grid: rows of `barsPerRow` bars; inside a bar, slots with `flex-grow: beats` (a 3+1 split looks like 3/4 + 1/4).
  Bar lines as borders. Section header: label + `x2` when `repeat > 1`. Empty section → a visible placeholder.
- Low-confidence slot (confidence < threshold): a distinct visual flag that doesn't rely on colour alone (e.g. dotted underline +
  colour), and an accessible name "low confidence".
- Header: title, artist, key, tempo (when present).
- Semantics: ARIA `grid` pattern (`role="grid"`, rows, `gridcell` per slot, one roving `tabIndex=0` at the cursor), each cell's
  accessible name like "Verse, bar 3, beat 1: A minor 7" (spell the chord for screen readers from `parseHarte`; a simple mapping is enough).
- Visible focus indicator ≥ 2 px, contrast ≥ 3:1 against its background (WCAG 2.2 2.4.7 / 2.4.11 / 1.4.11).
- `App.tsx` shows `emptySong()` in the grid for now.

## Tests
- Render a fixture: section labels present, `x2` shown, the right number of cells, a 3+1 bar has flex values 3 and 1,
  the low-confidence slot has the flag, the cursor cell has `tabIndex=0` and all the others −1.

## Verify
`cd app && pnpm test && pnpm build`; `pnpm tauri dev` for a visual check if possible (take a screenshot with `screencapture` if useful).

## Commit message
`feat(app): render the song as a sectioned bar grid`

## Out of scope
Editing (M1-10), print styles (M1-13).
