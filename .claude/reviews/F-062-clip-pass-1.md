# F-062 review, pass 1

**Reviewed**: working tree on `sprint/s21` at `640f7b922cffe4ee3ffc283c12be2d80797cdeab`. Unstaged product paths are `crates/ocelli-render/shaders/raycast.wgsl`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/tests/raycast.rs`, `docs/lld/README.md`, `docs/lld/render-graph.md`, `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md`, plus untracked `crates/ocelli-render/src/clip.rs`. The staged diff of those tracked paths hashes to `ca79430ceea5f72358b7b96d64eb6da43291f8e199c74139766e52c2363bcd3e`. `clip.rs` hashes to `e93e5e690f76299e93de8ae2c832e641912135c87c8d5a6fe3d2c21c744d94bb`. `raycast.wgsl` hashes to `a78aa186603bdab1f60826c9b9dab8b990413d22ac45b6b13863731250edb8e8`. This review file is not part of that product diff. Nothing was staged. Nothing was committed. The shader was not edited during this pass.
**Author / reviewer**: the implementation agent wrote the clip list, the uniform, the fragment test and the ledger edits. The reviewer is an independent microscope subagent in a separate context and did not write that code. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. An empty list keeps every finite patient point on the host, and the shader skips a slot whose normal `w` is 0. The registry starts from `ClipList::empty`. Bind writes that list. No caller plane is installed by default. Samples 200 and 200 at spacing 10 mm, both resident, flat alpha 0.5, produce the two-step colour `(200/255) * 0.75` and alpha 0.75. A default plane through the volume would have dropped a side and failed `no_planes_keep_both_resident_samples`. That row passed. The older ignored rows also stayed green, so the new binding did not change the F-060 integral or the F-061 shade rows.

AC2. `Volume::voxel_world` places index `(0, 0, 0)` at the origin and index `(0, 0, 1)` at `spacing[2]` on column 2. Origin 0 and spacing `(1, 1, 10)` put the samples at 0 mm and 10 mm. A plane through 5 mm with normal `(0, 0, 1)` has dot products -5 and +5. The kept side is `dot >= 0`, so only 10 mm stays. Stored 200 under `LINEAR_EXACT` centre 127.5 and width 255 is display 200. Flat alpha is 0.5. One front-to-back step is colour `(200/255) * 0.5` and alpha 0.5. Both samples kept would be the two-step colour. Both dropped would be 0. `a_plane_keeps_only_the_sample_past_five_millimetres` matched the one-step colour and the exact alpha. `draw_clip` checks `brick_uploads` across `set_clip_planes` and still sees one ready brick. `volume_ray_admission` does not read planes.

AC3. The same formula puts the only voxel of a 1 mm volume at 0 mm. A plane through 0 mm with normal `(0, 0, 1)` has dot 0. `>=` keeps it. A strict `>` drops it and the pixel is 0. `assert_ray` wants colour within 2 ULP of `(200/255) * 0.5` and alpha bits of 0.5, so 0 cannot pass. `a_sample_on_the_plane_stays_in_the_integral` passed on the same `draw_clip` path that AC2 showed is applying a plane. The source has one `dot(delta, normal.xyz) >= 0.0` and no `dot(delta, normal.xyz) > 0.0`. The host module test requires that `>=` string. This pass did not flip the operator.

AC4. `ClipList::from_planes` returns before any slot is published. Count is tested first. Finiteness is tested before the f64 length. The f64 length is tested before the f32 image. Seven planes are `Count`. NaN, infinity and `1e40` are `NotFinite`. A zero normal, a negative-zero normal, and `1e-46` are `ZeroNormal`. An origin of `1e-46` is `NotFinite` because it narrows to zero. The host test builds those errors and the previous local list still drops 0 mm and keeps 10 mm. On the device, `set_clip_planes` maps those errors and returns before `write_buffer` and before `state.clip` is replaced. `a_refused_plane_list_keeps_the_clipped_colour` sees `ClipCount`, `ClipNotFinite` and `ClipZeroNormal`, leaves `brick_uploads` unchanged, and a second write of the same plane is `FrameOutcome::Noop`. That no-op is the executed sign that the stored list still matched. A call before bind is `RayUnavailable` and does not store the plane.

AC5. Tier C has no ray image. `volume_ray_admission(Tier::Cpu, ...)` returns `TierCUnavailable`. `RenderGraph::new` returns the same error before a device exists. `set_clip_planes` returns that error before it reads a viewport. The ray module has no CPU twin and no second LUT.

The discriminating case this suite does not draw is a non-zero origin. Origin z of 100 mm and spacing z of 10 mm put voxel 0 at 100 mm and voxel 1 at 110 mm. A plane through 105 mm with normal +z keeps only 110 mm. An index-only point would put those voxels at 0 mm and 10 mm and drop both. The shader adds `crop.origin` before the scaled columns. The GPU fixtures use origin 0, so those pixels do not prove the add. The uniform writer and `patient_point` were read against `Volume::voxel_world`.

Colour bits, computed in `f32` outside the crate: display 200 is `1128792064`, grey `200/255` is `1061734601`, one-step colour is `1053345993`, two-step colour is `1058444951`, alpha 0.5 is `1056964608`, alpha 0.75 is `1061158912`. `LINEAR_EXACT` at centre 127.5 and width 255 has low end 0 and high end 255. Stored 200 is inside the body. The f32 window evaluates to 200.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff` hash | `sprint/s21` working tree | Executed now | HEAD `640f7b922cffe4ee3ffc283c12be2d80797cdeab`. Eight dirty tracked paths plus untracked `clip.rs`. Nothing staged. Diff sha256 `ca79430c...bcd3e`. `clip.rs` sha256 `e93e5e69...94bb`. Shader sha256 `a78aa186...b8e8`. |
| `f32` window, one-step product, two-step product, subnormal square | stored 200, centre 127.5, width 255, alpha 0.5 | Executed now, outside the crate | Bits quoted above. `1e-40_f32` is finite and non-zero, and its square bits are 0. |
| Read of `clip.rs`, `raycast.wgsl`, `bind_volume_ray`, `set_clip_planes`, `RayHeader`, `encode_frame` | current working tree | Executed now | Separate 256-byte binding 4. Header assert stays 48. One `queue.submit` in `encode_frame`. No `as`, `unsafe` or `wasm_bindgen` in the new Rust. |
| `bin/ocelli.sh test ocelli-render --lib --test raycast` | working tree | Executed now | Exit 0. Lib 115 passed, 3 ignored, 0 failed. Raycast host 12 passed, 14 ignored, 0 failed. Includes the six `clip::tests` rows. |
| `bin/ocelli.sh clippy ocelli-render` | working tree | Executed now | Exit 0. |
| `bin/ocelli.sh test ocelli-render --test raycast -- --ignored --test-threads=1` | Auto adapter, serial | Executed now | Exit 0. 14 passed, 0 failed, 1.90s. Includes the four clip rows and the ten older ray rows. |
| Same three clip pixel rows with `--nocapture` | same tree | Executed now | Exit 0. 3 passed in 0.96s. No tier-C skip line was printed. |
| Comparison flipped from `>=` to `>` | not edited | Not executed now | Source count of `dot(delta, normal.xyz) > 0.0` is 0. Count of `>= 0.0` on that dot is 1. Author mutation was not re-run. |
| Native GL, non-zero origin, rotated columns, column and row walks, device-loss rebind | not drawn | Not executed now | Recorded under limitations. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`patient_point` is `origin + axis_x * (spacing_x * i) + axis_y * (spacing_y * j) + axis_z * (spacing_z * k)` at `raycast.wgsl:200`. `PatientFrame::try_new` stores `direction[0]`, `direction[1]` and `direction[2]` as those axes. `direction_of` copies `Volume::direction.col(0)` through `col(2)`. That is the same product as `Volume::voxel_world` in `crates/ocelli-volume/src/ortho.rs`. The origin is the centre of voxel `(0, 0, 0)`. There is no second patient-point formula in the render crate. The clip test runs after the residency flag and before the brick load, at `raycast.wgsl:243`. A discarded step adds no colour and no alpha. `neighbour_stored` does not consult the plane, which matches the existing rule that a skipped step does not erase a stored texel. Unlit shade is ambient 1 and diffuse 0, so a non-zero gradient on the kept sample still multiplies by 1.

`sample_kept` loops six slots. `normal.w == 0.0` skips the slot. The keep test is `!(dot(delta, normal.xyz) >= 0.0)` at `raycast.wgsl:215`. A disabled slot is stored as zeros, including `w`. An active slot writes `w` of 1. The host length test uses f64 squares first. `narrow_normal` then refuses an f32 image whose square sum is zero, which includes a subnormal component whose square underflows. A zero-length shader normal is not installed as an active plane. Exact zeros in an origin are allowed. A non-zero origin or direction component that narrows to zero is refused. Bind turns that frame refusal into `RayUnavailable` before any texture for that call is created.

`ClipPlane` fields stay public because `set_clip_planes` copies a validated list and does not keep the caller struct. `from_planes` of more than six planes returns `Count` before the loop. A later bad plane returns `Err` and drops the partial array. `keeps` is `cfg(test)` and uses `partial_cmp` against 0, so equality is kept and NaN is dropped. The f64 host list and the f32 shader agree on 0 mm, 5 mm and 10 mm because those values are exact in both types. `uniform_bytes` starts from 256 zero bytes. The prefix is four `vec4` values. Each slot is 32 bytes from offset 64. The unit test reads axis z at byte 56, the plane origin z at 72, the normal z at 88, active `w` at 92, and the next slot `w` at 124. That row passed.

`RayHeader` is still 48 bytes at `graph.rs:778`. `spacing_x`, `spacing_y` and `spacing_z` remain the last three words. Shade stays group 0 binding 3. Opacity stays binding 2. The clip uniform is group 0 binding 4 and is not packed into the ray header. The stack layout at `graph.rs:1474` is still three bindings. The volume layout adds binding 4 only. `fusion.wgsl` and `voi.wgsl` are not in the diff. The ray module text has no `var<storage` and no `@compute`. The pipeline blend is unchanged.

`set_clip_planes` returns `TierCUnavailable` before a viewport lookup. It returns `RayUnavailable` when no ray buffer exists, and that return is before `from_planes` stores anything. A refused list hits `map_err` and `?` before `write_buffer`. The CPU list is assigned only after that write, at `graph.rs:3015`. Equal f64 bits return `Ok` without a write and without setting dirt. The write is `queue.write_buffer` of the existing 256-byte buffer. It does not call `create_buffer`, `write_texture` or `upload_brick`, and it does not increment `brick_uploads`, `uploads` or `lut_uploads`. `uniform_bytes` is a stack array. `from_planes` uses a stack array. `encode_frame` still submits one encoder. Bind creates the clip buffer with the remembered list and the new frame, then stores that frame. `install_recovered` drops GPU slots and does not clear `state.clip`. This pass did not run a device loss.

The new LLD sentences match those facts. The uniform is 256 bytes at binding 4. The kept side includes the plane. A discarded sample adds nothing and the brick stays bound. More than six planes, a non-finite component or a zero-length normal leaves the previous list. Zero planes leave the measured ray. `docs/lld/render-graph.md` has no em dash. The new clip paragraph has no semicolon. The README row names the patient clip planes and F-062. `BACKLOG.md` and `CURRENT_SPRINT.md` mark F-062 in progress. F-X003 is in progress in the backlog, so the sprint line that F-X004 and F-X005 stay pending while F-X003 is in progress is true. No new Rust `as` cast was added. The diff's only `as` hits are the words in comments.

`assert_ray` was not edited. Colour stays within 2 ULP and alpha stays exact bits. The one-step value and the two-step value differ by far more than 2 ULP. The on-plane failure mode is colour 0 against `0.39215687`, which is the same gap the author reported for a `>` shader. This pass did not produce that failure because the shader was left at `>=`.

## Limitations and handoff

Native GL tier B was not opened. The ray tests call `resolve_adapter` with `TierRequest::Auto`. This pass did not record the adapter name. The pixel rows prove some device executed the fragment shader. They do not prove a GL compile.

Column and row walks were not drawn with a plane. A non-zero origin was not drawn. A non-identity direction was not drawn. A second plane was not drawn on the device. Six active planes were accepted on the host and were not drawn. The shader loop bound is 6 and the slot stride was checked on the host. Those reads are not pixel evidence.

`set_clip_planes` on a CPU graph was not called. `RenderGraph::new` refuses tier C before a graph exists, and the CPU return inside `set_clip_planes` was read rather than executed. Device-loss recovery of a remembered plane list was read and not run. The full `ocelli-render` ignored suite outside `raycast.rs` was not run. This shader is not included by the slab or fusion modules.

The comparison was not mutated in this pass. The instruction for the pass was not to edit the shader. The live `>=` path passed the on-plane row, and the source does not contain the strict operator on that dot.

This is a bounded feature review of the uncommitted tree. It is not a sprint-profile verification and it is not a completion record. The story is still in progress in the ledgers. No product file was changed by this pass.
