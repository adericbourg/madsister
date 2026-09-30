// Tauri file access (spec F-ED-10), kept out of model/. Pure helpers are exported for tests.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { appConfigDir, join } from "@tauri-apps/api/path";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { mkdir, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { parseSong, serializeSong, type Song } from "../model/song";

// ponytail: "json" rather than "madsister.json", macOS dialogs don't handle multi-dot extensions.
const filters = [{ name: "madsister song", extensions: ["json"] }];

export const pushRecent = (list: readonly string[], path: string): string[] => [path, ...list.filter((p) => p !== path)].slice(0, 10);

export const isDirty = (song: Song, savedSong: Song): boolean => song !== savedSong;

export const confirmDiscard = (): Promise<boolean> => confirm("Discard unsaved changes?", { kind: "warning" });

export const pickOpenPath = async (): Promise<string | null> => open({ filters });

export const pickSavePath = async (song: Song): Promise<string | null> =>
  save({ defaultPath: `${song.meta.title}.madsister.json`, filters });

/** Throws when the file can't be read, isn't JSON or isn't a valid song. */
export const readSong = async (path: string): Promise<Song> => parseSong(JSON.parse(await readTextFile(path)));

export const writeSong = (path: string, song: Song): Promise<void> => writeTextFile(path, serializeSong(song));

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
