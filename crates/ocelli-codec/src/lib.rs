//! Decoder registry and codec adapters, registered at runtime.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.

mod htj2k;
mod jpeg;
mod jpeg2000;
mod jpegls;
mod native;
mod registry;
mod rle;
mod sample_convert;

pub use htj2k::{Htj2kDecoder, register_htj2k_decoders};
pub use jpeg::{JpegDecoder, register_jpeg_decoders};
pub use jpeg2000::{Jpeg2000Decoder, register_jpeg2000_decoders};
pub use jpegls::{JpegLsDecoder, register_jpegls_decoders};
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
