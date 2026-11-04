//! F-045 camera fixtures.
//!
//! Hand-computed from the approved composition. Canvas origin is top-left
//! and y increases down, HLD section 16. A quarter turn clockwise sends
//! `(dx, dy)` to `(-dy, dx)`. Fit letterboxes the axis-aligned bounds.
//! Spacing is inside that map: a 2 by 2 image at column 0.5 mm and row
//! 2.0 mm has physical size 1 by 4.

use ocelli_core::{Canvas, Pt};
use ocelli_viewport::view::{Radians, View, ViewError, Zoom};

fn assert_point(got: (f64, f64), x: f64, y: f64) {
    assert_eq!(got.0.to_bits(), x.to_bits(), "x {got:?} against ({x}, {y})");
    assert_eq!(got.1.to_bits(), y.to_bits(), "y {got:?} against ({x}, {y})");
}

fn image_of(matrix: [f32; 16], x: f32, y: f32) -> (f32, f32) {
    (
        matrix[0] * x + matrix[4] * y + matrix[12],
        matrix[1] * x + matrix[5] * y + matrix[13],
    )
}

fn assert_image(got: (f32, f32), x: f32, y: f32) {
    assert_eq!(got.0.to_bits(), x.to_bits());
    assert_eq!(got.1.to_bits(), y.to_bits());
}

#[test]
fn identity_maps_pixel_centres_onto_the_same_texel() {
    let view = View::new();
    // Width 4, height 1. WebGPU fragment position is the pixel centre, so
    // `i32` truncation of these image coordinates is texel (0, 0) and (3, 0).
    assert_point(view.image_to_canvas(0.5, 0.5, 4, 1), 0.5, 0.5);
    let matrix = view.camera_matrix(4, 1);
    assert_image(image_of(matrix, 0.5, 0.5), 0.5, 0.5);
    assert_image(image_of(matrix, 3.5, 0.5), 3.5, 0.5);
    // A default view is the identity for every image size, which is what
    // registration writes when nobody has called `set_view`.
    let wide = view.camera_matrix(2, 4);
    assert_image(image_of(wide, 0.5, 0.5), 0.5, 0.5);
}

#[test]
fn fit_letterboxes_a_2_by_4_image_into_an_8_canvas() {
    let mut view = View::new();
    view.fit(8.0, 8.0, 2, 4);
    assert_eq!(view.scale().to_bits(), 2.0_f64.to_bits());
    assert_point(view.offset(2, 4), 2.0, 0.0);
    // Image (0.5, 0.5) is the centre of texel (0, 0). It lands at canvas
    // (3, 1). The inverse, which the shader applies, returns the same image
    // point from that canvas position.
    assert_point(view.image_to_canvas(0.5, 0.5, 2, 4), 3.0, 1.0);
    assert_image(image_of(view.camera_matrix(2, 4), 3.0, 1.0), 0.5, 0.5);
}

#[test]
fn clockwise_quarter_turn_sends_the_origin_to_2_0() {
    let mut view = View::new();
    view.set_rotation(Radians::new(std::f64::consts::FRAC_PI_2));
    // Centre (1, 1). Vector (-1, -1) goes to (1, -1). Landing (2, 0).
    // The opposite sign, (dy, -dx), would land at (0, 2).
    assert_point(view.image_to_canvas(0.0, 0.0, 2, 2), 2.0, 0.0);
    assert_image(image_of(view.camera_matrix(2, 2), 2.0, 0.0), 0.0, 0.0);
}

#[test]
fn horizontal_flip_sends_image_x_0_to_x_2() {
    let mut view = View::new();
    view.set_flip_horizontal(true);
    assert_point(view.image_to_canvas(0.0, 1.0, 2, 2), 2.0, 1.0);
    assert_point(view.image_to_canvas(0.0, 0.0, 2, 2), 2.0, 0.0);
}

#[test]
fn fit_after_a_quarter_turn_letterboxes_the_swapped_bounds() {
    let mut view = View::new();
    view.set_rotation(Radians::new(std::f64::consts::FRAC_PI_2));
    view.fit(8.0, 8.0, 2, 4);
    // 2 by 4 becomes bounds 4 by 2. Scale 2. Offset (0, 2) in an 8 by 8 canvas.
    assert_eq!(view.scale().to_bits(), 2.0_f64.to_bits());
    assert_point(view.offset(2, 4), 0.0, 2.0);
}

#[test]
fn pan_moves_one_canvas_pixel_and_refuses_a_nonzero_z() {
    let mut view = View::new();
    assert!(matches!(
        view.set_pan(Pt::<Canvas>::new(0.0, 0.0, 1.0)),
        Err(ViewError::PanZ)
    ));
    assert!(view.set_pan(Pt::<Canvas>::new(1.0, 0.0, 0.0)).is_ok());
    assert_point(view.image_to_canvas(0.5, 0.5, 4, 1), 1.5, 0.5);
    // A refused pan must not replace the one already stored.
    assert!(matches!(
        view.set_pan(Pt::<Canvas>::new(0.0, 0.0, -1.0)),
        Err(ViewError::PanZ)
    ));
    assert_point(view.image_to_canvas(0.5, 0.5, 4, 1), 1.5, 0.5);
    // Negative zero is zero. It is not a slice index hiding in the pan.
    assert!(view.set_pan(Pt::<Canvas>::new(0.0, 0.0, -0.0)).is_ok());
    assert_point(view.image_to_canvas(0.5, 0.5, 4, 1), 0.5, 0.5);
}

#[test]
fn zoom_keeps_the_fitted_centre_and_doubles_the_scale() {
    let mut view = View::new();
    view.fit(8.0, 8.0, 2, 4);
    assert_point(view.image_to_canvas(1.0, 2.0, 2, 4), 4.0, 4.0);
    view.set_zoom(Zoom::new(2.0));
    assert_eq!(view.scale().to_bits(), 4.0_f64.to_bits());
    assert_point(view.image_to_canvas(1.0, 2.0, 2, 4), 4.0, 4.0);
    // Samples are not an input. The matrix is the view and the image size.
    let _matrix = view.camera_matrix(2, 4);
}

#[test]
fn spacing_scales_the_fitted_physical_size() {
    // Column spacing is the first argument. Physical size of a 2 by 2 image
    // at (0.5, 2.0) is 1 by 4. Scale is min(8/1, 8/4) = 2. Displayed size
    // is 2 by 8, so the x margin is (8 - 2) / 2 = 3.
    let mut view = View::new();
    view.set_spacing(0.5, 2.0);
    view.fit(8.0, 8.0, 2, 2);
    assert_eq!(view.scale().to_bits(), 2.0_f64.to_bits());
    assert_point(view.offset(2, 2), 3.0, 0.0);
}

#[test]
fn index_shift_is_an_axis_aligned_canvas_translation() {
    let mut view = View::new();
    view.set_index_shift(1.0, 0.0);
    assert_point(view.image_to_canvas(0.0, 0.0, 2, 2), 1.0, 0.0);
}

#[test]
fn zero_spacing_stays_finite() {
    let mut view = View::new();
    view.set_spacing(0.0, 1.0);
    view.set_index_shift(4.0, 3.0);
    let matrix = view.camera_matrix(1, 4);
    for value in matrix {
        assert!(value.is_finite());
    }
    // One column, centre `(0.5, 2)`, row shift 3. The empty axis stays on
    // the centre column. The row shift stays in the matrix.
    assert_image(image_of(matrix, 0.5, 5.0), 0.5, 2.0);
    assert_image(image_of(matrix, 9.0, 6.0), 0.5, 3.0);
    view.set_spacing(0.0, 0.0);
    let collapsed = view.camera_matrix(1, 4);
    for value in collapsed {
        assert!(value.is_finite());
    }
    assert_image(image_of(collapsed, 9.0, 6.0), 0.5, 2.0);
}

#[test]
fn a_coefficient_that_does_not_narrow_writes_the_image_centre() {
    let mut fitted = View::new();
    fitted.fit(512.0, 512.0, 512, 512);
    // The same fit at zoom 1 still names texel 0 from canvas 0.
    let kept = fitted.camera_matrix(512, 512);
    assert_image(image_of(kept, 0.0, 0.0), 0.0, 0.0);

    fitted.set_zoom(Zoom::new(1.0e-40));
    let zoomed = fitted.camera_matrix(512, 512);
    for value in zoomed {
        assert!(value.is_finite());
    }
    assert_image(image_of(zoomed, 0.0, 0.0), 256.0, 256.0);
    assert_image(image_of(zoomed, 100.0, 40.0), 256.0, 256.0);

    let mut panned = View::new();
    panned.fit(512.0, 512.0, 512, 512);
    assert!(panned.set_pan(Pt::<Canvas>::new(1.0e39, 0.0, 0.0)).is_ok());
    let shifted = panned.camera_matrix(512, 512);
    for value in shifted {
        assert!(value.is_finite());
    }
    assert_image(image_of(shifted, 0.0, 0.0), 256.0, 256.0);
}

#[test]
fn a_determinant_that_overflows_keeps_the_inverse() {
    // Scale 1e160 and pan 1e162. The anchor's 256 is lost in the pan sum,
    // and 1e162 / 1e160 is 100, so canvas 0 is image x 156. The y pan is 0,
    // so image y stays 256.
    let mut view = View::new();
    view.fit(512.0, 512.0, 512, 512);
    view.set_zoom(Zoom::new(1.0e160));
    assert!(view.set_pan(Pt::<Canvas>::new(1.0e162, 0.0, 0.0)).is_ok());
    let matrix = view.camera_matrix(512, 512);
    for value in matrix {
        assert!(value.is_finite());
    }
    assert_image(image_of(matrix, 0.0, 0.0), 156.0, 256.0);
}

#[test]
fn a_rank_one_basis_that_overflows_keeps_the_row() {
    // Column spacing is 0. Zoom 1e160 and a y pan of 1e162. The pan sum
    // loses the anchor, and 1e162 / 1e160 is 100, so canvas 0 is image y 156.
    // Image x stays on the centre column, 256.
    let mut view = View::new();
    view.set_spacing(0.0, 1.0);
    view.fit(512.0, 512.0, 512, 512);
    view.set_zoom(Zoom::new(1.0e160));
    assert!(view.set_pan(Pt::<Canvas>::new(0.0, 1.0e162, 0.0)).is_ok());
    let matrix = view.camera_matrix(512, 512);
    for value in matrix {
        assert!(value.is_finite());
    }
    assert_image(image_of(matrix, 0.0, 0.0), 256.0, 156.0);
}
