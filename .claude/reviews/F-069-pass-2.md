Result: 2 defects, 0 smells, 0 nitpicks

# F-069 review, pass 2

**Reviewed**: working tree on `sprint/s26` at HEAD `1d1e066868ca2b3591745759cd7fff9da3e9b08a`. `packages/core/src/pointer.ts` and `packages/core/src/pointer.test.ts` are untracked. `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and `.claude/plans/F-069-design.md` are modified. `.claude/reviews/F-069-pass-1.md` was read and not edited. Product source was mutated only for the check below and then restored.
**Author / reviewer**: the author is the F-069 implementer of this uncommitted tree. The reviewer is an independent pass-2 reviewer and did not write this code. This pass did not resume the pass 1 reviewer.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Prior findings

| ID | Replay | Result |
|---|---|---|
| D1 | Down at (0, 0), up at x 1, down at x 1, move to x 40, then a later move to x 80 | Pan x 49, then Pan x 89. Two `pointer.down` events. Closed. |
| D1 | Two clicks 100 ms apart in one `takeFrame` | `pointer.click` and `pointer.doubleClick`. Closed. |
| D2 | Class `setPointerCapture` and `releasePointerCapture` for pointer id 7 | Both ran with the target as `this`. A second id was not captured. Closed. |
| D3 | Pan through (100, 80), then back to (2, 1), then up there | Pan (110, 100), then (12, 21). The up is a click and still Pan (12, 21). Closed. |
| D3 | Scroll drag y 10, then y 2 | Step 1, then -1. An up from y 10 to y 2 is step -1 and a click. Closed. |
| D4 | Two active Scroll tools, primary drag y 10, line wheel 4 | Scroll step 1, reason `packed`, tool `primary-tool`. A later wheel-only frame is `ambiguous-scroll` and empty. Closed. |
| D5 | Hover of 20 px, button 3 plus a 20 px move, and down plus move to x 30 plus `pointercancel` | All three are `no-gesture` with no command. A 2 px move with the button down stays `no-delta`. Closed. |
| S1 | Zoom factor uses `lastY - segmentY` | The restored suite exits 1. The second frame is `exp(-1)` against `exp(-2)`. Closed. |

## Acceptance challenged

AC3, AC5, AC6, AC23, AC25, AC26, AC28, and AC29 were replayed through `createPointerDispatch` from `packages/core/src/index.ts`. The second zoom frame rejects a segment delta. Pixel deltas 0.4 and 0.4 reject a per-event sign. The line pair below rejects saturating addition. A pointerup with no down rejects `ignored-contact` and the implementation fails that case.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/pointer.probe.test.ts` | Deleted probe. Published `createPointerDispatch`. | Executed now. Not a receipt. | The spec-matching cases below passed. The two defects failed until the probe recorded the actual values. |
| `npx vitest run packages/core/src/pointer.test.ts packages/core/src/index.test.ts` | Zoom uses `segmentY` | Executed now | Exit 1. 1 failed, 32 passed. Second zoom factor was about 0.368 against about 0.135. |
| Same vitest command | Restored `pointer.ts` sha256 `09d805d2fbbc13d5cf5a2869ccf494cb702963882b482d87855fdec89146aae1` | Executed now | Exit 0. 29 pointer tests and 4 index tests. |
| `npx tsc --build` | Restored tree | Executed now | Exit 0. |
| `npx eslint` on the four TS files | Restored tree | Executed now | Exit 0. |

Node v24.13.0. Vitest 4.1.11. `cmp` against `/tmp/pointer.ts.f069-pass2` was identical after the mutation. `pointer.test.ts` sha256 stayed `9f819055feb6cbc7c0d6b8a6eedbadd5d62da1a2f412d1efa2c10e83febfc7b9`. `index.ts`, `index.test.ts`, and the design hash were unchanged from the start of the pass. No probe file remains.

## Defects

### D1. An up with no open gesture is reported as ignored-contact

**Where**: `packages/core/src/pointer.ts` lines 380 and 599. The frame starts at `no-gesture` on line 517.

**What**: `pointerup` and a touch end with no open gesture set `sawExtra`. `takeFrame` then returns `ignored-contact`. Commands are empty. The event is `pointer.up` or `touch.end`. A hover and that up in one frame is also `ignored-contact`.

**Why it is wrong**: `.claude/plans/F-069-design.md` says `ignored-contact` is used when a second pointer id is the only input in the frame. `no-gesture` means the frame has events and no eligible gesture. A stray up has events and no gesture. It is not a second pointer id. A move with no gesture does not set `sawExtra`, so a hover stays `no-gesture`.

**Evidence**: The deleted probe ingested `pointerup` at x 4 with no down. The reason was `ignored-contact`. A touch end did the same. A `pointer.move` of 8 px and then `pointerup` in one frame was `ignored-contact`. A second pointer move while a gesture was open, with no primary sample, stayed `ignored-contact`, and the next primary move packed from the original origin.

**Scope and regression**: Callers that branch on `reason`. The camera does not move, because the buffer is empty. Smallest regression is one `pointerup` before any down, expecting `no-gesture` and no command. Present on this working tree. Not a pass 1 finding.

### D2. Wheel steps saturate each addition instead of summing and then clamping

**Where**: `packages/core/src/pointer.ts` `wheelStepOf` lines 762 to 770, the drag plus wheel sum on line 574, and `addI32` lines 851 to 859.

**What**: Line truncations, page signs, the pixel sign, and a scroll drag step are combined with saturating i32 addition. A later opposite contribution is applied to the already clamped total. The same two line samples pack different steps in opposite orders.

| Samples in one frame | Observed step | Sum, then clamp |
|---|---|---|
| Line 3000000000 then line -1000000000 | 1147483647 | 2000000000 |
| Line -1000000000 then line 3000000000 | 2000000000 | 2000000000 |
| Line 3000000000 then line -3000000000 | -852516353 | 0, so no Scroll |
| Line 3000000000 then pixel deltaY -100 | 2147483646 | 2147483647 |
| Scroll drag y -10 plus line 3000000000 | 2147483646 | 2147483647 |

**Why it is wrong**: The plan says line mode contributes `trunc(deltaY)` per event, summed, and the frame sums those contributions and clamps to the i32 range. Zero packs no Scroll. A sum does not change with order. The cancelling pair must not pack a Scroll. Same-sign overflow still matches, which is why one line sample of 3000000000 clamps to 2147483647 under both readings. One line sample of -3000000000 clamps to -2147483648.

**Evidence**: The deleted probe, on one active Scroll tool, observed the steps in the table. The restored suite exits 0. `wheel mode changes the step` uses one positive line sample of 3000000000, so it stays green.

**Scope and regression**: A wheel frame for the one active Scroll tool, including a captured scroll drag in that frame. `scroll_by` adds the step. Smallest regression is two line-mode wheels, deltaY 3000000000 and deltaY -3000000000, in one `takeFrame`, expecting no Scroll. Present on this working tree.

## Smells

None.

## Nitpicks

None.

## Verified clean

These were executed on the restored dispatcher and matched the plan.

Basis is taken from the function at pointer down. After a frame at x 20, changing that function from panX 10 to 500 still produced Pan x 35 on a move to x 25. Setting the captured tool passive after down still packed the next pan. Pan is basis plus delta from the gesture origin. One frame with two moves packs one Pan. Exactly 3 CSS pixels packs and is not a click. `hypot(2, 2)` is a click and `no-delta`. A y of 3 packs Pan y 23.

Zoom at y 256 then y 512 is `exp(-1)` then `exp(-2)` in the permanent test. A later move back to y 0 packs factor 1. Rotate at x 256 then x 512 is 1 then 2. A later move back to x 0 packs radians 0. A non-finite zoom basis path still returns `non-finite` in the permanent test. A null basis and a NaN radian are `missing-basis`.

Pixel mode is the sign of the summed deltaY. 0.4 plus 0.4 is step 1. 0.8 is 1. -0.8 is -1. 5 and -5 is `no-delta`. An omitted `deltaMode` uses that pixel rule. Line mode `trunc(-1.9)` is -1, and two of those are -2. Page mode sums signs. Samples 0, 0.5, and 8 are step 2. Mixed pixel 0.8, line 1.9, and page -4 are step 1. A scroll drag of y 10 plus pixel 0.8 is one Scroll of 2. The next frame, drag step 1 plus a pixel sign of -1, is `no-delta`.

A Pan drag plus a line wheel packs Pan and Scroll in one buffer. `tool` and `command` stay the captured pan tool. A wheel with no Scroll tool is `no-owner`. Two Scroll tools and a wheel do not add a step. Fit, FlipHorizontal, FlipVertical, SetWindow, ApplyPreset, SetInvert, and SetColormap on a 40 px move are `payload-not-in-gesture` with no command. A Fit click uses that same reason.

Touch uses the touch names. Mouse and pen use the pointer names. There is no `touch.move` and no touch double-click name. Two quick touch taps stay `touch.tap`. A second touch down before `takeFrame` opens a gesture and a later move pans from that down. A touch of 24 px within 300 ms is `touch.swipe`, not `touch.tap`, and packs Pan. Button 2 zooms and button 1 pans. Button 3 does not start a gesture. A lone button 3 down is `empty`, because the down is dropped and the queue has nothing to report. That is not filed. The button 3 frame that also moves is `no-gesture`.

A second pointer id during an open gesture does not change the packed pan. Its event is still queued. When that second id is the only sample, the reason is `ignored-contact`. A same-frame Pan drag and Zoom drag keep both commands. A horizontal scroll drag is `no-delta`. Same-frame scroll samples y 10 then y -4 pack the sign of the net change, which is -1. An up that does not change y after a committed scroll packs no extra step.

A click at 400 ms is a double click. 401 ms is not. A second down 3 px from the first up is not a double click. A second down at (2, 2) and 400 ms is. A third click after a double click is `pointer.click`. A drag does not arm a double click. Modifier order ctrl then meta still finds the owner.

Cancel of a different pointer id does not release capture and does not drop the gesture. Cancel of the captured id releases capture, packs nothing, and is `empty` when it is the only input. The plan asks for `no-gesture` when that frame has events, and cancel has no shell name. The permanent test expects `empty`. Not filed. Same-frame cancel after a drag is `no-gesture` and is not a click.

Detach removes `pointerdown`, `pointermove`, `pointerup`, `pointercancel`, and `wheel`. A queued pan is still returned. The next `takeFrame` is `empty` and does not replay. Empty command lists use a zero-length buffer. The source calls `packCommands` only when the list is non-empty. It does not name `submitCommands`, `writeCommands`, `packPointerFrame`, `alloc`, or wasm. Infinity and NaN coordinates are dropped, and a later valid scroll still packs. Destroying the group is `unknown-group`. Detaching the viewport is `missing-viewport`.

`index.ts` publishes `DISPATCH_REASONS` and `createPointerDispatch`. The erased-type comment names the pointer types. `DISPATCH_REASONS` has 12 distinct names and matches the plan. Tiers A, B, and C are not applicable. There is no pixel fixture and no DICOM arithmetic.

The returned basis object is kept by reference. Writing `panX` on that object changed the next pan from 40 to 530. Not filed. The plan captures the object `basis()` returns, and `GestureBasis` fields are readonly. A later call to `basis()` is ignored.

## Limitations and handoff

No browser, no wasm, no `bin/ocelli.sh`, and no GPU. Capture was a class method, not a DOM element. A second pointer up while a gesture is open was not executed. The second pointer move was, and the unmatched up shares the `sawExtra` branch only when a gesture is open. Two Pan gestures in one frame were not executed. Pan and Zoom in one frame were. A second Pan would be another `set_pan` replacement from its own captured basis. `events.ts`, `commands.ts`, Rust, and F-072 were not reviewed beyond `packCommands` and the independent `set_pan`, `set_zoom`, `set_rotation`, and `scroll_by` operations.

This is not completion. The implementer should fix the two defects and a new independent pass should review that tree. The product bytes were restored. The probe was deleted.
