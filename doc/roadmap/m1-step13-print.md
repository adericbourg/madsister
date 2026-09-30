# M1 · Step 13 — Print on A4 + accessibility pass

## Goal
F-EX-1: print / save as PDF from the app on one A4 page when possible. Then close M1 with a WCAG 2.2 AA pass (NF-5).

## Spec refs
F-EX-1, NF-5, §8 M1 "done when" (print it on A4), §10 (webview quirks).

## Depends on
M1-9, M1-12.

## Tasks
- VERIFY first (Context7 + Tauri issues): does `window.print()` work in the Tauri v2 webview on macOS (WKWebView) and Linux (WebKitGTK)?
  If not, use the Tauri API that exists for it (e.g. a webview print method) behind a single `printSong()` function. Record the finding in
  `m0-results.md`'s VERIFY section or, if that doesn't exist yet, in this step's commit message body.
- `@media print` stylesheet: `@page { size: A4; margin: 12mm }`, hide the toolbar/cursor/flags UI, header (title, artist, key, tempo),
  sections with `break-inside: avoid`, a font size that fits a typical song (~4 sections × 8 bars) on one page; `x2` visible.
  Low-confidence flags are **not** printed.
- Print button + `Mod+P`.
- Best-effort automated check: `pnpm build && pnpm vite preview`, then headless Chrome if it is installed
  (`"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless --print-to-pdf=<scratchpad>/chart.pdf <url>`),
  and read the PDF (page count = 1 for the fixture song). If Chrome isn't there, skip it and say so in the commit body.
- Accessibility pass (NF-5): keyboard-only walkthrough of every control (M1-10's component test covers the grid), visible focus everywhere,
  contrast of text/flags/focus ≥ AA (compute it for the palette), labels on all inputs, errors announced (`role="alert"`/`aria-live`).
  Fix what fails.

## Tests
- Component: the print button calls `printSong`.
- The PDF check above (manual/automated best effort).

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): A4 print layout and accessibility fixes`

## Out of scope
PDF libraries (the spec says print stylesheet only).
