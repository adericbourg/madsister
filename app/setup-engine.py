# Installs the packaged engine (M6-2), run by the app with the bundled uv's managed Python (and by the release workflow's
# clean-install check): `uv run --no-project --managed-python --python 3.11 python -I setup-engine.py <args>`.
# Usage: setup-engine.py <uv> <engine project dir> <env dir> <app version> [sections] [separate]
# Features installed on demand (large downloads): `sections` adds all-in-one (section detection, slow), `separate` adds
# Demucs (accurate mode). A sync is exact: the caller passes the features already installed too.
# Prints the engine's JSON Lines (one synthetic `install` progress, then `madsister-engine setup`'s); uv's output goes to
# stderr. Idempotent: an update re-runs it, uv and setup then only fetch what changed.
# Python rather than sh: Windows has no sh. Stdlib only, it runs before the env exists.
import glob
import os
import subprocess
import sys

GROUPS = {"sections": "beats-allinone", "separate": "separate"}

uv, project, env, version, *features = sys.argv[1:]
unknown = [f for f in features if f not in GROUPS]
if unknown:
    sys.exit(f"unknown feature: {unknown[0]}")
print('{"type":"progress","stage":"install","pct":0}', flush=True)
os.environ.update(UV_PROJECT_ENVIRONMENT=env, UV_PYTHON_PREFERENCE="only-managed")  # uv's CPython ships the headers source builds need
scripts = os.path.join(env, "Scripts" if os.name == "nt" else "bin")
exe = ".exe" if os.name == "nt" else ""


def run(*cmd):
    subprocess.run(cmd, stdout=sys.stderr, check=True)


# --no-editable: the project dir may move (an AppImage mounts at a new path on every launch).
sync = [uv, "sync", "--frozen", "--no-dev", "--no-editable", "--project", project,
        "--group", "beats-madmom", "--group", "chords-cnnlstm", "--group", "fetch", "--group", "record"]
for feature in GROUPS:  # uv sync is exact: it removes the features not passed
    if os.path.exists(marker := os.path.join(env, f".madsister-{feature}")):
        os.remove(marker)
for feature in features:
    sync += ["--group", GROUPS[feature]]
wheels = glob.glob(os.path.join(project, "wheels", "madmom-*.whl"))
if wheels:  # built by the release workflow: no git nor C compiler needed here
    run(*sync, "--no-install-package", "madmom")
    run(uv, "pip", "install", "--python", os.path.join(scripts, "python" + exe), "--no-deps", wheels[0])
else:  # local package builds: madmom builds from its git source (needs git and a C compiler)
    run(*sync)
subprocess.run([os.path.join(scripts, "madsister-engine" + exe), "setup", *(f"--{f}" for f in features)], check=True)
for feature in features:
    with open(os.path.join(env, f".madsister-{feature}"), "w") as f:
        f.write(version)
with open(os.path.join(env, ".madsister-version"), "w") as f:
    f.write(version)
