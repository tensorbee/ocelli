//! ISO 14495-1 context model for JPEG-LS lossless and near-lossless coding,
//! shared by the scan decoder and the encoder.
//!
//! # Context Selection (ISO 14495-1 §A.3)
//! For each sample, three local gradients D1, D2, D3 are computed from the
//! causal neighborhood. Each gradient is quantized into one of 9 levels
//! [-4..4] using thresholds T1, T2, T3. The triplet is sign-normalized
//! (ensuring the first non-zero component is positive) and mapped to a
//! context index in [0, 365).
//!
//! # Context State (ISO 14495-1 §A.2.1 and §A.6)
//! Each context tracks:
//! - `a`: accumulated absolute error sum (initialized to `a_init = max(2, (RANGE+32)/64)`)
//! - `b`: accumulated signed error sum for bias tracking
//! - `c`: bias correction applied to predictor (clamped to [-128, 127])
//! - `n`: sample count (initialized to 1 per ISO 14495-1 §A.2.1 step 2, not a
//!   division-avoidance device, since A.5.1's k selection is a shift comparison)

/// ISO 14495-1 context state for one regular or run-interrupt context.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContextState {
    /// Accumulated absolute error magnitude (A in ISO 14495-1 §A.2.1 and §A.6.1).
    pub(crate) a: u32,
    /// Accumulated signed error sum (B, used for bias estimation).
    pub(crate) b: i32,
    /// Predictor bias correction value (C, clamped to [MIN_C, MAX_C]).
    pub(crate) c: i32,
    /// Sample count (N, initialized to 1 per ISO 14495-1 §A.2.1 step 2).
    pub(crate) n: u32,
}

impl Default for ContextState {
    /// A zeroed state with n = 1, used by tests. The T.87 initial state is
    /// `ContextModel::new`'s.
    fn default() -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            n: 1,
        }
    }
}

/// ISO 14495-1 §A.3.4 regular context count: (9×9×9 - 1) / 2 + 1 = 365
/// (sign-normalized triplets).
pub(crate) const CONTEXTS: usize = 365;

/// Context model reset threshold (ISO 14495-1 Table C.2: RESET = 64, applied in §A.6.1).
const RESET: u32 = 64;

/// Minimum bias correction value (ISO 14495-1 §3.3: MIN_C = −128, used in §A.6.2).
pub(super) const MIN_C: i32 = -128;

/// Maximum bias correction value (ISO 14495-1 §3.3: MAX_C = 127, used in §A.6.2).
pub(super) const MAX_C: i32 = 127;

/// ISO 14495-1 context model: 365 regular contexts + 2 run-interrupt contexts.
pub(crate) struct ContextModel {
    /// Regular-mode context states indexed by `context_index()`.
    pub(crate) regular: [ContextState; CONTEXTS],
    /// Run-interrupt context for `RItype = 0` (§A.7.2).
    pub(crate) run_int_diff: RunInterruptionContext,
    /// Run-interrupt context for `RItype = 1` (§A.7.2).
    pub(crate) run_int_same: RunInterruptionContext,
    /// Run-mode Golomb J-table index per component (§A.2.1 step 3 and §A.7.1, RUNindex).
    pub(crate) run_index: usize,
}

impl ContextModel {
    /// Initialize all context states.
    ///
    /// `a_init = max(2, (RANGE + 32) / 64)` where RANGE is A.2.1 step 1's,
    /// `MAXVAL + 1` for lossless and
    /// `floor((MAXVAL + 2*NEAR) / (2*NEAR + 1)) + 1` for NEAR > 0.
    /// From ISO 14495-1 §A.2.1: initial `A[q]` = a_init for all q.
    pub(crate) fn new(a_init: u32) -> Self {
        let s = ContextState {
            a: a_init,
            b: 0,
            c: 0,
            n: 1,
        };
        Self {
            regular: [s; CONTEXTS],
            run_int_diff: RunInterruptionContext::new(0, a_init),
            run_int_same: RunInterruptionContext::new(1, a_init),
            run_index: 0,
        }
    }
}

/// ISO 14495-1 run-interruption context state for indices 365 and 366.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunInterruptionContext {
    run_interruption_type: u32,
    a: u32,
    n: u32,
    nn: u32,
}

impl RunInterruptionContext {
    pub(crate) const fn new(run_interruption_type: u32, a_init: u32) -> Self {
        Self {
            run_interruption_type,
            a: a_init,
            n: 1,
            nn: 0,
        }
    }

    #[inline(always)]
    pub(crate) const fn run_interruption_type(self) -> u32 {
        self.run_interruption_type
    }

    /// Golomb parameter k for a run-interruption context (T.87 A.7.2 item 5,
    /// with TEMP from item 4, Code segment A.20), the smallest k with
    /// `n_test >= TEMP` where the compared value is
    /// `TEMP = A + (N >> 1) * run_interruption_type`, with the type supplied
    /// by `new`: 0 for the different-neighbours context (`run_int_diff`) and
    /// 1 for the same-neighbours context (`run_int_same`). Bounded by `qbpp`.
    ///
    /// Caller premise, decoder: the decoded error is held to the coded-error
    /// range by `in_coded_range` (`scan.rs`) **before** the run-interruption
    /// `update`, so the state stays within the A.2.1 steps 2 and 4
    /// initialization bounds and the A.7.2 Code segment A.23 reset bound.
    /// Caller premise, encoder: no refusal. `modulo_reduce` is applied
    /// before the update instead (T.87 A.4.5). Shared premises with the
    /// regular contexts: initial `A = max(2, floor((RANGE + 32)/64))` and
    /// `N = 1` from A.2.1 step 2 via `CodingParams::new`, `RANGE` and
    /// `qbpp` from the same constructor, and `RESET = 64`.
    #[inline(always)]
    pub(crate) fn compute_k(self, qbpp: u32) -> u32 {
        let temp = self.a + (self.n >> 1) * self.run_interruption_type;
        let mut n_test = self.n;
        let mut k = 0;
        while n_test < temp && k < qbpp {
            n_test <<= 1;
            k += 1;
        }
        k
    }

    /// Sign-mapping condition used by both the decoder's `compute_error_value`
    /// and the encoder's forward mapping (ISO 14495-1 §A.7.2): when `true`,
    /// a negative error value maps to an odd `temp`.
    #[inline(always)]
    pub(crate) fn negative_maps_to_odd(self, k: u32) -> bool {
        k != 0 || 2 * self.nn >= self.n
    }

    #[inline(always)]
    pub(crate) fn compute_error_value(self, temp: u32, k: u32) -> i32 {
        let map = temp & 1 != 0;
        let error_value_abs = ((temp + u32::from(map)) / 2) as i32;
        if self.negative_maps_to_odd(k) == map {
            -error_value_abs
        } else {
            error_value_abs
        }
    }

    #[inline(always)]
    pub(crate) fn update(&mut self, error_value: i32, mapped_error_value: u32) {
        if error_value < 0 {
            self.nn += 1;
        }
        self.a += (mapped_error_value + 1 - self.run_interruption_type) >> 1;
        if self.n == RESET {
            self.a >>= 1;
            self.n >>= 1;
            self.nn >>= 1;
        }
        self.n += 1;
    }
}

/// Update a regular-mode context state after decoding one sample.
///
/// Updates A, B, N with optional renormalization (RESET), and adjusts C.
/// ISO 14495-1 §A.6.
#[inline(always)]
pub(crate) fn update_context(ctx: &mut ContextState, errval: i32, near: u32) {
    ctx.a += errval.unsigned_abs();
    ctx.b += errval * (2 * near as i32 + 1);
    if ctx.n == RESET {
        ctx.a >>= 1;
        // Ocelli F-243: an arithmetic shift, which floors, as CharLS 2.4.4
        // does. `/ 2` truncates toward zero and leaves an odd negative B one
        // too high, which moves the bias correction and desynchronises the
        // scan from the encoder's at the context RESET.
        ctx.b >>= 1;
        ctx.n >>= 1;
    }
    ctx.n += 1;

    // Bias correction update
    if ctx.b + ctx.n as i32 <= 0 {
        ctx.b += ctx.n as i32;
        if ctx.b <= -(ctx.n as i32) {
            ctx.b = -(ctx.n as i32) + 1;
        }
        if ctx.c > MIN_C {
            ctx.c -= 1;
        }
    } else if ctx.b > 0 {
        ctx.b -= ctx.n as i32;
        if ctx.b > 0 {
            ctx.b = 0;
        }
        if ctx.c < MAX_C {
            ctx.c += 1;
        }
    }
}

#[inline(always)]
pub(crate) fn error_correction(ctx: &ContextState, k_or_near: u32) -> i32 {
    if k_or_near != 0 {
        0
    } else if 2 * ctx.b + ctx.n as i32 - 1 < 0 {
        -1
    } else {
        0
    }
}

/// Golomb parameter k, the smallest k with `N * 2^k >= A` (T.87 A.5.1 Code
/// segment A.10), bounded here by qbpp.
///
/// Caller premise, decoder: the decoder computes k before any update, and
/// `k < qbpp` never binds because the A and N state stays within the
/// initialization and reset bounds of A.2.1 and A.6.1, which rests on
/// `in_coded_range` (`scan.rs`) refusing a decoded
/// error outside `[-floor(RANGE/2), ceil(RANGE/2) - 1]` **before** the A and
/// N updates. F-243 working pass 8 measured at its tip that with that refusal
/// disabled the cap binds about two million times on damaged streams. The
/// refusal is the decoder's, measured by pass 8's damaged-stream decoder
/// sweep and not re-proven here. Caller premise, encoder: the encoder has no
/// refusal. Its own premise is `modulo_reduce`, applied before the state
/// updates, so every error entering the update is already in the canonical
/// range by T.87 A.4.5. `in_coded_range` is not shared with the encoder.
///
/// Shared premises, both callers: initial `A = max(2, floor((RANGE + 32)/64))`
/// and `N = 1` from T.87 A.2.1 step 2 via `CodingParams::new`, `RANGE` and
/// `qbpp = ceil(log2 RANGE)` from the same constructor, and `RESET = 64`.
#[inline(always)]
pub(crate) fn compute_k(a: u32, n: u32, qbpp: u32) -> u32 {
    let mut k = 0u32;
    // Smallest k such that N << k >= A
    while (n << k) < a && k < qbpp {
        k += 1;
    }
    k
}

/// Quantize gradient `d` into one of 9 levels {−4, −3, −2, −1, 0, 1, 2, 3, 4}
/// using thresholds T1, T2, T3 (where NEAR < T1 ≤ T2 ≤ T3) and the NEAR
/// dead-zone (`|d| ≤ NEAR → 0`; NEAR = 0 for lossless).
///
/// ISO 14495-1 §A.3.3 (code segment A.4).
#[inline(always)]
pub(crate) fn quant(d: i32, t1: i32, t2: i32, t3: i32, near: i32) -> i32 {
    if d <= -t3 {
        -4
    } else if d <= -t2 {
        -3
    } else if d <= -t1 {
        -2
    } else if d < -near {
        -1
    } else if d <= near {
        0
    } else if d < t1 {
        1
    } else if d < t2 {
        2
    } else if d < t3 {
        3
    } else {
        4
    }
}

/// Sign-normalize triplet (Q1, Q2, Q3) so the first non-zero component is positive.
///
/// Returns (Q1', Q2', Q3', sign) where sign ∈ {1, −1}.
/// ISO 14495-1 §A.3.4.
#[inline(always)]
pub(crate) fn sign_normalize(q1: i32, q2: i32, q3: i32) -> (i32, i32, i32, i32) {
    if q1 < 0 || (q1 == 0 && q2 < 0) || (q1 == 0 && q2 == 0 && q3 < 0) {
        (-q1, -q2, -q3, -1)
    } else {
        (q1, q2, q3, 1)
    }
}

/// Compute context index from sign-normalized (Q1, Q2, Q3) ∈ [0, 365).
///
/// Partition:
/// - Q1=0, Q2=0: indices [0, 5) (Q3 ∈ {0..4})
/// - Q1=0, Q2>0: indices [5, 41) (Q2 ∈ {1..4}, Q3 ∈ {−4..4}: 4×9=36)
/// - Q1>0: indices [41, 365) (Q1 ∈ {1..4}, Q2 ∈ {−4..4}, Q3 ∈ {−4..4}: 4×81=324)
///
/// ISO 14495-1 §A.3.4.
#[inline(always)]
pub(crate) fn context_index(q1: i32, q2: i32, q3: i32) -> usize {
    if q1 == 0 && q2 == 0 {
        // Q3 ∈ [0..4]: 5 contexts
        q3 as usize
    } else if q1 == 0 {
        // Q2 ∈ [1..4], Q3 ∈ [-4..4]: 4×9 = 36 contexts starting at 5
        5 + (q2 - 1) as usize * 9 + (q3 + 4) as usize
    } else {
        // Q1 ∈ [1..4], Q2 ∈ [-4..4], Q3 ∈ [-4..4]: 4×81 = 324 starting at 41
        41 + (q1 - 1) as usize * 81 + (q2 + 4) as usize * 9 + (q3 + 4) as usize
    }
}

/// Compute default gradient thresholds T1, T2, T3 from MAXVAL and NEAR.
///
/// ISO 14495-1 §C.2.4.1.1.1 default preset parameters (no LSE marker),
/// Table C.3 `BASIC = (BASIC_T1, BASIC_T2, BASIC_T3) = (3, 7, 21)`, and the
/// clamping function of Figure C.3 over integers, `CLAMP(i, j) = j` when
/// `i > MAXVAL` or `i < j`, otherwise `i`, with MAXVAL ambient.
/// For `MAXVAL >= 128`, Figure C.4:
/// `FACTOR = floor((min(MAXVAL, 4095) + 128) / 256)`,
/// `T1 = CLAMP(FACTOR·(BASIC_T1 - 2) + 2 + 3·NEAR, NEAR + 1)`,
/// `T2 = CLAMP(FACTOR·(BASIC_T2 - 3) + 3 + 5·NEAR, T1)`,
/// `T3 = CLAMP(FACTOR·(BASIC_T3 - 4) + 4 + 7·NEAR, T2)`.
/// For `MAXVAL < 128`, Figure C.5: `FACTOR = floor(256 / (MAXVAL + 1))`,
/// `T1 = CLAMP(max(2, floor(BASIC_T1 / FACTOR) + 3·NEAR), NEAR + 1)`,
/// `T2 = CLAMP(max(3, floor(BASIC_T2 / FACTOR) + 5·NEAR), T1)`,
/// `T3 = CLAMP(max(4, floor(BASIC_T3 / FACTOR) + 7·NEAR), T2)`.
/// At MAXVAL 127, NEAR 0 the low branch gives FACTOR 2 and thresholds
/// (2, 3, 10). The decoder accepts P from 2, and PS3.5 8.2.3 permits Bits
/// Stored below 8 in an 8 or 16-bit container, so a `MAXVAL < 128` frame can
/// reach this branch and the low arm is not dead code.
pub(crate) fn default_thresholds(maxval: i32, near: i32) -> (i32, i32, i32) {
    if maxval >= 128 {
        let factor = (maxval.min(4095) + 128) / 256;
        let t1 = clamp_threshold(factor + 2 + 3 * near, near + 1, maxval);
        let t2 = clamp_threshold(4 * factor + 3 + 5 * near, t1, maxval);
        let t3 = clamp_threshold(17 * factor + 4 + 7 * near, t2, maxval);
        (t1, t2, t3)
    } else {
        // Low-precision branch (ISO 14495-1 §C.2.4.1.1.1, MAXVAL < 128):
        // FACTOR = 256 / (MAXVAL + 1) divides the basic thresholds.
        let factor = 256 / (maxval + 1);
        let t1 = clamp_threshold((3 / factor + 3 * near).max(2), near + 1, maxval);
        let t2 = clamp_threshold((7 / factor + 5 * near).max(3), t1, maxval);
        let t3 = clamp_threshold((21 / factor + 7 * near).max(4), t2, maxval);
        (t1, t2, t3)
    }
}

/// ISO 14495-1 §C.2.4.1.1.1 Figure C.3 `CLAMP(i, j)` over integers: `j` when
/// `i > MAXVAL` or `i < j`, otherwise `i`, with MAXVAL ambient and not an
/// argument. Not a saturating clamp: a value above MAXVAL falls
/// to the lower bound `j` (Ocelli F-243).
fn clamp_threshold(i: i32, j: i32, maxval: i32) -> i32 {
    if i > maxval || i < j {
        j
    } else {
        i
    }
}

/// Coding parameters derived from the sample precision and the NEAR bound
/// (ISO 14495-1 §A.2.1) — shared by the encoder and the scan decoder.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CodingParams {
    /// Maximum sample value `2^bpp − 1`.
    pub(crate) maxval: i32,
    /// Range of quantized prediction errors; `2^bpp` for lossless.
    pub(crate) range: i32,
    /// `⌈log₂ RANGE⌉`; equals `bpp` for lossless.
    pub(crate) qbpp: u32,
    /// Golomb code length limit `2·(bpp + max(8, bpp))`.
    pub(crate) limit: u32,
    /// Context initialisation value `max(2, (RANGE + 32)/64)`.
    pub(crate) a_init: u32,
    /// NEAR bound (0 = lossless).
    pub(crate) near: i32,
}

impl CodingParams {
    pub(crate) fn new(bpp: u32, near: i32) -> Self {
        let maxval = (1i32 << bpp) - 1;
        let range = (maxval + 2 * near) / (2 * near + 1) + 1;
        let qbpp = u32::BITS - ((range - 1) as u32).leading_zeros();
        let limit = 2 * (bpp + bpp.max(8));
        let a_init = ((range as u32 + 32) >> 6).max(2);
        Self {
            maxval,
            range,
            qbpp,
            limit,
            a_init,
            near,
        }
    }
}

/// Reduce a quantized prediction error to the canonical modulo-RANGE
/// representative in `[−RANGE/2, (RANGE+1)/2)` (ISO 14495-1 §A.4.5).
#[inline(always)]
pub(crate) fn modulo_reduce(mut errval: i32, range: i32) -> i32 {
    if errval < 0 {
        errval += range;
    }
    if errval >= (range + 1) / 2 {
        errval -= range;
    }
    errval
}

/// Quantize a raw prediction error by the NEAR dead-zone (ISO 14495-1 §A.4.4):
/// `q = sign(e) · (|e| + NEAR) / (2·NEAR + 1)`. Identity for NEAR = 0.
#[inline(always)]
pub(crate) fn quantize_error(e: i32, near: i32) -> i32 {
    if e > 0 {
        (e + near) / (2 * near + 1)
    } else {
        -((near - e) / (2 * near + 1))
    }
}

/// Dequantize and reconstruct a sample (ISO 14495-1 §A.4.4, with the decoder's
/// modulo wrap from F.1 step 14): wrap into `[−NEAR, MAXVAL + NEAR]` by
/// RANGE·(2·NEAR+1) steps, then clamp to `[0, MAXVAL]`. For NEAR = 0 this is
/// the modulo-RANGE reconstruction.
#[inline(always)]
pub(crate) fn reconstruct(pred: i32, errval_q: i32, p: &CodingParams) -> i32 {
    let mut rx = pred + errval_q * (2 * p.near + 1);
    if rx < -p.near {
        rx += p.range * (2 * p.near + 1);
    } else if rx > p.maxval + p.near {
        rx -= p.range * (2 * p.near + 1);
    }
    rx.clamp(0, p.maxval)
}

/// Inverse error mapping: MErrval (≥0) → signed errval.
///
/// Forward mapping: errval ≥ 0 → MErrval = 2×errval (even); errval < 0 → MErrval = −2×errval−1 (odd).
/// Inverse: even MErrval → errval = MErrval/2; odd → errval = −(MErrval+1)/2.
/// ISO 14495-1 §A.5.2.
#[inline(always)]
pub(crate) fn inverse_map(me: u32) -> i32 {
    if me & 1 == 0 {
        (me >> 1) as i32
    } else {
        -(((me + 1) >> 1) as i32)
    }
}

#[cfg(test)]
mod tests;
