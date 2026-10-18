//! A brick is ready only when every slice it covers has landed.
//!
//! Section 19's presence bit is per slice. An eight-bit volume of 4 by 4 by
//! 200 uses edge 128, so the grid is one brick in x and y and two in z.
//! Brick `[0, 0, 0]` covers slices 0 to 127. Brick `[0, 0, 1]` covers slices
//! 128 to 199, which is 72 slices, because 200 - 128 = 72.

use glam::DMat3;
use ocelli_cache::BrickSample;
use ocelli_core::{Pt, World};
use ocelli_volume::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};

#[test]
fn a_brick_touching_an_absent_slice_is_not_ready() {
    let volume = empty_volume([4, 4, 200], VoxelKind::U8);
    let grid = volume.brick_grid();
    assert_eq!(grid.edge, 128);
    assert_eq!(grid.grid, [1, 1, 2]);
    assert_eq!(grid.extent_of([0, 0, 0]), Ok([4, 4, 128]));
    assert_eq!(grid.extent_of([0, 0, 1]), Ok([4, 4, 72]));
    // Slice 50 is inside brick 0. Slice 127 is that brick's last slice.
    // A walk that stops one short of the end still sees 50 and misses 127.
    // Slice 199 is brick 1's last slice, 128 + 72 - 1.
    assert_eq!(ready_except(50), vec![[0, 0, 1]]);
    assert_eq!(ready_except(127), vec![[0, 0, 1]]);
    assert_eq!(ready_except(199), vec![[0, 0, 0]]);
}

fn ready_except(absent: u32) -> Vec<[u32; 3]> {
    let mut volume = empty_volume([4, 4, 200], VoxelKind::U8);
    let slice = slice_bytes([4, 4, 200], VoxelKind::U8);
    let mut z = 0u32;
    while z < 200 {
        if z != absent {
            assert!(volume.place(z, &slice).is_ok());
        }
        z += 1;
    }
    volume.ready_bricks()
}

#[test]
fn ready_bricks_are_in_index_order() {
    // 130 cubed, edge 128, grid 2 by 2 by 2. Index order is x, then y, then z.
    // A walk with z innermost emits [0, 0, 1] before [1, 0, 0].
    let mut volume = empty_volume([130, 130, 130], VoxelKind::U8);
    let grid = volume.brick_grid();
    assert_eq!(grid.grid, [2, 2, 2]);
    assert_eq!(grid.index_of([1, 0, 0]), Ok(1));
    assert_eq!(grid.index_of([0, 0, 1]), Ok(4));
    let slice = slice_bytes([130, 130, 130], VoxelKind::U8);
    let mut z = 0u32;
    while z < 130 {
        assert!(volume.place(z, &slice).is_ok());
        z += 1;
    }
    assert_eq!(
        volume.ready_bricks(),
        vec![
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
        ]
    );
}

#[test]
fn a_complete_small_volume_lists_its_only_brick() {
    let mut volume = empty_volume([8, 8, 8], VoxelKind::U8);
    let slice = slice_bytes([8, 8, 8], VoxelKind::U8);
    let mut z = 0u32;
    while z < 8 {
        assert!(volume.place(z, &slice).is_ok());
        z += 1;
    }
    assert_eq!(volume.ready_bricks(), vec![[0, 0, 0]]);
}

#[test]
fn sixteen_bit_kinds_stay_distinct_as_brick_samples() {
    assert_eq!(BrickSample::from(VoxelKind::I16), BrickSample::I16);
    assert_eq!(BrickSample::from(VoxelKind::U16), BrickSample::U16);
    assert_ne!(
        BrickSample::from(VoxelKind::I16),
        BrickSample::from(VoxelKind::U16)
    );
    assert_eq!(BrickSample::from(VoxelKind::U8), BrickSample::U8);
    assert_eq!(BrickSample::from(VoxelKind::F32), BrickSample::F32);
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

fn slice_bytes(dims: [u32; 3], voxel: VoxelKind) -> Vec<u8> {
    let width = usize::try_from(dims[0]).unwrap_or(0);
    let height = usize::try_from(dims[1]).unwrap_or(0);
    let len = width
        .saturating_mul(height)
        .saturating_mul(voxel.bytes_per_voxel());
    vec![0u8; len]
}
