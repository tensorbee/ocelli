//! The volume crate's voxel kind selects the cache's brick edge.

use glam::DMat3;
use ocelli_cache::BrickGrid;
use ocelli_core::{Pt, World};
use ocelli_volume::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};

#[test]
fn each_voxel_kind_selects_one_brick_edge() {
    assert_eq!(grid_of([9, 9, 9], VoxelKind::U8).edge, 128);
    assert_eq!(grid_of([9, 9, 9], VoxelKind::I16).edge, 64);
    assert_eq!(grid_of([9, 9, 9], VoxelKind::U16).edge, 64);
    assert_eq!(grid_of([9, 9, 9], VoxelKind::F32).edge, 64);
}

#[test]
fn a_volume_brick_grid_follows_its_dims_and_voxel_kind() {
    let grid = grid_of([130, 70, 3], VoxelKind::U16);
    assert_eq!(grid.dims, [130, 70, 3]);
    assert_eq!(grid.edge, 64);
    assert_eq!(grid.grid, [3, 2, 1]);
    assert_eq!(grid.level, 0);
}

fn grid_of(dims: [u32; 3], voxel: VoxelKind) -> BrickGrid {
    let volume = Volume {
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
        order: vec![0],
    };
    volume.brick_grid()
}
