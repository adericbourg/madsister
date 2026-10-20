import hashlib
import json
import os
import sys
import types

import numpy as np
import pytest

from fixtures.make_clip import PROGRESSION
from madsister_engine import add_heuristic, bass, cli, pipeline
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
        return BeatResult(BEATS, BEATS[:: meter or 4], [(0.0, 8.0, "verse"), (8.0, 16.0, "chorus")])

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
    # And it references the original audio file, as its file name (the song is saved next to it)
    assert song["audio"] == {"path": wav.name, "sha256": hashlib.sha256(wav.read_bytes()).hexdigest()}

    # When sections are disabled and the meter forced
    code = cli.main(["transcribe", str(wav), "--out", str(out), "--no-sections", "--meter", "3"])

    # Then one "Song" section, and the meter reaches the tracker
    assert code == 0
    assert [s["label"] for s in json.loads(out.read_text())["sections"]] == ["Song"]
    assert fakes["track"][1] == 3

    # When 6/8 is forced
    code = cli.main(["transcribe", str(wav), "--out", str(out), "--meter", "6/8"])

    # Then the tracker gets 2 dotted-quarter beats per bar and the Song counts eighths
    song = json.loads(out.read_text())
    assert code == 0
    assert fakes["track"][1] == 2
    assert song["meta"]["meter"] == {"beats": 6, "unit": 8}
    assert {slot["beats"] for s in song["sections"] for bar in s["bars"] for slot in bar["chords"]} == {6}


def test_main_of_transcribe_accurate_with_add_tau_and_bass_tau_uses_the_stems(fakes, make_wav, tmp_path, capfd, monkeypatch):
    # Given a fake Demucs that copies its input into both stems, chroma with a strong D everywhere (C → C:add2), and a low
    # chroma with a strong E everywhere (C → C/3, and a bass on every other chord too)
    def stems(wav, out_dir):
        paths = out_dir / "harmonic.wav", out_dir / "bass.wav"
        for dst in paths:
            dst.write_bytes(wav.read_bytes())
        return paths

    monkeypatch.setitem(sys.modules, "madsister_engine.separate", types.SimpleNamespace(stems=stems))
    chroma = np.full((12, 400), 0.05)
    chroma[[0, 2, 4, 7]] = 1.0
    monkeypatch.setattr(add_heuristic, "chroma", lambda wav: (chroma, np.arange(400) * 0.05))
    low = np.full((12, 400), 0.05)
    low[4] = 1.0
    bass_inputs = []
    monkeypatch.setattr(bass, "chroma", lambda wav: (bass_inputs.append(wav), (low, np.arange(400) * 0.05))[1])
    out = tmp_path / "s.json"

    # When
    code = cli.main(["transcribe", str(make_wav()), "--out", str(out), "--mode", "accurate", "--add-tau", "0.5", "--bass-tau", "1.5"])

    # Then the harmonic stem feeds the chord model, the bass stem the bass detection, and the C bars become add2 over E
    assert code == 0
    assert ("separate", 20) in _stages(_lines(capfd))
    assert fakes["recognize"].name == "harmonic.wav"
    assert [p.name for p in bass_inputs] == ["bass.wav"]
    bars = [bar for s in json.loads(out.read_text())["sections"] for bar in s["bars"]]
    assert bars[0]["chords"][0]["chord"] == "C:add2/3"

    # When separating isn't asked
    code = cli.main(["transcribe", str(make_wav()), "--out", str(out), "--bass-tau", "1.5"])

    # Then the bass detection reads the mix
    assert code == 0
    assert bass_inputs[-1].name == "decoded.wav"

    # When the accurate mode is asked instead of --separate
    code = cli.main(["transcribe", str(make_wav()), "--out", str(out), "--mode", "accurate"])

    # Then it separates too
    assert code == 0
    assert ("separate", 20) in _stages(_lines(capfd))


def test_main_when_adapter_fails_finds_no_beats_or_group_missing_emits_error_and_exits_1(fakes, make_wav, tmp_path, capfd, monkeypatch):
    # Given a tracker that finds a single beat (all-in-one on one GuitarSet take, m0-results)
    monkeypatch.setitem(
        sys.modules, "madsister_engine.beats.madmom_tracker", types.SimpleNamespace(track=lambda wav, meter: BeatResult([1.0], [1.0]))
    )

    # When
    code = cli.main(["transcribe", str(make_wav()), "--out", str(tmp_path / "s.json")])

    # Then
    assert code == 1
    assert _lines(capfd)[-1] == {"type": "error", "message": "could not find beats in this audio"}

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


def test_to_song_of_beats_without_a_downbeat_fails_with_a_message():
    # Given a short clip whose beats never reach position 1, with and without sections
    beats = BeatResult([0.5, 1.0, 1.5], [], [(0.0, 2.0, "intro")])
    segments = [ChordSegment(0.0, 2.0, "C:maj")]

    # When / Then
    for has_sections in (False, True):
        with pytest.raises(ValueError, match="could not find bars in this audio"):
            pipeline.to_song("clip", beats, segments, None, has_sections)


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
    pipeline.transcribe(make_wav(), out, meter=None, has_sections=True)

    # Then
    song = json.loads(out.read_text())
    bars = [bar for s in song["sections"] for bar in s["bars"]]
    assert song["meta"]["meter"]["beats"] == 4
    assert abs(len(bars) - 8) <= 1
    first = [bar["chords"][0]["chord"] for bar in bars]
    assert sum(a == b for a, b in zip(first, PROGRESSION)) >= 6, first
