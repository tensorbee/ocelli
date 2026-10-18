//! F-043. Chunked brick residency is the upload path for every volume.
//!
//! Decision D11. A volume that fits in one 3D texture is still uploaded as
//! bricks. There is no size threshold. The texture lives here. `ocelli-cache`
//! keeps the host brick and does not gain a wgpu dependency. Section 30's
//! frustum, level selection, indirection texture and prefetch are not this
//! module. Phase 1 uploads every ready brick and does not evict one to a
//! byte budget.

use std::collections::BTreeMap;

use ocelli_cache::{Brick, BrickKey, BrickSample};

use crate::caps::Tier;
use crate::gpu::GpuContext;

/// Why a brick was not copied into a 3D texture.
///
/// A refusal names no substitute format. The caller gets the brick back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BrickUploadError {
    /// Tier C creates no texture. The format is not consulted.
    #[error("brick upload is unavailable on tier C, which creates no texture")]
    Unavailable,
    /// The one format for this sample cannot be bound and copied, or the
    /// caller named a different format. No second format is tried.
    #[error("the brick format cannot be sampled and copied, and no other format is used")]
    FormatUnavailable,
    /// An axis is longer than the device 3D limit, or the tight row does not
    /// fit in the copy dimensions.
    #[error("a brick axis is longer than the device 3D texture limit")]
    ExtentUnavailable,
    /// An axis is zero, so there is no texture to create.
    #[error("a brick extent is empty")]
    EmptyExtent,
    /// `samples.len()` is not the extent times the sample width. Nothing is written.
    #[error("brick byte length is not the extent times the sample width, so nothing was uploaded")]
    WrongLength,
}

/// The 3D texture a brick would become. The extent is the brick's own extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrickPlan {
    extent: [u32; 3],
    format: wgpu::TextureFormat,
    bytes_per_row: u32,
    rows_per_image: u32,
}

impl BrickPlan {
    /// Texture size in texels. Not the volume size and not the nominal edge.
    pub const fn extent(self) -> [u32; 3] {
        self.extent
    }

    /// The one format [`brick_format`] allows for this sample.
    pub const fn format(self) -> wgpu::TextureFormat {
        self.format
    }

    /// Tight row stride for `write_texture`. Not the 256-byte readback pad.
    pub const fn bytes_per_row(self) -> u32 {
        self.bytes_per_row
    }

    /// Rows in one z slice of the brick.
    pub const fn rows_per_image(self) -> u32 {
        self.rows_per_image
    }
}

/// The format that stores the container bytes with no numeric conversion.
///
/// `U8` is `R8Uint`. `U16` is `R16Uint`. `I16` is `R16Sint`. `F32` is
/// `R32Float`. `Unorm` scales to 0 to 1 and is not this function's result.
/// Widening an integer sample to `R32Float` is not this function's result.
pub const fn brick_format(sample: BrickSample) -> wgpu::TextureFormat {
    match sample {
        BrickSample::U8 => wgpu::TextureFormat::R8Uint,
        BrickSample::I16 => wgpu::TextureFormat::R16Sint,
        BrickSample::U16 => wgpu::TextureFormat::R16Uint,
        BrickSample::F32 => wgpu::TextureFormat::R32Float,
    }
}

/// Decide the texture before any device call.
///
/// Tier C returns [`BrickUploadError::Unavailable`] and does not name a
/// format. An empty axis is [`BrickUploadError::EmptyExtent`]. An axis past
/// `max_texture_dimension_3d` is [`BrickUploadError::ExtentUnavailable`].
/// `format` must be [`brick_format`] for the brick's sample, and the features
/// must include `TEXTURE_BINDING` and `COPY_DST`. No second format is tried.
///
/// # Errors
///
/// See [`BrickUploadError`]. [`BrickUploadError::WrongLength`] is
/// [`brick_bytes_match`], because the plan is the texture and the length is
/// the buffer.
pub fn brick_plan(
    tier: Tier,
    max_texture_dimension_3d: u32,
    format: wgpu::TextureFormat,
    features: wgpu::TextureFormatFeatures,
    brick: &Brick,
) -> Result<BrickPlan, BrickUploadError> {
    if tier == Tier::Cpu {
        return Err(BrickUploadError::Unavailable);
    }
    if format != brick_format(brick.sample) {
        return Err(BrickUploadError::FormatUnavailable);
    }
    let [width, height, depth] = brick.extent;
    if width == 0 || height == 0 || depth == 0 {
        return Err(BrickUploadError::EmptyExtent);
    }
    if width > max_texture_dimension_3d
        || height > max_texture_dimension_3d
        || depth > max_texture_dimension_3d
    {
        return Err(BrickUploadError::ExtentUnavailable);
    }
    let Some(bytes_per_row) = row_bytes(width, brick.sample) else {
        return Err(BrickUploadError::ExtentUnavailable);
    };
    if !features.allowed_usages.contains(required_usages()) {
        return Err(BrickUploadError::FormatUnavailable);
    }
    Ok(BrickPlan {
        extent: brick.extent,
        format,
        bytes_per_row,
        rows_per_image: height,
    })
}

/// Whether `brick.samples.len()` is the voxel count times the sample width.
///
/// That product is the only accepted length. The check does not write.
///
/// # Errors
///
/// [`BrickUploadError::WrongLength`] when the buffer is a different length.
/// [`BrickUploadError::ExtentUnavailable`] when the product does not fit in
/// `usize`.
pub fn brick_bytes_match(brick: &Brick) -> Result<(), BrickUploadError> {
    let Some(expected) = expected_len(brick.extent, brick.sample) else {
        return Err(BrickUploadError::ExtentUnavailable);
    };
    if brick.samples.len() == expected {
        Ok(())
    } else {
        Err(BrickUploadError::WrongLength)
    }
}

fn required_usages() -> wgpu::TextureUsages {
    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
}

fn texture_usage(features: wgpu::TextureFormatFeatures) -> wgpu::TextureUsages {
    let mut usage = required_usages();
    if features
        .allowed_usages
        .contains(wgpu::TextureUsages::COPY_SRC)
    {
        usage |= wgpu::TextureUsages::COPY_SRC;
    }
    usage
}

fn row_bytes(width: u32, sample: BrickSample) -> Option<u32> {
    let voxel = u32::try_from(sample.bytes_per_voxel()).ok()?;
    width.checked_mul(voxel)
}

fn expected_len(extent: [u32; 3], sample: BrickSample) -> Option<usize> {
    let [width, height, depth] = extent;
    let row = usize::try_from(width)
        .ok()?
        .checked_mul(sample.bytes_per_voxel())?;
    let plane = row.checked_mul(usize::try_from(height).ok()?)?;
    plane.checked_mul(usize::try_from(depth).ok()?)
}

/// One 3D texture per [`BrickKey`]. Phase 1 does not evict.
pub(crate) struct BrickResidency {
    textures: BTreeMap<BrickKey, wgpu::Texture>,
    uploads: u64,
}

impl BrickResidency {
    pub(crate) fn new() -> Self {
        Self {
            textures: BTreeMap::new(),
            uploads: 0,
        }
    }

    pub(crate) fn uploads(&self) -> u64 {
        self.uploads
    }

    /// Drop every texture. The upload counter stays where it is.
    pub(crate) fn clear(&mut self) {
        self.textures.clear();
    }

    pub(crate) fn texture(&self, key: &BrickKey) -> Option<&wgpu::Texture> {
        self.textures.get(key)
    }

    pub(crate) fn extent_of(&self, key: &BrickKey) -> Option<[u32; 3]> {
        let size = self.textures.get(key)?.size();
        Some([size.width, size.height, size.depth_or_array_layers])
    }

    /// Copy `brick` into a new 3D texture, or give the brick back.
    ///
    /// On acceptance the texture replaces any previous one for `key`, the
    /// host brick is dropped, and `uploads` grows by one. On refusal nothing
    /// is created and the counter stays put.
    pub(crate) fn upload(
        &mut self,
        ctx: &GpuContext,
        format: wgpu::TextureFormat,
        features: wgpu::TextureFormatFeatures,
        key: BrickKey,
        brick: Brick,
    ) -> Result<(), (Brick, BrickUploadError)> {
        let caps = ctx.caps();
        let plan = match brick_plan(caps.tier, caps.max_tex_3d, format, features, &brick) {
            Ok(plan) => plan,
            Err(error) => return Err((brick, error)),
        };
        if let Err(error) = brick_bytes_match(&brick) {
            return Err((brick, error));
        }
        let [width, height, depth] = plan.extent();
        let texture = ctx.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("ocelli brick"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: depth,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: plan.format(),
            usage: texture_usage(features),
            view_formats: &[],
        });
        ctx.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &brick.samples,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(plan.bytes_per_row()),
                rows_per_image: Some(plan.rows_per_image()),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: depth,
            },
        );
        self.textures.insert(key, texture);
        self.uploads += 1;
        Ok(())
    }
}
