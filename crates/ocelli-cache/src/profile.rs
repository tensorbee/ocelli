//! A load described by dimensions, not by an allocated volume.
//!
//! F-113. HLD section 8. Encoded bytes are already dropped. Decoded bytes are
//! the product of the dimensions. GPU bytes are the caller's figure, and a
//! tier C session cannot claim any.

use crate::lru::admits;
use crate::tier::{CacheTier, Pressure};

/// The dimensions and budgets of one synthetic load.
///
/// `times` is the fourth dimension. `samples` is 1 or 3. `bytes_per_sample`
/// is 1 or 2. Those limits are a local decision. Section 8 does not give a
/// formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSpec {
    /// Columns of one frame.
    pub columns: u32,
    /// Rows of one frame.
    pub rows: u32,
    /// Slices in the volume.
    pub slices: u32,
    /// The fourth dimension. At least 1.
    pub times: u32,
    /// Samples per voxel. 1 or 3.
    pub samples: u32,
    /// Bytes per sample. 1 or 2.
    pub bytes_per_sample: u32,
    /// Budget of the encoded tier.
    pub encoded_budget: usize,
    /// Budget of the decoded tier.
    pub decoded_budget: usize,
    /// Budget of the GPU tier.
    pub gpu_budget: usize,
    /// Caller figure for GPU bytes. Zero on tier C.
    pub gpu_bytes: usize,
    /// Tier C reports no texture.
    pub tier_c: bool,
}

/// The three pressures for one load that decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadProfile {
    /// Encoded used is 0. The bytes were dropped at decode.
    pub encoded: Pressure,
    /// Decoded used is the product when it fits, and 0 when it does not.
    pub decoded: Pressure,
    /// GPU used is the caller figure when it fits, and 0 on tier C.
    pub gpu: Pressure,
    /// The decoded product, whether or not it was admitted.
    pub decoded_bytes: usize,
    /// The GPU figure the caller passed, whether or not it was admitted.
    pub gpu_requested: usize,
    /// `admits` on the decoded budget and `decoded_bytes`.
    pub decoded_admitted: bool,
    /// `admits` on the GPU budget and the GPU figure.
    pub gpu_admitted: bool,
}

/// Report the three tiers for one load.
///
/// Returns nothing for a zero dimension, a sample count other than 1 or 3,
/// a byte width other than 1 or 2, a product that does not fit in `usize`,
/// or a non-zero GPU figure on tier C. It does not allocate the volume.
#[must_use]
pub fn profile_load(spec: LoadSpec) -> Option<LoadProfile> {
    if spec.samples != 1 && spec.samples != 3 {
        return None;
    }
    if spec.bytes_per_sample != 1 && spec.bytes_per_sample != 2 {
        return None;
    }
    if spec.tier_c && spec.gpu_bytes != 0 {
        return None;
    }
    let product = checked_product(&[
        spec.columns,
        spec.rows,
        spec.slices,
        spec.times,
        spec.samples,
        spec.bytes_per_sample,
    ])?;
    let product = fits_address_bits(product, usize::BITS)?;
    let decoded_bytes = usize::try_from(product).ok()?;
    let decoded_admitted = admits(spec.decoded_budget, decoded_bytes);
    let gpu_admitted = admits(spec.gpu_budget, spec.gpu_bytes);
    Some(LoadProfile {
        encoded: Pressure {
            tier: CacheTier::Encoded,
            used: 0,
            budget: spec.encoded_budget,
        },
        decoded: Pressure {
            tier: CacheTier::Decoded,
            used: if decoded_admitted { decoded_bytes } else { 0 },
            budget: spec.decoded_budget,
        },
        gpu: Pressure {
            tier: CacheTier::Gpu,
            used: if gpu_admitted { spec.gpu_bytes } else { 0 },
            budget: spec.gpu_budget,
        },
        decoded_bytes,
        gpu_requested: spec.gpu_bytes,
        decoded_admitted,
        gpu_admitted,
    })
}

/// `product` when it fits in an address of `bits`, and nothing otherwise.
///
/// A 64-bit address holds every `u64`. A narrower address holds a product
/// only when it is below two to the power of `bits`. Callers pass `usize::BITS`.
fn fits_address_bits(product: u64, bits: u32) -> Option<u64> {
    if bits >= 64 {
        Some(product)
    } else {
        let limit = 1u64.checked_shl(bits)?;
        if product < limit { Some(product) } else { None }
    }
}

fn checked_product(parts: &[u32]) -> Option<u64> {
    let mut total = 1u64;
    for part in parts {
        if *part == 0 {
            return None;
        }
        total = total.checked_mul(u64::from(*part))?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fourth dimension of the large fixture. The factor is 1.
    const LARGE_TIMES: u64 = 1;
    /// Samples of the large fixture. The factor is 1.
    const LARGE_SAMPLES: u64 = 1;
    /// 512 * 512 * 300 * LARGE_TIMES * LARGE_SAMPLES * 2.
    const LARGE: u64 = 512 * 512 * 300 * LARGE_TIMES * LARGE_SAMPLES * 2;
    /// Samples of the 4D fixture. The factor is 1.
    const FOUR_SAMPLES: u64 = 1;
    /// 64 * 64 * 8 * 4 * FOUR_SAMPLES * 2.
    const FOUR_D: u64 = 64 * 64 * 8 * 4 * FOUR_SAMPLES * 2;

    fn fitting(product: u64) -> usize {
        usize::try_from(product).unwrap_or(0)
    }

    fn demand(profile: Option<LoadProfile>) -> LoadProfile {
        profile.unwrap_or(LoadProfile {
            encoded: Pressure {
                tier: CacheTier::Encoded,
                used: 0,
                budget: 0,
            },
            decoded: Pressure {
                tier: CacheTier::Decoded,
                used: 0,
                budget: 0,
            },
            gpu: Pressure {
                tier: CacheTier::Gpu,
                used: 0,
                budget: 0,
            },
            decoded_bytes: 0,
            gpu_requested: 0,
            decoded_admitted: false,
            gpu_admitted: false,
        })
    }

    fn large(decoded_budget: usize, tier_c: bool, gpu_bytes: usize) -> LoadSpec {
        LoadSpec {
            columns: 512,
            rows: 512,
            slices: 300,
            times: 1,
            samples: 1,
            bytes_per_sample: 2,
            encoded_budget: decoded_budget,
            decoded_budget,
            gpu_budget: decoded_budget,
            gpu_bytes,
            tier_c,
        }
    }

    #[test]
    fn a_large_volume_is_157286400_decoded_bytes() {
        assert_eq!(LARGE, 157_286_400);
        let profile = demand(profile_load(large(fitting(LARGE), true, 0)));
        assert_eq!(profile.decoded_bytes, 157_286_400);
        assert!(profile.decoded_admitted);
        assert_eq!(profile.decoded.used, 157_286_400);
        assert_eq!(profile.encoded.used, 0);
        assert_eq!(profile.encoded.tier, CacheTier::Encoded);
        assert_eq!(profile.gpu.used, 0);
        assert_eq!(profile.gpu.tier, CacheTier::Gpu);
    }

    #[test]
    fn a_4d_load_is_262144_decoded_bytes() {
        assert_eq!(FOUR_D, 262_144);
        let profile = demand(profile_load(LoadSpec {
            columns: 64,
            rows: 64,
            slices: 8,
            times: 4,
            samples: 1,
            bytes_per_sample: 2,
            encoded_budget: 262_144,
            decoded_budget: 262_144,
            gpu_budget: 262_144,
            gpu_bytes: 262_144,
            tier_c: false,
        }));
        assert_eq!(profile.decoded_bytes, 262_144);
        assert!(profile.decoded_admitted);
        assert!(profile.gpu_admitted);
        assert_eq!(profile.gpu.used, 262_144);
        assert_eq!(profile.encoded.used, 0);
    }

    #[test]
    fn one_byte_under_the_product_is_not_admitted() {
        let profile = demand(profile_load(large(fitting(LARGE) - 1, true, 0)));
        assert_eq!(profile.decoded_bytes, 157_286_400);
        assert!(!profile.decoded_admitted);
        assert_eq!(profile.decoded.used, 0);
        assert_eq!(profile.encoded.used, 0);
    }

    #[test]
    fn a_zero_dimension_a_bad_sample_and_an_overflow_return_nothing() {
        let mut spec = large(1, true, 0);
        spec.times = 0;
        assert!(profile_load(spec).is_none());
        spec.times = 1;
        spec.samples = 2;
        assert!(profile_load(spec).is_none());
        spec.samples = 1;
        spec.bytes_per_sample = 4;
        assert!(profile_load(spec).is_none());
        spec.bytes_per_sample = 2;
        spec.columns = u32::MAX;
        spec.rows = u32::MAX;
        spec.slices = u32::MAX;
        spec.times = 2;
        assert!(profile_load(spec).is_none());
    }

    #[test]
    fn tier_c_rejects_a_nonzero_gpu_figure_and_reports_zero_otherwise() {
        assert!(profile_load(large(fitting(LARGE), true, 1)).is_none());
        let profile = demand(profile_load(large(fitting(LARGE), true, 0)));
        assert_eq!(profile.gpu.used, 0);
        assert_eq!(profile.gpu_requested, 0);
    }

    #[test]
    fn a_65536_square_fits_a_wide_address_only() {
        const SQUARE: u64 = 65536 * 65536;
        assert_eq!(SQUARE, 4_294_967_296);
        assert!(fits_address_bits(SQUARE, 32).is_none());
        assert_eq!(fits_address_bits(SQUARE, 64), Some(SQUARE));
        assert_eq!(
            fits_address_bits(u64::from(u32::MAX), 32),
            Some(u64::from(u32::MAX))
        );
        let budget = fitting(SQUARE);
        let spec = LoadSpec {
            columns: 65536,
            rows: 65536,
            slices: 1,
            times: 1,
            samples: 1,
            bytes_per_sample: 1,
            encoded_budget: budget,
            decoded_budget: budget,
            gpu_budget: budget,
            gpu_bytes: 0,
            tier_c: true,
        };
        if usize::BITS >= 64 {
            let profile = demand(profile_load(spec));
            assert_eq!(profile.decoded_bytes, budget);
            assert!(profile.decoded_admitted);
        } else {
            assert!(profile_load(spec).is_none());
        }
    }

    #[test]
    fn three_samples_are_786432_and_an_over_budget_gpu_figure_is_unused() {
        const COLOUR: u64 = 64 * 64 * 8 * 4 * 3 * 2;
        assert_eq!(COLOUR, 786_432);
        let budget = fitting(COLOUR);
        let profile = demand(profile_load(LoadSpec {
            columns: 64,
            rows: 64,
            slices: 8,
            times: 4,
            samples: 3,
            bytes_per_sample: 2,
            encoded_budget: budget,
            decoded_budget: budget,
            gpu_budget: 9,
            gpu_bytes: 10,
            tier_c: false,
        }));
        assert_eq!(profile.decoded_bytes, budget);
        assert!(profile.decoded_admitted);
        assert_eq!(profile.decoded.used, budget);
        assert_eq!(profile.gpu_requested, 10);
        assert!(!profile.gpu_admitted);
        assert_eq!(profile.gpu.used, 0);
        assert_eq!(profile.encoded.used, 0);
    }
}
