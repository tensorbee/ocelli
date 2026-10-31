//! F-052. Orthographic planes follow the constructed volume's axes.
//!
//! PS3.3 C.7.6.2.1.1, extended by the volume's own normal. Identity direction,
//! origin `(10, 20, 30)`, spacing `[0.5, 2.0, 3.0]`, dims `[2, 2, 2]`. Stored
//! value is `x + 10 * y + 100 * z`. IPP is the centre of voxel `(0, 0, 0)`.
//!
//! Axial slice 0 pixel `(1, 1)` is voxel `(1, 1, 0)`, value 11, world
//! `(10.5, 22, 30)`.
//! Sagittal column 1 pixel `(0, 0)` is voxel `(1, 0, 0)`, value 1, world
//! `(10.5, 20, 30)`. Pixel `(1, 0)` is voxel `(1, 1, 0)`, value 11. A walk
//! along the row cosine reads `(0, 1, 0)` and returns 10.
//! Sagittal column 0.5 pixel `(0, 0)` blends `(0, 0, 0)` and `(1, 0, 0)` to
//! 0.5, world `(10.25, 20, 30)`.
//! Coronal row 0 pixel `(1, 1)` is voxel `(1, 0, 1)`, value 101, world
//! `(10.5, 20, 33)`.

use glam::DMat3;
use ocelli_core::{Pt, World};
use ocelli_volume::{
    Level, OrthoImage, OrthoPlane, PlaneSample, SlicePresence, Volume, VolumeError, VoxelKind,
    VoxelStore,
};

#[test]
fn ortho_planes_follow_the_volume_axes() {
    let volume = filled_fixture();
    let axial = resliced(&volume, OrthoPlane::Axial { slice: 0.0 });
    assert_eq!(axial.width, 2);
    assert_eq!(axial.height, 2);
    assert_eq!(axial.column_mm.to_bits(), 0.5_f64.to_bits());
    assert_eq!(axial.row_mm.to_bits(), 2.0_f64.to_bits());
    assert_eq!(present_bits(at(&axial, 1, 1)), 11.0_f32.to_bits());
    assert_world(volume.voxel_world([1.0, 1.0, 0.0]), (10.5, 22.0, 30.0));

    let sagittal = resliced(&volume, OrthoPlane::Sagittal { column: 1.0 });
    assert_eq!(sagittal.width, 2);
    assert_eq!(sagittal.height, 2);
    assert_eq!(sagittal.column_mm.to_bits(), 2.0_f64.to_bits());
    assert_eq!(sagittal.row_mm.to_bits(), 3.0_f64.to_bits());
    assert_eq!(present_bits(at(&sagittal, 0, 0)), 1.0_f32.to_bits());
    assert_eq!(present_bits(at(&sagittal, 1, 0)), 11.0_f32.to_bits());
    assert_ne!(present_bits(at(&sagittal, 1, 0)), 10.0_f32.to_bits());
    assert_world(volume.voxel_world([1.0, 0.0, 0.0]), (10.5, 20.0, 30.0));

    let halfway = resliced(&volume, OrthoPlane::Sagittal { column: 0.5 });
    assert_eq!(present_bits(at(&halfway, 0, 0)), 0.5_f32.to_bits());
    assert_world(volume.voxel_world([0.5, 0.0, 0.0]), (10.25, 20.0, 30.0));

    let coronal = resliced(&volume, OrthoPlane::Coronal { row: 0.0 });
    assert_eq!(coronal.width, 2);
    assert_eq!(coronal.height, 2);
    assert_eq!(coronal.column_mm.to_bits(), 0.5_f64.to_bits());
    assert_eq!(coronal.row_mm.to_bits(), 3.0_f64.to_bits());
    assert_eq!(present_bits(at(&coronal, 1, 1)), 101.0_f32.to_bits());
    assert_world(volume.voxel_world([1.0, 0.0, 1.0]), (10.5, 20.0, 33.0));
}

#[test]
fn an_absent_slice_is_not_stored_zero() {
    let mut volume = fixture_volume();
    assert!(volume.place(0, &[0, 1, 10, 11]).is_ok());
    let axial = resliced(&volume, OrthoPlane::Axial { slice: 0.0 });
    assert_eq!(
        axial.samples.iter().map(present_bits).collect::<Vec<_>>(),
        [0.0_f32, 1.0, 10.0, 11.0].map(|value| value.to_bits())
    );
    let missing = resliced(&volume, OrthoPlane::Axial { slice: 1.0 });
    assert!(
        missing
            .samples
            .iter()
            .all(|sample| matches!(sample, PlaneSample::Absent))
    );
    let sagittal = resliced(&volume, OrthoPlane::Sagittal { column: 1.0 });
    assert!(matches!(at(&sagittal, 0, 1), PlaneSample::Absent));
    let between = resliced(&volume, OrthoPlane::Axial { slice: 0.5 });
    assert!(matches!(at(&between, 0, 0), PlaneSample::Absent));
    let edge = resliced(&volume, OrthoPlane::Sagittal { column: 0.5 });
    assert_eq!(present_bits(at(&edge, 0, 0)), 0.5_f32.to_bits());
}

#[test]
fn sagittal_and_axial_keep_the_sign() {
    let mut volume = empty_volume([1, 1, 1], VoxelKind::I16);
    assert!(volume.place(0, &[0xFD, 0xFF]).is_ok());
    let axial = resliced(&volume, OrthoPlane::Axial { slice: 0.0 });
    assert_eq!(present_bits(at(&axial, 0, 0)), (-3.0_f32).to_bits());
    assert_ne!(present_bits(at(&axial, 0, 0)), 65533.0_f32.to_bits());
    let sagittal = resliced(&volume, OrthoPlane::Sagittal { column: 0.0 });
    assert_eq!(present_bits(at(&sagittal, 0, 0)), (-3.0_f32).to_bits());
}

#[test]
fn a_plane_index_outside_the_axis_allocates_nothing() {
    let volume = filled_fixture();
    assert!(matches!(
        volume.reslice(OrthoPlane::Sagittal { column: -0.1 }),
        Err(VolumeError::PlaneOutOfRange { axis: 0, .. })
    ));
    assert!(matches!(
        volume.reslice(OrthoPlane::Sagittal { column: 2.0 }),
        Err(VolumeError::PlaneOutOfRange { axis: 0, .. })
    ));
    assert!(matches!(
        volume.reslice(OrthoPlane::Sagittal { column: f64::NAN }),
        Err(VolumeError::PlaneOutOfRange { axis: 0, .. })
    ));
}

fn resliced(volume: &Volume, plane: OrthoPlane) -> OrthoImage {
    let image = volume.reslice(plane);
    let Ok(image) = image else {
        unreachable!("the fixture index is inside the volume")
    };
    image
}

fn at(image: &OrthoImage, a: u32, b: u32) -> &PlaneSample {
    let width = usize::try_from(image.width).unwrap_or(0);
    let index = usize::try_from(b)
        .unwrap_or(0)
        .saturating_mul(width)
        .saturating_add(usize::try_from(a).unwrap_or(0));
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

fn assert_world(point: Pt<World>, expected: (f64, f64, f64)) {
    assert_eq!(point.x.to_bits(), expected.0.to_bits());
    assert_eq!(point.y.to_bits(), expected.1.to_bits());
    assert_eq!(point.z.to_bits(), expected.2.to_bits());
}

fn filled_fixture() -> Volume {
    let mut volume = fixture_volume();
    assert!(volume.place(0, &[0, 1, 10, 11]).is_ok());
    assert!(volume.place(1, &[100, 101, 110, 111]).is_ok());
    volume
}

fn fixture_volume() -> Volume {
    let mut volume = empty_volume([2, 2, 2], VoxelKind::U8);
    volume.spacing = [0.5, 2.0, 3.0];
    volume.origin = Pt::<World>::new(10.0, 20.0, 30.0);
    volume.levels = vec![Level {
        dims: [2, 2, 2],
        spacing: [0.5, 2.0, 3.0],
        offset: 0,
    }];
    volume
}

fn empty_volume(dims: [u32; 3], voxel: VoxelKind) -> Volume {
    Volume {
        dims,
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel,
        data: VoxelStore::new(dims, voxel),
        levels: vec![Level {
            dims,
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(dims[2]),
        order: Vec::new(),
    }
}
