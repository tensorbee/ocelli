# F-048 coordinate review, pass 2

**Reviewed**: staged tree `8d0a94a34d4b35cd38e26c5056311eab05937231`
**Previous**: `7de677af36b9122e60d2da4700c96b730f422e0a`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 1 defect, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Previous finding

Pass 1 D1 is repaired in the inspected GPU path. The camera buffer carries
the CSS matrix and DPR. The shader divides device fragment positions before
the CSS multiply. Rebind retains both. No new patient-space offset is added.

## Defects

### D2, LLD still specifies the former GPU camera layout

**Where**: `docs/lld/render-graph.md:52` and `docs/lld/viewport.md:89`.
**Contract**: WORKFLOW's living architecture rule requires current behavior,
and F-048 declares both affected areas in its LLD impact.
**Actual**: Both documents say group 0 binding 2 is a 64-byte uniform holding
sixteen floats. The repaired DeviceCamera is 80 bytes: sixteen matrix floats
and four scale/padding lanes. `set_view` also has a new DPR argument.
**Expected**: Document the real layout and its owner, including registration,
updates and recovery. Keep the distinct 96-byte ViewportState contract and
its 64-byte camera field unchanged. Do not expand state readback to match the
GPU-only uniform.
**Impact and regression**: The stale layout omits the scale that repairs D1
and can misdirect later buffer writes. Compare the LLD with DeviceCamera,
device_camera_words and the existing uniform-word test before completion.

## Evidence and verified clean

Executed now:

```bash
bin/ocelli.sh test ocelli-render -- --ignored --test-threads=1 --nocapture css
```

Outside the sandbox, both selected tests printed `auto opened tier A` and
passed. The DPR 2 frame has grey 128 for both halves of source texel 0.
DPR 1 and the translated black border controls passed. Log:
`/private/tmp/ocelli-pass2-gpu.log`. Native render tests also passed, including
the uniform-word check for DPR 1.25. That check is not a rendered 1.25 frame.
The source-level rebind and stored rebind paths both preserve DPR.

## Smells

None recorded.

## Nitpicks

None recorded.

## Limitations and handoff

No tier B or rendered DPR 1.25 observation is supplied. The separate browser
worker test concerns the CPU canvas, not GPU viewport parity. No corpus or
whole-sprint approval is claimed. Correct D2 and retain unperformed acceptance
observations explicitly. No product source or staged content was changed by
this pass.
