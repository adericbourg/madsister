# Installs the packaged engine (M6-2), run by the app with `sh` (and by the release workflow's clean-install check).
# Usage: setup-engine.sh <uv> <engine project dir> <env dir> <app version> [sections] [separate]
# Features installed on demand (large downloads): `sections` adds all-in-one (section detection, slow), `separate` adds
# Demucs (accurate mode). A sync is exact: the caller passes the features already installed too.
# Prints the engine's JSON Lines (one synthetic `install` progress, then `madsister-engine setup`'s); uv's output goes to
# stderr. Idempotent: an update re-runs it, uv and setup then only fetch what changed.
set -eu
uv=$1 project=$2 env=$3 version=$4
shift 4
features=$*
echo '{"type":"progress","stage":"install","pct":0}'
export UV_PROJECT_ENVIRONMENT="$env" UV_PYTHON_PREFERENCE=only-managed # uv's CPython ships the headers source builds need
# --no-editable: the project dir may move (an AppImage mounts at a new path on every launch).
set -- sync --frozen --no-dev --no-editable --project "$project" \
  --group beats-madmom --group chords-cnnlstm --group fetch --group record
rm -f "$env/.madsister-sections" "$env/.madsister-separate" # uv sync is exact: it removes the features not passed
setup_flags=
for feature in $features; do
  case $feature in
    sections) set -- "$@" --group beats-allinone ;;
    separate) set -- "$@" --group separate ;;
    *) echo "unknown feature: $feature" >&2; exit 2 ;;
  esac
  setup_flags="$setup_flags --$feature"
done
wheel=$(ls "$project"/wheels/madmom-*.whl 2>/dev/null || true)
if [ -n "$wheel" ]; then # built by the release workflow: no git nor C compiler needed here
  "$uv" "$@" --no-install-package madmom >&2
  "$uv" pip install --python "$env/bin/python" --no-deps "$wheel" >&2
else # local package builds: madmom builds from its git source (needs git and a C compiler)
  "$uv" "$@" >&2
fi
"$env/bin/madsister-engine" setup $setup_flags
for feature in $features; do printf '%s' "$version" >"$env/.madsister-$feature"; done
printf '%s' "$version" >"$env/.madsister-version"
