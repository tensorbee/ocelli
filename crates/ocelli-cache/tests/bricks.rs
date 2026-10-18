//! Brick addressing and residency, F-036.
//!
//! The expected numbers are hand-computed in the comments before the
//! assertions. PS3.3 C.7.6.3.1 names `Columns`, `Rows` and `Bits Allocated`.
//! PS3.3 C.7.6.2.1.1 is the voxel order the axes follow. The layout product
//! is HLD section 19.

use ocelli_cache::{
    Admission, Brick, BrickCache, BrickError, BrickGrid, BrickKey, BrickSample, BrickWidth,
    Budgeted, CacheTier, VolumeId,
};
use proptest::prelude::*;

/// 512 by 512 by 600 at sixteen bits. Edge 64.
/// Ceiling: 512 / 64 = 8, 600 / 64 = 9.375, so the grid is 8, 8, 10.
fn ct_grid() -> BrickGrid {
    BrickGrid::new([512, 512, 600], BrickWidth::SixteenBit)
}

#[test]
fn the_brick_containing_a_voxel_is_the_floor_of_each_axis() {
    let grid = ct_grid();
    assert_eq!(grid.edge, 64);
    assert_eq!(grid.grid, [8, 8, 10]);
    // 200 / 64 = 3, 130 / 64 = 2, 599 / 64 = 9.
    assert_eq!(grid.brick_of([200, 130, 599]), Ok([3, 2, 9]));
}

#[test]
fn a_brick_origin_and_its_byte_offset_agree() {
    let grid = ct_grid();
    // 3 * 64 = 192, 2 * 64 = 128, 9 * 64 = 576.
    assert_eq!(grid.origin_of([3, 2, 9]), Ok([192, 128, 576]));
    // ((576 * 512 + 128) * 512 + 192) * 2
    // 576 * 512 = 294_912
    // 294_912 + 128 = 295_040
    // 295_040 * 512 = 151_060_480
    // 151_060_480 + 192 = 151_060_672
    // 151_060_672 * 2 = 302_121_344
    assert_eq!(grid.byte_offset([192, 128, 576], 2), Ok(302_121_344));
}

#[test]
fn an_edge_brick_reports_its_true_extent_and_its_true_bytes() {
    // Eight-bit edge is 128.
    // Ceiling: 500 / 128 = 3.90625 -> 4, 300 / 128 = 2.34375 -> 3,
    // 130 / 128 = 1.015625 -> 2.
    let grid = BrickGrid::new([500, 300, 130], BrickWidth::EightBit);
    assert_eq!(grid.edge, 128);
    assert_eq!(grid.grid, [4, 3, 2]);
    // Last brick is index 3, 2, 1.
    // 500 - 3 * 128 = 116, 300 - 2 * 128 = 44, 130 - 128 = 2.
    assert_eq!(grid.extent_of([3, 2, 1]), Ok([116, 44, 2]));
    // 116 * 44 * 2 = 10_208 samples at one byte, not 128 cubed.
    let brick = cut_filled(&grid, [3, 2, 1], BrickSample::U8);
    assert_eq!(brick.extent, [116, 44, 2]);
    assert_eq!(brick.samples.len(), 10_208);
    assert_eq!(brick.bytes(), 10_208);
    assert_ne!(brick.bytes(), 128 * 128 * 128);
}

#[test]
fn a_one_brick_volume_uses_the_same_constructor() {
    // 64 / 128 and 40 / 128 both ceiling to 1. No second path.
    let grid = BrickGrid::new([64, 64, 40], BrickWidth::EightBit);
    assert_eq!(grid.grid, [1, 1, 1]);
    let volume = VolumeId(7);
    let mut cache = BrickCache::new(64 * 64 * 40);
    let missing = cache.missing(volume, &grid);
    assert_eq!(missing.len(), 1);
    let key = BrickKey {
        volume,
        level: grid.level,
        brick: [0, 0, 0],
    };
    assert_eq!(missing.first().copied(), Some(key));
    let _ = cache.insert(key, cut_filled(&grid, [0, 0, 0], BrickSample::U8));
    assert!(cache.missing(volume, &grid).is_empty());
    assert!(cache.is_resident(volume, &grid));
}

#[test]
fn a_sixteen_bit_volume_decomposes_at_edge_64() {
    let grid = ct_grid();
    assert_eq!(grid.grid, [8, 8, 10]);
    // An interior brick is 64 cubed voxels at two bytes: 524_288.
    // 600 is not a multiple of 64, so the last z brick is 24 voxels deep
    // and is not that size. 9 * 64 = 576, and 600 - 576 = 24.
    assert_eq!(grid.extent_of([0, 0, 0]), Ok([64, 64, 64]));
    assert_eq!(grid.extent_of([0, 0, 9]), Ok([64, 64, 24]));
    // The same extents cut from volumes that are exactly one brick.
    // 64 * 64 * 64 * 2 = 524_288. 64 * 64 * 24 * 2 = 196_608.
    let interior = cut_filled(
        &BrickGrid::new([64, 64, 64], BrickWidth::SixteenBit),
        [0, 0, 0],
        BrickSample::U16,
    );
    assert_eq!(interior.bytes(), 524_288);
    assert_eq!(interior.sample, BrickSample::U16);
    let edge = cut_filled(
        &BrickGrid::new([64, 64, 24], BrickWidth::SixteenBit),
        [0, 0, 0],
        BrickSample::U16,
    );
    assert_eq!(edge.extent, [64, 64, 24]);
    assert_eq!(edge.bytes(), 196_608);
    assert_eq!(edge.sample, BrickSample::U16);
    // 512 * 512 * 600 * 2 = 314_572_800, above a 256 MiB budget of 268_435_456.
    let volume_bytes = 512usize * 512 * 600 * 2;
    assert_eq!(volume_bytes, 314_572_800);
    assert!(volume_bytes > 256 * 1024 * 1024);
}

#[test]
fn ceiling_division_names_axis_x() {
    let exact = BrickGrid::new([64, 64, 64], BrickWidth::SixteenBit);
    let partial = BrickGrid::new([65, 64, 64], BrickWidth::SixteenBit);
    assert_eq!(exact.grid, [1, 1, 1]);
    assert_eq!(partial.grid, [2, 1, 1]);
}

#[test]
fn ceiling_division_names_axis_y() {
    let exact = BrickGrid::new([64, 64, 64], BrickWidth::SixteenBit);
    let partial = BrickGrid::new([64, 65, 64], BrickWidth::SixteenBit);
    assert_eq!(exact.grid, [1, 1, 1]);
    assert_eq!(partial.grid, [1, 2, 1]);
}

#[test]
fn ceiling_division_names_axis_z() {
    let exact = BrickGrid::new([64, 64, 64], BrickWidth::SixteenBit);
    let partial = BrickGrid::new([64, 64, 65], BrickWidth::SixteenBit);
    assert_eq!(exact.grid, [1, 1, 1]);
    assert_eq!(partial.grid, [1, 1, 2]);
}

#[test]
fn index_of_is_x_fastest_then_y_then_z() {
    let grid = ct_grid();
    assert_eq!(grid.index_of([0, 0, 0]), Ok(0));
    assert_eq!(grid.index_of([1, 0, 0]), Ok(1));
    // The first row has grid.x bricks, so brick 0, 1, 0 is index 8.
    assert_eq!(grid.index_of([0, 1, 0]), Ok(8));
    // One z step is 8 * 8 = 64.
    assert_eq!(grid.index_of([0, 0, 1]), Ok(64));
}

#[test]
fn an_out_of_range_voxel_or_brick_is_an_error_on_each_axis() {
    let grid = ct_grid();
    assert_eq!(
        grid.brick_of([512, 0, 0]),
        Err(BrickError::VoxelOutOfRange {
            axis: 0,
            value: 512
        })
    );
    assert_eq!(
        grid.brick_of([0, 512, 0]),
        Err(BrickError::VoxelOutOfRange {
            axis: 1,
            value: 512
        })
    );
    assert_eq!(
        grid.brick_of([0, 0, 600]),
        Err(BrickError::VoxelOutOfRange {
            axis: 2,
            value: 600
        })
    );
    assert_eq!(
        grid.origin_of([8, 0, 0]),
        Err(BrickError::BrickOutOfRange { axis: 0, value: 8 })
    );
    assert_eq!(
        grid.origin_of([0, 8, 0]),
        Err(BrickError::BrickOutOfRange { axis: 1, value: 8 })
    );
    assert_eq!(
        grid.index_of([0, 0, 10]),
        Err(BrickError::BrickOutOfRange { axis: 2, value: 10 })
    );
}

#[test]
fn missing_follows_index_order_and_is_not_a_use() {
    // Edge 128. Two bricks on x and on y, so x-fastest order is not the
    // order of an x-outermost walk. Each inserted brick is 9_216 bytes.
    let grid = BrickGrid::new([200, 200, 1], BrickWidth::EightBit);
    assert_eq!(grid.grid, [2, 2, 1]);
    let volume = VolumeId(1);
    let mut cache = BrickCache::new(18_432);
    let before = cache.missing(volume, &grid);
    assert_eq!(before.len(), 4);
    assert_eq!(brick_at(&before, 0), [0, 0, 0]);
    assert_eq!(brick_at(&before, 1), [1, 0, 0]);
    assert_eq!(brick_at(&before, 2), [0, 1, 0]);
    assert_eq!(brick_at(&before, 3), [1, 1, 0]);
    for (index, key) in before.iter().enumerate() {
        assert_eq!(grid.index_of(key.brick), Ok(index));
    }
    // Insert the later scan key first. A query that calls `get` refreshes
    // that key last, so the next eviction would take the other brick.
    let later = BrickKey {
        volume,
        level: 0,
        brick: [0, 1, 0],
    };
    let earlier = BrickKey {
        volume,
        level: 0,
        brick: [1, 0, 0],
    };
    let _ = cache.insert(later, cut_filled(&grid, later.brick, BrickSample::U8));
    let _ = cache.insert(earlier, cut_filled(&grid, earlier.brick, BrickSample::U8));
    let after = cache.missing(volume, &grid);
    assert_eq!(after.len(), 2);
    assert_eq!(brick_at(&after, 0), [0, 0, 0]);
    assert_eq!(brick_at(&after, 1), [1, 1, 0]);
    let _ = cache.missing(volume, &grid);
    let stranger = BrickKey {
        volume: VolumeId(2),
        level: 0,
        brick: [0, 0, 0],
    };
    let admission = cache.insert(stranger, brick_bytes(80));
    assert_eq!(admission.evicted.len(), 1);
    assert_eq!(
        admission.evicted.first().map(|(key, _)| key.brick),
        Some([0, 1, 0])
    );
    assert_eq!(
        admission.evicted.first().map(|(key, _)| key.volume),
        Some(volume)
    );
}

#[test]
fn an_empty_grid_is_not_resident() {
    // A zero axis ceilings to no bricks. `missing` is then empty, and an
    // empty list is not a complete level.
    let volume = VolumeId(7);
    let cache = BrickCache::new(64);
    let cases = [
        ([0, 4, 4], [0, 1, 1]),
        ([4, 0, 4], [1, 0, 1]),
        ([4, 4, 0], [1, 1, 0]),
    ];
    for (dims, expected) in cases {
        let grid = BrickGrid::new(dims, BrickWidth::EightBit);
        assert_eq!(grid.grid, expected);
        assert!(cache.missing(volume, &grid).is_empty());
        assert!(!cache.is_resident(volume, &grid));
    }
}

#[test]
fn eviction_removes_the_least_recently_used_brick_and_clears_residency() {
    let grid = BrickGrid::new([2, 2, 2], BrickWidth::EightBit);
    let volume = VolumeId(4);
    let key = BrickKey {
        volume,
        level: 0,
        brick: [0, 0, 0],
    };
    let mut cache = BrickCache::new(8);
    let _ = cache.insert(key, cut_filled(&grid, [0, 0, 0], BrickSample::U8));
    assert!(cache.is_resident(volume, &grid));
    let other = BrickKey {
        volume: VolumeId(5),
        level: 0,
        brick: [0, 0, 0],
    };
    let admission = cache.insert(other, brick_bytes(8));
    assert_eq!(admission.evicted.len(), 1);
    assert!(!cache.is_resident(volume, &grid));
    assert!(cache.is_resident(VolumeId(5), &grid));
}

#[test]
fn an_oversized_brick_is_refused_and_evicts_nothing() {
    let mut cache = BrickCache::new(4);
    let key = BrickKey {
        volume: VolumeId(1),
        level: 0,
        brick: [0, 0, 0],
    };
    let admission = cache.insert(key, brick_bytes(8));
    assert!(admission.evicted.is_empty());
    assert!(admission.refused.is_some());
    assert_eq!(cache.pressure().tier, CacheTier::Gpu);
    assert_eq!(cache.pressure().used, 0);
}

#[test]
fn a_byte_offset_past_u32_max_does_not_wrap() {
    // (1 * 65_536) * 65_536 = 4_294_967_296, which is one past `u32::MAX`.
    // The 512 by 512 by 600 offset fits in a `u32`, so that fixture cannot
    // pin a wrapping intermediate.
    let grid = BrickGrid::new([65_536, 65_536, 2], BrickWidth::EightBit);
    assert_eq!(grid.byte_offset([0, 0, 1], 1), Ok(4_294_967_296));
}

#[test]
fn a_byte_product_past_u64_is_an_error() {
    // Linear index is 2^63. Times 4 bytes is 2^65, which does not fit in `u64`.
    let grid = BrickGrid::new([1 << 20, 1 << 20, (1 << 23) + 1], BrickWidth::EightBit);
    assert_eq!(
        grid.byte_offset([0, 0, 1 << 23], 4),
        Err(BrickError::VoxelOutOfRange {
            axis: 2,
            value: 1 << 23
        })
    );
}

#[test]
fn a_linear_index_past_u64_is_an_error() {
    // 19_999_999 * 1_000_000 * 1_000_000 does not fit in `u64`.
    let grid = BrickGrid::new([1_000_000, 1_000_000, 20_000_000], BrickWidth::EightBit);
    assert_eq!(
        grid.byte_offset([0, 0, 19_999_999], 1),
        Err(BrickError::VoxelOutOfRange {
            axis: 2,
            value: 19_999_999
        })
    );
}

#[test]
fn a_brick_index_past_u64_is_an_error() {
    // Sixteen-bit edge 64. Brick z of 2^20 in a `u32::MAX` cube is inside
    // the grid, and its index is 2^72.
    let grid = BrickGrid::new([u32::MAX, u32::MAX, u32::MAX], BrickWidth::SixteenBit);
    assert_eq!(
        grid.index_of([0, 0, 1 << 20]),
        Err(BrickError::BrickOutOfRange {
            axis: 2,
            value: 1 << 20
        })
    );
}

#[test]
fn a_cut_copies_the_row_at_the_section_19_offset() {
    let grid = BrickGrid::new([200, 2, 1], BrickWidth::EightBit);
    let mut volume = vec![0u8; 400];
    // Brick 1 starts at voxel 128. Row 1 of the volume is byte 200 + 128 = 328.
    let offset = grid.byte_offset([128, 1, 0], 1).unwrap_or_default();
    assert_eq!(offset, 328);
    if let Some(slot) = volume.get_mut(offset) {
        *slot = 7;
    }
    let brick = match Brick::cut(&grid, [1, 0, 0], &volume, BrickSample::U8) {
        Ok(brick) => brick,
        Err(_) => brick_bytes(0),
    };
    // 200 - 128 = 72. The second row of the brick starts at sample 72.
    assert_eq!(brick.extent, [72, 2, 1]);
    assert_eq!(brick.sample, BrickSample::U8);
    assert_eq!(brick.samples.get(72).copied(), Some(7));
}

#[test]
fn cut_keeps_the_sample_kind_and_float_is_four_bytes() {
    // PS3.3 C.7.6.3 Pixel Representation. The same two bytes are signed or
    // unsigned, and the kind cut stored is the one the caller passed.
    // F32 is four bytes per voxel. 2 voxels are 8 bytes, and byte 4 is the
    // first byte of the second voxel.
    assert_eq!(BrickSample::F32.bytes_per_voxel(), 4);
    let grid = BrickGrid::new([1, 1, 1], BrickWidth::SixteenBit);
    let volume = vec![0x00, 0x80];
    let signed = match Brick::cut(&grid, [0, 0, 0], &volume, BrickSample::I16) {
        Ok(brick) => brick,
        Err(_) => brick_bytes(0),
    };
    let unsigned = match Brick::cut(&grid, [0, 0, 0], &volume, BrickSample::U16) {
        Ok(brick) => brick,
        Err(_) => brick_bytes(0),
    };
    assert_eq!(signed.samples, unsigned.samples);
    assert_eq!(signed.sample, BrickSample::I16);
    assert_eq!(unsigned.sample, BrickSample::U16);
    let float_grid = BrickGrid::new([2, 1, 1], BrickWidth::Float32);
    let mut floats = vec![0u8; 8];
    if let Some(slot) = floats.get_mut(4) {
        *slot = 9;
    }
    let float_brick = match Brick::cut(&float_grid, [0, 0, 0], &floats, BrickSample::F32) {
        Ok(brick) => brick,
        Err(_) => brick_bytes(0),
    };
    assert_eq!(float_brick.sample, BrickSample::F32);
    assert_eq!(float_brick.extent, [2, 1, 1]);
    assert_eq!(float_brick.bytes(), 8);
    assert_eq!(float_brick.samples.get(4).copied(), Some(9));
}

#[test]
fn float_bricks_take_the_sixteen_bit_edge() {
    let grid = BrickGrid::new([64, 64, 64], BrickWidth::Float32);
    assert_eq!(grid.edge, 64);
    assert_eq!(grid.grid, [1, 1, 1]);
}

fn cut_filled(grid: &BrickGrid, brick: [u32; 3], sample: BrickSample) -> Brick {
    let [dx, dy, dz] = grid.dims;
    let voxels = u64::from(dx) * u64::from(dy) * u64::from(dz);
    let width = u64::try_from(sample.bytes_per_voxel()).unwrap_or(0);
    let len = match usize::try_from(voxels.saturating_mul(width)) {
        Ok(len) => len,
        Err(_) => return brick_bytes(0),
    };
    let volume = vec![0u8; len];
    match Brick::cut(grid, brick, &volume, sample) {
        Ok(brick) => brick,
        Err(_) => brick_bytes(0),
    }
}

fn key_at(keys: &[BrickKey], index: usize) -> BrickKey {
    match keys.get(index) {
        Some(key) => *key,
        None => BrickKey {
            volume: VolumeId(0),
            level: 0,
            brick: [0, 0, 0],
        },
    }
}

fn brick_at(keys: &[BrickKey], index: usize) -> [u32; 3] {
    key_at(keys, index).brick
}

fn brick_bytes(len: usize) -> Brick {
    Brick {
        extent: [1, 1, 1],
        samples: vec![0; len],
        sample: BrickSample::U8,
    }
}

proptest! {
    #[test]
    fn every_voxel_lands_inside_its_brick_and_extents_sum_to_the_axis(
        dx in 1u32..=24,
        dy in 1u32..=24,
        dz in 1u32..=24,
        kind in prop_oneof![
            Just(BrickWidth::EightBit),
            Just(BrickWidth::SixteenBit),
            Just(BrickWidth::Float32),
        ],
    ) {
        let grid = BrickGrid::new([dx, dy, dz], kind);
        let [gx, gy, gz] = grid.grid;
        let mut sum_x = 0u32;
        let mut x = 0u32;
        while x < gx {
            let [ex, _, _] = grid.extent_of([x, 0, 0]).unwrap_or([0, 0, 0]);
            sum_x = sum_x.saturating_add(ex);
            x += 1;
        }
        prop_assert_eq!(sum_x, dx);
        let mut sum_y = 0u32;
        let mut y = 0u32;
        while y < gy {
            let [_, ey, _] = grid.extent_of([0, y, 0]).unwrap_or([0, 0, 0]);
            sum_y = sum_y.saturating_add(ey);
            y += 1;
        }
        prop_assert_eq!(sum_y, dy);
        let mut sum_z = 0u32;
        let mut z = 0u32;
        while z < gz {
            let [_, _, ez] = grid.extent_of([0, 0, z]).unwrap_or([0, 0, 0]);
            sum_z = sum_z.saturating_add(ez);
            z += 1;
        }
        prop_assert_eq!(sum_z, dz);

        let mut vz = 0u32;
        while vz < dz {
            let mut vy = 0u32;
            while vy < dy {
                let mut vx = 0u32;
                while vx < dx {
                    let brick = grid.brick_of([vx, vy, vz]).unwrap_or([u32::MAX, 0, 0]);
                    let [ox, oy, oz] = grid.origin_of(brick).unwrap_or([u32::MAX, 0, 0]);
                    let [ex, ey, ez] = grid.extent_of(brick).unwrap_or([0, 0, 0]);
                    prop_assert!(ox <= vx && vx < ox.saturating_add(ex));
                    prop_assert!(oy <= vy && vy < oy.saturating_add(ey));
                    prop_assert!(oz <= vz && vz < oz.saturating_add(ez));
                    vx += 1;
                }
                vy += 1;
            }
            vz += 1;
        }
    }
}

proptest! {
    #[test]
    fn pressure_used_is_the_live_bricks_and_never_exceeds_budget(
        budget in 0usize..=64,
        steps in prop::collection::vec((0u8..=3, 1usize..=9), 0..=12),
    ) {
        let mut cache = BrickCache::new(budget);
        let mut live: Vec<(BrickKey, usize)> = Vec::new();
        for (which, len) in steps {
            let key = BrickKey {
                volume: VolumeId(u64::from(which)),
                level: 0,
                brick: [0, 0, 0],
            };
            let admission = cache.insert(key, brick_bytes(len));
            apply(&mut live, key, len, &admission);
            let used: usize = live.iter().map(|(_, bytes)| *bytes).sum();
            prop_assert_eq!(cache.pressure().used, used);
            prop_assert!(cache.pressure().used <= budget);
        }
    }
}

fn apply(
    live: &mut Vec<(BrickKey, usize)>,
    key: BrickKey,
    len: usize,
    admission: &Admission<BrickKey, Brick>,
) {
    if admission.refused.is_some() {
        return;
    }
    live.retain(|(held, _)| *held != key);
    for (evicted, _) in &admission.evicted {
        live.retain(|(held, _)| held != evicted);
    }
    live.push((key, len));
}
