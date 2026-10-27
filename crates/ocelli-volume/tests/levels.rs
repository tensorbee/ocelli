//! F-058. Level-axis fixtures.
//!
//! Spacing on an appended level is level 0's spacing times
//! `dims_0[i] / dims[i]` on each axis. For `[4, 4, 4]` at `[1, 2, 3]`
//! and a `[2, 2, 2]` child, that is `[2, 4, 6]`. The offset is the
//! level-0 byte length, 64 for a `U8` cube of side 4.

use ocelli_cache::{Budgeted, DecodedFrame};
use ocelli_core::Pt;
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    PhotometricInterpretation, PixelRepresentation, PixelSpacing, SampleLayout, StoredBits,
    StoredPixelDescription,
};
use ocelli_volume::{
    FrameOfReferenceUid, SliceEvidence, Volume, VolumeCandidate, VolumeError, VoxelKind,
};

fn cube(kind: VoxelKind, fill: u8) -> Result<Volume, VolumeError> {
    let dims = [4, 4, 4];
    let spacing = [1.0, 2.0, 3.0];
    let mut volume = Volume {
        dims,
        spacing,
        origin: Pt::new(0.0, 0.0, 0.0),
        direction: glam::DMat3::IDENTITY,
        voxel: kind,
        data: ocelli_volume::VoxelStore::new(dims, kind),
        levels: alloc_level(dims, spacing),
        present: ocelli_volume::SlicePresence::new(dims[2]),
        order: vec![0, 1, 2, 3],
    };
    let voxels = 16_usize * kind.bytes_per_voxel();
    let slice = vec![fill; voxels];
    let mut index = 0_u32;
    while index < dims[2] {
        volume.place(index, &slice)?;
        index += 1;
    }
    Ok(volume)
}

fn alloc_level(dims: [u32; 3], spacing: [f64; 3]) -> Vec<ocelli_volume::Level> {
    vec![ocelli_volume::Level {
        dims,
        spacing,
        offset: 0,
    }]
}

#[test]
fn append_level_keeps_level_zero_and_scales_spacing() -> Result<(), VolumeError> {
    let mut volume = cube(VoxelKind::U8, 0x11)?;
    let index = volume.append_level([2, 2, 2], &[1, 2, 3, 4, 5, 6, 7, 8])?;
    assert_eq!(index, 1);
    assert_eq!(volume.levels.len(), 2);
    let coarse = volume.level(1)?;
    assert_eq!(coarse.dims, [2, 2, 2]);
    assert_eq!(
        coarse.spacing.map(f64::to_bits),
        [2.0, 4.0, 6.0].map(f64::to_bits)
    );
    assert_eq!(coarse.offset, 64);
    let stored = volume.data.slice(0).ok_or(VolumeError::LevelShape)?;
    assert_eq!(stored.first().copied(), Some(0x11));
    assert_eq!(volume.bytes(), 72);
    assert_eq!(volume.level_bytes(1)?, &[1, 2, 3, 4, 5, 6, 7, 8]);
    Ok(())
}

#[test]
fn a_missing_level_is_not_level_zero() -> Result<(), VolumeError> {
    let mut volume = cube(VoxelKind::U8, 0x11)?;
    volume.append_level([2, 2, 2], &[1, 2, 3, 4, 5, 6, 7, 8])?;
    let full = volume.brick_grid_at(0)?;
    let base = volume.brick_grid();
    assert_eq!(full.dims, base.dims);
    assert_eq!(full.edge, base.edge);
    assert_eq!(full.grid, base.grid);
    assert_eq!(full.level, 0);
    let coarse = volume.brick_grid_at(1)?;
    assert_eq!(coarse.dims, [2, 2, 2]);
    assert_eq!(coarse.level, 1);
    assert_eq!(volume.level(2), Err(VolumeError::LevelAbsent { index: 2 }));
    assert_eq!(
        volume.brick_grid_at(2),
        Err(VolumeError::LevelAbsent { index: 2 })
    );
    Ok(())
}

#[test]
fn a_bad_level_shape_leaves_the_store() -> Result<(), VolumeError> {
    let mut volume = cube(VoxelKind::U8, 0x11)?;
    let before = volume.bytes();
    assert_eq!(
        volume.append_level([4, 4, 4], &[0; 64]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.append_level([0, 2, 2], &[0; 0]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.append_level([5, 2, 2], &[0; 20]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(
        volume.append_level([2, 2, 2], &[0; 7]),
        Err(VolumeError::LevelShape)
    );
    assert_eq!(volume.levels.len(), 1);
    assert_eq!(volume.bytes(), before);
    Ok(())
}

#[test]
fn appended_sixteen_bit_level_keeps_the_brick_edge() -> Result<(), VolumeError> {
    let mut volume = cube(VoxelKind::I16, 0)?;
    // Four voxels, two bytes each.
    let samples = [1_u8, 0, 2, 0, 3, 0, 4, 0];
    volume.append_level([2, 1, 2], &samples)?;
    let grid = volume.brick_grid_at(1)?;
    assert_eq!(grid.edge, 64);
    assert_eq!(grid.level, 1);
    Ok(())
}

fn axial(z: f64, samples: Vec<u8>) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let bits = StoredBits::new(16, 16, 15, PixelRepresentation::Unsigned)?;
    let rows = 3_u32;
    let columns = 4_u32;
    let description =
        StoredPixelDescription::new(ImageDimensions::new(rows, columns)?, layout, bits);
    let plane = ImagePlane::new(
        ImagePositionPatient::new([0.0, 0.0, z])?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(1.0, 1.0)?,
        ImageDimensions::new(rows, columns)?,
    )?;
    Ok(DecodedFrame::new(description, plane, samples))
}

#[test]
fn construct_starts_with_one_level() -> Result<(), ocelli_pixel::PixelError> {
    let samples = vec![0_u8; 24];
    let candidate = VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        vec![
            SliceEvidence::new(axial(0.0, samples.clone())?),
            SliceEvidence::new(axial(3.0, samples)?),
        ],
    );
    let built = Volume::construct(candidate);
    let Ok(volume) = built else {
        unreachable!("two axial slices with one spacing are one volume");
    };
    assert_eq!(volume.levels.len(), 1);
    let Ok(level0) = volume.level(0) else {
        unreachable!("construct writes level 0");
    };
    assert_eq!(level0.offset, 0);
    assert_eq!(level0.dims, volume.dims);
    Ok(())
}
