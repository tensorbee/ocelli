//! F-059. A voxel is resident only when its level-0 brick has landed.
//!
//! HLD section 30 sets the eight-bit brick edge at 128. Dims `[1, 1, 129]`
//! are two bricks along z. Slices 0 through 127 fill the first brick. Slice
//! 128 is the second brick and is not placed. A cleared byte in that slice
//! is not a resident voxel.

use glam::DMat3;
use ocelli_core::{Pt, World};
use ocelli_volume::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};

#[test]
fn resident_follows_ready_bricks() {
    let mut volume = empty_volume([1, 1, 129], VoxelKind::U8);
    assert_eq!(volume.brick_grid().edge, 128);
    let mut slice = 0_u32;
    while slice < 128 {
        assert!(volume.place(slice, &[0]).is_ok());
        slice = slice.saturating_add(1);
    }
    assert_eq!(volume.ready_bricks(), vec![[0, 0, 0]]);
    assert!(volume.resident([0, 0, 0]));
    assert!(volume.resident([0, 0, 127]));
    assert!(!volume.resident([0, 0, 128]));
    assert!(!volume.resident([0, 0, 129]));
    assert!(!volume.resident([1, 0, 0]));
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
