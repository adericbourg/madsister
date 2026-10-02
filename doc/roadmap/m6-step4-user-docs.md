# M6 · Step 4 — User documentation

## Goal
The top of the root `README.md` explains what madsister is and how to use it, with one or two screenshots.

## Spec refs
§1 (product summary), §5 (features). User request 2026-10-01: short user doc in the README, features + 1–2 screenshots.

## Depends on
M6-2/M6-3 (installation instructions), M2-7 (developer section stays at the end).

## Tasks
- README top part: one-paragraph pitch (offline chord charts from audio, the editor is the product), install (the apt repository, README
  "Install" section, plus the latest release;
  Linux deb/AppImage, macOS dmg + Gatekeeper bypass; first-launch engine setup), quick start in 5 steps (import or new chart →
  correct chords → review low-confidence chords with F8 → print → save), feature list (from §5, only what exists), keyboard: press `?`
  in the app for all shortcuts, known limits (sections detection is slow and opt-in, Linux/macOS only, non-commercial model weights).
- Screenshots (`doc/images/*.png`, < 300 KB each): the grid editor on a transcribed song (low-confidence flags visible), and optionally
  the import progress. An unattended agent can't capture the real window reliably: build the UI (`pnpm build`), serve `dist/` with
  Tauri modules aliased to stubs (the M1-13 print check used this approach — keep the harness outside the repo or in a dev-only
  script), load a fixture song, and capture with headless Chrome (`--screenshot --window-size=…`). Light theme, readable at README width.
- Alt text for every image.

## Verify
Render the README (e.g. GitHub preview after push) and check the images load; every claimed feature exists in the app.

## Commit message
`docs: add user guide with screenshots to the README`
