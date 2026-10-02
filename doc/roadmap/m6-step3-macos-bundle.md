# M6 · Step 3 — macOS package (nice to have)

> Postponed 2026-10-01, taken up 2026-10-02. Refined below. The Homebrew cask is a second part of this step.

## Goal
A `.dmg` for macOS arm64 with the same first-launch engine setup.

## Spec refs
§8 M6, D4, NF-2 (MPS later), NF-3.

## Depends on
M6-2.

## Tasks
- `macos` job in `release.yml` on `macos-14` (arm64, so the host triple is `aarch64-apple-darwin`), the `linux` job's steps with
  `--bundles dmg`, following the snapshot/release rules from M6-2. Same package/identifier overrides, so `madsister` and
  `madsister-snapshot` install together. The madmom wheel is built on the runner (NATTEN stays an on-demand source build).
- `src-tauri/Info.plist` (merged by Tauri): `NSMicrophoneUsageDescription` (M5-2).
- Signing: ad-hoc (`signingIdentity: "-"`, which Apple silicon requires to run); no notarization, and no Apple credentials in CI.
  Gatekeeper is bypassed by the cask (`postflight` clears the quarantine flag) or, with the dmg, by one `xattr`.
- Homebrew: the `homebrew` job (after `publish`) writes `Casks/madsister.rb` (release) or `Casks/madsister-snapshot.rb`
  (snapshot) in `adericbourg/homebrew-tap` (arm64, `depends_on formula: "ffmpeg"`, `zap` of the data dir), pointing at the
  published dmg with its sha256. It pushes with the `HOMEBREW_TAP_TOKEN` secret (a fine-grained PAT, Contents read/write on
  the tap only: `GITHUB_TOKEN` can't write to another repository). Every push to `main` thus commits to the tap.
- `macos-clean-install`: mounts the dmg, runs `setup-engine.sh` against the .app's sidecar and resources, transcribes the
  synthetic C major clip. `publish` waits for it; the dmg is uploaded as `dist/*.dmg` (a single-file artifact has no directory).
- Not done: MPS for torch inference (the CPU results are the bench's); Intel.

## Verify
Install the dmg on the dev machine from scratch (fresh `MADSISTER_MODELS_DIR`) and transcribe a song.

## Commit message
`build: package the app for macOS`
