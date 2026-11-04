//! F-197. A tilted stack keeps its lateral step.
//!
//! Image Position Patient for slice k of the main fixture is
//! `(3k, 0, 3k)` millimetres. The row cosine is `(1, 0, 0)` and the column
//! cosine is `(0, 1, 0)`. DICOM PS3.3 C.7.6.2.1.1 takes the slice normal as
//! their cross product, `(0, 0, 1)`. The inter-slice vector is `(3, 0, 3)`.
//! Its projection on the normal is 3 mm. The lateral part is `(3, 0, 0)`,
//! length 3 mm. No gantry tilt tag exists in this fixture.

use ocelli_cache::DecodedFrame;
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    PhotometricInterpretation, PixelRepresentation, PixelSpacing, SampleLayout, StoredBits,
    StoredPixelDescription,
};
use ocelli_volume::{
    FrameOfReferenceUid, SliceEvidence, TiltedVolume, Volume, VolumeCandidate, VolumeError,
};

fn frame(position: [f64; 3], samples: Vec<u8>) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let bits = StoredBits::new(16, 16, 15, PixelRepresentation::Unsigned)?;
    let description = StoredPixelDescription::new(ImageDimensions::new(3, 4)?, layout, bits);
    let plane = ImagePlane::new(
        ImagePositionPatient::new(position)?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(1.0, 1.0)?,
        ImageDimensions::new(3, 4)?,
    )?;
    Ok(DecodedFrame::new(description, plane, samples))
}

fn candidate(frames: Vec<DecodedFrame>) -> VolumeCandidate {
    VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap_or_else(|_| unreachable!("fixture orientation must validate")),
        frames.into_iter().map(SliceEvidence::new).collect(),
    )
}

fn tilted_pair() -> Result<VolumeCandidate, ocelli_pixel::PixelError> {
    let mut frames = Vec::new();
    for step in [0.0, 3.0, 6.0] {
        frames.push(frame([step, 0.0, step], vec![9_u8; 24])?);
    }
    Ok(candidate(frames))
}

#[test]
fn the_same_three_millimetre_step_is_sheared_and_tilted() -> Result<(), ocelli_pixel::PixelError> {
    let held = tilted_pair()?;
    assert!(matches!(
        Volume::assemble(held.clone()),
        Err(VolumeError::ShearedVolume { .. })
    ));
    let tilted = TiltedVolume::assemble(held);
    assert!(tilted.is_ok());
    let Ok(tilted) = tilted else {
        unreachable!("asserted immediately above")
    };
    // 4 columns by 3 rows by 3 slices by 2 bytes. A resampled grid would
    // not have this length, and the input 9s would show if they were copied.
    assert_eq!(tilted.dims, [4, 3, 3]);
    assert_eq!(tilted.data.len(), 72);
    assert!(
        tilted
            .data
            .slice(0)
            .is_some_and(|bytes| bytes.iter().all(|byte| *byte == 0))
    );
    assert!(
        tilted
            .spacing
            .get(2)
            .is_some_and(|gap| (gap - 3.0).abs() < 1e-12)
    );
    assert!(
        tilted
            .lateral
            .iter()
            .zip([3.0_f64, 0.0, 0.0])
            .all(|(held, want)| held.to_bits() == want.to_bits())
    );
    assert!((tilted.lateral_length() - 3.0).abs() < 1e-12);
    Ok(())
}

#[test]
fn a_second_lateral_step_past_one_micrometre_is_refused() -> Result<(), ocelli_pixel::PixelError> {
    // The second lateral step is (3 + 2e-6, 0, 0). The difference from
    // (3, 0, 0) is 2e-6 mm, which is more than 1e-6 mm. An average of the
    // two steps would hide that disagreement.
    let frames = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([3.0, 0.0, 3.0], vec![0_u8; 24])?,
        frame([6.0 + 2.0e-6, 0.0, 6.0], vec![0_u8; 24])?,
    ];
    assert!(matches!(
        TiltedVolume::assemble(candidate(frames)),
        Err(VolumeError::InconsistentShear { .. })
    ));
    Ok(())
}

#[test]
fn a_gap_past_half_a_percent_is_still_refused() -> Result<(), ocelli_pixel::PixelError> {
    // Projected gaps 3, 3, 7, 3. Every lateral step is (3, 0, 0), so a
    // shear-only check would accept the stack. The median is 3 and the
    // worst gap is 7 at index 2.
    let frames = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([3.0, 0.0, 3.0], vec![0_u8; 24])?,
        frame([6.0, 0.0, 6.0], vec![0_u8; 24])?,
        frame([9.0, 0.0, 13.0], vec![0_u8; 24])?,
        frame([12.0, 0.0, 16.0], vec![0_u8; 24])?,
    ];
    assert_eq!(
        TiltedVolume::assemble(candidate(frames)),
        Err(VolumeError::NonUniformSpacing {
            median: 3.0,
            worst: 7.0,
            index: 2
        })
    );
    Ok(())
}

#[test]
fn an_orthogonal_stack_stays_on_the_orthogonal_constructor() -> Result<(), ocelli_pixel::PixelError>
{
    let mut frames = Vec::new();
    for z in [0.0, 3.0] {
        frames.push(frame([0.0, 0.0, z], vec![0_u8; 24])?);
    }
    let held = candidate(frames);
    assert!(Volume::assemble(held.clone()).is_ok());
    assert_eq!(
        TiltedVolume::assemble(held),
        Err(VolumeError::OrthogonalStack)
    );
    Ok(())
}

#[test]
fn a_one_micrometre_lateral_step_stays_orthogonal() -> Result<(), ocelli_pixel::PixelError> {
    // Length 1e-6 mm is the shared bound. `Volume::assemble` refuses only
    // a longer lateral step, and `TiltedVolume::assemble` requires a longer
    // one, so this stack belongs to exactly one constructor.
    let frames = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([1.0e-6, 0.0, 3.0], vec![0_u8; 24])?,
    ];
    let held = candidate(frames);
    assert!(Volume::assemble(held.clone()).is_ok());
    assert_eq!(
        TiltedVolume::assemble(held),
        Err(VolumeError::OrthogonalStack)
    );
    Ok(())
}

#[test]
fn a_two_micrometre_step_belongs_only_to_the_tilted_constructor()
-> Result<(), ocelli_pixel::PixelError> {
    // 2e-6 mm is above the shared bound and far below the 3 mm fixtures.
    // Loosening either constructor's comparison to 1e-3 mm accepts this
    // stack on the wrong side.
    let frames = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([2.0e-6, 0.0, 3.0], vec![0_u8; 24])?,
    ];
    let held = candidate(frames);
    assert!(matches!(
        Volume::assemble(held.clone()),
        Err(VolumeError::ShearedVolume { .. })
    ));
    let tilted = TiltedVolume::assemble(held);
    assert!(tilted.is_ok());
    let Ok(tilted) = tilted else {
        unreachable!("asserted immediately above")
    };
    assert!(
        tilted
            .lateral
            .iter()
            .zip([2.0e-6_f64, 0.0, 0.0])
            .all(|(held, want)| held.to_bits() == want.to_bits())
    );
    Ok(())
}

#[test]
fn the_longest_agreeing_step_decides_the_tilted_constructor() -> Result<(), ocelli_pixel::PixelError>
{
    // Positions (0, 0, 0), (6e-7, 0, 3) and (2.1e-6, 0, 6). The lateral
    // steps are (6e-7, 0, 0) and (2.1e-6 - 6e-7, 0, 0). They differ by
    // about 9e-7 mm, inside the 1e-6 mm agreement bound. The longer step
    // is above that bound, so Volume refuses the shear and TiltedVolume
    // stores that longer step. The decimal 1.5e-6 is one ULP away from
    // the subtraction, so the pin is the subtraction.
    let first_step = 6.0e-7_f64;
    let longer = 2.1e-6_f64 - first_step;
    let frames = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([first_step, 0.0, 3.0], vec![0_u8; 24])?,
        frame([2.1e-6, 0.0, 6.0], vec![0_u8; 24])?,
    ];
    let held = candidate(frames);
    assert!(matches!(
        Volume::assemble(held.clone()),
        Err(VolumeError::ShearedVolume { .. })
    ));
    let tilted = TiltedVolume::assemble(held);
    assert!(tilted.is_ok());
    let Ok(tilted) = tilted else {
        unreachable!("asserted immediately above")
    };
    assert!(
        tilted
            .lateral
            .iter()
            .zip([longer, 0.0, 0.0])
            .all(|(held, want)| held.to_bits() == want.to_bits())
    );
    Ok(())
}

#[test]
fn a_gap_just_over_half_a_percent_is_refused_and_one_just_under_is_kept()
-> Result<(), ocelli_pixel::PixelError> {
    // Two projected gaps. The median is their mean, so the departure is
    // half their difference. 3 mm and 3.031 mm depart by more than 0.5
    // percent. 3 mm and 3.02 mm do not. Moving 0.005 to 0.006 accepts the
    // first pair. Moving it to 0.003 refuses the second.
    let over = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([3.0, 0.0, 3.0], vec![0_u8; 24])?,
        frame([6.0, 0.0, 6.031], vec![0_u8; 24])?,
    ];
    assert!(matches!(
        TiltedVolume::assemble(candidate(over)),
        Err(VolumeError::NonUniformSpacing { .. })
    ));
    let under = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([3.0, 0.0, 3.0], vec![0_u8; 24])?,
        frame([6.0, 0.0, 6.02], vec![0_u8; 24])?,
    ];
    assert!(TiltedVolume::assemble(candidate(under)).is_ok());
    Ok(())
}

#[test]
fn duplicate_bound_stays_at_one_nanometre() -> Result<(), ocelli_pixel::PixelError> {
    // 5e-10 mm is inside 1e-9 mm. 1e-8 mm is outside it and inside 1e-6 mm,
    // so loosening the duplicate bound to a micrometre refuses the second
    // stack, and tightening it below 5e-10 mm accepts the first.
    let close = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([3.0, 0.0, 5.0e-10], vec![0_u8; 24])?,
    ];
    assert!(matches!(
        TiltedVolume::assemble(candidate(close)),
        Err(VolumeError::DuplicatePosition { .. })
    ));
    let apart = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([3.0, 0.0, 1.0e-8], vec![0_u8; 24])?,
    ];
    let tilted = TiltedVolume::assemble(candidate(apart));
    assert!(tilted.is_ok());
    let Ok(tilted) = tilted else {
        unreachable!("asserted immediately above")
    };
    assert!(
        tilted
            .lateral
            .iter()
            .zip([3.0_f64, 0.0, 0.0])
            .all(|(held, want)| held.to_bits() == want.to_bits())
    );
    Ok(())
}

#[test]
fn descending_arrival_keeps_the_sorted_lateral_step() -> Result<(), ocelli_pixel::PixelError> {
    let mut frames = Vec::new();
    for step in [6.0, 3.0, 0.0] {
        frames.push(frame([step, 0.0, step], vec![9_u8; 24])?);
    }
    let tilted = TiltedVolume::assemble(candidate(frames));
    assert!(tilted.is_ok());
    let Ok(tilted) = tilted else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(tilted.order, vec![2, 1, 0]);
    assert!(tilted.origin.x.to_bits() == 0.0_f64.to_bits());
    assert!(tilted.origin.y.to_bits() == 0.0_f64.to_bits());
    assert!(tilted.origin.z.to_bits() == 0.0_f64.to_bits());
    let axis = tilted.direction.col(0);
    assert!(axis.x.to_bits() == 1.0_f64.to_bits());
    assert!(axis.y.to_bits() == 0.0_f64.to_bits());
    assert!(axis.z.to_bits() == 0.0_f64.to_bits());
    assert!(
        tilted
            .lateral
            .iter()
            .zip([3.0_f64, 0.0, 0.0])
            .all(|(held, want)| held.to_bits() == want.to_bits())
    );
    Ok(())
}

#[test]
fn a_duplicate_projected_position_is_still_refused() -> Result<(), ocelli_pixel::PixelError> {
    let frames = vec![
        frame([0.0, 0.0, 0.0], vec![0_u8; 24])?,
        frame([5.0, 0.0, 0.0], vec![0_u8; 24])?,
    ];
    assert!(matches!(
        TiltedVolume::assemble(candidate(frames)),
        Err(VolumeError::DuplicatePosition { .. })
    ));
    Ok(())
}
