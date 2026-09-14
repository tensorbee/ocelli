//! LUT chain, photometric interpretation, frame model.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-018 keeps image-plane evidence, stored-value extraction and the modality
//! and VOI stages together here so pixel arithmetic has one implementation.
//! F-029 added the presentation stage and F-030 added the colour stage, so all
//! four of PS3.3 C.11's stages now live in this crate and nowhere else.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod color;
pub mod error;
pub mod image_plane;
pub mod lut;
pub mod stored_pixel;

pub use color::{
    ColorTransform, DecodedLayout, DecodedPhotometric, PaletteColorLut, PixelDataEncoding, Rgb,
};
pub use error::PixelError;
pub use image_plane::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, PixelSpacing,
};
pub use lut::{
    LutChain, LutDescriptor, ModalityTransform, PresentationLutEvidence, PresentationLutShape,
    PresentationTransform, VoiFunction, VoiTransform, modality,
};
pub use stored_pixel::{
    ByteOrder, PhotometricInterpretation, PixelRepresentation, PlanarConfiguration, SampleLayout,
    StoredBits, StoredPixelDescription,
};

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
