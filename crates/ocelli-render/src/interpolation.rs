//! Stack display interpolation. F-194.
//!
//! Nearest is the existing `texel_of` rule. Linear is a 2 by 2 blend of
//! display samples. The weights are the same ones the stack shader writes
//! out. A tier that cannot run the requested mode reports unavailable and
//! does not substitute nearest.

use crate::caps::Tier;
use crate::cpu::texel_of;

/// How a stack display samples one continuous image coordinate.
///
/// The discriminant is the uniform value. `0` is nearest. `1` is linear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Interpolation {
    /// The texel `texel_of` already selects.
    Nearest = 0,
    /// The four texels around the coordinate, weighted by the fractions.
    Linear = 1,
}

impl Interpolation {
    /// Map a uniform integer onto the two modes.
    ///
    /// # Errors
    ///
    /// [`InterpolationError::Unavailable`] for any value other than 0 or 1.
    pub fn from_mode(value: u32) -> Result<Self, InterpolationError> {
        match value {
            0 => Ok(Self::Nearest),
            1 => Ok(Self::Linear),
            _ => Err(InterpolationError::Unavailable),
        }
    }

    /// Uniform integer for this mode.
    #[must_use]
    pub const fn mode(self) -> u32 {
        match self {
            Self::Nearest => 0,
            Self::Linear => 1,
        }
    }

    /// Exact `f32` stored in the spare camera lane.
    ///
    /// The lane is `f32` because that uniform is a `vec4`. `0.0` and `1.0`
    /// convert back with `u32` truncation to the mode integer.
    #[must_use]
    pub const fn lane(self) -> f32 {
        match self {
            Self::Nearest => 0.0,
            Self::Linear => 1.0,
        }
    }
}

/// Why a sample was not produced. The destination is left unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationError {
    /// The coordinate, or the linear footprint, leaves the image.
    Outside,
    /// The mode is not one of the two declared modes, or the tier refuses it.
    Unavailable,
}

/// Whether the declared table offers this mode on this tier.
///
/// Both modes are available on tiers A, B and C. The test double is
/// [`available_when`].
#[must_use]
pub fn available(mode: Interpolation, tier: Tier) -> bool {
    available_when(mode, tier, true)
}

/// Same table as [`available`], with tier B linear forced by the caller.
///
/// `tier_b_linear` false is the test double. The draw must refuse linear
/// on tier B and must not return the nearest texel.
#[must_use]
pub fn available_when(mode: Interpolation, tier: Tier, tier_b_linear: bool) -> bool {
    match (mode, tier) {
        (Interpolation::Linear, Tier::B) => tier_b_linear,
        (Interpolation::Nearest | Interpolation::Linear, Tier::A | Tier::B | Tier::Cpu) => true,
    }
}

/// Sample one display image.
///
/// Linear treats `0` as the centre of texel `(0, 0)`. Nearest forwards the
/// same numbers to [`texel_of`], and [`texel_of`] reads a corner-origin image
/// coordinate. The approved pair at `(0.5, 0.5)` is nearest texel 0 and
/// linear `138.75`. Those two modes are not one geometric point. The raster
/// passes an index to linear and `index + 0.5` to [`texel_of`].
///
/// `samples` is row-major. `width` is the column count and the row count is
/// the slice length divided by `width`. A linear footprint that leaves the
/// image is [`InterpolationError::Outside`] and is not clamped.
///
/// # Errors
///
/// [`InterpolationError::Outside`] when the sample cannot be read from the
/// image. The mode is already a declared one, so this function does not
/// return [`InterpolationError::Unavailable`].
pub fn blend_display(
    width: u32,
    samples: &[f32],
    x: f32,
    y: f32,
    mode: Interpolation,
) -> Result<f32, InterpolationError> {
    let height = image_height(width, samples).ok_or(InterpolationError::Outside)?;
    match mode {
        Interpolation::Nearest => nearest(width, height, samples, x, y),
        Interpolation::Linear => linear(width, height, samples, x, y),
    }
}

/// Blend on a tier, or refuse when that tier does not offer the mode.
///
/// # Errors
///
/// [`InterpolationError::Unavailable`] when the table refuses the pair.
/// [`InterpolationError::Outside`] from [`blend_display`].
pub fn blend_on_tier(
    samples: &TierSample<'_>,
    tier: Tier,
    tier_b_linear: bool,
) -> Result<f32, InterpolationError> {
    if !available_when(samples.mode, tier, tier_b_linear) {
        return Err(InterpolationError::Unavailable);
    }
    blend_display(
        samples.width,
        samples.samples,
        samples.x,
        samples.y,
        samples.mode,
    )
}

/// One blend request. Grouped so the tier call stays within the argument limit.
pub struct TierSample<'a> {
    /// Column count.
    pub width: u32,
    /// Row-major display samples.
    pub samples: &'a [f32],
    /// Column coordinate. Nearest and linear do not share an origin.
    /// [`blend_display`] states the rule.
    pub x: f32,
    /// Row coordinate. The rule is the same as [`Self::x`].
    pub y: f32,
    /// Requested mode.
    pub mode: Interpolation,
}

/// Write one sample into `slot` only when the mode and the coordinate succeed.
///
/// A refusal leaves `slot` as it was.
///
/// # Errors
///
/// [`InterpolationError::Unavailable`] for a mode other than 0 or 1.
/// [`InterpolationError::Outside`] when the coordinate cannot be sampled.
pub fn write_sample(
    mode: u32,
    width: u32,
    samples: &[f32],
    x: f32,
    y: f32,
    slot: &mut f32,
) -> Result<(), InterpolationError> {
    let mode = Interpolation::from_mode(mode)?;
    let value = blend_display(width, samples, x, y, mode)?;
    *slot = value;
    Ok(())
}

fn nearest(
    width: u32,
    height: u32,
    samples: &[f32],
    x: f32,
    y: f32,
) -> Result<f32, InterpolationError> {
    let Some(column) = texel_of(f64::from(x), width) else {
        return Err(InterpolationError::Outside);
    };
    let Some(row) = texel_of(f64::from(y), height) else {
        return Err(InterpolationError::Outside);
    };
    sample_at(samples, width, column, row)
}

fn linear(
    width: u32,
    height: u32,
    samples: &[f32],
    x: f32,
    y: f32,
) -> Result<f32, InterpolationError> {
    blend_points(width, height, x, y, |column, row| {
        sample_at(samples, width, column, row).ok()
    })
}

/// Linear blend in centre-origin texel space.
///
/// `0` is the centre of texel `(0, 0)`. A fraction of `0` does not read the
/// next texel, so the centre of the last texel stays that texel. A non-zero
/// fraction whose next texel is outside the image is
/// [`InterpolationError::Outside`].
pub(crate) fn blend_points(
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    mut sample: impl FnMut(u32, u32) -> Option<f32>,
) -> Result<f32, InterpolationError> {
    let (column, tx) = floor_fraction(x, width)?;
    let (row, ty) = floor_fraction(y, height)?;
    let p00 = sample(column, row).ok_or(InterpolationError::Outside)?;
    let p10 = optional_neighbour(tx, column, width, |next| sample(next, row))?;
    let p01 = optional_neighbour(ty, row, height, |next| sample(column, next))?;
    let p11 = if fraction_is_zero(tx) || fraction_is_zero(ty) {
        0.0
    } else {
        let column_next = column
            .checked_add(1)
            .filter(|next| *next < width)
            .ok_or(InterpolationError::Outside)?;
        let row_next = row
            .checked_add(1)
            .filter(|next| *next < height)
            .ok_or(InterpolationError::Outside)?;
        sample(column_next, row_next).ok_or(InterpolationError::Outside)?
    };
    let one = 1.0_f32;
    let w00 = (one - tx) * (one - ty);
    let w10 = tx * (one - ty);
    let w01 = (one - tx) * ty;
    let w11 = tx * ty;
    Ok(w00 * p00 + w10 * p10 + w01 * p01 + w11 * p11)
}

fn fraction_is_zero(value: f32) -> bool {
    value.abs().to_bits() == 0.0_f32.to_bits()
}

fn optional_neighbour(
    fraction: f32,
    index: u32,
    limit: u32,
    sample: impl FnOnce(u32) -> Option<f32>,
) -> Result<f32, InterpolationError> {
    if fraction_is_zero(fraction) {
        return Ok(0.0);
    }
    let next = index
        .checked_add(1)
        .filter(|next| *next < limit)
        .ok_or(InterpolationError::Outside)?;
    sample(next).ok_or(InterpolationError::Outside)
}

fn image_height(width: u32, samples: &[f32]) -> Option<u32> {
    let columns = usize::try_from(width).ok()?;
    if columns == 0 || samples.is_empty() || !samples.len().is_multiple_of(columns) {
        return None;
    }
    u32::try_from(samples.len() / columns).ok()
}

fn sample_at(
    samples: &[f32],
    width: u32,
    column: u32,
    row: u32,
) -> Result<f32, InterpolationError> {
    let columns = usize::try_from(width).map_err(|_| InterpolationError::Outside)?;
    let x = usize::try_from(column).map_err(|_| InterpolationError::Outside)?;
    let y = usize::try_from(row).map_err(|_| InterpolationError::Outside)?;
    let index = y
        .checked_mul(columns)
        .and_then(|offset| offset.checked_add(x))
        .ok_or(InterpolationError::Outside)?;
    samples
        .get(index)
        .copied()
        .ok_or(InterpolationError::Outside)
}

/// Greatest integer `n` with `n <= value < limit`, and the fraction past it.
fn floor_fraction(value: f32, limit: u32) -> Result<(u32, f32), InterpolationError> {
    if !value.is_finite() || value < 0.0 {
        return Err(InterpolationError::Outside);
    }
    let limit_f = f32_from_u32(limit);
    if value >= limit_f {
        return Err(InterpolationError::Outside);
    }
    let mut lo = 0_u32;
    let mut hi = limit;
    while lo.saturating_add(1) < hi {
        let mid = lo + (hi - lo) / 2;
        if f32_from_u32(mid) <= value {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok((lo, value - f32_from_u32(lo)))
}

fn f32_from_u32(value: u32) -> f32 {
    let high = u16::try_from(value >> 16).unwrap_or(0);
    let low = u16::try_from(value & 0xffff).unwrap_or(0);
    f32::from(high) * 65_536.0 + f32::from(low)
}
