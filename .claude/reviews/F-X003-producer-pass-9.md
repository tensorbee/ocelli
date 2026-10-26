# F-X003 signed camera and flip composition review, pass 9

**Reviewed**: staged tree `37fc701125277675fe9cd243ca1df5a4dff923e9`
**Previous**: `f9b56c9261373bcd4e560b5c320ab35338ad7095`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 1 defect, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Verdict and previous finding

The D10 signed-direction trigger now changes pixels correctly when explicit
flip metadata is false. The library suite passes with the new sign controls.
The original zoom and sign findings are addressed in those exercised paths.
A new composition defect remains: the producer treats the reference's observed
flip state as another command after interpreting the already-flipped camera.
This prevents acceptance of the producer as a complete geometry contract.

## D11, reference flip readback is applied twice

**Where**: `tools/oracle/src/produce.rs:1214`, the
`requested_horizontal ^ reverses_horizontal` composition and candidate sidecar
publication. The new `each_gaze_sign_applies_its_own_horizontal_basis` test
also asserts the incorrect composed result for a reference readback.

**Contract and primary source**: AC5 requires candidate and reference to
execute equivalent geometry. `tools/oracle/page/app.mjs:680` obtains camera
state from `viewport.getCamera()` and serializes position, focal point,
view-up and flip flags. These are observed camera state, not a fresh command
batch. Inspected the installed permitted cornerstone3D core version 5.8.2.
Its `dist/esm/RenderingEngine/Viewport.js`, `flip` at line 172, negates the
view-plane normal and changes camera position when horizontally flipping,
then toggles its flipHorizontal state. `getCameraNoRotation` reports the
resulting VTK camera plus that flag. The reflection is already in the pose.

**Synthetic trigger**: Start from the asymmetric source `[0,40,0,40]`, identity
plane, 8 by 8 canvas and focal point `[0.5,0.5,0]`. A centred negative-Z camera
with view-up `[0,-1,0]` and flipHorizontal false displays black on the left
and white on the right. The reference horizontal flip changes its camera to
positive Z with the same view-up and flipHorizontal true. That observed pose
already places white on the left and black on the right.

**Actual**: The producer sees positive-normal gaze, derives an internal
horizontal reversal, then XORs it with the true readback flag. The two cancel.
It displays black on the left and white on the right, returning the original
unflipped frame. It also publishes the cancelled internal flag as false in
the reference camera field. The new test expects this cancellation, so its
success does not validate the actual reference readback contract.

**Independent evidence**: An isolated test uses the existing asymmetric
fixture and a sidecar matching the reference flip result. It requires pixel 0
to be 255 and pixel 6 to be 0. Pixel 0 is actually 0. The test exited 101,
one failed and 101 filtered. The camera-basis control from pass 8, using the
installed VTK Camera, independently establishes the horizontal sign reversal.
No new browser-rendered reference frame is claimed in this pass.

**Expected and repair**: Consume the complete observed camera basis once.
Do not interpret its descriptive flip flag as an additional reflection.
Preserve the meaning of reference camera fields in candidate metadata. If
recording an internal View flip coefficient, distinguish it from the observed
camera's flip state. Derive the mapping from the pinned reference producer,
not from the name of a field or the current fixture's expected values.

**Smallest meaningful regressions**: Construct before/after sidecars from the
pinned reference's real flip operation or independently verified equivalent
readback. Exercise the producer and assert asymmetric pixel order and metadata
semantics. Include horizontal flip, flip back, both camera signs and any
supported combined-flip route. Unsupported vertical/rotated poses may still
be explicitly refused. Do not make tests issue an extra command that the
real sidecar does not contain. Retain zoom and refused-pose controls.

**Scope**: Present in the supplied index. The producer is absent at the
reported HEAD. The implementing report says current reference stack poses
are negative-normal, and the standing corpus frames were unchanged. This
finding does not attribute the existing 67 failed comparisons to the flip
case or reopen the closed monochrome publication repairs.

## Checks executed now

`bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator`
exited 0, 100 passed and one filtered. Log:
`/private/tmp/ocelli-pass9-library.log`.

Archived the pinned source into
`/private/tmp/ocelli-pass9-5p21cne4/source` and appended the synthetic test
`independent_pass9_reference_flip_readback_is_not_applied_twice` inside the
isolated produce.rs test module. Used a distinct build output:

```bash
cd /private/tmp/ocelli-pass9-5p21cne4/source
CARGO_TARGET_DIR=/private/tmp/ocelli-pass9-probe-target cargo test -p ocelli-oracle --lib independent_pass9 -- --nocapture
```

Exit 101. Log: `/private/tmp/ocelli-pass9-probe.log`. Only synthetic inputs
were used. Original product source and staged index were preserved.

Did not regenerate corpus output, rerun the candidate gate, open an adapter,
rebuild wasm or measure host performance. Zero passing corpus rows, 67 failed
and 17 unmeasured remain the standing reported verdict, not a run executed
by this reviewer. No new verification receipt is supplied.

## Instruction refinements and next handoff

Refined the existing design paragraph to classify consumed fields as commands,
observed state or internal coefficients. Added a focused dicom-tooling
paragraph requiring reference field provenance before composition. This
addresses D11's wrong input interpretation without adding another workflow
mechanism. Regenerated adapters. These additions remain unstaged and need
independent review. Prior instruction approvals do not cover the new wording.
Record the trigger in the next actual AS_BUILT entry.

The implementing agent should repair D11 first, replace the incorrectly
interpreted composition test with reference-readback regressions, and update
the feature design's camera/flip account. Preserve semantic RED and repaired
GREEN. Independently review the new design and tooling paragraphs. Return
the staged tree and exact focused results for a fresh independent pass.

After the camera contract is reliable, reduce residual corpus disagreements
through both candidate and reference before attributing arithmetic, sampling
or background defects. Nearby passing local fixtures alone do not explain
the recorded disagreement. Keep AC5, missing interaction instruments,
collector work and the unavailable representative host separate.

All unfinished feature dispositions remain unchanged. No whole-sprint review,
completion, commit, push, sprint close or new sprint is authorized. This
review is left unstaged and the pinned product index is unchanged.
