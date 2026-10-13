import { useEffect, useId, useImperativeHandle, useState, type Ref } from "react";
import { toChordPro } from "../model/export/chordpro";
import { toMidi } from "../model/export/midi";
import { toMusicXml } from "../model/export/musicxml";
import { emptySong, type Notation, type Song } from "../model/song";
import { Editor } from "./Editor";
import { EngineSetup } from "./EngineSetup";
import { audioSha256, exportSong, pickAudioPath, pickSavePath, readSong, relativeAudioPath, resolveAudioPath, writeSong } from "./fileActions";
import { Importer } from "./Importer";
import { Sidebar } from "./Sidebar";
import { Toolbar, type Settings } from "./Toolbar";
import { useHistory } from "./useHistory";

const BEATS_OPTIONS = Array.from({ length: 16 }, (_, i) => i + 1);
const UNIT_OPTIONS = [2, 4, 8] as const;
const EXPORTS = {
  chordpro: ["ChordPro", "cho", toChordPro],
  musicxml: ["MusicXML", "musicxml", toMusicXml],
  midi: ["MIDI", "mid", toMidi],
} as const;

/** What the app shows of a tab: its tab label, the window title, the menu's enabled items. */
export type TabStatus = { title: string; path: string | null; hasSong: boolean; isDirty: boolean; isEditing: boolean; isExportLocked: boolean; isIdle: boolean };

/** What the menu and the ribbon do to the active tab. */
export type TabHandle = {
  load: (song: Song, path: string) => void;
  save: () => Promise<void>;
  saveAs: () => Promise<void>;
  exportAs: (format: keyof typeof EXPORTS) => void;
  undo: () => void;
  redo: () => void;
  toggleMode: () => void;
};

type Props = {
  ref: Ref<TabHandle>;
  isActive: boolean;
  /** The song this tab opens with; none for the start screen (new grid, import). */
  initial: { song: Song; path: string } | null;
  settings: Settings;
  onSettingsChange: (settings: Settings) => void;
  onStatus: (status: TabStatus) => void;
  /** A file was written or imported at this path: it goes to the recent files. */
  onFile: (path: string) => void;
};

/** One open song (F-UI-7), or the start screen until a grid is created or imported. */
export const SongTab = ({ ref, isActive, initial, settings, onSettingsChange, onStatus, onFile }: Props) => {
  const [first] = useState(() => initial?.song ?? emptySong());
  const history = useHistory(first);
  const { song } = history;
  const [savedSong, setSavedSong] = useState(first);
  const [path, setPath] = useState<string | null>(initial?.path ?? null);
  const [hasSong, setHasSong] = useState(initial !== null);
  const [mode, setMode] = useState<"view" | "edit">("view");
  const [panel, setPanel] = useState<HTMLElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  // ponytail: the meter is only chosen for new songs; changing it on an existing song would invalidate every bar.
  const [newBeats, setNewBeats] = useState(4);
  const [newUnit, setNewUnit] = useState<2 | 4 | 8>(4);
  const [newNotation, setNewNotation] = useState<Notation>("chords");
  const [isImporting, setIsImporting] = useState(false);
  const [audioNotice, setAudioNotice] = useState<string | null>(null);
  const id = useId();
  const isDirty = song !== savedSong;
  // The exports write real notes, which a key-less Nashville song (chords relative to C) doesn't have.
  const isExportLocked = song.meta.notation === "nashville" && song.meta.key === undefined;

  const fail = (e: unknown) => setError(e instanceof Error ? e.message : String(e));
  const load = (next: Song, nextPath: string | null, nextMode: "view" | "edit" = "view") => {
    history.reset(next);
    setMode(nextMode);
    setSavedSong(next);
    setPath(nextPath);
    setHasSong(true);
    setError(null);
  };
  const createGrid = () => load(emptySong({ beats: newBeats, unit: newUnit }, newNotation === "nashville" ? "nashville" : undefined), null, "edit");
  const loadImported = async (p: string) => {
    try {
      load(await readSong(p), p);
      onFile(p);
    } catch (e) {
      fail(e);
    }
  };
  const saveTo = async (p: string) => {
    try {
      await writeSong(p, song);
      setSavedSong(song);
      setError(null);
      if (p !== path) {
        setPath(p);
        onFile(p);
      }
    } catch (e) {
      fail(e);
    }
  };
  const saveAs = async () => {
    const p = await pickSavePath(song);
    if (p !== null) await saveTo(p);
  };
  const locateAudio = async () => {
    const p = await pickAudioPath();
    if (p === null) return;
    const sha256 = await audioSha256(p);
    if (sha256 === null) fail(new Error(`Can't read ${p}`));
    else history.apply((s) => ({ ...s, audio: { ...s.audio, path: relativeAudioPath(path, p), sha256 } }));
  };

  useImperativeHandle(ref, () => ({
    load: (next, nextPath) => load(next, nextPath),
    save: () => (path === null ? saveAs() : saveTo(path)),
    saveAs,
    exportAs: (format) => {
      const [name, extension, convert] = EXPORTS[format];
      exportSong(song, name, extension, convert).catch(fail);
    },
    // Read-only in view mode, like the grid.
    undo: () => mode === "edit" && history.undo(),
    redo: () => mode === "edit" && history.redo(),
    toggleMode: () => setMode((m) => (m === "edit" ? "view" : "edit")),
  }));

  const status: TabStatus = { title: song.meta.title, path, hasSong, isDirty, isEditing: mode === "edit", isExportLocked, isIdle: !hasSong && !isImporting };
  const statusKey = JSON.stringify(status);
  useEffect(() => onStatus(status), [statusKey]);

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

  // ponytail: songs without a path aren't autosaved (no temp/recovery file); add one if the user loses work.
  useEffect(() => {
    if (path === null || !isDirty) return;
    const timer = setTimeout(() => void saveTo(path), 2000);
    return () => clearTimeout(timer);
  });

  return (
    <>
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
              <Editor
                history={history}
                mode={mode}
                isActive={isActive}
                barsPerRow={settings.barsPerRow}
                compactPrint={settings.compactPrint}
                style={settings.style}
                minorConvention={settings.minorConvention}
                lowConfidenceThreshold={settings.lowConfidenceThreshold}
                panel={panel}
                audioPath={song.audio && resolveAudioPath(path, song.audio.path)}
                onLocateAudio={locateAudio}
                onShowPanels={() => onSettingsChange({ ...settings, isSidebarCollapsed: false })}
              />
            )}
          </div>
          <Sidebar isCollapsed={settings.isSidebarCollapsed} isEditing={mode === "edit"} onCollapsedChange={(isSidebarCollapsed) => onSettingsChange({ ...settings, isSidebarCollapsed })} panelsRef={setPanel}>
            <Toolbar history={history} settings={settings} />
          </Sidebar>
        </div>
      ) : (
        <div className="start">
          <section aria-labelledby={`${id}-new`} className="new-grid">
            <h2 id={`${id}-new`}>New grid</h2>
            <label htmlFor={`${id}-beats`}>Beats per bar</label>
            <select id={`${id}-beats`} value={newBeats} onChange={(e) => setNewBeats(Number(e.target.value))}>
              {BEATS_OPTIONS.map((n) => (
                <option key={n}>{n}</option>
              ))}
            </select>
            <label htmlFor={`${id}-unit`}>Beat unit</label>
            <select id={`${id}-unit`} value={newUnit} onChange={(e) => setNewUnit(Number(e.target.value) as 2 | 4 | 8)}>
              {UNIT_OPTIONS.map((n) => (
                <option key={n}>{n}</option>
              ))}
            </select>
            <label htmlFor={`${id}-notation`}>Notation</label>
            <select id={`${id}-notation`} value={newNotation} onChange={(e) => setNewNotation(e.target.value as Notation)}>
              <option value="chords">Chords</option>
              <option value="nashville">Nashville numbers</option>
            </select>
            <button type="button" className="primary" onClick={createGrid}>
              Create grid
            </button>
          </section>
          <section aria-labelledby={`${id}-import`}>
            <h2 id={`${id}-import`}>Import audio</h2>
            <EngineSetup>
              <Importer isActive={isActive} onRunningChange={setIsImporting} onResult={(p) => void loadImported(p)} />
            </EngineSetup>
          </section>
        </div>
      )}
    </>
  );
};
