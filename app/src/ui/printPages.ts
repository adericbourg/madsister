/** A run of bars of one section, printed under its (repeated) label. `to` may pass the last bar: slice clamps it. */
export type Chunk = { section: number; from: number; to: number };
/** A page's columns, left to right. */
export type Page = Chunk[][];
/** The first page also holds the chart header, so it has fewer lines. */
export type Layout = { linesPerColumn: number; firstPageLines: number; columnsPerPage: number };

// ponytail: fixed line budgets, because WebKitGTK ignores CSS columns, @page size and margin boxes when printing (the GTK
// dialog picks the paper). Sized for Letter, the smaller page; A4 keeps some slack. Measure at print time if paper varies more.
/** Two columns of 9mm lines, as grid.css sizes them. */
export const COMPACT: Layout = { linesPerColumn: 25, firstPageLines: 21, columnsPerPage: 2 };
/** One column of 15mm lines, as grid.css sizes them. */
export const FULL: Layout = { linesPerColumn: 15, firstPageLines: 12, columnsPerPage: 1 };

/** Fills each column of a page in turn, then the next page. A label and a bar row each take a line. */
export const printPages = (sections: readonly { bars: readonly unknown[] }[], barsPerRow: number, { linesPerColumn, firstPageLines, columnsPerPage }: Layout): Page[] => {
  const columns: Chunk[][] = [[]];
  let free = firstPageLines;
  sections.forEach((section, i) => {
    // An empty section still prints its label and a placeholder row.
    const rows = Math.max(1, Math.ceil(section.bars.length / barsPerRow));
    for (let row = 0; row < rows; ) {
      // A label never ends a column alone.
      if (free < 2) {
        columns.push([]);
        free = columns.length > columnsPerPage ? linesPerColumn : firstPageLines;
      }
      const take = Math.min(rows - row, free - 1);
      columns.at(-1)!.push({ section: i, from: row * barsPerRow, to: (row + take) * barsPerRow });
      row += take;
      free -= take + 1;
    }
  });
  return Array.from({ length: Math.ceil(columns.length / columnsPerPage) }, (_, p) => Array.from({ length: columnsPerPage }, (_, c) => columns[p * columnsPerPage + c] ?? []));
};
