# Backlog

Needs the user reported that no roadmap step covers yet. Newest first.

## Detect short break bars (e.g. one 2/4 bar in a 4/4 song) — 2026-10-01
- User: "I often find 2-beat bars as a break."
- Today: the file format supports it (`Bar.meter` override), and M2-5 lets the user set it by hand. Transcription doesn't
  detect it: madmom's DBN tracker uses a fixed `beats_per_bar`, so a 2-beat break shifts the bar phase for the rest of the
  song instead of producing one short bar (spec §11.3: meter-change detection is out of scope).
- Ideas: compare downbeat-phase likelihoods around the break (madmom's activation), or detect a phase jump after the fact
  and propose "insert a 2-beat bar here" as a one-click fix (close to F-PB-3's phase nudge, M3-3).
- Until then: M2-5 should make "set this bar to 2 beats" fast (one shortcut), and M3-3's phase nudge should be usable from a
  given bar onward, not only for the whole song, since that's exactly the repair a break bar needs.
