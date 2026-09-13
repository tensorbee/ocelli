//! DICOM PS3.3 C.7.6.3 stored-pixel fixtures.

use ocelli_core::Stored;
use ocelli_pixel::{
    ByteOrder, ImageDimensions, PhotometricInterpretation, PixelRepresentation, SampleLayout,
    StoredBits, StoredPixelDescription,
};

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok());
        let Ok(value) = result else { return };
        value
    }};
}

#[test]
fn twelve_bit_signed_values_sign_extend_from_bit_eleven() {
    let description = StoredPixelDescription::new(
        require_ok!(ImageDimensions::new(1, 2)),
        require_ok!(SampleLayout::new(
            1,
            PhotometricInterpretation::Monochrome2,
            None
        )),
        require_ok!(StoredBits::new(16, 12, 11, PixelRepresentation::Signed)),
    );
    let mut output = [Stored(99.0); 2];

    // PS3.3 C.7.6.3: mask to 12 bits, then sign-extend from bit 11.
    assert_eq!(
        description.unpack(
            &[0x00, 0xf8, 0xff, 0x07],
            ByteOrder::LittleEndian,
            &mut output,
        ),
        Ok(())
    );

    assert_eq!(output[0].0.to_bits(), (-2048.0_f32).to_bits());
    assert_eq!(output[1].0.to_bits(), 2047.0_f32.to_bits());
}

#[test]
fn twelve_bit_unsigned_values_mask_unused_container_bits() {
    let description = StoredPixelDescription::new(
        require_ok!(ImageDimensions::new(1, 2)),
        require_ok!(SampleLayout::new(
            1,
            PhotometricInterpretation::Monochrome2,
            None
        )),
        require_ok!(StoredBits::new(16, 12, 11, PixelRepresentation::Unsigned)),
    );
    let mut output = [Stored(99.0); 2];

    assert_eq!(
        description.unpack(
            &[0xff, 0xff, 0xff, 0x0f],
            ByteOrder::LittleEndian,
            &mut output,
        ),
        Ok(())
    );

    assert_eq!(output[0].0.to_bits(), 4095.0_f32.to_bits());
    assert_eq!(output[1].0.to_bits(), 4095.0_f32.to_bits());
}
