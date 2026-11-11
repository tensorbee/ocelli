# S46 review, pass 5

**Reviewed**: the staged pass 4 remediation on sprint/s46 a48a9172, staged tree 185def6e.
**Author / reviewer**: remediation by GLM with coordinator adaptations. Review by Claude, a read-only subagent that ran nothing, composed by the Claude coordinator.
**Result**: 0 defects, 0 smells, 6 nitpicks

## Acceptance challenged

- **Claim.** Round 4 closes pass 4. Every window setter drops a late reply a newer choice superseded, tile mode is never blank, and an image is never drawn at two windows.
- **Combinations tried against the code.** A transient series-window failure, a refusal outliving a reopen, a keyboard preset during a drag, a link target holding a draft, region, preset and drag commits after an await, and series B loaded during A's wait.
- **Result.** No defect and no smell.

## Nitpicks

N1 `examples/folder-viewer/src/page.html` ensureSeriesWindow. A connection reset during the request makes the series per-image for the rest of the open, because only `dropCache` clears `seriesWindowRefused`. Documented and labelled `this image`, so nothing is quietly wrong. Carried into F-280.

N2 `tests/browser/window.mjs` heldPresetDuringDragProblems asserts only negatives, so a page that dropped both the preset and the drag would pass. Corrected with a positive check for the manual window.

N3 `tests/browser/auto.mjs` observeSeriesSwitchDuringWait presses `s`, which selects the Scroll tool and does nothing for the series list. Removed.

N4 `auto.mjs` tilesBeforeSeriesWindowProblems keeps a tile width clause that cannot fail. Removed, as pass 4 S4 did for the failed-request check.

N5 `crates/ocelli-codec/tests/jpegxl.rs` the rewrapped refusal comment splits a sentence across a line end awkwardly. Rewrapped.

N6 `page.html` refreshFrameWindows calls `paintTiles` without the series-window guard. It was added this sprint by F-213's review pass 2, b8ec548f, and runs only for frames with per-frame windows, which are not auto-eligible in practice. Carried into F-280. Pass 6 N1 corrected an earlier line here that said it predated S46.

## Verified clean

The refusal lifetime and its generation check, the 410 restart path, the draft guard and every place a draft is set or cleared, a re-enumeration of every window setter, the guarded tile paint after the wait, determinism of every held control, the SPRINT_PLAN tables and milestone sums, the CURRENT_SPRINT AC18 sentence, the prose rules and the spec gold.

## Evidence

The nitpick corrections change the tree, so a fresh sprint profile and a further pass are required before close.
