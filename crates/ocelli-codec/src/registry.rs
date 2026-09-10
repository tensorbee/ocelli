//! Explicit runtime codec registration and exact transfer-syntax dispatch.
//!
//! HLD section 21 requires an explicit registry because inventory-based
//! registration is unavailable on WebAssembly. Registration owns setup-time
//! allocation. Capability lookup and decode dispatch only borrow existing
//! state, and decoding writes into the caller-provided output slice.

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    sync::Arc,
};

/// Transfer Syntax UIDs covered by the repository's codec surface.
///
/// These sixteen UIDs are defined by DICOM PS3.6 Annex A and enumerated by the
/// corpus manifest. Membership means the syntax is known. It does not mean a
/// decoder is registered or that this build can decode it.
pub const KNOWN_TRANSFER_SYNTAXES: &[&str] = &[
    "1.2.840.10008.1.2",
    "1.2.840.10008.1.2.1",
    "1.2.840.10008.1.2.1.99",
    "1.2.840.10008.1.2.2",
    "1.2.840.10008.1.2.5",
    "1.2.840.10008.1.2.4.50",
    "1.2.840.10008.1.2.4.51",
    "1.2.840.10008.1.2.4.57",
    "1.2.840.10008.1.2.4.70",
    "1.2.840.10008.1.2.4.80",
    "1.2.840.10008.1.2.4.81",
    "1.2.840.10008.1.2.4.90",
    "1.2.840.10008.1.2.4.91",
    "1.2.840.10008.1.2.4.201",
    "1.2.840.10008.1.2.4.202",
    "1.2.840.10008.1.2.4.203",
];

/// Whether a transfer syntax is usable by this registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// The UID is known and has a registered decoder.
    Available,
    /// The UID is known but no decoder is registered in this build.
    KnownUnavailable,
    /// The UID is outside this registry's explicit catalogue.
    Unknown,
}

/// The signedness of stored pixel samples from DICOM `(0028,0103)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelRepresentation {
    /// Stored samples are unsigned.
    Unsigned,
    /// Stored samples use two's-complement signed representation.
    Signed,
}

/// Photometric Interpretation presented by a decoder's output buffer.
///
/// Most decoders preserve the interpretation in [`FrameDesc`]. JPEG colour
/// decoders commonly convert encoded YCbCr samples to packed RGB. Keeping that
/// distinction typed reports the conversion for downstream consumers. The
/// query is separate from decode, so consumers remain responsible for using it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodePhotometricInterpretation {
    /// The output retains [`FrameDesc::photometric_interpretation`].
    Preserved,
    /// The output is packed RGB, regardless of the encapsulating DICOM value.
    Rgb,
}

/// Unvalidated fields used to construct a [`FrameDesc`].
///
/// Naming every DICOM pixel field prevents positional arguments with the same
/// integer type from being silently swapped at a call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDescInput {
    /// Frame height from DICOM Rows `(0028,0010)`, whose VR is US.
    pub rows: u16,
    /// Frame width from DICOM Columns `(0028,0011)`, whose VR is US.
    pub columns: u16,
    /// Number of samples stored for each pixel.
    pub samples_per_pixel: u16,
    /// Width of each sample container.
    pub bits_allocated: u16,
    /// Number of meaningful stored bits.
    pub bits_stored: u16,
    /// Position of the most significant stored bit.
    pub high_bit: u16,
    /// Whether stored samples are signed or unsigned.
    pub pixel_representation: PixelRepresentation,
    /// The DICOM Photometric Interpretation value.
    pub photometric_interpretation: String,
}

/// Validated descriptive input for one decoded frame.
///
/// This type performs size and bit-layout validation only. It does not unpack,
/// transform, or otherwise perform pixel arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDesc {
    rows: u16,
    columns: u16,
    samples_per_pixel: u16,
    bits_allocated: u16,
    bits_stored: u16,
    high_bit: u16,
    pixel_representation: PixelRepresentation,
    photometric_interpretation: String,
    output_len: usize,
}

impl FrameDesc {
    /// Validate the DICOM pixel container description and compute its output
    /// byte length.
    ///
    /// # Errors
    ///
    /// Returns [`FrameDescError`] when a dimension is zero, the sample
    /// container width is unsupported, Bits Stored does not fit its container,
    /// High Bit is not one less than Bits Stored, or the output byte length
    /// cannot be represented on this target.
    pub fn new(input: FrameDescInput) -> Result<Self, FrameDescError> {
        let FrameDescInput {
            rows,
            columns,
            samples_per_pixel,
            bits_allocated,
            bits_stored,
            high_bit,
            pixel_representation,
            photometric_interpretation,
        } = input;
        if rows == 0 {
            return Err(FrameDescError::ZeroRows);
        }
        if columns == 0 {
            return Err(FrameDescError::ZeroColumns);
        }
        if samples_per_pixel == 0 {
            return Err(FrameDescError::ZeroSamplesPerPixel);
        }
        if !matches!(bits_allocated, 8 | 16 | 32) {
            return Err(FrameDescError::UnsupportedBitsAllocated {
                bits: bits_allocated,
            });
        }
        if bits_stored == 0 || bits_stored > bits_allocated {
            return Err(FrameDescError::BitsStoredOutOfRange {
                bits_stored,
                bits_allocated,
            });
        }
        let expected_high_bit = bits_stored - 1;
        if high_bit != expected_high_bit {
            return Err(FrameDescError::HighBitMismatch {
                high_bit,
                expected: expected_high_bit,
            });
        }

        let output_len =
            checked_output_len::<usize>(rows, columns, samples_per_pixel, bits_allocated / 8)
                .ok_or(FrameDescError::OutputLengthOverflow)?;

        Ok(Self {
            rows,
            columns,
            samples_per_pixel,
            bits_allocated,
            bits_stored,
            high_bit,
            pixel_representation,
            photometric_interpretation,
            output_len,
        })
    }

    /// Frame height from DICOM Rows `(0028,0010)`, whose VR is US.
    #[must_use]
    pub const fn rows(&self) -> u16 {
        self.rows
    }

    /// Frame width from DICOM Columns `(0028,0011)`, whose VR is US.
    #[must_use]
    pub const fn columns(&self) -> u16 {
        self.columns
    }

    /// Number of samples stored for each pixel.
    #[must_use]
    pub const fn samples_per_pixel(&self) -> u16 {
        self.samples_per_pixel
    }

    /// Width of each stored sample container.
    #[must_use]
    pub const fn bits_allocated(&self) -> u16 {
        self.bits_allocated
    }

    /// Number of meaningful bits in each sample container.
    #[must_use]
    pub const fn bits_stored(&self) -> u16 {
        self.bits_stored
    }

    /// Position of the most significant stored bit.
    #[must_use]
    pub const fn high_bit(&self) -> u16 {
        self.high_bit
    }

    /// Whether samples are signed or unsigned.
    #[must_use]
    pub const fn pixel_representation(&self) -> PixelRepresentation {
        self.pixel_representation
    }

    /// The DICOM Photometric Interpretation value.
    #[must_use]
    pub fn photometric_interpretation(&self) -> &str {
        &self.photometric_interpretation
    }

    /// Required caller-provided output length in bytes.
    #[must_use]
    pub const fn output_len(&self) -> usize {
        self.output_len
    }
}

/// Why a [`FrameDesc`] could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameDescError {
    /// Rows was zero.
    ZeroRows,
    /// Columns was zero.
    ZeroColumns,
    /// Samples per pixel was zero.
    ZeroSamplesPerPixel,
    /// Bits Allocated was not 8, 16, or 32.
    UnsupportedBitsAllocated { bits: u16 },
    /// Bits Stored was zero or exceeded Bits Allocated.
    BitsStoredOutOfRange {
        bits_stored: u16,
        bits_allocated: u16,
    },
    /// High Bit was not one less than Bits Stored.
    HighBitMismatch { high_bit: u16, expected: u16 },
    /// The decoded frame byte length cannot be represented on this target.
    OutputLengthOverflow,
}

impl fmt::Display for FrameDescError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroRows => "frame rows must be nonzero",
            Self::ZeroColumns => "frame columns must be nonzero",
            Self::ZeroSamplesPerPixel => "frame samples per pixel must be nonzero",
            Self::UnsupportedBitsAllocated { .. } => "frame bits allocated must be 8, 16, or 32",
            Self::BitsStoredOutOfRange { .. } => {
                "frame bits stored must fit the allocated sample container"
            }
            Self::HighBitMismatch { .. } => "frame high bit must be one less than bits stored",
            Self::OutputLengthOverflow => "decoded frame byte length overflows this target",
        })
    }
}

impl Error for FrameDescError {}

fn checked_output_len<T>(
    rows: u16,
    columns: u16,
    samples_per_pixel: u16,
    bytes_per_sample: u16,
) -> Option<T>
where
    T: TryFrom<u64>,
{
    let length = u64::from(rows)
        .checked_mul(u64::from(columns))?
        .checked_mul(u64::from(samples_per_pixel))?
        .checked_mul(u64::from(bytes_per_sample))?;
    T::try_from(length).ok()
}

/// A decode dispatch or decoder failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodecError {
    /// The requested UID is outside the explicit known catalogue.
    UnknownTransferSyntax,
    /// The UID is known but no decoder is registered in this build.
    KnownUnavailable,
    /// The caller-provided output slice has the wrong byte length.
    OutputLength { expected: usize, actual: usize },
    /// The encoded frame is structurally invalid or truncated.
    InvalidCodestream,
    /// Bytes follow the first complete encoded image.
    TrailingData,
    /// Encoded dimensions, components, precision, or process differ from the descriptor.
    FrameMismatch,
    /// The dependency produced an output layout this adapter cannot represent.
    UnsupportedPixelFormat,
    /// A concrete decoder failed.
    DecoderFailure,
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnknownTransferSyntax => "unknown DICOM transfer syntax",
            Self::KnownUnavailable => "known DICOM transfer syntax has no registered decoder",
            Self::OutputLength { .. } => "caller-provided decode output has the wrong length",
            Self::InvalidCodestream => "encoded frame is invalid or truncated",
            Self::TrailingData => "encoded frame has trailing data",
            Self::FrameMismatch => "encoded frame does not match its DICOM description",
            Self::UnsupportedPixelFormat => "decoder output pixel format is unsupported",
            Self::DecoderFailure => "registered DICOM decoder failed",
        })
    }
}

impl Error for CodecError {}

/// A decoder for one or more exact DICOM Transfer Syntax UIDs.
pub trait Decoder: Send + Sync {
    /// The complete static set of UIDs this decoder accepts.
    fn transfer_syntaxes(&self) -> &'static [&'static str];

    /// Describe the Photometric Interpretation of decoded output.
    ///
    /// The default preserves the DICOM frame description. Concrete colour
    /// decoders override this when they perform a colour transform.
    fn decode_photometric_interpretation(
        &self,
        _desc: &FrameDesc,
    ) -> DecodePhotometricInterpretation {
        DecodePhotometricInterpretation::Preserved
    }

    /// Decode one frame atomically into `out`.
    ///
    /// Registry lookup and dispatch add no allocation of their own. Raw plus
    /// RLE implementations remain allocation-free per call. Under deviation
    /// D-21, concrete JPEG and JPEG 2000 adapters may allocate bounded
    /// dependency-owned decoded storage and may copy encoded input when a safe
    /// packet API requires owned bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] when the encoded frame cannot be decoded.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError>;
}

/// Why a decoder could not be registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryError {
    /// The decoder declared no transfer syntax.
    NoTransferSyntaxes,
    /// A decoder or catalogue entry was the empty string.
    EmptyTransferSyntax,
    /// A UID occurred more than once in one declaration.
    RepeatedTransferSyntax { uid: &'static str },
    /// The decoder claimed a UID outside the registry's known catalogue.
    UnknownTransferSyntax { uid: &'static str },
    /// Another decoder is already registered for the UID.
    AlreadyRegistered { uid: &'static str },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoTransferSyntaxes => "decoder declares no transfer syntaxes",
            Self::EmptyTransferSyntax => "transfer syntax UID must not be empty",
            Self::RepeatedTransferSyntax { .. } => {
                "decoder repeats a transfer syntax UID in its declaration"
            }
            Self::UnknownTransferSyntax { .. } => {
                "decoder declares a transfer syntax outside the known catalogue"
            }
            Self::AlreadyRegistered { .. } => {
                "a decoder is already registered for the transfer syntax"
            }
        })
    }
}

impl Error for RegistryError {}

/// Explicit transfer-syntax capability and decoder registry.
pub struct Registry {
    known: HashSet<&'static str>,
    by_ts: HashMap<&'static str, Arc<dyn Decoder>>,
}

impl Registry {
    /// Create the production registry with every repository-known syntax
    /// unavailable until a concrete decoder is explicitly registered.
    #[must_use]
    pub fn new() -> Self {
        Self {
            known: KNOWN_TRANSFER_SYNTAXES.iter().copied().collect(),
            by_ts: HashMap::new(),
        }
    }

    /// Create a registry with an explicit known UID catalogue.
    ///
    /// This constructor is useful for build-specific registries and narrow
    /// tests. It registers no decoder.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if an entry is empty or repeated.
    pub fn with_known(known: &'static [&'static str]) -> Result<Self, RegistryError> {
        let mut known_set = HashSet::with_capacity(known.len());
        for uid in known {
            if uid.is_empty() {
                return Err(RegistryError::EmptyTransferSyntax);
            }
            if !known_set.insert(*uid) {
                return Err(RegistryError::RepeatedTransferSyntax { uid });
            }
        }
        Ok(Self {
            known: known_set,
            by_ts: HashMap::new(),
        })
    }

    /// Report exact-UID capability without assuming a common syntax.
    #[must_use]
    pub fn capability(&self, transfer_syntax: &str) -> Capability {
        if self.by_ts.contains_key(transfer_syntax) {
            Capability::Available
        } else if self.known.contains(transfer_syntax) {
            Capability::KnownUnavailable
        } else {
            Capability::Unknown
        }
    }

    /// Register all UIDs declared by one decoder as an atomic operation.
    ///
    /// Deviation D-19 replaces HLD section 21's silent `HashMap::insert`
    /// overwrite. The complete declaration is validated before the map is
    /// changed, so a later collision cannot leave a decoder partly registered.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] for an empty declaration, an empty or
    /// repeated UID, an unknown UID, or a UID already owned by another
    /// decoder. The registry is unchanged on every error.
    pub fn register(&mut self, decoder: Arc<dyn Decoder>) -> Result<(), RegistryError> {
        let syntaxes = decoder.transfer_syntaxes();
        if syntaxes.is_empty() {
            return Err(RegistryError::NoTransferSyntaxes);
        }

        let mut declared = HashSet::with_capacity(syntaxes.len());
        for uid in syntaxes {
            if uid.is_empty() {
                return Err(RegistryError::EmptyTransferSyntax);
            }
            if !declared.insert(*uid) {
                return Err(RegistryError::RepeatedTransferSyntax { uid });
            }
            if !self.known.contains(uid) {
                return Err(RegistryError::UnknownTransferSyntax { uid });
            }
            if self.by_ts.contains_key(uid) {
                return Err(RegistryError::AlreadyRegistered { uid });
            }
        }

        for uid in syntaxes {
            self.by_ts.insert(*uid, Arc::clone(&decoder));
        }
        Ok(())
    }

    /// Look up the exact decoder for a transfer syntax.
    ///
    /// # Errors
    ///
    /// Returns distinct errors for an unknown syntax and a known syntax with
    /// no decoder in this build.
    pub fn decoder(&self, transfer_syntax: &str) -> Result<&dyn Decoder, CodecError> {
        if let Some(decoder) = self.by_ts.get(transfer_syntax) {
            return Ok(decoder.as_ref());
        }
        if self.known.contains(transfer_syntax) {
            Err(CodecError::KnownUnavailable)
        } else {
            Err(CodecError::UnknownTransferSyntax)
        }
    }

    /// Describe the Photometric Interpretation produced for one frame.
    ///
    /// # Errors
    ///
    /// Returns the same exact-UID capability errors as [`Self::decoder`].
    pub fn decode_photometric_interpretation(
        &self,
        transfer_syntax: &str,
        desc: &FrameDesc,
    ) -> Result<DecodePhotometricInterpretation, CodecError> {
        Ok(self
            .decoder(transfer_syntax)?
            .decode_photometric_interpretation(desc))
    }

    /// Decode one frame through the decoder registered for the exact UID.
    ///
    /// # Errors
    ///
    /// Returns distinct capability failures, rejects a caller buffer whose
    /// length differs from [`FrameDesc::output_len`], and otherwise propagates
    /// the selected decoder's error unchanged.
    pub fn decode(
        &self,
        transfer_syntax: &str,
        src: &[u8],
        desc: &FrameDesc,
        out: &mut [u8],
    ) -> Result<(), CodecError> {
        let decoder = self.decoder(transfer_syntax)?;
        if out.len() != desc.output_len() {
            return Err(CodecError::OutputLength {
                expected: desc.output_len(),
                actual: out.len(),
            });
        }
        decoder.decode(src, desc, out)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::checked_output_len;

    #[test]
    fn conforming_dicom_dimensions_can_overflow_a_32_bit_output_length() {
        assert_eq!(checked_output_len::<u32>(u16::MAX, u16::MAX, 1, 4), None);
        assert_eq!(
            checked_output_len::<u32>(u16::MAX, u16::MAX, 1, 1),
            Some(4_294_836_225)
        );
    }
}
