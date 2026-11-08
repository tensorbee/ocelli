//! Colour preparation and bounded conversion for the local process.

use std::fmt;

use ocelli_core::Stored;
use ocelli_dicom::{FrameOpenError, MetadataSet, has_icc_profile, icc_profile, palette_color_lut};
use ocelli_pixel::{
    ColorTransform, ColourWindow, DecodedLayout, DecodedPhotometric, DisplayScale, IccError,
    IccProfile, IccTransform, ImageDimensions, PixelDataEncoding, PixelRepresentation,
    PlanarConfiguration, Rgb, StoredPixelDescription,
};
use ocelli_render::{RasterColor, colour_bytes_into};

use crate::preview::{FrameRequest, HeldFrame, PreviewError, Recipe};

/// A closed reason for refusing a colour shape, without copying file values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourUnavailableReason {
    /// No supported stage owns this colour space.
    Space,
    /// Colour samples must be unsigned and have 8 to 16 stored bits.
    Samples,
    /// A palette descriptor must declare 8 to 16 bits.
    PaletteDepth,
    /// Segmented palette data is not implemented.
    SegmentedPalette,
    /// A palette table is missing or cannot be read.
    Palette,
    /// A native chroma pair needs an even number of columns.
    ChromaColumns,
    /// This build does not decode the codestream's component shape.
    Codestream,
}

impl fmt::Display for ColourUnavailableReason {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Space => "this image has a colour space this build cannot draw",
            Self::Samples => {
                "this preview draws only unsigned colour samples with 8 to 16 stored bits"
            }
            Self::PaletteDepth => {
                "this preview draws palette entries with 8 to 16 declared bits only"
            }
            Self::SegmentedPalette => "this preview does not draw segmented palette tables",
            Self::Palette => "this image has a missing or unusable palette table",
            Self::ChromaColumns => "a native 4:2:2 colour frame needs an even number of columns",
            Self::Codestream => "this build does not decode this colour codestream shape",
        })
    }
}

/// A profile's disposition is decided from its bytes, never from view state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IccStatus {
    Absent,
    Applied,
    NotApplied(&'static str),
}

impl IccStatus {
    pub(crate) fn present(self) -> bool {
        self != Self::Absent
    }
    pub(crate) fn applied(self) -> bool {
        self == Self::Applied
    }
    pub(crate) fn kind(self) -> &'static str {
        match self {
            Self::NotApplied(kind) => kind,
            _ => "",
        }
    }
}

/// The same classification supplies scan JSON and the held recipe. Only the
/// held path attaches and tabulates the transform, once before admission.
pub(crate) fn read_profile(metadata: &MetadataSet) -> (IccStatus, Option<IccProfile>) {
    if !has_icc_profile(metadata) {
        return (IccStatus::Absent, None);
    }
    let Some(bytes) = icc_profile(metadata) else {
        return (IccStatus::NotApplied("malformed"), None);
    };
    match IccProfile::parse(bytes) {
        Ok(profile) => (IccStatus::Applied, Some(profile)),
        Err(error) => {
            let kind = match error {
                IccError::LookupTableProfile => "lookup-table",
                IccError::UnsupportedClass => match bytes.get(12..16) {
                    Some(b"prtr") => "printer",
                    Some(b"link") => "device-link",
                    Some(b"spac") => "colour-space",
                    Some(b"abst") => "abstract",
                    Some(b"nmcl") => "named-colour",
                    _ => "unsupported-class",
                },
                IccError::UnsupportedColourSpace => match bytes.get(16..20) {
                    Some(b"GRAY") => "greyscale",
                    _ => "unsupported-colour-space",
                },
                IccError::UnsupportedPcs => match bytes.get(20..24) {
                    Some(b"Lab ") => "Lab PCS",
                    _ => "unsupported-PCS",
                },
                IccError::MissingTag => "incomplete",
                IccError::UnsupportedCurve => "unsupported-curve",
                IccError::Malformed => "malformed",
                IccError::TooLarge => "oversized",
            };
            (IccStatus::NotApplied(kind), None)
        }
    }
}

pub(crate) struct ColourRecipe {
    pub(crate) transform: ColorTransform,
    pub(crate) icc: IccStatus,
    pub(crate) palette_fallback: bool,
}

impl ColourRecipe {
    pub(crate) fn new(
        metadata: &MetadataSet,
        description: StoredPixelDescription,
        encoding: PixelDataEncoding,
        photometric: DecodedPhotometric,
        layout: DecodedLayout,
    ) -> Result<Self, PreviewError> {
        let bits = description.bits;
        if bits.pixel_representation() != PixelRepresentation::Unsigned
            || !(8..=16).contains(&bits.bits_stored())
        {
            return Err(PreviewError::ColourUnavailable(
                ColourUnavailableReason::Samples,
            ));
        }
        let palette = palette_color_lut(metadata).map_err(|error| {
            PreviewError::ColourUnavailable(match error {
                FrameOpenError::PaletteDepth => ColourUnavailableReason::PaletteDepth,
                FrameOpenError::SegmentedPalette => ColourUnavailableReason::SegmentedPalette,
                _ => ColourUnavailableReason::Palette,
            })
        })?;
        let palette_fallback = palette
            .as_ref()
            .is_some_and(|lut| lut.entries_read_as_8_bit());
        let mut transform =
            ColorTransform::resolve(description.layout, encoding, photometric, layout, palette)
                .map_err(|_| PreviewError::ColourUnavailable(ColourUnavailableReason::Space))?;
        if transform.palette().is_none() {
            transform = transform
                .with_sample_scale(
                    DisplayScale::from_bits(u16::from(bits.bits_stored())).map_err(|_| {
                        PreviewError::ColourUnavailable(ColourUnavailableReason::Samples)
                    })?,
                )
                .map_err(|_| PreviewError::ColourUnavailable(ColourUnavailableReason::Samples))?;
        }
        let (icc, profile) = read_profile(metadata);
        if let Some(profile) = profile {
            transform = transform.with_icc(IccTransform::from(profile));
        }
        Ok(Self {
            transform,
            icc,
            palette_fallback,
        })
    }

    pub(crate) fn heap_bytes(&self) -> usize {
        size_of::<Self>().saturating_add(self.transform.heap_bytes())
    }
}

/// A conversion band is whole rows and no larger than this many pixels.
pub(crate) const BAND_PIXELS: usize = 65_536;

/// Reusable conversion storage, shared by render, pointer and ROI paths.
#[derive(Default)]
pub(crate) struct ColourScratch {
    pub(crate) stored: Vec<Stored>,
    pub(crate) rgb: Vec<Rgb>,
    pub(crate) packed: Vec<u8>,
}

impl ColourScratch {
    pub(crate) fn heap_bytes(&self) -> usize {
        self.stored
            .capacity()
            .saturating_mul(size_of::<Stored>())
            .saturating_add(self.rgb.capacity().saturating_mul(size_of::<Rgb>()))
            .saturating_add(self.packed.capacity())
    }
}

pub(crate) fn rows_per_band(width: u32) -> Result<u32, PreviewError> {
    let width = usize::try_from(width).map_err(|_| PreviewError::TooLarge)?;
    if width == 0 {
        return Err(PreviewError::Attribute);
    }
    u32::try_from((BAND_PIXELS / width).max(1)).map_err(|_| PreviewError::TooLarge)
}

/// Unpack whole rows through the library's container rules. Planar source
/// planes are gathered into one planar band, so the transform's offsets remain
/// relative to the band rather than the full image.
pub(crate) fn unpack_band(
    held: &HeldFrame,
    first_row: u32,
    rows: u32,
    scratch: &mut ColourScratch,
) -> Result<(), PreviewError> {
    let last = first_row.checked_add(rows).ok_or(PreviewError::Point)?;
    if rows == 0 || last > held.height {
        return Err(PreviewError::Point);
    }
    let dimensions = ImageDimensions::new(rows, held.width).map_err(|_| PreviewError::Attribute)?;
    let band =
        StoredPixelDescription::new(dimensions, held.description.layout, held.description.bits);
    let count = band.sample_count().map_err(|_| PreviewError::Decode)?;
    scratch.stored.clear();
    scratch
        .stored
        .try_reserve_exact(count)
        .map_err(|_| PreviewError::TooLarge)?;
    scratch.stored.resize(count, Stored(0.0));
    let columns = usize::try_from(held.width).map_err(|_| PreviewError::TooLarge)?;
    let first = usize::try_from(first_row).map_err(|_| PreviewError::TooLarge)?;
    let row_count = usize::try_from(rows).map_err(|_| PreviewError::TooLarge)?;
    let bytes_per = usize::from(held.description.bits.bits_allocated() / 8);
    if held.description.layout.planar_configuration() == Some(PlanarConfiguration::Planar) {
        let plane = columns
            .checked_mul(usize::try_from(held.height).map_err(|_| PreviewError::TooLarge)?)
            .and_then(|n| n.checked_mul(bytes_per))
            .ok_or(PreviewError::TooLarge)?;
        let start = first
            .checked_mul(columns)
            .and_then(|n| n.checked_mul(bytes_per))
            .ok_or(PreviewError::TooLarge)?;
        let len = row_count
            .checked_mul(columns)
            .and_then(|n| n.checked_mul(bytes_per))
            .ok_or(PreviewError::TooLarge)?;
        scratch.packed.clear();
        scratch
            .packed
            .try_reserve_exact(len.checked_mul(3).ok_or(PreviewError::TooLarge)?)
            .map_err(|_| PreviewError::TooLarge)?;
        for channel in 0_usize..3 {
            let at = channel
                .checked_mul(plane)
                .and_then(|n| n.checked_add(start))
                .ok_or(PreviewError::TooLarge)?;
            let span = held
                .samples
                .get(at..at.checked_add(len).ok_or(PreviewError::TooLarge)?)
                .ok_or(PreviewError::Decode)?;
            scratch.packed.extend_from_slice(span);
        }
        band.unpack(&scratch.packed, held.byte_order, &mut scratch.stored)
            .map_err(|_| PreviewError::Decode)
    } else {
        let per_row = columns
            .checked_mul(usize::from(
                held.description.layout.stored_samples_per_pixel(),
            ))
            .and_then(|n| n.checked_mul(bytes_per))
            .ok_or(PreviewError::TooLarge)?;
        let start = first.checked_mul(per_row).ok_or(PreviewError::TooLarge)?;
        let len = row_count
            .checked_mul(per_row)
            .ok_or(PreviewError::TooLarge)?;
        let span = held
            .samples
            .get(start..start.checked_add(len).ok_or(PreviewError::TooLarge)?)
            .ok_or(PreviewError::Decode)?;
        band.unpack(span, held.byte_order, &mut scratch.stored)
            .map_err(|_| PreviewError::Decode)
    }
}

/// Stage-4 values before window and byte rounding, used by both readout paths.
pub(crate) fn map_band(
    held: &HeldFrame,
    recipe: &ColourRecipe,
    first_row: u32,
    rows: u32,
    scratch: &mut ColourScratch,
) -> Result<(), PreviewError> {
    unpack_band(held, first_row, rows, scratch)?;
    let pixels = usize::try_from(rows)
        .ok()
        .and_then(|n| n.checked_mul(usize::try_from(held.width).ok()?))
        .ok_or(PreviewError::TooLarge)?;
    if scratch.rgb.len() < pixels {
        scratch
            .rgb
            .try_reserve_exact(pixels - scratch.rgb.len())
            .map_err(|_| PreviewError::TooLarge)?;
        scratch.rgb.resize(pixels, Rgb::BLACK);
    }
    recipe
        .transform
        .map_into(
            &scratch.stored,
            scratch.rgb.get_mut(..pixels).ok_or(PreviewError::Decode)?,
            usize::try_from(held.width).map_err(|_| PreviewError::TooLarge)?,
        )
        .map_err(|_| PreviewError::ColourUnavailable(ColourUnavailableReason::Space))
}

pub(crate) fn draw(
    held: &HeldFrame,
    recipe: &ColourRecipe,
    request: FrameRequest,
    scratch: &mut ColourScratch,
    out: &mut Vec<u8>,
) -> Result<(f32, f32), PreviewError> {
    if request.window_preset.is_some()
        || request.window_index.is_some()
        || request.series_window.is_some()
        || request
            .window_function
            .is_some_and(|function| function != ocelli_pixel::VoiFunction::Linear)
    {
        return Err(PreviewError::ColourNotApplicable);
    }
    let center = request.window_center.unwrap_or(128.0);
    let width = request.window_width.unwrap_or(256.0);
    let window =
        ColourWindow::new(center, width, request.invert).map_err(|_| PreviewError::Window)?;
    let len = usize::try_from(held.width)
        .ok()
        .and_then(|n| n.checked_mul(usize::try_from(held.height).ok()?))
        .and_then(|n| n.checked_mul(3))
        .ok_or(PreviewError::TooLarge)?;
    out.clear();
    out.try_reserve_exact(len)
        .map_err(|_| PreviewError::TooLarge)?;
    out.resize(len, 0);
    let step = rows_per_band(held.width)?;
    let stride = usize::try_from(held.width)
        .map_err(|_| PreviewError::TooLarge)?
        .checked_mul(3)
        .ok_or(PreviewError::TooLarge)?;
    // The same immutable transform supplies every band of this draw.
    let color = RasterColor::supported(&recipe.transform)
        .map_err(|_| PreviewError::ColourUnavailable(ColourUnavailableReason::Space))?;
    let mut row = 0;
    while row < held.height {
        let rows = step.min(held.height - row);
        unpack_band(held, row, rows, scratch)?;
        let begin = usize::try_from(row)
            .map_err(|_| PreviewError::TooLarge)?
            .checked_mul(stride)
            .ok_or(PreviewError::TooLarge)?;
        let end = begin
            .checked_add(
                usize::try_from(rows)
                    .map_err(|_| PreviewError::TooLarge)?
                    .checked_mul(stride)
                    .ok_or(PreviewError::TooLarge)?,
            )
            .ok_or(PreviewError::TooLarge)?;
        colour_bytes_into(
            color,
            &window,
            &scratch.stored,
            held.width,
            &mut scratch.rgb,
            out.get_mut(begin..end).ok_or(PreviewError::Decode)?,
        )
        .map_err(|_| PreviewError::ColourUnavailable(ColourUnavailableReason::Space))?;
        row += rows;
    }
    Ok((center, width))
}

pub(crate) fn recipe(held: &HeldFrame) -> Result<Option<&ColourRecipe>, PreviewError> {
    match held.recipe.as_ref().map_err(|error| *error)? {
        Recipe::Monochrome(_) => Ok(None),
        Recipe::Colour(recipe) => Ok(Some(recipe)),
    }
}

/// Synthetic PS3.3 C.7.6.3.1.5 palette used by the independent review's
/// cache-admission reproduction. No patient attributes or binary fixture.
#[cfg(test)]
pub(crate) fn palette_admission_fixture(with_palette: bool) -> Result<Vec<u8>, String> {
    let short = |tag: (u16, u16), vr: [u8; 2], value: &[u8]| -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        bytes.extend(tag.0.to_le_bytes());
        bytes.extend(tag.1.to_le_bytes());
        bytes.extend(vr);
        bytes.extend(
            u16::try_from(value.len())
                .map_err(|error| error.to_string())?
                .to_le_bytes(),
        );
        bytes.extend(value);
        Ok(bytes)
    };
    let long = |tag: (u16, u16), vr: [u8; 2], value: &[u8]| -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        bytes.extend(tag.0.to_le_bytes());
        bytes.extend(tag.1.to_le_bytes());
        bytes.extend(vr);
        bytes.extend([0, 0]);
        bytes.extend(
            u32::try_from(value.len())
                .map_err(|error| error.to_string())?
                .to_le_bytes(),
        );
        bytes.extend(value);
        Ok(bytes)
    };
    let ui = |value: &str| {
        let mut bytes = value.as_bytes().to_vec();
        if !bytes.len().is_multiple_of(2) {
            bytes.push(0);
        }
        bytes
    };
    let mut meta = long((2, 1), *b"OB", &[0, 1])?;
    for (tag, value) in [
        ((2, 2), "1.2.840.10008.5.1.4.1.1.7"),
        ((2, 3), "2.25.239901"),
        ((2, 0x10), "1.2.840.10008.1.2.1"),
        ((2, 0x12), "2.25.2399"),
    ] {
        meta.extend(short(tag, *b"UI", &ui(value))?);
    }
    let mut out = vec![0; 128];
    out.extend(b"DICM");
    out.extend(short(
        (2, 0),
        *b"UL",
        &u32::try_from(meta.len())
            .map_err(|error| error.to_string())?
            .to_le_bytes(),
    )?);
    out.extend(meta);
    out.extend(short((8, 0x16), *b"UI", &ui("1.2.840.10008.5.1.4.1.1.7"))?);
    out.extend(short((8, 0x18), *b"UI", &ui("2.25.239901"))?);
    for (tag, value) in [
        ((0x28, 2), 1_u16),
        ((0x28, 0x10), 1),
        ((0x28, 0x11), 2),
        ((0x28, 0x100), 8),
        ((0x28, 0x101), 8),
        ((0x28, 0x102), 7),
        ((0x28, 0x103), 0),
    ] {
        out.extend(short(tag, *b"US", &value.to_le_bytes())?);
    }
    out.extend(short((0x28, 4), *b"CS", b"PALETTE COLOR ")?);
    let descriptor = [256_u16, 0, 16]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    out.extend(short((0x28, 0x30), *b"DS", b"1\\1 ")?);
    if with_palette {
        for channel in 0_u16..3 {
            out.extend(short((0x28, 0x1101 + channel), *b"US", &descriptor)?);
            let table = (0_u16..256)
                .flat_map(|index| {
                    match channel {
                        0 => index * 257,
                        1 => (255 - index) * 257,
                        _ => 128 * 257,
                    }
                    .to_le_bytes()
                })
                .collect::<Vec<_>>();
            out.extend(long((0x28, 0x1201 + channel), *b"OW", &table)?);
        }
    }
    out.extend(long((0x7fe0, 0x10), *b"OB", &[0, 255])?);
    Ok(out)
}
