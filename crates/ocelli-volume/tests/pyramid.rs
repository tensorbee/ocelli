//! F-119. Brick decomposition and multiscale pyramid fixtures.
//!
//! No DICOM section specifies a pyramid. The sections below fix the inputs.
//! PS3.3 C.7.6.3 Pixel Representation makes the signed container two's
//! complement. C.11.1 Rescale Slope and Intercept is affine, so a stored
//! mean is the modality mean before rounding. C.7.6.2.1.1 puts the origin
//! at the centre of voxel zero. C.7.6.2 Pixel Spacing is the in-plane step
//! the dimension ratio extends.
//!
//! The filter is the exact-overlap box from level 0. Integer rounding is
//! nearest, with ties away from zero. Child spacing is level 0 spacing
//! times `dims_0 / dims` on each axis.

use ocelli_cache::{BrickCache, BrickKey, BrickSample, Budgeted, VolumeId};
use ocelli_core::Pt;
use ocelli_volume::{Level, SlicePresence, Volume, VolumeError, VoxelKind, VoxelStore};

fn bare(dims: [u32; 3], spacing: [f64; 3], kind: VoxelKind) -> Volume {
    let [_dx, _dy, dz] = dims;
    let mut order = Vec::new();
    let mut index = 0u32;
    while index < dz {
        order.push(usize::try_from(index).unwrap_or(0));
        index += 1;
    }
    Volume {
        dims,
        spacing,
        origin: Pt::new(0.0, 0.0, 0.0),
        direction: glam::DMat3::IDENTITY,
        voxel: kind,
        data: VoxelStore::new(dims, kind),
        levels: vec![Level {
            dims,
            spacing,
            offset: 0,
        }],
        present: SlicePresence::new(dz),
        order,
    }
}

fn place_all(volume: &mut Volume, samples: &[u8]) -> Result<(), VolumeError> {
    let [dx, dy, dz] = volume.dims;
    let width = volume.voxel.bytes_per_voxel();
    let columns = usize::try_from(dx).map_err(|_err| VolumeError::LevelShape)?;
    let rows = usize::try_from(dy).map_err(|_err| VolumeError::LevelShape)?;
    let slice_len = columns
        .checked_mul(rows)
        .and_then(|plane| plane.checked_mul(width))
        .ok_or(VolumeError::LevelShape)?;
    let mut index = 0u32;
    while index < dz {
        let start = usize::try_from(index)
            .ok()
            .and_then(|slot| slot.checked_mul(slice_len))
            .ok_or(VolumeError::LevelShape)?;
        let end = start
            .checked_add(slice_len)
            .ok_or(VolumeError::LevelShape)?;
        let slice = samples
            .get(start..end)
            .ok_or(VolumeError::SliceSizeMismatch {
                expected: slice_len,
                actual: samples.len(),
            })?;
        volume.place(index, slice)?;
        index += 1;
    }
    Ok(())
}

fn filled(
    dims: [u32; 3],
    spacing: [f64; 3],
    kind: VoxelKind,
    samples: &[u8],
) -> Result<Volume, VolumeError> {
    let mut volume = bare(dims, spacing, kind);
    place_all(&mut volume, samples)?;
    Ok(volume)
}

fn le_i16(values: &[i16]) -> Vec<u8> {
    let mut out = Vec::new();
    for value in values {
        out.extend(value.to_le_bytes());
    }
    out
}

fn le_u16(values: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    for value in values {
        out.extend(value.to_le_bytes());
    }
    out
}

fn le_f32(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::new();
    for value in values {
        out.extend(value.to_le_bytes());
    }
    out
}

fn repeat_u16(voxels: usize, value: u16) -> Vec<u8> {
    let unit = value.to_le_bytes();
    let mut out = Vec::new();
    let mut left = voxels;
    while left > 0 {
        out.extend(unit);
        left -= 1;
    }
    out
}

fn repeat_u8(voxels: usize, value: u8) -> Vec<u8> {
    vec![value; voxels]
}

fn read_f32(bytes: &[u8]) -> Result<f32, VolumeError> {
    let quad = [
        bytes.first().copied().ok_or(VolumeError::LevelShape)?,
        bytes.get(1).copied().ok_or(VolumeError::LevelShape)?,
        bytes.get(2).copied().ok_or(VolumeError::LevelShape)?,
        bytes.get(3).copied().ok_or(VolumeError::LevelShape)?,
    ];
    Ok(f32::from_le_bytes(quad))
}

fn assert_u16_constant(bytes: &[u8], value: u16) -> Result<(), VolumeError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(VolumeError::LevelShape);
    }
    let mut offset = 0usize;
    while offset < bytes.len() {
        let next = offset.checked_add(1).ok_or(VolumeError::LevelShape)?;
        let pair = [
            bytes.get(offset).copied().ok_or(VolumeError::LevelShape)?,
            bytes.get(next).copied().ok_or(VolumeError::LevelShape)?,
        ];
        assert_eq!(u16::from_le_bytes(pair), value);
        offset += 2;
    }
    Ok(())
}

fn assert_u8_constant(bytes: &[u8], value: u8) {
    for byte in bytes {
        assert_eq!(*byte, value);
    }
}

/// AC1. Even 2 by 2 by 2 box. Ties 3.5 and 2.5 round away from zero to 4 and 3.
///
/// Level 0 is `[4, 2, 2]`, x fastest:
/// `0,1,0,0, 2,3,0,0, 4,5,5,5, 6,7,5,5`.
/// Cell 0 covers x 0..2, y 0..2, z 0..2 and sums
/// `0+1+2+3+4+5+6+7 = 28` over 8, which is 3.5.
/// Cell 1 covers x 2..4 and sums `5+5+5+5 = 20` over 8, which is 2.5.
/// Spacing `[1, 2, 3]` times `[4/2, 2/1, 2/1]` is `[2, 4, 6]`.
#[test]
fn fixture_even_box_rounds_ties_away_from_zero() -> Result<(), VolumeError> {
    let samples = [0_u8, 1, 0, 0, 2, 3, 0, 0, 4, 5, 5, 5, 6, 7, 5, 5];
    let mut volume = filled([4, 2, 2], [1.0, 2.0, 3.0], VoxelKind::U8, &samples)?;
    let index = volume.downsample_level([2, 1, 1])?;
    assert_eq!(index, 1);
    assert_eq!(volume.level_bytes(1)?, &[4, 3]);
    let spacing = volume.level(1)?.spacing;
    assert_eq!(spacing.map(f64::to_bits), [2.0, 4.0, 6.0].map(f64::to_bits));
    Ok(())
}

/// AC2. PS3.3 C.7.6.3. Sum `-1-2-3-4-5-6-7+0 = -28`. `-28/8 = -3.5`.
/// Ties away from zero give `-4`, little-endian `FC FF`.
#[test]
fn fixture_signed_tie_rounds_away_from_zero() -> Result<(), VolumeError> {
    let samples = le_i16(&[-1, -2, -3, -4, -5, -6, -7, 0]);
    let mut volume = filled([2, 2, 2], [1.0, 1.0, 1.0], VoxelKind::I16, &samples)?;
    volume.downsample_level([1, 1, 1])?;
    assert_eq!(volume.level_bytes(1)?, &[0xFC, 0xFF]);
    let pair = [
        volume
            .level_bytes(1)?
            .first()
            .copied()
            .ok_or(VolumeError::LevelShape)?,
        volume
            .level_bytes(1)?
            .get(1)
            .copied()
            .ok_or(VolumeError::LevelShape)?,
    ];
    assert_eq!(i16::from_le_bytes(pair), -4);
    Ok(())
}

/// AC3. Odd axis. `D = 5`, `d = 3`, divisor 5.
/// Child 0 weights 3 and 2: `(3*10 + 2*20) / 5 = 14`.
/// Child 1 weights 1, 3, 1: `(1*20 + 3*30 + 1*40) / 5 = 30`.
/// Child 2 weights 2 and 3: `(2*40 + 3*50) / 5 = 46`.
/// Spacing x is `1.0 * 5.0 / 3.0` by bits.
#[test]
fn fixture_odd_axis_uses_exact_overlap() -> Result<(), VolumeError> {
    let mut volume = filled(
        [5, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::U8,
        &[10, 20, 30, 40, 50],
    )?;
    volume.downsample_level([3, 1, 1])?;
    assert_eq!(volume.level_bytes(1)?, &[14, 30, 46]);
    let spacing_x = volume.level(1)?.spacing[0];
    assert_eq!(spacing_x.to_bits(), (1.0_f64 * 5.0 / 3.0).to_bits());
    Ok(())
}

/// AC4. Level 2 is a downsample of level 0, not of level 1.
///
/// `[0, 1, 0, 0]` to `[2, 1, 1]` is means 0.5 and 0, rounded to `[1, 0]`.
/// `[0, 1, 0, 0]` to `[1, 1, 1]` is `1/4 = 0.25`, which rounds to 0.
/// Averaging the level 1 pair `(1, 0)` would be 0.5 and would round to 1.
#[test]
fn fixture_every_level_is_a_downsample_of_level_zero() -> Result<(), VolumeError> {
    let mut volume = filled([4, 1, 1], [1.0, 1.0, 1.0], VoxelKind::U8, &[0, 1, 0, 0])?;
    volume.downsample_level([2, 1, 1])?;
    assert_eq!(volume.level_bytes(1)?, &[1, 0]);
    volume.downsample_level([1, 1, 1])?;
    assert_eq!(volume.level_bytes(2)?, &[0]);
    Ok(())
}

/// AC5. The mean of in-range samples stays in range.
/// Eight `u16` samples of 65535 stay 65535. Eight `i16` samples of -32768
/// stay -32768. PS3.3 C.7.6.3 for the signed minimum.
#[test]
fn fixture_extremes_do_not_overflow() -> Result<(), VolumeError> {
    let unsigned = le_u16(&[0xFFFF; 8]);
    let mut wide = filled([2, 2, 2], [1.0, 1.0, 1.0], VoxelKind::U16, &unsigned)?;
    wide.downsample_level([1, 1, 1])?;
    assert_eq!(wide.level_bytes(1)?, &[0xFF, 0xFF]);

    let signed = le_i16(&[-32768; 8]);
    let mut narrow = filled([2, 2, 2], [1.0, 1.0, 1.0], VoxelKind::I16, &signed)?;
    narrow.downsample_level([1, 1, 1])?;
    assert_eq!(narrow.level_bytes(1)?, &[0x00, 0x80]);
    let pair = [
        narrow
            .level_bytes(1)?
            .first()
            .copied()
            .ok_or(VolumeError::LevelShape)?,
        narrow
            .level_bytes(1)?
            .get(1)
            .copied()
            .ok_or(VolumeError::LevelShape)?,
    ];
    assert_eq!(i16::from_le_bytes(pair), -32768);
    Ok(())
}

/// AC6. `(1.0 + 4.0) / 2 = 2.5`. `(-3.0 + 0.5) / 2 = -1.25`. Compared by bits.
///
/// `[2, 4, 4]` to `[1, 2, 2]` watches a y weight and a z weight above 1.
/// Values are `index + 1`, x fastest. Child `(0, 0, 0)` covers eight parents.
/// Values `1, 2, 3, 4, 9, 10, 11, 12` sum to 52. Axis weights are 1, 2 and 2,
/// so each parent weighs `1 * 2 * 2 = 4`. The divisor is `2 * 4 * 4 = 32`.
/// `52 * 4 / 32 = 6.5`.
///
/// `[4, 6, 8]` to `[2, 3, 4]` watches an x weight above 1 as well.
/// Child `(0, 0, 0)` covers values `1, 2, 5, 6, 25, 26, 29, 30`, summing to 124.
/// Axis weights are 2, 3 and 4, so each parent weighs 24.
/// The divisor is `4 * 6 * 8 = 192`. `124 * 24 / 192 = 15.5`.
///
/// Decision 5 takes the mean in `f64`. `[3, 1, 1]` of `16777216`, `1` and `1`
/// downsampled to `[1, 1, 1]` has weight 1 on each sample and divisor 3.
/// The exact sum is 16777218 and `16777218 / 3 = 5592406.0`.
/// Those bits are 1252698796.
/// A running `f32` sum loses both ones at `2^24` and leaves
/// `16777216 / 3 = 5592405.5`, whose bits are 1252698795.
///
/// `[3, 1, 1]` of `33554432`, `5` and `8388609` sums to 41943046.
/// `41943046 / 3 = 13981015.33`, which narrows to `13981015.0`.
/// Those bits are 1263883607.
/// Rounding the sum to `f32` first is a tie and goes to 41943048.
/// `41943048 / 3 = 13981016`.
#[test]
fn fixture_float_mean() -> Result<(), VolumeError> {
    let mut positive = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[1.0, 4.0]),
    )?;
    positive.downsample_level([1, 1, 1])?;
    assert_eq!(
        read_f32(positive.level_bytes(1)?)?.to_bits(),
        2.5_f32.to_bits()
    );

    let mut mixed = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[-3.0, 0.5]),
    )?;
    mixed.downsample_level([1, 1, 1])?;
    assert_eq!(
        read_f32(mixed.level_bytes(1)?)?.to_bits(),
        (-1.25_f32).to_bits()
    );

    let mut grid_values = Vec::new();
    let mut index = 0u8;
    while index < 32 {
        grid_values.push(f32::from(index) + 1.0);
        index += 1;
    }
    let mut grid = filled(
        [2, 4, 4],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&grid_values),
    )?;
    grid.downsample_level([1, 2, 2])?;
    assert_eq!(read_f32(grid.level_bytes(1)?)?.to_bits(), 6.5_f32.to_bits());

    let mut wide_values = Vec::new();
    let mut wide_index = 0u16;
    while wide_index < 192 {
        wide_values.push(f32::from(wide_index) + 1.0);
        wide_index += 1;
    }
    let mut wide = filled(
        [4, 6, 8],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&wide_values),
    )?;
    wide.downsample_level([2, 3, 4])?;
    assert_eq!(
        read_f32(wide.level_bytes(1)?)?.to_bits(),
        15.5_f32.to_bits()
    );

    let mut lost_units = filled(
        [3, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[16_777_216.0, 1.0, 1.0]),
    )?;
    lost_units.downsample_level([1, 1, 1])?;
    assert_eq!(
        read_f32(lost_units.level_bytes(1)?)?.to_bits(),
        1_252_698_796
    );

    let mut tie_sum = filled(
        [3, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[33_554_432.0, 5.0, 8_388_609.0]),
    )?;
    tie_sum.downsample_level([1, 1, 1])?;
    assert_eq!(read_f32(tie_sum.level_bytes(1)?)?.to_bits(), 1_263_883_607);
    Ok(())
}

/// AC7. PS3.3 C.7.6.2 extended by the dimension ratio.
/// `0.5 * 6/3 = 1`, `0.75 * 4/2 = 1.5`, `2.5 * 3/2 = 3.75`.
/// Doubling spacing would make z 5. Copying level 0 spacing would leave z at 2.5.
/// The x and y ratios above are both 2, so `[8, 6, 5]` to `[2, 3, 2]` separates them.
/// `1 * 8/2 = 4`, `1 * 6/3 = 2`, `1 * 5/2 = 2.5`.
#[test]
fn fixture_spacing_is_the_dimension_ratio() -> Result<(), VolumeError> {
    let voxels = 6 * 4 * 3;
    let mut volume = filled(
        [6, 4, 3],
        [0.5, 0.75, 2.5],
        VoxelKind::U8,
        &repeat_u8(voxels, 0),
    )?;
    volume.downsample_level([3, 2, 2])?;
    let spacing = volume.level(1)?.spacing;
    let expected = [1.0_f64, 1.5, 3.75];
    assert_eq!(spacing.map(f64::to_bits), expected.map(f64::to_bits));
    assert_ne!(spacing[2].to_bits(), 5.0_f64.to_bits());
    assert_ne!(spacing[2].to_bits(), 2.5_f64.to_bits());

    let mut axes = filled(
        [8, 6, 5],
        [1.0, 1.0, 1.0],
        VoxelKind::U8,
        &repeat_u8(8 * 6 * 5, 0),
    )?;
    axes.downsample_level([2, 3, 2])?;
    let distinct = axes.level(1)?.spacing;
    assert_eq!(
        distinct.map(f64::to_bits),
        [4.0_f64, 2.0, 2.5].map(f64::to_bits)
    );
    Ok(())
}

/// AC8. `ceil` halving stops after the first one-brick level.
///
/// U16 edge is 64. Dims `[130, 64, 64]`, `[65, 32, 32]`, `[33, 16, 16]`.
/// Grids `[3, 1, 1]`, `[2, 1, 1]`, `[1, 1, 1]`.
/// U8 edge is 128, so the same level 0 dims stop at 2 levels.
/// U8 `[4, 4, 129]` has grid `[1, 1, 2]`. Level 1 is `[2, 2, 65]`.
/// `ceil(4 / 2) = 2` and `ceil(129 / 2) = 65`. Floor on z would leave 64.
#[test]
fn fixture_pyramid_levels_follow_brick_grid() -> Result<(), VolumeError> {
    let voxels = 130 * 64 * 64;
    let mut wide = filled(
        [130, 64, 64],
        [1.0, 1.0, 1.0],
        VoxelKind::U16,
        &repeat_u16(voxels, 1000),
    )?;
    assert_eq!(wide.generate_pyramid()?, 3);
    assert_eq!(wide.level(0)?.dims, [130, 64, 64]);
    assert_eq!(wide.level(1)?.dims, [65, 32, 32]);
    assert_eq!(wide.level(2)?.dims, [33, 16, 16]);
    let mut index = 0u32;
    while index < 3 {
        let grid = wide.brick_grid_at(index)?;
        assert_eq!(grid.edge, 64);
        assert_eq!(grid.level, index);
        assert_u16_constant(wide.level_bytes(index)?, 1000)?;
        index += 1;
    }
    assert_eq!(wide.brick_grid_at(0)?.grid, [3, 1, 1]);
    assert_eq!(wide.brick_grid_at(1)?.grid, [2, 1, 1]);
    assert_eq!(wide.brick_grid_at(2)?.grid, [1, 1, 1]);

    let mut bytes = filled(
        [130, 64, 64],
        [1.0, 1.0, 1.0],
        VoxelKind::U8,
        &repeat_u8(voxels, 7),
    )?;
    assert_eq!(bytes.generate_pyramid()?, 2);
    assert_eq!(bytes.brick_grid_at(0)?.edge, 128);
    assert_eq!(bytes.brick_grid_at(1)?.edge, 128);
    assert_eq!(bytes.level(1)?.dims, [65, 32, 32]);
    assert_eq!(bytes.brick_grid_at(1)?.grid, [1, 1, 1]);
    assert_eq!(bytes.level(2), Err(VolumeError::LevelAbsent { index: 2 }));
    assert_u8_constant(bytes.level_bytes(0)?, 7);
    assert_u8_constant(bytes.level_bytes(1)?, 7);

    let mut stack = filled(
        [4, 4, 129],
        [1.0, 1.0, 1.0],
        VoxelKind::U8,
        &repeat_u8(4 * 4 * 129, 3),
    )?;
    assert_eq!(stack.brick_grid_at(0)?.grid, [1, 1, 2]);
    assert_eq!(stack.generate_pyramid()?, 2);
    assert_eq!(stack.level(1)?.dims, [2, 2, 65]);
    Ok(())
}

/// Odd length on every axis. U8 `[3, 129, 5]` has grid `[1, 2, 1]`.
/// Level 1 is `[2, 65, 3]`.
/// `ceil(3 / 2) = 2`, `ceil(129 / 2) = 65` and `ceil(5 / 2) = 3`.
/// The one brick has that extent. The next brick on each axis is refused.
#[test]
fn fixture_each_axis_keeps_ceil_stop_and_brick_bounds() -> Result<(), VolumeError> {
    let mut volume = filled(
        [3, 129, 5],
        [1.0, 1.0, 1.0],
        VoxelKind::U8,
        &repeat_u8(3 * 129 * 5, 3),
    )?;
    assert_eq!(volume.brick_grid_at(0)?.grid, [1, 2, 1]);
    assert_eq!(volume.generate_pyramid()?, 2);
    assert_eq!(volume.level(1)?.dims, [2, 65, 3]);
    let brick = volume.cut_level_brick(1, [0, 0, 0])?;
    assert_eq!(brick.extent, [2, 65, 3]);
    assert_eq!(
        volume.cut_level_brick(1, [1, 0, 0]),
        Err(VolumeError::BrickCut { axis: 0, value: 1 })
    );
    assert_eq!(
        volume.cut_level_brick(1, [0, 1, 0]),
        Err(VolumeError::BrickCut { axis: 1, value: 1 })
    );
    assert_eq!(
        volume.cut_level_brick(1, [0, 0, 1]),
        Err(VolumeError::BrickCut { axis: 2, value: 1 })
    );
    Ok(())
}

/// AC9. A one-brick level 0 gains no coarser level, and a missing index is not level 0.
#[test]
fn a_missing_generated_level_is_not_level_zero() -> Result<(), VolumeError> {
    let mut volume = filled([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8, &repeat_u8(64, 9))?;
    assert_eq!(volume.generate_pyramid()?, 1);
    assert_eq!(volume.levels.len(), 1);
    let absent = VolumeError::LevelAbsent { index: 1 };
    assert_eq!(volume.level(1), Err(absent));
    assert_eq!(volume.brick_grid_at(1), Err(absent));
    assert_eq!(volume.cut_level_brick(1, [0, 0, 0]), Err(absent));
    assert_eq!(
        volume.level_voxel_to_level0(1, [0.0, 0.0, 0.0]),
        Err(absent)
    );
    Ok(())
}

/// AC10. Generation leaves level 0 bytes and the level 0 entry alone.
/// The budget is the sum of the three level lengths.
#[test]
fn generation_keeps_level_zero() -> Result<(), VolumeError> {
    let voxels = 130 * 64 * 64;
    let mut volume = filled(
        [130, 64, 64],
        [1.0, 1.0, 2.0],
        VoxelKind::U16,
        &repeat_u16(voxels, 1000),
    )?;
    let before = volume.level_bytes(0)?.to_vec();
    let level0 = *volume.level(0)?;
    volume.generate_pyramid()?;
    assert_eq!(volume.level_bytes(0)?, before.as_slice());
    assert_eq!(*volume.level(0)?, level0);
    assert_eq!(volume.bytes(), 1_064_960 + 133_120 + 16_896);
    Ok(())
}

/// AC11. An absent slice is not averaged as zero. The first gap is named.
/// An interior gap and a missing last slice are both refused. The last
/// slice is `dz - 1`, which an off-by-one completeness loop never visits.
#[test]
fn incomplete_level_zero_is_refused() -> Result<(), VolumeError> {
    let mut volume = bare([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8);
    let slice = [1_u8; 16];
    volume.place(0, &slice)?;
    volume.place(1, &slice)?;
    volume.place(3, &slice)?;
    let before = volume.clone();
    assert_eq!(
        volume.generate_pyramid(),
        Err(VolumeError::SliceNotPresent { index: 2 })
    );
    assert_eq!(
        volume.downsample_level([2, 2, 2]),
        Err(VolumeError::SliceNotPresent { index: 2 })
    );
    assert_eq!(volume.levels.len(), 1);
    assert_eq!(volume.data.len(), 64);
    assert_eq!(volume, before);

    let mut tail = bare([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8);
    tail.place(0, &slice)?;
    tail.place(1, &slice)?;
    tail.place(2, &slice)?;
    let [_dx, _dy, dz] = tail.dims;
    let last = dz.checked_sub(1).ok_or(VolumeError::LevelShape)?;
    let before_tail = tail.clone();
    assert_eq!(
        tail.generate_pyramid(),
        Err(VolumeError::SliceNotPresent { index: last })
    );
    assert_eq!(
        tail.downsample_level([2, 2, 2]),
        Err(VolumeError::SliceNotPresent { index: last })
    );
    assert_eq!(tail.levels.len(), 1);
    assert_eq!(tail.data.len(), 64);
    assert_eq!(tail, before_tail);

    // Slice 0 is absent and a caller level is already stored. The refusal
    // is checked before truncation, so the appended bytes stay.
    let mut caller = bare([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8);
    caller.place(1, &slice)?;
    caller.place(2, &slice)?;
    caller.place(3, &slice)?;
    caller.append_level([2, 2, 2], &[200_u8; 8])?;
    let before_caller = caller.clone();
    assert_eq!(
        caller.generate_pyramid(),
        Err(VolumeError::SliceNotPresent { index: 0 })
    );
    assert_eq!(
        caller.downsample_level([2, 2, 2]),
        Err(VolumeError::SliceNotPresent { index: 0 })
    );
    assert_eq!(caller, before_caller);
    Ok(())
}

/// AC12. A shape that is not strictly coarser leaves the store alone.
#[test]
fn a_bad_downsample_shape_leaves_the_store() -> Result<(), VolumeError> {
    let mut volume = filled([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8, &repeat_u8(64, 3))?;
    let before = volume.clone();
    assert_eq!(
        volume.downsample_level([4, 4, 4]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([0, 2, 2]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([5, 2, 2]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([2, 5, 2]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([2, 2, 5]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([2, 0, 2]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([2, 2, 0]),
        Err(VolumeError::LevelShape)
    );
    // `u32::MAX` is past level 0. The shape check returns before the child
    // walk, which would not finish.
    assert_eq!(
        volume.downsample_level([u32::MAX, 1, 1]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([1, u32::MAX, 1]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.downsample_level([1, 1, u32::MAX]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(volume, before);
    Ok(())
}

/// One shorter axis is enough. AC3 shortens x. These shorten y and z.
#[test]
fn each_axis_can_make_the_level_coarser() -> Result<(), VolumeError> {
    let mut y_only = filled([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8, &repeat_u8(64, 3))?;
    y_only.downsample_level([4, 2, 4])?;
    assert_eq!(y_only.level(1)?.dims, [4, 2, 4]);
    assert_u8_constant(y_only.level_bytes(1)?, 3);

    let mut z_only = filled([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8, &repeat_u8(64, 3))?;
    z_only.downsample_level([4, 4, 2])?;
    assert_eq!(z_only.level(1)?.dims, [4, 4, 2]);
    assert_u8_constant(z_only.level_bytes(1)?, 3);
    Ok(())
}

/// Y and z overlaps differ. Level 0 is `[1, 5, 3]`, values `index + 1`.
/// Child dims are `[1, 3, 2]`. The divisor is `1 * 5 * 3 = 15`.
///
/// Y weights are the AC3 overlaps: `(3, 2)`, `(1, 3, 1)` and `(2, 3)`.
/// Z weights are `(2, 1)` then `(1, 2)`. X weight is 1.
/// Child `(0, 0, 0)` sums `6*1 + 4*2 + 3*6 + 2*7 = 46`. `46/15` rounds to 3.
/// The six children round to `3, 5, 6, 10, 11, 13`.
#[test]
fn fixture_y_and_z_overlaps_differ() -> Result<(), VolumeError> {
    let samples = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let mut volume = filled([1, 5, 3], [1.0, 1.0, 1.0], VoxelKind::U8, &samples)?;
    volume.downsample_level([1, 3, 2])?;
    assert_eq!(volume.level_bytes(1)?, &[3, 5, 6, 10, 11, 13]);
    Ok(())
}

/// AC13. A level 1 brick is cut from that level and shares the one brick cache.
#[test]
fn level_bricks_share_one_cache() -> Result<(), VolumeError> {
    let samples = [0_u8, 1, 0, 0, 2, 3, 0, 0, 4, 5, 5, 5, 6, 7, 5, 5];
    let mut volume = filled([4, 2, 2], [1.0, 2.0, 3.0], VoxelKind::U8, &samples)?;
    volume.downsample_level([2, 1, 1])?;
    let coarse = volume.cut_level_brick(1, [0, 0, 0])?;
    assert_eq!(coarse.extent, [2, 1, 1]);
    assert_eq!(coarse.samples, vec![4, 3]);
    assert_eq!(coarse.sample, BrickSample::U8);
    let from_level = volume.cut_level_brick(0, [0, 0, 0])?;
    let from_ready = volume.cut_ready_brick([0, 0, 0])?;
    assert_eq!(from_level, from_ready);

    let id = VolumeId(7);
    let mut cache = BrickCache::new(4_096);
    let key0 = BrickKey {
        volume: id,
        level: 0,
        brick: [0, 0, 0],
    };
    let key1 = BrickKey {
        volume: id,
        level: 1,
        brick: [0, 0, 0],
    };
    let admitted0 = cache.insert(key0, from_level);
    assert!(admitted0.refused.is_none());
    let admitted1 = cache.insert(key1, coarse);
    assert!(admitted1.refused.is_none());
    assert!(admitted1.displaced.is_none());
    let grid = volume.brick_grid_at(1)?;
    assert_eq!(grid.level, 1);
    assert!(cache.missing(id, &grid).is_empty());
    let stored0 = cache.get(&key0).ok_or(VolumeError::LevelShape)?;
    assert_eq!(stored0.samples, samples);
    let stored1 = cache.get(&key1).ok_or(VolumeError::LevelShape)?;
    assert_eq!(stored1.samples, vec![4, 3]);
    Ok(())
}

/// AC14. A second generation replaces levels above 0 with the same bytes.
#[test]
fn regenerating_is_idempotent() -> Result<(), VolumeError> {
    let voxels = 130 * 64 * 64;
    let mut volume = filled(
        [130, 64, 64],
        [1.0, 1.0, 1.0],
        VoxelKind::U16,
        &repeat_u16(voxels, 1000),
    )?;
    assert_eq!(volume.generate_pyramid()?, 3);
    let first = volume.clone();
    assert_eq!(volume.generate_pyramid()?, 3);
    assert_eq!(volume.levels, first.levels);
    assert_eq!(volume.data, first.data);
    assert_eq!(volume.bytes(), first.bytes());
    Ok(())
}

/// Decision 11. `generate_pyramid` replaces levels above 0.
///
/// Level 0 is U8 `[130, 64, 64]` of constant 7. The appended level is
/// `[65, 32, 32]` of constant 200. A box mean of a constant field is that
/// constant, so level 1 is 7 after generation. Skipping the truncation
/// would leave the appended 200 in place.
#[test]
fn generate_pyramid_replaces_caller_appended_levels() -> Result<(), VolumeError> {
    let voxels = 130 * 64 * 64;
    let mut volume = filled(
        [130, 64, 64],
        [1.0, 1.0, 1.0],
        VoxelKind::U8,
        &repeat_u8(voxels, 7),
    )?;
    let coarse = 65 * 32 * 32;
    let index = volume.append_level([65, 32, 32], &repeat_u8(coarse, 200))?;
    assert_eq!(index, 1);
    assert_u8_constant(volume.level_bytes(1)?, 200);
    assert_eq!(volume.generate_pyramid()?, 2);
    assert_eq!(volume.levels.len(), 2);
    assert_u8_constant(volume.level_bytes(0)?, 7);
    assert_u8_constant(volume.level_bytes(1)?, 7);
    assert_eq!(volume.level(1)?.dims, [65, 32, 32]);
    assert_eq!(volume.level(2), Err(VolumeError::LevelAbsent { index: 2 }));
    Ok(())
}

/// AC15. A successful place drops levels above 0. A short buffer does not.
/// An index past the last slice does not either.
#[test]
fn placing_level_zero_drops_stale_levels() -> Result<(), VolumeError> {
    let samples = [0_u8, 1, 0, 0, 2, 3, 0, 0, 4, 5, 5, 5, 6, 7, 5, 5];
    let mut volume = filled([4, 2, 2], [1.0, 2.0, 3.0], VoxelKind::U8, &samples)?;
    volume.downsample_level([2, 1, 1])?;
    assert_eq!(volume.level_bytes(1)?, &[4, 3]);
    assert_eq!(volume.data.len(), 18);
    let refused = volume.place(0, &[1, 2, 3]);
    assert_eq!(
        refused,
        Err(VolumeError::SliceSizeMismatch {
            expected: 8,
            actual: 3,
        })
    );
    assert_eq!(volume.levels.len(), 2);
    assert_eq!(volume.level_bytes(1)?, &[4, 3]);
    assert_eq!(volume.data.len(), 18);
    let outside = volume.place(2, &[0_u8; 8]);
    assert_eq!(
        outside,
        Err(VolumeError::SliceOutOfRange { index: 2, count: 2 })
    );
    assert_eq!(volume.levels.len(), 2);
    assert_eq!(volume.level_bytes(1)?, &[4, 3]);
    assert_eq!(volume.data.len(), 18);
    let slice = [9_u8; 8];
    volume.place(0, &slice)?;
    assert_eq!(volume.levels.len(), 1);
    assert_eq!(volume.data.len(), 16);
    assert_eq!(volume.data.slice(0), Some(slice.as_slice()));
    assert_eq!(volume.level(1), Err(VolumeError::LevelAbsent { index: 1 }));
    Ok(())
}

/// AC16. PS3.3 C.7.6.2.1.1. The level voxel centre is
/// `(c + 0.5) * dims_0 / dims_k - 0.5`.
/// Halved axes put voxel 0 at 0.5 and voxel 1 at 2.5.
/// The odd axis puts voxel 2 at `2.5 * 5 / 3 - 0.5`.
/// Level 0 maps a voxel to itself, including a fractional index.
#[test]
fn fixture_level_voxel_centres() -> Result<(), VolumeError> {
    let samples = [0_u8, 1, 0, 0, 2, 3, 0, 0, 4, 5, 5, 5, 6, 7, 5, 5];
    let mut even = filled([4, 2, 2], [1.0, 2.0, 3.0], VoxelKind::U8, &samples)?;
    even.downsample_level([2, 1, 1])?;
    let origin = even.level_voxel_to_level0(1, [0.0, 0.0, 0.0])?;
    assert_eq!(origin.map(f64::to_bits), [0.5, 0.5, 0.5].map(f64::to_bits));
    let next = even.level_voxel_to_level0(1, [1.0, 0.0, 0.0])?;
    assert_eq!(next.map(f64::to_bits), [2.5, 0.5, 0.5].map(f64::to_bits));
    let same = even.level_voxel_to_level0(0, [1.0, 0.0, 0.0])?;
    assert_eq!(same.map(f64::to_bits), [1.0, 0.0, 0.0].map(f64::to_bits));

    // Axis length 1319. Input 119.25118477159562.
    // `(c + 0.5) * 1319 / 1319 - 0.5` is 119.25118477159563, one ULP high.
    // Level 0 returns the input bits.
    let fractional = 119.25118477159562;
    let wide = bare([1319, 1, 1], [1.0, 1.0, 1.0], VoxelKind::U8);
    let mapped = wide.level_voxel_to_level0(0, [fractional, 0.0, 0.0])?;
    assert_eq!(
        mapped.map(f64::to_bits),
        [fractional, 0.0, 0.0].map(f64::to_bits)
    );

    let mut odd = filled(
        [5, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::U8,
        &[10, 20, 30, 40, 50],
    )?;
    odd.downsample_level([3, 1, 1])?;
    let [x, y, z] = odd.level_voxel_to_level0(1, [2.0, 0.0, 0.0])?;
    let expected_x = 2.5 * 5.0 / 3.0 - 0.5;
    assert!((x - expected_x).abs() < 1e-12);
    assert_eq!(y.to_bits(), 0.0_f64.to_bits());
    assert_eq!(z.to_bits(), 0.0_f64.to_bits());

    // [2, 5, 1] to [1, 3, 1]. Voxel [0, 2, 0.25].
    // x is `(0.5) * 2 / 1 - 0.5 = 0.5`.
    // y is `(2.5) * 5 / 3 - 0.5 = 11/3`.
    // z dims match, so 0.25 stays 0.25.
    let mut split = filled([2, 5, 1], [1.0, 1.0, 1.0], VoxelKind::U8, &repeat_u8(10, 0))?;
    split.downsample_level([1, 3, 1])?;
    let [sx, sy, sz] = split.level_voxel_to_level0(1, [0.0, 2.0, 0.25])?;
    assert_eq!(sx.to_bits(), 0.5_f64.to_bits());
    let expected_sy = 2.5 * 5.0 / 3.0 - 0.5;
    assert!((sy - expected_sy).abs() < 1e-12);
    assert_eq!(sz.to_bits(), 0.25_f64.to_bits());

    // The equal-dims shortcut is per axis. Dim 1319 and this input are one
    // ULP high in the product form, the same pair as the x case above.
    let tall_y = bare([1, 1319, 1], [1.0, 1.0, 1.0], VoxelKind::U8);
    let mapped_y = tall_y.level_voxel_to_level0(0, [0.0, fractional, 0.0])?;
    assert_eq!(
        mapped_y.map(f64::to_bits),
        [0.0, fractional, 0.0].map(f64::to_bits)
    );
    let tall_z = bare([1, 1, 1319], [1.0, 1.0, 1.0], VoxelKind::U8);
    let mapped_z = tall_z.level_voxel_to_level0(0, [0.0, 0.0, fractional])?;
    assert_eq!(
        mapped_z.map(f64::to_bits),
        [0.0, 0.0, fractional].map(f64::to_bits)
    );
    Ok(())
}

/// A coarse brick keeps the volume's sample kind.
///
/// U16 level 0 is `[130, 4, 1]`, value `300 * x`. Level 1 is `[65, 2, 1]`.
/// The brick at `[0, 0, 0]` has extent `[64, 2, 1]` because the sixteen-bit
/// edge is 64. `64 * 2 * 1 * 2 = 256` bytes.
/// Voxel `(1, 0, 0)` covers level 0 x parents 2 and 3, values 600 and 900.
/// The y weights are 2 and 2 and the z weight is 1, so they cancel.
/// `(600 + 900) / 2 = 750`, little-endian `EE 02`.
///
/// I16 `[2, 1, 1]` of `-3, -3` downsamples to `-3`, little-endian `FD FF`.
/// PS3.3 C.7.6.3. A `U16` tag would read those bytes as 65533.
///
/// F32 `[2, 1, 1]` of `1.0, 3.0` is the mean `2.0`.
#[test]
fn coarse_level_brick_keeps_its_sample_kind() -> Result<(), VolumeError> {
    let mut wide = filled(
        [130, 4, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::U16,
        &ramp_u16([130, 4, 1], 300)?,
    )?;
    assert_eq!(wide.generate_pyramid()?, 3);
    let brick = wide.cut_level_brick(1, [0, 0, 0])?;
    assert_eq!(brick.sample, BrickSample::U16);
    assert_eq!(brick.extent, [64, 2, 1]);
    assert_eq!(brick.samples.len(), 256);
    let pair = [
        brick
            .samples
            .get(2)
            .copied()
            .ok_or(VolumeError::LevelShape)?,
        brick
            .samples
            .get(3)
            .copied()
            .ok_or(VolumeError::LevelShape)?,
    ];
    assert_eq!(pair, [0xEE, 0x02]);
    assert_eq!(u16::from_le_bytes(pair), 750);

    let mut signed = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::I16,
        &le_i16(&[-3, -3]),
    )?;
    signed.downsample_level([1, 1, 1])?;
    let signed_brick = signed.cut_level_brick(1, [0, 0, 0])?;
    assert_eq!(signed_brick.sample, BrickSample::I16);
    assert_eq!(signed_brick.extent, [1, 1, 1]);
    assert_eq!(signed_brick.samples.as_slice(), &[0xFD, 0xFF]);

    let mut floats = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[1.0, 3.0]),
    )?;
    floats.downsample_level([1, 1, 1])?;
    let float_brick = floats.cut_level_brick(1, [0, 0, 0])?;
    assert_eq!(float_brick.sample, BrickSample::F32);
    assert_eq!(float_brick.extent, [1, 1, 1]);
    assert_eq!(float_brick.samples.len(), 4);
    assert_eq!(read_f32(&float_brick.samples)?.to_bits(), 2.0_f32.to_bits());
    Ok(())
}

fn ramp_u16(dims: [u32; 3], scale: u16) -> Result<Vec<u8>, VolumeError> {
    let [dx, dy, dz] = dims;
    let mut out = Vec::new();
    let mut z = 0u32;
    while z < dz {
        let mut y = 0u32;
        while y < dy {
            let mut x = 0u32;
            while x < dx {
                let wide = u32::from(scale)
                    .checked_mul(x)
                    .ok_or(VolumeError::LevelShape)?;
                let value = u16::try_from(wide).map_err(|_err| VolumeError::LevelShape)?;
                out.extend(value.to_le_bytes());
                x += 1;
            }
            y += 1;
        }
        z += 1;
    }
    Ok(out)
}

/// Level 0 goes through `cut_ready_brick`. An absent slice is not cut as zero.
///
/// `[4, 4, 4]` U8 with only slice 0 placed. The one brick is `[0, 0, 0]`.
/// Slices 1, 2 and 3 are absent, so the cut is `BrickNotReady`.
#[test]
fn level_zero_cut_on_an_absent_slice_is_not_ready() -> Result<(), VolumeError> {
    let mut volume = bare([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8);
    volume.place(0, &[1_u8; 16])?;
    let missing = Err(VolumeError::BrickNotReady { brick: [0, 0, 0] });
    assert_eq!(volume.cut_level_brick(0, [0, 0, 0]), missing);
    assert_eq!(volume.cut_ready_brick([0, 0, 0]), missing);
    Ok(())
}

/// Level 0 forwards each brick coordinate. Brick 1 is not brick 0.
///
/// U8 edge is 128. Length 200 gives grid 2 on that axis and extent 72
/// on brick 1, whose first sample is 128 and whose last is 199.
#[test]
fn level_zero_cut_forwards_each_brick_coordinate() -> Result<(), VolumeError> {
    let mut samples = Vec::new();
    let mut value = 0u32;
    while value < 200 {
        samples.push(u8::try_from(value).map_err(|_err| VolumeError::LevelShape)?);
        value += 1;
    }
    let along_x = filled([200, 1, 1], [1.0, 1.0, 1.0], VoxelKind::U8, &samples)?;
    let x_brick = along_x.cut_level_brick(0, [1, 0, 0])?;
    assert_eq!(x_brick, along_x.cut_ready_brick([1, 0, 0])?);
    assert_eq!(x_brick.extent, [72, 1, 1]);
    assert_eq!(x_brick.samples.first().copied(), Some(128));
    assert_eq!(x_brick.samples.last().copied(), Some(199));

    let along_y = filled([1, 200, 1], [1.0, 1.0, 1.0], VoxelKind::U8, &samples)?;
    let y_brick = along_y.cut_level_brick(0, [0, 1, 0])?;
    assert_eq!(y_brick, along_y.cut_ready_brick([0, 1, 0])?);
    assert_eq!(y_brick.extent, [1, 72, 1]);
    assert_eq!(y_brick.samples.first().copied(), Some(128));
    assert_eq!(y_brick.samples.last().copied(), Some(199));

    let along_z = filled([1, 1, 200], [1.0, 1.0, 1.0], VoxelKind::U8, &samples)?;
    let z_brick = along_z.cut_level_brick(0, [0, 0, 1])?;
    assert_eq!(z_brick, along_z.cut_ready_brick([0, 0, 1])?);
    assert_eq!(z_brick.extent, [1, 1, 72]);
    assert_eq!(z_brick.samples.first().copied(), Some(128));
    assert_eq!(z_brick.samples.last().copied(), Some(199));
    Ok(())
}

/// Two gaps. The refusal names the first absent slice, index 0, not index 2.
#[test]
fn the_first_absent_slice_is_the_one_named() -> Result<(), VolumeError> {
    let mut volume = bare([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8);
    let slice = [1_u8; 16];
    volume.place(1, &slice)?;
    volume.place(3, &slice)?;
    let before = volume.clone();
    assert_eq!(
        volume.generate_pyramid(),
        Err(VolumeError::SliceNotPresent { index: 0 })
    );
    assert_eq!(
        volume.downsample_level([2, 2, 2]),
        Err(VolumeError::SliceNotPresent { index: 0 })
    );
    assert_eq!(volume, before);
    Ok(())
}

/// Completeness is checked before the shape. Both are wrong here, and the
/// absent slice is the error that comes back. The volume stays as it was.
#[test]
fn an_incomplete_volume_reports_the_gap_before_the_shape() -> Result<(), VolumeError> {
    let mut volume = bare([4, 4, 4], [1.0, 1.0, 1.0], VoxelKind::U8);
    volume.place(0, &[1_u8; 16])?;
    let before = volume.clone();
    assert_eq!(
        volume.downsample_level([4, 4, 4]),
        Err(VolumeError::SliceNotPresent { index: 1 })
    );
    assert_eq!(
        volume.downsample_level([0, 2, 2]),
        Err(VolumeError::SliceNotPresent { index: 1 })
    );
    // Completeness wins before a child walk. `u32::MAX` is also the wrong
    // shape, and walking it would not finish.
    assert_eq!(
        volume.downsample_level([u32::MAX, 1, 1]),
        Err(VolumeError::SliceNotPresent { index: 1 })
    );
    assert_eq!(volume, before);
    Ok(())
}

/// A zero dimension on level 0 is `LevelShape` and the volume is unchanged.
///
/// The fields are public, so `[0, 4, 4]`, `[4, 0, 4]` and `[4, 4, 0]` can
/// be built. Ceiling division of 0 is 0, so the brick grid is not one brick.
/// The child that follows is longer than the zero axis, and the shape check
/// refuses it.
#[test]
fn a_zero_dimension_returns_level_shape() -> Result<(), VolumeError> {
    let shapes = [[0, 4, 4], [4, 0, 4], [4, 4, 0]];
    for dims in shapes {
        let mut volume = bare(dims, [1.0, 1.0, 1.0], VoxelKind::U8);
        let [_dx, _dy, dz] = dims;
        let mut index = 0u32;
        while index < dz {
            volume.place(index, &[])?;
            index += 1;
        }
        let before = volume.clone();
        assert_eq!(volume.generate_pyramid(), Err(VolumeError::LevelShape));
        assert_eq!(volume.levels.len(), 1);
        assert_eq!(volume, before);
    }
    Ok(())
}

/// `levels` is public, so a complete volume can hold an empty axis.
/// Both calls ask for level 0 after the completeness check and return
/// [`VolumeError::LevelAbsent`] with index 0. The volume is unchanged.
#[test]
fn an_empty_level_axis_returns_level_absent() -> Result<(), VolumeError> {
    let mut volume = filled([2, 1, 1], [1.0, 1.0, 1.0], VoxelKind::U8, &[1, 2])?;
    volume.levels.clear();
    let before = volume.clone();
    let absent = VolumeError::LevelAbsent { index: 0 };
    assert_eq!(volume.downsample_level([1, 1, 1]), Err(absent));
    assert_eq!(volume, before);
    assert_eq!(volume.generate_pyramid(), Err(absent));
    assert_eq!(volume, before);
    Ok(())
}

/// Decision 5 narrows a finite mean through `narrow_f32`.
/// It does not say what a non-finite stored sample becomes, and decision 12
/// does not say what a non-finite centre input becomes.
/// This test pins the current behaviour only.
/// The decision is open.
/// It is recorded in `docs/sprints/CURRENT_SPRINT.md` under
/// "Questions for the operator" and is to be answered before F-120.
///
/// `[2, 1, 1]` with a payload NaN and `1.0` has a NaN mean. `narrow_f32`
/// writes the canonical quiet NaN `0x7fc0_0000`.
/// `+inf` with `1.0` stays `+inf`. Two `-inf` samples stay `-inf`.
/// `+inf` with `-inf` is a NaN mean, so the same canonical NaN.
///
/// Level 0 returns a non-finite centre unchanged, payload included.
/// A coarser axis stays NaN or the same-signed infinity. The formula's NaN
/// payload is not pinned.
#[test]
fn non_finite_floats_keep_the_current_bits() -> Result<(), VolumeError> {
    let canon = f32::from_bits(0x7fc0_0000);
    let mut nan_pair = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[f32::from_bits(0x7fc0_0001), 1.0]),
    )?;
    nan_pair.downsample_level([1, 1, 1])?;
    assert_eq!(
        read_f32(nan_pair.level_bytes(1)?)?.to_bits(),
        canon.to_bits()
    );

    let mut pos_inf = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[f32::INFINITY, 1.0]),
    )?;
    pos_inf.downsample_level([1, 1, 1])?;
    assert_eq!(
        read_f32(pos_inf.level_bytes(1)?)?.to_bits(),
        f32::INFINITY.to_bits()
    );

    let mut neg_inf = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[f32::NEG_INFINITY, f32::NEG_INFINITY]),
    )?;
    neg_inf.downsample_level([1, 1, 1])?;
    assert_eq!(
        read_f32(neg_inf.level_bytes(1)?)?.to_bits(),
        f32::NEG_INFINITY.to_bits()
    );

    let mut opposite = filled(
        [2, 1, 1],
        [1.0, 1.0, 1.0],
        VoxelKind::F32,
        &le_f32(&[f32::INFINITY, f32::NEG_INFINITY]),
    )?;
    opposite.downsample_level([1, 1, 1])?;
    assert_eq!(
        read_f32(opposite.level_bytes(1)?)?.to_bits(),
        canon.to_bits()
    );

    let payload = f64::from_bits(0x7ff8_0000_0000_0001);
    let level0 = bare([4, 1, 1], [1.0, 1.0, 1.0], VoxelKind::U8);
    let same = level0.level_voxel_to_level0(0, [payload, f64::INFINITY, f64::NEG_INFINITY])?;
    assert_eq!(same[0].to_bits(), payload.to_bits());
    assert_eq!(same[1].to_bits(), f64::INFINITY.to_bits());
    assert_eq!(same[2].to_bits(), f64::NEG_INFINITY.to_bits());

    let mut coarse = filled([2, 2, 2], [1.0, 1.0, 1.0], VoxelKind::U8, &repeat_u8(8, 1))?;
    coarse.downsample_level([1, 1, 1])?;
    let mapped = coarse.level_voxel_to_level0(1, [payload, f64::INFINITY, f64::NEG_INFINITY])?;
    assert!(mapped[0].is_nan());
    assert!(mapped[1].is_infinite() && mapped[1].is_sign_positive());
    assert!(mapped[2].is_infinite() && mapped[2].is_sign_negative());
    Ok(())
}
