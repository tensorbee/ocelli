//! Contour upload errors.
//!
//! The bytes come from `ocelli-seg`. The overlay texture is the same binding
//! the labelmap uses. This module does not stroke.

use ocelli_seg::ContourError;

use crate::labelmap::LabelmapDrawError;

/// Pixel size of one contour upload. One argument, so `show_contour` stays
/// inside the argument limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContourView {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
}

/// Why [`crate::graph::RenderGraph::show_contour`] uploaded nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourDrawError {
    /// The graph is on tier C. The presenter is the CPU image.
    TierCUnavailable,
    /// The viewport is not a segmentation overlay.
    NotOverlay,
    /// The overlay blend is not composite.
    BlendUnavailable,
    /// The viewport is showing a different representation. The presenter did not run.
    RepresentationMismatch,
    /// The presenter refused. The previous texture stays.
    Present(ContourError),
    /// The viewport id was never registered.
    UnknownViewport,
    /// Device recovery dropped the GPU objects. Show again after rebind.
    NotRebound,
    /// The presented size does not fit a padded upload.
    Extent,
}

impl ContourDrawError {
    pub(crate) fn from_labelmap(err: LabelmapDrawError) -> Self {
        match err {
            LabelmapDrawError::NotRebound => Self::NotRebound,
            LabelmapDrawError::Extent => Self::Extent,
            LabelmapDrawError::TierCUnavailable => Self::TierCUnavailable,
            LabelmapDrawError::NotOverlay => Self::NotOverlay,
            LabelmapDrawError::BlendUnavailable => Self::BlendUnavailable,
            LabelmapDrawError::RepresentationMismatch => Self::RepresentationMismatch,
            LabelmapDrawError::UnknownViewport => Self::UnknownViewport,
            LabelmapDrawError::Present(_) => Self::Extent,
        }
    }
}
