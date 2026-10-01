import { useEffect, useRef, useState } from "react";
import { audioObjectUrl } from "./fileActions";

/**
 * Plays the song's audio (spec F-PB-1); spread `audioProps` on an `<audio>`. While playing, `time` follows the audio through a
 * requestAnimationFrame loop, rounded to 0.1 s: enough for a bar cursor without re-rendering the grid on every frame.
 */
export const usePlayer = (path: string | undefined) => {
  const ref = useRef<HTMLAudioElement>(null);
  const [src, setSrc] = useState<string>();
  const [isPlaying, setIsPlaying] = useState(false);
  const [time, setTime] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const fail = (e: unknown) => setError(`Can't play this audio: ${String(e)}`);

  useEffect(() => {
    setSrc(undefined);
    setError(null);
    setTime(0);
    if (path === undefined) return;
    let url: string | undefined;
    let isCurrent = true;
    audioObjectUrl(path).then((u) => (isCurrent ? setSrc((url = u)) : URL.revokeObjectURL(u)), fail);
    return () => {
      isCurrent = false;
      ref.current?.pause();
      if (url !== undefined) URL.revokeObjectURL(url);
    };
  }, [path]);

  useEffect(() => {
    if (!isPlaying) return;
    let frame = requestAnimationFrame(function tick() {
      setTime(Math.floor(ref.current!.currentTime * 10) / 10);
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

  const audioProps = {
    ref,
    src,
    onPlay: () => setIsPlaying(true),
    onPause: () => setIsPlaying(false),
    onError: () => fail(ref.current?.error?.message || "unsupported format"),
  };
  return { audioProps, isReady: src !== undefined, isPlaying, time, error, toggle };
};
