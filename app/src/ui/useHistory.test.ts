import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { emptySong } from "../model/song";
import { useHistory } from "./useHistory";

test("useHistory_appliesUndoesRedoesAndResets", () => {
  // Given a hook on an initial song
  const [s0, s1, s2] = [emptySong(), emptySong(), emptySong()];
  const { result } = renderHook(() => useHistory(s0));

  // When applying an edit then undoing
  act(() => result.current.apply(() => s1));
  act(() => result.current.undo());

  // Then the initial song is back and redo is available
  expect(result.current.song).toBe(s0);
  expect(result.current.canRedo).toBe(true);

  // When redoing then resetting to another song
  act(() => result.current.redo());
  expect(result.current.song).toBe(s1);
  act(() => result.current.reset(s2));

  // Then history is empty
  expect(result.current.song).toBe(s2);
  expect(result.current.canUndo).toBe(false);
  expect(result.current.canRedo).toBe(false);
});
