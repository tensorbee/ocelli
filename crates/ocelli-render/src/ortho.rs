//! Orthographic display is a host reslice uploaded through the stack pass.
//!
//! The plane is level 0 stored samples. Windowing stays the existing VOI
//! shader. An absent sample uploads nothing, so the last complete plane stays.

use ocelli_volume::{OrthoImage, PlaneSample};

use crate::caps::Tier;
use crate::graph::UploadError;

/// Why an orthographic plane was not uploaded.
#[derive(Debug, thiserror::Error)]
pub enum OrthoError {
    /// Tier C has no device image. The host reslice is not presented here.
    #[error("the orthographic viewport is unavailable on tier C, which holds no device")]
    TierCUnavailable,
    /// At least one sample is absent. The previous texture is left alone.
    #[error("the plane still has an absent sample")]
    PlaneIncomplete,
    /// The stack upload refused the present samples.
    #[error(transparent)]
    Upload(#[from] UploadError),
}

/// Upload a fully present plane, or refuse before the upload runs.
///
/// Tier C returns [`OrthoError::TierCUnavailable`] before the image is read.
/// A [`PlaneSample::Absent`] returns [`OrthoError::PlaneIncomplete`] before
/// `upload` is called.
///
/// # Errors
///
/// [`OrthoError`].
pub fn present_plane(
    tier: Tier,
    image: &OrthoImage,
    upload: impl FnOnce(&[f32]) -> Result<(), UploadError>,
) -> Result<(), OrthoError> {
    if tier == Tier::Cpu {
        return Err(OrthoError::TierCUnavailable);
    }
    let mut stored = Vec::new();
    for sample in &image.samples {
        match *sample {
            PlaneSample::Present(value) => stored.push(value),
            PlaneSample::Absent => return Err(OrthoError::PlaneIncomplete),
        }
    }
    upload(&stored).map_err(OrthoError::Upload)
}
