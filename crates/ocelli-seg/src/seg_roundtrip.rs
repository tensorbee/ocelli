//! Logical DICOM SEG frame round trip.
//!
//! PS3.3 C.8.20.2 says a segmentation pixel is a segment number and that 0
//! is background. Frame order is the caller order. The referenced SOP
//! instance UID stays on the frame that carried it. This is not a Part 10
//! file. The function does not call `present_labelmap`.

use alloc::string::String;
use alloc::vec::Vec;

/// One SEG frame and the source instance it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct SegFrame {
    /// Referenced SOP instance UID. Empty is refused.
    pub referenced_sop_instance_uid: String,
    /// Pixel rows. At least 1.
    pub rows: u32,
    /// Pixel columns. At least 1.
    pub columns: u32,
    /// One segment number per pixel, in row-major order. 0 is background.
    pub samples: Vec<f64>,
}

/// Why a SEG frame list was not copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegError {
    /// The referenced SOP instance UID is empty.
    Uid,
    /// Rows or columns are 0.
    Side,
    /// The sample count is not `rows * columns`.
    Length,
    /// A sample is negative, non-finite, or not an integer.
    Sample,
}

fn integer_sample(value: f64) -> bool {
    if !value.is_finite() || value < 0.0 {
        return false;
    }
    // `f64::trunc` is not in this `no_std` crate. At 2^53 and above every
    // finite value is an integer. Below that the fractional bits must be 0.
    const EXACT_INT_LIMIT: f64 = 9_007_199_254_740_992.0;
    if value >= EXACT_INT_LIMIT {
        return true;
    }
    let bits = value.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let Ok(biased) = u32::try_from(biased) else {
        return false;
    };
    if biased < 1023 {
        return value == 0.0;
    }
    let unbiased = biased - 1023;
    if unbiased >= 52 {
        return true;
    }
    let drop = 52 - unbiased;
    let fraction = bits & ((1_u64 << drop) - 1);
    fraction == 0
}

fn check_frame(frame: &SegFrame) -> Result<(), SegError> {
    if frame.referenced_sop_instance_uid.is_empty() {
        return Err(SegError::Uid);
    }
    if frame.rows == 0 || frame.columns == 0 {
        return Err(SegError::Side);
    }
    let rows = usize::try_from(frame.rows).map_err(|_| SegError::Side)?;
    let columns = usize::try_from(frame.columns).map_err(|_| SegError::Side)?;
    let expected = rows.checked_mul(columns).ok_or(SegError::Length)?;
    if frame.samples.len() != expected {
        return Err(SegError::Length);
    }
    for sample in &frame.samples {
        if !integer_sample(*sample) {
            return Err(SegError::Sample);
        }
    }
    Ok(())
}

fn copy_frames(frames: &[SegFrame]) -> Result<Vec<SegFrame>, SegError> {
    for frame in frames {
        check_frame(frame)?;
    }
    let mut copied = Vec::with_capacity(frames.len());
    for frame in frames {
        copied.push(SegFrame {
            referenced_sop_instance_uid: frame.referenced_sop_instance_uid.clone(),
            rows: frame.rows,
            columns: frame.columns,
            samples: frame.samples.clone(),
        });
    }
    Ok(copied)
}

/// Copy frames in caller order.
///
/// One bad frame refuses the whole call. The caller buffers are not written.
///
/// # Errors
///
/// [`SegError`] when a UID, a side, a length, or a sample is refused.
pub fn encode_seg(frames: &[SegFrame]) -> Result<Vec<SegFrame>, SegError> {
    copy_frames(frames)
}

/// Copy an encoded frame list without sorting.
///
/// The same UID may occur twice. Both frames stay, in the order given.
///
/// # Errors
///
/// [`SegError`] when a frame would also be refused by [`encode_seg`].
pub fn decode_seg(frames: &[SegFrame]) -> Result<Vec<SegFrame>, SegError> {
    copy_frames(frames)
}
