# F-X003 camera repair review, pass 8

**Reviewed**: staged tree `f9b56c9261373bcd4e560b5c320ab35338ad7095`
**Previous**: `c21279c6aa1e5028b10297db4311978cb3b6aed1`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 1 defect, 0 smells, 0 nitpicks in the camera repair scope
**Date**: 2026-10-19

## Verdict

The original D10 zoom trigger is repaired and the new permanent camera tests
pass. The broader applied-pose contract remains incomplete. The candidate
accepts cameras on both sides of the image plane without changing the
projection. AC5 is not ready for acceptance. The reported red corpus result
remains unresolved and is not attributed to this one defect.

## D10 follow-up, signed camera direction is accepted as equivalent

**Where**: `tools/oracle/src/produce.rs:1258`, `:1266`, `:1271`,
`executed_camera` and `candidate_sidecar`.

**Contract**: F-X003 AC5 and the D10 repair require published geometry to
describe the candidate's actual projection. Camera position, focal point and
view-up jointly determine image orientation. Collinearity with the plane
normal alone does not establish equivalent orientation.

**Mechanism**: `accept_pose` checks `along.abs()` and lateral distance, so
both signs of the normal-axis displacement pass. It checks the same view-up
for both. The View applies scale and explicit flips, but no change for the
sign of that displacement. The sidecar retains the requested position. Thus
opposite horizontal projections can be published as the same rasterized image.

**Synthetic trigger**: Use the new `centred(1.0,4.0,false)` fixture: 2 by 2
source `[0,40,0,40]`, 8 by 8 canvas, identity image orientation, focal point
`[0.5,0.5,0]`, view-up `[0,-1,0]`. Render with positions `[0.5,0.5,10]`
and `[0.5,0.5,-10]`, keeping explicit flips false.

**Actual**: Both render successfully and produce byte-identical nonconstant
frames. The second sidecar records the negative-Z position. The probe exits
101 at the assertion that the second frame must change or be refused.

**Independent orientation control**: Executed the installed permitted
`@kitware/vtk.js/Rendering/Core/Camera.js` used by the reference dependency
tree. For the same positions, focal point and view-up, `getViewMatrix()`
changes its horizontal coefficient from -1 to +1. Its view-plane normal
changes from positive Z to negative Z. This agrees with the hand-derived
camera basis: reversing the gaze while retaining view-up reverses the
horizontal basis. Source uses `mat4.lookAt` and transpose in `getViewMatrix`.
Log: `/private/tmp/ocelli-pass8-camera.log`. This is a camera-matrix control,
not a fresh browser render or corpus observation.

The permitted synthetic `synthetic__ct_unsigned_16` sidecar uses a negative-Z
camera position with identity orientation and view-up `[0,-1,0]`. Do not fix
this by arbitrarily rejecting that sign because an existing new unit fixture
used positive Z. Derive the supported pose from the executed mapping and
reference camera contract. Other reference rows were not inspected for this
observation.

**Expected and regression**: Either execute each supported signed camera
basis correctly, or explicitly refuse the sign that the fixed View mapping
does not represent. Test both sides with an asymmetric nonconstant frame and
independently derived left/right expectations. Compare declared geometry to
the applied basis, not only to the requested values. Retain zoom, flip,
off-centre and rotated-view-up tests. Do not change tolerances or exclude
failing corpus rows to obtain a green result.

**Scope**: Confirmed on the supplied staged tree. The producer is absent at
the reported HEAD. This continues D10's applied-state finding, rather than
claiming that the original zoom repair failed. It does not reopen D8 or D9.

## Checks executed now

`bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator`
exited 0, 99 passed and one filtered. Log:
`/private/tmp/ocelli-pass8-library.log`. This includes the permanent zoom,
flip and refusal cases plus the new producer arithmetic controls.

Archived the pinned tree into
`/private/tmp/ocelli-pass8-il2t0vbs/source` and added only the independent
opposite-side test inside its produce.rs test module. The original source
and index were preserved. The probe uses its own Cargo target directory:

```bash
cd /private/tmp/ocelli-pass8-il2t0vbs/source
CARGO_TARGET_DIR=/private/tmp/ocelli-pass8-probe-target cargo test -p ocelli-oracle --lib independent_pass8 -- --nocapture
```

Exit 101, one failed and 100 filtered. Build finished in 13.71 seconds.
Log: `/private/tmp/ocelli-pass8-probe.log`. All probe inputs and pixels are
synthetic. The reference camera matrix control exited 0 separately.

No corpus producer, candidate gate, GPU/browser rendering, new wasm size or
representative-host measurement was run in this review. The 84 compared,
67 failed, 17 unmeasured and zero passing corpus verdicts remain implementing
agent evidence from `S16-repair-evidence-pass-7.md`.

## Instruction disposition and next assignment

Existing design and microscope instructions already require a material camera
change to affect the nonconstant image correctly or be refused. Dicom-tooling
already requires applied-state metadata. This finding is a missed signed-basis
case in executing those rules. No additional general instruction paragraph
was added. Instruction review pass 5 retains its stated scope. Put the
signed-direction cases in the concrete feature design and permanent tests.

The implementing agent should complete D10's pose contract first, using the
independent camera basis above. Update the feature design's supported pose
description and preserve the distinction between requested and executed state.
Return permanent old-source RED and repaired-source GREEN, plus the full
focused library run, for a fresh independent review.

For the remaining corpus signatures, current small fixtures establish useful
local contracts but do not reproduce or explain the actual one-code interior,
letterbox boundary, nonsquare edge or sigmoid-shoulder discrepancy. Preserve
those as unattributed. Next diagnosis needs a minimal case that retains the
observed disagreement across both candidate and reference, followed by layer
attribution. Merely passing a nearby standard-value fixture is not causal
reduction of the disagreement. Existing skill guidance already calls for
independent representatives and failure attribution.

Keep AC5, interaction instruments, collector work and unavailable host
measurements separate. All current stories retain their existing unfinished
dispositions. No verification receipt, independent whole-sprint approval,
commit, push, sprint close or new sprint is supplied. This new review remains
unstaged and the reviewed index is unchanged.
