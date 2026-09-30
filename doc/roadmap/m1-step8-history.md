# M1 · Step 8 — Undo/redo

## Goal
Unlimited undo/redo within the session for every edit (F-ED-7).

## Spec refs
F-ED-7, §9 ("undo/redo = command stack").

## Depends on
M1-6.

## Tasks
- `app/src/model/history.ts`: `History = {past: Song[], present: Song, future: Song[]}`; `apply(h, fn: (s: Song) => Song)`,
  `undo(h)`, `redo(h)`, `canUndo/canRedo`. Since Songs are immutable with structural sharing, snapshots are cheap
  (`ponytail:` comment: snapshots instead of inverse commands; revisit only if memory becomes a problem).
- `apply` with a function that returns the same object (no-op) doesn't push history. A new `apply` clears `future`.
- `reset(song)` for opening a file (empty history).
- React hook `useHistory(initial)` in `ui/` (thin wrapper, `useReducer`). No state library.

## Tests
- apply ×3, undo ×2, redo ×1 → expected present; apply after undo clears redo; no-op apply doesn't create an entry.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): add undo/redo history`

## Out of scope
Persisting history across sessions.
