//! ISO 14495-1 JPEG-LS scan decoder.
//!
//! Implements both regular-mode and run-mode decoding per §A.3 to §A.7 and
//! Annex F, F.1.
//! Processes one single-component scan; multi-component support is not
//! implemented by RITK's grayscale DICOM codec boundary.

use anyhow::{bail, Result};

use super::bitstream::BitReader;
use super::context::{
    compute_k, context_index, error_correction, inverse_map, quant, reconstruct, sign_normalize,
    update_context, CodingParams, ContextModel,
};
use super::reconstruction::ReconstructionRows;

/// Golomb run-length table `J[0..32]` — ISO 14495-1 §A.2.1 step 3.
///
/// `J[i]` is the Golomb order: at run index i, a hit extends the run by `2^J[i]`.
/// Shared by the scan decoder and the [`super::encoder`] (single source of truth).
pub(super) const J: [u32; 32] = [
    0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, 12, 13,
    14, 15,
];

/// Predictor modes carried by upstream as exhaustive match arms in `predict()`.
///
/// ISO 14495-1 defines one predictor, the edge-detecting predictor of §A.4.1,
/// and the SOS header selects none. DICOM JPEG-LS production decode uses the
/// adaptive predictor.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Predictor {
    /// Predictor 0: Px = 0. At the image edges `predict` gives 0 for the first
    /// sample, b in the first column and a in the first row.
    None = 0,
    /// Predictor 1: left neighbor (a).
    Left = 1,
    /// Predictor 2: above neighbor (b).
    Up = 2,
    /// Predictor 3: above-left diagonal (c).
    UpLeft = 3,
    /// Predictor 4: a + b − c.
    UpPlusLeftMinusUpLeft = 4,
    /// Predictor 5: b + (c − a)/2.
    UpPlusHalfDiff = 5,
    /// Predictor 6: a + (c − b)/2.
    LeftPlusHalfDiff = 6,
    /// Predictor 7: ISO edge-detecting adaptive predictor (§A.4.1).
    Adaptive = 7,
}

/// ISO 14495-1 §A.4.1 edge-detecting predictor.
///
/// Proof:
/// - c ≥ max(a,b): both a and b are below the diagonal → select min(a,b).
/// - c ≤ min(a,b): both a and b are above the diagonal → select max(a,b).
/// - Otherwise: linear combination a + b − c (unbiased gradient estimate).
#[inline(always)]
fn predict_adaptive(a: i32, b: i32, c: i32) -> i32 {
    let min_ab = a.min(b);
    let max_ab = a.max(b);
    if c >= max_ab {
        min_ab
    } else if c <= min_ab {
        max_ab
    } else {
        a + b - c
    }
}

/// Compute the predictor for sample at (row, col) given causal neighborhood.
///
/// Boundary conditions per ISO 14495-1 §A.2.1:
/// - (0, 0): Px = 0.
/// - (r, 0): Px = above = b.
/// - (0, c): Px = left = a.
#[inline(always)]
pub(super) fn predict(
    a: i32,
    b: i32,
    c: i32,
    _d: i32,
    predictor: Predictor,
    row: usize,
    col: usize,
) -> i32 {
    if col == 0 && row == 0 {
        return 0;
    } else if col == 0 {
        return b; // first column: UP
    } else if row == 0 {
        return a; // first row: LEFT
    }
    match predictor {
        Predictor::None => 0,
        Predictor::Left => a,
        Predictor::Up => b,
        Predictor::UpLeft => c,
        Predictor::UpPlusLeftMinusUpLeft => a + b - c,
        Predictor::UpPlusHalfDiff => b + (c - a) / 2,
        Predictor::LeftPlusHalfDiff => a + (c - b) / 2,
        Predictor::Adaptive => predict_adaptive(a, b, c),
    }
}

/// Decoded scan parameters extracted from the SOF55 and SOS headers.
pub(super) struct ScanParams {
    pub(super) rows: usize,
    pub(super) cols: usize,
    pub(super) bpp: u32,
    pub(super) near: u32,
    pub(super) predictor: Predictor,
    /// Override thresholds from LSE marker; (0, 0, 0) means use defaults.
    pub(super) t1: i32,
    pub(super) t2: i32,
    pub(super) t3: i32,
}

/// Decode a single-component JPEG-LS scan into sample values.
///
/// Implements ISO 14495-1 §A.3 to §A.6 (regular mode, decoded per F.1 steps 2
/// to 17) and §A.7 (run mode, decoded per F.1 step 18).
///
/// # Arguments
/// * `reader` — Bit reader positioned at the start of scan data.
/// * `params` — Decoded frame and scan header parameters.
/// * `samples` — Output buffer; receives `rows × cols` decoded samples.
pub(super) fn decode_scan(
    reader: &mut BitReader<'_>,
    params: &ScanParams,
    samples: &mut Vec<i32>,
) -> Result<()> {
    let rows = params.rows;
    let cols = params.cols;
    let near = i32::try_from(params.near)
        .expect("invariant: the fragment boundary validates JPEG-LS NEAR");
    let cp = CodingParams::new(params.bpp, near);
    let maxval = cp.maxval;
    let qbpp = cp.qbpp;
    let limit = cp.limit;

    let (t1, t2, t3) = if params.t1 > 0 {
        (params.t1, params.t2, params.t3)
    } else {
        super::context::default_thresholds(maxval, near)
    };

    let mut ctx = ContextModel::new(cp.a_init);
    // Prediction is causal: each sample reads only the row above and samples
    // already reconstructed in the current row. Reuse two rows so scratch is
    // O(cols), independent of image height.
    let mut reconstructed = ReconstructionRows::new(cols);

    for r in 0..rows {
        let current_line_left_guard = reconstructed.current_line_left_guard();

        let mut c = 0usize;
        while c < cols {
            // Causal neighborhood
            let (a, b, cc, d) = reconstructed.neighborhood(c, current_line_left_guard);

            // Local gradients (ISO 14495-1 §A.3.1 names D1, D2 and D3 the
            // local gradients, without orientation adjectives)
            let d1 = d - b;
            let d2 = b - cc;
            let d3 = cc - a;

            // Quantize gradients
            let q1 = quant(d1, t1, t2, t3, near);
            let q2 = quant(d2, t1, t2, t3, near);
            let q3 = quant(d3, t1, t2, t3, near);

            if q1 == 0 && q2 == 0 && q3 == 0 {
                // ── Run mode (ISO 14495-1 §A.7, F.1 step 18) ─────────────────────
                let run_val = a;
                let remaining = cols - c;
                let mut run_len = 0usize;

                // Decode run length using J-table Golomb coding.
                //
                // Ocelli F-243, ITU-T T.87 F.1 step 18 (informative): a hit
                // adds min(2^J[RUNindex], samples left on the line), and
                // RUNindex rises only when that count is the full segment. A
                // hit that only fills the rest of the line leaves it alone.
                loop {
                    let idx = ctx.run_index.min(31);
                    if reader.read_bit() == 1 {
                        let full = 1usize << J[idx];
                        let count = full.min(remaining - run_len);
                        run_len += count;
                        if count == full && ctx.run_index < 31 {
                            ctx.run_index += 1;
                        }
                        if run_len == remaining {
                            break;
                        }
                    } else {
                        // Run miss: read J[idx] remainder bits
                        let rem = if J[idx] > 0 {
                            reader.read_bits(J[idx]) as usize
                        } else {
                            0
                        };
                        run_len += rem;
                        // A remainder that carries the run past the line is
                        // not a stream any encoder writes (Ocelli F-243).
                        if run_len > remaining {
                            bail!("JPEG-LS run length passes the end of line {r}");
                        }
                        break;
                    }
                }

                // Fill run with run_val
                reconstructed.fill_current(c, run_len, run_val);
                c += run_len;

                if run_len < remaining {
                    // Run interrupt: decode interruption sample
                    let (rb, ra) = reconstructed.interruption_neighbors(c);
                    let run_index = ctx.run_index.min(31);
                    let ri_ctx = if (rb - ra).abs() <= params.near as i32 {
                        &mut ctx.run_int_same
                    } else {
                        &mut ctx.run_int_diff
                    };
                    let k = ri_ctx.compute_k(qbpp);
                    let me = reader.read_golomb(k, limit - J[run_index] - 1, qbpp);
                    let errval = ri_ctx.compute_error_value(me + ri_ctx.run_interruption_type(), k);
                    if !in_coded_range(errval, cp.range) {
                        bail!("JPEG-LS run interruption error outside the coded range in line {r}");
                    }
                    ri_ctx.update(errval, me);

                    let rx = if ri_ctx.run_interruption_type() == 1 {
                        reconstruct(ra, errval, &cp)
                    } else {
                        reconstruct(rb, errval * (rb - ra).signum(), &cp)
                    };

                    // Decrement run index on interrupt
                    if ctx.run_index > 0 {
                        ctx.run_index -= 1;
                    }

                    reconstructed.set_current(c, rx);
                    c += 1;
                }
                continue;
            }

            // ── Regular mode (ISO 14495-1 §A.3 to §A.6) ──────────────────────
            let (nq1, nq2, nq3, sign) = sign_normalize(q1, q2, q3);
            let qi = context_index(nq1, nq2, nq3);
            debug_assert!(qi < 365, "context index out of range: {}", qi);
            let ctx_reg = &mut ctx.regular[qi];

            // Golomb-Rice parameter k
            let k = compute_k(ctx_reg.a, ctx_reg.n, qbpp);

            // Edge-detecting predictor with sign-normalized bias correction.
            let px_raw = predict(a, b, cc, d, params.predictor, r, c);
            let signed_c = if sign < 0 { -ctx_reg.c } else { ctx_reg.c };
            let px = (px_raw + signed_c).clamp(0, maxval);

            // Decode MErrval from bitstream
            let me = reader.read_golomb(k, limit, qbpp);
            let mut errval_canon = inverse_map(me);
            if k == 0 {
                errval_canon ^= error_correction(ctx_reg, params.near);
            }
            if !in_coded_range(errval_canon, cp.range) {
                bail!("JPEG-LS prediction error outside the coded range in line {r}");
            }
            let errval = sign * errval_canon;

            // Reconstruct sample
            let rx = reconstruct(px, errval, &cp);
            reconstructed.set_current(c, rx);

            // Update context (errval relative to context sign)
            update_context(ctx_reg, errval_canon, params.near);
            c += 1;
        }
        samples.extend_from_slice(reconstructed.current());
        reconstructed.finish_row(current_line_left_guard);
        // Ocelli F-243: past the end of the data the reader returns zero
        // bits, which decode as plausible samples, so running out is refused.
        if reader.exhausted() {
            bail!("JPEG-LS scan ran out of entropy-coded data in line {r}");
        }
    }

    Ok(())
}

/// Whether a decoded prediction error is one an encoder can write (Ocelli
/// F-243). T.87 A.4.5 reduces every error modulo RANGE into
/// `[-floor(RANGE / 2), ceil(RANGE / 2) - 1]` before coding, so a value outside
/// it is a damaged scan, which would otherwise wrap or clamp to some sample.
/// CharLS 2.4.4 was measured to open some such scans.
///
/// Decoder-only premise for the `k < qbpp` cap: this refusal runs before both
/// state updates, the regular update and the run-interruption update, so the
/// A and N state stays within the T.87 A.2.1 initialization bound and the
/// reset bounds of A.6.1 and A.7.2 Code segment A.23, the run-interruption
/// reset. The encoder instead relies on its own `modulo_reduce` before its
/// updates and does not call this function. Pass 8 measured at its tip that
/// with this refusal disabled the cap binds about two million times on
/// damaged streams, a decoder-only measurement not re-proven here.
fn in_coded_range(errval: i32, range: i32) -> bool {
    -(range / 2) <= errval && errval < (range + 1) / 2
}

/// Refuse a scan that does not end where its last sample does (Ocelli F-243).
///
/// ISO 14495-1 §A.1 pads the final byte to a byte boundary. The bits after
/// the last sample must all be zero, there may be at most 7 of them, the rest
/// of one byte, and the marker after them must be EOI, which 0xFF fill bytes
/// may precede. A non-zero bit there is how a damaged final data bit shows,
/// and a whole byte more is how a damaged scan that reads as a shorter valid
/// one shows, so both are refused. Bytes after that EOI are the adapter's to
/// judge. A scan that ran out of data was already refused by `decode_scan`.
pub(super) fn verify_scan_end(reader: &BitReader<'_>) -> Result<()> {
    let end = reader.segment_end();
    if !end.unread_zero {
        bail!("JPEG-LS scan leaves non-zero entropy-coded bits after its last sample");
    }
    if end.unread_bits > 7 {
        bail!(
            "JPEG-LS scan leaves {} entropy-coded bits unread before its marker",
            end.unread_bits
        );
    }
    if !end.eoi {
        bail!("JPEG-LS scan is not followed by the EOI marker");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predict_adaptive_c_above_max() {
        // c=10 >= max(a=3,b=5)=5 → min(a,b) = 3
        assert_eq!(predict_adaptive(3, 5, 10), 3);
    }

    #[test]
    fn predict_adaptive_c_below_min() {
        // c=1 <= min(a=3,b=5)=3 → max(a,b) = 5
        assert_eq!(predict_adaptive(3, 5, 1), 5);
    }

    #[test]
    fn predict_adaptive_interior() {
        // c=4, a=3, b=5: min=3 <= 4 <= max=5 → a+b-c = 3+5-4 = 4
        assert_eq!(predict_adaptive(3, 5, 4), 4);
    }

    #[test]
    fn predict_left_mode_interior() {
        // Predictor::Left → a
        assert_eq!(predict(7, 10, 6, 11, Predictor::Left, 1, 1), 7);
    }

    #[test]
    fn predict_boundary_first_sample() {
        // row=0, col=0 → always 0
        assert_eq!(predict(5, 5, 5, 5, Predictor::Adaptive, 0, 0), 0);
    }

    #[test]
    fn predict_boundary_first_col() {
        // col=0, row>0 → UP = b
        assert_eq!(predict(3, 8, 2, 9, Predictor::Left, 1, 0), 8);
    }

    #[test]
    fn predict_boundary_first_row() {
        // row=0, col>0 → LEFT = a
        assert_eq!(predict(4, 0, 0, 0, Predictor::Up, 0, 1), 4);
    }

    #[test]
    fn decode_scan_constant_zero_left_predictor_2x2() {
        // All gradients are zero, so rows are represented by run hits only.
        // The test byte provides five hit bits plus padding.
        let data: &[u8] = &[0b11111000u8];
        let mut reader = BitReader::new(data);
        let params = ScanParams {
            rows: 2,
            cols: 2,
            bpp: 8,
            near: 0,
            predictor: Predictor::Left,
            t1: 0,
            t2: 0,
            t3: 0, // use defaults
        };
        let mut samples = Vec::new();
        decode_scan(&mut reader, &params, &mut samples).expect("decode_scan should succeed");
        assert_eq!(samples.len(), 4);
        assert_eq!(samples, vec![0, 0, 0, 0]);
    }
}
