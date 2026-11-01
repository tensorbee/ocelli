//! F-053. An arbitrary patient plane is the level-0 trilinear sample.
//!
//! PS3.3 C.7.6.2.1.1. Identity volume, spacing `(1, 1, 1)`, origin `(0, 0, 0)`,
//! direction identity, `U8`. Bytes are x-fastest, then y, then z:
//! slice 0 is `[10, 30, 70, 90]` and slice 1 is `[110, 130, 150, 170]`.
//!
//! The mix plane has origin `(0.5, 0.5, 0.5)`, row `(1, 0, 0)`, column
//! `(0, 0, 1)`, `column_mm` 1 and `row_mm` 1. `P(0, 0)` is `(0.5, 0.5, 0.5)`.
//! All eight corners weigh `1/8`. `760 / 8 = 95`. A y = 0 snap is 70. A y = 1
//! snap is 120. Store index 1 changed from 30 to 38 adds 1, so the pixel is 96.
//! `P(1, 0)` is `(1.5, 0.5, 0.5)` and is outside the grid.
//!
//! `reslice_nearest` rounds `(0.5, 0.5, 0.5)` half away from zero to
//! `(1, 1, 1)`. That voxel is 170. The trilinear mix of the same point
//! stays 95.
//!
//! Slice 1 absent on that same pixel is `Absent`. Stored zero would be 25.
//! Renormalising the four present corners would be 50. The literals below are
//! 95, 96, 10 and 60. This file does not call `sample_at`.

use glam::{DMat3, DVec3};
use ocelli_core::{Pt, World};
use ocelli_volume::{
    Level, ObliquePlane, OrthoImage, OrthoPlane, PlaneSample, SlicePresence, Volume, VolumeError,
    VoxelKind, VoxelStore,
};

#[test]
fn half_voxel_oblique_is_the_trilinear_mix() {
    let mut volume = identity_volume(true);
    let plane = mix_plane();
    let image = sampled(&volume, plane);
    assert_eq!(image.width, 2);
    assert_eq!(image.height, 1);
    assert_eq!(image.samples.len(), 2);
    assert_world(volume.voxel_world([0.5, 0.5, 0.5]), (0.5, 0.5, 0.5));
    assert_eq!(present_bits(at(&image, 0, 0)), 95.0_f32.to_bits());
    assert_ne!(present_bits(at(&image, 0, 0)), 70.0_f32.to_bits());
    assert_ne!(present_bits(at(&image, 0, 0)), 120.0_f32.to_bits());
    assert!(matches!(at(&image, 1, 0), PlaneSample::Absent));
    let again = sampled(&volume, plane);
    assert_eq!(again, image);
    let before = slice_bits(&volume, 0);
    let _kept = sampled(&volume, plane);
    assert_eq!(slice_bits(&volume, 0), before);

    assert!(volume.place(0, &[10, 38, 70, 90]).is_ok());
    let mutated = sampled(&volume, plane);
    assert_eq!(present_bits(at(&mutated, 0, 0)), 96.0_f32.to_bits());
    assert!(matches!(at(&mutated, 1, 0), PlaneSample::Absent));
}

#[test]
fn nearest_half_index_reads_the_upper_voxel() {
    let volume = identity_volume(true);
    let image = volume.reslice_nearest(mix_plane());
    let Ok(image) = image else {
        unreachable!("the fixture plane is inside the identity volume")
    };
    assert_eq!(present_bits(at(&image, 0, 0)), 170.0_f32.to_bits());
    assert_ne!(present_bits(at(&image, 0, 0)), 95.0_f32.to_bits());
    assert!(matches!(at(&image, 1, 0), PlaneSample::Absent));
}

#[test]
fn an_absent_slice_is_not_stored_zero() {
    let volume = identity_volume(false);
    let image = sampled(&volume, mix_plane());
    assert!(matches!(at(&image, 0, 0), PlaneSample::Absent));
    for wrong in [0.0_f32, 25.0, 50.0, 95.0] {
        assert_ne!(sample_bits(at(&image, 0, 0)), wrong.to_bits());
    }
}

#[test]
fn an_integer_index_reads_one_voxel() {
    let mut volume = identity_volume(true);
    let image = sampled(&volume, integer_plane());
    assert_eq!(present_bits(at(&image, 0, 0)), 10.0_f32.to_bits());
    assert!(volume.place(0, &[10, 38, 70, 90]).is_ok());
    let mutated = sampled(&volume, integer_plane());
    assert_eq!(present_bits(at(&mutated, 0, 0)), 10.0_f32.to_bits());

    let partial = identity_volume(false);
    let still = sampled(&partial, integer_plane());
    assert_eq!(present_bits(at(&still, 0, 0)), 10.0_f32.to_bits());
}

#[test]
fn direction_moves_the_patient_point() {
    let mut volume = identity_volume(true);
    volume.direction = DMat3::from_cols(
        DVec3::new(0.0, 1.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
        DVec3::new(1.0, 0.0, 0.0),
    );
    assert_world(volume.voxel_world([0.0, 0.0, 0.5]), (0.5, 0.0, 0.0));
    let image = sampled(&volume, patient_point_plane());
    assert_eq!(present_bits(at(&image, 0, 0)), 60.0_f32.to_bits());
    assert_ne!(present_bits(at(&image, 0, 0)), 20.0_f32.to_bits());
}

#[test]
fn unequal_spacing_selects_each_axis() {
    // PS3.3 C.7.6.2.1.1. Spacing `(2, 4, 8)`, origin `(10, 20, 30)`.
    // Index `(0.25, 0.5, 0.75)` is patient `(10.5, 22, 36)`.
    // Weights are `0.75/0.25`, `0.5/0.5` and `0.25/0.75`. The eight
    // products of the identity bytes sum to `112.5`. Dropping the three
    // spacing divisors reads delta `(0.5, 2, 6)` and leaves the grid.
    let volume = spaced_volume([2.0, 4.0, 8.0], (10.0, 20.0, 30.0), DMat3::IDENTITY);
    assert_world(volume.voxel_world([0.25, 0.5, 0.75]), (10.5, 22.0, 36.0));
    let image = sampled(
        &volume,
        ObliquePlane {
            origin: Pt::<World>::new(10.5, 22.0, 36.0),
            row: DVec3::X,
            column: DVec3::Y,
            columns: 1,
            rows: 1,
            column_mm: 1.0,
            row_mm: 1.0,
        },
    );
    assert_eq!(present_bits(at(&image, 0, 0)), 112.5_f32.to_bits());

    // Columns `(0, 1, 0)`, `(0, 0, 1)`, `(1, 0, 0)`. Index `(0, 0, 0.5)`
    // is patient `(14, 20, 30)`. The two corners are 10 and 110, so the
    // sample is 60. A dropped `spacing[2]` divisor reads index 4.
    let mut turned = volume;
    turned.direction = DMat3::from_cols(
        DVec3::new(0.0, 1.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
        DVec3::new(1.0, 0.0, 0.0),
    );
    assert_world(turned.voxel_world([0.0, 0.0, 0.5]), (14.0, 20.0, 30.0));
    let along = sampled(
        &turned,
        ObliquePlane {
            origin: Pt::<World>::new(14.0, 20.0, 30.0),
            row: DVec3::X,
            column: DVec3::Y,
            columns: 1,
            rows: 1,
            column_mm: 1.0,
            row_mm: 1.0,
        },
    );
    assert_eq!(present_bits(at(&along, 0, 0)), 60.0_f32.to_bits());
}

#[test]
fn named_planes_stay_ortho() {
    let volume = identity_volume(true);
    let axial = volume.reslice(OrthoPlane::Axial { slice: 0.0 });
    let Ok(axial) = axial else {
        unreachable!("axial slice 0 is inside the identity volume")
    };
    let oblique = sampled(
        &volume,
        ObliquePlane {
            origin: Pt::<World>::new(0.0, 0.0, 0.0),
            row: DVec3::new(1.0, 0.0, 0.0),
            column: DVec3::new(0.0, 1.0, 0.0),
            columns: 2,
            rows: 2,
            column_mm: 1.0,
            row_mm: 1.0,
        },
    );
    assert_eq!(oblique.width, axial.width);
    assert_eq!(oblique.height, axial.height);
    assert_eq!(
        oblique.samples.iter().map(sample_bits).collect::<Vec<_>>(),
        [10.0_f32, 30.0, 70.0, 90.0].map(f32::to_bits)
    );
    assert_eq!(
        oblique.samples.iter().map(sample_bits).collect::<Vec<_>>(),
        axial.samples.iter().map(sample_bits).collect::<Vec<_>>()
    );
}

#[test]
fn a_non_unit_row_is_not_renormalised() {
    let volume = identity_volume(true);
    let image = sampled(
        &volume,
        ObliquePlane {
            origin: Pt::<World>::new(0.0, 0.0, 0.0),
            row: DVec3::new(2.0, 0.0, 0.0),
            column: DVec3::new(0.0, 0.0, 1.0),
            columns: 2,
            rows: 1,
            column_mm: 0.5,
            row_mm: 1.0,
        },
    );
    assert_eq!(image.column_mm.to_bits(), 0.5_f64.to_bits());
    assert_eq!(image.row_mm.to_bits(), 1.0_f64.to_bits());
    assert_eq!(present_bits(at(&image, 0, 0)), 10.0_f32.to_bits());
    assert_eq!(present_bits(at(&image, 1, 0)), 30.0_f32.to_bits());
    assert_ne!(present_bits(at(&image, 1, 0)), 20.0_f32.to_bits());
}

#[test]
fn a_bad_plane_allocates_nothing() {
    let volume = identity_volume(true);
    let good = mix_plane();
    let mut zero_row = good;
    zero_row.row = DVec3::ZERO;
    let mut non_finite = good;
    non_finite.column = DVec3::new(f64::NAN, 0.0, 1.0);
    let mut no_columns = good;
    no_columns.columns = 0;
    let mut no_rows = good;
    no_rows.rows = 0;
    let mut zero_spacing = good;
    zero_spacing.column_mm = 0.0;
    let mut negative_spacing = good;
    negative_spacing.row_mm = -1.0;
    for plane in [
        zero_row,
        non_finite,
        no_columns,
        no_rows,
        zero_spacing,
        negative_spacing,
    ] {
        assert!(matches!(
            volume.reslice_oblique(plane),
            Err(VolumeError::ObliquePlane)
        ));
    }

    let mut bad_spacing = identity_volume(true);
    bad_spacing.spacing[2] = 0.0;
    assert!(matches!(
        bad_spacing.reslice_oblique(good),
        Err(VolumeError::ObliquePlane)
    ));
    let mut zero_direction = identity_volume(true);
    zero_direction.direction = DMat3::from_cols(DVec3::ZERO, DVec3::Y, DVec3::Z);
    assert!(matches!(
        zero_direction.reslice_oblique(good),
        Err(VolumeError::ObliquePlane)
    ));
}

fn mix_plane() -> ObliquePlane {
    ObliquePlane {
        origin: Pt::<World>::new(0.5, 0.5, 0.5),
        row: DVec3::new(1.0, 0.0, 0.0),
        column: DVec3::new(0.0, 0.0, 1.0),
        columns: 2,
        rows: 1,
        column_mm: 1.0,
        row_mm: 1.0,
    }
}

fn integer_plane() -> ObliquePlane {
    ObliquePlane {
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        row: DVec3::new(1.0, 0.0, 0.0),
        column: DVec3::new(0.0, 0.0, 1.0),
        columns: 1,
        rows: 1,
        column_mm: 1.0,
        row_mm: 1.0,
    }
}

fn patient_point_plane() -> ObliquePlane {
    ObliquePlane {
        origin: Pt::<World>::new(0.5, 0.0, 0.0),
        row: DVec3::new(1.0, 0.0, 0.0),
        column: DVec3::new(0.0, 1.0, 0.0),
        columns: 1,
        rows: 1,
        column_mm: 1.0,
        row_mm: 1.0,
    }
}

fn sampled(volume: &Volume, plane: ObliquePlane) -> OrthoImage {
    let image = volume.reslice_oblique(plane);
    let Ok(image) = image else {
        unreachable!("the fixture plane is inside the identity volume")
    };
    image
}

fn at(image: &OrthoImage, column: u32, row: u32) -> &PlaneSample {
    let width = usize::try_from(image.width).unwrap_or(0);
    let index = usize::try_from(row)
        .unwrap_or(0)
        .saturating_mul(width)
        .saturating_add(usize::try_from(column).unwrap_or(0));
    let Some(sample) = image.samples.get(index) else {
        unreachable!("the fixture pixel is inside the plane")
    };
    sample
}

fn present_bits(sample: &PlaneSample) -> u32 {
    match sample {
        PlaneSample::Present(value) => value.to_bits(),
        PlaneSample::Absent => unreachable!("the fixture sample has landed"),
    }
}

fn sample_bits(sample: &PlaneSample) -> u32 {
    match sample {
        PlaneSample::Present(value) => value.to_bits(),
        PlaneSample::Absent => u32::MAX,
    }
}

fn slice_bits(volume: &Volume, index: u32) -> Vec<u32> {
    let Ok(values) = volume.present_slice_f32(index) else {
        unreachable!("the identity slice was placed")
    };
    values.iter().copied().map(f32::to_bits).collect()
}

fn assert_world(point: Pt<World>, expected: (f64, f64, f64)) {
    assert_eq!(point.x.to_bits(), expected.0.to_bits());
    assert_eq!(point.y.to_bits(), expected.1.to_bits());
    assert_eq!(point.z.to_bits(), expected.2.to_bits());
}

fn spaced_volume(spacing: [f64; 3], origin: (f64, f64, f64), direction: DMat3) -> Volume {
    let mut volume = identity_volume(true);
    volume.spacing = spacing;
    volume.origin = Pt::<World>::new(origin.0, origin.1, origin.2);
    volume.direction = direction;
    if let Some(level) = volume.levels.first_mut() {
        level.spacing = spacing;
    }
    volume
}

fn identity_volume(both_slices: bool) -> Volume {
    let mut volume = Volume {
        dims: [2, 2, 2],
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U8,
        data: VoxelStore::new([2, 2, 2], VoxelKind::U8),
        levels: vec![Level {
            dims: [2, 2, 2],
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(2),
        order: Vec::new(),
    };
    assert!(volume.place(0, &[10, 30, 70, 90]).is_ok());
    if both_slices {
        assert!(volume.place(1, &[110, 130, 150, 170]).is_ok());
    }
    volume
}
