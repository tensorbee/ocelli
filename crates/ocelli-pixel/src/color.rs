//! Colour interpretation, DICOM PS3.3 C.7.6.3.1.2, C.7.6.3.1.3 and C.7.9.
//!
//! This is stage 4 of PS3.3 C.11's chain, and **deviation D-23 records that it
//! is not the `Display -> RGB` stage HLD section 18's table names**. Palette
//! colour maps the stored value, per C.7.6.3.1.5's "the first stored pixel
//! value mapped". The `RGB` and `YBR_*` routes map decoded samples that never
//! entered the chain at all.
//!
//! Stage 3 and stage 4 partition the photometric interpretations exactly.
//! [`crate::PresentationTransform::new`] refuses every colour space with
//! [`PixelError::PresentationLutNotApplicable`], and
//! [`ColorTransform::resolve`] refuses both monochrome ones with
//! [`PixelError::ColorTransformNotApplicable`]. A frame therefore reaches
//! exactly one of the two, never both and never neither.

use ocelli_core::Stored;

use crate::{
    PixelError,
    lut::LutDescriptor,
    stored_pixel::{PhotometricInterpretation, PlanarConfiguration, SampleLayout},
};

/// One colour sample triple, the output of PS3.3 C.11's stage 4.
///
/// `f32` channels to match the three scalar value spaces in
/// `ocelli_core::value`, and because rounding to an 8-bit channel is a
/// decision HLD 27.3 makes a human review item. It belongs at the render or
/// export boundary rather than inside the arithmetic, so this stage does not
/// make it and does not clamp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
}

impl Rgb {
    /// All three channels zero, for pre-filling caller-provided storage.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
}

/// What a decoder reports about the colour space of its own output.
///
/// Mirrors `ocelli_codec::DecodePhotometricInterpretation`. The two crates do
/// not depend on each other: `ocelli-pixel` carries portable arithmetic and
/// `ocelli-codec` carries five codec libraries, so a dependency either way
/// would be the wrong shape. The mapping between them is owed by whichever
/// story first wires a decoder's output into this stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodedPhotometric {
    /// The output retains the data set's Photometric Interpretation.
    Preserved,
    /// The output is RGB, whatever the data set's Photometric Interpretation says.
    Rgb,
}

/// What a decoder reports about the sample ordering of its own output.
///
/// Mirrors `ocelli_codec::DecodeSampleLayout`, for the reason above.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodedLayout {
    /// The decoder preserves the layout the data set describes.
    Preserved,
    /// Samples for each pixel are adjacent in the decoder's output.
    Interleaved,
}

/// PS3.5 section 8.2, whether Pixel Data arrived native or encapsulated.
///
/// A named enum rather than a `bool`, because the call site otherwise reads as
/// a positional flag beside two other two-state arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelDataEncoding {
    /// Pixel Data is an uncompressed native data-set element.
    Native,
    /// Pixel Data is encapsulated, so a codec defined the output layout.
    Encapsulated,
}

/// The three Palette Color Lookup Tables, PS3.3 C.7.9.
///
/// Each channel is a [`LutDescriptor`], which already validates every one of
/// PS3.3 C.7.6.3.1.5's three descriptor values: the zero-means-65,536 entry
/// count, the first mapped input, and an 8 or 16-bit entry size with its value
/// range. Its lookup already clamps below the first mapped input and above the
/// last entry, which is C.7.6.3.1.5's stated clamping.
///
/// **No lookup arithmetic is written here.** HLD section 18 requires that
/// arithmetic to exist exactly once, and this is the one place in stage 4
/// where a second copy was available for free.
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteColorLut {
    red: LutDescriptor,
    green: LutDescriptor,
    blue: LutDescriptor,
}

impl PaletteColorLut {
    /// Validate that the three channels describe the same input mapping.
    ///
    /// # Errors
    ///
    /// [`PixelError::MismatchedPaletteDescriptors`] when the three channels do
    /// not agree on entry count and first mapped input. A palette whose
    /// channels mapped different input ranges would shift hue against
    /// luminance, which no shape check on the result reveals.
    pub fn new(
        red: LutDescriptor,
        green: LutDescriptor,
        blue: LutDescriptor,
    ) -> Result<Self, PixelError> {
        let key =
            |descriptor: &LutDescriptor| (descriptor.entry_count(), descriptor.first_input_bits());
        if key(&red) != key(&green) || key(&red) != key(&blue) {
            return Err(PixelError::MismatchedPaletteDescriptors);
        }
        Ok(Self { red, green, blue })
    }

    fn lookup(&self, stored: Stored) -> Rgb {
        Rgb {
            r: self.red.lookup(stored.0),
            g: self.green.lookup(stored.0),
            b: self.blue.lookup(stored.0),
        }
    }
}

/// The colour space a frame's samples are actually in, after decoder evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorSpace {
    Palette,
    Rgb,
    YbrFull,
    YbrFull422,
    YbrPartial422,
}

impl ColorSpace {
    /// Stored samples on the wire for each pixel.
    ///
    /// Two for a 4:2:2 space, because PS3.3 C.7.6.3.1.2 subsamples the chroma
    /// two to one horizontally and stores each pair of pixels as `Y1 Y2 Cb Cr`.
    /// A frame is `Rows * Columns * 2` and not `* 3`.
    const fn stored_samples_per_pixel(self) -> usize {
        match self {
            Self::Palette => 1,
            Self::Rgb | Self::YbrFull => 3,
            Self::YbrFull422 | Self::YbrPartial422 => 2,
        }
    }

    const fn is_subsampled(self) -> bool {
        matches!(self, Self::YbrFull422 | Self::YbrPartial422)
    }
}

/// DICOM PS3.3 C.11 stage 4, resolved once against the decoder's own evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorTransform {
    space: ColorSpace,
    /// Only consulted for a three-sample non-subsampled space.
    planar: PlanarConfiguration,
    palette: Option<PaletteColorLut>,
}

impl ColorTransform {
    /// Resolve the colour space and sample layout, each from one rule.
    ///
    /// **The colour transform is decided here and nowhere else**, which is what
    /// stops it from being applied twice. A JPEG decoder usually outputs RGB
    /// even though the data set still says `YBR_FULL_422`, and a second
    /// conversion on top of the decoder's own darkens and shifts hue on an
    /// image that still looks like an image.
    ///
    /// ```text
    /// colour space = Rgb                       when the decoder reports Rgb
    ///                the data set's value       otherwise
    ///
    /// layout       = Interleaved               when the decoder reports Interleaved
    ///                Interleaved               when the encoding is Encapsulated
    ///                the data set's value       otherwise
    /// ```
    ///
    /// The second layout line is PS3.3 C.7.6.3.1.3's "required to be 0 when the
    /// Pixel Data is encapsulated", so a data set saying `1` for a JPEG frame
    /// is ignored rather than honoured.
    ///
    /// # Errors
    ///
    /// [`PixelError::ColorTransformNotApplicable`] for a monochrome frame,
    /// whose route is stage 3. [`PixelError::CodecOwnedColorTransform`] for
    /// `YBR_ICT` or `YBR_RCT` that a decoder did not convert, because PS3.3
    /// permits those only with JPEG 2000 and the codestream's own multiple
    /// component transform is what inverts them. RCT is an integer lifting
    /// transform and not a matrix at all, so applying the full-range matrix to
    /// it would produce a plausible image in the wrong colours.
    /// [`PixelError::MissingPaletteLut`] and
    /// [`PixelError::UnexpectedPaletteLut`] when the supplied palette and the
    /// resolved space disagree.
    pub fn resolve(
        layout: SampleLayout,
        encoding: PixelDataEncoding,
        decoded_photometric: DecodedPhotometric,
        decoded_layout: DecodedLayout,
        palette: Option<PaletteColorLut>,
    ) -> Result<Self, PixelError> {
        let space = match decoded_photometric {
            DecodedPhotometric::Rgb => ColorSpace::Rgb,
            DecodedPhotometric::Preserved => match layout.photometric_interpretation() {
                PhotometricInterpretation::Monochrome1 | PhotometricInterpretation::Monochrome2 => {
                    return Err(PixelError::ColorTransformNotApplicable);
                }
                PhotometricInterpretation::YbrIct | PhotometricInterpretation::YbrRct => {
                    return Err(PixelError::CodecOwnedColorTransform);
                }
                PhotometricInterpretation::PaletteColor => ColorSpace::Palette,
                PhotometricInterpretation::Rgb => ColorSpace::Rgb,
                PhotometricInterpretation::YbrFull => ColorSpace::YbrFull,
                PhotometricInterpretation::YbrFull422 => ColorSpace::YbrFull422,
                PhotometricInterpretation::YbrPartial422 => ColorSpace::YbrPartial422,
            },
        };

        let planar = match (decoded_layout, encoding) {
            (DecodedLayout::Interleaved, _) | (_, PixelDataEncoding::Encapsulated) => {
                PlanarConfiguration::Interleaved
            }
            (DecodedLayout::Preserved, PixelDataEncoding::Native) => layout
                .planar_configuration()
                .unwrap_or(PlanarConfiguration::Interleaved),
        };

        match (space, &palette) {
            (ColorSpace::Palette, None) => return Err(PixelError::MissingPaletteLut),
            (ColorSpace::Palette, Some(_)) => {}
            (_, Some(_)) => return Err(PixelError::UnexpectedPaletteLut),
            (_, None) => {}
        }

        Ok(Self {
            space,
            planar,
            palette,
        })
    }

    /// Map one frame of stored samples into caller-provided RGB storage.
    ///
    /// The destination length is the frame's pixel count, and the source must
    /// hold exactly that many pixels' worth of stored samples for the resolved
    /// space. Both are checked before the first destination element is
    /// written, which is the contract [`crate::LutChain::map_into`] and
    /// [`crate::StoredPixelDescription::unpack`] already keep.
    ///
    /// # Errors
    ///
    /// [`PixelError::SourceLength`] when the source is not exactly one frame
    /// for the resolved space. [`PixelError::SubsampledChromaAlignment`] when
    /// a 4:2:2 space is asked for an odd number of pixels, which would split a
    /// chroma pair.
    pub fn map_into(&self, source: &[Stored], destination: &mut [Rgb]) -> Result<(), PixelError> {
        let pixels = destination.len();
        if self.space.is_subsampled() && !pixels.is_multiple_of(2) {
            return Err(PixelError::SubsampledChromaAlignment);
        }
        let expected = pixels
            .checked_mul(self.space.stored_samples_per_pixel())
            .ok_or(PixelError::LengthOverflow)?;
        if source.len() != expected {
            return Err(PixelError::SourceLength);
        }

        for (index, output) in destination.iter_mut().enumerate() {
            *output = self.pixel(source, index, pixels);
        }
        Ok(())
    }

    /// One pixel, by index, from a source already length-checked.
    ///
    /// Every `get` below is inside a range `map_into` proved, and each falls
    /// back to zero rather than panicking because an exported path may not
    /// panic (HLD section 23) and a bounds proof carried in a comment is not
    /// the same as one the compiler keeps. The palette arm's `map_or` default
    /// is unreachable for the same kind of reason: [`ColorTransform::resolve`]
    /// refuses a palette space with no palette, so the `None` branch describes
    /// a state that cannot be constructed.
    ///
    /// Every arm names its variant. **No wildcard arm**, deliberately: a
    /// wildcard here would absorb a colour space added later and treat it as
    /// pass-through RGB, which is a plausible image in the wrong colours and
    /// exactly the defect class this crate exists to prevent. The compiler
    /// refusing to build is the intended way to find out.
    fn pixel(&self, source: &[Stored], index: usize, pixels: usize) -> Rgb {
        let at = |offset: usize| source.get(offset).copied().unwrap_or(Stored(0.0)).0;
        // PS3.3 C.7.6.3.1.3. Interleaved is RGBRGB, planar is RRR GGG BBB.
        let triple = |planar: PlanarConfiguration| {
            let (first, second, third) = match planar {
                PlanarConfiguration::Interleaved => (index * 3, index * 3 + 1, index * 3 + 2),
                PlanarConfiguration::Planar => (index, index + pixels, index + pixels * 2),
            };
            (at(first), at(second), at(third))
        };
        // PS3.3 C.7.6.3.1.2. Each pair of horizontally adjacent pixels is
        // stored Y1 Y2 Cb Cr, and both pixels of the pair share the one chroma
        // sample. Replication rather than interpolation, because the standard
        // names no filter.
        let subsampled_triple = || {
            let pair = index / 2;
            (at(pair * 4 + index % 2), at(pair * 4 + 2), at(pair * 4 + 3))
        };
        match self.space {
            ColorSpace::Palette => self
                .palette
                .as_ref()
                .map_or(Rgb::BLACK, |palette| palette.lookup(Stored(at(index)))),
            ColorSpace::Rgb => {
                let (r, g, b) = triple(self.planar);
                Rgb { r, g, b }
            }
            ColorSpace::YbrFull => ybr_to_rgb(triple(self.planar), &FULL_RANGE),
            ColorSpace::YbrFull422 => ybr_to_rgb(subsampled_triple(), &FULL_RANGE),
            ColorSpace::YbrPartial422 => ybr_to_rgb(subsampled_triple(), &PARTIAL_RANGE),
        }
    }
}

/// An inverse YCbCr matrix with the input offsets its forward form added.
///
/// PS3.3 C.7.6.3.1.2 states only the `RGB -> YBR` direction, so every
/// coefficient here is the exact rational inverse of the matrix the standard
/// states, rounded once to `f32`. The alternative was the textbook BT.601
/// inverse, which differs from this by at most **0.020028 of 255** over the
/// 8-bit cube. Inverting the stated matrix is self-consistent by construction
/// and that divergence is measured rather than claimed to be zero.
struct YbrMatrix {
    /// Subtracted from Y, Cb and Cr before the matrix is applied.
    offset: (f32, f32, f32),
    red: (f32, f32, f32),
    green: (f32, f32, f32),
    blue: (f32, f32, f32),
}

/// Inverse of PS3.3 C.7.6.3.1.2's `YBR_FULL` equations.
///
/// ```text
/// Y  = + .2990 R + .5870 G + .1140 B
/// Cb = - .1687 R - .3313 G + .5000 B + 128
/// Cr = + .5000 R - .4187 G - .0813 B + 128
/// ```
///
/// The two near-zero coefficients are the residue of the standard's rounding
/// to four decimals. Writing them as zero would be a second rounding decision
/// this stage is not entitled to make.
///
/// The exact rational inverse, for the reviewer checking these against PS3.3:
///
/// ```text
/// R = 1.000000000 Y - 0.000036820 Cb' + 1.401987577 Cr'
/// G = 1.000000000 Y - 0.344113281 Cb' - 0.714103821 Cr'
/// B = 1.000000000 Y + 1.771978117 Cb' - 0.000134583 Cr'
/// ```
///
/// Each literal below is the shortest decimal that round-trips to the nearest
/// `f32` to its exact value above. Carrying the extra digits in the literal
/// would claim a precision `f32` does not have, which is what
/// `clippy::excessive_precision` refuses, so the exact values live in this
/// comment and the representable ones live in the code.
const FULL_RANGE: YbrMatrix = YbrMatrix {
    offset: (0.0, 128.0, 128.0),
    red: (1.0, -0.000_036_82, 1.401_987_6),
    green: (1.0, -0.344_113_3, -0.714_103_8),
    blue: (1.0, 1.771_978_1, -0.000_134_583),
};

/// Inverse of PS3.3 C.7.6.3.1.2's `YBR_PARTIAL_422` equations.
///
/// ```text
/// Y  = + .2568 R + .5041 G + .0979 B + 16
/// Cb = - .1482 R - .2910 G + .4392 B + 128
/// Cr = + .4392 R - .3678 G - .0714 B + 128
/// ```
///
/// **The `+ 16` is the detail a full-range implementation omits.** Omitting it
/// reads studio black as 18.63 of 255 rather than 0, on every channel.
///
/// The exact rational inverse, for the reviewer checking these against PS3.3:
///
/// ```text
/// R = 1.164415463 Y' - 0.000095036 Cb' + 1.596001878 Cr'
/// G = 1.164415463 Y' - 0.391724564 Cb' - 0.813013368 Cr'
/// B = 1.164415463 Y' + 2.017290682 Cb' - 0.000135273 Cr'
/// ```
const PARTIAL_RANGE: YbrMatrix = YbrMatrix {
    offset: (16.0, 128.0, 128.0),
    red: (1.164_415_5, -0.000_095_036, 1.596_001_9),
    green: (1.164_415_5, -0.391_724_56, -0.813_013_4),
    blue: (1.164_415_5, 2.017_290_6, -0.000_135_273),
};

/// PS3.3 C.7.6.3.1.2, applied in the decode direction.
///
/// No clamping to `[0, 255]`. The standard's rounded forward matrix is not
/// exactly normalised, so a saturated primary encodes to `Cb = 255.5`, and a
/// stage that clamped would hide that rather than report it.
fn ybr_to_rgb(ybr: (f32, f32, f32), matrix: &YbrMatrix) -> Rgb {
    let y = ybr.0 - matrix.offset.0;
    let cb = ybr.1 - matrix.offset.1;
    let cr = ybr.2 - matrix.offset.2;
    let apply = |row: (f32, f32, f32)| row.0 * y + row.1 * cb + row.2 * cr;
    Rgb {
        r: apply(matrix.red),
        g: apply(matrix.green),
        b: apply(matrix.blue),
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{
        ColorTransform, DecodedLayout, DecodedPhotometric, PaletteColorLut, PixelDataEncoding, Rgb,
    };
    use crate::{
        PixelError,
        lut::LutDescriptor,
        stored_pixel::{PhotometricInterpretation, PlanarConfiguration, SampleLayout},
    };
    use ocelli_core::Stored;

    fn rgb_layout(planar: PlanarConfiguration) -> Result<SampleLayout, PixelError> {
        SampleLayout::new(3, PhotometricInterpretation::Rgb, Some(planar))
    }

    #[test]
    fn an_encapsulated_frame_ignores_a_declared_planar_configuration() {
        let layout = rgb_layout(PlanarConfiguration::Planar);
        assert!(layout.is_ok());
        let Ok(layout) = layout else { return };

        let encapsulated = ColorTransform::resolve(
            layout,
            PixelDataEncoding::Encapsulated,
            DecodedPhotometric::Preserved,
            DecodedLayout::Preserved,
            None,
        );
        let native = ColorTransform::resolve(
            layout,
            PixelDataEncoding::Native,
            DecodedPhotometric::Preserved,
            DecodedLayout::Preserved,
            None,
        );
        assert!(encapsulated.is_ok());
        assert!(native.is_ok());
        let (Ok(encapsulated), Ok(native)) = (encapsulated, native) else {
            return;
        };
        assert_eq!(encapsulated.planar, PlanarConfiguration::Interleaved);
        assert_eq!(native.planar, PlanarConfiguration::Planar);
    }

    #[test]
    fn a_decoder_claiming_interleaved_overrides_a_native_planar_declaration() {
        let layout = rgb_layout(PlanarConfiguration::Planar);
        assert!(layout.is_ok());
        let Ok(layout) = layout else { return };
        let resolved = ColorTransform::resolve(
            layout,
            PixelDataEncoding::Native,
            DecodedPhotometric::Preserved,
            DecodedLayout::Interleaved,
            None,
        );
        assert!(resolved.is_ok());
        let Ok(resolved) = resolved else { return };
        assert_eq!(resolved.planar, PlanarConfiguration::Interleaved);
    }

    #[test]
    fn a_palette_supplied_for_a_colour_space_is_refused() {
        let layout = rgb_layout(PlanarConfiguration::Interleaved);
        assert!(layout.is_ok());
        let Ok(layout) = layout else { return };
        let red = LutDescriptor::new(1, 0, 8, vec![1.0]);
        let green = LutDescriptor::new(1, 0, 8, vec![2.0]);
        let blue = LutDescriptor::new(1, 0, 8, vec![3.0]);
        assert!(red.is_ok() && green.is_ok() && blue.is_ok());
        let (Ok(red), Ok(green), Ok(blue)) = (red, green, blue) else {
            return;
        };
        let palette = PaletteColorLut::new(red, green, blue);
        assert!(palette.is_ok());
        let Ok(palette) = palette else { return };
        assert_eq!(
            ColorTransform::resolve(
                layout,
                PixelDataEncoding::Native,
                DecodedPhotometric::Preserved,
                DecodedLayout::Preserved,
                Some(palette),
            ),
            Err(PixelError::UnexpectedPaletteLut)
        );
    }

    #[test]
    fn mapping_refuses_an_odd_pixel_count_for_subsampled_chroma() {
        let layout = SampleLayout::new(
            3,
            PhotometricInterpretation::YbrFull422,
            Some(PlanarConfiguration::Interleaved),
        );
        assert!(layout.is_ok());
        let Ok(layout) = layout else { return };
        let resolved = ColorTransform::resolve(
            layout,
            PixelDataEncoding::Native,
            DecodedPhotometric::Preserved,
            DecodedLayout::Preserved,
            None,
        );
        assert!(resolved.is_ok());
        let Ok(resolved) = resolved else { return };
        let mut destination = [Rgb::BLACK; 1];
        assert_eq!(
            resolved.map_into(&[Stored(1.0), Stored(2.0)], &mut destination),
            Err(PixelError::SubsampledChromaAlignment)
        );
    }
}
