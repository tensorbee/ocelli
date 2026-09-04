//! Image-plane geometry, hand-computed from DICOM PS3.3 C.7.6.2.1.1.
//!
//! # The specification
//!
//! PS3.3 C.7.6.2.1.1 maps a pixel of an image plane into the patient
//! coordinate system with
//!
//! ```text
//! | Px |   | Xx*di  Yx*dj  0  Sx |   | i |
//! | Py | = | Xy*di  Yy*dj  0  Sy | * | j |
//! | Pz |   | Xz*di  Yz*dj  0  Sz |   | 0 |
//! | 1  |   | 0      0      0  1  |   | 1 |
//! ```
//!
//! where
//!
//! - `S` is ImagePositionPatient (0020,0032), the centre of the first
//!   transmitted voxel, not its corner,
//! - `X` is the ROW direction cosine, ImageOrientationPatient (0020,0037)
//!   elements 0 to 2,
//! - `Y` is the COLUMN direction cosine, ImageOrientationPatient elements
//!   3 to 5,
//! - `i` is the COLUMN index and `j` is the ROW index,
//! - `di` is the column pixel resolution and `dj` the row pixel resolution.
//!
//! PixelSpacing (0028,0030) is "adjacent row spacing \ adjacent column
//! spacing", so `PixelSpacing[0]` is the distance between adjacent rows,
//! which is `dj`, the number that multiplies the COLUMN direction cosine.
//! `PixelSpacing[1]` is `di` and multiplies the ROW direction cosine. That
//! crossing is the whole reason this fixture exists. The coordinate system is
//! LPS and the units are millimetres.
//!
//! # The frame this fixture uses
//!
//! Deliberately oblique, with a non-square pixel, so that transposing the two
//! spacings or swapping the two cosines moves the answer by far more than the
//! 1e-6 mm tolerance of HLD section 25.1. A square pixel could not tell the
//! transposition apart at all.
//!
//! ```text
//! ImagePositionPatient   (0020,0032) = -45.2 \ 118.7 \ -32.5
//! ImageOrientationPatient(0020,0037) = 0.6 \ -0.64 \ 0.48 \ 0.8 \ 0.48 \ -0.36
//! PixelSpacing           (0028,0030) = 0.5 \ 0.25
//! ```
//!
//! The two cosines are a genuine orthonormal pair, checked below as part of
//! the fixture rather than asserted in prose:
//!
//! ```text
//! X.X = 0.36 + 0.4096 + 0.2304 = 1
//! Y.Y = 0.64 + 0.2304 + 0.1296 = 1
//! X.Y = 0.48 - 0.3072 - 0.1728 = 0
//! ```
//!
//! # The hand computation
//!
//! The two step vectors, each an exact terminating decimal:
//!
//! ```text
//! di * X = 0.25 * ( 0.6, -0.64,  0.48) = ( 0.15, -0.16,  0.12)   per column
//! dj * Y = 0.5  * ( 0.8,  0.48, -0.36) = ( 0.4,   0.24, -0.18)   per row
//! ```
//!
//! so `P(i, j) = S + i * (0.15, -0.16, 0.12) + j * (0.4, 0.24, -0.18)`.
//!
//! ```text
//! P(0, 0)     = (-45.2,                118.7,                 -32.5)
//!             = (-45.2,                118.7,                 -32.5)
//!
//! P(1, 0)     = (-45.2 + 0.15,         118.7 - 0.16,          -32.5 + 0.12)
//!             = (-45.05,               118.54,                -32.38)
//!
//! P(0, 1)     = (-45.2 + 0.4,          118.7 + 0.24,          -32.5 - 0.18)
//!             = (-44.8,                118.94,                -32.68)
//!
//! P(255, 191): 255 * ( 0.15, -0.16,  0.12) = ( 38.25, -40.8,   30.6)
//!              191 * ( 0.4,   0.24, -0.18) = ( 76.4,   45.84, -34.38)
//!             = (-45.2 + 38.25 + 76.4,
//!                118.7 - 40.8   + 45.84,
//!               -32.5  + 30.6   - 34.38)
//!             = ( 69.45,              123.74,                 -36.28)
//! ```
//!
//! Every value above is an exact decimal, so a reviewer can redo the
//! arithmetic on paper without running anything. It was cross-checked with
//! exact rational arithmetic (`fractions.Fraction`), not with floating point,
//! and never against this crate's output.

use glam::{DMat4, DVec4};
use ocelli_core::{Index, Pt, Transform, World};

/// HLD section 25.1, the geometry row that actually governs here: "world
/// coordinates within 1e-6 mm". Every point this file asserts on is a
/// `Pt<World>` in LPS millimetres, so this is the right row and the right
/// unit.
const TOLERANCE_MM: f64 = 1e-6;

/// The bound on the orthonormality checks below, which are dot products of
/// unit vectors and therefore dimensionless. HLD section 25.1 has no row for
/// them, and borrowing the millimetre one would carry a unit into a place it
/// does not belong. The value is the same 1e-6 and the cosines are exact
/// terminating decimals, so nothing is at risk either way.
const DIMENSIONLESS_TOLERANCE: f64 = 1e-6;

/// ImagePositionPatient (0020,0032), LPS millimetres.
const IMAGE_POSITION_PATIENT: (f64, f64, f64) = (-45.2, 118.7, -32.5);

/// ImageOrientationPatient (0020,0037) elements 0 to 2, the ROW direction
/// cosine. Stepping one COLUMN moves along this.
const ROW_COSINE: (f64, f64, f64) = (0.6, -0.64, 0.48);

/// ImageOrientationPatient (0020,0037) elements 3 to 5, the COLUMN direction
/// cosine. Stepping one ROW moves along this.
const COLUMN_COSINE: (f64, f64, f64) = (0.8, 0.48, -0.36);

/// PixelSpacing (0028,0030) element 0, the distance between adjacent ROWS.
/// PS3.3 calls this `dj` and it multiplies the COLUMN direction cosine.
const SPACING_BETWEEN_ROWS: f64 = 0.5;

/// PixelSpacing (0028,0030) element 1, the distance between adjacent COLUMNS.
/// PS3.3 calls this `di` and it multiplies the ROW direction cosine.
const SPACING_BETWEEN_COLUMNS: f64 = 0.25;

/// The PS3.3 C.7.6.2.1.1 matrix for the frame above.
///
/// `Pt::<Index>::new(i, j, k)` carries the COLUMN index in `x` and the ROW
/// index in `y`, matching the order of the standard's column vector. The
/// third column of the matrix is zero because C.7.6.2.1.1 is an in-plane
/// equation and says nothing about `k`. Slice stepping is a volume concern
/// and belongs to whoever builds one.
fn image_plane_transform() -> Transform<Index, World> {
    let (xx, xy, xz) = ROW_COSINE;
    let (yx, yy, yz) = COLUMN_COSINE;
    let (sx, sy, sz) = IMAGE_POSITION_PATIENT;
    let di = SPACING_BETWEEN_COLUMNS;
    let dj = SPACING_BETWEEN_ROWS;

    // glam's DMat4 is column major, and from_cols takes the columns of the
    // matrix printed in the doc comment above.
    Transform::from_mat4(DMat4::from_cols(
        DVec4::new(xx * di, xy * di, xz * di, 0.0),
        DVec4::new(yx * dj, yy * dj, yz * dj, 0.0),
        DVec4::ZERO,
        DVec4::new(sx, sy, sz, 1.0),
    ))
}

/// The same matrix with the two PixelSpacing elements transposed. Nothing in
/// the library uses it. It exists so that `the_transposition_is_visible`
/// can measure how far the defect this fixture hunts would move the answer.
fn transposed_spacing_transform() -> Transform<Index, World> {
    let (xx, xy, xz) = ROW_COSINE;
    let (yx, yy, yz) = COLUMN_COSINE;
    let (sx, sy, sz) = IMAGE_POSITION_PATIENT;
    let di = SPACING_BETWEEN_ROWS;
    let dj = SPACING_BETWEEN_COLUMNS;

    Transform::from_mat4(DMat4::from_cols(
        DVec4::new(xx * di, xy * di, xz * di, 0.0),
        DVec4::new(yx * dj, yy * dj, yz * dj, 0.0),
        DVec4::ZERO,
        DVec4::new(sx, sy, sz, 1.0),
    ))
}

#[track_caller]
fn assert_world_within_tolerance(actual: Pt<World>, expected: (f64, f64, f64), case: &str) {
    let (ex, ey, ez) = expected;
    assert!(
        (actual.x - ex).abs() < TOLERANCE_MM,
        "{case}: x was {} and PS3.3 gives {ex}",
        actual.x
    );
    assert!(
        (actual.y - ey).abs() < TOLERANCE_MM,
        "{case}: y was {} and PS3.3 gives {ey}",
        actual.y
    );
    assert!(
        (actual.z - ez).abs() < TOLERANCE_MM,
        "{case}: z was {} and PS3.3 gives {ez}",
        actual.z
    );
}

/// The fixture's own direction cosines are a valid DICOM frame. If this fails
/// the rest of the file is testing arithmetic against a frame no scanner
/// would emit.
#[test]
fn the_fixture_frame_is_orthonormal() {
    let (xx, xy, xz) = ROW_COSINE;
    let (yx, yy, yz) = COLUMN_COSINE;

    assert!(
        (xx * xx + xy * xy + xz * xz - 1.0).abs() < DIMENSIONLESS_TOLERANCE,
        "row cosine is not a unit vector"
    );
    assert!(
        (yx * yx + yy * yy + yz * yz - 1.0).abs() < DIMENSIONLESS_TOLERANCE,
        "column cosine is not a unit vector"
    );
    assert!(
        (xx * yx + xy * yy + xz * yz).abs() < DIMENSIONLESS_TOLERANCE,
        "the two cosines are not orthogonal"
    );
}

/// PS3.3 C.7.6.2.1.1 with `i = 0, j = 0` leaves only `S`. ImagePositionPatient
/// is the centre of the first voxel, so no half-pixel offset appears here.
#[test]
fn voxel_0_0_is_image_position_patient() {
    let p = image_plane_transform().apply(Pt::<Index>::new(0.0, 0.0, 0.0));
    assert_world_within_tolerance(p, (-45.2, 118.7, -32.5), "P(0, 0)");
}

/// One step in `i` is one COLUMN, so it moves by `PixelSpacing[1]` along the
/// ROW direction cosine. Getting the spacing index wrong here doubles the
/// step, and getting the cosine wrong sends it in a different direction.
#[test]
fn voxel_1_0_steps_one_column_by_pixel_spacing_1_along_the_row_cosine() {
    let p = image_plane_transform().apply(Pt::<Index>::new(1.0, 0.0, 0.0));
    assert_world_within_tolerance(p, (-45.05, 118.54, -32.38), "P(1, 0)");
}

/// One step in `j` is one ROW, so it moves by `PixelSpacing[0]` along the
/// COLUMN direction cosine.
#[test]
fn voxel_0_1_steps_one_row_by_pixel_spacing_0_along_the_column_cosine() {
    let p = image_plane_transform().apply(Pt::<Index>::new(0.0, 1.0, 0.0));
    assert_world_within_tolerance(p, (-44.8, 118.94, -32.68), "P(0, 1)");
}

/// A voxel far from the origin, where both terms are large and an error in
/// either one is unmissable.
#[test]
fn voxel_255_191_accumulates_both_terms() {
    let p = image_plane_transform().apply(Pt::<Index>::new(255.0, 191.0, 0.0));
    assert_world_within_tolerance(p, (69.45, 123.74, -36.28), "P(255, 191)");
}

/// The fixture can actually see the defect it was built for. With
/// `PixelSpacing` transposed, every one of the three interior cases moves by
/// orders of magnitude more than the 1e-6 mm tolerance, so a transposition
/// cannot pass by hiding inside the tolerance.
#[test]
fn the_transposition_is_visible_at_this_tolerance() {
    let correct = image_plane_transform();
    let wrong = transposed_spacing_transform();

    for (i, j) in [(1.0, 0.0), (0.0, 1.0), (255.0, 191.0)] {
        let a = correct.apply(Pt::<Index>::new(i, j, 0.0));
        let b = wrong.apply(Pt::<Index>::new(i, j, 0.0));
        let worst = (a.x - b.x)
            .abs()
            .max((a.y - b.y).abs())
            .max((a.z - b.z).abs());
        assert!(
            worst > TOLERANCE_MM * 1000.0,
            "at ({i}, {j}) a transposed PixelSpacing moves the point by only {worst} mm, \
             which is too little for this fixture to be evidence"
        );
    }
}
