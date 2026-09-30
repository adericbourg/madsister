"""License-free synthetic clip: 120 BPM 4/4, C G A:min F one bar each, twice (~16 s)."""

import wave
from pathlib import Path

import numpy as np

BPM = 120
BEATS_PER_BAR = 4
PROGRESSION = ["C:maj", "G:maj", "A:min", "F:maj"] * 2
# MIDI notes of each triad (root position, around middle C)
_TRIADS = {"C:maj": (60, 64, 67), "G:maj": (55, 59, 62), "A:min": (57, 60, 64), "F:maj": (53, 57, 60)}


def _hz(midi: int) -> float:
    return 440.0 * 2 ** ((midi - 69) / 12)


def make_clip(path: Path, rate: int = 44100, channels: int = 1) -> Path:
    beat = 60 / BPM
    bar_len = int(round(BEATS_PER_BAR * beat * rate))
    t = np.arange(bar_len) / rate
    bars = []
    for label in PROGRESSION:
        notes = _TRIADS[label]
        bar = sum(np.sin(2 * np.pi * _hz(n) * t) for n in notes) / len(notes)
        bar += 0.5 * np.sin(2 * np.pi * _hz(notes[0] - 12) * t)  # quieter bass an octave down
        bars.append(0.3 * bar)
    signal = np.concatenate(bars)

    # Kick on each beat: decaying low sine, accented on the downbeat.
    kick_len = int(0.1 * rate)
    kt = np.arange(kick_len) / rate
    kick = np.sin(2 * np.pi * 60 * kt) * np.exp(-kt * 40)
    for i in range(len(PROGRESSION) * BEATS_PER_BAR):
        start = int(round(i * beat * rate))
        gain = 0.6 if i % BEATS_PER_BAR == 0 else 0.3
        signal[start : start + kick_len] += gain * kick[: len(signal) - start]

    pcm = (np.clip(signal, -1, 1) * 32767).astype("<i2")
    with wave.open(str(path), "wb") as out:
        out.setnchannels(channels)
        out.setsampwidth(2)
        out.setframerate(rate)
        out.writeframes(np.repeat(pcm, channels).tobytes())
    return path
