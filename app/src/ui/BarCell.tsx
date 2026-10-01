import type { ReactNode } from "react";
import type { BarRef, SlotRef } from "../model/commands";
import { displayChord, speakChord, type DisplayStyle } from "../model/display";
import type { Bar } from "../model/song";
import type { Selection } from "./SectionBlock";

type Props = {
  bar: Bar;
  at: { section: number; bar: number };
  sectionLabel: string;
  style: DisplayStyle;
  tabStop: SlotRef;
  playing: BarRef | null;
  lowConfidenceThreshold: number;
  selection: Selection;
  editor: ReactNode;
  onCellClick?: (ref: SlotRef) => void;
};

export const BarCell = ({ bar, at, sectionLabel, style, tabStop, playing, lowConfidenceThreshold, selection, editor, onCellClick }: Props) => {
  const isSelected = selection?.section === at.section && at.bar >= selection.from && at.bar <= selection.to;
  const isPlaying = playing?.section === at.section && playing.bar === at.bar;
  const meter = bar.meter && `${bar.meter.beats}/${bar.meter.unit}`;
  let beat = 1;
  return (
    <div className={`bar${isSelected ? " is-selected" : ""}${isPlaying ? " is-playing" : ""}`}>
      {meter && (
        <span className="bar-meter" aria-hidden="true">
          {meter}
        </span>
      )}
      {bar.chords.map((slot, i) => {
        const isLowConfidence = slot.confidence !== undefined && slot.confidence < lowConfidenceThreshold;
        const isTabStop = tabStop.section === at.section && tabStop.bar === at.bar && tabStop.slot === i;
        const isEditing = isTabStop && editor !== undefined;
        const name = `${sectionLabel}, bar ${at.bar + 1}${meter ? ` in ${meter}` : ""}, beat ${beat}: ${speakChord(slot.chord)}${isLowConfidence ? ", low confidence" : ""}`;
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
            onClick={() => onCellClick?.({ ...at, slot: i })}
          >
            {isEditing ? editor : displayChord(slot.chord, style)}
          </div>
        );
      })}
    </div>
  );
};
