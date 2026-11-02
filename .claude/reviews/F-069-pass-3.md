Result: 0 defects, 0 smells, 0 nitpicks

# F-069 review, pass 3

**Reviewed**: working tree on `sprint/s26` at HEAD `1d1e066868ca2b3591745759cd7fff9da3e9b08a`. `packages/core/src/pointer.ts` and `packages/core/src/pointer.test.ts` are untracked. `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and `.claude/plans/F-069-design.md` are modified. Pass 1 and pass 2 were read and not edited. Product source was mutated only for the check below and then restored.
**Author / reviewer**: the author is the F-069 implementer of this uncommitted tree. The reviewer is an independent pass-3 reviewer, did not write this code, and did not resume pass 1 or pass 2.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Prior findings

| ID | Replay | Result |
|---|---|---|
| Pass 2 D1 | `pointerup` at x 4 with no down, a touch end, and a hover of 8 px plus that up | All three are `no-gesture` with no command. The touch name is `touch.end`. Closed. |
| Pass 2 D1 | Second pointer move while a gesture is open, then a later primary move | `ignored-contact`, then Pan from the original origin `(5, 7)` to `(25, 11)`, which is `(30, 24)`. A primary 1 px move beside that second id stays `no-delta`. Closed. |
| Pass 2 D2 | Line `3000000000` and `-3000000000` in both orders, line `3000000000` and `-1000000000` in both orders, line `3000000000` plus pixel `-100` in both orders, one line of each sign, drag y `-10` plus line `3000000000` in both ingest orders | No Scroll, step `2000000000`, step `2147483647`, step `2147483647`, step `-2147483648`, and step `2147483647`. A zero sum is `no-delta`. Closed. |
| Pass 1 D1 | Down, up at x 1, down at x 1, move to x 40, then a later move to x 80 | Pan x 49, then Pan x 89. Two clicks 100 ms apart emit `pointer.click` and `pointer.doubleClick`. Closed. |
| Pass 1 D2 | Class `setPointerCapture` and `releasePointerCapture` | Both ran with the target as `this` for pointer id 7. Pointer id 8 was not captured. Cancel of id 7 released with the target as `this`. Closed. |
| Pass 1 D3 | Pan through `(100, 80)`, back to `(2, 1)`, then up there. Scroll y 10 then y 2. Up from y 10 to y 2 | Pan `(110, 100)`, then `(12, 21)`. The up is a click and still Pan `(12, 21)`. Scroll steps are 1, then -1. The direct up is step -1 and a click. Closed. |
| Pass 1 D4 | Two active Scroll tools, drag y 10, line wheel 4, then a later wheel-only frame | Scroll step 1, reason `packed`, tool `tool`. The later wheel is `ambiguous-scroll` and empty. The next drag to y 2 is step -1. Closed. |
| Pass 1 D5 | Hover of 20 px, button 3 plus a 20 px move, and down plus move to x 30 plus `pointercancel` | All three are `no-gesture` with no command. A 1 px click is `no-delta`. Closed. |
| Pass 1 S1 | Zoom factor uses `segmentY` | The restored suite exits 1. The second factor is about 0.368 against about 0.135. Closed. |

## Acceptance challenged

AC3, AC5, AC6, AC7, AC8, AC23, AC25, AC26, AC27, AC28, AC31, and AC32 were replayed through `createPointerDispatch` imported from `packages/core/src/index.ts`. The published function is the caller, not a copy of the assertions.

AC3 was challenged with a non-zero origin and a non-identity basis, which the permanent fixture does not use. Zoom basis 2, down at y 100, then y 356 and y 612, is `2 * exp(-1)` then `2 * exp(-2)`. A return to y 100 is factor 2. Rotate basis 0.25, down at x 10, then x 266 and x 522, is 1.25 then 2.25. A return to x 10 is 0.25. That rejects both a segment delta and a formula that drops the origin or the basis term.

AC32 was challenged with a sum that per-add saturation gets wrong. Line `2147483647`, page `1`, and line `-10` are step `2147483638`. Saturating each addition would leave `2147483637`. Drag step 1 plus line `3000000000` and line `-3000000000` stays step 1. Order does not change these results.

AC31 was challenged with the touch end name and with a hover in the same frame as the stray up. Neither is `ignored-contact`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/pointer.probe.test.ts` | Deleted probe. Published `createPointerDispatch`. 23 cases, including the replays above. | Executed now. Not a receipt. | Exit 0. 23 passed. The probe was deleted before the restored run. |
| `npx vitest run packages/core/src/pointer.test.ts packages/core/src/index.test.ts` | Zoom uses `segmentY` instead of `originY` | Executed now | Exit 1. 1 failed, 34 passed. Second zoom factor was 0.36787944117144233 against 0.1353352832366127. |
| Same vitest command | Restored `pointer.ts` sha256 `0b359fd185998ef14c8c6aa963ad85ccdaf516b10852aab502226abff60c0a63` | Executed now | Exit 0. 31 pointer tests and 4 index tests. |
| `npx tsc --build` | Restored tree | Executed now | Exit 0. |
| `npx eslint` on the four TS files | Restored tree | Executed now | Exit 0. |

Node v24.13.0. Vitest 4.1.11. `cmp` against `/tmp/pointer.ts.f069-pass3` was identical after the mutation. `pointer.test.ts` sha256 stayed `b587ff4bc30f7c2d5110e286427f183eaaebafd704be60e81d87676983065558`. `index.ts` stayed `db7ee52364e6fb2a8d01192db125b5b4040786cb2e45b04b1389acee62ba586f`. `index.test.ts` stayed `6211b10b98fcc06daea7d344714e3937c7117553d6f9a9c7bbdc189a461bf71e`. The design sha256 stayed `1433daee7af1e56eb9fc4a29afdb8997161411278b29d44428a4f1415e288cf4`. No probe file remains.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

These were executed on the restored dispatcher, through the published export, and matched the plan.

A stray `pointerup` is `no-gesture` and the event is `pointer.up`. A stray touch end is `no-gesture` and the event is `touch.end`. A hover and that up in one frame are `no-gesture`. `ignored-contact` remains the frame whose only samples belong to a second pointer id while a gesture is open. The next primary move packs from the original origin. A primary sample in the same frame keeps the primary reason, including `no-delta` when the primary has moved only 1 px.

Wheel contributions sum as JavaScript numbers and clamp once. Both orders of the cancelling pair pack no Scroll. Both orders of `3000000000` and `-1000000000` are step `2000000000`. Both orders of line `3000000000` and pixel `-100` are step `2147483647`. One line of `3000000000` is `2147483647`. One line of `-3000000000` is `-2147483648`. A drag of y `-10` plus that line is `2147483647` whether the wheel is ingested before or after the drag. A drag of y 10 plus line `-3000000000` is `-2147483648`. Line `2147483647` plus line 1 clamps to `2147483647`. Line `-2147483648` plus line `-1` clamps to `-2147483648`. The bytes for these steps equal `packCommands` of the same commands, so the packer accepts the clamped i32. A zero sum packs no Scroll and the reason is `no-delta`.

Pixel mode is the sign of the summed `deltaY`. 0.4 plus 0.4 is step 1. 5 and -5 is `no-delta`. An omitted `deltaMode` uses that rule. Line mode `trunc(-1.9)` twice is step -2. Page mode samples 0, 0.5, and 8 are step 2. Mixed pixel 0.8, line 1.9, and page -4 are step 1. A scroll drag of y 10 plus pixel 0.8 is one Scroll of 2. The next frame, drag step 1 plus a pixel sign of -1, is `no-delta`. A drag that is still inside 3 px does not swallow a line wheel of 4. That frame is Scroll step 4.

Pan, zoom, and rotate use the gesture origin. The permanent two-frame zoom test rejects `segmentY`. The probe also rejects a zero origin and a basis of 1 or 0, as written above. A return inside 3 px still replaces the command. An up whose net distance is under 3 is still a click. `hypot(2, 2)` is a click and `no-delta`. A vertical move of exactly 3 is Pan y 23 and is not a click. Same-frame scroll samples y 10 then y -4 are step -1, the sign of the net change. A horizontal scroll of x 20 is `no-delta`.

The basis object is the one `basis()` returned at pointer down. Replacing the function and setting the tool passive still produced Pan x 35 from the original basis. Button 1 pans and button 2 zooms. A ctrl then meta binding matches. Pen uses the pointer names and can double-click. Two touch taps stay `touch.tap`. A touch of 24 px at 300 ms is `touch.swipe`, not `touch.tap`, and packs Pan x 34.

A second down before `takeFrame` opens a gesture. Two pan gestures in one frame leave one Pan, the later replacement, and both `pointer.down` events. A closed Pan and a later Zoom in one frame keep both commands. `set_pan` replacement makes the single later Pan the same camera result as applying both in order.

Fit, FlipHorizontal, FlipVertical, SetWindow, ApplyPreset, SetInvert, and SetColormap pack nothing on a 40 px move and on a click. The reason is `payload-not-in-gesture`. The click event is still emitted. A wheel with the scroll tool off is `no-owner`, and a later pan on an open gesture still uses the captured origin. An inactive second Scroll tool does not make a wheel ambiguous. Destroying the group is `unknown-group` and the events remain. Detaching the viewport is `missing-viewport`.

A non-finite move is dropped. A later y of 10 still scrolls. A non-finite up does not close the gesture. `ingest` does not throw on `pointerleave`, a down with no button, a NaN wheel, or a NaN point. A double click whose down is at 400 ms is a double click even when the up is at 900 ms. A second down exactly 3 px from the first up is not a double click. A second down at `(2, 2)` is. A third click after a double click is `pointer.click`.

Attach subscribes to `pointerdown`, `pointermove`, `pointerup`, `pointercancel`, and `wheel`. Detach removes all five. A queued pan is still returned. The next `takeFrame` is `empty`. Empty command lists are a zero-length buffer. Across the probe, `packed` is true exactly when `commands` is non-empty, and the bytes equal `packCommands` only then. The source does not name `submitCommands`, `writeCommands`, `packPointerFrame`, `alloc`, or `wasm`, and it does not call `preventDefault`. `index.ts` publishes `DISPATCH_REASONS` and `createPointerDispatch`. The erased-type comment names the pointer types. `DISPATCH_REASONS` has 12 distinct names and matches the plan.

On every probed frame the reason was `packed` exactly when a command was present. Tiers A, B, and C are not applicable. There is no pixel fixture and no DICOM arithmetic.

## Limitations and handoff

No browser, no wasm, no `bin/ocelli.sh`, and no GPU. Capture was a class method, not a DOM element. The packer check shows that the clamped steps are the bytes `packCommands` writes. It does not apply those bytes through `scroll_by`.

A lone button 3 down is `empty`, because the down is dropped and the queue has nothing to report. That is not filed. AC18 rejects `no-delta`, and `no-gesture` requires events. The button 3 frame that also moves is `no-gesture`. A lone `pointercancel` is `empty`. The plan asks for `no-gesture` when that frame has events, and cancel has no shell name. The permanent test expects `empty`. Not filed.

The returned basis object is kept by reference. Replacing `basis()` does not retarget the open gesture. Writing a field on the captured object would change a later sample. Not filed. The plan captures the object `basis()` returns, and `GestureBasis` fields are readonly at compile time.

`events.ts`, `commands.ts`, Rust, shaders, and F-072 were not reviewed beyond `packCommands` accepting the i32 steps and the independent `set_pan`, `set_zoom`, `set_rotation`, and `scroll_by` operations named by the plan.

This pass found no blocking defect and no blocking smell. It is not sprint completion. A later edit to `pointer.ts` needs a new independent pass. The product bytes were restored. The probe was deleted.
