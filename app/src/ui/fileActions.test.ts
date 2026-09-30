import { expect, test } from "vitest";
import { emptySong } from "../model/song";
import { isDirty, pushRecent } from "./fileActions";

test("pushRecent_movesThePathToTheFrontWithoutDuplicatesAndKeepsTen", () => {
  // Given a full list of 10 paths
  const list = Array.from({ length: 10 }, (_, i) => `/${i}`);

  // When pushing a path already in the list, then a new one
  const moved = pushRecent(list, "/5");
  const pushed = pushRecent(moved, "/new");

  // Then the path moves to the front, and the oldest one drops off
  expect(moved).toEqual(["/5", "/0", "/1", "/2", "/3", "/4", "/6", "/7", "/8", "/9"]);
  expect(pushed).toEqual(["/new", "/5", "/0", "/1", "/2", "/3", "/4", "/6", "/7", "/8"]);
});

test("isDirty_comparesSongsByReference", () => {
  // Given a saved song and an equal but distinct copy
  const saved = emptySong();

  // When / Then only a different reference is dirty
  expect(isDirty(saved, saved)).toBe(false);
  expect(isDirty({ ...saved }, saved)).toBe(true);
});
