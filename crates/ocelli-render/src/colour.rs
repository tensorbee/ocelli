//! Display-keyed presets and a stored-keyed palette texture, F-044.
//!
//! The preset bytes are the two tables named by the design plan. Palette
//! texels are [`ColorTransform::map_into`], which is [`PaletteColorLut`]'s
//! lookup, copied once. There is no second palette formula.
//!
//! A preset is indexed by the display value `lut_chain` already produced.
//! The index clamp is a property of the table address. It is not applied to
//! the value [`LutChain`] returns. A palette is indexed by the stored sample,
//! per deviation D-23, and the shader does not walk the descriptor.

use bytemuck::{Pod, Zeroable};
use ocelli_core::{Display, Stored};
use ocelli_pixel::{
    ColorTransform, DecodedLayout, DecodedPhotometric, LutChain, PaletteColorLut,
    PhotometricInterpretation, PixelDataEncoding, PlanarConfiguration, Rgb, SampleLayout,
    VoiFunction,
};
use ocelli_viewport::presentation::Colormap;

/// `grey` or `stops`. No clinical name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetId {
    /// Display value `d` in `0..255` maps to RGB `(d, d, d)`.
    Grey,
    /// Texels `(0, 0, 0)`, `(255, 0, 0)` and `(255, 255, 255)` at 0, 128 and 255.
    ///
    /// Every other index is black.
    Stops,
}

/// Why a colour table was not built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ColourLutError {
    /// ICC, a palette on a monochrome frame, a preset on a palette image,
    /// or a palette channel that does not fit an 8-bit texel.
    ///
    /// The value is not passed through as device RGB.
    #[error("the colour LUT is unavailable")]
    Unavailable,
    /// The destination does not have one RGB triple per stored sample.
    #[error("colour destination length does not match the stored samples")]
    Length,
}

/// Shader `colour.mode`. Zero is the greyscale path.
const MODE_PRESET: u32 = 1;
const MODE_PALETTE: u32 = 2;

/// The uniform beside section 18.4. It does not replace those thirty-two bytes.
///
/// `mode` 0 leaves the greyscale normalisation. `mode` 1 indexes a preset by
/// the display value. `mode` 2 indexes a palette by the stored sample.
/// `origin` is the stored value of palette texel 0. Presets use 0.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub(crate) struct ColourParams {
    pub mode: u32,
    pub origin: i32,
    pub last: u32,
    pub pad: u32,
}

impl ColourParams {
    pub(crate) const NONE: Self = Self {
        mode: 0,
        origin: 0,
        last: 0,
        pad: 0,
    };

    pub(crate) fn from_lut(lut: &ColourLut) -> Self {
        // `pad` 1 marks the grey identity table. The fragment shader then
        // uses the 1024 ramp for LINEAR and LINEAR_EXACT. Stops and palette
        // stay on `table_index`.
        let grey = u32::from(lut.preset_id() == Some(PresetId::Grey));
        Self {
            mode: lut.mode(),
            origin: lut.origin(),
            last: lut.last_index(),
            pad: grey,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ColourKind {
    Preset { id: PresetId, bytes: Vec<u8> },
    Palette { bytes: Vec<u8>, origin: i32 },
}

/// A resolved colour table. Either a preset or a palette, never both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourLut {
    kind: ColourKind,
}

impl ColourLut {
    /// Build a preset table.
    ///
    /// # Errors
    ///
    /// [`ColourLutError::Unavailable`] when the frame is palette colour.
    /// A preset is indexed by a display value, and a palette image has none.
    pub fn preset(
        id: PresetId,
        photometric: PhotometricInterpretation,
    ) -> Result<Self, ColourLutError> {
        if photometric == PhotometricInterpretation::PaletteColor {
            return Err(ColourLutError::Unavailable);
        }
        Ok(Self {
            kind: ColourKind::Preset {
                id,
                bytes: preset_bytes(id),
            },
        })
    }

    /// Bake palette texels with [`ColorTransform::map_into`].
    ///
    /// Texel `i` is the lookup of stored value `origin + i`. The shader
    /// subtracts `origin` and does not read the descriptor.
    ///
    /// # Errors
    ///
    /// [`ColourLutError::Unavailable`] when [`ColorTransform::resolve`]
    /// refuses the photometric interpretation, which includes both monochrome
    /// values, or when a channel is not an integer in `0..=255`. A 16-bit
    /// entry above 255 is not scaled into the byte. That scale is not this
    /// story's formula.
    pub fn palette(
        lut: PaletteColorLut,
        photometric: PhotometricInterpretation,
    ) -> Result<Self, ColourLutError> {
        let (origin, end) = address_span(&lut)?;
        let transform = resolve_palette(lut, photometric)?;
        let bytes = bake(&transform, origin, end)?;
        Ok(Self {
            kind: ColourKind::Palette { bytes, origin },
        })
    }

    /// ICC is not implemented.
    ///
    /// # Errors
    ///
    /// Always [`ColourLutError::Unavailable`]. The profile is not returned as
    /// device RGB.
    pub fn icc(_photometric: PhotometricInterpretation) -> Result<Self, ColourLutError> {
        Err(ColourLutError::Unavailable)
    }

    /// The preset id, when this table is one.
    #[must_use]
    pub fn preset_id(&self) -> Option<PresetId> {
        match self.kind {
            ColourKind::Preset { id, .. } => Some(id),
            ColourKind::Palette { .. } => None,
        }
    }

    /// RGBA8 texels, height 1, alpha 255.
    #[must_use]
    pub fn texture_bytes(&self) -> &[u8] {
        match &self.kind {
            ColourKind::Preset { bytes, .. } | ColourKind::Palette { bytes, .. } => bytes,
        }
    }

    pub(crate) fn mode(&self) -> u32 {
        match self.kind {
            ColourKind::Preset { .. } => MODE_PRESET,
            ColourKind::Palette { .. } => MODE_PALETTE,
        }
    }

    pub(crate) fn origin(&self) -> i32 {
        match self.kind {
            ColourKind::Preset { .. } => 0,
            ColourKind::Palette { origin, .. } => origin,
        }
    }

    pub(crate) fn last_index(&self) -> u32 {
        let texels = self.texture_bytes().len() / 4;
        u32::try_from(texels.saturating_sub(1)).unwrap_or(0)
    }

    /// Index this preset after [`LutChain::map_into`].
    ///
    /// The display value is not clamped. Only the table address is. A palette
    /// is [`ColourLutError::Unavailable`] here, because its CPU path is
    /// [`ColorTransform::map_into`].
    ///
    /// # Errors
    ///
    /// [`ColourLutError::Unavailable`] for a palette table.
    /// [`ColourLutError::Length`] when the slices differ in length. Nothing
    /// is written in that case.
    pub fn map_preset_into(
        &self,
        chain: &LutChain,
        stored: &[Stored],
        destination: &mut [Rgb],
    ) -> Result<(), ColourLutError> {
        let bytes = match &self.kind {
            ColourKind::Preset { bytes, .. } => bytes.as_slice(),
            ColourKind::Palette { .. } => return Err(ColourLutError::Unavailable),
        };
        if stored.len() != destination.len() {
            return Err(ColourLutError::Length);
        }
        let last = self.last_index();
        for (sample, output) in stored.iter().copied().zip(destination.iter_mut()) {
            let mut display = [Display(0.0)];
            chain
                .map_into(&[sample], &mut display)
                .map_err(|_| ColourLutError::Length)?;
            let shown = display.first().copied().unwrap_or(Display(0.0));
            let index = preset_index(self, chain, shown.0, last);
            *output = rgb_at(bytes, index);
        }
        Ok(())
    }
}

/// Map [`Colormap`] onto the two preset tables and build that table.
///
/// Palette colour stays [`ColourLutError::Unavailable`], because a preset is
/// indexed by a display value and a palette image has none.
///
/// # Errors
///
/// [`ColourLutError::Unavailable`] when `photometric` is palette colour.
pub fn colour_lut_for(
    colormap: Colormap,
    photometric: PhotometricInterpretation,
) -> Result<ColourLut, ColourLutError> {
    let id = match colormap {
        Colormap::Grey => PresetId::Grey,
        Colormap::Stops => PresetId::Stops,
    };
    ColourLut::preset(id, photometric)
}

fn preset_bytes(id: PresetId) -> Vec<u8> {
    let mut bytes = vec![0_u8; 256 * 4];
    for index in 0..256_u16 {
        let rgb = match id {
            PresetId::Grey => {
                let channel = u8::try_from(index).unwrap_or(0);
                [channel, channel, channel]
            }
            PresetId::Stops => {
                if index == 128 {
                    [255, 0, 0]
                } else if index == 255 {
                    [255, 255, 255]
                } else {
                    [0, 0, 0]
                }
            }
        };
        let start = usize::from(index).saturating_mul(4);
        if let Some(slot) = bytes.get_mut(start..start.saturating_add(4)) {
            slot.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    bytes
}

fn address_span(palette: &PaletteColorLut) -> Result<(i32, i32), ColourLutError> {
    let first = palette.first_mapped_input();
    let count = i32::try_from(palette.entry_count()).map_err(|_| ColourLutError::Unavailable)?;
    let last = first
        .checked_add(count.checked_sub(1).ok_or(ColourLutError::Unavailable)?)
        .ok_or(ColourLutError::Unavailable)?;
    Ok((first.min(0), last.max(0)))
}

fn resolve_palette(
    lut: PaletteColorLut,
    photometric: PhotometricInterpretation,
) -> Result<ColorTransform, ColourLutError> {
    let samples = match photometric {
        PhotometricInterpretation::Monochrome1
        | PhotometricInterpretation::Monochrome2
        | PhotometricInterpretation::PaletteColor => 1,
        PhotometricInterpretation::Rgb
        | PhotometricInterpretation::YbrFull
        | PhotometricInterpretation::YbrFull422
        | PhotometricInterpretation::YbrPartial422
        | PhotometricInterpretation::YbrIct
        | PhotometricInterpretation::YbrRct => 3,
    };
    let planar = if samples == 1 {
        None
    } else {
        Some(PlanarConfiguration::Interleaved)
    };
    let layout =
        SampleLayout::new(samples, photometric, planar).map_err(|_| ColourLutError::Unavailable)?;
    ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        Some(lut),
    )
    .map_err(|_| ColourLutError::Unavailable)
}

fn bake(transform: &ColorTransform, origin: i32, end: i32) -> Result<Vec<u8>, ColourLutError> {
    let span = end.checked_sub(origin).ok_or(ColourLutError::Unavailable)?;
    let width_i = span.checked_add(1).ok_or(ColourLutError::Unavailable)?;
    let width = usize::try_from(width_i).map_err(|_| ColourLutError::Unavailable)?;
    let byte_len = width.checked_mul(4).ok_or(ColourLutError::Unavailable)?;
    let mut bytes = vec![0_u8; byte_len];
    for index in 0..width {
        let offset = i32::try_from(index).map_err(|_| ColourLutError::Unavailable)?;
        let stored = origin
            .checked_add(offset)
            .ok_or(ColourLutError::Unavailable)?;
        let sample = f32_from_i32(stored).ok_or(ColourLutError::Unavailable)?;
        let mut destination = [Rgb::BLACK];
        transform
            .map_into(&[Stored(sample)], &mut destination)
            .map_err(|_| ColourLutError::Unavailable)?;
        let rgb = destination.first().copied().unwrap_or(Rgb::BLACK);
        let channels = [
            byte_from_channel(rgb.r).ok_or(ColourLutError::Unavailable)?,
            byte_from_channel(rgb.g).ok_or(ColourLutError::Unavailable)?,
            byte_from_channel(rgb.b).ok_or(ColourLutError::Unavailable)?,
            255_u8,
        ];
        let start = index.checked_mul(4).ok_or(ColourLutError::Unavailable)?;
        let Some(slot) = bytes.get_mut(start..start.saturating_add(4)) else {
            return Err(ColourLutError::Unavailable);
        };
        slot.copy_from_slice(&channels);
    }
    Ok(bytes)
}

/// Grey `LINEAR` and `LINEAR_EXACT` use the pinned renderer's colour texture.
///
/// cornerstone3D 5.8.2 stores the ramp in a 1024-wide texture. The table
/// samples `i / 1023`, `Uint8ClampedArray` rounds half to even, and the
/// fragment shader reads that texture with NEAREST after `CLAMP_TO_EDGE`.
/// Nearest-byte of the continuous display misses that table by one code on
/// the values the signed-mean bound can see. `SIGMOID` is not this ramp.
/// A VOI LUT sequence is not this ramp. `stops` keeps the 256-entry address.
fn preset_index(lut: &ColourLut, chain: &LutChain, display: f32, last: u32) -> u32 {
    if lut.preset_id() == Some(PresetId::Grey) && linear_byte_ramp(chain) {
        reference_ramp_index(display, chain.inverts())
    } else {
        display_index(display, last)
    }
}

fn linear_byte_ramp(chain: &LutChain) -> bool {
    let function = chain.voi().window().map(|(_, _, function)| function);
    let linear = matches!(
        function,
        Some(VoiFunction::Linear | VoiFunction::LinearExact)
    );
    let (ymin, ymax) = chain.voi().output_range();
    linear && ymin.to_bits() == 0.0_f32.to_bits() && (ymax - 255.0).to_bits() == 0.0_f32.to_bits()
}

/// Byte stored by the 1024-wide ramp. `inverted` complements the table entry,
/// which is how the reference inverts `MONOCHROME1` without a second formula.
fn reference_ramp_index(display: f32, inverted: bool) -> u32 {
    if !display.is_finite() {
        return 0;
    }
    let rising = if inverted { 255.0 - display } else { display };
    if !rising.is_finite() || rising <= 0.0 {
        return if inverted { 255 } else { 0 };
    }
    if rising >= 255.0 {
        return if inverted { 0 } else { 255 };
    }
    let width = 1024.0_f32;
    let half = 0.5 / width;
    let mut coord = rising / 255.0;
    if coord < half {
        coord = half;
    } else if coord > 1.0 - half {
        coord = 1.0 - half;
    }
    let bin = (coord * width).floor() / 1023.0;
    let sample = if inverted { 1.0 - bin } else { bin };
    u32::from(byte_from_channel(round_half_even(sample * 255.0)).unwrap_or(0))
}

/// Round half to even. `Uint8ClampedArray` uses that rule when the image
/// mapper stores `255 * table`.
fn round_half_even(value: f32) -> f32 {
    if !value.is_finite() || value <= 0.0 {
        return 0.0;
    }
    if value >= 255.0 {
        return 255.0;
    }
    let floor = value.floor();
    let fraction = value - floor;
    if fraction > 0.5 {
        return floor + 1.0;
    }
    if fraction < 0.5 {
        return floor;
    }
    let half = floor * 0.5;
    let even = (half.floor() - half).to_bits() == 0.0_f32.to_bits();
    if even { floor } else { floor + 1.0 }
}

/// Table address. `display` itself is not modified.
///
/// Values at or below 0 use texel 0. Values at or above `last` use texel
/// `last`. A finite value in between uses the NEAREST integer, ties up.
///
/// Rounding rather than truncating is not a preference. The pinned reference
/// quantises its colour table with `Math.floor(value * 255 + 0.5)` and reads
/// it back through an 8-bit texture, so it rounds. Deviation D-11 makes that
/// reference the definition of correct, and truncation darkens every frame by
/// about half a display code on the informative region, which HLD 25.1's
/// signed-mean bound of `0.1` rejects. `byte_from_channel` already rounds for
/// the colour path, so truncating here was also an internal inconsistency.
/// `table_index` in `graph.rs` applies the same rule in WGSL.
pub(crate) fn display_index(display: f32, last: u32) -> u32 {
    if !display.is_finite() || display <= 0.0 {
        return 0;
    }
    let last_f = f32_from_u32(last);
    if display >= last_f {
        return last;
    }
    let rounded = display + 0.5;
    if rounded >= last_f {
        return last;
    }
    let mut lo = 0_u32;
    let mut hi = last;
    while lo < hi {
        let step = hi - lo;
        if step <= 1 {
            break;
        }
        let mid = lo + step / 2;
        if f32_from_u32(mid) <= rounded {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

fn f32_from_u32(value: u32) -> f32 {
    let high = u16::try_from(value >> 16).unwrap_or(0);
    let low = u16::try_from(value & 0xffff).unwrap_or(0);
    f32::from(high) * 65_536.0 + f32::from(low)
}

fn f32_from_i32(value: i32) -> Option<f32> {
    if value < 0 {
        let magnitude = u16::try_from(value.unsigned_abs()).ok()?;
        Some(-f32::from(magnitude))
    } else {
        Some(f32_from_u32(u32::try_from(value).ok()?))
    }
}

pub(crate) fn byte_from_channel(value: f32) -> Option<u8> {
    if !(0.0..=255.0).contains(&value) {
        return None;
    }
    let mut candidate = 0_u8;
    loop {
        let delta = (f32::from(candidate) - value).abs();
        if delta.to_bits() == 0_f32.to_bits() {
            return Some(candidate);
        }
        if candidate == 255 {
            return None;
        }
        candidate = candidate.saturating_add(1);
    }
}

fn rgb_at(bytes: &[u8], index: u32) -> Rgb {
    let start = usize::try_from(index).unwrap_or(0).saturating_mul(4);
    let channel = |offset: usize| {
        bytes
            .get(start.saturating_add(offset))
            .copied()
            .unwrap_or(0)
    };
    Rgb {
        r: f32::from(channel(0)),
        g: f32::from(channel(1)),
        b: f32::from(channel(2)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_colour_shader_keeps_one_lut_chain_and_the_two_keys() {
        let module = crate::graph::tier_b_fragment_module();
        assert_eq!(module.matches("fn lut_chain(").count(), 1);
        assert_eq!(module.matches("fn voi_linear(").count(), 1);
        assert_eq!(module.matches("fn voi_linear_exact(").count(), 1);
        assert_eq!(module.matches("fn voi_sigmoid(").count(), 1);
        assert!(module.contains("colour.mode == 2u"));
        assert!(module.contains("colour.mode == 1u"));
        assert_eq!(module.matches("fn reference_ramp_byte(").count(), 1);
        assert!(module.contains("colour.pad == 1u"));
        assert!(!module.contains("@compute"));
        assert!(!module.contains("var<storage"));
    }

    #[test]
    fn colour_params_stay_sixteen_bytes_beside_the_voi_uniform() {
        assert_eq!(std::mem::size_of::<super::ColourParams>(), 16);
        assert_eq!(super::ColourParams::NONE.pad, 0);
        assert_eq!(super::ColourParams::NONE.mode, 0);
        use ocelli_pixel::PhotometricInterpretation;
        let grey = super::ColourLut::preset(
            super::PresetId::Grey,
            PhotometricInterpretation::Monochrome2,
        );
        let Ok(grey) = grey else {
            unreachable!("grey preset is available");
        };
        assert_eq!(super::ColourParams::from_lut(&grey).pad, 1);
        let stops = super::ColourLut::preset(
            super::PresetId::Stops,
            PhotometricInterpretation::Monochrome2,
        );
        let Ok(stops) = stops else {
            unreachable!("stops preset is available");
        };
        assert_eq!(super::ColourParams::from_lut(&stops).pad, 0);
    }

    #[test]
    fn a_colormap_selects_the_matching_preset() {
        use ocelli_pixel::PhotometricInterpretation;
        use ocelli_viewport::presentation::Colormap;

        let grey = super::colour_lut_for(Colormap::Grey, PhotometricInterpretation::Monochrome2);
        assert!(grey.is_ok());
        let Ok(grey) = grey else {
            unreachable!("asserted immediately above");
        };
        assert_eq!(grey.preset_id(), Some(super::PresetId::Grey));

        let stops = super::colour_lut_for(Colormap::Stops, PhotometricInterpretation::Monochrome2);
        assert!(stops.is_ok());
        let Ok(stops) = stops else {
            unreachable!("asserted immediately above");
        };
        assert_eq!(stops.preset_id(), Some(super::PresetId::Stops));

        let palette =
            super::colour_lut_for(Colormap::Grey, PhotometricInterpretation::PaletteColor);
        assert!(matches!(palette, Err(super::ColourLutError::Unavailable)));
    }

    /// Stored 149, 202, 213 and 224 at LINEAR 40/400.
    ///
    /// The continuous display rounds to 197, 231, 238 and 245. The reference
    /// frame `syntax__explicit_vr_le` stores 198, 232, 239 and 246. Those four
    /// codes are the whole signed-mean failure of that view.
    #[test]
    fn grey_linear_matches_the_reference_1024_ramp() {
        use ocelli_core::Stored;
        use ocelli_pixel::{PhotometricInterpretation, VoiFunction};

        let lut = grey_lut(PhotometricInterpretation::Monochrome2);
        let chain = window_chain(VoiFunction::Linear, PhotometricInterpretation::Monochrome2);
        let stored = [Stored(149.0), Stored(202.0), Stored(213.0), Stored(224.0)];
        let mut destination = [ocelli_pixel::Rgb::BLACK; 4];
        assert!(
            lut.map_preset_into(&chain, &stored, &mut destination)
                .is_ok()
        );
        let bytes = [
            channel_byte(&destination, 0),
            channel_byte(&destination, 1),
            channel_byte(&destination, 2),
            channel_byte(&destination, 3),
        ];
        assert_eq!(bytes, [198, 232, 239, 246]);

        let first = stored.first().copied().unwrap_or(Stored(0.0));
        let nearest = super::display_index(chain.apply(first).0, 255);
        assert_eq!(nearest, 197);
    }

    #[test]
    fn grey_monochrome1_complements_the_reference_ramp() {
        use ocelli_core::Stored;
        use ocelli_pixel::{PhotometricInterpretation, VoiFunction};

        let lut = grey_lut(PhotometricInterpretation::Monochrome1);
        let chain = window_chain(VoiFunction::Linear, PhotometricInterpretation::Monochrome1);
        assert!(chain.inverts());
        let mut destination = [ocelli_pixel::Rgb::BLACK; 1];
        assert!(
            lut.map_preset_into(&chain, &[Stored(149.0)], &mut destination)
                .is_ok()
        );
        assert_eq!(channel_byte(&destination, 0), 57);
    }

    #[test]
    fn sigmoid_keeps_the_nearest_byte() {
        use ocelli_core::Stored;
        use ocelli_pixel::{PhotometricInterpretation, VoiFunction};

        let lut = grey_lut(PhotometricInterpretation::Monochrome2);
        let chain = window_chain(VoiFunction::Sigmoid, PhotometricInterpretation::Monochrome2);
        let sample = Stored(40.0);
        let mut destination = [ocelli_pixel::Rgb::BLACK; 1];
        assert!(
            lut.map_preset_into(&chain, &[sample], &mut destination)
                .is_ok()
        );
        let nearest = super::display_index(chain.apply(sample).0, 255);
        let byte = u8::try_from(nearest).unwrap_or(0);
        assert_eq!(channel_byte(&destination, 0), byte);
    }

    fn grey_lut(photometric: ocelli_pixel::PhotometricInterpretation) -> super::ColourLut {
        let lut = super::ColourLut::preset(super::PresetId::Grey, photometric);
        let Ok(lut) = lut else {
            unreachable!("grey preset is available");
        };
        lut
    }

    fn channel_byte(rgb: &[ocelli_pixel::Rgb], index: usize) -> u8 {
        let value = rgb.get(index).map(|pixel| pixel.r).unwrap_or(0.0);
        super::byte_from_channel(value).unwrap_or(u8::MAX)
    }

    fn window_chain(
        function: ocelli_pixel::VoiFunction,
        photometric: ocelli_pixel::PhotometricInterpretation,
    ) -> ocelli_pixel::LutChain {
        use ocelli_pixel::{LutChain, ModalityTransform, PresentationLutEvidence, VoiTransform};

        let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
        let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, function, 0.0, 255.0);
        let (Ok(modality), Ok(voi)) = (modality, voi) else {
            unreachable!("40/400 is a legal window");
        };
        let chain = LutChain::new(modality, voi, photometric, PresentationLutEvidence::Absent);
        let Ok(chain) = chain else {
            unreachable!("the chain accepts this window");
        };
        chain
    }
}
