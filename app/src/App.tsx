import { useEffect, useState } from "react";
import { emptySong, type Song } from "./model/song";
import { Editor } from "./ui/Editor";
import {
  confirmDiscard,
  isDirty,
  pickOpenPath,
  pickSavePath,
  pushRecent,
  readConfigJson,
  readSong,
  setWindowTitle,
  writeConfigJson,
  writeSong,
} from "./ui/fileActions";
import { useHistory } from "./ui/useHistory";

const RECENT = "recent.json";

function App() {
  const [initial] = useState(emptySong);
  const history = useHistory(initial);
  const { song } = history;
  const [savedSong, setSavedSong] = useState(initial);
  const [path, setPath] = useState<string | null>(null);
  const [recent, setRecent] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const isSongDirty = isDirty(song, savedSong);

  const fail = (e: unknown) => setError(e instanceof Error ? e.message : String(e));
  const updateRecent = (next: string[]) => {
    setRecent(next);
    writeConfigJson(RECENT, next).catch(fail);
  };
  const load = (next: Song, nextPath: string | null) => {
    history.reset(next);
    setSavedSong(next);
    setPath(nextPath);
    setError(null);
  };
  const canDiscard = async () => !isSongDirty || (await confirmDiscard());

  const newSong = async () => {
    if (await canDiscard()) load(emptySong(), null);
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

  useEffect(() => {
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

  // Re-bound on every render so the handler sees the current song; the grid lets Mod+N/O/S bubble up.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      const action = { n: newSong, o: () => openSong(), s: e.shiftKey ? saveAs : saveSong }[e.key.toLowerCase()];
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
      {error !== null && <p role="alert">{error}</p>}
      <Editor history={history} barsPerRow={4} style="fr" />
    </main>
  );
}

export default App;
