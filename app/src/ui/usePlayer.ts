import { useEffect, useRef, useState } from "react";
import { audioObjectUrl } from "./fileActions";

/**
 * Plays the song's audio (spec F-PB-1); spread `audioProps` on an `<audio>`. While playing, `time` follows the audio through a
 * requestAnimationFrame loop, rounded up to 0.1 s: enough for a bar cursor without re-rendering the grid on every frame.
 */
export const usePlayer = (path: string | undefined) => {
  const ref = useRef<HTMLAudioElement>(null);
  const [src, setSrc] = useState<string>();
  const [isPlaying, setIsPlaying] = useState(false);
  const [time, setTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const fail = (e: unknown) => setError(`Can't play this audio: ${String(e)}`);

  useEffect(() => {
    setSrc(undefined);
    setError(null);
    setTime(0);
    setDuration(0);
    if (path === undefined) return;
    const audio = ref.current; // React has already nulled the ref when this cleanup runs after the <audio> unmounted
    let url: string | undefined;
    let isCurrent = true;
    audioObjectUrl(path).then((u) => (isCurrent ? setSrc((url = u)) : URL.revokeObjectURL(u)), fail);
    return () => {
      isCurrent = false;
      if (audio !== null) {
        // The same <audio> plays the next song: unload the blob before revoking it. WebKit reads it lazily, and a revoked blob
        // still in use fails the element's next load at random ("unsupported format", no duration). Removing `src` alone doesn't unload.
        audio.pause();
        audio.removeAttribute("src");
        audio.load();
      }
      if (url !== undefined) URL.revokeObjectURL(url);
    };
  }, [path]);

  useEffect(() => {
    if (!isPlaying) return;
    let frame = requestAnimationFrame(function tick() {
      setTime(Math.ceil(ref.current!.currentTime * 10) / 10);
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  }, [isPlaying]);

  /** False when there's nothing to play (no audio, or not loaded yet). */
  const toggle = (): boolean => {
    const audio = ref.current;
    if (audio === null || src === undefined) return false;
    setError(null);
    if (isPlaying) audio.pause();
    else audio.play().catch(fail);
    return true;
  };

  /** Moves the playback position (and the bar cursor, even while paused). False when there's nothing to play. */
  const seek = (sec: number): boolean => {
    const audio = ref.current;
    if (audio === null || src === undefined) return false;
    audio.currentTime = sec;
    setTime(Math.ceil(sec * 10) / 10);
    return true;
  };

  // WebKitGTK can report the metadata with a 0 duration and set the real one moments later without another durationchange:
  // retry for up to 2 s while the element has metadata.
  const syncDuration = (retries = 40) => {
    const audio = ref.current;
    if (audio === null) return;
    if (audio.duration > 0) setDuration(audio.duration);
    else if (retries > 0 && audio.readyState > HTMLMediaElement.HAVE_NOTHING) setTimeout(() => syncDuration(retries - 1), 50);
  };
  const audioProps = {
    ref,
    src,
    onPlay: () => setIsPlaying(true),
    onDurationChange: () => syncDuration(),
    onPause: () => setIsPlaying(false),
    onError: () => fail(ref.current?.error?.message || "unsupported format"),
  };
  return { audioProps, isReady: src !== undefined, isPlaying, time, duration, error, toggle, seek };
};
