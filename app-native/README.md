# app-native

The native rewrite of `app/` in Rust with [Slint](https://slint.dev), built next to it until it reaches parity; then it
replaces `app/`. The chord sheet keeps its look; everything around it uses Slint's native widgets.

```sh
cargo run            # the app
cargo run -- print FILE...   # PDFs without a window; options in src/print.rs (USAGE)
cargo test
cargo clippy --all-targets -- -D warnings
```

## Platform notes

- **Linux runs under XWayland.** winit (under Slint) doesn't implement drag & drop on Wayland, and dropping an audio
  file is how F-IN-1 imports. Under XWayland the window may look blurry with fractional scaling. Without XWayland (no
  `DISPLAY`), the app runs on native Wayland, without drops. Switch back to native Wayland once winit supports drag &
  drop there (`main.rs`, the `with_x11` builder).
- File drops come from winit's `WindowEvent::DroppedFile`: Slint's own `DropArea` only takes drops from inside the app.
- Audio is decoded by ffmpeg (already the engine's prerequisite on every platform) at the output's rate, and played with
  cpal. The output stream runs while paused, writing silence: PipeWire's ALSA layer refuses to pause one.
- Native menus (muda on macOS and Windows) must not change structure while an item's activation is handled: such
  changes are deferred to the next event loop turn.
- On macOS, Slint builds the app menu itself (its own About panel, Services, Hide, Quit) and has no menu roles yet, so
  About and Preferences stay in the Help and Settings menus there too.
- The accent (F-UI-1) is the Fluent style's `Palette.accent-background`: Slint tints it with the OS accent hue, keeping
  its lightness (contrast 5.1:1 to 10:1 on the app's surfaces), and has no public API to set it. The app's own accents
  (active tab, links, focus rings) use it too, so there's one hue; the chord sheet keeps its own `Ink` colours.

## Packaging

[cargo-packager](https://github.com/crabnebula-dev/cargo-packager) builds the deb, the AppImage, the dmg and the NSIS
installer from `[package.metadata.packager]` in `Cargo.toml` (`cargo install cargo-packager --locked`):

```sh
# the uv sidecar first, named after the binary and the target triple (the release workflow downloads it the same way)
t=$(rustc -vV | sed -n 's/^host: //p'); mkdir -p binaries
curl -fsSL "https://github.com/astral-sh/uv/releases/download/0.12.21/uv-$t.tar.gz" | tar -xzO "uv-$t/uv" > "binaries/madsister-uv-$t"
chmod +x "binaries/madsister-uv-$t"
cargo packager --release -f deb,appimage   # into target/release/
```

- The app finds its engine by its binary's name (`Packaged::beside`): the sidecar `<binary>-uv` next to it, the
  `engine/` project in the resources (`/usr/lib/<binary>` in the deb and the AppImage, `Contents/Resources` on macOS,
  the install folder on Windows), the env in `<local data>/dev.dericbourg.<binary>/engine-env`, stamped with the
  crate's version. Dev builds and `MADSISTER_ENGINE` use the dev engine.
- A local package has no madmom wheel: its first launch builds madmom from git, which needs git and a C compiler. The
  release workflow builds the wheel and adds it (`release-config.py`), with the snapshot package's names.
- The deb depends on what the app loads at run time too (winit's X11 and Wayland libraries, OpenGL, D-Bus for the
  file dialogs): `ldd` only shows fontconfig and ALSA. The file dialogs go through xdg-desktop-portal, which desktops
  install.
- The AppImage bundles no GStreamer any more (audio goes through ffmpeg and ALSA): only FreeType's libpng, brotli and
  bzip2, which linuxdeploy copies. Its name keeps the deb's arch, `amd64`, as the Tauri one had (the workflow renames
  cargo-packager's `x86_64`).

## Parity checklist

Product brief §5, checked against `app/` on the same songs. Removed when `app/` is.

- [ ] Import: file dialog + drop (F-IN-1), URL (F-IN-2), recording (F-IN-3), progress + cancel (F-IN-4), meter (F-IN-5),
      empty grid (F-IN-6), mode (F-IN-7)
- [x] Grid editor: F-ED-1 to F-ED-11
- [x] Display styles: F-DS-1, F-DS-2
- [x] Playback: F-PB-1 to F-PB-4
- [x] Exports: PDF (F-EX-1), ChordPro (F-EX-2), MusicXML (F-EX-3), MIDI (F-EX-4)
- [x] App shell: F-UI-1 to F-UI-8
- [ ] `madsister print`
- [ ] Packaging: deb and AppImage built and checked locally (contents; the deb in an Ubuntu 26.04 container: every
      library resolves, the first launch runs the setup with the bundled uv and engine into the expected env); dmg and
      NSIS installer only from the release workflow, not run yet
