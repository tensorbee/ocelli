//! Backing-store size in device pixels.
//!
//! This is the only definition of that size. It touches no GPU, so a tier C
//! session can size a buffer later without a second formula. F-039.

/// Why a CSS size could not become a backing-store extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ExtentError {
    /// A CSS length or the device pixel ratio was NaN or infinite.
    #[error("a backing-store input is not finite")]
    NonFinite,
    /// The device pixel ratio was zero or negative, or a positive ratio
    /// narrowed to a non-positive `f32` and so cannot be carried by the GPU
    /// camera uniform.
    #[error("device pixel ratio must be positive and narrow to a positive f32")]
    NonPositiveRatio,
}

/// Whether a device pixel ratio can be carried by the GPU camera uniform.
///
/// Finite, strictly positive, and narrows to a positive `f32`. The CPU extent
/// and CSS functions and `RenderGraph::set_view` all use this predicate, so
/// the CPU and GPU paths cannot disagree about the same input.
#[must_use]
pub fn usable_ratio(dpr: f64) -> bool {
    dpr.is_finite() && dpr > 0.0 && ocelli_viewport::view::f32_from_f64(dpr) > 0.0
}

/// Device-pixel extent of a CSS box at `dpr`.
///
/// Each axis is `round(css * dpr)` with [`f64::round`], which is half away
/// from zero, then a saturating conversion to `u32`. A product that overflows
/// to `+infinity` is at or above `2^32` and becomes `u32::MAX`. A strictly
/// positive CSS axis whose rounded product is 0 becomes 1, so a visible
/// element does not get an empty backing store. A non-finite input is
/// [`ExtentError::NonFinite`]. A device pixel ratio that is not strictly
/// positive, or that narrows to a non-positive `f32` and so cannot be carried
/// by the GPU camera uniform, is [`ExtentError::NonPositiveRatio`].
pub fn device_extent(css_width: f64, css_height: f64, dpr: f64) -> Result<(u32, u32), ExtentError> {
    if !css_width.is_finite() || !css_height.is_finite() || !dpr.is_finite() {
        return Err(ExtentError::NonFinite);
    }
    if !usable_ratio(dpr) {
        return Err(ExtentError::NonPositiveRatio);
    }
    Ok((axis_extent(css_width, dpr), axis_extent(css_height, dpr)))
}

/// CSS pixel coordinate for one device pixel position.
///
/// This is the inverse direction of [`device_extent`]'s scale, without that
/// function's rounding. A fragment at a device position divides by the same
/// dpr once before a CSS camera mapping. A second division is a different point.
///
/// # Errors
///
/// [`ExtentError::NonFinite`] when either input is not finite or the quotient
/// is not finite. [`ExtentError::NonPositiveRatio`] when `dpr` is not strictly
/// positive or does not narrow to a positive `f32`.
pub fn css_from_device(device: f64, dpr: f64) -> Result<f64, ExtentError> {
    if !device.is_finite() || !dpr.is_finite() {
        return Err(ExtentError::NonFinite);
    }
    if !usable_ratio(dpr) {
        return Err(ExtentError::NonPositiveRatio);
    }
    let css = device / dpr;
    if !css.is_finite() {
        return Err(ExtentError::NonFinite);
    }
    Ok(css)
}

fn axis_extent(css: f64, dpr: f64) -> u32 {
    let rounded = f64::round(css * dpr);
    // Two finite inputs can overflow to ±infinity. +infinity is past 2^32
    // and saturates. The empty-axis floor below is only a rounded 0, so an
    // overflowing product must not become 1.
    if !rounded.is_finite() {
        return if rounded > 0.0 { u32::MAX } else { 0 };
    }
    let pixels = saturating_u32(rounded);
    // A positive CSS length is visible. A finite product that rounded to 0
    // would make the backing store empty on this axis, so the axis becomes 1.
    if css > 0.0 && pixels == 0 {
        return 1;
    }
    pixels
}

/// Convert a finite `f64` to `u32`, saturating at both ends.
///
/// The workspace denies float-to-int `as` casts. The value is read from its
/// IEEE-754 bits instead. Every integer with magnitude at most 2^53 is exact
/// in `f64`, and `u32::MAX` is inside that range, so a value [`f64::round`]
/// produced is not rounded again here. A non-positive input, including a NaN
/// that passed the caller's check, becomes 0. A value at or above 2^32
/// becomes `u32::MAX`.
fn saturating_u32(value: f64) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    let bits = value.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let Ok(biased) = u32::try_from(biased) else {
        return u32::MAX;
    };
    // Bias is 1023. A smaller biased exponent is a magnitude below 1, which
    // is 0 after the caller has already rounded to an integer.
    if biased < 1023 {
        return 0;
    }
    let unbiased = biased - 1023;
    if unbiased >= 32 {
        return u32::MAX;
    }
    let mantissa = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let shift = 52 - unbiased;
    let whole = mantissa >> shift;
    u32::try_from(whole).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{ExtentError, css_from_device, device_extent, usable_ratio};

    /// The one ratio policy the CPU extent and the GPU camera both use.
    #[test]
    fn a_ratio_is_usable_only_when_it_narrows_to_a_positive_f32() {
        assert!(usable_ratio(1.0));
        assert!(usable_ratio(1.25));
        assert!(usable_ratio(2.0));
        assert!(!usable_ratio(0.0));
        assert!(!usable_ratio(-1.0));
        assert!(!usable_ratio(f64::NAN));
        assert!(!usable_ratio(f64::INFINITY));
        assert!(!usable_ratio(f64::MIN_POSITIVE / 2.0));
    }

    /// CSS 100 by 50 at the three ratios the story names.
    ///
    /// Hand-computed, not copied from a run of this function.
    /// dpr 1: 100 * 1 = 100 and 50 * 1 = 50.
    /// dpr 2: 100 * 2 = 200 and 50 * 2 = 100. The CSS size itself would be
    /// 100 by 50, and that answer is what this row refuses.
    /// dpr 1.25: 100 * 1.25 = 125 and 50 * 1.25 = 62.5. `f64::round` takes
    /// halfway cases away from zero, so 62.5 becomes 63.
    #[test]
    fn extent_at_dpr_one_two_and_one_and_a_quarter() {
        assert_eq!(css_from_device(100.0, 1.0), Ok(100.0));
        assert_eq!(css_from_device(200.0, 2.0), Ok(100.0));
        assert_eq!(css_from_device(125.0, 1.25), Ok(100.0));
        assert_eq!(
            css_from_device(1.0, 0.0),
            Err(ExtentError::NonPositiveRatio)
        );
        // A positive subnormal dpr narrows to a non-positive f32 and cannot be
        // carried by the GPU camera uniform. Refusing it keeps the CPU and GPU
        // paths from disagreeing about the same input.
        let subnormal = f64::MIN_POSITIVE / 2.0;
        assert_eq!(
            css_from_device(1.0, subnormal),
            Err(ExtentError::NonPositiveRatio)
        );
        assert_eq!(
            device_extent(1.0, 1.0, subnormal),
            Err(ExtentError::NonPositiveRatio)
        );
        assert_eq!(device_extent(100.0, 50.0, 1.0), Ok((100, 50)));
        assert_eq!(device_extent(100.0, 50.0, 2.0), Ok((200, 100)));
        assert_eq!(device_extent(100.0, 50.0, 1.25), Ok((125, 63)));
        // 2.5 is halfway. Ties-to-even would answer 2. Half away from zero
        // answers 3. This row is what makes those two disagree.
        assert_eq!(device_extent(2.5, 1.0, 1.0), Ok((3, 1)));
    }

    /// A positive CSS size whose rounded product is 0 becomes a non-empty
    /// backing store. Each strictly positive axis that rounded to 0 becomes
    /// 1, which is the smallest extent whose product is at least 1 without
    /// discarding an axis that did round to pixels.
    #[test]
    fn a_positive_css_size_whose_product_rounds_to_zero_becomes_one() {
        assert_eq!(device_extent(0.4, 0.4, 1.0), Ok((1, 1)));
        assert_eq!(device_extent(0.4, 10.0, 1.0), Ok((1, 10)));
    }

    #[test]
    fn a_non_positive_device_pixel_ratio_is_refused() {
        assert_eq!(
            device_extent(100.0, 50.0, 0.0),
            Err(ExtentError::NonPositiveRatio)
        );
        assert_eq!(
            device_extent(100.0, 50.0, -2.0),
            Err(ExtentError::NonPositiveRatio)
        );
        assert_eq!(
            device_extent(100.0, 50.0, -0.0),
            Err(ExtentError::NonPositiveRatio)
        );
    }

    #[test]
    fn a_non_finite_input_is_refused() {
        assert_eq!(
            device_extent(f64::NAN, 50.0, 1.0),
            Err(ExtentError::NonFinite)
        );
        assert_eq!(
            device_extent(100.0, f64::INFINITY, 1.0),
            Err(ExtentError::NonFinite)
        );
        assert_eq!(
            device_extent(100.0, 50.0, f64::NEG_INFINITY),
            Err(ExtentError::NonFinite)
        );
    }

    /// `1e308` is finite and at least `2^32`, so at dpr 1 it saturates and
    /// the other axis stays 10. The same CSS length at dpr 10 overflows to
    /// +infinity. That axis saturates at `u32::MAX`. The other axis of that
    /// call is `1 * 10 = 10`. The empty-axis floor is only a rounded 0.
    #[test]
    fn an_overflowing_product_saturates_to_u32_max() {
        assert_eq!(device_extent(1e308, 10.0, 1.0), Ok((u32::MAX, 10)));
        assert_eq!(device_extent(1e308, 1.0, 10.0), Ok((u32::MAX, 10)));
    }

    /// `u32::MAX` is exactly representable. Twice that saturates rather than
    /// wrapping, which is the cast the signature names.
    #[test]
    fn the_cast_saturates_at_u32_max() {
        let limit = f64::from(u32::MAX);
        assert_eq!(device_extent(limit, 1.0, 1.0), Ok((u32::MAX, 1)));
        assert_eq!(device_extent(limit, 1.0, 2.0), Ok((u32::MAX, 2)));
    }
}
