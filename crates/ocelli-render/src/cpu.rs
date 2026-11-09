//! Scalar CPU stack raster. F-X003.
//!
//! The GPU graph still refuses tier C. This type samples with
//! [`View::canvas_to_index`] and shades with [`LutChain`] or
//! [`ColorTransform`]. It does not open a device and it does not allocate
//! while shading.

use ocelli_core::{Canvas, Index, Pt, Stored, Transform, exact_f32_integer};
use ocelli_pixel::{
    ColorTransform, ColourWindow, DecodedLayout, DecodedPhotometric, LutChain,
    PhotometricInterpretation, PixelDataEncoding, PixelError, PlanarConfiguration, Rgb,
    SampleLayout,
};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::view::{CoordinateError, INDEX_TO_IMAGE_CENTRE, View};

use crate::caps::Tier;
use crate::colour::{ColourLut, ColourLutError, PresetId};
use crate::extent::{css_from_device, device_extent};
use crate::interpolation::{
    Interpolation, InterpolationError, available, blend_display, blend_points,
};

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

/// A colour transform whose palette, when it has one, fits a display byte.
///
/// The check runs in [`RasterColor::supported`]. A draw does not repeat it.
#[derive(Clone, Copy)]
pub struct AcceptedColor<'a> {
    transform: &'a ColorTransform,
}

/// A resolved colour transform, or a space this raster will not invent.
///
/// Reuse this descriptor while its immutable transform borrow remains valid.
#[derive(Clone, Copy)]
pub enum RasterColor<'a> {
    /// Decoder-reported space [`AcceptedColor`] already accepted.
    Supported(AcceptedColor<'a>),
    /// ICC and any other unsupported space. The call draws nothing.
    Unsupported,
}

impl<'a> RasterColor<'a> {
    /// Accept a colour transform for tier C.
    ///
    /// A palette entry that is not an exact display byte is refused here.
    /// The GPU bake uses the same predicate, once, when the table is built.
    ///
    /// # Errors
    ///
    /// [`RasterError::Unavailable`] when a palette entry is outside `0..=255`
    /// or is not an integer.
    pub fn supported(transform: &'a ColorTransform) -> Result<Self, RasterError> {
        if let Some(palette) = transform.palette() {
            let display_byte = palette
                .channel_entries()
                .all(|entry| crate::colour::byte_from_channel(entry).is_some());
            if !display_byte {
                return Err(RasterError::Unavailable);
            }
        }
        Ok(Self::Supported(AcceptedColor { transform }))
    }
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
    interpolation: Interpolation,
    display_scratch: Vec<f32>,
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
            interpolation: Interpolation::Nearest,
            display_scratch: Vec::new(),
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

    /// Mode used by the next draw. The default is nearest.
    #[must_use]
    pub const fn interpolation(&self) -> Interpolation {
        self.interpolation
    }

    /// Select nearest or linear. `0` is nearest and `1` is linear.
    ///
    /// A change marks the frame dirty. Any other integer is refused and
    /// leaves the mode and the caller's frame buffer untouched.
    ///
    /// # Errors
    ///
    /// [`RasterError::Unavailable`] when `mode` is not 0 or 1.
    pub fn set_interpolation(&mut self, mode: u32) -> Result<(), RasterError> {
        let next = Interpolation::from_mode(mode).map_err(|_| RasterError::Unavailable)?;
        if !available(next, Tier::Cpu) {
            return Err(RasterError::Unavailable);
        }
        if next != self.interpolation {
            self.interpolation = next;
            self.invalidate();
        }
        Ok(())
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
    /// is refused until the caller reserves it again. The linear display plane
    /// grows only when the selected mode is linear, and only inside this call.
    /// Nearest draws do not read that plane. A linear draw made after a nearest
    /// reserve still returns [`RasterError::Extent`] until the caller reserves
    /// again while linear is selected.
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
        if self.interpolation != Interpolation::Linear {
            return Ok(());
        }
        let display_len = pixels.checked_mul(3).ok_or(RasterError::Extent)?;
        if self.display_scratch.len() < display_len {
            self.display_scratch = vec![0.0; display_len];
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
            RasterColor::Supported(accepted) => accepted.transform,
        };
        let gated = self.gate(target, quality, out)?;
        source_extent(source)?;
        let pixels = pixel_count(source)?;
        let columns = usize::try_from(source[0]).map_err(|_| RasterError::Extent)?;
        transform
            .accept_source(stored, pixels, columns)
            .map_err(color_error)?;
        if let Some(done) = gated {
            return Ok(done);
        }
        // A palette was checked when it was accepted. A stored colour sample
        // is tested after the transform's sample scale, so a 12-bit 4095 is
        // the display byte 255. The samples change per draw.
        if !stored_colour_is_a_display_byte(transform, stored) {
            return Err(RasterError::Unavailable);
        }
        let map = index_map(view, source)?;
        self.prepare_color(transform, stored, pixels, columns)?;
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
            RasterColor::supported(&transform)?,
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
        let columns = usize::try_from(source[0]).map_err(|_| RasterError::Extent)?;
        interleaved_native_rgb()?
            .accept_source(stored, pixels, columns)
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
        if self.interpolation == Interpolation::Linear {
            return self.display_linear(presentation, stored, source, target, &map, pixel);
        }
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
        columns: usize,
    ) -> Result<(), RasterError> {
        let Some(destination) = self.rgb_scratch.get_mut(..pixels) else {
            return Err(RasterError::Extent);
        };
        map_colour(
            transform,
            &ColourWindow::initial(),
            stored,
            destination,
            columns,
        )
    }

    fn write_mono(
        &mut self,
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
        if self.interpolation == Interpolation::Linear {
            return self.write_mono_linear(presentation, stored, source, target, map, out);
        }
        let table = match presentation.colormap() {
            Colormap::Grey => &self.grey,
            Colormap::Stops => &self.stops,
        };
        let chain = presentation.chain();
        let ramp = crate::colour::linear_byte_ramp(chain);
        self.shade(source, target, map, out, |texel| {
            rgba_of_preset(table, chain, stored, texel, ramp)
        })
    }

    fn write_color(
        &mut self,
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        out: &mut [u8],
    ) -> Result<(), RasterError> {
        if self.interpolation == Interpolation::Linear {
            return self.write_color_linear(source, target, map, out);
        }
        let scratch = &self.rgb_scratch;
        self.shade(source, target, map, out, |texel| {
            let rgb = scratch.get(texel).copied().unwrap_or(Rgb::BLACK);
            Ok(rgba_from_rgb(rgb))
        })
    }

    fn write_mono_linear(
        &mut self,
        presentation: &Presentation,
        stored: &[Stored],
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        out: &mut [u8],
    ) -> Result<(), RasterError> {
        self.fill_display(presentation, stored)?;
        let ramp = crate::colour::linear_byte_ramp(presentation.chain());
        let count = stored.len();
        let mut y = 0_u32;
        while y < target[1] {
            let css_y = css_of(y, target[1], self.device[1], self.dpr)?;
            let mut x = 0_u32;
            while x < target[0] {
                let css_x = css_of(x, target[0], self.device[0], self.dpr)?;
                let rgba = match self.blended_display(source[0], 0, count, map, css_x, css_y)? {
                    None => OPAQUE_BLACK,
                    Some(display) => {
                        let table = match presentation.colormap() {
                            Colormap::Grey => &self.grey,
                            Colormap::Stops => &self.stops,
                        };
                        let rgb =
                            crate::colour::preset_rgb(table, presentation.chain(), display, ramp)
                                .map_err(|error| match error {
                                ColourLutError::Length => RasterError::Length,
                                ColourLutError::Unavailable => RasterError::Unavailable,
                            })?;
                        rgba_from_rgb(rgb)
                    }
                };
                write_pixel(out, pixel_index(x, y, target[0])?, rgba)?;
                x = x.saturating_add(1);
            }
            y = y.saturating_add(1);
        }
        Ok(())
    }

    fn write_color_linear(
        &mut self,
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        out: &mut [u8],
    ) -> Result<(), RasterError> {
        let pixels = pixel_count(source)?;
        self.fill_rgb_planes(pixels)?;
        let mut y = 0_u32;
        while y < target[1] {
            let css_y = css_of(y, target[1], self.device[1], self.dpr)?;
            let mut x = 0_u32;
            while x < target[0] {
                let css_x = css_of(x, target[0], self.device[0], self.dpr)?;
                let red = self.blended_display(source[0], 0, pixels, map, css_x, css_y)?;
                let green = self.blended_display(source[0], pixels, pixels, map, css_x, css_y)?;
                let blue = self.blended_display(
                    source[0],
                    pixels.saturating_mul(2),
                    pixels,
                    map,
                    css_x,
                    css_y,
                )?;
                let rgba = match (red, green, blue) {
                    (Some(r), Some(g), Some(b)) => rgba_from_rgb(Rgb { r, g, b }),
                    _ => OPAQUE_BLACK,
                };
                write_pixel(out, pixel_index(x, y, target[0])?, rgba)?;
                x = x.saturating_add(1);
            }
            y = y.saturating_add(1);
        }
        Ok(())
    }

    fn display_linear(
        &self,
        presentation: &Presentation,
        stored: &[Stored],
        source: [u32; 2],
        target: [u32; 2],
        map: &Transform<Canvas, Index>,
        pixel: [u32; 2],
    ) -> Result<Option<f32>, RasterError> {
        if pixel[0] >= target[0] || pixel[1] >= target[1] {
            return Err(RasterError::Extent);
        }
        let css_x = css_of(pixel[0], target[0], self.device[0], self.dpr)?;
        let css_y = css_of(pixel[1], target[1], self.device[1], self.dpr)?;
        let (x, y) = centre_sample(map, css_x, css_y);
        let blended = blend_points(source[0], source[1], x, y, |column, row| {
            let texel = pixel_index(column, row, source[0]).ok()?;
            display_of(presentation, stored, texel).ok()
        });
        match blended {
            Ok(value) => Ok(Some(value)),
            Err(InterpolationError::Outside) => Ok(None),
            Err(InterpolationError::Unavailable) => Err(RasterError::Unavailable),
        }
    }

    fn fill_display(
        &mut self,
        presentation: &Presentation,
        stored: &[Stored],
    ) -> Result<(), RasterError> {
        if self.display_scratch.len() < stored.len() {
            return Err(RasterError::Extent);
        }
        let chain = presentation.chain();
        for (index, sample) in stored.iter().copied().enumerate() {
            let Some(slot) = self.display_scratch.get_mut(index) else {
                return Err(RasterError::Length);
            };
            *slot = chain.apply(sample).0;
        }
        Ok(())
    }

    fn fill_rgb_planes(&mut self, pixels: usize) -> Result<(), RasterError> {
        let needed = pixels.checked_mul(3).ok_or(RasterError::Extent)?;
        if self.display_scratch.len() < needed {
            return Err(RasterError::Extent);
        }
        for index in 0..pixels {
            let rgb = self
                .rgb_scratch
                .get(index)
                .copied()
                .ok_or(RasterError::Length)?;
            let Some(red) = self.display_scratch.get_mut(index) else {
                return Err(RasterError::Length);
            };
            *red = rgb.r;
            let green_at = index.saturating_add(pixels);
            let Some(green) = self.display_scratch.get_mut(green_at) else {
                return Err(RasterError::Length);
            };
            *green = rgb.g;
            let blue_at = index.saturating_add(pixels.saturating_mul(2));
            let Some(blue) = self.display_scratch.get_mut(blue_at) else {
                return Err(RasterError::Length);
            };
            *blue = rgb.b;
        }
        Ok(())
    }

    fn blended_display(
        &self,
        width: u32,
        offset: usize,
        count: usize,
        map: &Transform<Canvas, Index>,
        css_x: f64,
        css_y: f64,
    ) -> Result<Option<f32>, RasterError> {
        let end = offset.checked_add(count).ok_or(RasterError::Length)?;
        let samples = self
            .display_scratch
            .get(offset..end)
            .ok_or(RasterError::Length)?;
        let (x, y) = centre_sample(map, css_x, css_y);
        match blend_display(width, samples, x, y, Interpolation::Linear) {
            Ok(value) => Ok(Some(value)),
            Err(InterpolationError::Outside) => Ok(None),
            Err(InterpolationError::Unavailable) => Err(RasterError::Unavailable),
        }
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
            let css_y = css_of(y, target[1], self.device[1], self.dpr)?;
            let mut x = 0_u32;
            while x < target[0] {
                let css_x = css_of(x, target[0], self.device[0], self.dpr)?;
                let rgba = match self.sample_at(source, map, css_x, css_y)? {
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
        self.sample_at(source, map, css_x, css_y)
    }

    fn sample_at(
        &self,
        source: [u32; 2],
        map: &Transform<Canvas, Index>,
        css_x: f64,
        css_y: f64,
    ) -> Result<Option<usize>, RasterError> {
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

/// Three display bytes per pixel, `R G B` row major, for one band of rows.
///
/// F-239. The same accept, map and byte steps as
/// [`CpuRaster::raster_color_into`], with no view, no resampling and no
/// generation, and `window` applied per channel between the map and the byte
/// step, which is deviation D-29's place for it. `stored` is the band's
/// samples for the resolved colour space and `out` is three bytes for each of
/// its pixels, so a band is `out.len() / 3` pixels and must be whole rows of
/// `columns`. `scratch` is grown to the band when it is smaller and is never
/// shrunk, so a caller drawing a frame band by band allocates once.
///
/// # Errors
///
/// Nothing is written to `out` on any refusal.
/// [`RasterError::Unavailable`] for [`RasterColor::Unsupported`], or a stored
/// colour sample that is not a display byte after the transform's sample
/// scale. [`RasterError::OutputLength`] when `out` is not three bytes per
/// pixel. [`RasterError::Extent`] for an empty band, zero columns, a band
/// that is not whole rows, or scratch that cannot be reserved.
/// [`RasterError::Length`] when `stored` is not the band for the resolved
/// space.
pub fn colour_bytes_into(
    color: RasterColor<'_>,
    window: &ColourWindow,
    stored: &[Stored],
    columns: u32,
    scratch: &mut Vec<Rgb>,
    out: &mut [u8],
) -> Result<(), RasterError> {
    let transform = match color {
        RasterColor::Unsupported => return Err(RasterError::Unavailable),
        RasterColor::Supported(accepted) => accepted.transform,
    };
    if !out.len().is_multiple_of(3) {
        return Err(RasterError::OutputLength);
    }
    let pixels = out.len() / 3;
    let columns = usize::try_from(columns).map_err(|_| RasterError::Extent)?;
    if pixels == 0 || columns == 0 || !pixels.is_multiple_of(columns) {
        return Err(RasterError::Extent);
    }
    transform
        .accept_source(stored, pixels, columns)
        .map_err(color_error)?;
    if !stored_colour_is_a_display_byte(transform, stored) {
        return Err(RasterError::Unavailable);
    }
    if scratch.len() < pixels {
        scratch
            .try_reserve_exact(pixels - scratch.len())
            .map_err(|_| RasterError::Extent)?;
        scratch.resize(pixels, Rgb::BLACK);
    }
    let destination = scratch.get_mut(..pixels).ok_or(RasterError::Extent)?;
    map_colour(transform, window, stored, destination, columns)?;
    for (rgb, bytes) in destination.iter().zip(out.chunks_exact_mut(3)) {
        bytes.copy_from_slice(&rgb_bytes(*rgb));
    }
    Ok(())
}

/// Stage 4, then the colour window, into caller-provided RGB.
///
/// The map step both colour entries share. [`CpuRaster::raster_color_into`]
/// passes [`ColourWindow::initial`], which is skipped, so its frame is the
/// stage 4 output as it was before F-239.
fn map_colour(
    transform: &ColorTransform,
    window: &ColourWindow,
    stored: &[Stored],
    destination: &mut [Rgb],
    columns: usize,
) -> Result<(), RasterError> {
    transform
        .map_into(stored, destination, columns)
        .map_err(color_error)?;
    window.apply_in_place(destination);
    Ok(())
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

/// Centre-origin coordinate. Index space already puts `0` on the texel centre.
fn centre_sample(map: &Transform<Canvas, Index>, css_x: f64, css_y: f64) -> (f32, f32) {
    let index = map.apply(Pt::<Canvas>::new(css_x, css_y, 0.0));
    (
        ocelli_viewport::view::f32_from_f64(index.x),
        ocelli_viewport::view::f32_from_f64(index.y),
    )
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
    ramp: bool,
) -> Result<[u8; 4], RasterError> {
    let sample = stored.get(texel).copied().ok_or(RasterError::Length)?;
    let mut rgb = [Rgb::BLACK];
    table
        .map_preset(chain, &[sample], &mut rgb, ramp)
        .map_err(|error| match error {
            ColourLutError::Length => RasterError::Length,
            ColourLutError::Unavailable => RasterError::Unavailable,
        })?;
    Ok(rgba_from_rgb(rgb.first().copied().unwrap_or(Rgb::BLACK)))
}

fn rgba_from_rgb(rgb: Rgb) -> [u8; 4] {
    let [r, g, b] = rgb_bytes(rgb);
    [r, g, b, 255]
}

/// The byte step both colour entries share.
fn rgb_bytes(rgb: Rgb) -> [u8; 3] {
    [
        byte_from_channel(rgb.r),
        byte_from_channel(rgb.g),
        byte_from_channel(rgb.b),
    ]
}

/// Whether every stored colour sample enters stage 4 as a display byte.
///
/// The test is on the value after the transform's F-239 sample scale, which
/// is the value stage 4 reads, so a 12-bit sample of 4095 is accepted as 255
/// and an unscaled 4095 is refused. A palette was checked when it was
/// accepted, and its stored value is an index rather than a sample.
fn stored_colour_is_a_display_byte(transform: &ColorTransform, stored: &[Stored]) -> bool {
    if transform.palette().is_some() {
        return true;
    }
    let scale = transform.sample_scale();
    stored
        .iter()
        .all(|sample| (0.0..=255.0).contains(&scale.apply(sample.0)))
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

/// Device centre of one target pixel, then one division by the device pixel ratio.
///
/// The centre is `(pixel + 0.5) * device / target`. Equal extents skip that
/// ratio and stay on `pixel + 0.5`. That value is exact for every `u32`.
fn css_of(pixel: u32, target: u32, device: u32, dpr: f64) -> Result<f64, RasterError> {
    let target_f = f64_from_u32(target);
    if target_f.to_bits() == 0.0_f64.to_bits() {
        return Err(RasterError::Extent);
    }
    let device_center = if device == target {
        f64_from_u32(pixel) + 0.5
    } else {
        (f64_from_u32(pixel) + 0.5) * f64_from_u32(device) / target_f
    };
    css_from_device(device_center, dpr).map_err(|_| RasterError::Canvas)
}

pub(crate) fn texel_of(image: f64, limit: u32) -> Option<u32> {
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

/// Nearest byte in `0..=255`. A tie picks the EVEN byte.
///
/// Round half to even is the operator's decision of 2026-11-07 in F-239's
/// plan. It is `Uint8ClampedArray`'s rule, so cornerstone3D 5.8.2 stores the
/// same byte for every colour channel it windows. It was "ties pick the
/// higher byte" until then. Tier C's monochrome draws reach this function
/// only with a preset table's own bytes, which are whole codes, so the
/// monochrome tie decision is the table address in `colour.rs` and not this.
///
/// The pinned standard library rounds to the nearest integer with ties to
/// even. The guards keep every non-finite channel at zero, including positive
/// infinity. For the remaining positive value below 255, the rounded integer
/// is exactly representable and lies in 0 through 255. The shared exact-integer
/// reader and checked conversion therefore preserve that integer unchanged.
/// This final store does not change the separate monochrome table address.
fn byte_from_channel(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    if value >= 255.0 {
        return 255;
    }
    match exact_f32_integer(value.round_ties_even()) {
        Ok(integer) => u8::try_from(integer.magnitude).unwrap_or(0),
        Err(_) => 0,
    }
}

fn f64_from_u32(value: u32) -> f64 {
    let high = u16::try_from(value >> 16).unwrap_or(0);
    let low = u16::try_from(value & 0xffff).unwrap_or(0);
    f64::from(high) * 65_536.0 + f64::from(low)
}

#[cfg(test)]
mod tests {
    use super::byte_from_channel;

    #[test]
    fn non_finite_channels_keep_the_final_byte_policy() {
        // The existing store policy sends every non-finite value to zero.
        // Positive infinity distinguishes this from a saturating byte cast
        // or a clamp performed before the non-finite guard.
        for bits in [
            0x7f80_0000,
            0xff80_0000, // positive and negative infinity
            0x7fc0_0000,
            0xffc0_0000, // quiet NaNs, both signs
            0x7f80_0001,
            0xff80_0001, // nonzero signalling NaN payloads
        ] {
            assert_eq!(byte_from_channel(f32::from_bits(bits)), 0);
        }
        // IEEE-754 zero, subnormal and finite endpoints, with no float cast.
        for bits in [0x0000_0000, 0x8000_0000, 0x0000_0001, 0x8000_0001] {
            assert_eq!(byte_from_channel(f32::from_bits(bits)), 0);
        }
        assert_eq!(byte_from_channel(f32::MAX), 255);
        assert_eq!(byte_from_channel(-f32::MAX), 0);
    }

    /// Nearest code, and a tie to the EVEN code, the operator's decision of
    /// 2026-11-07 in F-239's plan, which is `Uint8ClampedArray`'s rule and so
    /// cornerstone3D 5.8.2's for the colour bytes its CPU path stores. Tier C
    /// monochrome reaches this rule only with whole table bytes.
    ///
    /// ```text
    /// tie    0.5 -> 0    1.5 -> 2    2.5 -> 2    3.5 -> 4
    ///      127.5 -> 128  128.5 -> 128  253.5 -> 254  254.5 -> 254
    /// ```
    ///
    /// Each tie is paired with the float either side of it, which is not a tie
    /// and goes to the nearer code whatever the tie rule says.
    #[test]
    fn a_tie_rounds_to_the_even_channel_byte() {
        // (tie, the even code it stores, the code just below it, just above)
        for (tie, even, lower, upper) in [
            (0.5_f32, 0_u8, 0_u8, 1_u8),
            (1.5, 2, 1, 2),
            (2.5, 2, 2, 3),
            (3.5, 4, 3, 4),
            (127.5, 128, 127, 128),
            (128.5, 128, 128, 129),
            (253.5, 254, 253, 254),
            (254.5, 254, 254, 255),
        ] {
            assert_eq!(byte_from_channel(tie), even, "tie {tie}");
            let below = f32::from_bits(tie.to_bits() - 1);
            let above = f32::from_bits(tie.to_bits() + 1);
            assert_eq!(byte_from_channel(below), lower, "below {tie}");
            assert_eq!(byte_from_channel(above), upper, "above {tie}");
        }
        assert_eq!(byte_from_channel(1.0), 1);
        assert_eq!(byte_from_channel(254.6), 255);
        assert_eq!(byte_from_channel(-0.0), 0);
        assert_eq!(byte_from_channel(f32::NAN), 0);
        assert_eq!(byte_from_channel(-1.0), 0);
        assert_eq!(byte_from_channel(255.0), 255);
        assert_eq!(byte_from_channel(256.0), 255);
    }
}
