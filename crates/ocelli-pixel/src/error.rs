//! Refusals for malformed image-plane and pixel evidence.

/// A malformed descriptor or a caller-provided buffer of the wrong size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelError {
    /// Image Position Patient contains a non-finite component.
    InvalidImagePosition,
    /// Direction cosines are non-finite, non-unit or not mutually orthogonal.
    InvalidImageOrientation,
    /// Spacing is non-finite, negative, or zero without a matching singleton dimension.
    InvalidPixelSpacing,
    /// Rows or Columns is zero.
    InvalidDimensions,
    /// Bits Allocated is not an 8, 16 or 32-bit container.
    UnsupportedBitsAllocated,
    /// Bits Stored is zero or exceeds Bits Allocated.
    InvalidBitsStored,
    /// High Bit is not Bits Stored minus one.
    InvalidHighBit,
    /// Samples per Pixel does not agree with Photometric Interpretation.
    InvalidSamplesPerPixel,
    /// Planar Configuration is absent or present where the pixel layout forbids it.
    InvalidPlanarConfiguration,
    /// Computing a frame byte or sample count overflowed `usize`.
    LengthOverflow,
    /// The source byte slice is not exactly one described frame.
    SourceLength,
    /// The destination slice is not exactly one described frame.
    DestinationLength,
    /// LUT Descriptor entry count and LUT Data length disagree.
    InvalidLutLength,
    /// LUT Descriptor bits per entry is not 8 or 16.
    InvalidLutBits,
    /// LUT Descriptor first mapped input is outside the 16-bit DICOM range.
    InvalidLutFirstInput,
    /// LUT Data contains a non-finite value.
    NonFiniteLutValue,
    /// LUT Data contains a fractional value.
    NonIntegerLutValue,
    /// LUT Data contains a value outside the unsigned range declared by the descriptor.
    LutValueOutOfRange,
    /// A rescale path is missing either slope or intercept.
    MissingRescale,
    /// Rescale slope or intercept is non-finite.
    InvalidRescale,
    /// Window Center and Window Width have different multiplicities or are empty.
    MismatchedWindowMultiplicity,
    /// The selected window pair does not exist.
    WindowIndexOutOfRange,
    /// The selected centre is non-finite or its width violates the function rule.
    InvalidWindow,
    /// The requested display range is non-finite or descends.
    InvalidDisplayRange,
    /// The presentation stage was asked for a non-greyscale photometric interpretation.
    PresentationLutNotApplicable,
    /// A Presentation LUT Sequence is present and this build does not execute one.
    PresentationLutSequenceUnsupported,
    /// The colour stage was asked for a monochrome frame, whose route is stage 3.
    ColorTransformNotApplicable,
    /// `YBR_ICT` or `YBR_RCT` reached the colour stage untouched by its codec.
    CodecOwnedColorTransform,
    /// A `PALETTE COLOR` frame arrived without its three lookup tables.
    MissingPaletteLut,
    /// Palette lookup tables arrived for a photometric interpretation that has no index.
    UnexpectedPaletteLut,
    /// The three palette channels disagree on entry count or first mapped input.
    MismatchedPaletteDescriptors,
    /// A 4:2:2 chroma pair would be split by an odd Columns or an odd pixel count.
    SubsampledChromaAlignment,
}
