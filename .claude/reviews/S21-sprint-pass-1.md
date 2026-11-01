# S21 whole-sprint review, pass 1

**Reviewed**: `sprint/s21` at `7d459fbba321237cc0effb11d75d22d0cc07a6c7`. `git rev-parse HEAD^{tree}` is `ed4c54cedf94268e2239996fb669de2d8946ca4a`. `git status --porcelain` was empty before this file. The implementation range is `git diff f7e43e166e6ac0149115a38dba21979e884267f4..HEAD`, commits `f6a63709c5ac0f2e172bbee2f22444c255b716f3`, `640f7b922cffe4ee3ffc283c12be2d80797cdeab` and `7d459fbba321237cc0effb11d75d22d0cc07a6c7`. The design commit `3a86bf0` was not reopened. F-X004 and F-X005 were not treated as done. This file is the only worktree change.
**Author / reviewer**: independent. This reviewer did not write the delta. The per-story reviews were not used as observations.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

An empty clip list still yields the unlit two-step integral. A discarded sample adds neither colour nor alpha. Shade does not rewrite the stored sample.

The fragment loop is one front-to-back walk. A residency flag of 0 continues before the clip test. A kept step loads the stored texel, windows it, multiplies that colour by the shade, and takes opacity from the display. A discarded step continues before the brick load and before either accumulator write.

`ClipList::empty` is six inactive slots. `uniform_bytes` leaves a disabled slot at zero, so `normal.w` is 0. `sample_kept` skips that slot and returns true when every slot is disabled. Registration starts from that list, and `draw_clip` with no planes does not call `set_clip_planes` or `set_shade`. The default weights are ambient 1 and diffuse 0.

The two stored values of 200 are not a zero gradient. The volume is `1 x 1 x 2` with slice spacing 10 mm, origin 0 and an identity direction. Both texels share the one ready brick. The near voxel's z neighbours are stored 200 and the outside value 0, so the central difference is `(200 - 0) / (2 * 10) = 10`. The far voxel is `(0 - 200) / 20 = -10`. The x and y neighbours are outside the volume, so those components are 0. The unit normals are `(0, 0, 1)` and `(0, 0, -1)`. `light_toward_near` for axis 2 is `(0, 0, -1)` at `raycast.wgsl:184`. The dots are -1 and +1. Unlit shade is `1 + 0 * max(0, dot)`, which is 1 on both samples. Ambient 0 and diffuse 1 would zero the near colour and still add its alpha, leaving `(200/255) * 0.25` and alpha 0.75. The unlit two-step pair is `(200/255) * 0.75` and alpha 0.75. `no_planes_keep_both_resident_samples` asserts that pair. It passed in the combined ignored run. The colour channel is what rejects a default diffuse weight. The alpha channel is what rejects a dropped sample. The host row `no_planes_keep_the_two_step_colour` only repeats the `f32` product. It does not draw.

A plane through 5 mm with normal `(0, 0, 1)` sees patient z of 0 and 10. The dots are -5 and +5. The keep test is `dot(delta, normal.xyz) >= 0.0` at `raycast.wgsl:215`. The shader has no `dot(delta, normal.xyz) > 0.0`. The near step is discarded. The far step is the one-step product `(200/255) * 0.5` with alpha 0.5. A discard that added alpha and not colour would leave `(200/255) * 0.25` and alpha 0.75, because the near step is first when `from_far` is false. A discard that added both would leave the two-step pair. `a_plane_keeps_only_the_sample_past_five_millimetres` matched the one-step pair. `a_sample_on_the_plane_stays_in_the_integral` matched the same pair for a dot of 0. This pass did not flip the operator. The host string lock in `a_volume_mip_is_not_the_composite_ray` also requires the `>= 0.0` text. That row passed with the other host tests.

Shade writes no texel. `set_shade` calls `queue.write_buffer` on the 16-byte uniform at `graph.rs:2930` and assigns `state.shade` only after that write. It does not call `write_texture` or `upload_brick`. `set_opacity` and `set_clip_planes` write only their own uniforms. The centre value that enters `lut_chain` is `f32(load_brick(...))`. `lit` is that colour times `shade_of`. `neighbour_stored` reads the brick texel and does not read `lit`. A clip skip does not erase the neighbour. The along-light row stores 300, 200 and 100, with ambient 0 and diffuse 1, and only the centre step resident. The centre difference is `(100 - 300) / 2 = -100`. The normal `(0, 0, -1)` dots the light at +1, so the shade is 1 and the pixel stays the flat one-step product. A centre texel replaced before the window would not display as 200. That row passed. `light_against_the_gradient_zeroes_the_colour` keeps `brick_uploads` across `set_shade` and passed. Its alpha stays 0.5, so the shade is not a second transfer opacity.

`patient_point` is `crop.origin` plus each direction column times that axis spacing times the index, at `raycast.wgsl:200`. `direction_of` copies `Volume::direction.col(0)` through `col(2)`. That is the product in `Volume::voxel_world`. The ray header stays 48 bytes at `graph.rs:778`. Opacity stays binding 2 and 1024 bytes. Shade stays binding 3 and 16 bytes. Clip stays binding 4 and 256 bytes. The stack layout at `graph.rs:1474` is still three bindings. `encode_frame` has one `queue.submit`, at `graph.rs:3467`. The test readback submits its own copy after the frame. `voi.wgsl` and `fusion.wgsl` are not in the sprint diff. Tier C is `TierCUnavailable` from `volume_ray_admission` and from `RenderGraph::new` before a device. The three CPU rows call admission and passed. No new Rust `as` cast, `unsafe` or `wasm_bindgen` is in the sprint diff.

The verify trailers are not a false tree. F-060 names `2bc95639fbda`, F-061 names `f076513306da` and F-062 names `ed4c54cedf94`. `git rev-parse` of those commits is `2bc95639fbda3ee0325430534818de05cbc70339`, `f076513306dafe45b57af29e5603b52dac5396f1` and `ed4c54cedf94268e2239996fb669de2d8946ca4a`. `BACKLOG.md` marks F-060, F-061 and F-062 done. It marks F-X004 and F-X005 pending. F-X003 is `in-progress`. `CURRENT_SPRINT.md` says the same. The AS_BUILT test counts match the functions added in each commit. F-060 added four host ray tests and two ignored GPU tests. F-061 added four host ray tests and four ignored GPU tests. F-062 added six `clip.rs` tests, two host ray tests and four ignored GPU tests. This pass did not re-run those feature gates.

## Evidence

| Check | Identity | Executed now or prior receipt | Observation |
|---|---|---|---|
| `git rev-parse HEAD` and `git rev-parse HEAD^{tree}` | `sprint/s21` | executed now, before this file | commit `7d459fbba321237cc0effb11d75d22d0cc07a6c7`, tree `ed4c54cedf94268e2239996fb669de2d8946ca4a` |
| `git status --porcelain` | worktree | executed now, before this file | empty |
| `git diff --name-only` for `voi.wgsl`, `fusion.wgsl` and section 25.1 | `f7e43e1..HEAD` | executed now | those paths are absent |
| `bin/ocelli.sh test ocelli-render --lib --test raycast` | current tree | executed now | exit 0. Lib 115 passed, 3 ignored, 0 failed. Raycast host 12 passed, 14 ignored, 0 failed |
| `bin/ocelli.sh clippy ocelli-render` | `cargo clippy -p ocelli-render --all-targets -- -D warnings` | executed now | exit 0. No warning text |
| `bin/ocelli.sh test ocelli-render --test raycast -- --ignored --test-threads=1` | Auto adapter, one process | executed now, once | exit 0. 14 passed, 0 failed, 1.92s. Includes the empty-clip two-step row, the one-kept-sample row, the on-plane row and the four shade rows |
| `f32` products outside the crate | stored 200, flat alpha 0.5, spacing 10 | executed now | display bits `1128792064`. Grey bits `1061734601`. One-step bits `1053345993`. Two-step bits `1058444951`. Alpha 0.5 bits `1056964608`. Alpha 0.75 bits `1061158912`. A near sample darkened by diffuse 1 is bits `1044957385` with alpha still 0.75 |
| Shader text | `raycast.wgsl`, not mutated | executed now | one `dot(delta, normal.xyz) >= 0.0`. No `dot(delta, normal.xyz) > 0.0`. Slice light z is `-1.0` |
| Verify trees | the three implementation commits | executed now | trailer prefixes match `git rev-parse COMMIT^{tree}` |
| Native GL, non-zero origin, rotated columns, device-loss rebind | not drawn | not executed now | recorded under limitations |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Limitations

A non-zero origin was not drawn. Rotated direction columns were not drawn. The GPU fixtures use origin 0 and `DMat3::IDENTITY`. The 5 mm plane does use spacing, because voxel 1 as an index of 1 mm would fall on the discarded side. It does not prove the origin add or a non-identity column. `patient_point` and `direction_of` were read against `Volume::voxel_world`.

Native GL was not opened. `session` calls `resolve_adapter` with `TierRequest::Auto`. `resolve_native_gl` was not called. This pass did not record the adapter name. The ignored rows ran once, and a passing test hides stdout, so this command does not print a tier C skip line. A skip would return before `assert_ray` and would also exit 0. The run took 1.92s for 14 rows. The host-only invocation listed those same rows as ignored in 0.00s.

Device-loss rebind was not run. `install_recovered` drops the GPU slots and does not clear `state.opacity`, `state.shade` or `state.clip`. The next `bind_volume_ray` copies those remembered values. That path was read.

A shaded sample beside a clipped neighbour was not drawn. The empty-clip row is unlit, so a gradient change would not move it. The directional shade rows do not set a plane. Column and row walks were not drawn. A spacing other than 1 mm and 10 mm was not drawn. The shader operators were not mutated.

No wasm-pack build, floor gate, oracle or corpus command was run. The wasm byte notes in `AS_BUILT.md` were not remeasured. F-X004 and F-X005 remain pending while F-X003 is in progress.
