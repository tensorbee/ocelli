# F-060 review, pass 1

**Reviewed**: working tree on `sprint/s21` at `3a86bf08291e8ce6f8fd55aa0e5a5cdc68adb605`. Unstaged product paths are `crates/ocelli-render/shaders/raycast.wgsl`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/tests/raycast.rs`, `docs/lld/README.md` and `docs/lld/render-graph.md`, plus untracked `crates/ocelli-render/src/opacity.rs`. This review file is not part of that product diff. Nothing was staged. Nothing was committed.
**Author / reviewer**: the implementation agent wrote the opacity table, the uniform, the shader load and the LLD edit. The reviewer is an independent microscope subagent in a separate context and did not write that code. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 and AC2, one resident step, stored 200, `LINEAR_EXACT` centre 127.5 width 255, output 0 to 255. PS3.3 C.11.2.1.3.2 puts the lower clamp at `c - w/2 = 0` and the upper clamp at `c + w/2 = 255`, with `<=` on the low side and `>` on the high side. Stored 200 is inside that open-closed body. Evaluated in `f32`, the display is exactly 200.0 (bits `1128792064`). The grey colour is `200/255` (bits `1061734601`). Flat multiplies by 0.5, so colour bits are `1053345993` and alpha bits are `1056964608`. Ramp multiplies by `200/255`, so colour bits are `1058896472` and alpha bits are `1061734601`. That is the plan's one-step pair. It rejects a flat constant other than 0.5 and a ramp that leaves the F-059 product in place.

The wrong address is the normalised colour, not the display. `table_index` of `200/255` is 1, and ramp entry 1 is `1/255`. The shader calls `table_index(display, 255u)` on the `lut_chain` result, then normalises that same display into colour. The ray module does not sample the 1024-wide grey ramp. `volume_ray_module` is `VOI_WGSL`, `TABLE_INDEX_WGSL` and `raycast.wgsl` only.

AC3. `from_entries` returns `Length`, `NotFinite` or `OutOfRange` and does not return a table. A caller that still holds the previous `OpacityTable` cannot have it replaced by that `Err`. The host test poisons index 0. The loop checks every index. Finiteness is tested before the range, so a NaN is `NotFinite` rather than `OutOfRange`.

AC4. `set_opacity` calls `queue.write_buffer` on the buffer created in `bind_volume_ray`. It does not call `create_buffer`, `write_texture` or `upload_brick`, and it does not touch `brick_uploads`, `uploads` or `lut_uploads`. Equal bits return before the write.

AC5. Tier C still has no ray image. `volume_ray_admission` and `RenderGraph::new` return `TierCUnavailable` before a device. The ray shader has no CPU twin.

Unchanged F-059 expectations still assume flat 0.5. `draw_ray` passes no table, registration stores `OpacityPreset::Flat`, and bind uploads that table. Two flat steps of stored 0 then stored 200 still produce colour `(200/255)*0.5*0.5` (bits `1044957385`) and alpha 0.75 (bits `1061158912`). Fusion opacity, the colour LUT and `RayHeader` are outside the diff.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff HEAD` | `sprint/s21` working tree | Executed now | HEAD `3a86bf08291e8ce6f8fd55aa0e5a5cdc68adb605`. Six modified paths and untracked `opacity.rs`. Designs only at HEAD. |
| `f32` evaluation of C.11.2.1.3.2 and the one-step products | stored 0, 100, 200, 255 at centre 127.5 width 255 | Executed now, outside the crate | Display 200 is exact. Flat and ramp products match the bits above. Author's 0.5 bits `1056964608` and 0.25 bits `1048576000` match. |
| Read of `opacity.rs`, `raycast.wgsl`, `set_opacity`, `bind_volume_ray`, `RayHeader` | current working tree | Executed now | Table length 256. Flat is 0.5. Ramp entry `i` is `f32(i)/255`. Uniform is 1024 bytes. `RayHeader` is 12 `u32` fields and the compile-time assert stays 48. |
| Packing map for entries 0, 1, 2, 3, 4, 200, 201, 255 | `uniform_bytes` and `opacity_of` | Executed now | Entry `i` is bytes `4*i`, vec `i/4`, component `i%4` in `xyzw` order. Entry 200 is vec 50 `.x` at byte 800. Entry 255 is vec 63 `.w` at byte 1020. |
| `table_index` against `display_index` | `graph.rs` 699-717 and `colour.rs` 433-460 | Executed now, read | Same clamps. Non-finite and `<= 0` use 0. `>= last` uses `last`. Between, add 0.5 and truncate toward zero. At last 255 the integers are exact in `f32`. |
| naga 30.0.1 GLSL uniform writer | pinned `naga-30.0.1/src/back/glsl/writer.rs` | Executed now, read only | Uniforms are emitted as `std140` interface blocks. The backend does not reject a dynamically indexed array inside that block. |
| Host and ignored GPU tests, clippy, `gate unsafe`, wasm | author receipt | Not executed now | Not used as this pass's observation. A wasm-pack build was already running and was not started again. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`OpacityTable::preset(Flat)` is `[0.5; 256]` at `opacity.rs:47`. `ramp_entries` writes `f32::from(i) / 255.0` for every index 0..=255. `from_entries` refuses a length other than 256, then each value that is not finite, then each finite value outside `0.0..=1.0`. Endpoints 0 and 1 are inside the range. Failure returns `Err` and builds no value, so the previous table object stays where the caller left it. `set_opacity` takes `&OpacityTable` only, so a refused slice never becomes the uniform.

The shader index is `opacity_of(table_index(display, 255u))` at `raycast.wgsl:143`. `display` is `lut_chain(stored)`, which is the section 18.4 window output, not the normalised colour computed on the next lines. `opacity_of` clamps a negative index to lane 0 and a lane above 255 to 255, then loads `values[lane / 4u]` and returns `.x`, `.y`, `.z` or `.w`. That is the `std140` and WGSL `vec4` order. The host helper `uniform_bytes` stores entry `i` at little-endian bytes `4*i`. The buffer size checked after bind is 1024. `array<vec4<f32>, 64>` is the stride-16 packing the plan required. It is not `array<f32, 256>`, which `std140` would pad to 16 bytes per element.

`bind_volume_ray` creates one opacity buffer with `UNIFORM | COPY_DST` and places it at group 0 binding 2 of `volume_uniform_layout`. `set_opacity` writes that same buffer. A bit-identical table returns `Ok` before `write_buffer` and does not set dirt again. The encode path still binds `ray.bind_uniform` and still submits once. No new buffer is created inside `tick`. The 1024-byte array lives on the stack in the setter, which is the same class of uniform write as `set_voi`.

`RayHeader` was not given a field. The ray params stay binding 1. The stack layout still has the camera at its own group 0 binding 2. `tier_b_fragment_module` still appends `GRAPH_WGSL` and does not append `raycast.wgsl`, so the two binding-2 uniforms are not in one shader. `voi.wgsl`, `lut_chain`, `colour.rs` and `fusion.rs` are not in the diff. The volume module text still has no `var<storage` and no `@compute`. Tier A and tier B compile that one fragment module. There is no opacity texture.

The new LLD sentences match those facts. Flat stays 0.5. Ramp is `i / 255` as a real quotient. The integral is `(1 - A) * colour * alpha` and `(1 - A) * alpha`. `docs/lld/render-graph.md` and the README row have no em dash and no prose semicolon. `RenderGraph::new`, admission and the `set_opacity` source all name `TierCUnavailable` for tier C. No `unsafe`, no Rust `as` cast, and no `unwrap` or `expect` were added on the product path. The WGSL `u32(index)` runs only when `index > 0`, and `table_index` for last 255 returns 0..=255.

`set_opacity`'s tier C branch cannot run. `RenderGraph::new` returns before a graph exists when the context tier is CPU. `install_recovered` copies `recovered.caps.tier`, and `Recovered` documents that those caps are the adapter caps from before the loss. A graph that exists was opened on a device, and CPU does not open one. The branch is therefore unreachable. It is not a defect and not a smell. The reachable refusals already produce no image and no table write, and recovery cannot move an existing graph onto tier C. `cpu_opacity_is_unavailable_before_a_sample` calls `volume_ray_admission` only. That function is reachable and is the same check `volume_raycast_has_no_cpu_image` already makes. Removing the unreachable branch inside `set_opacity` would not change any call a caller can make today.

Native GL tier B was not opened. That is a recorded limit, not a defect. The new load dynamically indexes a uniform array. The F-059 ray already does that for `ray.coords[i]` in the same module. The approved plan chooses one fragment module and a uniform, not a texture. naga 30.0.1 will emit the block as `std140`. This pass has no GL compile or pixel result, so tier B on GL is not claimed as executed evidence.

The flat host assertion compares entry 200 with `0.5_f32.to_bits()`. A table of 0.25 would fail that comparison with the bits the author reported. This pass did not mutate the constant or re-run the test. The following product assert is the same multiply written twice once the alpha assert has passed. It is not an independent colour proof. The colour proof is the `f32` evaluation above and, on a device, the readback. The ramp host assert that entry 200 equals `200/255` is the discriminating half of that test. Its product assert is the same tautology.

## Limitations and handoff

No crate test, clippy, wasm-pack, gate, oracle or corpus command was run. The author's host exit 0, the six ignored GPU passes, the flat-to-0.25 mutation, clippy and `gate unsafe` stay that agent's receipts. They are not measurements taken on this pass. Ignored GPU tests are not executed GPU evidence here, including the claim that dynamic indexing compiled on this machine's adapter.

Entry 200 is component `.x`. No test reads a ramp entry whose component is `.y`, `.z` or `.w`. The mapping for those lanes was checked by layout, not by a rendered pixel. `from_entries` is not uploaded through `set_opacity` in a test. The bad-entry fixture poisons index 0 only. Device-loss rebind of the remembered table was read, not run. `uploads` and `lut_uploads` are unchanged by reading `set_opacity`, and the GPU test asserts `brick_uploads` only.

The unchanged two-brick expectation still uses alpha 0.75. That alpha needs two flat contributions. Its colour equals one flat step of stored 200 because `(100/255)*0.5 + 0.5*((200/255)*0.5)` is the same `f32` as `(200/255)*0.5`. Colour alone does not separate those frames. That identity is older than this story.

This pass does not accept the story, mark it done, or commit. Completion still needs a later verify step. The running wasm build was left untouched.
