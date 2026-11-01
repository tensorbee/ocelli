# F-053 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s20`, base `3f9954df100c5ff1a2e6982fa9b9873f4560772c`. Nothing is staged. `git status --short` lists `M crates/ocelli-volume/src/lib.rs`, `M crates/ocelli-volume/src/ortho.rs`, `?? crates/ocelli-volume/tests/oblique.rs`, and `?? crates/ocelli-render/tests/oblique.rs`. No other product path is modified. `docs/lld/volumes.md` is not in the diff.
**Author / reviewer**: implementation by grok in the parent session. This pass is a separate reviewer subagent that did not write the diff. Independence is limited to that split. It is not a second organisation.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC4 were recomputed in Python from the eight stored bytes. The script does not call the Rust sampler. Index `(0.5, 0.5, 0.5)` weights every corner by `1/8`. `760 / 8 = 95`. The y = 0 face is 70. The y = 1 face is 120. Filling the absent slice with stored zero is 25. Renormalising the four present corners is 50. Index `(0, 0, 0.5)` is 60. Index `(0.5, 0, 0)` is 20. Changing 30 to 38 makes 96. The `f32` bits of 95 are 1119748096. The bits of 70 are 1116471296. The bits of 60 are 1114636288. The bits of 90 are 1119092736.

The checked-in mix test locks 95 against 70. The row index in `reslice_oblique` was forced to `0.0` and the file was restored afterwards.

```rust
let j = 0.0;
```

`bin/ocelli.sh test ocelli-volume --test oblique` then exited 101. `half_voxel_oblique_is_the_trilinear_mix` panicked at `crates/ocelli-volume/tests/oblique.rs:33`. Left bits were 1116471296. Right bits were 1119748096. `named_planes_stay_ortho` panicked at line 108. The samples were 10, 30, 10, 30 against 10, 30, 70, 90. Five tests still passed. `direction_moves_the_patient_point` is one of them, because that fixture's row index is already 0. The restored `ortho.rs` hash is `81af0de565a78e2c0bd6c42e3d53ad27b9e823ba67ca054047757d11e030570b`, the same hash as before the edit. The same command then exited 0. `git diff` does not contain the forced zero.

The checked-in direction fixture uses volume spacing `(1, 1, 1)`, so deleting the three divisors is invisible there. This pass derived another point. Spacing is `(2, 4, 8)`. Origin is `(10, 20, 30)`. Direction columns are `(0, 1, 0)`, `(0, 0, 1)`, and `(1, 0, 0)`. Index `(0, 0, 0.5)` is patient `(14, 20, 30)`, because half of the 8 mm normal step is 4 mm along X. The corners are still 10 and 110, so the sample is 60. Index `(1, 0, 0)` is patient `(10, 22, 30)` and value 30. Using the patient delta as an index, or dividing that delta by spacing without the columns, does not land on those indices.

The same spacing and origin with an identity direction put index `(0.5, 0.5, 0.5)` at patient `(11, 22, 34)`. The mix is still 95. Index `(0.25, 0.5, 0.75)` is 112.5. That is not 95 and it is not a nearest corner. Python evaluated the eight weights. The products are dyadic, and 112.5 is the sum.

`/tmp/f053-probe` called `Volume::reslice_oblique` and `Volume::voxel_world`. It is not in the repo. Exit 0. It observed `Present(95)`, `Present(60)`, `Present(30)`, and `Present(112.5)` at those patient points. `voxel_world` matched `(11, 22, 34)`, `(14, 20, 30)`, and `(10, 22, 30)`. Slice 1 absent at the scaled centre was `Absent`. Patient `(-0.5, 0, 0)` returned `Ok` and `Absent`, not `Present(10)`. Plane row `(0, 2, 0)` with `column_mm` 0.5 put pixel `(1, 0)` at 70. The renormalised row would blend 10 and 70 into 40. An `I16` voxel `FD FF` was `Present(-3)`. `append_level([1, 1, 1], [255])` left the centre at 95. A NaN origin was `VolumeError::ObliquePlane`, not `PlaneOutOfRange`. The image already held stayed `Present(95)`. The next valid call was `Present(95)`. A second `Ok` stayed 95 after the first buffer was replaced with `Absent`. A zero volume axis, an infinite `column_mm`, and a negative volume spacing were `ObliquePlane`.

A non-unit volume column `(2, 0, 0)` makes `voxel_world([0.5, 0, 0])` equal `(1, 0, 0)`. The plan's dot then recovers index 2, and the sample is `Absent`. A true inverse would be `Present(20)`. Renormalising that column would be `Present(30)`. The probe observed `Absent`. That is the approved formula, not a round trip. Assembly is a different path. `ImageOrientationPatient::new` normalises the cosines, and `slice_normal` normalises the cross product, before `Volume::assemble` stores the columns.

The three divisors were then deleted, leaving only the dots. The seven checked-in tests stayed green. Exit 0. The edit was reverted to the same hash. The probe is what rejects a dropped divisor. The checked-in suite does not, because every fixture volume uses spacing `(1, 1, 1)`.

AC5 and AC6 ran in the suite. Axial slice 0 and the matching oblique plane are 10, 30, 70, 90. The non-unit row `(2, 0, 0)` with `column_mm` 0.5 is 10 then 30, not 20. AC7's listed refusals are `VolumeError::ObliquePlane`. The source returns on that path before `Vec::with_capacity`. This pass did not install an allocator counter.

AC8. `present_plane` returns `TierCUnavailable` at `crates/ocelli-render/src/ortho.rs:39` before it reads samples. `oblique_display_reuses_present_plane` kept the closure at 0 calls for the mixed image and for a one-pixel `Present(95)` image on tier C. Tier A of that one-pixel image received the bits of 95. Tier A of the mixed image was `PlaneIncomplete` with 0 calls. Exit 0. `show_ortho` is not in the diff. It still calls `present_plane` at `crates/ocelli-render/src/graph.rs:2493`. No adapter was opened.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Python trilinear on bytes 10, 30, 70, 90, 110, 130, 150, 170 | No Rust sampler. Centre, faces, index `(0, 0, 0.5)`, index `(0.25, 0.5, 0.75)`, scaled patient points | Executed now | Exit 0. 95, 70, 120, 60, 20, 96, 25, 50, 112.5. Patient points `(11, 22, 34)`, `(14, 20, 30)`, `(10, 22, 30)` |
| `bin/ocelli.sh test ocelli-volume --test oblique` | Working tree before mutation. Seven host tests | Executed now. Author receipt was a prior exit 0 | Exit 0. 7 passed |
| `bin/ocelli.sh test ocelli-render --test oblique` | `present_plane` only. No adapter | Executed now. Author receipt was a prior exit 0 | Exit 0. 1 passed |
| `/tmp/f053-probe` | Public `reslice_oblique` and `voxel_world`. Unequal spacing, permuted columns, weights 112.5, absence, refusal, `I16`, coarser level | Executed now | Exit 0. Values in the acceptance section |
| `j` forced to `0.0`, then the volume oblique test | `crates/ocelli-volume/src/ortho.rs` index solve. Restored after the run | Executed now | Exit 101. Panic at `tests/oblique.rs:33`. Bits 1116471296 against 1119748096. Named plane also failed |
| Divisors removed, then the volume oblique test | Same three index lines. Restored after the run | Executed now | Exit 0. 7 passed. Suite does not see spacing `(1, 1, 1)` |
| `bin/ocelli.sh test ocelli-volume --test oblique` | Restored `ortho.rs` hash `81af0de565a78e2c0bd6c42e3d53ad27b9e823ba67ca054047757d11e030570b` | Executed now | Exit 0. 7 passed. Diff has no forced zero |
| Environment | arm64, rustc 1.97.1 `(8bab26f4f 2026-07-14)` | Executed now | Recorded |

Author receipts for full `ocelli-volume`, clippy, and `gate unsafe` were not re-run. The source diff has no `unsafe`, no `unwrap`, no `expect`, no `normalize`, and no `as` cast.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`patient_point` is `origin + c * column_mm * row + r * row_mm * column`. That is PS3.3 C.7.6.2.1.1 with this plane's names. `column_mm` multiplies the row cosine. `row_mm` multiplies the column cosine. The vectors are multiplied as given. `ortho.rs` contains no `normalize` call. The output copies `columns`, `rows`, `column_mm`, and `row_mm`. The loop is column inside row, so samples are x-fastest.

The index is the plan's inverse, not a second patient frame.

```rust
i = dot(direction.col(0), delta) / spacing[0]
j = dot(direction.col(1), delta) / spacing[1]
k = dot(direction.col(2), delta) / spacing[2]
```

`delta` is the patient point minus the volume origin. Each pixel is one `sample_at` call. There is no second copy of the trilinear weights. An outside corner or an absent contributing slice stays `Absent`, including the low side and a scaled volume. A zero-weight slice is not read. The integer origin stays `Present(10)` when slice 1 is clear. Level 0 is the sample source. An appended coarser byte did not move the centre.

Validation in `oblique_pixel_count` runs before `Vec::with_capacity`. A bad plane is `VolumeError::ObliquePlane` and carries no image. `PlaneOutOfRange` is still only the `OrthoPlane` fixed-index error. `reslice` is unchanged. `ObliquePlane` is exported beside `OrthoImage`, `OrthoPlane`, and `PlaneSample`.

`show_ortho`, the VOI shader, and `ocelli-wasm` are not edited. There is no new pass and no second upload. Tier C does not draw. Tier A and tier B share `present_plane`, which special-cases only `Tier::Cpu`. A refused display does not call the upload closure. A held `OrthoImage` survives a later refusal, and the volume bytes are not written.

No new trait, generic, or `Box<dyn>` was added. The new production lines do not use `unsafe`, `unwrap`, `expect`, or an `as` cast.

## Limitations and handoff

This is a bounded code review, not a story close. The plan says `docs/lld/volumes.md` gains `ObliquePlane`, `reslice_oblique`, the patient-point formula, the `Absent` rule, the validation refusal, and the statement that axial, sagittal and coronal stay `OrthoPlane`. That file is unchanged. The orthographic section at line 134 still describes only `Volume::reslice`.

The checked-in suite does not lock the volume-spacing divisor. Deleting it left exit 0. Real assembled spacing is column millimetres, row millimetres, and the median projected gap, not `(1, 1, 1)`. The `/tmp` probe is the check that was run. It is not a regression test in the tree. A later pass that only reruns `tests/oblique.rs` will not repeat it.

`columns` times `rows` uses `checked_mul`. On this 64-bit host every pair of `u32` values fits in `usize`, so the overflow refusal was not executed. A non-orthonormal volume direction does not round-trip, which is the formula the plan approved for the orthonormal matrix assembly stores. The error path's "nothing allocated" claim was read from the early return. It was not measured with a counting allocator. No GPU adapter was opened, and `show_ortho` itself was not called. Clippy, `gate unsafe`, and the rest of the `ocelli-volume` suite were not re-run in this pass.

The only file this pass adds is `.claude/reviews/F-053-oblique-pass-1.md`.
