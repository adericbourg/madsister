/** A run of bars of one section, printed under its (repeated) label. `to` may pass the last bar: slice clamps it. */
export type Chunk = { section: number; from: number; to: number };
export type Page = [Chunk[], Chunk[]];

// ponytail: fixed line budget, because WebKitGTK ignores both CSS columns and @page size when printing (the GTK
// dialog picks the paper). Sized for Letter, the smaller page; A4 keeps some slack. Measure at print time if paper varies more.
const LINES_PER_COLUMN = 25;
/** The first page also holds the chart header. */
const FIRST_PAGE_LINES = 21;

/** Compact print: fills the left column, then the right one, then the next page. A label and a bar row each take a line. */
export const compactPages = (sections: readonly { bars: readonly unknown[] }[], barsPerRow: number, linesPerColumn = LINES_PER_COLUMN, firstPageLines = FIRST_PAGE_LINES): Page[] => {
  const columns: Chunk[][] = [[]];
  let free = firstPageLines;
  sections.forEach((section, i) => {
    // An empty section still prints its label and a placeholder row.
    const rows = Math.max(1, Math.ceil(section.bars.length / barsPerRow));
    for (let row = 0; row < rows; ) {
      // A label never ends a column alone.
      if (free < 2) {
        columns.push([]);
        free = columns.length > 2 ? linesPerColumn : firstPageLines;
      }
      const take = Math.min(rows - row, free - 1);
      columns.at(-1)!.push({ section: i, from: row * barsPerRow, to: (row + take) * barsPerRow });
      row += take;
      free -= take + 1;
    }
  });
  return Array.from({ length: Math.ceil(columns.length / 2) }, (_, p) => [columns[2 * p], columns[2 * p + 1] ?? []]);
};
