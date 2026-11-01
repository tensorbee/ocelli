//! Segmentation state and the three representations.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-063 fills the labelmap representation. F-064 fills the contour.
//! F-065 fills the surface. F-066 names which of those three a viewport
//! shows. The presenter does not import wgpu.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod contour;
mod labelmap;
mod surface;

pub use contour::{Contour, ContourError, ContourGeometricType, PlaneFrame, present_contour};
pub use labelmap::{LabelColourLut, LabelGrid, LabelStyle, LabelmapError, Rgba8, present_labelmap};
pub use surface::{SurfaceError, SurfaceMesh, present_surface};

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
