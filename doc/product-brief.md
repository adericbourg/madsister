# madsister — Technical Specification

Status: draft v0.1 — 2026-09-30
Audience: Claude (code generation). Terse, normative. `MUST` / `SHOULD` / `MAY` per RFC 2119.

## 1. Product summary

Offline desktop app. Input: a song (file, URL, or mic recording). Output: an editable chord grid organized by
sections (intro, verse, chorus…), with 1–N chords per bar, printable and exportable to open formats.

**Core product insight:** state-of-the-art chord recognition is roughly 80–85% accurate on major/minor triads
and much lower on sevenths, sus, and rare qualities. Model output is a **draft**. The product is the **editor**
that turns the draft into a correct grid fast. Transcription quality matters, but editing UX is the differentiator.

### 1.1 User
Single user: a band leader who prepares chord charts for rehearsal and prints them.
Repertoire: pop, rock, chanson, folk/acoustic. Jazz extensions are out of scope.
Playback exists only to **verify** the transcription (hear it, see a cursor on the grid). It is not a practice tool.

### 1.2 Pain points this addresses
| Pain | Existing tool | madsister answer |
|---|---|---|
| Service may disappear, data locked | Chordify, Moises, Chord ai, Capo (commercial) | Local-only, open file format |
| Grid is read-only or clumsy to correct | ChordMiniApp, Sheet Sage | Keyboard-first grid editor (§5) |
| Grid doesn't look like a musician's chart | ChordMiniApp (beat-level grid) | Bars, sections, repeats, print layout |
| Wrong chords, beats, downbeats | All | Best available OSS models + confidence flags + fast correction |
| Cloud dependency | ChordMiniApp (Firebase, Gemini, YouTube API) | Fully offline (except optional yt-dlp download) |

### 1.3 Non-goals (v1)
- Melody or lyrics transcription, notation/staff output.
- Jazz harmony (9/11/13, alterations) as recognition targets. They remain enterable manually.
- Practice features (slow-down, looping, stem muting).
- Cloud sync, accounts, multi-user.
- Commercial distribution. Non-commercial (CC BY-NC-SA) model weights are acceptable.

## 2. State of the art

### 2.1 Models / libraries

| Component | Task | License (code / weights) | Notes |
|---|---|---|---|
| **madmom** (CPJKU) | Beats, downbeats (DBN), major/minor chords (CNN+CRF), key | BSD / CC BY-NC-SA 4.0 | Reference beat/downbeat tracker. PyPI release 0.16.1 dates from 2018 and is broken on modern Python/NumPy, so it MUST be installed from git `main`. Its chord model is major/minor only, so it is **not usable for chords** here. |
| **BTC** (Bi-directional Transformer for Chords, ISMIR'19, `jayg996/BTC-ISMIR19`) | Chords | MIT | Two vocabularies: majmin (25 classes) or large (170 classes = 12 roots × 14 qualities + N + X). Used by ChordMiniApp. Large-vocabulary weights ship in the repo (`test/btc_model_large_voca.pt`, MIT) ([M0 results](roadmap/m0-results.md)). No inversions. |
| **Chord-CNN-LSTM** (`music-x-lab/ISMIR2019-Large-Vocabulary-Chord-Recognition`) | Chords, large vocab incl. inversions | MIT | Configurable chord dictionary (`data/*_chord_list.txt`). Pretrained models in repo. Used by ChordMiniApp. Little maintenance. |
| **ChordFormer** (2025, arXiv 2502.11840) | Chords, large vocab | none (no license file) | Conformer-based. Reports +2% frame-wise and +6% class-wise accuracy over prior SOTA. Official code + 5 checkpoints at [`mwaseemrandhawa/ChordFormer`](https://github.com/mwaseemrandhawa/ChordFormer), built on the Chord-CNN-LSTM codebase, but unlicensed: not usable as is ([M0 results](roadmap/m0-results.md)). |
| **all-in-one** (`mir-aidj/all-in-one`) | Beats, downbeats, tempo, **section boundaries + labels** | MIT / MIT ([model card](https://huggingface.co/taejunkim/allinone)) | Labels: intro, verse, chorus, bridge, inst, solo, break, outro. Uses Demucs internally. Best candidate for structure detection. |
| **Beat-Transformer** | Beats, downbeats | MIT | Alternative to madmom. Used by ChordMiniApp. |
| **Demucs** (htdemucs) | Source separation | MIT | Optional pre-processing (e.g. remove drums/vocals before chord recognition). Evaluate in M0. |
| **Chordino / NNLS-Chroma** (Vamp plugin) | Chords, rule-based | GPL | Non-ML baseline, fast. Useful as a sanity comparison only. |
| **mir_eval** | Evaluation metrics (chord WCSR, beat F-measure, segmentation) | MIT | Used in M0 benchmarking. |

**Vocabulary gap (important):** the 14 qualities of the standard large vocabulary (MIREX/Harte-based) are
`maj, min, dim, aug, maj6, min6, 7, maj7, min7, minmaj7, dim7, hdim7, sus2, sus4`.
There is **no add2/add9 or add4/add11** class, although the user requires them.
Consequence: off-the-shelf models can't recognize `add*` chords. v1 approach (decision D6):
manual entry always works, and a **chroma post-processing heuristic** (§3.3 step 5b) is evaluated in M0 and shipped only if it passes.

**Known limits (literature + tools):** rare chords are poorly classified (class imbalance). Downbeat and tempo
detection is brittle (double/half tempo, wrong bar phase). Dense mixes degrade results.

### 2.2 Software

| Tool | Type | Strengths | Weaknesses for our user |
|---|---|---|---|
| **ChordMiniApp** (MIT) | Web app, Next.js + Flask | Several chord models, beat sync, section overlays, lead sheet | Web/cloud deps (Firebase, Gemini, YouTube API). Beat grid, not a bar-based chart. Weak editing. |
| **Sheet Sage** (MIT code, NC models) | CLI, Docker, Linux | Melody + chords → LilyPond lead sheet | Needs Docker, 12 GB GPU for the Jukebox mode. Trained on ~24 s segments. Brittle downbeats. No editing. |
| **Chordify** | Commercial, web/mobile | Huge pre-analysed catalog | Subscription, closed, may vanish, limited export |
| **Moises** (Chord Finder) | Commercial, mobile/web | Stems + chords | Paywall (free = first minute), cloud |
| **Chord ai** | Commercial, mobile | Offline on phone | Mobile-first, closed format |
| **Capo** | Commercial, macOS | Time-aligned chords | macOS only, closed |

**Takeaway:** the recognition building blocks exist as OSS (MIT/NC). No tool combines local-only inference,
a musician-style bar/section grid, and fast correction. madsister's value is the **integration + editor**,
not a new model.

## 3. Architecture

```
┌──────────────────────── Tauri app ────────────────────────┐
│  UI (TypeScript, React)          Rust shell (thin)         │
│  - grid editor                   - file dialogs, fs        │
│  - audio player + cursor         - spawns engine process   │
│  - print / export                                          │
└──────────────┬────────────────────────────────────────────┘
               │ spawn: `madsister-engine <cmd> ...`
               │ stdout: JSON Lines (progress events + final result)
┌──────────────▼────────────────────────────────────────────┐
│  engine (Python 3.11+, uv-managed)                         │
│  fetch (yt-dlp) · record (sounddevice) · decode (ffmpeg)   │
│  beats/downbeats/sections · chords · grid quantization     │
└───────────────────────────────────────────────────────────┘
```

Decisions:
- **D1 Tauri + TS UI + Python engine.** Models are Python-only. Tauri keeps the shell light and is good on Linux.
- **D2 Engine is a CLI invoked per job**, not a server. No port or lifecycle management, and it can be tested standalone.
  Model load per job (a few seconds) is acceptable. Revisit only if measured as a problem.
- **D3 Recording happens in the engine** (`sounddevice`), not via `getUserMedia`: WebKitGTK (Tauri on Linux) has unreliable media capture.
- **D4 Platform priority:** Linux (dev + v1) → macOS (nice to have) → Windows. Engine environment via `uv`.
  Packaging a bundled Python + PyTorch (~1–2 GB) is deferred to M6.
- **D5 Offline:** no network call except yt-dlp when the user pastes a URL. Model weights are downloaded once at setup.
- **D6 add2/add4:** chroma heuristic, gated by M0. Acceptance criterion: precision of detected `add*` on `bench/` (target
  set in M0, precision prioritized over recall). If it fails, the heuristic is disabled and `add*` stays manual-only.
- **D7 Transcription modes (from M4b):** two fixed presets over the §3.3 pipeline options, with no per-option UI.
  `fast` = no source separation, fastest acceptable model combo. `accurate` = the best-scoring combo from M0
  (e.g. Demucs + best chord model + add2/add4 heuristic), whatever its cost. The preset contents are defined from the M0
  results table. If M0 shows that no slower combo is measurably better, only one mode ships.
  Before M4b, the engine runs a single default pipeline.

### 3.1 Repository layout
```
madsister/
  app/            # Tauri project (src-tauri/ + src/ TS UI)
  engine/         # Python package, pyproject.toml (uv)
    madsister_engine/
    tests/
  doc/product-brief.md
  bench/          # M0: reference annotations + benchmark script (not shipped)
```

### 3.2 Engine CLI contract
```
madsister-engine transcribe <audio_path> --out <song.json> [--meter 4|3|6/8] [--no-sections] [--mode fast|accurate]
madsister-engine fetch <url> --out-dir <dir>          # yt-dlp → audio file path
madsister-engine record --out <file.wav>              # stops on SIGINT / stdin "stop"
```
stdout: one JSON object per line:
```json
{"type":"progress","stage":"beats","pct":40}
{"type":"progress","stage":"record","pct":0,"elapsedSec":3}
{"type":"result","path":"/…/song.json"}
{"type":"error","message":"…"}
```
Exit code 0 on success, non-zero on error (with an `error` line emitted first).
`record` has no known end: its progress keeps `pct` at 0 and adds `elapsedSec` (whole seconds recorded), once at start then every second.
`--meter 6/8`: the trackers run at the dotted-quarter pulse (2 beats per bar); the Song is written in eighths (`meta.meter` 6/8, slot beats ×3, `tempoBpm` = dotted-quarter BPM).

### 3.3 Transcription pipeline
1. Decode to mono 44.1 kHz WAV (ffmpeg).
2. Beats + downbeats (+ sections) → all-in-one (candidate) or madmom DBN downbeat tracker. Chosen in M0.
3. Frame-level chords → BTC-large or Chord-CNN-LSTM. Chosen in M0. Keep per-segment confidence (max softmax prob).
4. **Beat-synchronous quantization:** for each beat, majority chord label over its frames. Then per bar, group
   consecutive equal beat-labels into chord slots (`beats` = duration). This yields 1–N chords per bar.
5. Simplification: absorb a slot < 1 beat or that is `N` into its neighbour (conservative, configurable later only if needed).
5b. **add2/add4 refinement (D6):** for each slot labelled `maj`/`min` lasting ≥ 2 beats, compute the mean chroma over the slot
   on the harmonic signal (Demucs, vocals + drums removed). If the energy of the 2nd (resp. 4th) degree relative to the
   triad tones is above a threshold τ (tuned in M0), relabel as `add2` (resp. `add4`) and cap confidence below the F-ED-8
   flag threshold so the user reviews it. Never apply to `sus*` (no 3rd = already sus).
6. Sections → map segment boundaries to the nearest downbeat. Labels from all-in-one. Without it, one section "Song".
7. Key estimate (MAY, for display/transposition) — madmom key or skip.
8. Write `Song` JSON (§4).

## 4. Data model (file format `*.madsister.json`)

```ts
type Song = {
  version: 1;
  meta: { title: string; artist?: string; key?: string; tempoBpm?: number; meter: { beats: 3 | 4 | 6; unit: 4 | 8 } };
  audio?: { path: string; sha256: string };          // referenced, not embedded
  sections: Section[];
};
type Section = {
  id: string;
  label: string;                                      // free text; suggested: Intro, Verse, Chorus, Bridge, Solo, Outro
  repeat?: number;                                    // "x2"
  bars: Bar[];
};
type Bar = {
  startSec?: number;                                  // alignment to audio (absent for manually added bars)
  meter?: { beats: number; unit: 4 | 8 };             // override of song meter for this bar only (UI from M2)
  chords: ChordSlot[];                                // 1..N; sum(beats) === (bar.meter ?? song.meta.meter).beats
};
type ChordSlot = {
  chord: string;                                      // canonical Harte-like syntax, e.g. "C:maj7/3", "A:min", "N" (no chord), "%" (repeat previous)
  beats: number;                                      // positive integer
  confidence?: number;                                // 0..1, from engine; cleared when user edits
};
```

Rules:
- Canonical internal syntax = **Harte** (mir_eval-compatible), extended with `add2`, `add4`. Display is a separate
  concern (§5.3).
- Bars are the unit of editing. Chord timing inside a bar is in beats, not seconds.
- `startSec` enables the verification cursor. Edits that insert or delete bars keep other bars' `startSec`.

### 4.1 Supported chord qualities (F-CHORD)
| ID | Quality | Harte | Display (default FR style) | Auto-recognized v1 |
|---|---|---|---|---|
| Q1 | major | `maj` | `C` | yes |
| Q2 | minor | `min` | `Cm` | yes |
| Q3 | augmented | `aug` | `C+` | yes |
| Q4 | diminished | `dim` | `C°` | yes |
| Q5 | sus2 / sus4 | `sus2` / `sus4` | `Csus2` / `Csus4` | yes |
| Q6 | add2 / add4 | `add2` / `add4` (ext.) | `Cadd2` / `Cadd4` | heuristic, gated by M0 (D6); always enterable manually |
| Q7 | 7 / 7M / m7 / m7M | `7` `maj7` `min7` `minmaj7` | `C7` `C7M` `Cm7` `Cm7M` | yes |
| Q8 | 6 / m6 | `maj6` `min6` | `C6` `Cm6` | yes |
| Q9 | dim7 / m7b5 | `dim7` `hdim7` | `C°7` `Cm7b5` | yes |
| Q10 | slash chords / inversions | `/3`, `/5`, `/b7` | `C/E` | model-dependent: Chord-CNN-LSTM yes (`/3 /5 /b7 /2`, rarely emitted), BTC no ([M0 results](roadmap/m0-results.md)) |
| Q11 | no chord | `N` | `N.C.` | yes |

## 5. Functional requirements

### 5.1 Import & transcription
- **F-IN-1** MUST import local audio files: mp3, wav, flac, m4a, ogg (drag & drop + file dialog).
- **F-IN-2** MUST accept a URL (YouTube etc.) downloaded via yt-dlp, then transcribe it.
- **F-IN-3** MUST record from the microphone (start/stop), then transcribe.
- **F-IN-4** MUST show progress by stage and allow cancelling (kill the engine process).
- **F-IN-5** Before transcription, the user MAY force the meter (3/4, 4/4, 6/8). Default: auto.
- **F-IN-6** MUST allow creating an empty grid without audio (manual chart).
- **F-IN-7** (M4b) The user picks the transcription mode `fast` / `accurate` (D7) before transcribing. The UI shows the
  estimated duration of each mode. The user MAY re-run in `accurate` mode on an existing song: this produces a new grid
  proposal and MUST NOT overwrite manual edits silently (the user confirms before replacing).

### 5.2 Grid editor (core)
- **F-ED-1** Display sections as titled blocks. Bars are laid out in rows of 4 (configurable 2/4/8 per row).
- **F-ED-2** Each bar shows 1–N chords. Visual split follows beats (e.g. 2+2 or 3+1 in 4/4).
- **F-ED-3** Keyboard-first: arrows move between bars/slots. Typing replaces the chord via a **chord parser**
  that accepts common spellings (`Am7`, `A-7`, `Amin7`, `C7M`, `Cmaj7`, `CΔ`, `C/E`, `Csus`, `C°`) and normalizes to Harte.
  Invalid input is rejected inline, not silently accepted.
- **F-ED-4** Split a bar (add a slot) / merge slots / change slot duration in beats (integers only).
- **F-ED-11** (M2) Set a per-bar meter override (e.g. a single 2/4 bar in a 4/4 song).
- **F-ED-5** Insert, delete, duplicate bars. Copy/paste bar ranges, including across sections.
- **F-ED-6** Create, rename, reorder, delete sections. Split a section at a bar, merge it with the previous one. Bar and section actions are also in a right-click menu. Set a repeat count.
- **F-ED-7** Undo/redo for every edit (unlimited within the session).
- **F-ED-8** Low-confidence chords (below a threshold, default 0.5) are visually flagged. Editing a chord clears its flag.
- **F-ED-9** Transpose the whole song ±N semitones, with sharp/flat spelling choice.
- **F-ED-10** Autosave and explicit save to `*.madsister.json`. Open recent files.

### 5.3 Display
- **F-DS-1** Display style setting: French (`C7M`, `C°`, `Cm7b5`) default, or international (`Cmaj7`, `Cdim`, `Cø7`).
- **F-DS-2** Latin display style: note names Do, Ré, Mi, Fa, Sol, La, Si with the French suffixes (`Sol7M`, `Ré°`, `Sim7b5`). Display only; chords are still typed with A–G.

### 5.4 Verification playback
- **F-PB-1** Play/pause the source audio. A cursor highlights the current bar (via `startSec`).
- **F-PB-2** Clicking a bar seeks the audio to its `startSec`.
- **F-PB-3** Adjust a bar's alignment: nudge the downbeat phase by ±1 beat for the whole song. This fixes the common
  "wrong bar phase" error in one action.
- **F-PB-4** Fix double/half tempo in one action: merge pairs of bars / split each bar.

### 5.5 Print & export
- **F-EX-1** Print / export PDF: A4, one page when possible, sections + bars grid, title/artist/key/tempo header.
  Implemented as a print stylesheet on the UI (no PDF library).
- **F-EX-2** Export **ChordPro** (`.cho`) using `{start_of_grid}` / `{end_of_grid}` sections (ChordPro 6 grid syntax),
  compatible with OSS apps (ChordPro reference implementation, OnSong-style readers). VERIFY grid support in target apps.
- **F-EX-3** Export **MusicXML** with `<harmony>` elements and empty or slash measures. Opens in MuseScore.
- **F-EX-4** (nice to have) Export **MIDI** with chord pads aligned to the audio tempo, for DAW import.
- **F-EX-5** (later) Import ChordPro.
- **F-EX-6** (later) Classic repeat signs `|: :|` and voltas (1., 2.) in the grid and on print. v1 uses the section `repeat` count (`x2`).

### 5.6 Structure detection (nice to have)
- **F-ST-1** SHOULD auto-detect sections with labels (all-in-one). Falls back to a single section.

## 6. Non-functional requirements
- **NF-1** Fully offline after setup (except yt-dlp downloads).
- **NF-2** Transcription of a 4-min song SHOULD finish in < 2 min on a recent laptop CPU (no GPU required).
  From M4b this applies to `fast` mode. `accurate` has no hard limit but SHOULD stay < 10 min on CPU.
  CUDA MAY be used if present. Apple MPS is used later on macOS.
- **NF-3** Linux x86_64 first-class. macOS arm64 nice to have. Windows later.
- **NF-4** The UI stays responsive during transcription (engine runs in a separate process).
- **NF-5** Accessibility: WCAG 2.2 AA for the editor (keyboard-operable, focus visible, contrast).
- **NF-6** Licensing: code **GPLv3**. A `THIRD_PARTY.md` MUST list every dependency and model with its code and weight license
  (several weights are CC BY-NC-SA → non-commercial). Weights are never committed to the repo; they're downloaded at setup.
- **NF-7** File format is versioned (`version` field). Never silently drop unknown fields on load/save.

## 7. Quality targets & evaluation
- Build a personal reference set in `bench/`: ≥ 10 songs from the target repertoire, hand-annotated
  (downbeats + chords per bar, as `Song` JSON). The user provides the audio. At least 3 songs MUST contain real
  `add2`/`add4` chords (for D6).
- add2/add4 heuristic: precision and recall of `add*` labels at the slot level, reported per τ value.
- Metrics (mir_eval): chord WCSR at `majmin`, `sevenths`, `tetrads`. Downbeat F-measure. Segmentation boundary F.
- **Practical metric (primary):** *edits needed* = number of chord slots the user must change to reach the reference
  grid, after the bar-level quantization. That is what the user experiences.
- No absolute target is set before M0. M0 produces the baseline, and later changes MUST NOT regress it.

## 8. Milestones
Each milestone is usable on its own.

| # | Scope | Done when |
|---|---|---|
| **M0** Spike (engine only) | Python CLI: pipeline §3.3 with 2 chord models × 2 beat trackers (± Demucs), plus the add2/add4 heuristic (step 5b). Benchmark on `bench/`. | Table of metrics **and processing time** per combination. Default models chosen, `fast`/`accurate` preset candidates identified (D7) and recorded in this spec. D6 go/no-go decided with the chosen τ. VERIFY items resolved. |
| **M1** Manual grid editor | Tauri app: F-ED-*, F-DS-1, F-EX-1, F-IN-6, file save/load. **No ML.** | Can type a full song chart by keyboard and print it on A4. |
| **M2** Transcription | Engine CLI integrated: F-IN-1, F-IN-4, F-IN-5, F-ED-8, F-ED-11. | Drop an mp3 → an editable grid in < 2 min. |
| **M3** Verification | F-PB-*. | Cursor follows the bars. Phase and tempo fixes work on a bench song with a known error. |
| **M4** Exports | F-EX-2, F-EX-3, then F-EX-4. | Exported files open in ChordPro CLI and MuseScore. |
| **M4b** Modes | D7, F-IN-7: `--mode` in the engine and a mode picker in the UI. | Both modes run on `bench/`. `accurate` scores measurably better than `fast` in "edits needed", or it is dropped. |
| **M5** Sources | F-IN-2 (yt-dlp), F-IN-3 (mic). F-ST-1 if not done in M2. | URL and recording both yield a grid. |
| **M6** Packaging | Linux AppImage/deb with engine setup. Then macOS. | Installs on a clean machine with one command/package. |

### 8.1 M0 results (provisional — proxy bench, GuitarSet)
Measured on 72 GuitarSet solo-guitar takes, not on the user's songs; the user's bench (§7) confirms or overturns it. Details: [M0 results](roadmap/m0-results.md).
- **Default pipeline:** madmom (beats) + Chord-CNN-LSTM (chords), no Demucs, add2/add4 heuristic off. Fewest edits
  (65.7 per 100 slots) among the combos meeting NF-2: 19 s measured for a 4-min file.
- **D7 candidates:** `fast` = the default. No `accurate` candidate: the best combo regardless of time (+ Demucs) is 0.1 edit/100
  better, which is not measurable → single mode unless the user's band-mix songs show a Demucs gain.
- **D6:** undecided — needs the user's bench with ≥ 3 add2/add4 songs (GuitarSet: precision ≤ 1.1 % at every τ in 0.3–0.7).
- all-in-one is in no preset (6–7 min per 4-min song, lower downbeat F); only section detection (F-ST-1) still depends on it.

## 9. Testing
- Engine: pytest. Unit tests on quantization (§3.3 steps 4–6) with synthetic frame labels. One end-to-end test
  on a short CC-licensed audio clip.
- UI: unit tests on the chord parser/normalizer (table-driven, every spelling in F-ED-3 and the §4.1 table), transposition,
  ChordPro/MusicXML serializers (golden files). Editor commands tested as pure functions on `Song` (undo/redo = command stack).
- Given/When/Then structure. Test names follow `<method>_(when<cond>|of<input>)_<expected>`.

## 10. Risks
| Risk | Mitigation |
|---|---|
| madmom unmaintained, breaks on new Python/NumPy | Pin the Python version in uv. Install from git. all-in-one or Beat-Transformer as alternatives. |
| No model recognizes add2/add4 | Chroma heuristic gated by M0 (D6), with manual entry (F-ED-3) as fallback. Melody/bass passing notes → false positives: mitigated by the harmonic stem, the ≥ 2-beat rule, and low-confidence flagging. |
| Wrong downbeat phase / double tempo | One-click fixes F-PB-3/4. Meter override F-IN-5. |
| PyTorch environment size and packaging | Deferred to M6. uv env in dev. |
| yt-dlp breakage / ToS | Isolated, optional feature. Update via uv. |
| WebKitGTK media quirks on Linux | Recording in the engine (D3). Playback via `<audio>` (GStreamer) — VERIFY in M1. |

## 11. Resolved decisions (formerly open questions)
1. **Repeats:** a `x2` repeat count on sections in M1. Classic repeat signs `|: :|` (and voltas) come later (F-EX-6).
2. **Bar splits:** integer beats only.
3. **Meter changes:** the data model supports a per-bar meter override from the start (`Bar.meter`). M1 UI doesn't expose it.
   M2 MUST allow setting it manually (e.g. one 2/4 bar). Auto-detection is out of scope: the beat trackers assume a constant meter.
4. **License:** GPLv3 for madsister code. Compatible with MIT/BSD dependencies. CC BY-NC-SA weights are downloaded at setup,
   not committed to the repo. Bundling them in a package (M6) counts as aggregation (GPLv3 §5), to be re-checked before any public release.
