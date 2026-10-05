import { expect, test } from "vitest";
import { compactPages } from "./compactPages";

const section = (bars: number) => ({ bars: Array.from({ length: bars }) });

test("fills the left column, then the right one, then the next page", () => {
  // Given three 2-row sections (3 lines each), 3 lines per column on the first page and 6 after
  const pages = compactPages([section(8), section(8), section(8)], 4, 6, 3);

  // Then page 1 holds one section per column, page 2 the third one
  expect(pages).toEqual([
    [[{ section: 0, from: 0, to: 8 }], [{ section: 1, from: 0, to: 8 }]],
    [[{ section: 2, from: 0, to: 8 }], []],
  ]);
});

test("splits a long section between rows and repeats its label", () => {
  // Given a 5-row section and 4-line columns
  const pages = compactPages([section(10)], 2, 4, 4);

  // Then it continues in the right column, under its label again
  expect(pages).toEqual([[[{ section: 0, from: 0, to: 6 }], [{ section: 0, from: 6, to: 10 }]]]);
});

test("never leaves a label alone at the bottom of a column", () => {
  // Given a first section leaving one free line
  const pages = compactPages([section(4), section(4)], 4, 3, 3);

  // Then the second section starts the right column
  expect(pages[0]).toEqual([[{ section: 0, from: 0, to: 4 }], [{ section: 1, from: 0, to: 4 }]]);
});

test("gives an empty section its label and a placeholder row", () => {
  expect(compactPages([section(0)], 4, 25, 21)).toEqual([[[{ section: 0, from: 0, to: 4 }], []]]);
});
