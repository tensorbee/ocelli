# F-059 review, pass 2

**Reviewed**: working tree on `sprint/s19` at `42336565115749daa4f20e8e04a18f1cb72729f2`. Unstaged product paths are `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/tests/graph.rs`, `crates/ocelli-render/tests/view.rs`, `crates/ocelli-volume/src/lib.rs`, plus untracked `crates/ocelli-render/shaders/raycast.wgsl`, `crates/ocelli-render/tests/raycast.rs` and `crates/ocelli-volume/tests/resident.rs`. This review file is not part of that product diff. Pass 1's review file was already untracked and was not edited.
**Author / reviewer**: the sprint implementation agent wrote the ray cast and the D1 remediation. The reviewer is an independent microscope subagent in a separate context and did not write pass 1's code or this remediation. This is not a human review.
**Result**: 1 defects, 0 smells, 0 nitpicks

The pass 1 defect is closed. A ready count of 2 with sampled limit 16 is accepted, both bricks are sampled, and a count that does not leave a residency binding is refused. This pass's defect is a false comment, not a wrong pixel.

## Acceptance challenged

AC1, AC2 and AC6 were re-run on the GPU after the remediation. The host `f32` integral was computed again for stored 0, 100 and 200 under LINEAR_EXACT at centre 127.5 and width 255. Display equals the stored value for all three. The two-brick colour `(100/255)*0.5 + 0.5*(200/255)*0.5` is bits `3ec8c8c9`. The shader's front-to-back order on stored 100 then stored 200 is the same bits, at 0 ULP. Alpha is `0.75`, bits `3f400000`.

That colour is also the integral of stored 200 then stored 0. Alpha 0.75 does not reject that pair. Two mutations do. Forcing every load through slot 0 produced red `0.2941177`, which is 1 ULP from the host integral of stored 100 then stored 100 and 3289650 ULP from the expected colour. Zeroing slot 0 and keeping later slots produced red `0.19607845`, which is 1 ULP from the host integral of a zero step then stored 200. The second slot is the 200 sample. Sampling only brick 0 cannot produce the expected colour. A skipped second step would also leave alpha `0.5`.

The same window under the included `voi_linear_exact` body is what the module still prefixes. The host test checks that `volume_ray_module(2)` starts with `VOI_WGSL` and contains `let alpha = 0.5`.

The flag mutation was repeated. `if (flag == 0u)` became `if (flag == 0xFFFFFFFFu)`. `an_absent_step_is_skipped` failed with red `0.5882354` against `0.39215687`. That red is 1 ULP above the host integral of two stored-200 samples. It is the texture's value at the absent step, not a skipped step. The shader was restored.

`volume_ray_admission(Tier::A, 2, 16)` is `Ok`. `volume_ray_admission(Tier::A, 16, 16)` and `volume_ray_admission(Tier::A, 1, 0)` are `SampledTextureLimit`. `volume_ray_admission(Tier::Cpu, 1, 16)` is `TierCUnavailable`. Those are the assertions in `volume_raycast_has_no_cpu_image`, and that test passed. A ready count above `brick_slots` returns before any brick view is created. It does not bind a prefix. That refusal branch was not executed with a live set larger than the compiled slot count.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --short` and `git rev-parse HEAD` | `sprint/s19` working tree | Executed at the start and after every shader restore | HEAD `42336565115749daa4f20e8e04a18f1cb72729f2`. Product paths are the five modified files and three untracked files listed above. |
| `bin/ocelli.sh test ocelli-render --test raycast` | host admission, tier C, module text | Executed now | Exit 0. 2 passed, 4 ignored. |
| `bin/ocelli.sh clippy ocelli-render` | `-p ocelli-render --all-targets -D warnings` | Executed now | Exit 0. |
| `bin/ocelli.sh test ocelli-render --test raycast -- --ignored --test-threads=1 --nocapture` | AC1, AC2, AC6, and `two_ready_bricks_both_contribute` | Executed now, before any mutation | Exit 0. 4 passed in 1.44s. No tier C line. Pixel assertions ran. |
| Flag mutation, then `an_absent_step_is_skipped` | condition `flag == 0xFFFFFFFFu` | Executed now | Exit 101. Panic at `tests/raycast.rs:305`. red `0.5882354` versus `0.39215687`. |
| Restore, SHA-256 | `raycast.wgsl` | Executed now | Before and after `50b3ec4adc1acd0c5d52120d843afbb2c03df413510e77a5341ab46e7d1b1694`. `if (flag == 0u)` is back. |
| `load_brick(0u)` mutation, then `two_ready_bricks_both_contribute` | both steps read slot 0 | Executed now | Exit 101. red `0.2941177` versus `0.39215687`. |
| Restore, SHA-256 | same hash | Executed now | Hash matches. `load_brick(u32(slot), local)` is back. |
| Zero slot 0, keep later slots, then the same test | `select(0.0, loaded, slot != 0)` | Executed now | Exit 101. red `0.19607845` versus `0.39215687`. |
| Restore, SHA-256 | same hash | Executed now | Hash matches. |
| Host `f32` program | LINEAR_EXACT 127.5 / 255, opacity 0.5 | Executed now | Stored 100 and stored 200 display as themselves. Two-brick colour matches the shader order at 0 ULP. Stored 100 then 100 is 3289650 ULP away. A skipped second step has alpha 0.5. |

An earlier zero-slot edit used `slot != 0u`. Naga rejected it because `slot` is `i32`. That run is not pixel evidence. The shader was restored before the `slot != 0` retry. The hash after that restore matched as well.

## Defects

### D1, comments still describe a one-brick bind and a refused short slice

**Severity**: Defect

**Where**: `GraphError::SampledTextureLimit` in `crates/ocelli-render/src/graph.rs:296-298`, and `VolumeRaySource` in `crates/ocelli-render/src/graph.rs:803-804`.

**Trigger**: Read those comments against `volume_ray_admission` at `graph.rs:713-726` and `bind_volume_ray` at `graph.rs:2177-2431`. The host test calls `volume_ray_admission(Tier::A, 2, 16)`. The GPU test binds two ready bricks of a `[1, 1, 129]` volume.

**Expected**: The comments match the bind. A ready set that leaves room for the residency texture is bound in full. A comment does not say the bind refuses a shorter slice unless the function has that check.

**Actual**: The error docs say "The ray binds one brick texture" and that a ready set larger than that one binding is refused. Admission of 2 bricks with limit 16 is `Ok`, and the two-brick draw passed. `VolumeRaySource` says a shorter slice is refused. `bind_volume_ray` takes no `Volume`. The slice length is the ready count. A one-element slice that fits the limit is bound. Nothing compares it with `Volume::ready_bricks`.

**What**: Two sentences describe refusals the function does not apply. The third sentence of the error docs, that a non-fitting set is not drawn as a subset, matches the code. The one-binding sentences contradict the code and the test that locks `volume_ray_admission(Tier::A, 2, 16)`.

**Why it is wrong**: The approved plan says a ready set of several bricks is bound up to the sampled-texture limit, and above that limit nothing is drawn. A false doc comment is a defect. Pass 1 recorded the one-binding comment as true for the tree it reviewed. The remediation changed the bind and left the comment. The short-slice sentence is new with `VolumeRaySource` and describes a guard that was not added.

**Evidence**: `volume_raycast_has_no_cpu_image` passed, including `volume_ray_admission(Tier::A, 2, 16)` as `Ok` and `volume_ray_admission(Tier::A, 16, 16)` as `SampledTextureLimit`. `two_ready_bricks_both_contribute` passed. The bind path was read. There is no completeness check against the volume.

**Scope and regression**: Present on this working tree. It does not change a pixel. Consumers are the next reader of the error and of `VolumeRaySource`. The smallest change is to delete or rewrite those sentences so they state the residency room and that the caller slice is trusted. Do not make admission fail for two bricks. The `volume_ray_admission` docs at `graph.rs:702-708` already state the rule the function implements.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1's multi-brick refusal is gone. `volume_ray_admission` subtracts one sampled binding for residency. Limit 0, a ready count of 0, and a ready count above `sampled_limit - 1` return `SampledTextureLimit`. Tier C returns `TierCUnavailable` before the sampled-limit check. `bind_volume_ray` calls admission first, then returns `SampledTextureLimit` when `ready_bricks > brick_slots`, before the residency texture and before any brick view. The later loop pushes one view per passed brick and fills only the unused compiled slots with the dummy. `brick_count` in the uniform is the passed count, not the slot count. `slot_for` searches `i < brick_count`. A dummy slot is not a stand-in for a brick that was dropped.

`brick_slot_count` is `min(sampled - 1, bind_group - 1)`. The generated module puts `texture_3d<u32>` at bindings `0..slots` and `resident_tex` at binding `slots`. The two-slot host assertion sees `brick_0` and `brick_1`, no `var<storage`, and no `@compute`. `compile` builds one fragment pipeline for the composite volume key. The ray module text has no storage and no compute entry.

The 129-slice volume is not refused for being longer than one brick. `clipped_brick` asks each brick only for the voxels that remain inside `dims`. Edge 128 gives brick `[0, 0, 0]` a z need of 128 and brick `[0, 0, 1]` a z need of 1. `two_ready_bricks_both_contribute` synced that volume, bound both ready bricks, and the readback matched. `ready_bricks` returned 2.

Flag 0 still `continue`s at `raycast.wgsl:89` before `load_brick` at line 109. The restored mutation failure is the same red pass 1 recorded. `from_far` still selects the voxel index before the residency load. AC6 passed, so the far sample is the first contributor.

Front to back is unchanged. Colour is the LINEAR_EXACT display divided by the output span. Opacity is the constant 0.5. Green and blue copy red. The test target is `Rgba32Float`. `kind` is written as 0. A non-zero kind `continue`s before the load. `R8Uint` and `R16Uint` are the only brick formats accepted, and two different formats in one ready set return `RayUnavailable` inside the loop that runs before any ray view is created. The dummy texture is `R8Uint` with sample type `Uint`. An all-`R16Uint` ready set was not uploaded, so binding it beside that dummy is unexercised, not a reproduced failure.

No new Rust `as` cast was found in `graph.rs`, `raycast.rs`, or `raycast.wgsl`. WGSL uses constructors. Clippy on `ocelli-render` for all targets exited 0 under `-D warnings`. `tick` and `encode_frame` do not allocate the residency upload or the brick-view list. Those `Vec`s are in `bind_volume_ray`. `encode_frame` still calls `queue.submit` once. An unbound volume slot is not encoded. `set_view` still matches only `Pass::Stack`. `set_voi` still writes `VolumeRaycast`.

No `unsafe`, no `wasm-bindgen`, no `Box<dyn`, and no new trait in the ray files. No tolerance was changed. No gate was disabled. Stale LLD text was not treated as a defect.

## Limitations and handoff

The refusal branch `ready_bricks > brick_slots` was read and was not executed with a live ready set larger than the compiled slot count. Current GPU fixtures use one or two bricks. Deleting that comparison would leave those fixtures green.

Mixed `R8Uint` and `R16Uint` rejection was read and not executed. An `R16Uint` ready set beside the `R8Uint` dummy was not bound. The layout sample type is `Uint` for every brick slot. That is not a draw of `R16Uint`.

Row and Column axes were not drawn. `set_voi` on a volume viewport was not read back as pixels. The ignored rows in `tests/view.rs` and `tests/graph.rs` were not re-run. `Volume::resident` was not re-run. The 129-slice GPU volume did report two ready bricks. Native GL tier B was not opened. This pass did not run `gate --sprint`, wasm, the oracle, or the corpus. A missing adapter did not occur. The four ignored tests did not print the tier C line.

The story is not accepted and is not sprint-ready. The pixel remediation of pass 1's defect can stand. The comment defect is for the implementation agent. A later independent pass should review that edit. Do not close it by refusing two bricks again.
