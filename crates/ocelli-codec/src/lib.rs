//! Decoder registry and codec adapters, registered at runtime.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.

mod jpeg;
mod native;
mod registry;
mod rle;

pub use jpeg::{JpegDecoder, register_jpeg_decoders};
pub use native::{NativeFrameIndex, RawDecoder, register_native_and_rle_decoders};
pub use rle::RleDecoder;

pub use registry::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescError, FrameDescInput, KNOWN_TRANSFER_SYNTAXES, PixelDataVr,
    PixelRepresentation, Registry, RegistryError,
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
