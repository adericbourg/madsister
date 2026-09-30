"""Benchmark runner (spec §7, §8 M0): every installed pipeline combination on the reference set -> CSV + Markdown table.

Run from the repo root: `uv run --project engine python bench/run.py [--limit N] [--combos madmom+btc,...] [--add-tau 0.3,0.5]`.

Each expensive output is computed once per take and reused by every combination, cached as JSON under
`bench/data/cache/<dataset>/<id>/` so an interrupted run resumes: decoded audio, Demucs stem, beats per tracker (always on
the full mix, as in `transcribe`), chords per (model, mix|stem). A combination's time is the sum of its components, i.e.
what one `transcribe` call costs. Quantization goes through `pipeline.to_song`, the code path of `transcribe`.
"""

import argparse
import bisect
import csv
import dataclasses
import datetime
import itertools
import json
import statistics
import sys
import time
from pathlib import Path

from madsister_engine import add_heuristic, pipeline
from madsister_engine.beats import BeatResult
from madsister_engine.decode import decode
from madsister_engine.quantize import NO_CHORD, ChordSegment
from madsister_engine.song import to_json

BENCH = Path(__file__).parent
DATASET = "guitarset"  # ponytail: the only reference set so far; add a --dataset option with the user's songs
REFS, AUDIO = BENCH / "data" / DATASET / "refs", BENCH / "data" / DATASET / "audio"
FIELDS = ["file", "combo", "edits", "ref_slots", "majmin", "sevenths", "tetrads", "downbeat_f", "sec"]
FIELDS += ["add_pred", "add_pred_ok", "add_ref", "add_ref_found"]
Slot = tuple[float, float, str]  # start, end, Harte chord


def timed_slots(song: dict, beats: list[float]) -> list[Slot]:
    """Song JSON -> slots in time: bar `startSec` snapped to the nearest beat, then one beat per slot beat."""
    period = statistics.median(b - a for a, b in zip(beats, beats[1:]))
    slots = []
    for bar in (bar for section in song["sections"] for bar in section["bars"]):
        i = min(range(len(beats)), key=lambda k: abs(beats[k] - bar["startSec"]))
        for slot in bar["chords"]:
            end = i + slot["beats"]
            slots.append((beats[i], beats[end] if end < len(beats) else beats[-1] + period, slot["chord"]))
            i = end
    return slots


def _at(slots: list[Slot], t: float) -> int | None:
    """Index of the slot sounding at time t, None outside every slot."""
    i = bisect.bisect_right([start for start, _, _ in slots], t) - 1
    return i if i >= 0 and t < slots[i][1] else None


def _same(a: str, b: str) -> bool:
    """Same Harte chord, the root compared as a pitch class (`Ab:maj` = `G#:maj`: a spelling, not an edit)."""
    (root_a, _, rest_a), (root_b, _, rest_b) = a.partition(":"), b.partition(":")
    if NO_CHORD in (a, b):
        return a == b
    return add_heuristic._pitch_class(root_a) == add_heuristic._pitch_class(root_b) and rest_a == rest_b


def edits_needed(ref: list[Slot], est: list[Slot], ref_beats: list[float]) -> int:
    """Edits needed (spec §7, primary metric): the number of reference slots the user must fix in the predicted grid.

    Each reference beat is looked up in the prediction at its middle (half-way to the next beat, so tracker jitter under
    half a beat doesn't count). A reference slot needs one edit unless all its beats fall in the same predicted slot and
    that slot's chord is the same (exact Harte after `to_harte`, the root compared as a pitch class). So a wrong chord,
    a slot split differently, a shifted bar line or a missing bar each cost one edit per reference slot touched.
    """
    period = statistics.median(b - a for a, b in zip(ref_beats, ref_beats[1:]))
    middles = [(a + b) / 2 for a, b in zip(ref_beats, ref_beats[1:] + [ref_beats[-1] + period])]
    edits = 0
    for start, end, chord in ref:
        hits = {_at(est, t) for t in middles if start <= t < end}
        i = hits.pop() if len(hits) == 1 else None
        edits += i is None or not _same(est[i][2], chord)
    return edits


def add_counts(ref: list[Slot], est: list[Slot]) -> tuple[int, int, int, int]:
    """add2/add4 at slot level (D6): (predicted add slots, of which the reference has the same label at their middle,
    reference add slots, of which the prediction has the same label at their middle)."""

    def matches(slots, other):
        found = [(i := _at(other, (s + e) / 2)) is not None and _same(other[i][2], chord) for s, e, chord in slots]
        return len(found), sum(found)

    def adds(slots):
        return [s for s in slots if s[2].split(":")[-1].split("/")[0] in ("add2", "add4")]

    return *matches(adds(est), ref), *matches(adds(ref), est)


def _mir_eval_label(label: str) -> str:
    return label.replace(":add2", ":maj(2)").replace(":add4", ":maj(4)")  # the §4 extension in plain Harte


def evaluate(ref_id: str, song: dict, beat_result: BeatResult) -> dict:
    import mir_eval  # chords-btc group: not in the CI environment
    import numpy as np

    ref_beats = json.loads((REFS / f"{ref_id}.beats.json").read_text())
    ref = timed_slots(json.loads((REFS / f"{ref_id}.madsister.json").read_text()), ref_beats["beats"])
    est = timed_slots(song, beat_result.beats)
    ref_intervals, ref_labels = mir_eval.io.load_labeled_intervals(str(REFS / f"{ref_id}.lab"))
    scores = mir_eval.chord.evaluate(
        ref_intervals,
        [_mir_eval_label(label) for label in ref_labels],
        np.array([(start, end) for start, end, _ in est]),
        [_mir_eval_label(chord) for _, _, chord in est],
    )
    downbeat_f = mir_eval.beat.f_measure(np.array(ref_beats["downbeats"]), np.array(beat_result.downbeats))
    return {
        "edits": edits_needed(ref, est, ref_beats["beats"]),
        "ref_slots": len(ref),
        **{key: round(scores[key], 4) for key in ("majmin", "sevenths", "tetrads")},
        "downbeat_f": round(downbeat_f, 4),
        **dict(zip(FIELDS[-4:], add_counts(ref, est))),
    }


def _cached(path: Path, compute) -> dict:
    """{"value": compute(), "sec": its wall time}, from `path` when an earlier run already computed it."""
    if not path.exists():
        start = time.perf_counter()
        with pipeline._stdout_to_stderr():
            value = compute()
        path.write_text(json.dumps({"value": value, "sec": time.perf_counter() - start}))
    return json.loads(path.read_text())


def run_take(ref_id: str, combos: list[tuple], done: set) -> list[dict]:
    cache = BENCH / "data" / "cache" / DATASET / ref_id
    cache.mkdir(parents=True, exist_ok=True)
    wav = _cached(cache / "decode.json", lambda: str(decode(AUDIO / f"{ref_id}_mic.wav", cache)))
    chromas = {}  # mix|stem -> (chroma, sec): cheap enough not to cache on disk
    rows = []
    for beats, chords, separate, tau in combos:
        name = "+".join([beats, chords] + ["demucs"] * separate + [f"tau{tau}"] * (tau is not None))
        if (ref_id, name) in done:
            continue
        parts = [wav]
        harmonic = wav["value"]
        if separate:
            separator = pipeline._adapter("source separation", pipeline._SEPARATE, "demucs")
            stem = _cached(cache / "separate.json", lambda: str(separator.harmonic_stem(Path(wav["value"]), cache)))
            parts.append(stem)
            harmonic = stem["value"]
        tracker = pipeline._adapter("beat tracker", pipeline._BEATS, beats)
        beat_part = _cached(cache / f"beats-{beats}.json", lambda: dataclasses.asdict(tracker.track(wav["value"], None)))
        recognizer = pipeline._adapter("chord model", pipeline._CHORDS, chords)
        input_name = "stem" if separate else "mix"
        chord_part = _cached(
            cache / f"chords-{chords}-{input_name}.json",
            lambda: [dataclasses.astuple(s) for s in recognizer.recognize(harmonic)],
        )
        parts += [beat_part, chord_part]
        chroma = None
        if tau is not None:
            if input_name not in chromas:
                start = time.perf_counter()
                chromas[input_name] = (add_heuristic.chroma(harmonic), time.perf_counter() - start)
            chroma, sec = chromas[input_name]
            parts.append({"sec": sec})

        start = time.perf_counter()
        beat_result = BeatResult(**beat_part["value"])
        segments = [ChordSegment(*s) for s in chord_part["value"]]
        song = to_json(pipeline.to_song(ref_id, beat_result, segments, None, False, chroma, tau))
        sec = sum(p["sec"] for p in parts) + time.perf_counter() - start
        rows.append({"file": ref_id, "combo": name, **evaluate(ref_id, song, beat_result), "sec": round(sec, 2)})
    return rows


def table(rows: list[dict]) -> str:
    """Mean per combination. Edits per 100 reference slots and add precision/recall are pooled over files."""
    lines = [
        "| combo | files | edits /100 slots | majmin | sevenths | tetrads | downbeat F | time (s) | add pred (FP) | add P | add R |",
        "|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for combo, group in itertools.groupby(sorted(rows, key=lambda r: r["combo"]), key=lambda r: r["combo"]):
        group = list(group)

        def total(key):
            return sum(float(r[key]) for r in group)

        def mean(key):
            return f"{total(key) / len(group):.3f}"

        def ratio(num, den):
            return f"{total(num) / total(den):.2f}" if total(den) else "–"

        pred, fp = int(total("add_pred")), int(total("add_pred") - total("add_pred_ok"))
        lines.append(
            f"| {combo} | {len(group)} | {100 * total('edits') / total('ref_slots'):.1f} | {mean('majmin')} "
            f"| {mean('sevenths')} | {mean('tetrads')} | {mean('downbeat_f')} | {total('sec') / len(group):.1f} "
            f"| {pred} ({fp}) | {ratio('add_pred_ok', 'add_pred')} | {ratio('add_ref_found', 'add_ref')} |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--limit", type=int, help="first N takes only")
    parser.add_argument("--combos", help="comma-separated, e.g. madmom+btc,allinone+cnnlstm+demucs (default: all installed)")
    parser.add_argument("--add-tau", default="", help="comma-separated τ values: each combination is also run with them")
    parser.add_argument("--out", type=Path, help=f"CSV path (default: bench/results/<date>-{DATASET}.csv)")
    args = parser.parse_args()

    beats = [name for name, (_, marker, _) in pipeline._BEATS.items() if pipeline._installed(marker)]
    chords = [name for name, (_, marker, _) in pipeline._CHORDS.items() if pipeline._installed(marker)]
    separate = [False, True] if pipeline._installed(pipeline._SEPARATE["demucs"][1]) else [False]
    combos = list(itertools.product(beats, chords, separate))
    if args.combos:
        wanted = set(args.combos.split(","))
        combos = [c for c in combos if "+".join([c[0], c[1]] + ["demucs"] * c[2]) in wanted]
    taus = [None] + [float(t) for t in args.add_tau.split(",") if t]
    combos = [(*c, tau) for c in combos for tau in taus]

    out = args.out or BENCH / "results" / f"{datetime.date.today()}-{DATASET}.csv"
    out.parent.mkdir(parents=True, exist_ok=True)
    rows = list(csv.DictReader(out.open())) if out.exists() else []
    done = {(r["file"], r["combo"]) for r in rows}
    ids = sorted(p.name.removesuffix(".lab") for p in REFS.glob("*.lab"))[: args.limit]
    started = time.perf_counter()
    with out.open("a", newline="") as f:
        writer = csv.DictWriter(f, FIELDS)
        if not rows:
            writer.writeheader()
        for n, ref_id in enumerate(ids, 1):
            print(f"[{n}/{len(ids)}] {ref_id} ({time.perf_counter() - started:.0f} s)", file=sys.stderr)
            for row in run_take(ref_id, combos, done):
                writer.writerow(row)
                rows.append(row)
            f.flush()
    markdown = table(rows)
    out.with_suffix(".md").write_text(f"# Bench {DATASET} ({len({r['file'] for r in rows})} files)\n\n{markdown}")
    print(markdown)


if __name__ == "__main__":
    main()
