//! Scalar CPU stack raster. F-X003.
//!
//! The GPU graph still refuses tier C. This type samples with
//! [`View::canvas_to_index`] and shades with [`LutChain`] or
//! [`ColorTransform`]. It does not open a device and it does not allocate
//! while shading.

use ocelli_core::{Canvas, Index, Pt, Stored, Transform};
use ocelli_pixel::{
    ColorTransform, DecodedLayout, DecodedPhotometric, LutChain, PhotometricInterpretation,
    PixelDataEncoding, PixelError, PlanarConfiguration, Rgb, SampleLayout,
};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::view::{CoordinateError, INDEX_TO_IMAGE_CENTRE, View};

use crate::colour::{ColourLut, ColourLutError, PresetId};
use crate::extent::{css_from_device, device_extent};

// PS3.3 C.7.6.2.1.1 puts the integer index on the pixel centre. Image space
// stores the corner, so the centre is `index + 0.5`. `INDEX_TO_IMAGE_CENTRE`
// is imported from `ocelli-viewport` rather than defined again. The fragment
// test truncates that image coordinate. Mutating the constant must reject the
// left half of texel 0.

const OPAQUE_BLACK: [u8; 4] = [0, 0, 0, 255];

/// Why a CPU frame was not drawn. The output slice is left untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RasterError {
    /// The target does not fit the pre-sized device extent.
    #[error("cpu raster extent is outside the pre-sized canvas")]
    Extent,
    /// `out` is not exactly four bytes per target pixel.
    #[error("cpu raster output length does not match the target")]
    OutputLength,
    /// Stored samples do not cover the source grid or the colour layout.
    #[error("cpu raster source length does not match the grid")]
    Length,
    /// The canvas map is singular. The view is not modified.
    #[error("cpu raster canvas map is singular")]
    Singular,
    /// A settled draw named a generation that is no longer current.
    #[error("cpu raster generation is stale")]
    Stale,
    /// ICC or another space [`ColorTransform`] does not implement.
    #[error("cpu raster colour space is unavailable")]
    Unavailable,
    /// CSS size or the device pixel ratio was refused.
    #[error("cpu raster canvas size was refused")]
    Canvas,
}

/// Interacting draws a reduced target and keeps refining set.
/// Settled draws the full device extent for one generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterQuality {
    /// Reduced resolution. Sets the refining flag after a successful draw.
    Interacting,
    /// Full device extent for `generation`. A mismatch draws nothing.
    Settled {
        /// Generation this completion was scheduled for.
        generation: u32,
    },
}

/// What a successful call produced. `wrote` is false when a settled frame
/// was already current and [`CpuRaster::invalidate`] has not been called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterResult {
    /// Target extent written. Zero when nothing was written.
    pub extent: [u32; 2],
    /// Generation stored on the raster after the call.
    pub generation: u32,
    /// Refining flag after the call.
    pub refining: bool,
    /// Whether `out` was written.
    pub wrote: bool,
}

/// Source columns and rows, and the target this call fills.
///
/// The two extents stay separate values. They are one argument so a draw
/// stays within the argument limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterGrid {
    /// Source image columns and rows.
    pub source: [u32; 2],
    /// Target columns and rows. A settled draw must equal the device extent.
    pub target: [u32; 2],
}

/// A resolved colour transform, or a space this raster will not invent.
pub enum RasterColor<'a> {
    /// Decoder-reported space [`ColorTransform`] already accepted.
    Supported(&'a ColorTransform),
    /// ICC and any other unsupported space. The call draws nothing.
    Unsupported,
}

/// Pre-sized scalar raster for one stack viewport.
#[derive(Debug)]
pub struct CpuRaster {
    css_width: f64,
    css_height: f64,
    dpr: f64,
    device: [u32; 2],
    grey: ColourLut,
    stops: ColourLut,
    rgb_scratch: Vec<Rgb>,
    generation: u32,
    refining: bool,
    settled: Option<u32>,
    needs_draw: bool,
    allocations: u32,
}

impl CpuRaster {
    /// Pre-size to the device extent of this CSS box.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] when the CSS size or `dpr` is refused.
    /// [`RasterError::Extent`] when the device extent is empty.
    pub fn open(css_width: f64, css_height: f64, dpr: f64) -> Result<Self, RasterError> {
        let (grey, stops) = preset_tables()?;
        let mut raster = Self {
            css_width,
            css_height,
            dpr,
            device: [0, 0],
            grey,
            stops,
            rgb_scratch: Vec::new(),
            generation: 0,
            refining: false,
            settled: None,
            needs_draw: true,
            allocations: 0,
        };
        raster.resize_storage(css_width, css_height, dpr)?;
        Ok(raster)
    }

    /// Device extent the settled frame must fill.
    #[must_use]
    pub const fn device(&self) -> [u32; 2] {
        self.device
    }

    /// Times the backing storage has been allocated. A draw does not increment it.
    #[must_use]
    pub const fn allocations(&self) -> u32 {
        self.allocations
    }

    /// Flag bit 3. True after an interaction until the latest generation settles.
    #[must_use]
    pub const fn refining(&self) -> bool {
        self.refining
    }

    /// Generation a settled completion must name.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Replace the device pixel ratio and reallocate when the extent changes.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] or [`RasterError::Extent`], as in [`Self::open`].
    pub fn set_device_pixel_ratio(&mut self, dpr: f64) -> Result<(), RasterError> {
        self.resize_storage(self.css_width, self.css_height, dpr)
    }

    /// Bump the generation and mark the viewport refining.
    ///
    /// The previous settled generation is no longer current. A completion
    /// that still names it is [`RasterError::Stale`].
    pub const fn begin_interaction(&mut self) -> u32 {
        self.generation = self.generation.saturating_add(1);
        self.refining = true;
        self.needs_draw = true;
        self.generation
    }

    /// Allow one more settled draw of the current generation.
    pub const fn invalidate(&mut self) {
        self.needs_draw = true;
    }

    /// Replace the current generation and drop any settled frame.
    ///
    /// A completion that still names the previous generation is
    /// [`RasterError::Stale`].
    pub const fn note_new_source(&mut self, generation: u32) {
        self.generation = generation;
        self.refining = true;
        self.needs_draw = true;
        self.settled = None;
    }

    /// Change the CSS box and the device pixel ratio together.
    ///
    /// # Errors
    ///
    /// [`RasterError::Canvas`] or [`RasterError::Extent`], as in [`Self::open`].
    pub fn resize(&mut self, css_width: f64, css_height: f64, dpr: f64) -> Result<(), RasterError> {
        self.resize_storage(css_width, css_height, dpr)
    }

    /// A drawn frame is not presented until the blit is accepted.
    ///
    /// The next settled draw writes again. Settlement stays empty.
    pub const fn defer_presentation(&mut self) {
        self.needs_draw = true;
        self.settled = None;
        self.refining = true;
    }

    /// The blit of the current generation was accepted.
    pub const fn accept_presentation(&mut self) {
        self.refining = false;
        self.settled = Some(self.generation);
        self.needs_draw = false;
    }

    /// Reserve colour scratch for one source grid.
    ///
    /// The draw does not call this. A source larger than the last reservation
    /// is refused until the caller reserves it again.
    ///
    /// # Errors
    ///
    /// [`RasterError::Extent`] when the grid is empty or the count overflows.
    pub fn reserve_source(&mut self, columns: u32, rows: u32) -> Result<(), RasterError> {
        let pixels = pixel_count([columns, rows])?;
        if self.rgb_scratch.len() < pixels {
            self.rgb_scratch = vec![Rgb::BLACK; pixels];
            self.allocations = self.allocations.saturating_add(1);
        }
        Ok(())
    }

    /// Draw monochrome samples through [`LutChain`] and the preset table.
    ///
    /// The source length is accepted before a settled idle return. A mismatch
    /// is [`RasterError::Length`] and writes nothing. A stale generation is
    /// still [`RasterError::Stale`].
    ///
    /// # Errors
    ///
    /// See [`RasterError`]. A refusal writes nothing. A singular map does not
    /// modify `view`.
    pub fn raster_into(
        &mut self,
        view: &View,
        presentation: &Presentation,
        stored: &[Stored],
        grid: RasterGrid,
        quality: RasterQuality,
        out: &mut [u8],
    ) -> Result<RasterResult, RasterError> {
        let RasterGrid { source, target } = grid;
        let gated = self.gate(target, quality, out)?;
        Self::accept_mono_source(stored, source)?;
        if let Some(done) = gated {
            return Ok(done);
        }
        let map = index_map(view, source)?;
        self.write_mono(presentation, stored, source, target, &map, out)?;
        Ok(self.finish(target, quality))
    }

    /// Draw palette or RGB through [`ColorTransform`].
    ///
    /// [`RasterColor::Unsupported`] is ICC and any space the transform does
    /// not implement. Nothing is written and no threshold is applied.
    /// A supported transform accepts its source length before a settled idle
    /// return. That check does not allocate colour scratch.
    ///
    /// # Errors
    ///
    /// See [`RasterError`].
    pub fn raster_color_into(
        &mut self,
        view: &View,
        color: RasterColor<'_>,
        stored: &[Stored],
        grid: RasterGrid,
        quality: RasterQuality,
        out: &mut [u8],
    ) -> Result<RasterResult, RasterError> {
        let RasterGrid { source, target } = grid;
        let transform = match color {
            RasterColor::Unsupported => return Err(RasterError::Unavailable),
            RasterColor::Supported(transform) => transform,
        };
        let gated = self.gate(target, quality, out)?;
        source_extent(source)?;
        let pixels = pixel_count(source)?;
        transform
            .accept_source(stored, pixels)
            .map_err(color_error)?;
        if let Some(done) = gated {
            return Ok(done);
        }
        let map = index_map(view, source)?;
        self.prepare_color(transform, stored, pixels)?;
        self.write_color(source, target, &map, out)?;
        Ok(self.finish(target, quality))
    }

    /// Draw interleaved native RGB through [`ColorTransform::resolve`].
    ///
    /// The resolution is samples per pixel 3, photometric RGB, interleaved
    /// planar configuration, native pixel data, preserved decoder evidence
    /// and no palette. It is built once per call. [`RasterColor::Unsupported`]
    /// is not this entry.
    ///
    /// # Errors
    ///
    /// See [`RasterError`]. [`RasterError::Unavailable`] when that resolution
    /// is refused.
    pub fn raster_rgb_into(
        &mut self,
        view: &View,
        stored: &[Stored],
        grid: RasterGrid,
        quality: RasterQuality,
        out: &mut [u8],
    ) -> Result<RasterResult, RasterError> {
        let transform = interleaved_native_rgb()?;
        self.raster_color_into(
            view,
            RasterColor::Supported(&transform),
            stored,
            grid,
            quality,
            out,
        )
    }

    /// Monochrome samples are one stored value per source pixel.
    ///
    /// # Errors
    ///
    /// [`RasterError::Extent`] when the grid is empty or the count overflows.
    /// [`RasterError::Length`] when `stored` is not that count.
    pub fn accept_mono_source(stored: &[Stored], source: [u32; 2]) -> Result<(), RasterError> {
        source_extent(source)?;
        if stored.len() != pixel_count(source)? {
            return Err(RasterError::Length);
        }
        Ok(())
    }

    /// Interleaved native RGB is three stored values per source pixel.
    ///
    /// The resolution matches [`Self::raster_rgb_into`]. No scratch is
    /// allocated.
    ///
    /// # Errors
    ///
    /// [`RasterError::Extent`] when the grid is empty or the count overflows.
    /// [`RasterError::Length`] when the sample count is not three per pixel.
    /// [`RasterError::Unavailable`] when that RGB resolution is refused.
    pub fn accept_rgb_source(stored: &[Stored], source: [u32; 2]) -> Result<(), RasterError> {
        source_extent(source)?;
        let pixels = pixel_count(source)?;
        interleaved_native_rgb()?
            .accept_source(stored, pixels)
            .map_err(color_error)
    }

    /// Display value at one target pixel, before the colour table.
    ///
    /// `Ok(None)` is outside the image. The chain is not called for that
    /// pixel. The returned display is not clamped.
    ///
    /// # Errors
    ///
    /// [`RasterError::Extent`], [`RasterError::Length`], [`RasterError::Singular`],
    /// or [`RasterError::Canvas`].
    pub fn display_at(
        &self,
        view: &View,
        presentation: &Presentation,
        stored: &[Stored],
        source: [u32; 2],
        target: [u32; 2],
        pixel: [u32; 2],
    ) -> Result<Option<f32>, RasterError> {
        source_extent(source)?;
        if stored.len() != pixel_count(source)? {
            return Err(RasterError::Length);
        }
        let map = index_map(view, source)?;
        match self.texel(source, target, &map, pixel)? {
            None => Ok(None),
            Some(texel) => Ok(Some(display_of(presentation, stored, texel)?)),
        }
    }

    fn resize_storage(
        &mut self,
        css_width: f64,
        css_height: f64,
        dpr: f64,
    ) -> Result<(), RasterError> {
        let (width, height) =
            device_extent(css_width, css_height, dpr).map_err(|_| RasterError::Canvas)?;
        if width == 0 || height == 0 {
            return Err(RasterError::Extent);
        }
        let initial = self.device == [0, 0];
        let scale_changed = self.css_width.to_bits() != css_width.to_bits()
            || self.css_height.to_bits() != css_height.to_bits()
            || self.dpr.to_bits() != dpr.to_bits();
        let extent_changed = self.device != [width, height];
        self.css_width = css_width;
        self.css_height = css_height;
        self.dpr = dpr;
        self.device = [width, height];
        if !initial && (scale_changed || extent_changed) {
            self.generation = self.generation.saturating_add(1);
            self.refining = true;
            self.needs_draw = true;
            self.settled = None;
        }
        Ok(())
    }

    fn gate(
        &self,
        target: [u32; 2],
        quality: RasterQuality,
        out: &[u8],
    ) -> Result<Option<RasterResult>, RasterError> {
        if target[0] == 0
            || target[1] == 0
            || target[0] > self.device[0]
            || target[1] > self.device[1]
        {
            return Err(RasterError::Extent);
        }
        if let RasterQuality::Settled { .. } = quality
            && target != self.device
        {
            return Err(RasterError::Extent);
        }
        let bytes = pixel_count(target)?
            .checked_mul(4)
            .ok_or(RasterError::Extent)?;
        if out.len() != bytes {
            return Err(RasterError::OutputLength);
        }
        if let RasterQuality::Settled { generation } = quality {
            if generation != self.generation {
                return Err(RasterError::Stale);
            }
            if self.settled == Some(generation) && !self.needs_draw {
                return Ok(Some(self.idle_result()));
            }
        }
        Ok(None)
    }

    fn idle_result(&self) -> RasterResult {
        RasterResult {
            extent: [0, 0],
            generation: self.generation,
            refining: self.refining,
            wrote: false,
        }
    }

    fn finish(&mut self, target: [u32; 2], quality: RasterQuality) -> RasterResult {
        match quality {
            RasterQuality::Interacting => {
                self.refining = true;
                self.needs_draw = true;
            }
            RasterQuality::Settled { generation } => {
                self.refining = false;
                self.settled = Some(generation);
                self.needs_draw = false;
            }
        }
        RasterResult {
            extent: target,
            generation: self.generation,
            refining: self.refining,
            wrote: true,
        }
    }

    fn prepare_color(
        &mut self,
        transform: &ColorTransform,
        stored: &[Stored],
        pixels: usize,
    ) -> Result<(), RasterError> {
        let Some(destination) = self.rgb_scratch.get_mut(..pixels) else {
            return Err(RasterError::Extent);
        };
        transform.map_into(stored, destination).map_err(color_error)
    }

    fn write_mono(
        &self,
        presentation: &Presentation,
        stored: &[Stored],
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        out: &mut [u8],
    ) -> Result<(), RasterError> {
        if stored.len() != pixel_count(source)? {
            return Err(RasterError::Length);
        }
        let table = match presentation.colormap() {
            Colormap::Grey => &self.grey,
            Colormap::Stops => &self.stops,
        };
        self.shade(source, target, map, out, |texel| {
            rgba_of_preset(table, presentation.chain(), stored, texel)
        })
    }

    fn write_color(
        &self,
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        out: &mut [u8],
    ) -> Result<(), RasterError> {
        let scratch = &self.rgb_scratch;
        self.shade(source, target, map, out, |texel| {
            let rgb = scratch.get(texel).copied().unwrap_or(Rgb::BLACK);
            Ok(rgba_from_rgb(rgb))
        })
    }

    fn shade(
        &self,
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        out: &mut [u8],
        mut rgba_of: impl FnMut(usize) -> Result<[u8; 4], RasterError>,
    ) -> Result<(), RasterError> {
        let mut y = 0_u32;
        while y < target[1] {
            let mut x = 0_u32;
            while x < target[0] {
                let rgba = match self.texel(source, target, map, [x, y])? {
                    None => OPAQUE_BLACK,
                    Some(texel) => rgba_of(texel)?,
                };
                write_pixel(out, pixel_index(x, y, target[0])?, rgba)?;
                x = x.saturating_add(1);
            }
            y = y.saturating_add(1);
        }
        Ok(())
    }

    fn texel(
        &self,
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        pixel: [u32; 2],
    ) -> Result<Option<usize>, RasterError> {
        if pixel[0] >= target[0] || pixel[1] >= target[1] {
            return Err(RasterError::Extent);
        }
        let css_x = css_of(pixel[0], target[0], self.device[0], self.dpr)?;
        let css_y = css_of(pixel[1], target[1], self.device[1], self.dpr)?;
        let index = map.apply(Pt::<Canvas>::new(css_x, css_y, 0.0));
        let Some(column) = texel_of(index.x + INDEX_TO_IMAGE_CENTRE, source[0]) else {
            return Ok(None);
        };
        let Some(row) = texel_of(index.y + INDEX_TO_IMAGE_CENTRE, source[1]) else {
            return Ok(None);
        };
        Ok(Some(pixel_index(column, row, source[0])?))
    }
}

fn interleaved_native_rgb() -> Result<ColorTransform, RasterError> {
    let layout = match SampleLayout::new(
        3,
        PhotometricInterpretation::Rgb,
        Some(PlanarConfiguration::Interleaved),
    ) {
        Ok(layout) => layout,
        Err(_) => return Err(RasterError::Unavailable),
    };
    ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    )
    .map_err(|_| RasterError::Unavailable)
}

fn preset_tables() -> Result<(ColourLut, ColourLut), RasterError> {
    let photometric = PhotometricInterpretation::Monochrome2;
    let grey =
        ColourLut::preset(PresetId::Grey, photometric).map_err(|_| RasterError::Unavailable)?;
    let stops =
        ColourLut::preset(PresetId::Stops, photometric).map_err(|_| RasterError::Unavailable)?;
    Ok((grey, stops))
}

fn index_map(view: &View, source: [u32; 2]) -> Result<Transform<Canvas, Index>, RasterError> {
    view.canvas_to_index(source[0], source[1])
        .map_err(|error| match error {
            CoordinateError::Singular => RasterError::Singular,
        })
}

fn display_of(
    presentation: &Presentation,
    stored: &[Stored],
    texel: usize,
) -> Result<f32, RasterError> {
    let sample = stored.get(texel).copied().ok_or(RasterError::Length)?;
    Ok(presentation.chain().apply(sample).0)
}

fn rgba_of_preset(
    table: &ColourLut,
    chain: &LutChain,
    stored: &[Stored],
    texel: usize,
) -> Result<[u8; 4], RasterError> {
    let sample = stored.get(texel).copied().ok_or(RasterError::Length)?;
    let mut rgb = [Rgb::BLACK];
    table
        .map_preset_into(chain, &[sample], &mut rgb)
        .map_err(|error| match error {
            ColourLutError::Length => RasterError::Length,
            ColourLutError::Unavailable => RasterError::Unavailable,
        })?;
    Ok(rgba_from_rgb(rgb.first().copied().unwrap_or(Rgb::BLACK)))
}

fn rgba_from_rgb(rgb: Rgb) -> [u8; 4] {
    [
        byte_from_channel(rgb.r),
        byte_from_channel(rgb.g),
        byte_from_channel(rgb.b),
        255,
    ]
}

fn color_error(error: PixelError) -> RasterError {
    match error {
        PixelError::SourceLength
        | PixelError::DestinationLength
        | PixelError::SubsampledChromaAlignment
        | PixelError::LengthOverflow => RasterError::Length,
        _ => RasterError::Unavailable,
    }
}

fn css_of(pixel: u32, target: u32, device: u32, dpr: f64) -> Result<f64, RasterError> {
    let target_f = f64_from_u32(target);
    if target_f.to_bits() == 0.0_f64.to_bits() {
        return Err(RasterError::Extent);
    }
    let device_center = (f64_from_u32(pixel) + 0.5) / target_f * f64_from_u32(device);
    css_from_device(device_center, dpr).map_err(|_| RasterError::Canvas)
}

fn texel_of(image: f64, limit: u32) -> Option<u32> {
    if !image.is_finite() || image < 0.0 {
        return None;
    }
    let limit_f = f64_from_u32(limit);
    if image >= limit_f {
        return None;
    }
    let mut lo = 0_u32;
    let mut hi = limit;
    while lo.saturating_add(1) < hi {
        let mid = lo + (hi - lo) / 2;
        if f64_from_u32(mid) <= image {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo)
}

fn source_extent(source: [u32; 2]) -> Result<(), RasterError> {
    if source[0] == 0 || source[1] == 0 {
        Err(RasterError::Extent)
    } else {
        Ok(())
    }
}

fn pixel_count(extent: [u32; 2]) -> Result<usize, RasterError> {
    let width = usize::try_from(extent[0]).map_err(|_| RasterError::Extent)?;
    let height = usize::try_from(extent[1]).map_err(|_| RasterError::Extent)?;
    width.checked_mul(height).ok_or(RasterError::Extent)
}

fn pixel_index(x: u32, y: u32, columns: u32) -> Result<usize, RasterError> {
    let width = usize::try_from(columns).map_err(|_| RasterError::Extent)?;
    let column = usize::try_from(x).map_err(|_| RasterError::Extent)?;
    let row = usize::try_from(y).map_err(|_| RasterError::Extent)?;
    row.checked_mul(width)
        .and_then(|offset| offset.checked_add(column))
        .ok_or(RasterError::Extent)
}

fn write_pixel(out: &mut [u8], index: usize, rgba: [u8; 4]) -> Result<(), RasterError> {
    let start = index.saturating_mul(4);
    let Some(slot) = out.get_mut(start..start.saturating_add(4)) else {
        return Err(RasterError::OutputLength);
    };
    slot.copy_from_slice(&rgba);
    Ok(())
}

/// Nearest byte in `0..=255`. Ties pick the higher byte.
///
/// Values outside that range become 0 or 255 because the destination is an
/// 8-bit channel. The display value itself is not clamped before this store.
fn byte_from_channel(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    if value >= 255.0 {
        return 255;
    }
    let mut best = 0_u8;
    let mut best_delta = (f32::from(0_u8) - value).abs();
    let mut candidate = 0_u8;
    loop {
        let delta = (f32::from(candidate) - value).abs();
        if delta < best_delta || delta.to_bits() == best_delta.to_bits() {
            best = candidate;
            best_delta = delta;
        }
        if candidate == 255 {
            break;
        }
        candidate = candidate.saturating_add(1);
    }
    best
}

fn f64_from_u32(value: u32) -> f64 {
    let high = u16::try_from(value >> 16).unwrap_or(0);
    let low = u16::try_from(value & 0xffff).unwrap_or(0);
    f64::from(high) * 65_536.0 + f64::from(low)
}
