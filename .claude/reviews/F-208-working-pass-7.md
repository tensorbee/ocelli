# F-208 review, pass 7

**Reviewed**: `git diff 7af38f72 HEAD` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `b6d2fcca31b5c057ab96fd8e8a6ad5336963483a`, tree `dd1348fdc82fda3f25b324f7df1a26469437b196`. Nothing staged or unstaged before or after the pass. The untracked paths are the pass 1 to 6 reports, present before this pass, and this report.
**Author / reviewer**: the commit under review is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 7) and wrote none of the diff.
**Result**: 2 defects, 0 smells, 0 nitpicks

Both defects are present at the base `bbf624de` in the same shape, so F-208 did not introduce them. They are reported because the pass was asked to sweep the whole pointer readout and magnifier family, and both are a magnifier showing a picture the cell does not show. The lead may rule them out of F-208 and into a new story. Either way they are open.

## Acceptance challenged

- **Pass 6 D1, an emptied tile kept the magnifier. Closed.** Four images at index 0, pointer still on tile 4. The scrubber to 9 cold: hidden at 100, 400 and 1500 ms, tile 4 width 1. `dropCache()` then `ArrowRight` from 8 to 9: the same. The catch branch, with `requestGrey` wrapped to reject tile 4's request and the cache dropped, then `ArrowRight`: hidden, tile 4 width 1. Adjacent case: the pointer on tile 2 while tile 4 empties keeps the magnifier shown, changed, and equal to a fresh draw at that point. So the added `refreshMag` fires for the tile under the pointer only.
- **Pass 6 S1, `pointerPlaces` under a throw. Closed.** `pointerPlaces = true` is set at two sites only (page.html 2881 and 2891), and both are now inside `try`/`finally`. With `paintSelection` throwing on the fourth and last move, for a right-button pan and a middle-button zoom drag, each followed by `=`, `0` and `o` with the pointer still: `pointerPlaces` false after the throw in all six, and the readout, cross and magnifier hidden after the key in all six.
- **Pass 6 N2 ruling, `showFault` hides the cross too. Closed as ruled.** 2 by 1, readout and magnifier on cell 1, `showFault(viewports[0])`: the readout, cross and magnifier on cell 1 stay shown, the magnifier pixels are unchanged, `mag.dataset.cell` stays `1`. With the pointer on cell 0 the same call hides all three and clears the cell. So `leavePointerFor` fires for the faulting cell only. What a later move does over that cell is D1.
- **Pass 6 N1, README step 8.** It now ends "the magnifier shows the new image, or hides". Each half was observed: shows on `ArrowRight`, Play, invert and Four images invert, hides when the tile empties.
- **Family sweep.** These were checked as image or geometry changes under a still pointer: a step, Play, the scrubber, invert in single view and in Four images, a tile that empties, a tile request that fails, a main preview that fails, and a fault by direct call. They were checked in single view, 2 by 1, and Four images. Also checked were what a pointer move does after each of these, and a move onto parts of a cell that show no picture. Every still-pointer change was clean. Two pointer-move routes were not, D1 and D2.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree dd1348f, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer` | same | now | 0 |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards` | same, OCELLI_PORT=0 | now | 0, ALL GREEN, 434 refusal probes, `viewer-browser.readout-survives` `red` |
| `bin/ocelli.sh gate viewer-browser` | release build, Chromium 151.0.7922.34, viewer PID 32446 | now | 0, ALL GREEN. Emptied tile `806723252 before, 806723252 after, hidden`. Thrown layout `threw true`, after hidden `true/true` |
| `bin/ocelli.sh gate ci` | same | now | 0, ALL GREEN |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 ("895 file(s)"), 0 ("1758 files") |
| `target/f208-review7/probe.mjs` A to F | release viewer at HEAD, PID 33552 stopped | now | `probe.log`. Page errors are the six injected throws only |
| `target/f208-review7/probe2.mjs` D2, E2 | same, PID 35098 stopped | now | `probe2.log` |
| HEAD's readout.mjs against 7af38f72's page | scratch `git archive HEAD` copy with `git show 7af38f72:` page.html, own CARGO_TARGET_DIR, PID 37321 stopped, copy deleted | now | exit 1. "tile 4 empties in Four images: the magnifier kept the pixels of an image the tile no longer shows" (shown, hash equal) and "a zoom after a throw inside a drag ... left the readout or magnifier showing" (after hidden `false/false`). Every other case green, so the two new cases discriminate the fix |

## Defects

### D1, a pointer move over a cell showing a fault shows the previous image in the magnifier and a value readout for the current one
**Where**: `examples/folder-viewer/src/page.html` `pointReadout` (3104 to 3140), `const point = tile ? tilePoint(tile, event) : imagePoint(vp, event);`. `imagePoint` reads `vp.imageWidth` and the view transform and does not look at `vp.dom.stage.hidden` or the fault. `updateMag` then draws from `vp.dom.canvas`, which `showFault` hides but does not clear.
**What**: single view, cross and magnifier on, at index 0. The pointer rests at the ring edge (image point 104.5, 49.5), where image 1 and image 2 differ. `requestGrey` is wrapped to reject the main preview, the cache is dropped, then `ArrowRight`. The cell shows "injected main failure" with the stage hidden, and the still-pointer half is right: all three hide. Then a move away and back to the same point shows the cross, the readout `C 105  R 50  40 HU` and the magnifier. The magnifier's pixels equal image 1's fresh draw at that point and differ from image 2's. The case is discriminating: image 1 and image 2 hash differently there, and image 1 hashes the same again after a round trip. The readout value comes from `POST /value` for image 2, the image that failed to show. So the screen shows a fault sentence, a magnified piece of image 1, and an HU value from image 2, all at once.
**Why it is wrong**: the lead's pass 5 D2 ruling is that the magnifier shows the image the cell shows, or hides. The pass 6 N2 ruling is that a cell showing a fault has no readout or cross. The diff's own comment says "A cell showing a fault shows no picture, so no readout or magnifier stays over it", which is true only until the next move. `refreshMag`'s comment says "it never shows an image the cell no longer shows". `refreshMag` makes that true for a still pointer, and `pointReadout` makes it false for a moving one. This is the quietly wrong picture CLAUDE.md names, because the magnifier looks like a valid enlargement.
**Evidence**: probe2 D2 `{"fault":"injected main failure","stageHidden":true,"index":1,"discriminating":true,"magAfter":"shown","magEqualsImage1":true,"magEqualsImage2":false,"readout":["shown","C 105  R 50  40 HU"]}`. Probe C, `showFault` by direct call then a move: `nextMove ["shown","shown","shown","C 67  R 50  40 HU"]`.
**Scope and regression**: present on HEAD and at `bbf624de`, where `pointReadout` already used `imagePoint(event)` and `updateMag` drew from `canvas`. Pass 6 N2 recorded the cross half ("a later move places them again from the old image's size") as pre-existing. The magnifier showing the previous image, and the value readout for the failed image, are new evidence. Smallest fix: in `pointReadout`, take `point` as null when the cell shows no picture, the same rule `refreshMag` applies, for example `tile ? tilePoint(tile, event) : (vp.tileMode || vp.dom.stage.hidden ? null : imagePoint(vp, event))`. This also hides the cross, because the cross needs `point`. Regression: a readout.mjs case that makes the main preview fail (the probe's `requestGrey` wrap), moves, and asserts the readout, cross and magnifier hidden.

### D2, in Four images a pointer over a tile's letterbox shows the hidden single view in the magnifier
**Where**: the same line of `pointReadout`. In tile mode, a pointer that is over the `.tile` box but not its canvas has `tile` null, so `imagePoint(vp, event)` maps it through the single view's hidden geometry. `updateMag(vp, event, null, point)` then draws `vp.dom.canvas`, which is the hidden single-view canvas.
**What**: Four images at index 0, magnifier on. Each tile canvas (400.9 px wide) is centred in a `.tile` box 551.5 px wide, so there is a 151.6 px strip between tile 1's picture and tile 2's that shows no image. `elementFromPoint` there is `DIV.tile`. Resting the pointer in that strip shows the magnifier with 21354 lit pixels. To prove where they come from, the hidden single-view canvas was filled red and the pointer moved again: the magnifier was 25600 of 25600 pixels red. The magnifier shows image 1 at the single view's geometry, beside a tile that shows another image, over a place with no picture at all.
**Why it is wrong**: the same ruling as D1. `refreshMag` gives `vp.tileMode ? null` for this exact state, so the two paths disagree: a still pointer hides the magnifier here and a moving one shows it. README step 8 says "In Four images only the magnifier comes back", meaning over a tile.
**Evidence**: probe2 E2 `{"under":"DIV.tile","stageHidden":true,"tileMode":true,"gapWidthPx":151.5625,"before":["shown",21354],"afterRedHiddenCanvas":["shown",25600,25600]}`. Probe E without the red fill: `mag "shown"`.
**Scope and regression**: present on HEAD and at `bbf624de`, same line shape. The cross stays hidden here because `pointReadout` shows it only when `!vp.tileMode`, so only the magnifier is affected. The fix is the same one-line change as D1. Regression: a readout.mjs case in Four images with the pointer in the strip between tile 1 and tile 2, asserting the magnifier hidden.

## Smells

None.

## Nitpicks

None.

## Verified clean

- **Diff, page.html.** Both added `refreshMag` calls are guarded by `lastPointer.tile === node` and run after `sizeTile`, so `tilePoint` sees a zero-size rect and `refreshMag` hides (A, A2). The painted branch's call is unchanged and still redraws (A3, F Four images). `showFault` hides only through `leavePointerFor(vp)`, so the clearing is scoped to the cell and respects `pointerPlaces` (C). Nothing calls `showFault` during a drag's layout, so that early return cannot skip a fault. The `try`/`finally` covers both sites that set `pointerPlaces` (grep, B).
- **readout.mjs.** The two new cases go red on 7af38f72's page and green on HEAD, with every other case green both ways, so each discriminates its own fix. `faultProblems` refuses an unobserved case, a throw that did not happen and a nothing-shown-before state. The emptied-tile case accepts "hidden", which is the ruling's "or hides". The thrown case replaces `window.paintSelection`, which the page's classic script resolves by global name, and `threw true` confirms that the injection reached the layout.
- **Comments and prose, executed.** README step 8's new sentence: both halves observed. readout.mjs header: "the scrubber moves to image 10, so tile 4 has no image". The series has 12 images (probe), so tile 4 would be image 13, and tile 4 width is 1 (observed). The eslint comment "three of the page's own top-level functions": `imageToClient`, `activeViewport` and `paintSelection`. The `Event` and `window` globals are used. The catalogue `refuses` additions match `faultProblems`. The `limit` and runbook sentences agree (`--check-runbook` 0) and are true: the only probe drives `readoutProblems`' survival branch. use.html and more.html have no magnifier or readout sentence.
- **Still-pointer family, every route clean.** Step, Play, scrubber, invert in single view (changed, equals fresh) and in Four images (changed, equals fresh), an emptied tile, a failed tile, a failed main preview and a direct fault. Multi-cell scoping as in pass 6 and C. Drags as in pass 6, and the gate's 20 of 20 again.
- **Pass 1 to 6 items** hold through the gate: five geometry changes hide and a move shows again, pan 0 `/value`, and ArrowRight and Play equal fresh.
- No wasm, no `unsafe`, no cast and no pixel arithmetic in the diff.

## Limitations and handoff

- One synthetic CT series with one image size. Colour images, mixed sizes and real failing requests were not available, so failures were injected by wrapping `requestGrey` and `paintSelection`. A colour image reaches `showFault` by the same route as D1 by code reading, not replayed.
- D1 and D2 are pre-existing at `bbf624de`. Scope is for the lead to decide.
- No oracle, GPU or `--sprint` profile. Ports 8742 and 8765 not used, PID 98891 not touched, nothing killed by name. Every viewer started (32446, 33552, 35098, 37321) was stopped by its PID and confirmed gone. The scratch copy and its target directory are deleted. Probe scripts and logs are under `target/f208-review7/`.
- Tree unchanged: HEAD `b6d2fcca`, tree `dd1348f`, nothing staged or unstaged.
- Next owner: the lead rules on D1 and D2's scope. If they stay in F-208, the implementer applies the one-line `pointReadout` fix with the two regression cases, then a fresh independent pass 8 runs. This is a bounded review, not sprint readiness.

F-208 pass 7: 2 defects, 0 smells, 0 nitpicks.
