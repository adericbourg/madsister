import json
from pathlib import Path

from references import write_references

FIXTURE = Path(__file__).parent / "fixtures" / "guitarset_tiny.jams"


def test_write_references_of_guitarset_jams_writes_performed_chords_beats_and_song(tmp_path):
    # Given a 2-bar JAMS with instructed (C, G7) and performed (C power chord, G7 no 5th, G/D) chords
    jams = json.loads(FIXTURE.read_text())

    # When
    write_references("take", jams, tmp_path)

    # Then the .lab holds the performed chords, normalized to Harte
    assert (tmp_path / "take.lab").read_text().splitlines() == [
        "0.000000\t2.000000\tC:maj",
        "2.000000\t3.000000\tG:7",
        "3.000000\t4.000000\tG:maj/5",
    ]
    # And downbeats are the beats at position 1
    beats = json.loads((tmp_path / "take.beats.json").read_text())
    assert beats == {"beats": [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5], "downbeats": [0.0, 2.0], "meterBeats": 4}
    # And the reference Song is quantized per bar
    song = json.loads((tmp_path / "take.madsister.json").read_text())
    bars = song["sections"][0]["bars"]
    assert [[(slot["chord"], slot["beats"]) for slot in bar["chords"]] for bar in bars] == [
        [("C:maj", 4)],
        [("G:7", 2), ("G:maj/5", 2)],
    ]
