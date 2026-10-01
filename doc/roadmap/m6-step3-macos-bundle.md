# M6 · Step 3 — macOS package (nice to have)

> **Postponed (user, 2026-10-01).** Known macOS items: `NSMicrophoneUsageDescription` in `Info.plist` (M5-2), and the madmom and NATTEN source builds (ship CI-built wheels as M6-2 does for madmom on Linux; `release.yml` takes a `macos` job next to `linux`).

> Coarse step. Refine before starting.

## Goal
A `.dmg` for macOS arm64 with the same first-launch engine setup.

## Spec refs
§8 M6, D4, NF-2 (MPS later), NF-3.

## Depends on
M6-2.

## Tasks
- `tauri build --target aarch64-apple-darwin` in `release.yml` on `macos-latest`, following the snapshot/release rules from M6-2. `NSMicrophoneUsageDescription` in `Info.plist` (M5-2).
- Signing/notarization: none for personal use (document the Gatekeeper bypass); ask the user before adding any Apple credentials to CI.
- Try MPS for torch inference; keep CPU if the results differ from the bench.

## Verify
Install the dmg on the dev machine from scratch (fresh `MADSISTER_MODELS_DIR`) and transcribe a song.

## Commit message
`build: package the app for macOS`
