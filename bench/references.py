"""GuitarSet JAMS -> bench references: chords as a mir_eval .lab, beats JSON, and the reference Song (spec §7).

JAMS is plain JSON, so no `jams` dependency.
"""

import json
from pathlib import Path

from madsister_engine.chords.labels import to_harte
from madsister_engine.quantize import ChordSegment, build_song
from madsister_engine.song import write


def _annotations(jams: dict, namespace: str) -> list[dict]:
    return [a for a in jams["annotations"] if a["namespace"] == namespace]


def write_references(ref_id: str, jams: dict, out_dir: Path) -> None:
    """Write `<ref_id>.lab`, `<ref_id>.beats.json` and `<ref_id>.madsister.json` into out_dir."""
    # GuitarSet stores the instructed chords first, then the performed ones
    # (data_source "Semi-automatic chord transcription with manual verification").
    performed = _annotations(jams, "chord")[-1]["data"]
    chords = [ChordSegment(o["time"], o["time"] + o["duration"], to_harte(o["value"])) for o in performed]
    positions = _annotations(jams, "beat_position")[0]["data"]
    beats = [o["time"] for o in positions]
    downbeats = [o["time"] for o in positions if o["value"]["position"] == 1]
    meter_beats = positions[0]["value"]["num_beats"]

    lab = "".join(f"{c.start:.6f}\t{c.end:.6f}\t{c.label}\n" for c in chords)
    (out_dir / f"{ref_id}.lab").write_text(lab)
    (out_dir / f"{ref_id}.beats.json").write_text(
        json.dumps({"beats": beats, "downbeats": downbeats, "meterBeats": meter_beats})
    )
    write(build_song(ref_id, chords, beats, downbeats, meter_beats, None), out_dir / f"{ref_id}.madsister.json")
