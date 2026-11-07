# F-208 review, pass 1

**Reviewed**: `git diff bbf624de 8b8bcada` on `work/f-208-xai` in `ocelli-wt-f-208`, tree `4281577936abb303ea92b10855c0914bfc08b79a`. Nothing unstaged before or after the pass. The only file this pass adds is this report.
**Author / reviewer**: the implementation is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 1). It wrote none of this diff, and the plan's design author was not this reviewer either.
**Result**: 3 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **AC3, per-viewport conversion.** I compared every top-level `let`/`const`/`var` in the base page with the new one. All 26 globals the plan names are gone, plus `scrollDir`, `mainBusy`, `mainTail`, `prefetchAbort`, `tileAbort`, `layoutDirty`, `layoutStamp`, `viewContext` and `inkViewBox`. The F-216 request layer (`ensureFile`, `inflight`, `cache`, `preparedSlots`, `canvasBuffers`) is shared by design and is keyed by file, window and invert, so a shared entry gives two cells identical bytes. One F-210 global is left unconverted, `valueState`, and it leaks across cells (D1).
- **Late replies.** `pumpMain` captures `vp`, its `loadToken`, `viewIndex` and image, and paints only through `applyGrey(vp, ...)` after `moved()` is false. Tiles check `vp.tileToken` and the signal. `scheduleLive` checks `vp.liveToken`. Prefetch only caches. Replay: with every `/preview` held 1.2 s, cell 1 asked for image 12 and the layout went to 1 by 1 while the reply was in flight. `putImageData` recorded no paint into cell 1 while it was hidden (`paints [{"cell":"0","hidden":false}]`), and on regrow cell 1 read `Image 12 / 12`.
- **pumpMain AbortError retry, can it loop.** It is bounded. `fetchGrey`'s `inflight` entry is the promise with `finally` chained on, so a joiner sees the rejection only after the entry is deleted, and its `continue` issues a request under its own view. That request is superseded only when this viewport moves, and then `moved()` is true. Each abort needs a viewport to move, which takes user input. Prefetch, tiles and live W/L never register in `inflight`, and `PLAIN_VIEW` is never superseded. Replay: cell 0 and cell 1 both wanted uncached image 7 while `/preview` answered 503 busy, then cell 0 moved to image 10. Once busy was lifted, cell 1 read `Image 7 / 12` and cell 0 read `Image 10 / 12`, with 0 `/preview` requests in the following idle second.
- **Active cell, B2 to B5.** Replayed through the real UI. A drag onto cell 3 loads it there and outlines it (`box-shadow rgb(222, 222, 222) 0px 0px 0px 2px inset`). There is no outline at 1 by 1 (`none`). One 40 px wheel over cell 3 moves it from Image 1 to Image 2, cell 1 stays on Image 1, and cell 3 becomes active. Ctrl-wheel over cell 1 reads `Zoom 111%` while cell 3 reads `Zoom 100%`. Lung on cell 3 reads `WW 1500  WL -600` while cell 1 reads `WW 400  WL 40`.
- **B7, layout changes keep view state.** After 2 by 2, then 1 by 1, then 2 by 2, cell 1 kept `fitMode: false` and its scale. The prior observation log's B7 row agrees for cell 3.
- **Drag type.** I dispatched `dragover` with only `text/plain` and it was not accepted (`defaultPrevented` false, no `drop-target`). With `application/x-ocelli-series` it was accepted. `onCellDrop` also refuses a hidden cell and an unknown key.
- **Tile mode per viewport and the 32 MiB rule.** Shift G on cell 3 shows tiles in cell 3 only. With Length chosen, Shift G does nothing. When the third of the four images reports 33 MiB, the tile button is disabled and `toggleTileMode` leaves `tileMode` false.
- **Keys, operator answer 1.** Bare `1` and `2` change neither the layout nor the tile mode. `g` opens the picker, Escape closes it, and Shift G toggles. `0` stays Fit.
- **AC14 aspect.** Re-derived from PS3.3 10.7.1.3, where Pixel Spacing is row spacing and then column spacing. Width is 128 x 0.5 = 64 mm and height is 96 x 0.7 = 67.2 mm, so the ratio is 0.952381. Swapped it is 179.2 : 96 = 1.866667. The script measures `canvas.view` and each `canvas[data-tile]` with `getBoundingClientRect()`, which includes the stage transform. **M1**: I served a page with `vp.aspectX`/`vp.aspectY` swapped at their assignment in `drawGrey` and ran the committed `measureAspects`. The single view read 1.866688 and the 2 by 2 cells read 1.866709, with 5 problems, so the check goes red. **M2**: tile `dataset.aspect` inverted gives tiles at 1.866723, with 4 problems. The unmutated page gives 0 problems.
- **Bench driver.** F-216's `viewer.window_step` and `viewer.resident_memory` were re-run on this tree. Both were measured (2.2 ms, 453885952 bytes), against the prior receipt's 2.1 ms and 455983104 bytes. At 1 by 1 the driver's selectors (`#wrap .cell.active .stage`, `canvas.view`) each resolve to the one node the old ids named. `windowSteps` and `canvasMatchesReply` send the same window commands as before.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 4281577 | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | tree 4281577, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo test -p ocelli-folder-viewer` | tree 4281577 | now | 0, every suite ok (73, 1, 14, 60, 1, 1, 24) |
| `npm run lint`, `npm run typecheck` | tree 4281577 | now | 0, 0 |
| `bin/ocelli.sh gate guards` | tree 4281577 | now | ALL GREEN. The `viewer-browser.aspect-swapped` probe ran inside it |
| `bin/ocelli.sh gate bench`, `gate ci` | tree 4281577 | now | ALL GREEN, ALL GREEN |
| `bin/ocelli.sh gate viewer-browser` | release build, OCELLI_PORT=0, Chromium 151.0.7922.34 | now | ALL GREEN. Single view 802.844 by 843.000 = 0.952365. Tiles 400.938 by 421.000 = 0.952346. 2 by 2 cells 400.469 by 420.500 = 0.952363. All 9 are within 0.001 |
| `python3 scripts/ci_floor_check.py` | tree 4281577 | now | 0. "outside the floor, CI runs `viewer-browser` on pull_request, push, workflow_dispatch" |
| `guard_census.py`, `--check-runbook` | tree 4281577 | now | 0, 0. "0 watched by nothing" |
| `sync_agent_skills.py --check`, `prose_check.py`, `source_provenance_check.py` | tree 4281577 | now | 0, 0, 0 |
| F-216 corpus identity: `cargo test -p ocelli-folder-viewer corpus` | `git archive HEAD` scratch copy, main checkout's `corpus/data` symlinked, own target dir, deleted after | now | 2 passed (`borrowed_bytes_match...`, `recycled_grey_matches...`). `corpus-identity.txt`: files 86, frames 88, differences 0 |
| `node tools/bench/run.mjs --out target/f208-review/bench-out viewer.window_step viewer.resident_memory` | release, corpus | now | 2 measured |
| Playwright replays `target/f208-review/replay/replay*.mjs` | release viewer, OCELLI_PORT=0, stopped by PID | now | logs `replay*.log` beside them, no page errors |
| M1 and M2 served-page mutations, `m1.mjs` | committed `measureAspects`/`aspectProblems` | now | M1 red (5), M2 red (4), unmutated 0 |
| Rust page-test mutations in a scratch copy (AC3, AC13, AC13b, AC4) | `the_page_has_viewport_cells` | now | AC3 `let scale = 1;` red. Dropping the Shift G branch red. `"1": "layout"` added to `clicks` **green** (S1). Type check removed from `onCellDragOver` **green** (N1) |
| B1 to B7 screenshots and `log.txt` | target/f208/observe | prior receipt | read. Consistent with the replays above |

## Defects

### D1, the pointer readout in one cell is erased whenever another cell changes image
**Where**: `examples/folder-viewer/src/page.html:1863-1870` (`showIndex`) with `resetValue` at 3104
**What**: `showIndex(vp, next)` calls `resetValue(); paintValue();` when the image of *any* viewport changes. `valueState` is one page global that belongs to the cell under the pointer, so a key step or Play in the active cell clears the readout of a different cell the pointer is resting on. It returns only on the next pointer move.
**Why it is wrong**: the plan's invariant is that no function reads or changes another cell's state, apart from `positions`. F-210's own comment above `valueState` says a reply is shown only for the cell and the image it was asked for. Here a valid reply for the right cell and image is discarded because a different cell moved.
**Evidence**: `replay3.mjs` at 2 by 1, cell 0 active, cross on, pointer still over cell 1. Before: `C 65  R 49  40 HU`. After ArrowDown in cell 0: `C 65  R 49`. After 1.5 s of Play in cell 0: `C 65  R 49`. `replay.mjs` shows the same through `showIndex` directly.
**Scope and regression**: present on HEAD, and new with multiple cells. It affects the F-210 readout only. The smallest regression is a replay that hovers cell 1 with the cross on, steps cell 0 and asserts the value text survives. The fix is to reset the readout only when `vp` is the viewport the readout belongs to (record the cell in `valueState`, or compare `noteKey(currentImage(vp))` with `valueState.key`).

### D2, at 1 by 1 a click on the open series row no longer resets window, preset and fit
**Where**: `page.html:1739-1745` (`loadSeries` returns early when `vp.seriesKey === key`)
**What**: the base `selectSeries` reset `fitMode`, `overrideWindow`, `draftWindow` and `activePreset` on every row click, including a click on the series already open. Now that click is a no-op.
**Why it is wrong**: the approved plan says, under compatibility kept on purpose, "With layout 1 by 1 and tile mode off, every F-207 behaviour reads exactly as today". Approach step 3 says `loadSeries` "keeps the reset of fit, window and preset". The implementer's decision 1 extends the ownership row for a *drop* ("Dropping the series a cell already shows keeps its index and view") to a click. Both plan sentences can hold together if the same-series check lives in `onCellDrop` and not in `loadSeries`.
**Evidence**: `replay3.mjs` at 1 by 1, Lung, `=`, then a click on the open row. Before the click the corner read `WW 1500  WL -600 / Zoom 111%`, and after it the same. The base code at `selectSeries` resets to the file window and fit.
**Scope and regression**: present on HEAD, and visible at 1 by 1. It is a plan-contract defect. The other resolution is for the lead to amend the compatibility clause and accept decision 1, and either way the decision needs a ruling rather than a note. The regression is a 1 by 1 replay of Lung, row click and an expected `WW 400  WL 40`, or the amended expectation.

### D3, more.html's lede still says the image page shows one series
**Where**: `examples/folder-viewer/src/more.html:119`
**What**: "The image page shows one slice of one series." It now shows up to nine viewports, each with its own series, and the same file's `#series` section was edited by this diff to say so.
**Why it is wrong**: microscope section 3. more.html is always in scope, and a stale sentence there is a defect.
**Evidence**: `grep -n "The image page shows one slice" more.html` matches line 119. The replays show two series cells side by side.
**Scope and regression**: present on HEAD, docs only. Delete or correct the sentence.

## Smells

### S1, AC13's negative check can never fire
**Where**: `examples/folder-viewer/tests/preview.rs`, the `for bound in ["\"1\": \"grid-1\"", "\"2\": \"grid-2\""]` loop
**What**: the ids `grid-1` and `grid-2` no longer exist in the page, so those strings cannot reappear. A digit bound to any present control passes. Mutation: adding `"1": "layout",` to the `clicks` map leaves `the_page_has_viewport_cells` green.
**Mechanism**: the plan's rejected wrong implementation is "Digits left bound to layouts, which F-213 then cannot use". F-213 is about to edit exactly this key map, and a check that is counted as coverage for "digits are free" does not cover it. The test matches the plan's literal text, so the fix is small: assert that neither key map in `onKey` has a digit key, or that no `"[1-9]":` key appears in the `clicks` and `map` literals.

## Nitpicks

- **N1.** AC4's page test pins strings only. Removing `if (!carriesSeries(event)) return;` from `onCellDragOver` leaves it green. The plan chose string pins for AC4 and my browser replay rejects the any-drag behaviour, so this does not block. A committed browser check would be the durable form.
- **N2.** `.claude/scratch/F-208-progress.md` (ignored) still says "Not committed" although `8b8bcada` exists. The lead's brief mentions seven plan-versus-tree differences, and the note lists none separately. Decision 8 (`schedulePrefetch` kept its name) is the only one it names. Judged here: decisions 2 to 9 are consistent with the plan or sit where it is silent, and decision 1 is D2.
- **N3.** The `viewer-browser.aspect` catalogue entry's `refuses` names "a run that measured no canvas", and its one probe drives only the ratio branch. The `limit` text does not say so.

## Verified clean

- The gate wiring. `GATES` has `viewer-browser|no|...`, `run_gate` refuses without `tools/bench/node_modules`, the floor `case` excludes it, and `NOT_IN_FLOOR` agrees (`ci_floor_check.py` exit 0). `--sprint`/`--all` take every `GATES` entry. `gates_declared` is 35 and the census digest was re-recorded.
- The CI job installs exactly what the gate needs: Rust 1.97.1 (the pin `rust-toolchain.toml` and the other jobs use), rust-cache, Node 24, and `npm ci` plus `playwright install --with-deps chromium` in `tools/bench`, which has its own lockfile and is not a root workspace member. `aspect.mjs` imports only `tools/bench` code and node builtins. The job inherits the workflow triggers (push to main and sprint/**, pull_request, workflow_dispatch), which is what `ci_floor_check` reports.
- The catalogue's `silent` claim is true. `SCAN_ROOTS` in `scripts/guards/discover.py` is `scripts/ ci/ .githooks/ bin/ tools/`, and `examples/` is not among them. The probe's substitution anchor appears once in `aspect.mjs`. The runbook rows were renumbered 578 to 581.
- README, use.html and the more.html `#series` edits were checked against the replays: up to nine viewports, the outlined viewport, the wheel activating the cell under it, drag or click or double-click into a cell, "Drag a series here.", `G` and `Shift G`, and no outline at 1 by 1. The "next series in list order" fill rule was read in `fillCells`/`nextUnshownKey` but not executed (see limitations).
- Tiles are `image-rendering: pixelated` in CSS at every fit. The active outline is `#dedede` only under `#wrap.many`. The drop outline is equal-channel `#9a9a9a`.
- Boundary and tier: no wasm, no `unsafe`, no new arithmetic beyond the display transform already in the page. The grey bytes still come from F-206's CPU LUT, and the page maps no grey value.

## Limitations and handoff

- The synthetic study has one series, so the multi-series fill on open, growing a layout with unshown series, and the `shown` row class were read but not executed. No corpus folder was opened in the browser.
- A hidden-cell late *tile* reply was read, not replayed.
- B6 and the full B1 to B7 screenshot set are the implementer's prior receipt. I replayed B1 to B5, B7's state retention for cell 1, the keys and the drag type.
- No oracle, GPU or `--sprint` profile was run, as instructed. Ports 8742 and 8765 were not used. PID 98891 was not touched. Every viewer I started was stopped by its PID, and the scratch copy with its target dir was deleted.
- Next owner: the implementer fixes D1 to D3 and S1, the lead rules on D2, and then a fresh independent pass 2 reviews.

F-208 pass 1: 3 defects, 1 smell, 3 nitpicks.
