//! F-209 linked viewports. Every expected number is worked by hand below.
//!
//! PS3.3 C.7.6.2.1.1, Equation C.7.6.2.1-1: the patient point of column `i`
//! and row `j` is `S + i * Δi * X + j * Δj * Y`, where `S` is Image Position
//! (Patient), the centre of the first voxel transmitted, `X` the row
//! direction cosine, `Y` the column direction cosine, `Δi` the column pixel
//! resolution and `Δj` the row pixel resolution. PS3.3 10.7.1.3: the first
//! Pixel Spacing value is the row spacing and the second the column
//! spacing, so `Δj` is `PixelSpacing[0]` and `Δi` is `PixelSpacing[1]`.
//!
//! C.7.6.2.1.1 spans the image plane with `X` and `Y` and requires them
//! orthogonal and of unit length, so `X cross Y` is a unit normal of that
//! plane. The section does not name a normal. Its sign is not used here,
//! because every comparison below takes the absolute value of a dot product.
//!
//! The thresholds are cornerstone3D 5.8.2's: `areViewportsCoplanar` needs
//! `abs(dot) > 0.9`, and `ReferenceLinesTool` uses `EPSILON = 1e-3` for
//! `isParallel` and `isPerpendicular`.
//!
//! Planes used throughout. Axial: orientation `1\0\0\0\1\0`, Pixel Spacing
//! `0.7\0.5`, 96 rows and 128 columns. Coronal `C`: position `0\20\30`,
//! orientation `1\0\0\0\0\-1`, Pixel Spacing `2.5\0.5`, 13 rows and 128
//! columns, normal `(1,0,0) x (0,0,-1) = (0,1,0)`. Sagittal `G`: position
//! `10\0\30`, orientation `0\1\0\0\0\-1`, Pixel Spacing `2.5\0.7`, 13 rows
//! and 96 columns, normal `(0,1,0) x (0,0,-1) = (-1,0,0)`.

use ocelli_core::{Index, Pt};
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, PixelError,
    PixelSpacing,
};
use ocelli_viewport::link::{
    COPLANAR_DOT, LinkError, PLANE_EPSILON, closest_coplanar, reference_segment,
    same_frame_of_reference,
};
use ocelli_volume::FrameOfReferenceUid;

mod link {
    use super::axial;
    use super::{
        COPLANAR_DOT, FrameOfReferenceUid, Index, LinkError, PLANE_EPSILON, Pt, closest_coplanar,
        coronal, near, plane, reference_segment, sagittal, same_frame_of_reference,
    };

    #[test]
    fn the_thresholds_are_the_oracle_constants() {
        assert_eq!(COPLANAR_DOT.to_bits(), 0.9_f64.to_bits());
        assert_eq!(PLANE_EPSILON.to_bits(), 1e-3_f64.to_bits());
    }

    #[test]
    fn closest_keeps_the_lower_index_on_a_tie() -> Result<(), String> {
        // AC1. Source axial at z = 7.5 against a stack at z = 0, 3, 6, 9 and
        // 12. Distances are 7.5, 4.5, 1.5, 1.5 and 4.5. Index 2 and index 3
        // tie at 1.5, and the strict `<` of `_getClosestImageIdIndex` keeps
        // the first, 2.
        let source = axial(7.5)?;
        let stack = [
            axial(0.0)?,
            axial(3.0)?,
            axial(6.0)?,
            axial(9.0)?,
            axial(12.0)?,
        ];
        let shown = stack[0];
        assert_eq!(closest_coplanar(&source, &shown, &stack), Ok(2));
        // Control: at z = 7.6 index 3 is nearer, 1.4 against 1.6.
        let source = axial(7.6)?;
        assert_eq!(closest_coplanar(&source, &shown, &stack), Ok(3));
        Ok(())
    }

    #[test]
    fn coplanar_needs_more_than_0_9() -> Result<(), String> {
        // AC2. The source is axial, normal (0,0,1). A shown plane with row
        // (1,0,0) and column (0,0.8,0.6) has normal
        // (1,0,0) x (0,0.8,0.6) = (0*0.6 - 0*0.8, 0*0 - 1*0.6, 1*0.8 - 0*0)
        // = (0,-0.6,0.8), dot 0.8, which is not above 0.9. Column
        // (0,0.96,0.28) gives (0,-0.28,0.96), dot 0.96, which is.
        let source = axial(0.0)?;
        let stack = [axial(0.0)?];
        let tilted = plane(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.8, 0.6],
            0.7,
            0.5,
            96,
            128,
        )?;
        assert_eq!(
            closest_coplanar(&source, &tilted, &stack),
            Err(LinkError::NotCoplanar)
        );
        let nearly = plane(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.96, 0.28],
            0.7,
            0.5,
            96,
            128,
        )?;
        assert_eq!(closest_coplanar(&source, &nearly, &stack), Ok(0));
        // The sign of the normal does not matter: a flipped column gives
        // normal (0,0,-1) and dot -1.
        let flipped = plane(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            0.7,
            0.5,
            96,
            128,
        )?;
        assert_eq!(closest_coplanar(&source, &flipped, &stack), Ok(0));
        Ok(())
    }

    #[test]
    fn orthogonal_series_do_not_scroll() -> Result<(), String> {
        // AC3. Axial (0,0,1) against coronal (0,1,0) is dot 0. The coronal
        // position (0,20,30) is a finite distance from the source, so a rule
        // by distance alone would answer 0.
        let source = axial(30.0)?;
        let shown = coronal()?;
        assert_eq!(
            closest_coplanar(&source, &shown, &[shown]),
            Err(LinkError::NotCoplanar)
        );
        Ok(())
    }

    #[test]
    fn an_empty_stack_is_degenerate() -> Result<(), String> {
        let source = axial(0.0)?;
        assert_eq!(
            closest_coplanar(&source, &source, &[]),
            Err(LinkError::Degenerate)
        );
        Ok(())
    }

    #[test]
    fn axial_cuts_coronal_at_row_10() -> Result<(), String> {
        // AC4. Source axial at z = 5. Corners by Equation C.7.6.2.1-1 with
        // Δi = 0.5 and Δj = 0.7: TL (0,0,5), TR = TL + 127 * 0.5 * X =
        // (63.5,0,5), BL = TL + 95 * 0.7 * Y = (0,66.5,5), BR (63.5,66.5,5).
        // The top-bottom vector TL - BL = (0,-66.5,0), normalised (0,-1,0),
        // has dot -1 with the target normal (0,1,0), so it is not
        // perpendicular and the edges are TL to BL and TR to BR. The target
        // plane is y = 20, so the cuts are (0,20,5) and (63.5,20,5).
        // Into `C`'s index space: P - S = (0,0,-25) and (63.5,0,-25).
        // i = (P - S).X / Δi = 0 / 0.5 = 0 and 63.5 / 0.5 = 127.
        // j = (P - S).Y / Δj = ((0,0,-25).(0,0,-1)) / 2.5 = 25 / 2.5 = 10.
        // Swapping the two spacings would give j = 25 / 0.5 = 50.
        let (start, end) = segment(&axial(5.0)?, &coronal()?)?;
        near(start, 0.0, 10.0, "start")?;
        near(end, 127.0, 10.0, "end")
    }

    #[test]
    fn sagittal_cuts_coronal_at_column_20() -> Result<(), String> {
        // AC5. Source `G` with Δi = 0.7 and Δj = 2.5: TL (10,0,30), TR =
        // TL + 95 * 0.7 * (0,1,0) = (10,66.5,30), BL = TL + 12 * 2.5 *
        // (0,0,-1) = (10,0,0), BR (10,66.5,0). TL - BL = (0,0,30), normalised
        // (0,0,1), has dot 0 with (0,1,0), which is below 1e-3, so the edges
        // are TL to TR and BL to BR. The TL to BL edge lies in the plane
        // x = 10 at y = 0 and never meets y = 20, which is why the first
        // pair cannot be used. The cuts are (10,20,30) and (10,20,0).
        // P - S = (10,0,0) and (10,0,-30). i = 10 / 0.5 = 20.
        // j = 0 and 30 / 2.5 = 12.
        let (start, end) = segment(&sagittal()?, &coronal()?)?;
        near(start, 20.0, 0.0, "start")?;
        near(end, 20.0, 12.0, "end")
    }

    #[test]
    fn parallel_planes_have_no_line() -> Result<(), String> {
        // AC6. Two axial planes have normals with dot 1, above 1 - 1e-3.
        assert_eq!(
            reference_segment(&axial(0.0)?, &axial(5.0)?),
            Err(LinkError::Parallel)
        );
        Ok(())
    }

    #[test]
    fn an_empty_uid_links_nothing() {
        // AC7. PS3.3 C.7.4.1.1.1: the UID identifies a Frame of Reference.
        // A missing one identifies none, so two missing UIDs are not one
        // frame.
        let a = FrameOfReferenceUid("2.25.44000".to_owned());
        let b = FrameOfReferenceUid("2.25.44000".to_owned());
        let c = FrameOfReferenceUid("2.25.44001".to_owned());
        assert!(!same_frame_of_reference(None, None));
        assert!(!same_frame_of_reference(Some(&a), None));
        assert!(!same_frame_of_reference(None, Some(&a)));
        assert!(same_frame_of_reference(Some(&a), Some(&b)));
        assert!(!same_frame_of_reference(Some(&a), Some(&c)));
    }

    #[test]
    fn a_singleton_axis_target_has_no_line() -> Result<(), String> {
        // AC23. A target with 1 row, row spacing 0 and 128 columns. PS3.3
        // 10.7.1.3 allows a zero spacing only for a single row or column.
        // `index_to_world` puts a unit extension on that axis, so an index
        // read through it would be millimetres, not rows. The coronal
        // orientation is used so the axial source is not parallel to it.
        let target = plane(
            [0.0, 20.0, 30.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            0.0,
            0.5,
            1,
            128,
        )?;
        assert_eq!(
            reference_segment(&axial(5.0)?, &target),
            Err(LinkError::Degenerate)
        );
        Ok(())
    }

    fn segment(
        source: &super::ImagePlane,
        target: &super::ImagePlane,
    ) -> Result<(Pt<Index>, Pt<Index>), String> {
        reference_segment(source, target).map_err(|error| format!("refused: {error:?}"))
    }
}

/// An axial plane of the F-206 synthetic geometry at height `z`.
fn axial(z: f64) -> Result<ImagePlane, String> {
    plane(
        [0.0, 0.0, z],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        0.7,
        0.5,
        96,
        128,
    )
}

fn coronal() -> Result<ImagePlane, String> {
    plane(
        [0.0, 20.0, 30.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        2.5,
        0.5,
        13,
        128,
    )
}

fn sagittal() -> Result<ImagePlane, String> {
    plane(
        [10.0, 0.0, 30.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, -1.0],
        2.5,
        0.7,
        13,
        96,
    )
}

/// `row_mm` and `column_mm` in Pixel Spacing's own order, PS3.3 10.7.1.3.
fn plane(
    position: [f64; 3],
    row: [f64; 3],
    column: [f64; 3],
    row_mm: f64,
    column_mm: f64,
    rows: u32,
    columns: u32,
) -> Result<ImagePlane, String> {
    let built = || -> Result<ImagePlane, PixelError> {
        ImagePlane::new(
            ImagePositionPatient::new(position)?,
            ImageOrientationPatient::new(row, column)?,
            PixelSpacing::new(row_mm, column_mm)?,
            ImageDimensions::new(rows, columns)?,
        )
    };
    built().map_err(|error| format!("the fixture plane was refused: {error:?}"))
}

/// Within 1e-9 of the hand-computed column and row, and on the plane.
fn near(point: Pt<Index>, column: f64, row: f64, name: &str) -> Result<(), String> {
    let close = |got: f64, want: f64| (got - want).abs() < 1e-9;
    if close(point.x, column) && close(point.y, row) && close(point.z, 0.0) {
        Ok(())
    } else {
        Err(format!(
            "{name}: ({}, {}, {}) is not ({column}, {row}, 0)",
            point.x, point.y, point.z
        ))
    }
}

/// Both sides of `PLANE_EPSILON`, for the parallel test on the normals and
/// for the perpendicular test that chooses the edge pair. The cosines are
/// rational, `(m^2 - 1) / (m^2 + 1)` and `2m / (m^2 + 1)`, so every expected
/// point below is an exact fraction, written out.
mod epsilon {
    use super::{LinkError, near, plane, reference_segment};

    /// `m` to the pair `((m^2 - 1) / (m^2 + 1), 2m / (m^2 + 1))`, unit length.
    fn rational(m: f64) -> (f64, f64) {
        let square = m * m;
        ((square - 1.0) / (square + 1.0), 2.0 * m / (square + 1.0))
    }

    #[test]
    fn a_dot_just_above_1_less_epsilon_is_parallel() -> Result<(), String> {
        // Source axial, normal (0,0,1). Target row (1,0,0), column (0,c,s),
        // normal (0,-s,c), so the dot is c. m = 50 gives c = 2499 / 2501 =
        // 0.99920..., above 1 - 1e-3 = 0.999.
        let (c, s) = rational(50.0);
        let target = plane([0.0; 3], [1.0, 0.0, 0.0], [0.0, c, s], 1.0, 1.0, 13, 128)?;
        let source = super::axial(5.0)?;
        if reference_segment(&source, &target) != Err(LinkError::Parallel) {
            return Err("a dot of 0.99920 was not parallel".to_owned());
        }
        Ok(())
    }

    #[test]
    fn a_dot_just_below_1_less_epsilon_has_a_line() -> Result<(), String> {
        // m = 40 gives c = 1599 / 1601 = 0.99875..., below 0.999, so there
        // is a line. Target position 0, spacing 1\1. The source is axial at
        // z = 5 (corners from Δi = 0.5, Δj = 0.7). TL - BL is (0,-66.5,0),
        // dot with (0,-s,c) is 66.5 s, s = 80 / 1601, so not perpendicular
        // and the edges are TL to BL and TR to BR. The plane is -s y + c z
        // = 0, so y = 5 c / s = 5 * 1599 / 80 = 1599 / 16 = 99.9375, at
        // x = 0 and x = 63.5. Column i = x / 1. Row j = (P - S).Y / 1 =
        // 99.9375 c + 5 s = (159800.0625 + 400) / 1601 = 1601 / 16 =
        // 100.0625.
        let (c, s) = rational(40.0);
        let target = plane([0.0; 3], [1.0, 0.0, 0.0], [0.0, c, s], 1.0, 1.0, 13, 128)?;
        let (start, end) = reference_segment(&super::axial(5.0)?, &target)
            .map_err(|error| format!("refused: {error:?}"))?;
        near(start, 0.0, 100.0625, "start")?;
        near(end, 63.5, 100.0625, "end")
    }

    /// A sagittal-like source at `10\0\30`, 13 rows at 2.5 mm and 96 columns
    /// at 0.7 mm, with row (0,p,q) and column (0,q,-p). Its normal is
    /// (-1,0,0), dot 0 with `C`'s (0,1,0), so it is never parallel to `C`.
    /// TL - BL is -12 * 2.5 * column, normalised (0,-q,p), whose dot with
    /// (0,1,0) is -q, so `q` alone decides the edge pair.
    fn tilted_sagittal(m: f64) -> Result<super::ImagePlane, String> {
        let (p, q) = rational(m);
        plane(
            [10.0, 0.0, 30.0],
            [0.0, p, q],
            [0.0, q, -p],
            2.5,
            0.7,
            13,
            96,
        )
    }

    #[test]
    fn an_edge_just_inside_epsilon_is_perpendicular() -> Result<(), String> {
        // m = 4000: q = 8000 / 16000001 = 4.99999969e-4, below 1e-3, so the
        // edges are TL to TR and BL to BR. TL (10,0,30), TR = TL + 66.5
        // (0,p,q), BL = TL + 30 (0,q,-p). TL to TR meets y = 20 at
        // t = 20 / (66.5 p), z = 30 + 20 q / p. BL to BR meets it at
        // z = 30 - 30 p + q (20 - 30 q) / p. In `C`, i = 10 / 0.5 = 20 and
        // j = (30 - z) / 2.5, so j = -8 q / p = -64000 / 15999999 =
        // -0.00400000025000002 and j = (12 - 8 q) / p = 191936012 /
        // 15999999 = 11.9960014997501.
        let (start, end) = reference_segment(&tilted_sagittal(4000.0)?, &super::coronal()?)
            .map_err(|error| format!("refused: {error:?}"))?;
        near(start, 20.0, -64000.0 / 15_999_999.0, "start")?;
        near(end, 20.0, 191_936_012.0 / 15_999_999.0, "end")
    }

    #[test]
    fn an_edge_just_outside_epsilon_is_not_perpendicular() -> Result<(), String> {
        // m = 1000: q = 2000 / 1000001 = 1.999998e-3, above 1e-3, so the
        // edges are TL to BL and TR to BR. TL to BL meets y = 20 at
        // t = 20 / (30 q), z = 30 - 20 p / q, so j = 8 p / q =
        // 8 * 999999 / 2000 = 3999.996. TR to BR meets it at
        // t = (20 - 66.5 p) / (30 q), z = 30 + 66.5 q - p (20 - 66.5 p) / q,
        // so j = (20 p - 66.5) / (2.5 q) = -93000173 / 10000 = -9300.0173.
        // i = 20 on both.
        let (start, end) = reference_segment(&tilted_sagittal(1000.0)?, &super::coronal()?)
            .map_err(|error| format!("refused: {error:?}"))?;
        near(start, 20.0, 3999.996, "start")?;
        near(end, 20.0, -9300.0173, "end")
    }
}
