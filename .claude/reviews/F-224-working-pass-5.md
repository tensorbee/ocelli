# F-224 review, pass 5

**Reviewed**: `git diff c2b37457 8e97cc90`, the folder viewer half of F-224, one commit 8e97cc90 "F-224, review pass 4 remediation for the folder viewer half", in worktree `ocelli-wt-f-224`, branch `work/f-224-xai`, HEAD `8e97cc90b7ee810eb41ba444c30630d73cedc3fd`, tree `aee3decae452c899b62a305466db0b1bd17de5cd`. No unstaged tracked paths. Untracked `.venv` and `tools/oracle/out` were present and are not part of the review. Normative source: `.claude/plans/F-224-design.md` decisions 8 to 11 and its lead amendments, `.claude/reviews/F-224-working-pass-4.md`, and the lead's rulings for this remediation: D1 a changed file's tile shows the sentence in that tile and other tiles keep drawing, D2 `setActive` calls `noteChanged` and probes on activation, D3 the README keying sentence, S1 the late-reply case, S2 the thumbnail case, N1, N2.
**Author / reviewer**: remediation by a Claude implementer, per the lead's brief. The commit author field is Atul Sharma, so authorship rests on the brief, not on a trailer. Reviewer Claude Opus 5.5 as an independent `/microscope` subagent. It wrote none of this material and none of the earlier F-224 material.
**Result**: 0 defects, 1 smell, 3 nitpicks

## Acceptance challenged

Every case below ran through the real page served by a release build of the scratch copy, on synthetic 8 by 8 mono16 CT files written by the script into a temporary folder, one browser at a time, with `OCELLI_PORT=0`.

- **D1, tile mode.** The new `tiles` case: five files, `Shift+G`, file 1 changed and focus gives widths `8 1 8 8`, then file 0 gives `1 1 8 8`, no fault text, the grid shown. Mutation M1 (the tile label branch in `paintTiles` disabled) and M2 (the `vp.tileMode` branch in `pumpMain` disabled) each turn `changed.mjs` red for the stated reason. M2's output is the pass 4 state: tiles hidden, the sentence over the grid, tiles 3 and 4 at width 1.
- **One step from D1, a 2 by 2 layout with cells on different series.** Two series of five files, cells 0 and 1 on series A and B, cells 2 and 3 empty, both filled cells in tile mode. B's file 1 changed with cell 1 active: cell 1 `8 1 8 8` with the sentence in tile 2, cell 0 `8 8 8 8`, no fault in either cell. A's file 0 changed with cell 1 still active: focus finds nothing in cell 0, which is the documented "active series" rule. A click on cell 0 then gives cell 0 `1 8 8 8` with the sentence in tile 1, and cell 1 is unchanged. Leaving tile mode in cell 0 shows the sentence in place of the single image with the stage hidden. A screenshot shows the sentence wrapped and legible in a quarter of a quarter cell.
- **One step from D1, the tile label after the modification time is restored.** File 1 marked in tile mode, then its time set back to the original with `utimesSync` and a focus, then `ArrowDown` and `ArrowUp`. The tile stays at width 1 with the sentence and the `changed` class on tile 2 only, and the status keeps the count. The page does not unmark a file, which agrees with the sentence's "Open the folder again to see it."
- **One step from S1, a late reply in tile mode.** Tile mode, every preview reply held after the process answered, `i` (four held), file 1 rewritten, focus, the replies released. Widths `8 1 8 8`, the sentence in tile 2, 0 grey keys for file 1's serial, no fault.
- **D2, activation.** The new `cells` case passes on HEAD. M3 (`probeActive` removed from `setActive`) turns it red: "activating B's cell after its file changed showed "", not the changed-file sentence". M5 (`noteChanged` removed from `setActive`) stays green, as the lead noted. The reason is that `probeFiles` ends with `noteChanged()` whenever a probe ran, and `probeActive` runs one for every cell that has a series. So the extra call in `setActive` changes what is shown only when the activated cell has no series, because `probeActive` then returns before probing. See S1 for the ruling.
- **One step from D2, activating an empty cell.** One series, layout 2 by 1, cell 1 empty. File 0 changed, focus, status counts 1. A click on cell 1: HEAD reads `1 series.`, M5 reads `1 series. 1 file of this series changed on disk after the folder was opened.` with no series active. A click back on cell 0 restores the count and the sentence on HEAD. In the 2 by 2 case, activating empty cell 2 reads `2 series.` on HEAD.
- **One step from D2, a probe already running when activation fires.** Series A of 120 files and B of 2 in a 2 by 1 layout, every `Blob.prototype.arrayBuffer` delayed 15 ms by an init script so the probe of A runs about 0.5 s. B's file 0 rewritten, focus, then a click on cell 1 after 40 ms. At the click `probe.busy` was true and `probe.again` became true. Cell 1 then showed the sentence, the status counted 1 for B, and 122 reads were made in all, so the queued probe ran once for B after A's probe finished.
- **One step from D2, a change found during a linked scroll from F-209.** Two axial series of five files on one frame of reference (z = k and z = k + 0.25), layout 2 by 1, Link Scroll on. B's file 2 was marked first by activating cell 1 and returning to cell 0. Then A's file 2 was rewritten and `ArrowDown` was followed by a focus. Steps gave indices `[1,1]`, `[2,2]`, `[3,3]`, `[4,4]`. At `[2,2]` both cells showed the sentence, the status counted 1 for A. At `[3,3]` and `[4,4]` both cells drew again with no fault. So the link still moves the target from a source whose image is refused, and a target landing on a changed file shows the sentence and leaves it when it moves on.
- **S1 and S2.** The new `late` and `thumb` cases pass on HEAD. M4 (the guard in `requestGrey` removed) turns `late` red with "the viewport showed """ and "1 grey keys of file 0". M6 (the thumbnail loop in `markChanged` disabled) turns `thumb` red with "red sum 2391296". Both pass 4 smells are closed.
- **N2, a moved modification time with identical bytes.** `utimesSync` alone on file 0 and a focus: the sentence and the count. The new README and `use.html` sentence holds.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree HEAD 8e97cc90 | now | 0. 202 passed, 0 failed across the targets |
| `bin/ocelli.sh gate viewer-browser` | worktree HEAD, Chromium headless from tools/bench's pinned Playwright, node 24.13.0 | now | 0. aspect, readout, link, forget and changed green. changed: sentence, held 3 then 2, old id 410, restored unmarked, late 0 keys, thumb 2391296 then 0, tiles `8 1 8 8` then `1 1 8 8`, cells as in Acceptance challenged, probe 1693 reads, peak 4, 144.8 ms |
| `node changed.mjs` baseline | scratch copy of HEAD from `git archive`, own `CARGO_TARGET_DIR`, release build | now | 0, the same observations as the gate |
| M1, tile label branch disabled | scratch copy | now | 1, three "read "Image N", not the changed-file sentence" lines |
| M2, `pumpMain` tile branch disabled | scratch copy | now | 1, tiles hidden, the sentence over the tiles, tiles 3 and 4 at width 1 |
| M3, `probeActive` removed from `setActive` | scratch copy | now | 1, B's sentence and status missing |
| M4, the `requestGrey` late-reply guard removed | scratch copy | now | 1, fault "" and 1 grey key of file 0 |
| M5, `noteChanged` removed from `setActive` | scratch copy | now | `changed.mjs` 0. Reviewer `empty` case shows the stale count. `the_page_keys_pixels_by_file_and_probes_for_changes` fails with "activating a viewport does not count and probe its series" |
| M5b, `if (seriesOf(activeViewport())) noteChanged();` in `setActive` | scratch copy | now | `changed.mjs` 0, the source pin 0, reviewer `empty` case shows the stale count |
| M6, thumbnail loop disabled | scratch copy | now | 1, "red sum 2391296" |
| M7, `classList.remove("changed")` in `paintTiles` removed | scratch copy | now | 0, see N2 |
| Reviewer cases empty, grid, busy, restore, linked, touch, tilelate | scratch copy, a reviewer script beside `changed.mjs` reusing its helpers | now | as in Acceptance challenged |
| `guard_probe.py --only` each of `viewer-browser.changed-old-image`, `-claimed-detection`, `-whole-file` | worktree HEAD | now | 0 each, red for the declared reason, control green |
| `guard_census.py`, `--check-runbook`, `unittest scripts/tests/test_guard_catalogue.py` | worktree HEAD | now | 0, 0, 57 tests OK |
| `npx eslint changed.mjs eslint.config.js`, `prose_check.py` | worktree HEAD | now | 0, voice rules clean over 924 files |

## Defects

None.

## Smells

### S1, the empty-cell effect of `noteChanged` in `setActive` is watched by text alone

**Where**: `examples/folder-viewer/src/page.html:4925`, the `noteChanged();` the D2 ruling added to `setActive`, its source pin in `examples/folder-viewer/tests/preview.rs:4108` to `4113`, and `observeCells` in `examples/folder-viewer/tests/browser/changed.mjs`.
**Trigger**: one series, layout 2 by 1, file 0 changed and counted, then a click on the empty cell 1.
**Expected / actual**: the status drops the count, because no series is active and the line counts "the changed files of the series" (decision 10). HEAD does this. Under M5 and under M5b the status keeps "1 file of this series changed on disk after the folder was opened." with no series in the active cell, which is a false sentence on screen.
**Why it blocks**: the browser check cannot see this line at all. In every state `observeCells` reaches, the activated cell has a series, so `probeActive` runs a probe and `probeFiles` ends with `noteChanged()`, which recounts whether or not `setActive` did. M5 green in the browser is therefore not chance: the only state in which the call changes the page is one no check reaches. The source pin goes red for M5 but checks only that the text `noteChanged();` appears in the function body, so M5b, a guard on the active series that keeps the text, passes the pin, the browser check and the gate while showing the false count. The maintenance hazard is concrete: a reader who sees M5 green in the browser and the probe's own `noteChanged()` will judge the call redundant, and the only thing that disagrees is a text match.
**Ruling on M5**: it is a test gap, and this is it. It is not a gap in the two-series case, which is covered by the probe's own recount.
**Smallest regression**: in `observeCells`, choose layout 3 by 1 so cell 2 is empty, and after `aStatus` click cell 2 and require the status not to contain "changed". The reviewer `empty` case did the same in a 2 by 1 layout and separates HEAD from M5 and M5b.

## Nitpicks

- **N1.** README and `use.html` call the mode "the four-image view". The page's button is "Four images" (`aria-label="Four images"`) and README lines 245 and 268 call it "Four images". Using the control's name would let a reader find it.
- **N2.** `paintTiles` now removes the `changed` class on every repaint (M7). Removing that line leaves every check green. Because a changed file stays changed until the next open, the only effect would be a centred "Image N" label on a tile that once held a changed file and was scrolled to an unchanged one. Cosmetic.
- **N3.** The `changed.mjs` header says of `thumb` that "image 1 must still draw", while `changedProblems` checks only that image 1's fault text is empty. A blank canvas with no fault would pass. Either the wording or a canvas width check would close it.

## Verified clean

- **Pass 4 D1 closed.** A changed tile carries the sentence, other tiles draw, the grid is not hidden, in one cell and in a 2 by 2 layout on two series. M1 and M2 show the `tiles` case watches both branches.
- **Pass 4 D2 closed.** Activating a cell recounts and probes its series, including when a probe is already running. M3 shows the `cells` case watches the probe. The empty cell reads no count on HEAD.
- **Pass 4 D3 closed.** README now says "every grey the page keeps". `cacheKey` and `inflight` are keyed by `fileSerial`. "The series list's thumbnails are keyed by series and drawn again on every open" agrees with `thumbBitmaps.set(series.key, ...)` and the `thumbBitmaps.clear()` and `thumbFiles.clear()` in `readFolder`, which the synthetic study also goes through.
- **Pass 4 S1 and S2 closed.** M4 and M6 turn `changed.mjs` red.
- **Pass 4 N1 and N2 closed.** `more.html` `#worker` now says "A file of the active series" and names the change of active viewport. The moved-time sentence in README and `use.html` was executed with a `touch` and holds.
- **Changed sentences executed.** README line 191: the four triggers (focus by `focus` event, series selection and activation driven in a browser, visibility pinned in source only), the tile sentence, the active-series count, the moved-time sentence, the limit sentences. `use.html` section 12 and `more.html` `#worker`: the same claims. The page comments on `probeFiles`, `markChanged`, `pumpMain`'s tile branch and the CSS rule: `probeActive` has exactly the callers the comment names (focus, visibility, `loadSeries` for the active cell, `setActive`, the queued re-run). The `changed.mjs` header and the `seen` doc comment match the observers apart from N3. The catalogue's new refusal text matches what `changedProblems` refuses. "a status that counts another series after a viewport becomes active" is refused by the `bQuiet` check when both recounts are missing, though not by either alone (S1). The runbook row agrees with the catalogue and `--check-runbook` passes.
- **Lint globals.** `fileSerials` and `cache` added to the browser test globals are the page's top-level bindings that `observeLate` reads.
- **Boundary and tier.** No wasm-bindgen, no new route, no pixel arithmetic in the page. The grey still comes from the local process. No new arithmetic, so no new PS3.3 fixture is owed. The pass 4 arithmetic at (5, 2) is unchanged.
- **No regression** in F-208, F-209, F-210, F-211, F-216 or F-226 on the evidence of the `viewer-browser` gate, the reviewer linked-scroll case and `cargo test -p ocelli-folder-viewer`, all green on HEAD.

## Limitations and handoff

- `visibilitychange` was not driven in a browser. It is pinned in source by the AC16 test only, as in pass 4.
- Only Chromium headless from the pinned Playwright was used. AC18 was not run, and the plan stages it at sprint verification.
- The in-flight re-post race against the forget from pass 4 remains reasoned, not driven, and is not counted.
- In the 2 by 2 screenshot the cell overlay of a tile-mode cell whose tile 1 changed still reads "Instance 1" and the cell's window. It describes the group, the other tiles use that window, and decision 10 speaks of the image, so it is not counted.
- The scratch copy at `/private/tmp/claude-501/-Users-atulsharma-Documents-projects-tensorbee-ocelli/7387c89c-42ee-4e98-93b8-dbe52072290d/scratchpad/f224-p5` and its target directory were removed with Python `shutil.rmtree` after the pass. The reviewer script and mutation driver lived only in the scratchpad.
- Next owner: the implementer for S1, then a fresh independent pass 6 on the changed material. This pass is a bounded review of the folder viewer half and does not establish sprint readiness.
