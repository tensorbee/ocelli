# F-208 review, pass 3

**Reviewed**: `git diff 919e61ab fba5e580` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `fba5e5803dd564753082141247502b2b64e8cac5`, tree `0815bdbed63dc69da904c48e0dbb4c8612c49c65`. Nothing staged or unstaged before or after the pass. The untracked paths are pass 1's and pass 2's reports, present before this pass, and this report.
**Author / reviewer**: the remediation commit is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 3). It wrote none of the diff and neither earlier pass.
**Result**: 2 defects, 1 smell, 0 nitpicks

## Acceptance challenged

- **Pass 2 D1, the hidden cell's readout. Closed for the case it names.** 2 by 1, cross on, pointer still on cell 1, `/value` held 1.5 s. Keyboard (`G`, Shift Tab, Enter), the layout button and picker by mouse, and `G` then a mouse click on the picker all give the same result: just after the shrink the readout and both cross lines are hidden, `valueState.cell` is -1 and the generation has moved (2 to 3). After the held reply lands it stays hidden with no value (`C 56  R 49`, hidden). One pointer move gives `C 103  R 49  40 HU`, cell 0. Pass 2's own `replay.mjs` "S" lines now read hidden with no `700 HU`.
- **A shrink where the readout's cell survives (3 by 3 to 2 by 2). Fails**, D1 below.
- **A grow (1 by 1 to 2 by 2) with the pointer still.** Fails the same way, D1.
- **A shrink while the magnifier is shown.** Passes with the cross on, fails with the cross off, D2.
- **The moved `valueState` declaration.** The page has one `<script>` (line 811). The only top-level statement before line 990 is the grey ramp loop at 888, which does not read it. `setLayout(1, 1)` at 1238 is the only start-up call, and `setLayout` is called from nowhere else at start. Executed: the page served with the declaration moved back to its old place (by `page.route`, no tracked edit) throws `ReferenceError: Cannot access 'valueState' before initialization` twice and never loads the synthetic study. The served page throws nothing. So the comment's claim that `setLayout` reads it at start-up is true and the move is needed.
- **S1, the new pins.** 23 mutations in a scratch copy, table under the smell. F-213's digit binding, as a regex and as `event.code`, stays green, so the pins do not block F-213. Nine routes to the picker or tile mode go red. Seven realistic routes, including all three "through a new helper" cases, stay green.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 0815bdb, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer` | same | now | 0, every suite ok (73, 1, 14, 60, 1, 1, 24) |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards` | same, OCELLI_PORT=0 | now | ALL GREEN, `aspect-swapped` and `aspect-no-canvas` both `red` |
| `bin/ocelli.sh gate viewer-browser` | release, Chromium 151.0.7922.34 | now | ALL GREEN, all 9 canvases within 0.001 of 0.952381 |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 ("895 file(s)"), 0 ("1757 files") |
| `target/f208-review3/replay.mjs`, cases A to G | release viewer, stopped by PID | now | `replay.log`, `A.png`, `D.png`, `E-false.png`, `E-true.png`, no page errors |
| `target/f208-review3/routed.mjs` | served page variants by `page.route` | now | `routed.log` |
| Pin mutations, 23 cases | `git archive HEAD` scratch copy in the scratchpad, own target dir, deleted after | now | `target/f208-review3/mutations.log`, restored copy green |
| Pass 2's `replay.mjs`, `replay-tile.mjs`, pass 1's `replay.mjs`, `replay2.mjs` | same | now | `rerun-*.log`, see verified clean |
| `target/f208/replay3/log.txt` | implementer's | prior receipt | read, agrees with case A |

## Defects

### D1, a layout change with the pointer still leaves a surviving cell's value printed over another cell
**Where**: `examples/folder-viewer/src/page.html:4398` (`setLayout`'s new block clears the cross only when `valueState.cell >= count`), with `pointReadout` at `page.html:3062` positioning the cross from the cell box at the last pointer event
**What**: when the readout's cell is still shown after a layout change, its cross lines, its text and its value stay where the last pointer event put them, although every cell has moved. Replay, keyboard only and pointer still: 3 by 3, the series in cells 0, 2, 3 and 4, cell 2 stepped to Image 5, cross on, pointer resting in cell 3 reads `C 51  R 52  -800 HU`. `G`, focus "2 by 2", Enter. After the shrink the readout still reads `C 51  R 52  -800 HU`, visible, `valueState.cell` 3, while `viewportAt` at the pointer is cell 2. `D.png` shows the text printed in cell 2's top band and the cross lines still drawn on cell 3's old 3 by 3 box. One 2 px move at the same spot gives `C 26  R 3  -1000 HU`, cell 2, which is the value actually under the pointer. The grow is the same: 1 by 1 with the pointer at the centre plus (200, 150) reads `C 97  R 66  -1000 HU`. After `G` and 2 by 2 it still reads that, visible, cell 0, while the pointer is over cell 3.
**Why it is wrong**: the design (`F-208-design.md`, DOM paragraph) says `#cross-h`, `#cross-v` and `#cross-read` are "positioned from the cell under the pointer", and its invariant is that no function reads another cell's state. The value printed is cell 3's at a point that belongs to cell 2, which is the quietly wrong value CLAUDE.md names. Pass 2 D1 is this class, and the fix closes only the hidden-cell instance of it.
**Evidence**: `target/f208-review3/replay.log` lines "D before", "D after keyboard shrink to 2x2", "D after one move", "F before", "F after keyboard grow". `D.png`.
**Scope and regression**: present on HEAD and at `8b8bcada`. Not introduced by the remediation. Every layout change made without moving the pointer, so the keyboard picker path, and any later programmatic layout (F-213's presets if they set a layout). A mouse change hides the cross through `wrap`'s `pointerleave`. The smallest fix is to treat any layout change as the pointer leaving: in `setLayout`, unconditionally `resetValue()`, `hideCross()` and `mag.hidden = true`, rather than only when the readout's cell is hidden. The regression is case D and case F of the replay, asserting the readout hidden after the change. A pre-existing relative, not counted here: `[` with the cross on at 1 by 1 leaves `C 97  R 66  -1000 HU` on screen while the image moves (a move then reads `C 115  R 66`), case G. That path is not F-208's, and moving the reset into `relayout()` would close it too, but that widens the change to the resize observer.

### D2, with the cross off the magnifier keeps showing the hidden cell's pixels after a shrink
**Where**: `examples/folder-viewer/src/page.html:4398-4403`, where the new `mag.hidden = true` sits inside the `valueState.cell >= count` test
**What**: `valueState.cell` is set only by `requestValue`, which runs only when the cross is on. With the magnifier on and the cross off, it stays -1, so the block never runs and the magnifier stays up. Replay: 2 by 1, the series dropped in cell 1, cell 0 stepped to Image 3, `m` only, pointer resting in cell 1. Keyboard shrink to 1 by 1: `mag.hidden` false before and false after, one cell shown. `E-false.png` shows the magnifier's white interior drawn over the grey ring of cell 0's image beneath it, which is cell 1's Image 1 at the old pointer. With `c` and `m` both on (case E, cross true) the magnifier is hidden after the shrink, so the outcome depends on an unrelated toggle.
**Why it is wrong**: the same design sentence covers `#mag`, positioned from the cell under the pointer, and `setLayout`'s own comment says a hidden cell's tokens move on "so no late frame paints into it". A magnified image of a cell that is no longer shown, drawn over the cell that is, breaks both. The new line exists to close this, and its guard keys on the readout's owner, which is not the magnifier's.
**Evidence**: `target/f208-review3/replay.log` lines "E cross=false" and "E cross=true". `E-false.png`, `E-true.png`.
**Scope and regression**: introduced as an incomplete fix in this commit (the magnifier was unconfirmed in pass 2). The same unconditional reset as D1 closes it. Regression: case E with the cross off, asserting `#mag` hidden after the shrink.

## Smells

### S1 (carried, third pass in this family), the pins catch named tokens, and their comments claim helpers and nodes
**Where**: `examples/folder-viewer/tests/preview.rs:419-472`
**What**: the lead's ruling is to pin that the picker and tile mode are reached from the keyboard only through `G` and Shift G and their buttons. The new pins do that for the tokens they list, and they leave F-213 free to bind digits. Their comments claim more: "whatever key or spelling another branch would use", "A branch reaching them by name, by the buttons' ids or nodes, or by the picker's own buttons is counted", and "A second key path added through a helper function adds a call site here".

| Mutation (each adds `else if (event.key === "1")` unless stated) | Result |
|---|---|
| M1 `setLayout(1, 1)` | red |
| M2 new helper calling `setLayout` | red (page count 4) |
| M3 `layoutButton.click()` | red |
| M4 `getElementById("layout").click()` | red |
| M5 `clicks` map `1: "layout"` | red |
| M6 `clicks` map `1: "tile-mode"` | red |
| M7 `tileButton.click()` | red |
| M8 `setTileMode(vp, true)` | red |
| M9 picker `[data-columns]` click | red |
| M10 `layoutButton.dispatchEvent(new MouseEvent("click"))` | **green**, and in the browser key 1 opens the picker |
| M11 `layoutPicker.querySelector("button").click()` | **green** |
| M12 new helper `tileOn()` calling `setTileMode` | **green**, and in the browser key 1 turns tile mode on |
| M13 new helper calling `layoutButton.click()` | **green** |
| M14 new helper setting `layoutPicker.hidden = false` | **green** |
| M15 new helper setting `activeViewport().tileMode = true` | **green** |
| M16 a second `keydown` listener calling `layoutButton.click()` | **green** |
| M17 a second `keydown` listener calling `setLayout` | red |
| M18 `layoutPicker.firstElementChild.click()` | **green** |
| M19 the Escape branch also calls `layoutButton.click()` | red |
| F-213 `/^[1-9]$/` to `applyPreset` | green, as required |
| F-213 `event.code` `Digit` to `applyPreset` | green, as required |
| R1 the `G` branch removed | red |
| R2 the Shift G call removed | red |

**Mechanism**: the page-wide counts pin `setLayout(`, `openLayoutPicker(` and `toggleTileMode` but not `setTileMode(`, `layoutButton`, `layoutPicker` or `tileMode =`, and the handler ban list matches `layoutButton.click` but not `layoutButton` as a node. So M12 to M15 add a key path through a helper with no new call site of a counted name, which the comment says cannot happen, and M10, M11 and M18 reach the buttons' nodes or the picker's own buttons, which the comment says are counted. A later reader will take the comment as the guarantee. **Smallest fix**: either narrow both comments to the tokens actually checked, or extend the page-wide counts to `setTileMode(` and to `layoutButton` and `layoutPicker` uses outside their listeners and the Escape branch. Deleting the overclaiming sentences is the cheaper of the two.

The same S1 family has now survived three passes with changing specifics. That is worth telling the operator, and the cheap route is the narrower comment.

## Nitpicks

None. Pass 2's N1 is taken as suggested (`more.html:119`, "each showing one series").

## Verified clean

- Pass 2 D1 for the hidden cell, by keyboard, by mouse on the button and picker, and by `G` plus a mouse click on the picker. The late reply is dropped because the generation moved, not merely hidden.
- The `valueState` move, on the one start path, with the old placement executed and shown to fail.
- The new `setLayout` comment ("A readout for a hidden cell goes, and so does a reply still on its way for it, because resetValue moves the generation the reply carries") is true as stated: generation 2 to 3, reply landed, no value painted.
- The new AC13 comment's first sentence and the three page-wide counts: `setLayout(` 3 (definition, picker, start-up), `openLayoutPicker(` 3, `toggleTileMode` 3, each red when a fourth call site is added (M2, M17).
- The Escape branch's three named tokens are what that branch calls, and adding an opening call to it goes red (M19).
- Pass 1 and pass 2 verified-clean items, by re-running their scripts on this tree. Pass 1's `replay2.mjs` log is identical. Pass 1's `replay.mjs` differs only in a busy-reply count (85 against 89, a timing count) and still gives 0 previews in the idle second. Pass 2's `replay-tile.mjs` log is identical. Pass 2's `replay.mjs` is identical up to its "S" lines, which now read hidden (the fix), and it stops at the different-series drop because pass 2 deleted the two-series study copy. The diff touches neither `onCellDrop` nor `loadSeries`, so that case was not rebuilt.
- Boundary and tier: the diff adds no wasm, no `unsafe`, no cast and no pixel arithmetic. README, `use.html` and `more.html` have no sentence about the readout or magnifier on a layout change, and the changed `more.html` lede is accurate.

## Limitations and handoff

- One synthetic CT series only. A colour series, where `requestValue` returns early and `valueState.cell` is not updated, was read and not replayed. The D1 fix above covers it.
- No oracle, GPU or `--sprint` profile, as instructed. Ports 8742 and 8765 not used, PID 98891 not touched. Every viewer I started was stopped by its PID (63613, 64694, 73842, and 80109 by the gate plus those of the re-run scripts), and none is running. The scratch copy and its target dir were deleted. Pass 2's logs under `target/f208-review2/` were restored after the re-runs, and the re-run logs are under `target/f208-review3/`.
- Tree unchanged: HEAD `fba5e580`, tree `0815bdb`, nothing staged or unstaged.
- Next owner: the implementer fixes D1 and D2 (one unconditional reset in `setLayout`) and S1, then a fresh independent pass 4.

F-208 pass 3: 2 defects, 1 smell, 0 nitpicks.
