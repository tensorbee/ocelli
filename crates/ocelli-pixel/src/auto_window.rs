//! The automatic window for a frame or series that declares none.
//!
//! DICOM says nothing about a window for a frame that has none, and
//! `open_frame` invents nothing: `WindowInput::Absent` means the caller
//! chooses. This module is the one implementation of that choice, from the
//! operator's answers of 2026-11-05. The pool measures modality values,
//! PS3.3 C.11.1, each frame through its own stage, leaves out stored zero and
//! the frame's own padding range, PS3.3 C.7.5.1.1.2, and takes the 1st and
//! 99th nearest-rank percentiles as a LINEAR_EXACT window, PS3.3
//! C.11.2.1.3.2. A caller pools a whole series with [`AutoWindowPool`] or one
//! image with [`auto_window`], which is a pool of one.

use alloc::vec::Vec;
use core::cmp::Ordering;

use ocelli_core::{Modality, Stored};

use crate::{ModalityTransform, PixelError, VoiFunction, VoiTransform};

/// Samples read over one pool. Split equally over the declared frames.
pub const AUTO_WINDOW_SAMPLE_CAP: usize = 1 << 20;
/// Frames sampled from one series.
pub const AUTO_WINDOW_SERIES_FRAMES: usize = 16;

/// The highest sample each declared frame may contribute. The cap split, so
/// the pool's total never exceeds [`AUTO_WINDOW_SAMPLE_CAP`].
fn sample_budget(frames: usize) -> usize {
    AUTO_WINDOW_SAMPLE_CAP / frames
}

/// Indices of the sampled images of a series of `images`, first and last
/// included.
///
/// For `M` images, `K = min(M, AUTO_WINDOW_SERIES_FRAMES)` are sampled at
/// `i_k = k * (M - 1) / (K - 1)` in integer division, which keeps the indices
/// strictly increasing because the step is at least 1 when `M >= K`. A series
/// of at most 16 images is sampled in full, and a one-image series is `[0]`.
///
/// # Errors
///
/// [`AutoWindowError::EmptySeries`] for zero images, because there is nothing
/// to sample.
pub fn series_sample(images: usize) -> Result<Vec<usize>, AutoWindowError> {
    if images == 0 {
        return Err(AutoWindowError::EmptySeries);
    }
    let sampled = images.min(AUTO_WINDOW_SERIES_FRAMES);
    let mut indices = Vec::with_capacity(sampled);
    if sampled == 1 {
        indices.push(0);
        return Ok(indices);
    }
    for k in 0..sampled {
        indices.push(k * (images - 1) / (sampled - 1));
    }
    Ok(indices)
}

/// An inclusive stored-value range left out of the window, PS3.3
/// C.7.5.1.1.2. Both bounds are stored-space values, before the modality
/// stage. The bounds are honoured in either declared order, as
/// `ocelli_dicom::PaddingRange::contains` reads the file's value and limit.
#[derive(Clone, Copy, Debug)]
pub struct StoredRange {
    /// One end of the range.
    pub low: Stored,
    /// The other end of the range.
    pub high: Stored,
}

impl PartialEq for StoredRange {
    /// Bitwise equality of the two bounds. The bounds are public f32 fields,
    /// so this is not value equality: negative zero differs from positive
    /// zero, and a NaN range compares equal to itself. `Stored` carries no
    /// `PartialEq` of its own, so the derive cannot reach inside it.
    fn eq(&self, other: &Self) -> bool {
        self.low.0.to_bits() == other.low.0.to_bits()
            && self.high.0.to_bits() == other.high.0.to_bits()
    }
}

fn padding_contains(padding: Option<StoredRange>, sample: Stored) -> bool {
    let Some(range) = padding else {
        return false;
    };
    let low = range.low.0.min(range.high.0);
    let high = range.low.0.max(range.high.0);
    sample.0 >= low && sample.0 <= high
}

fn padding_is_valid(padding: Option<StoredRange>) -> bool {
    let Some(range) = padding else {
        return true;
    };
    range.low.0.is_finite() && range.high.0.is_finite()
}

/// Which counted set produced the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoWindowBasis {
    /// The 1st and 99th nearest-rank percentiles of a non-empty counted set.
    Percentile,
    /// The 1st and 99th nearest-rank percentiles are equal, or fewer than
    /// three f32 units apart, so the width starts at one and the centre is
    /// their f32 midpoint, the value itself when they are equal. The counted
    /// set need not be constant: one outlier among equal values lands here.
    /// Close pairs are refused from about 2^34 = 1.7e10 in magnitude and
    /// constants from just below 2^35 = 3.4e10, at the measured boundaries
    /// [`AutoWindowPool::finish`] records.
    Constant,
    /// Every sampled value left out. The window is the one whose lower side
    /// maps the largest left-out modality value to the output minimum, so
    /// every sample is `ymin`.
    Background,
}

/// A LINEAR_EXACT window from the 1st and 99th nearest-rank percentiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoWindow {
    /// Window Center.
    pub center: f32,
    /// Window Width.
    pub width: f32,
    /// The 1st nearest-rank percentile of the counted values. For
    /// [`AutoWindowBasis::Background`] there are none, and this is the
    /// largest left-out modality value, which the window check maps to the
    /// output minimum.
    pub low: f32,
    /// The 99th nearest-rank percentile of the counted values. For
    /// [`AutoWindowBasis::Background`] this is the window's upper edge,
    /// `center + width / 2`.
    pub high: f32,
    /// Counted values over the pool, after stride and exclusion.
    pub counted: usize,
    /// Frames added.
    pub frames: usize,
    /// Which counted set produced the window.
    pub basis: AutoWindowBasis,
}

impl AutoWindow {
    /// The window as a stage 2 transform, PS3.3 C.11.2.1.3.2.
    ///
    /// # Errors
    ///
    /// [`PixelError::InvalidWindow`] when the centre or width is not finite or
    /// the width violates the function rule, and
    /// [`PixelError::InvalidDisplayRange`] when the range descends.
    pub fn transform(&self, ymin: f32, ymax: f32) -> Result<VoiTransform, PixelError> {
        VoiTransform::new(
            None,
            &[self.center],
            &[self.width],
            0,
            VoiFunction::LinearExact,
            ymin,
            ymax,
        )
    }
}

/// Why a window could not be produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoWindowError {
    /// `series_sample(0)`.
    EmptySeries,
    /// `new(0)` or beyond `AUTO_WINDOW_SERIES_FRAMES`.
    FrameCount,
    /// `add_frame` beyond the declared count.
    TooManyFrames,
    /// An empty `stored` slice.
    EmptyFrame,
    /// `finish` with no frame added.
    EmptyPool,
    /// A sampled modality value is not finite. A failed frame leaves the pool
    /// unchanged, so the caller can skip it and continue.
    NonFiniteModality,
    /// A padding bound is not finite.
    InvalidPadding,
    /// The computed centre or width is not finite.
    NonFiniteWindow,
    /// The window check could not map the percentiles, the constant value,
    /// or the largest left-out value to its target within its search budget.
    UnverifiableWindow,
}

/// One pool over up to `frames` frames, each with its own modality stage.
///
/// The declared count fixes each frame's budget,
/// [`AUTO_WINDOW_SAMPLE_CAP`] divided by it, for the whole life of the pool.
/// A frame skipped after a refusal does not move the budgets, so the cap
/// still holds and the pool stays a pure function of the sampled bytes.
pub struct AutoWindowPool {
    /// Counted modality values, after stride and exclusion.
    values: Vec<f32>,
    /// The largest modality value among the left-out samples.
    left_out_max: Option<f32>,
    /// The declared frame count the budgets come from.
    declared: usize,
    /// Frames added so far.
    added: usize,
}

impl AutoWindowPool {
    /// Declare a pool over `frames` frames, from 1 to
    /// [`AUTO_WINDOW_SERIES_FRAMES`].
    ///
    /// The count is declared up front so the cap split never depends on how
    /// many frames actually arrive, which is what makes a mixed series with
    /// skipped frames sample the same values as an unmixed one.
    ///
    /// # Errors
    ///
    /// [`AutoWindowError::FrameCount`] when `frames` is zero or beyond the
    /// series frame count.
    pub fn new(frames: usize) -> Result<Self, AutoWindowError> {
        if frames == 0 || frames > AUTO_WINDOW_SERIES_FRAMES {
            return Err(AutoWindowError::FrameCount);
        }
        Ok(Self {
            // Plan item 7: the whole pool is reserved once, at most
            // AUTO_WINDOW_SAMPLE_CAP values and so at most 4 MiB of f32, and
            // never grows. A frame reads `len.div_ceil(stride)` samples, at
            // most its budget since `stride >= len / budget`, and at most
            // `frames` frames are added, so no push reaches past it.
            values: Vec::with_capacity(sample_budget(frames) * frames),
            left_out_max: None,
            declared: frames,
            added: 0,
        })
    }

    /// Fold one frame. Its stride is
    /// `len.div_ceil(AUTO_WINDOW_SAMPLE_CAP / frames)`, so a frame at or
    /// under its budget is read exactly and a larger one is read from index 0
    /// at a fixed step that depends on the size and the declared count only,
    /// never on content. Exclusions are stored-space predicates applied after
    /// the stride: stored zero, PS3.3 C.7.5.1.1.2's padding, and nothing
    /// else. The modality stage, PS3.3 C.11.1, is this frame's own.
    ///
    /// # Errors
    ///
    /// [`AutoWindowError::TooManyFrames`] beyond the declared count,
    /// [`AutoWindowError::EmptyFrame`] for an empty slice,
    /// [`AutoWindowError::InvalidPadding`] for a non-finite padding bound and
    /// [`AutoWindowError::NonFiniteModality`] when any sampled modality value
    /// is not finite. A refused frame leaves the pool exactly as it was, so
    /// the caller can skip it and continue with the next.
    pub fn add_frame(
        &mut self,
        stored: &[Stored],
        modality: &ModalityTransform,
        padding: Option<StoredRange>,
    ) -> Result<(), AutoWindowError> {
        if !padding_is_valid(padding) {
            return Err(AutoWindowError::InvalidPadding);
        }
        if self.added >= self.declared {
            return Err(AutoWindowError::TooManyFrames);
        }
        if stored.is_empty() {
            return Err(AutoWindowError::EmptyFrame);
        }
        let stride = stored.len().div_ceil(sample_budget(self.declared));
        let mark = self.values.len();
        let mut left_out_max = self.left_out_max;
        for sample in stored.iter().step_by(stride) {
            let value = modality.apply(*sample).0;
            if !value.is_finite() {
                self.values.truncate(mark);
                return Err(AutoWindowError::NonFiniteModality);
            }
            let is_stored_zero = sample.0.to_bits() & 0x7fff_ffff == 0;
            if is_stored_zero || padding_contains(padding, *sample) {
                left_out_max = Some(match left_out_max {
                    Some(current) if current >= value => current,
                    _ => value,
                });
            } else {
                debug_assert!(self.values.len() < self.values.capacity());
                self.values.push(value);
            }
        }
        self.left_out_max = left_out_max;
        self.added += 1;
        Ok(())
    }

    /// The sample buffer's capacity in values, for the allocation test.
    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.values.capacity()
    }

    /// The window over every counted value the pool holds.
    ///
    /// The ranks are the plan's nearest-rank rule, one-based:
    /// `n.div_ceil(100)` and `(99 * n).div_ceil(100)`, both at least 1.
    /// `select_nth_unstable_by` places a rank's value at index rank - 1
    /// without sorting, and the second rank is selected inside the tail the
    /// first partition leaves, which is all at or above `low`.
    ///
    /// The window is then checked by the transform that draws it, per the lead's amendments of
    /// 2026-11-06 after review passes 2, 3, 4, 5 and 6. A Percentile window is accepted only when
    /// `VoiTransform::new(.., LinearExact, ..)` over the 0 to 255 output range, evaluated in f32 as
    /// `ocelli-pixel` evaluates it, gives exactly the output minimum for the 1st percentile and for
    /// the 1st moved 16 f32 units toward the 99th, and exactly the output maximum for the 99th and
    /// for the 99th moved 16 units toward the 1st, with the moves capped at the midpoint as
    /// `margin_pair` says. A tier whose stage 1 rounds differently by fewer units than the move
    /// therefore still draws the two percentiles black and white. The move is 16 units for
    /// percentiles at least 33 units apart and smaller below that, so the margin is smaller there
    /// too, and one unit for percentiles 3 or 4 units apart. Percentiles fewer than three units
    /// apart are not a Percentile window. The moved values bound the window, so the window is
    /// computed and searched on the moved pair: the first width is their difference and narrower
    /// widths are tried with the centre kept the midpoint `low_moved + width / 2`. A value at or
    /// below the lower edge, or strictly above the upper edge, draws through C.11.2.1.3.2's edge
    /// branches, whose values do not depend on rounding. A moved 99th on the upper edge itself, the
    /// usual case, or a moved 1st above a lower edge that rounded below it, draws through the ramp,
    /// and its exactness there is established for this evaluation only. A Constant window is
    /// accepted only when its centre's display value through the same transform is exactly the
    /// middle of the output range. The centre is the constant value, or, per the lead's amendment
    /// after review pass 5, the f32 midpoint of percentiles 1 or 2 units apart, which for 1 unit is
    /// a tie and rounds onto whichever has the even significand. Per the lead's amendments after
    /// review passes 5 and 6, each percentile, and each moved 16 units away from the other, must
    /// also draw within half a grey of the middle, above 127.0 and below 128.0, for equal
    /// percentiles as for a close pair. For equal percentiles the four points are the value moved
    /// 16 units down, the value twice, and the value moved 16 units up. At width 1 that holds
    /// while every outward point is below 1024 in magnitude, where a unit is at most 2^-14 and
    /// the farthest point, at most 17 units out, draws at most 17 * 2^-14 * 255 = 0.26 grey from
    /// the middle, and from 1024 up the search may widen. A point D from the centre draws
    /// 255 * D / w from the middle, so the check needs w > 510 * D for the farthest outward
    /// point. The Background window is
    /// accepted only when its largest left-out value maps to the output minimum, and otherwise
    /// wider widths are tried with the centre kept at `value + width / 2`. Every search is
    /// `search_window`'s: the change from the first width starts at one f32 unit of that width and
    /// doubles on each of at most 48 attempts, and the first width that passes the check is
    /// returned. The window is refused when none does.
    ///
    /// Widening from width 1 tries 1 + 2^(k - 23) at attempt `k`, and the last, 1 + 2^24, rounds
    /// to 2^24, the widest. At 2^24 a point 32768 from the centre draws 255 * 32768 / 2^24 =
    /// 0.498 from the middle and one 34816 away draws 0.529, so the reach ends in [2^34, 2^35),
    /// where the spacing is 2048 and 16 units are 32768. Measured on 2026-11-06 by bisection over
    /// the f32 values, slope 1, and then over every binade from the subnormals to 2^40, 16384
    /// values at each end of the binade and 16384 drawn inside it, with no exception beyond those
    /// named here:
    ///
    /// - A constant is accepted up to 34359705600 (0x50FFFFF0), 16 units below 2^35, at width
    ///   2^24, where the value moved 16 units up is 2^35 itself. It is refused from 34359707648
    ///   (0x50FFFFF1) up, whose 16 units up cross 2^35 into spacing 4096 and span 34816. The
    ///   negative values mirror this exactly.
    /// - A close pair 1 unit apart is accepted when its 1st percentile is below 2^34 and refused
    ///   when it is 2^34 (0x50800000) or above, apart from seven pairs whose 1st percentile is
    ///   2^34 plus an odd number of units from 1 to 13, 0x50800001 to 0x5080000D. Their midpoint
    ///   ties onto the 99th percentile, so the 99th moved 16 units up is 32768 from the centre.
    ///   The 1st moved 16 units down crosses below 2^34 into spacing 1024 and lies
    ///   18432 + 1024 * k from the centre for k units, 31744 at k = 13 and 33792 at k = 15. The
    ///   pairs mirrored to negative values behave the same.
    /// - A close pair 2 units apart is accepted when its 1st percentile is below 0x507FFFFE, two
    ///   units below 2^34, and refused from 0x507FFFFE up, where the 99th percentile is 2^34 and
    ///   moved 16 units up is 33792 from the centre. The negative values mirror this exactly.
    ///
    /// # Errors
    ///
    /// [`AutoWindowError::EmptyPool`] with no frame added,
    /// [`AutoWindowError::NonFiniteWindow`] when the computed centre or width
    /// is not finite, which a rescale this extreme would otherwise hide, and
    /// [`AutoWindowError::UnverifiableWindow`] when the search budget cannot
    /// make the transform map the percentiles, the constant value, or the
    /// largest left-out value to its target.
    pub fn finish(self) -> Result<AutoWindow, AutoWindowError> {
        if self.added == 0 {
            return Err(AutoWindowError::EmptyPool);
        }
        let Self {
            values,
            left_out_max,
            declared: _,
            added,
        } = self;
        let counted = values.len();
        if counted == 0 {
            // Every sampled value was left out, so at least one sample was
            // read and the maximum exists. The unreachable arm keeps the type
            // total. The window is the one whose lower side maps the largest
            // left-out value to the output minimum, and its upper side is free.
            let Some(max_left_out) = left_out_max else {
                return Err(AutoWindowError::EmptyPool);
            };
            let maps_to_minimum = |center, width| {
                display_bits(center, width, max_left_out) == Some(CHECK_YMIN.to_bits())
            };
            let Some((center, width)) = search_window(
                1.0,
                Search::Widen,
                |width| max_left_out + width / 2.0,
                maps_to_minimum,
            ) else {
                return Err(AutoWindowError::UnverifiableWindow);
            };
            return Ok(AutoWindow {
                center,
                width,
                low: max_left_out,
                high: center + width / 2.0,
                counted: 0,
                frames: added,
                basis: AutoWindowBasis::Background,
            });
        }
        let rank_low = counted.div_ceil(100);
        let rank_high = (99 * counted).div_ceil(100);
        let mut values = values;
        let (_, at_low, after_low) = values.select_nth_unstable_by(rank_low - 1, total_order);
        let low = *at_low;
        let high = if rank_high == rank_low {
            low
        } else {
            let (_, at_high, _) =
                after_low.select_nth_unstable_by(rank_high - 1 - rank_low, total_order);
            *at_high
        };

        let span = high - low;
        // The lead's amendment after review pass 5: percentiles fewer than
        // three f32 units apart are constant to f32 precision and take the
        // Constant window, centred on the f32 midpoint of the pair. Two
        // `next_up` steps from `low` reach every value 1 or 2 units above it,
        // and a span of zero, equal values or the two zeros, is d = 0.
        let close = span > 0.0 && high <= low.next_up().next_up();
        let (center, width, basis) = if close {
            (low + span / 2.0, 1.0, AutoWindowBasis::Constant)
        } else if span > 0.0 {
            (low + span / 2.0, span, AutoWindowBasis::Percentile)
        } else {
            (low, 1.0, AutoWindowBasis::Constant)
        };
        if !center.is_finite() || !width.is_finite() {
            return Err(AutoWindowError::NonFiniteWindow);
        }
        let found = if basis == AutoWindowBasis::Percentile {
            // The margin of the lead's amendment after review pass 4. The
            // moved values must draw the output minimum and maximum too, so
            // the window's edges cannot lie outside them, and the search runs on the
            // moved pair. The moved pair is strictly ordered, so its
            // difference is positive, and it is no wider than the finite span.
            //
            // The checks at the unmoved `low` and `high` are kept as a
            // defence, and review pass 5 found mutants removing either one
            // survive. When the moved value draws its extreme through an edge
            // branch the unmoved one is further out and takes the same
            // branch, so the check is implied. When the moved value is on the
            // ramp, the f32 ramp is non-decreasing in `x`, which bounds the
            // unmoved value's display on one side only, and that it is
            // exactly 0.0 or 255.0 has been measured, not proved.
            let (low_moved, high_moved) = margin_pair(low, high);
            let maps_extremes = |center, width| {
                display_bits(center, width, low) == Some(CHECK_YMIN.to_bits())
                    && display_bits(center, width, low_moved) == Some(CHECK_YMIN.to_bits())
                    && display_bits(center, width, high_moved) == Some(CHECK_YMAX.to_bits())
                    && display_bits(center, width, high) == Some(CHECK_YMAX.to_bits())
            };
            search_window(
                high_moved - low_moved,
                Search::Narrow,
                |width| low_moved + width / 2.0,
                maps_extremes,
            )
        } else {
            // The centre must draw exactly the middle. Each percentile, and
            // each moved PERCENTILE_MARGIN units away from the other, must
            // draw within half a grey of it, so a tier whose stage 1 rounds
            // differently by fewer units still draws grey 127 or 128. Per the
            // lead's amendment after review pass 6 this holds for equal
            // percentiles too, whose outward values are the constant moved
            // 16 units down and 16 units up.
            let (low_out, high_out) = outward_pair(low, high);
            let constant = center;
            let maps_to_middle = |center, width| {
                display_bits(center, width, constant) == Some(CHECK_MIDDLE.to_bits())
                    && [low_out, low, high, high_out]
                        .into_iter()
                        .all(|x| near_middle(center, width, x))
            };
            search_window(width, Search::Widen, |_| constant, maps_to_middle)
        };
        let Some((center, width)) = found else {
            return Err(AutoWindowError::UnverifiableWindow);
        };
        Ok(AutoWindow {
            center,
            width,
            low,
            high,
            counted,
            frames: added,
            basis,
        })
    }
}

/// `f32::total_cmp` as a named comparator, so both selections share one.
fn total_order(a: &f32, b: &f32) -> Ordering {
    a.total_cmp(b)
}

/// The output range the window check evaluates: the 0 to 255 grey scale the
/// fixtures draw and `ocelli_render`'s `display_index` clamps to.
const CHECK_YMIN: f32 = 0.0;
const CHECK_YMAX: f32 = 255.0;

/// The middle of the output range, which a Constant window must give for its
/// constant value: 0.5 * 255 = 127.5, the ramp's value at the centre.
const CHECK_MIDDLE: f32 = CHECK_YMIN + (CHECK_YMAX - CHECK_YMIN) / 2.0;

/// How many changed widths a window search tries after the first width.
const SEARCH_ATTEMPTS: usize = 48;

/// How many f32 units each percentile is moved toward the other for the
/// margin check, per the lead's amendment of 2026-11-06 after review pass 4.
const PERCENTILE_MARGIN: usize = 16;

/// The 1st and 99th percentiles, `low < high`, each moved up to
/// [`PERCENTILE_MARGIN`] f32 units toward the other, one `next_up` or
/// `next_down` at a time.
///
/// The moves are capped at the midpoint. Two values fewer than 33 units apart
/// would meet or cross after 16 moves each, and a low-side value at or above a
/// high-side one cannot draw the output minimum while the other draws the
/// maximum, so the check would refuse every such pair for a reason that is
/// not the window's. A step is therefore taken only while the two moved values
/// stay strictly apart after it: each stops at the last f32 strictly on its
/// own side of the midpoint, `min(16, (d - 1) / 2)` units for a pair `d`
/// units apart, in integer division. `finish` calls it only for pairs at
/// least three units apart, which move at least one unit. This is the
/// reading the lead's brief of 2026-11-06 directs, that the window is checked
/// with the moves capped at the midpoint.
fn margin_pair(low: f32, high: f32) -> (f32, f32) {
    let mut low_moved = low;
    let mut high_moved = high;
    for _ in 0..PERCENTILE_MARGIN {
        let up = low_moved.next_up();
        let down = high_moved.next_down();
        if up >= down {
            break;
        }
        low_moved = up;
        high_moved = down;
    }
    (low_moved, high_moved)
}

/// How far from [`CHECK_MIDDLE`] a close pair's values may draw: half a grey,
/// so each draws above 127.0 and below 128.0, grey 127 or 128 under
/// `display_index`'s rounding.
const CONSTANT_TOLERANCE: f32 = 0.5;

/// The 1st and 99th percentiles of a close pair, each moved
/// [`PERCENTILE_MARGIN`] f32 units away from the other, one `next_down` or
/// `next_up` at a time.
fn outward_pair(low: f32, high: f32) -> (f32, f32) {
    let mut low_out = low;
    let mut high_out = high;
    for _ in 0..PERCENTILE_MARGIN {
        low_out = low_out.next_down();
        high_out = high_out.next_up();
    }
    (low_out, high_out)
}

/// Whether the LINEAR_EXACT transform built from `center` and `width` draws
/// `x` strictly within [`CONSTANT_TOLERANCE`] of [`CHECK_MIDDLE`]. False when
/// the pair is not a valid transform or the display value is NaN.
fn near_middle(center: f32, width: f32, x: f32) -> bool {
    display_bits(center, width, x)
        .map(f32::from_bits)
        .is_some_and(|y| {
            y > CHECK_MIDDLE - CONSTANT_TOLERANCE && y < CHECK_MIDDLE + CONSTANT_TOLERANCE
        })
}

/// The display bits the LINEAR_EXACT transform built from `center` and
/// `width` gives `x`, or `None` when that pair is not a valid transform.
/// PS3.3 C.11.2.1.3.2. This calls the existing [`VoiTransform`], evaluated in
/// f32 as `ocelli-pixel` evaluates it, so the LUT chain exists once and no
/// second copy of its arithmetic is made here.
fn display_bits(center: f32, width: f32, x: f32) -> Option<u32> {
    let voi = VoiTransform::new(
        None,
        &[center],
        &[width],
        0,
        VoiFunction::LinearExact,
        CHECK_YMIN,
        CHECK_YMAX,
    )
    .ok()?;
    Some(voi.apply(Modality(x)).0.to_bits())
}

/// Which way a window search moves the width.
#[derive(Clone, Copy)]
enum Search {
    /// Narrower widths, for the Percentile window, whose upper edge is
    /// `low_moved + width` in real arithmetic and falls as the width falls.
    Narrow,
    /// Wider widths, for the Constant and Background windows.
    Widen,
}

/// The first width that passes `check`, with its centre `center_of(width)`,
/// per the lead's amendment of 2026-11-06 after review pass 3.
///
/// `first` is tried as computed. Then the change from it starts at one f32 unit
/// of `first`, the gap from `first` to its neighbouring f32 in the direction of
/// the search, so the first attempt is one bit step. The change doubles on each
/// attempt, at most [`SEARCH_ATTEMPTS`] attempts: attempt `k`, from 0, tries
/// `first - unit * 2^k` when narrowing and `first + unit * 2^k` when widening,
/// each subtraction or addition one f32 operation. The unit is a power of two,
/// so each doubling is exact until it overflows to infinity, which ends the
/// search. Returns `None` when no attempt passes, or when the width leaves the
/// positive finite range, which ends a narrowing search once the change reaches
/// `first`. `first` is positive and finite.
fn search_window(
    first: f32,
    search: Search,
    center_of: impl Fn(f32) -> f32,
    check: impl Fn(f32, f32) -> bool,
) -> Option<(f32, f32)> {
    let center = center_of(first);
    if check(center, first) {
        return Some((center, first));
    }
    let bits = first.to_bits();
    let mut change = match search {
        Search::Narrow => first - f32::from_bits(bits.checked_sub(1)?),
        Search::Widen => f32::from_bits(bits.checked_add(1)?) - first,
    };
    for _ in 0..SEARCH_ATTEMPTS {
        let width = match search {
            Search::Narrow => first - change,
            Search::Widen => first + change,
        };
        if width <= 0.0 || !width.is_finite() {
            return None;
        }
        let center = center_of(width);
        if check(center, width) {
            return Some((center, width));
        }
        change *= 2.0;
    }
    None
}

/// A pool of one frame. The per-image window.
///
/// # Errors
///
/// [`AutoWindowPool::add_frame`]'s and [`AutoWindowPool::finish`]'s refusals,
/// and [`AutoWindowError::FrameCount`] never, because the count is one.
pub fn auto_window(
    stored: &[Stored],
    modality: &ModalityTransform,
    padding: Option<StoredRange>,
) -> Result<AutoWindow, AutoWindowError> {
    let mut pool = AutoWindowPool::new(1)?;
    pool.add_frame(stored, modality, padding)?;
    pool.finish()
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{AUTO_WINDOW_SAMPLE_CAP, AutoWindowPool};
    use crate::ModalityTransform;
    use ocelli_core::Stored;

    /// Plan item 7: the scratch is at most 4 MiB of `f32`, 2^20 values of 4
    /// bytes, and never grows. `finish` allocates nothing, since it selects
    /// in place inside the buffer it takes, so the buffer's capacity is the
    /// pool's whole allocation. Every stored value here is 100, never zero
    /// and with no padding, so every strided sample is counted and each
    /// frame fills its budget as far as its stride allows.
    fn assert_scratch_bounded(frames: usize, pixels: usize) {
        let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
        assert!(modality.is_ok());
        let Ok(modality) = modality else { return };
        let frame = vec![Stored(100.0); pixels];
        let pool = AutoWindowPool::new(frames);
        assert!(pool.is_ok());
        let Ok(mut pool) = pool else { return };
        let reserved = pool.capacity();
        assert!(reserved * 4 <= 4 * 1024 * 1024);
        for _ in 0..frames {
            assert!(pool.add_frame(&frame, &modality, None).is_ok());
            assert_eq!(pool.capacity(), reserved);
            assert!(pool.capacity() * 4 <= 4 * 1024 * 1024);
        }
        assert!(pool.values.len() <= AUTO_WINDOW_SAMPLE_CAP);
        assert!(pool.finish().is_ok());
    }

    /// Five 1421 by 1024 frames: budget 2^20 / 5 = 209,715, stride
    /// 1,455,104.div_ceil(209,715) = 7, so 207,872 values a frame and
    /// 1,039,360 in all, which a buffer grown by doubling from 209,715
    /// holds only at 1,677,720 values, 6,710,880 bytes.
    #[test]
    fn five_mammography_frames_stay_within_four_mib() {
        assert_scratch_bounded(5, 1421 * 1024);
    }

    /// Three 1024 by 1024 frames: budget 349,525, stride 4, 262,144 values
    /// a frame and 786,432 in all, which doubling from 349,525 holds only at
    /// 1,398,100 values, 5,592,400 bytes.
    #[test]
    fn three_megapixel_frames_stay_within_four_mib() {
        assert_scratch_bounded(3, 1024 * 1024);
    }

    /// The reserve's exact boundary, review pass 14 N2. The two bounded
    /// tests above push under the reserve, so a reserve short by a few
    /// thousand values passes them. Every frame here has the length item 7's
    /// stride rule makes push exactly its budget, `stride =
    /// len.div_ceil(budget)` then `pushed = len.div_ceil(stride)`, so the
    /// pool ends at `len == capacity` and a reserve short by even one value
    /// trips an assertion in either profile, the push's `debug_assert` in
    /// debug and the capacity comparison after the last frame in release.
    fn assert_reserve_filled(frames: usize, pixels: usize) {
        let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
        assert!(modality.is_ok());
        let Ok(modality) = modality else { return };
        let frame = vec![Stored(100.0); pixels];
        let pool = AutoWindowPool::new(frames);
        assert!(pool.is_ok());
        let Ok(mut pool) = pool else { return };
        let reserved = pool.capacity();
        assert!(reserved * 4 <= 4 * 1024 * 1024);
        for _ in 0..frames {
            assert!(pool.add_frame(&frame, &modality, None).is_ok());
            assert_eq!(pool.capacity(), reserved);
        }
        assert_eq!(pool.values.len(), pool.capacity());
        assert!(pool.finish().is_ok());
    }

    /// Seven frames of 149,796 values. Budget 2^20 / 7 = 149,796, since
    /// 7 * 149,796 = 1,048,572 is 4 under 2^20. Item 7's rule on a frame of
    /// 149,796 values: stride = 149,796.div_ceil(149,796) = 1 and pushed =
    /// 149,796.div_ceil(1) = 149,796, the budget exactly, so seven frames
    /// push 7 * 149,796 = 1,048,572 and fill the reserve itself.
    #[test]
    fn seven_frame_series_fills_its_reserve_exactly() {
        assert_reserve_filled(7, 149_796);
    }

    /// Thirteen frames of 80,659 values. Budget 2^20 / 13 = 80,659, since
    /// 13 * 80,659 = 1,048,567 is 9 under 2^20. Item 7's rule on a frame of
    /// 80,659 values: stride = 80,659.div_ceil(80,659) = 1 and pushed =
    /// 80,659.div_ceil(1) = 80,659, the budget exactly, so thirteen frames
    /// push 13 * 80,659 = 1,048,567 and fill the reserve itself.
    #[test]
    fn thirteen_frame_series_fills_its_reserve_exactly() {
        assert_reserve_filled(13, 80_659);
    }
}
