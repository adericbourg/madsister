import { describe, expect, it } from "vitest";
import { mergeSettings } from "./Toolbar";

describe("mergeSettings", () => {
  it("puts the overrides on top of the saved settings", () => {
    const settings = mergeSettings({ font: "kalam", barsPerRow: 8, compactPrint: true }, { barsPerRow: 2, compactPrint: false });
    expect(settings).toMatchObject({ font: "kalam", barsPerRow: 2, compactPrint: false });
  });

  it("uses the defaults when nothing is saved", () => {
    expect(mergeSettings(null, { style: "intl" })).toMatchObject({ style: "intl", barsPerRow: 4 });
  });

  it("rejects an invalid override instead of falling back to the default", () => {
    expect(() => mergeSettings({}, { font: "comic" })).toThrow('invalid --font: "comic"');
    expect(() => mergeSettings({}, { barsPerRow: 5 })).toThrow("invalid --bars-per-row: 5");
  });
});
