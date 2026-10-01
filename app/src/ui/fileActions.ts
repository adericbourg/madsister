// Tauri file access (spec F-ED-10), kept out of model/. Pure helpers are exported for tests.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { appConfigDir, join } from "@tauri-apps/api/path";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { exists, mkdir, readTextFile, writeFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { parseSong, serializeSong, type Song } from "../model/song";

// ponytail: "json" rather than "madsister.json", macOS dialogs don't handle multi-dot extensions.
const filters = [{ name: "madsister song", extensions: ["json"] }];

export const pushRecent = (list: readonly string[], path: string): string[] => [path, ...list.filter((p) => p !== path)].slice(0, 10);

export const confirmDiscard = (): Promise<boolean> => confirm("Discard unsaved changes?", { kind: "warning" });

export const confirmDeleteSection = (label: string): Promise<boolean> =>
  confirm(`Delete the section "${label}" and its chords?`, { kind: "warning" });

// The MIME type lets WKWebView (AVFoundation) play a blob; keep the extensions in sync with `read_audio` (lib.rs).
const AUDIO_TYPES: Record<string, string> = { mp3: "audio/mpeg", wav: "audio/wav", flac: "audio/flac", m4a: "audio/mp4", ogg: "audio/ogg" };
export const AUDIO_EXTENSIONS = Object.keys(AUDIO_TYPES);

export const pickAudioPath = async (): Promise<string | null> => open({ filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }] });

/** The file's sha256 (hashed in Rust: no fs scope needed for audio files), or null when it can't be read. */
export const audioSha256 = (path: string): Promise<string | null> => invoke("audio_sha256", { path });

/**
 * An object URL for playing the audio file in `<audio>` (revoke it when done). Read in Rust and played as a blob rather than
 * through the asset protocol: WebKitGTK's GStreamer can't read `asset://` URLs.
 */
export const audioObjectUrl = async (path: string): Promise<string> => {
  const bytes = await invoke<ArrayBuffer>("read_audio", { path });
  return URL.createObjectURL(new Blob([bytes], { type: AUDIO_TYPES[path.split(".").pop()!.toLowerCase()] }));
};

/** True when there's no file at `path` yet, or the user agrees to overwrite it. */
export const canWrite = async (path: string): Promise<boolean> =>
  !(await exists(path)) || confirm(`${path} already exists. Overwrite it?`, { kind: "warning" });

export const pickOpenPath = async (): Promise<string | null> => open({ filters });

export const pickSavePath = async (song: Song): Promise<string | null> =>
  save({ defaultPath: `${song.meta.title}.madsister.json`, filters });

/** Throws when the file can't be read, isn't JSON or isn't a valid song. */
export const readSong = async (path: string): Promise<Song> => parseSong(JSON.parse(await readTextFile(path)));

export const writeSong = (path: string, song: Song): Promise<void> => writeTextFile(path, serializeSong(song));

/** Asks where to export the song as `<title>.<extension>`, then writes `render(song)` there (text or bytes); does nothing on cancel. */
export const exportSong = async (
  song: Song,
  name: string,
  extension: string,
  render: (song: Song) => string | Uint8Array,
): Promise<void> => {
  const path = await save({ defaultPath: `${song.meta.title}.${extension}`, filters: [{ name, extensions: [extension] }] });
  if (path === null) return;
  const content = render(song);
  await (typeof content === "string" ? writeTextFile(path, content) : writeFile(path, content));
};

/** On macOS, Tauri replaces window.print with its webview print command (needs core:webview:allow-print). */
export const printSong = (): void => window.print();

export const setWindowTitle = (title: string): Promise<void> => getCurrentWindow().setTitle(title);

/** Reads `<appConfigDir>/<name>` as JSON; null when it's missing or unreadable (first run). */
export const readConfigJson = async (name: string): Promise<unknown> => {
  try {
    return JSON.parse(await readTextFile(await join(await appConfigDir(), name)));
  } catch {
    return null;
  }
};

export const writeConfigJson = async (name: string, value: unknown): Promise<void> => {
  const dir = await appConfigDir();
  await mkdir(dir, { recursive: true });
  await writeTextFile(await join(dir, name), JSON.stringify(value));
};
