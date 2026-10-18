# F-049, Calibration, spacing and gantry-tilt correction

**Status**: approved
**Epic ref**: E7.5
**Sprint**: S15
**Estimate**: 2w

## Normative source, transcribed

### DICOM PS3.3 C.7.6.2.1.1, as HLD section 16's plane and as the existing function

`ImagePlane::index_to_world` is the one implementation. The patient point of column `i` and row `j` is:

```text
P = IPP + i * column_spacing * row_direction + j * row_spacing * column_direction
```

`row_direction` is Image Orientation Patient `X`. `column_direction` is `Y`. Pixel Spacing is row spacing then column spacing, so row spacing multiplies the column direction and column spacing multiplies the row direction. IPP is the centre of voxel `(0, 0, 0)`. There is no half-voxel offset. `docs/lld/pixel-pipeline.md` already says this, and the non-square fixture there uses row spacing `2.0` and column spacing `0.5` because a square fixture cannot see a swap.

### `docs/hld/10-extension-points.md`, section 13, calibrated display

```text
Calibrated display presentation behind a trait the browser implements as a no-op. DICOM Part 14 greyscale calibration is unreachable from a web page, and is the strongest reason the desktop target exists.
```

That trait is display calibration, GSDF, not pixel spacing. This story does not implement it and does not claim a diagnostic display calibration. Spacing here is Pixel Spacing of the image.

### What F-020 already measured

`SpacingEvidence` keeps `image_mm` and `imager_mm` distinct. `image_mm` is Pixel Spacing `(0028,0030)`. `imager_mm` is Imager Pixel Spacing `(0018,1164)` and is never substituted. `FrameGeometry::derive` returns `UncalibratedSpacingOnly` when only the imager value is present.

`StackShear` is measured from image positions. `AxisAligned` carries the projected step. `Sheared` carries the largest off-axis component. `nominal_tilt_degrees` retains Gantry/Detector Tilt `(0018,1120)` and is not the shear. `docs/lld/volumes.md` refuses a sheared series as a volume. This story does not rebuild that volume.

## What the specification does not cover

The HLD does not say how a stack view consumes the shear. The sprint text does: correct the view with the spacing and the measured shear, and do not read the tilt tag as the shear.

1. **The shift reuses `index_to_world`.** Build one `ImagePlane` whose origin is the reference frame's image position, and whose orientation, spacing and dimensions are the displayed frame's. Apply `index_to_world().inverse()` to the displayed frame's image position as a `Pt<World>`. The column shift is the resulting `x`. The row shift is the resulting `y`. The `z` component is not a canvas translation. This is the reference grid expressed in the displayed frame's pixel size, so a spacing difference is not applied twice and a second dot-product formula is not written.

2. **A zero spacing on a singleton axis contributes no shift.** `index_to_world` substitutes `1.0` on that axis so the matrix stays invertible. That substitute is not a pixel size. If `column_mm` is `0`, the column shift is `0`. If `row_mm` is `0`, the row shift is `0`.

3. **Aspect uses Pixel Spacing of the displayed frame.** `View::set_spacing` receives `column_mm` and `row_mm` from `plane.spacing()`. Imager spacing is not a parameter of `frame_shift` or of `set_spacing`. Gantry tilt is not a parameter. A sheared stack still returns a shift. `Volume::assemble` is not called.

4. **The reference frame is the first frame of the stored sequence.** F-047's order, not an IPP sort. Frame 0 against itself shifts by `(0, 0)`.

5. **F-045 already multiplies the shift into the camera matrix.** This story sets the inputs. It does not edit `graph.rs` or `view.rs` except to call `set_spacing` and `set_index_shift`.

## Approach

`frame_shift` in `crates/ocelli-viewport/src/placement.rs` takes the reference `ImagePlane` and the displayed `ImagePlane` and returns the two image-pixel shifts. `apply_placement` writes those shifts and the displayed spacing into the `View` from F-045.

The crate already depends on `ocelli-pixel` once F-046 has landed. This story uses that dependency. It does not depend on `ocelli-dicom`, so a plane can be built in a test without a data set. `lib.rs` gains `pub mod placement`.

`nominal_tilt_degrees` is not read. A caller that has a `StackGeometry` does not pass `shear()` into this function either. The positions already carry the shear. Reading `StackShear::Sheared.max_offaxis_mm` and treating it as a pan would be a second, coarser number.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The shift is two `f64` stored on `View` before the frame.
- unsafe: none
- Tier A (WebGPU): full, because the shift is inside the camera matrix F-045 uploads
- Tier B (WebGL2): full, the same matrix
- Tier C (CPU): full for the shift math, which has no device. Drawing remains F-X003.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Reference IPP `(0, 0, 0)`, row direction `(1, 0, 0)`, column direction `(0, 1, 0)`, column spacing `0.5`, row spacing `2.0`. Displayed IPP `(1, 4, 0)`. PS3.3 C.7.6.2.1.1 gives column index `1 / 0.5 = 2` and row index `4 / 2 = 2`. A swapped spacing would yield `0.5` and `8`. | `crates/ocelli-viewport/tests/placement.rs` |
| fixture | The same planes with displayed IPP `(1, 4, 5)`. The shift stays `(2, 2)`. The normal component `5` is not added to either shift. | same |
| fixture | Displayed IPP equal to the reference. The shift is `(0, 0)`. | same |
| fixture | One column, `column_mm` `0`, `row_mm` `1`, an offset of `3` mm along the row direction. Column shift is `0`. Row shift follows the row spacing and is not the unit-extension substitute. | same |
| unit | The function's parameters are two `ImagePlane` values. A tilt in degrees cannot be passed. The test builds the non-square fixture and does not mention `(0018,1120)`. | same |
| unit | `apply_placement` stores spacing `(0.5, 2.0)` on the `View`. Fit of a 2 by 2 image into an 8 by 8 canvas then uses physical size `1` by `4`, scale `min(8/1, 8/4) = 2`, displayed size `2` by `8`, x offset `(8 - 2) / 2 = 3`. | same, through F-045's `fit` |

Mutate the row and column spacing in the shift path, swapping which one divides which axis, and confirm the `(2, 2)` fixture goes red. Revert.

## Parity surface covered

none as a checklist row. Spacing is not an Appendix B line. The stack viewport type is F-045's.

## Deviations

none

## LLD impact

- `docs/lld/viewport.md`, a placement section

`docs/lld/volumes.md` stays as it is. Volume assembly is not changed.

## Open questions

None.
