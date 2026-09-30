# M3 · Step 4 — Double/half tempo fix

> Coarse step. Refine before starting.

## Goal
F-PB-4: merge pairs of bars (tracker found double tempo) or split each bar in two (half tempo), in one action.

## Spec refs
F-PB-4, §8 M3 "done when".

## Depends on
M3-3 (reuses the flatten/re-group helpers).

## Tasks
- `halveBars(song)`: merge bars 2 by 2; each slot's beats halved (integer beats: a slot with an odd beat count → round, keeping the bar sum;
  document the rounding), `startSec` of the first of each pair, `meta.tempoBpm` halved.
- `doubleBars(song)`: each bar split into two bars; slot beats doubled; the second bar's `startSec` = midpoint; `meta.tempoBpm` doubled.
- Toolbar buttons + undo.

## Tests
- Pure tests for both, including an odd bar count and slot rounding; double then halve = identity when there's no rounding.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): fix double or half tempo in one action`
