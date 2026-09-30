"""Local cache of model weights and third-party model repos (never in the repo)."""

import os
import shutil
import subprocess
from pathlib import Path


def models_dir() -> Path:
    return Path(os.environ.get("MADSISTER_MODELS_DIR", Path.home() / ".cache/madsister/models"))


def ensure_repo(name: str, url: str, sha: str) -> Path:
    """Clone `url` at `sha` into `models_dir()/name` unless it is already there."""
    path = models_dir() / name
    if not path.exists():
        # Clone next to the target and rename, so an interrupted clone is never mistaken for a complete one.
        tmp = path.with_name(name + ".partial")
        shutil.rmtree(tmp, ignore_errors=True)
        subprocess.run(["git", "clone", "--quiet", url, str(tmp)], check=True)
        subprocess.run(["git", "-C", str(tmp), "checkout", "--quiet", sha], check=True)
        tmp.rename(path)
    return path
