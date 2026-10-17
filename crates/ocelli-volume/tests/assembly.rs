//! F-033, volume assembly from projected slice positions.
//!
//! Fixtures are hand-computed from DICOM PS3.3 C.7.6.2.1.1. Spacing is the
//! difference of `ImagePositionPatient` projected on the slice normal, never
//! a tag, so the first fixture's positions say 3.0 where a tag would say 5.0.

use ocelli_cache::DecodedFrame;
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    PhotometricInterpretation, PixelRepresentation, PixelSpacing, SampleLayout, StoredBits,
    StoredPixelDescription,
};
use ocelli_volume::{
    FrameOfReferenceUid, SliceEvidence, Volume, VolumeCandidate, VolumeError, VoxelKind,
    group_volumes,
};
use proptest::prelude::*;

fn frame(
    position: [f64; 3],
    row_cosine: [f64; 3],
    column_cosine: [f64; 3],
    spacing: [f64; 2],
    rows: u32,
    columns: u32,
    samples: Vec<u8>,
) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let bits = StoredBits::new(16, 16, 15, PixelRepresentation::Unsigned)?;
    let description =
        StoredPixelDescription::new(ImageDimensions::new(rows, columns)?, layout, bits);
    let plane = ImagePlane::new(
        ImagePositionPatient::new(position)?,
        ImageOrientationPatient::new(row_cosine, column_cosine)?,
        PixelSpacing::new(spacing[0], spacing[1])?,
        ImageDimensions::new(rows, columns)?,
    )?;
    Ok(DecodedFrame::new(description, plane, samples))
}

fn axial(z: f64, samples: Vec<u8>) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    frame(
        [0.0, 0.0, z],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0],
        3,
        4,
        samples,
    )
}

fn candidate(frames: Vec<DecodedFrame>) -> VolumeCandidate {
    VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap_or_else(|_| unreachable!("fixture orientation must validate")),
        frames.into_iter().map(SliceEvidence::new).collect(),
    )
}

fn slice_bytes() -> Vec<u8> {
    vec![0_u8; 24]
}

#[test]
fn spacing_is_the_projected_difference_and_not_a_tag() -> Result<(), ocelli_pixel::PixelError> {
    // Positions 0, 3, 6, 9, 12. A SpacingBetweenSlices tag would say 5.0,
    // and this fixture has no such tag at all, so a tag reader cannot pass.
    let mut frames = Vec::new();
    for z in [0.0, 3.0, 6.0, 9.0, 12.0] {
        frames.push(axial(z, slice_bytes())?);
    }
    let volume = Volume::assemble(candidate(frames));
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert!((volume.spacing[2] - 3.0).abs() < 1e-12);
    assert_eq!(volume.dims, [4, 3, 5]);
    assert_eq!(volume.voxel, VoxelKind::U16);
    Ok(())
}

#[test]
fn sorting_is_by_projected_position_and_origin_is_the_lowest_slice()
-> Result<(), ocelli_pixel::PixelError> {
    // Delivered in descending projected order. InstanceNumber would sort
    // these the other way, because the evidence carries no instance order
    // the assembler may read.
    let mut frames = Vec::new();
    for z in [12.0, 9.0, 6.0, 3.0, 0.0] {
        frames.push(axial(z, slice_bytes())?);
    }
    let volume = Volume::assemble(candidate(frames));
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(
        (volume.origin.x, volume.origin.y, volume.origin.z),
        (0.0, 0.0, 0.0)
    );
    Ok(())
}

#[test]
fn row_spacing_multiplies_the_column_direction_cosine() -> Result<(), ocelli_pixel::PixelError> {
    // A non-square 0.5 by 1.0 pixel. PixelSpacing[0] is between rows and
    // multiplies the column direction cosine, so voxel (1, 0) steps 1.0
    // along the row cosine and voxel (0, 1) steps 0.5 along the column one.
    let mut frames = Vec::new();
    for z in [0.0, 2.0] {
        frames.push(frame(
            [10.0, 20.0, 30.0 + z],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.5, 1.0],
            3,
            4,
            slice_bytes(),
        )?);
    }
    let volume = Volume::assemble(VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap_or_else(|_| unreachable!("fixture orientation must validate")),
        frames.into_iter().map(SliceEvidence::new).collect(),
    ));
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert!(
        volume
            .spacing
            .iter()
            .zip([1.0, 0.5, 2.0])
            .all(|(held, want)| (held - want).abs() < 1e-12)
    );
    // Hand-computed patient points from the origin and direction.
    let step_x = volume.direction.col(0) * volume.spacing[0];
    let step_y = volume.direction.col(1) * volume.spacing[1];
    assert_eq!((step_x.x, step_x.y, step_x.z), (1.0, 0.0, 0.0));
    assert_eq!((step_y.x, step_y.y, step_y.z), (0.0, 0.5, 0.0));
    Ok(())
}

#[test]
fn slice_one_starts_at_byte_twenty_four() -> Result<(), ocelli_pixel::PixelError> {
    let mut frames = Vec::new();
    for z in [0.0, 3.0] {
        frames.push(axial(z, slice_bytes())?);
    }
    let volume = Volume::assemble(candidate(frames));
    assert!(volume.is_ok());
    let Ok(mut volume) = volume else {
        unreachable!("asserted immediately above")
    };
    // 4 * 3 * 2 = 24 bytes per slice, hand-computed.
    assert!(volume.place(1, &[7_u8; 24]).is_ok());
    assert_eq!(volume.data.slice(1), Some([7_u8; 24].as_slice()));
    assert_eq!(volume.data.slice(0), Some([0_u8; 24].as_slice()));
    Ok(())
}

#[test]
fn non_uniform_spacing_refuses_with_the_median_worst_gap_and_index()
-> Result<(), ocelli_pixel::PixelError> {
    let mut frames = Vec::new();
    for z in [0.0, 3.0, 6.0, 13.0, 16.0] {
        frames.push(axial(z, slice_bytes())?);
    }
    // Gaps 3, 3, 7, 3. The median is 3, the worst is 7 at index 2. A mean
    // would name a different index, and averaging would not refuse at all.
    assert_eq!(
        Volume::assemble(candidate(frames)),
        Err(VolumeError::NonUniformSpacing {
            median: 3.0,
            worst: 7.0,
            index: 2
        })
    );
    Ok(())
}

#[test]
fn duplicate_position_and_single_slice_and_shear_refuse() -> Result<(), ocelli_pixel::PixelError> {
    let mut frames = Vec::new();
    for z in [0.0, 3.0, 3.0] {
        frames.push(axial(z, slice_bytes())?);
    }
    assert!(matches!(
        Volume::assemble(candidate(frames)),
        Err(VolumeError::DuplicatePosition { .. })
    ));
    let one = vec![axial(0.0, slice_bytes())?];
    assert_eq!(
        Volume::assemble(candidate(one)),
        Err(VolumeError::TooFewSlices)
    );
    // Each slice steps 3 along the normal and 3 laterally. No tilt tag
    // exists in this fixture, so only the geometry can refuse it.
    let mut sheared = Vec::new();
    for step in [0.0, 3.0, 6.0] {
        sheared.push(frame(
            [step, 0.0, step],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            3,
            4,
            slice_bytes(),
        )?);
    }
    assert!(matches!(
        Volume::assemble(candidate(sheared)),
        Err(VolumeError::ShearedVolume { .. })
    ));
    Ok(())
}

#[test]
fn two_orientations_group_apart_and_mixed_assembly_refuses() -> Result<(), ocelli_pixel::PixelError>
{
    let axial_frame = axial(0.0, slice_bytes())?;
    let coronal = frame(
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [1.0, 1.0],
        3,
        4,
        slice_bytes(),
    )?;
    let groups = group_volumes(vec![
        (FrameOfReferenceUid("1.2.3".to_owned()), axial_frame),
        (FrameOfReferenceUid("1.2.3".to_owned()), coronal),
    ]);
    assert_eq!(groups.len(), 2);
    // Handing the assembler a candidate whose evidence disagrees with its
    // stated orientation refuses rather than mixing them.
    let mixed = VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap_or_else(|_| unreachable!("fixture orientation must validate")),
        vec![
            SliceEvidence::new(axial(0.0, slice_bytes())?),
            SliceEvidence::new(frame(
                [0.0, 0.0, 3.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
                [1.0, 1.0],
                3,
                4,
                slice_bytes(),
            )?),
        ],
    );
    assert_eq!(Volume::assemble(mixed), Err(VolumeError::NotOneVolume));
    Ok(())
}

#[test]
fn place_is_idempotent_and_a_partial_volume_reads_cleared() -> Result<(), ocelli_pixel::PixelError>
{
    let mut frames = Vec::new();
    for z in [0.0, 3.0, 6.0] {
        frames.push(axial(z, slice_bytes())?);
    }
    let volume = Volume::assemble(candidate(frames));
    assert!(volume.is_ok());
    let Ok(mut volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert!(!volume.is_complete());
    assert!(volume.place(0, &[1_u8; 24]).is_ok());
    assert!(volume.place(0, &[2_u8; 24]).is_ok());
    assert_eq!(volume.present_count(), 1);
    assert_eq!(volume.data.slice(0), Some([2_u8; 24].as_slice()));
    assert_eq!(volume.data.slice(2), Some([0_u8; 24].as_slice()));
    Ok(())
}

proptest! {
    #[test]
    fn a_randomised_uniform_series_recovers_its_generating_spacing(
        gap in 0.5_f64..5.0,
        count in 2_usize..8,
        base in -100.0_f64..100.0,
    ) {
        let mut frames = Vec::new();
        for index in 0..count {
            let z = base + gap * f64::from(u32::try_from(index).unwrap_or(0));
            let made = axial(z, slice_bytes());
            prop_assert!(made.is_ok());
            let Ok(made) = made else { return Ok(()) };
            frames.push(made);
        }
        let assembled = Volume::assemble(candidate(frames));
        prop_assert!(assembled.is_ok());
        let Ok(volume) = assembled else { return Ok(()) };
        prop_assert!((volume.spacing[2] - gap).abs() < 1e-9);
        prop_assert!((volume.origin.z - base).abs() < 1e-12);
    }
}

fn frame_bits(
    position: [f64; 3],
    allocated: u8,
    representation: PixelRepresentation,
) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let bits = StoredBits::new(allocated, allocated, allocated - 1, representation)?;
    let description = StoredPixelDescription::new(ImageDimensions::new(3, 4)?, layout, bits);
    let plane = ImagePlane::new(
        ImagePositionPatient::new(position)?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(1.0, 1.0)?,
        ImageDimensions::new(3, 4)?,
    )?;
    let bytes = usize::from(allocated / 8) * 12;
    Ok(DecodedFrame::new(description, plane, vec![0_u8; bytes]))
}

#[test]
fn an_oblique_series_sorts_by_projection_and_not_by_ipp_z() -> Result<(), ocelli_pixel::PixelError>
{
    // Row (1, 0, 0) and column (0, -0.6, -0.8) give a normal of
    // (0, 0.8, -0.6). Stepping 3 mm along it raises y and lowers z, so
    // ascending IPP z is descending projected position, the two orders
    // disagree, and only the projection can produce this origin.
    let orientation = ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, -0.6, -0.8])
        .unwrap_or_else(|_| unreachable!("fixture orientation must validate"));
    let mut frames = Vec::new();
    for k in [2.0_f64, 1.0, 0.0] {
        frames.push(frame(
            [0.0, 2.4 * k, -1.8 * k],
            [1.0, 0.0, 0.0],
            [0.0, -0.6, -0.8],
            [1.0, 1.0],
            3,
            4,
            slice_bytes(),
        )?);
    }
    let volume = Volume::assemble(VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        orientation,
        frames.into_iter().map(SliceEvidence::new).collect(),
    ));
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert!((volume.origin.y - 0.0).abs() < 1e-12 && (volume.origin.z - 0.0).abs() < 1e-12);
    assert!((volume.spacing[2] - 3.0).abs() < 1e-9);
    Ok(())
}

#[test]
fn kinds_voxel_kind_cannot_name_refuse() -> Result<(), ocelli_pixel::PixelError> {
    for (allocated, representation) in [
        (8, PixelRepresentation::Signed),
        (32, PixelRepresentation::Unsigned),
        (32, PixelRepresentation::Signed),
    ] {
        let frames = vec![
            frame_bits([0.0, 0.0, 0.0], allocated, representation)?,
            frame_bits([0.0, 0.0, 3.0], allocated, representation)?,
        ];
        assert!(matches!(
            Volume::assemble(candidate(frames)),
            Err(VolumeError::UnsupportedVoxelKind { .. })
        ));
    }
    Ok(())
}

#[test]
fn inconsistent_spacing_and_dimensions_refuse() -> Result<(), ocelli_pixel::PixelError> {
    let wide = frame(
        [0.0, 0.0, 3.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [2.0, 2.0],
        3,
        4,
        slice_bytes(),
    )?;
    let frames = vec![axial(0.0, slice_bytes())?, wide];
    assert_eq!(
        Volume::assemble(candidate(frames)),
        Err(VolumeError::InconsistentSpacing)
    );
    let small = frame(
        [0.0, 0.0, 3.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0],
        2,
        4,
        vec![0_u8; 16],
    )?;
    let frames = vec![axial(0.0, slice_bytes())?, small];
    assert_eq!(
        Volume::assemble(candidate(frames)),
        Err(VolumeError::InconsistentDimensions)
    );
    Ok(())
}

#[test]
fn place_errors_levels_order_and_cache_are_guarded() -> Result<(), ocelli_pixel::PixelError> {
    use ocelli_cache::{Budgeted, CacheTier, SeriesId};
    use ocelli_volume::{SlicePresence, VolumeCache, VolumeKey};

    let mut arrivals = Vec::new();
    for (index, z) in [6.0, 3.0, 0.0].into_iter().enumerate() {
        let mut samples = vec![0_u8; 24];
        for byte in &mut samples {
            *byte = u8::try_from(index + 1).unwrap_or(0);
        }
        arrivals.push((z, samples));
    }
    let mut frames = Vec::new();
    for (z, samples) in &arrivals {
        frames.push(axial(*z, samples.clone())?);
    }
    let volume = Volume::assemble(candidate(frames));
    assert!(volume.is_ok());
    let Ok(mut volume) = volume else {
        unreachable!("asserted immediately above")
    };
    // One level, full resolution, offset zero, per decision C1.
    assert_eq!(volume.levels.len(), 1);
    assert_eq!(
        volume.levels.first().map(|level| level.dims),
        Some([4, 3, 3])
    );
    assert_eq!(volume.levels.first().map(|level| level.offset), Some(0));
    // Arrival order was descending, so sorted order is its reverse, and
    // placing through it puts each arrival's bytes at its sorted slice.
    assert_eq!(volume.order, [2, 1, 0]);
    for (sorted, arrival) in volume.order.clone().iter().enumerate() {
        let samples = arrivals
            .get(*arrival)
            .map(|(_, samples)| samples.clone())
            .unwrap_or_default();
        assert!(
            volume
                .place(u32::try_from(sorted).unwrap_or(0), &samples)
                .is_ok()
        );
    }
    assert_eq!(
        volume.data.slice(0),
        arrivals.get(2).map(|(_, s)| s.as_slice())
    );
    assert!(volume.is_complete());
    assert_eq!(
        volume.place(9, &[0_u8; 24]),
        Err(VolumeError::SliceOutOfRange { index: 9, count: 3 })
    );
    assert_eq!(
        volume.place(0, &[0_u8; 3]),
        Err(VolumeError::SliceSizeMismatch {
            expected: 24,
            actual: 3
        })
    );
    // Budgeted bytes is the contiguous allocation: 4 * 3 * 3 * 2 = 72.
    assert_eq!(volume.bytes(), 72);
    let mut present = SlicePresence::new(130);
    present.set(64);
    present.set(129);
    present.set(130);
    assert_eq!(present.count(), 2);
    assert!(present.test(64) && present.test(129) && !present.test(65));
    let key = VolumeKey {
        series: SeriesId(7),
    };
    let mut cache = VolumeCache::new(100);
    assert!(cache.insert(key, volume).evicted.is_empty());
    assert_eq!(cache.pressure().used, 72);
    assert_eq!(cache.pressure().tier, CacheTier::Decoded);
    assert!(cache.get(&key).is_some());
    assert!(cache.remove(&key).is_some());
    assert_eq!(cache.pressure().used, 0);
    Ok(())
}

#[test]
fn a_multi_sample_slice_refuses_before_it_can_never_be_placed()
-> Result<(), ocelli_pixel::PixelError> {
    // Interleaved RGB: three samples per pixel, so one slice is 72 bytes
    // against the store's 24. Assembling it would build a volume its own
    // frames can never fill, so assembly refuses the layout itself.
    let layout = SampleLayout::new(
        3,
        PhotometricInterpretation::Rgb,
        Some(ocelli_pixel::PlanarConfiguration::Interleaved),
    )?;
    let bits = StoredBits::new(8, 8, 7, PixelRepresentation::Unsigned)?;
    let mut frames = Vec::new();
    for z in [0.0, 3.0] {
        let description = StoredPixelDescription::new(ImageDimensions::new(3, 4)?, layout, bits);
        let plane = ImagePlane::new(
            ImagePositionPatient::new([0.0, 0.0, z])?,
            ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
            PixelSpacing::new(1.0, 1.0)?,
            ImageDimensions::new(3, 4)?,
        )?;
        frames.push(DecodedFrame::new(description, plane, vec![0_u8; 36]));
    }
    assert_eq!(
        Volume::assemble(candidate(frames)),
        Err(VolumeError::InconsistentSpacing)
    );
    Ok(())
}
