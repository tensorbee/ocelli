//! Direct NIfTI-1.1 single-file ingest.
//!
//! Field offsets, datatype codes, dimensions, byte ordering, scaling, and
//! affine formulas come from the official NIfTI-1 `nifti1.h` definition:
//! <https://github.com/NIFTI-Imaging/nifti_clib/blob/master/niftilib/nifti1.h>.
//! NIfTI-2 detection comes from its official 540-byte header definition:
//! <https://github.com/NIFTI-Imaging/nifti_clib/blob/master/nifti2/nifti2.h>.

use core::{num::FpCategory, ops::Range};

use glam::{DMat4, DVec4};
use ocelli_core::{Index, Transform, World};

const HEADER_LENGTH: usize = 348;
const HEADER_AND_EXTENDER: usize = 352;
const NIFTI1_HEADER_SIZE: i32 = 348;
const NIFTI2_HEADER_SIZE: i32 = 540;
const NIFTI1_SINGLE_FILE_MAGIC: &[u8; 4] = b"n+1\0";
const NIFTI1_PAIRED_MAGIC: &[u8; 4] = b"ni1\0";
const NIFTI2_SINGLE_FILE_MAGIC: &[u8; 8] = b"n+2\0\r\n\x1a\n";
const NIFTI2_PAIRED_MAGIC: &[u8; 8] = b"ni2\0\r\n\x1a\n";
const GZIP_MAGIC: &[u8; 2] = &[0x1f, 0x8b];
const SPATIAL_UNITS_MASK: u8 = 0x07;
const NIFTI_UNITS_MM: u8 = 2;

/// The byte order validated for this ingest result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NiftiByteOrder {
    /// Little endian, the only F-022 input byte order.
    LittleEndian,
}

/// The supported NIfTI scalar datatype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NiftiDataType {
    /// Unsigned 8-bit integer, NIfTI datatype 2.
    U8,
    /// Signed 16-bit integer, NIfTI datatype 4.
    I16,
    /// Unsigned 16-bit integer, NIfTI datatype 512.
    U16,
    /// IEEE-754 32-bit float, NIfTI datatype 16.
    F32,
}

impl NiftiDataType {
    const fn bytes_per_voxel(self) -> u32 {
        match self {
            Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::F32 => 4,
        }
    }
}

/// Validated spatial units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NiftiSpatialUnits {
    /// Millimetres, NIfTI spatial unit code 2.
    Millimetres,
}

/// The raw NIfTI scaling declaration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NiftiScaling {
    slope: f32,
    intercept: f32,
}

impl NiftiScaling {
    /// The raw `scl_slope`. Zero means scaling is inactive.
    #[must_use]
    pub const fn slope(self) -> f32 {
        self.slope
    }

    /// The raw `scl_inter`, retained even when the slope is zero.
    #[must_use]
    pub const fn intercept(self) -> f32 {
        self.intercept
    }
}

/// The raw qform declaration, before validation or coordinate conversion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NiftiQForm {
    code: i16,
    quaternion: [f32; 3],
    offset: [f32; 3],
    qfac: f32,
}

impl NiftiQForm {
    /// The raw signed qform code. Only positive codes are active.
    #[must_use]
    pub const fn code(self) -> i16 {
        self.code
    }

    /// The raw `(b, c, d)` quaternion fields.
    #[must_use]
    pub const fn quaternion(self) -> [f32; 3] {
        self.quaternion
    }

    /// The raw qform RAS translation.
    #[must_use]
    pub const fn offset(self) -> [f32; 3] {
        self.offset
    }

    /// The raw `pixdim[0]`. Zero remains zero in this evidence.
    #[must_use]
    pub const fn qfac(self) -> f32 {
        self.qfac
    }
}

/// The raw sform declaration, before validation or coordinate conversion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NiftiSForm {
    code: i16,
    rows: [[f32; 4]; 3],
}

impl NiftiSForm {
    /// The raw signed sform code. Only positive codes are active.
    #[must_use]
    pub const fn code(self) -> i16 {
        self.code
    }

    /// The three raw RAS affine rows.
    #[must_use]
    pub const fn rows(self) -> [[f32; 4]; 3] {
        self.rows
    }
}

/// The declaration selected to produce the internal LPS transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NiftiAffineSource {
    /// The general affine sform was active and took precedence.
    SForm,
    /// No sform was active, so the quaternion qform was selected.
    QForm,
}

/// Immutable validated NIfTI header evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NiftiHeader {
    byte_order: NiftiByteOrder,
    data_type: NiftiDataType,
    dimensions: [u32; 3],
    declared_rank: u8,
    spatial_units: NiftiSpatialUnits,
    voxel_spacing_mm: [f32; 3],
    scaling: NiftiScaling,
    qform: NiftiQForm,
    sform: NiftiSForm,
}

impl NiftiHeader {
    /// The validated byte order.
    #[must_use]
    pub const fn byte_order(&self) -> NiftiByteOrder {
        self.byte_order
    }

    /// The exact supported scalar datatype.
    #[must_use]
    pub const fn data_type(&self) -> NiftiDataType {
        self.data_type
    }

    /// The three positive spatial dimensions in x-fastest order.
    #[must_use]
    pub const fn dimensions(&self) -> [u32; 3] {
        self.dimensions
    }

    /// The raw declared rank, from three through seven.
    #[must_use]
    pub const fn declared_rank(&self) -> u8 {
        self.declared_rank
    }

    /// The validated spatial units.
    #[must_use]
    pub const fn spatial_units(&self) -> NiftiSpatialUnits {
        self.spatial_units
    }

    /// The positive spatial `pixdim[1..=3]` values in millimetres.
    #[must_use]
    pub const fn voxel_spacing_mm(&self) -> [f32; 3] {
        self.voxel_spacing_mm
    }

    /// The raw scaling declaration, which this parser does not apply.
    #[must_use]
    pub const fn scaling(&self) -> NiftiScaling {
        self.scaling
    }

    /// The raw qform declaration, selected or unselected.
    #[must_use]
    pub const fn qform(&self) -> NiftiQForm {
        self.qform
    }

    /// The raw sform declaration, selected or unselected.
    #[must_use]
    pub const fn sform(&self) -> NiftiSForm {
        self.sform
    }
}

/// Validated NIfTI header, selected geometry, and exact payload range.
#[derive(Debug, Clone)]
pub struct ParsedNifti {
    header: NiftiHeader,
    payload_range: Range<usize>,
    selected_affine_source: NiftiAffineSource,
    index_to_world: Transform<Index, World>,
}

impl ParsedNifti {
    /// The immutable validated header evidence.
    #[must_use]
    pub const fn header(&self) -> &NiftiHeader {
        &self.header
    }

    /// The exact byte range occupied by the supported scalar payload.
    #[must_use]
    pub fn payload_range(&self) -> Range<usize> {
        self.payload_range.clone()
    }

    /// Which active affine declaration produced the geometry.
    #[must_use]
    pub const fn selected_affine_source(&self) -> NiftiAffineSource {
        self.selected_affine_source
    }

    /// Map zero-based NIfTI voxel indices into DICOM LPS world millimetres.
    #[must_use]
    pub const fn index_to_world(&self) -> Transform<Index, World> {
        self.index_to_world
    }
}

/// A NIfTI ingest refusal that retains no source bytes or free text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum NiftiError {
    /// Input begins with the gzip signature.
    #[error("gzip-compressed NIfTI input is unsupported")]
    GzipUnsupported,
    /// Input identifies the NIfTI-2 540-byte header.
    #[error("NIfTI-2 input is unsupported")]
    Nifti2Unsupported,
    /// Input cannot contain the complete NIfTI-1 header and extender.
    #[error("NIfTI-1 input ends before the 352-byte header and extender")]
    TruncatedHeader,
    /// A byte-swapped NIfTI-1 header was detected.
    #[error("big-endian NIfTI-1 input is unsupported")]
    BigEndianUnsupported,
    /// The NIfTI-1 `sizeof_hdr` field is not 348.
    #[error("NIfTI-1 header size is invalid")]
    InvalidHeaderSize,
    /// The magic selects paired `.hdr` and `.img` storage.
    #[error("paired-file NIfTI-1 input is unsupported")]
    PairedFileUnsupported,
    /// The header lacks an exact NIfTI-1 single-file magic.
    #[error("NIfTI-1 single-file magic is invalid")]
    InvalidMagic,
    /// The NIfTI extension flag is active.
    #[error("NIfTI header extensions are unsupported")]
    ExtensionUnsupported,
    /// The datatype is outside the four supported scalar kinds.
    #[error("NIfTI datatype is unsupported")]
    UnsupportedDataType,
    /// The bit width does not match the declared supported datatype.
    #[error("NIfTI datatype and bit width do not match")]
    DataTypeBitWidthMismatch,
    /// The declared rank is outside three through seven.
    #[error("NIfTI rank is invalid for a three-dimensional volume")]
    InvalidRank,
    /// A spatial dimension is not positive.
    #[error("NIfTI spatial dimension is invalid")]
    InvalidDimension,
    /// A declared dimension above the three spatial axes is not one.
    #[error("NIfTI higher dimensions must be singleton")]
    NonSingletonHigherDimension,
    /// A spatial voxel width is non-finite or not positive.
    #[error("NIfTI spatial voxel width is invalid")]
    InvalidSpatialSpacing,
    /// The spatial unit bits do not select millimetres.
    #[error("NIfTI spatial units must be millimetres")]
    UnsupportedSpatialUnits,
    /// A qform or sform code is negative.
    #[error("NIfTI affine codes cannot be negative")]
    InvalidAffineCode,
    /// Neither qform nor sform is active.
    #[error("NIfTI input has no active affine")]
    MissingAffine,
    /// The active qform quaternion cannot define the required unit rotation.
    #[error("NIfTI qform quaternion is invalid")]
    InvalidQuaternion,
    /// The active qform qfac is not -1, 0, or 1.
    #[error("NIfTI qform qfac is invalid")]
    InvalidQfac,
    /// A component or determinant of the selected affine is non-finite.
    #[error("the selected NIfTI affine is non-finite")]
    NonFiniteSelectedAffine,
    /// The selected affine cannot be inverted.
    #[error("the selected NIfTI affine is singular")]
    SingularSelectedAffine,
    /// `vox_offset` is non-finite, fractional, negative, or otherwise invalid.
    #[error("NIfTI voxel offset is invalid")]
    InvalidVoxelOffset,
    /// Ocelli requires the payload to start after the header and extender.
    #[error("NIfTI voxel offset is before byte 352")]
    VoxelOffsetBeforeData,
    /// A validated dimension, byte length, offset, or range exceeds its type.
    #[error("NIfTI payload range arithmetic overflowed")]
    ArithmeticOverflow,
    /// The declared payload range extends beyond the input.
    #[error("NIfTI voxel payload is truncated")]
    TruncatedPayload,
}

/// Parse and validate one uncompressed little-endian single-file NIfTI-1.1
/// input entirely in memory.
///
/// The returned payload range borrows no input and the raw bytes are neither
/// copied nor scaled. The selected affine is the only declaration interpreted
/// as geometry, and it is converted from NIfTI RAS to DICOM LPS before it is
/// wrapped in [`Transform<Index, World>`].
///
/// # Errors
///
/// Returns a distinct [`NiftiError`] for unsupported containers, invalid
/// header declarations, invalid selected geometry, arithmetic overflow, or a
/// payload that does not fit inside `bytes`.
pub fn parse_nifti(bytes: &[u8]) -> Result<ParsedNifti, NiftiError> {
    validate_envelope(bytes)?;

    let rank = read_i16_le(bytes, 40)?;
    if !(3..=7).contains(&rank) {
        return Err(NiftiError::InvalidRank);
    }
    let declared_rank = u8::try_from(rank).map_err(|_| NiftiError::InvalidRank)?;

    let dimensions_i16 = [
        read_i16_le(bytes, 42)?,
        read_i16_le(bytes, 44)?,
        read_i16_le(bytes, 46)?,
    ];
    if dimensions_i16.iter().any(|dimension| *dimension <= 0) {
        return Err(NiftiError::InvalidDimension);
    }
    let dimensions = [
        u32::try_from(dimensions_i16[0]).map_err(|_| NiftiError::InvalidDimension)?,
        u32::try_from(dimensions_i16[1]).map_err(|_| NiftiError::InvalidDimension)?,
        u32::try_from(dimensions_i16[2]).map_err(|_| NiftiError::InvalidDimension)?,
    ];
    validate_higher_dimensions(bytes, declared_rank)?;

    let datatype_code = read_i16_le(bytes, 70)?;
    let bitpix = read_i16_le(bytes, 72)?;
    let data_type = parse_data_type(datatype_code, bitpix)?;

    let qfac = read_f32_le(bytes, 76)?;
    let voxel_spacing_mm = [
        read_f32_le(bytes, 80)?,
        read_f32_le(bytes, 84)?,
        read_f32_le(bytes, 88)?,
    ];
    if voxel_spacing_mm
        .iter()
        .any(|spacing| !spacing.is_finite() || *spacing <= 0.0)
    {
        return Err(NiftiError::InvalidSpatialSpacing);
    }

    let units = *bytes.get(123).ok_or(NiftiError::TruncatedHeader)?;
    if units & SPATIAL_UNITS_MASK != NIFTI_UNITS_MM {
        return Err(NiftiError::UnsupportedSpatialUnits);
    }

    let qform = NiftiQForm {
        code: read_i16_le(bytes, 252)?,
        quaternion: [
            read_f32_le(bytes, 256)?,
            read_f32_le(bytes, 260)?,
            read_f32_le(bytes, 264)?,
        ],
        offset: [
            read_f32_le(bytes, 268)?,
            read_f32_le(bytes, 272)?,
            read_f32_le(bytes, 276)?,
        ],
        qfac,
    };
    let sform = NiftiSForm {
        code: read_i16_le(bytes, 254)?,
        rows: [
            read_f32_row(bytes, 280)?,
            read_f32_row(bytes, 296)?,
            read_f32_row(bytes, 312)?,
        ],
    };
    if qform.code < 0 || sform.code < 0 {
        return Err(NiftiError::InvalidAffineCode);
    }

    let (selected_affine_source, ras_affine) = if sform.code > 0 {
        (NiftiAffineSource::SForm, sform_matrix(sform))
    } else if qform.code > 0 {
        (
            NiftiAffineSource::QForm,
            qform_matrix(qform, voxel_spacing_mm)?,
        )
    } else {
        return Err(NiftiError::MissingAffine);
    };
    validate_affine(ras_affine)?;

    let lps_from_ras = DMat4::from_diagonal(DVec4::new(-1.0, -1.0, 1.0, 1.0));
    let lps_affine = lps_from_ras * ras_affine;
    validate_affine(lps_affine)?;

    let voxel_offset = exact_usize_from_f32(read_f32_le(bytes, 108)?)?;
    if voxel_offset < HEADER_AND_EXTENDER {
        return Err(NiftiError::VoxelOffsetBeforeData);
    }
    let payload_length = checked_payload_length::<usize>(dimensions, data_type.bytes_per_voxel())
        .ok_or(NiftiError::ArithmeticOverflow)?;
    let payload_end = voxel_offset
        .checked_add(payload_length)
        .ok_or(NiftiError::ArithmeticOverflow)?;
    if payload_end > bytes.len() {
        return Err(NiftiError::TruncatedPayload);
    }

    Ok(ParsedNifti {
        header: NiftiHeader {
            byte_order: NiftiByteOrder::LittleEndian,
            data_type,
            dimensions,
            declared_rank,
            spatial_units: NiftiSpatialUnits::Millimetres,
            voxel_spacing_mm,
            scaling: NiftiScaling {
                slope: read_f32_le(bytes, 112)?,
                intercept: read_f32_le(bytes, 116)?,
            },
            qform,
            sform,
        },
        payload_range: voxel_offset..payload_end,
        selected_affine_source,
        index_to_world: Transform::from_mat4(lps_affine),
    })
}

fn validate_envelope(bytes: &[u8]) -> Result<(), NiftiError> {
    if bytes.starts_with(GZIP_MAGIC) {
        return Err(NiftiError::GzipUnsupported);
    }
    let size_bytes = read_array::<4>(bytes, 0)?;
    let little_size = i32::from_le_bytes(size_bytes);
    let big_size = i32::from_be_bytes(size_bytes);
    if little_size == NIFTI2_HEADER_SIZE || big_size == NIFTI2_HEADER_SIZE {
        let magic = bytes.get(4..12).ok_or(NiftiError::TruncatedHeader)?;
        if magic == NIFTI2_SINGLE_FILE_MAGIC || magic == NIFTI2_PAIRED_MAGIC {
            return Err(NiftiError::Nifti2Unsupported);
        }
    }
    if bytes.len() < HEADER_AND_EXTENDER {
        return Err(NiftiError::TruncatedHeader);
    }

    let dimension_bytes = read_array::<2>(bytes, 40)?;
    let little_rank = i16::from_le_bytes(dimension_bytes);
    let big_rank = i16::from_be_bytes(dimension_bytes);
    let swapped_rank = !(1..=7).contains(&little_rank) && (1..=7).contains(&big_rank);
    if big_size == NIFTI1_HEADER_SIZE && swapped_rank {
        return Err(NiftiError::BigEndianUnsupported);
    }
    if little_size != NIFTI1_HEADER_SIZE {
        return Err(NiftiError::InvalidHeaderSize);
    }

    let magic = bytes
        .get(344..HEADER_LENGTH)
        .ok_or(NiftiError::TruncatedHeader)?;
    if magic == NIFTI1_PAIRED_MAGIC {
        return Err(NiftiError::PairedFileUnsupported);
    }
    if magic != NIFTI1_SINGLE_FILE_MAGIC {
        return Err(NiftiError::InvalidMagic);
    }
    if bytes.get(HEADER_LENGTH).copied().unwrap_or_default() != 0 {
        return Err(NiftiError::ExtensionUnsupported);
    }
    Ok(())
}

fn validate_higher_dimensions(bytes: &[u8], rank: u8) -> Result<(), NiftiError> {
    for dimension_index in 4_u8..=rank {
        let offset = 40_usize
            .checked_add(
                usize::from(dimension_index)
                    .checked_mul(2)
                    .ok_or(NiftiError::ArithmeticOverflow)?,
            )
            .ok_or(NiftiError::ArithmeticOverflow)?;
        if read_i16_le(bytes, offset)? != 1 {
            return Err(NiftiError::NonSingletonHigherDimension);
        }
    }
    Ok(())
}

fn parse_data_type(code: i16, bitpix: i16) -> Result<NiftiDataType, NiftiError> {
    let (data_type, required_bitpix) = match code {
        2 => (NiftiDataType::U8, 8),
        4 => (NiftiDataType::I16, 16),
        16 => (NiftiDataType::F32, 32),
        512 => (NiftiDataType::U16, 16),
        _ => return Err(NiftiError::UnsupportedDataType),
    };
    if bitpix != required_bitpix {
        return Err(NiftiError::DataTypeBitWidthMismatch);
    }
    Ok(data_type)
}

fn qform_matrix(qform: NiftiQForm, spacing: [f32; 3]) -> Result<DMat4, NiftiError> {
    let [b_raw, c_raw, d_raw] = qform.quaternion;
    let [qx_raw, qy_raw, qz_raw] = qform.offset;
    if [b_raw, c_raw, d_raw, qx_raw, qy_raw, qz_raw, qform.qfac]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(NiftiError::NonFiniteSelectedAffine);
    }

    let qfac = match qform.qfac.to_bits() {
        bits if bits == (-1.0_f32).to_bits() => -1.0,
        bits if bits == 1.0_f32.to_bits() => 1.0,
        _ if qform.qfac.classify() == FpCategory::Zero => 1.0,
        _ => return Err(NiftiError::InvalidQfac),
    };

    let b = f64::from(b_raw);
    let c = f64::from(c_raw);
    let d = f64::from(d_raw);
    let squared_vector = b * b + c * c + d * d;
    if squared_vector > 1.0 {
        return Err(NiftiError::InvalidQuaternion);
    }
    let a = (1.0 - squared_vector).sqrt();
    let aa = a * a;
    let bb = b * b;
    let cc = c * c;
    let dd = d * d;
    let two = 2.0;

    let r11 = aa + bb - cc - dd;
    let r12 = two * b * c - two * a * d;
    let r13 = two * b * d + two * a * c;
    let r21 = two * b * c + two * a * d;
    let r22 = aa + cc - bb - dd;
    let r23 = two * c * d - two * a * b;
    let r31 = two * b * d - two * a * c;
    let r32 = two * c * d + two * a * b;
    let r33 = aa + dd - cc - bb;

    let dx = f64::from(spacing[0]);
    let dy = f64::from(spacing[1]);
    let dz = f64::from(spacing[2]) * qfac;
    Ok(DMat4::from_cols(
        DVec4::new(r11 * dx, r21 * dx, r31 * dx, 0.0),
        DVec4::new(r12 * dy, r22 * dy, r32 * dy, 0.0),
        DVec4::new(r13 * dz, r23 * dz, r33 * dz, 0.0),
        DVec4::new(f64::from(qx_raw), f64::from(qy_raw), f64::from(qz_raw), 1.0),
    ))
}

fn sform_matrix(sform: NiftiSForm) -> DMat4 {
    let [x, y, z] = sform.rows;
    DMat4::from_cols(
        DVec4::new(f64::from(x[0]), f64::from(y[0]), f64::from(z[0]), 0.0),
        DVec4::new(f64::from(x[1]), f64::from(y[1]), f64::from(z[1]), 0.0),
        DVec4::new(f64::from(x[2]), f64::from(y[2]), f64::from(z[2]), 0.0),
        DVec4::new(f64::from(x[3]), f64::from(y[3]), f64::from(z[3]), 1.0),
    )
}

fn validate_affine(matrix: DMat4) -> Result<(), NiftiError> {
    if !matrix.is_finite() {
        return Err(NiftiError::NonFiniteSelectedAffine);
    }
    let determinant = matrix.determinant();
    if !determinant.is_finite() {
        return Err(NiftiError::NonFiniteSelectedAffine);
    }
    if determinant.classify() == FpCategory::Zero {
        return Err(NiftiError::SingularSelectedAffine);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OffsetConversionError {
    Invalid,
    Overflow,
}

fn exact_usize_from_f32(value: f32) -> Result<usize, NiftiError> {
    exact_u64_from_f32(value)
        .map_err(|error| match error {
            OffsetConversionError::Invalid => NiftiError::InvalidVoxelOffset,
            OffsetConversionError::Overflow => NiftiError::ArithmeticOverflow,
        })
        .and_then(|value| usize::try_from(value).map_err(|_| NiftiError::ArithmeticOverflow))
}

fn exact_u64_from_f32(value: f32) -> Result<u64, OffsetConversionError> {
    if value.classify() == FpCategory::Zero {
        return Ok(0);
    }
    if !value.is_finite() || value.is_sign_negative() {
        return Err(OffsetConversionError::Invalid);
    }

    let bits = value.to_bits();
    let exponent_bits = (bits >> 23) & 0xff;
    let exponent = i32::try_from(exponent_bits).map_err(|_| OffsetConversionError::Overflow)? - 127;
    if exponent < 0 {
        return Err(OffsetConversionError::Invalid);
    }
    let significand = u64::from((bits & 0x007f_ffff) | 0x0080_0000);
    if exponent >= 23 {
        let shift = u32::try_from(exponent - 23).map_err(|_| OffsetConversionError::Overflow)?;
        let maximum_significand = u64::MAX
            .checked_shr(shift)
            .ok_or(OffsetConversionError::Overflow)?;
        if significand > maximum_significand {
            return Err(OffsetConversionError::Overflow);
        }
        Ok(significand << shift)
    } else {
        let shift = u32::try_from(23 - exponent).map_err(|_| OffsetConversionError::Overflow)?;
        let fractional_mask = 1_u64
            .checked_shl(shift)
            .and_then(|value| value.checked_sub(1))
            .ok_or(OffsetConversionError::Overflow)?;
        if significand & fractional_mask != 0 {
            return Err(OffsetConversionError::Invalid);
        }
        Ok(significand >> shift)
    }
}

trait CheckedLength: Copy {
    fn from_u32(value: u32) -> Option<Self>;
    fn checked_multiply(self, other: Self) -> Option<Self>;
}

impl CheckedLength for usize {
    fn from_u32(value: u32) -> Option<Self> {
        Self::try_from(value).ok()
    }

    fn checked_multiply(self, other: Self) -> Option<Self> {
        self.checked_mul(other)
    }
}

impl CheckedLength for u32 {
    fn from_u32(value: u32) -> Option<Self> {
        Some(value)
    }

    fn checked_multiply(self, other: Self) -> Option<Self> {
        self.checked_mul(other)
    }
}

fn checked_payload_length<T: CheckedLength>(
    dimensions: [u32; 3],
    bytes_per_voxel: u32,
) -> Option<T> {
    let mut length = T::from_u32(bytes_per_voxel)?;
    for dimension in dimensions {
        length = length.checked_multiply(T::from_u32(dimension)?)?;
    }
    Some(length)
}

fn read_i16_le(bytes: &[u8], offset: usize) -> Result<i16, NiftiError> {
    read_array(bytes, offset).map(i16::from_le_bytes)
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Result<f32, NiftiError> {
    read_array(bytes, offset).map(f32::from_le_bytes)
}

fn read_f32_row(bytes: &[u8], offset: usize) -> Result<[f32; 4], NiftiError> {
    Ok([
        read_f32_le(bytes, offset)?,
        read_f32_le(bytes, offset + 4)?,
        read_f32_le(bytes, offset + 8)?,
        read_f32_le(bytes, offset + 12)?,
    ])
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], NiftiError> {
    let tail = bytes.get(offset..).ok_or(NiftiError::TruncatedHeader)?;
    let value = tail.get(..N).ok_or(NiftiError::TruncatedHeader)?;
    <[u8; N]>::try_from(value).map_err(|_| NiftiError::TruncatedHeader)
}

#[cfg(test)]
mod tests {
    use super::{OffsetConversionError, checked_payload_length, exact_u64_from_f32};

    #[test]
    fn checked_length_helper_refuses_a_32_bit_product_overflow() {
        assert_eq!(checked_payload_length::<u32>([32_767; 3], 4), None);
    }

    #[test]
    fn exact_u64_conversion_preserves_the_upper_boundary() {
        assert_eq!(exact_u64_from_f32(-0.0), Ok(0));
        assert_eq!(
            exact_u64_from_f32(f32::from_bits(0x5f7f_ffff)),
            Ok(0xffff_ff00_0000_0000)
        );
        assert_eq!(
            exact_u64_from_f32(f32::from_bits(0x5f80_0000)),
            Err(OffsetConversionError::Overflow)
        );
        assert_eq!(
            exact_u64_from_f32(f32::from_bits(0x5f80_0001)),
            Err(OffsetConversionError::Overflow)
        );
    }
}
