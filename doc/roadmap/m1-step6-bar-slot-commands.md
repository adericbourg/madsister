# M1 · Step 6 — Bar and slot editing commands

## Goal
Pure functions `(song, args) => Song` for chord and bar edits. The UI only calls these (§9: editor commands tested as pure functions).

## Spec refs
F-ED-3 (set chord), F-ED-4, F-ED-5 (insert/delete/duplicate), F-ED-8 (edit clears confidence), §4 rules (`startSec` kept).

## Depends on
M1-2.

## Tasks
`app/src/model/commands.ts`. Addressing: `BarRef = {section: number, bar: number}`, `SlotRef = BarRef & {slot: number}` (indexes).
- `setChord(song, slotRef, harte)` → replaces the chord, **removes `confidence`**.
- `splitSlot(song, slotRef)` → a slot of n ≥ 2 beats becomes `ceil(n/2)` + `floor(n/2)`, the new slot copies the chord (the user
  then types over it). Throws if n = 1 (the UI prevents it).
- `mergeSlotWithNext(song, slotRef)` → beats added, keeps the first chord.
- `resizeSlot(song, slotRef, delta: 1 | -1)` → takes/gives one beat from/to the next slot (or the previous one for the last slot);
  a slot reaching 0 beats is removed; the bar sum is invariant. Throws if impossible.
- `insertBar(song, barRef, position: "before" | "after")` → a bar with one `N` slot of the song meter, no `startSec`.
- `deleteBar(song, barRef)` → removes it; a section can end up with 0 bars (allowed; the view shows an empty-section placeholder).
- `duplicateBar(song, barRef)` → copy inserted after, without `startSec` (the copy isn't aligned to audio).
- Every command returns a new Song (no mutation; a structural-sharing helper `updateBar(song, ref, fn)` is enough).
  Never touch other bars' `startSec`.

## Tests
- Each command once on a small fixture song (Given/When/Then), asserting the beat sum invariant (`parseSong(result)` doesn't throw)
  and that the input object is unchanged (`Object.freeze` the fixture deeply).
- `setChord` clears `confidence`; `insertBar` keeps the neighbours' `startSec`.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): add pure bar and slot editing commands`

## Out of scope
Per-bar meter override UI (F-ED-11 is M2); section commands and clipboard (M1-7).
