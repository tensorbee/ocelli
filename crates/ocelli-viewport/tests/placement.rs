//! F-049 placement.
//!
//! PS3.3 C.7.6.2.1.1. The patient point of column `i` and row `j` is
//! `IPP + i * column_spacing * row_direction + j * row_spacing * column_direction`.
//! IPP is the centre of voxel `(0, 0)`. There is no half-voxel offset.
//! Pixel Spacing is row spacing, then column spacing.

use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, PixelError,
    PixelSpacing,
};
use ocelli_viewport::placement::{apply_placement, frame_shift};
use ocelli_viewport::view::View;

fn plane(
    position: [f64; 3],
    row_mm: f64,
    column_mm: f64,
    rows: u32,
    columns: u32,
) -> Result<ImagePlane, PixelError> {
    ImagePlane::new(
        ImagePositionPatient::new(position)?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(row_mm, column_mm)?,
        ImageDimensions::new(rows, columns)?,
    )
}

fn assert_shift(got: (f64, f64), column: f64, row: f64) {
    assert_eq!(got.0.to_bits(), column.to_bits(), "column shift");
    assert_eq!(got.1.to_bits(), row.to_bits(), "row shift");
}

#[test]
fn non_square_spacing_shifts_by_the_section_c_7_6_2_index() -> Result<(), PixelError> {
    // Column spacing 0.5 and row spacing 2.0. Displayed IPP (1, 4, 0).
    // Column index is 1 / 0.5 = 2. Row index is 4 / 2 = 2.
    // Swapped spacing would yield 0.5 and 8.
    let reference = plane([0.0, 0.0, 0.0], 2.0, 0.5, 2, 2)?;
    let displayed = plane([1.0, 4.0, 0.0], 2.0, 0.5, 2, 2)?;
    assert_shift(frame_shift(&reference, &displayed), 2.0, 2.0);
    Ok(())
}

#[test]
fn a_normal_component_is_not_a_canvas_shift() -> Result<(), PixelError> {
    // Same planes with displayed IPP (1, 4, 5). The in-plane indices stay
    // 2 and 2. The 5 mm normal is not added to either shift.
    let reference = plane([0.0, 0.0, 0.0], 2.0, 0.5, 2, 2)?;
    let displayed = plane([1.0, 4.0, 5.0], 2.0, 0.5, 2, 2)?;
    assert_shift(frame_shift(&reference, &displayed), 2.0, 2.0);
    Ok(())
}

#[test]
fn a_frame_against_itself_shifts_by_zero() -> Result<(), PixelError> {
    let reference = plane([1.0, 4.0, 0.0], 2.0, 0.5, 2, 2)?;
    let displayed = plane([1.0, 4.0, 0.0], 2.0, 0.5, 2, 2)?;
    assert_shift(frame_shift(&reference, &displayed), 0.0, 0.0);
    Ok(())
}

#[test]
fn a_zero_column_spacing_contributes_no_column_shift() -> Result<(), PixelError> {
    // One column, column spacing 0, row spacing 1. Offset 3 mm along the
    // row direction. `index_to_world` extends the unused axis by 1 and would
    // report column index 3. That extension is not a pixel size, so the
    // column shift is 0. There is no offset along the column direction, so
    // the row shift is 0 / row_mm = 0.
    let reference = plane([0.0, 0.0, 0.0], 1.0, 0.0, 2, 1)?;
    let displayed = plane([3.0, 0.0, 0.0], 1.0, 0.0, 2, 1)?;
    assert_shift(frame_shift(&reference, &displayed), 0.0, 0.0);
    Ok(())
}

#[test]
fn a_zero_row_spacing_contributes_no_row_shift() -> Result<(), PixelError> {
    // One row, row spacing 0, column spacing 1. Offset 3 mm along the
    // column direction. The unit extension would report row index 3. The
    // row shift is 0. The column shift is 0 / column_mm = 0.
    let reference = plane([0.0, 0.0, 0.0], 0.0, 1.0, 1, 2)?;
    let displayed = plane([0.0, 3.0, 0.0], 0.0, 1.0, 1, 2)?;
    assert_shift(frame_shift(&reference, &displayed), 0.0, 0.0);
    Ok(())
}

#[test]
fn displayed_spacing_is_the_divisor() -> Result<(), PixelError> {
    // The reference grid uses the displayed frame's pixel size. Reference
    // spacing of 1 mm would report indices 1 and 4 for IPP (1, 4, 0).
    let reference = plane([0.0, 0.0, 0.0], 1.0, 1.0, 4, 4)?;
    let displayed = plane([1.0, 4.0, 0.0], 2.0, 0.5, 2, 2)?;
    assert_shift(frame_shift(&reference, &displayed), 2.0, 2.0);
    Ok(())
}

#[test]
fn shift_is_computed_from_two_planes() -> Result<(), PixelError> {
    let reference = plane([0.0, 0.0, 0.0], 2.0, 0.5, 2, 2)?;
    let displayed = plane([1.0, 4.0, 0.0], 2.0, 0.5, 2, 2)?;
    assert_shift(frame_shift(&reference, &displayed), 2.0, 2.0);
    Ok(())
}

#[test]
fn apply_placement_stores_spacing_and_the_measured_shift() -> Result<(), PixelError> {
    // Column spacing 0.5 and row spacing 2.0. Shift (2, 2) from the
    // non-square fixture. Centre of a 2 by 2 image is (1, 1). Point (0, 0)
    // on an unfitted view, scale 1:
    // dx = 0.5 * (0 - 1) = -0.5
    // dy = 2.0 * (0 - 1) = -2
    // translate = (1 + 2 * 0.5, 1 + 2 * 2) = (2, 5)
    // canvas = (2 - 0.5, 5 - 2) = (1.5, 3)
    // Fit of that 2 by 2 image into an 8 by 8 canvas uses physical size
    // 1 by 4. Scale is min(8/1, 8/4) = 2. Displayed size is 2 by 8, so the
    // x offset is (8 - 2) / 2 = 3.
    let reference = plane([0.0, 0.0, 0.0], 2.0, 0.5, 2, 2)?;
    let displayed = plane([1.0, 4.0, 0.0], 2.0, 0.5, 2, 2)?;
    let mut view = View::new();
    apply_placement(&mut view, &reference, &displayed);
    let (x, y) = view.image_to_canvas(0.0, 0.0, 2, 2);
    assert_eq!(x.to_bits(), 1.5_f64.to_bits());
    assert_eq!(y.to_bits(), 3.0_f64.to_bits());
    view.fit(8.0, 8.0, 2, 2);
    assert_eq!(view.scale().to_bits(), 2.0_f64.to_bits());
    let (offset_x, offset_y) = view.offset(2, 2);
    assert_eq!(offset_x.to_bits(), 3.0_f64.to_bits());
    assert_eq!(offset_y.to_bits(), 0.0_f64.to_bits());
    Ok(())
}
