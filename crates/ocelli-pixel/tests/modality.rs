//! DICOM PS3.3 C.11.1 modality fixtures.

use ocelli_core::Stored;
use ocelli_pixel::{LutDescriptor, ModalityTransform, PixelError};

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok());
        let Ok(value) = result else { return };
        value
    }};
}

#[test]
fn rescale_uses_the_hand_computed_slope_and_intercept() {
    let transform = require_ok!(ModalityTransform::new(None, Some(2.0), Some(-1000.0)));

    // PS3.3 C.11.1: 100 * 2 + -1000 = -800.
    assert_eq!(
        transform.apply(Stored(100.0)).0.to_bits(),
        (-800.0_f32).to_bits()
    );
}

#[test]
fn modality_lut_sequence_takes_precedence_over_rescale() {
    let lut = require_ok!(LutDescriptor::new(3, -1, 16, vec![10.0, 20.0, 30.0]));
    let transform = require_ok!(ModalityTransform::new(Some(lut), Some(99.0), Some(1234.0)));

    // PS3.3 C.11.1: input 0 selects LUT entry 1. The rescale values are
    // deliberately incompatible and must be ignored when the sequence exists.
    assert_eq!(transform.apply(Stored(0.0)).0.to_bits(), 20.0_f32.to_bits());
}

#[test]
fn modality_lut_clamps_below_and_above_its_input_range() {
    let lut = require_ok!(LutDescriptor::new(3, -1, 16, vec![10.0, 20.0, 30.0]));
    let transform = require_ok!(ModalityTransform::new(Some(lut), None, None));

    assert_eq!(
        transform.apply(Stored(-20.0)).0.to_bits(),
        10.0_f32.to_bits()
    );
    assert_eq!(
        transform.apply(Stored(20.0)).0.to_bits(),
        30.0_f32.to_bits()
    );
}

#[test]
fn zero_descriptor_count_maps_exactly_65_536_entries_through_the_final_index() {
    // DICOM PS3.3 C.11.1.1.1 defines a zero first LUT Descriptor value as
    // exactly 2^16 entries. A distinct final value proves index 65,535 is
    // present and selected rather than clamped from a shorter table.
    let mut values = vec![0.0; 65_536];
    assert_eq!(values.len(), 65_536);
    let Some(final_value) = values.last_mut() else {
        return;
    };
    *final_value = 65_535.0;
    let lut = require_ok!(LutDescriptor::new(0, 0, 16, values));
    let transform = require_ok!(ModalityTransform::new(Some(lut), None, None));

    assert_eq!(
        transform.apply(Stored(65_535.0)).0.to_bits(),
        65_535.0_f32.to_bits()
    );
    assert_eq!(
        LutDescriptor::new(0, 0, 16, vec![0.0; 65_535]),
        Err(PixelError::InvalidLutLength)
    );
}

#[test]
fn lut_data_is_unsigned_integral_and_bounded_by_bits_per_entry() {
    // PS3.3 C.11.1.1.1 and C.11.2.1.1 define LUT output values as unsigned
    // integers in the range 0 through 2^n - 1.
    assert_eq!(
        LutDescriptor::new(1, 0, 8, vec![-1.0]),
        Err(PixelError::LutValueOutOfRange)
    );
    assert_eq!(
        LutDescriptor::new(1, 0, 8, vec![1.5]),
        Err(PixelError::NonIntegerLutValue)
    );
    assert_eq!(
        LutDescriptor::new(1, 0, 8, vec![256.0]),
        Err(PixelError::LutValueOutOfRange)
    );

    let eight_bit = require_ok!(LutDescriptor::new(2, 0, 8, vec![0.0, 255.0]));
    let eight_bit = require_ok!(ModalityTransform::new(Some(eight_bit), None, None));
    assert_eq!(
        eight_bit.apply(Stored(1.0)).0.to_bits(),
        255.0_f32.to_bits()
    );

    let sixteen_bit = require_ok!(LutDescriptor::new(2, 0, 16, vec![0.0, 65_535.0]));
    let sixteen_bit = require_ok!(ModalityTransform::new(Some(sixteen_bit), None, None));
    assert_eq!(
        sixteen_bit.apply(Stored(1.0)).0.to_bits(),
        65_535.0_f32.to_bits()
    );
}
