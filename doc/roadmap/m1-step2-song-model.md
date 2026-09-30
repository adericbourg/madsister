# M1 · Step 2 — Song model (app side)

## Goal
TS types for the file format, parsing/validation of loaded files, serialization that never drops unknown fields, empty song.

## Spec refs
§4, NF-7, F-IN-6.

## Depends on
M1-1.

## Tasks
- `app/src/model/song.ts`: the §4 types exactly (readonly properties, readonly arrays: model values are immutable).
- `parseSong(json: unknown): Song` — validates structure, `version === 1` (other version → error "unsupported version N"),
  beat sums (§4 rule, with `Bar.meter` override), positive integer beats. Throws a descriptive `Error` (path like
  `sections[1].bars[3]`). Unknown fields at any level are **kept** as-is on the object (NF-7): validate the known fields and spread
  the rest (`{...raw, ...validatedKnownFields}`).
- `serializeSong(song): string` → `JSON.stringify(song, null, 2)`; unknown fields survive because they are still on the objects.
- `emptySong(meter = {beats: 4, unit: 4})`: title "Untitled", one section "Verse" with 4 bars of `N` (one slot per bar).
  Section id: `crypto.randomUUID().slice(0, 8)`.
- `barBeats(song, bar)` helper = `(bar.meter ?? song.meta.meter).beats` (used everywhere after this).

## Tests
- `parseSong` then `serializeSong` round-trips a file that has unknown fields at root, section, bar and slot level.
- Rejects: wrong version, beat sum mismatch, non-integer beats — each with its path in the message.
- Accepts a 2/4 bar override in a 4/4 song.
- `emptySong` passes `parseSong`.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): add Song model with lossless load and save`

## Out of scope
File dialogs (M1-11).
