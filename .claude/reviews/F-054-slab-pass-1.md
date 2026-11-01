# F-054 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s20`, base `96e1bbe4bc61b074f2a9cb4fac61f0c8c071ad92`. Nothing is staged. The diff is the F-054 volume sampler and the slab render pass.
**Author / reviewer**: implementation by grok in the parent session. This pass is a separate reviewer subagent (`01a19000-3239-7ab0-ac38-f8ed725e59ba`) that did not write the diff. The subagent returned the findings in its final message and did not write this file. The parent transcribed that report. Independence is limited to that split.
**Result**: 1 defect, 1 smell, 2 nitpicks

## Acceptance challenged

AC10 and the GPU blend readback. The reviewer held the column stored 0, 200, 0 and the GPU column stored 0, 10, 200 against LINEAR_EXACT centre 127.5 width 255. On that window an interior stored value maps to itself, so reduce-then-VOI and VOI-then-reduce agree. AC1 to AC3. Every fixture volume uses `DMat3::IDENTITY`, so a step built from a diagonal of spacing matches the checked-in expectations.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Reading of the host reduction test, the GPU blend readback, `slab.wgsl` and `step_along` | Working tree against `96e1bbe` | Reviewer pass. The returned report has no command log | Findings D1 and S1 below. The parent did not invent an exit code |
| wgpu 30.0.1 texture-view ownership | `wgpu-core` 30.0.1 resource tracking | Reviewer pass | Dropping the Rust texture after `create_bind_group` is not a smell. The view keeps an `Arc` of the texture |

## Defects

### D1, The reduce-then-VOI tests pass if each sample is windowed first

**Where**: `crates/ocelli-volume/tests/slab.rs` `stored_reduction_then_one_voi_matches_the_fixture`, and `crates/ocelli-render/tests/slab.rs` `slab_blend_readback_follows_one_voi_after_the_stored_reduction`
**What**: The host test calls `window_once` on the constants 200, 0 and 100. The column is unused. The GPU column is stored 0, 10 and 200. The mean of the stored values is 70 and the mean of the windowed values is also 70. Composite, MIP, MinIP and average stay green if `lut_chain` runs on each sample and the results are then reduced. The shader does call `lut_chain` once after the reduction. Nothing fails if that order is reversed.
**Why it is wrong**: PS3.3 C.11.2.1.3.2 is not linear outside the window. The approved order is one window after the stored reduction. A test that passes for the other order does not lock AC10.
**Evidence**: The reviewer's reading of the two tests and of `slab.wgsl`. No mutation was recorded in the returned report.
**Scope and regression**: The host receipt and the GPU blend readback. Present on the working tree under review. The smallest regression is a column with a sample outside the window.

## Smells

### S1, The step formula's direction term is invisible to the tests

**Where**: `crates/ocelli-volume/src/slab.rs` `step_along`, and `crates/ocelli-volume/tests/slab.rs` `line_volume`
**What**: `M` column `i` is `spacing[i] * direction.col(i)`. Every fixture sets an identity direction, including the oblique case. Replacing that product with a diagonal of spacing still returns the checked-in steps. On a rotated orientation the index step walks the wrong axis and no current test goes red.
**Why it is a smell**: The formula is correct on the identity fixtures and will be wrong for a real `ImageOrientationPatient` that is not identity. No test notices.

## Nitpicks

### N1, `Pass::kind` still says there are three kinds

**Where**: `crates/ocelli-render/src/graph.rs` the `Pass` docs and `kind`
**What**: The enum comment says the pass enum is section 22 transcribed exactly, and `kind` says it returns one of three kinds. `Pass::Slab` is a fourth kind.

### N2, `COMPILED_BLEND` still says composite is the only body

**Where**: `crates/ocelli-render/src/graph.rs` `COMPILED_BLEND` and `pipeline_family_compiled`
**What**: The constant comment says composite is the only blend with a fragment body. Slab MIP, MinIP and average now have bodies.

## Verified clean

Dropping the slab textures after `create_bind_group` is not a smell. In wgpu 30.0.1 the bind group keeps an `Arc` of the core texture view, and that view keeps an `Arc` of the texture. The dropped Rust handles are not the last owners.

The reviewer also checked the step formula text, absence, centre-only composite, one `lut_chain` after the stored reduction, four pipelines, volume and stack MIP left uncompiled, tier C before the image, extent not truncated, `Load`, one submit, and `pub(crate)` on the shared helpers. Those checks did not become further findings.

## Limitations and handoff

The parent transcribed this report. It did not re-run the reviewer's reads as a second opinion. AC8 and AC9 were not re-opened by this pass. Remediation follows, then a fresh independent pass.
