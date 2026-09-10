//! Parsing, transfer-syntax dispatch, metadata model and providers.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-016 adds the Part 10 parser and observable transfer-syntax dispatch.
//! F-017 adds lossless metadata projection and ordered providers.

mod metadata;
mod parse;
mod provider;

pub use dicom_core::VR;
pub use dicom_object::{DefaultDicomObject, Tag};
pub use metadata::{
    BulkDataUri, InlineBinary, MetadataElement, MetadataError, MetadataSet, MetadataValue,
    PersonName,
};
pub use parse::{DispatchPath, ParseError, ParsedDicom, TransferSyntaxInfo, parse_part10};
pub use provider::{
    MetadataRequest, ProviderAnswer, ProviderFn, ProviderId, ProviderRegistry,
    ProviderRegistryError, data_set_provider, file_meta_provider,
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
