//! F-051, construct a volume and place each slice's stored samples.
//!
//! Expected values are hand-computed from DICOM PS3.3 C.7.6.2.1.1 before
//! `Volume::construct` is allowed to pass them. Spacing is the projected
//! Image Position Patient gap. PixelSpacing is row then column, so index 0
//! of `Volume::spacing` is the column step and index 1 is the row step.

use ocelli_cache::DecodedFrame;
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    PhotometricInterpretation, PixelRepresentation, PixelSpacing, SampleLayout, StoredBits,
    StoredPixelDescription,
};
use ocelli_volume::{FrameOfReferenceUid, SliceEvidence, Volume, VolumeCandidate, VolumeError};

fn frame(
    position: [f64; 3],
    row_cosine: [f64; 3],
    column_cosine: [f64; 3],
    spacing: [f64; 2],
    samples: Vec<u8>,
) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let bits = StoredBits::new(8, 8, 7, PixelRepresentation::Unsigned)?;
    let dimensions = ImageDimensions::new(2, 2)?;
    let description = StoredPixelDescription::new(dimensions, layout, bits);
    let plane = ImagePlane::new(
        ImagePositionPatient::new(position)?,
        ImageOrientationPatient::new(row_cosine, column_cosine)?,
        PixelSpacing::new(spacing[0], spacing[1])?,
        dimensions,
    )?;
    Ok(DecodedFrame::new(description, plane, samples))
}

fn candidate(
    row_cosine: [f64; 3],
    column_cosine: [f64; 3],
    frames: Vec<DecodedFrame>,
) -> VolumeCandidate {
    VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        ImageOrientationPatient::new(row_cosine, column_cosine)
            .unwrap_or_else(|_| unreachable!("fixture orientation must validate")),
        frames.into_iter().map(SliceEvidence::new).collect(),
    )
}

fn axial_stack() -> Result<VolumeCandidate, ocelli_pixel::PixelError> {
    // Arrival order is z = 6, 0, 3. Projected order is z = 0, 3, 6.
    // Row spacing 0.5 multiplies the column cosine. Column spacing 1.0
    // multiplies the row cosine. The gap is 3, not a tag of 5.
    let frames = vec![
        frame(
            [0.0, 0.0, 6.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.5, 1.0],
            vec![1, 1, 1, 1],
        )?,
        frame(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.5, 1.0],
            vec![2, 2, 2, 2],
        )?,
        frame(
            [0.0, 0.0, 3.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.5, 1.0],
            vec![3, 3, 3, 3],
        )?,
    ];
    Ok(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames))
}

#[test]
fn construct_places_samples_in_projected_order() -> Result<(), ocelli_pixel::PixelError> {
    let volume = Volume::construct(axial_stack()?);
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(volume.data.slice(0), Some([2, 2, 2, 2].as_slice()));
    assert_eq!(volume.data.slice(1), Some([3, 3, 3, 3].as_slice()));
    assert_eq!(volume.data.slice(2), Some([1, 1, 1, 1].as_slice()));
    assert_eq!(
        (volume.origin.x, volume.origin.y, volume.origin.z),
        (0.0, 0.0, 0.0)
    );
    assert!(
        volume
            .spacing
            .iter()
            .zip([1.0, 0.5, 3.0])
            .all(|(held, want)| (held - want).abs() < 1e-12)
    );
    let row = volume.direction.col(0);
    let column = volume.direction.col(1);
    let normal = volume.direction.col(2);
    assert_eq!((row.x, row.y, row.z), (1.0, 0.0, 0.0));
    assert_eq!((column.x, column.y, column.z), (0.0, 1.0, 0.0));
    assert_eq!((normal.x, normal.y, normal.z), (0.0, 0.0, 1.0));
    assert_eq!(volume.present_count(), 3);
    assert!(volume.is_complete());
    Ok(())
}

#[test]
fn construct_sorts_oblique_stack_by_projection() -> Result<(), ocelli_pixel::PixelError> {
    // Row (1, 0, 0) cross column (0, -0.6, -0.8) is (0, 0.8, -0.6).
    // A 3 mm step is (0, 2.4, -1.8). Arrival order is k = 2, 1, 0.
    // Ascending z is that same arrival order, so a z sort would place
    // [6, 6, 6, 6] at slice 0. Projection places [4, 4, 4, 4] there.
    let frames = vec![
        frame(
            [0.0, 4.8, -3.6],
            [1.0, 0.0, 0.0],
            [0.0, -0.6, -0.8],
            [1.0, 1.0],
            vec![6, 6, 6, 6],
        )?,
        frame(
            [0.0, 2.4, -1.8],
            [1.0, 0.0, 0.0],
            [0.0, -0.6, -0.8],
            [1.0, 1.0],
            vec![5, 5, 5, 5],
        )?,
        frame(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, -0.6, -0.8],
            [1.0, 1.0],
            vec![4, 4, 4, 4],
        )?,
    ];
    let volume = Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, -0.6, -0.8], frames));
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(volume.data.slice(0), Some([4, 4, 4, 4].as_slice()));
    assert_eq!(volume.data.slice(1), Some([5, 5, 5, 5].as_slice()));
    assert_eq!(volume.data.slice(2), Some([6, 6, 6, 6].as_slice()));
    assert_eq!(
        (volume.origin.x, volume.origin.y, volume.origin.z),
        (0.0, 0.0, 0.0)
    );
    assert!((volume.spacing[2] - 3.0).abs() < 1e-9);
    let normal = volume.direction.col(2);
    assert!((normal.x - 0.0).abs() < 1e-12);
    assert!((normal.y - 0.8).abs() < 1e-12);
    assert!((normal.z + 0.6).abs() < 1e-12);
    Ok(())
}

#[test]
fn construct_spacing_is_the_projected_gap() -> Result<(), ocelli_pixel::PixelError> {
    // Positions 0, 3, 6. A Spacing Between Slices tag would say 5.0, and
    // these frames carry no such tag, so a tag reader cannot pass.
    let mut frames = Vec::new();
    for z in [0.0, 3.0, 6.0] {
        frames.push(frame(
            [0.0, 0.0, z],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            vec![0, 0, 0, 0],
        )?);
    }
    let volume = Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames));
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert!((volume.spacing[2] - 3.0).abs() < 1e-12);
    Ok(())
}

#[test]
fn construct_spacing_follows_a_four_millimetre_gap() -> Result<(), ocelli_pixel::PixelError> {
    // Arrivals at z = 10, 2, 6. The projected gap is 4 mm, so a writer that
    // forces spacing[2] to 3 cannot pass. The bytes inside each slice differ,
    // so repeating the first byte cannot pass either. Row 0.7, column 1.3.
    // PS3.3 C.7.6.2.1.1.
    let frames = vec![
        frame(
            [0.0, 0.0, 10.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.7, 1.3],
            vec![9, 8, 7, 6],
        )?,
        frame(
            [0.0, 0.0, 2.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.7, 1.3],
            vec![1, 2, 3, 4],
        )?,
        frame(
            [0.0, 0.0, 6.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.7, 1.3],
            vec![5, 6, 7, 8],
        )?,
    ];
    let volume = Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames));
    assert!(volume.is_ok());
    let Ok(volume) = volume else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(volume.data.slice(0), Some([1, 2, 3, 4].as_slice()));
    assert_eq!(volume.data.slice(1), Some([5, 6, 7, 8].as_slice()));
    assert_eq!(volume.data.slice(2), Some([9, 8, 7, 6].as_slice()));
    assert_eq!(
        (volume.origin.x, volume.origin.y, volume.origin.z),
        (0.0, 0.0, 2.0)
    );
    assert!(
        volume
            .spacing
            .iter()
            .zip([1.3, 0.7, 4.0])
            .all(|(held, want)| (held - want).abs() < 1e-12)
    );
    assert_eq!(volume.levels.len(), 1);
    assert!(volume.levels.first().is_some_and(|level| {
        level
            .spacing
            .iter()
            .zip(volume.spacing)
            .all(|(level_spacing, volume_spacing)| (level_spacing - volume_spacing).abs() < 1e-12)
    }));
    Ok(())
}

#[test]
fn construct_refuses_shear() -> Result<(), ocelli_pixel::PixelError> {
    // The second position steps 3 along the normal and 5 laterally.
    // No tilt tag exists on a decoded frame, so only the geometry can refuse.
    let frames = vec![
        frame(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            vec![9, 9, 9, 9],
        )?,
        frame(
            [5.0, 0.0, 3.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            vec![8, 8, 8, 8],
        )?,
    ];
    assert!(matches!(
        Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames)),
        Err(VolumeError::ShearedVolume { .. })
    ));
    Ok(())
}

#[test]
fn construct_refuses_non_uniform_spacing() -> Result<(), ocelli_pixel::PixelError> {
    let mut frames = Vec::new();
    for z in [0.0, 3.0, 6.0, 13.0, 16.0] {
        frames.push(frame(
            [0.0, 0.0, z],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            vec![0, 0, 0, 0],
        )?);
    }
    // Gaps 3, 3, 7, 3. Median 3, worst 7, index 2. The mean is 4 and the
    // worst index stays 2, so the median field is what rejects a mean.
    assert_eq!(
        Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames)),
        Err(VolumeError::NonUniformSpacing {
            median: 3.0,
            worst: 7.0,
            index: 2,
        })
    );
    Ok(())
}

#[test]
fn construct_refuses_duplicate_and_singleton() -> Result<(), ocelli_pixel::PixelError> {
    let mut frames = Vec::new();
    for z in [0.0, 3.0, 3.0] {
        frames.push(frame(
            [0.0, 0.0, z],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            vec![0, 0, 0, 0],
        )?);
    }
    assert!(matches!(
        Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames)),
        Err(VolumeError::DuplicatePosition { .. })
    ));
    let one = vec![frame(
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0],
        vec![0, 0, 0, 0],
    )?];
    assert_eq!(
        Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], one)),
        Err(VolumeError::TooFewSlices)
    );
    Ok(())
}

#[test]
fn construct_refuses_a_short_slice() -> Result<(), ocelli_pixel::PixelError> {
    let frames = vec![
        frame(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            vec![1, 1, 1, 1],
        )?,
        frame(
            [0.0, 0.0, 3.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0],
            vec![2, 2, 2],
        )?,
    ];
    assert_eq!(
        Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames)),
        Err(VolumeError::SliceSizeMismatch {
            expected: 4,
            actual: 3,
        })
    );
    Ok(())
}

#[test]
fn assemble_stays_cleared_when_construct_fills() -> Result<(), ocelli_pixel::PixelError> {
    let assembled = Volume::assemble(axial_stack()?);
    assert!(assembled.is_ok());
    let Ok(assembled) = assembled else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(assembled.present_count(), 0);
    assert_eq!(assembled.data.slice(0), Some([0, 0, 0, 0].as_slice()));
    assert_eq!(assembled.data.slice(1), Some([0, 0, 0, 0].as_slice()));
    assert_eq!(assembled.data.slice(2), Some([0, 0, 0, 0].as_slice()));
    let built = Volume::construct(axial_stack()?);
    assert!(built.is_ok());
    let Ok(built) = built else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(built.data.slice(0), Some([2, 2, 2, 2].as_slice()));
    Ok(())
}

#[test]
fn construct_refuses_a_multi_sample_layout() -> Result<(), ocelli_pixel::PixelError> {
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
        Volume::construct(candidate([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], frames)),
        Err(VolumeError::InconsistentSpacing)
    );
    Ok(())
}
