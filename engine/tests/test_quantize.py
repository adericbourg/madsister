from madsister_engine.quantize import (
    ChordSegment,
    bars_from_beats,
    beat_labels,
    build_song,
    sections_from_segments,
    simplify,
)
from madsister_engine.song import Bar, ChordSlot, Meter

BEATS_8 = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]  # 120 BPM, two 4/4 bars


def _chords(bar):
    return [(s.chord, s.beats) for s in bar.chords]


def test_beat_labels_when_short_spurious_label_uses_majority_by_duration():
    # Given C over the whole beat except a 0.1 s blip of G
    chords = [
        ChordSegment(0.0, 0.2, "C:maj", 0.8),
        ChordSegment(0.2, 0.3, "G:maj", 0.4),
        ChordSegment(0.3, 1.0, "C:maj", 0.6),
    ]

    # When
    labels = beat_labels(chords, [0.0, 0.5])

    # Then the blip loses; confidence is the duration-weighted mean of the C segments
    assert [label for label, _ in labels] == ["C:maj", "C:maj"]
    assert abs(labels[0][1] - (0.8 * 0.2 + 0.6 * 0.2) / 0.4) < 1e-9
    assert labels[1] == ("C:maj", 0.6)


def test_bars_from_beats_when_chord_changes_mid_bar_splits_slot():
    # Given C for 2 beats then G for 6
    labels = [("C:maj", 0.9)] * 2 + [("G:maj", 0.5)] * 6

    # When
    bars = bars_from_beats(labels, BEATS_8, downbeats=[0.0, 2.0], meter_beats=4)

    # Then
    assert [_chords(b) for b in bars] == [[("C:maj", 2), ("G:maj", 2)], [("G:maj", 4)]]
    assert [b.start_sec for b in bars] == [0.0, 2.0]
    assert bars[0].chords[0].confidence == 0.9
    assert all(b.meter is None for b in bars)


def test_bars_from_beats_of_pickup_drops_one_beat_and_keeps_two():
    # Given 1 beat, then 2 beats, before the first downbeat
    labels = [("C:maj", None)] * 6

    # When
    one = bars_from_beats(labels[:5], BEATS_8[:5], downbeats=[0.5], meter_beats=4)
    two = bars_from_beats(labels, BEATS_8[:6], downbeats=[1.0], meter_beats=4)

    # Then
    assert [(b.start_sec, b.meter) for b in one] == [(0.5, None)]
    assert [(b.start_sec, b.meter) for b in two] == [(0.0, Meter(2, 4)), (1.0, None)]


def test_bars_from_beats_of_three_four_produces_three_beat_bars():
    # When
    bars = bars_from_beats([("A:min", None)] * 6, [i * 0.5 for i in range(6)], downbeats=[0.0, 1.5], meter_beats=3)

    # Then
    assert [_chords(b) for b in bars] == [[("A:min", 3)], [("A:min", 3)]]


def test_simplify_absorbs_no_chord_slots_and_merges_equal_neighbours():
    # Given N inside a bar, N at the start of a bar, and an all-N bar
    bars = [
        Bar([ChordSlot("C:maj", 1), ChordSlot("N", 2), ChordSlot("C:maj", 1)]),
        Bar([ChordSlot("N", 1), ChordSlot("G:maj", 3, 0.7)]),
        Bar([ChordSlot("N", 4)]),
    ]

    # When
    result = simplify(bars)

    # Then
    assert [_chords(b) for b in result] == [[("C:maj", 4)], [("G:maj", 4)], [("N", 4)]]
    assert result[1].chords[0].confidence == 0.7


def test_sections_from_segments_snaps_to_nearest_bar_and_maps_labels():
    # Given 4 bars of 2 s and segments starting slightly after downbeats
    bars = [Bar([ChordSlot("C:maj", 4)], start_sec=2.0 * i) for i in range(4)]
    segments = [(0.0, 4.3, "intro"), (4.3, 6.0, "chorus"), (6.0, 6.1, "chorus"), (6.1, 8.0, "weird")]

    # When
    sections = sections_from_segments(bars, segments)

    # Then zero-bar sections are dropped
    assert [(s.label, len(s.bars)) for s in sections] == [("Intro", 2), ("Chorus", 1), ("Weird", 1)]


def test_sections_from_segments_without_segments_is_one_song_section():
    # When
    sections = sections_from_segments([Bar([ChordSlot("C:maj", 4)])], None)

    # Then
    assert [(s.label, len(s.bars)) for s in sections] == [("Song", 1)]


def test_build_song_composes_the_pipeline():
    # Given
    chords = [ChordSegment(0.0, 2.0, "C:maj", 0.9), ChordSegment(2.0, 4.0, "N", 0.9)]

    # When
    song = build_song("my song", chords, BEATS_8, [0.0, 2.0], meter_beats=4, segments=None)

    # Then
    assert song.meta.title == "my song"
    assert song.meta.tempo_bpm == 120
    assert [_chords(b) for b in song.sections[0].bars] == [[("C:maj", 4)], [("N", 4)]]


def test_build_song_of_compound_unit_writes_tracker_beats_as_three_eighths():
    # Given 6/8 tracked as 2 dotted-quarter beats per bar, with a 4-beat bar at the end
    chords = [ChordSegment(0.0, 2.0, "C:maj", 0.9), ChordSegment(2.0, 4.0, "G:maj", 0.9)]

    # When
    song = build_song("jig", chords, BEATS_8, [0.0, 1.0, 2.0], meter_beats=2, segments=None, unit=8)

    # Then the song is 6/8, slots count eighths and the long bar keeps an override in eighths
    bars = song.sections[0].bars
    assert song.meta.meter == Meter(6, 8)
    assert [_chords(b) for b in bars] == [[("C:maj", 6)], [("C:maj", 6)], [("G:maj", 12)]]
    assert [b.meter for b in bars] == [None, None, Meter(12, 8)]
