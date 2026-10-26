//! F-048 canvas, index and world fixtures.
//!
//! DICOM PS3.3 C.7.6.2.1.1:
//! `P = IPP + i * column_spacing * X + j * row_spacing * Y`.
//! Integer `(i, j)` is the pixel centre. Image space stores the corner, so
//! that centre is image `(i + 0.5, j + 0.5)`. No half-voxel is added to `P`.
//!
//! `PixelSpacing::new` takes row millimetres, then column millimetres.
//! `View::set_spacing` takes column millimetres, then row millimetres.

use ocelli_core::{Canvas, Index, Pt, World};
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, PixelError,
    PixelSpacing,
};
use ocelli_viewport::view::{CoordinateError, Radians, View, Zoom};

const TOLERANCE_MM: f64 = 1e-6;
const QUARTER_PIXEL: f64 = 0.25;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok());
        let Ok(value) = result else { return };
        value
    }};
}

fn axial_plane() -> Result<ImagePlane, PixelError> {
    ImagePlane::new(
        ImagePositionPatient::new([10.0, 20.0, 30.0])?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(2.0, 0.5)?,
        ImageDimensions::new(8, 9)?,
    )
}

fn oblique_plane() -> Result<ImagePlane, PixelError> {
    let s = std::f64::consts::FRAC_1_SQRT_2;
    ImagePlane::new(
        ImagePositionPatient::new([10.0, 20.0, 30.0])?,
        ImageOrientationPatient::new([s, s, 0.0], [0.0, 0.0, 1.0])?,
        PixelSpacing::new(2.0, 0.5)?,
        ImageDimensions::new(8, 9)?,
    )
}

/// Hand application of PS3.3 C.7.6.2.1.1. Not a call into `ImagePlane`.
fn patient(i: f64, j: f64, row: [f64; 3], column: [f64; 3]) -> [f64; 3] {
    let column_spacing = 0.5;
    let row_spacing = 2.0;
    [
        10.0 + i * column_spacing * row[0] + j * row_spacing * column[0],
        20.0 + i * column_spacing * row[1] + j * row_spacing * column[1],
        30.0 + i * column_spacing * row[2] + j * row_spacing * column[2],
    ]
}

fn assert_near(got: f64, expect: f64, tolerance: f64) {
    assert!(
        (got - expect).abs() <= tolerance,
        "{got} against {expect} tolerance {tolerance}"
    );
}

fn view_matching_plane() -> View {
    let mut view = View::new();
    view.set_spacing(0.5, 2.0);
    view
}

#[test]
fn index_zero_is_ipp_and_its_canvas_point_is_the_texel_centre() {
    let plane = require_ok!(axial_plane());
    let view = view_matching_plane();
    let columns = plane.dimensions().columns();
    let rows = plane.dimensions().rows();
    let canvas =
        require_ok!(view.index_to_canvas(columns, rows)).apply(Pt::<Index>::new(0.0, 0.0, 0.0));
    let image = view.image_to_canvas(0.5, 0.5, columns, rows);
    assert_near(canvas.x, image.0, QUARTER_PIXEL);
    assert_near(canvas.y, image.1, QUARTER_PIXEL);
    let world = require_ok!(view.canvas_to_world(&plane))
        .apply(Pt::<Canvas>::new(canvas.x, canvas.y, canvas.z));
    assert_near(world.x, 10.0, TOLERANCE_MM);
    assert_near(world.y, 20.0, TOLERANCE_MM);
    assert_near(world.z, 30.0, TOLERANCE_MM);
}

#[test]
fn anisotropic_index_2_3_is_world_11_26_30() {
    // i = 2, column spacing 0.5, X = (1, 0, 0) contributes (1, 0, 0).
    // j = 3, row spacing 2, Y = (0, 1, 0) contributes (0, 6, 0).
    // IPP (10, 20, 30) becomes (11, 26, 30). Swapped spacing would be
    // (10 + 2*2, 20 + 3*0.5, 30) = (14, 21.5, 30).
    let expect = patient(2.0, 3.0, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    assert_near(expect[0], 11.0, TOLERANCE_MM);
    assert_near(expect[1], 26.0, TOLERANCE_MM);
    assert_near(expect[2], 30.0, TOLERANCE_MM);

    let plane = require_ok!(axial_plane());
    let view = view_matching_plane();
    let columns = plane.dimensions().columns();
    let rows = plane.dimensions().rows();
    let canvas =
        require_ok!(view.index_to_canvas(columns, rows)).apply(Pt::<Index>::new(2.0, 3.0, 0.0));
    // Independent of `image_to_canvas`. The plane is 8 rows by 9 columns, so
    // the unfitted anchor is the image centre (4.5, 4.0). Image (2.5, 3.5) is
    // offset (-2.0, -0.5). Column 0.5 and row 2.0 scale it to (-1.0, -1.0),
    // so canvas is (3.5, 3.0). A transposed `set_spacing(2.0, 0.5)` gives
    // (0.5, 3.75) and fails both assertions.
    assert_near(canvas.x, 3.5, QUARTER_PIXEL);
    assert_near(canvas.y, 3.0, QUARTER_PIXEL);
    let image = view.image_to_canvas(2.5, 3.5, columns, rows);
    assert_near(canvas.x, image.0, QUARTER_PIXEL);
    assert_near(canvas.y, image.1, QUARTER_PIXEL);
    let world =
        require_ok!(view.canvas_to_world(&plane)).apply(Pt::<Canvas>::new(canvas.x, canvas.y, 0.0));
    assert_near(world.x, expect[0], TOLERANCE_MM);
    assert_near(world.y, expect[1], TOLERANCE_MM);
    assert_near(world.z, expect[2], TOLERANCE_MM);
}

#[test]
fn oblique_pan_turn_flip_zoom_and_shift_keep_the_patient_point() {
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let expect = patient(2.0, 3.0, [s, s, 0.0], [0.0, 0.0, 1.0]);
    // 2 * 0.5 * (s, s, 0) + 3 * 2 * (0, 0, 1) = (s, s, 6).
    assert_near(expect[0], 10.0 + s, TOLERANCE_MM);
    assert_near(expect[1], 20.0 + s, TOLERANCE_MM);
    assert_near(expect[2], 36.0, TOLERANCE_MM);

    let plane = require_ok!(oblique_plane());
    let mut view = view_matching_plane();
    view.set_rotation(Radians::new(std::f64::consts::FRAC_PI_2));
    view.set_flip_horizontal(true);
    view.set_zoom(Zoom::new(2.0));
    assert!(view.set_pan(Pt::<Canvas>::new(3.0, -4.0, 0.0)).is_ok());
    view.set_index_shift(1.5, -0.25);
    let columns = plane.dimensions().columns();
    let rows = plane.dimensions().rows();
    let canvas =
        require_ok!(view.index_to_canvas(columns, rows)).apply(Pt::<Index>::new(2.0, 3.0, 0.0));
    let image = view.image_to_canvas(2.5, 3.5, columns, rows);
    assert_near(canvas.x, image.0, QUARTER_PIXEL);
    assert_near(canvas.y, image.1, QUARTER_PIXEL);
    let world =
        require_ok!(view.canvas_to_world(&plane)).apply(Pt::<Canvas>::new(canvas.x, canvas.y, 0.0));
    assert_near(world.x, expect[0], TOLERANCE_MM);
    assert_near(world.y, expect[1], TOLERANCE_MM);
    assert_near(world.z, expect[2], TOLERANCE_MM);

    let back = require_ok!(view.world_to_canvas(&plane))
        .apply(Pt::<World>::new(expect[0], expect[1], expect[2]));
    assert_near(back.x, canvas.x, QUARTER_PIXEL);
    assert_near(back.y, canvas.y, QUARTER_PIXEL);
}

#[test]
fn one_dpr_division_is_invariant_and_a_second_division_is_not() {
    let view = View::new();
    let map = require_ok!(view.canvas_to_index(4, 1));
    for dpr in [1.0, 1.25, 2.0] {
        let device_x = 1.0 * dpr;
        let device_y = 0.5 * dpr;
        let css_x = device_x / dpr;
        let css_y = device_y / dpr;
        let index = map.apply(Pt::<Canvas>::new(css_x, css_y, 0.0));
        assert_near(index.x, 0.5, TOLERANCE_MM);
        assert_near(index.y, 0.0, TOLERANCE_MM);
        let doubled = map.apply(Pt::<Canvas>::new(css_x / dpr, css_y / dpr, 0.0));
        let missing = map.apply(Pt::<Canvas>::new(device_x, device_y, 0.0));
        if dpr.to_bits() != 1.0_f64.to_bits() {
            assert!((doubled.x - index.x).abs() > TOLERANCE_MM);
            assert!((missing.x - index.x).abs() > TOLERANCE_MM);
        }
    }
}

#[test]
fn a_singular_map_is_an_error_and_the_view_stays() {
    let mut view = View::new();
    view.set_zoom(Zoom::new(0.0));
    assert!(matches!(
        view.canvas_to_index(4, 4),
        Err(CoordinateError::Singular)
    ));
    assert_eq!(view.scale().to_bits(), 0.0_f64.to_bits());
    let raster = view.camera_matrix(4, 4);
    assert!(raster.iter().all(|value| value.is_finite()));
    view.set_zoom(Zoom::new(1.0));
    view.set_spacing(0.0, 0.0);
    assert!(matches!(
        view.canvas_to_world(&require_ok!(axial_plane())),
        Err(CoordinateError::Singular)
    ));
}

#[test]
fn mapping_does_not_require_a_renderer_and_round_trips_inside_the_budget() {
    let plane = require_ok!(axial_plane());
    let view = view_matching_plane();
    let columns = plane.dimensions().columns();
    let rows = plane.dimensions().rows();
    let forward = require_ok!(view.index_to_canvas(columns, rows));
    let back = require_ok!(view.canvas_to_index(columns, rows));
    let index = Pt::<Index>::new(4.0, 3.0, 1.0);
    let canvas = forward.apply(index);
    let again = back.apply(Pt::<Canvas>::new(canvas.x, canvas.y, canvas.z));
    assert_near(again.x, index.x, TOLERANCE_MM);
    assert_near(again.y, index.y, TOLERANCE_MM);
    assert_near(again.z, index.z, TOLERANCE_MM);
}
