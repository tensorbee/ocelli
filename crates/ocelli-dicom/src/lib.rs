//! Parsing, transfer-syntax dispatch, metadata model and providers.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-016 adds the Part 10 parser and observable transfer-syntax dispatch.
//! F-017 adds lossless metadata projection and ordered providers.
//! F-022 adds direct NIfTI-1.1 header, affine, and payload validation.
//! F-020 derives per-frame geometry, stack shear and calibrated spacing.
//! F-192 orders a bag of frames into display groups and reads cine timing.
//! F-193 reads overlay planes and display shutters.

#![doc(test(attr(forbid(unsafe_code))))]

mod dicomweb;
mod frame_geometry;
mod frame_index;
mod frame_open;
mod metadata;
mod multiframe;
mod nifti;
mod overlay;
mod parse;
mod provider;
mod series_open;

pub use dicom_core::VR;
pub use dicom_object::{DefaultDicomObject, Tag};
pub use dicomweb::{
    DicomwebSource, EncodedFramePart, EncodedFrames, SeriesSource, SourceBatch, SourceError,
    SourceResponse, SourceResponseKind,
};
pub use frame_geometry::{
    FrameGeometry, FrameGeometryError, FrameGeometrySources, SpacingEvidence, SpacingRelationship,
    StackGeometry, StackShear, parse_decimal_string,
};
pub use frame_index::{EncapsulatedFrameIndex, FrameIndexError};
pub use frame_open::{
    BurnedInFlag, ColourSelection, FrameOpenError, LossyFlag, OpenedFrame, PaddingRange,
    PrivateCreator, WindowInput, fill_padding, open_frame, read_padding,
};
pub use metadata::{
    BulkDataUri, InlineBinary, MetadataElement, MetadataError, MetadataSet, MetadataValue,
    NullSlots, PersonName,
};
pub use multiframe::{
    DuplicateSources, FunctionalGroupSource, MultiframeError, MultiframeMetadata,
    ResolvedFunctionalGroup, TopLevelFallback,
};
pub use nifti::{
    NiftiAffineSource, NiftiByteOrder, NiftiDataType, NiftiError, NiftiHeader, NiftiQForm,
    NiftiSForm, NiftiScaling, NiftiSpatialUnits, ParsedNifti, parse_nifti,
};
pub use overlay::{
    OverlayError, OverlayPlane, OverlayReason, OverlaySet, OverlayType, Shutter, ShutterShape,
    read_overlays,
};
pub use parse::{DispatchPath, ParseError, ParsedDicom, TransferSyntaxInfo, parse_part10};
pub use provider::{
    MetadataRequest, ProviderAnswer, ProviderFn, ProviderId, ProviderRegistry,
    ProviderRegistryError, data_set_provider, file_meta_provider,
};
pub use series_open::{
    CineTiming, DisplaySeries, FramePlace, ParsedFrame, Playback, SeriesOpenError,
    SeriesOpenReason, SpatialOrder, assemble_display, declared_frame_count,
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
