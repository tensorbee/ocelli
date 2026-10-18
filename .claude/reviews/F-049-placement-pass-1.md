# F-049 review, pass 1

**Reviewed**: working tree on sprint/s15 against 1e54993
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

`docs/lld/viewport.md` lines 9 and 10 say that `frame_shift` and `apply_placement` store the displayed spacing and the measured index shift on the view. `frame_shift` at `crates/ocelli-viewport/src/placement.rs:27` returns `(f64, f64)` and does not take a `View`. The store is `apply_placement` at `placement.rs:54` through `placement.rs:58`, which calls `set_spacing` and `set_index_shift`. The placement section at `docs/lld/viewport.md:125` already names those two calls. The opening sentence is false for `frame_shift`. It does not change the arithmetic.

## Verified clean

HEAD is `1e54993071319dd74158b66bdd191f03ef7a5d53` on `sprint/s15`. `git diff --name-only 1e54993` is `crates/ocelli-viewport/src/lib.rs`, `docs/lld/README.md`, and `docs/lld/viewport.md`. `placement.rs` and `tests/placement.rs` are untracked. `git diff --exit-code 1e54993 -- crates/ocelli-viewport/src/view.rs crates/ocelli-render/src/graph.rs docs/lld/volumes.md docs/hld/DEVIATIONS.md` printed nothing and exited 0. `view.rs` and `graph.rs` were not edited. `volumes.md` was not edited. No HLD deviation file was added.

**Formula.** `frame_shift` builds one plane from `reference.position`, `displayed.orientation`, `displayed.spacing()`, and `displayed.dimensions()` at `placement.rs:30`. It then calls `index_to_world().inverse().apply` on the displayed position at `placement.rs:38`. There is no second dot product in this module. The only PS3.3 C.7.6.2.1.1 product is `ImagePlane::index_to_world` at `crates/ocelli-pixel/src/image_plane.rs:218`. Column step is the row direction times column spacing. Row step is the column direction times row spacing. The translation is the image position, with no half-voxel term, at `image_plane.rs:225`. IPP stays the centre of voxel `(0, 0, 0)`.

Hand arithmetic for the approved fixture. Reference IPP `(0, 0, 0)`, row direction `(1, 0, 0)`, column direction `(0, 1, 0)`, column spacing `0.5`, row spacing `2.0`, displayed IPP `(1, 4, 0)`.

```text
1 = i * 0.5
4 = j * 2.0
i = 2
j = 2
```

A swapped spacing would be `i = 1 / 2.0 = 0.5` and `j = 4 / 0.5 = 8`. The same point with `z = 5` keeps `i = 2` and `j = 2`, because the third matrix column is the unit normal `(0, 0, 1)` and `z` is not added to either index. The tests compare `to_bits`, not a widened epsilon. `tests/placement.rs:31` is exact bit equality.

**Spacing swap.** `PixelSpacing::new` returns `Result`, so the mutation was `PixelSpacing::new(column_mm, row_mm)` in place of `displayed.spacing()`, with the `Ok` value passed into `ImagePlane::new`. The zero-spacing guards still read `displayed.spacing()`. `bin/ocelli.sh test ocelli-viewport --test placement non_square_spacing_shifts_by_the_section_c_7_6_2_index` exited 101. The panic was `tests/placement.rs:31`, message `column shift`. Left bits `4602678819172646912` are `0.5`. Right bits `4611686018427387904` are `2.0`. The row assertion did not run. The swapped formula would have made that row `8`. The original `displayed.spacing()` call was restored before any other edit. The same test then exited 0.

**Zero column.** Deleting the `column_mm > 0.0` guard so a zero column returns `index.x` made `a_zero_column_spacing_contributes_no_column_shift` exit 101. The panic was `tests/placement.rs:31`, message `column shift`. Left bits `4613937818241073152` are `3.0`. Right bits are `0`. That `3.0` is the unit substitute inside `index_to_world` at `image_plane.rs:208`, not a pixel size. The guard was restored. The row guard is the same comparison and was not deleted. `PixelSpacing::new` rejects a negative finite value at `image_plane.rs:83`, and `ImagePlane::new` allows `0` only on a singleton axis at `image_plane.rs:157`. The `> 0.0` test matches the substitute.

**Index shift is observed.** Removing only `view.set_index_shift(column, row)` made `apply_placement_stores_spacing_and_the_measured_shift` exit 101. The panic was `tests/placement.rs:123`. Left bits `4602678819172646912` are `0.5`. Right bits `4609434218613702656` are `1.5`. The call was restored. Forgetting the shift does not leave `image_to_canvas` green.

Independent canvas arithmetic, from `View` and not from the test comment. After `set_spacing(0.5, 2.0)` and `set_index_shift(2, 2)`, before `fit`, scale is `1` and the anchor is the image centre. `f64_from_u32(2) * 0.5` at `view.rs:462` is `1`. Point `(0, 0)` on a 2 by 2 image gives `dx = 0.5 * (0 - 1) = -0.5` and `dy = 2.0 * (0 - 1) = -2`. Rotation `0` is exact cos `1` and sin `0` at `view.rs:429`, so `clockwise` at `view.rs:396` leaves that pair. Translation at `view.rs:291` is `(1 + 2 * 0.5, 1 + 2 * 2) = (2, 5)`. Canvas is `(2 - 0.5, 5 - 2) = (1.5, 3)`. Without the index shift the translation is `(1, 1)` and canvas x is `0.5`, which is the red left value above.

`set_spacing` takes column millimetres then row millimetres at `view.rs:146`. `PixelSpacing::new` takes row then column at `image_plane.rs:82`. `apply_placement` passes `column_mm()` then `row_mm()` at `placement.rs:57`. Fit of a 2 by 2 image into an 8 by 8 canvas uses physical bounds `(2 * 0.5, 2 * 2.0) = (1, 4)` from `view.rs:310`. Scale is `min(8 / 1, 8 / 4) = 2`. Displayed size is `2` by `8`. Offset at `view.rs:191` is `(4 - 1, 4 - 4) = (3, 0)`. `offset` does not read the index shift. The spacing observable is that offset. The shift observable is `image_to_canvas`. Both assertions use `to_bits`. No tolerance was widened.

**`ImagePlane::new` failure arm.** The `else` at `placement.rs:35` returns `(0.0, 0.0)`. `ImagePlane::new` fails only when zero row spacing is paired with more than one row, or zero column spacing with more than one column, at `image_plane.rs:157`. Spacing and dimensions are private at `image_plane.rs:138`. The only constructor is `new`. This call copies the pair `displayed` already accepted, and `new` does not recheck position or orientation. The arm is not reachable for a valid displayed plane. It does not hide a failure that pair can produce. Returning `(0, 0)` is how the copy stays free of `unwrap`. `unwrap_used` is deny in the workspace. No allow attribute is warranted.

**Boundary.** The new files have no `unsafe`, no `unwrap`, no `expect`, no `panic`, no `usize`, and no `as` cast. The word "as" in `placement.rs` is only the comments at lines 13 and 28. `bin/ocelli.sh clippy ocelli-viewport` exited 0. That command is `cargo clippy -p ocelli-viewport --all-targets -- -D warnings`. The exit was read from the command status, not from a pipe. `lib.rs:8` is still `#![cfg_attr(not(test), no_std)]`. `python3 scripts/no_std_check.py` printed `OK: 7 no_std crate(s) reach no std feature` and exited 0. `ocelli-viewport` is in that set. `cargo tree -p ocelli-viewport -e normal` lists `glam`, `ocelli-cache`, `ocelli-pixel`, and `ocelli-core`. It does not list wgpu, `ocelli-render`, or `ocelli-dicom`. The module does not read `nominal_tilt_degrees`, `StackShear`, imager spacing, or `Volume::assemble`, and it does not sort by image position. The reference is the caller argument. Frame 0 against itself is the `(0, 0)` test, which passed in the restored suite.

**Tiers and LLD.** `docs/lld/viewport.md:131` states tier A and tier B upload the same camera matrix, and tier C computes the same two numbers, with drawing still unavailable. `placement.rs` has no tier branch, so CPU does not produce a different shift. A search of `docs/lld/viewport.md` found no prose semicolon, no em dash, and no en dash. `docs/lld/README.md:32` names measured placement and F-049, matching the viewport header. The placement section matches the code except for the opening sentence recorded above. `docs/lld/viewport.md:117` says the column shift is the inverse x of `index_to_world`. In the code that is the x of `inverse().apply` on the displayed position, not a bare matrix element. With the sentences above it, that shorthand matches `placement.rs:38`.

**Restored tree.** All three mutations were removed before clippy and before the suite. The restored `frame_shift` passes `displayed.spacing()` into `ImagePlane::new`, keeps the `column_mm > 0.0` guard, and `apply_placement` calls `set_index_shift`. `git diff -- crates/ocelli-viewport/src/placement.rs` printed nothing because the file is untracked. `git status --short` is the three tracked paths above plus the two untracked placement files. `bin/ocelli.sh test ocelli-viewport --test placement` exited 0. All 8 tests passed. None were filtered out.
