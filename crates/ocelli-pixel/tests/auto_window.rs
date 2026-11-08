//! Automatic window fixtures.
//!
//! Every expected value below is hand-computed from DICOM PS3.3 C.11.1 (the
//! modality stage, applied per frame), C.11.2.1.3.2 (LINEAR_EXACT), C.7.5.1.1.2
//! (Pixel Padding Value, in stored space), C.7.6.3.1 (signed Pixel
//! Representation) and C.7.6.16.2.9 (per-frame rescale in an enhanced object),
//! together with the plan's nearest-rank rule: for `n` counted values sorted
//! ascending, `low = x[n.div_ceil(100)]` and `high = x[(99 n).div_ceil(100)]`,
//! one-based, and the operator's answers of 2026-11-05.
//!
//! Grey is the display value through `floor(y + 0.5)` clamped to 255, the
//! rounding `display_index` in `ocelli-render` performs. The window check and
//! its search follow the lead's amendments of 2026-11-05 and 2026-11-06
//! after review passes 1 to 6. Since pass 5, percentiles fewer than three
//! f32 units apart take the Constant window and are not a Percentile window.
//! Since pass 6, a Constant window of equal percentiles is checked at the
//! value moved 16 f32 units each way too, as a close pair is.
//!
//! Since pass 4 a Percentile window is searched on the 1st and 99th
//! percentiles each moved 16 f32 units toward the other, and is accepted
//! only when all four values draw exactly the output minimum or maximum. For
//! the integer percentiles of the AC fixtures below 256 in magnitude, 16
//! units is at most 16 * 2^-16 = 2^-12 = 2.4e-4, in [128, 256), so the two
//! moves together are at most 2^-11 = 4.9e-4 and a centre or width written
//! there as an integer is that value to within `assert_close`'s 0.001. The
//! grey each fixture asserts for a percentile is what that fixture
//! measures. A fixture whose window moves further, or whose bits are
//! asserted, derives the moved values in its own comment.

use ocelli_core::{Modality, Stored};
use ocelli_pixel::{
    AUTO_WINDOW_SAMPLE_CAP, AutoWindowBasis, AutoWindowError, AutoWindowPool, ModalityTransform,
    PixelError, StoredRange, VoiFunction, VoiTransform, auto_window, series_sample,
};

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok());
        let Ok(value) = result else { return };
        value
    }};
}

#[track_caller]
fn assert_close(actual: f32, expected: f32) {
    let tolerance = 0.001;
    assert!(
        (actual - expected).abs() < tolerance,
        "was {actual}, wanted {expected}"
    );
}

fn rescale(slope: f32, intercept: f32) -> Result<ModalityTransform, PixelError> {
    ModalityTransform::new(None, Some(slope), Some(intercept))
}

/// A stored fixture value. Exact in `u16`, the conversion `tests/palette.rs`
/// uses, and the fixture values never reach its bound.
fn stored(value: usize) -> Stored {
    Stored(f32::from(u16::try_from(value).unwrap_or(0)))
}

/// The same, signed, for the C.7.6.3.1 fixture.
fn signed(value: i32) -> Stored {
    Stored(f32::from(i16::try_from(value).unwrap_or(0)))
}

/// Deterministic splitmix64, so the distribution fixtures draw the same
/// values on every run and their refusal count is a pinned fact, not a
/// per-run sample. `next_f32_unit` is uniform over the 24 bit grid of
/// [0, 1), exact in f32.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        SplitMix64(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_f32_unit(&mut self) -> f32 {
        // Uniform over the 2^24 grid of [0, 1). The numerator is rebuilt from
        // two exact halves, an integer below 2^24, so the value and the
        // quotient are exact in f32.
        let m = u32::try_from(self.next_u64() >> 40).unwrap_or(0);
        let hi = u16::try_from(m >> 12).unwrap_or(0);
        let lo = u16::try_from(m & 0xFFF).unwrap_or(0);
        (f32::from(hi) * 4096.0 + f32::from(lo)) / 16777216.0
    }

    fn next_f32_range(&mut self, hi: f32) -> f32 {
        self.next_f32_unit() * hi
    }
}

/// `display_index`: the nearest display code, ties up, clamped to the range.
#[track_caller]
fn assert_grey(display: f32, expected: f32) {
    let grey = (display + 0.5).floor().clamp(0.0, 255.0);
    assert_eq!(grey.to_bits(), expected.to_bits());
}

/// The display value is the exact expected `f32`, bit for bit. The window
/// check's contract is exact edge and middle values through
/// `VoiTransform::apply`, so the fixtures assert those bits and not merely
/// the nearest grey.
#[track_caller]
fn assert_display_exact(display: f32, expected: f32) {
    assert_eq!(
        display.to_bits(),
        expected.to_bits(),
        "was {display}, wanted {expected}"
    );
}

/// AC1. PS3.3 C.11.1 and C.11.2.1.3.2, and the operator's first answer of
/// 2026-11-05: background zeros are ignored.
///
/// 200 samples, slope 0.5, intercept 0. Stored 0 one hundred times (excluded),
/// stored 1 to 99, and one stored 1000. Modality values of the counted set are
/// 0.5, 1.0, ..., 49.5 and 500.0, so n is 100. The ranks are
/// 100.div_ceil(100) = 1 and (99 * 100).div_ceil(100) = 99, so low = 0.5 and
/// high = 49.5.
///
/// The margin of the lead's amendment after review pass 4 moves each 16 f32
/// units toward the other. 0.5 is in [0.5, 1), spacing 2^-24, so it moves to
/// 0.5 + 2^-20 (0x3F000010). 49.5 is in [32, 64), spacing 2^-18, so it moves
/// to 49.5 - 2^-14 (0x4245FFF0). The width 49 - 2^-14 - 2^-20 is a quarter of
/// a spacing above 49 - 2^-14 and rounds to it, 48.999939 (0x4243FFF0). Its
/// half 24.5 - 2^-15 is exact, and the centre 0.5 + 2^-20 + 24.5 - 2^-15 is
/// the tie between 25 - 2^-15 and the next f32 up in [16, 32), spacing 2^-19,
/// and rounds to the even 25 - 2^-15 = 24.999969 (0x41C7FFF0). The edges are
/// then 0.5 and 49.5 - 2^-14, both exact. The moved low 0.5 + 2^-20 is on the
/// ramp, where x - centre = -24.5 + 33 * 2^-20 ties onto the even
/// -(24.5 - 2^-15), minus the half width, so the ramp gives (-0.5 + 0.5) *
/// 255 = 0.0 exactly. The moved high is the upper edge, (0.5 + 0.5) * 255 =
/// 255.0 exactly. 0.5 is at the lower edge and 49.5 above the upper edge,
/// ymin and ymax. The check passes as computed: width 49 and centre 25 to
/// within 7e-5.
///
/// Stored 20 is modality 10.0: y = ((10 - 24.999969) / 48.999939 + 0.5) * 255
/// = 49.4388, grey 49. Stored 50 is 25.0, 2^-15 above the centre, 127.50015,
/// grey 128. Stored 0 is 0.0, below the lower edge 0.5, grey 0. Stored 1 is
/// 0.5, the lower edge itself, grey 0. Stored 99 is 49.5, above the upper
/// edge, 255. Stored 1000 is above it, 255.
#[test]
fn fixture_pet_like_frame_leaves_out_background_zeros() {
    let mut frame = vec![stored(0); 100];
    frame.extend((1..=99).map(stored));
    frame.push(stored(1000));
    let modality = require_ok!(rescale(0.5, 0.0));

    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.counted, 100);
    assert_close(window.low, 0.5);
    assert_close(window.high, 49.5);
    assert_close(window.center, 25.0);
    assert_close(window.width, 49.0);
    assert_eq!(window.center.to_bits(), 0x41C7_FFF0);
    assert_eq!(window.width.to_bits(), 0x4243_FFF0);
    assert_eq!(window.basis, AutoWindowBasis::Percentile);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_display_exact(voi.apply(Modality(f32::from_bits(0x3F00_0010))).0, 0.0);
    assert_display_exact(voi.apply(Modality(f32::from_bits(0x4245_FFF0))).0, 255.0);
    assert_close(voi.apply(Modality(10.0)).0, 49.4388);
    assert_grey(voi.apply(Modality(10.0)).0, 49.0);
    assert_grey(voi.apply(Modality(25.0)).0, 128.0);
    assert_grey(voi.apply(Modality(0.0)).0, 0.0);
    assert_grey(voi.apply(Modality(0.5)).0, 0.0);
    assert_grey(voi.apply(Modality(49.5)).0, 255.0);
    assert_grey(voi.apply(Modality(500.0)).0, 255.0);
}

/// AC2. PS3.3 C.7.6.16.2.9 and C.11.2.1.3.2. One window over the series, not
/// one per image.
///
/// Frame A is stored 1 to 100, frame B is stored 101 to 200, both slope 1.
/// Pooled, n is 200 and the values are 1 to 200. The ranks are
/// 200.div_ceil(100) = 2 and (99 * 200).div_ceil(100) = 198, so low = 2 and
/// high = 198. Width 196, centre 100. Frame A's stored 100 is modality 100.0:
/// y = ((100 - 100) / 196 + 0.5) * 255 = 127.5, grey 128. Frame B's stored 150
/// is modality 150.0: y = ((150 - 100) / 196 + 0.5) * 255 = 192.551, grey 193.
/// Per image, A alone has n = 100, ranks 1 and 99, low 1, high 99, and its
/// stored 100 is above the upper edge 99, grey 255.
#[test]
fn fixture_series_pool_is_one_window() {
    let frame_a: Vec<Stored> = (1..=100).map(stored).collect();
    let frame_b: Vec<Stored> = (101..=200).map(stored).collect();
    let modality = require_ok!(rescale(1.0, 0.0));

    let mut pool = require_ok!(AutoWindowPool::new(2));
    require_ok!(pool.add_frame(&frame_a, &modality, None));
    require_ok!(pool.add_frame(&frame_b, &modality, None));
    let window = require_ok!(pool.finish());
    assert_eq!(window.counted, 200);
    assert_close(window.low, 2.0);
    assert_close(window.high, 198.0);
    assert_close(window.center, 100.0);
    assert_close(window.width, 196.0);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(100.0)).0, 128.0);
    assert_close(voi.apply(Modality(150.0)).0, 192.551);
    assert_grey(voi.apply(Modality(150.0)).0, 193.0);

    let alone_a = require_ok!(auto_window(&frame_a, &modality, None));
    assert_close(alone_a.center, 50.0);
    assert_close(alone_a.width, 98.0);
    let voi_a = require_ok!(alone_a.transform(0.0, 255.0));
    assert_grey(voi_a.apply(Modality(100.0)).0, 255.0);
}

/// AC3. PS3.3 C.7.6.16.2.9: rescale differs per frame, so the pool holds
/// modality values, each frame through its own stage.
///
/// Frame A is stored 1 to 100 through slope 1, frame B is stored 1 to 100
/// through slope 2. The pooled modality values are A's 1 to 100 and B's 2 to
/// 200. n is 200. Rank 2 is 2. For rank 198: values at or below 196 are A's 1
/// to 100 (100 values) and B's evens 2 to 196 (98 values), 198 in all, so
/// high = 196. Width 194, centre 99. A's stored 50 is modality 50.0:
/// y = ((50 - 99) / 194 + 0.5) * 255 = 63.093, grey 63. B's stored 50 is
/// modality 100.0: y = ((100 - 99) / 194 + 0.5) * 255 = 128.814, grey 129.
#[test]
fn fixture_series_pool_uses_each_frame_rescale() {
    let frame_a: Vec<Stored> = (1..=100).map(stored).collect();
    let frame_b: Vec<Stored> = (1..=100).map(stored).collect();

    let mut pool = require_ok!(AutoWindowPool::new(2));
    require_ok!(pool.add_frame(&frame_a, &require_ok!(rescale(1.0, 0.0)), None));
    require_ok!(pool.add_frame(&frame_b, &require_ok!(rescale(2.0, 0.0)), None));
    let window = require_ok!(pool.finish());
    assert_eq!(window.counted, 200);
    assert_close(window.low, 2.0);
    assert_close(window.high, 196.0);
    assert_close(window.center, 99.0);
    assert_close(window.width, 194.0);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_close(voi.apply(Modality(50.0)).0, 63.093);
    assert_grey(voi.apply(Modality(50.0)).0, 63.0);
    assert_close(voi.apply(Modality(100.0)).0, 128.814);
    assert_grey(voi.apply(Modality(100.0)).0, 129.0);
}

/// AC4. PS3.3 C.7.5.1.1.2 and C.7.6.16.2.9. The padding range is a
/// stored-space predicate declared per object, so it applies to its own frame
/// only.
///
/// Frame A is 50 of stored 0 and stored 1 to 50, no padding. Frame B is 20 of
/// stored -2000 and stored 51 to 100, signed, padding -2000 to -2000 for B
/// only. With B's padding honoured the counted set is 1 to 100, n = 100, ranks
/// 1 and 99, low = 1, high = 99, centre 50, width 98. Without it, the counted
/// set is -2000 twenty times and 1 to 100, n = 120, ranks 2 and 119, low =
/// -2000, high = 99, centre -2000 + 1049.5 = -950.5, width 2099 before the
/// pass 4 margin.
///
/// The margin moves -2000, in [1024, 2048) in magnitude with spacing 2^-13,
/// to -2000 + 2^-9 = -1999.998046875 (0xC4F9FFF0), and 99, in [64, 128)
/// with spacing 2^-17, to 99 - 2^-13 = 98.9998779296875 (0x42C5FFF0). Their difference
/// 2098.9979248046875 is an odd multiple of 2^-13 in [2048, 4096), where the
/// spacing is 2^-12, so it ties and rounds to the even 2098.998046875
/// (0x45032FF8). The half 1049.4990234375 and the centre
/// -1999.998046875 + 1049.4990234375 = -950.4990234375 are exact, and the
/// upper edge is exactly 99.0, above the moved high, which is then on the
/// ramp at 254.99998: the check fails. The search's first attempt narrows one
/// unit, 2^-12, to 2098.997802734375 (0x45032FF7). Its half 1049.4989013671875
/// and the centre -950.4991455078125 (0xC46D9FF2) are exact, the lower edge
/// is exactly the moved low -1999.998046875 and the upper edge exactly
/// 98.999755859375 = 99 - 2^-12. So -2000 and the moved low are at or below
/// the lower edge, exactly 0.0, and the moved high and 99 are above the upper
/// edge, exactly 255.0. Accepted.
#[test]
fn fixture_series_pool_excludes_per_frame() {
    let mut frame_a = vec![stored(0); 50];
    frame_a.extend((1..=50).map(stored));
    let mut frame_b = vec![signed(-2000); 20];
    frame_b.extend((51..=100).map(stored));
    let modality = require_ok!(rescale(1.0, 0.0));
    let padding = StoredRange {
        low: signed(-2000),
        high: signed(-2000),
    };

    let mut pool = require_ok!(AutoWindowPool::new(2));
    require_ok!(pool.add_frame(&frame_a, &modality, None));
    require_ok!(pool.add_frame(&frame_b, &modality, Some(padding)));
    let window = require_ok!(pool.finish());
    assert_eq!(window.counted, 100);
    assert_close(window.low, 1.0);
    assert_close(window.high, 99.0);
    assert_close(window.center, 50.0);
    assert_close(window.width, 98.0);

    let mut unguarded = require_ok!(AutoWindowPool::new(2));
    require_ok!(unguarded.add_frame(&frame_a, &modality, None));
    require_ok!(unguarded.add_frame(&frame_b, &modality, None));
    let unguarded = require_ok!(unguarded.finish());
    assert_eq!(unguarded.counted, 120);
    assert_close(unguarded.low, -2000.0);
    assert_eq!(unguarded.center.to_bits(), 0xC46D_9FF2);
    assert_eq!(unguarded.width.to_bits(), 0x4503_2FF7);
    let voi = require_ok!(unguarded.transform(0.0, 255.0));
    assert_display_exact(voi.apply(Modality(-2000.0)).0, 0.0);
    assert_display_exact(voi.apply(Modality(f32::from_bits(0xC4F9_FFF0))).0, 0.0);
    assert_display_exact(voi.apply(Modality(f32::from_bits(0x42C5_FFF0))).0, 255.0);
    assert_display_exact(voi.apply(Modality(99.0)).0, 255.0);
}

/// AC4. PS3.3 C.7.5.1.1.2 and C.11.1. Stored 0 is the background predicate,
/// not modality 0.
///
/// One frame, an unsigned CT with intercept -1024: 50 of stored 0 and stored
/// 1000 to 1099, slope 1, intercept -1024. Stored 0 is modality -1024, air,
/// and is left out. The
/// counted set is stored 1000 to 1099, modality -24 to 75, n = 100, ranks 1
/// and 99, low = -24, high = 74, centre 25, width 98. Stored 1049 is modality
/// 25.0, the centre, grey 128. Stored 0 is modality -1024, below the lower
/// edge, grey 0. Counting the zeros instead would put low at -1024, and
/// excluding modality 0, the stored 1024, would too.
#[test]
fn fixture_stored_zero_is_left_out_not_modality_zero() {
    let mut frame = vec![stored(0); 50];
    frame.extend((1000..=1099).map(stored));
    let modality = require_ok!(rescale(1.0, -1024.0));

    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.counted, 100);
    assert_close(window.low, -24.0);
    assert_close(window.high, 74.0);
    assert_close(window.center, 25.0);
    assert_close(window.width, 98.0);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(25.0)).0, 128.0);
    assert_grey(voi.apply(Modality(-1024.0)).0, 0.0);
}

/// AC5. The cap split. PS3.3 C.11.1 for the per-frame stage, the plan's cap
/// rule for the stride: budget = 2^20 / 2 = 2^19 per declared frame, so a
/// frame of 2^19 + 1 samples has stride 2 and 262,145 values read.
///
/// Frame A has 2^19 + 1 samples: stored 60000 at odd indices and
/// (i / 2) mod 100 + 1 at even index i. Read from index 0 at stride 2, the
/// counted set from A is the 262,145 even-index values, which are 1 to 100.
/// Frame B is stored 1 to 100, read exactly. The pool is 262,245 values.
/// Value 1 appears 2622 times in A (i / 2 congruent 0 mod 100, from 0 to
/// 262100) plus once in B, 2623 in all, and rank_low is
/// 262245.div_ceil(100) = 2623, so low = 1. Values 1 to 99 fill ranks 1 to
/// 259,623 exactly: 45 values at 2623 each and 54 at 2622 each, and rank_high
/// is (99 * 262245).div_ceil(100) = 259,623, so high = 99. Centre 50, width
/// 98. A frame of 2^20 samples alone in new(1) keeps the whole cap and is read
/// exactly.
#[test]
fn fixture_cap_is_split_across_frames() {
    let mut frame_a = Vec::with_capacity((1_usize << 19) + 1);
    for i in 0..((1_usize << 19) + 1) {
        if i % 2 == 1 {
            frame_a.push(stored(60000));
        } else {
            frame_a.push(stored(i / 2 % 100 + 1));
        }
    }
    let frame_b: Vec<Stored> = (1..=100).map(stored).collect();
    let modality = require_ok!(rescale(1.0, 0.0));

    let mut pool = require_ok!(AutoWindowPool::new(2));
    require_ok!(pool.add_frame(&frame_a, &modality, None));
    require_ok!(pool.add_frame(&frame_b, &modality, None));
    let window = require_ok!(pool.finish());
    assert_eq!(window.counted, 262_245);
    assert_close(window.low, 1.0);
    assert_close(window.high, 99.0);
    assert_close(window.center, 50.0);
    assert_close(window.width, 98.0);

    // Every sample from 1 to 251, so no sample is stored zero and all of the
    // cap is counted.
    let mut full = Vec::with_capacity(1_usize << 20);
    for i in 0..(1_usize << 20) {
        full.push(stored(i % 251 + 1));
    }
    let alone = require_ok!(auto_window(&full, &modality, None));
    assert_eq!(alone.counted, AUTO_WINDOW_SAMPLE_CAP);
}

/// AC6. The sampled image indices of a series. First and last included, at
/// `i_k = k * (M - 1) / (K - 1)` in integer division, all of a series of at
/// most 16, and `series_sample(0)` refused.
#[test]
fn fixture_series_sample_spacing() {
    assert_eq!(require_ok!(series_sample(1)), vec![0]);
    assert_eq!(require_ok!(series_sample(2)), vec![0, 1]);
    assert_eq!(
        require_ok!(series_sample(16)),
        (0..16).collect::<Vec<usize>>()
    );
    assert_eq!(
        require_ok!(series_sample(17)),
        (0..15).chain(16..17).collect::<Vec<usize>>()
    );
    assert_eq!(
        require_ok!(series_sample(267)),
        vec![
            0, 17, 35, 53, 70, 88, 106, 124, 141, 159, 177, 195, 212, 230, 248, 266
        ]
    );
    assert_eq!(
        require_ok!(series_sample(175)),
        vec![
            0, 11, 23, 34, 46, 58, 69, 81, 92, 104, 116, 127, 139, 150, 162, 174
        ]
    );
    assert_eq!(series_sample(0), Err(AutoWindowError::EmptySeries));
}

/// AC7. The nearest-rank rule over one frame, PS3.3 C.11.2.1.3.2 for the grey.
///
/// One frame of stored 1 to 150, slope 1. n = 150, ranks 2 and
/// (99 * 150).div_ceil(100) = 149, so low = 2 and high = 149. Width 147,
/// centre 75.5. Stored 75 is modality 75.0: y = ((75 - 75.5) / 147 + 0.5) *
/// 255 = 0.49659864 * 255 = 126.633, grey 127. Stored 75.5 is the centre,
/// grey 128. Under linear interpolation the percentiles would be 2.49 and
/// 148.51, the same centre 75.5 and the same centre grey, so the window
/// values and the width 146.02 against 147 are what tell it apart, and under
/// zero-based ranks low would be 3 and high 150.
#[test]
fn fixture_nearest_rank_is_exact() {
    let frame: Vec<Stored> = (1..=150).map(stored).collect();
    let modality = require_ok!(rescale(1.0, 0.0));

    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.counted, 150);
    assert_close(window.low, 2.0);
    assert_close(window.high, 149.0);
    assert_close(window.center, 75.5);
    assert_close(window.width, 147.0);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_close(voi.apply(Modality(75.0)).0, 126.633);
    assert_grey(voi.apply(Modality(75.0)).0, 127.0);
    assert_grey(voi.apply(Modality(75.5)).0, 128.0);
}

/// AC7. The ranks round up. For n = 101 the ranks are 101.div_ceil(100) = 2
/// and (99 * 101).div_ceil(100) = 100, so low = 2 and high = 100, width 98,
/// centre 51. Flooring the ranks would give 1 and 99, the same width 98 with
/// centre 50 and low 1, so the centre and low are what tell them apart.
#[test]
fn fixture_ranks_round_up() {
    let frame: Vec<Stored> = (1..=101).map(stored).collect();
    let modality = require_ok!(rescale(1.0, 0.0));

    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.counted, 101);
    assert_close(window.low, 2.0);
    assert_close(window.high, 100.0);
    assert_close(window.center, 51.0);
    assert_close(window.width, 98.0);
}

/// AC8. Edge cases: one stored value, an all-background pool, a signed frame
/// whose stored 0 is left out, and two counted values. PS3.3 C.7.6.3.1 for
/// the signed frame, C.11.2.1.3.2 for the greys.
///
/// Signed frame: stored -50 to 49, 100 samples, slope 1. Stored 0 is left out,
/// so the counted set is -50 to -1 and 1 to 49, n = 99, ranks 1 and
/// (99 * 99).div_ceil(100) = 99, low = -50, high = 49, centre -0.5, width 99.
/// Stored -1 is modality -1.0: y = ((-1 + 0.5) / 99 + 0.5) * 255 = 126.212,
/// grey 126. Stored 0 is 0.0: y = 128.788, grey 129. Stored 1 is 1.0: y =
/// 131.364, grey 131.
///
/// Two counted values, stored 3 and 9: the ranks are 1 and 2, and the window
/// is the two values each moved 16 f32 units inward by the pass 4 margin,
/// 3 + 2^-18 and 9 - 2^-16, so centre 6 and width 6 to within 2e-5. Stored 3
/// is below the lower edge, grey 0, stored 6 within 6e-6 of the centre, grey
/// 128, stored 9 above the upper edge, grey 255.
#[test]
fn fixture_constant_background_signed_and_two_values() {
    let constant = [stored(7); 16];
    let window = require_ok!(auto_window(
        &constant,
        &require_ok!(rescale(1.0, 0.0)),
        None
    ));
    assert_eq!(window.basis, AutoWindowBasis::Constant);
    assert_close(window.center, 7.0);
    assert_close(window.width, 1.0);
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(7.0)).0, 128.0);

    let blank = [stored(0); 16];
    let window = require_ok!(auto_window(&blank, &require_ok!(rescale(1.0, 0.0)), None));
    assert_eq!(window.basis, AutoWindowBasis::Background);
    assert_eq!(window.counted, 0);
    assert_close(window.center, 0.5);
    assert_close(window.width, 1.0);
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(0.0)).0, 0.0);

    let mut pool = require_ok!(AutoWindowPool::new(2));
    let modality = require_ok!(rescale(1.0, 0.0));
    require_ok!(pool.add_frame(&[stored(0); 8], &modality, None));
    let padding = StoredRange {
        low: signed(-2000),
        high: signed(-2000),
    };
    require_ok!(pool.add_frame(&[signed(-2000); 8], &modality, Some(padding)));
    let window = require_ok!(pool.finish());
    assert_eq!(window.basis, AutoWindowBasis::Background);
    assert_close(window.center, 0.5);
    assert_close(window.width, 1.0);

    let signed_frame: Vec<Stored> = (-50..=49).map(signed).collect();
    let window = require_ok!(auto_window(&signed_frame, &modality, None));
    assert_eq!(window.counted, 99);
    assert_close(window.center, -0.5);
    assert_close(window.width, 99.0);
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_close(voi.apply(Modality(-1.0)).0, 126.212);
    assert_grey(voi.apply(Modality(-1.0)).0, 126.0);
    assert_grey(voi.apply(Modality(0.0)).0, 129.0);
    assert_grey(voi.apply(Modality(1.0)).0, 131.0);

    let two = [stored(3), stored(9)];
    let window = require_ok!(auto_window(&two, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Percentile);
    assert_close(window.center, 6.0);
    assert_close(window.width, 6.0);
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(3.0)).0, 0.0);
    assert_grey(voi.apply(Modality(6.0)).0, 128.0);
    assert_grey(voi.apply(Modality(9.0)).0, 255.0);
}

/// D1, the review's background trigger. PS3.3 C.11.1 and C.11.2.1.3.2, and
/// C.7.5.1.1.2 for the padding.
///
/// One stored sample 32768, slope 256, left out by its own padding, so the
/// frame's largest left-out modality value is 32768 * 256 = 8388608.0, exact.
/// The window that draws this frame must map 8388608.0 to the output minimum.
/// The centre first computed is 8388608.0 + 0.5, which rounds to 8388608.0
/// (the tie at 8388608.5 goes to the even mantissa), so the lower edge
/// 8388608.0 - 0.5 = 8388607.5 sits below the sample and the ramp draws it at
/// 127.5, grey 128. The search's first attempt widens the width by one f32
/// unit of 1.0, 2^-23, to 1.0000001, and keeps the centre the midpoint of the
/// window anchored at 8388608.0: the centre
/// 8388608.0 + 0.50000006 rounds up to 8388609.0, the lower edge
/// 8388609.0 - 0.50000006 rounds down to 8388608.0, and the sample at or below
/// the lower edge maps to ymin, grey 0.
#[test]
fn the_window_check_draws_an_all_padding_frame_black() {
    let modality = require_ok!(rescale(256.0, 0.0));
    let padding = StoredRange {
        low: Stored(32768.0),
        high: Stored(32768.0),
    };

    let window = require_ok!(auto_window(&[Stored(32768.0)], &modality, Some(padding)));
    assert_eq!(window.counted, 0);
    assert_eq!(window.basis, AutoWindowBasis::Background);
    assert_close(window.center, 8388609.0);
    // The width by bits: 1.0 is the unwidened width and must not pass.
    assert_eq!(window.width.to_bits(), 1.0000001f32.to_bits());
    // The upper edge is the centre plus half the width, in the transform's
    // own f32 operations.
    assert_eq!(window.high.to_bits(), 8388610.0f32.to_bits());

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(8388608.0)).0, 0.0);
}

/// D1, the review's second background trigger. PS3.3 C.11.1 and
/// C.11.2.1.3.2, and C.7.5.1.1.2 for the padding.
///
/// One stored sample 33554430 = 2^25 - 2, slope 0.125, left out by its own
/// padding, so the largest left-out modality value is 33554430 * 0.125 =
/// 2^22 - 0.25 = 4194303.75, exact. The centre first computed is
/// 4194303.75 + 0.5 = 4194304.25, the tie between 4194304.0 and 4194304.5,
/// which rounds to the even mantissa 4194304.0, so the lower edge 4194303.5
/// sits below the sample and the ramp draws it at 63.75, grey 64. The search's
/// first attempt widens the width by one f32 unit of 1.0, 2^-23, to
/// 1.0000001: the centre 4194303.75 + 0.50000006 rounds up to
/// 4194304.5, the lower edge 4194304.5 - 0.50000006 rounds down to 4194304.0,
/// and the sample maps to ymin, grey 0.
#[test]
fn the_window_check_draws_the_4194303_75_maximum_black() {
    let maximum = 4194304.0 - 0.25;
    let modality = require_ok!(rescale(0.125, 0.0));
    let padding = StoredRange {
        low: Stored(33554430.0),
        high: Stored(33554430.0),
    };

    let window = require_ok!(auto_window(&[Stored(33554430.0)], &modality, Some(padding)));
    assert_eq!(window.counted, 0);
    assert_eq!(window.basis, AutoWindowBasis::Background);
    assert_close(window.center, 4194304.5);
    // The width by bits: 1.0 is the unwidened width and must not pass. The
    // upper edge is 4194305.0, which centre + width would not give.
    assert_eq!(window.width.to_bits(), 1.0000001f32.to_bits());
    assert_eq!(window.high.to_bits(), 4194305.0f32.to_bits());

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(maximum)).0, 0.0);
}

/// Percentiles fewer than three f32 units apart take the Constant window, per
/// the lead's amendment of 2026-11-06 after review pass 5. PS3.3
/// C.11.2.1.3.2.
///
/// Such a pair is constant to f32 precision. Its window is the Constant
/// window centred on the midpoint of the pair, `low + (high - low) / 2` in
/// f32, width 1 widened by the Constant search, and the check asks that the
/// centre draw exactly the middle 127.5 and that each percentile, and each
/// moved 16 f32 units away from the other, draw within half a grey of it,
/// above 127.0 and below 128.0.
///
/// 10.0 = 1.25 * 2^3 is in [8, 16), where the f32 spacing is 2^-20. Slope 1,
/// n = 2, ranks 1 and 2, so low is 10.0 (0x41200000) and high the other.
/// At width 1 the half width 0.5 is a multiple of the spacing, so the lower
/// edge is the centre minus 0.5 and the upper edge the centre plus 0.5,
/// both exact, and every value below lies between them on the ramp
/// ((x - c) / 1 + 0.5) * 255.
///
/// - d = 1, high 0x41200001. The span is 2^-20 and its half 2^-21 is the
///   tie between 10.0 (even) and 0x41200001 (odd), so the midpoint rounds
///   to 10.0, the 1st percentile. 10.0 draws (0 + 0.5) * 255 = 127.5. The
///   99th is x - c = 2^-20 above it: 0.5 + 2^-20 is exact in [0.5, 1), and
///   times 255 is 127.5 + 31.875 * 2^-17, which rounds in [64, 128), spacing
///   2^-17, to 127.5 + 2^-12 = 127.50024. The values moved 16 units out are
///   16 and 17 units from the centre, at most 17 * 255 * 2^-20 = 0.0041
///   from 127.5. Accepted at width 1, grey 128 and 128.
/// - d = 2, high 0x41200002. The span 2^-19 halves exactly to 2^-20, so
///   the midpoint is 0x41200001, an f32. It draws 127.5. The 1st is one
///   unit below: 0.5 - 2^-20 is exact in [0.25, 0.5), and times 255 is
///   127.5 - 31.875 * 2^-17, which rounds to 127.5 - 2^-12 = 127.49976,
///   grey 127. The 99th draws 127.5 + 2^-12, grey 128. Accepted at width 1.
///   Centred on the low value instead, the 99th would be two units above
///   the centre and the centre would be 0x41200000.
/// - d = 3, high 0x41200003, is a Percentile window. The margin moves each
///   min(16, (3 - 1) / 2) = 1 unit, to L1 = 0x41200001 and L2 = 0x41200002.
///   The first width is 2^-20 and the centre L1 + 2^-21 is the tie between
///   L1 (odd) and L2 (even), so it rounds to L2, and both edges L2 -/+ 2^-21
///   tie onto L2 as well. L2 is then at or below the lower edge and draws
///   ymin, 0.0, where the 99th side needs 255.0: the computed window fails.
///   2^-20 is a power of two, so one unit below it is 2^-44 and the first
///   narrowing attempt is 2^-20 - 2^-44 (0x357FFFFF). Its half h is just
///   under 2^-21, so L1 + h is nearer L1 than L2 and the centre is L1, and
///   L1 -/+ h, within half the spacing, round onto L1 too. L1 and 10.0 are
///   at or below the lower edge, exactly 0.0, and L2 and 0x41200003 above
///   the upper edge, exactly 255.0. Accepted at the first narrowing step.
#[test]
fn percentiles_fewer_than_three_units_apart_take_the_constant_window() {
    let modality = require_ok!(rescale(1.0, 0.0));
    let above = 127.5 + 1.0 / 4096.0;
    let below = 127.5 - 1.0 / 4096.0;

    for (high, center, low_display, high_display) in [
        (0x4120_0001_u32, 0x4120_0000_u32, 127.5f32, above),
        (0x4120_0002, 0x4120_0001, below, above),
    ] {
        let frame = [Stored(10.0), Stored(f32::from_bits(high))];
        let window = require_ok!(auto_window(&frame, &modality, None));
        assert_eq!(window.basis, AutoWindowBasis::Constant, "for {high:#x}");
        assert_eq!(window.low.to_bits(), 10.0f32.to_bits());
        assert_eq!(window.high.to_bits(), high);
        assert_eq!(window.center.to_bits(), center, "for {high:#x}");
        assert_eq!(window.width.to_bits(), 1.0f32.to_bits(), "for {high:#x}");
        let voi = require_ok!(window.transform(0.0, 255.0));
        assert_display_exact(voi.apply(Modality(f32::from_bits(center))).0, 127.5);
        assert_display_exact(voi.apply(Modality(window.low)).0, low_display);
        assert_display_exact(voi.apply(Modality(window.high)).0, high_display);
    }

    let frame = [Stored(10.0), Stored(f32::from_bits(0x4120_0003))];
    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Percentile);
    assert_eq!(window.center.to_bits(), 0x4120_0001);
    assert_eq!(window.width.to_bits(), 0x357F_FFFF);
    let voi = require_ok!(window.transform(0.0, 255.0));
    for (value, display) in [
        (0x4120_0000_u32, 0.0f32),
        (0x4120_0001, 0.0),
        (0x4120_0002, 255.0),
        (0x4120_0003, 255.0),
    ] {
        assert_display_exact(voi.apply(Modality(f32::from_bits(value))).0, display);
    }
}

/// Review pass 4's tier A example, per the lead's amendment of 2026-11-06
/// after review pass 5. PS3.3 C.11.1 and C.11.2.1.3.2.
///
/// Stored 2371 and 2372, slope 9.00603e-7, intercept 1854.0933, so n = 2,
/// ranks 1 and 2. The CPU modality stage rounds the product and then the
/// sum, and gives 0x44E7C30D and 0x44E7C30E, one f32 unit apart in
/// [1024, 2048), where the spacing is 2^-13. That is a Constant window. The
/// span 2^-13 halves to the tie between 0x44E7C30D (odd) and 0x44E7C30E
/// (even), so the midpoint rounds to 0x44E7C30E, the 99th percentile.
///
/// The width starts at 1, and the check asks the values moved 16 units
/// away from the other percentile to draw within half a grey of 127.5. The
/// 1st percentile moved 16 units down is 17 units below the centre, and a
/// unit draws 255 * 2^-13 / w below the middle, so 17 units draw
/// 0.52917 / w. At w = 1 that is more than half a grey and the check
/// fails. The Constant search widens by 2^(k - 23) at attempt `k`: up to
/// k = 18, w = 1.03125, the 17 units draw 0.513139 below and fail. At
/// k = 19, w = 1.0625 = 17 / 16 (0x3F880000), they draw exactly
/// 17 * 255 * 16 / (8192 * 17) = 0.49805 below, 127.00195, and pass. The
/// half width 17 / 32 is a multiple of 2^-13, so the edges are exact and
/// every value here is on the ramp. The centre draws 127.5 exactly.
///
/// The 1st percentile is one unit below, 255 * 16 / (8192 * 17) =
/// 0.029297 below, 127.4707, grey 127. The 99th is the centre, 127.5,
/// grey 128.
///
/// The tier A rescale, emulated with `f32::mul_add` as pass 4 measured on
/// Metal, rounds once and gives 0x44E7C30D for both stored values. Both then
/// draw 127.4707, grey 127, within half a grey of the middle. Before this
/// amendment the window was one unit wide on the pair and the fused 99th
/// drew grey 0.
#[test]
fn the_pass_4_tier_a_pair_draws_the_middle_grey() {
    let slope = 9.00603e-7f32;
    let intercept = 1854.0933f32;
    let modality = require_ok!(rescale(slope, intercept));

    let frame = [Stored(2371.0), Stored(2372.0)];
    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Constant);
    assert_eq!(window.low.to_bits(), 0x44E7_C30D);
    assert_eq!(window.high.to_bits(), 0x44E7_C30E);
    assert_eq!(window.center.to_bits(), 0x44E7_C30E);
    assert_eq!(window.width.to_bits(), 0x3F88_0000);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_display_exact(voi.apply(Modality(window.high)).0, 127.5);
    assert_grey(voi.apply(Modality(window.high)).0, 128.0);
    assert_close(voi.apply(Modality(window.low)).0, 127.4707);
    assert_grey(voi.apply(Modality(window.low)).0, 127.0);

    for stored in [2371.0f32, 2372.0] {
        let fused = stored.mul_add(slope, intercept);
        assert_eq!(fused.to_bits(), 0x44E7_C30D, "for {stored}");
        let display = voi.apply(Modality(fused)).0;
        assert_close(display, 127.4707);
        assert_grey(display, 127.0);
    }
}

/// A close pair at a magnitude where width 1 draws it black and white, which
/// is why the Constant check of a close pair looks at the percentiles and
/// not only at the centre. PS3.3 C.11.2.1.3.2 and the lead's amendment of
/// 2026-11-06 after review pass 5.
///
/// Stored 8388608 = 2^23 and 8388610, slope 1, d = 2, in [2^23, 2^24) where
/// the spacing is 1. The midpoint is 8388609. At width 1 the lower edge
/// 8388608.5 ties onto the even 8388608 and the upper edge 8388609.5 ties
/// onto the even 8388610, so the centre is on the ramp at 127.5, but the 1st
/// percentile is at or below the lower edge and draws 0.0, and the 99th is
/// on the ramp at (1 / 1 + 0.5) * 255 = 382.5. A check at the centre alone
/// accepts that window.
///
/// Each unit from the centre draws 255 / w from the middle. The 99th moved
/// 16 units up is 17 units above it. The 1st moved 16 units down crosses
/// 2^23, below which the spacing is 0.5, and lands on 8388600, 9 below. So
/// the check needs
/// 17 * 255 / w below half a grey, w above 8670. The search tries
/// w = 1 + 2^(k - 23), exact. At k = 36, w = 8193 gives 0.52911 and fails.
/// At k = 37, w = 16385 gives 0.26457 and passes. The half width 8192.5 is
/// exact, and the lower edge 8380416.5 is exact in [2^22, 2^23), spacing
/// 0.5, so every value here is on the ramp. The 1st draws 127.5 - 255 /
/// 16385 = 127.48444, grey 127, and the 99th 127.51556, grey 128. A check of
/// the two percentiles without the moved values would stop at k = 32,
/// w = 513, where one unit draws 0.49708.
#[test]
fn a_close_pair_above_2_23_is_widened_until_it_draws_the_middle_grey() {
    let modality = require_ok!(rescale(1.0, 0.0));

    let frame = [Stored(8388608.0), Stored(8388610.0)];
    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Constant);
    assert_eq!(window.center.to_bits(), 8388609.0f32.to_bits());
    assert_eq!(window.width.to_bits(), 16385.0f32.to_bits());
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_display_exact(voi.apply(Modality(8388609.0)).0, 127.5);
    assert_close(voi.apply(Modality(window.low)).0, 127.48444);
    assert_grey(voi.apply(Modality(window.low)).0, 127.0);
    assert_close(voi.apply(Modality(window.high)).0, 127.51556);
    assert_grey(voi.apply(Modality(window.high)).0, 128.0);
}

/// Review pass 6's D1, constants from 2^15 up, per the lead's amendment of
/// 2026-11-06 after review pass 6. PS3.3 C.11.1 and C.11.2.1.3.2.
///
/// Before pass 6 a constant was checked at its centre alone and kept width 1
/// below 2^23, so a value one f32 unit away drew 127.5 +/- 255 * spacing,
/// grey 129 from 65536. Now the value moved 16 units each way must draw
/// within half a grey of 127.5, so the search widens until w > 510 * D for
/// the larger 16 unit distance D, which at a power of two is the side above.
/// The widths tried are 1 + 2^(k - 23).
///
/// - 32768 = 2^15, spacing 2^-8 above and 2^-9 below: D = 0.0625 up and
///   0.03125 down, w > 31.875. `k` = 27 gives 17, which fails, and `k` = 28
///   gives 33: 127.5 + 0.0625 * 255 / 33 = 127.98295 and 127.5 - 0.03125 *
///   255 / 33 = 127.25852.
/// - 65536 = 2^16, spacing 2^-7 above and 2^-8 below: D = 0.125 and 0.0625,
///   w > 63.75, width 65: 127.5 + 0.125 * 255 / 65 = 127.99038 and
///   127.5 - 0.0625 * 255 / 65 = 127.25481.
/// - 4194304 = 2^22, spacing 0.5 above and 0.25 below: D = 8 and 4,
///   w > 4080, width 4097: 127.5 + 8 * 255 / 4097 = 127.99793 and
///   127.5 - 4 * 255 / 4097 = 127.25104.
///
/// The review's tier case: stored 36000 four times, slope 3.1631906 (f32
/// 3.1631906032562256) and intercept 809.33826 (f32 809.3382568359375). The
/// CPU stage rounds the product 113874.86171722412 to 113874.859375, in
/// [65536, 131072) where the spacing is 2^-7, and the sum 114684.19763183594
/// to 114684.1953125 (0x47DFFE19). The tier A rescale, emulated with
/// `f32::mul_add` as pass 4 measured on Metal, rounds the exact
/// 114684.19997406 once, to 114684.203125 (0x47DFFE1A), one unit above. The
/// window is centred on the CPU value with D = 16 * 2^-7 = 0.125, so width
/// 65 as for 2^16. One unit draws 2^-7 * 255 / 65 = 0.030649, so the fused
/// value draws 127.53065, grey 128, and the fused value moved 16 units down
/// and up, 15 and 17 units from the centre, draw 127.5 - 0.45974 = 127.04026
/// and 127.5 + 0.52103 = 128.02103, grey 127 and 128. Before pass 6 the
/// window was width 1 and the fused value drew 129.49, grey 129.
#[test]
fn a_constant_from_2_15_up_draws_the_middle_grey_on_tier_a() {
    let identity = require_ok!(rescale(1.0, 0.0));
    for (value, width, low_out, high_out) in [
        (32768.0f32, 33.0f32, 127.25852f32, 127.98295f32),
        (65536.0, 65.0, 127.25481, 127.99038),
        (4194304.0, 4097.0, 127.25104, 127.99793),
    ] {
        let window = require_ok!(auto_window(&[Stored(value); 4], &identity, None));
        assert_eq!(window.basis, AutoWindowBasis::Constant);
        assert_eq!(window.width.to_bits(), width.to_bits(), "for {value}");
        let voi = require_ok!(window.transform(0.0, 255.0));
        let mut down = value;
        let mut up = value;
        for _ in 0..16 {
            down = down.next_down();
            up = up.next_up();
        }
        assert_display_exact(voi.apply(Modality(value)).0, 127.5);
        assert_close(voi.apply(Modality(down)).0, low_out);
        assert_grey(voi.apply(Modality(down)).0, 127.0);
        assert_close(voi.apply(Modality(up)).0, high_out);
        assert_grey(voi.apply(Modality(up)).0, 128.0);
    }

    let slope = 3.1631906f32;
    let intercept = 809.33826f32;
    let modality = require_ok!(rescale(slope, intercept));
    let window = require_ok!(auto_window(&[Stored(36000.0); 4], &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Constant);
    assert_eq!(window.center.to_bits(), 0x47DF_FE19);
    assert_eq!(window.width.to_bits(), 65.0f32.to_bits());
    let voi = require_ok!(window.transform(0.0, 255.0));
    let fused = 36000.0f32.mul_add(slope, intercept);
    assert_eq!(fused.to_bits(), 0x47DF_FE1A);
    let mut down = fused;
    let mut up = fused;
    for _ in 0..16 {
        down = down.next_down();
        up = up.next_up();
    }
    assert_close(voi.apply(Modality(fused)).0, 127.53065);
    assert_grey(voi.apply(Modality(fused)).0, 128.0);
    assert_close(voi.apply(Modality(down)).0, 127.04026);
    assert_grey(voi.apply(Modality(down)).0, 127.0);
    assert_close(voi.apply(Modality(up)).0, 128.02103);
    assert_grey(voi.apply(Modality(up)).0, 128.0);
}

/// D1, the review's example pair, which pass 2 found refusing 1,362,438
/// windows like it. PS3.3 C.11.2.1.3.2 and the lead's amendments of
/// 2026-11-06 after review passes 2 and 4.
///
/// The margin of review pass 4 moves each percentile 16 f32 units toward
/// the other and requires the moved values to draw exactly 0.0 and 255.0
/// too, so the window is searched on the moved pair. The inputs here are
/// therefore the pass 2 pair moved 16 units outward: stored 0x40DDEC4A =
/// 6.935094 and 0x4211274A = 36.288368, slope 1, n = 2, ranks 1 and 2. Both
/// are in one binade each, so 16 steps back are 16 bit steps back, and the
/// moved pair is 0x40DDEC5A = 6.9351015 and 0x4211273A = 36.288307, the pass
/// 2 pair, which the derivation below calls `low` and `high` and the 1st and
/// 99th percentiles, as pass 2 did. The window found on it has its
/// lower edge on 6.9351015 and its upper edge on 36.288307, so the 1st
/// percentile 6.935094 is below the lower edge and the 99th 36.288368 above
/// the upper edge, and both draw through C.11.2.1.3.2's edge branches,
/// exactly 0.0 and 255.0.
///
/// On the moved pair, low = 6.9351015 and high = 36.288307. The span rounds
/// to 29.353207 and the centre to 21.611706, so the upper edge lands on the
/// ramp at
/// t = (36.288307 - 21.611706) / 29.353207 + 0.5 = 0.99999994 and the 99th
/// percentile draws 254.99998, not the output maximum: the computed window
/// fails the check. Widening cannot help, because the real-arithmetic upper
/// edge t = span / width only falls as the width grows. The search's first
/// attempt narrows one bit step to width 29.353205 (0x41EAD35D), centre
/// 6.9351015 + 29.353205 / 2 = 21.611704 (0x41ACE4C5). The lower edge
/// 21.611704 - 14.6766025 rounds to the 1st percentile itself, 0x40DDEC5A,
/// so the 1st draws ymin through the first branch, exactly 0.0. The upper
/// edge 21.611704 + 14.6766025 rounds to the 99th percentile itself,
/// 0x4211273A, so the 99th is not above it and draws through the ramp, where
/// ((36.288307 - 21.611704) / 29.353205 + 0.5) * 255 rounds in f32 to
/// exactly 255.0.
#[test]
fn the_review_example_pair_is_accepted_after_one_narrowing_step() {
    let modality = require_ok!(rescale(1.0, 0.0));

    let frame = [
        Stored(f32::from_bits(0x40DD_EC4A)),
        Stored(f32::from_bits(0x4211_274A)),
    ];
    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Percentile);
    assert_eq!(window.counted, 2);
    assert_eq!(window.low.to_bits(), 0x40DD_EC4A);
    assert_eq!(window.high.to_bits(), 0x4211_274A);
    // The returned pair is the narrowed one, centre and width together.
    assert_eq!(window.center.to_bits(), 21.611704f32.to_bits());
    assert_eq!(window.width.to_bits(), 29.353205f32.to_bits());
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_display_exact(voi.apply(Modality(window.low)).0, 0.0);
    assert_display_exact(voi.apply(Modality(window.high)).0, 255.0);
    // The margin itself: each percentile moved 16 units toward the other
    // draws exactly the same output, 6.9351015 at the lower edge and
    // 36.288307 on the upper edge, where the ramp gives exactly 255.0.
    assert_display_exact(voi.apply(Modality(6.9351015)).0, 0.0);
    assert_display_exact(voi.apply(Modality(36.288307)).0, 255.0);
}

/// D1 and S1, a percentile pair whose check passes only on the second
/// narrowing step, pinning that the search runs past its first. PS3.3
/// C.11.2.1.3.2 and the lead's amendments of 2026-11-06 after review passes
/// 2 and 4.
///
/// The inputs are the pass 2 pair moved 16 f32 units outward, stored
/// 0x41CDC567 = 25.721388 and 0x420D3D23 = 35.309704, slope 1, n = 2, ranks
/// 1 and 2, each within one binade of its moved value. The margin moves them
/// back to 0x41CDC577 = 25.721418 and 0x420D3D13 = 35.309643, which the
/// derivation below calls `low` and `high` and the 1st and 99th percentiles.
/// The accepted window's lower edge is at or above 25.721418 and its upper
/// edge below 35.309643, so the inputs themselves are below and above the
/// edges and draw exactly 0.0 and 255.0 through the edge branches.
///
/// On the moved pair, low = 25.721418 and high = 35.309643. The computed
/// window's lower edge rounds
/// one ulp below the 1st percentile, 0x41CDC576, so the 1st percentile sits
/// on the ramp and draws a small ramp value, and the upper edge ties onto
/// the 99th percentile, both 0x420D3D13, so the 99th draws a ramp value just
/// above the output maximum: the check fails. The first attempt, a change
/// of one unit to width 9.588223, rounds the lower edge up onto the 1st
/// percentile while the upper edge still ties. The second, a change of two
/// units to width 9.5882225 with the centre 30.51553, drops the upper edge to
/// 0x420D3D12, one ulp below the 99th percentile. Both percentiles then map
/// through the edge branches to exactly 0.0 and 255.0. A budget of one
/// attempt would refuse this window, which is what the count is pinned
/// against.
#[test]
fn a_pair_is_accepted_on_the_second_narrowing_step() {
    let modality = require_ok!(rescale(1.0, 0.0));

    let frame = [
        Stored(f32::from_bits(0x41CD_C567)),
        Stored(f32::from_bits(0x420D_3D23)),
    ];
    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Percentile);
    assert_eq!(window.counted, 2);
    assert_eq!(window.center.to_bits(), 30.51553f32.to_bits());
    assert_eq!(window.width.to_bits(), 9.5882225f32.to_bits());
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_display_exact(voi.apply(Modality(window.low)).0, 0.0);
    assert_display_exact(voi.apply(Modality(window.high)).0, 255.0);
    assert_display_exact(voi.apply(Modality(f32::from_bits(0x41CD_C577))).0, 0.0);
    assert_display_exact(voi.apply(Modality(f32::from_bits(0x420D_3D13))).0, 255.0);
}

/// D1 of review pass 3, a percentile pair straddling 64 that one bit step at
/// a time reaches only at step 65, accepted on the geometric search's eighth
/// attempt. PS3.3 C.11.2.1.3.2 and the lead's amendments of 2026-11-06 after
/// review passes 3 and 4.
///
/// The inputs are the pass 3 pair moved 16 f32 units outward, stored
/// 0x427EBFBF = 63.687252 and 0x4281916E = 64.78404, slope 1, n = 2, ranks 1
/// and 2. 64 lies between them, but each input is in the binade of its own
/// moved value, so the margin's 16 steps are 16 bit steps and give back
/// 63.687313 (0x427EBFCF) and 64.78392 (0x4281915E), which the derivation
/// below calls `low` and `high` and the 1st and 99th percentiles. The
/// accepted window's lower edge 0x427EBFD0 is above 0x427EBFBF and its upper
/// edge 0x4281915C below 0x4281916E, so the inputs draw exactly 0.0 and
/// 255.0 through the edge branches.
///
/// On the moved pair, low = 63.687313 and high = 64.78392. The span is
/// 1.0966072 (0x3F8C5DA0), in [1, 2), so one unit below it is 2^-23 and
/// attempt `k` tries the width 0x3F8C5DA0 - 2^k bit steps. Each window's
/// centre is low + width / 2, and its edges are centre -/+ width / 2.
///
/// - The computed window: centre 0x428078A3, lower edge 0x427EBFD0 at or
///   above the 1st percentile, which draws ymin, 0.0. The upper edge is the
///   99th percentile bit for bit, so the 99th is on the ramp and draws
///   254.99956. Fails.
/// - Attempts 0 to 4, changes of 1 to 16 units: the centre and the upper
///   edge do not move, and the 99th draws 254.99957 to 254.99979. Fail.
/// - Attempts 5 and 6, widths 0x3F8C5D80 and 0x3F8C5D60: the centre drops to
///   0x428078A2 and the upper edge to 0x4281915D, below the 99th, which
///   draws ymax. But the lower edge rounds to 0x427EBFCE, one ulp below the
///   1st percentile, which is then on the ramp and draws 0.00088915 and
///   0.00044078. Fail, on the low side only.
/// - Attempt 7, a change of 128 units, width 0x3F8C5D20 = 1.096592: centre
///   0x428078A2 = 64.23561, lower edge 0x427EBFD0 at or above the 1st, upper
///   edge 0x4281915C below the 99th. Both percentiles draw through the edge
///   branches, exactly 0.0 and exactly 255.0. Accepted.
///
/// One bit step at a time over 48 attempts never reaches 128 units, a check
/// without its low side accepts attempt 5, and one without its high side
/// accepts the computed window, so each of those turns this red.
#[test]
fn a_pair_straddling_64_is_accepted_on_the_eighth_attempt() {
    let modality = require_ok!(rescale(1.0, 0.0));

    let frame = [
        Stored(f32::from_bits(0x427E_BFBF)),
        Stored(f32::from_bits(0x4281_916E)),
    ];
    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Percentile);
    assert_eq!(window.low.to_bits(), 0x427E_BFBF);
    assert_eq!(window.high.to_bits(), 0x4281_916E);
    assert_eq!(window.center.to_bits(), 0x4280_78A2);
    assert_eq!(window.width.to_bits(), 0x3F8C_5D20);
    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_display_exact(voi.apply(Modality(window.low)).0, 0.0);
    assert_display_exact(voi.apply(Modality(window.high)).0, 255.0);
    assert_display_exact(voi.apply(Modality(f32::from_bits(0x427E_BFCF))).0, 0.0);
    assert_display_exact(voi.apply(Modality(f32::from_bits(0x4281_915E))).0, 255.0);
}

/// The margin's moves capped at the midpoint, per the lead's amendments of
/// 2026-11-06 after review pass 4. PS3.3 C.11.2.1.3.2.
///
/// The margin moves each percentile 16 f32 units toward the other. When the
/// two are fewer than 33 units apart, 16 moves each would make the moved
/// values meet or cross, and a low-side value at or above a high-side one
/// cannot draw 0.0 while the other draws 255.0 under a non-decreasing ramp.
/// So each move stops at the last f32 strictly on its own side of the
/// midpoint: a pair `d` units apart moves `min(16, (d - 1) / 2)` units each,
/// in integer division.
///
/// 10.0 = 1.25 * 2^3 is in [8, 16), where the f32 spacing is 2^-20.
///
/// - Stored 10.0 (0x41200000) and 0x4120000A, 10 units apart, slope 1:
///   n = 2, ranks 1 and 2. The midpoint is 10 + 5 * 2^-20, so each moves
///   (10 - 1) / 2 = 4 units, to 0x41200004 and 0x41200006. The width is
///   6 * 2^-20 - 4 * 2^-20 = 2^-19 (0x36000000), exact, and the centre
///   0x41200004 + 2^-20 = 0x41200005, exact. The half width 2^-20 is exact,
///   so the lower edge is 0x41200004 and the upper edge 0x41200006. 10.0 is
///   below the lower edge and 0x41200004 on it, both ymin, exactly 0.0.
///   0x41200006 is the upper edge, not above it, so it is on the ramp at
///   ((2^-20) / 2^-19 + 0.5) * 255 = (0.5 + 0.5) * 255 = 255.0 exactly, and
///   0x4120000A is above it, ymax, 255.0. Accepted as computed. Uncapped,
///   the low side would move to 0x41200010, above the 99th percentile, and
///   the high side to 0x411FFFFA, below the 1st, and the window is refused.
/// - Stored 10.0 and 0x41200020, 32 units apart. The midpoint 0x41200010 is
///   an f32, which 16 moves each would both reach. Capped, each moves
///   (32 - 1) / 2 = 15 units, to 0x4120000F and 0x41200011, the same shape:
///   width 2^-19, centre 0x41200010, edges 0x4120000F and 0x41200011, and
///   the four values draw exactly 0.0, 0.0, 255.0 and 255.0.
#[test]
fn the_margin_moves_are_capped_at_the_midpoint() {
    let modality = require_ok!(rescale(1.0, 0.0));

    for (high, center, moved_low, moved_high) in [
        (
            0x4120_000A_u32,
            0x4120_0005_u32,
            0x4120_0004_u32,
            0x4120_0006_u32,
        ),
        (0x4120_0020, 0x4120_0010, 0x4120_000F, 0x4120_0011),
    ] {
        let frame = [Stored(10.0), Stored(f32::from_bits(high))];
        let window = require_ok!(auto_window(&frame, &modality, None));
        assert_eq!(window.basis, AutoWindowBasis::Percentile);
        assert_eq!(window.low.to_bits(), 10.0f32.to_bits());
        assert_eq!(window.high.to_bits(), high);
        assert_eq!(window.center.to_bits(), center, "for {high:#x}");
        assert_eq!(window.width.to_bits(), 0x3600_0000, "for {high:#x}");
        let voi = require_ok!(window.transform(0.0, 255.0));
        assert_display_exact(voi.apply(Modality(window.low)).0, 0.0);
        assert_display_exact(voi.apply(Modality(f32::from_bits(moved_low))).0, 0.0);
        assert_display_exact(voi.apply(Modality(f32::from_bits(moved_high))).0, 255.0);
        assert_display_exact(voi.apply(Modality(window.high)).0, 255.0);
    }
}

/// The refusal path, per the lead's amendments of 2026-11-06 after review
/// passes 3, 4 and 6. PS3.3 C.11.2.1.3.2.
///
/// Four samples of one stored value `v`, slope 1: counted 4, ranks 1 and 4,
/// low = high = v, a Constant, centre v, width 1. Since pass 6 the check asks
/// that v draw exactly 127.5 and that v moved 16 f32 units down and up draw
/// within half a grey of it. The widening search tries 1 + 2^(k - 23) at
/// attempt `k`, from 0 to 47, and the last, 1 + 2^24 = 16777217, ties between
/// 2^24 (even) and 16777218 and rounds to 2^24, the widest width. A point D
/// from the centre draws 255 * D / w from the middle on the ramp, so at most
/// half a grey needs D < 2^24 / 510 = 32896.5 at the widest, and every
/// narrower width needs less. The next widest, 1 + 2^23 = 8388609, allows
/// D < 16448.
///
/// - 0x50FFFFF0 = 34359705600 = 2^35 - 16 * 2048, in [2^34, 2^35) where the
///   spacing is 2048. 16 units either way span D = 32768, which fails at
///   8388609 and passes only at 2^24. The edges v -/+ 2^23 are exact, v - 2^23
///   in v's binade and v + 2^23 = 2^35 + 2040 * 4096 in the next. The value
///   draws 127.5, and v + 32768 = 2^35 draws (2^-9 + 0.5) * 255 = 65535 / 512
///   = 127.998046875 and v - 32768 draws 65025 / 512 = 127.001953125, exact
///   in [64, 128), spacing 2^-17. Accepted at width 2^24. Its negative is the
///   mirror image, the 16 units down reaching -2^35, the same width and the
///   same two values.
/// - 0x50FFFFF1 = 2^35 - 15 * 2048. 15 units up reach 2^35 and the 16th is a
///   unit of 4096 above it, D = 30720 + 4096 = 34816, at least 32896.5, so
///   v + 16 units draws 127.5 + 255 * 34816 / 2^24 = 128.02917 at the widest
///   width. Refused, and its negative is refused the same way.
/// - 0x57000001 = 2^47 + 2^24, 1e30, -1e30, -2^47 and 0x57800001 = 2^48 +
///   2^25. Their spacing is at least 2^24, so 16 units are at least 2^28,
///   far beyond 32896.5. Refused. Before pass 6 the first was accepted at
///   width 2^24 on its centre alone.
///
/// A search that returned its last tried window instead of refusing would
/// answer each with centre v and width 2^24, which for 0x50FFFFF1 draws the
/// value moved 16 units up at grey 128 only by 0.029 and for 1e30 draws it
/// white.
#[test]
fn a_constant_beyond_the_search_reach_is_refused() {
    let modality = require_ok!(rescale(1.0, 0.0));

    for value in [
        f32::from_bits(0x50FF_FFF1),
        f32::from_bits(0xD0FF_FFF1),
        f32::from_bits(0x5700_0001),
        1e30f32,
        -1e30,
        f32::from_bits(0xD700_0000),
        f32::from_bits(0x5780_0001),
    ] {
        assert_eq!(
            auto_window(&[Stored(value); 4], &modality, None),
            Err(AutoWindowError::UnverifiableWindow),
            "for {value}"
        );
    }

    for bits in [0x50FF_FFF0_u32, 0xD0FF_FFF0] {
        let value = f32::from_bits(bits);
        let window = require_ok!(auto_window(&[Stored(value); 4], &modality, None));
        assert_eq!(window.basis, AutoWindowBasis::Constant);
        assert_eq!(window.center.to_bits(), bits);
        assert_eq!(window.width.to_bits(), 0x4B80_0000, "for {bits:#x}");
        let voi = require_ok!(window.transform(0.0, 255.0));
        let mut down = value;
        let mut up = value;
        for _ in 0..16 {
            down = down.next_down();
            up = up.next_up();
        }
        assert_display_exact(voi.apply(Modality(down)).0, 65025.0 / 512.0);
        assert_display_exact(voi.apply(Modality(value)).0, 127.5);
        assert_display_exact(voi.apply(Modality(up)).0, 65535.0 / 512.0);
    }
}

/// The measured reach of a close pair, per the lead's amendment of
/// 2026-11-06 after review pass 6. PS3.3 C.11.2.1.3.2.
///
/// Two stored values `d` f32 units apart, slope 1, n = 2, ranks 1 and 2. The
/// Constant window is centred on their f32 midpoint, and the check asks the
/// centre to draw exactly 127.5 and each percentile, and each moved 16 units
/// away from the other, to draw within half a grey of it. As in the constant
/// refusal fixture, the widest width is 2^24 and allows D < 32896.5 from the
/// centre, and the next widest, 8388609, D < 16448. 2^34 = 0x50800000, the
/// spacing is 1024 below it and 2048 above it.
///
/// - d = 1, 0x507FFFFF and 2^34. The half span 512 ties between 0x507FFFFF
///   (odd) and 2^34 (even), so the centre is 2^34, the 99th. Its 16 units up
///   span D = 32768, and the 1st is 1024 below the centre and moved 16 units
///   down 17408 below. Width 2^24, edges 2^34 -/+ 2^23 exact. The displays are
///   127.5 - 255 * 17408 / 2^24 = 127.5 - 4335 / 16384 = 127.23541259765625,
///   127.5 - 255 / 16384 = 127.48443603515625, 127.5 at the 99th, and
///   127.998046875 for 2^34 + 32768. Accepted.
/// - d = 1, 2^34 and 2^34 + 2048. The tie now goes to 2^34 (even), the 1st,
///   so the 99th moved 16 units up is 17 units of 2048 above the centre,
///   D = 34816. Refused.
/// - d = 1, 0x50800001 = 2^34 + 2048 and 0x50800002. The 1st is odd, so the
///   centre is the 99th, 2^34 + 4096. The 99th moved up spans 32768. The 1st
///   moved 16 units down takes one unit of 2048 to 2^34 and 15 of 1024 below
///   it, D = 4096 + 15360 = 19456. Width 2^24. The 1st draws 127.5 - 255 *
///   2048 / 2^24 = 127.4688720703125 and moved down 127.5 - 255 * 19 / 16384
///   = 127.20428466796875. Accepted, one of the seven exceptions the source
///   records.
/// - d = 1, 0x5080000F = 2^34 + 15 * 2048 and 0x50800010. The centre is the
///   99th, 2^34 + 32768, and the 1st moved down takes 15 units of 2048 to 2^34
///   and one of 1024, D = 32768 + 1024 = 33792. Refused, the first odd 1st
///   past the exceptions.
/// - d = 2, 0x507FFFFD and 0x507FFFFF. The span 2048 halves exactly onto the
///   centre 0x507FFFFE = 2^34 - 2048. The 99th moved up takes one unit of 1024
///   to 2^34 and 15 of 2048, D = 1024 + 1024 + 30720 = 32768, and the 1st
///   moved down D = 1024 + 16384 = 17408. Width 2^24. The displays are
///   127.23541259765625, 127.48443603515625, 127.5 at the centre,
///   127.5 + 255 / 16384 = 127.51556396484375 and 127.998046875. Accepted.
/// - d = 2, 0x507FFFFE and 2^34. The centre is 0x507FFFFF, and the 99th
///   moved 16 units up is 2^34 + 32768, D = 33792. Refused.
///
/// Each frame mirrored to negative values, the pair negated and swapped,
/// behaves the same, the accepted displays mirrored to 255 minus each.
#[test]
fn a_close_pair_beyond_the_search_reach_is_refused() {
    let modality = require_ok!(rescale(1.0, 0.0));

    for (low, high) in [
        (0x5080_0000_u32, 0x5080_0001_u32),
        (0x5080_000F, 0x5080_0010),
        (0x507F_FFFE, 0x5080_0000),
    ] {
        for sign in [1.0f32, -1.0] {
            let (a, b) = (f32::from_bits(low) * sign, f32::from_bits(high) * sign);
            assert_eq!(
                auto_window(&[Stored(a), Stored(b)], &modality, None),
                Err(AutoWindowError::UnverifiableWindow),
                "for {a} and {b}"
            );
        }
    }

    // The 1st and 99th, then each moved 16 units away from the other, their
    // expected displays, and the expected centre.
    for (low, high, center, displays) in [
        (
            0x507F_FFFF_u32,
            0x5080_0000_u32,
            0x5080_0000_u32,
            [
                2088705.0f32 / 16384.0,
                127.5,
                2084625.0 / 16384.0,
                65535.0 / 512.0,
            ],
        ),
        (
            0x5080_0001,
            0x5080_0002,
            0x5080_0002,
            [
                1044225.0 / 8192.0,
                127.5,
                2084115.0 / 16384.0,
                65535.0 / 512.0,
            ],
        ),
        (
            0x507F_FFFD,
            0x507F_FFFF,
            0x507F_FFFE,
            [
                2088705.0 / 16384.0,
                2089215.0 / 16384.0,
                2084625.0 / 16384.0,
                65535.0 / 512.0,
            ],
        ),
    ] {
        for negative in [false, true] {
            let (lo, hi) = if negative {
                (-f32::from_bits(high), -f32::from_bits(low))
            } else {
                (f32::from_bits(low), f32::from_bits(high))
            };
            let window = require_ok!(auto_window(&[Stored(lo), Stored(hi)], &modality, None));
            assert_eq!(window.basis, AutoWindowBasis::Constant);
            let want_center = if negative {
                -f32::from_bits(center)
            } else {
                f32::from_bits(center)
            };
            assert_eq!(window.center.to_bits(), want_center.to_bits(), "for {lo}");
            assert_eq!(window.width.to_bits(), 0x4B80_0000, "for {lo}");
            let voi = require_ok!(window.transform(0.0, 255.0));
            let mut lo_out = lo;
            let mut hi_out = hi;
            for _ in 0..16 {
                lo_out = lo_out.next_down();
                hi_out = hi_out.next_up();
            }
            let [at_low, at_high, at_low_out, at_high_out] = displays;
            // Mirrored, the 99th of the negative frame is the negated 1st,
            // and each display is 255 minus the positive one.
            let pairs = if negative {
                [
                    (hi, 255.0 - at_low),
                    (lo, 255.0 - at_high),
                    (hi_out, 255.0 - at_low_out),
                    (lo_out, 255.0 - at_high_out),
                ]
            } else {
                [
                    (lo, at_low),
                    (hi, at_high),
                    (lo_out, at_low_out),
                    (hi_out, at_high_out),
                ]
            };
            assert_display_exact(voi.apply(Modality(window.center)).0, 127.5);
            for (x, display) in pairs {
                assert_display_exact(voi.apply(Modality(x)).0, display);
                let grey = (display + 0.5).floor();
                assert!((127.0..=128.0).contains(&grey), "{display} for {x}");
            }
        }
    }
}

/// The seeds every distribution block below runs on. Written down before the
/// test was first run, per the lead's amendment of 2026-11-06 after review
/// pass 3, and never searched: a seed that refuses is a defect in the search,
/// not in the seed. The last four were written down on 2026-11-06 before the
/// first run after the lead's amendment after review pass 5, the same way.
const REFUSAL_SEEDS: [u64; 12] = [
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    0x5EED_0227,
    0xC0FF_EE11,
    0x1234_5678_9ABC,
    0xDEAD_BEEF_0005,
];

/// Pairs drawn per seed in each pair distribution.
const PAIRS_PER_SEED: usize = 25_000;

/// Slices in the PET-like series.
const PET_SLICES: usize = 2_000;

/// One refusal count, and the first refused inputs, for one distribution on
/// one seed.
struct RefusalCount {
    distribution: &'static str,
    seed: u64,
    refused: usize,
    first: Option<String>,
}

/// A stored integer drawn uniformly from 1 to 65535, exact in f32.
fn stored_integer(rng: &mut SplitMix64) -> f32 {
    f32::from(u16::try_from(rng.next_u64() % 65535 + 1).unwrap_or(0))
}

fn count_pairs(
    distribution: &'static str,
    seed: u64,
    mut draw: impl FnMut(&mut SplitMix64) -> (f32, f32, f32, f32),
) -> RefusalCount {
    let mut rng = SplitMix64::new(seed);
    let mut refused = 0;
    let mut first = None;
    for _ in 0..PAIRS_PER_SEED {
        let (a, b, slope, intercept) = draw(&mut rng);
        let Ok(modality) = rescale(slope, intercept) else {
            refused += 1;
            continue;
        };
        if let Err(error) = auto_window(&[Stored(a), Stored(b)], &modality, None) {
            refused += 1;
            if first.is_none() {
                first = Some(format!(
                    "stored {a:?} {b:?} slope {slope:?} intercept {intercept:?}: {error:?}"
                ));
            }
        }
    }
    RefusalCount {
        distribution,
        seed,
        refused,
        first,
    }
}

/// One PET-like slice: a 48 by 48 frame, stored 0 outside a disc of radius
/// 20 and skewed counts `floor(u^3 * 32767)` inside, so most of the frame is
/// background and the counted values cluster low with a long tail.
fn pet_slice(rng: &mut SplitMix64) -> Vec<Stored> {
    let mut frame = Vec::with_capacity(48 * 48);
    for r in -24i32..24 {
        for c in -24i32..24 {
            if r * r + c * c > 20 * 20 {
                frame.push(Stored(0.0));
            } else {
                let u = rng.next_f32_unit();
                frame.push(Stored((u * u * u * 32767.0).floor()));
            }
        }
    }
    frame
}

/// A per-image slope, log-uniform over [1e-4, 10).
fn pet_slope(rng: &mut SplitMix64) -> f32 {
    10f32.powf(-4.0 + 5.0 * rng.next_f32_unit())
}

/// The 2,000 slice series on one seed: every slice's own window through
/// `auto_window`, and the series window pooled over `series_sample`'s 16
/// slices, each through its own slope.
fn count_pet_series(seed: u64) -> RefusalCount {
    let mut rng = SplitMix64::new(seed);
    let sampled = series_sample(PET_SLICES).unwrap_or_default();
    let mut pool = AutoWindowPool::new(sampled.len()).ok();
    let mut refused = 0;
    let mut first = None;
    for slice in 0..PET_SLICES {
        let frame = pet_slice(&mut rng);
        let slope = pet_slope(&mut rng);
        let Ok(modality) = rescale(slope, 0.0) else {
            refused += 1;
            continue;
        };
        if let Err(error) = auto_window(&frame, &modality, None) {
            refused += 1;
            if first.is_none() {
                first = Some(format!("slice {slice} slope {slope:?}: {error:?}"));
            }
        }
        if sampled.contains(&slice)
            && let Some(pool) = pool.as_mut()
            && pool.add_frame(&frame, &modality, None).is_err()
        {
            refused += 1;
        }
    }
    if let Err(error) = pool
        .ok_or(AutoWindowError::FrameCount)
        .and_then(AutoWindowPool::finish)
    {
        refused += 1;
        if first.is_none() {
            first = Some(format!("the pooled series window: {error:?}"));
        }
    }
    RefusalCount {
        distribution: "PET-like series",
        seed,
        refused,
        first,
    }
}

/// D1 of review pass 3, the check over the review's four distributions, per
/// the lead's amendments of 2026-11-06: refusal is measured on fixed seeds,
/// and none of these distributions may refuse.
///
/// Each distribution runs on every seed in [`REFUSAL_SEEDS`], and the test
/// records the refusal count of each with its first refused inputs before
/// asserting that every count is zero. The pair blocks draw 25,000 two
/// sample frames per seed. The PET-like block is 2,000 slices per seed, each
/// with its own content and slope, plus the pooled series window.
///
/// Uniform fractional [0, 100), slope 1: two f32 values in [0, 100).
///
/// Stored integer times a slope in (0.001, 10): two stored integers in
/// 1 to 65535 through one random slope, intercept 0.
///
/// CT-like float rescales: two stored integers through a slope in [0.5, 2)
/// with intercept -1024, so the counted values straddle air and water like
/// an unsigned CT.
///
/// A 2,000 slice PET-like series with a different slope per image, drawn by
/// `pet_slice` and `pet_slope`.
///
/// Measured on 2026-11-06. Before the geometric search, the one bit step
/// search of checkpoint e952721 refused, on seeds 1 to 8 in order:
///
/// - uniform fractional: 1, 1, 1, 1, 0, 3, 1, 1, total 9.
/// - integer times slope: 1, 1, 1, 5, 1, 0, 1, 2, total 12.
/// - CT-like: 1, 0, 0, 1, 1, 0, 0, 1, total 4.
/// - PET-like series: 0, 0, 0, 0, 0, 0, 0, 0, total 0.
///
/// With the geometric search every count is zero. The before run's first
/// refusals on seed 1 were the uniform pair 63.38857 and 64.07005, and
/// stored 46549 and 46377 at slope 1.411621. A refusal prints its seed and
/// first inputs in the failure message.
///
/// Measured again on 2026-11-06 with the 16 unit margin of the lead's
/// amendment after review pass 4, on the same eight seeds: every count of
/// every distribution was zero before the margin and is zero after it.
///
/// Measured again on 2026-11-06 after the lead's amendment after review pass
/// 5, which sends pairs fewer than three units apart to the Constant window,
/// on all twelve seeds: every count of every distribution is zero.
///
/// Measured again on 2026-11-06 after the lead's amendment after review pass
/// 6, which checks a constant at its value moved 16 units each way, on all
/// twelve seeds: every count of every distribution is zero.
#[test]
fn no_window_is_refused_on_the_review_distributions() {
    let mut counts = Vec::new();
    for &seed in &REFUSAL_SEEDS {
        counts.push(count_pairs("uniform fractional", seed, |rng| {
            (
                rng.next_f32_range(100.0),
                rng.next_f32_range(100.0),
                1.0,
                0.0,
            )
        }));
        counts.push(count_pairs("integer times slope", seed, |rng| {
            let a = stored_integer(rng);
            let b = stored_integer(rng);
            (a, b, 0.001 + rng.next_f32_unit() * 9.999, 0.0)
        }));
        counts.push(count_pairs("CT-like", seed, |rng| {
            let a = stored_integer(rng);
            let b = stored_integer(rng);
            (a, b, 0.5 + rng.next_f32_unit() * 1.5, -1024.0)
        }));
        counts.push(count_pet_series(seed));
    }
    let mut record = String::new();
    for count in &counts {
        record.push_str(&format!(
            "{} seed {}: {} refused{}\n",
            count.distribution,
            count.seed,
            count.refused,
            count
                .first
                .as_ref()
                .map(|inputs| format!(", first {inputs}"))
                .unwrap_or_default()
        ));
    }
    println!("{record}");
    assert!(
        counts.iter().all(|count| count.refused == 0),
        "refusals on the review distributions:\n{record}"
    );
}

/// D3. The Constant basis means the 1st and 99th nearest-rank percentiles are
/// equal, not that every counted value is. PS3.3 C.11.1 and C.11.2.1.3.2.
///
/// 200 samples of stored 7 and one of stored 1000, slope 1. Both are counted
/// (neither is stored zero), so n = 201. The ranks are 201.div_ceil(100) = 3
/// and (99 * 201).div_ceil(100) = 199, and the sorted set is 200 sevens then
/// 1000, so the 3rd and 199th values are both 7: low = high = 7, a Constant,
/// centre 7, width 1. The counted set is not constant, it holds 1000. Stored
/// 7 is the centre, 127.5, grey 128. Stored 1000 is above the upper edge
/// 7.5, grey 255.
#[test]
fn constant_basis_is_equal_percentiles_not_constant_values() {
    let mut frame = vec![stored(7); 200];
    frame.push(stored(1000));
    let modality = require_ok!(rescale(1.0, 0.0));

    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.counted, 201);
    assert_eq!(window.basis, AutoWindowBasis::Constant);
    assert_close(window.center, 7.0);
    assert_close(window.width, 1.0);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(7.0)).0, 128.0);
    assert_grey(voi.apply(Modality(1000.0)).0, 255.0);
}

/// D2, the constant window is checked like any other, per the lead's
/// amendments of 2026-11-06 after review passes 2, 3 and 6. PS3.3
/// C.11.2.1.3.2.
///
/// Four samples of one stored value `v`, slope 1: counted 4, ranks
/// 4.div_ceil(100) = 1 and (99 * 4).div_ceil(100) = 4, so low = high = v, a
/// Constant, centre v, width 1 widened by the Constant search. Since pass 6
/// the check asks that v draw exactly the middle 127.5 and that v moved 16
/// f32 units down and 16 units up draw within half a grey of it, above 127.0
/// and below 128.0.
///
/// The search tries the width 1 + 2^(k - 23) at attempt `k`, exact in f32
/// for every `k` used here. Whenever v is above the lower edge it is on the
/// ramp at ((v - v) / w + 0.5) * 255 = 127.5 exactly. A point D above or
/// below v draws 255 * D / w from the middle, so the side whose 16 units span
/// the larger distance D binds, and the check needs w > 510 * D. The 16 units
/// are the spacing above v on the high side and below v on the low side, and
/// a move that crosses a power of two takes the smaller spacing below it for
/// the units past it. Every width tried here has a half width far above the
/// spacing, so the lower edge is below v and the outward points are on the
/// ramp.
///
/// - 8388610 = 2^23 + 2, spacing 1. Up: D = 16. Down: two units reach 2^23
///   and the other 14 are 0.5 each, D = 9. w > 8160. `k` = 35 gives 4097,
///   where 16 * 255 / 4097 = 0.996 fails, and `k` = 36 gives 8193: the high
///   side draws 127.5 + 16 * 255 / 8193 = 127.5 + 0.49799 = 127.99799, the
///   low side 127.5 - 9 * 255 / 8193 = 127.21988. Width 8193.
/// - 16777216 = 2^24, spacing 2 above and 1 below. D = 32 up and 16 down,
///   w > 16320. `k` = 37 gives 16385: 127.5 + 32 * 255 / 16385 = 127.99802
///   and 127.5 - 16 * 255 / 16385 = 127.25099. Width 16385.
/// - 16777218 = 2^24 + 2, spacing 2. Up D = 32. Down one unit of 2 reaches
///   2^24 and 15 of 1 follow, D = 17. Width 16385, and the low side draws
///   127.5 - 17 * 255 / 16385 = 127.23543.
/// - 33554432 = 2^25, spacing 4 above and 2 below. D = 64 up, 32 down,
///   w > 32640. `k` = 38 gives 32769: 127.5 + 64 * 255 / 32769 = 127.99803
///   and 127.5 - 32 * 255 / 32769 = 127.25098. Width 32769.
/// - -16777216 = -2^24, the mirror of 2^24: spacing 2 below and 1 above,
///   D = 32 down and 16 up. Width 16385, the low side 127.5 - 0.49802 =
///   127.00198 and the high side 127.5 + 0.24901 = 127.74901.
/// - 1e9 = 15625000 * 64, exact in f32, in [2^29, 2^30) where the spacing is
///   64 and both moves stay inside the binade, D = 1024 each way. w > 522240.
///   `k` = 41 gives 262145, which fails, and `k` = 42 gives 524289:
///   127.5 -/+ 1024 * 255 / 524289 = 127.5 -/+ 0.49805, so 127.00195 and
///   127.99805. Width 524289.
///
/// Each value draws exactly 127.5, grey 128, and each outward point grey 127
/// or 128. Before pass 6 the check was the centre's alone, and these widths
/// were 1.0000001, 1.0000001, 2, 3, 3 and 65.
#[test]
fn a_constant_frame_draws_the_middle_grey_at_large_magnitudes() {
    let modality = require_ok!(rescale(1.0, 0.0));

    for (value, width, low_out, high_out) in [
        (8388610.0f32, 8193.0f32, 127.21988f32, 127.99799f32),
        (16777216.0, 16385.0, 127.25099, 127.99802),
        (16777218.0, 16385.0, 127.23543, 127.99802),
        (33554432.0, 32769.0, 127.25098, 127.99803),
        (-16777216.0, 16385.0, 127.00198, 127.74901),
        (1e9, 524289.0, 127.00195, 127.99805),
    ] {
        let frame = [Stored(value); 4];
        let window = require_ok!(auto_window(&frame, &modality, None));
        assert_eq!(window.basis, AutoWindowBasis::Constant);
        assert_eq!(window.counted, 4);
        assert_eq!(window.center.to_bits(), value.to_bits());
        assert_eq!(window.width.to_bits(), width.to_bits(), "for {value}");
        let voi = require_ok!(window.transform(0.0, 255.0));
        assert_display_exact(voi.apply(Modality(value)).0, 127.5);
        assert_grey(voi.apply(Modality(value)).0, 128.0);
        let mut down = value;
        let mut up = value;
        for _ in 0..16 {
            down = down.next_down();
            up = up.next_up();
        }
        assert_close(voi.apply(Modality(down)).0, low_out);
        assert_grey(voi.apply(Modality(down)).0, 127.0);
        assert_close(voi.apply(Modality(up)).0, high_out);
        assert_grey(voi.apply(Modality(up)).0, 128.0);
    }
}

/// D4. `StoredRange`'s `PartialEq` is bitwise equality of the two bounds,
/// which for public f32 fields is not value equality: negative zero differs
/// from zero, and a NaN range compares equal to itself.
#[test]
fn stored_range_equality_is_bitwise() {
    let zero = StoredRange {
        low: Stored(0.0),
        high: Stored(0.0),
    };
    let negative_zero = StoredRange {
        low: Stored(-0.0),
        high: Stored(-0.0),
    };
    assert_ne!(zero, negative_zero);

    let nan = StoredRange {
        low: Stored(f32::NAN),
        high: Stored(0.0),
    };
    assert_eq!(nan, nan);
}

/// S1, M6. The padding bounds are honoured in either declared order, as
/// `ocelli_dicom::PaddingRange::contains` reads the file's value and limit.
/// PS3.3 C.7.5.1.1.2 and C.11.2.1.3.2.
///
/// Stored 1 to 10 with the range declared as 10 to 8, reversed. The effective
/// range is 8 to 10, so stored 8, 9 and 10 are left out and the counted set
/// is 1 to 7, n = 7, ranks 1 and 7, low = 1, high = 7, centre 4, width 6 to
/// within the pass 4 margin of 16 f32 units at each end, under 1e-5 here.
/// Stored 1 is below the lower edge 1 + 2^-19, grey 0, and stored 7 above the
/// upper edge, grey 255. Reading the bounds in declared order would exclude nothing and
/// give counted 10, high 10, centre 5.5, width 9.
#[test]
fn padding_bounds_are_honoured_in_either_declared_order() {
    let frame: Vec<Stored> = (1..=10).map(stored).collect();
    let modality = require_ok!(rescale(1.0, 0.0));
    let padding = StoredRange {
        low: stored(10),
        high: stored(8),
    };

    let window = require_ok!(auto_window(&frame, &modality, Some(padding)));
    assert_eq!(window.counted, 7);
    assert_close(window.low, 1.0);
    assert_close(window.high, 7.0);
    assert_close(window.center, 4.0);
    assert_close(window.width, 6.0);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(1.0)).0, 0.0);
    assert_grey(voi.apply(Modality(7.0)).0, 255.0);
}

/// S1, M13. A refused frame leaves the pool exactly as it was, including the
/// left-out maximum. PS3.3 C.11.1.
///
/// Frame A holds stored 0 and stored 2 through slope 3.0e38 with intercept
/// 3.0e38. Stored 0 is 3.0e38, finite, and left out as stored zero, staging a
/// left-out maximum of 3.0e38. Stored 2 is 6.0e38 + 3.0e38 = 9.0e38, beyond
/// f32's maximum, so the frame is refused and the staged maximum must be
/// discarded with it. Frame B is one stored 0 through slope 1, left out, so
/// the pool's largest left-out value is 0.0 and the window is the Background
/// centre 0.5, width 1. A pool that kept frame A's staged maximum would
/// answer with the centre 3.0e38 instead.
#[test]
fn a_refused_frame_leaves_the_left_out_maximum_unchanged() {
    let overflow = require_ok!(ModalityTransform::new(None, Some(3.0e38), Some(3.0e38)));
    let identity = require_ok!(rescale(1.0, 0.0));

    let mut pool = require_ok!(AutoWindowPool::new(2));
    assert_eq!(
        pool.add_frame(&[stored(0), stored(2)], &overflow, None),
        Err(AutoWindowError::NonFiniteModality)
    );
    require_ok!(pool.add_frame(&[stored(0)], &identity, None));

    let window = require_ok!(pool.finish());
    assert_eq!(window.basis, AutoWindowBasis::Background);
    assert_close(window.center, 0.5);
    assert_close(window.width, 1.0);
}

/// S1, M14. Stored negative zero is stored zero: the sign is masked when the
/// bits are tested. PS3.3 C.11.1 and C.11.2.1.3.2, and C.7.6.3.1 for the sign.
///
/// Four samples of stored -0.0, slope 1. Every sample is left out, so the
/// largest left-out modality value is -0.0 and the window is the Background
/// centre -0.0 + 0.5 = 0.5, width 1. The sample -0.0 is at or below the lower
/// edge 0.0 and maps to ymin, grey 0. Testing the bits without masking the
/// sign would count -0.0 and give a Constant window centred on -0.0, grey 128.
#[test]
fn stored_negative_zero_is_counted_as_stored_zero() {
    let frame = [Stored(-0.0); 4];
    let modality = require_ok!(rescale(1.0, 0.0));

    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(window.basis, AutoWindowBasis::Background);
    assert_close(window.center, 0.5);
    assert_close(window.width, 1.0);

    let voi = require_ok!(window.transform(0.0, 255.0));
    assert_grey(voi.apply(Modality(-0.0)).0, 0.0);
}

/// S1, M22. A sampled modality value that is not finite is refused even when
/// the sample is left out. PS3.3 C.11.1 and C.7.5.1.1.2.
///
/// One stored 5 through slope 3.0e38, left out by its own padding. The
/// modality value is 5 * 3.0e38 = 1.5e39, beyond f32's maximum, so the frame
/// is refused before the exclusion. A pool that only checked counted samples
/// would accept the frame and carry the infinite value into the left-out
/// maximum, which the refusal assertion catches. The pool stays usable: the
/// valid frame that follows is the whole window.
#[test]
fn a_left_out_sample_is_refused_when_not_finite() {
    let overflow = require_ok!(rescale(3.0e38, 0.0));
    let identity = require_ok!(rescale(1.0, 0.0));
    let padding = StoredRange {
        low: stored(5),
        high: stored(5),
    };

    let mut pool = require_ok!(AutoWindowPool::new(2));
    assert_eq!(
        pool.add_frame(&[stored(5)], &overflow, Some(padding)),
        Err(AutoWindowError::NonFiniteModality)
    );
    require_ok!(pool.add_frame(&[stored(7)], &identity, None));

    let window = require_ok!(pool.finish());
    assert_eq!(window.basis, AutoWindowBasis::Constant);
    assert_close(window.center, 7.0);
}

/// S1, M25. A computed window whose centre or width is not finite is refused.
/// PS3.3 C.11.1 and C.11.2.1.3.2.
///
/// Stored -3.0e38 and 3.0e38 through slope 1 are finite modality values, so
/// the frame is accepted, but the width 3.0e38 - (-3.0e38) = 6.0e38 is beyond
/// f32's maximum and the window is refused rather than returned.
#[test]
fn a_non_finite_window_is_refused() {
    let modality = require_ok!(rescale(1.0, 0.0));

    assert_eq!(
        auto_window(&[Stored(-3.0e38), Stored(3.0e38)], &modality, None),
        Err(AutoWindowError::NonFiniteWindow)
    );
}

/// AC9. Refusals, pool recovery and the transform the window builds.
///
/// A rescale slope of 3.0e38 on stored 2 gives 6.0e38, beyond f32's maximum,
/// so the modality value is not finite and the frame is refused
/// (PS3.3 C.11.1). The refusal leaves the pool as it was, so the following
/// valid frame and finish give that frame's window: one counted value 5.0,
/// both ranks 1, low = high = 5.0, a constant, centre 5.0, width 1.
///
/// AC1's window builds the same VOI stage as the constructor called directly
/// with AC1's centre 25 - 2^-15 (0x41C7FFF0) and width 49 - 2^-14
/// (0x4243FFF0), derived there under the pass 4 margin, LINEAR_EXACT and the
/// 0 to 255 range.
#[test]
fn auto_window_pool_refusals() {
    assert_eq!(
        AutoWindowPool::new(0).err(),
        Some(AutoWindowError::FrameCount)
    );
    assert_eq!(
        AutoWindowPool::new(17).err(),
        Some(AutoWindowError::FrameCount)
    );
    assert_eq!(series_sample(0), Err(AutoWindowError::EmptySeries));

    let overflow = require_ok!(ModalityTransform::new(None, Some(3.0e38), Some(0.0)));
    let identity = require_ok!(rescale(1.0, 0.0));

    let mut pool = require_ok!(AutoWindowPool::new(2));
    assert_eq!(
        pool.add_frame(&[stored(2)], &overflow, None),
        Err(AutoWindowError::NonFiniteModality)
    );
    assert_eq!(
        pool.add_frame(&[], &identity, None),
        Err(AutoWindowError::EmptyFrame)
    );
    let padding = StoredRange {
        low: Stored(f32::NAN),
        high: Stored(0.0),
    };
    assert_eq!(
        pool.add_frame(&[stored(1)], &identity, Some(padding)),
        Err(AutoWindowError::InvalidPadding)
    );

    let mut refused_twice = require_ok!(AutoWindowPool::new(2));
    require_ok!(refused_twice.add_frame(&[stored(1)], &identity, None));
    require_ok!(refused_twice.add_frame(&[stored(1)], &identity, None));
    assert_eq!(
        refused_twice.add_frame(&[stored(1)], &identity, None),
        Err(AutoWindowError::TooManyFrames)
    );

    let empty = require_ok!(AutoWindowPool::new(2));
    assert_eq!(empty.finish(), Err(AutoWindowError::EmptyPool));

    // A refused frame leaves the pool unchanged, so the valid frame that
    // follows is the whole window. The refused frame holds a finite sample
    // before its overflowing one, so a pool that kept refused values would
    // answer with a percentile of them instead.
    let mut pool = require_ok!(AutoWindowPool::new(2));
    assert_eq!(
        pool.add_frame(&[stored(1), stored(2)], &overflow, None),
        Err(AutoWindowError::NonFiniteModality)
    );
    require_ok!(pool.add_frame(&[stored(5)], &identity, None));
    let window = require_ok!(pool.finish());
    assert_eq!(window.frames, 1);
    assert_eq!(window.basis, AutoWindowBasis::Constant);
    assert_close(window.center, 5.0);
    assert_close(window.width, 1.0);

    // AC1's window and the direct constructor agree.
    let mut frame = vec![stored(0); 100];
    frame.extend((1..=99).map(stored));
    frame.push(stored(1000));
    let modality = require_ok!(rescale(0.5, 0.0));
    let window = require_ok!(auto_window(&frame, &modality, None));
    assert_eq!(
        window.transform(0.0, 255.0),
        VoiTransform::new(
            None,
            &[f32::from_bits(0x41C7_FFF0)],
            &[f32::from_bits(0x4243_FFF0)],
            0,
            VoiFunction::LinearExact,
            0.0,
            255.0
        )
    );
}
