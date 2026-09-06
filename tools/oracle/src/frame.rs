//! RGBA8 frames and the difference distribution over two of them.
//!
//! This is the pixel arithmetic of F-011, so HLD 27.2 R3 applies in full and
//! `tests/tolerance_fixture.rs` is not optional. Nothing here knows about
//! DICOM, about a window function or about a tolerance. It counts differences
//! and it partitions them, and every judgement is made in `tolerance.rs`
//! against the numbers this module produces.
//!
//! Three things are deliberate.
//!
//! **Alpha never enters a difference.** The reference records `opaque` per
//! frame and the loader refuses a frame that is not fully opaque, because a
//! difference in alpha is a difference in the canvas rather than in the image.
//! So the compared lanes are red, green and blue, and never the fourth.
//!
//! **The histogram is the whole statistic.** 25.1 needs counts at 0, 1 and 2
//! and a count over 2, class two needs a high percentile, and all four fall
//! out of one array of 256 counters. Keeping the histogram rather than four
//! counters is what lets a later story ask a question nobody has asked yet
//! without a second pass over the corpus.
//!
//! **There is no `as` cast in this file.** `cast_possible_truncation`,
//! `cast_precision_loss` and `cast_sign_loss` are denied at the workspace and
//! no `#[allow]` is added, so every count that becomes a float goes through
//! `u32::try_from` and `f64::from`, which is exact over the whole of `u32`,
//! and a count too large to convert is an error rather than a wrong number.

use sha2::{Digest, Sha256};
use thiserror::Error;

/// The number of bytes one RGBA8 pixel occupies.
const BYTES_PER_PIXEL: u32 = 4;

/// The alpha lane. Never compared.
const ALPHA_LANE: usize = 3;

/// Signed display-code differences from -255 through 255, inclusive.
const SIGNED_DIFFERENCE_BINS: usize = 511;

/// The array index corresponding to a signed difference of zero.
const SIGNED_DIFFERENCE_OFFSET: i32 = 255;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error(
        "frame is {actual} bytes, and {width} by {height} RGBA8 is {expected}. \
         A `.raw` whose length is not width * height * 4 is not the frame the \
         sidecar describes"
    )]
    Length {
        width: u32,
        height: u32,
        expected: u64,
        actual: u64,
    },
    #[error("frame dimension {0} by {1} overflows the addressable pixel count")]
    Dimensions(u32, u32),
    #[error(
        "rectangle {x0},{y0} {width} by {height} does not fit inside a {frame_width} \
         by {frame_height} frame"
    )]
    RectOutsideFrame {
        x0: u32,
        y0: u32,
        width: u32,
        height: u32,
        frame_width: u32,
        frame_height: u32,
    },
    #[error(
        "the two sides disagree about the frame size: reference is {a_width} by \
         {a_height} and candidate is {b_width} by {b_height}. Two frames of \
         different sizes are two different pictures and there is nothing to \
         compare"
    )]
    SizeDisagreement {
        a_width: u32,
        a_height: u32,
        b_width: u32,
        b_height: u32,
    },
    #[error("pixel {0},{1} is outside the frame")]
    PixelOutside(u32, u32),
    #[error("{0}")]
    Stats(#[from] StatsError),
}

#[derive(Debug, Error)]
pub enum StatsError {
    #[error(
        "a count of {0} does not convert exactly to a double. The largest count \
         a frame can produce is its pixel count, and a frame with over four \
         billion pixels is not something this instrument measures"
    )]
    CountTooLarge(u64),
    #[error(
        "a signed difference sum of {0} does not convert exactly to a double. \
         The bound is the pixel count times 255"
    )]
    SumTooLarge(i64),
    #[error("a statistic over zero pixels has no value, and reporting one would invent evidence")]
    NoPixels,
    #[error("a percentile of {0} is not a fraction between 0 and 1")]
    NotAFraction(f64),
}

/// Exact for every value a frame can produce. Every `u32` is exactly
/// representable in `f64`, so this conversion loses nothing, and the
/// `try_from` is what makes that claim true rather than assumed.
fn count_to_f64(count: u64) -> Result<f64, StatsError> {
    u32::try_from(count)
        .map(f64::from)
        .map_err(|_| StatsError::CountTooLarge(count))
}

/// Exact for the same reason. The signed sum is bounded by the pixel count
/// times 255, so `i32` covers every frame up to eight million pixels and a
/// larger one is refused rather than rounded.
fn signed_to_f64(sum: i64) -> Result<f64, StatsError> {
    i32::try_from(sum)
        .map(f64::from)
        .map_err(|_| StatsError::SumTooLarge(sum))
}

/// Which lanes of the RGBA quadruple are compared.
///
/// Monochrome is one lane and not three because a `mono16` frame has red equal
/// to green equal to blue on every pixel, which the loader asserts, so three
/// identical channels would give three identical statistics and one verdict
/// counted three times. Class two is three lanes, because chroma subsampling
/// and YBR conversion move the channels differently and a single number would
/// hide which one moved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelSet {
    Monochrome,
    Rgb,
}

impl ChannelSet {
    #[must_use]
    pub fn lanes(self) -> &'static [usize] {
        match self {
            Self::Monochrome => &[0],
            Self::Rgb => &[0, 1, 2],
        }
    }

    #[must_use]
    pub fn count(self) -> usize {
        self.lanes().len()
    }
}

/// A rectangle of canvas pixels, in pixel indices, half-open on the far edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x0: u32,
    pub y0: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// # Errors
    /// When the rectangle's far edge overflows `u32`.
    pub fn new(x0: u32, y0: u32, width: u32, height: u32) -> Result<Self, FrameError> {
        x0.checked_add(width)
            .and_then(|_| y0.checked_add(height))
            .ok_or(FrameError::Dimensions(width, height))?;
        Ok(Self {
            x0,
            y0,
            width,
            height,
        })
    }

    #[must_use]
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x0: 0,
            y0: 0,
            width,
            height,
        }
    }

    #[must_use]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x0
            && y >= self.y0
            && x.saturating_sub(self.x0) < self.width
            && y.saturating_sub(self.y0) < self.height
    }

    #[must_use]
    pub fn pixels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// One lane's difference distribution over one region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelStats {
    pixels: u64,
    histogram: Box<[u64; 256]>,
    signed_histogram: Box<[u64; SIGNED_DIFFERENCE_BINS]>,
    signed_sum: i64,
}

impl ChannelStats {
    fn new() -> Self {
        Self {
            pixels: 0,
            histogram: Box::new([0; 256]),
            signed_histogram: Box::new([0; SIGNED_DIFFERENCE_BINS]),
            signed_sum: 0,
        }
    }

    fn add(&mut self, absolute: u8, signed: i16) {
        self.pixels = self.pixels.saturating_add(1);
        if let Some(slot) = self.histogram.get_mut(usize::from(absolute)) {
            *slot = slot.saturating_add(1);
        }
        if let Ok(index) = usize::try_from(i32::from(signed) + SIGNED_DIFFERENCE_OFFSET)
            && let Some(slot) = self.signed_histogram.get_mut(index)
        {
            *slot = slot.saturating_add(1);
        }
        self.signed_sum = self.signed_sum.saturating_add(i64::from(signed));
    }

    #[must_use]
    pub fn pixels(&self) -> u64 {
        self.pixels
    }

    #[must_use]
    pub fn histogram(&self) -> &[u64; 256] {
        &self.histogram
    }

    #[must_use]
    pub fn signed_count_at(&self, difference: i16) -> u64 {
        usize::try_from(i32::from(difference) + SIGNED_DIFFERENCE_OFFSET)
            .ok()
            .and_then(|index| self.signed_histogram.get(index))
            .copied()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn signed_sum(&self) -> i64 {
        self.signed_sum
    }

    /// The largest absolute difference present. Zero on an empty region, which
    /// is why every caller that gates also reads `pixels`.
    #[must_use]
    pub fn max_abs_diff(&self) -> u8 {
        for difference in (0..=u8::MAX).rev() {
            if self.count_at(difference) > 0 {
                return difference;
            }
        }
        0
    }

    #[must_use]
    pub fn count_at(&self, difference: u8) -> u64 {
        self.histogram
            .get(usize::from(difference))
            .copied()
            .unwrap_or(0)
    }

    /// The number of pixels whose absolute difference is at most `difference`.
    #[must_use]
    pub fn count_within(&self, difference: u8) -> u64 {
        (0..=difference).map(|d| self.count_at(d)).sum()
    }

    /// The number of pixels whose absolute difference EXCEEDS `difference`.
    #[must_use]
    pub fn count_over(&self, difference: u8) -> u64 {
        self.pixels.saturating_sub(self.count_within(difference))
    }

    /// # Errors
    /// When the region is empty, because a fraction over no pixels is not zero,
    /// it is undefined, and reporting zero would invent evidence.
    pub fn fraction_within(&self, difference: u8) -> Result<f64, StatsError> {
        if self.pixels == 0 {
            return Err(StatsError::NoPixels);
        }
        Ok(count_to_f64(self.count_within(difference))? / count_to_f64(self.pixels)?)
    }

    /// The fraction of pixels differing at all, `|d| >= 1`.
    ///
    /// # Errors
    /// When the region is empty.
    pub fn differing_fraction(&self) -> Result<f64, StatsError> {
        if self.pixels == 0 {
            return Err(StatsError::NoPixels);
        }
        Ok(count_to_f64(self.count_over(0))? / count_to_f64(self.pixels)?)
    }

    /// Candidate minus reference, averaged over the region.
    ///
    /// # Errors
    /// When the region is empty.
    pub fn signed_mean_diff(&self) -> Result<f64, StatsError> {
        if self.pixels == 0 {
            return Err(StatsError::NoPixels);
        }
        Ok(signed_to_f64(self.signed_sum)? / count_to_f64(self.pixels)?)
    }

    /// The smallest absolute difference at or below which at least `fraction`
    /// of the region's pixels lie.
    ///
    /// # Errors
    /// When the region is empty, or the fraction is not between 0 and 1.
    pub fn percentile_abs_diff(&self, fraction: f64) -> Result<u8, StatsError> {
        if !(0.0..=1.0).contains(&fraction) || fraction.is_nan() {
            return Err(StatsError::NotAFraction(fraction));
        }
        if self.pixels == 0 {
            return Err(StatsError::NoPixels);
        }
        let total = count_to_f64(self.pixels)?;
        let mut running = 0_u64;
        for difference in 0..=u8::MAX {
            running = running.saturating_add(self.count_at(difference));
            if count_to_f64(running)? / total >= fraction {
                return Ok(difference);
            }
        }
        Ok(u8::MAX)
    }
}

/// One region's statistics, one entry per compared lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionStats {
    channels: Vec<ChannelStats>,
}

impl RegionStats {
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&ChannelStats> {
        self.channels.get(index)
    }

    #[must_use]
    pub fn channels(&self) -> &[ChannelStats] {
        &self.channels
    }

    /// The largest absolute difference across every lane.
    #[must_use]
    pub fn max_abs_diff(&self) -> u8 {
        self.channels
            .iter()
            .map(ChannelStats::max_abs_diff)
            .max()
            .unwrap_or(0)
    }
}

/// The full difference record for one pair of frames.
///
/// Four regions, computed in a single pass so that no two of them can be taken
/// over different readings of the same buffers.
#[derive(Clone, Debug)]
pub struct FrameDifference {
    /// Every pixel of the frame. This is the region 25.1's monochrome rule is
    /// evaluated over: "of pixels", excluding nothing.
    pub full: RegionStats,
    /// The image rectangle. This is NOT the region 25.1's bias bullet names,
    /// which is `informative` below. It is the rectangle that bounds it, and
    /// it is the denominator of `informative_fraction`.
    pub image: RegionStats,
    /// The letterbox. Both sides paint the declared clear colour here, so a
    /// difference in this region is a difference in the fit and not in the
    /// picture.
    pub background: RegionStats,
    /// The image-rectangle pixels that are not clipped to the same extreme on
    /// both sides. The subset a saturated frame has any evidence in.
    pub informative: RegionStats,
    /// Distinct canvas rows carrying any difference. A resampling phase error
    /// touches whole rows and columns, a LUT error is scattered, and that
    /// distinction is the useful signal on a decimated frame.
    pub rows_touched: u32,
    pub columns_touched: u32,
    pub image_pixels: u64,
    pub informative_pixels: u64,
}

impl FrameDifference {
    /// # Errors
    /// When the image rectangle is empty.
    pub fn informative_fraction(&self) -> Result<f64, StatsError> {
        if self.image_pixels == 0 {
            return Err(StatsError::NoPixels);
        }
        Ok(count_to_f64(self.informative_pixels)? / count_to_f64(self.image_pixels)?)
    }
}

/// An RGBA8 canvas frame, top row first, exactly as the reference half's
/// `<id>.raw` holds it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

impl Frame {
    /// # Errors
    /// When the byte count is not `width * height * 4`.
    pub fn new(width: u32, height: u32, bytes: Vec<u8>) -> Result<Self, FrameError> {
        let expected = u64::from(width) * u64::from(height) * u64::from(BYTES_PER_PIXEL);
        let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if actual != expected {
            return Err(FrameError::Length {
                width,
                height,
                expected,
                actual,
            });
        }
        Ok(Self {
            width,
            height,
            bytes,
        })
    }

    /// Build a frame from one grey value per pixel, opaque. Used by the
    /// fixtures and by the mutation catalogue, never by the loader.
    ///
    /// # Errors
    /// When the grey count is not `width * height`.
    pub fn from_monochrome(width: u32, height: u32, greys: &[u8]) -> Result<Self, FrameError> {
        let mut bytes = Vec::with_capacity(greys.len().saturating_mul(4));
        for grey in greys {
            bytes.extend_from_slice(&[*grey, *grey, *grey, u8::MAX]);
        }
        Self::new(width, height, bytes)
    }

    #[must_use]
    pub fn black(width: u32, height: u32) -> Self {
        let count = usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(0);
        let mut bytes = Vec::with_capacity(count.saturating_mul(4));
        for _ in 0..count {
            bytes.extend_from_slice(&[0, 0, 0, u8::MAX]);
        }
        Self {
            width,
            height,
            bytes,
        }
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn offset(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let linear = u64::from(y) * u64::from(self.width) + u64::from(x);
        usize::try_from(linear.checked_mul(u64::from(BYTES_PER_PIXEL))?).ok()
    }

    /// # Errors
    /// When the coordinate is outside the frame.
    pub fn pixel(&self, x: u32, y: u32) -> Result<[u8; 4], FrameError> {
        let at = self.offset(x, y).ok_or(FrameError::PixelOutside(x, y))?;
        let slice = self
            .bytes
            .get(at..at.saturating_add(4))
            .ok_or(FrameError::PixelOutside(x, y))?;
        <[u8; 4]>::try_from(slice).map_err(|_| FrameError::PixelOutside(x, y))
    }

    /// # Errors
    /// When the coordinate is outside the frame.
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: [u8; 4]) -> Result<(), FrameError> {
        let at = self.offset(x, y).ok_or(FrameError::PixelOutside(x, y))?;
        let slice = self
            .bytes
            .get_mut(at..at.saturating_add(4))
            .ok_or(FrameError::PixelOutside(x, y))?;
        slice.copy_from_slice(&pixel);
        Ok(())
    }

    /// The first pixel whose alpha is not 255, if there is one.
    #[must_use]
    pub fn first_non_opaque(&self) -> Option<(u32, u32)> {
        self.find_pixel(|pixel| pixel.get(ALPHA_LANE).copied().unwrap_or(0) != u8::MAX)
    }

    /// The first pixel whose red, green and blue are not all equal, if there
    /// is one. A `mono16` token over a frame this answers `Some` for means the
    /// token is lying.
    #[must_use]
    pub fn first_non_monochrome(&self) -> Option<(u32, u32)> {
        self.find_pixel(|pixel| {
            let (Some(r), Some(g), Some(b)) = (pixel.first(), pixel.get(1), pixel.get(2)) else {
                return true;
            };
            r != g || g != b
        })
    }

    fn find_pixel(&self, predicate: impl Fn(&[u8; 4]) -> bool) -> Option<(u32, u32)> {
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = self.pixel(x, y).ok()?;
                if predicate(&pixel) {
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// The third hash of the same bytes.
    ///
    /// The reference half already hashes each frame twice, in the page before
    /// the bytes leave the browser and in the driver over what arrived. This
    /// one is taken at read, so a file edited after the run is refused rather
    /// than compared as though somebody had rendered it.
    #[must_use]
    pub fn sha256_hex(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.bytes);
        let digest = hasher.finalize();
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

/// Per-lane accumulators for the four regions, filled in one pass.
struct LaneAccumulators {
    full: ChannelStats,
    image: ChannelStats,
    background: ChannelStats,
    informative: ChannelStats,
}

impl LaneAccumulators {
    fn new() -> Self {
        Self {
            full: ChannelStats::new(),
            image: ChannelStats::new(),
            background: ChannelStats::new(),
            informative: ChannelStats::new(),
        }
    }
}

/// A pixel is at an extreme when it is clipped to black or to white. A pixel
/// clipped to the SAME extreme on both sides carries no evidence: it agrees by
/// construction and a defect in the value behind it cannot show.
fn clipped_to_the_same_extreme(reference: u8, candidate: u8) -> bool {
    reference == candidate && (reference == 0 || reference == u8::MAX)
}

/// The difference distribution over two frames, partitioned four ways.
///
/// # Errors
/// When the two frames disagree about their size, when the rectangle does not
/// fit inside them, or when a pixel read fails.
pub fn difference(
    reference: &Frame,
    candidate: &Frame,
    image: Rect,
    channels: ChannelSet,
) -> Result<FrameDifference, FrameError> {
    if reference.width != candidate.width || reference.height != candidate.height {
        return Err(FrameError::SizeDisagreement {
            a_width: reference.width,
            a_height: reference.height,
            b_width: candidate.width,
            b_height: candidate.height,
        });
    }
    if image.x0.saturating_add(image.width) > reference.width
        || image.y0.saturating_add(image.height) > reference.height
    {
        return Err(FrameError::RectOutsideFrame {
            x0: image.x0,
            y0: image.y0,
            width: image.width,
            height: image.height,
            frame_width: reference.width,
            frame_height: reference.height,
        });
    }

    let lanes = channels.lanes();
    let mut accumulators: Vec<LaneAccumulators> =
        (0..lanes.len()).map(|_| LaneAccumulators::new()).collect();
    let mut rows_touched = vec![false; usize::try_from(reference.height).unwrap_or(0)];
    let mut columns_touched = vec![false; usize::try_from(reference.width).unwrap_or(0)];
    let mut image_pixels = 0_u64;
    let mut informative_pixels = 0_u64;

    for y in 0..reference.height {
        for x in 0..reference.width {
            let left = reference.pixel(x, y)?;
            let right = candidate.pixel(x, y)?;
            let inside = image.contains(x, y);
            let mut informative = false;
            let mut differs = false;

            for (accumulator, lane) in accumulators.iter_mut().zip(lanes.iter()) {
                let (Some(&a), Some(&b)) = (left.get(*lane), right.get(*lane)) else {
                    return Err(FrameError::PixelOutside(x, y));
                };
                let absolute = a.abs_diff(b);
                let signed = i16::from(b) - i16::from(a);
                accumulator.full.add(absolute, signed);
                if inside {
                    accumulator.image.add(absolute, signed);
                } else {
                    accumulator.background.add(absolute, signed);
                }
                if absolute != 0 {
                    differs = true;
                }
                if !clipped_to_the_same_extreme(a, b) {
                    informative = true;
                }
            }

            if inside {
                image_pixels = image_pixels.saturating_add(1);
                if informative {
                    informative_pixels = informative_pixels.saturating_add(1);
                    for (accumulator, lane) in accumulators.iter_mut().zip(lanes.iter()) {
                        let (Some(&a), Some(&b)) = (left.get(*lane), right.get(*lane)) else {
                            return Err(FrameError::PixelOutside(x, y));
                        };
                        accumulator
                            .informative
                            .add(a.abs_diff(b), i16::from(b) - i16::from(a));
                    }
                }
            }

            if differs {
                if let Some(row) = rows_touched.get_mut(usize::try_from(y).unwrap_or(0)) {
                    *row = true;
                }
                if let Some(column) = columns_touched.get_mut(usize::try_from(x).unwrap_or(0)) {
                    *column = true;
                }
            }
        }
    }

    let count_true = |flags: &[bool]| -> u32 {
        u32::try_from(flags.iter().filter(|flag| **flag).count()).unwrap_or(u32::MAX)
    };

    Ok(FrameDifference {
        full: RegionStats {
            channels: accumulators.iter().map(|a| a.full.clone()).collect(),
        },
        image: RegionStats {
            channels: accumulators.iter().map(|a| a.image.clone()).collect(),
        },
        background: RegionStats {
            channels: accumulators.iter().map(|a| a.background.clone()).collect(),
        },
        informative: RegionStats {
            channels: accumulators.iter().map(|a| a.informative.clone()).collect(),
        },
        rows_touched: count_true(&rows_touched),
        columns_touched: count_true(&columns_touched),
        image_pixels,
        informative_pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::{ChannelSet, Frame, Rect, StatsError, difference};

    /// A `.raw` whose length is not `width * height * 4` is refused. This is
    /// the loader refusal the design plan lists, observed red here rather than
    /// described.
    #[test]
    fn a_frame_of_the_wrong_length_is_refused() {
        let outcome = Frame::new(2, 2, vec![0; 15]);
        let Err(error) = outcome else {
            assert!(core::hint::black_box(false), "a short buffer was accepted");
            return;
        };
        assert!(format!("{error}").contains("is 15 bytes"));
    }

    /// Two frames of different sizes have nothing to compare, and saying so is
    /// better than comparing the overlap.
    #[test]
    fn frames_of_different_sizes_are_refused() {
        let (Ok(a), Ok(b)) = (
            Frame::from_monochrome(2, 2, &[0; 4]),
            Frame::from_monochrome(2, 3, &[0; 6]),
        ) else {
            assert!(
                core::hint::black_box(false),
                "the fixture frames did not build"
            );
            return;
        };
        let outcome = difference(&a, &b, Rect::full(2, 2), ChannelSet::Monochrome);
        let Err(error) = outcome else {
            assert!(
                core::hint::black_box(false),
                "two differently sized frames were compared"
            );
            return;
        };
        assert!(format!("{error}").contains("disagree about the frame size"));
    }

    /// A statistic over an empty region is an error and never zero. A
    /// comparator that answered zero here would report a perfect result for a
    /// region it never looked at.
    #[test]
    fn a_fraction_over_no_pixels_is_an_error_and_not_zero() {
        let Ok(frame) = Frame::from_monochrome(2, 2, &[0; 4]) else {
            assert!(
                core::hint::black_box(false),
                "the fixture frame did not build"
            );
            return;
        };
        let Ok(diff) = difference(
            &frame,
            &frame,
            Rect {
                x0: 0,
                y0: 0,
                width: 0,
                height: 0,
            },
            ChannelSet::Monochrome,
        ) else {
            assert!(
                core::hint::black_box(false),
                "the difference did not compute"
            );
            return;
        };
        let Some(image) = diff.image.channel(0) else {
            assert!(core::hint::black_box(false), "no channel 0");
            return;
        };
        assert!(matches!(
            image.fraction_within(1),
            Err(StatsError::NoPixels)
        ));
        assert!(matches!(
            diff.informative_fraction(),
            Err(StatsError::NoPixels)
        ));
    }

    /// Alpha is never compared. Two frames differing only in alpha produce no
    /// difference here, which is what makes the loader's "alpha is 255
    /// everywhere" refusal the thing that catches it rather than a silent
    /// contribution to a colour statistic.
    #[test]
    fn alpha_never_enters_a_difference() {
        let Ok(mut a) = Frame::from_monochrome(2, 2, &[10, 20, 30, 40]) else {
            assert!(
                core::hint::black_box(false),
                "the fixture frame did not build"
            );
            return;
        };
        let b = a.clone();
        let Ok(()) = a.set_pixel(0, 0, [10, 10, 10, 254]) else {
            assert!(core::hint::black_box(false), "the pixel was not set");
            return;
        };
        let Ok(diff) = difference(&a, &b, Rect::full(2, 2), ChannelSet::Rgb) else {
            assert!(
                core::hint::black_box(false),
                "the difference did not compute"
            );
            return;
        };
        assert_eq!(diff.full.max_abs_diff(), 0);
        assert_eq!(a.first_non_opaque(), Some((0, 0)));
        assert_eq!(b.first_non_opaque(), None);
    }

    /// The letterbox and the picture are counted separately, and a difference
    /// in one does not appear in the other.
    #[test]
    fn the_image_rectangle_partitions_the_frame() {
        let Ok(a) = Frame::from_monochrome(4, 1, &[0, 100, 100, 0]) else {
            assert!(
                core::hint::black_box(false),
                "the fixture frame did not build"
            );
            return;
        };
        let Ok(b) = Frame::from_monochrome(4, 1, &[5, 100, 101, 0]) else {
            assert!(
                core::hint::black_box(false),
                "the fixture frame did not build"
            );
            return;
        };
        let Ok(rect) = Rect::new(1, 0, 2, 1) else {
            assert!(core::hint::black_box(false), "the rectangle did not build");
            return;
        };
        let Ok(diff) = difference(&a, &b, rect, ChannelSet::Monochrome) else {
            assert!(
                core::hint::black_box(false),
                "the difference did not compute"
            );
            return;
        };
        let (Some(image), Some(background)) = (diff.image.channel(0), diff.background.channel(0))
        else {
            assert!(core::hint::black_box(false), "no channel 0");
            return;
        };
        assert_eq!(image.pixels(), 2);
        assert_eq!(background.pixels(), 2);
        assert_eq!(image.max_abs_diff(), 1);
        assert_eq!(background.max_abs_diff(), 5);
        assert_eq!(image.signed_count_at(0), 1);
        assert_eq!(image.signed_count_at(1), 1);
        assert_eq!(image.signed_count_at(-1), 0);
        assert_eq!(background.signed_count_at(0), 1);
        assert_eq!(background.signed_count_at(5), 1);
        let Some(informative) = diff.informative.channel(0) else {
            assert!(core::hint::black_box(false), "no informative channel 0");
            return;
        };
        assert_eq!(informative.signed_count_at(1), image.signed_count_at(1));
        assert_eq!(diff.image_pixels, 2);
    }

    /// Pixels clipped to the same extreme on both sides carry no evidence and
    /// are excluded from the informative subset. A frame that is entirely
    /// clipped has an informative fraction of zero.
    #[test]
    fn clipped_pixels_are_not_informative() {
        let Ok(a) = Frame::from_monochrome(4, 1, &[0, 255, 255, 40]) else {
            assert!(
                core::hint::black_box(false),
                "the fixture frame did not build"
            );
            return;
        };
        let b = a.clone();
        let Ok(diff) = difference(&a, &b, Rect::full(4, 1), ChannelSet::Monochrome) else {
            assert!(
                core::hint::black_box(false),
                "the difference did not compute"
            );
            return;
        };
        assert_eq!(diff.informative_pixels, 1, "only the 40 carries evidence");
        assert_eq!(diff.image_pixels, 4);
        let Ok(fraction) = diff.informative_fraction() else {
            assert!(core::hint::black_box(false), "the fraction did not compute");
            return;
        };
        assert_eq!(fraction.to_bits(), 0.25_f64.to_bits());
    }

    /// The 99.9th percentile of the absolute difference, which is what class
    /// two publishes in place of a bound nobody wrote. Ten pixels at 1 and one
    /// thousand at 0 puts the 99.9th percentile at 1, and the 99th at 0.
    #[test]
    fn the_percentile_reads_the_histogram() {
        let mut greys = vec![0_u8; 1000];
        let mut other = greys.clone();
        for index in 0..10 {
            if let Some(slot) = other.get_mut(index) {
                *slot = 1;
            }
        }
        greys.truncate(1000);
        let (Ok(a), Ok(b)) = (
            Frame::from_monochrome(1000, 1, &greys),
            Frame::from_monochrome(1000, 1, &other),
        ) else {
            assert!(
                core::hint::black_box(false),
                "the fixture frames did not build"
            );
            return;
        };
        let Ok(diff) = difference(&a, &b, Rect::full(1000, 1), ChannelSet::Monochrome) else {
            assert!(
                core::hint::black_box(false),
                "the difference did not compute"
            );
            return;
        };
        let Some(stats) = diff.full.channel(0) else {
            assert!(core::hint::black_box(false), "no channel 0");
            return;
        };
        assert_eq!(stats.percentile_abs_diff(0.99).ok(), Some(0));
        assert_eq!(stats.percentile_abs_diff(0.999).ok(), Some(1));
        assert!(matches!(
            stats.percentile_abs_diff(1.5),
            Err(StatsError::NotAFraction(_))
        ));
    }
}
