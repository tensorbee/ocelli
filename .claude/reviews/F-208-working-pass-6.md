# F-208 review, pass 6

**Reviewed**: `git diff e664ecf9 HEAD` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `7af38f7290848b9957fdbbf39412dc29687f6529`, tree `74eef4f418b8d0a8183056e8529faa5afbe06d73`. Nothing staged or unstaged before or after the pass. The untracked paths are the pass 1 to 5 reports, present before this pass, and this report.
**Author / reviewer**: the commit under review is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 6) and wrote none of the diff.
**Result**: 1 defect, 1 smell, 2 nitpicks

## Acceptance challenged

Pass 5 D1 (drags hide the readout) and D2 (the magnifier keeps a previous image), against the lead's ruling on D2: the magnifier redraws from the new image at the last pointer position, or hides.

- **Pass 5 D1, pan and zoom drags. Closed.** A per-animation-frame recorder compared the readout's C and R against a fresh `clientToImage` of the last pointer position on every frame, and every value seen was checked afterwards against a direct `fetchValue` for that pixel. Right-button pan: 208 frames, shown on all, C and R wrong on 0, 0 `POST /value` during the drag, and the text at release (`C 60  R 46  40 HU`) equals a fresh request at the end point. Middle-button zoom drag: 219 frames, 26 distinct scales, C and R wrong on 0, 20 distinct values all equal to a fresh request, 0 conflicts. Zoom tool and Pan tool left-button drags the same. So no frame paints a value from the previous geometry. The zoom drag sends about one request per move, which is the pixel under the pointer changing, coalesced by `valueState.busy`.
- **Pass 5 D2, single view. Closed.** `ArrowRight` and Play under a still pointer: the magnifier's pixels after the step equal a fresh draw at that point and differ from the previous image's (readout.mjs and probe).
- **Multi-cell 2 by 2. Holds.** Pointer at the ring edge of cell 0, cell 0 stepped: the magnifier equals a fresh draw and differs from before (discriminating). Pointer on cell 1 while active cell 0 steps: `updateMag` called 0 times, magnifier pixels and position unchanged, `mag.dataset.cell` stays `1`.
- **Four images, a tile that keeps an image. Holds.** Steps 6 to 7 to 8 with the pointer on tile 4: the magnifier equals a fresh draw each time. A wheel step on tile 1 likewise.
- **Four images, a tile that empties. Fails**, D1.
- **`pointerPlaces` under a throw. Fails**, S1.
- **`lastPointer` invalidation.** The pointer leaving `#wrap` keeps `lastPointer` and `mag.dataset.cell`, but hides `#mag`, and `refreshMag` returns on `mag.hidden`, so a step after the pointer leaves leaves the magnifier hidden (observed). A layout change goes through `leavePointer`, which hides `#mag`. Every route that shows `#mag` (`pointReadout`, `refreshMag`) sets `lastPointer` or reads it in the same call, so a shown magnifier always has a current `lastPointer`. Not invalidating it is therefore harmless on HEAD.
- **Ordinary moves.** 60 moves at 25 ms, 290 frames, readout and magnifier hidden on 0 frames, C and R wrong on 0, every value equal to a fresh request.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 74eef4f, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer` | same | now | 0 |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards` | same, OCELLI_PORT=0 | now | 0, ALL GREEN, `viewer-browser.readout-survives` `red` |
| `bin/ocelli.sh gate viewer-browser` | release build, Chromium 151.0.7922.34 | now | 0, ALL GREEN. pan 20 of 20 with 0 `/value`, zoom 20 of 20, ArrowRight and Play after equal fresh |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 ("895 file(s)"), 0 ("1758 files") |
| `target/f208-review6/probe.mjs` P1 to P7 | release viewer at HEAD, PID 98287 stopped | now | `probe.log`, only the injected page error |
| `target/f208-review6/probe2.mjs` Q1 to Q3 | same, PID 99806 stopped | now | `probe2.log` |
| `probe3.mjs` Q4, `probe4.mjs` Q5 | same, PIDs 2296 and 3181 stopped | now | stdout recorded below |
| readout.mjs against e664ecf9's page | scratch `git archive HEAD` copy with `git show e664ecf9:` page.html, own CARGO_TARGET_DIR, PID 5248 stopped, copy deleted | now | exit 1. pan 0 of 20 (19 posts), zoom 0 of 20, ArrowRight and Play "kept pixels that are not the image now shown". All four new cases red |

## Defects

### D1, in Four images the magnifier keeps the previous image when the tile under the pointer empties
**Where**: `examples/folder-viewer/src/page.html` `paintTiles`, the `!image || !image.info.monochrome` branch (about 2329 to 2335) and the `catch` branch (about 2351 to 2356). Only the painted branch calls `refreshMag` (2350).
**What**: Four images at index 0, magnifier on, pointer still on tile 4 (image 4). Move the scrubber to 9, so tile 4 has no image. Tile 4's canvas goes to width 1 and 0 px, and the magnifier stays shown with exactly its previous pixels (hash unchanged) for 2 s and until the next pointer move. No fault injection (probe4 Q5, `scrubber`). When the earlier tiles are cached the blanking happens before `drawGrey`'s `refreshMag` and the magnifier hides (`scrubber-after-warm`), so the outcome depends on whether tile 1's request awaits. With a cleared cache the same holds for an `ArrowRight` from 8 to 9 (Q4, `cold true`, shown and unchanged at 250 to 1500 ms).
**Why it is wrong**: the lead's ruling for pass 5 D2 is that the magnifier redraws from the new image at the last pointer position, or hides. Here it shows image 4 enlarged beside an empty tile. The same holds for a tile whose new image is colour, or whose request fails, by the same code. README step 8 says "When the image under a still pointer changes, by a key, the wheel or Play, the magnifier shows the new image", which this case contradicts.
**Evidence**: Q5 `{"before":"shown","index":9,"tile4Width":1,"after":"shown","sameAsBefore":true}`. Q4 cold timeline all `shown`, `sameAsBefore true`.
**Scope and regression**: present on HEAD. readout.mjs has no tile-mode image-step case, so it cannot see this. Smallest fix: in both blank branches, `if (lastPointer && lastPointer.tile === node) refreshMag(vp);` after `sizeTile(node)` (`tilePoint` then returns null and `refreshMag` hides). Regression: a readout.mjs case in Four images with the pointer on tile 4 and a jump that empties it, asserting the magnifier hidden.

## Smells

### S1, `pointerPlaces` is not cleared when `layoutViewport` or `zoomAt` throws, and a set flag stops every later geometry clear for that cell
**Where**: `page.html` `onPointerMove`, the pan branch (`pointerPlaces = true; layoutViewport(vp); pointerPlaces = false;`) and the zoom branch the same around `zoomAt`. No `try`/`finally`. `leavePointerFor` returns at once while the flag is set.
**What**: a throw from anything `layoutViewport` calls (`paintOverlays`, `syncTextScale`, `paintSelection`) on the last move of a drag leaves the flag true until the next pan or zoom move. Injected by wrapping `paintSelection` to throw once on the fourth and last move of a right-button pan (Q1): afterwards, with the pointer still, `=` leaves the readout shown, `0` (fit) leaves `C 60  R 46  40 HU` shown while the pixel under the pointer is `C 63  R 48`, and `o` leaves `C 63  R 48` while it is `C 63  R 49`. A window resize still clears, because `relayout` calls `leavePointer` itself. A throw on an earlier move is healed by the next move (probe P3), so only the last move matters.
**Why it is a smell and not a defect**: it needs an exception the viewer does not raise on the synthetic study, so it is a failure mechanism and not a reproduced fault. The consequence is the quietly wrong readout the whole F-208 family exists to prevent, and it outlives the fault that caused it, which is why it blocks.
**Scope and regression**: present on HEAD only. Smallest fix: `try { layoutViewport(vp); } finally { pointerPlaces = false; }`, the same around `zoomAt`. A browser regression is optional, the shape is the review item.

## Nitpicks

### N1, README step 8's last sentence holds for an image of the same size only
"the magnifier shows the new image" is true when the new image has the old one's size. A step to an image of another size changes `layoutViewport`'s stamp inside `drawGrey`, so `leavePointerFor` hides the cross, readout and magnifier, which is the safe direction and is covered by the sentence's opening clause ("hide when the picture moves under a still pointer"). With D1's fix the Four images case also hides. "shows the new image, or hides" would be exact. Not separately replayed, the synthetic study has one image size.

### N2, `showFault` hides the magnifier and leaves the cross and its C and R
Driving `showFault(activeViewport(), ...)` directly under a still pointer hides `#mag` and leaves `C 66  R 50  40 HU` shown over the fault. Through the real route (`showIndex` to a colour image) `showIndex` resets the value first, so `C` and `R` without a value stay over a cell showing no picture, and a later move places them again from the old image's size. That half is as at `bbf624de` and not in this diff. Recorded for the lead because the diff made the two halves disagree. Not driven through a real colour image, the synthetic study has none.

## Verified clean

- **Drag readout geometry.** C and R matched the pixel under the pointer on every recorded frame of four drag kinds, and every value matched a fresh `/value` for its pixel (P1, P2, P2b, P2c). The pan readout at release equals a fresh request at the end point, and the magnifier at release equals a fresh draw.
- **The `moving` flag and early returns.** A pan or zoom drag that starts on a mark (`drag.markId` with travel under 5) still places the readout through the added branch. `grip` and `move` never coexist with a pan or zoom `drag` for the same pointer, by the mark and grip handlers. `zoomAt`'s clamped early return still reaches `pointReadout`.
- **`refreshMag` scoping.** It draws only when `#mag` is shown and `mag.dataset.cell` names the cell, and in Four images only from the tile under the last pointer, or hides. Observed 0 draws for an unrelated cell.
- **`showFault` hides `#mag`** and clears its cell (P6).
- **readout.mjs new cases** all red on e664ecf9's page and green on HEAD. `stepProblems` refuses a non-discriminating point, and a magnifier hidden after the step passes, which matches the ruling's "or hides".
- **Catalogue and runbook.** The `refuses` and `limit` additions match `dragProblems` and `stepProblems`, and the census and `--check-runbook` agree. The eslint globals name two page functions the test calls.
- **README step 8**: "In Four images only the magnifier comes back" is true (`pointReadout` shows the cross only when `!vp.tileMode`). "During a pan or zoom drag they stay with the pointer" is true as observed. The last sentence, D1 and N1.
- **Pass 1 to 5 items.** The five geometry changes in readout.mjs still hide and a move shows again, through the gate. Ordinary moves, 0 hidden frames (P7). Earlier passes' replay scripts were not re-run.
- No wasm, no `unsafe`, no cast, no pixel arithmetic in the diff.

## Limitations and handoff

- One synthetic CT series of one image size. Colour images, mixed sizes and failing tile requests were read from code.
- S1 needs an injected exception, the page raises none on this data.
- D1's warm and cold outcomes depend on cache timing. The scrubber route reproduced it without injection on the first try in one run.
- No oracle, GPU or `--sprint` profile. Ports 8742 and 8765 not used, PID 98891 not touched, nothing killed by name. Every viewer started was stopped by its PID. The scratch copy and its target directory are deleted. Probe scripts and logs are under `target/f208-review6/`.
- Tree unchanged: HEAD `7af38f72`, tree `74eef4f`, nothing staged or unstaged.
- Next owner: the implementer fixes D1 and S1, then a fresh independent pass 7. This is a bounded review, not sprint readiness, and B1 to B7 still need the operator's recorded run.

F-208 pass 6: 1 defect, 1 smell, 2 nitpicks.
