import json

import pytest

from madsister_engine.song import Bar, ChordSlot, Meta, Meter, Section, Song, to_json, validate, write


def _song(bars=None):
    return Song(
        meta=Meta(title="Test", meter=Meter(4, 4), tempo_bpm=120),
        sections=[
            Section(id="a1", label="Verse", bars=bars or [Bar(chords=[ChordSlot("C:maj", 4, 0.9)], start_sec=0.0)]),
            Section(id="b2", label="Chorus", repeat=2, bars=[Bar(chords=[ChordSlot("F:maj", 2), ChordSlot("G:7", 2)])]),
        ],
    )


def test_to_json_of_two_sections_uses_camel_case_and_omits_absent_fields():
    # When
    data = to_json(_song())

    # Then
    assert data == {
        "version": 1,
        "meta": {"title": "Test", "tempoBpm": 120, "meter": {"beats": 4, "unit": 4}},
        "sections": [
            {"id": "a1", "label": "Verse", "bars": [{"startSec": 0.0, "chords": [{"chord": "C:maj", "beats": 4, "confidence": 0.9}]}]},
            {
                "id": "b2",
                "label": "Chorus",
                "repeat": 2,
                "bars": [{"chords": [{"chord": "F:maj", "beats": 2}, {"chord": "G:7", "beats": 2}]}],
            },
        ],
    }


def test_validate_when_beats_do_not_sum_to_meter_raises_with_location():
    # Given a 4/4 song whose first bar has 3 beats
    song = _song([Bar(chords=[ChordSlot("C:maj", 3)])])

    # When / Then
    with pytest.raises(ValueError, match=r"sections\[0\]\.bars\[0\]"):
        validate(song)


def test_validate_when_bar_meter_overrides_song_meter_accepts_it():
    # Given a 2/4 bar in a 4/4 song
    song = _song([Bar(chords=[ChordSlot("C:maj", 2)], meter=Meter(2, 4))])

    # When / Then (no exception)
    validate(song)


def test_write_replaces_the_file_with_validated_json_and_leaves_no_temporary_file(tmp_path):
    # Given a previous song file
    path = tmp_path / "s.madsister.json"
    path.write_text("old")

    # When
    write(_song(), path)

    # Then
    assert json.loads(path.read_text())["meta"]["title"] == "Test"
    assert [p.name for p in tmp_path.iterdir()] == ["s.madsister.json"]


@pytest.mark.parametrize(
    "song, where",
    [
        (Song(meta=Meta(title="T", meter=Meter(4, 3)), sections=[]), r"meta\.meter"),
        (_song([Bar(chords=[ChordSlot("C:maj", 4)], meter=Meter(4, 3))]), r"sections\[0\]\.bars\[0\]\.meter"),
        (_song([Bar(chords=[ChordSlot("C:maj", 4)], meter=Meter(0, 4))]), r"sections\[0\]\.bars\[0\]\.meter"),
    ],
)
def test_validate_when_meter_is_not_readable_by_the_app_raises_with_location(song, where):
    # When / Then
    with pytest.raises(ValueError, match=where):
        validate(song)
