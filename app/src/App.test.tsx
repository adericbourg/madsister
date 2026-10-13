import { invoke } from "@tauri-apps/api/core";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { exists, readTextFile, writeFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { openUrl } from "@tauri-apps/plugin-opener";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import App from "./App";
import { serializeSong, type Song } from "./model/song";
import { transcribe } from "./ui/engine";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn(), confirm: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ readTextFile: vi.fn(), writeTextFile: vi.fn(), writeFile: vi.fn(), mkdir: vi.fn(), exists: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
// jsdom has no modal <dialog>: only expose it in the accessibility tree while open
HTMLDialogElement.prototype.showModal = function () {
  this.setAttribute("open", "");
};
HTMLDialogElement.prototype.close = function () {
  this.removeAttribute("open");
};
vi.mock("./ui/engine", () => ({ transcribe: vi.fn(), cancel: vi.fn(), engineNeedsSetup: async () => false, engineHasSections: async () => true }));
const drop = vi.hoisted(() => ({ handler: (_: { payload: unknown }) => {} }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (handler: typeof drop.handler) => {
      drop.handler = handler;
      return () => {};
    },
  }),
}));
const setTitle = vi.hoisted(() => vi.fn());
const closeWindow = vi.hoisted(() => vi.fn());
const closeRequest = vi.hoisted(() => ({ handler: async (_: { preventDefault: () => void }) => {} }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    setTitle,
    close: closeWindow,
    onCloseRequested: async (handler: typeof closeRequest.handler) => {
      closeRequest.handler = handler;
      return () => {};
    },
  }),
}));
type MenuNode = { text?: string; id?: string; enabled?: boolean; checked?: boolean; items?: MenuNode[]; action?: (id: string) => void };
const appMenu = vi.hoisted(() => ({ items: [] as MenuNode[] }));
vi.mock("@tauri-apps/api/menu", () => ({
  Menu: {
    new: async ({ items }: { items: MenuNode[] }) => {
      appMenu.items = items;
      return { setAsAppMenu: async () => {} };
    },
  },
}));
vi.mock("@tauri-apps/api/path", () => ({ appConfigDir: async () => "/config", join: async (...p: string[]) => p.join("/") }));

const song: Song = {
  version: 1,
  meta: { title: "Blues", meter: { beats: 4, unit: 4 } },
  sections: [{ id: "a", label: "Chorus", bars: [{ chords: [{ chord: "C:7", beats: 4 }] }] }],
};
const files: Record<string, string> = {};

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  for (const k of Object.keys(files)) delete files[k];
  vi.mocked(readTextFile).mockImplementation(async (path) => {
    const text = files[String(path)];
    if (text === undefined) throw new Error(`not found: ${String(path)}`);
    return text;
  });
  vi.mocked(writeTextFile).mockImplementation(async (path, text) => {
    files[String(path)] = String(text);
  });
  vi.mocked(confirm).mockResolvedValue(true);
  vi.mocked(exists).mockResolvedValue(false);
  vi.mocked(transcribe).mockResolvedValue(42);
});

const createGrid = (user: ReturnType<typeof userEvent.setup>) => user.click(screen.getByRole("button", { name: "Create grid" }));

/** The native menu item at that path (Linux layout: jsdom isn't a Mac), once the menu shows it. */
const menuItem = (...path: string[]) =>
  vi.waitFor(() => {
    let nodes = appMenu.items;
    for (const text of path.slice(0, -1)) nodes = nodes.find((n) => n.text === text)!.items!;
    const item = nodes.find((n) => n.text === path.at(-1));
    if (item === undefined) throw new Error(`no menu item ${path.join(" > ")}`);
    return item;
  });
const clickMenu = async (...path: string[]) => {
  const item = await menuItem(...path);
  await act(async () => item.action!(item.id!));
};

test("App_onStart_offersANewGridOrAnAudioImportButNoGridNorParameters", async () => {
  // Given / When the app just started
  const user = userEvent.setup();
  render(<App />);

  // Then both ways in are offered, without grid, parameters or song-specific actions
  expect(screen.getByRole("button", { name: "Create grid" })).toBeDefined();
  expect(screen.getByRole("button", { name: "Import audio…" })).toBeDefined();
  expect(screen.queryByRole("grid")).toBeNull();
  expect(screen.queryByRole("combobox", { name: "Chord style" })).toBeNull();
  expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);

  // When creating a grid
  await createGrid(user);

  // Then the empty song is shown with its own parameters, the user settings staying in Preferences
  expect(screen.getByRole("heading", { name: "Untitled" })).toBeDefined();
  expect(screen.getAllByRole("gridcell", { name: /^Verse, bar \d, beat 1: no chord$/ })).toHaveLength(4);
  expect(screen.getByRole("combobox", { name: "Spelling" })).toBeDefined();
  expect(screen.queryByRole("combobox", { name: "Chord style" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Create grid" })).toBeNull();
});

test("App_whenCollapsingTheSidebar_showsAnIconRailThatReopensItOnAPanel", async () => {
  // Given a grid in edit mode, and its expanded sidebar
  const user = userEvent.setup();
  const scrollIntoView = vi.fn();
  Element.prototype.scrollIntoView = scrollIntoView;
  render(<App />);
  await createGrid(user);
  const toggle = screen.getByRole("button", { name: "Parameters" });
  expect(toggle.getAttribute("aria-expanded")).toBe("true");
  expect(screen.getByRole("combobox", { name: "Spelling" })).toBeDefined();
  expect(screen.getByRole("group", { name: "Section" })).toBeDefined();
  expect(screen.queryByRole("button", { name: "Transpose" })).toBeNull();

  // When collapsing it, Then the panels give way to one rail button each, the grid stays, and the state is saved
  await user.click(toggle);
  expect(toggle.getAttribute("aria-expanded")).toBe("false");
  expect(screen.queryByRole("combobox", { name: "Spelling" })).toBeNull();
  expect(screen.queryByRole("group", { name: "Section" })).toBeNull();
  for (const name of ["Song", "Transpose", "Section", "Bar", "Bar lines", "Tempo"]) expect(screen.getByRole("button", { name })).toBeDefined();
  expect(screen.getByRole("grid")).toBeDefined();
  expect(JSON.parse(files["/config/settings.json"]).isSidebarCollapsed).toBe(true);

  // When switching to view mode, Then the rail drops the editing panels
  await clickMenu("View", "Edit mode");
  expect(screen.getByRole("button", { name: "Transpose" })).toBeDefined();
  expect(screen.queryByRole("button", { name: "Section" })).toBeNull();

  // When clicking Transpose on the rail, Then the sidebar expands, scrolled to that panel
  await user.click(screen.getByRole("button", { name: "Transpose" }));
  expect(toggle.getAttribute("aria-expanded")).toBe("true");
  expect(screen.getByRole("combobox", { name: "Spelling" })).toBeDefined();
  expect(scrollIntoView.mock.contexts.at(-1)).toBe(screen.getByRole("group", { name: "Transpose" }));
  expect(JSON.parse(files["/config/settings.json"]).isSidebarCollapsed).toBe(false);
  delete (Element.prototype as Partial<Element>).scrollIntoView;
});

test("App_whenOpeningFiles_loadsValidOnesAndReportsInvalidOnes", async () => {
  // Given a valid and an invalid song file
  const user = userEvent.setup();
  files["/blues.madsister.json"] = serializeSong(song);
  files["/bad.madsister.json"] = '{"version": 1}';
  render(<App />);

  // When opening the valid file from the menu
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await clickMenu("File", "Open…");

  // Then the song is loaded and the file is listed as recent
  expect(await screen.findByRole("heading", { name: "Blues" })).toBeDefined();
  expect(await menuItem("File", "Open recent", "blues.madsister.json")).toBeDefined();
  expect(files["/config/recent.json"]).toBe('["/blues.madsister.json"]');

  // When opening the invalid file from the toolbar
  vi.mocked(open).mockResolvedValueOnce("/bad.madsister.json");
  await user.click(screen.getByRole("button", { name: "Open…" }));

  // Then the error is shown and the current song is kept
  expect((await screen.findByRole("alert")).textContent).toMatch(/meta/);
  expect(screen.getByRole("heading", { name: "Blues" })).toBeDefined();

  // When the song is closed, its file goes missing, and it is reopened from Open recent
  await clickMenu("File", "Close tab");
  delete files["/blues.madsister.json"];
  await clickMenu("File", "Open recent", "blues.madsister.json");

  // Then the file is dropped from the recent list
  await vi.waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/not found/));
  await vi.waitFor(() => expect((appMenu.items[0].items!.find((n) => n.text === "Open recent")!).enabled).toBe(false));

  // When reopening the valid file, then clearing the recent files
  files["/blues.madsister.json"] = serializeSong(song);
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await clickMenu("File", "Open…");
  await clickMenu("File", "Open recent", "Clear recent");

  // Then the list is empty
  await vi.waitFor(() => expect(files["/config/recent.json"]).toBe("[]"));

  // When starting a new song from the menu
  await clickMenu("File", "New tab");

  // Then the start screen replaces the loaded song
  expect(await screen.findByRole("button", { name: "Create grid" })).toBeDefined();
  expect(screen.queryByRole("grid")).toBeNull();
});

test("App_whenImportingAudio_loadsTheResultAndReportsErrors", async () => {
  // Given the app, and the song the engine will write
  const user = userEvent.setup();
  render(<App />);
  files["/music/blues.madsister.json"] = serializeSong(song);
  const engineSays = (event: Parameters<Parameters<typeof transcribe>[4]>[0]) =>
    act(() => vi.mocked(transcribe).mock.lastCall![4](event));

  // When dropping the mp3, running the import and the engine reports its result
  await act(() => drop.handler({ payload: { type: "drop", paths: ["/music/blues.mp3"] } }));
  await user.click(screen.getByRole("button", { name: "Run" }));
  await engineSays({ type: "result", path: "/music/blues.madsister.json" });

  // Then the song is loaded and listed as recent
  expect(await screen.findByRole("heading", { name: "Blues" })).toBeDefined();
  expect(await menuItem("File", "Open recent", "blues.madsister.json")).toBeDefined();

  // When a second import fails, started from the start screen
  await user.click(screen.getByRole("button", { name: "New" }));
  await act(() => drop.handler({ payload: { type: "drop", paths: ["/music/other.mp3"] } }));
  await user.click(screen.getByRole("button", { name: "Run" }));
  await engineSays({ type: "error", message: "boom", stderr: "" });

  // Then the error is shown on the start screen
  expect(screen.getByRole("alert").textContent).toContain("boom");
  expect(screen.getByRole("button", { name: "Create grid" })).toBeDefined();
});

test("App_whenOpeningASongWithMissingOrChangedAudio_showsANoticeAndLocatesTheAudio", async () => {
  // Given a song whose audio is missing, and the hashes of the audio files that exist
  const user = userEvent.setup();
  const hashes: Record<string, string> = { "/music/changed.mp3": "new", "/music/found.mp3": "abc" };
  vi.mocked(invoke).mockImplementation(async (_, args) => hashes[(args as { path: string }).path] ?? null);
  files["/blues.madsister.json"] = serializeSong({ ...song, audio: { path: "/music/blues.mp3", sha256: "abc" } });
  files["/changed.madsister.json"] = serializeSong({ ...song, audio: { path: "/music/changed.mp3", sha256: "old" } });
  render(<App />);

  // When opening the song
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await user.click(screen.getByRole("button", { name: "Open…" }));

  // Then the song is shown, with a notice that the audio is missing
  expect(await screen.findByRole("heading", { name: "Blues" })).toBeDefined();
  expect(await screen.findByText("Audio file not found next to the song: /music/blues.mp3. The song can't be played until you locate it.")).toBeDefined();

  // When locating the audio
  vi.mocked(open).mockResolvedValueOnce("/music/found.mp3");
  await user.click(screen.getByRole("button", { name: "Locate audio…" }));

  // Then the notice goes away, and the new path (relative to the song) and hash are saved
  await vi.waitFor(() => expect(screen.queryByText(/Audio file/)).toBeNull());
  vi.mocked(save).mockResolvedValueOnce("/blues2.madsister.json");
  await clickMenu("File", "Save as…");
  await vi.waitFor(() => expect(files["/blues2.madsister.json"]).toBeDefined());
  expect(JSON.parse(files["/blues2.madsister.json"]).audio).toEqual({ path: "music/found.mp3", sha256: "abc" });

  // When switching to edit mode and undoing, Then the old path is back with its notice (locating is one history entry)
  await clickMenu("View", "Edit mode");
  await user.click(screen.getByRole("gridcell", { name: /^Chorus, bar 1, beat 1/ }));
  await user.keyboard("{Escape}{Control>}z{/Control}");
  expect(await screen.findByText("Audio file not found next to the song: /music/blues.mp3. The song can't be played until you locate it.")).toBeDefined();

  // When opening a song whose audio has changed since transcription
  vi.mocked(open).mockResolvedValueOnce("/changed.madsister.json");
  await user.click(screen.getByRole("button", { name: "Open…" }));

  // Then the notice says so
  expect(await screen.findByText("Audio file has changed since the transcription: /music/changed.mp3")).toBeDefined();
});

test("App_whenSaving_writesTheSerializedSongThenAutosavesEdits", async () => {
  // Given a new song
  vi.useFakeTimers({ shouldAdvanceTime: true });
  const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
  render(<App />);
  await createGrid(user);
  expect(setTitle).toHaveBeenLastCalledWith("Untitled — madsister");

  // When saving it from the menu (no path yet: Save As)
  vi.mocked(save).mockResolvedValueOnce("/untitled.madsister.json");
  await clickMenu("File", "Save");

  // Then Save As proposes a name from the title and writes the song there
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "Untitled.madsister.json" }));
  await vi.waitFor(() => expect(files["/untitled.madsister.json"]).toBeDefined());
  expect(JSON.parse(files["/untitled.madsister.json"])).toMatchObject({ meta: { title: "Untitled" } });

  // When editing the song, then waiting 2 s
  await user.click(screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ }));
  await user.keyboard("C{Enter}");
  expect(setTitle).toHaveBeenLastCalledWith("• Untitled — madsister");
  await act(() => vi.advanceTimersByTimeAsync(2000));

  // Then the edit is autosaved
  expect(files["/untitled.madsister.json"]).toContain('"C:maj"');
  expect(setTitle).toHaveBeenLastCalledWith("Untitled — madsister");
  vi.useRealTimers();
});

test("App_onAboutMenu_opensADialogWithAuthorModelAndFontCreditsThenTheRepoLink", async () => {
  // Given the app just started, with no song
  const user = userEvent.setup();
  render(<App />);
  expect(screen.queryByRole("dialog")).toBeNull();

  // When opening About from the menu
  await clickMenu("Help", "About madsister");

  // Then a dialog credits the author, the models and every chart font, ending with the repo link
  const dialog = screen.getByRole("dialog", { name: "About madsister" });
  expect(dialog.textContent).toContain("Alban Dericbourg");
  for (const name of ["madmom", "all-in-one", "Demucs", "BTC", "Chord-CNN-LSTM", "Patrick Hand", "Kalam", "Petaluma"])
    expect(within(dialog).getByRole("link", { name })).toBeDefined();
  expect(within(dialog).getAllByText(/SIL Open Font License/)).toHaveLength(3);
  const links = within(dialog).getAllByRole("link");
  expect(links[links.length - 1].getAttribute("href")).toBe("https://github.com/adericbourg/madsister");

  // When clicking a link, then it opens in the system browser
  await user.click(links[links.length - 1]);
  expect(openUrl).toHaveBeenCalledWith("https://github.com/adericbourg/madsister");

  // When closing the dialog, then it is gone
  await user.click(within(dialog).getByRole("button", { name: "Close" }));
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("App_whenUsingPreferencesAndTheToolbar_restylesTransposesEditsMetadataAndPersistsSettings", async () => {
  // Given saved settings (international style, 2 bars per row) and Cmaj7 typed in the first slot
  const user = userEvent.setup();
  files["/config/settings.json"] = '{"style":"intl","barsPerRow":2}';
  render(<App />);
  await createGrid(user);
  await vi.waitFor(() => expect(screen.getAllByRole("row")).toHaveLength(2));
  await user.keyboard("Cmaj7{Enter}");
  const firstSlot = () => screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ });
  expect(firstSlot().textContent).toBe("Cmaj7");

  const preferences = () => within(screen.getByRole("dialog", { name: "Preferences" }));
  const editPreferences = async (edit: () => Promise<void>, button = "Save") => {
    await user.click(screen.getByRole("button", { name: "Preferences" }));
    await edit();
    await user.click(preferences().getByRole("button", { name: button }));
    expect(screen.queryByRole("dialog")).toBeNull();
  };

  // When switching to the French style in Preferences but cancelling, Then nothing changes
  await editPreferences(async () => {
    await user.selectOptions(screen.getByRole("combobox", { name: "Chord style" }), "fr");
    expect(firstSlot().textContent).toBe("Cmaj7");
  }, "Cancel");
  expect(firstSlot().textContent).toBe("Cmaj7");
  expect(files["/config/settings.json"]).toBe('{"style":"intl","barsPerRow":2}');

  // When reopening it, Then the cancelled edit is gone
  await user.click(screen.getByRole("button", { name: "Preferences" }));
  expect((screen.getByRole("combobox", { name: "Chord style" }) as HTMLSelectElement).value).toBe("intl");
  await user.click(preferences().getByRole("button", { name: "Cancel" }));

  // When switching to the French style and saving, Then the chord is re-rendered and the setting saved
  await editPreferences(() => user.selectOptions(screen.getByRole("combobox", { name: "Chord style" }), "fr"));
  expect(firstSlot().textContent).toBe("C7M");
  expect(JSON.parse(files["/config/settings.json"])).toEqual({ style: "fr", barsPerRow: 2, lowConfidenceThreshold: 0.5, compactPrint: false, font: "petaluma-script", minorConvention: "relative", isSidebarCollapsed: false });

  // When switching to the Latin style, Then the root is spelled Do and the setting saved
  await editPreferences(() => user.selectOptions(screen.getByRole("combobox", { name: "Chord style" }), "latin"));
  expect(firstSlot().textContent).toBe("Do7M");
  expect(JSON.parse(files["/config/settings.json"]).style).toBe("latin");

  // When switching back to French, ticking Compact print and picking Kalam, Then the settings are saved and the app root carries the classes
  await editPreferences(async () => {
    await user.selectOptions(screen.getByRole("combobox", { name: "Chord style" }), "fr");
    // Then the review threshold is offered, though the grid went through no chord recognition
    expect(screen.getByRole("spinbutton", { name: "Review chords below confidence" })).toBeDefined();
    await user.click(screen.getByRole("checkbox", { name: "Compact print (2 columns)" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Font" }), "kalam");
  });
  expect(JSON.parse(files["/config/settings.json"])).toMatchObject({ style: "fr", compactPrint: true, font: "kalam" });
  expect(screen.getByRole("main").className).toBe("compact-print font-kalam");
  for (const font of ["serif", "petaluma-text", "petaluma-script"]) {
    await editPreferences(() => user.selectOptions(screen.getByRole("combobox", { name: "Font" }), font));
    expect(JSON.parse(files["/config/settings.json"]).font).toBe(font);
    expect(screen.getByRole("main").className).toBe(`compact-print font-${font}`);
  }

  // When transposing up twice with flats, then undoing once
  await user.selectOptions(screen.getByRole("combobox", { name: "Spelling" }), "flat");
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  expect(firstSlot().textContent).toBe("D7M");
  expect(document.activeElement).toBe(screen.getByRole("button", { name: "+1 semitone" }));
  await user.click(firstSlot());
  await user.keyboard("{Escape}{Control>}z{/Control}");

  // Then each click is one history entry
  expect(firstSlot().textContent).toBe("Db7M");

  // When filling the metadata, with an invalid key first
  const key = screen.getByRole("textbox", { name: "Key" });
  await user.type(key, "H{Enter}");
  expect(screen.getByRole("alert").textContent).toMatch(/note/);
  expect(key.getAttribute("aria-invalid")).toBe("true");
  await user.clear(key);
  await user.type(key, "Bbm{Enter}");
  const title = screen.getByRole("textbox", { name: "Title" });
  await user.clear(title);
  await user.type(title, "Blues{Tab}");
  await user.type(screen.getByRole("spinbutton", { name: "Tempo (BPM)" }), "96{Enter}");

  // Then the chart header shows them and the error is gone
  expect(screen.getByRole("heading", { name: "Blues" })).toBeDefined();
  expect(screen.getByText("Bbm")).toBeDefined();
  expect(screen.getByText("♩ = 96")).toBeDefined();
  expect(screen.queryByRole("alert")).toBeNull();

  // When starting a new 7/8 song and saving it
  await user.click(screen.getByRole("button", { name: "New" }));
  await user.selectOptions(screen.getByRole("combobox", { name: "Beats per bar" }), "7");
  await user.selectOptions(screen.getByRole("combobox", { name: "Beat unit" }), "8");
  await createGrid(user);
  vi.mocked(save).mockResolvedValueOnce("/waltz.madsister.json");
  await clickMenu("File", "Save as…");

  // Then the new song has that meter
  await vi.waitFor(() => expect(files["/waltz.madsister.json"]).toBeDefined());
  expect(JSON.parse(files["/waltz.madsister.json"]).meta.meter).toEqual({ beats: 7, unit: 8 });
});

test("App_whenTheSongHasRecognizedChords_reviewsThemAgainstThePersistedThreshold", async () => {
  // Given a song whose first chord carries a recognition confidence
  const user = userEvent.setup();
  const [first, ...others] = song.sections;
  const [firstBar, ...bars] = first.bars;
  const recognized = { ...song, sections: [{ ...first, bars: [{ ...firstBar, chords: firstBar.chords.map((c) => ({ ...c, confidence: 0.4 })) }, ...bars] }, ...others] };
  files["/blues.madsister.json"] = serializeSong(recognized);
  render(<App />);
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await clickMenu("File", "Open…");
  await screen.findByRole("heading", { name: "Blues" });
  expect(screen.getByText("1 chord to review")).toBeDefined();

  // When raising the low-confidence threshold in Preferences, Then the setting is saved; an out-of-range value is ignored
  await user.click(screen.getByRole("button", { name: "Preferences" }));
  const threshold = screen.getByRole("spinbutton", { name: "Review chords below confidence" });
  fireEvent.change(threshold, { target: { value: "0.7" } });
  fireEvent.change(threshold, { target: { value: "2" } });
  expect((threshold as HTMLInputElement).value).toBe("0.7");
  await user.click(within(screen.getByRole("dialog", { name: "Preferences" })).getByRole("button", { name: "Save" }));
  await vi.waitFor(() => expect(JSON.parse(files["/config/settings.json"]).lowConfidenceThreshold).toBe(0.7));

});

test("App_whenPrintingOrExporting_printsFromTheRibbonAndTheMenuAndWritesChordProMusicXmlAndMidi", async () => {
  // Given the app
  const user = userEvent.setup();
  const print = vi.spyOn(window, "print").mockImplementation(() => undefined);
  render(<App />);
  await createGrid(user);

  // When clicking Print on the ribbon, then in the menu
  await user.click(screen.getByRole("button", { name: "Print…" }));
  await clickMenu("File", "Print…");

  // Then the chart is printed twice
  expect(print).toHaveBeenCalledTimes(2);
  print.mockRestore();

  // When exporting ChordPro to a picked path
  vi.mocked(save).mockResolvedValueOnce("/out/untitled.cho");
  await clickMenu("File", "Export", "ChordPro…");

  // Then the dialog proposes a .cho named after the title and the grid is written there
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "Untitled.cho" }));
  await vi.waitFor(() => expect(files["/out/untitled.cho"]).toContain('{start_of_grid shape="1+4x4+1" label="Verse"}'));

  // When exporting MusicXML to a picked path
  vi.mocked(save).mockResolvedValueOnce("/out/untitled.musicxml");
  await clickMenu("File", "Export", "MusicXML…");

  // Then the dialog proposes a .musicxml named after the title and the score is written there
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "Untitled.musicxml" }));
  await vi.waitFor(() => expect(files["/out/untitled.musicxml"]).toContain("<rehearsal>Verse</rehearsal>"));

  // When exporting MIDI to a picked path
  vi.mocked(save).mockResolvedValueOnce("/out/untitled.mid");
  await clickMenu("File", "Export", "MIDI…");

  // Then the dialog proposes a .mid named after the title and a Standard MIDI File is written there as bytes
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "Untitled.mid" }));
  await vi.waitFor(() => expect(writeFile).toHaveBeenCalledWith("/out/untitled.mid", expect.any(Uint8Array)));
  const midi = vi.mocked(writeFile).mock.calls[0][1] as Uint8Array;
  expect(String.fromCharCode(...midi.slice(0, 4))).toBe("MThd");
});

test("App_whenTabbingThroughTheControls_reachesEveryEnabledControl", async () => {
  // Given the app, with a grid so that every control is present
  const user = userEvent.setup();
  render(<App />);
  await createGrid(user);

  // When tabbing forward past the end
  const reached = new Set<Element | null>();
  for (let i = 0; i < 50; i++) {
    await user.tab();
    reached.add(document.activeElement);
  }

  // Then every enabled control and the grid's single tab stop were focused
  const controls = [...document.querySelectorAll('button:not([disabled]), input, select:not([disabled]), [tabindex="0"]')].filter(
    (control) => !control.closest("dialog:not([open])"),
  );
  expect(controls.length).toBeGreaterThan(15);
  for (const control of controls) expect(reached).toContain(control);
});

test("App_whenResettingTheTranspose_restoresTheInitialKey", async () => {
  // Given Cmaj7 transposed up by two semitones
  const user = userEvent.setup();
  render(<App />);
  await createGrid(user);
  await user.keyboard("Cmaj7{Enter}");
  const firstSlot = () => screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ });
  const reset = screen.getByRole("button", { name: "Reset" });
  expect((reset as HTMLButtonElement).disabled).toBe(true);
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  expect(firstSlot().textContent).toBe("D7M");

  // When resetting, Then the chord is back and Reset is disabled again
  await user.click(reset);
  expect(firstSlot().textContent).toBe("C7M");
  expect((reset as HTMLButtonElement).disabled).toBe(true);
});

test("App_whenUndoingATranspose_resetStillRestoresTheInitialKey", async () => {
  // Given Cmaj7 transposed up twice, then one transpose undone
  const user = userEvent.setup();
  render(<App />);
  await createGrid(user);
  await user.keyboard("Cmaj7{Enter}");
  const firstSlot = () => screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ });
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  await user.click(firstSlot());
  await user.keyboard("{Escape}{Control>}z{/Control}");
  expect(firstSlot().textContent).toBe("C#7M");

  // When resetting, Then the chord is back to C
  await user.click(screen.getByRole("button", { name: "Reset" }));
  expect(firstSlot().textContent).toBe("C7M");
});

test("App_whenCreatingANashvilleGrid_needsAKeyToSwitchNotationOrExport", async () => {
  // Given a blank Nashville grid, which has no key
  const user = userEvent.setup();
  render(<App />);
  await user.selectOptions(screen.getByRole("combobox", { name: "Notation" }), "nashville");
  await createGrid(user);
  const cell = (bar: number) => screen.getByRole("gridcell", { name: new RegExp(`^Verse, bar ${bar}, beat 1`) }).textContent;
  const notation = () => screen.getByRole("combobox", { name: "Notation" }) as HTMLSelectElement;

  // Then notation can't be switched and nothing can be exported
  expect(notation().value).toBe("nashville");
  expect(notation().disabled).toBe(true);
  expect(screen.getByText("Set a key to see the chords")).toBeDefined();
  expect((await menuItem("File", "Export", "ChordPro…")).enabled).toBe(false);
  expect(screen.getByRole("button", { name: "+1 semitone" }).matches(":disabled")).toBe(true);

  // When typing degrees
  await user.keyboard("1{Enter}4m{Enter}b7{Enter}");

  // Then they are shown as typed; a letter chord is refused
  expect([cell(1), cell(2), cell(3)]).toEqual(["1", "4m", "b7"]);
  await user.keyboard("Am{Enter}");
  expect(screen.getByRole("alert").textContent).toBe('"Am" isn\'t a degree (1–7, optional b or #)');
  await user.keyboard("{Escape}");

  // When setting the key to G
  await user.type(screen.getByRole("textbox", { name: "Key" }), "G{Enter}");

  // Then the numbers stay and switching and exporting are possible
  expect([cell(1), cell(2), cell(3)]).toEqual(["1", "4m", "b7"]);
  expect(notation().disabled).toBe(false);
  await vi.waitFor(async () => expect((await menuItem("File", "Export", "ChordPro…")).enabled).toBe(true));

  // When switching to chords, Then they are G, Cm and F (b7 of G)
  await user.selectOptions(notation(), "chords");
  expect([cell(1), cell(2), cell(3)]).toEqual(["G", "Cm", "F"]);
});

test("App_whenOpeningASong_startsInViewModeUntilSwitchedToEdit", async () => {
  // Given a saved song opened from disk
  const user = userEvent.setup();
  files["/blues.madsister.json"] = serializeSong(song);
  render(<App />);
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await clickMenu("File", "Open…");
  await screen.findByRole("heading", { name: "Blues" });
  const editButton = () => screen.getByRole("switch", { name: "Edit mode" }) as HTMLInputElement;

  // Then it is read-only: clicking a bar opens no input
  expect(editButton().checked).toBe(false);
  await user.click(screen.getByRole("gridcell", { name: /^Chorus, bar 1, beat 1/ }));
  expect(screen.queryByRole("textbox", { name: "Chord" })).toBeNull();

  // When choosing View > Edit mode, Then clicking a bar edits it, the menu item is checked, and the ribbon switch goes back
  await clickMenu("View", "Edit mode");
  expect(editButton().checked).toBe(true);
  await vi.waitFor(async () => expect((await menuItem("View", "Edit mode")).checked).toBe(true));
  await user.click(screen.getByRole("gridcell", { name: /^Chorus, bar 1, beat 1/ }));
  expect(screen.getByRole("textbox", { name: "Chord" })).toBeDefined();
  await user.keyboard("{Escape}");
  await user.click(editButton());
  expect(editButton().checked).toBe(false);

  // When creating a new grid, Then it starts in edit mode
  await clickMenu("File", "New tab");
  await createGrid(user);
  expect(editButton().checked).toBe(true);
});

test("App_fromTheMenu_undoesShowsTheSidebarOpensPreferencesAndQuits", async () => {
  // Given a grid with a chord typed, focus outside any text field
  const user = userEvent.setup();
  render(<App />);
  await createGrid(user);
  const firstSlot = () => screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ });
  const empty = firstSlot().textContent;
  await user.keyboard("C{Enter}");
  expect(firstSlot().textContent).toBe("C");

  // When choosing Edit > Undo, then Redo
  await clickMenu("Edit", "Undo");
  expect(firstSlot().textContent).toBe(empty);
  await clickMenu("Edit", "Redo");
  expect(firstSlot().textContent).toBe("C");

  // When unticking View > Show sidebar, Then the sidebar is collapsed, and ticking it expands it again
  await clickMenu("View", "Show sidebar");
  expect(screen.getByRole("button", { name: "Parameters" }).getAttribute("aria-expanded")).toBe("false");
  await clickMenu("View", "Show sidebar");
  expect(screen.getByRole("button", { name: "Parameters" }).getAttribute("aria-expanded")).toBe("true");

  // When choosing Settings > Preferences…, Then the dialog opens
  await clickMenu("Settings", "Preferences…");
  expect(screen.getByRole("dialog", { name: "Preferences" })).toBeDefined();

  // When choosing Quit, Then the window is closed
  await clickMenu("File", "Quit madsister");
  expect(closeWindow).toHaveBeenCalled();
});

test("App_withSeveralSongs_opensEachInItsOwnTabWithItsOwnState", async () => {
  // Given two saved songs, and the app on its start screen: no tabs yet
  const user = userEvent.setup();
  files["/blues.madsister.json"] = serializeSong(song);
  files["/waltz.madsister.json"] = serializeSong({ ...song, meta: { ...song.meta, title: "Waltz" } });
  render(<App />);
  expect(screen.queryByRole("tablist", { name: "Songs" })).toBeNull();
  const tabs = () => within(screen.getByRole("tablist", { name: "Songs" })).getAllByRole("tab");
  const activeTab = () => tabs().find((t) => t.getAttribute("aria-selected") === "true")!;

  // When opening the first song, Then it takes the start screen's place, in a single tab
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await clickMenu("File", "Open…");
  await screen.findByRole("heading", { name: "Blues" });
  expect(tabs().map((t) => t.textContent)).toEqual(["Blues"]);

  // When opening the second one, Then it opens in a new, active tab, in front of the first song
  vi.mocked(open).mockResolvedValueOnce("/waltz.madsister.json");
  await clickMenu("File", "Open…");
  await screen.findByRole("heading", { name: "Waltz" });
  expect(tabs().map((t) => t.textContent)).toEqual(["Blues", "Waltz"]);
  expect(activeTab().textContent).toBe("Waltz");
  expect(screen.queryByRole("heading", { name: "Blues" })).toBeNull();
  expect(setTitle).toHaveBeenLastCalledWith("Waltz — madsister");

  // When opening the first song again, Then its tab comes back to the front, no tab added
  await clickMenu("File", "Open recent", "blues.madsister.json");
  await vi.waitFor(() => expect(activeTab().textContent).toBe("Blues"));
  expect(tabs()).toHaveLength(2);

  // When editing the first song, then cycling to the other tab and back with Ctrl+Tab
  await clickMenu("View", "Edit mode");
  const firstSlot = () => screen.getByRole("gridcell", { name: /^Chorus, bar 1, beat 1/ });
  await user.click(firstSlot());
  await user.keyboard("G{Enter}");
  expect(activeTab().textContent).toBe("•Blues");
  await user.keyboard("{Control>}{Tab}{/Control}");
  expect(activeTab().textContent).toBe("Waltz");
  expect(firstSlot().textContent).toBe("C7");
  await user.keyboard("{Control>}{Shift>}{Tab}{/Shift}{/Control}");

  // Then the first song kept its edit, its mode and its undo history
  expect(activeTab().textContent).toBe("•Blues");
  expect(firstSlot().textContent).toBe("G");
  await user.keyboard("{Escape}");
  await clickMenu("Edit", "Undo");
  expect(firstSlot().textContent).toBe("C7");

  // When pressing the right arrow on the focused tab, Then the next tab is shown and focused
  activeTab().focus();
  await user.keyboard("{ArrowRight}");
  expect(activeTab().textContent).toBe("Waltz");
  expect(document.activeElement).toBe(activeTab());
});

test("App_whenClosingTabsOrTheWindow_asksBeforeDroppingUnsavedWork", async () => {
  // Given a saved song, and a new unsaved grid in a second tab
  const user = userEvent.setup();
  files["/blues.madsister.json"] = serializeSong(song);
  render(<App />);
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await clickMenu("File", "Open…");
  await screen.findByRole("heading", { name: "Blues" });
  await clickMenu("File", "New tab");
  await createGrid(user);
  await user.keyboard("C{Enter}");
  const tabs = () => within(screen.getByRole("tablist", { name: "Songs" })).queryAllByRole("tab");
  expect(tabs().map((t) => t.textContent)).toEqual(["Blues", "•Untitled"]);

  // When the window is asked to close and the user declines, Then it stays open
  vi.mocked(confirm).mockResolvedValueOnce(false);
  const event = { preventDefault: vi.fn() };
  await act(() => closeRequest.handler(event));
  expect(event.preventDefault).toHaveBeenCalled();

  // When closing the unsaved tab and declining, Then it stays
  vi.mocked(confirm).mockResolvedValueOnce(false);
  await user.click(screen.getByRole("button", { name: "Close Untitled" }));
  expect(tabs()).toHaveLength(2);

  // When closing it and accepting, Then it goes, and the saved song is in front again
  await user.click(screen.getByRole("button", { name: "Close Untitled" }));
  await vi.waitFor(() => expect(tabs().map((t) => t.textContent)).toEqual(["Blues"]));
  expect(screen.getByRole("heading", { name: "Blues" })).toBeDefined();

  // When the window is asked to close with nothing unsaved, Then it closes without asking
  vi.mocked(confirm).mockClear();
  const clean = { preventDefault: vi.fn() };
  await act(() => closeRequest.handler(clean));
  expect(confirm).not.toHaveBeenCalled();
  expect(clean.preventDefault).not.toHaveBeenCalled();

  // When closing the last tab with File > Close tab, Then the start screen is back, without tabs
  await clickMenu("File", "Close tab");
  expect(await screen.findByRole("button", { name: "Create grid" })).toBeDefined();
  expect(screen.queryByRole("tablist", { name: "Songs" })).toBeNull();
});

test("App_atLaunch_reopensTheTabsOfThePreviousSessionThatStillHaveTheirFile", async () => {
  // Given a session of three files, the second one active, and the third gone
  const user = userEvent.setup();
  files["/blues.madsister.json"] = serializeSong(song);
  files["/waltz.madsister.json"] = serializeSong({ ...song, meta: { ...song.meta, title: "Waltz" } });
  files["/config/session.json"] = JSON.stringify({ paths: ["/blues.madsister.json", "/waltz.madsister.json", "/gone.madsister.json"], active: 1 });

  // When the app starts
  render(<App />);

  // Then the two remaining songs are back in their tabs, the second in front, without any error
  expect(await screen.findByRole("heading", { name: "Waltz" })).toBeDefined();
  const tabs = () => within(screen.getByRole("tablist", { name: "Songs" })).getAllByRole("tab");
  expect(tabs().map((t) => t.textContent)).toEqual(["Blues", "Waltz"]);
  expect(tabs()[1].getAttribute("aria-selected")).toBe("true");
  expect(screen.queryByRole("alert")).toBeNull();

  // When adding an untitled grid and coming back to the first song
  await clickMenu("File", "New tab");
  await createGrid(user);
  await user.click(tabs()[0]);

  // Then the session keeps the saved songs only, with the first one active
  await vi.waitFor(() => expect(JSON.parse(files["/config/session.json"])).toEqual({ paths: ["/blues.madsister.json", "/waltz.madsister.json"], active: 0 }));
});
