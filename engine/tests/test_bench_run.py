from run import edits_needed, is_add, is_slash, label_counts, timed_slots

BEATS = [i * 0.5 for i in range(8)]  # 2 bars of 4/4 at 120 BPM


def _song(*bars):
    """Song JSON with one bar per argument, each a list of (chord, beats), bars starting every 4 beats."""
    return {
        "sections": [
            {"bars": [{"startSec": BEATS[4 * i], "chords": [{"chord": c, "beats": n} for c, n in bar]} for i, bar in enumerate(bars)]}
        ]
    }


REF = _song([("C:maj", 4)], [("G:7", 2), ("A:min", 2)])


def test_timed_slots_of_song_uses_bar_start_and_beat_times():
    # Then the last slot ends one beat period after the last beat
    assert timed_slots(REF, BEATS) == [(0.0, 2.0, "C:maj"), (2.0, 3.0, "G:7"), (3.0, 4.0, "A:min")]


def test_edits_needed_counts_wrong_and_differently_split_reference_slots():
    # Given the reference slots
    ref = timed_slots(REF, BEATS)

    # When the prediction is identical, but tracked 40 ms late (jitter below half a beat)
    late = [b + 0.04 for b in BEATS]
    identical = edits_needed(ref, timed_slots(REF, late), BEATS)

    # And when a root is spelled enharmonically (B# for C)
    enharmonic = edits_needed(ref, timed_slots(_song([("B#:maj", 4)], [("G:7", 2), ("A:min", 2)]), BEATS), BEATS)

    # And when one chord is wrong
    one_wrong = edits_needed(ref, timed_slots(_song([("C:maj", 4)], [("G:7", 2), ("F:maj", 2)]), BEATS), BEATS)

    # And when C is split in two and G:7 / A:min merged into one G:7 slot
    split = edits_needed(ref, timed_slots(_song([("C:maj", 2), ("E:min", 2)], [("G:7", 4)]), BEATS), BEATS)

    # And when an inversion is missed (C:maj/5 predicted as C:maj), counted exactly and with the bass ignored
    inversion = timed_slots(_song([("C:maj/5", 4)], [("G:7", 2), ("A:min", 2)]), BEATS)
    missed, missed_bass_ignored = edits_needed(inversion, ref, BEATS), edits_needed(inversion, ref, BEATS, ignore_bass=True)

    # And when there is no predicted grid at all (the tracker found < 2 beats)
    no_grid = edits_needed(ref, [], BEATS)

    # Then 0 edits, 0, 1 edit, 2 edits (the split C slot, the A:min slot now G:7; G:7 itself is right), 1, 0 and all 3
    assert (identical, enharmonic, one_wrong, split, missed, missed_bass_ignored, no_grid) == (0, 0, 1, 2, 1, 0, 3)


def test_label_counts_of_prediction_counts_add_or_slash_slots_and_exact_matches():
    # Given a reference with C:add2 then A:minadd2 over E, and a prediction with that C:add2, a false G:add4 and A:min over E
    ref = timed_slots(_song([("C:add2", 4)], [("G:maj", 2), ("A:minadd2/5", 2)]), BEATS)
    est = timed_slots(_song([("C:add2", 4)], [("G:add4", 2), ("A:min/5", 2)]), BEATS)

    # When counting the add slots (minor adds included), then the slash chords
    adds = label_counts(ref, est, is_add)
    slashes = label_counts(ref, est, is_slash)

    # Then (predicted slots, of which right, reference slots, of which found): the slash chord is predicted with the wrong
    # quality, so it counts as neither right nor found
    assert adds == (2, 1, 2, 1)
    assert slashes == (1, 0, 1, 0)
