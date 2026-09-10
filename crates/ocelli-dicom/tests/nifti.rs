//! Synthetic NIfTI-1.1 fixtures for F-022.
//!
//! The byte offsets, datatype codes, storage order, affine formulas, and
//! RAS coordinates come from the official NIfTI-1 `nifti1.h` definition:
//! <https://github.com/NIFTI-Imaging/nifti_clib/blob/master/niftilib/nifti1.h>.
//! NIfTI-2 recognition uses its official 540-byte header definition:
//! <https://github.com/NIFTI-Imaging/nifti_clib/blob/master/nifti2/nifti2.h>.

use ocelli_core::{Index, Pt, World};
use ocelli_dicom::{
    NiftiAffineSource, NiftiByteOrder, NiftiDataType, NiftiError, NiftiSpatialUnits, ParsedNifti,
    parse_nifti,
};
use proptest::prelude::*;

const HEADER_AND_EXTENDER: usize = 352;
const DIMS: [u16; 3] = [2, 3, 2];
const VOXELS: usize = 12;

fn put<const N: usize>(bytes: &mut [u8], offset: usize, value: [u8; N]) {
    let Some(tail) = bytes.get_mut(offset..) else {
        return;
    };
    let Some(slot) = tail.get_mut(..N) else {
        return;
    };
    slot.copy_from_slice(&value);
}

fn put_i16(bytes: &mut [u8], offset: usize, value: i16) {
    put(bytes, offset, value.to_le_bytes());
}

fn put_i16_be(bytes: &mut [u8], offset: usize, value: i16) {
    put(bytes, offset, value.to_be_bytes());
}

fn put_i32(bytes: &mut [u8], offset: usize, value: i32) {
    put(bytes, offset, value.to_le_bytes());
}

fn put_f32(bytes: &mut [u8], offset: usize, value: f32) {
    put(bytes, offset, value.to_le_bytes());
}

fn bytes_per_voxel(data_type: NiftiDataType) -> usize {
    match data_type {
        NiftiDataType::U8 => 1,
        NiftiDataType::I16 | NiftiDataType::U16 => 2,
        NiftiDataType::F32 => 4,
    }
}

fn datatype_fields(data_type: NiftiDataType) -> (i16, i16) {
    match data_type {
        NiftiDataType::U8 => (2, 8),
        NiftiDataType::I16 => (4, 16),
        NiftiDataType::U16 => (512, 16),
        NiftiDataType::F32 => (16, 32),
    }
}

fn fixture(data_type: NiftiDataType) -> Vec<u8> {
    fixture_with_dims(data_type, DIMS, HEADER_AND_EXTENDER)
}

fn fixture_with_dims(data_type: NiftiDataType, dims: [u16; 3], payload_offset: usize) -> Vec<u8> {
    let voxel_count = usize::from(dims[0]) * usize::from(dims[1]) * usize::from(dims[2]);
    let payload_length = voxel_count * bytes_per_voxel(data_type);
    let mut bytes = vec![0; payload_offset + payload_length];
    put_i32(&mut bytes, 0, 348);
    put_i16(&mut bytes, 40, 3);
    for (index, dimension) in dims.into_iter().enumerate() {
        put_i16(
            &mut bytes,
            42 + index * 2,
            i16::try_from(dimension).unwrap_or(i16::MAX),
        );
    }
    let (datatype, bitpix) = datatype_fields(data_type);
    put_i16(&mut bytes, 70, datatype);
    put_i16(&mut bytes, 72, bitpix);
    put_f32(&mut bytes, 76, 1.0);
    put_f32(&mut bytes, 80, 1.0);
    put_f32(&mut bytes, 84, 1.0);
    put_f32(&mut bytes, 88, 1.0);
    put_f32(
        &mut bytes,
        108,
        u16::try_from(payload_offset).map_or(352.0, f32::from),
    );
    put_f32(&mut bytes, 112, 2.5);
    put_f32(&mut bytes, 116, -7.25);
    if let Some(units) = bytes.get_mut(123) {
        *units = 2 | 16;
    }
    put_i16(&mut bytes, 252, 1);
    put_i16(&mut bytes, 254, 0);
    put(&mut bytes, 344, *b"n+1\0");
    for (index, value) in bytes
        .get_mut(payload_offset..)
        .unwrap_or_default()
        .iter_mut()
        .enumerate()
    {
        *value = u8::try_from(index % 251).unwrap_or_default();
    }
    bytes
}

fn parse(bytes: &[u8]) -> Result<ParsedNifti, NiftiError> {
    parse_nifti(bytes)
}

fn error(bytes: &[u8]) -> Option<NiftiError> {
    parse(bytes).err()
}

fn assert_world(actual: Pt<World>, expected: (f64, f64, f64)) {
    assert!((actual.x - expected.0).abs() <= 1e-6);
    assert!((actual.y - expected.1).abs() <= 1e-6);
    assert!((actual.z - expected.2).abs() <= 1e-6);
}

#[test]
fn every_supported_datatype_preserves_header_scaling_and_exact_payload() {
    for data_type in [
        NiftiDataType::U8,
        NiftiDataType::I16,
        NiftiDataType::U16,
        NiftiDataType::F32,
    ] {
        let bytes = fixture(data_type);
        let result = parse(&bytes);
        assert!(result.is_ok());
        let Ok(parsed) = result else {
            continue;
        };
        assert_eq!(parsed.header().byte_order(), NiftiByteOrder::LittleEndian);
        assert_eq!(parsed.header().data_type(), data_type);
        assert_eq!(parsed.header().dimensions(), [2, 3, 2]);
        assert_eq!(parsed.header().declared_rank(), 3);
        assert_eq!(
            parsed.header().spatial_units(),
            NiftiSpatialUnits::Millimetres
        );
        assert_eq!(
            parsed.header().scaling().slope().to_bits(),
            2.5_f32.to_bits()
        );
        assert_eq!(
            parsed.header().scaling().intercept().to_bits(),
            (-7.25_f32).to_bits()
        );
        assert_eq!(
            parsed.payload_range(),
            HEADER_AND_EXTENDER..HEADER_AND_EXTENDER + VOXELS * bytes_per_voxel(data_type)
        );
    }
}

#[test]
fn undeclared_higher_dimension_slots_and_temporal_unit_bits_are_ignored() {
    let mut bytes = fixture(NiftiDataType::U8);
    put_i16(&mut bytes, 48, 29);
    put_i16(&mut bytes, 50, -8);
    put_i16(&mut bytes, 52, 17);
    put_i16(&mut bytes, 54, -2);
    assert!(parse(&bytes).is_ok());

    let mut declared_singletons = fixture(NiftiDataType::U8);
    put_i16(&mut declared_singletons, 40, 7);
    for offset in [48, 50, 52, 54] {
        put_i16(&mut declared_singletons, offset, 1);
    }
    assert!(parse(&declared_singletons).is_ok());
}

#[test]
fn sform_precedes_qform_and_converts_a_non_symmetric_affine_to_lps() {
    let mut bytes = fixture(NiftiDataType::U8);
    put_i16(&mut bytes, 252, 1);
    put_f32(&mut bytes, 256, 0.125);
    put_f32(&mut bytes, 260, -0.25);
    put_f32(&mut bytes, 264, 0.5);
    put_f32(&mut bytes, 268, 101.0);
    put_f32(&mut bytes, 272, 202.0);
    put_f32(&mut bytes, 276, 303.0);
    put_i16(&mut bytes, 254, 2);
    let rows = [
        [2.0, 0.5, 0.0, 10.0],
        [0.0, 3.0, -0.25, 20.0],
        [0.75, 0.0, 4.0, -30.0],
    ];
    for (row_index, row) in rows.into_iter().enumerate() {
        for (column_index, value) in row.into_iter().enumerate() {
            put_f32(&mut bytes, 280 + row_index * 16 + column_index * 4, value);
        }
    }

    let result = parse(&bytes);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    assert_eq!(parsed.selected_affine_source(), NiftiAffineSource::SForm);
    assert_eq!(parsed.header().qform().code(), 1);
    assert_eq!(
        parsed.header().qform().quaternion().map(f32::to_bits),
        [0.125_f32, -0.25, 0.5].map(f32::to_bits)
    );
    assert_eq!(
        parsed.header().qform().offset().map(f32::to_bits),
        [101.0_f32, 202.0, 303.0].map(f32::to_bits)
    );
    assert_eq!(parsed.header().sform().code(), 2);
    assert_eq!(
        parsed
            .header()
            .sform()
            .rows()
            .map(|row| row.map(f32::to_bits)),
        rows.map(|row| row.map(f32::to_bits))
    );

    let transform = parsed.index_to_world();
    // NIfTI rows give RAS. Left multiplication by diag(-1,-1,1,1)
    // negates the first two complete rows, including their translations.
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 0.0)),
        (-10.0, -20.0, -30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(1.0, 0.0, 0.0)),
        (-12.0, -20.0, -29.25),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 1.0, 0.0)),
        (-10.5, -23.0, -30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 1.0)),
        (-10.0, -19.75, -26.0),
    );
}

#[test]
fn qform_uses_pixdim_qfac_and_offsets_before_lps_conversion() {
    let mut bytes = fixture(NiftiDataType::U8);
    put_f32(&mut bytes, 76, -1.0);
    put_f32(&mut bytes, 80, 2.0);
    put_f32(&mut bytes, 84, 3.0);
    put_f32(&mut bytes, 88, 4.0);
    put_f32(&mut bytes, 264, core::f32::consts::FRAC_1_SQRT_2);
    put_f32(&mut bytes, 268, 10.0);
    put_f32(&mut bytes, 272, -20.0);
    put_f32(&mut bytes, 276, 30.0);

    let result = parse(&bytes);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    assert_eq!(parsed.selected_affine_source(), NiftiAffineSource::QForm);
    assert_eq!(
        parsed.header().voxel_spacing_mm().map(f32::to_bits),
        [2.0_f32, 3.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(
        parsed.header().qform().qfac().to_bits(),
        (-1.0_f32).to_bits()
    );
    assert_eq!(
        parsed.header().qform().quaternion().map(f32::to_bits),
        [0.0_f32, 0.0, core::f32::consts::FRAC_1_SQRT_2].map(f32::to_bits)
    );
    assert_eq!(
        parsed.header().qform().offset().map(f32::to_bits),
        [10.0_f32, -20.0, 30.0].map(f32::to_bits)
    );
    let transform = parsed.index_to_world();
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 0.0)),
        (-10.0, 20.0, 30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(1.0, 0.0, 0.0)),
        (-10.0, 18.0, 30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 1.0, 0.0)),
        (-7.0, 20.0, 30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 1.0)),
        (-10.0, 20.0, 26.0),
    );
}

#[test]
fn qfac_zero_means_positive_one() {
    let mut bytes = fixture(NiftiDataType::U8);
    put_f32(&mut bytes, 76, 0.0);
    let result = parse(&bytes);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    assert_eq!(parsed.header().qform().qfac().to_bits(), 0.0_f32.to_bits());
    assert_world(
        parsed
            .index_to_world()
            .apply(Pt::<Index>::new(0.0, 0.0, 1.0)),
        (0.0, 0.0, 1.0),
    );
}

#[test]
fn qform_quaternion_rotation_uses_the_official_row_signs() {
    let mut bytes = fixture(NiftiDataType::U8);
    put_f32(&mut bytes, 80, 2.0);
    put_f32(&mut bytes, 84, 3.0);
    put_f32(&mut bytes, 88, 4.0);
    put_f32(&mut bytes, 264, core::f32::consts::FRAC_1_SQRT_2);
    put_f32(&mut bytes, 268, 10.0);
    put_f32(&mut bytes, 272, 20.0);
    put_f32(&mut bytes, 276, 30.0);

    let result = parse(&bytes);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    let transform = parsed.index_to_world();
    // b=c=0, d=sqrt(1/2) encodes +90 degrees about RAS z. The NIfTI
    // affine sends the i basis to +A by 2 mm and j to -R by 3 mm.
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 0.0)),
        (-10.0, -20.0, 30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(1.0, 0.0, 0.0)),
        (-10.0, -22.0, 30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 1.0, 0.0)),
        (-7.0, -20.0, 30.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 1.0)),
        (-10.0, -20.0, 34.0),
    );
}

#[test]
fn qform_mixed_terms_rotate_about_a_non_axis_direction() {
    let mut bytes = fixture(NiftiDataType::U8);
    put_f32(&mut bytes, 76, -1.0);
    put_f32(&mut bytes, 80, 2.0);
    put_f32(&mut bytes, 84, 3.0);
    put_f32(&mut bytes, 88, 4.0);
    put_f32(&mut bytes, 256, 0.5);
    put_f32(&mut bytes, 260, 0.5);
    put_f32(&mut bytes, 264, 0.5);
    put_f32(&mut bytes, 268, 11.0);
    put_f32(&mut bytes, 272, -13.0);
    put_f32(&mut bytes, 276, 17.0);

    let result = parse(&bytes);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    // a=b=c=d=1/2 is a 120 degree rotation about the non-axis direction
    // (1,1,1), with the hand-calculated RAS rotation rows
    // [0,0,1], [1,0,0], [0,1,0]. Unequal spacing and qfac=-1 make every
    // mixed quaternion column independently observable after LPS conversion.
    let transform = parsed.index_to_world();
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 0.0)),
        (-11.0, 13.0, 17.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(1.0, 0.0, 0.0)),
        (-11.0, 11.0, 17.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 1.0, 0.0)),
        (-11.0, 13.0, 20.0),
    );
    assert_world(
        transform.apply(Pt::<Index>::new(0.0, 0.0, 1.0)),
        (-7.0, 13.0, 17.0),
    );
}

#[test]
fn envelope_refusals_are_distinct() {
    assert_eq!(error(&[]), Some(NiftiError::TruncatedHeader));
    assert_eq!(error(&[0x1f, 0x8b]), Some(NiftiError::GzipUnsupported));

    let mut nifti2 = vec![0; 12];
    put_i32(&mut nifti2, 0, 540);
    put(&mut nifti2, 4, *b"n+2\0\r\n\x1a\n");
    assert_eq!(error(&nifti2), Some(NiftiError::Nifti2Unsupported));

    put(&mut nifti2, 4, *b"ni2\0\r\n\x1a\n");
    assert_eq!(error(&nifti2), Some(NiftiError::Nifti2Unsupported));

    let mut swapped_nifti2 = vec![0; 12];
    put(&mut swapped_nifti2, 0, 540_i32.to_be_bytes());
    put(&mut swapped_nifti2, 4, *b"n+2\0\r\n\x1a\n");
    assert_eq!(error(&swapped_nifti2), Some(NiftiError::Nifti2Unsupported));

    let mut truncated_nifti2_magic = vec![0; 4];
    put_i32(&mut truncated_nifti2_magic, 0, 540);
    assert_eq!(
        error(&truncated_nifti2_magic),
        Some(NiftiError::TruncatedHeader)
    );

    let mut wrong_nifti2_magic = fixture(NiftiDataType::U8);
    put_i32(&mut wrong_nifti2_magic, 0, 540);
    put(&mut wrong_nifti2_magic, 4, *b"not-two!");
    assert_eq!(
        error(&wrong_nifti2_magic),
        Some(NiftiError::InvalidHeaderSize)
    );

    let mut paired = fixture(NiftiDataType::U8);
    put(&mut paired, 344, *b"ni1\0");
    assert_eq!(error(&paired), Some(NiftiError::PairedFileUnsupported));

    let mut extension = fixture(NiftiDataType::U8);
    if let Some(flag) = extension.get_mut(348) {
        *flag = 1;
    }
    assert_eq!(error(&extension), Some(NiftiError::ExtensionUnsupported));

    let mut big_endian = fixture(NiftiDataType::U8);
    put(&mut big_endian, 0, 348_i32.to_be_bytes());
    put_i16_be(&mut big_endian, 40, 3);
    put_i16_be(&mut big_endian, 42, 2);
    put_i16_be(&mut big_endian, 44, 3);
    put_i16_be(&mut big_endian, 46, 2);
    assert_eq!(error(&big_endian), Some(NiftiError::BigEndianUnsupported));

    let fixture = fixture(NiftiDataType::U8);
    assert_eq!(
        error(fixture.get(..351).unwrap_or_default()),
        Some(NiftiError::TruncatedHeader)
    );

    let mut wrong_size = fixture.clone();
    put_i32(&mut wrong_size, 0, 347);
    assert_eq!(error(&wrong_size), Some(NiftiError::InvalidHeaderSize));

    let mut wrong_magic = fixture;
    put(&mut wrong_magic, 344, *b"BAD\0");
    assert_eq!(error(&wrong_magic), Some(NiftiError::InvalidMagic));
}

#[test]
fn datatype_and_bit_width_refusals_are_distinct() {
    let mut unsupported = fixture(NiftiDataType::U8);
    put_i16(&mut unsupported, 70, 8);
    put_i16(&mut unsupported, 72, 32);
    assert_eq!(error(&unsupported), Some(NiftiError::UnsupportedDataType));

    let mut mismatch = fixture(NiftiDataType::U16);
    put_i16(&mut mismatch, 72, 8);
    assert_eq!(error(&mismatch), Some(NiftiError::DataTypeBitWidthMismatch));
}

#[test]
fn rank_and_dimension_refusals_are_distinct() {
    let mut rank = fixture(NiftiDataType::U8);
    put_i16(&mut rank, 40, 2);
    assert_eq!(error(&rank), Some(NiftiError::InvalidRank));

    let mut dimension = fixture(NiftiDataType::U8);
    put_i16(&mut dimension, 44, 0);
    assert_eq!(error(&dimension), Some(NiftiError::InvalidDimension));

    let mut higher = fixture(NiftiDataType::U8);
    put_i16(&mut higher, 40, 5);
    put_i16(&mut higher, 48, 1);
    put_i16(&mut higher, 50, 2);
    assert_eq!(
        error(&higher),
        Some(NiftiError::NonSingletonHigherDimension)
    );
}

#[test]
fn geometry_units_and_affine_code_refusals_are_distinct() {
    let mut spacing = fixture(NiftiDataType::U8);
    put_f32(&mut spacing, 84, f32::NAN);
    assert_eq!(error(&spacing), Some(NiftiError::InvalidSpatialSpacing));

    let mut units = fixture(NiftiDataType::U8);
    if let Some(field) = units.get_mut(123) {
        *field = 1;
    }
    assert_eq!(error(&units), Some(NiftiError::UnsupportedSpatialUnits));

    let mut no_affine = fixture(NiftiDataType::U8);
    put_i16(&mut no_affine, 252, 0);
    assert_eq!(error(&no_affine), Some(NiftiError::MissingAffine));

    let mut negative_qform = fixture(NiftiDataType::U8);
    put_i16(&mut negative_qform, 252, -1);
    assert_eq!(error(&negative_qform), Some(NiftiError::InvalidAffineCode));

    let mut negative_sform = fixture(NiftiDataType::U8);
    put_i16(&mut negative_sform, 254, -1);
    assert_eq!(error(&negative_sform), Some(NiftiError::InvalidAffineCode));
}

#[test]
fn qform_validation_refuses_invalid_quaternion_and_qfac() {
    let mut quaternion = fixture(NiftiDataType::U8);
    put_f32(&mut quaternion, 256, 0.8);
    put_f32(&mut quaternion, 260, 0.8);
    assert_eq!(error(&quaternion), Some(NiftiError::InvalidQuaternion));

    let mut qfac = fixture(NiftiDataType::U8);
    put_f32(&mut qfac, 76, 2.0);
    assert_eq!(error(&qfac), Some(NiftiError::InvalidQfac));

    let mut nonfinite = fixture(NiftiDataType::U8);
    put_f32(&mut nonfinite, 256, f32::NAN);
    assert_eq!(error(&nonfinite), Some(NiftiError::NonFiniteSelectedAffine));
}

#[test]
fn only_the_selected_affine_is_required_to_be_finite_and_invertible() {
    let mut unselected_sform = fixture(NiftiDataType::U8);
    put_f32(&mut unselected_sform, 280, f32::NAN);
    let result = parse(&unselected_sform);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    assert!(
        parsed
            .header()
            .sform()
            .rows()
            .first()
            .and_then(|row| row.first())
            .is_some_and(|value| value.is_nan())
    );

    let mut unselected_qform = fixture(NiftiDataType::U8);
    put_f32(&mut unselected_qform, 256, f32::NAN);
    put_i16(&mut unselected_qform, 254, 1);
    put_f32(&mut unselected_qform, 280, 1.0);
    put_f32(&mut unselected_qform, 300, 1.0);
    put_f32(&mut unselected_qform, 320, 1.0);
    assert!(parse(&unselected_qform).is_ok());

    let mut nonfinite = unselected_qform.clone();
    put_f32(&mut nonfinite, 280, f32::INFINITY);
    assert_eq!(error(&nonfinite), Some(NiftiError::NonFiniteSelectedAffine));

    let mut singular = unselected_qform;
    put_f32(&mut singular, 280, 0.0);
    assert_eq!(error(&singular), Some(NiftiError::SingularSelectedAffine));
}

#[test]
fn scaling_is_retained_even_when_not_finite() {
    let mut bytes = fixture(NiftiDataType::U8);
    put_f32(&mut bytes, 112, f32::NAN);
    put_f32(&mut bytes, 116, f32::INFINITY);
    let result = parse(&bytes);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    assert!(parsed.header().scaling().slope().is_nan());
    assert!(parsed.header().scaling().intercept().is_infinite());
}

#[test]
fn payload_offset_and_range_rules_are_exact() {
    let mut negative_zero = fixture(NiftiDataType::U8);
    put_f32(&mut negative_zero, 108, -0.0);
    assert_eq!(
        error(&negative_zero),
        Some(NiftiError::VoxelOffsetBeforeData)
    );

    let mut before = fixture(NiftiDataType::U8);
    put_f32(&mut before, 108, 351.0);
    assert_eq!(error(&before), Some(NiftiError::VoxelOffsetBeforeData));

    let mut fractional = fixture(NiftiDataType::U8);
    put_f32(&mut fractional, 108, 352.5);
    assert_eq!(error(&fractional), Some(NiftiError::InvalidVoxelOffset));

    let mut nonfinite = fixture(NiftiDataType::U8);
    put_f32(&mut nonfinite, 108, f32::INFINITY);
    assert_eq!(error(&nonfinite), Some(NiftiError::InvalidVoxelOffset));

    let mut unrepresentable = fixture(NiftiDataType::U8);
    put_f32(&mut unrepresentable, 108, f32::MAX);
    assert_eq!(
        error(&unrepresentable),
        Some(NiftiError::ArithmeticOverflow)
    );

    let mut two_to_64 = fixture(NiftiDataType::U8);
    put_f32(&mut two_to_64, 108, f32::from_bits(0x5f80_0000));
    assert_eq!(error(&two_to_64), Some(NiftiError::ArithmeticOverflow));

    let mut above_two_to_64 = fixture(NiftiDataType::U8);
    put_f32(&mut above_two_to_64, 108, f32::from_bits(0x5f80_0001));
    assert_eq!(
        error(&above_two_to_64),
        Some(NiftiError::ArithmeticOverflow)
    );

    let mut short = fixture(NiftiDataType::F32);
    short.pop();
    assert_eq!(error(&short), Some(NiftiError::TruncatedPayload));

    let mut offset_353 = fixture_with_dims(NiftiDataType::U8, DIMS, 353);
    if let Some(reserved) = offset_353.get_mut(349) {
        *reserved = 7;
    }
    offset_353.extend_from_slice(&[91, 92, 93]);
    let result = parse(&offset_353);
    assert!(result.is_ok());
    let Ok(parsed) = result else {
        return;
    };
    assert_eq!(parsed.payload_range(), 353..365);
}

proptest! {
    #[test]
    fn exact_payload_product_accepts_complete_and_refuses_one_byte_short(
        x in 1_u16..9,
        y in 1_u16..9,
        z in 1_u16..9,
        kind in 0_u8..4,
    ) {
        let data_type = match kind {
            0 => NiftiDataType::U8,
            1 => NiftiDataType::I16,
            2 => NiftiDataType::U16,
            _ => NiftiDataType::F32,
        };
        let mut bytes = fixture_with_dims(data_type, [x, y, z], HEADER_AND_EXTENDER);
        prop_assert!(parse(&bytes).is_ok());
        bytes.pop();
        prop_assert_eq!(error(&bytes), Some(NiftiError::TruncatedPayload));
    }
}
