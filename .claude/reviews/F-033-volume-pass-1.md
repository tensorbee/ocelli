# F-033 review, pass 1

**Reviewed**: working tree diff for F-033 in `crates/ocelli-volume/src/lib.rs`, `crates/ocelli-volume/Cargo.toml`, `crates/ocelli-volume/tests/assembly.rs`, `crates/ocelli-pixel/src/image_plane.rs` and the F-033 row of `docs/sprints/BACKLOG.md`
**Result**: 4 defects, 6 smells, 2 nitpicks

## Defects

### D1, Signed 8-bit and 32-bit integer frames are relabelled as U8 and F32
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:499)
**What**: `describe` matches only on `bits_allocated` and on signedness at 16 bits. An 8-bit signed frame assembles with `VoxelKind::U8` and a 32-bit unsigned or signed integer frame assembles with `VoxelKind::F32`. The volume then tells every later reader the wrong sample kind.
**Why it is wrong**: HLD section 19 gives `VoxelKind` as `U8 | I16 | U16 | F32`. A kind the set cannot name must refuse. It must not be renamed to the kind with the same byte width. `ocelli-pixel` treats 32-bit containers as integers in `StoredPixelDescription::unpack` and tests them as integers, so 32 does not mean float in this system.
**Evidence**: A temporary probe built two slices with `StoredBits::new(8, 8, 7, PixelRepresentation::Signed)` and two with `StoredBits::new(32, 32, 31, PixelRepresentation::Unsigned)`. Both assembled. The probe asserted the observed kinds `VoxelKind::U8` and `VoxelKind::F32` and passed. The probe file was deleted after the run.

### D2, Assembly accepts slices whose in-plane evidence disagrees and takes spacing from the first arrival
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:363)
**What**: The per-slice loop checks orientation plus the dimensions and kind from `describe`. It never compares `PixelSpacing` across slices and never reads the sample layout. `spacing[0]` and `spacing[1]` come from `first_plane`, which is the first slice in arrival order. A colour frame with three samples per pixel assembles into a store whose slice length assumes one sample per pixel, so its own frames can never be placed.
**Why it is wrong**: The plan refuses what cannot be a volume rather than averaging or assuming. A volume has one spacing and one slice length, so evidence that disagrees on either is not one volume. Taking the first arrival value makes the result depend on delivery order.
**Evidence**: A temporary probe assembled slices spaced `[1.0, 1.0]` and `[2.0, 2.0]` and observed `spacing` of `[1.0, 1.0, 3.0]` with no error. A second probe assembled interleaved RGB slices of 72 bytes and observed `place` refuse them with `SliceSizeMismatch` of expected 24 and actual 72. Both probes passed on the observed behaviour and were deleted after the run.

### D3, No assembled fixture separates projected position from the IPP z component
**Where**: [crates/ocelli-volume/tests/assembly.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/tests/assembly.rs:41)
**What**: Every assembled fixture uses row cosine `[1.0, 0.0, 0.0]` and column cosine `[0.0, 1.0, 0.0]`, where the slice normal is the z axis and projected position equals the IPP z component. The one coronal fixture is only grouped and never assembled. Sorting or spacing derived from the z component alone would pass the whole suite.
**Why it is wrong**: The microscope rule is that a test which passes both ways is a defect. PS3.3 C.7.6.2.1.1 spacing and sorting come from the projection on `X cross Y`, and the z shortcut is exactly the regression this story exists to prevent on sagittal, coronal and oblique series.
**Evidence**: The projection at lib.rs line 382 was mutated to `plane.position_vector().z` with no other change. `bin/ocelli.sh test ocelli-volume` stayed green with 1 unit test and 9 assembly tests passing. The mutation was reverted and the file hash returned to `34c4558abcc3da218418f6c15b9359b81c4c8cea202e04deacad32f7006dc35f`.

### D4, The `NotOneVolume` claim about frames of reference is not performed by `assemble`
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:154)
**What**: The variant doc says mixed orientations or frames of reference reached assembly. `assemble` compares only `plane.orientation` against `candidate.orientation` and never reads `candidate.frame_of_reference`. `SliceEvidence` and `DecodedFrame` carry no frame of reference value to compare against, so no input to `assemble` can trigger a frame of reference refusal.
**Why it is wrong**: A claim in prose that is false is a defect under the microscope severity table. Grouping in `group_volumes` does enforce the frame of reference, but that is a different function from the one the claim describes.
**Evidence**: Read of the `assemble` body at lines 359 to 457 found no use of `frame_of_reference`. The grouping test at assembly.rs line 224 covers two orientations under one frame of reference and no mixed frame case exists.

## Smells

### S1, `VolumeCache`, `VolumeKey` and `Budgeted::bytes` have no committed test
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:536)
**What**: No committed test names `VolumeCache`, `VolumeKey` or `bytes`. Half of the story title is a cache and its tier, budget and displacement behaviour could regress under a green suite.
**Evidence**: The imports at assembly.rs lines 13 to 16 name none of them. A temporary probe inserted, read and removed a 72-byte volume in a `VolumeCache` on `CacheTier::Decoded` and observed correct behaviour and `bytes()` of 72. Correct today and unguarded tomorrow is a smell. The probe was deleted after the run.

### S2, `SlicePresence` is never exercised past one 64-bit word
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:200)
**What**: Committed fixtures assemble at most five slices, so word index 1, bit 64 and the count cap at the slice total have no test. The word boundary is the one place this hand-rolled set under deviation D-25 can be wrong.
**Evidence**: A temporary probe built `SlicePresence::new(65)`, set slices 0, 63 and 64, tried slice 65 and observed a count of 3 with slice 64 present and slice 65 absent. The probe passed and was deleted after the run.

### S3, The `place` error variants and `InconsistentDimensions` have no committed test
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:467)
**What**: No committed test reaches `SliceOutOfRange`, `SliceSizeMismatch` or the dimensions and kind mismatch return. Only the happy path and the idempotent rewrite are tested.
**Evidence**: A temporary probe observed `SliceOutOfRange` with index 9 and count 3 and `SliceSizeMismatch` with expected 24 and actual 3 from the unmodified code. Both variants behave correctly and neither is guarded. The probe was deleted after the run.

### S4, The level axis landed by decision C1 is asserted by no test
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:451)
**What**: Decision C1 lands `levels` with exactly one full resolution entry carrying dims, spacing and offset. No committed test reads `levels`, so the axis the S12 round paid for could vanish under a green suite.
**Evidence**: `levels` was mutated to an empty vector and `bin/ocelli.sh test ocelli-volume` stayed green with 1 unit test and 9 assembly tests passing. A temporary probe on the unmodified code observed one entry with dims `[4, 3, 3]`, spacing `[1.0, 1.0, 3.0]` and offset 0. The mutation was reverted to the recorded hash and the probe was deleted.

### S5, `assemble` discards the sorted order and the candidate samples it consumed
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:359)
**What**: `assemble` takes the candidate by value, sorts a local vector of projected positions, builds a cleared store and returns no sorted index mapping. A caller who then fills the volume through `place` must derive the sorting again outside this crate or place slices in arrival order into wrong z slots.
**Why it matters**: The code is correct today because no caller exists yet. The first consumer in F-051 is where arrival order placement becomes a quietly wrong volume, and the sorting logic would then exist twice.
**Evidence**: Read of lines 377 to 457 found the sorted vector dropped at return and no read of frame samples in `assemble`. The tests place synthetic byte blocks by index and never place a candidate frame sample.

### S6, Two new acceptance bounds have no recorded rationale
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:392)
**What**: Duplicate positions use an absolute bound of `1e-9` and shear uses a lateral bound of `1e-6`. Neither bound appears in the plan decisions, where only the E1 bound of 0.5 per cent of the median gap carries a rationale.
**Why it matters**: The plan records E1 with its reasoning under the rule for a bound written for the first time. Two more bounds written for the first time with no reasoning will be defended later as though they were measured.

## Nitpicks

### N1, `place` reports size before range on a doubly wrong write
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:467)
**What**: A write that is both out of range and the wrong size returns `SliceSizeMismatch`. No text specifies an order, so this is a preference for reporting the range fault first.

### N2, Unreachable fallback arithmetic hides two invariants
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:404)
**What**: The gaps closure falls back to `0.0` for window elements that `windows(2)` always supplies, and `slice_mut` saturates its start offset where `slice` checks the same product. The fallbacks read as though a missing element were a real case.

## Verified clean

- Projected spacing arithmetic. `assemble` projects each IPP with `plane.position_vector().dot(normal)`, sorts ascending, takes consecutive differences and refuses past 0.5 per cent of the median at lib.rs lines 376 to 430. The normal comes from `slice_normal`, which is `X cross Y` normalised at image_plane.rs lines 185 to 190, so spacing is derived from geometry and never from a tag. Search of the crate found no read of `SpacingBetweenSlices`, `SliceThickness`, `SliceLocation` or `InstanceNumber`.
- Median rather than mean. `median` sorts the gaps and averages the two middle values on an even count at lib.rs lines 520 to 534, which gives 3.0 for gaps of 3, 3, 7 and 3 with the worst gap named at index 2.
- `PixelSpacing[0]` direction. Volume spacing is `[column_mm(), row_mm(), median]` at lib.rs lines 433 to 437 against dims of `[columns, rows, count]`, so spacing index 0 steps along the row cosine and index 1 along the column cosine. That agrees with `index_to_world` in image_plane.rs lines 218 to 219 and with PS3.3 C.7.6.2.1.1 as transcribed in the plan. The new `row()` and `column()` accessors name `X` and `Y` correctly.
- Origin as first voxel centre. Origin is the IPP of the lowest projected slice copied exactly at lib.rs lines 438 to 447 with no half voxel offset, and `ImagePositionPatient` is documented as the centre of the first voxel at image_plane.rs line 14.
- Shear from geometry. Each inter-slice vector is compared with the normal through its lateral residual at lib.rs lines 390 to 402 with a cosine reported from the vectors. No tilt tag is read anywhere in the crate.
- Slice layout. Slice `n` starts at `n * dims[0] * dims[1] * bytes_per_voxel` in both `slice` and `slice_mut` at lib.rs lines 283 to 315 over one contiguous `Vec<u8>` with no per-slice vector, and the 4 by 3 by 2 fixture puts slice 1 at byte 24.
- Required mutations all went red and were reverted exactly. A fixed spacing of 5.0 failed 3 tests. The mean in place of the median failed the non-uniform test alone. A reversed sort failed 5 tests. Swapped spacing indices failed the non-square spacing test alone. A half voxel origin offset of 0.5 failed the sorting and origin test alone. The file hash after every revert was `34c4558abcc3da218418f6c15b9359b81c4c8cea202e04deacad32f7006dc35f`.
- Fixture values are hand-computed. Expected spacing of 3.0 against positions 0, 3, 6, 9 and 12, steps of 1.0 and 0.5 for the non-square pixel and byte 24 for slice 1 all cite PS3.3 C.7.6.2.1.1 and none is copied from implementation output.
- Boundary and tier. Search of the diff found no `unsafe`, no `wasm-bindgen` and no `bitvec`. Cargo adds only glam, thiserror, ocelli-cache, ocelli-core and ocelli-pixel with proptest as a dev dependency. No pixel crosses the boundary, no allocation sits in a render loop and `place` copies into the one allocation made at assembly.
- Structure. No new trait, generic parameter, `Box<dyn>` or forwarding wrapper appears. `Budgeted` is an existing trait with a new implementer and the newtypes each hold one invariant. Deviation D-25 for `SlicePresence` is recorded in `docs/hld/DEVIATIONS.md` and decision C1, D1, E1 and F1 match the code. Brick decomposition is F-036 scope in the current sprint and not part of this diff.
- Casts. Search found no `as` cast in `crates/ocelli-volume/src/lib.rs`. Conversions use `try_from` with stated fallbacks and the shift in `SlicePresence` is bounded by `n % 64`.
- Gates with exit codes read from the commands. `bin/ocelli.sh test ocelli-volume` exited 0 with 1 unit test and 9 assembly tests passing. `bin/ocelli.sh clippy ocelli-volume` exited 0. `bin/ocelli.sh test ocelli-pixel` exited 0 with all suites passing.
- Backlog. The F-033 row reads `in-progress`, which matches a working diff under review.
