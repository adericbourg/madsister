import type { ReactNode } from "react";
import type { BarRef, SlotRef } from "../model/commands";
import type { DisplayStyle } from "../model/display";
import type { Section } from "../model/song";
import { BarCell } from "./BarCell";

export type Selection = { section: number; from: number; to: number } | undefined;

type Props = {
  section: Section;
  index: number;
  barsPerRow: 2 | 4 | 8;
  style: DisplayStyle;
  tabStop: SlotRef;
  playing: BarRef | null;
  lowConfidenceThreshold: number;
  selection: Selection;
  editor: ReactNode;
  onCellClick?: (ref: SlotRef) => void;
};

export const SectionBlock = ({ section, index, barsPerRow, ...cell }: Props) => {
  const repeat = section.repeat ?? 1;
  const rows = Array.from({ length: Math.ceil(section.bars.length / barsPerRow) }, (_, r) => r * barsPerRow);
  return (
    <div role="rowgroup" className="section" aria-label={repeat > 1 ? `${section.label}, ${repeat} times` : section.label}>
      <div className="section-header" aria-hidden="true">
        <span className="section-label">{section.label}</span>
        {repeat > 1 && <span className="section-repeat">x{repeat}</span>}
      </div>
      {rows.map((start) => (
        <div role="row" key={start} className="bar-row" style={{ gridTemplateColumns: `repeat(${barsPerRow}, 1fr)` }}>
          {section.bars.slice(start, start + barsPerRow).map((bar, i) => (
            <BarCell key={start + i} bar={bar} at={{ section: index, bar: start + i }} sectionLabel={section.label} {...cell} />
          ))}
        </div>
      ))}
      {rows.length === 0 && (
        <div role="row" className="bar-row">
          <div role="gridcell" className="section-empty" tabIndex={cell.tabStop.section === index ? 0 : -1}>
            {cell.tabStop.section === index && cell.editor !== undefined ? cell.editor : "No bars yet"}
          </div>
        </div>
      )}
    </div>
  );
};
