# app-native

The native rewrite of `app/` in Rust with [Slint](https://slint.dev), built next to it until it reaches parity; then it
replaces `app/`. The chord sheet keeps its look; everything around it uses Slint's native widgets.

```sh
cargo run            # the app
cargo test
cargo clippy --all-targets -- -D warnings
```

## Platform notes

- **Linux runs under XWayland.** winit (under Slint) doesn't implement drag & drop on Wayland, and dropping an audio
  file is how F-IN-1 imports. Under XWayland the window may look blurry with fractional scaling. Without XWayland (no
  `DISPLAY`), the app runs on native Wayland, without drops. Switch back to native Wayland once winit supports drag &
  drop there (`main.rs`, the `with_x11` builder).
- File drops come from winit's `WindowEvent::DroppedFile`: Slint's own `DropArea` only takes drops from inside the app.
- Native menus (muda on macOS and Windows) must not change structure while an item's activation is handled: such
  changes are deferred to the next event loop turn.

## Parity checklist

Product brief §5, checked against `app/` on the same songs. Removed when `app/` is.

- [ ] Import: file dialog + drop (F-IN-1), URL (F-IN-2), recording (F-IN-3), progress + cancel (F-IN-4), meter (F-IN-5),
      empty grid (F-IN-6), mode (F-IN-7)
- [ ] Grid editor: F-ED-1 to F-ED-11
- [ ] Display styles: F-DS-1, F-DS-2
- [ ] Playback: F-PB-1 to F-PB-4
- [ ] Exports: PDF (F-EX-1), ChordPro (F-EX-2), MusicXML (F-EX-3), MIDI (F-EX-4)
- [ ] App shell: F-UI-1 to F-UI-8
- [ ] `madsister print`
- [ ] Packaging: deb, AppImage, dmg, msi
