import { invoke } from "@tauri-apps/api/core";
import { toSheet } from "../model/export/sheet";
import { parseSong } from "../model/song";
import { readConfigJson } from "./fileActions";
import { mergeSettings, type Settings } from "./Toolbar";

/** What `madsister print …` asks for (print.rs `Plan`). */
export type PrintPlan = { songs: string[]; overrides: Record<string, unknown> };

const log = (line: string) => invoke("print_log", { line });
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Writes each song of the plan to its PDF, then quits (exit 1 if any failed). */
export const batchPrint = async (plan: PrintPlan): Promise<void> => {
  let settings: Settings;
  try {
    settings = mergeSettings(await readConfigJson("settings.json"), plan.overrides);
  } catch (e) {
    await log(`madsister print: ${message(e)}`);
    return invoke("print_finish", { failed: true });
  }
  let hasFailed = false;
  for (const [index, path] of plan.songs.entries()) {
    try {
      const song = parseSong(JSON.parse(await invoke<string>("print_read", { index })));
      await log(`${path} → ${await invoke<string>("print_pdf", { index, sheet: toSheet(song, settings) })}`);
    } catch (e) {
      hasFailed = true;
      await log(`${path}: ${message(e)}`);
    }
  }
  await invoke("print_finish", { failed: hasFailed });
};
