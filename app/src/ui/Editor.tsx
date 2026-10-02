import { useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { parseChord } from "../model/chord";
import { addSection, deleteSection, doubleTempo, halveTempo, moveSection, renameSection, setBarMeter, setChord, setRepeat, shiftPhase, splitSlot, type BarRef, type SlotRef } from "../model/commands";
import { displayChord, type DisplayStyle } from "../model/display";
import { displayNashville, parseNashville, tonicPc, type MinorConvention } from "../model/nashville";
import { barAtTime } from "../model/playback";
import type { Bar, Song } from "../model/song";
import { confirmDeleteSection } from "./fileActions";
import { Grid } from "./Grid";
import { Field } from "./Toolbar";
import { clampCursor, countFlagged, keyToCommand, MOD_LABEL, nextSlot, selectedBars, SHORTCUTS } from "./keymap";
import type { useHistory } from "./useHistory";
import { usePlayer } from "./usePlayer";

type Props = {
  history: ReturnType<typeof useHistory>;
  barsPerRow: 2 | 4 | 8;
  style: DisplayStyle;
  minorConvention: MinorConvention;
  lowConfidenceThreshold: number;
  /** Where the section/bar/tempo parameters are rendered; inline when absent. */
  panel?: HTMLElement | null;
  /** The audio file to play, already resolved against the song's location. */
  audioPath?: string;
};
// `select` is where the caret lands on focus: "all" selects the whole text, "end" keeps a typed first character.
type Draft = { kind: "chord" | "label"; text: string; select: "all" | "end"; error?: string };

const minutes = (sec: number) => `${String(Math.floor(sec / 60)).padStart(2, "0")}:${String(Math.floor(sec % 60)).padStart(2, "0")}`;

/** Keyboard-first editing of the chart (spec F-ED-3..7): cursor, bar selection, clipboard, inline input and help. */
export const Editor = ({ history, barsPerRow, style, minorConvention, lowConfidenceThreshold, panel, audioPath }: Props) => {
  const { song } = history;
  // Nashville: chords are typed and shown as degrees of this tonic; undefined = chord names.
  const tonic = song.meta.notation === "nashville" ? tonicPc(song.meta.key, minorConvention) : undefined;
  const parse = (text: string) => (tonic === undefined ? parseChord(text) : parseNashville(text, tonic));
  const show = (harte: string) => (tonic === undefined ? displayChord(harte, "intl") : displayNashville(harte, tonic, "intl"));
  const [rawCursor, setCursor] = useState<SlotRef>({ section: 0, bar: 0, slot: 0 });
  const [anchor, setAnchor] = useState<BarRef | null>(null);
  const [clipboard, setClipboard] = useState<readonly Bar[]>([]);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [notice, setNotice] = useState("");
  const helpOpener = useRef<HTMLElement | null>(null);
  const isCancelled = useRef(false);
  // Undo/redo and structural edits can leave the cursor dangling: always read it clamped.
  const cursor = clampCursor(song, rawCursor);
  const flaggedCount = countFlagged(song, lowConfidenceThreshold);
  const player = usePlayer(audioPath);

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
        setDraft({ kind: "chord", text: command.text, select: "end" });
        break;
      case "rename":
        setDraft({ kind: "label", text: song.sections[cursor.section].label, select: "all" });
        break;
      case "undo":
        history.undo();
        break;
      case "redo":
        history.redo();
        break;
      case "play":
        if (!player.toggle()) setNotice("No audio to play");
        break;
      case "playFromCursor": {
        const startSec = song.sections[cursor.section].bars[cursor.bar]?.startSec;
        if (startSec === undefined) setNotice("This bar has no time in the audio");
        else if (!player.seek(startSec)) setNotice("No audio to play");
        else if (!player.isPlaying) player.toggle();
        break;
      }
      case "help":
        helpOpener.current = document.activeElement as HTMLElement | null;
        setIsHelpOpen(true);
        break;
    }
  };

  // `isBlur`: the input lost focus, so an invalid draft is dropped rather than flagged, and the cursor stays where the click put it.
  const commit = (draft: Draft, isBlur = false) => {
    if (draft.kind === "label") {
      const label = draft.text.trim();
      if (label === "") return isBlur ? setDraft(null) : setDraft({ ...draft, error: "a section needs a name" });
      if (label !== song.sections[cursor.section].label) history.apply((s) => renameSection(s, cursor.section, label));
    } else {
      const parsed = parse(draft.text);
      if (!parsed.ok) return isBlur ? setDraft(null) : setDraft({ ...draft, error: parsed.error });
      const current = song.sections[cursor.section].bars[cursor.bar].chords[cursor.slot];
      // Re-typing a flagged chord confirms it (setChord drops the confidence), so only an unflagged no-op is skipped.
      if (parsed.harte !== current.chord || current.confidence !== undefined) history.apply((s) => setChord(s, cursor, parsed.harte));
      if (!isBlur) setCursor(nextSlot(song, cursor, 1));
    }
    setAnchor(null);
    setDraft(null);
  };

  // Mod+/ while typing: sets the chord, splits the slot and edits the new half, in one undo step.
  const commitAndSplit = (draft: Draft) => {
    const parsed = parse(draft.text);
    if (!parsed.ok) return setDraft({ ...draft, error: parsed.error });
    // Computed before apply: splitSlot refuses a 1-beat slot and must not throw inside the reducer.
    let next: Song;
    try {
      next = splitSlot(setChord(song, cursor, parsed.harte), cursor);
    } catch {
      return setNotice("Not possible here");
    }
    isCancelled.current = true;
    history.apply(() => next);
    setCursor({ ...cursor, slot: cursor.slot + 1 });
    setDraft({ kind: "chord", text: show(parsed.harte), select: "all" });
  };

  // Mouse/labelled equivalents of the section shortcuts (F-ED-6), acting on the cursor's section.
  const section = song.sections[cursor.section];
  const cursorBar = section.bars[cursor.bar];
  const barMeter = cursorBar && (cursorBar.meter ?? song.meta.meter);
  const editSection = (fn: (s: Song) => Song, to = cursor.section) => {
    history.apply(fn);
    setCursor({ section: to, bar: 0, slot: 0 });
    setAnchor(null);
  };
  const selectSection = (i: number) => {
    setCursor({ section: i, bar: 0, slot: 0 });
    setAnchor(null);
    setDraft(null);
  };
  const removeSection = async () => {
    const hasChords = section.bars.some((b) => b.chords.some((c) => c.chord !== "N"));
    if (hasChords && !(await confirmDeleteSection(section.label))) return;
    editSection((s) => deleteSection(s, cursor.section), Math.max(0, cursor.section - 1));
  };

  // Computed before apply: history.apply runs in a reducer, where a refused shift must not throw.
  const shift = (delta: 1 | -1, from?: BarRef) => {
    try {
      const next = shiftPhase(song, delta, from);
      history.apply(() => next);
      setNotice("");
    } catch (e) {
      setNotice((e as Error).message);
    }
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
        onFocus={(e) => {
          isCancelled.current = false;
          if (draft.select === "all") e.currentTarget.select();
          else e.currentTarget.setSelectionRange(draft.text.length, draft.text.length);
        }}
        onBlur={() => !isCancelled.current && commit(draft, true)}
        onChange={(e) => setDraft({ kind: draft.kind, text: e.target.value, select: draft.select })}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (draft.kind === "chord" && (e.metaKey || e.ctrlKey) && e.key === "/") {
            e.preventDefault();
            commitAndSplit(draft);
          } else if (e.key === "Enter" || e.key === "Tab") {
            e.preventDefault();
            commit(draft);
          } else if (e.key === "Escape") {
            isCancelled.current = true;
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

  const inPanel = (node: ReactNode) => (panel ? createPortal(node, panel) : node);

  return (
    <>
      {inPanel(
        <>
          <fieldset className="toolbar">
            <legend>Section</legend>
            <select aria-label="Section" value={cursor.section} onChange={(e) => selectSection(Number(e.target.value))}>
              {song.sections.map((s, i) => (
                <option key={s.id} value={i}>
                  {s.label}
                  {(s.repeat ?? 1) > 1 ? ` ×${s.repeat}` : ""}
                </option>
              ))}
            </select>
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
          <fieldset className="toolbar" disabled={barMeter === undefined}>
            <legend>Bar</legend>
            <label>
              Beats in this bar{" "}
              <input
                type="number"
                min={1}
                value={barMeter?.beats ?? ""}
                onChange={(e) => {
                  const n = Number(e.target.value);
                  if (barMeter && Number.isInteger(n) && n >= 1 && n !== barMeter.beats)
                    history.apply((s) => setBarMeter(s, cursor, { beats: n, unit: barMeter.unit }));
                }}
              />
            </label>{" "}
            /{barMeter?.unit ?? song.meta.meter.unit}
          </fieldset>
          <fieldset className="toolbar">
            <legend>Bar lines</legend>
            <button type="button" onClick={() => shift(-1)}>
              Whole song −1 beat
            </button>
            <button type="button" onClick={() => shift(1)}>
              Whole song +1 beat
            </button>
            <button type="button" disabled={cursorBar === undefined} onClick={() => shift(-1, cursor)}>
              From this bar −1 beat
            </button>
            <button type="button" disabled={cursorBar === undefined} onClick={() => shift(1, cursor)}>
              From this bar +1 beat
            </button>
          </fieldset>
          <fieldset className="toolbar">
            <legend>Tempo</legend>
            <button type="button" onClick={() => history.apply((s) => halveTempo(s))}>
              Whole song half tempo
            </button>
            <button type="button" onClick={() => history.apply((s) => doubleTempo(s))}>
              Whole song double tempo
            </button>
            <button type="button" disabled={cursorBar === undefined} onClick={() => history.apply((s) => halveTempo(s, cursor))}>
              From this bar half tempo
            </button>
            <button type="button" disabled={cursorBar === undefined} onClick={() => history.apply((s) => doubleTempo(s, cursor))}>
              From this bar double tempo
            </button>
          </fieldset>
        </>,
      )}
      {song.audio !== undefined && (
        <fieldset className="toolbar playback" disabled={!player.isReady}>
          <legend>Playback</legend>
          <audio {...player.audioProps} />
          <button type="button" onClick={player.toggle}>
            {player.isPlaying ? "Pause" : "Play"}
          </button>
          <input
            type="range"
            aria-label="Seek"
            min={0}
            max={player.duration}
            step={0.1}
            value={Math.min(player.time, player.duration)}
            onChange={(e) => player.seek(Number(e.target.value))}
          />
          <span>
            {minutes(player.time)}/{minutes(player.duration)}
          </span>
          {player.error !== null && (
            <span role="alert" className="field-error">
              {player.error}
            </span>
          )}
        </fieldset>
      )}
      <Grid
        song={song}
        barsPerRow={barsPerRow}
        style={style}
        tonic={tonic}
        cursor={cursor}
        playing={barAtTime(song, player.time)}
        lowConfidenceThreshold={lowConfidenceThreshold}
        selection={anchor ? selectedBars({ cursor, anchor }) : undefined}
        editor={editor}
        onKeyDown={onKeyDown}
        onSectionClick={selectSection}
        onCellClick={(ref) => {
          const startSec = song.sections[ref.section].bars[ref.bar].startSec;
          if (startSec !== undefined) player.seek(startSec);
          setCursor(ref);
          setAnchor(null);
          const chord = song.sections[ref.section].bars[ref.bar].chords[ref.slot].chord;
          setDraft({ kind: "chord", text: show(chord), select: "all" });
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
