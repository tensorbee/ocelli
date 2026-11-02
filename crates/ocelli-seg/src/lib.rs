//! Segmentation state and the three representations.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-063 fills the labelmap representation. F-064 fills the contour.
//! F-065 fills the surface. F-066 names which of those three a viewport
//! shows. F-067 cuts a contour by a patient Z plane and draws the labelmap
//! edge as outline without fill. F-080 stamps a circle or a sphere into
//! the label grid. F-084 reads a segment, stores a label and reports the
//! longest runs. F-081 erases a rectangle, a circle or a sphere to
//! background. F-082 flood-fills a connected region and paint-fills every
//! copy of one value. F-083 paints a start-end span inside a rectangle or
//! a circle. F-085 paints samples inside a closed index polygon. The
//! presenter does not import wgpu.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod brush;
mod contour;
mod contour_edit;
mod fill;
mod labelmap;
mod scissors;
mod segment_tools;
mod surface;
mod threshold;

pub use brush::{Brush, BrushCoverage, BrushError, BrushShape, stamp_brush};
pub use contour::{
    Contour, ContourError, ContourGeometricType, PlaneFrame, intersect_contour, present_contour,
};
pub use contour_edit::{
    ContourEditCoverage, ContourEditError, IndexPoint, edit_labelmap_with_contour,
};
pub use fill::{FillCoverage, FillError, flood_fill, paint_fill};
pub use labelmap::{LabelColourLut, LabelGrid, LabelStyle, LabelmapError, Rgba8, present_labelmap};
pub use scissors::{ScissorCoverage, ScissorError, ScissorShape, cut_scissor};
pub use segment_tools::{
    SegmentBidirectional, SegmentLabel, SegmentRun, SegmentToolError, label_segment, segment_at,
    segment_bidirectional,
};
pub use surface::{SurfaceError, SurfaceMesh, present_surface};
pub use threshold::{
    ThresholdCoverage, ThresholdError, ThresholdRoi, ThresholdShape, threshold_roi,
};

/// Which segmentation representation a viewport shows.
///
/// This is not a colour, a grid or a mesh. A new overlay starts at
/// [`Representation::Labelmap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Representation {
    /// The label grid.
    Labelmap,
    /// A polyline in patient space.
    Contour,
    /// A triangle mesh in patient space.
    Surface,
}

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }
}
