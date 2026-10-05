import { useEffect, useRef, type KeyboardEventHandler, type ReactNode } from "react";
import type { BarRef, SlotRef } from "../model/commands";
import { displayKey, type DisplayStyle } from "../model/display";
import type { Song } from "../model/song";
import { compactPages } from "./compactPages";
import { SectionBlock, type MenuTarget, type Selection } from "./SectionBlock";
import "./grid.css";

type Props = {
  song: Song;
  barsPerRow: 2 | 4 | 8;
  style: DisplayStyle;
  /** Pitch class of degree 1 when the chart is in Nashville notation. */
  tonic?: number;
  cursor: SlotRef | null;
  /** The bar being played (spec F-PB-1). */
  playing?: BarRef | null;
  lowConfidenceThreshold?: number;
  selection?: Selection;
  /** Edit mode: hovered bars are framed to show they can be edited. */
  isEditable?: boolean;
  /** Rendered in the cursor cell instead of its chord (inline input). */
  editor?: ReactNode;
  /** Also lays the chart out as two-column pages, shown instead of the grid when printing. */
  compactPrint?: boolean;
  onKeyDown?: KeyboardEventHandler<HTMLDivElement>;
  onCellClick?: (ref: SlotRef) => void;
  onSectionClick?: (index: number) => void;
  /** Right-click on a chord, a bar line or a section label; the browser menu is only suppressed when set. */
  onContextMenu?: (target: MenuTarget) => void;
};

export const Grid = ({ song, barsPerRow, style, tonic, cursor, playing = null, lowConfidenceThreshold = 0.5, selection, isEditable = false, editor, compactPrint = false, onKeyDown, onCellClick, onSectionClick, onContextMenu }: Props) => {
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
              Key <b>{displayKey(key, style)}</b>
            </span>
          )}
          {tempoBpm !== undefined && <span>♩ = {tempoBpm}</span>}
        </p>
      </header>
      <div role="grid" aria-label={`${title} chord chart`} className={isEditable ? "grid is-editable" : "grid"} ref={gridRef} onKeyDown={onKeyDown}>
        {song.sections.map((section, i) => (
          <SectionBlock
            key={section.id}
            section={section}
            index={i}
            barsPerRow={barsPerRow}
            style={style}
            tonic={tonic}
            tabStop={tabStop}
            playing={playing}
            lowConfidenceThreshold={lowConfidenceThreshold}
            selection={selection}
            editor={editor}
            onCellClick={onCellClick}
            onSectionClick={onSectionClick}
            onContextMenu={onContextMenu}
          />
        ))}
      </div>
      {compactPrint && (
        <div className="print-pages" aria-hidden="true">
          {compactPages(song.sections, barsPerRow).map((page, p) => (
            <div key={p} className="print-page">
              {page.map((column, c) => (
                <div key={c}>
                  {column.map(({ section, from, to }) => (
                    <SectionBlock
                      key={`${section}-${from}`}
                      section={{ ...song.sections[section], bars: song.sections[section].bars.slice(from, to) }}
                      index={section}
                      barsPerRow={barsPerRow}
                      style={style}
                      tonic={tonic}
                      tabStop={{ section: -1, bar: 0, slot: 0 }}
                      playing={null}
                      lowConfidenceThreshold={0}
                      selection={undefined}
                      editor={undefined}
                    />
                  ))}
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
    </article>
  );
};
