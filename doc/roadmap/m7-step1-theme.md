# M7 · Step 1 — Colour theme and dark mode

> Written on 2026-10-13 with the user.

## Goal
The chrome gets colour and a dark mode. The chart follows it on screen and stays light on paper.

## Spec refs
F-UI-1, F-UI-2, §5.7.

## Depends on
None.

## Tasks
- `app.css`: replace the grey tokens with semantic tokens on `:root`: `--accent`, `--accent-hover`, `--accent-text` (the text on
  the accent), `--surface` (the chrome), `--surface-2` (cards, inputs), `--border`, `--control-border`, `--text`, `--text-muted`,
  `--danger`, `--focus`.
  - Pick one accent hue. Note the contrast ratio of each text/background pair as a comment, the way `grid.css` does.
  - Restyle the buttons (neutral and primary `.primary`), selects, inputs, fieldsets, the mode switch and the focus ring with
    these tokens. Keep the current layout: the menus and tabs come in later steps.
- Dark mode: the same tokens redefined under `@media (prefers-color-scheme: dark)`, and `color-scheme: light dark`.
- `grid.css`: redefine its 9 tokens under the same media query.
  - The sheet (`--paper`) is one step lighter than `--surface` (for example #26262a on #1c1c1f).
  - `--ink` is off-white (about #e6e6e6). The boxed rehearsal marks invert as they do today.
  - Re-pick `--doubt`, `--playing`, `--selection`, `--editing` and `--focus` for the dark sheet. Note their ratios.
  - `@media print` sets the light values back, so dark mode never reaches paper or PDF (including `madsister print`).

## Tests
CSS only, so there is no vitest test. Each colour pair is checked with a contrast checker, and the ratio is written next to the
token.

## Verify
`cd app && pnpm test && pnpm build`.

## Check by hand
`cd app && pnpm tauri dev`:
- [ ] Light mode: the primary buttons and the mode switch use the accent, and focus rings are visible.
- [ ] macOS set to dark: the chrome and the chart turn dark, with the sheet lighter than the chrome. The selection, the playing
      bar and the low-confidence chords stay readable.
- [ ] Print preview while in dark mode → black on white.

## Commit message
`feat(app): colour theme and dark mode`
