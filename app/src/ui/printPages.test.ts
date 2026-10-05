import { expect, test } from "vitest";
import { COMPACT, printPages } from "./printPages";

const section = (bars: number) => ({ bars: Array.from({ length: bars }) });
const twoColumns = (linesPerColumn: number, firstPageLines: number) => ({ linesPerColumn, firstPageLines, columnsPerPage: 2 });

test("fills the left column, then the right one, then the next page", () => {
  // Given three 2-row sections (3 lines each), 3 lines per column on the first page and 6 after
  const pages = printPages([section(8), section(8), section(8)], 4, twoColumns(6, 3));

  // Then page 1 holds one section per column, page 2 the third one
  expect(pages).toEqual([
    [[{ section: 0, from: 0, to: 8 }], [{ section: 1, from: 0, to: 8 }]],
    [[{ section: 2, from: 0, to: 8 }], []],
  ]);
});

test("splits a long section between rows and repeats its label", () => {
  // Given a 5-row section and 4-line columns
  const pages = printPages([section(10)], 2, twoColumns(4, 4));

  // Then it continues in the right column, under its label again
  expect(pages).toEqual([[[{ section: 0, from: 0, to: 6 }], [{ section: 0, from: 6, to: 10 }]]]);
});

test("never leaves a label alone at the bottom of a column", () => {
  // Given a first section leaving one free line
  const pages = printPages([section(4), section(4)], 4, twoColumns(3, 3));

  // Then the second section starts the right column
  expect(pages[0]).toEqual([[{ section: 0, from: 0, to: 4 }], [{ section: 1, from: 0, to: 4 }]]);
});

test("gives an empty section its label and a placeholder row", () => {
  expect(printPages([section(0)], 4, COMPACT)).toEqual([[[{ section: 0, from: 0, to: 4 }], []]]);
});

test("lays out one column per page", () => {
  // Given two 2-row sections, 3 lines on the first page and 4 after
  const pages = printPages([section(8), section(8)], 4, { linesPerColumn: 4, firstPageLines: 3, columnsPerPage: 1 });

  // Then each section gets its own page
  expect(pages).toEqual([[[{ section: 0, from: 0, to: 8 }]], [[{ section: 1, from: 0, to: 8 }]]]);
});
