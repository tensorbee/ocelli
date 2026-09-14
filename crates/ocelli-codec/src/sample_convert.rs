//! The stored-domain boundary shared by the adapters whose dependency returns
//! `f32` samples.
//!
//! `ritk-codecs` decodes JPEG 2000 and JPEG-LS through one `PixelLayout` and
//! returns `Vec<f32>`, and `openjph-core` returns `Vec<i32>` per row. The
//! stored domain for every DICOM syntax those cover is integer, so the
//! conversion back has to refuse anything that is not exactly an integer inside
//! the descriptor's stored range. That check exists once, here, because a
//! byte-identical copy of it in each adapter is the defect class this
//! repository exists to catch.

use crate::{CodecError, FrameDesc, PixelRepresentation};

/// Validate every decoded sample and write the caller's output atomically.
///
/// An error leaves `out` byte-unchanged, which is the contract every adapter in
/// this crate holds.
pub(crate) fn convert_samples(
    decoded: &[f32],
    desc: &FrameDesc,
    out: &mut [u8],
) -> Result<(), CodecError> {
    let expected_samples = usize::from(desc.rows())
        .checked_mul(usize::from(desc.columns()))
        .ok_or(CodecError::FrameMismatch)?;
    if decoded.len() != expected_samples {
        return Err(CodecError::FrameMismatch);
    }
    if out.len() != desc.output_len() {
        return Err(CodecError::OutputLength {
            expected: desc.output_len(),
            actual: out.len(),
        });
    }

    let mut bytes = Vec::with_capacity(desc.output_len());
    for &value in decoded {
        append_sample(&mut bytes, value, desc)?;
    }
    if bytes.len() != out.len() {
        return Err(CodecError::FrameMismatch);
    }
    out.copy_from_slice(&bytes);
    Ok(())
}

fn append_sample(bytes: &mut Vec<u8>, value: f32, desc: &FrameDesc) -> Result<(), CodecError> {
    let sample = exact_i64(value).ok_or(CodecError::UnsupportedPixelFormat)?;
    let bits = desc.bits_stored();
    match (desc.bits_allocated(), desc.pixel_representation()) {
        (8, PixelRepresentation::Unsigned) => {
            let max = (1_i64 << bits) - 1;
            if !(0..=max).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.push(u8::try_from(sample).map_err(|_| CodecError::UnsupportedPixelFormat)?);
        }
        (8, PixelRepresentation::Signed) => {
            let limit = 1_i64 << (bits - 1);
            if !(-limit..limit).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.extend_from_slice(
                &i8::try_from(sample)
                    .map_err(|_| CodecError::UnsupportedPixelFormat)?
                    .to_le_bytes(),
            );
        }
        (16, PixelRepresentation::Unsigned) => {
            let max = (1_i64 << bits) - 1;
            if !(0..=max).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.extend_from_slice(
                &u16::try_from(sample)
                    .map_err(|_| CodecError::UnsupportedPixelFormat)?
                    .to_le_bytes(),
            );
        }
        (16, PixelRepresentation::Signed) => {
            let limit = 1_i64 << (bits - 1);
            if !(-limit..limit).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.extend_from_slice(
                &i16::try_from(sample)
                    .map_err(|_| CodecError::UnsupportedPixelFormat)?
                    .to_le_bytes(),
            );
        }
        _ => return Err(CodecError::UnsupportedPixelFormat),
    }
    Ok(())
}

fn exact_i64(value: f32) -> Option<i64> {
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    if value == 0.0 {
        return Some(0);
    }
    let bits = value.to_bits();
    let exponent = i32::try_from((bits >> 23) & 0xff).ok()?.checked_sub(127)?;
    if !(0..=62).contains(&exponent) {
        return None;
    }
    let mantissa = u64::from((bits & 0x7f_ffff) | 0x80_0000);
    let magnitude = if exponent >= 23 {
        mantissa.checked_shl(u32::try_from(exponent - 23).ok()?)?
    } else {
        let shift = u32::try_from(23 - exponent).ok()?;
        let mask = (1_u64.checked_shl(shift)?).checked_sub(1)?;
        if mantissa & mask != 0 {
            return None;
        }
        mantissa >> shift
    };
    let signed = i64::try_from(magnitude).ok()?;
    if bits >> 31 == 0 {
        Some(signed)
    } else {
        signed.checked_neg()
    }
}

#[cfg(test)]
mod tests {
    use super::{convert_samples, exact_i64};
    use crate::{
        CodecError, FrameDesc, FrameDescError, FrameDescInput, PixelDataVr, PixelRepresentation,
    };

    #[test]
    fn exact_integer_conversion_refuses_fractional_and_nonfinite_values() {
        assert_eq!(exact_i64(4_095.0), Some(4_095));
        assert_eq!(exact_i64(-2_048.0), Some(-2_048));
        assert_eq!(exact_i64(0.5), None);
        assert_eq!(exact_i64(f32::NAN), None);
        assert_eq!(exact_i64(f32::INFINITY), None);
    }

    #[test]
    fn conversion_errors_preserve_caller_output() -> Result<(), FrameDescError> {
        let desc = FrameDesc::new(FrameDescInput {
            rows: 1,
            columns: 1,
            samples_per_pixel: 1,
            bits_allocated: 8,
            bits_stored: 8,
            high_bit: 7,
            pixel_representation: PixelRepresentation::Unsigned,
            photometric_interpretation: "MONOCHROME2".to_owned(),
            pixel_data_vr: PixelDataVr::Ob,
        })?;

        for decoded in [&[0.5_f32][..], &[256.0_f32][..]] {
            let mut out = [0xa5];
            assert_eq!(
                convert_samples(decoded, &desc, &mut out),
                Err(CodecError::UnsupportedPixelFormat)
            );
            assert_eq!(out, [0xa5]);
        }

        let mut out = [0xa5];
        assert_eq!(
            convert_samples(&[0.0, 1.0], &desc, &mut out),
            Err(CodecError::FrameMismatch)
        );
        assert_eq!(out, [0xa5]);
        Ok(())
    }
}
