import { expect, test } from "vitest";
import { pushRecent, shortestUniquePaths } from "./fileActions";

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

test("shortestUniquePaths_keepsJustTheFileNameUnlessSeveralPathsShareIt", () => {
  // Given paths, two of which share a file name (one under the same parent name as well)
  const paths = ["/a/blues.json", "/b/blues.json", "/x/c/rock.json", "/y/c/blues.json", "/solo.json"];

  // When shortening them
  const labels = shortestUniquePaths(paths);

  // Then each one keeps only as many trailing parts as needed to be unique
  expect(labels).toEqual(["a/blues.json", "b/blues.json", "rock.json", "c/blues.json", "solo.json"]);
});
