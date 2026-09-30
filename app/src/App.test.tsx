import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "./App";

test("App_rendersAnEmptySongGrid", () => {
  // Given / When
  render(<App />);

  // Then
  expect(screen.getByRole("heading", { name: "Untitled" })).toBeDefined();
  expect(screen.getAllByRole("gridcell", { name: /^Verse, bar \d, beat 1: no chord$/ })).toHaveLength(4);
});
