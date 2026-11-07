# F-208 review, pass 5

**Reviewed**: `git diff 8c6be74b HEAD` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `e664ecf98bc9b534477d6baeea1169ccb59e39ae`, tree `f65651727ec00221ea69c9f7f189396c1b33e2ef`. Nothing staged or unstaged before or after the pass. The untracked paths are the pass 1 to 4 reports, present before this pass, and this report.
**Author / reviewer**: the commit under review is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 5) and wrote none of the diff.
**Result**: 2 defects, 0 smells, 2 nitpicks

## Acceptance challenged

The lead's ruling is that the readout, the cross and the magnifier clear whenever the view geometry under them can change, and stay visible during an ordinary pointer move. I tried to break both halves rather than replay the listed triggers.

- **Pass 4 D1, resize and full screen. Closed.** 2 by 2, all cells filled, cross and magnifier on, pointer still on cell 0: Ctrl-wheel, `[`, `]`, `Shift F`, `Shift F` back, a resize to 1100 by 800, `0` and `o` each leave the readout, both cross lines and `#mag` hidden with `valueState.cell` -1 (probe T7). `i` (Invert, no geometry change) correctly leaves them shown.
- **An ordinary pointer move, no geometry change. Holds.** 80 moves at 25 ms across cell 0 at 1 by 1, and across cell 0 into cell 1 at 2 by 2: readout hidden 0 times, magnifier hidden 0 times, C and R wrong 0 times, and a second `ResizeObserver` on `#wrap` fired 0 times during the walk, so `relayout` does not run on a plain move (T1).
- **A drop onto the cell under a still pointer. Cleared** (T5, `loadSeries` sets `layoutDirty` and `layoutViewport` clears for that cell).
- **devicePixelRatio 1 to 2 through CDP.** No resize event, no `relayout`, readout kept and its `C 58  R 52` still equals the pixel under the pointer, because every coordinate in the readout path is CSS pixels. Correct (T6).
- **W/L drag.** Readout shown on all 20 moves, `scheduleLive` relays out with an unchanged stamp so nothing clears. Correct (T2).
- **A pan drag and a middle-button zoom drag. Fails**, D1.
- **A slice step, a wheel step and Play with the pointer still. The readout holds** (C and R kept, the value dropped and not re-requested until a move, so no wrong number is printed). **The magnifier fails**, D2.
- **Mid-measure.** A Length drag across a window resize keeps the draft, the readout hides on the resize and comes back on the next move of the drag, and the release adds the mark (0 to 1, T8). `leavePointer` still touches only `valueState`, the cross nodes and `#mag`.
- **readout.mjs against the mutation.** Red with all four calls removed (all five observations fail). Red with only `layoutViewport`'s call removed (zoom) and with only `setTileMode`'s call removed (both tile toggles). **Green** with only `relayout`'s call removed and with only `setLayout`'s call removed, N1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree f656517, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer` | same | now | 0, suites 73, 1, 14, 60, 1, 1, 24 passed |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards` | same, OCELLI_PORT=0 | now | ALL GREEN, `viewer-browser.readout-survives` `red` |
| `bin/ocelli.sh gate viewer-browser` | release build, Chromium 151.0.7922.34 | now | ALL GREEN, aspect 9 within 0.001, readout "all 5 changes hid the readout and a move showed it again" |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 ("895 file(s)"), 0 ("1758 files") |
| `target/f208-review5/probe.mjs`, T1 to T8 | release viewer at HEAD, OCELLI_PORT=0, PID 63868 stopped | now | `probe.log`, no page errors |
| readout.mjs under 5 mutations plus the unmutated page | scratch `git archive HEAD` copy, own CARGO_TARGET_DIR, PIDs 66892 to 69633 stopped, copy deleted | now | all: exit 1, relayout: 0, layoutViewport: 1, setTileMode: 1, setLayout: 0, none: 0 |
| probe T2 against `8c6be74b`'s page | same scratch copy, page from `git show 8c6be74b:`, PID 71134 stopped | now | pan shown 20 of 20 with 1 `/value`, zoom drag shown 20 of 20 |

## Defects

### D1, a pan or zoom drag now hides the readout and the magnifier for the whole drag and sends one discarded `/value` per move
**Where**: `examples/folder-viewer/src/page.html:2394` (`leavePointerFor(vp)` in `layoutViewport`), reached from `onPointerMove` at 2862 (pan) and through `zoomAt` at 2866 (zoom), after `pointReadout(event)` at 2831 has already placed the readout and sent its request.
**What**: `onPointerMove` places the readout and calls `requestValue` first, then the pan or zoom branch calls `layoutViewport` with a changed stamp, which clears what was just placed and moves the generation. So the move that changes the geometry never places the readout for the new geometry. Right-button pan, 20 moves at 40 ms, cross and magnifier on: readout shown 0 of 20, magnifier shown 0 of 20, 20 `POST /value` whose replies are all dropped, and still hidden after the button comes up. Middle-button zoom drag: the same, 0 of 20 and 20 posts. At `8c6be74b` the same pan shows the readout and magnifier on 20 of 20 moves with 1 `/value` (pan keeps the same pixel under the pointer, so the coalesced request is reused), and the zoom drag shows 20 of 20.
**Why it is wrong**: the ruling is that the readout clears when the geometry changes and is placed again by the next pointer move. Here the pointer is moving and every move re-hides, so the "next move" never shows it, which is the "cleared so often it cannot be read" case the lead named. It is a regression of F-207's pan readout. The request per move whose answer is always thrown away also runs against HLD section 5.1, "Never one call per pointer move", which the design adopts for the page.
**Evidence**: `target/f208-review5/probe.log` T2 lines, and the `8c6be74b` replay above.
**Scope and regression**: present on HEAD only, introduced by this commit. W/L drags are unaffected. Smallest fix: in `onPointerMove`, place the readout after the pan or zoom has laid out (call `pointReadout(event)` again after `layoutViewport` or `zoomAt` in those two branches, or move the first call below them for drag modes). Regression: add a pan case to readout.mjs that asserts the readout shown during and after a right-button drag.

### D2, after a slice step, a wheel step or a Play frame under a still pointer the magnifier keeps showing the previous image
**Where**: `page.html` `showIndex` (about 1874 to 1900) clears the readout's value when the image changes but nothing hides or redraws `#mag`. `updateMag` (3229) draws only from `pointReadout`, which runs only on a pointer event.
**What**: 1 by 1, cross and magnifier on, pointer still on the image. Before, image 1, magnifier hash 1224425326. `ArrowRight`: the counter moves to image 2, the readout correctly drops its value to `C 58  R 52`, and the magnifier hash is still 1224425326, image 1's pixels. One move changes it to 2683161032. A wheel step and 1.5 s of Play (index 0 to 5) leave the magnifier frozen in the same way, so during Play the magnifier shows a frame the viewport no longer shows.
**Why it is wrong**: the magnifier is placed "from the cell under the pointer" (design, DOM paragraph) and shows that cell's picture enlarged. A magnified picture of a different image beside a counter naming the new one is a quietly wrong pixel, and the brief's family is anything placed under the pointer for a state the cell no longer shows. The readout half of this path is already handled (`showIndex` resets the value), so the page treats an image change as invalidating, just not for the magnifier.
**Evidence**: `target/f208-review5/probe.log` T4 lines ("mag changed by step while still: false", "mag changed after move: true", and the Play line).
**Scope and regression**: the single-cell form exists at `bbf624de` (F-207's magnifier is drawn only on pointer events), so the lead may rule it out of F-208's scope as pass 4 did for `=` and `Shift G`. I count it because the ruling closes the family by rule and this path breaks the rule as stated in the brief (a slice step, a cine frame). The same applies to the four tiles of a tile-mode cell on a wheel step, by the same code, not separately replayed. Smallest fix: when `showIndex` changes the image of the cell the magnifier is over (`mag.dataset.cell`), hide `#mag`, or redraw it once the new grey is painted. Regression: a readout.mjs case asserting the magnifier hidden, or its pixels changed, after `ArrowRight` with the pointer still.

## Smells

None.

## Nitpicks

### N1, readout.mjs does not hold the `relayout` and `setLayout` calls individually
Removing only `relayout`'s `leavePointer()` or only `setLayout`'s keeps readout.mjs green, because `relayout` sets `layoutDirty` on every shown cell and `layoutViewport`'s `leavePointerFor` then clears the cell the readout is on. Behaviour in the observed cases is held, and the guard's `refuses` text claims behaviour, not those call sites, so nothing false is stated. The two calls matter only where `layoutViewport` returns early (a cell with no image or a hidden stage), which no committed case drives.

### N2, README step 8 says the readout comes back with the next move after Four images
`README.md` step 8: "... Four images, ... and come back with the next pointer move." In tile mode the cross and its readout never come back (`pointReadout` shows them only when `!vp.tileMode`), only the magnifier does. Every other trigger in the sentence was executed and is true (zoom by `=` and Ctrl-wheel, fit and reset by `0`, turn by `o`, layout, `[` and `]`, `Shift F`, window resize). The sentence says nothing about pans, which D1 makes the reader's surprise.

## Verified clean

- **No over-clearing on an ordinary move**, T1 above, including crossing a cell boundary at 2 by 2. The hover-sentence line does not resize `#wrap`.
- **`leavePointerFor` scoping.** It clears only when `valueState.cell` or `mag.dataset.cell` names the changing cell. A drop or tile toggle on another cell leaves the readout alone by code. `mag.dataset.cell` is set in `updateMag` after `mag.hidden = false` and reset in `leavePointer`.
- **`leavePointer` comment** ("Unlike the pointer leaving #wrap, it also moves the readout's generation") matches the code and the real leave at 1177, so pass 4 N2 is closed.
- **`relayout` comment and the `setLayout` comment** are true as observed.
- **Catalogue guard and runbook.** Probe `viewer-browser.readout-survives` is red in `gate guards`. Runbook rows 580 to 583 renumbered with no gap, census 0 watched by nothing, `--check-runbook` 0. The `limit` sentence (no-change and no-repaint branches have no probe) is true by reading `readoutProblems`.
- **`bin/ocelli.sh` gate.** `aspect.mjs && readout.mjs`, so an aspect failure stops the arm and a readout failure is the arm's status. The comment's claim about case-arm status is true. Exporting `loadPlaywright` and `stopByPid` from aspect.mjs changes no behaviour of aspect.mjs.
- **Pass 1 to 4 items.** The layout change, `[` and `]`, tile toggles, layout picker by keyboard and the mid-measure survival were re-observed through readout.mjs, T7 and T8. The AC map of pass 4 is unchanged by this diff, and its committed tests are green now. The earlier passes' replay scripts were not re-run in this pass.
- No wasm, no `unsafe`, no cast, no pixel arithmetic in the diff.

## Limitations and handoff

- One synthetic CT series. A colour image (stage hidden, fault shown) under a still readout was not driven, and that is the only state where `relayout`'s own call is not redundant.
- Tile-mode wheel steps with the magnifier were read from code, not replayed.
- Full screen in headless Chromium, as pass 4. No operator Chrome run.
- No oracle, GPU or `--sprint` profile. Ports 8742 and 8765 not used, PID 98891 not touched, nothing killed by name. Every viewer started was stopped by its PID and `ps` shows none left. The scratch copy and its target directory are deleted.
- Tree unchanged: HEAD `e664ecf9`, tree `f656517`, nothing staged or unstaged.
- Next owner: the implementer fixes D1 (place the readout after a pan or zoom layout), and D2 unless the lead rules it outside F-208. Then a fresh independent pass 6. This is a bounded review, not sprint readiness, and B1 to B7 still need the operator's recorded run.

F-208 pass 5: 2 defects, 0 smells, 2 nitpicks.
