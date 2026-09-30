import { useEffect, useRef, type KeyboardEventHandler, type ReactNode } from "react";
import type { SlotRef } from "../model/commands";
import { displayChord, type DisplayStyle } from "../model/display";
import type { Song } from "../model/song";
import { SectionBlock, type Selection } from "./SectionBlock";
import "./grid.css";

type Props = {
  song: Song;
  barsPerRow: 2 | 4 | 8;
  style: DisplayStyle;
  cursor: SlotRef | null;
  lowConfidenceThreshold?: number;
  selection?: Selection;
  /** Rendered in the cursor cell instead of its chord (inline input). */
  editor?: ReactNode;
  onKeyDown?: KeyboardEventHandler<HTMLDivElement>;
  onCellClick?: (ref: SlotRef) => void;
};

export const Grid = ({ song, barsPerRow, style, cursor, lowConfidenceThreshold = 0.5, selection, editor, onKeyDown, onCellClick }: Props) => {
  const { title, artist, key, tempoBpm } = song.meta;
  // Roving tabindex: without a cursor, the first slot is the grid's single tab stop.
  const tabStop = cursor ?? { section: 0, bar: 0, slot: 0 };
  const gridRef = useRef<HTMLDivElement>(null);
  const isEditing = editor !== undefined;
  useEffect(() => {
    // Don't steal focus from the toolbar: only follow the cursor when focus is in the grid or nowhere.
    const isFocusFree = document.activeElement === document.body || gridRef.current?.contains(document.activeElement);
    if (!isEditing && isFocusFree) gridRef.current?.querySelector<HTMLElement>('[tabindex="0"]')?.focus();
  }, [tabStop.section, tabStop.bar, tabStop.slot, song, isEditing]);
  return (
    <article className="chart">
      <header className="chart-header">
        <h1>{title}</h1>
        {artist !== undefined && <p className="chart-artist">{artist}</p>}
        <p className="chart-facts">
          {key !== undefined && (
            <span>
              Key <b>{displayChord(key, style)}</b>
            </span>
          )}
          {tempoBpm !== undefined && <span>♩ = {tempoBpm}</span>}
        </p>
      </header>
      <div role="grid" aria-label={`${title} chord chart`} className="grid" ref={gridRef} onKeyDown={onKeyDown}>
        {song.sections.map((section, i) => (
          <SectionBlock
            key={section.id}
            section={section}
            index={i}
            barsPerRow={barsPerRow}
            style={style}
            tabStop={tabStop}
            lowConfidenceThreshold={lowConfidenceThreshold}
            selection={selection}
            editor={editor}
            onCellClick={onCellClick}
          />
        ))}
      </div>
    </article>
  );
};
