//! Decoder registry and codec adapters, registered at runtime.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.

mod jpeg;
mod registry;

pub use jpeg::{JpegDecoder, register_jpeg_decoders};

pub use registry::{
    Capability, CodecError, DecodePhotometricInterpretation, Decoder, FrameDesc, FrameDescError,
    FrameDescInput, KNOWN_TRANSFER_SYNTAXES, PixelRepresentation, Registry, RegistryError,
};

/// The crate's own name.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    #[test]
    fn crate_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }
}
