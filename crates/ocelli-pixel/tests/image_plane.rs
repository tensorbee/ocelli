//! DICOM PS3.3 C.7.6.2.1.1 image-plane fixtures.

use ocelli_core::{Index, Pt};
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, PixelSpacing,
};

const TOLERANCE_MM: f64 = 1e-6;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok());
        let Ok(value) = result else { return };
        value
    }};
}

#[test]
fn non_square_spacing_uses_column_spacing_for_i_and_row_spacing_for_j() {
    let plane = require_ok!(ImagePlane::new(
        require_ok!(ImagePositionPatient::new([10.0, 20.0, 30.0])),
        require_ok!(ImageOrientationPatient::new(
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0]
        )),
        require_ok!(PixelSpacing::new(2.0, 0.5)),
        require_ok!(ImageDimensions::new(8, 9)),
    ));

    // PS3.3 C.7.6.2.1.1:
    // P = [10,20,30] + 4 * 0.5 * [1,0,0] + 3 * 2 * [0,1,0].
    let world = plane
        .index_to_world()
        .apply(Pt::<Index>::new(4.0, 3.0, 0.0));
    assert!((world.x - 12.0).abs() < TOLERANCE_MM);
    assert!((world.y - 26.0).abs() < TOLERANCE_MM);
    assert!((world.z - 30.0).abs() < TOLERANCE_MM);

    let origin = plane
        .index_to_world()
        .apply(Pt::<Index>::new(0.0, 0.0, 0.0));
    assert!((origin.x - 10.0).abs() < TOLERANCE_MM);
    assert!((origin.y - 20.0).abs() < TOLERANCE_MM);
    assert!((origin.z - 30.0).abs() < TOLERANCE_MM);
}

#[test]
fn invertible_plane_transform_round_trips_representative_index_points() {
    let plane = require_ok!(ImagePlane::new(
        require_ok!(ImagePositionPatient::new([-4.0, 8.0, 12.0])),
        require_ok!(ImageOrientationPatient::new(
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0]
        )),
        require_ok!(PixelSpacing::new(0.75, 0.25)),
        require_ok!(ImageDimensions::new(16, 32)),
    ));
    let transform = plane.index_to_world();
    let inverse = transform.inverse();

    for original in [
        Pt::<Index>::new(0.0, 0.0, 0.0),
        Pt::<Index>::new(7.5, 3.25, 0.0),
        Pt::<Index>::new(31.0, 15.0, 2.0),
    ] {
        let round_trip = inverse.apply(transform.apply(original));
        assert!((round_trip.x - original.x).abs() < TOLERANCE_MM);
        assert!((round_trip.y - original.y).abs() < TOLERANCE_MM);
        assert!((round_trip.z - original.z).abs() < TOLERANCE_MM);
    }
}

#[test]
fn direction_cosine_domain_accepts_rounding_noise_and_rejects_scale_or_skew() {
    // PS3.3 C.7.6.2.1.1 requires normal, mutually orthogonal direction
    // cosines. These oblique values are a conceptual orthonormal pair rounded
    // component-wise to six decimal places.
    let accepted = require_ok!(ImageOrientationPatient::new(
        [0.800_831, 0.390_592, 0.453_990],
        [0.114_807, -0.844_119, 0.523_720]
    ));
    let plane = require_ok!(ImagePlane::new(
        require_ok!(ImagePositionPatient::new([0.0, 0.0, 0.0])),
        accepted,
        require_ok!(PixelSpacing::new(1.0, 1.0)),
        require_ok!(ImageDimensions::new(2, 2)),
    ));
    let transform = plane.index_to_world();
    let along_row = transform.apply(Pt::<Index>::new(1.0, 0.0, 0.0));
    let along_column = transform.apply(Pt::<Index>::new(0.0, 1.0, 0.0));

    // Accepted rounding noise must not survive as scale or shear in the
    // generated basis.
    let row_length_squared =
        along_row.x * along_row.x + along_row.y * along_row.y + along_row.z * along_row.z;
    let column_length_squared = along_column.x * along_column.x
        + along_column.y * along_column.y
        + along_column.z * along_column.z;
    let dot =
        along_row.x * along_column.x + along_row.y * along_column.y + along_row.z * along_column.z;
    assert!((row_length_squared - 1.0).abs() < 1e-12);
    assert!((column_length_squared - 1.0).abs() < 1e-12);
    assert!(dot.abs() < 1e-12);
    assert!(ImageOrientationPatient::new([1.0 + 2.5e-6, 0.0, 0.0], [0.0, 1.0, 0.0]).is_err());
    assert!(ImageOrientationPatient::new([1.0, 0.0, 0.0], [2.5e-6, 1.0, 0.0]).is_err());
}

#[test]
fn zero_spacing_is_allowed_only_for_its_corresponding_singleton_dimension() {
    // DICOM PS3.3 10.7.1.3 permits zero only for the spacing corresponding to
    // a singleton row or column dimension.
    let position = require_ok!(ImagePositionPatient::new([0.0, 0.0, 0.0]));
    let orientation = require_ok!(ImageOrientationPatient::new(
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0]
    ));
    let row_zero = require_ok!(PixelSpacing::new(0.0, 1.0));
    let column_zero = require_ok!(PixelSpacing::new(1.0, 0.0));
    let both_zero = require_ok!(PixelSpacing::new(0.0, 0.0));

    let single_row = require_ok!(ImagePlane::new(
        position,
        orientation,
        row_zero,
        require_ok!(ImageDimensions::new(1, 2))
    ));
    let single_column = require_ok!(ImagePlane::new(
        position,
        orientation,
        column_zero,
        require_ok!(ImageDimensions::new(2, 1))
    ));
    assert!(
        ImagePlane::new(
            position,
            orientation,
            both_zero,
            require_ok!(ImageDimensions::new(1, 1))
        )
        .is_ok()
    );

    for (spacing, dimensions) in [
        (row_zero, require_ok!(ImageDimensions::new(2, 1))),
        (row_zero, require_ok!(ImageDimensions::new(2, 2))),
        (column_zero, require_ok!(ImageDimensions::new(1, 2))),
        (column_zero, require_ok!(ImageDimensions::new(2, 2))),
    ] {
        assert!(ImagePlane::new(position, orientation, spacing, dimensions).is_err());
    }

    for (plane, point) in [
        (single_row, Pt::<Index>::new(1.0, 0.0, 0.0)),
        (single_column, Pt::<Index>::new(0.0, 1.0, 0.0)),
    ] {
        let transform = plane.index_to_world();
        let round_trip = transform.inverse().apply(transform.apply(point));
        assert!((round_trip.x - point.x).abs() < TOLERANCE_MM);
        assert!((round_trip.y - point.y).abs() < TOLERANCE_MM);
        assert!((round_trip.z - point.z).abs() < TOLERANCE_MM);
    }
}
