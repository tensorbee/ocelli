# F-061 review, pass 1

**Reviewed**: working tree on `sprint/s21` at `f6a63709c5ac0f2e172bbee2f22444c255b716f3`. Unstaged product paths are `crates/ocelli-render/shaders/raycast.wgsl`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/tests/raycast.rs`, `docs/lld/README.md` and `docs/lld/render-graph.md`, plus untracked `crates/ocelli-render/src/shade.rs`. `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md` are also dirty and still say F-061 is in-progress. This review file is not part of that product diff. Nothing was staged. Nothing was committed. One light component in `raycast.wgsl` was flipped for a mutation run and then restored. The shader sha256 after the restore is `1e257f5ab4078436772c41f7c61e0be17edbdfe106f1c6aee5808a945911f973`, the same digest taken before the flip.
**Author / reviewer**: the implementation agent wrote the shade weights, the uniform, the fragment gradient and the LLD edit. The reviewer is an independent microscope subagent in a separate context and did not write that code. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. `ShadeWeights::unlit` is ambient 1 and diffuse 0. `shade_of` returns that ambient weight when the gradient length is 0. Otherwise it returns `ambient + diffuse * max(0, dot(n, light))`. Diffuse 0 makes that sum 1 even when the gradient is not zero, so a viewport that never calls `set_shade` keeps the F-060 colour. Samples 100, 200, 300 point the gradient at the far end. A default diffuse weight would zero that pixel. The ignored row `unlit_shade_keeps_the_flat_integral` kept the flat one-step product. The older ignored rows also stayed green: an absent step, two resident steps, two bricks, the far end first, the ramp, and `set_opacity` leaving `brick_uploads` unchanged.

The centre colour is still PS3.3 C.11.2.1.3.2. `LINEAR_EXACT` at centre 127.5 and width 255, output 0 to 255, clamps at `c - w/2 = 0` with `<=` and at `c + w/2 = 255` with `>`. Stored 200 is inside that open-closed body. Slope 1 and intercept 0 leave it unchanged before the window. Evaluated in `f32`, the display is exactly 200.0 (bits `1128792064`). The grey colour is `200/255` (bits `1061734601`). Flat multiplies by 0.5, so the one-step colour bits are `1053345993` and the alpha bits are `1056964608`.

AC2. Samples 300, 200, 100 along z, spacing 1, ambient 0, diffuse 1. Only the centre step is resident (`flags` 0, 1, 0). The central difference is `(100 - 300) / 2 = -100` (bits `3267887104`). The unit normal is `(0, 0, -1)`. A slice walk uses light `(0, 0, -1)`. The dot is +1 and the shade is 1 (bits `1065353216`). The pixel is the flat one-step product, and the alpha stays 0.5. `light_along_the_gradient_keeps_the_voi_product` passed. Transfer opacity is multiplied once, in the front-to-back term, and is not multiplied by the shade.

AC3. Samples 100, 200, 300 give gradient `(300 - 100) / 2 = 100` (bits `1120403456`). The unit normal is `(0, 0, 1)`. The dot with `(0, 0, -1)` is -1, so the diffuse term is 0, the colour is 0 and the alpha stays 0.5. `light_against_the_gradient_zeroes_the_colour` passed. The plan table names 100, 200, 300 as the bright fixture and says "light along the gradient". The locked decision fixes the light toward decreasing index, so the bright samples are 300, 200, 100 and the dark samples are 100, 200, 300. The shader and both GPU rows follow that lock. This pass does not treat the table's sample order as a second light.

AC4. `from_weights` tests finiteness before `0.0..=1.0`. NaN and infinity are `NotFinite`. A finite weight outside that range is `OutOfRange`. The `Err` builds no value. The fields are private and the only constructors are `unlit` and `from_weights`, so a caller cannot write NaN into a value the constructor already accepted. The plan sketch showed public fields. Private fields are the lock that makes the refusal hold, not a defect. `set_shade` takes `&ShadeWeights` only. The host test builds an unlit pair, refuses NaN, infinity, 1.1 and -0.1, and then reads the same unlit bits.

AC5. Tier C still has no ray image. `volume_ray_admission` and `RenderGraph::new` return `TierCUnavailable` before a device. `set_shade` names the same error. The ray module has no CPU twin.

The 48-byte header and the F-060 opacity uniform stay intact. `RayHeader` is still twelve 4-byte words and the compile-time assert stays 48. The last three words are `spacing_x`, `spacing_y` and `spacing_z` where the pads were. Shade weights are not in that header. They are a 16-byte uniform at group 0 binding 3. Opacity stays group 0 binding 2 and 1024 bytes. The ignored ramp row still passed, so the opacity address did not move. The ignored two-brick row and the far-end row still passed, so the brick-coordinate array still begins at byte 48.

A zero gradient uses the ambient weight and does not read the light. Samples 200, 200, 200 with ambient 0.5 and diffuse 1 produced colour bits `1044957385`, which is `(200/255) * 0.25` and also `(colour * 0.5) * 0.5`. Alpha stayed 0.5. `a_zero_gradient_uses_the_ambient_weight` passed.

`set_shade` before bind returns `RayUnavailable` and returns before `state.shade` is assigned, so the later bind still uploads unlit. Equal bits return before `write_buffer` and do not set dirt again. The against-light row calls `set_shade` before bind, expects `RayUnavailable`, then binds, writes the diffuse pair, draws colour 0, and a second `set_shade` of those same bits is followed by `FrameOutcome::Noop`. `brick_uploads` does not change across `set_shade`. The product call is `queue.write_buffer` on the shade buffer. `encode_frame` still finishes one encoder and submits once. There is no gradient texture and no compute entry.

A neighbour outside the volume, or in another brick, contributes 0. `neighbour_stored` still samples the centre brick when the neighbour step's residency flag is 0. The mutation shows that read is real. With the axis-2 light `z` component changed from `-1.0` to `1.0`, the against-light pixel became the one-step product (assertion text `red 0.3921569 is not within 2 ULP of 0`) and the along-light pixel became 0 (assertion text `red 0 is not within 2 ULP of 0.39215687`). A zero gradient would not have moved either pixel. The non-resident neighbours are in the brick texture. The host command stayed green under that same edit, because those rows do not execute the shader. The shader was restored before this file was written.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s21` working tree | Executed now | HEAD `f6a63709c5ac0f2e172bbee2f22444c255b716f3`. Six modified product paths plus untracked `shade.rs`. Ledgers dirty and in-progress. Nothing staged. |
| `f32` evaluation of C.11.2.1.3.2, the one-step product, and both central differences | stored 100, 200, 300 at centre 127.5 width 255, spacing 1, light `(0, 0, -1)` | Executed now, outside the crate | Display 200 bits `1128792064`. Colour bits `1061734601`. Product bits `1053345993`. Alpha bits `1056964608`. Gradient `+100` bits `1120403456`, dot `-1`, shade 0. Gradient `-100` bits `3267887104`, dot `+1`, shade 1. Zero-gradient colour bits `1044957385`. |
| Read of `shade.rs`, `raycast.wgsl`, `set_shade`, `bind_volume_ray`, `RayHeader`, `encode_frame` | current working tree | Executed now, read | Weights private. Uniform 16 bytes at binding 3. Header assert 48. Opacity binding 2 unchanged. One `queue.submit` in `encode_frame`. No `@compute` and no `var<storage` in the ray module. |
| `bin/ocelli.sh test ocelli-render --test raycast` | working tree before the mutation | Executed now | Exit 0. 10 passed, 10 ignored, 0 failed. |
| `bin/ocelli.sh test ocelli-render --test raycast -- --ignored` | same tree, Auto adapter | Executed now | Exit 0. 10 passed, 0 failed, about 0.84s. Includes the four shade rows and the six older ray rows. |
| Axis-2 light `z` changed from `-1.0` to `1.0`, then both commands above | one constant in `raycast.wgsl` | Executed now | Host exit 0, 10 passed and 10 ignored. Ignored exit 101, 8 passed and 2 failed. Failed rows were the two directional shade tests, with the swapped colours quoted above. Unlit, zero gradient, and the F-060 rows stayed green. |
| Restore of that constant, `shasum -a 256` of `raycast.wgsl` | same path | Executed now | Digest matches the pre-mutation digest. `git diff` again shows `z` as `-1.0`. |
| `bin/ocelli.sh test ocelli-render --lib shade` | restored tree | Executed now | Exit 0. `shade::tests` rows for unlit bytes, a refused weight, and spacing all passed. The filter also matches the letters inside `shader`, so 10 lib tests passed and 102 were filtered out. |
| Native GL, device-loss rebind, column and row walks | not opened | Not executed now | Recorded under limitations. No author receipt was used as an observation. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`ShadeWeights::unlit` stores ambient 1 and diffuse 0 at `shade.rs:36`. `from_weights` returns `NotFinite` when either argument is not finite, and only then checks `0.0..=1.0`, at `shade.rs:52`. Endpoints 0 and 1 are inside the range. `from_weights(0.0, 1.0)` and `from_weights(0.5, 1.0)` are used by the ray tests. Failure returns `Err` and does not build a value. `uniform_bytes` writes ambient at bytes 0 to 3 and diffuse at bytes 4 to 7, and the other eight bytes stay 0. The lib test reads those two lanes back with `from_le_bytes`. `spacing_mm_ok` requires every axis finite and greater than 0. `bind_volume_ray` calls it before creating a texture and returns `RayUnavailable` without replacing an existing ray.

`light_toward_near` returns `(-1, 0, 0)`, `(0, -1, 0)` and `(0, 0, -1)` for axis codes 0, 1 and any other code. `RayAxis::Slice` is code 2. The fixture registers `RayAxis::Slice` and `from_far: false`. The light does not read `from_far`. Near stays the decreasing index when the walk starts at the far end.

`stored_difference` is `(plus - minus) / (2.0 * spacing)` when spacing is greater than 0, and 0 otherwise. The samples are `f32(load_brick(...))`, which is the stored texel, not `lut_chain`. The centre colour is still `lut_chain(stored)` and then `(display - ymin) / span`. `lit` is `colour * shade_of(...)`. Alpha is `opacity_of(table_index(display, 255u))`. The integral adds `(1 - A) * lit * alpha` and `(1 - A) * alpha`. The pipeline blend is unchanged. `voi.wgsl` and `fusion.wgsl` are not in the diff.

`neighbour_stored` returns 0 for a negative voxel, a voxel at or past `dim_x`, `dim_y` or `dim_z`, a zero edge, or a brick coordinate other than the centre brick. Otherwise it loads the centre slot at the local texel. A residency flag of 0 still skips that step's integral before the brick read. It does not remove the texel. The U16 brick edge is 64, and the fixture volume is `1 x 1 x 3`, so all three samples share brick `(0, 0, 0)`. The GPU mutation would have left both directional pixels dark if those neighbours had been read as 0.

`RayHeader` field order matches `RayParams` through `spacing_z`. `ray_uniform_bytes` copies `bytemuck::bytes_of` of that 48-byte prefix and then one `vec4<u32>` per slot. The bind rejects a params buffer whose size is not `48 + slots * 16`, an opacity buffer whose size is not 1024, or a shade buffer whose size is not 16. The volume bind group puts the VOI view at binding 0, the params buffer at 1, the opacity buffer at 2 and the shade buffer at 3. The stack layout is still three uniform bindings. The ray pipeline is the only one given the four-binding layout. Tier A and tier B compile `volume_ray_module`, which is `VOI_WGSL`, `TABLE_INDEX_WGSL` and `raycast.wgsl`. That module text has no `var<storage` and no `@compute`.

`set_shade` writes the 16-byte uniform and does not call `create_buffer`, `write_texture` or `upload_brick`. It does not touch `brick_uploads`, `uploads` or `lut_uploads`. The same bits return `Ok` before the write. A call with no ray binding returns `RayUnavailable` before the registry assignment. `install_recovered` drops the GPU slots and sets dirt. It does not clear `state.shade`. The next `bind_volume_ray` copies that remembered pair into a new buffer. This pass did not run a device loss.

The new LLD sentences match those facts. Flat shade stays ambient 1 and diffuse 0. The normal is the central difference of stored samples. Spacing is the last three header words. The shaded integral is `(1 - A) * colour * shade * alpha` and `(1 - A) * alpha`. `docs/lld/render-graph.md` has no semicolon and no em dash in the file. The README row names the gradient shade and F-061 and has no prose semicolon. No Rust `as` cast, no `unsafe` and no `wasm_bindgen` were added. The product path uses `Result` and does not add `unwrap` or `expect`.

`assert_ray` was not edited. Colour stays within 2 ULP and alpha stays exact bits. That tolerance is older than this story. The directional failure under the flipped light is the whole product against 0, not a 2 ULP miss. The host test `centre_gradient_of_100_200_300_is_one_hundred_per_millimetre` recomputes `(300 - 100) / 2` in the test body. It does not call the shader. The GPU rows are the dot-product proof. The product asserts in that host test are the same multiply written twice. They are not a second proof.

Private fields are a justified lock. A public field would let a caller store a non-finite weight after `from_weights` returned, which is the failure AC4 names. The accessors return the stored weights. This pass does not call the privacy a defect.

## Limitations and handoff

Native GL tier B was not opened. The ray tests call `resolve_adapter` with `TierRequest::Auto`. `resolve_native_gl` forces `Backends::GL` and was not called. The comment on that function says a miss from the gles backend is not a tier B run. This pass did not record the Auto adapter's name. The mutation proves some device executed the fragment shader, because the two directional pixels swapped. It does not prove a GL compile or a GL pixel.

Column and row walks were not drawn. The shader arms for axis 0 and axis 1 were read. A spacing other than 1 was not drawn. On one non-zero axis a positive scale cancels in the normalize, so these pixels do not prove the `2 * spacing` divisor. The divisor was checked by reading `stored_difference`. A neighbour in a second bound brick was not drawn. The code returns 0 when the brick coordinate differs, which is the locked rule, and this pass did not render that case.

These samples do not separate a stored gradient from a display gradient. Stored 300 clamps to display 255 under the fixture window, so the display-space difference is -77.5 or +77.5 rather than -100 or +100, and the unit normal stays on the same side. The shader load is the stored texel. The tests would still pass if that load were replaced by the display value.

The against-light row checks the pre-bind `RayUnavailable` result and then calls `set_shade` again after bind before drawing. The pixel does not by itself prove the refused call left the weights unlit. The source returns before the assignment. Device-loss rebind of the remembered pair was read, not run. `set_shade`'s tier C branch cannot run, because `RenderGraph::new` returns before a CPU graph exists. The reachable refusal is `volume_ray_admission`, which the host test calls.

No wasm-pack build, floor gate, oracle or corpus command was run. The sprint ledgers were not reviewed beyond the in-progress status this pass was told to expect. This pass does not accept the story, mark it done, or commit.
