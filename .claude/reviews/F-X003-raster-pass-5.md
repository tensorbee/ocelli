# F-X003 colour and interaction review, pass 5

**Reviewed**: staged tree `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502`
**Previous**: `9ae40681bae54a69a21705d98a40a6337adce74d`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 1 defect, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Defects

### D9, a colour request changes remembered mode before input acceptance

**Where**: `crates/ocelli-wasm/src/session.rs:482`, `:495`, `:527`,
`crates/ocelli-wasm/src/canvas.rs:691`,
`crates/ocelli-render/src/cpu.rs:295`.

**Impact and scope**: Public mono/RGB publication routes share a remembered
shade. A mismatched request can silently succeed when settled, or explicitly
fail when invalidated, but both leave that shade incompatible with the current
source. Subsequent interaction fails until the caller explicitly reselects the
correct route. The visible bitmap stays old while refining remains set. This
is a reproduced state-preservation defect, not a claim that ordinary RGB
publication is wrong. It exists in the supplied index. These new publication
routes are not present at the reported HEAD.

**Contract**: F-X003's approved design requires distinct resolved colour input,
current visible content, and failure/recovery through the owning worker.
`RasterError::Length` explicitly covers samples that do not match the colour
layout. Its refusal must not replace the last usable drawing mode. An idle
result cannot validate incompatible input. This is a lifecycle contract, not
a new DICOM arithmetic rule or tolerance.

**Reproducer**: In a fresh real Chromium module worker, bind a 1 by 1 canvas
at DPR 1, allocate four bytes, write little-endian f32 stored value 40, and
commit it with viewport 1, columns 1, rows 1 and generation 1. Publish via
`publishCpuCanvas` and verify opaque white. Then run these two independent
cases with fresh workers:

1. Call `publishCpuColor(1)` directly, then `publishCpuInteraction`.
2. Apply SetInvert(true) through the allocation ticket and numeric command
   export, call `publishCpuColor(1)`, then `publishCpuInteraction`.

**Actual**: Case 1's colour call returns success, no event and unchanged white.
The settled shortcut returns before RGB sample-length validation. It has
already selected RGB. The next interaction fails with
`cpu raster source length does not match the grid`, keeps white, sets refining
and emits no event. In case 2 the colour call itself returns that length error,
but the next interaction returns the same error because RGB remains selected.
The valid monochrome source and prior mode have not been restored.

**Expected**: Reject the incompatible RGB request consistently before and
after settlement. Preserve the last usable mode and frame state on rejection.
The following valid monochrome interaction must draw and remain recoverable.
For case 2 it must apply the accepted invert command, rather than leaving the
previous white bitmap because of the rejected colour selection.

**Mechanism**: `select_supported_color` changes shade before drawing or
validating the committed payload. `draw_committed_mono` has the symmetric
early assignment. `raster_color_into` can return idle before `prepare_color`
checks source length. Handling only unsupported photometric codes does not
cover a supported code with an incompatible payload.

**Evidence**: `node /private/tmp/ocelli-pass5-colour.mjs` exited 1 with two
failed controls against freshly rebuilt, unmodified wasm. Output is in
`/private/tmp/ocelli-pass5-colour.log`. It also runs the successful independent
odd-size RGB control below. All inputs are synthetic.

**Smallest meaningful repair and regressions**: Validate the requested
source/layout before an idle shortcut and commit remembered mode only at the
defined successful operation boundary, or restore it on refusal. Cover both
mono-to-RGB and RGB-to-mono incompatible requests. Cover settled and invalidated
starting states, then a valid interaction and full settle without manually
resetting mode from the test. Assert actual RGBA, refining, generation and
events. Keep unsupported-code, failed-blit, resize and ordinary colour tests.
Add the tests to the permanent worker suite and retain old-source RED and
repaired-source GREEN evidence. Do not weaken source-length checks.

## Verified clean and checks executed now

- Rebuilt dev wasm with `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm`: exit 0.
- `bin/ocelli.sh test ocelli-wasm --lib`: exit 0, 26 passed.
- `npx vitest run --config vitest.boundary.config.ts`: exit 0, 5 passed.
- `node packages/core/cpu-canvas.mjs`: exit 0 in real headless Chromium with GPU disabled.

Logs are `/private/tmp/ocelli-pass5-build.log`,
`/private/tmp/ocelli-pass5-native.log`, `/private/tmp/ocelli-pass5-boundary.log`
and `/private/tmp/ocelli-pass5-worker.log`.

The permanent worker confirms ordinary native RGB settle, reduced publication,
failed-blit retry and current-generation full settle, alongside previous mono,
resize and ImageData construction controls. The earlier resize finding stays
closed within its reviewed scope.

An independent identity-view fixture used a 7 by 5 source and device at DPR 1,
with RGB channels `i, 2*i, 3*i` for source index i. A 3 by 2 reduced grid samples
source columns 1, 3, 5 and rows 1, 3. Integer cell replication matched all 35
device pixels, and full settlement restored the original full-resolution
frame. Expected selection and channel bytes were derived independently from
the declared mapping and interleaved RGB layout, not copied from the raster.
This is an odd-extent and two-dimensional control beyond the supplied 2 by 1
case. It supplies no corpus or external parity claim.

## Guidance changes and handoff

Added one focused paragraph each to canonical `design.md` and `microscope.md`:
identify cache-validity inputs, validate before idle shortcuts, and follow an
incompatible request with a valid operation to detect hidden mode changes.
Generated adapters were regenerated. These edits remain unstaged and require
fresh independent review. The earlier instruction pass does not approve these
new paragraphs. No additional workflow, verification or domain-skill rule was
needed. Record this trigger in the next actual AS_BUILT entry.

The implementing agent should repair D9 and return the staged tree with the
explicit dev-wasm rebuild, boundary suite and permanent worker results for a
fresh independent pass. Preserve the supplied index during this review and
keep unrelated instruction changes separate until explicit integration.

AC5's corpus candidate producer, the missing interaction instruments, and
AC6's missing collector and representative host remain separate open work.
F-048, F-101 and F-X003 remain in progress. No sprint gate, new release-size
measurement, GPU observation, completion or whole-sprint approval is supplied.
Nothing was committed, pushed or closed.
