# M7 · Step 3 — Sidebar: per-song panels and icon rail

> Written on 2026-10-13 with the user.

## Goal
The sidebar holds only the current song's panels, styled as cards, and collapses to an icon rail.

## Spec refs
F-UI-4.

## Depends on
M7-2.

## Tasks
- Add `lucide-react`. It is the only new dependency of M7. Import icons one by one (tree-shaken).
- Panels, in this order: Song (title, artist, key, notation, tempo), Transpose, then Section and Bar (edit mode only, still portals
  from `Editor.tsx`). Each panel is a card with a header (icon + title) and compact inputs. The Transpose buttons and Add section
  use the neutral style. Only destructive actions (Delete section) use `--danger`.
- Collapse: a button at the top of the sidebar (`aria-expanded`, `aria-controls`) switches between full width and an icon rail.
  The rail shows one button per visible panel (`aria-label` = the panel title). Clicking one expands the sidebar and scrolls to
  that panel.
- The collapsed state is stored in `settings.json` as `isSidebarCollapsed` (default false, handled by `parseSettings`).
- This replaces the "Parameters" toggle in the `<nav>`.

## Tests
vitest:
- `parseSettings`: an invalid `isSidebarCollapsed` falls back to false (enrich the existing `parseSettings` test).
- Given the sidebar is expanded, when the user clicks Collapse, then the panels are hidden, the rail shows Song and Transpose, and
  `onSettingsChange` gets `isSidebarCollapsed: true`.
- Given the sidebar is collapsed, when the user clicks the Transpose rail button, then the sidebar is expanded.
- Given view mode, then the rail has no Section or Bar button.

## Verify
`cd app && pnpm test && pnpm build`.

## Check by hand
- [ ] Collapse and expand, in light and dark mode. The grid takes the freed width.
- [ ] Relaunch → the collapsed state is kept.
- [ ] Keyboard only: Tab reaches the collapse button and the rail buttons. VoiceOver reads their labels.

## Commit message
`feat(app): collapsible per-song sidebar`
