# Current sprint, S46

**Milestone**: M14, out-of-core volume streaming, and M20, local folder viewer.
**Branch**: `sprint/s46`
**Opened**: 2026-11-05
**Goal**: Brick decomposition and multiscale pyramid generation, and the folder viewer at the level of the viewers radiologists use.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-119 | E30.1 | Brick decomposition and multiscale pyramid generation | Rust | 10w | pending |
| F-208 | E38.18 | Folder viewer viewports: layouts from 1 by 1 to 3 by 3, per-viewport series and view state, an active viewport, a series dragged from its thumbnail onto a cell, and the four-image view kept as a tile mode | TS | 6w | pending |
| F-209 | E38.19 | Folder viewer linked viewports: scroll by position in a shared frame of reference, synced window and zoom, and reference lines from the library | TS | 4w | pending |
| F-210 | E38.20 | Folder viewer value at the pointer: the modality value and its unit from the modality stage of the LUT chain, HU on CT, returned by the local process | Rust | 3w | pending |
| F-211 | E38.21 | Folder viewer measurements: angle, Cobb angle, bidirectional, arrow and probe, region area with mean, SD, minimum and maximum in modality units, editable handles and a measurement list | All | 6w | pending |
| F-212 | E38.22 | Folder viewer four corners at parity: birth date, sex, age, study time, institution, slice location and thickness, kVp and mAs, TR and TE, a scale bar, and edge letters from Patient Orientation on projection images | All | 4w | pending |
| F-213 | E38.23 | Folder viewer window presets by modality on number keys, every VOI window in the file listed by its explanation, and a region window from the library | All | 3w | pending |
| F-214 | E38.24 | Folder viewer mouse profiles and preferences: the current binding, a scroll-on-left profile and a window-on-left profile, saved in the browser, and cine at Frame Time (0018,1063) | TS | 2w | pending |
| F-215 | E38.25 | Folder viewer marks written as a TID 1500 document and key images as a Key Object Selection through @ocelli/core, saved to a download and loaded back from a folder | TS | 4w | pending |
| F-216 | E38.26 | Folder viewer performance: each file posted once and held in a bounded cache, decode reused across window changes, measured budgets for open, first image, scroll rate and resident memory, and a release build | All | 4w | pending |
| F-217 | E38.27 | Licence hygiene: no copyleft dependency in any tracked manifest, a permissive JPEG reference decoder for the corpus, licence allowlist gates for Rust, npm and Python, and a third-party notices file | All | 2w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S46 / {print $2, $9}'
```

## What this sprint is

S45 is closed at tag `s45`. That close delivered F-206 and F-207 and published nothing.
This sprint starts M14. F-119 decomposes a volume into bricks and generates
the multiscale pyramid. F-058 is done, so the volume model already has the
level axis this story fills. F-208 to F-216 bring the local folder viewer to the
baseline of the viewers radiologists use. They were found by a browser test
on 2026-11-05 and the operator placed them here. The operator assigned the
implementation of every story in this sprint to an external coding agent,
with design, review and finalisation in Claude. Closing this sprint does not publish.

## Review findings carried into a story

- **F-207 polish review S1, carried into F-208.** The display aspect has no
  behavioural test. Swapping `aspectX` and `aspectY` where they are assigned in
  `examples/folder-viewer/src/page.html` still passes every test, because the
  only checks are pinned source strings. The fix needs a headless browser
  harness, and F-208 is the story that builds one for its viewport checks. The
  lead carried it under the operator's instruction of 2026-11-05, "we cannot
  compromise the accuracy, quality and robustness so you make a call". It is
  not waived. F-208 carries it as a numbered
  acceptance row: the single view and the first tile of the synthetic study
  measure 64 to 67.2 within 1e-3, and the assignment swap must turn that row
  red. F-208 does not complete while that row is open.

## What is carried in

- **D-22 still blocks publication.** The version stays 0.1.0.
- **One cache.** A brick is a key in `ocelli-cache`. There is no second cache.
- **Level 0 is full resolution.** A series that fits in memory still has the level axis.
- **F-202 to F-205 stay unscheduled.** This sprint does not build them.

## The defect class this sprint is exposed to

A child level that is not a downsample of level 0. Spacing on that level copied from slice thickness instead of the dimension ratio. A missing level that returns level 0. A second brick edge beside `BrickGrid::new`.

## What done means

F-119 generates the coarser levels by downsampling level 0. `append_level` stores caller-supplied bytes and does not downsample, and this story is that downsample. Level 0 stays the full grid. Child spacing is level 0 spacing times the level 0 dimension divided by the child dimension, on each axis. The brick edge stays the one `BrickGrid::new` already chooses. Opacity correction stays F-121.
