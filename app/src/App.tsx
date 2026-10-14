import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import type { Song } from "./model/song";
import { confirmDiscard, pickOpenPath, printSong, pushRecent, readConfigJson, readSong, setWindowTitle, writeConfigJson } from "./ui/fileActions";
import { parseSettings, type Settings } from "./ui/Toolbar";
import { About } from "./ui/About";
import { Preferences } from "./ui/Preferences";
import { IS_MAC } from "./ui/keymap";
import { menuItems, RECENT_PREFIX, type MenuState } from "./ui/menu";
import { Ribbon } from "./ui/Ribbon";
import { SongTab, type TabHandle, type TabStatus } from "./ui/SongTab";
import { TabBar, tabId } from "./ui/TabBar";

const RECENT = "recent.json";
const SETTINGS = "settings.json";
const SESSION = "session.json";

type Tab = { id: number; initial: { song: Song; path: string } | null };

/** A tab before it reports its status: the start screen. */
const START: TabStatus = { title: "Untitled", path: null, hasSong: false, isDirty: false, isEditing: false, isExportLocked: false, isIdle: true };

const panelId = (id: number) => `song-panel-${id}`;

function App() {
  const [tabs, setTabs] = useState<Tab[]>([{ id: 0, initial: null }]);
  const [activeId, setActiveId] = useState(0);
  const [statuses, setStatuses] = useState<Record<number, TabStatus>>({});
  const nextId = useRef(1);
  const handles = useRef<Record<number, TabHandle | null>>({});
  const [recent, setRecent] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(() => parseSettings(null));
  const [isRestored, setIsRestored] = useState(false);
  const preferences = useRef<HTMLDialogElement>(null);
  const about = useRef<HTMLDialogElement>(null);
  const status = (id: number) => statuses[id] ?? START;
  const active = status(activeId);
  // Tabs show up with the first song: a lone start screen has none.
  const isTabBarShown = tabs.length > 1 || active.hasSong;

  const fail = (e: unknown) => setError(e instanceof Error ? e.message : String(e));
  const updateRecent = (next: string[]) => {
    setRecent(next);
    writeConfigJson(RECENT, next).catch(fail);
  };
  const changeSettings = (next: Settings) => {
    setSettings(next);
    writeConfigJson(SETTINGS, next).catch(fail);
  };
  const addTab = (initial: Tab["initial"]) => {
    const id = nextId.current++;
    setTabs((t) => [...t, { id, initial }]);
    setActiveId(id);
  };
  const openSong = async (from?: string) => {
    const p = from ?? (await pickOpenPath());
    if (p === null) return;
    const openTab = tabs.find((t) => status(t.id).path === p);
    if (openTab !== undefined) return setActiveId(openTab.id);
    try {
      const song = await readSong(p);
      // An untouched start screen becomes the song; otherwise the song gets its own tab.
      if (active.isIdle) handles.current[activeId]!.load(song, p);
      else addTab({ song, path: p });
      updateRecent(pushRecent(recent, p));
      setError(null);
    } catch (e) {
      fail(e);
      if (from !== undefined) updateRecent(recent.filter((r) => r !== p));
    }
  };
  const closeTab = async (id: number) => {
    if (status(id).isDirty && !(await confirmDiscard())) return;
    const rest = tabs.filter((t) => t.id !== id);
    if (rest.length === 0) {
      const fresh = nextId.current++;
      setTabs([{ id: fresh, initial: null }]);
      setActiveId(fresh);
    } else {
      setTabs(rest);
      if (id === activeId) setActiveId(rest[Math.min(tabs.findIndex((t) => t.id === id), rest.length - 1)].id);
    }
    setStatuses(({ [id]: _, ...others }) => others);
  };
  const cycleTab = (step: number) => {
    const i = tabs.findIndex((t) => t.id === activeId);
    setActiveId(tabs[(i + step + tabs.length) % tabs.length].id);
  };

  useEffect(() => {
    readConfigJson(RECENT).then((json) => Array.isArray(json) && setRecent(json.filter((p) => typeof p === "string")));
  }, []);

  // F-UI-8: the tabs that have a file come back at launch; the files that can't be read any more are skipped.
  useEffect(() => {
    void Promise.all([readConfigJson(SETTINGS), readConfigJson(SESSION)]).then(async ([settingsJson, json]) => {
      const raw = (typeof json === "object" && json !== null ? json : {}) as { paths?: unknown; active?: unknown };
      const saved = parseSettings(settingsJson);
      setSettings(saved);
      const paths = saved.reopenTabs && Array.isArray(raw.paths) ? raw.paths.filter((p): p is string => typeof p === "string") : [];
      const opened = (await Promise.all(paths.map((path) => readSong(path).then((song) => ({ song, path }), () => null)))).filter((t) => t !== null);
      if (opened.length > 0) {
        const restored = opened.map((initial) => ({ id: nextId.current++, initial }));
        const activePath = typeof raw.active === "number" ? paths[raw.active] : undefined;
        setTabs(restored);
        setActiveId((restored.find((t) => t.initial.path === activePath) ?? restored[0]).id);
      }
      setIsRestored(true);
    });
  }, []);

  // A restored tab has its path before it reports its status.
  const sessionPaths = tabs.map((t) => statuses[t.id]?.path ?? t.initial?.path ?? null).filter((p) => p !== null);
  const session = JSON.stringify({ paths: sessionPaths, active: Math.max(0, sessionPaths.indexOf(active.path ?? "")) });
  useEffect(() => {
    if (isRestored) writeConfigJson(SESSION, JSON.parse(session)).catch(fail);
  }, [session, isRestored]);

  useEffect(() => {
    void setWindowTitle(`${active.isDirty ? "• " : ""}${active.title} — madsister`);
  }, [active.isDirty, active.title]);

  // Closing the window (or Quit) with unsaved songs asks first.
  const statusesRef = useRef(statuses);
  statusesRef.current = statuses;
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async (event) => {
      if (Object.values(statusesRef.current).some((s) => s.isDirty) && !(await confirmDiscard())) event.preventDefault();
    });
    return () => void unlisten.then((f) => f());
  }, []);

  // Re-bound on every render so the handler sees the current tabs.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.key !== "Tab") return;
      e.preventDefault();
      cycleTab(e.shiftKey ? -1 : 1);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  const runMenu = (id: string) => {
    if (id.startsWith(RECENT_PREFIX)) return void openSong(id.slice(RECENT_PREFIX.length));
    const tab = handles.current[activeId];
    // A text field undoes its own typing; otherwise the song history.
    const isInText = document.activeElement?.matches("input, textarea") ?? false;
    const actions: Record<string, () => unknown> = {
      new: () => addTab(null),
      open: () => openSong(),
      "clear-recent": () => updateRecent([]),
      close: () => closeTab(activeId),
      save: () => tab?.save(),
      "save-as": () => tab?.saveAs(),
      chordpro: () => tab?.exportAs("chordpro"),
      musicxml: () => tab?.exportAs("musicxml"),
      midi: () => tab?.exportAs("midi"),
      print: printSong,
      undo: () => (isInText ? document.execCommand("undo") : tab?.undo()),
      redo: () => (isInText ? document.execCommand("redo") : tab?.redo()),
      "edit-mode": () => tab?.toggleMode(),
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
  const menuState: MenuState = {
    isMac: IS_MAC,
    hasSong: active.hasSong,
    isExportLocked: active.isExportLocked,
    isEditing: active.isEditing,
    isSidebarShown: !settings.isSidebarCollapsed,
    recent,
  };
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
        hasSong={active.hasSong}
        isEditing={active.isEditing}
        onNew={() => addTab(null)}
        onOpen={() => openSong()}
        onSave={() => void handles.current[activeId]?.save()}
        onPrint={printSong}
        onToggleMode={() => handles.current[activeId]?.toggleMode()}
        onPreferences={() => preferences.current?.showModal()}
      />
      <Preferences ref={preferences} settings={settings} onSettingsChange={changeSettings} />
      <About ref={about} />
      {isTabBarShown && (
        <TabBar
          tabs={tabs.map(({ id }) => ({ id, title: status(id).hasSong ? status(id).title : "New tab", isDirty: status(id).isDirty }))}
          activeId={activeId}
          panelId={panelId}
          onSelect={setActiveId}
          onClose={(id) => void closeTab(id)}
        />
      )}
      {error !== null && <p role="alert">{error}</p>}
      {tabs.map(({ id, initial }) => (
        <div
          key={id}
          id={panelId(id)}
          role={isTabBarShown ? "tabpanel" : undefined}
          aria-labelledby={isTabBarShown ? tabId(id) : undefined}
          className="tabpanel"
          hidden={id !== activeId}
        >
          <SongTab
            ref={(handle) => {
              handles.current[id] = handle;
            }}
            isActive={id === activeId}
            initial={initial}
            settings={settings}
            onSettingsChange={changeSettings}
            onStatus={(next) => setStatuses((s) => ({ ...s, [id]: next }))}
            onFile={(p) => updateRecent(pushRecent(recent, p))}
          />
        </div>
      ))}
    </main>
  );
}

export default App;
