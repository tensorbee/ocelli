# F-059 review, pass 1

**Reviewed**: working tree on `sprint/s19` at `42336565115749daa4f20e8e04a18f1cb72729f2`. Unstaged product paths are `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/tests/graph.rs`, `crates/ocelli-render/tests/view.rs`, `crates/ocelli-volume/src/lib.rs`, plus untracked `crates/ocelli-render/shaders/raycast.wgsl`, `crates/ocelli-render/tests/raycast.rs` and `crates/ocelli-volume/tests/resident.rs`. This review file is not part of that product diff.
**Author / reviewer**: the sprint implementation agent wrote the ray cast. The reviewer is an independent microscope subagent in a separate context and did not write this code. This is not a human review.
**Result**: 1 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1, AC2 and AC6 were checked against PS3.3 C.11.2.1.3.2 and the plan integral, not against the shader comment. LINEAR_EXACT at centre 127.5 and width 255 uses `x <= c - w/2` and `x > c + w/2`, with no `c - 0.5` and no `w - 1`. Stored 0 meets the lower bound because `127.5 - 127.5` is 0, so the display is `ymin` 0. Stored 200 is inside `(0, 255]`. The body `((200 - 127.5) / 255 + 0.5) * 255` is exactly 200 in `f32` for this window. Colour is that display divided by `ymax - ymin`. Opacity stays 0.5. Front to back gives AC1 `C = (200/255) * 0.5 * 0.5` and `A = 0.75`, AC2 `C = (200/255) * 0.5` and `A = 0.5`, and AC6 the AC2 colour with `A = 0.75`. A host evaluation of that operation order matches the test constants at 0 ULP. Alpha 0.5 and 0.75 are exact `f32` values. The same window under LINEAR is 51805 ULP away from the AC1 colour, so a swapped window function cannot hide inside the 2 ULP bound.

The three GPU tests encode those three different pairs. One shared formula cannot satisfy all three. A missing adapter did not occur on this run. The tests did not print the tier C return, and they passed the pixel assertions.

The flag mutation rejects a shader that samples the absent texel. Changing `if (flag == 0u)` to `if (flag == 0xFFFFFFFFu)` made `an_absent_step_is_skipped` fail with `red 0.5882354 is not within 2 ULP of 0.39215687`. That red is the integral of two stored-200 samples, which is what the texture holds at both steps. Treating the absent step as stored 0 with alpha 0.5 would have produced alpha 0.75. The exact alpha 0.5 rejects that too. The shader was restored. SHA-256 before and after is `e704f21ef92467de0262c75a9380d7b8b5db3d9ff3f295c2851dab1b4b1e3051`. The restored absent-step test passed.

AC3 was run. Dims `[1, 1, 129]`, edge 128, slices 0 through 127 placed, slice 128 absent. `resident` is true at z 0 and z 127, and false at z 128, z 129 and x 1. AC4's host table refuses MIP and keeps composite fragment-only. AC5's host admission returns `TierCUnavailable` on `Tier::Cpu`.

The plan sentence that does not hold is the multi-brick bind, judged in D1. It is not a wrong pixel. A subset is not drawn.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --short` and `git rev-parse HEAD` | `sprint/s19` | Executed now | HEAD `42336565115749daa4f20e8e04a18f1cb72729f2`. Product diff is the five modified paths and three untracked paths listed above. |
| `bin/ocelli.sh test ocelli-volume --test resident` | AC3 volume, edge 128 | Executed now | Exit 0. `resident_follows_ready_bricks` passed. |
| `bin/ocelli.sh test ocelli-render --test raycast` | host AC4 and AC5 | Executed now | Exit 0. 2 passed, 3 ignored. |
| `bin/ocelli.sh test ocelli-render --test graph` | non-ignored graph rows | Executed now | Exit 0. 2 passed, 5 ignored. |
| `bin/ocelli.sh test ocelli-render --test raycast -- --ignored --test-threads=1` | AC1, AC2, AC6 on `TierRequest::Auto` | Executed now, twice, the second with `--nocapture` | Exit 0 both times. 3 passed. No tier C print. Pixel assertions ran. |
| `bin/ocelli.sh test ocelli-render --test view --test graph -- --ignored --test-threads=1` | edited `set_voi` and `set_view` rows, plus the other ignored rows in those files | Executed now | Exit 0. graph 5 passed. view 5 passed. |
| `bin/ocelli.sh test ocelli-render --test view -- --ignored --test-threads=1 --nocapture tier_b_margin_or_a_recorded_miss` | native GL | Executed now | Exit 0. Printed `native Backends::GL found no adapter. This is not a tier B run.` |
| Flag mutation of `raycast.wgsl`, then `an_absent_step_is_skipped` | same AC2 fixture, condition `flag == 0xFFFFFFFFu` | Executed now | Test failed. Panic at `tests/raycast.rs:289`: red `0.5882354` versus `0.39215687`. |
| Restore from the pre-mutation backup, then the same test | SHA-256 `e704f21e...1e3051` | Executed now | Exit 0. `if (flag == 0u)` is back. Hash matches. |
| `bin/ocelli.sh clippy ocelli-volume` and `bin/ocelli.sh clippy ocelli-render` | workspace lints, `-D warnings` | Executed now | Exit 0 and exit 0. |
| Host `f32` integral of the plan formula | stored 0 and stored 200, LINEAR_EXACT, opacity 0.5 | Executed now | 0 ULP against the three test constants. LINEAR misses AC1 by 51805 ULP. |

The mutation shell stopped on a readonly `status` assignment before its own restore. The shader was still mutated at that moment. The next command copied the backup back. The hash check and the green rerun are from that restored file.

## Defects

### D1, a ready set that fits the device limit is refused

**Severity**: Defect

**Where**: `volume_ray_admission` in `crates/ocelli-render/src/graph.rs:644-662`, the single brick binding in `crates/ocelli-render/shaders/raycast.wgsl:17`, and the lock in `crates/ocelli-render/tests/raycast.rs:106-109`. `bind_volume_ray` calls admission at `graph.rs:2081`, before it creates a residency texture. The one-brick extent test is `graph.rs:2128-2131`.

**Trigger**: `volume_ray_admission(Tier::B, 2, 16)`, or `bind_volume_ray` with `ready_bricks` greater than 1 while `sampled_limit` is at least that count.

**Expected**: Bind every ready brick when the count is within the device sampled-texture limit. Above that limit, return unavailable and do not draw a subset.

**Actual**: `bound` is `1u32`. The function returns `GraphError::SampledTextureLimit` when `ready_bricks > bound`, even if `ready_bricks <= sampled_limit`. A fake cap of 0 also returns that error, which the plan does ask for. No subset is drawn. `bind_volume_ray` still takes one `BrickKey`. It also returns `RayUnavailable` when any axis of `dims` is longer than that brick's extent, so a 129-slice volume is refused even when only one brick is ready.

**What**: Two ready bricks on a limit of 16 never reach the shader. The ordinary volume that spans two level-0 bricks is unavailable. The error name says the sampled-texture limit, but the count fits that limit. The shader declares one `texture_3d<u32>` plus the residency `texture_2d<u32>`.

**Why it is wrong**: The approved plan says: "A viewport whose ready set is several bricks binds them up to the device's sampled-texture limit. Above that limit, registration returns unavailable and does not drop bricks." The same paragraph says the host check is a fake cap of 0, and that the AC1 volume is the one-brick case under the limit. "One brick texture is bound for the fixture" does not replace the next sentence. The safety sentence "The ray does not drop that brick and draw the rest, because the rest would be a different image" is why this is not classed as an acceptable refusal or as a smell. The refusal avoids a wrong image. It does not do the bind the plan requires for a ready set that fits the device limit. Deleting `ready_bricks > bound` while the bind still stores one key would draw a subset, which the plan also forbids. That is not the fix.

**Evidence**: The unit test expects `volume_ray_admission(Tier::B, 2, 16)` to be `SampledTextureLimit`, and that test passed. `volume_ray_admission(Tier::A, 1, 0)` also returns `SampledTextureLimit`, and that half matches the plan. No GPU test draws two bricks.

**Scope and regression**: Present on this working tree. Consumers are volume viewports whose ready count is greater than 1 and not greater than the adapter limit, and a one-ready-brick volume whose dims exceed that brick. The smallest regression is a ready count of 2 with sampled limit 16 that is not `SampledTextureLimit`, together with a draw that includes every ready brick and does not sample a dropped brick. Today's test expects the error, so it stays green while the plan is missed. This review does not specify a multi-brick binding layout. The residency texture is a second sampled binding. An admission check that treats `ready_bricks <= sampled_limit` as enough must still leave room for it.

## Smells

None.

## Nitpicks

None.

## Verified clean

The ray module is `VOI_WGSL` followed by `raycast.wgsl`. The window functions are not retyped. `fn_kind` 1 selects the existing LINEAR_EXACT body. The fixture chain is LinearExact, slope 1, intercept 0, output 0 to 255, Monochrome2. Stored samples are the container bytes. Kind 0 converts with `f32(textureLoad().r)` and the shader never divides by 255. `R8Uint` and `R16Uint` pass the host format check. Any other brick format returns `RayUnavailable` before a view is bound, so an unorm texture is not sampled. The AC1 upload is `U8`, which is `R8Uint`. `R16Uint` was not drawn.

Front to back matches the plan. `C` and `A` start at 0. Each resident step adds `(1 - A) * colour * 0.5` and `(1 - A) * 0.5`. Colour is `(display - ymin) / (ymax - ymin)`, or 0 when the span is not positive. The span `select` is the same shape the stack shader already uses. Green and blue are written equal to red. The attachment stays the format `compile` was given. The GPU target in the test is `Rgba32Float`. Comparison of `C` is `to_bits` within 2 ULP. Alpha compares exact bits.

`from_far` reverses the voxel index with `select(i, n - 1u - i, ray.from_far != 0u)`. The residency texel is loaded at that voxel index, not at the loop index. WGSL `select` evaluates both arms. For `steps >= 1` and `i < steps`, `n - 1u - i` does not wrap. `bind_volume_ray` returns `RayUnavailable` when `flags.len()` is 0 or is not the axis length, and that check runs before the uniform is written. AC6 passed, so the far sample is the first contributor. Ignoring `from_far` would have left the AC1 colour, which is not within 2 ULP of the AC6 colour.

A flag of 0 `continue`s before the brick `textureLoad`. The restored source still has that order at `raycast.wgsl:61-69`. The mutation above is what shows the test goes red when the condition no longer matches 0.

`compile` builds one fragment pipeline for `(VolumeRaycast, Composite, tier)` from `volume_ray_module` on both GPU tiers. It does not build a compute pipeline, a storage pipeline, or a non-composite volume pipeline. The module text has no `var<storage` and no `@compute`. The host test asserts both. MIP, MinIP, average and labelmap edge stay uncompiled because the blend must be composite. Tier C `RenderGraph::new` returns `TierCUnavailable` before `compile`. `volume_ray_admission(Tier::Cpu, ...)` returns `TierCUnavailable`. There is no CPU `Vec` of ray colours in the new path. The only new `Vec` is the residency upload in `bind_volume_ray`, which is not `tick`.

An unbound composite volume viewport is not encoded. `tick` requires `slot.ray` before it counts the viewport as encodable, and `encode_frame` skips a volume slot whose ray binding is absent. The composite volume pipeline being compiled does not substitute for that binding. `set_view_dirties_only_the_camera_and_keeps_the_css_matrix` registers a stack and an unbound volume and still expects `Submitted { viewports: 1, passes: 1 }`. That ignored test passed, so the unbound volume was not a second pass. `set_view` still matches only `Pass::Stack` and returns `PassHasNoUniform` for the volume viewport. `set_voi` accepts `VolumeRaycast`, writes `bytemuck::bytes_of` of the same `VoiParams` buffer the ray bind group uses, and sets `Dirty.voi`. `SegOverlay` remains the no-uniform witness. `set_voi_is_the_single_uniform_route_and_rejects_in_order` passed on the device. A second tick with no new dirt is `Noop`. `encode_frame` calls `queue.submit` once. `present` is not a second submit. The test readback submits on its own encoder after the frame.

`Volume::resident` is true only when `brick_of` succeeds and `brick_is_ready` is true for that level-0 brick. Outside `dims` is false. It does not call `ready_bricks` and does not build that `Vec`. `brick_grid` returns a `Copy` grid. The comment matches the function.

No new trait, no `Box<dyn>`, no `unsafe`, and no `wasm-bindgen` in the product diff. New Rust has no `as` cast. WGSL `u32(at.x)` truncates the fragment centre toward zero, which is the pixel index. `f32` of an `R8Uint` or `R16Uint` texel is exact for every value those formats hold. The host never sets `kind` to anything but 0, and a non-zero kind adds nothing rather than normalising. Clippy on both crates exited 0 under `-D warnings`, including the workspace denies for truncation, precision loss, sign loss, `float_cmp`, `unwrap_used` and `expect_used`.

`pipeline_compiled` is still tier-agnostic, as it already was for the stack key. A CPU composite volume key is reported compiled by that predicate. The image is still refused by graph creation and by `volume_ray_admission`. No tolerance was changed. No gate was disabled. Stale LLD text was not treated as a defect. Comments in the new files describe the one-binding refusal as it exists in this tree. They do not claim that two bricks are drawn.

## Limitations and handoff

Native GL tier B was not this host. `tier_b_margin_or_a_recorded_miss` recorded the miss quoted above. Tier A and tier B share `volume_ray_module` in `compile`, but this machine did not compile or draw that module on a GL device. Row and Column axes were not drawn. Only `RayAxis::Slice` was. `R16Uint` was accepted by reading the format check and was not uploaded. `set_voi` on a volume viewport was not read back as pixels. The buffer write is the same statement the stack path uses, and the volume match arm stores the CPU copy. A no-adapter return in the three ignored ray tests prints a tier C line and then passes. That return was not taken. It would not be evidence for AC1, AC2 or AC6. `bin/ocelli.sh gate gpu` documents the same no-adapter pass for the rest of the crate. This pass did not run `gate --sprint`, wasm, the oracle or the corpus. The story is not accepted and is not sprint-ready. D1 is for the implementation agent. A later independent pass should review the remediation. Do not clear D1 by letting admission return success while one brick texture still stands in for a larger ready set.
