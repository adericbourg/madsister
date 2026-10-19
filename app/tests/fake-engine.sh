# Fake madsister-engine for tests/engine.rs: $1 picks the scenario.
case "$1" in
success)
  echo '{"type":"progress","stage":"decode","pct":0}'
  echo '{"type":"progress","stage":"chords","pct":50}'
  echo '{"type":"result","path":"/tmp/song.json"}' ;;
error)
  echo 'not json'
  echo 'Traceback: boom' >&2
  echo '{"type":"error","message":"boom"}'
  exit 1 ;;
crash)
  exit 3 ;;
hang)
  # A child, like ffmpeg or Demucs: cancel must kill it too. Its pid goes in the stage.
  sleep 60 &
  echo "{\"type\":\"progress\",\"stage\":\"$!\",\"pct\":0}"
  wait ;;
record)
  # Like `record`, stops on a `stop` line (other lines ignored); unlike it, stdin EOF is a failure, so tests see the stop.
  echo '{"type":"progress","stage":"record","pct":0,"elapsedSec":0}'
  while read -r line; do
    [ "$line" = stop ] && echo '{"type":"result","path":"/tmp/take.wav"}' && exit 0
  done
  exit 4 ;;
esac
