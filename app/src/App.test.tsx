import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "./App";

test("App_rendersTheMadsisterHeading", () => {
  // Given / When
  render(<App />);

  // Then
  expect(screen.getByRole("heading", { name: "madsister" })).toBeDefined();
});
