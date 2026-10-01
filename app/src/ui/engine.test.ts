import { beforeEach, expect, test, vi } from "vitest";
import { Channel, invoke } from "@tauri-apps/api/core";
import { cancel, transcribe, type EngineEvent } from "./engine";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: (event: unknown) => void = () => {};
  },
}));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

test("transcribe_invokesTheCommandAndForwardsChannelEvents", async () => {
  // Given the backend returns job 42
  vi.mocked(invoke).mockResolvedValue(42);
  const events: EngineEvent[] = [];

  // When transcribing, then the backend sends an event on the channel
  const jobId = await transcribe("/a.mp3", "/a.madsister.json", 3, true, (event) => events.push(event));
  const [command, args] = vi.mocked(invoke).mock.calls[0] as [string, { onEvent: Channel<EngineEvent> }];
  args.onEvent.onmessage({ type: "progress", stage: "beats", pct: 40 });

  // Then the command gets the paths, meter and sections flag, and the event reaches the callback
  expect(jobId).toBe(42);
  expect(command).toBe("transcribe");
  expect(args).toMatchObject({ audioPath: "/a.mp3", outPath: "/a.madsister.json", meter: 3, sections: true });
  expect(events).toEqual([{ type: "progress", stage: "beats", pct: 40 }]);
});

test("cancel_invokesTheCommandWithTheJobId", async () => {
  // Given / When cancelling job 42
  await cancel(42);

  // Then
  expect(invoke).toHaveBeenCalledWith("cancel", { jobId: 42 });
});
