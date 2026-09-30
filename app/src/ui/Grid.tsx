import type { SlotRef } from "../model/commands";
import { displayChord, type DisplayStyle } from "../model/display";
import type { Song } from "../model/song";
import { SectionBlock } from "./SectionBlock";
import "./grid.css";

type Props = {
  song: Song;
  barsPerRow: 2 | 4 | 8;
  style: DisplayStyle;
  cursor: SlotRef | null;
  lowConfidenceThreshold?: number;
};

export const Grid = ({ song, barsPerRow, style, cursor, lowConfidenceThreshold = 0.5 }: Props) => {
  const { title, artist, key, tempoBpm } = song.meta;
  // Roving tabindex: without a cursor, the first slot is the grid's single tab stop.
  const tabStop = cursor ?? { section: 0, bar: 0, slot: 0 };
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
      <div role="grid" aria-label={`${title} chord chart`} className="grid">
        {song.sections.map((section, i) => (
          <SectionBlock
            key={section.id}
            section={section}
            index={i}
            barsPerRow={barsPerRow}
            style={style}
            tabStop={tabStop}
            lowConfidenceThreshold={lowConfidenceThreshold}
          />
        ))}
      </div>
    </article>
  );
};
