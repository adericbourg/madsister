import json
import os
import sys
import types

import numpy as np
import pytest

from fixtures.make_clip import PROGRESSION
from madsister_engine import add_heuristic, cli, pipeline
from madsister_engine.beats import BeatResult
from madsister_engine.quantize import ChordSegment

BEATS = [i * 0.5 for i in range(32)]  # the synthetic clip: 120 BPM, 8 bars of 4/4


def _lines(capfd):
    return [json.loads(line) for line in capfd.readouterr().out.splitlines()]


def _stages(lines):
    return [(line["stage"], line["pct"]) for line in lines if line["type"] == "progress"]


@pytest.fixture
def fakes(monkeypatch):
    """The default tracker (writing to fd 1, as all-in-one does) and chord model replaced by fixed outputs matching the synthetic
    clip; every group reported installed."""
    calls = {}

    def track(wav, meter=None):
        calls["track"] = (wav, meter)
        os.write(1, b"model chatter\n")  # as allin1's Demucs subprocess does
        return BeatResult(BEATS, BEATS[::4], [(0.0, 8.0, "verse"), (8.0, 16.0, "chorus")])

    def recognize(wav):
        calls["recognize"] = wav
        return [ChordSegment(2.0 * i, 2.0 * i + 2, label, 0.9) for i, label in enumerate(PROGRESSION)]

    monkeypatch.setitem(sys.modules, "madsister_engine.beats.madmom_tracker", types.SimpleNamespace(track=track))
    monkeypatch.setitem(sys.modules, "madsister_engine.chords.cnnlstm", types.SimpleNamespace(recognize=recognize))
    monkeypatch.setattr(pipeline, "_installed", lambda module: True)
    return calls


def test_main_of_transcribe_emits_stages_in_order_and_writes_song(fakes, make_wav, tmp_path, capfd):
    # Given the synthetic clip and fixed model outputs
    wav = make_wav()
    out = tmp_path / "s.madsister.json"

    # When
    code = cli.main(["transcribe", str(wav), "--out", str(out)])

    # Then the stages come in order, then the result with the absolute path
    lines = _lines(capfd)
    assert code == 0
    assert _stages(lines) == [("decode", 5), ("beats", 40), ("chords", 70), ("quantize", 90), ("write", 100)]
    assert lines[-1] == {"type": "result", "path": str(out.resolve())}
    # And the Song has the detected meter, the sections and one chord per bar
    song = json.loads(out.read_text())
    assert song["meta"]["meter"] == {"beats": 4, "unit": 4}
    assert [s["label"] for s in song["sections"]] == ["Verse", "Chorus"]
    bars = [bar for s in song["sections"] for bar in s["bars"]]
    assert [bar["chords"][0]["chord"] for bar in bars] == PROGRESSION

    # When sections are disabled and the meter forced
    code = cli.main(["transcribe", str(wav), "--out", str(out), "--no-sections", "--meter", "3"])

    # Then one "Song" section, and the meter reaches the tracker
    assert code == 0
    assert [s["label"] for s in json.loads(out.read_text())["sections"]] == ["Song"]
    assert fakes["track"][1] == 3


def test_main_of_transcribe_with_separate_and_add_tau_uses_harmonic_stem(fakes, make_wav, tmp_path, capfd, monkeypatch):
    # Given a fake Demucs that copies its input, and chroma with a strong D everywhere (C → C:add2)
    def harmonic_stem(wav, out_dir):
        dst = out_dir / "harmonic.wav"
        dst.write_bytes(wav.read_bytes())
        return dst

    monkeypatch.setitem(sys.modules, "madsister_engine.separate", types.SimpleNamespace(harmonic_stem=harmonic_stem))
    chroma = np.full((12, 400), 0.05)
    chroma[[0, 2, 4, 7]] = 1.0
    monkeypatch.setattr(add_heuristic, "chroma", lambda wav: (chroma, np.arange(400) * 0.05))
    out = tmp_path / "s.json"

    # When
    code = cli.main(["transcribe", str(make_wav()), "--out", str(out), "--separate", "--add-tau", "0.5"])

    # Then the stem feeds the chord model and the C bars become add2
    assert code == 0
    assert ("separate", 20) in _stages(_lines(capfd))
    assert fakes["recognize"].name == "harmonic.wav"
    bars = [bar for s in json.loads(out.read_text())["sections"] for bar in s["bars"]]
    assert bars[0]["chords"][0]["chord"] == "C:add2"


def test_main_when_adapter_fails_or_group_missing_emits_error_and_exits_1(fakes, make_wav, tmp_path, capfd, monkeypatch):
    # Given a chord model that raises
    def recognize(wav):
        raise RuntimeError("boom")

    monkeypatch.setitem(sys.modules, "madsister_engine.chords.cnnlstm", types.SimpleNamespace(recognize=recognize))
    wav = str(make_wav())

    # When
    code = cli.main(["transcribe", wav, "--out", str(tmp_path / "s.json")])

    # Then
    assert code == 1
    assert _lines(capfd)[-1] == {"type": "error", "message": "boom"}

    # Given no optional group installed
    monkeypatch.setattr(pipeline, "_installed", lambda module: False)

    # When a beat tracker is forced, then left on auto
    forced = cli.main(["transcribe", wav, "--out", str(tmp_path / "s.json"), "--beats", "madmom"])
    forced_error = _lines(capfd)[-1]
    auto = cli.main(["transcribe", wav, "--out", str(tmp_path / "s.json")])
    auto_error = _lines(capfd)[-1]

    # Then the error names the group to install
    assert (forced, auto) == (1, 1)
    assert "beats-madmom" in forced_error["message"]
    assert "beats-allinone" in auto_error["message"] and "beats-madmom" in auto_error["message"]


def test_beats_per_bar_is_median_downbeat_spacing_or_4_without_two_downbeats():
    # Then
    assert pipeline.beats_per_bar(BEATS, BEATS[::3]) == 3
    assert pipeline.beats_per_bar(BEATS, BEATS[1::4] + [BEATS[3]]) == 4
    assert pipeline.beats_per_bar(BEATS, BEATS[:1]) == 4


@pytest.mark.slow
def test_transcribe_e2e_of_synthetic_clip_finds_8_bars_of_4_4_and_the_progression(make_wav, tmp_path):
    # Given the synthetic clip, with whatever models are installed
    out = tmp_path / "s.madsister.json"

    # When
    result = pipeline.transcribe(make_wav(), out, meter=None, has_sections=True)

    # Then
    song = json.loads(out.read_text())
    bars = [bar for s in song["sections"] for bar in s["bars"]]
    assert song["meta"]["meter"]["beats"] == 4
    assert abs(len(bars) - 8) <= 1
    first = [bar["chords"][0]["chord"] for bar in bars]
    assert sum(a == b for a, b in zip(first, PROGRESSION)) >= 6, first
    assert result.beats.beats and result.chords
