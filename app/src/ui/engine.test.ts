import { beforeEach, expect, test, vi } from "vitest";
import { Channel, invoke } from "@tauri-apps/api/core";
import { cancel, engineNeedsSetup, fetchAudio, record, setupEngine, stop, transcribe, type EngineEvent } from "./engine";

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
  const jobId = await transcribe("/a.mp3", "/a.madsister.json", "6/8", true, (event) => events.push(event));
  const [command, args] = vi.mocked(invoke).mock.calls[0] as [string, { onEvent: Channel<EngineEvent> }];
  args.onEvent.onmessage({ type: "progress", stage: "beats", pct: 40 });

  // Then the command gets the paths, meter and sections flag, and the event reaches the callback
  expect(jobId).toBe(42);
  expect(command).toBe("transcribe");
  expect(args).toMatchObject({ audioPath: "/a.mp3", outPath: "/a.madsister.json", meter: "6/8", sections: true });
  expect(events).toEqual([{ type: "progress", stage: "beats", pct: 40 }]);
});

test("fetchAudio_record_and_setupEngine_invokeTheirCommandsWithAChannel", async () => {
  // Given / When fetching a URL, recording, setting the engine up and checking it
  const onEvent = () => {};
  await fetchAudio("https://x/v", "/data/sources", onEvent);
  await record("/data/sources/r.wav", onEvent);
  await setupEngine(onEvent);
  await engineNeedsSetup();

  // Then each command gets its arguments and a channel
  expect(invoke).toHaveBeenNthCalledWith(1, "fetch", { url: "https://x/v", outDir: "/data/sources", onEvent: expect.any(Object) });
  expect(invoke).toHaveBeenNthCalledWith(2, "record", { outPath: "/data/sources/r.wav", onEvent: expect.any(Object) });
  expect(invoke).toHaveBeenNthCalledWith(3, "setup_engine", { onEvent: expect.any(Object) });
  expect(invoke).toHaveBeenNthCalledWith(4, "engine_needs_setup");
});

test("cancel_and_stop_invokeTheirCommandWithTheJobId", async () => {
  // Given / When cancelling job 42, then stopping job 43
  await cancel(42);
  await stop(43);

  // Then
  expect(invoke).toHaveBeenCalledWith("cancel", { jobId: 42 });
  expect(invoke).toHaveBeenCalledWith("stop", { jobId: 43 });
});
