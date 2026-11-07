//! The one rule for whether two in-plane spacing pairs are the same.
//!
//! F-211, the lead ruling after its review pass 2. PS3.3 10.7.1.1 compares
//! Pixel Spacing (0028,0030) with Imager Pixel Spacing (0018,1164): equal
//! values mean no correction for geometric magnification, different values
//! mean one. `ocelli-dicom` classifies the pair with this function, and
//! `ocelli-geom`'s `select_calibration` chooses a source with it, so the two
//! public APIs give the same answer for the same file. `ocelli-dicom` also
//! uses it for in-plane spacing agreement between frames.

/// Millimetres. It is not a physical tolerance. A Decimal String is at most
/// 16 bytes (PS3.5 Table 6.2-1), so a spacing such as `0.1430000001` can
/// state a difference near 1e-10 mm, and two such values within this bound
/// are treated as the same. Text that differs only in trailing zeros or in
/// its exponent form already parses to the same `f64`.
///
/// It is chosen once, here. Changing it is a design-plan decision under
/// `.claude/WORKFLOW.md`'s tolerance policy, not a fix for a failing case.
pub const SPACING_EQUALITY_MM: f64 = 1e-9;

/// Whether two spacing pairs, row then column, agree in both halves within
/// [`SPACING_EQUALITY_MM`].
///
/// A non-finite half never agrees, because its difference is not finite.
#[must_use]
pub fn spacing_pair_agrees(
    left_row: f64,
    left_column: f64,
    right_row: f64,
    right_column: f64,
) -> bool {
    (left_row - right_row).abs() <= SPACING_EQUALITY_MM
        && (left_column - right_column).abs() <= SPACING_EQUALITY_MM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pair_agrees_within_the_tolerance_and_in_both_halves() {
        // 0.1430000001 - 0.143 is about 1e-10, inside 1e-9. 0.143001 - 0.143
        // is about 1e-6, outside. Either half outside is a difference.
        assert!(spacing_pair_agrees(
            0.143_000_000_1,
            0.143_000_000_1,
            0.143,
            0.143
        ));
        assert!(!spacing_pair_agrees(0.143_001, 0.143, 0.143, 0.143));
        assert!(!spacing_pair_agrees(0.143, 0.143_001, 0.143, 0.143));
        assert!(!spacing_pair_agrees(f64::NAN, 0.2, f64::NAN, 0.2));
    }
}
