Result: 5 defects, 1 smell, 0 nitpicks

# F-069 review, pass 1

Reviewed tree: the F-069 files are uncommitted on sprint/s26 at base 1d1e066868ca2b3591745759cd7fff9da3e9b08a. `packages/core/src/pointer.ts` and `packages/core/src/pointer.test.ts` are untracked. `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and `.claude/plans/F-069-design.md` are modified. The design diff is the `no-delta` sentence. Product source was mutated only for the checks below and then restored.

## What I ran

`npx vitest run packages/core/src/pointer.test.ts packages/core/src/index.test.ts` on the restored tree. Exit 0. 21 pointer tests and 4 index tests passed. Node v24.13.0. Vitest 4.1.11.

A temporary probe, `packages/core/src/pointer.probe.test.ts`, exercised the cases below. It was deleted before the baseline run and is not in the tree.

`packages/core/src/pointer.ts` sha256 started and ended at `382a6fbbf714d57550193ff1f223bdf39bee300fe763884b9832cdc8e5f1ea9c`. `cmp` against the pre-mutation copy was identical. The other four reviewed files were not edited by this pass. Their sha256 values are unchanged from the start of the pass.

## Mutations

Each edit was one site in `pointer.ts`. The two vitest files were re-run. The original bytes were restored before the next edit.

| Mutation | Exit | What the suite did |
|---|---|---|
| Pan stores `lastX` and `lastY` | 1 | `one frame packs the last pan` received Pan (110, 250) against (20, 70) |
| Mouse click comparison becomes `<=` 3 | 1 | `three pixels is not a click` received `pointer.click` at distance 3 |
| Pixel wheel returns the raw sum instead of its sign | 1 | `a wheel frame scrolls once` received step 52 against 1 |
| A second pointer id updates the open gesture | 1 | `a second contact does not pack` received Pan (50, 60) against (30, 20) |
| Zoom uses `lastY - segmentY` instead of `lastY - originY` | 0 | all 25 tests passed |

The first four went red. The last row is smell S1.

## Findings

### D1. A second down of the same pointer is dropped until takeFrame

Severity: defect

File: `packages/core/src/pointer.ts` line 257

Trigger: pointer id 1 goes down at (0, 0), up at (1, 0), down at (1, 0), and move to (40, 0) before `takeFrame`. The next frame moves to (80, 0). A second queue puts two clicks 100 ms apart before one `takeFrame`.

Expected: an up closes the gesture and the next down opens one, including in the same queued frame. The plan state table says idle plus a primary down captures the tool. A second click at most 400 ms after the first up is `pointer.doubleClick`. Once that second down is open, a later move packs Pan from that down's origin.

Actual: `gesture !== null || closing !== null` treats the second down as an extra contact and does not capture a tool. The same-frame names are `pointer.down`, `pointer.up`, `pointer.click`, `pointer.down`, `pointer.move`, with no command. The following move to (80, 0) is a hover and packs nothing. The two clicks 100 ms apart emit one `pointer.click` and then `pointer.down` plus `pointer.up`, with no `pointer.doubleClick`. The condition does not look at pointer id, so a second touch down is refused the same way.

### D2. Pointer capture is called without the target as receiver

Severity: defect

File: `packages/core/src/pointer.ts` lines 690 and 701

Trigger: `setPointerCapture` and `releasePointerCapture` are methods on the target. Ingest pointer id 7 down, then up.

Expected: both methods run with the target as `this`. The plan says they are used when present. A DOM element can capture only when the receiver is that element.

Actual: the functions are copied off the target and called. The probe entered both methods once and passed pointer id 7, and `this` was not the target either time. On a DOM element that call throws because `this` is not the element. The existing catch discards the throw, so the capture does not stick. The detach test shows a throwing capture still packs a pan, which is the failure rule, not a successful capture. This probe was a plain object method, not a browser. `addEventListener` is called on the target and does not have this bug. The detach test stays green because its capture method throws on purpose and does not read `this`.

### D3. Returning inside 3 CSS pixels drops a replacement that was already committed

Severity: defect

File: `packages/core/src/pointer.ts` lines 592 and 606. Zoom and rotate use the same gate at lines 622 and 633.

Trigger: basis pan (10, 20). Down at (0, 0), move to (100, 0), `takeFrame` packs Pan (110, 20). The next frame moves to (2, 0). Separately, a Scroll drag from y 0 to y 10 packs step 1, and the next frame moves to y 2.

Expected: `set_pan` replaces, so the return frame is Pan (12, 20), basis plus (2 - 0). A scroll drag contributes the sign of that frame's y change, which is sign(2 - 10) = -1. The move row says one command from the last point. The `no-delta` sentence added with this implementation says the pointer has not moved by 3 CSS pixels. This pointer already moved 100 CSS pixels, and the scroll frame's step is not zero.

Actual: both return frames have no command and reason `no-delta`. `segmentY` still advances, so the scroll's -8 px is not carried into a later frame. The committed pan stays (110, 20). An up at x 2 is `pointer.click`. That click name matches the net-under-3 rule and is not the defect. The missing replacement is.

### D4. An ambiguous wheel deletes the captured scroll drag

Severity: defect

File: `packages/core/src/pointer.ts` lines 539 to 546

Trigger: two active Scroll tools. Primary drag from y 0 to y 10 and a line-mode wheel of deltaY 4 in the same frame. The same drag with no wheel packs Scroll step 1.

Expected: the captured drag still packs Scroll step 1. The wheel does not add a step. The plan says a wheel packs only when exactly one active tool has command Scroll, and that two tools is `ambiguous-scroll` for that wheel. The drag already captured one owner at pointer down. The frame sums wheel contributions only when the wheel is allowed.

Actual: the wheel branch filters out every Scroll, then reports `ambiguous-scroll` with tool null and command null. Commands are empty. The drag step is discarded.

### D5. `no-delta` is the fallback for frames the sentence does not describe

Severity: defect

File: `packages/core/src/pointer.ts` line 514. The definition is `.claude/plans/F-069-design.md` line 91.

Trigger: a pointer move of 20 px with no down. Also a down, a move to x 30, and `pointercancel` in one frame. A button 3 down followed by a 20 px move takes the same hover path.

Expected: cancel packs nothing and is not a click. The reason is not `no-delta`. The added sentence says `no-delta` means the frame has events, the gesture is eligible, and it packs nothing because the pointer has not moved by 3 CSS pixels or a scroll step is zero. A hover has no gesture. The cancelled sample moved 30 px and was `pointer.drag`.

Actual: hover, the button 3 move, and the cancelled drag all report `no-delta`. The cancel frame's commands are empty, which is right, and its events are `pointer.down` and `pointer.drag`. The reason claims the click slop.

## Smells

### S1. The zoom test stays green if a later frame replaces instead of using the origin

Severity: smell

File: `packages/core/src/pointer.test.ts` lines 120 to 131. Formula under test: `packages/core/src/pointer.ts` line 626.

Trigger: change the zoom factor to `basis.zoom * exp(-(lastY - segmentY) / 256)` and re-run the suite. That is per-frame replacement. `set_zoom` would keep only the latest frame's delta.

Expected: AC3 says `zoom and rotate use the gesture origin` rejects per-event replacement. After a frame at y 256, a frame at y 512 is `exp(-2)`.

Actual: the suite exits 0. The shipped zoom test has one move of 256 in the first frame, and on that frame `segmentY` is still the origin, so the wrong formula matches `exp(-1)`. The restored formula was checked across two frames and the second factor was about 0.135, which is `exp(-2)`. The behaviour is correct now. The rotate test is one sample of 256 as well. That rotate replacement was not mutated.

## Checked, not filed

These were executed on the restored dispatcher and matched the plan.

Basis is frozen at pointer down. Moving the live basis panX from 10 to 500 after the first frame still produced Pan x 30, the original basis plus the total delta. Setting the captured tool passive after down still packed that pan. A tool that is passive at down is `no-owner`. Pen uses the pointer names. Buttons 1 and 2 pack auxiliary Pan and secondary Zoom. Line mode `trunc(-1.9)` is step -1. Omitted `deltaMode` is the sign of `deltaY`. Pixel, line, and page samples in one frame sum. A line delta of -3000000000 clamps to -2147483648. Two touch taps stay `touch.tap`. A second click whose down is 3 px from the first up is not a double click. Detach leaves a queued pan in place, and the next `takeFrame` is `empty` with a zero-length buffer. Fit, both flips, ApplyPreset, SetInvert, and SetColormap on a 40 px move are `payload-not-in-gesture`. Destroying the group before the frame is `unknown-group`. A second pointer alone is `ignored-contact` and packs nothing. With a primary move, the reason stays `packed` and the pan uses the primary delta. A NaN basis is `missing-basis`. A wheel with no Scroll tool is `no-owner`. A pixel pair that sums to zero packs nothing. Distance hypot(2, 2) is a click. The listener list is `pointerdown`, `pointermove`, `pointerup`, `pointercancel`, and `wheel`, and detach removes all five.

A SetWindow drag plus a line wheel packed Scroll step 9 while `command` stayed SetWindow and `tool` stayed the window tool. Not filed. The plan asks for the captured tool, and the commands array is the scroll.

The source does not call `submitCommands`, `writeCommands`, `alloc`, or wasm. Empty command lists do not call `packCommands`. `index.ts` publishes `DISPATCH_REASONS` and `createPointerDispatch`. The erased-type comment names the pointer types. F-072 was not reviewed.

No wasm, no browser, no `tsc`, no eslint, and no cargo. Tiers A, B, and C are not applicable. There is no pixel fixture.
