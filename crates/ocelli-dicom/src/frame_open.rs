//! Frame-open inputs for one Part 10 object.
//!
//! F-191. The LUT chain stays in `ocelli-pixel`. This module only chooses the
//! inputs that chain already accepts, plus the padding mask, the lossy and
//! burned-in flags, the colour selection and the private creators.

use dicom_core::{DicomValue, Tag};
use dicom_object::{
    StandardDataDictionary,
    mem::{InMemDicomObject, InMemElement},
};
use ocelli_core::Display;
use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PaletteColorLut, PhotometricInterpretation,
    PixelError, PresentationLutEvidence, PresentationLutShape, VoiFunction, VoiTransform,
};

use crate::{
    MetadataElement, MetadataSet, MetadataValue, MultiframeError, MultiframeMetadata, ParsedDicom,
    TopLevelFallback,
};

const PHOTOMETRIC: Tag = Tag(0x0028, 0x0004);
const PIXEL_REPRESENTATION: Tag = Tag(0x0028, 0x0103);
const PIXEL_PADDING_VALUE: Tag = Tag(0x0028, 0x0120);
const PIXEL_PADDING_RANGE_LIMIT: Tag = Tag(0x0028, 0x0121);
const WINDOW_CENTER: Tag = Tag(0x0028, 0x1050);
const WINDOW_WIDTH: Tag = Tag(0x0028, 0x1051);
const RESCALE_INTERCEPT: Tag = Tag(0x0028, 0x1052);
const RESCALE_SLOPE: Tag = Tag(0x0028, 0x1053);
const VOI_LUT_FUNCTION: Tag = Tag(0x0028, 0x1056);
const LOSSY_IMAGE_COMPRESSION: Tag = Tag(0x0028, 0x2110);
const BURNED_IN_ANNOTATION: Tag = Tag(0x0028, 0x0301);
const MODALITY_LUT_SEQUENCE: Tag = Tag(0x0028, 0x3000);
const LUT_DESCRIPTOR: Tag = Tag(0x0028, 0x3002);
const LUT_DATA: Tag = Tag(0x0028, 0x3006);
const VOI_LUT_SEQUENCE: Tag = Tag(0x0028, 0x3010);
const PRESENTATION_LUT_SEQUENCE: Tag = Tag(0x2050, 0x0010);
const PRESENTATION_LUT_SHAPE: Tag = Tag(0x2050, 0x0020);
const PIXEL_VALUE_TRANSFORMATION: Tag = Tag(0x0028, 0x9145);
const FRAME_VOI_LUT: Tag = Tag(0x0028, 0x9132);
const PIXEL_DATA: Tag = Tag(0x7FE0, 0x0010);
const FLOAT_PIXEL_DATA: Tag = Tag(0x7FE0, 0x0008);
const DOUBLE_FLOAT_PIXEL_DATA: Tag = Tag(0x7FE0, 0x0009);

/// Red, green and blue Palette Color Lookup Table Descriptor, PS3.3 C.7.6.3.1.5.
const PALETTE_DESCRIPTORS: [Tag; 3] = [
    Tag(0x0028, 0x1101),
    Tag(0x0028, 0x1102),
    Tag(0x0028, 0x1103),
];
/// Red, green and blue Palette Color Lookup Table Data, PS3.3 C.7.6.3.1.6.
const PALETTE_DATA: [Tag; 3] = [
    Tag(0x0028, 0x1201),
    Tag(0x0028, 0x1202),
    Tag(0x0028, 0x1203),
];
/// Red, green and blue Segmented Palette Color Lookup Table Data, PS3.3 C.7.9.2.
const SEGMENTED_PALETTE_DATA: [Tag; 3] = [
    Tag(0x0028, 0x1221),
    Tag(0x0028, 0x1222),
    Tag(0x0028, 0x1223),
];
const ICC_PROFILE: Tag = Tag(0x0028, 0x2000);

const DISPLAY_MIN: f32 = 0.0;
const DISPLAY_MAX: f32 = 255.0;

/// Why one frame could not be opened.
///
/// The error names a tag and a closed reason. It does not carry bytes, names
/// or pixel samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FrameOpenError {
    /// The requested frame is outside the declared frame count.
    #[error("the frame index is outside the object")]
    FrameOutOfRange,
    /// The attribute is present and cannot be read for this purpose.
    #[error("DICOM attribute ({group:#06x},{element:#06x}) cannot be read as frame-open input")]
    Attribute {
        /// Group number.
        group: u16,
        /// Element number.
        element: u16,
    },
    /// `(0028,2110)` is present and is neither `00` nor `01`.
    #[error("the lossy image compression code is not 00 or 01")]
    LossyCode,
    /// `(0028,0301)` is present and is neither `YES` nor `NO`.
    #[error("the burned in annotation code is not YES or NO")]
    BurnedInCode,
    /// The padding pair contradicts PS3.3 C.7.5.1.1.2 for this photometric interpretation.
    #[error("the pixel padding range contradicts the photometric interpretation")]
    PaddingOrder,
    /// `(0028,0121)` is present and `(0028,0120)` is not.
    #[error("a pixel padding range limit is present without a pixel padding value")]
    PaddingLimitWithoutValue,
    /// A colour conversion was requested for samples that are not colour, or twice.
    #[error("colour samples would be converted a second time")]
    SecondColourConversion,
    /// The existing modality or VOI constructor refused the inputs.
    #[error("the modality or VOI inputs are not a valid chain")]
    Chain,
    /// Photometric Interpretation is missing or not one of the known codes.
    #[error("the photometric interpretation is not one this reader knows")]
    Photometric,
    /// The metadata projection refused the object.
    #[error("the object metadata could not be projected")]
    Metadata,
    /// The mask and the display buffer disagree in length.
    #[error("the padding mask and the display buffer disagree in length")]
    MaskLength,
    /// A Palette Color Lookup Table Descriptor's bits per entry is outside 8 to 16.
    #[error("a palette colour lookup table entry depth is outside 8 to 16 bits")]
    PaletteDepth,
    /// Segmented Palette Color Lookup Table Data is present, which this build does not expand.
    #[error("the palette colour lookup table is segmented")]
    SegmentedPalette,
    /// A `PALETTE COLOR` frame lacks one of its three descriptors or tables.
    #[error("a palette colour lookup table descriptor or table is missing")]
    MissingPalette,
}

/// Window inputs for the existing VOI stage.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowInput {
    /// A sequence or a validated window pair.
    Transform(VoiTransform),
    /// No sequence and no centre or width. Nothing is invented.
    Absent,
}

/// Which colour path the frame uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourSelection {
    /// `MONOCHROME1` or `MONOCHROME2`.
    Monochrome,
    /// `PALETTE COLOR`.
    Palette,
    /// RGB or a YBR family. `codec_already_rgb` means the codec already converted.
    Colour {
        /// The codec reported RGB output, so a second conversion is forbidden.
        codec_already_rgb: bool,
    },
}

/// `(0028,2110)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossyFlag {
    /// The attribute is absent or zero-length.
    Absent,
    /// Code `00`.
    Lossless,
    /// Code `01`.
    Lossy,
}

/// `(0028,0301)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BurnedInFlag {
    /// The attribute is absent or zero-length.
    Absent,
    /// Code `NO`.
    No,
    /// Code `YES`.
    Yes,
}

/// Inclusive stored-pixel padding from PS3.3 C.7.5.1.1.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddingRange {
    value: i32,
    limit: Option<i32>,
}

impl PaddingRange {
    /// The Pixel Padding Value, in stored space, before the modality stage.
    #[must_use]
    pub const fn value(self) -> i32 {
        self.value
    }

    /// The Pixel Padding Range Limit, when the file declared one.
    #[must_use]
    pub const fn limit(self) -> Option<i32> {
        self.limit
    }

    /// Whether one stored integer is inside the inclusive padding range.
    #[must_use]
    pub fn contains(self, stored: i32) -> bool {
        match self.limit {
            None => stored == self.value,
            Some(limit) => stored >= self.value.min(limit) && stored <= self.value.max(limit),
        }
    }
}

/// One private creator identifier, PS3.5 7.8.1.
///
/// The record is the creator string. It does not copy the private element values
/// that name this creator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateCreator {
    group: u16,
    slot: u8,
    creator: String,
}

impl PrivateCreator {
    /// The odd group that owns the creator.
    #[must_use]
    pub const fn group(&self) -> u16 {
        self.group
    }

    /// The element number of `(gggg,00xx)`, from `0x10` to `0xFF`.
    #[must_use]
    pub const fn slot(&self) -> u8 {
        self.slot
    }

    /// The creator identifier text.
    #[must_use]
    pub fn creator(&self) -> &str {
        &self.creator
    }
}

/// Chain inputs and the flags around one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenedFrame {
    modality: ModalityTransform,
    window: WindowInput,
    photometric: PhotometricInterpretation,
    presentation: PresentationLutEvidence,
    colour: ColourSelection,
    padding: Option<PaddingRange>,
    lossy: LossyFlag,
    burned_in: BurnedInFlag,
    private_creators: Vec<PrivateCreator>,
}

impl OpenedFrame {
    /// Stage 1, already constructed.
    #[must_use]
    pub const fn modality(&self) -> &ModalityTransform {
        &self.modality
    }

    /// Stage 2, or [`WindowInput::Absent`] when the file declares neither a
    /// sequence nor a window.
    #[must_use]
    pub const fn window(&self) -> &WindowInput {
        &self.window
    }

    /// Photometric Interpretation.
    #[must_use]
    pub const fn photometric(&self) -> PhotometricInterpretation {
        self.photometric
    }

    /// Presentation LUT evidence for the caller that builds [`ocelli_pixel::LutChain`].
    #[must_use]
    pub const fn presentation(&self) -> PresentationLutEvidence {
        self.presentation
    }

    /// Colour path.
    #[must_use]
    pub const fn colour(&self) -> ColourSelection {
        self.colour
    }

    /// Stored padding range, when declared.
    #[must_use]
    pub const fn padding(&self) -> Option<PaddingRange> {
        self.padding
    }

    /// Lossy Image Compression.
    #[must_use]
    pub const fn lossy(&self) -> LossyFlag {
        self.lossy
    }

    /// Burned In Annotation.
    #[must_use]
    pub const fn burned_in(&self) -> BurnedInFlag {
        self.burned_in
    }

    /// Private creators in ascending tag order.
    #[must_use]
    pub fn private_creators(&self) -> &[PrivateCreator] {
        &self.private_creators
    }

    /// One boolean per stored sample. `true` means padding.
    #[must_use]
    pub fn padding_mask(&self, stored: &[i32]) -> Vec<bool> {
        let Some(range) = self.padding else {
            return vec![false; stored.len()];
        };
        stored
            .iter()
            .copied()
            .map(|sample| range.contains(sample))
            .collect()
    }

    /// The creator for one private element `(gggg,xxee)`, or nothing when the
    /// element is not private data or no creator was declared for that slot.
    ///
    /// PS3.5 7.8.1. The element value is not read.
    #[must_use]
    pub fn creator_for(&self, group: u16, element: u16) -> Option<&str> {
        if !private_data_element(group, element) {
            return None;
        }
        let Ok(slot) = u8::try_from(element >> 8) else {
            return None;
        };
        self.private_creators
            .iter()
            .find(|creator| creator.group == group && creator.slot == slot)
            .map(|creator| creator.creator.as_str())
    }

    /// Record that the codec already emitted RGB.
    ///
    /// # Errors
    ///
    /// [`FrameOpenError::SecondColourConversion`] when the frame is not colour,
    /// or when this was already recorded.
    pub fn with_codec_rgb(mut self, already_rgb: bool) -> Result<Self, FrameOpenError> {
        if !already_rgb {
            return Ok(self);
        }
        match self.colour {
            ColourSelection::Colour {
                codec_already_rgb: false,
            } => {
                self.colour = ColourSelection::Colour {
                    codec_already_rgb: true,
                };
                Ok(self)
            }
            ColourSelection::Colour {
                codec_already_rgb: true,
            }
            | ColourSelection::Monochrome
            | ColourSelection::Palette => Err(FrameOpenError::SecondColourConversion),
        }
    }
}

/// Open one frame of a parsed Part 10 object.
///
/// # Errors
///
/// See [`FrameOpenError`]. A missing window stays [`WindowInput::Absent`].
pub fn open_frame(parsed: &ParsedDicom, frame: u32) -> Result<OpenedFrame, FrameOpenError> {
    let metadata = project_metadata(parsed)?;
    let multi = MultiframeMetadata::new(&metadata).map_err(map_multi)?;
    let frame_index = usize::try_from(frame).map_err(|_| FrameOpenError::FrameOutOfRange)?;
    if frame_index >= multi.frame_count() {
        return Err(FrameOpenError::FrameOutOfRange);
    }
    let signed = pixel_representation(&metadata)?;
    let photometric = photometric_of(&metadata)?;
    let colour = colour_of(photometric);
    let modality = modality_of(&multi, frame_index, signed)?;
    let window = window_of(&multi, frame_index, signed)?;
    let presentation = presentation_of(&metadata)?;
    let padding = padding_of(&metadata, photometric, signed)?;
    let lossy = lossy_of(&metadata)?;
    let burned_in = burned_in_of(&metadata)?;
    let private_creators = private_creators_of(&metadata)?;
    Ok(OpenedFrame {
        modality,
        window,
        photometric,
        presentation,
        colour,
        padding,
        lossy,
        burned_in,
        private_creators,
    })
}

/// The three Palette Color Lookup Tables of a `PALETTE COLOR` frame.
///
/// F-239. PS3.3 C.7.9 and C.7.6.3.1.5, read from the top level:
/// `(0028,1101)` to `(0028,1103)` give each channel's entry count (0 means
/// 65,536), first stored value mapped and bits per entry, and `(0028,1201)` to
/// `(0028,1203)` hold the entries. The first stored value mapped is signed
/// when Pixel Representation is 1, as the descriptor's VR follows it.
///
/// An 8-bit table is "stored in a format equivalent to 8 bits allocated", one
/// byte per entry with OW's pad byte after an odd count. C.7.6.3.1.5's note
/// says some writers store 8-bit entries in 16 bits allocated instead and that
/// the value length tells the two apart, so a value of two bytes per entry is
/// read as words. Every other depth is one word per entry.
///
/// The scaling to the display's 8 bits is [`PaletteColorLut::with_entry_bits`]'s,
/// given each channel's declared depth.
///
/// # Errors
///
/// `Ok(None)` for any photometric interpretation other than `PALETTE COLOR`.
/// [`FrameOpenError::Photometric`] when it is missing or unknown.
/// [`FrameOpenError::SegmentedPalette`] when any of `(0028,1221)` to
/// `(0028,1223)` is present. [`FrameOpenError::MissingPalette`] when a
/// descriptor or a table is absent or empty. [`FrameOpenError::PaletteDepth`]
/// for bits per entry outside `8 ..= 16`. [`FrameOpenError::Attribute`] naming
/// the descriptor or table that cannot be read, including a table whose value
/// length matches neither container and an entry beyond its declared depth.
pub fn palette_color_lut(
    metadata: &MetadataSet,
) -> Result<Option<PaletteColorLut>, FrameOpenError> {
    if photometric_of(metadata)? != PhotometricInterpretation::PaletteColor {
        return Ok(None);
    }
    if SEGMENTED_PALETTE_DATA
        .iter()
        .any(|tag| metadata.get(*tag).is_some())
    {
        return Err(FrameOpenError::SegmentedPalette);
    }
    let signed = pixel_representation(metadata)?;
    let (red, red_bits) =
        palette_channel(metadata, PALETTE_DESCRIPTORS[0], PALETTE_DATA[0], signed)?;
    let (green, green_bits) =
        palette_channel(metadata, PALETTE_DESCRIPTORS[1], PALETTE_DATA[1], signed)?;
    let (blue, blue_bits) =
        palette_channel(metadata, PALETTE_DESCRIPTORS[2], PALETTE_DATA[2], signed)?;
    // Every per-channel refusal happened in `palette_channel`, where the
    // channel's own tags are known. What is left is the three channels
    // disagreeing, which names the first descriptor that differs from red's.
    let disagrees = |lut: &LutDescriptor| {
        lut.entry_count() != red.entry_count()
            || lut.first_mapped_input() != red.first_mapped_input()
    };
    let culprit = if disagrees(&green) {
        PALETTE_DESCRIPTORS[1]
    } else {
        PALETTE_DESCRIPTORS[2]
    };
    PaletteColorLut::with_entry_bits(red, green, blue, [red_bits, green_bits, blue_bits])
        .map(Some)
        .map_err(|_| attribute(culprit))
}

/// Whether ICC Profile `(0028,2000)` is present at the top level.
///
/// Deviation D-30. The presence test is the element alone, whatever its
/// length, because the claim it supports is "this file asked for colour
/// management and did not get it". An ICC Profile inside the Optical Path
/// Sequence is a whole-slide concern and is not this test's.
#[must_use]
pub fn has_icc_profile(metadata: &MetadataSet) -> bool {
    metadata.get(ICC_PROFILE).is_some()
}

/// The bytes of ICC Profile `(0028,2000)`, present at the top level.
///
/// F-240. Beside [`has_icc_profile`], which answers D-30's presence question
/// for a frame that has no profile route yet. This returns the element's
/// bytes for the caller that parses them, and `None` when the element is
/// absent, empty, or not a byte payload: the standard's VR for the attribute
/// is OB, so anything projected as words is not a profile and reads `None`
/// rather than being re-encoded. A profile only inside the Optical Path
/// Sequence is a whole-slide concern and reads `None` here too.
#[must_use]
pub fn icc_profile(metadata: &MetadataSet) -> Option<&[u8]> {
    match metadata.get(ICC_PROFILE)?.value() {
        MetadataValue::Bytes(bytes) => Some(bytes),
        _ => None,
    }
}

/// One palette channel and its declared bits per entry.
fn palette_channel(
    metadata: &MetadataSet,
    descriptor_tag: Tag,
    data_tag: Tag,
    signed: bool,
) -> Result<(LutDescriptor, u16), FrameOpenError> {
    let descriptor = metadata
        .get(descriptor_tag)
        .ok_or(FrameOpenError::MissingPalette)?;
    let data = metadata
        .get(data_tag)
        .ok_or(FrameOpenError::MissingPalette)?;
    let [entry_count, first_raw, bits] =
        descriptor_words(descriptor).map_err(|_| attribute(descriptor_tag))?;
    if !(8..=16).contains(&bits) {
        return Err(FrameOpenError::PaletteDepth);
    }
    let entries = if entry_count == 0 {
        65_536
    } else {
        usize::from(entry_count)
    };
    let bytes = palette_bytes(data, data_tag)?;
    let (values, container) = palette_entries(&bytes, entries, bits).ok_or(attribute(data_tag))?;
    // An entry beyond the declared depth is refused here, naming this table.
    // `bits` is in `8 ..= 16`, so `2^bits - 1` is a `u16` and exact in `f32`.
    let largest = f32::from(u16::MAX >> (16 - bits));
    if values.iter().any(|value| *value > largest) {
        return Err(attribute(data_tag));
    }
    let first = stored_from_u16(first_raw, signed);
    let lut = LutDescriptor::new(entry_count, first, container, values)
        .map_err(|_| attribute(data_tag))?;
    Ok((lut, bits))
}

/// The table's value as the little-endian byte stream PS3.5 encodes OW in.
///
/// A parser that projected OW as words gave each word its value, so writing
/// each back little-endian recovers the bytes an 8-bit table packs.
fn palette_bytes(data: &MetadataElement, tag: Tag) -> Result<Vec<u8>, FrameOpenError> {
    match data.value() {
        MetadataValue::Empty => Err(FrameOpenError::MissingPalette),
        MetadataValue::Bytes(bytes) => Ok(bytes.clone()),
        MetadataValue::Unsigned16(words) => {
            Ok(words.iter().flat_map(|word| word.to_le_bytes()).collect())
        }
        _ => Err(attribute(tag)),
    }
}

/// Entries and the container [`LutDescriptor`] holds them in, or `None` when
/// the value length matches neither PS3.3 C.7.6.3.1.5 form.
fn palette_entries(bytes: &[u8], entries: usize, bits: u16) -> Option<(Vec<f32>, u8)> {
    let words = entries.checked_mul(2)?;
    let one_byte_each = bytes.len() == entries || (entries % 2 == 1 && bytes.len() == entries + 1);
    if bits == 8 && one_byte_each && bytes.len() != words {
        let values = bytes.iter().take(entries).copied().map(f32::from).collect();
        return Some((values, 8));
    }
    if bytes.len() != words {
        return None;
    }
    let values = bytes
        .chunks_exact(2)
        .map(|pair| match pair {
            [low, high] => f32::from(u16::from_le_bytes([*low, *high])),
            _ => 0.0,
        })
        .collect();
    // A word is held in a 16-bit container whatever the declared depth. An
    // entry beyond that depth, a word above 255 in an 8-bit table among them,
    // is refused by `palette_channel` naming this table, the one check.
    Some((values, 16))
}

/// Replace padding samples with the black display end of `chain`.
///
/// [`LutChain::inverts`] is the only inversion decision. `MONOCHROME1` black
/// is `ymin` when that flag is set and `ymax` when it is not. `MONOCHROME2`
/// black is the other end. Colour and palette samples use `ymin`. The range
/// is [`LutChain::voi`]'s output range, the same range stage 3 was built
/// with. The write is a constant. It is not a window function.
///
/// # Errors
///
/// [`FrameOpenError::MaskLength`] when the slices differ. Nothing is written
/// in that case.
pub fn fill_padding(
    display: &mut [Display],
    mask: &[bool],
    photometric: PhotometricInterpretation,
    chain: &LutChain,
) -> Result<(), FrameOpenError> {
    if display.len() != mask.len() {
        return Err(FrameOpenError::MaskLength);
    }
    let black = padding_black(photometric, chain);
    for (sample, padded) in display.iter_mut().zip(mask.iter().copied()) {
        if padded {
            *sample = Display(black);
        }
    }
    Ok(())
}

/// Black from the chain's resolved inversion, not a second presentation rule.
fn padding_black(photometric: PhotometricInterpretation, chain: &LutChain) -> f32 {
    let (ymin, ymax) = chain.voi().output_range();
    let monochrome1 = photometric == PhotometricInterpretation::Monochrome1;
    if !monochrome1 && photometric != PhotometricInterpretation::Monochrome2 {
        return ymin;
    }
    // MONOCHROME1 black starts at ymax and is ymin after inversion.
    // MONOCHROME2 black starts at ymin and is ymax after inversion.
    if monochrome1 == chain.inverts() {
        ymin
    } else {
        ymax
    }
}

fn map_multi(error: MultiframeError) -> FrameOpenError {
    match error {
        MultiframeError::FrameOutOfRange { .. } => FrameOpenError::FrameOutOfRange,
        MultiframeError::InvalidFunctionalGroup { tag, .. } => attribute(tag),
        MultiframeError::InvalidNumberOfFramesVr(_)
        | MultiframeError::EmptyNumberOfFrames
        | MultiframeError::MultipleNumberOfFrames(_)
        | MultiframeError::InvalidNumberOfFrames
        | MultiframeError::ZeroNumberOfFrames
        | MultiframeError::NumberOfFramesOverflow
        | MultiframeError::InvalidSharedFunctionalGroups
        | MultiframeError::SharedFunctionalGroupsCount(_)
        | MultiframeError::InvalidPerFrameFunctionalGroups
        | MultiframeError::PerFrameFunctionalGroupsCount { .. } => FrameOpenError::Attribute {
            group: 0x5200,
            element: 0x9230,
        },
    }
}

fn attribute(tag: Tag) -> FrameOpenError {
    FrameOpenError::Attribute {
        group: tag.0,
        element: tag.1,
    }
}

fn resolved<'a>(
    multi: &'a MultiframeMetadata<'a>,
    frame: usize,
    group: Tag,
    attribute_tag: Tag,
) -> Result<Option<&'a MetadataElement>, FrameOpenError> {
    match multi.resolve(frame, group, attribute_tag, TopLevelFallback::Allowed) {
        Ok(value) => Ok(value.map(|resolved| resolved.element())),
        Err(error) => Err(map_multi(error)),
    }
}

fn photometric_of(metadata: &MetadataSet) -> Result<PhotometricInterpretation, FrameOpenError> {
    let Some(element) = metadata.get(PHOTOMETRIC) else {
        return Err(FrameOpenError::Photometric);
    };
    let Some(code) = one_code(element, PHOTOMETRIC)? else {
        return Err(FrameOpenError::Photometric);
    };
    match code {
        "MONOCHROME1" => Ok(PhotometricInterpretation::Monochrome1),
        "MONOCHROME2" => Ok(PhotometricInterpretation::Monochrome2),
        "PALETTE COLOR" => Ok(PhotometricInterpretation::PaletteColor),
        "RGB" => Ok(PhotometricInterpretation::Rgb),
        "YBR_FULL" => Ok(PhotometricInterpretation::YbrFull),
        "YBR_FULL_422" => Ok(PhotometricInterpretation::YbrFull422),
        "YBR_PARTIAL_422" => Ok(PhotometricInterpretation::YbrPartial422),
        "YBR_ICT" => Ok(PhotometricInterpretation::YbrIct),
        "YBR_RCT" => Ok(PhotometricInterpretation::YbrRct),
        "XYB" => Ok(PhotometricInterpretation::Xyb),
        _ => Err(FrameOpenError::Photometric),
    }
}

fn colour_of(photometric: PhotometricInterpretation) -> ColourSelection {
    match photometric {
        PhotometricInterpretation::Monochrome1 | PhotometricInterpretation::Monochrome2 => {
            ColourSelection::Monochrome
        }
        PhotometricInterpretation::PaletteColor => ColourSelection::Palette,
        PhotometricInterpretation::Rgb
        | PhotometricInterpretation::YbrFull
        | PhotometricInterpretation::YbrFull422
        | PhotometricInterpretation::YbrPartial422
        | PhotometricInterpretation::YbrIct
        | PhotometricInterpretation::YbrRct
        | PhotometricInterpretation::Xyb => ColourSelection::Colour {
            codec_already_rgb: false,
        },
    }
}

fn pixel_representation(metadata: &MetadataSet) -> Result<bool, FrameOpenError> {
    let Some(element) = metadata.get(PIXEL_REPRESENTATION) else {
        return Ok(false);
    };
    let Some(values) = unsigned_list(element, PIXEL_REPRESENTATION)? else {
        return Ok(false);
    };
    match values {
        [0] => Ok(false),
        [1] => Ok(true),
        _ => Err(attribute(PIXEL_REPRESENTATION)),
    }
}

fn modality_of(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    signed: bool,
) -> Result<ModalityTransform, FrameOpenError> {
    let lut = optional_lut(
        multi,
        frame,
        PIXEL_VALUE_TRANSFORMATION,
        MODALITY_LUT_SEQUENCE,
        signed,
    )?;
    let slope = optional_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_SLOPE)?;
    let intercept = optional_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_INTERCEPT)?;
    let slope = one_decimal(slope, RESCALE_SLOPE)?;
    let intercept = one_decimal(intercept, RESCALE_INTERCEPT)?;
    let (slope, intercept) = rescale_or_identity(lut.is_some(), slope, intercept)?;
    ModalityTransform::new(lut, slope, intercept).map_err(map_pixel)
}

/// PS3.3 C.11.1. An absent slope and intercept are the identity. A sequence
/// does not need them. One side of the pair without the other is an error.
fn rescale_or_identity(
    modality_lut: bool,
    slope: Option<f32>,
    intercept: Option<f32>,
) -> Result<(Option<f32>, Option<f32>), FrameOpenError> {
    if modality_lut {
        return Ok((slope, intercept));
    }
    match (slope, intercept) {
        (None, None) => Ok((Some(1.0), Some(0.0))),
        (Some(slope), Some(intercept)) => Ok((Some(slope), Some(intercept))),
        (None, Some(_)) => Err(attribute(RESCALE_SLOPE)),
        (Some(_), None) => Err(attribute(RESCALE_INTERCEPT)),
    }
}

fn window_of(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    signed: bool,
) -> Result<WindowInput, FrameOpenError> {
    let slope = one_decimal(
        optional_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_SLOPE)?,
        RESCALE_SLOPE,
    )?;
    let intercept = one_decimal(
        optional_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_INTERCEPT)?,
        RESCALE_INTERCEPT,
    )?;
    let modality_lut = has_sequence(
        multi,
        frame,
        PIXEL_VALUE_TRANSFORMATION,
        MODALITY_LUT_SEQUENCE,
    )?;
    let second_is_signed = voi_input_is_signed(signed, modality_lut, slope, intercept);
    if let Some(lut) = optional_lut(
        multi,
        frame,
        FRAME_VOI_LUT,
        VOI_LUT_SEQUENCE,
        second_is_signed,
    )? {
        let voi = VoiTransform::new(
            Some(lut),
            &[],
            &[],
            0,
            VoiFunction::Linear,
            DISPLAY_MIN,
            DISPLAY_MAX,
        )
        .map_err(map_pixel)?;
        return Ok(WindowInput::Transform(voi));
    }
    let centers = optional_decimals(multi, frame, FRAME_VOI_LUT, WINDOW_CENTER)?;
    let widths = optional_decimals(multi, frame, FRAME_VOI_LUT, WINDOW_WIDTH)?;
    match (centers, widths) {
        (None, None) => Ok(WindowInput::Absent),
        (Some(centers), Some(widths)) => {
            let function = voi_function(multi, frame)?;
            let voi = VoiTransform::new(
                None,
                &centers,
                &widths,
                0,
                function,
                DISPLAY_MIN,
                DISPLAY_MAX,
            )
            .map_err(map_pixel)?;
            Ok(WindowInput::Transform(voi))
        }
        (Some(_), None) => Err(attribute(WINDOW_WIDTH)),
        (None, Some(_)) => Err(attribute(WINDOW_CENTER)),
    }
}

fn voi_function(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
) -> Result<VoiFunction, FrameOpenError> {
    let Some(element) = resolved(multi, frame, FRAME_VOI_LUT, VOI_LUT_FUNCTION)? else {
        return Ok(VoiFunction::Linear);
    };
    let Some(code) = one_code(element, VOI_LUT_FUNCTION)? else {
        return Ok(VoiFunction::Linear);
    };
    match code {
        "LINEAR" => Ok(VoiFunction::Linear),
        "LINEAR_EXACT" => Ok(VoiFunction::LinearExact),
        "SIGMOID" => Ok(VoiFunction::Sigmoid),
        _ => Err(attribute(VOI_LUT_FUNCTION)),
    }
}

fn presentation_of(metadata: &MetadataSet) -> Result<PresentationLutEvidence, FrameOpenError> {
    if let Some(element) = metadata.get(PRESENTATION_LUT_SEQUENCE)
        && !matches!(element.value(), MetadataValue::Empty)
    {
        return Ok(PresentationLutEvidence::Sequence);
    }
    let Some(element) = metadata.get(PRESENTATION_LUT_SHAPE) else {
        return Ok(PresentationLutEvidence::Absent);
    };
    let Some(code) = one_code(element, PRESENTATION_LUT_SHAPE)? else {
        return Ok(PresentationLutEvidence::Absent);
    };
    match code {
        "IDENTITY" => Ok(PresentationLutEvidence::Shape(
            PresentationLutShape::Identity,
        )),
        "INVERSE" => Ok(PresentationLutEvidence::Shape(
            PresentationLutShape::Inverse,
        )),
        _ => Err(attribute(PRESENTATION_LUT_SHAPE)),
    }
}

/// Read Pixel Padding Value and Pixel Padding Range Limit, and nothing else.
///
/// PS3.3 C.7.5.1.1.2. This is the reader [`open_frame`] uses, for a caller
/// that needs the stored padding range without the window, presentation,
/// lossy, burned-in and private creator attributes that `open_frame` also
/// reads and can refuse. `signed` is Pixel Representation 1.
///
/// # Errors
///
/// [`FrameOpenError::PaddingLimitWithoutValue`] for a limit with no value,
/// [`FrameOpenError::PaddingOrder`] for a range in the wrong order for the
/// photometric interpretation, and an attribute error for a value that is
/// not one US or SS.
pub fn read_padding(
    metadata: &MetadataSet,
    photometric: PhotometricInterpretation,
    signed: bool,
) -> Result<Option<PaddingRange>, FrameOpenError> {
    padding_of(metadata, photometric, signed)
}

fn padding_of(
    metadata: &MetadataSet,
    photometric: PhotometricInterpretation,
    signed: bool,
) -> Result<Option<PaddingRange>, FrameOpenError> {
    let value = match metadata.get(PIXEL_PADDING_VALUE) {
        Some(element) => one_stored(element, PIXEL_PADDING_VALUE, signed)?,
        None => None,
    };
    let limit = match metadata.get(PIXEL_PADDING_RANGE_LIMIT) {
        Some(element) => one_stored(element, PIXEL_PADDING_RANGE_LIMIT, signed)?,
        None => None,
    };
    let Some(value) = value else {
        if limit.is_some() {
            return Err(FrameOpenError::PaddingLimitWithoutValue);
        }
        return Ok(None);
    };
    if let Some(limit) = limit {
        let ordered = match photometric {
            PhotometricInterpretation::Monochrome1 => value >= limit,
            PhotometricInterpretation::Monochrome2
            | PhotometricInterpretation::PaletteColor
            | PhotometricInterpretation::Rgb
            | PhotometricInterpretation::YbrFull
            | PhotometricInterpretation::YbrFull422
            | PhotometricInterpretation::YbrPartial422
            | PhotometricInterpretation::YbrIct
            | PhotometricInterpretation::YbrRct
            | PhotometricInterpretation::Xyb => value <= limit,
        };
        if !ordered {
            return Err(FrameOpenError::PaddingOrder);
        }
    }
    Ok(Some(PaddingRange { value, limit }))
}

fn lossy_of(metadata: &MetadataSet) -> Result<LossyFlag, FrameOpenError> {
    let Some(element) = metadata.get(LOSSY_IMAGE_COMPRESSION) else {
        return Ok(LossyFlag::Absent);
    };
    let Some(code) = one_code(element, LOSSY_IMAGE_COMPRESSION)? else {
        return Ok(LossyFlag::Absent);
    };
    match code {
        "00" => Ok(LossyFlag::Lossless),
        "01" => Ok(LossyFlag::Lossy),
        _ => Err(FrameOpenError::LossyCode),
    }
}

fn burned_in_of(metadata: &MetadataSet) -> Result<BurnedInFlag, FrameOpenError> {
    let Some(element) = metadata.get(BURNED_IN_ANNOTATION) else {
        return Ok(BurnedInFlag::Absent);
    };
    let Some(code) = one_code(element, BURNED_IN_ANNOTATION)? else {
        return Ok(BurnedInFlag::Absent);
    };
    match code {
        "NO" => Ok(BurnedInFlag::No),
        "YES" => Ok(BurnedInFlag::Yes),
        _ => Err(FrameOpenError::BurnedInCode),
    }
}

fn private_creators_of(metadata: &MetadataSet) -> Result<Vec<PrivateCreator>, FrameOpenError> {
    let mut creators = Vec::new();
    for (tag, element) in metadata.iter() {
        if !private_creator_element(tag) {
            continue;
        }
        let MetadataValue::Text(_) = element.value() else {
            continue;
        };
        let Some(text) = one_code(element, tag)? else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        let Ok(slot) = u8::try_from(tag.1) else {
            continue;
        };
        creators.push(PrivateCreator {
            group: tag.0,
            slot,
            creator: text.to_owned(),
        });
    }
    Ok(creators)
}

/// PS3.5 7.8.1. Groups 0001, 0003, 0005, 0007 and FFFF are not private slots.
fn private_creator_element(tag: Tag) -> bool {
    if !odd_private_group(tag.0) {
        return false;
    }
    (0x0010..=0x00FF).contains(&tag.1)
}

fn private_data_element(group: u16, element: u16) -> bool {
    if !odd_private_group(group) || element < 0x1000 {
        return false;
    }
    let slot = element >> 8;
    (0x10..=0xFF).contains(&slot)
}

fn odd_private_group(group: u16) -> bool {
    group % 2 == 1
        && group != 0x0001
        && group != 0x0003
        && group != 0x0005
        && group != 0x0007
        && group != 0xFFFF
}

/// PS3.3 C.11.2.1.1. The second descriptor value is signed when the VOI input
/// may be negative. A Modality LUT makes that value unsigned. Absent rescale
/// follows Pixel Representation. Specified rescale is signed when the stored
/// pixels are signed or either rescale term is negative.
fn voi_input_is_signed(
    pixel_signed: bool,
    modality_lut: bool,
    slope: Option<f32>,
    intercept: Option<f32>,
) -> bool {
    if modality_lut {
        return false;
    }
    if slope.is_none() && intercept.is_none() {
        return pixel_signed;
    }
    pixel_signed
        || slope.is_some_and(|value| value < 0.0)
        || intercept.is_some_and(|value| value < 0.0)
}

fn optional_lut(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    sequence_tag: Tag,
    signed: bool,
) -> Result<Option<LutDescriptor>, FrameOpenError> {
    let Some(element) = resolved(multi, frame, group, sequence_tag)? else {
        return Ok(None);
    };
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Sequence(items) => {
            let Some(item) = items.first() else {
                return Err(attribute(sequence_tag));
            };
            let descriptor = item
                .get(LUT_DESCRIPTOR)
                .ok_or_else(|| attribute(LUT_DESCRIPTOR))?;
            let data = item.get(LUT_DATA).ok_or_else(|| attribute(LUT_DATA))?;
            Ok(Some(lut_descriptor(descriptor, data, signed)?))
        }
        _ => Err(attribute(sequence_tag)),
    }
}

fn has_sequence(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    sequence_tag: Tag,
) -> Result<bool, FrameOpenError> {
    let Some(element) = resolved(multi, frame, group, sequence_tag)? else {
        return Ok(false);
    };
    match element.value() {
        MetadataValue::Empty => Ok(false),
        MetadataValue::Sequence(items) if items.is_empty() => Err(attribute(sequence_tag)),
        MetadataValue::Sequence(_) => Ok(true),
        _ => Err(attribute(sequence_tag)),
    }
}

fn lut_descriptor(
    descriptor: &MetadataElement,
    data: &MetadataElement,
    second_is_signed: bool,
) -> Result<LutDescriptor, FrameOpenError> {
    let [entry_count, first_raw, bits_raw] = descriptor_words(descriptor)?;
    let values = unsigned_list(data, LUT_DATA)?.ok_or_else(|| attribute(LUT_DATA))?;
    let mut mapped = Vec::with_capacity(values.len());
    for value in values {
        mapped.push(f32::from(*value));
    }
    let bits = u8::try_from(bits_raw).map_err(|_| attribute(LUT_DESCRIPTOR))?;
    // PS3.3 C.11.1.1.1 and C.11.2.1.1. The first and third values stay unsigned
    // even when Explicit VR stores the whole descriptor as SS.
    let first = stored_from_u16(first_raw, second_is_signed);
    LutDescriptor::new(entry_count, first, bits, mapped).map_err(map_pixel)
}

fn descriptor_words(element: &MetadataElement) -> Result<[u16; 3], FrameOpenError> {
    match element.value() {
        MetadataValue::Unsigned16(values) => three_words(values),
        MetadataValue::Signed16(values) => match values.as_slice() {
            [entry_count, first_raw, bits_raw] => Ok([
                u16::from_le_bytes(entry_count.to_le_bytes()),
                u16::from_le_bytes(first_raw.to_le_bytes()),
                u16::from_le_bytes(bits_raw.to_le_bytes()),
            ]),
            _ => Err(attribute(LUT_DESCRIPTOR)),
        },
        _ => Err(attribute(LUT_DESCRIPTOR)),
    }
}

fn three_words(words: &[u16]) -> Result<[u16; 3], FrameOpenError> {
    match words {
        [entry_count, first_raw, bits_raw] => Ok([*entry_count, *first_raw, *bits_raw]),
        _ => Err(attribute(LUT_DESCRIPTOR)),
    }
}

fn optional_decimals(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    tag: Tag,
) -> Result<Option<Vec<f32>>, FrameOpenError> {
    let Some(element) = resolved(multi, frame, group, tag)? else {
        return Ok(None);
    };
    decimal_list(element, tag)
}

fn one_decimal(values: Option<Vec<f32>>, tag: Tag) -> Result<Option<f32>, FrameOpenError> {
    match values {
        None => Ok(None),
        Some(values) => match values.as_slice() {
            [value] => Ok(Some(*value)),
            _ => Err(attribute(tag)),
        },
    }
}

fn decimal_list(element: &MetadataElement, tag: Tag) -> Result<Option<Vec<f32>>, FrameOpenError> {
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Text(_) => {
            let texts = element.semantic_text().ok_or_else(|| attribute(tag))?;
            if texts.iter().all(|text| text.is_empty()) {
                return Ok(None);
            }
            let mut values = Vec::with_capacity(texts.len());
            for text in texts {
                if text.is_empty() {
                    return Err(attribute(tag));
                }
                let value = text.parse::<f32>().map_err(|_| attribute(tag))?;
                if !value.is_finite() {
                    return Err(attribute(tag));
                }
                values.push(value);
            }
            Ok(Some(values))
        }
        _ => Err(attribute(tag)),
    }
}

fn one_code(element: &MetadataElement, tag: Tag) -> Result<Option<&str>, FrameOpenError> {
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Text(_) => {
            let texts = element.semantic_text().ok_or_else(|| attribute(tag))?;
            match texts.as_slice() {
                [] | [""] => Ok(None),
                [text] => Ok(Some(*text)),
                _ => Err(attribute(tag)),
            }
        }
        _ => Err(attribute(tag)),
    }
}

fn unsigned_list(element: &MetadataElement, tag: Tag) -> Result<Option<&[u16]>, FrameOpenError> {
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Unsigned16(values) => Ok(Some(values)),
        _ => Err(attribute(tag)),
    }
}

fn one_stored(
    element: &MetadataElement,
    tag: Tag,
    signed: bool,
) -> Result<Option<i32>, FrameOpenError> {
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Unsigned16(values) => match values.as_slice() {
            [value] => Ok(Some(stored_from_u16(*value, signed))),
            _ => Err(attribute(tag)),
        },
        MetadataValue::Signed16(values) => match values.as_slice() {
            [value] => Ok(Some(i32::from(*value))),
            _ => Err(attribute(tag)),
        },
        _ => Err(attribute(tag)),
    }
}

fn stored_from_u16(value: u16, signed: bool) -> i32 {
    if signed {
        i32::from(i16::from_le_bytes(value.to_le_bytes()))
    } else {
        i32::from(value)
    }
}

fn map_pixel(_error: PixelError) -> FrameOpenError {
    FrameOpenError::Chain
}

/// Project every element except Pixel Data.
///
/// `MetadataSet::from_object` keeps an explicit OW payload and refuses an
/// encapsulated Pixel Sequence. Frame open skips Pixel Data and the float
/// pixel data elements, so those bytes are not copied into this projection.
/// Series assembly uses the same projection.
pub(crate) fn project_metadata(parsed: &ParsedDicom) -> Result<MetadataSet, FrameOpenError> {
    project_elements(parsed.object().iter())
}

fn project_item(
    item: &InMemDicomObject<StandardDataDictionary>,
) -> Result<MetadataSet, FrameOpenError> {
    project_elements(item.iter())
}

fn project_elements<'a>(
    elements: impl Iterator<Item = &'a InMemElement<StandardDataDictionary>>,
) -> Result<MetadataSet, FrameOpenError> {
    let mut metadata = MetadataSet::new();
    for element in elements {
        let tag = element.header().tag;
        if tag == PIXEL_DATA || tag == FLOAT_PIXEL_DATA || tag == DOUBLE_FLOAT_PIXEL_DATA {
            continue;
        }
        let projected = match element.value() {
            DicomValue::Primitive(value) => MetadataElement::from_primitive(element.vr(), value),
            DicomValue::Sequence(sequence) => {
                let mut items = Vec::new();
                for item in sequence.items() {
                    items.push(project_item(item)?);
                }
                MetadataElement::sequence(items)
            }
            DicomValue::PixelSequence(_) => continue,
        };
        metadata
            .insert(tag, projected)
            .map_err(|_| FrameOpenError::Metadata)?;
    }
    Ok(metadata)
}
