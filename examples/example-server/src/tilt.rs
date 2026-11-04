//! JSON for `POST /tilt`.
//!
//! The body is the token `synthetic`. The tilted stack steps 3 mm along the
//! normal and 3 mm across the row axis. Orthogonal assembly refuses it.
//! Tilted assembly keeps projected spacing 3 mm and lateral length 3 mm.
//! An orthogonal stack is still accepted by `Volume::assemble` and refused
//! by `TiltedVolume::assemble`.

use ocelli_cache::DecodedFrame;
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    PhotometricInterpretation, PixelRepresentation, PixelSpacing, SampleLayout, StoredBits,
    StoredPixelDescription,
};
use ocelli_volume::{
    FrameOfReferenceUid, SliceEvidence, TiltedVolume, Volume, VolumeCandidate, VolumeError,
};

/// Answer the tilt fixture.
#[must_use]
pub fn tilt_reply(body: &[u8]) -> (u16, String) {
    if body != b"synthetic" {
        return (400, "{\"error\":\"fixture\"}".to_string());
    }
    let Some(text) = fixture_json() else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    (200, text)
}

fn fixture_json() -> Option<String> {
    let tilted = tilted_candidate()?;
    if !matches!(
        Volume::assemble(tilted.clone()),
        Err(VolumeError::ShearedVolume { .. })
    ) {
        return None;
    }
    let volume = TiltedVolume::assemble(tilted).ok()?;
    let projected = volume.spacing.get(2).copied()?;
    let spacing = known_three(projected)?;
    let [x, y, z] = volume.lateral;
    if !exact(x, 3.0) || !exact(y, 0.0) || !exact(z, 0.0) {
        return None;
    }
    let lateral = known_three(volume.lateral_length())?;
    let orthogonal = orthogonal_candidate()?;
    if Volume::assemble(orthogonal.clone()).is_err() {
        return None;
    }
    if !matches!(
        TiltedVolume::assemble(orthogonal),
        Err(VolumeError::OrthogonalStack)
    ) {
        return None;
    }
    Some(format!(
        "{{\"orthogonal\":\"sheared\",\"spacingMm\":{spacing},\"lateralMm\":{lateral}}}"
    ))
}

fn tilted_candidate() -> Option<VolumeCandidate> {
    let mut frames = Vec::new();
    for step in [0.0, 3.0, 6.0] {
        frames.push(frame([step, 0.0, step])?);
    }
    candidate(frames)
}

fn orthogonal_candidate() -> Option<VolumeCandidate> {
    let mut frames = Vec::new();
    for z in [0.0, 3.0] {
        frames.push(frame([0.0, 0.0, z])?);
    }
    candidate(frames)
}

fn candidate(frames: Vec<DecodedFrame>) -> Option<VolumeCandidate> {
    Some(VolumeCandidate::new(
        FrameOfReferenceUid("1.2.3".to_owned()),
        orientation()?,
        frames.into_iter().map(SliceEvidence::new).collect(),
    ))
}

fn frame(position: [f64; 3]) -> Option<DecodedFrame> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None).ok()?;
    let bits = StoredBits::new(16, 16, 15, PixelRepresentation::Unsigned).ok()?;
    let dimensions = ImageDimensions::new(3, 4).ok()?;
    let description = StoredPixelDescription::new(dimensions, layout, bits);
    let plane = ImagePlane::new(
        ImagePositionPatient::new(position).ok()?,
        orientation()?,
        PixelSpacing::new(1.0, 1.0).ok()?,
        dimensions,
    )
    .ok()?;
    Some(DecodedFrame::new(description, plane, vec![0_u8; 24]))
}

fn orientation() -> Option<ImageOrientationPatient> {
    ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]).ok()
}

fn exact(value: f64, want: f64) -> bool {
    value.to_bits() == want.to_bits()
}

fn known_three(value: f64) -> Option<&'static str> {
    if exact(value, 3.0) { Some("3") } else { None }
}
