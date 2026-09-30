"""Beat/downbeat trackers (spec §3.3 step 2). Each adapter exposes `track(wav_path, meter) -> BeatResult`."""

from dataclasses import dataclass


@dataclass(frozen=True)
class BeatResult:
    beats: list[float]  # seconds
    downbeats: list[float]  # seconds, subset of beats
    segments: list[tuple[float, float, str]] | None = None  # (start, end, label), from trackers that also segment
