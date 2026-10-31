//! Thick-slab admission and the slab fragment module.
//!
//! Tier C returns before the image is read. An uncompiled blend returns
//! before the image is read. The sample texture is acquired by the caller
//! only after both of those, and only when the extent fits `max_tex_3d`.

use ocelli_volume::SlabImage;

use crate::caps::Tier;
use crate::graph::{BlendMode, PassKind, PipelineKey, UploadError};
use crate::voi::VOI_WGSL;

const SLAB_WGSL: &str = include_str!("../shaders/slab.wgsl");

/// Why a slab was not presented.
#[derive(Debug, thiserror::Error)]
pub enum SlabError {
    /// Tier C has no device image.
    #[error("the slab viewport is unavailable on tier C, which holds no device")]
    TierCUnavailable,
    /// The blend has no slab pipeline, or the viewport is not a slab pass.
    #[error("the slab blend has no compiled pipeline")]
    BlendUnavailable,
    /// A dimension is empty or above the device 3D texture limit.
    #[error("the slab extent is above the device 3D texture limit")]
    ExtentUnavailable,
    /// The sample upload refused the image.
    #[error(transparent)]
    Upload(#[from] UploadError),
}

/// Specialisation value for `override slab_blend`.
///
/// Composite is 0, MIP is 1, MinIP is 2 and average is 3. Labelmap edge
/// projection has no value, so it is not compiled and it is not drawn as
/// composite.
pub const fn slab_blend_code(blend: BlendMode) -> Option<u32> {
    match blend {
        BlendMode::Composite => Some(0),
        BlendMode::Mip => Some(1),
        BlendMode::MinIp => Some(2),
        BlendMode::Average => Some(3),
        BlendMode::LabelmapEdgeProjection => None,
    }
}

/// Slab pipelines `compile` builds, with the override each one receives.
pub fn slab_compile_list(tier: Tier) -> [(PipelineKey, u32); 4] {
    [
        (slab_key(tier, BlendMode::Composite), 0),
        (slab_key(tier, BlendMode::Mip), 1),
        (slab_key(tier, BlendMode::MinIp), 2),
        (slab_key(tier, BlendMode::Average), 3),
    ]
}

/// Fragment keys compiled for one tier.
///
/// Stack composite, volume composite, then the four slab modes. Volume MIP
/// and stack MIP are absent. Labelmap edge is absent.
pub fn fragment_keys_for(tier: Tier) -> [PipelineKey; 6] {
    let [composite, mip, minip, average] = slab_compile_list(tier);
    [
        PipelineKey {
            pass: PassKind::Stack,
            blend: BlendMode::Composite,
            tier,
        },
        PipelineKey {
            pass: PassKind::VolumeRaycast,
            blend: BlendMode::Composite,
            tier,
        },
        composite.0,
        mip.0,
        minip.0,
        average.0,
    ]
}

const fn slab_key(tier: Tier, blend: BlendMode) -> PipelineKey {
    PipelineKey {
        pass: PassKind::Slab,
        blend,
        tier,
    }
}

/// `VOI_WGSL` once, then the slab entry points.
///
/// The window functions are not re-typed here. The entry has no compute
/// and no storage buffer.
pub fn slab_module() -> String {
    format!("{VOI_WGSL}{SLAB_WGSL}")
}

/// Admit a slab, or refuse before `upload` runs.
///
/// Tier C returns [`SlabError::TierCUnavailable`] before `image` is read.
/// A viewport that is not a slab, or a blend with no code, returns
/// [`SlabError::BlendUnavailable`] before `image` is read. A zero dimension
/// or a dimension above `max_tex_3d` returns [`SlabError::ExtentUnavailable`]
/// and does not call `upload`.
///
/// # Errors
///
/// [`SlabError`].
pub fn present_slab(
    tier: Tier,
    viewport_is_slab: bool,
    blend: BlendMode,
    image: &SlabImage,
    max_tex_3d: u32,
    upload: impl FnOnce(&SlabImage) -> Result<(), UploadError>,
) -> Result<(), SlabError> {
    if tier == Tier::Cpu {
        return Err(SlabError::TierCUnavailable);
    }
    if !viewport_is_slab || slab_blend_code(blend).is_none() {
        return Err(SlabError::BlendUnavailable);
    }
    if extent_unavailable(image.width, image.height, image.sample_count, max_tex_3d) {
        return Err(SlabError::ExtentUnavailable);
    }
    upload(image).map_err(SlabError::Upload)
}

fn extent_unavailable(width: u32, height: u32, depth: u32, max_tex_3d: u32) -> bool {
    width == 0
        || height == 0
        || depth == 0
        || width > max_tex_3d
        || height > max_tex_3d
        || depth > max_tex_3d
}
