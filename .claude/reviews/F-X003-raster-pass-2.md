# F-X003 raster review, pass 2

**Reviewed**: staged tree `8d0a94a34d4b35cd38e26c5056311eab05937231`
**Previous**: `7de677af36b9122e60d2da4700c96b730f422e0a`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 3 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Previous findings and current checks

The native Session now invalidates after successful commands and new sources.
Refused and empty batches retain the prior settlement. DPR changes invalidate
even when the rounded extent stays the same. Source-colour reservation is
separate from target size. Native drawing borrows its Presentation instead
of cloning LUT buffers. These repairs address pass 1's corresponding paths.

Native render tests passed 135 tests with 37 ignored, including 13 CPU cases.
Native wasm tests passed 23 cases. The new LUT clone counter observes the
specific native clone route only, not total render-path allocations.

## Defects

### D5, a worker OffscreenCanvas context is cast to the HTML canvas type

**Where**: `crates/ocelli-wasm/src/canvas.rs:19` and `:698`.
**Contract**: F-X003's CPU worker binding must draw through its owned
OffscreenCanvas before publishing a frame.
**Trigger**: In a real Chromium module worker create OffscreenCanvas(1,1),
bind it, commit one f32 stored sample 40 and call publishCpuCanvas.
**Actual**: Commit returns 1, but publish throws `cpu canvas context is not 2d`.
The actual context class is OffscreenCanvasRenderingContext2D. The code uses
CanvasRenderingContext2d. In that worker CanvasRenderingContext2D is undefined.
The canvas remains transparent and no event is emitted.
**Expected and regression**: Use the correct concrete worker context type
and test a real worker draw, observable pixel and kind-6 publication. Export
names alone do not prove canvas execution. The original browser probe uses
the unmodified rebuilt module and no class shim.

### D6, a failed blit consumes settlement and cannot be retried

**Where**: `crates/ocelli-wasm/src/canvas.rs:643`, `:672`, and the raster's
successful draw/settlement transition.
**Contract**: F-X003 AC4 requires visible current state. Failure must not
publish or mark an undelivered frame settled. A retry must remain possible.
**Trigger**: Draw the committed frame, fail the canvas blit, then call
publishCpuCanvas again with no state change.
**Actual**: The first failure already cleared refining and cached settlement.
The second call returns success without attempting a blit or emitting kind 6.
The unchanged transparent canvas is treated as an idle settled result.
**Independent control**: To isolate this from D5, the test-only worker aliases
the HTML context class to the real offscreen class, then makes the first
putImageData throw. The second call still makes no putImageData attempt.
This alias is only an experiment and is not a proposed product fix.
**Expected and regression**: Separate raster completion from accepted
presentation, or retain pending output/invalidation until publication succeeds.
After a transient failure, the next attempt must blit the current frame and
emit exactly one correct-generation event. A stale retry must not overwrite
newer state. Inspect the copied refining flag before and after success.

### D7, browser publication reintroduces per-frame allocation

**Where**: `crates/ocelli-wasm/src/canvas.rs:608`, `:682`, `:701` and `:703`.
**Contract**: HLD section 26 and F-X003 AC7 prohibit render-loop allocation
after setup/resize across the full publishing entry point.
**Actual source path**: prepare_committed copies the complete RGBA Vec using
to_vec for every drawn frame. blit_rgba allocates a fresh clamped array and
ImageData every time. Removing the native Presentation clone does not remove
these allocations. The LUT clone counter cannot observe them.
**Browser evidence**: In a test-only context-type control, two successful
publications separated by SetInvert constructed two Uint8ClampedArrays and
two ImageData objects. Counts rose from 1/1 to 2/2. Real offscreen blits and
kind-6 events occurred in this control. The untouched module still fails D5.
**Expected and regression**: Size and retain the publication storage and
context during binding/resize and avoid an owned full-frame copy per draw.
Test allocations around the whole browser publication path with ordinary and
LUT presentations, plus unchanged idle and resize controls. Keep the existing
native no-clone regression, but do not label it zero-allocation coverage.

## Browser reproduction and controls

Rebuilt the dev web artifact from the reviewed product source. The module
worker uses synthetic f32 sample 40 with the identity presentation and a
1 by 1 target. It serves only the local generated glue, wasm and test code.
Chromium was headless with `--disable-gpu`. This is a tier C canvas test,
not a representative-host performance run or GPU-rendering observation.

```bash
node /private/tmp/ocelli-pass2-browser.mjs
node /private/tmp/ocelli-pass2-browser-controls.mjs
```

Both experiment drivers exited 0 after collecting results. Expected-behavior
assertions for the reported original outcomes would fail. Logs and complete
probe sources are the corresponding paths ending in `.log` and `.mjs`.

The original module reports first error, then success without a retry, flags
0 both times, zero events and pixel `[0,0,0,0]`. With only the test class alias,
two successful publications produce one event each. With that same alias and
a controlled first putImageData failure, only one putImageData call occurs,
the second publication is skipped and the pixel remains transparent.

The initial sandbox browser launch failed with Mach-port permission denial.
The working experiments were rerun outside the sandbox. That environment
failure is not counted as a product defect.

## Verified clean and remaining acceptance

The three supplied built-wasm tests passed. Native invalidation, idle,
source-colour capacity and selected LUT paths passed their focused controls.
The synthetic candidate comparator is explicitly one frame and is not AC5.
No corpus or tier A/C verdict is inferred from it. Representative-host AC6
remains unavailable and no thresholds or baselines were changed.

The browser binding currently calls draw_committed_mono only. Supported
colour-to-canvas and interactive reduced-resolution integration still need
their own implementation/evidence disposition under the approved acceptance
inventory. The host-cost example exercises a workload and always refuses a
figure. It has no host-selection or CPU-time collection path yet, so an
unavailable host is not its only missing prerequisite.

## Smells

None separately recorded.

## Nitpicks

None recorded.

## Limitations and handoff

No original product source was changed during these experiments. The type
alias and injected failure live only in temporary worker test code. Keep the
story in progress, repair D5 to D7 and add permanent unmodified-worker tests.
This pass supplies no final verification receipt or whole-sprint approval.
