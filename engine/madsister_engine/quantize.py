"""Model-independent quantization: chord segments + beats -> bars and sections (spec §3.3 steps 4-6)."""

import itertools
import statistics
import uuid
from collections import defaultdict
from collections.abc import Callable
from dataclasses import dataclass, replace

from madsister_engine.song import Bar, ChordSlot, Meta, Meter, Section, Song

NO_CHORD = "N"

_SECTION_LABELS = {
    "intro": "Intro",
    "verse": "Verse",
    "chorus": "Chorus",
    "bridge": "Bridge",
    "inst": "Instrumental",
    "solo": "Solo",
    "break": "Break",
    "outro": "Outro",
}


@dataclass(frozen=True)
class ChordSegment:
    start: float
    end: float
    label: str  # Harte
    confidence: float | None = None


def _mean(values: list[float]) -> float | None:
    return statistics.fmean(values) if values else None


def beat_labels(chords: list[ChordSegment], beats: list[float]) -> list[tuple[str, float | None]]:
    """For each beat interval, the label covering most of it, with the duration-weighted mean confidence of that label."""
    period = statistics.median(b - a for a, b in zip(beats, beats[1:])) if len(beats) > 1 else 0.5
    ends = beats[1:] + [beats[-1] + period]
    labels = []
    for start, end in zip(beats, ends):
        durations: dict[str, float] = defaultdict(float)
        weighted: dict[str, list[tuple[float, float]]] = defaultdict(list)
        for seg in chords:
            overlap = min(end, seg.end) - max(start, seg.start)
            if overlap > 0:
                durations[seg.label] += overlap
                if seg.confidence is not None:
                    weighted[seg.label].append((overlap, seg.confidence))
        if not durations:
            labels.append((NO_CHORD, None))
            continue
        label = max(durations, key=durations.get)
        pairs = weighted[label]
        confidence = sum(d * c for d, c in pairs) / sum(d for d, _ in pairs) if pairs else None
        labels.append((label, confidence))
    return labels


def _slots(labels: list[tuple[str, float | None]]) -> list[ChordSlot]:
    slots = []
    for label, group in itertools.groupby(labels, key=lambda item: item[0]):
        confidences = [c for _, c in group]
        slots.append(ChordSlot(label, len(confidences), _mean([c for c in confidences if c is not None])))
    return slots


def bars_from_beats(
    labels: list[tuple[str, float | None]], beats: list[float], downbeats: list[float], meter_beats: int
) -> list[Bar]:
    """Split beats into bars at downbeats and group equal consecutive beat labels into slots.

    A bar whose beat count differs from the meter keeps it through a meter override (song edges, tracker errors).
    """
    starts = sorted({min(range(len(beats)), key=lambda i: abs(beats[i] - d)) for d in downbeats})
    # ponytail: a 1-beat pickup is dropped (usually a tracker artefact); 2+ beats become a short bar.
    if starts and starts[0] >= 2:
        starts.insert(0, 0)
    bars = []
    for start, end in zip(starts, starts[1:] + [len(beats)]):
        count = end - start
        bars.append(
            Bar(
                chords=_slots(labels[start:end]),
                start_sec=beats[start],
                meter=None if count == meter_beats else Meter(count, 4),
            )
        )
    return bars


def simplify(bars: list[Bar]) -> list[Bar]:
    """Absorb N slots into their neighbour within the bar, then merge equal consecutive slots. An all-N bar stays N."""
    result = []
    for bar in bars:
        if all(slot.chord == NO_CHORD for slot in bar.chords):
            result.append(replace(bar, chords=[ChordSlot(NO_CHORD, sum(s.beats for s in bar.chords))]))
            continue
        slots: list[ChordSlot] = []
        pending = 0  # N beats before the first real chord of the bar
        for slot in bar.chords:
            if slot.chord == NO_CHORD:
                if slots:
                    slots[-1] = replace(slots[-1], beats=slots[-1].beats + slot.beats)
                else:
                    pending += slot.beats
            elif slots and slots[-1].chord == slot.chord:
                slots[-1] = replace(slots[-1], beats=slots[-1].beats + slot.beats)
            else:
                slots.append(replace(slot, beats=slot.beats + pending))
                pending = 0
        result.append(replace(bar, chords=slots))
    return result


def _section_id() -> str:
    return uuid.uuid4().hex[:8]


def sections_from_segments(bars: list[Bar], segments: list[tuple[float, float, str]] | None) -> list[Section]:
    """Snap each segment start to the nearest bar start. Without segments, a single "Song" section."""
    if not segments:
        return [Section(_section_id(), "Song", bars)]
    starts = [
        min(range(len(bars)), key=lambda i: abs(bars[i].start_sec - seg_start)) for seg_start, _, _ in segments
    ]
    starts[0] = 0
    sections = []
    for (_, _, label), start, end in zip(segments, starts, starts[1:] + [len(bars)]):
        if end > start:
            sections.append(Section(_section_id(), _SECTION_LABELS.get(label, label.capitalize()), bars[start:end]))
    return sections


def build_song(
    title: str,
    chords: list[ChordSegment],
    beats: list[float],
    downbeats: list[float],
    meter_beats: int,
    segments: list[tuple[float, float, str]] | None,
    refine: Callable[[list[Bar]], list[Bar]] | None = None,
) -> Song:
    """`refine` rewrites the simplified bars before sectioning (the add2/add4 step 5b)."""
    bars = simplify(bars_from_beats(beat_labels(chords, beats), beats, downbeats, meter_beats))
    if refine:
        bars = refine(bars)
    period = statistics.median(b - a for a, b in zip(beats, beats[1:]))
    return Song(
        meta=Meta(title=title, meter=Meter(meter_beats, 4), tempo_bpm=round(60 / period)),
        sections=sections_from_segments(bars, segments),
    )
