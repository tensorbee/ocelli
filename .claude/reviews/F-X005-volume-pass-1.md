# F-X005 review, pass 1

**Reviewed**: working tree on `sprint/s21` at `b43bf5e538fefaada6118db72c1ddb724a869754`. Unstaged product paths are `crates/ocelli-render/tests/raycast.rs`, `docs/lld/README.md` and `docs/lld/feature-availability.md`. Nothing is staged. The unstaged diff of those paths hashes to `a80f79893f035b7202461d819cb67c80c1d8787fc851f4a28a31beb5b794cada`. `raycast.rs` hashes to `8478a26e1eb84fc46333da7b75b1370dfe809b189f59fe5fba5fd146a08cd948`. `feature-availability.md` hashes to `00de787ada1011bb9f42e762425dcdff1d7f314728da126dab3928654815e6f9`. `docs/lld/README.md` hashes to `850789d774ecca11c4f3e52c9e8133faf0a99fd63d29b6e92411892a20d24a5c`. `graph.rs` hashes to `f99809416394bf737665c8dafa99eb369c7dc27af8e573b5e23b5d4ac7c26e1e` and its diff is empty. `raycast.wgsl` hashes to `a78aa186603bdab1f60826c9b9dab8b990413d22ac45b6b13863731250edb8e8` and is not in the diff. This review file is not part of that product diff. Nothing was committed. During this pass the CPU comparison inside `volume_ray_admission` was changed to `Tier::A` and then restored. The restored `graph.rs` hash is the hash above.
**Author / reviewer**: the implementation agent wrote the admission assertion, the availability row and the index F-ID. The reviewer is an independent microscope subagent in a separate context and did not write that code. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. Tier C, one ready brick and a sampled limit of 16 is `volume_ray_admission(Tier::Cpu, 1, 16)`. The binding is `Result<(), GraphError>`, so the success type is `()`. A colour vector would not type-check. The function takes no buffer. Its body returns `TierCUnavailable` or `Ok(())`. The CPU comparison is the first check, at `graph.rs:848`, and the sampled-texture check follows at `graph.rs:851`. Limit 16 leaves room for 15 bricks, so one ready brick would be `Ok(())` if the CPU check did not return. Replacing `tier == Tier::Cpu` with `tier == Tier::A` made `volume_raycast_has_no_cpu_image` panic at `raycast.rs:121` because the CPU call was no longer `TierCUnavailable`. The comparison was restored. The same test then passed on the restored file.

AC2. The tier A one-step row is the ignored test `an_absent_step_is_skipped`. It expects `one_resident_colour()`, which is `(200 / 255) * 0.5`, and alpha 0.5. That test is not in this diff. The shader integral at `raycast.wgsl:268` is `accum_c + (1.0 - accum_a) * lit * alpha`, and the alpha step is the next line. That file is not in the diff. Its hash matches the hash recorded by `.claude/reviews/F-062-clip-pass-1.md`. No GPU process was started. The restored admission function still returns `Ok(())` for `Tier::A` with one brick and limit 16. The same host test asserts that. The CPU refusal is not applied to tier A.

AC3. `docs/lld/feature-availability.md` says `VOLUME_3D` is full on tier A and tier B and unavailable on tier C. It says `volume_ray_admission` returns `TierCUnavailable` before a sample is read and writes no colour buffer. It says there is no CPU ray. `RenderGraph::new` returns `TierCUnavailable` before `compile`. `compile` builds `volume_ray_module` with no tier branch in the shader text. `CpuRaster::raster_into` shades a stack through `LutChain` and is not called from admission. `sync_ready_bricks` and `brick_plan` return unavailable on CPU before a brick is cut or a format is named. No second ray integrator was found under `crates/ocelli-render`.

The order case that this host test does not execute is `volume_ray_admission(Tier::Cpu, 1, 0)`. With the CPU check first that call is `TierCUnavailable`. If the limit check moved above it, the same call would be `SampledTextureLimit`. The source still has the CPU return first. The executed input is the AC1 input. One brick and limit 16 do not trip the limit check, so that green row does not by itself prove the order. The order was read.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff` hash | `sprint/s21` working tree | Executed now | HEAD `b43bf5e538fefaada6118db72c1ddb724a869754`. Three dirty tracked paths. Nothing staged. Diff sha256 `a80f7989...cada`. `graph.rs` diff is 0 bytes. Shader sha256 `a78aa186...b8e8`. |
| `bin/ocelli.sh test ocelli-render --test raycast volume_raycast_has_no_cpu_image` | restored tree, `Tier::Cpu` at `graph.rs:848` | Executed now, after the mutation was restored | Exit 0. 1 passed, 0 failed, 25 filtered out. The crate recompiled first. |
| Same test after `tier == Tier::Cpu` became `tier == Tier::A` inside `volume_ray_admission` only | temporary edit of `graph.rs`, not left in the tree | Executed now | Test result failed. 0 passed, 1 failed. Panic at `raycast.rs:121`: `assertion failed: matches!(cpu_ray, Err(GraphError::TierCUnavailable))`. File restored. Hash matches the reviewed `graph.rs` hash. |
| `bin/ocelli.sh clippy ocelli-render` | tree before the temporary edit, same three-path diff | Executed now | Exit 0. Finished in 0.11s. No warning text. |
| Read of `volume_ray_admission`, `bind_volume_ray`, `RenderGraph::new`, `compile`, `encode_frame`, `CpuRaster` | current working tree | Executed now | CPU return is before the limit check. Bind calls admission after `bricks.len()` and before spacing, the patient frame and texture creation. One `queue().submit` remains at `graph.rs:3467`. |
| Ignored ray rows and `raycast.wgsl` | not in the diff | Not executed now | No GPU process. The one-step ignore and the shader integral were compared by path and hash only. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`volume_ray_admission` is `Result<(), GraphError>` at `graph.rs:843`. `tier == Tier::Cpu` returns `GraphError::TierCUnavailable` before `sampled_limit` is read. A limit of 0, a ready count of 0, or a ready count above `sampled_limit - 1` returns `SampledTextureLimit` only after that return. `Tier` has three variants. A and B do not take the CPU return. The host test calls CPU with one brick and limit 16, pins the success type as `()`, and still requires tier A with one brick, tier A with two bricks, and tier B with one brick and limit 2 to be `Ok(())`. The diff changes only that CPU assertion and its comment. It adds no `as`, no `unsafe` and no `wasm_bindgen`.

`bind_volume_ray` converts `bricks.len()` and then calls `volume_ray_admission` at `graph.rs:2494`. A CPU error returns before `spacing_mm_ok`, `PatientFrame::try_new` and any texture for that call. The admission function does not read a sample and has nowhere to write a colour buffer. `RenderGraph::new` returns `TierCUnavailable` at `graph.rs:1395` before `compile`, so a CPU context never builds the ray module. `set_opacity`, `set_shade` and `set_clip_planes` each return `TierCUnavailable` before a viewport lookup. `encode_frame` still submits one encoder. None of those lines are in the diff.

`compile` assigns `volume_ray_module(brick_slots)` with no second shader for tier B. The fragment entry remains `fs_main` in `raycast.wgsl`. The front-to-back sum is unchanged. `an_absent_step_is_skipped` stays `#[ignore]` and was not edited. `flat_opacity_keeps_the_one_step_integral` was not edited. That is the existing F-060 one-step evidence this story was told to leave in place.

The availability header and the README row both list `F-037, F-041, F-X001, F-X003, F-056, F-X005`. The date on the LLD is `2026-11-01`, which is the date these commands ran. The new sentences have no em dash and no semicolon. The remaining-work table no longer leaves the volume declaration as future work owned by F-X005. The paragraph under it states full on A and B, unavailable on C, and no CPU ray. The not-built bullet says F-X003 draws the stack in `CpuRaster`, that this contract does not own that raster, that MPR remains unbuilt in this file, and that tier C volume rendering stays unavailable. `CpuRaster` is the stack raster in `cpu.rs`. Its module says it does not open a device. `raster_into` documents a monochrome draw through `LutChain`. Admission does not call it. `docs/sprints/BACKLOG.md` still marks F-X003 `in-progress`, F-X004 `pending` and F-X005 `pending`. This diff does not edit those rows and does not treat F-X004 as done.

D-07 is the rule the new paragraph names. No deviation file was edited. The plan asked for no new deviation. The GPU ray binding was not given a new resource.

## Limitations and handoff

The ignored raycast rows were not run. Tier B was not opened on a native GL adapter. Full on tier B is the shared fragment module in `compile`, not a new device draw. The check-order swap for `Tier::Cpu` with limit 0 was read and not executed. `RenderGraph::new` on a CPU context was not called. `set_opacity`, `set_shade` and `set_clip_planes` were not called on CPU.

`docs/lld/render-graph.md` still says F-X005 owns the path that F-059 did not implement as a CPU colour vector. That file is outside this diff. The sentence does not say a CPU image exists. `docs/hld/DEVIATIONS.md` still says F-X005 decides between a slow path and unavailable. The plan does not edit that file. The availability note now records unavailable.

F-X003 remains in progress. This story does not call `CpuRaster` and does not draw a CPU volume image. F-X004 stays pending. This is a bounded feature review of the uncommitted tree. It is not a sprint-profile verification and it is not a completion record. The backlog row for F-X005 is still `pending`. No product file was left changed by this pass.
