//! Surface upload errors.
//!
//! The bytes come from `ocelli-seg`. The overlay texture is the same binding
//! the labelmap uses. This module does not rasterise a triangle.

use ocelli_seg::SurfaceError;

use crate::labelmap::LabelmapDrawError;

/// Pixel size of one surface upload. One argument, so `show_surface` stays
/// inside the argument limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceView {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
}

/// Why [`crate::graph::RenderGraph::show_surface`] uploaded nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceDrawError {
    /// The graph is on tier C. The presenter is the CPU image.
    TierCUnavailable,
    /// The viewport is not a segmentation overlay.
    NotOverlay,
    /// The overlay blend is not composite.
    BlendUnavailable,
    /// The presenter refused. The previous texture stays.
    Present(SurfaceError),
    /// The viewport id was never registered.
    UnknownViewport,
    /// Device recovery dropped the GPU objects. Show again after rebind.
    NotRebound,
    /// The presented size does not fit a padded upload.
    Extent,
}

impl SurfaceDrawError {
    pub(crate) fn from_labelmap(err: LabelmapDrawError) -> Self {
        match err {
            LabelmapDrawError::NotRebound => Self::NotRebound,
            LabelmapDrawError::Extent => Self::Extent,
            LabelmapDrawError::TierCUnavailable => Self::TierCUnavailable,
            LabelmapDrawError::NotOverlay => Self::NotOverlay,
            LabelmapDrawError::BlendUnavailable => Self::BlendUnavailable,
            LabelmapDrawError::UnknownViewport => Self::UnknownViewport,
            LabelmapDrawError::Present(_) => Self::Extent,
        }
    }
}
