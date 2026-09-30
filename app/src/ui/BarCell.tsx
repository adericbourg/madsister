import type { SlotRef } from "../model/commands";
import { displayChord, speakChord, type DisplayStyle } from "../model/display";
import type { Bar } from "../model/song";

type Props = {
  bar: Bar;
  at: { section: number; bar: number };
  sectionLabel: string;
  style: DisplayStyle;
  tabStop: SlotRef;
  lowConfidenceThreshold: number;
};

export const BarCell = ({ bar, at, sectionLabel, style, tabStop, lowConfidenceThreshold }: Props) => {
  let beat = 1;
  return (
    <div className="bar">
      {bar.chords.map((slot, i) => {
        const isLowConfidence = slot.confidence !== undefined && slot.confidence < lowConfidenceThreshold;
        const isTabStop = tabStop.section === at.section && tabStop.bar === at.bar && tabStop.slot === i;
        const name = `${sectionLabel}, bar ${at.bar + 1}, beat ${beat}: ${speakChord(slot.chord)}${isLowConfidence ? ", low confidence" : ""}`;
        beat += slot.beats;
        return (
          <div
            key={i}
            role="gridcell"
            aria-label={name}
            tabIndex={isTabStop ? 0 : -1}
            className={isLowConfidence ? "slot is-low-confidence" : "slot"}
            style={{ flexGrow: slot.beats }}
          >
            {displayChord(slot.chord, style)}
          </div>
        );
      })}
    </div>
  );
};
