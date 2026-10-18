//! Measured spacing and index shift for one displayed frame.
//!
//! PS3.3 C.7.6.2.1.1 is implemented once, in [`ImagePlane::index_to_world`].
//! This module builds a plane on the reference image position with the
//! displayed frame's orientation, spacing and dimensions, then reads the
//! inverse. The `z` component is not a canvas translation.
//!
//! A zero spacing is legal only on a singleton axis. `index_to_world`
//! substitutes `1.0` there so the matrix stays invertible. That substitute
//! is not a pixel size, so the shift on that axis is `0`.
//!
//! Imager pixel spacing is not an argument. Gantry tilt is not an argument.
//! The caller passes the first frame of the stored sequence as the
//! reference. This module does not sort by image position.

use ocelli_core::{Pt, World};
use ocelli_pixel::ImagePlane;

use crate::view::View;

/// Column and row index of `displayed` on the reference origin.
///
/// The column shift is inverse `x`. The row shift is inverse `y`. A zero
/// spacing on that axis contributes `0`, not the unit extension inside
/// `index_to_world`.
#[must_use]
pub fn frame_shift(reference: &ImagePlane, displayed: &ImagePlane) -> (f64, f64) {
    // Spacing and dimensions are copied as the pair `displayed` already
    // accepted. `ImagePlane::new` only rejects that pair.
    let Ok(grid) = ImagePlane::new(
        reference.position,
        displayed.orientation,
        displayed.spacing(),
        displayed.dimensions(),
    ) else {
        return (0.0, 0.0);
    };
    let patient = displayed.position_vector();
    let index = grid
        .index_to_world()
        .inverse()
        .apply(Pt::<World>::new(patient.x, patient.y, patient.z));
    let spacing = displayed.spacing();
    let column = if spacing.column_mm() > 0.0 {
        index.x
    } else {
        0.0
    };
    let row = if spacing.row_mm() > 0.0 { index.y } else { 0.0 };
    (column, row)
}

/// Store the displayed spacing and the measured index shift on `view`.
pub fn apply_placement(view: &mut View, reference: &ImagePlane, displayed: &ImagePlane) {
    let (column, row) = frame_shift(reference, displayed);
    let spacing = displayed.spacing();
    view.set_spacing(spacing.column_mm(), spacing.row_mm());
    view.set_index_shift(column, row);
}
