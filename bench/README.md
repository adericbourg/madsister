# Bench

Reference set for the M0 model comparison (spec §7). Scripts and small result files are committed; `bench/data/` is not.

## Proxy set: GuitarSet (provisional)

Until the user's own songs are annotated, the bench runs on a proxy: the 72 Rock + Singer-Songwriter
accompaniment ("comp") takes of [GuitarSet](https://guitarset.weebly.com/) (36 each). Results on it are **provisional**.

```sh
uv run --project engine python bench/fetch_guitarset.py
```

This downloads `annotation.zip` (37 MB) and `audio_mono-mic.zip` (626 MB) from Zenodo record
[3371780](https://zenodo.org/records/3371780) (v1.1.0) into `bench/data/guitarset/` (skipped when present), then writes:

| Path under `bench/data/guitarset/` | Content |
|---|---|
| `audio/<id>_mic.wav` | Mono mic recording of the take |
| `refs/<id>.lab` | Reference chords, mir_eval format (`start end label`, Harte) |
| `refs/<id>.beats.json` | `{"beats": [...], "downbeats": [...], "meterBeats": 4}` (downbeat = `beat_position` 1) |
| `refs/<id>.madsister.json` | Reference `Song`: the reference chords and beats through the engine quantization (for "edits needed") |

`<id>` is the GuitarSet take name, e.g. `03_Rock1-90-C#_comp`.

Chords are the **performed** annotation (not the instructed one), normalized to the §4.1 qualities by
`madsister_engine.chords.labels.to_harte`: extensions dropped (`maj(#11)` → `maj`), `/1` bass removed, `maj(2)` → `add2`,
`maj(11)` → `add4`, interval sets without a quality (power chords `(1,5)`) → `maj`. A bass note that came from an
extension is kept (`F:maj(#11)/b5` → `F:maj/b5`).

**License:** GuitarSet is CC BY 4.0. Attribution: Q. Xi, R. Bittner, J. Pauwels, X. Ye, J. P. Bello,
"GuitarSet: A Dataset for Guitar Transcription", ISMIR 2018. The data is only used here, never shipped.

**Limits:** solo acoustic guitar (no band mix, no vocals), jazzy voicings and extension-derived basses, very few
add chords (14 add2/add4 segments), no sections, 4/4 only, short excerpts (22–46 s).

## User songs (later)

The user's own songs go in `bench/data/user/`: `<id>.<ext>` (audio) + `<id>.madsister.json` (hand-annotated reference
`Song`). The same runner evaluates them.
