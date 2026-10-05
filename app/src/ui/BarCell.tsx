import type { ReactNode } from "react";
import type { BarRef, SlotRef } from "../model/commands";
import { displayChord, speakChord, type DisplayStyle } from "../model/display";
import { displayNashville, speakNashville } from "../model/nashville";
import type { Bar } from "../model/song";
import type { MenuTarget, Selection } from "./SectionBlock";

type Props = {
  bar: Bar;
  at: { section: number; bar: number };
  sectionLabel: string;
  style: DisplayStyle;
  /** Pitch class of degree 1 when the chart is in Nashville notation. */
  tonic?: number;
  tabStop: SlotRef;
  playing: BarRef | null;
  lowConfidenceThreshold: number;
  selection: Selection;
  editor: ReactNode;
  onCellClick?: (ref: SlotRef) => void;
  onContextMenu?: (target: MenuTarget) => void;
};

/** Bar lines are borders, not elements: a right-click this close to a bar's edge is on the line. */
const LINE_REACH_PX = 6;

export const BarCell = ({ bar, at, sectionLabel, style, tonic, tabStop, playing, lowConfidenceThreshold, selection, editor, onCellClick, onContextMenu }: Props) => {
  const isSelected = selection?.section === at.section && at.bar >= selection.from && at.bar <= selection.to;
  const isPlaying = playing?.section === at.section && playing.bar === at.bar;
  const meter = bar.meter && `${bar.meter.beats}/${bar.meter.unit}`;
  let beat = 1;
  return (
    <div
      className={`bar${isSelected ? " is-selected" : ""}${isPlaying ? " is-playing" : ""}`}
      onContextMenu={
        onContextMenu &&
        ((e) => {
          e.preventDefault();
          const { left, right } = e.currentTarget.getBoundingClientRect();
          if (e.clientX - left <= LINE_REACH_PX) return onContextMenu({ kind: "separator", section: at.section, bar: at.bar });
          if (right - e.clientX <= LINE_REACH_PX) return onContextMenu({ kind: "separator", section: at.section, bar: at.bar + 1 });
          // The slot under the pointer; a click on the bar's padding (meter label) falls back to its first slot.
          const slot = (e.target as HTMLElement).closest<HTMLElement>('[role="gridcell"]');
          const index = slot ? Array.from(e.currentTarget.querySelectorAll('[role="gridcell"]')).indexOf(slot) : 0;
          onContextMenu({ kind: "slot", ref: { ...at, slot: Math.max(index, 0) } });
        })
      }
    >
      {meter && (
        <span className="bar-meter" aria-hidden="true">
          {meter}
        </span>
      )}
      {bar.chords.map((slot, i) => {
        const isLowConfidence = slot.confidence !== undefined && slot.confidence < lowConfidenceThreshold;
        const isTabStop = tabStop.section === at.section && tabStop.bar === at.bar && tabStop.slot === i;
        const isEditing = isTabStop && editor !== undefined;
        const name = `${sectionLabel}, bar ${at.bar + 1}${meter ? ` in ${meter}` : ""}, beat ${beat}: ${tonic === undefined ? speakChord(slot.chord) : speakNashville(slot.chord, tonic)}${isLowConfidence ? ", low confidence" : ""}`;
        beat += slot.beats;
        return (
          <div
            key={i}
            role="gridcell"
            aria-label={name}
            aria-selected={isSelected || undefined}
            tabIndex={isTabStop ? 0 : -1}
            className={`slot${isLowConfidence ? " is-low-confidence" : ""}${isEditing ? " is-editing" : ""}`}
            style={{ flexGrow: slot.beats }}
            onClick={() => !isEditing && onCellClick?.({ ...at, slot: i })}
          >
            {isEditing ? editor : tonic === undefined ? displayChord(slot.chord, style) : displayNashville(slot.chord, tonic, style)}
          </div>
        );
      })}
    </div>
  );
};
