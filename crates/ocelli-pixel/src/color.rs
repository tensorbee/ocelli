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

use glam::Vec2;
use ocelli_core::Modality;

use crate::{
    PixelError,
    icc::IccTransform,
    lut::{LutDescriptor, PresentationTransform, VoiFunction, VoiTransform},
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
    /// Red, green and blue display scales. The identity for all three when
    /// ruling R1 reads the palette as 8-bit, otherwise each channel's declared
    /// depth minus 8.
    scales: [DisplayScale; 3],
    /// Ruling R1's fallback was taken, for all three channels together.
    read_as_8_bit: bool,
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
    ///
    /// Each channel's declared depth is its descriptor's container, 8 or 16,
    /// and [`Self::with_entry_bits`] applies the one palette rule to it.
    pub fn new(
        red: LutDescriptor,
        green: LutDescriptor,
        blue: LutDescriptor,
    ) -> Result<Self, PixelError> {
        let bits = [
            u16::from(red.bits_per_entry()),
            u16::from(green.bits_per_entry()),
            u16::from(blue.bits_per_entry()),
        ];
        Self::with_entry_bits(red, green, blue, bits)
    }

    /// [`Self::new`] with each channel's declared entry depth given apart from
    /// its container.
    ///
    /// F-239's display scale accepts `8 ..= 16` bits, the operator's answer to
    /// its question 8, while [`LutDescriptor`] holds entries in an 8 or 16-bit
    /// container. A descriptor declaring 12 bits therefore arrives as a 16-bit
    /// container with `12` here.
    ///
    /// The one palette rule, ruling R1: the palette is read as 8-bit, and
    /// [`Self::entries_read_as_8_bit`] is set, only when at least one channel
    /// is declared above 8 bits and every entry of every such channel fits in
    /// 8 bits. Otherwise each channel is shifted by its own declared depth
    /// minus 8.
    ///
    /// # Errors
    ///
    /// [`PixelError::UnsupportedColourDepth`] for a depth outside `8 ..= 16`.
    /// [`PixelError::InvalidLutBits`] for a depth above the channel's own
    /// container. [`PixelError::LutValueOutOfRange`] for an entry above
    /// `2^bits - 1`, which is not a value of the declared depth. Then
    /// [`PixelError::MismatchedPaletteDescriptors`] as [`Self::new`] documents.
    pub fn with_entry_bits(
        red: LutDescriptor,
        green: LutDescriptor,
        blue: LutDescriptor,
        bits: [u16; 3],
    ) -> Result<Self, PixelError> {
        let [red_bits, green_bits, blue_bits] = bits;
        let declared = [
            channel_scale(&red, red_bits)?,
            channel_scale(&green, green_bits)?,
            channel_scale(&blue, blue_bits)?,
        ];
        // Ruling R1, decided once for the whole palette by the lead's ruling
        // of 2026-11-07. The palette is read as 8-bit only when at least one
        // channel is declared above 8 bits and every entry of every such
        // channel fits in 8 bits. Deciding per channel would read a near-zero
        // channel of a conforming 16-bit palette as 8-bit, about 256 times too
        // bright beside its shifted neighbours, which is a quietly wrong colour.
        let wide = [(&red, red_bits), (&green, green_bits), (&blue, blue_bits)];
        let any_wide = wide.iter().any(|(_, bits)| *bits > 8);
        let read_as_8_bit = any_wide
            && wide
                .iter()
                .filter(|(_, bits)| *bits > 8)
                .all(|(lut, _)| lut.values().iter().all(|value| *value <= 255.0));
        let scales = if read_as_8_bit {
            [DisplayScale::IDENTITY; 3]
        } else {
            declared
        };
        let key =
            |descriptor: &LutDescriptor| (descriptor.entry_count(), descriptor.first_input_bits());
        if key(&red) != key(&green) || key(&red) != key(&blue) {
            return Err(PixelError::MismatchedPaletteDescriptors);
        }
        Ok(Self {
            red,
            green,
            blue,
            scales,
            read_as_8_bit,
        })
    }

    fn lookup(&self, stored: Stored) -> Rgb {
        // The three ramps are the same input mapping. Search once.
        let index = self.red.lookup_index(stored.0);
        let [red, green, blue] = self.scales;
        Rgb {
            r: red.apply(self.red.values().get(index).copied().unwrap_or(0.0)),
            g: green.apply(self.green.values().get(index).copied().unwrap_or(0.0)),
            b: blue.apply(self.blue.values().get(index).copied().unwrap_or(0.0)),
        }
    }

    /// Number of entries, already resolved so a declared `0` is 65,536.
    ///
    /// The three channels agreed on this in [`PaletteColorLut::new`].
    pub fn entry_count(&self) -> usize {
        self.red.entry_count()
    }

    /// Capacity in bytes of the three channels' owned lookup storage.
    /// Excludes this value itself, which its owner already counts.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.red
            .heap_bytes()
            .saturating_add(self.green.heap_bytes())
            .saturating_add(self.blue.heap_bytes())
    }

    /// PS3.3 C.7.6.3.1.5's first stored pixel value mapped.
    /// Shared by the three channels. [`ColorTransform::map_into`] applies it.
    pub fn first_mapped_input(&self) -> i32 {
        self.red.first_mapped_input()
    }

    /// Whether ruling R1 read the palette as 8-bit.
    ///
    /// True only when at least one channel is declared above 8 bits and every
    /// entry of every such channel fits in 8 bits, so the whole palette was
    /// taken as it is rather than shifted. That is how cornerstone3D 5.8.2
    /// reads such a palette, and it departs from PS3.3 C.7.6.3.1.5's "scaled
    /// across the full range", so a caller states it rather than hiding it.
    #[must_use]
    pub const fn entries_read_as_8_bit(&self) -> bool {
        self.read_as_8_bit
    }

    /// Red, then green, then blue LUT entries, each after its display scale.
    ///
    /// These are the values [`ColorTransform::map_into`] emits, so a check of
    /// them is a check of the stage's output rather than of the raw table.
    pub fn channel_entries(&self) -> impl Iterator<Item = f32> + '_ {
        let [red, green, blue] = self.scales;
        self.red
            .values()
            .iter()
            .map(move |value| red.apply(*value))
            .chain(
                self.green
                    .values()
                    .iter()
                    .map(move |value| green.apply(*value)),
            )
            .chain(
                self.blue
                    .values()
                    .iter()
                    .map(move |value| blue.apply(*value)),
            )
    }
}

/// One palette channel's display scale from its declared depth, refused when
/// that depth does not describe the entries the container holds. Ruling R1's
/// fallback is decided over the whole palette in
/// [`PaletteColorLut::with_entry_bits`], not here.
fn channel_scale(descriptor: &LutDescriptor, bits: u16) -> Result<DisplayScale, PixelError> {
    let scale = DisplayScale::from_bits(bits)?;
    if bits > u16::from(descriptor.bits_per_entry()) {
        return Err(PixelError::InvalidLutBits);
    }
    // `bits` is at most 16 here, so `2^bits - 1` is at most 65,535 and is
    // exact in `f32`. Built from a `u16` rather than cast.
    let largest = f32::from(u16::MAX >> (16 - bits));
    if descriptor.values().iter().any(|value| *value > largest) {
        return Err(PixelError::LutValueOutOfRange);
    }
    Ok(scale)
}

/// The one display scaling rule for colour above 8 bits, F-239 item 11.
///
/// A colour sample or a palette entry of `n` bits is shifted right by
/// `n - 8` for `9 <= n <= 16`, and taken as it is for `n = 8`. For a sample,
/// `n` is Bits Stored `(0028,0101)`, PS3.3 C.7.6.3.1's range of each sample.
/// For a palette entry, [`PaletteColorLut::with_entry_bits`] applies the one
/// palette rule.
///
/// The shift truncates, as cornerstone's `>> 8` does, so every output code
/// covers an equal share of the input range. Rounding is a separate decision
/// made once at the render boundary, not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayScale {
    shift: u8,
}

impl DisplayScale {
    /// Eight bits, the identity.
    pub const IDENTITY: Self = Self { shift: 0 };

    /// The scale for an `n`-bit value.
    ///
    /// # Errors
    ///
    /// [`PixelError::UnsupportedColourDepth`] for `n` outside `8 ..= 16`.
    /// Below 8 there is nothing to shift and above 16 the value is not exact
    /// through this crate's `f32` path, and both are refused by name rather
    /// than clamped into range.
    pub fn from_bits(bits: u16) -> Result<Self, PixelError> {
        if !(8..=16).contains(&bits) {
            return Err(PixelError::UnsupportedColourDepth);
        }
        // `bits - 8` is in `0 ..= 8` after the check above.
        let shift = u8::try_from(bits - 8).map_err(|_| PixelError::UnsupportedColourDepth)?;
        Ok(Self { shift })
    }

    /// How many bits the value is shifted right by.
    #[must_use]
    pub const fn shift(self) -> u8 {
        self.shift
    }

    /// `floor(value / 2^shift)`, the right shift of an integer-valued `f32`.
    ///
    /// Division by a power of two is exact in `f32`, and so is the floor, for
    /// every integer up to `2^24`, which covers the 16-bit range this rule
    /// accepts. The identity returns the value untouched, so an 8-bit frame
    /// is bit-identical to one that was never scaled.
    #[must_use]
    pub fn apply(self, value: f32) -> f32 {
        if self.shift == 0 {
            return value;
        }
        let divisor = f32::from(1_u16 << self.shift);
        // `ocelli-pixel` is `no_std`, where scalar `f32::floor` is not
        // available. glam's libm backend is the one `lut.rs` already uses.
        Vec2::splat(value / divisor).floor().x
    }
}

/// Deviation D-29, the colour window and invert, applied per channel after
/// stage 4.
///
/// PS3.3 C.11.2 defines no window for a colour image, and D-23 keeps colour
/// out of stages 1 to 3. The operator chose on 2026-11-07 to follow
/// cornerstone3D 5.8.2, which applies one LINEAR window to each of R, G and B
/// (`getVOILut.js:1` to `:7`) and inverts after it as `255 - y`
/// (`generateColorLUT.js:13` to `:21`).
///
/// **No window arithmetic is written here.** cornerstone's function is
/// PS3.3 C.11.2.1.2's LINEAR with output range `0 ..= 255`, so the window is
/// [`VoiTransform`] with [`VoiFunction::Linear`] and the invert is
/// [`PresentationTransform`]'s INVERSE over the same range, each applied to
/// each channel. HLD section 18's rule that the arithmetic exists once holds.
#[derive(Clone, Debug, PartialEq)]
pub struct ColourWindow(ColourWindowKind);

#[derive(Clone, Debug, PartialEq)]
enum ColourWindowKind {
    /// cornerstone3D's skipped table, `renderColorImage.js:30` to `:36`.
    Identity,
    /// The LINEAR window, then INVERSE when inverted.
    Window {
        voi: VoiTransform,
        invert: Option<PresentationTransform>,
    },
}

impl ColourWindow {
    /// Width 256 and centre 128 with invert off, every colour image's initial
    /// window whatever the file says, `createImage.js:321` to `:324`.
    ///
    /// That window is the identity on `0 ..= 255` and is skipped.
    #[must_use]
    pub const fn initial() -> Self {
        Self(ColourWindowKind::Identity)
    }

    /// A LINEAR window over `0 ..= 255`, inverted after it when `invert`.
    ///
    /// The window is skipped, exactly as `renderColorImage.js:30` to `:36`
    /// skips its table, when the width is 255 or 256, the centre is 127 or
    /// 128 and invert is off. At width 255 and centre 127 the formula is not
    /// the identity, so applying it there would differ from the reference by
    /// a byte.
    ///
    /// # Errors
    ///
    /// [`VoiTransform::new`]'s refusals unchanged, so a width below 1 or a
    /// non-finite centre is [`PixelError::InvalidWindow`].
    pub fn new(center: f32, width: f32, invert: bool) -> Result<Self, PixelError> {
        let voi = VoiTransform::new(
            None,
            &[center],
            &[width],
            0,
            VoiFunction::Linear,
            COLOUR_DISPLAY_MIN,
            COLOUR_DISPLAY_MAX,
        )?;
        let skipped_width = [255.0_f32, 256.0]
            .iter()
            .any(|value| value.to_bits() == width.to_bits());
        let skipped_center = [127.0_f32, 128.0]
            .iter()
            .any(|value| value.to_bits() == center.to_bits());
        if skipped_width && skipped_center && !invert {
            return Ok(Self::initial());
        }
        let invert = if invert {
            Some(PresentationTransform::inverse_over(
                COLOUR_DISPLAY_MIN,
                COLOUR_DISPLAY_MAX,
            )?)
        } else {
            None
        };
        Ok(Self(ColourWindowKind::Window { voi, invert }))
    }

    /// Whether this window is skipped, so a caller can skip the pass too.
    #[must_use]
    pub const fn is_identity(&self) -> bool {
        matches!(self.0, ColourWindowKind::Identity)
    }

    /// One colour triple, each channel through the window and then the invert.
    #[must_use]
    pub fn apply(&self, rgb: Rgb) -> Rgb {
        match &self.0 {
            ColourWindowKind::Identity => rgb,
            ColourWindowKind::Window { voi, invert } => {
                // Each channel is a display sample, so it enters the LINEAR
                // window as that stage's input and the INVERSE stage as its
                // output. The type changes, the arithmetic is the stages'.
                let channel = |value: f32| {
                    let windowed = voi.apply(Modality(value));
                    invert
                        .as_ref()
                        .map_or(windowed, |inverse| inverse.apply(windowed))
                        .0
                };
                Rgb {
                    r: channel(rgb.r),
                    g: channel(rgb.g),
                    b: channel(rgb.b),
                }
            }
        }
    }

    /// Map caller-provided slices without allocating.
    ///
    /// # Errors
    ///
    /// [`PixelError::DestinationLength`] before the first write when the two
    /// slices differ in length.
    pub fn map_into(&self, source: &[Rgb], destination: &mut [Rgb]) -> Result<(), PixelError> {
        if source.len() != destination.len() {
            return Err(PixelError::DestinationLength);
        }
        for (input, output) in source.iter().copied().zip(destination.iter_mut()) {
            *output = self.apply(input);
        }
        Ok(())
    }

    /// [`Self::map_into`] over one buffer, for a caller with a single scratch.
    pub fn apply_in_place(&self, rgb: &mut [Rgb]) {
        if self.is_identity() {
            return;
        }
        for value in rgb {
            *value = self.apply(*value);
        }
    }
}

/// The colour window's output range, cornerstone3D's `0 ..= 255`.
const COLOUR_DISPLAY_MIN: f32 = 0.0;
const COLOUR_DISPLAY_MAX: f32 = 255.0;

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
    /// F-239's display scale for every stored sample entering a non-palette
    /// space. [`DisplayScale::IDENTITY`] unless [`Self::with_sample_scale`].
    sample_scale: DisplayScale,
    /// F-240's profile transform, attached once per held frame by
    /// [`Self::with_icc`]. `None` keeps the routes below byte for byte.
    icc: Option<IccTransform>,
}

impl ColorTransform {
    /// The palette, when this transform is palette colour.
    #[must_use]
    pub fn palette(&self) -> Option<&PaletteColorLut> {
        self.palette.as_ref()
    }

    /// Heap storage retained by the palette and the attached ICC transform.
    /// Inline fields belong to the owner, which counts them separately.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.palette
            .as_ref()
            .map_or(0, PaletteColorLut::heap_bytes)
            .saturating_add(self.icc.as_ref().map_or(0, IccTransform::heap_bytes))
    }

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
    /// `YBR_ICT`, `YBR_RCT` or `XYB` that a decoder did not convert. PS3.3
    /// permits the first two only with JPEG 2000, and the codestream's own
    /// multiple component transform is what inverts them. RCT is an integer
    /// lifting transform and not a matrix at all, so applying the full-range
    /// matrix to it would produce a plausible image in the wrong colours.
    /// PS3.3 C.7.6.3.1.2 gives no matrix for `XYB`, so the same refusal applies.
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
                PhotometricInterpretation::YbrIct
                | PhotometricInterpretation::YbrRct
                | PhotometricInterpretation::Xyb => {
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
            sample_scale: DisplayScale::IDENTITY,
            icc: None,
        })
    }

    /// Scale every stored sample by `scale` before stage 4, F-239 item 11.
    ///
    /// The shift happens at the one place a sample enters stage 4, so the
    /// 8-bit YCbCr matrix and its `+ 128` offset of PS3.3 C.7.6.3.1.2 meet an
    /// 8-bit value. [`Self::resolve`] leaves the identity, which is every
    /// 8-bit frame.
    ///
    /// # Errors
    ///
    /// [`PixelError::SampleScaleNotApplicable`] for palette colour, whose
    /// stored value is an index PS3.3 C.7.6.3.1.5 keys on the raw value.
    /// Scaling it would read the wrong entry. A palette's entries are scaled
    /// by [`PaletteColorLut`] instead.
    pub fn with_sample_scale(mut self, scale: DisplayScale) -> Result<Self, PixelError> {
        if self.space == ColorSpace::Palette {
            return Err(PixelError::SampleScaleNotApplicable);
        }
        self.sample_scale = scale;
        Ok(self)
    }

    /// The display scale applied to every stored sample before stage 4.
    #[must_use]
    pub const fn sample_scale(&self) -> DisplayScale {
        self.sample_scale
    }

    /// Attach a profile's transform, decided once for the held frame.
    ///
    /// F-240. The device depths the transform normalises by come from the
    /// frame, so `with_sample_scale` goes first for a frame above 8 bits:
    /// the depth is that scale's own. A palette uses each channel's declared
    /// entry depth, ruling R1 included, and the YCbCr routes use 8, because
    /// their matrices consume 8-bit samples and return 8-bit device RGB. The
    /// TRC tables are built here, once, for a sample or palette space. A YBR
    /// route keeps the curves, because its device values leave the matrix
    /// fractional and no table of codes covers them.
    #[must_use]
    pub fn with_icc(mut self, icc: IccTransform) -> Self {
        let depths = match self.space {
            // A palette transform always has its palette, `resolve` guaranteed
            // it, so the fallback depth here is unreachable.
            ColorSpace::Palette => self.palette.as_ref().map_or([8, 8, 8], palette_icc_depths),
            ColorSpace::Rgb => {
                let depth = u16::from(self.sample_scale.shift()) + 8;
                [depth, depth, depth]
            }
            ColorSpace::YbrFull | ColorSpace::YbrFull422 | ColorSpace::YbrPartial422 => [8, 8, 8],
        };
        let icc = match self.space {
            ColorSpace::YbrFull | ColorSpace::YbrFull422 | ColorSpace::YbrPartial422 => icc,
            _ => icc.tabulated(&depths),
        };
        self.icc = Some(icc);
        self
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
    /// a 4:2:2 space is asked for an odd number of pixels or an odd column
    /// count. The chroma pair is horizontal, so an even pixel count with odd
    /// columns still joins the last pixel of one row to the first of the next.
    pub fn map_into(
        &self,
        source: &[Stored],
        destination: &mut [Rgb],
        columns: usize,
    ) -> Result<(), PixelError> {
        let pixels = destination.len();
        self.accept_source(source, pixels, columns)?;
        for (index, output) in destination.iter_mut().enumerate() {
            *output = self.pixel(source, index, pixels);
        }
        Ok(())
    }

    /// Accept one frame's stored samples before any destination write.
    ///
    /// `pixels` is the frame's pixel count and `columns` is its column count.
    /// [`Self::map_into`] passes the destination length and the caller's
    /// column count. A caller that has not allocated the destination yet can
    /// use the same rule.
    ///
    /// # Errors
    ///
    /// [`PixelError::SourceLength`] when the source is not exactly one frame
    /// for the resolved space. [`PixelError::SubsampledChromaAlignment`] when
    /// a 4:2:2 space is asked for an odd number of pixels or an odd column
    /// count. [`PixelError::LengthOverflow`] when the expected sample count
    /// overflows.
    pub fn accept_source(
        &self,
        source: &[Stored],
        pixels: usize,
        columns: usize,
    ) -> Result<(), PixelError> {
        if self.space.is_subsampled()
            && (columns == 0 || !columns.is_multiple_of(2) || !pixels.is_multiple_of(2))
        {
            return Err(PixelError::SubsampledChromaAlignment);
        }
        let expected = pixels
            .checked_mul(self.space.stored_samples_per_pixel())
            .ok_or(PixelError::LengthOverflow)?;
        if source.len() != expected {
            return Err(PixelError::SourceLength);
        }
        Ok(())
    }

    /// One pixel, by index, from a source already length-checked.
    ///
    /// Every `get` below is inside a range `map_into` proved, and each falls
    /// back to zero rather than panicking because an exported path may not
    /// panic (HLD section 23) and a bounds proof carried in a comment is not
    /// the same as one the compiler keeps. The palette arm's `Rgb::BLACK`
    /// fallback is unreachable for the same kind of reason:
    /// [`ColorTransform::resolve`] refuses a palette space with no palette,
    /// so the `None` branch describes a state that cannot be constructed.
    ///
    /// Every arm names its variant. **No wildcard arm**, deliberately: a
    /// wildcard here would absorb a colour space added later and treat it as
    /// pass-through RGB, which is a plausible image in the wrong colours and
    /// exactly the defect class this crate exists to prevent. The compiler
    /// refusing to build is the intended way to find out.
    fn pixel(&self, source: &[Stored], index: usize, pixels: usize) -> Rgb {
        // F-239 item 11. A palette transform keeps the identity, so its index
        // is read raw, as `with_sample_scale` guarantees. With a profile the
        // scale is bypassed instead: the profile normalises device values by
        // the encoding range, so the codes must stay whole.
        let raw_at = |offset: usize| source.get(offset).copied().unwrap_or(Stored(0.0)).0;
        let at = |offset: usize| self.sample_scale.apply(raw_at(offset));
        let addresses = |planar: PlanarConfiguration| match planar {
            PlanarConfiguration::Interleaved => (index * 3, index * 3 + 1, index * 3 + 2),
            PlanarConfiguration::Planar => (index, index + pixels, index + pixels * 2),
        };
        let triple = |planar: PlanarConfiguration| {
            let (first, second, third) = addresses(planar);
            (at(first), at(second), at(third))
        };
        let raw_triple = |planar: PlanarConfiguration| {
            let (first, second, third) = addresses(planar);
            (raw_at(first), raw_at(second), raw_at(third))
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
            ColorSpace::Palette => {
                let Some(palette) = self.palette.as_ref() else {
                    return Rgb::BLACK;
                };
                let stored = Stored(at(index));
                match &self.icc {
                    // The device values are the raw entries. Each channel's
                    // declared depth is the encoding range its entries are
                    // normalised by, so the display scale is the profile's
                    // replacement and not a step before it.
                    Some(icc) => {
                        let entry = palette.red.lookup_index(stored.0);
                        let device = Rgb {
                            r: palette.red.values().get(entry).copied().unwrap_or(0.0),
                            g: palette.green.values().get(entry).copied().unwrap_or(0.0),
                            b: palette.blue.values().get(entry).copied().unwrap_or(0.0),
                        };
                        icc.apply(device, palette_icc_depths(palette))
                    }
                    None => palette.lookup(stored),
                }
            }
            ColorSpace::Rgb => match &self.icc {
                // Codes stay whole for the encoding-range normalisation.
                Some(icc) => {
                    let (r, g, b) = raw_triple(self.planar);
                    let depth = u16::from(self.sample_scale.shift()) + 8;
                    icc.apply(Rgb { r, g, b }, [depth, depth, depth])
                }
                None => {
                    let (r, g, b) = triple(self.planar);
                    Rgb { r, g, b }
                }
            },
            ColorSpace::YbrFull => {
                let rgb = ybr_to_rgb(triple(self.planar), &FULL_RANGE);
                self.icc
                    .as_ref()
                    .map_or(rgb, |icc| icc.apply(rgb, [8, 8, 8]))
            }
            ColorSpace::YbrFull422 => {
                let rgb = ybr_to_rgb(subsampled_triple(), &FULL_RANGE);
                self.icc
                    .as_ref()
                    .map_or(rgb, |icc| icc.apply(rgb, [8, 8, 8]))
            }
            ColorSpace::YbrPartial422 => {
                let rgb = ybr_to_rgb(subsampled_triple(), &PARTIAL_RANGE);
                self.icc
                    .as_ref()
                    .map_or(rgb, |icc| icc.apply(rgb, [8, 8, 8]))
            }
        }
    }
}

/// The depth each palette channel's entries are normalised by. Ruling R1's
/// read-as-8-bit palettes keep the identity scale, so the shift plus 8 is 8
/// there, and every other palette's shift plus 8 is its declared depth. One
/// rule covers both, which is how `channel_scale` built the scales.
fn palette_icc_depths(palette: &PaletteColorLut) -> [u16; 3] {
    let [red, green, blue] = palette.scales;
    let depth = |scale: DisplayScale| u16::from(scale.shift()) + 8;
    [depth(red), depth(green), depth(blue)]
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
/// Inverse coefficients shown rounded to nine decimal places for review:
///
/// ```text
/// R = 1.000000000 Y - 0.000036820 Cb' + 1.401987577 Cr'
/// G = 1.000000000 Y - 0.344113281 Cb' - 0.714103821 Cr'
/// B = 1.000000000 Y + 1.771978117 Cb' - 0.000134583 Cr'
/// ```
///
/// Each literal below rounds the exact rational inverse once to the nearest
/// `f32`, rather than rounding the decimal summaries above again. In
/// particular, red-Cb is exactly `-870 / 23628469` and blue-Cr is exactly
/// `-3180 / 23628469`. Their near-zero magnitude makes the extra digits
/// significant even though the nine-place summaries look alike.
const FULL_RANGE: YbrMatrix = YbrMatrix {
    offset: (0.0, 128.0, 128.0),
    red: (1.0, -0.000_036_819_99, 1.401_987_6),
    green: (1.0, -0.344_113_3, -0.714_103_8),
    blue: (1.0, 1.771_978_1, -0.000_134_583_42),
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
/// Inverse coefficients shown rounded to nine decimal places for review:
///
/// ```text
/// R = 1.164415463 Y' - 0.000095036 Cb' + 1.596001878 Cr'
/// G = 1.164415463 Y' - 0.391724564 Cb' - 0.813013368 Cr'
/// B = 1.164415463 Y' + 2.017290682 Cb' - 0.000135273 Cr'
/// ```
/// As for FULL_RANGE, literals round the exact inverse directly to `f32`.
/// Red-Cb is exactly `-310000 / 3261922071`. The source equations are in
/// PS3.3 2017b C.7.6.3.1.2, where YBR_PARTIAL_422 was not yet retired.
const PARTIAL_RANGE: YbrMatrix = YbrMatrix {
    offset: (16.0, 128.0, 128.0),
    red: (1.164_415_5, -0.000_095_035_99, 1.596_001_9),
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
            resolved.map_into(&[Stored(1.0), Stored(2.0)], &mut destination, 2),
            Err(PixelError::SubsampledChromaAlignment)
        );
    }
}
