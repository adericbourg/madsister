import { useEffect, useState } from "react";
import { toChordPro } from "./model/export/chordpro";
import { toMidi } from "./model/export/midi";
import { toMusicXml } from "./model/export/musicxml";
import { emptySong, type Song, type SongMeter } from "./model/song";
import { Editor } from "./ui/Editor";
import { EngineSetup } from "./ui/EngineSetup";
import { Importer } from "./ui/Importer";
import {
  audioSha256,
  confirmDiscard,
  exportSong,
  isDirty,
  pickAudioPath,
  pickOpenPath,
  pickSavePath,
  printSong,
  pushRecent,
  readConfigJson,
  readSong,
  setWindowTitle,
  writeConfigJson,
  writeSong,
} from "./ui/fileActions";
import { parseSettings, Toolbar, type Settings } from "./ui/Toolbar";
import { useHistory } from "./ui/useHistory";

const RECENT = "recent.json";
const SETTINGS = "settings.json";
const METERS: Record<string, SongMeter> = { "3/4": { beats: 3, unit: 4 }, "4/4": { beats: 4, unit: 4 }, "6/8": { beats: 6, unit: 8 } };

function App() {
  const [initial] = useState(emptySong);
  const history = useHistory(initial);
  const { song } = history;
  const [savedSong, setSavedSong] = useState(initial);
  const [path, setPath] = useState<string | null>(null);
  const [recent, setRecent] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(() => parseSettings(null));
  // ponytail: the meter is only chosen for new songs; changing it on an existing song would invalidate every bar.
  const [newMeter, setNewMeter] = useState("4/4");
  const [audioNotice, setAudioNotice] = useState<string | null>(null);
  const isSongDirty = isDirty(song, savedSong);

  const fail = (e: unknown) => setError(e instanceof Error ? e.message : String(e));
  const updateRecent = (next: string[]) => {
    setRecent(next);
    writeConfigJson(RECENT, next).catch(fail);
  };
  const changeSettings = (next: Settings) => {
    setSettings(next);
    writeConfigJson(SETTINGS, next).catch(fail);
  };
  const load = (next: Song, nextPath: string | null) => {
    history.reset(next);
    setSavedSong(next);
    setPath(nextPath);
    setError(null);
  };
  const canDiscard = async () => !isSongDirty || (await confirmDiscard());

  const newSong = async () => {
    if (await canDiscard()) load(emptySong(METERS[newMeter]), null);
  };
  const openSong = async (from?: string) => {
    if (!(await canDiscard())) return;
    const p = from ?? (await pickOpenPath());
    if (p === null) return;
    try {
      load(await readSong(p), p);
      updateRecent(pushRecent(recent, p));
    } catch (e) {
      fail(e);
      if (from !== undefined) updateRecent(recent.filter((r) => r !== p));
    }
  };
  const saveTo = async (p: string) => {
    try {
      await writeSong(p, song);
      setSavedSong(song);
      setError(null);
      if (p !== path) {
        setPath(p);
        updateRecent(pushRecent(recent, p));
      }
    } catch (e) {
      fail(e);
    }
  };
  const saveAs = async () => {
    const p = await pickSavePath(song);
    if (p !== null) await saveTo(p);
  };
  const saveSong = () => (path === null ? saveAs() : saveTo(path));
  const locateAudio = async () => {
    const p = await pickAudioPath();
    if (p === null) return;
    const sha256 = await audioSha256(p);
    if (sha256 === null) fail(new Error(`Can't read ${p}`));
    else history.apply((s) => ({ ...s, audio: { ...s.audio, path: p, sha256 } }));
  };

  // Checked after the song is shown, and again whenever the reference changes (locate, undo).
  useEffect(() => {
    setAudioNotice(null);
    const audio = song.audio;
    if (audio === undefined) return;
    let isCurrent = true;
    void audioSha256(audio.path).then((sha256) => {
      if (!isCurrent || sha256 === audio.sha256) return;
      setAudioNotice(sha256 === null ? `Audio file not found: ${audio.path}` : `Audio file has changed since the transcription: ${audio.path}`);
    });
    return () => {
      isCurrent = false;
    };
  }, [song.audio]);

  useEffect(() => {
    readConfigJson(SETTINGS).then((json) => setSettings(parseSettings(json)));
    readConfigJson(RECENT).then((json) => Array.isArray(json) && setRecent(json.filter((p) => typeof p === "string")));
  }, []);

  useEffect(() => {
    void setWindowTitle(`${isSongDirty ? "• " : ""}${song.meta.title} — madsister`);
  }, [isSongDirty, song.meta.title]);

  // ponytail: songs without a path aren't autosaved (no temp/recovery file); add one if the user loses work.
  useEffect(() => {
    if (path === null || !isSongDirty) return;
    const timer = setTimeout(() => void saveTo(path), 2000);
    return () => clearTimeout(timer);
  });

  // Re-bound on every render so the handler sees the current song; the grid lets Mod+N/O/P/S bubble up.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      const action = { n: newSong, o: () => openSong(), p: printSong, s: e.shiftKey ? saveAs : saveSong }[e.key.toLowerCase()];
      if (action === undefined) return;
      e.preventDefault();
      void action();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  return (
    <main>
      <nav aria-label="File">
        <label>
          New song meter{" "}
          <select value={newMeter} onChange={(e) => setNewMeter(e.target.value)}>
            {Object.keys(METERS).map((m) => (
              <option key={m}>{m}</option>
            ))}
          </select>
        </label>
        <button type="button" onClick={newSong}>
          New
        </button>
        <button type="button" onClick={() => openSong()}>
          Open…
        </button>
        <button type="button" onClick={saveSong}>
          Save
        </button>
        <button type="button" onClick={saveAs}>
          Save as…
        </button>
        <button type="button" onClick={printSong}>
          Print…
        </button>
        <span role="group" aria-label="Export">
          Export{" "}
          <button type="button" onClick={() => exportSong(song, "ChordPro", "cho", toChordPro).catch(fail)}>
            ChordPro…
          </button>
          <button type="button" onClick={() => exportSong(song, "MusicXML", "musicxml", toMusicXml).catch(fail)}>
            MusicXML…
          </button>
          <button type="button" onClick={() => exportSong(song, "MIDI", "mid", toMidi).catch(fail)}>
            MIDI…
          </button>
        </span>
        {recent.length > 0 && (
          <details>
            <summary>Recent</summary>
            <ul>
              {recent.map((p) => (
                <li key={p}>
                  <button type="button" onClick={() => openSong(p)}>
                    {p}
                  </button>
                </li>
              ))}
            </ul>
          </details>
        )}
      </nav>
      <EngineSetup>
        <Importer onResult={(p) => void openSong(p)} />
      </EngineSetup>
      <Toolbar history={history} settings={settings} onSettingsChange={changeSettings} />
      {error !== null && <p role="alert">{error}</p>}
      {audioNotice !== null && (
        <p role="status">
          <span>{audioNotice}</span>{" "}
          <button type="button" onClick={locateAudio}>
            Locate audio…
          </button>
        </p>
      )}
      <Editor history={history} barsPerRow={settings.barsPerRow} style={settings.style} lowConfidenceThreshold={settings.lowConfidenceThreshold} />
    </main>
  );
}

export default App;
