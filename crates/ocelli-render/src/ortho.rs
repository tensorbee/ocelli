//! Orthographic display is a host reslice.
//!
//! Tier A and tier B upload that plane through the stack pass.
//! Tier C presents the same samples through [`present_cpu_plane`] and
//! [`crate::cpu::CpuRaster`]. [`present_plane`] still returns
//! [`OrthoError::TierCUnavailable`] before it reads the image. An absent
//! sample publishes nothing, so the last complete frame stays.

use ocelli_core::Stored;
use ocelli_viewport::presentation::Presentation;
use ocelli_viewport::view::View;
use ocelli_volume::{OrthoImage, PlaneSample};

use crate::caps::Tier;
use crate::cpu::{CpuRaster, RasterError, RasterGrid, RasterQuality, RasterResult};
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
    /// The CPU raster refused the frame. The output slice is left untouched.
    #[error(transparent)]
    Raster(#[from] RasterError),
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

/// Copy one host plane into stored samples.
///
/// Each [`PlaneSample::Present`] value is copied as [`Stored`] with the same
/// `f32` bits. This function does not sample the volume. An absent sample,
/// an empty grid, or a length that is not `width * height` returns before
/// any slot is written.
///
/// # Errors
///
/// [`OrthoError::PlaneIncomplete`] when any sample is absent.
/// [`OrthoError::Raster`] when the grid or the buffer length is refused.
pub fn copy_plane_stored(image: &OrthoImage, stored: &mut [Stored]) -> Result<(), OrthoError> {
    let source = [image.width, image.height];
    CpuRaster::accept_mono_source(stored, source)?;
    let mut pending = Vec::with_capacity(stored.len());
    for sample in &image.samples {
        match *sample {
            PlaneSample::Present(value) => pending.push(Stored(value)),
            PlaneSample::Absent => return Err(OrthoError::PlaneIncomplete),
        }
    }
    if pending.len() != stored.len() {
        return Err(OrthoError::Raster(RasterError::Length));
    }
    stored.copy_from_slice(&pending);
    Ok(())
}

/// Present one host plane through the CPU raster.
///
/// The samples are the plane's existing values. This function does not call
/// `Volume::reslice`. Tier A and tier B stay on [`present_plane`]. The target
/// is the raster's device extent. A refusal writes nothing to `out`.
///
/// # Errors
///
/// [`OrthoError::PlaneIncomplete`] when any sample is absent.
/// [`OrthoError::Raster`] when the raster refuses the frame.
pub fn present_cpu_plane(
    raster: &mut CpuRaster,
    view: &View,
    presentation: &Presentation,
    image: &OrthoImage,
    quality: RasterQuality,
    out: &mut [u8],
) -> Result<RasterResult, OrthoError> {
    let mut stored = vec![Stored(0.0); image.samples.len()];
    copy_plane_stored(image, &mut stored)?;
    let source = [image.width, image.height];
    CpuRaster::accept_mono_source(&stored, source)?;
    raster
        .raster_into(
            view,
            presentation,
            &stored,
            RasterGrid {
                source,
                target: raster.device(),
            },
            quality,
            out,
        )
        .map_err(OrthoError::Raster)
}
