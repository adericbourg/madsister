import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { engineNeedsSetup, setupEngine, type EngineEvent } from "./engine";
import { EngineSetup } from "./EngineSetup";

vi.mock("./engine", () => ({ engineNeedsSetup: vi.fn(), setupEngine: vi.fn() }));

const engineSays = (event: EngineEvent) => act(() => vi.mocked(setupEngine).mock.lastCall![0](event));

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(setupEngine).mockResolvedValue(7);
});

test("EngineSetup_whenTheEngineIsReady_showsItsChildrenWithoutSetup", async () => {
  // Given a ready engine (dev build, or set up already)
  vi.mocked(engineNeedsSetup).mockResolvedValue(false);

  // When
  render(<EngineSetup>importer</EngineSetup>);

  // Then
  expect(await screen.findByText("importer")).toBeDefined();
  expect(setupEngine).not.toHaveBeenCalled();
});

test("EngineSetup_whenTheEngineNeedsSetup_runsItWithProgressThenShowsItsChildren", async () => {
  // Given a first launch
  vi.mocked(engineNeedsSetup).mockResolvedValue(true);

  // When
  render(<EngineSetup>importer</EngineSetup>);

  // Then the setup starts at once, installing with an indeterminate progress bar
  expect((await screen.findByRole("status")).textContent).toBe("Installing Python and the engine's libraries…");
  expect(screen.getByRole("progressbar").hasAttribute("value")).toBe(false);
  expect(screen.queryByText("importer")).toBeNull();

  // When the models download
  await engineSays({ type: "progress", stage: "setup", pct: 50 });

  // Then
  expect(screen.getByRole("status").textContent).toBe("Downloading the models…");
  expect(screen.getByRole("progressbar").getAttribute("value")).toBe("50");

  // When the setup ends
  await engineSays({ type: "result", path: "/models" });

  // Then the children replace the setup
  expect(screen.getByText("importer")).toBeDefined();
});

test("EngineSetup_whenTheSetupFails_showsTheErrorAndRetries", async () => {
  // Given a first launch whose setup fails
  const user = userEvent.setup();
  vi.mocked(engineNeedsSetup).mockResolvedValue(true);
  render(<EngineSetup>importer</EngineSetup>);
  await screen.findByRole("progressbar");
  await engineSays({ type: "error", message: "no network", stderr: "uv: dns error" });

  // Then
  expect(screen.getByRole("alert").textContent).toContain("The engine setup failed: no network");
  expect(screen.getByText("uv: dns error")).toBeDefined();
  expect(screen.queryByRole("progressbar")).toBeNull();

  // When retrying
  await user.click(screen.getByRole("button", { name: "Retry" }));

  // Then the setup runs again
  expect(setupEngine).toHaveBeenCalledTimes(2);
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.getByRole("progressbar")).toBeDefined();
});
