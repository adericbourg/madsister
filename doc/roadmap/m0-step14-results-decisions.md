# M0 · Step 14 — Run the benchmark, record decisions

## Goal
Meet the M0 "done when" as far as the proxy bench allows, and record everything so the user can review it in the morning.

## Spec refs
§8 M0 "done when", D6, D7, NF-2, NF-6, every VERIFY in §2.1 and §4.1.

## Depends on
M0-13.

## Tasks
1. Run the full bench (all combos × all selected GuitarSet takes). Run it in the background with the resumable CSV; while it runs,
   do the research in task 3. If it can't finish tonight, commit partial results and continue with M1, then come back.
2. `doc/roadmap/m0-results.md`: environment (machine, CPU/GPU, versions), dataset + count, the mean table per combo (all metrics +
   time, extrapolated to a 4-min song for NF-2), and the observations (failure modes seen, e.g. half/double tempo).
3. VERIFY research (web/Context7, record the source URLs): BTC weights (from M0-7), all-in-one weights license (M0-6),
   ChordFormer code/weights availability + license (research only), slash-chord support per model (Q10), WebKitGTK/WKWebView
   `<audio>` note stays for M1/M3.
4. Decisions, all marked **provisional (proxy bench, GuitarSet)**:
   - default combo = lowest "edits needed" among the combos meeting NF-2 (< 2 min for 4 min of audio). Set it as the `auto`
     preference in `pipeline.py`.
   - D7 candidates: `fast` = default; `accurate` = the best "edits needed" combo regardless of time, **only if** it's measurably better
     (state the margin). Otherwise note "single mode".
   - D6: "undecided — needs the user's bench with ≥ 3 add2/add4 songs". Report the false-positive rate per τ from GuitarSet.
5. Spec edits (`doc/product-brief.md`): replace the resolved VERIFY markers with the findings + a link to `m0-results.md`;
   add a "M0 results (provisional)" subsection under §8 with the chosen defaults and the D7 candidates. Don't rewrite anything else.
6. `THIRD_PARTY.md`: complete (every dependency group + weights). 
7. Update the M2/M4b/M6 roadmap step files if the decisions change them (e.g. which optional groups the app needs).

## Verify
- `m0-results.md` has a row for each combo that ran; the spec has no VERIFY left that M0 could resolve.
- `cd engine && uv run pytest -m "not slow"` still passes after the default change.

## Commit message
`docs: record M0 benchmark results and provisional model choices` (the `pipeline.py` default change goes in the same commit,
because the results justify it).

## Out of scope
Final decisions: the user re-runs the bench on their own songs.
