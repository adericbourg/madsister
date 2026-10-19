import { describe, expect, it, vi } from "vitest";
import { menuItems, type MenuState } from "./menu";

type Node = { text?: string; item?: unknown; id?: string; enabled?: boolean; checked?: boolean; items?: Node[]; action?: (id: string) => void };

const state: MenuState = { isMac: false, hasSong: true, isExportLocked: false, isEditing: true, isSidebarShown: true, recent: [] };
const menu = (patch: Partial<MenuState>, run = vi.fn()) => menuItems({ ...state, ...patch }, run) as Node[];
const texts = (nodes: Node[]) => nodes.map((n) => n.text ?? n.item);
const sub = (nodes: Node[], text: string) => nodes.find((n) => n.text === text)!.items!;

describe("menuItems", () => {
  it("follows each platform's conventions", () => {
    // Given Linux, Then File ends with Quit, About is under Help and the clipboard is left to the webview
    const linux = menu({});
    expect(texts(linux)).toEqual(["File", "Edit", "View", "Settings", "Help"]);
    expect(texts(sub(linux, "File")).at(-1)).toBe("Quit madsister");
    expect(texts(sub(linux, "Settings"))).toEqual(["Preferences…"]);
    expect(texts(sub(linux, "Help"))).toEqual(["About madsister"]);
    expect(texts(sub(linux, "Edit"))).toEqual(["Undo", "Redo"]);

    // Given macOS, Then the app menu holds About, Settings and Quit, and Edit has the clipboard items
    const mac = menu({ isMac: true });
    expect(texts(mac)).toEqual(["madsister", "File", "Edit", "View", "Window"]);
    expect(texts(sub(mac, "madsister"))).toEqual(expect.arrayContaining(["About madsister", "Settings…", "Quit madsister", "Services"]));
    expect(texts(sub(mac, "File"))).not.toContain("Quit madsister");
    expect(texts(sub(mac, "Edit"))).toEqual(["Undo", "Redo", "Separator", "Cut", "Copy", "Paste", "SelectAll"]);
  });

  it("disables what needs a song, and the exports of a key-less Nashville song", () => {
    // Given no song, Then the song actions are disabled and View's items unchecked
    const file = sub(menu({ hasSong: false }), "File");
    for (const text of ["Close tab", "Save", "Save as…", "Export PDF…", "Export"]) expect(file.find((n) => n.text === text)!.enabled).toBe(false);
    expect(file.find((n) => n.text === "Open…")!.enabled).toBe(true);
    expect(sub(menu({ hasSong: false }), "View").every((n) => n.enabled === false && n.checked === false)).toBe(true);

    // Given an export-locked song, Then the three exports are disabled
    const exports = sub(sub(menu({ isExportLocked: true }), "File"), "Export");
    expect(exports.every((n) => n.enabled === false)).toBe(true);
  });

  it("lists the recent files by their shortest unique name, each opening its path", () => {
    // Given two recent files
    const run = vi.fn();
    const recent = sub(sub(menu({ recent: ["/a/song.madsister.json", "/b/song.madsister.json"] }, run), "File"), "Open recent");

    // Then each has an item, followed by Clear recent
    expect(texts(recent)).toEqual(["a/song.madsister.json", "b/song.madsister.json", "Separator", "Clear recent"]);

    // When clicking the second, Then its path is run
    recent[1].action!(recent[1].id!);
    expect(run).toHaveBeenCalledWith("recent:/b/song.madsister.json");

    // Given no recent file, Then the submenu is disabled
    expect(sub(menu({}), "File").find((n) => n.text === "Open recent")!.enabled).toBe(false);
  });
});
