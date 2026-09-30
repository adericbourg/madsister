# M6 · Step 3 — macOS package (nice to have)

> Coarse step. Refine before starting.

## Goal
A `.dmg` for macOS arm64 with the same first-launch engine setup.

## Spec refs
§8 M6, D4, NF-2 (MPS later), NF-3.

## Depends on
M6-2.

## Tasks
- `tauri build --target aarch64-apple-darwin` in `release.yml` on `macos-latest`. `NSMicrophoneUsageDescription` in `Info.plist` (M5-2).
- Signing/notarization: none for personal use (document the Gatekeeper bypass); ask the user before adding any Apple credentials to CI.
- Try MPS for torch inference; keep CPU if the results differ from the bench.

## Verify
Install the dmg on the dev machine from scratch (fresh `MADSISTER_MODELS_DIR`) and transcribe a song.

## Commit message
`build: package the app for macOS`
