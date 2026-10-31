# F-052 review, pass 1

**Reviewed**: HEAD `897c94c29f96944e02bd5e1758091db5baafe0d7`. `git write-tree` is `19ae6e26fec4361b0730c5aa0825ff2da670a464`. The index tree omits this work. The pass reviewed the unstaged working tree.
**Author / reviewer**: This diff has no commit, so no trailer names an author. The reviewer is independent of this implementation.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

AC1 was checked from PS3.3 C.7.6.2.1.1 plus the volume's third term. Identity direction, origin `(10, 20, 30)`, spacing `[0.5, 2.0, 3.0]`, value `x + 10*y + 100*z`.

| Plane | Pixel | Voxel | Value | World |
|-------|-------|-------|-------|-------|
| Axial slice 0 | `(1, 1)` | `(1, 1, 0)` | 11 | `(10.5, 22, 30)` |
| Sagittal column 1 | `(0, 0)` | `(1, 0, 0)` | 1 | `(10.5, 20, 30)` |
| Sagittal column 1 | `(1, 0)` | `(1, 1, 0)` | 11 | `(10.5, 22, 30)` |
| Sagittal column 0.5 | `(0, 0)` | blend of `(0, 0, 0)` and `(1, 0, 0)` | 0.5 | `(10.25, 20, 30)` |
| Coronal row 0 | `(1, 1)` | `(1, 0, 1)` | 101 | `(10.5, 20, 33)` |

The plan says a sagittal tuple `(a, i, b)` yields 10 at column 1 pixel `(1, 0)`. That product is 11. Pixel `(1, 0)` under `(a, i, b)` is voxel `(1, 1, 0)`, the same voxel as `(i, a, b)`. The pixel that separates the swap is `(0, 0)`. Correct is voxel `(1, 0, 0)`, value 1. Swapped is voxel `(0, 1, 0)`, value 10. The plan table already lists those two pixels.

The host test rejects the swap. `voxel_of` was changed from `(column, along_a, along_b)` to `(along_a, column, along_b)` and restored in a `finally` block. `ortho_planes_follow_the_volume_axes` panicked at `crates/ocelli-volume/tests/ortho.rs:40`. That line is the assert on sagittal pixel `(0, 0)`. Left bits were 1092616192, which are the bits of 10.0. Right bits were 1065353216, which are the bits of 1.0. Exit 101. The restored SHA-256 is `af33beb6506e44a86189253468dee10a68bd4833ed270d9b7e9c0819406acc52`.

Under that swap, pixel `(1, 0)` stays 11. The assert against 11 holds. The assert against 10 holds. Those lines reject a different walk, the one that reads `(0, a, b)` and returns 10 at pixel `(1, 0)`. The test comment describes that walk.

A throwaway binary outside the repo called the public API. Exit 0. Both slices were present. A swapped per-axis weight fails each fractional row.

| Case | Observed | Swapped-weight value |
|------|----------|----------------------|
| Axial slice 0.25 pixel `(0, 0)` | 25 | 75 |
| Axial slice 0.25 pixel `(1, 1)` | 36 | 86 |
| Sagittal column 0.25 pixel `(0, 0)` | 0.25 | 0.75 |
| Coronal row 0.25 pixel `(0, 0)` | 2.5 | 7.5 |
| `voxel_world([0.25, 0, 0])` | `(10.125, 20, 30)` | a half-voxel origin moves this point |
| Columns `(0,1,0)`, `(0,0,1)`, `(1,0,0)`, index `(1, 1, 0)` | world `(10, 20.5, 32)`, sagittal pixel `(0, 0)` is 1 | a direction relabel would change the sample |

U16 bytes `FD FF` decode as 65533. An on-grid F32 value 0.1 round-trips bit for bit. NaN and `+inf` on a sagittal column return `PlaneOutOfRange` on axis 0. Axial slice `1 + epsilon` returns axis 2. Coronal row 2.0 returns axis 1. Coronal row 1 reads `[10, 11, 110, 111]`, which is `(a, j, b)`. `append_level([1, 1, 1], [7])` left axial slice 0 unchanged. With slice 1 absent, axial slice 0 stays `[0, 1, 10, 11]`, axial slice 0.5 is all `Absent`, and sagittal column 1 pixel `(0, 1)` is `Absent`.

AC2 names sagittal column 0.5 pixel `(0, 0.5)`. That point is voxel `(0.5, 0, 0.5)`. `reslice` emits integer pixels, so an `OrthoImage` has no such pixel. The public test already refuses a non-zero weight on an absent slice, by axial slice 0.5, and it keeps sagittal column 0.5 pixel `(0, 0)` at 0.5 while slice 1 is absent. The library test calls `sample_at(0.5, 0.0, 0.5)` and expects `Absent`. That test passed. One sampler serves both calls. The named half-pixel is covered.

AC5 and AC6 ran here. `axial_upload_matches_the_lut_chain` and `an_absent_plane_does_not_replace_the_texture` passed. Exit 0. The ignored pair finished in 1.83s with `--test-threads=1`. The bound is `1e-4` and the compare is `<`, the same constant and operator as `SWEEP_TOLERANCE` in `crates/ocelli-render/tests/voi_shader.rs`. The fragment shader writes `(display - ymin) / (ymax - ymin)`. The test multiplies the R channel by 255, which is the recovery `read_displays` already uses. `upload_stored` calls `accepts` before `uploads += 1`. A wrong length is `WrongLength` and the counter stays. That order was read. A bad-length upload stayed off the device.

AC7. `RenderGraph::new` returns `TierCUnavailable` when the context tier is `Cpu`, so a tier C graph is never constructed. `present_plane` returns `TierCUnavailable` before the sample loop and before the `Vec`. `ortho_viewport_is_unavailable_on_tier_c` passed. An absent image on tier C also returned `TierCUnavailable`, and the upload closure stayed at 0 calls.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test ortho` | working tree, four host tests | Executed now | Exit 0. 4 passed |
| `bin/ocelli.sh test ocelli-volume -- ortho` | library test `a_fractional_sagittal_sample_does_not_blend_an_absent_slice` | Executed now | Exit 0. 1 passed |
| `bin/ocelli.sh test ocelli-render --test ortho` | host `present_plane` tests | Executed now | Exit 0. 3 passed, 2 ignored |
| Sagittal tuple swapped to `(along_a, column, along_b)`, then `bin/ocelli.sh test ocelli-volume --test ortho ortho_planes_follow_the_volume_axes -- --exact` | `crates/ocelli-volume/src/ortho.rs` `voxel_of`, restored in `finally` | Executed now | Exit 101. Panic at `tests/ortho.rs:40`. Left 1092616192, right 1065353216. Restored hash matches |
| `/tmp/f052-probe` against `ocelli-volume` | fixture plus fractional weights, rotated direction, U16, F32, range, `append_level` | Executed now | Exit 0. Values in the acceptance section |
| `/tmp/f052-narrow` copy of `floor_nonneg` and `narrow_f32` | compared with `f64::floor` and `as f32` | Executed now | Exit 0. 408191 samples, 0 floor mismatches, 0 narrow mismatches, 0 round-trip mismatches, 0 index mismatches |
| `bin/ocelli.sh test ocelli-render --test ortho -- --ignored --test-threads=1` | AC5 axial and sagittal readback, AC6 refusal | Executed now. Author receipt was a prior 1.33s pass | Exit 0. 2 passed in 1.83s |
| `bin/ocelli.sh clippy ocelli-volume` and `bin/ocelli.sh clippy ocelli-render` | crate sources, `-D warnings` | Executed now | Exit 0 and exit 0 |
| Environment | arm64, rustc 1.97.1 `(8bab26f4f 2026-07-14)` | Executed now | recorded |

End SHA-256 of the seven working files matches the hashes recorded at the start. `git write-tree` is still `19ae6e26fec4361b0730c5aa0825ff2da670a464`.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, a reslice error ends the ignored GPU tests as success

**Where**: `crates/ocelli-render/tests/ortho.rs:308`, and the same binding at lines 327, 372, and 389.
**What**: `let Ok(image) = image else { return }` finishes the test successfully when `reslice` returns `Err`. Graph construction in those functions asserts `is_ok` before the same pattern.
**Why it stays a nitpick**: A wrong sample still fails `assert_within_bound`. The host suite fails when this fixture's `reslice` returns `Err`. The axis-swap mutation went red on the host test.

## Verified clean

`voxel_of` matches the plan table. Axial is `(a, b, k)`. Sagittal is `(i, a, b)`. Coronal is `(a, j, b)`. Sagittal `column_mm` is `spacing[1]` and `row_mm` is `spacing[2]`. The host test checks those bits.

`voxel_world` is origin plus `i * spacing[0] * col(0)` plus `j * spacing[1] * col(1)` plus `k * spacing[2] * col(2)`. `col(0)` is the row cosine. `spacing[0]` scales that cosine. That is PS3.3 C.7.6.2.1.1. The origin term is the centre of voxel `(0, 0, 0)`.

`in_range` is the closed interval `[0, dim - 1]` and requires a finite index. `PlaneOutOfRange` returns before the sample `Vec` exists. The fixed axis at `dim - 1` has fraction 0, so the neighbour past the volume has weight 0 and is skipped.

Weights are `1 - frac` and `frac` in `f64`. A zero component is skipped before `present.test` and before `container_at`. Decode is little-endian through one function. `I16` uses `from_le_bytes`. There is no second Bits Stored mask.

Level 0 is the store prefix. `slice(z)` uses the store dimensions. An appended coarse level sits after that prefix. The probe left axial slice 0 unchanged after `append_level`.

`Pass` gained no variant. There is no new WGSL sampler and no second LUT. `show_ortho` uploads `Present` `f32` values through `upload_stored`. An `Absent` sample returns `PlaneIncomplete` before that call. Tier C returns before the upload. Tier A and tier B share the upload. `RenderGraph::new` still refuses tier C.

`tick` is unchanged. `show_ortho` is a command. `tick` never calls it. The GPU refusal left the following `tick` at `Noop`.

The new ortho modules contain no `unsafe`, no `expect`, no `unwrap`, and no `as` cast. `narrow_f32` and `rounded_shift` match `f32_from_f64` and its helper in `crates/ocelli-viewport/src/view.rs` after the rename. The throwaway sweep round-trips on-grid container values.

## Limitations and handoff

Wasm size is unmeasured. This story adds no wasm export. The full floor gate is unrun this pass. `unsafe` was checked by search of the new sources.

This adapter selected its usual stored format. A forced tier B wider float format was outside this run. The samples still enter the existing `upload_stored` choice.

A machine with no adapter returns success from the ignored tests after a printed line. This pass had an adapter. Both ignored tests executed their asserts. One GPU process ran.

Oblique planes, thick slab, MIP, and a tier C framebuffer stay out of scope. dwv, Horos, and Grok stayed closed.

The next owner can clear N1 by asserting `reslice` before the early return. That edit needs a fresh pass. This pass found no blocking item.
