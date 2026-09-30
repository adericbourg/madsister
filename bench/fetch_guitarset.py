"""Download GuitarSet (CC BY 4.0) and build the proxy bench references from its Rock and Singer-Songwriter comp takes.

Run from the repo root: `uv run --project engine python bench/fetch_guitarset.py`. Files already present are skipped.
"""

import json
import re
import urllib.request
import zipfile
from pathlib import Path

from references import write_references

RECORD = "https://zenodo.org/api/records/3371780/files"  # GuitarSet 1.1.0
DATA = Path(__file__).parent / "data" / "guitarset"
SELECTED = re.compile(r"^\d+_(Rock|SS)\d+-.*_comp")  # <player>_<Style><n>-<bpm>-<key>_comp


def download(name: str) -> Path:
    path = DATA / name
    if not path.exists():
        print(f"downloading {name}")
        part = DATA / f"{name}.part"
        urllib.request.urlretrieve(f"{RECORD}/{name}/content", part)
        part.rename(path)
    return path


def main() -> None:
    refs, audio = DATA / "refs", DATA / "audio"
    refs.mkdir(parents=True, exist_ok=True)
    audio.mkdir(exist_ok=True)
    with zipfile.ZipFile(download("annotation.zip")) as archive:
        takes = [name for name in archive.namelist() if SELECTED.match(name)]
        for name in takes:
            write_references(Path(name).stem, json.loads(archive.read(name)), refs)
    with zipfile.ZipFile(download("audio_mono-mic.zip")) as archive:
        for name in archive.namelist():
            if SELECTED.match(name) and not (audio / name).exists():
                archive.extract(name, audio)  # <id>_mic.wav
    print(f"{len(takes)} takes -> {refs}")


if __name__ == "__main__":
    main()
