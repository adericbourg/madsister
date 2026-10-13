import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { toChordPro } from "./model/export/chordpro";
import { toMidi } from "./model/export/midi";
import { toMusicXml } from "./model/export/musicxml";
import { emptySong, type Notation, type Song } from "./model/song";
import { Editor } from "./ui/Editor";
import { EngineSetup } from "./ui/EngineSetup";
import { Importer } from "./ui/Importer";
import {
  audioSha256,
  confirmDiscard,
  exportSong,
  pickAudioPath,
  pickOpenPath,
  pickSavePath,
  printSong,
  pushRecent,
  readConfigJson,
  readSong,
  relativeAudioPath,
  resolveAudioPath,
  setWindowTitle,
  writeConfigJson,
  writeSong,
} from "./ui/fileActions";
import { parseSettings, Toolbar, type Settings } from "./ui/Toolbar";
import { About } from "./ui/About";
import { Preferences } from "./ui/Preferences";
import { IS_MAC } from "./ui/keymap";
import { menuItems, RECENT_PREFIX, type MenuState } from "./ui/menu";
import { Ribbon } from "./ui/Ribbon";
import { Sidebar } from "./ui/Sidebar";
import { useHistory } from "./ui/useHistory";

const RECENT = "recent.json";
const SETTINGS = "settings.json";
const BEATS_OPTIONS = Array.from({ length: 16 }, (_, i) => i + 1);
const UNIT_OPTIONS = [2, 4, 8] as const;

function App() {
  const [initial] = useState(emptySong);
  const history = useHistory(initial);
  const { song } = history;
  const [savedSong, setSavedSong] = useState(initial);
  const [path, setPath] = useState<string | null>(null);
  const [hasSong, setHasSong] = useState(false);
  const [mode, setMode] = useState<"view" | "edit">("view");
  const [panel, setPanel] = useState<HTMLElement | null>(null);
  const [recent, setRecent] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(() => parseSettings(null));
  // ponytail: the meter is only chosen for new songs; changing it on an existing song would invalidate every bar.
  const [newBeats, setNewBeats] = useState(4);
  const [newUnit, setNewUnit] = useState<2 | 4 | 8>(4);
  const [newNotation, setNewNotation] = useState<Notation>("chords");
  const [audioNotice, setAudioNotice] = useState<string | null>(null);
  const preferences = useRef<HTMLDialogElement>(null);
  const about = useRef<HTMLDialogElement>(null);
  const isSongDirty = song !== savedSong;
  // The exports write real notes, which a key-less Nashville song (chords relative to C) doesn't have.
  const isExportLocked = song.meta.notation === "nashville" && song.meta.key === undefined;

  const fail = (e: unknown) => setError(e instanceof Error ? e.message : String(e));
  const updateRecent = (next: string[]) => {
    setRecent(next);
    writeConfigJson(RECENT, next).catch(fail);
  };
  const changeSettings = (next: Settings) => {
    setSettings(next);
    writeConfigJson(SETTINGS, next).catch(fail);
  };
  const load = (next: Song, nextPath: string | null, nextMode: "view" | "edit" = "view") => {
    history.reset(next);
    setMode(nextMode);
    setSavedSong(next);
    setPath(nextPath);
    setHasSong(true);
    setError(null);
  };
  const toggleMode = () => setMode((m) => (m === "edit" ? "view" : "edit"));
  const canDiscard = async () => !isSongDirty || (await confirmDiscard());

  const createGrid = () => load(emptySong({ beats: newBeats, unit: newUnit }, newNotation === "nashville" ? "nashville" : undefined), null, "edit");
  // Back to the start screen; the song is reset so that it isn't dirty or autosaved behind it.
  const newSong = async () => {
    if (!(await canDiscard())) return;
    load(emptySong(), null);
    setHasSong(false);
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
    else history.apply((s) => ({ ...s, audio: { ...s.audio, path: relativeAudioPath(path, p), sha256 } }));
  };

  // Checked after the song is shown, and again whenever the reference changes (locate, undo).
  useEffect(() => {
    setAudioNotice(null);
    const audio = song.audio;
    if (audio === undefined) return;
    let isCurrent = true;
    const audioPath = resolveAudioPath(path, audio.path);
    void audioSha256(audioPath).then((sha256) => {
      if (!isCurrent || sha256 === audio.sha256) return;
      setAudioNotice(sha256 === null ? `Audio file not found next to the song: ${audioPath}. The song can't be played until you locate it.` : `Audio file has changed since the transcription: ${audioPath}`);
    });
    return () => {
      isCurrent = false;
    };
  }, [song.audio, path]);

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

  const runMenu = (id: string) => {
    if (id.startsWith(RECENT_PREFIX)) return void openSong(id.slice(RECENT_PREFIX.length));
    const isInText = document.activeElement?.matches("input, textarea") ?? false;
    const actions: Record<string, () => unknown> = {
      new: newSong,
      open: () => openSong(),
      "clear-recent": () => updateRecent([]),
      close: newSong,
      save: saveSong,
      "save-as": saveAs,
      chordpro: () => exportSong(song, "ChordPro", "cho", toChordPro).catch(fail),
      musicxml: () => exportSong(song, "MusicXML", "musicxml", toMusicXml).catch(fail),
      midi: () => exportSong(song, "MIDI", "mid", toMidi).catch(fail),
      print: printSong,
      // A text field undoes its own typing; otherwise the song history, which is read-only in view mode like the grid.
      undo: () => (isInText ? document.execCommand("undo") : mode === "edit" && history.undo()),
      redo: () => (isInText ? document.execCommand("redo") : mode === "edit" && history.redo()),
      "edit-mode": toggleMode,
      sidebar: () => changeSettings({ ...settings, isSidebarCollapsed: !settings.isSidebarCollapsed }),
      preferences: () => preferences.current?.showModal(),
      about: () => about.current?.showModal(),
      quit: () => getCurrentWindow().close(),
    };
    void actions[id]();
  };
  // The menu calls the current render's actions; it is rebuilt only when what it shows changes.
  const runMenuRef = useRef(runMenu);
  runMenuRef.current = runMenu;
  const menuState: MenuState = { isMac: IS_MAC, hasSong, isExportLocked, isEditing: mode === "edit", isSidebarShown: !settings.isSidebarCollapsed, recent };
  const menuKey = JSON.stringify(menuState);
  useEffect(() => {
    let isCurrent = true;
    void Menu.new({ items: menuItems(menuState, (id) => runMenuRef.current(id)) }).then((menu) => {
      if (isCurrent) void menu.setAsAppMenu();
    });
    return () => {
      isCurrent = false;
    };
  }, [menuKey]);

  return (
    <main className={[settings.compactPrint && "compact-print", `font-${settings.font}`].filter(Boolean).join(" ")}>
      <Ribbon
        hasSong={hasSong}
        isEditing={mode === "edit"}
        onNew={newSong}
        onOpen={() => openSong()}
        onSave={saveSong}
        onPrint={printSong}
        onToggleMode={toggleMode}
        onPreferences={() => preferences.current?.showModal()}
      />
      <Preferences ref={preferences} settings={settings} onSettingsChange={changeSettings} />
      <About ref={about} />
      {error !== null && <p role="alert">{error}</p>}
      {hasSong ? (
        <div className="workspace">
          <div className="content">
            {audioNotice !== null && (
              <p role="status" className="audio-notice">
                <span>{audioNotice}</span>{" "}
                <button type="button" onClick={locateAudio}>
                  Locate audio…
                </button>
              </p>
            )}
            {panel && (
              <Editor history={history} mode={mode} barsPerRow={settings.barsPerRow} compactPrint={settings.compactPrint} style={settings.style} minorConvention={settings.minorConvention} lowConfidenceThreshold={settings.lowConfidenceThreshold} panel={panel} audioPath={song.audio && resolveAudioPath(path, song.audio.path)} onLocateAudio={locateAudio} />
            )}
          </div>
          <Sidebar isCollapsed={settings.isSidebarCollapsed} isEditing={mode === "edit"} onCollapsedChange={(isSidebarCollapsed) => changeSettings({ ...settings, isSidebarCollapsed })} panelsRef={setPanel}>
            <Toolbar history={history} settings={settings} />
          </Sidebar>
        </div>
      ) : (
        <div className="start">
          <section aria-labelledby="new-grid-title" className="new-grid">
            <h2 id="new-grid-title">New grid</h2>
            <label htmlFor="new-beats">Beats per bar</label>
            <select id="new-beats" value={newBeats} onChange={(e) => setNewBeats(Number(e.target.value))}>
              {BEATS_OPTIONS.map((n) => (
                <option key={n}>{n}</option>
              ))}
            </select>
            <label htmlFor="new-unit">Beat unit</label>
            <select id="new-unit" value={newUnit} onChange={(e) => setNewUnit(Number(e.target.value) as 2 | 4 | 8)}>
              {UNIT_OPTIONS.map((n) => (
                <option key={n}>{n}</option>
              ))}
            </select>
            <label htmlFor="new-notation">Notation</label>
            <select id="new-notation" value={newNotation} onChange={(e) => setNewNotation(e.target.value as Notation)}>
              <option value="chords">Chords</option>
              <option value="nashville">Nashville numbers</option>
            </select>
            <button type="button" className="primary" onClick={createGrid}>
              Create grid
            </button>
          </section>
          <section aria-labelledby="import-title">
            <h2 id="import-title">Import audio</h2>
            <EngineSetup>
              <Importer onResult={(p) => void openSong(p)} />
            </EngineSetup>
          </section>
        </div>
      )}
    </main>
  );
}

export default App;
