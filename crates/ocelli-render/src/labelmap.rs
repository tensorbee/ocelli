//! Composite labelmap upload.
//!
//! `ocelli-seg` chooses the bytes. This module copies them into an
//! `Rgba8Unorm` texture. The shader samples that texture and writes the
//! texel. It does not recompute the outline and it does not run the image VOI.

use ocelli_seg::LabelmapError;

use crate::gpu::GpuContext;

/// The copy shader. Compiled at graph creation, not inside a tick.
pub(crate) const LABELMAP_WGSL: &str = include_str!("../shaders/labelmap.wgsl");

/// Why [`crate::graph::RenderGraph::show_labelmap`] uploaded nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelmapDrawError {
    /// The graph is on tier C. The presenter is the CPU image.
    TierCUnavailable,
    /// The viewport is not a segmentation overlay.
    NotOverlay,
    /// The overlay blend is not composite. Labelmap edge stays uncompiled.
    BlendUnavailable,
    /// The viewport is showing a different representation. The presenter did not run.
    RepresentationMismatch,
    /// The presenter refused. The previous texture stays.
    Present(LabelmapError),
    /// The viewport id was never registered.
    UnknownViewport,
    /// Device recovery dropped the GPU objects. Show again after rebind.
    NotRebound,
    /// The presented size does not fit a padded upload.
    Extent,
}

/// One uploaded overlay. The texture stays alive with the bind group.
pub(crate) struct LabelmapBinding {
    pub(crate) bind: wgpu::BindGroup,
    texture: wgpu::Texture,
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

impl LabelmapBinding {
    /// Equal presenter bytes do not allocate a new texture.
    pub(crate) fn same_bytes(&self, width: u32, height: u32, bytes: &[u8]) -> bool {
        self.width == width
            && self.height == height
            && self.bytes == bytes
            && self.texture.width() == width
            && self.texture.height() == height
    }
}

/// Pad each row to the copy alignment, then upload.
///
/// The binding is returned only after the texture exists. The caller
/// replaces the slot after that, so a failure keeps the previous binding.
pub(crate) fn stage(
    layout: &wgpu::BindGroupLayout,
    ctx: &GpuContext,
    width: u32,
    height: u32,
    bytes: Vec<u8>,
) -> Result<LabelmapBinding, LabelmapDrawError> {
    let tight = width.checked_mul(4).ok_or(LabelmapDrawError::Extent)?;
    let row = aligned_row_bytes(tight)?;
    let row_us = usize::try_from(row).map_err(|_| LabelmapDrawError::Extent)?;
    let height_us = usize::try_from(height).map_err(|_| LabelmapDrawError::Extent)?;
    let tight_us = usize::try_from(tight).map_err(|_| LabelmapDrawError::Extent)?;
    let expected = tight_us
        .checked_mul(height_us)
        .ok_or(LabelmapDrawError::Extent)?;
    if bytes.len() != expected || height == 0 || width == 0 {
        return Err(LabelmapDrawError::Extent);
    }
    let padded_len = row_us
        .checked_mul(height_us)
        .ok_or(LabelmapDrawError::Extent)?;
    let mut padded = vec![0_u8; padded_len];
    for y in 0..height_us {
        let src = y.checked_mul(tight_us).ok_or(LabelmapDrawError::Extent)?;
        let dst = y.checked_mul(row_us).ok_or(LabelmapDrawError::Extent)?;
        let src_end = src.checked_add(tight_us).ok_or(LabelmapDrawError::Extent)?;
        let dst_end = dst.checked_add(tight_us).ok_or(LabelmapDrawError::Extent)?;
        let Some(from) = bytes.get(src..src_end) else {
            return Err(LabelmapDrawError::Extent);
        };
        let Some(into) = padded.get_mut(dst..dst_end) else {
            return Err(LabelmapDrawError::Extent);
        };
        into.copy_from_slice(from);
    }
    let device = ctx.device();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli labelmap"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    ctx.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &padded,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ocelli labelmap"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        }],
    });
    Ok(LabelmapBinding {
        bind,
        texture,
        width,
        height,
        bytes,
    })
}

fn aligned_row_bytes(unpadded: u32) -> Result<u32, LabelmapDrawError> {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let room = unpadded
        .checked_add(align - 1)
        .ok_or(LabelmapDrawError::Extent)?;
    Ok(room / align * align)
}
