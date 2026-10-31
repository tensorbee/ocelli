# F-054 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s20`, base `96e1bbe4bc61b074f2a9cb4fac61f0c8c071ad92`. Nothing is staged. The product diff is the F-054 slab sampler and render pass. This file is the only path this pass adds.
**Author / reviewer**: Implementation and the pass-1 remediation were done in the parent session. This pass is a separate reviewer subagent that did not write that diff. The reviewer mutated two sites, observed the failures below, and restored both before writing this file.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC10, the reduce-then-VOI order. Stored 0, 30 and 300 under LINEAR_EXACT centre 127.5 and width 255, PS3.3 C.11.2.1.3.2. The stored mean is 110 and one window leaves 110. Windowing each sample first yields 0, 30 and 255, and their mean is 95. The midrange of the same triple is 150. Those three display codes differ, so a test can reject both the other order and a min-plus-max average.

The host test reduces the walker column 0, 200, 0 by max, min and `(min + max) / 2`. That pair average is 100. The arithmetic mean of the three samples is 200/3, not 100. AC10's sentence is the pair "stored 0 and 200". The shader mean is not that pair formula. It is locked by the triple above, where 150, 110 and 95 cannot be confused.

S1, the direction term in `step_along`. Spacing `[2, 4, 8]` and direction columns `(0, 1, 0)`, `(0, 0, 1)`, `(1, 0, 0)`. Patient Z follows the second column, so the step is 4 mm and the index step is `(0, 1, 0)`. A diagonal of spacing steps 8 mm on `(0, 0, 1)`. World of index `(0, 0, 1)` is `(18, 20, 30)`.

N1 and N2. `Pass::kind` calls the slab the fourth kind. `pipeline_compiled`, `pipeline_family_compiled` and `COMPILED_BLEND` name composite, MIP, MinIP and average as the slab bodies.

AC8 and AC9 were run on this machine, not left as prior receipts.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test slab` | Working tree, restored direction product | Executed now | Exit 0. 8 passed |
| Same command after `step_along` used a diagonal of spacing | Temporary edit, then restored | Executed now | Exit 101. Only `rotated_direction_is_part_of_the_step` failed, bits `4620693217682128896` against `4616189618054758400` (8.0 against 4.0). The other seven volume slab tests stayed green |
| `bin/ocelli.sh test ocelli-volume --test slab -- rotated_direction_is_part_of_the_step` | Restored `direction.col(i) * spacing[i]` | Executed now | Exit 0 |
| `bin/ocelli.sh test ocelli-render --test slab` | Working tree | Executed now | Exit 0. 4 passed, 3 ignored |
| `slab_blend_readback_follows_one_voi_after_the_stored_reduction` | Restored shader, one `lut_chain` after `reduced` | Executed now | Exit 0. The tier C early return was not taken |
| Same GPU test after `lut_chain` was applied to each average sample | Temporary edit of `slab.wgsl`, outer `lut_chain` left in place. A second window does not move 95 | Executed now | Exit 101. `Average red 0.372549 is not within 0.0001 of 0.43137255` (95/255 against 110/255). Composite, MIP and MinIP had already been drawn in that test |
| Same GPU test after restoring the average sum | Restored `textureLoad` without `lut_chain` inside the loop | Executed now | Exit 0 |
| `one_submit_for_a_slab_and_a_stack` and `an_absent_slab_pixel_is_not_painted_zero` | Ignored GPU tests | Executed now | Exit 0. Both passed |
| `git status --short` and `git diff` for `tools/oracle/volume-params.json` and `raycast.wgsl` | Working tree | Executed now | Nothing staged. Those two paths are not in the diff |
| wgpu-core 30.0.1 `TextureView` and `BindGroup` | `resource.rs` `parent: Arc<Texture>`. `device/resource.rs` `used.views.insert_single(view.clone())` | Executed now | The dropped Rust texture is not the last owner |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1 D1 is closed. The restored average loop sums stored samples, and `fs_main` calls `lut_chain` once on that result. The GPU readback of stored 0, 30, 300 expects composite `30/255`, MIP `1.0`, MinIP `0` and average `110/255`, and it requires the average pixel to sit more than `0.01` away from `95/255`. The absolute gap `1.0e-4` appears only in that test assertion. It is not a product tolerance. Windowing each average sample made that assertion fail with the two colours above. The shader was restored and the test passed again.

Pass 1 S1 is closed. The matrix columns are `direction.col(i) * spacing[i]`. Replacing that product with a diagonal of spacing turned the rotated step from 4 mm into 8 mm and failed only that test. The product was restored and the test passed again.

Pass 1 N1 and N2 are closed. The pass docs and `COMPILED_BLEND` name four slab kinds and the four slab bodies. Volume MIP and stack MIP stay uncompiled. Labelmap edge has no blend code and `present_slab` does not call the upload.

`reslice_oblique` is absent from the `ortho.rs` diff. That diff is `pub(crate)` on `in_range`, `sample_at` and `u32_from_nonneg_floor`, plus a doc comment on `sample_at`. No `unsafe`. No `wasm_bindgen`. No new trait. `tools/oracle/volume-params.json` and `raycast.wgsl` are unchanged.

Composite uses `depth / 2`, which is the centre slot for the odd count `slab_samples` builds. Absence in any slot clears the column, the mask texel stays 0, and the value texture is not filled with 0 for that pixel. The fragment discards on mask 0 before `reduced`. The slab load op is `Load`. The stack load op stays clear. Tier C returns before the upload closure. A dimension of 0 or a dimension above `max_tex_3d` returns before the upload. One tick of a dirty slab plus a dirty stack increased `submissions` by 1. An absent slab pixel left the previous stack colour in place.

Dropping the slab textures after `create_bind_group` is still not a smell. In wgpu-core 30.0.1 the view holds an `Arc` of the texture, and the bind group keeps a clone of the view.

The host pair average of 0 and 200 is 100, and the shader mean of 0, 200 and 0 would be 200/3. That is not a second operator in the product. The GPU test rejects a min-plus-max average of 0, 30 and 300, which would be 150, and it rejects window-then-mean, which is 95.

## Limitations and handoff

Clippy, wasm, the oracle and `bin/ocelli.sh gate` were not run. Device-loss recovery was read in `install_recovered` and was not executed. The GPU draws were 1 by 1. A coronal `slab_plane` was compared with `voxel_of` and was not given its own test. The LLD files named in the plan are not in this diff, so this pass does not review them.

The product tree matches the tree that passed the restored tests above. No mutation remains.
