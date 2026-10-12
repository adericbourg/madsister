import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { parseSong, type Song } from "../model/song";
import { Editor } from "./Editor";
import { readConfigJson } from "./fileActions";
import { mergeSettings, type Settings } from "./Toolbar";
import { useHistory } from "./useHistory";

/** What `madsister print …` asks for (print.rs `Plan`). */
export type PrintPlan = { songs: string[]; overrides: Record<string, unknown> };

const log = (line: string) => invoke("print_log", { line });
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

const Chart = ({ song, settings }: { song: Song; settings: Settings }) => {
  const history = useHistory(song);
  return (
    <main className={[settings.compactPrint && "compact-print", `font-${settings.font}`].filter(Boolean).join(" ")}>
      <Editor
        history={history}
        mode="view"
        barsPerRow={settings.barsPerRow}
        compactPrint={settings.compactPrint}
        style={settings.style}
        minorConvention={settings.minorConvention}
        lowConfidenceThreshold={settings.lowConfidenceThreshold}
      />
    </main>
  );
};

/** Renders each song of the plan in the hidden window and prints it to PDF, then quits (exit 1 if any failed). */
export const BatchPrint = ({ plan }: { plan: PrintPlan }) => {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [job, setJob] = useState<{ index: number; song: Song } | null>(null);
  const hasFailed = useRef(false);

  const fail = (index: number, e: unknown) => {
    hasFailed.current = true;
    return log(`${plan.songs[index]}: ${message(e)}`);
  };
  // Loads the next readable song from `index`, or quits.
  const load = async (index: number) => {
    for (let i = index; i < plan.songs.length; i++) {
      try {
        return setJob({ index: i, song: parseSong(JSON.parse(await invoke<string>("print_read", { index: i }))) });
      } catch (e) {
        await fail(i, e);
      }
    }
    await invoke("print_finish", { failed: hasFailed.current });
  };

  useEffect(() => {
    readConfigJson("settings.json")
      .then((saved) => {
        setSettings(mergeSettings(saved, plan.overrides));
        return load(0);
      })
      .catch(async (e) => {
        await log(`madsister print: ${message(e)}`);
        await invoke("print_finish", { failed: true });
      });
  }, []);

  useEffect(() => {
    if (job === null) return;
    (async () => {
      // Fonts load once text using them is laid out: force the layout (a hidden window never runs requestAnimationFrame).
      document.body.getBoundingClientRect();
      await document.fonts.ready;
      try {
        await log(`${plan.songs[job.index]} → ${await invoke<string>("print_pdf", { index: job.index })}`);
      } catch (e) {
        await fail(job.index, e);
      }
      await load(job.index + 1);
    })();
  }, [job]);

  return settings && job && <Chart key={job.index} song={job.song} settings={settings} />;
};
