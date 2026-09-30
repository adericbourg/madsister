import { expect, test } from "vitest";
import { apply, canRedo, canUndo, redo, reset, undo } from "./history";
import { emptySong } from "./song";

test("apply_undo_redo_navigatesSnapshots", () => {
  // Given a history with three applied edits
  const [s0, s1, s2, s3] = [emptySong(), emptySong(), emptySong(), emptySong()];
  let h = reset(s0);
  expect(canUndo(h)).toBe(false);
  h = apply(apply(apply(h, () => s1), () => s2), () => s3);

  // When undoing twice then redoing once
  h = redo(undo(undo(h)));

  // Then the present is the second edit, both directions are available
  expect(h.present).toBe(s2);
  expect(canUndo(h)).toBe(true);
  expect(canRedo(h)).toBe(true);
});

test("apply_whenAfterUndo_clearsRedo", () => {
  // Given an undone edit
  const [s0, s1, s2] = [emptySong(), emptySong(), emptySong()];
  const h = undo(apply(reset(s0), () => s1));

  // When applying a new edit
  const next = apply(h, () => s2);

  // Then redo is no longer possible, undo returns to the start
  expect(canRedo(next)).toBe(false);
  expect(undo(next).present).toBe(s0);
});

test("apply_ofNoOp_createsNoEntry", () => {
  // Given a fresh history
  const h = reset(emptySong());

  // When applying a function returning the same song
  const next = apply(h, (s) => s);

  // Then nothing is recorded; undo/redo at the edges are no-ops
  expect(next).toBe(h);
  expect(undo(h)).toBe(h);
  expect(redo(h)).toBe(h);
});
