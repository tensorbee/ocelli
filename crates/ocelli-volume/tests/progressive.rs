//! F-057. A brick is cut only when every slice it covers has landed.
//!
//! PS3.3 C.7.6.3 Pixel Representation. The container is already what `place`
//! stored. `FD FF` is little-endian `i16` `-3`. The same bytes read as `u16`
//! are `65533`. This file does not apply Bits Stored a second time.

use glam::DMat3;
use ocelli_core::{Pt, World};
use ocelli_volume::{Level, SlicePresence, Volume, VolumeError, VoxelKind, VoxelStore};

#[test]
fn an_incomplete_brick_is_not_cut() {
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert_eq!(volume.brick_grid().edge, 128);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    assert!(volume.ready_bricks().is_empty());
    assert!(matches!(
        volume.cut_ready_brick([0, 0, 0]),
        Err(VolumeError::BrickNotReady { brick: [0, 0, 0] })
    ));
    assert!(matches!(
        volume.present_slice_f32(1),
        Err(VolumeError::SliceNotPresent { index: 1 })
    ));
    let decoded = volume.present_slice_f32(0);
    let Ok(decoded) = decoded else {
        unreachable!("slice 0 was placed")
    };
    assert_eq!(
        decoded
            .iter()
            .copied()
            .map(f32::to_bits)
            .collect::<Vec<_>>(),
        [1.0_f32, 2.0, 3.0, 4.0].map(f32::to_bits)
    );
}

#[test]
fn a_ready_brick_copies_both_slices() {
    let mut volume = empty_volume([4, 1, 2], VoxelKind::U8);
    assert!(volume.place(0, &[1, 2, 3, 4]).is_ok());
    assert!(volume.place(1, &[5, 6, 7, 8]).is_ok());
    let cut = volume.cut_ready_brick([0, 0, 0]);
    let Ok(brick) = cut else {
        unreachable!("both slices of the only brick have landed")
    };
    assert_eq!(brick.extent, [4, 1, 2]);
    assert_eq!(brick.samples, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn signed_samples_stay_signed() {
    let mut volume = empty_volume([1, 1, 2], VoxelKind::I16);
    assert!(volume.place(0, &[0xFD, 0xFF]).is_ok());
    assert!(matches!(
        volume.cut_ready_brick([0, 0, 0]),
        Err(VolumeError::BrickNotReady { brick: [0, 0, 0] })
    ));
    assert!(volume.place(1, &[0x00, 0x00]).is_ok());
    let decoded = volume.present_slice_f32(0);
    let Ok(decoded) = decoded else {
        unreachable!("slice 0 was placed")
    };
    let Some(first) = decoded.first().copied() else {
        unreachable!("one voxel was placed")
    };
    assert_eq!(first.to_bits(), (-3.0_f32).to_bits());
    assert_ne!(first.to_bits(), 65533.0_f32.to_bits());
    let cut = volume.cut_ready_brick([0, 0, 0]);
    let Ok(brick) = cut else {
        unreachable!("both slices have landed")
    };
    assert_eq!(brick.samples, vec![0xFD, 0xFF, 0x00, 0x00]);
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
