# F-033 review, pass 2

**Reviewed**: working tree diff for F-033 in `crates/ocelli-volume/src/lib.rs`, `crates/ocelli-volume/Cargo.toml`, `crates/ocelli-volume/tests/assembly.rs`, `crates/ocelli-pixel/src/image_plane.rs` and the F-033 row of `docs/sprints/BACKLOG.md`, against pass 1 in `.claude/reviews/F-033-volume-pass-1.md`
**Result**: 0 defects, 1 smell, 3 nitpicks

## Defects

None. Every pass 1 defect is remediated and the remediation is guarded, as itemised under Verified clean.

## Smells

### S1, The multi-sample half of the `InconsistentSpacing` guard is never exercised by a committed test
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:398)
**What**: The guard refuses when `samples_per_pixel() != 1`, and the behaviour is correct today. No committed fixture builds a layout with more than one sample per pixel, because both fixture helpers in `assembly.rs` hardcode `SampleLayout::new(1, ...)`. The branch could be deleted under a green suite, and a future colour series would then assemble into a store whose slice length assumes one sample per pixel, which is the second half of pass 1 defect D2 returning silently.
**Evidence**: A temporary probe built two interleaved RGB slices of 72 bytes and observed `Err(VolumeError::InconsistentSpacing)` from the unmodified code. The probe passed and was deleted. The layout clause at lib.rs lines 398 to 400 was then mutated away and `bin/ocelli.sh test ocelli-volume` stayed green with 1 unit test and 13 assembly tests passing. The mutation was reverted and the file hash returned to `094fc08eba885e32e72ec84ee24a4e02a84fcf17f6e22ead088c74508fcd88de`.

## Nitpicks

### N1, `place` reports size before range on a doubly wrong write
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:509)
**What**: Carried from pass 1 N1 and unchanged. A write that is both out of range and the wrong size returns `SliceSizeMismatch`. No text specifies an order, so this stays a preference.

### N2, Unreachable fallback arithmetic hides two invariants
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:444)
**What**: Carried from pass 1 N2 and unchanged. The gaps closure falls back to `0.0` for window elements that `windows(2)` always supplies, and `slice_mut` saturates its start offset where `slice` checks the same product.

### N3, The `assemble` error list omits two refusals it can return
**Where**: [crates/ocelli-volume/src/lib.rs](/Users/atulsharma/Documents/projects/tensorbee/ocelli/crates/ocelli-volume/src/lib.rs:366)
**What**: The `# Errors` list names six variants and omits `InconsistentSpacing` and `UnsupportedVoxelKind`, both returned from the per-slice loop. Each variant carries its own accurate doc on the enum, so a reader loses nothing except the summary.

## Verified clean

- D1 remediated and guarded. `describe` at lib.rs lines 547 to 560 names only `(8, Unsigned)`, `(16, Signed)` and `(16, Unsigned)` and returns `UnsupportedVoxelKind` for everything else. The committed test `kinds_voxel_kind_cannot_name_refuse` refuses signed 8-bit and both 32-bit representations, and the full suite passes with it in place.
- D2 spacing half remediated and guarded. The per-slice loop at lib.rs lines 393 to 402 compares `plane.spacing()` against the first plane and returns `InconsistentSpacing`, so spacing no longer comes from first arrival alone. `inconsistent_spacing_and_dimensions_refuse` covers differing `PixelSpacing` of `[2.0, 2.0]` against `[1.0, 1.0]` and still covers `InconsistentDimensions`. The multi-sample half is correct but unguarded, recorded as S1 above.
- D3 remediated and mutation-proved. `an_oblique_series_sorts_by_projection_and_not_by_ipp_z` uses row cosine `(1, 0, 0)` and column cosine `(0, -0.6, -0.8)`, whose cross product is the normal `(0, 0.8, -0.6)`. Positions step 3 mm along that normal while IPP z descends, so the two orders disagree. Mutating the projection at lib.rs line 412 to `plane.position_vector().z` turned the suite red with exit 101 and exactly that one test failing out of 13. The mutation was reverted and the file hash returned to `094fc08eba885e32e72ec84ee24a4e02a84fcf17f6e22ead088c74508fcd88de`.
- D4 remediated. The `NotOneVolume` doc at lib.rs lines 154 to 156 now claims only mixed orientations for assembly and names `group_volumes` as the frame of reference owner. A search of the crate confirms `frame_of_reference` is read only in grouping at lines 107 to 124 and never in `assemble`, so the prose now matches the code.
- S1 to S4 of pass 1 remediated in one committed test. `place_errors_levels_order_and_cache_are_guarded` covers `VolumeCache` insert, get, remove and pressure, `Budgeted` bytes of 72 for a 4 by 3 by 3 volume of 2-byte voxels, `SlicePresence` at 130 slices across the 64-bit word boundary, both `place` error variants with exact payloads, exactly one level with dims `[4, 3, 3]` and offset 0, and `order` of `[2, 1, 0]` with each arriving slice sample block placed through that order to its sorted position.
- Levels mutation-proved. Replacing the one-entry `levels` vector with an empty vector turned the suite red with exit 101 and exactly `place_errors_levels_order_and_cache_are_guarded` failing at the length assertion. The mutation was reverted to the recorded hash above.
- S5 of pass 1 remediated. `Volume.order` at lib.rs lines 350 to 354 and 482 records the arrival index of each slice in sorted order, built from the same projected sort that derives spacing, and the guarded test places candidate samples through it. Sorting is derived once, inside this crate.
- S6 of pass 1 remediated. The duplicate bound carries its rationale at lib.rs lines 425 to 428, a nanometre against `f64` dot product noise with real duplicates exact. The shear bound carries its rationale at lines 434 to 436, a micrometre of lateral step on the same reasoning.
- Pixel row and column accessors. `row()` and `column()` at image_plane.rs lines 57 to 65 return the validated `X` and `Y` cosines under their PS3.3 C.7.6.2.1.1 names. Their one caller builds the direction matrix as `DMat3::from_cols(row, column, normal)` at lib.rs line 484, which agrees with `index_to_world` and with the spacing order `[column_mm, row_mm, median]` asserted by the non-square spacing test.
- Arithmetic rechecked. Projected spacing still comes from `position_vector().dot(normal)` with the normal from `slice_normal` as `X cross Y`, the median still averages the two middle gaps on an even count, the origin is still the lowest projected IPP with no half voxel offset, and shear still compares the lateral residual of each inter-slice vector against the normal. No tag for spacing, thickness, location or instance order is read anywhere in the crate.
- Boundary, tier and structure. The diff adds no `unsafe`, no `as` cast outside prose, no `wasm-bindgen`, no new trait or generic, and no forwarding wrapper. `VolumeCache` forwards to the existing `Lru` and `Budgeted` gains one implementer, both within the structural rules.
- Gates with exit codes read from the commands themselves. `bin/ocelli.sh test ocelli-volume` exited 0 with 1 unit test and 13 assembly tests passing. `bin/ocelli.sh clippy ocelli-volume` exited 0. `bin/ocelli.sh test ocelli-pixel` exited 0 with every suite passing. The tree after all mutations hashes to `094fc08eba885e32e72ec84ee24a4e02a84fcf17f6e22ead088c74508fcd88de` for lib.rs, `c4b2e40aa6bd6610f7e8133128b3d63265ce32e350c8f50191e335eefb298d2b` for assembly.rs and `8ab87c8167ef07f92e312ddefa5fed6f490632b7078b7a2907bdba670e177555` for image_plane.rs, identical to the hashes recorded before any mutation.
