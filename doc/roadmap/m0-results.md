# M0 results — **provisional (proxy bench, GuitarSet)**

Everything below comes from the GuitarSet proxy set (solo acoustic guitar), not from the user's songs. It picks a default
so M2 can start. The user's own bench (spec §7: ≥ 10 songs, ≥ 3 with add2/add4) must confirm or overturn it.

## Environment
- Apple M4 Pro (14 cores), 48 GB RAM, macOS 27.0.1. **CPU only**: no adapter uses MPS or CUDA.
- Python 3.11.16 (uv 0.12.19), torch 2.5.1, numpy 2.4.6, madmom 0.17.dev0 (git `27f032e`), allin1 1.1.0 + NATTEN 0.17.4,
  demucs 4.1.0, librosa 0.11.0, mir_eval 0.8.2, ffmpeg 9.0.2. BTC and Chord-CNN-LSTM at the pinned shas (see `THIRD_PARTY.md`).
- Command: `uv run --project engine python bench/run.py --add-tau 0.3,0.4,0.5,0.6,0.7` (commit after `0e75b99`), run on 2026-09-30.
  Raw rows: [`bench/results/2026-09-30-guitarset.csv`](../../bench/results/2026-09-30-guitarset.csv), full table:
  [`.md`](../../bench/results/2026-09-30-guitarset.md).

## Dataset
72 GuitarSet "comp" takes (36 Rock, 36 Singer-Songwriter), 22–46 s each (mean 35 s, 42 min in total), 1,056 reference slots,
14 reference add2/add4 slots. Details and limits: [`bench/README.md`](../../bench/README.md).

## Results per combination (τ off)
Edits are per 100 reference slots, pooled over the 72 takes. "Bass ignored" compares root + quality only (`C:maj/5` = `C:maj`).
Time = the sum of the cached components (decode, Demucs, beats, chords, quantization), model loading included, **Python
imports excluded**. The 4-min figure scales that by the audio length (seconds of compute per second of audio × 240).

| combo | edits /100 | edits, bass ignored | majmin | sevenths | tetrads | downbeat F | time / take (s) | 4-min song (s) |
|---|---|---|---|---|---|---|---|---|
| **madmom+cnnlstm** | **65.7** | **58.0** | **0.927** | **0.887** | 0.799 | **0.576** | 2.2 | **15** |
| madmom+cnnlstm+demucs | 65.6 | 57.9 | 0.926 | 0.887 | 0.800 | 0.576 | 12.1 | 83 |
| madmom+btc | 67.6 | 60.0 | 0.905 | 0.868 | 0.788 | 0.576 | 1.9 | 13 |
| madmom+btc+demucs | 67.6 | 59.9 | 0.904 | 0.867 | 0.787 | 0.576 | 11.7 | 80 |
| allinone+cnnlstm | 71.2 | 64.4 | 0.770 | 0.736 | 0.660 | 0.428 | 52.0 | 356 |
| allinone+cnnlstm+demucs | 71.1 | 64.3 | 0.768 | 0.735 | 0.659 | 0.428 | 61.8 | 423 |
| allinone+btc | 72.2 | 65.9 | 0.754 | 0.722 | 0.652 | 0.428 | 51.6 | 353 |
| allinone+btc+demucs | 72.3 | 66.0 | 0.753 | 0.721 | 0.651 | 0.428 | 61.4 | 421 |

Real CLI calls (`madsister-engine transcribe`, default combo, imports and uv start-up included): **3.8 s** for a 22 s take,
**19 s for a 4 min 16 s file** (8 takes concatenated). NF-2 (< 2 min) holds with a wide margin for madmom-based combos; every
all-in-one combo misses it (~6–7 min for 4 min of audio, on a fast CPU).

### NF-2 on a 4-min mp3 (M2-6)
`madsister-engine transcribe` (default combo, via `uv run --no-sync`), 2026-10-01, same machine: a 240 s mp3 (192 kb/s, 8
GuitarSet takes concatenated with ffmpeg, cut at 4 min) → **20.4 s** wall time, 99 bars. Per stage, from the progress events:
start-up + imports 0.5 s, decode 0.3 s, beats (madmom) 12.3 s, chords (CNN-LSTM) 7.2 s, quantize + hash + write 0.1 s.
NF-2 (< 2 min) holds, ~6× under the limit.

- **Bar phase dominates the edits.** For madmom+cnnlstm, the 46 takes with downbeat F ≥ 0.5 need 49 edits/100 (38 bass ignored);
  the 26 takes with downbeat F < 0.5 need 94/100 (92 bass ignored), although their chord accuracy is similar (majmin 0.885 vs 0.951).
  A shifted bar line costs one edit per slot. The M3 phase/tempo fixes (F-PB-3/4) matter more than the choice of chord model.
- **Inversions** cost ~8 edits/100 for every combo (edits vs bass ignored). 23 % of the GuitarSet reference segments have a bass
  (`/5`, `/b6`…, some from extensions); only 1.6 % of the CNN-LSTM segments do (15/967), and BTC can't produce any.
- **Tempo octave / meter errors** (median beat period vs reference): madmom 50/72 right, 9 double tempo, 13 at ~2/3 or ~3/2
  (swing/shuffle takes, e.g. `SS1-68`, `SS2-88`, `SS3-84`, where the tracker locks on the triplet grid). all-in-one 45/72 right,
  7 half, 3 double, 16 at ~2/3 or ~3/2, and **one failure**: 1 beat on `00_SS2-107-Ab_comp` (scored as a full failure: every slot
  to re-enter). `transcribe` crashed on that case (`StatisticsError` in `build_song`); since M2-6 it reports `could not find beats in this audio`.
- **Demucs changes nothing** on solo guitar (±0.1 edit, +10 s per take, ~+70 s per 4-min song). Expected: nothing to remove.
  It may matter on band mixes: the user's bench decides.
- **CNN-LSTM beats BTC** by ~2 edits/100 and ~0.02 majmin with either tracker, at the same cost.
- Singer-Songwriter takes are easier than Rock (63.8 vs 67.6 edits/100; downbeat F 0.669 vs 0.484).

## add2/add4 heuristic (D6): τ sweep
Slot level, pooled over the 72 takes, default combo madmom+cnnlstm (the other combos are within a few slots, see the full table).
14 reference add slots in total.

| τ | add slots predicted | false positives | precision | recall | edits /100 (65.7 with τ off) |
|---|---|---|---|---|---|
| 0.3 | 668 | 664 | 0.006 | 0.29 (4/14) | 82.7 |
| 0.4 | 482 | 478 | 0.008 | 0.29 (4/14) | 78.5 |
| 0.5 | 362 | 358 | 0.011 | 0.29 (4/14) | 75.5 |
| 0.6 | 244 | 242 | 0.008 | 0.14 (2/14) | 72.3 |
| 0.7 | 160 | 159 | 0.006 | 0.07 (1/14) | 69.4 |

Even at τ 0.7 the heuristic relabels ~15 % of the slots wrongly and adds ~4 edits/100. Open strings ringing on an acoustic
guitar are a likely cause. With Demucs the numbers are the same (see the full table).

## VERIFY findings (spec §2.1, §4.1)
| Item | Finding | Source |
|---|---|---|
| BTC weights | Available: `test/btc_model_large_voca.pt` (170-class large vocabulary) ships in the MIT-licensed repo, pinned at `2682317`. | https://github.com/jayg996/BTC-ISMIR19 |
| all-in-one weights license | MIT (model card). | https://huggingface.co/taejunkim/allinone |
| ChordFormer code/weights | Official code by the first author at `mwaseemrandhawa/ChordFormer`, with 5 cross-validation checkpoints (`cache_data/chordformer_head16(1.0,1.0)_s{0..4}.best.sdict`). Built on the Chord-CNN-LSTM (music-x-lab) codebase, so an adapter would be close to ours. **No license file** (GitHub API: `license: null`), so all rights reserved by default: not usable until the author adds one. The arXiv page links no code. Research only, no adapter. | https://github.com/mwaseemrandhawa/ChordFormer, https://arxiv.org/abs/2502.11840 |
| Slash chords (Q10) | Model-dependent. Chord-CNN-LSTM outputs inversions (`submission` dictionary: `/3 /5 /b7 /2` for maj, `/b3 /5 /b7 /2` for min). BTC does not (root × 14 qualities only). madmom is not used for chords. | https://github.com/music-x-lab/ISMIR2019-Large-Vocabulary-Chord-Recognition, https://github.com/jayg996/BTC-ISMIR19 |
| madmom weights | Ship inside the package, CC BY-NC-SA 4.0 (non-commercial). | https://github.com/CPJKU/madmom |
| `window.print()` in the Tauri v2 webview (M1-13) | Works. On Linux (WebKitGTK) and Windows the native `window.print()` opens the print dialog. On macOS WKWebView has no `window.print()`, so Tauri's webview plugin injects a shim that calls `plugin:webview\|print`, which needs the `core:webview:allow-print` permission (not in `core:default`; added to `capabilities/default.json`). `printSong()` just calls `window.print()`. Not tried in a running app tonight (no display session). | tauri 2.12.0 `src/webview/plugin.rs` + `scripts/print.js`, https://github.com/tw93/Pake/issues/1396, https://github.com/tauri-apps/tauri/issues/3066 |
| WebKitGTK/WKWebView `<audio>` (M3-1) | Plays through the platform media stack: AVFoundation on macOS (mp3, m4a, wav, flac natively; a blob needs its MIME type), GStreamer on Linux (`gstreamer1.0-plugins-base`/`-good` for wav/ogg/mp3/flac, `gstreamer1.0-libav` for m4a/AAC, AppImage `bundleMediaFramework`). GStreamer has no source for Tauri's `asset://` scheme, so `<audio src={convertFileSrc(…)}>` fails on Linux (`NotSupportedError`, `GST_CORE_ERROR_MISSING_PLUGIN`); blob URLs work. Decision: Rust `read_audio` returns the bytes, played as a typed blob on both platforms (no asset protocol). From docs and reports, not tried in a running app (no display session). | https://yanovskyy.com/blog/en/tauri-webkit, https://github.com/InstaZDLL/WaveFlow/pull/773, https://v2.tauri.app/distribute/appimage/ |

## Richer chord list (Chord-CNN-LSTM)
`chords/chord_list.txt` adds 12 templates (6, m6, mMaj7, `maj(9)`, `min(9)`, `maj(11)`, `min(11)`, `sus4(b7,9)`, `maj6(9)`,
`min6(9)`, m11, maj13) to upstream's `submission` list, and `to_harte` keeps 9/11/13/maj9/min9 instead of degrading them.
Same bench, `--combos madmom+cnnlstm`, 72 takes, before (`submission` list) and after:

| chord list | edits /100 | edits, bass ignored | majmin | sevenths | tetrads |
|---|---|---|---|---|---|
| `submission` | 65.7 | 58.0 | 0.927 | 0.889 | 0.799 |
| `chord_list.txt` | 65.7 | 58.0 | 0.927 | 0.889 | 0.799 |

Identical on every take: none of the 1056 predicted slots uses a rich quality. On a probe take the decoder holds all 445
states, but the best rich template never exceeds 0.06 of a frame's probability mass. The five ensemble networks were trained
with the `submission` vocabulary and a reweighting that favors common classes, so the extra templates are reachable but
never win on guitar. No regression, no gain: detecting rich chords would need a larger `diff_trans_penalty`/prior change
in the decoder or a different model, to be tried on the user's own songs.

## Provisional decisions
- **Default (`auto`) = madmom + Chord-CNN-LSTM, no Demucs, add2/add4 heuristic off.** Lowest edits (65.7/100) among the combos
  meeting NF-2 (~15 s per 4-min song estimated, 19 s measured). Set as the `auto` order in `engine/madsister_engine/pipeline.py`.
- **D7: single mode on this bench.** The best combo regardless of time (madmom+cnnlstm+demucs, 65.6) is 0.1 edit/100 better:
  not measurable. `fast` = the default. An `accurate` candidate (the default + Demucs) only makes sense if the user's band-mix
  songs show a gain.
- **D6: undecided — needs the user's bench with ≥ 3 add2/add4 songs.** On GuitarSet, precision is ≤ 1.1 % at every τ, so
  nothing supports enabling it; it stays off (`--add-tau` unset).
- **all-in-one is not in any preset.** It's the slowest tracker (~1.5 s per second of audio), its downbeats are worse here (F 0.428
  vs 0.576), and it crashed on one take. Its only remaining value is section detection (F-ST-1, nice to have), which the default
  no longer gives (single section). Dropping it would unblock the torch upgrade (`BLOCKERS.md`, Dependabot): NATTEN 0.17.4 is
  what forces `torch==2.5.1`. The chord adapters call `torch.load(..., weights_only=False)` on pickled checkpoints, which would
  need checking with torch ≥ 2.6.

## Open questions for the user
1. Confirm the default on your own songs (band mixes, vocals): does Demucs start to pay off there (→ an `accurate` mode)?
2. all-in-one: keep it for section detection only (slow, opt-in), replace section detection by something cheaper (M5-4), or drop
   it and unpin torch?
3. D6: the heuristic only relabels `maj` slots. Minor add chords (e.g. `Am(add9)`) have no quality in the §4 format. Add
   `madd2`/`madd4`-style qualities, or leave minor adds to manual entry?
4. Inversions: 8 edits/100 come from basses. Is it acceptable for the default grid to rarely show slash chords (CNN-LSTM can emit
   `/3 /5 /b7 /2` but did on 1.6 % of its segments here)?
5. ChordFormer has no license: ask the author, or ignore it?
