"""The Song file format (spec §4), engine side: build, validate, write."""

import dataclasses
import json
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Meter:
    beats: int
    unit: int


@dataclass(frozen=True)
class ChordSlot:
    chord: str  # Harte syntax, e.g. "C:maj7/3", "A:min", "N"
    beats: int
    confidence: float | None = None


@dataclass(frozen=True)
class Bar:
    chords: list[ChordSlot]
    start_sec: float | None = None
    meter: Meter | None = None  # override of the song meter for this bar only


@dataclass(frozen=True)
class Section:
    id: str
    label: str
    bars: list[Bar]
    repeat: int | None = None


@dataclass(frozen=True)
class Meta:
    title: str
    meter: Meter
    artist: str | None = None
    key: str | None = None
    tempo_bpm: float | None = None


@dataclass(frozen=True)
class Audio:
    path: str
    sha256: str


@dataclass(frozen=True)
class Song:
    meta: Meta
    sections: list[Section]
    audio: Audio | None = None


def _camel(name: str) -> str:
    head, *rest = name.split("_")
    return head + "".join(part.capitalize() for part in rest)


def _clean(value):
    if isinstance(value, dict):
        return {_camel(k): _clean(v) for k, v in value.items() if v is not None}
    if isinstance(value, list):
        return [_clean(v) for v in value]
    return value


def to_json(song: Song) -> dict:
    return {"version": 1, **_clean(dataclasses.asdict(song))}


def validate(song: Song) -> None:
    for s, section in enumerate(song.sections):
        for b, bar in enumerate(section.bars):
            where = f"sections[{s}].bars[{b}]"
            if not bar.chords:
                raise ValueError(f"{where}: a bar needs at least one chord slot")
            for slot in bar.chords:
                if not isinstance(slot.beats, int) or slot.beats <= 0:
                    raise ValueError(f"{where}: slot beats must be positive integers, got {slot.beats!r}")
                if slot.confidence is not None and not 0 <= slot.confidence <= 1:
                    raise ValueError(f"{where}: confidence must be in [0, 1], got {slot.confidence}")
            expected = (bar.meter or song.meta.meter).beats
            actual = sum(slot.beats for slot in bar.chords)
            if actual != expected:
                raise ValueError(f"{where}: slot beats sum to {actual}, meter expects {expected}")


def write(song: Song, path: Path) -> None:
    validate(song)
    Path(path).write_text(json.dumps(to_json(song), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
