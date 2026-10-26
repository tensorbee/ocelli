# F-048 coordinate review, pass 1

**Reviewed**: staged tree `7de677af36b9122e60d2da4700c96b730f422e0a`
**Base**: `6b562be784f4365e7781869b5c511e9e0a1162a8`
**Author / reviewer**: xAI model per operator handoff / Codex
**Result**: 1 defect, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Defects

### D1, GPU consumer has no device-to-CSS adaptation

**Where**: `crates/ocelli-render/src/graph.rs:480`,
`crates/ocelli-viewport/src/view.rs:242`,
`crates/ocelli-render/tests/view.rs:413`.

**Contract**: F-048 AC4 and the approved CSS convention require one DPR
conversion through the affected consumer. HLD section 16 defines Canvas as
CSS pixels. The new typed maps use that convention correctly.

**Trigger and observation**: For an identity CSS view at DPR 2, a device
fragment at x=1.5 is CSS x=0.75 and should sample source texel 0. The shader
multiplies the raw fragment position by `camera_matrix`, which expects CSS.
It therefore maps to image x=1.5 and samples texel 1. Neither the graph nor
the canvas binding supplies the missing scale. The added test divides its
inputs manually and never exercises that shader-side composition.

**Evidence**: The isolated `review_dpr_two_shader_arithmetic_needs_a_device_to_css_adapter`
case reproduces the existing shader multiplication with the actual camera.
It exits 101 with `shader input maps to 1.5, expected .75 CSS`.
The first native consumer attempt exited 101 because the sandbox could not
open an adapter. A subsequent attempt outside the sandbox opened tier A and
failed its pixel assertion. The first two fragments were grey 128 and 255,
where DPR 2 requires 128 and 128. Thus the actual GPU path also reproduces
the mismatch. The earlier environment failure remains separate evidence.

**Scope and regression**: This integrates an unchanged GPU consumer with the
new CSS mapping contract. The underlying shader behavior predates this delta.
Add one concrete owner for device-to-CSS adaptation while preserving the
public CSS maps, and test its actual uniform or shader path at DPR 1, 1.25
and 2. Include DPR 2 fragments from both halves of one source texel and a
negative-coordinate border. Retain actual GPU/browser observations as open
until exercised. Do not change patient-space half-voxel conventions.

## Evidence and verified clean

`bin/ocelli.sh test ocelli-viewport` exited 0 with 47 tests, 0 ignored and
0 doc tests. All six committed coordinate fixtures passed. The independent
oblique fixture uses a manually computed canvas point `(11,1)`, without
calling `image_to_canvas` for its expected position. It maps to patient
`(10+s,20+s,36)`, where `s=1/sqrt(2)`, and passes at `1e-6` mm.
This covers anisotropic spacing, a quarter turn, horizontal flip, zoom, pan
and measured shift. Typed transforms and singular-map refusals were inspected.

An earlier independent fixture transposed its grid dimensions and expected
canvas `(12.5,1)`. It failed with world z=35.25. The fixture was corrected
after checking that `ImageDimensions::new(8,9)` means 8 rows and 9 columns.
That failure is not a product finding and its log is retained separately.

The tests ran natively in dev configuration. The isolated export is
`/private/tmp/ocelli-s16-review-b1q22i4y`. Reproduce with:

```bash
bin/ocelli.sh test ocelli-viewport --test coordinates review_
bin/ocelli.sh test ocelli-render --test view review_dpr_two_shader_arithmetic_needs_a_device_to_css_adapter -- --exact
```

Run those commands inside the isolated export containing the appended probes.
Logs are `/private/tmp/ocelli-s16-geometry-control-corrected.log`,
`/private/tmp/ocelli-s16-dpr-arithmetic.log`,
`/private/tmp/ocelli-s16-gpu-dpr-probe.log` and
`/private/tmp/ocelli-s16-gpu-dpr-probe-unsandboxed.log`. The latter contains
the actual tier A pixel mismatch. The earlier reviewer-fixture failure
is `/private/tmp/ocelli-s16-geometry-control.log`.

## Smells

None recorded.

## Nitpicks

None recorded.

## Limitations and handoff

No browser DPR run, tier B run or oracle comparison was performed. The native
tier A observation covers this synthetic camera case only. Repair D1
and obtain a fresh independent pass. The reviewed index was preserved.
Codex's new instruction refinements are unstaged and require another reviewer.
This is not whole-sprint approval, feature completion or a verify receipt.
