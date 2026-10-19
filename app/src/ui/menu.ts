import type { MenuOptions } from "@tauri-apps/api/menu";
import { shortestUniquePaths } from "./fileActions";

export type MenuState = {
  isMac: boolean;
  hasSong: boolean;
  isExportLocked: boolean;
  isEditing: boolean;
  isSidebarShown: boolean;
  recent: string[];
};

type Items = NonNullable<MenuOptions["items"]>;

export const RECENT_PREFIX = "recent:";

/**
 * The native menu (F-UI-5) as plain options. Every item calls `run` with its id; an Open recent item's id is
 * `RECENT_PREFIX` + its path.
 */
export const menuItems = ({ isMac, hasSong, isExportLocked, isEditing, isSidebarShown, recent }: MenuState, run: (id: string) => void): Items => {
  const item = (id: string, text: string, accelerator?: string, enabled = true) => ({ id, text, accelerator, enabled, action: run });
  const separator = { item: "Separator" } as const;
  const about = item("about", "About madsister");
  const preferences = item("preferences", isMac ? "Settings…" : "Preferences…", "CmdOrCtrl+,");
  const quit = item("quit", "Quit madsister", "CmdOrCtrl+Q");
  const canExport = hasSong && !isExportLocked;
  const file = {
    text: "File",
    items: [
      item("new", "New tab", "CmdOrCtrl+N"),
      item("open", "Open…", "CmdOrCtrl+O"),
      {
        text: "Open recent",
        enabled: recent.length > 0,
        items: [
          ...shortestUniquePaths(recent).map((label, i) => item(RECENT_PREFIX + recent[i], label)),
          separator,
          item("clear-recent", "Clear recent"),
        ],
      },
      separator,
      item("close", "Close tab", "CmdOrCtrl+W", hasSong),
      item("save", "Save", "CmdOrCtrl+S", hasSong),
      item("save-as", "Save as…", "CmdOrCtrl+Shift+S", hasSong),
      separator,
      {
        text: "Export",
        enabled: canExport,
        items: [item("chordpro", "ChordPro…", undefined, canExport), item("musicxml", "MusicXML…", undefined, canExport), item("midi", "MIDI…", undefined, canExport)],
      },
      item("print", "Export PDF…", "CmdOrCtrl+P", hasSong),
      ...(isMac ? [] : [separator, quit]),
    ],
  };
  // ponytail: the clipboard items are macOS only, where the webview needs them; on Linux a Ctrl+C accelerator would
  // take the key from the grid's own bar copy/paste.
  const edit = {
    text: "Edit",
    items: [
      item("undo", "Undo", "CmdOrCtrl+Z", hasSong),
      item("redo", "Redo", "CmdOrCtrl+Shift+Z", hasSong),
      ...(isMac ? [separator, { item: "Cut" } as const, { item: "Copy" } as const, { item: "Paste" } as const, { item: "SelectAll" } as const] : []),
    ],
  };
  const view = {
    text: "View",
    items: [
      { id: "edit-mode", text: "Edit mode", accelerator: "CmdOrCtrl+E", enabled: hasSong, checked: hasSong && isEditing, action: run },
      { id: "sidebar", text: "Show sidebar", enabled: hasSong, checked: hasSong && isSidebarShown, action: run },
    ],
  };
  if (!isMac) return [file, edit, view, { text: "Settings", items: [preferences] }, { text: "Help", items: [about] }];
  const app = {
    text: "madsister",
    items: [about, separator, preferences, separator, { item: "Services" } as const, separator, { item: "Hide" } as const, { item: "HideOthers" } as const, { item: "ShowAll" } as const, separator, quit],
  };
  const window = { text: "Window", items: [{ item: "Minimize" } as const, { item: "Maximize" } as const, separator, { item: "BringAllToFront" } as const] };
  return [app, file, edit, view, window];
};
