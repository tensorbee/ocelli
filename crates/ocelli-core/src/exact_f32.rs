//! One IEEE-754 reading of an exact integer `f32`.
//!
//! Sample conversion and NIfTI voxel offsets both need this decomposition.
//! A second copy can accept a fraction the first one refuses.

use core::num::FpCategory;

/// One finite f32 that is an integer and fits in `u64`.
///
/// Negative zero is recorded as a non-negative zero. The sign is separate so
/// a caller that rejects negatives can do so without a second decomposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactF32Integer {
    /// True when the value is negative and not zero.
    pub negative: bool,
    /// Absolute value.
    pub magnitude: u64,
}

/// Why an f32 is not an exact `u64` magnitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactF32Error {
    /// Non-finite, fractional, or smaller than one and not zero.
    NotAnInteger,
    /// The integer does not fit in `u64`.
    ExceedsU64,
}

/// Decompose an f32 that is an exact integer.
///
/// Zero and negative zero succeed as a non-negative zero. A non-finite value
/// or a fraction is [`ExactF32Error::NotAnInteger`]. A magnitude above `u64`
/// is [`ExactF32Error::ExceedsU64`], including a negative whose absolute value
/// does not fit. Callers that treat every negative as invalid must check the
/// sign before they map [`ExactF32Error::ExceedsU64`] to overflow.
pub fn exact_f32_integer(value: f32) -> Result<ExactF32Integer, ExactF32Error> {
    if value.classify() == FpCategory::Zero {
        return Ok(ExactF32Integer {
            negative: false,
            magnitude: 0,
        });
    }
    if !value.is_finite() {
        return Err(ExactF32Error::NotAnInteger);
    }
    let bits = value.to_bits();
    let negative = bits >> 31 != 0;
    let exponent_bits = (bits >> 23) & 0xff;
    let exponent = i32::try_from(exponent_bits).map_err(|_| ExactF32Error::ExceedsU64)? - 127;
    if exponent < 0 {
        return Err(ExactF32Error::NotAnInteger);
    }
    let significand = u64::from((bits & 0x007f_ffff) | 0x0080_0000);
    let magnitude = if exponent >= 23 {
        let shift = u32::try_from(exponent - 23).map_err(|_| ExactF32Error::ExceedsU64)?;
        let maximum = u64::MAX
            .checked_shr(shift)
            .ok_or(ExactF32Error::ExceedsU64)?;
        if significand > maximum {
            return Err(ExactF32Error::ExceedsU64);
        }
        significand << shift
    } else {
        let shift = u32::try_from(23 - exponent).map_err(|_| ExactF32Error::ExceedsU64)?;
        let mask = 1_u64
            .checked_shl(shift)
            .and_then(|width| width.checked_sub(1))
            .ok_or(ExactF32Error::ExceedsU64)?;
        if significand & mask != 0 {
            return Err(ExactF32Error::NotAnInteger);
        }
        significand >> shift
    };
    Ok(ExactF32Integer {
        negative,
        magnitude,
    })
}

#[cfg(test)]
mod tests {
    use super::{ExactF32Error, ExactF32Integer, exact_f32_integer};

    #[test]
    fn zero_fraction_and_the_u64_boundary() {
        assert_eq!(
            exact_f32_integer(-0.0),
            Ok(ExactF32Integer {
                negative: false,
                magnitude: 0,
            })
        );
        assert_eq!(
            exact_f32_integer(f32::from_bits(0x5f7f_ffff)),
            Ok(ExactF32Integer {
                negative: false,
                magnitude: 0xffff_ff00_0000_0000,
            })
        );
        assert_eq!(
            exact_f32_integer(f32::from_bits(0x5f80_0000)),
            Err(ExactF32Error::ExceedsU64)
        );
        assert_eq!(
            exact_f32_integer(f32::from_bits(0x5f80_0001)),
            Err(ExactF32Error::ExceedsU64)
        );
        assert_eq!(exact_f32_integer(0.5), Err(ExactF32Error::NotAnInteger));
        assert_eq!(
            exact_f32_integer(f32::NAN),
            Err(ExactF32Error::NotAnInteger)
        );
    }
}
