"""madmom RNN + DBN downbeat tracker (spec §2.1). Needs the `beats-madmom` dependency group."""

from pathlib import Path

from madmom.features.downbeats import DBNDownBeatTrackingProcessor, RNNDownBeatProcessor

from madsister_engine.beats import BeatResult


def track(wav_path: str | Path, meter: int | None = None) -> BeatResult:
    activations = RNNDownBeatProcessor()(str(wav_path))
    dbn = DBNDownBeatTrackingProcessor(beats_per_bar=[meter] if meter else [3, 4], fps=100)
    rows = dbn(activations)  # (time, position in bar), position 1 = downbeat
    return BeatResult(
        beats=[float(t) for t, _ in rows],
        downbeats=[float(t) for t, pos in rows if pos == 1],
    )
