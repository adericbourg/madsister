"""Local cache of model weights and third-party model repos (never in the repo)."""

import hashlib
import io
import os
import shutil
import tarfile
import urllib.request
from pathlib import Path


def models_dir() -> Path:
    return Path(os.environ.get("MADSISTER_MODELS_DIR", Path.home() / ".cache/madsister/models"))


def ensure_repo(name: str, url: str, sha: str, sha256: str) -> Path:
    """Download GitHub repo `url` at `sha` (its archive: no git needed) into `models_dir()/name` unless it is already there.

    The archive must hash to `sha256`: its code is imported and its checkpoints unpickled."""
    path = models_dir() / name
    if not path.exists():
        # Extract next to the target and rename, so an interrupted download is never mistaken for a complete one.
        tmp = path.with_name(name + ".partial")
        shutil.rmtree(tmp, ignore_errors=True)
        with urllib.request.urlopen(f"{url}/archive/{sha}.tar.gz") as response:
            archive = response.read()
        if hashlib.sha256(archive).hexdigest() != sha256:
            raise RuntimeError(f"sha256 mismatch for {url}/archive/{sha}.tar.gz")
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar:
            tar.extractall(tmp, filter="data")
        (top,) = tmp.iterdir()  # GitHub archives hold one `<repo>-<sha>/` directory
        top.rename(path)
        tmp.rmdir()
    return path
