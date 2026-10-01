import { useRef, useState, type KeyboardEvent } from "react";
import { parseChord } from "../model/chord";
import { addSection, deleteSection, moveSection, renameSection, setChord, setRepeat, type BarRef, type SlotRef } from "../model/commands";
import type { DisplayStyle } from "../model/display";
import type { Bar, Song } from "../model/song";
import { confirmDeleteSection } from "./fileActions";
import { Grid } from "./Grid";
import { Field } from "./Toolbar";
import { clampCursor, countFlagged, keyToCommand, MOD_LABEL, nextSlot, selectedBars, SHORTCUTS } from "./keymap";
import type { useHistory } from "./useHistory";

type Props = { history: ReturnType<typeof useHistory>; barsPerRow: 2 | 4 | 8; style: DisplayStyle; lowConfidenceThreshold: number };
type Draft = { kind: "chord" | "label"; text: string; error?: string };

/** Keyboard-first editing of the chart (spec F-ED-3..7): cursor, bar selection, clipboard, inline input and help. */
export const Editor = ({ history, barsPerRow, style, lowConfidenceThreshold }: Props) => {
  const { song } = history;
  const [rawCursor, setCursor] = useState<SlotRef>({ section: 0, bar: 0, slot: 0 });
  const [anchor, setAnchor] = useState<BarRef | null>(null);
  const [clipboard, setClipboard] = useState<readonly Bar[]>([]);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [notice, setNotice] = useState("");
  const helpOpener = useRef<HTMLElement | null>(null);
  // Undo/redo and structural edits can leave the cursor dangling: always read it clamped.
  const cursor = clampCursor(song, rawCursor);
  const flaggedCount = countFlagged(song, lowConfidenceThreshold);

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const command = keyToCommand(e, { song, cursor, anchor, clipboard, barsPerRow, lowConfidenceThreshold });
    if (command === null) return;
    e.preventDefault();
    setNotice(command.kind === "refused" ? "Not possible here" : "");
    switch (command.kind) {
      case "move":
        setCursor(command.cursor);
        setAnchor(command.anchor);
        break;
      case "edit":
        history.apply(() => command.song);
        setCursor(command.cursor);
        setAnchor(null);
        break;
      case "copy":
        setClipboard(command.bars);
        break;
      case "type":
        setDraft({ kind: "chord", text: command.text });
        break;
      case "rename":
        setDraft({ kind: "label", text: song.sections[cursor.section].label });
        break;
      case "undo":
        history.undo();
        break;
      case "redo":
        history.redo();
        break;
      case "help":
        helpOpener.current = document.activeElement as HTMLElement | null;
        setIsHelpOpen(true);
        break;
    }
  };

  const commit = (draft: Draft) => {
    if (draft.kind === "label") {
      const label = draft.text.trim();
      if (label === "") return setDraft({ ...draft, error: "a section needs a name" });
      history.apply((s) => renameSection(s, cursor.section, label));
    } else {
      const parsed = parseChord(draft.text);
      if (!parsed.ok) return setDraft({ ...draft, error: parsed.error });
      history.apply((s) => setChord(s, cursor, parsed.harte));
      setCursor(nextSlot(song, cursor, 1));
    }
    setAnchor(null);
    setDraft(null);
  };

  // Mouse/labelled equivalents of the section shortcuts (F-ED-6), acting on the cursor's section.
  const section = song.sections[cursor.section];
  const editSection = (fn: (s: Song) => Song, to = cursor.section) => {
    history.apply(fn);
    setCursor({ section: to, bar: 0, slot: 0 });
    setAnchor(null);
  };
  const removeSection = async () => {
    const hasChords = section.bars.some((b) => b.chords.some((c) => c.chord !== "N"));
    if (hasChords && !(await confirmDeleteSection(section.label))) return;
    editSection((s) => deleteSection(s, cursor.section), Math.max(0, cursor.section - 1));
  };

  const closeHelp = () => {
    setIsHelpOpen(false);
    helpOpener.current?.focus();
  };

  const editor = draft ? (
    <>
      <input
        className="slot-input"
        aria-label={draft.kind === "chord" ? "Chord" : "Section name"}
        aria-invalid={draft.error !== undefined}
        aria-describedby={draft.error === undefined ? undefined : "draft-error"}
        value={draft.text}
        autoFocus
        onFocus={(e) => (draft.kind === "label" ? e.currentTarget.select() : e.currentTarget.setSelectionRange(draft.text.length, draft.text.length))}
        onChange={(e) => setDraft({ kind: draft.kind, text: e.target.value })}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter" || e.key === "Tab") {
            e.preventDefault();
            commit(draft);
          } else if (e.key === "Escape") {
            setDraft(null);
          }
        }}
      />
      {draft.error !== undefined && (
        <span id="draft-error" role="alert" className="slot-error">
          {draft.error}
        </span>
      )}
    </>
  ) : undefined;

  return (
    <>
      <fieldset className="toolbar">
        <legend>Section</legend>
        <Field
          key={section.id}
          label="Name"
          value={section.label}
          check={(t) => (t === "" ? "a section needs a name" : null)}
          onCommit={(label) => history.apply((s) => renameSection(s, cursor.section, label))}
        />
        <label>
          Repeat{" "}
          <input
            type="number"
            min={1}
            value={section.repeat ?? 1}
            onChange={(e) => {
              const n = Number(e.target.value);
              if (Number.isInteger(n) && n >= 1 && n !== (section.repeat ?? 1)) history.apply((s) => setRepeat(s, cursor.section, n));
            }}
          />
        </label>
        <button type="button" disabled={cursor.section === 0} onClick={() => editSection((s) => moveSection(s, cursor.section, cursor.section - 1), cursor.section - 1)}>
          Move up
        </button>
        <button
          type="button"
          disabled={cursor.section === song.sections.length - 1}
          onClick={() => editSection((s) => moveSection(s, cursor.section, cursor.section + 1), cursor.section + 1)}
        >
          Move down
        </button>
        <button type="button" onClick={() => editSection((s) => addSection(s, cursor.section + 1, "New section"), cursor.section + 1)}>
          Add section
        </button>
        <button type="button" onClick={() => void removeSection()}>
          Delete section
        </button>
      </fieldset>
      <Grid
        song={song}
        barsPerRow={barsPerRow}
        style={style}
        cursor={cursor}
        lowConfidenceThreshold={lowConfidenceThreshold}
        selection={anchor ? selectedBars({ cursor, anchor }) : undefined}
        editor={editor}
        onKeyDown={onKeyDown}
        onCellClick={(ref) => {
          setCursor(ref);
          setAnchor(null);
          setDraft(null);
        }}
      />
      <p aria-live="polite" className="review-status">
        {flaggedCount === 0 ? "No chords to review" : `${flaggedCount} chord${flaggedCount === 1 ? "" : "s"} to review`}
      </p>
      <p role="status" className="visually-hidden">
        {notice}
      </p>
      {isHelpOpen && (
        <div role="dialog" aria-modal="true" aria-labelledby="help-title" className="help" onKeyDown={(e) => e.key === "Escape" && closeHelp()}>
          <h2 id="help-title">Keyboard shortcuts</h2>
          <table>
            <tbody>
              {SHORTCUTS.map(([keys, what]) => (
                <tr key={keys}>
                  <th scope="row">
                    <kbd>{keys.replace(/Mod/g, MOD_LABEL)}</kbd>
                  </th>
                  <td>{what}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <button type="button" autoFocus onClick={closeHelp}>
            Close
          </button>
        </div>
      )}
    </>
  );
};
