# M0 · Step 10 — add2/add4 chroma heuristic

## Goal
`madsister_engine/add_heuristic.py`: relabel long `maj`/`min` slots as `add2`/`add4` when the chroma supports it (D6).
Disabled by default (τ = None) until the user's bench validates it.

## Spec refs
§3.3 step 5b, D6, §4.1 Q6, F-ED-8, §7.

## Depends on
M0-3 (bars), M0-9 (harmonic stem; the pure function doesn't need it, only the wiring does).

## Tasks
- Pure function `refine_add(bars, chroma, chroma_times, beats, tau, flag_threshold=0.5)`:
  for each slot whose quality is `maj` or `min` (no bass/inversion; skip anything else, especially `sus*`) and `beats >= 2`:
  mean chroma over the slot's time span (from the beat times); energy ratio of the 2nd degree (root + 2 semitones) and the
  4th degree (root + 5) to the mean of the triad tones. If the ratio ≥ τ → relabel `Root:add2` / `Root:add4` (2nd wins if both);
  confidence = `min(old, flag_threshold - 0.01)` (so the user reviews it).
- `chroma(wav)`: `librosa.feature.chroma_cqt` on the harmonic stem, hop 2048 → (12, T) + times. Separate from the pure function.
- τ is a parameter; the bench (M0-13) sweeps it.

## Tests (synthetic chroma arrays, no audio)
- C major slot with a strong D bin → `C:add2`, confidence capped below 0.5.
- Same with strong F → `C:add4`; with neither → unchanged.
- `A:sus4` and a 1-beat slot → never touched; `C:maj/3` → untouched (inversions excluded, `ponytail:` comment).
- `tau=None` → the function is a no-op.

## Verify
`cd engine && uv run pytest -m "not slow" -k add`

## Commit message
`feat(engine): add chroma heuristic for add2/add4 chords`

## Out of scope
Deciding τ or go/no-go: that needs the user's bench with real add chords (§7). GuitarSet has none.
