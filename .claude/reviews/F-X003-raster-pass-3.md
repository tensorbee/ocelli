# F-X003 raster repair review, pass 3

**Reviewed**: staged tree `f6057868244a71679ebfd95a17e440084699aa76`
**Previous**: `8d0a94a34d4b35cd38e26c5056311eab05937231`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 1 defect, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Previous findings and verified clean

Pass 2 D5 to D7 are repaired in the exercised path. The module worker uses
the real OffscreenCanvasRenderingContext2D. Publication retains ImageData
and the pixel view, borrows the session frame, and accepts settlement only
after successful putImageData. Failure remains retryable and emits no event.
The supplied worker test uses no context-class alias.

Rebuilt the dev module, then executed `node packages/core/cpu-canvas.mjs`
outside the sandbox. It exited 0. The first frame was opaque white with
kind 6 generation 1, idle preserved it without an event, inversion published
generation 2, a failed blit retained refining, retry published generation 3,
and the newer command published generation 5. ImageData constructions stayed
flat across those publishes and rose once on a changed-size resize.
This proves the observed construction counts, not every possible allocation.
Log: `/private/tmp/ocelli-pass3-worker.log`.

`bin/ocelli.sh test ocelli-render --test cpu` exited 0 with 13 cases.
Native wasm tests exited 0 with 25 cases. The explicit wasm suite passed 5
cases and default vitest passed 103 cases. No new release-size measurement,
GPU observation, corpus verdict or representative-host sample is claimed.

## Defects

### D8, resize clears the external bitmap without invalidating settlement

**Where**: `crates/ocelli-wasm/src/canvas.rs:137`, `:651`, `:699`,
`crates/ocelli-wasm/src/session.rs:469`,
`crates/ocelli-render/src/cpu.rs:332`.

**Contract**: F-X003's failed-resize preservation and settled/idle contract
apply through the public browser binding. AC4 requires current visible
content and AC7 permits idle only when the visible frame is unchanged.

**Trigger 1**: Bind 1 by 1 at DPR 1, commit f32 sample 40 and publish opaque
white. Call resizeCpuCanvas(1,1,1), then publishCpuCanvas again.
**Actual**: sized writes canvas.width and height even though unchanged, which
clears the bitmap. The native raster sees no size/scale change and stays
settled. The next publish returns success with no draw or event. The pixel
is `[0,0,0,0]` and refining is clear, so the blank result persists.

**Trigger 2**: In a separate worker start from the same visible white frame,
then call resizeCpuCanvas(0,1,1).
**Actual**: The raster refuses the empty extent, but sized has already set
canvas.width to 0 and destroyed the displayed frame. Publication afterward
returns success without recovery. The cached state is still settled.

**Expected**: Validate the complete resize before destructive external
changes. A refused resize preserves canvas extent, visible frame and usable
publication state. For equal arguments, either preserve the bitmap as a
true no-op or explicitly invalidate and republish it. Never leave a cleared
bitmap classified as current settled content. Preserve retained storage
where the chosen no-op policy requires it.

**Evidence**: `node /private/tmp/ocelli-pass3-resize.mjs` launches unmodified
built wasm in real Chromium module workers. It exits 1 with two failed
preservation checks. Both independent workers first verify the opaque white
positive control. Complete probe and output are
`/private/tmp/ocelli-pass3-resize.mjs` and
`/private/tmp/ocelli-pass3-resize.log`. This test uses the same headless CPU
browser route as the earlier review and no class alias.

**Smallest regressions and repair boundary**: Add a same-configuration resize
after settled publication and a rejected zero extent after settled publication
to the permanent worker suite. Assert actual RGBA including alpha, canvas
extent, refining, generation and event count. Retain the existing valid
changed-extent, retry and storage-count controls. Correct ordering and bitmap
invalidation in the public binding, rather than manually invalidating from
the test or weakening the pixel assertion.

## Smells

None separately recorded.

## Nitpicks

None recorded.

## Limitations and handoff

The current reviewed product tree was preserved. No product fix was applied.
F-048/F-101's reported repair findings are closed within their scoped pass-3
reports, while the stories remain in progress for their remaining acceptance.
AC5, AC6, browser colour/reduced-resolution integration and the interaction
instruments remain open as declared. The representative host is still absent.
No full sprint gate, completion or independent whole-sprint approval is supplied.
