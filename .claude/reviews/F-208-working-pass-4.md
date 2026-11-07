# F-208 review, pass 4

**Reviewed**: `git diff fba5e580 HEAD` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `8c6be74b9211c7ea414432002a9905cf7545938b`, tree `2c00893f965e8d47fa30e336d3901c5667901f78`, plus one sweep of `git diff bbf624de HEAD -- . ':!.claude/reviews'`. Nothing staged or unstaged before or after the pass. The untracked paths are the pass 1 to 3 reports, present before this pass, and this report.
**Author / reviewer**: the remediation commit is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 4). It wrote none of the diff.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 3 D1, a surviving cell after a layout change. Closed.** 3 by 3 to 2 by 2 by keyboard, pointer still on cell 3 reading `C 51  R 52  -800 HU`: after the change the readout, both cross lines and the magnifier are hidden and `valueState.cell` is -1. One move at the same spot reads `C 26  R 3  -1000 HU` for cell 2, which is the cell under the pointer, with the vertical line inside cell 2's box. The held-reply variant (1.5 s on `/value`) stays hidden after the reply lands, because the generation moved (8 to 9), and the next move repaints. Reading the same point again after leaving and coming back gives the same text. The grow (1 by 1 to 2 by 2, case F) is hidden after the change, and once cell 3 holds a series one move reads `C 41  R 35  40 HU` for cell 3 with the magnifier over cell 3.
- **Pass 3 D2, the magnifier with the cross off. Closed.** 2 by 1 to 1 by 1, `m` only: `#mag` hidden after the shrink, and one move shows it again over cell 0. With the cross on as well, both readout and magnifier come back on the move.
- **Pass 2 D1 (case A). Still closed.** Generation 4 to 5 on the shrink, hidden after the held reply lands, one move reads `C 103  R 49  40 HU` for cell 0.
- **The `[` and `]` toggles.** 2 by 2, cross and magnifier on, pointer still on cell 3, reply held: hidden after the key and after the reply lands. After `[` one move reads cell 3 at the right C and R with the magnifier in cell 3. After `]` the old spot is cell 3's background (cell 3 widened and the image is centred), so the readout correctly stays hidden, and a move onto cell 3's image reads `C 58  R 44  700 HU` for cell 3. The toolbar button toggle back hides everything too, and the next move into cell 2 repaints it.
- **A layout change with the pointer outside every cell** (over the inspector), by keyboard to 2 by 2 and by mouse to 3 by 2: hidden after both. Moving into cells 0 and 1 repaints the readout and magnifier for each. An empty cell (cell 4) keeps the readout hidden.
- **Mid-measure.** `onKey` returns at its top while a draft exists, so `G` and `[` do nothing during a Length drag (picker stayed hidden, panel stayed shown). The keyboard route that does reach `setLayout` mid-drag is Enter on a focused Layout button, then Enter on a picker button, which are native button activations. With a Length drag in progress in cell 1 at 2 by 2, that route grew the layout to 3 by 3, the draft and the pointer capture survived, and the release added the mark (0 to 1). Enter on the focused Series panel button mid-drag toggled the panel and the release added the mark (1 to 2). An Angle waiting for its second stage survived a shrink to 2 by 2 (cell 1 still active) and the next click finished it (2 to 3). `leavePointer` touches only `valueState`, the cross nodes and `#mag`, never `draft`, `drag`, `move` or `grip`. A shrink that hides the active cell cancels the draft through `setActive(0)`, which is the design's rule for a change of active cell and not this commit's.
- **S1, the narrowed comments.** Each now says exactly what its loop checks, see Verified clean.
- **The relayout path, one adjacent failure.** Fails, D1 below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 2c00893, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer` | same | now | 0, suites 73, 1, 14, 60, 1, 1, 24 passed |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards` | same, OCELLI_PORT=0 | now | ALL GREEN, `viewer-browser.aspect-swapped` and `aspect-no-canvas` both `red` |
| `bin/ocelli.sh gate viewer-browser` | release build, Chromium 151.0.7922.34 | now | ALL GREEN, single view 0.952365, four tiles 0.952346, four cells 0.952363, all 9 within 0.001 of 0.952381 |
| `bin/ocelli.sh gate ci` | same | now | ALL GREEN, `viewer-browser` runs on pull_request, push, workflow_dispatch |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 ("895 file(s)"), 0 ("1757 files") |
| `target/f208-review4/replay.mjs`, cases A, D, D2, E, F, P, O, M, R | release viewer, OCELLI_PORT=0, stopped by PID 20374 | now | `replay-main.log`, 2 FAIL lines both in `P ]` and both the test's own wrong expectation (the old spot is off the image after `]`), re-checked in `extra.log`. No page errors |
| `target/f208-review4/extra.mjs`, `extra2.mjs` | same, PIDs 22321, 23918, 24326 | now | `extra.log`, `extra2.log`, `shift-f.png` |
| `target/f208-review4/bobs.mjs`, B1 to B7 | same, PID 26363 | now | `bobs.log`, `B1.png` to `B7.png` |
| Pass 1 `replay.mjs`, `replay2.mjs`, pass 2 `replay-tile.mjs`, pass 3 `replay.mjs`, copies under `target/f208-review4/rerun/` | same | now | p1 replay2 and p2 tile logs identical to their originals. p1 replay identical to pass 3's re-run. p3 replay differs from pass 3's log only where pass 3 recorded D1 and D2 and the `[` relative, which now read hidden, and in generation counts |
| `target/f208/replay4/log.txt`, `panel.txt` | implementer's | prior receipt | read, agrees with D, E, F and `[` |
| Pin mutations for AC13 | pass 3, 23 cases | prior receipt | the token lists are unchanged by this commit, only the comments moved |

## Defects

### D1, a resize of the grid with the pointer still leaves one cell's value printed over another cell
**Where**: `examples/folder-viewer/src/page.html:1194` (`relayout`, run by the window `resize` listener at 1190 and the `ResizeObserver` on `#wrap` at 1191), and `toggleFull` at 4175. The reset is in `setLayout` (4408) and `togglePanel` (1372) only.
**What**: anything that resizes `#wrap` without a pointer event moves every cell and leaves the readout, its value and its cross where the last pointer event put them. Replay, keyboard only: 2 by 2, the series in all four cells, cross on, pointer resting near the bottom-right of cell 0's image reads `C 127  R 95  -1000 HU`. `Shift F`. After it the readout is still visible with that text, `valueState.cell` is 0, the cell under the pointer is cell 2 (on its background), and the vertical line is not in cell 2's box. Window resize from 1600 by 1000 to 1100 by 800, pointer still on cell 0 reading `C 84  R 58  -800 HU`: after it the readout still shows that, visible, while the pointer is over cell 1, and one move reads `C 7  R 69  -1000 HU` for cell 1.
**Why it is wrong**: it is the symptom pass 3 counted as D1, reached by the page's own `Shift F` key. The design's DOM paragraph positions `#cross-h`, `#cross-v` and `#cross-read` "from the cell under the pointer", and its invariant is that no function reads another cell's state. The lead's ruling is that nothing placed under the pointer for the old geometry is kept, and the new `setLayout` comment says "Every cell may have moved, so nothing placed under the pointer for the previous layout is kept". A full-screen change moves every cell just as a layout change does.
**Evidence**: `target/f208-review4/extra2.log` lines "SF before" and "SF after Shift F, pointer still", `shift-f.png`. `target/f208-review4/replay-main.log` lines "R before" and "R after window resize, pointer still", then "PASS R resize move repaints readout".
**Scope and regression**: present on HEAD. The single-cell form (a stale C, R and value on the same image after `Shift F` or a resize) is present at `bbf624de`, where neither `relayout` nor `toggleFull` resets anything, so it is F-207's and F-210's. The cross-cell form needs F-208's grid. Pass 3 recorded the `[` relative as not counted and named `relayout()` as the place that would close it. This commit closed `[` and `]` in `togglePanel` and left the resize path open. Smallest fix: call `leavePointer()` from `relayout()`, which `setLayout` already calls and which the observer runs for `[`, `]`, `Shift F` and a window resize. That makes the `togglePanel` call redundant. Uncertainty: I did not check whether `#wrap` also resizes on unrelated content changes, which would then hide the cross until the next move. That is harmless but visible. Regression: the replay's SF and R cases, asserting the readout hidden after the change and correct after one move.

## Smells

None. Pass 3 S1 is closed (Verified clean). That family ran three passes and now ends.

## Nitpicks

### N1, the reset is held by an observation, not by a committed check
`grep -rn "leavePointer\|cross-read" examples/folder-viewer/tests/` finds nothing. The Rust page test has no needle on it and `aspect.mjs` reads only canvas boxes. So deleting either `leavePointer()` call keeps every committed check green. The design makes browser behaviour an observation and no AC names the readout on a layout change, so this does not block. A later story editing `setLayout` (F-209, F-213) would lose it silently.

### N2, `leavePointer`'s first comment sentence says a little more than a leave does
`page.html:3108`, "The readout, the cross and the magnifier as if the pointer had left." The real leave at 1177 hides the cross and the magnifier and does not reset the value, so a reply in flight survives a real leave and not a `leavePointer`. The second sentence says what differs, so a reader is not misled for long.

## Verified clean

- **S1's comments, against the code they sit on.** The first says the check is the two branch texts and the two `data-key` attributes, which is the loop below it. The second says that in the text of `onKey`, including `clicks` and `map`, `toggleTileMode(` and `openLayoutPicker(` appear exactly once and every other listed token zero times, "That is the whole check", and that a route using none of the tokens is not seen. The slice runs from `function onKey(event) {` to the first column-0 `}` at line 4122, which is `onKey`'s own end, and both literals are inside it. The third says that across the page `setLayout(`, `openLayoutPicker(` and `toggleTileMode` each appear 3 times, their definition and two uses. Executed by grep: `setLayout(` at 1135 (picker), 1238 (start-up), 4386 (definition), `openLayoutPicker(` at 1127, 4109, 4430, `toggleTileMode` at 1144, 1839, 4107. No sentence claims a helper, node or spelling is caught.
- **`leavePointer` at start-up.** `setLayout(1, 1)` at 1238 now calls it. It reads `valueState` (declared at 990) and `mag` (930), both before 1238, and the function is hoisted. Every run in this pass recorded no page error.
- **The `setLayout` comment** ("Every cell may have moved ... The next pointer move paints it again") is true for `setLayout`. Every case above repainted on the next move.
- **The next move is never dead.** After the reset `valueState.key` is empty, so `requestValue` resets again and requests. A reply in flight across the reset is dropped by generation, and a `want` queued behind it is sent when it returns (D2 held variant, `[` held variant).
- **Pass 1 to 3 verified-clean items**, by re-running their scripts (Evidence). The pass 1 cross-over-cell-1-while-cell-0-moves line still keeps `40 HU`.
- **Whole F-208 diff sweep.** The non-page files (`ci.yml` job, `bin/ocelli.sh` gate and floor exclusion, `ci_floor_check.py`, `catalogue.py` guard and probes, runbook rows 578 to 582, `guard-probe-budget.json`, `eslint.config.js`, `viewer_work.mjs`) are unchanged since pass 3 and their gates are green now. README, `use.html` and `more.html` have no sentence about the readout on a layout or panel change, and their F-208 sentences match the B observations below. No wasm, no `unsafe`, no cast, no pixel arithmetic in the page diff.

AC map:

| AC | Held by | Status now |
|----|---------|------------|
| AC1 picker 1 by 1 to 3 by 3 | `the_page_has_viewport_cells`, AC1 block | green |
| AC2 one template | same, AC2 block | green |
| AC3 no global view state | same, AC3 block, `let scale = 1;` mutation red in pass 1 | green |
| AC4 drag type | same, AC4 needles and the two `carriesSeries` refusals | green |
| AC5 tile mode sentences | same, AC5 block | green |
| AC6 F-207 rules | `the_page_offers_a_folder_picker` | green |
| AC7 empty-cell sentence | `the_page_has_viewport_cells`, AC7 | green |
| AC8 independent zoom, window, slice | B3 to B5 | observed |
| AC9 drop loads and outlines | B2 | observed |
| AC10 tile mode per cell | B6 | observed |
| AC11 records kept across layouts | B7 | observed |
| AC12 1 by 1 is the F-207 page | B1 | observed |
| AC13 `G`, `Shift G`, digits free | AC13 blocks, pass 3 mutations | green |
| AC14 aspect 0.952381 | `gate viewer-browser`, guard probes | green, ratios in Evidence |

Browser observations, headless Chromium 151.0.7922.34, synthetic study, 1600 by 1000, `target/f208-review4/bobs.log`. The design asks for the operator's Chrome through the Playwright MCP tools. These are a headless reviewer record, not that.

- **B1.** Status `Synthetic study, not a patient. 1 series.`, counter `1 / 12`, `#wrap` not `many`, no outline (`::after` content none).
- **B2.** Layout 2 by 2, the series dragged onto cell 3. Cell 3 shows Image 1, is active, and has the `inset 0 0 0 2px #dedede` outline.
- **B3.** One `deltaY = 40` wheel over cell 3. Counter `2 / 12`, cell 3 index 0 to 1, cells 0 to 2 unchanged. Cell 1 was still empty at this step, so "cell 1 does not move" is trivially true here.
- **B4.** Series dropped on cell 1, then Ctrl-wheel over it. Cell 1 reads `Zoom 111%`, cells 0 and 3 still `Zoom 100%`.
- **B5.** Cell 3 active, Lung. Cell 3 reads `WW 1500  WL -600`, cells 0 and 1 read `WW 400  WL 40`.
- **B6.** Cell 1 active, Four images. Cell 1 has four tiles, other cells single view, Length dim, hovering it gives "Drawing works in the single view." Double-click on tile 3 returns cell 1 to one image at index 3, two after the first tile (index 1).
- **B7.** 3 by 3, 1 by 1, 2 by 2. Cell 3 reads Image 2, `WW 1500  WL -600  Zoom 100%`, scale 3.1287, the same as at B5. The active cell moved to 0 on the 1 by 1 step, as the design says.

## Limitations and handoff

- **Not counted, plausible concerns for the operator.** `=` with the pointer still leaves a stale C and R in the same cell (`C 78  R 55` shown while the pixel under the pointer is `C 77  R 54`). `Shift G` leaves the single-view readout visible over the four tiles until the next move, which then hides it. Both stay inside one cell and both exist in kind at `bbf624de` (F-207's keys and its four-image view). The D1 fix does not cover them.
- One synthetic CT series only. A colour series was not replayed.
- No oracle, GPU or `--sprint` profile, as instructed. Ports 8742 and 8765 not used, PID 98891 not touched. Every viewer I started was stopped by its PID (20374, 22321, 23918, 24326, 26363, those of the four re-run scripts, and 19851 by the gate), and `ps` shows none running. No scratch copy was made in this pass.
- Tree unchanged: HEAD `8c6be74b`, tree `2c00893`, nothing staged or unstaged.
- Next owner: the implementer fixes D1 (one `leavePointer()` call in `relayout()`), then a fresh independent pass 5 reviews that one change and replays SF and R. This is a bounded clean review of the remaining material. It is not sprint readiness, and B1 to B7 still need the operator's recorded run against the final tree.

F-208 pass 4: 1 defect, 0 smells, 2 nitpicks.
