//! H.264 parameter sets, slice headers, picture order count and access units.
//!
//! Every syntax table and field width here is from Rec. ITU-T H.264 (V15)
//! (08/2024), the clause named beside each parser. Only the fields that lead
//! to the ones this module needs are read, and every field before those is
//! read at its specified width so the position is exact.

use std::ops::Range;

use super::{
    ColourDescription, ColourSource, FrameSize, VideoError,
    bits::{Bits, units},
    table::{AccessUnit, StreamInfo},
};

/// 7.4.2.1.1, seq_parameter_set_id is 0 to 31.
const SPS_IDS: usize = 32;
/// 7.4.2.2, pic_parameter_set_id is 0 to 255.
const PPS_IDS: usize = 256;
/// 7.4.2.1.1, log2_max_frame_num_minus4 and log2_max_pic_order_cnt_lsb_minus4
/// are 0 to 12.
const MAX_LOG2_MINUS4: u32 = 12;
/// 7.4.2.1.1, num_ref_frames_in_pic_order_cnt_cycle is 0 to 255.
const MAX_POC_CYCLE: u32 = 255;
/// 7.4.2.2, num_ref_idx_l0_default_active_minus1 and l1 are 0 to 31.
const MAX_REF_IDX: u32 = 31;
/// 7.4.3, num_ref_idx_l0_active_minus1 and l1 are 0 to 15 for a frame.
const MAX_FRAME_REF_IDX: u32 = 15;
/// The widest se(v) range a clause gives, -2^31 + 1 to 2^31 - 1, 7.4.2.1.1 and
/// 7.4.3. The reader stops at 31 leading zeros, so se(v) never leaves this
/// range and a check against it never refuses, the reader refusing first. It
/// is written out so each field states its clause's range.
const SE_LIMIT: i32 = i32::MAX;
/// 7-4, QpBdOffsetY = 6 * bit_depth_luma_minus8, at most 36 because
/// bit_depth_luma_minus8 is at most 6, 7.4.2.1.1.
const MAX_QP_BD_OFFSET: i32 = 36;
/// 7.4.2.1.1 bounds max_num_ref_frames by MaxDpbFrames, which A.3.1 item h
/// gives as Min( MaxDpbMbs / ( PicWidthInMbs * FrameHeightInMbs ), 16 ).
const MAX_DPB_FRAMES: u32 = 16;
/// A.3.1 items e to g at Table A-1's largest MaxFS, 139 264 macroblocks:
/// PicWidthInMbs and FrameHeightInMbs are at most Sqrt( MaxFS * 8 ), 1055,
/// and their product at most MaxFS.
const MAX_PIC_SIDE_IN_MBS: u32 = 1055;
const MAX_FRAME_SIZE_IN_MBS: u64 = 139_264;
/// Table E-1, aspect_ratio_idc 255 is Extended_SAR.
const EXTENDED_SAR: u8 = 255;

/// Table 7-1 nal_unit_type values this module acts on.
mod nal {
    pub(super) const SLICE: u8 = 1;
    pub(super) const PARTITION_A: u8 = 2;
    pub(super) const PARTITION_C: u8 = 4;
    pub(super) const IDR: u8 = 5;
    pub(super) const SEI: u8 = 6;
    pub(super) const SPS: u8 = 7;
    pub(super) const PPS: u8 = 8;
    pub(super) const AUD: u8 = 9;
    pub(super) const PREFIX_FIRST: u8 = 14;
    pub(super) const PREFIX_LAST: u8 = 18;
}

/// The picture order count parameters of 7.3.2.1.1.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PocType {
    Zero {
        log2_max_lsb: u8,
    },
    One {
        delta_always_zero: bool,
        offset_for_non_ref_pic: i32,
        offset_for_top_to_bottom_field: i32,
        offset_for_ref_frame: Vec<i32>,
    },
    Two,
}

/// The fields of one sequence parameter set this module uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sps {
    pub(crate) profile_idc: u8,
    /// constraint_set0_flag to constraint_set5_flag and reserved_zero_2bits.
    pub(crate) constraint_flags: u8,
    pub(crate) level_idc: u8,
    separate_colour_plane: bool,
    chroma_array_type: u32,
    bit_depth_luma_minus8: u32,
    log2_max_frame_num: u8,
    max_num_ref_frames: u32,
    poc: PocType,
    pub(crate) frame_mbs_only: bool,
    width_in_mbs: u64,
    frame_height_in_mbs: u64,
    pub(crate) coded_size: FrameSize,
    pub(crate) visible_size: FrameSize,
    /// The VUI video signal type, E.1.1.
    pub(crate) signal: Option<SignalType>,
}

/// video_signal_type_present_flag's fields, E.1.1, shared with H.265.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SignalType {
    pub(crate) full_range: bool,
    /// colour_primaries, transfer_characteristics and matrix_coefficients
    /// when colour_description_present_flag is 1.
    pub(crate) description: Option<(u8, u8, u8)>,
}

impl SignalType {
    /// The description as the VUI states it. E.2.1 infers 2, unspecified,
    /// for each code point when colour_description_present_flag is 0.
    pub(crate) fn colour(self) -> ColourDescription {
        let (primaries, transfer, matrix) = self.description.unwrap_or((2, 2, 2));
        ColourDescription {
            code_point_source: ColourSource::Vui,
            range_source: ColourSource::Vui,
            colour_primaries: primaries.into(),
            transfer_characteristics: transfer.into(),
            matrix_coefficients: matrix.into(),
            full_range: self.full_range,
        }
    }
}

impl Sps {
    /// RFC 6381 3.3: the hexadecimal profile_idc, constraint flags byte and
    /// level_idc after the sample entry type.
    pub(crate) fn codec_string(&self, prefix: &str) -> String {
        format!(
            "{prefix}.{:02X}{:02X}{:02X}",
            self.profile_idc, self.constraint_flags, self.level_idc
        )
    }

    pub(crate) fn stream_info(&self, prefix: &str) -> StreamInfo {
        StreamInfo {
            codec_string: Some(self.codec_string(prefix)),
            coded_size: self.coded_size,
            visible_size: self.visible_size,
            colour: self.signal.map(SignalType::colour),
            code_points_signalled: self
                .signal
                .is_some_and(|signal| signal.description.is_some()),
            // 7.4.2.1.1: frame_mbs_only_flag 0 says coded pictures may be
            // coded fields or coded frames.
            interlaced: !self.frame_mbs_only,
        }
    }
}

/// 7.3.2.1.1, seq_parameter_set_data, from a NAL unit including its header.
pub(crate) fn parse_sps(nal: &[u8]) -> Result<(usize, Sps), VideoError> {
    let mut bits = Bits::rbsp(
        nal.get(1..).unwrap_or_default(),
        "H.264 sequence parameter set",
    );
    let profile_idc = bits.byte()?;
    let constraint_flags = bits.byte()?;
    let level_idc = bits.byte()?;
    let id = bits.ue_max(31, "seq_parameter_set_id over 31")?;
    let mut chroma_format_idc = 1;
    let mut separate_colour_plane = false;
    let mut bit_depth_luma_minus8 = 0;
    if matches!(
        profile_idc,
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
    ) {
        chroma_format_idc = bits.ue_max(3, "chroma_format_idc over 3")?;
        if chroma_format_idc == 3 {
            separate_colour_plane = bits.flag()?;
        }
        bit_depth_luma_minus8 = bits.ue_max(6, "bit_depth_luma_minus8 over 6")?;
        bits.ue_max(6, "bit_depth_chroma_minus8 over 6")?;
        bits.flag()?; // qpprime_y_zero_transform_bypass_flag
        if bits.flag()? {
            // seq_scaling_matrix_present_flag
            let lists = if chroma_format_idc == 3 { 12 } else { 8 };
            for list in 0..lists {
                if bits.flag()? {
                    skip_scaling_list(&mut bits, if list < 6 { 16 } else { 64 })?;
                }
            }
        }
    }
    let log2_max_frame_num = bits.ue_max(MAX_LOG2_MINUS4, "log2_max_frame_num_minus4 over 12")? + 4;
    let poc = match bits.ue_max(2, "pic_order_cnt_type over 2")? {
        0 => PocType::Zero {
            log2_max_lsb: small(
                bits.ue_max(MAX_LOG2_MINUS4, "log2_max_pic_order_cnt_lsb_minus4 over 12")? + 4,
            )?,
        },
        1 => {
            let delta_always_zero = bits.flag()?;
            // 7.4.2.1.1: each offset is -2^31 + 1 to 2^31 - 1.
            let offset_for_non_ref_pic =
                bits.se_range(-SE_LIMIT, SE_LIMIT, "offset_for_non_ref_pic out of range")?;
            let offset_for_top_to_bottom_field = bits.se_range(
                -SE_LIMIT,
                SE_LIMIT,
                "offset_for_top_to_bottom_field out of range",
            )?;
            let cycle = bits.ue_max(
                MAX_POC_CYCLE,
                "num_ref_frames_in_pic_order_cnt_cycle over 255",
            )?;
            let mut offset_for_ref_frame = Vec::new();
            for _ in 0..cycle {
                offset_for_ref_frame.push(bits.se_range(
                    -SE_LIMIT,
                    SE_LIMIT,
                    "offset_for_ref_frame out of range",
                )?);
            }
            PocType::One {
                delta_always_zero,
                offset_for_non_ref_pic,
                offset_for_top_to_bottom_field,
                offset_for_ref_frame,
            }
        }
        _ => PocType::Two,
    };
    let max_num_ref_frames = bits.ue_max(MAX_DPB_FRAMES, "max_num_ref_frames over 16")?;
    bits.flag()?; // gaps_in_frame_num_value_allowed_flag
    let width_in_mbs =
        u64::from(bits.ue_max(MAX_PIC_SIDE_IN_MBS - 1, "pic_width_in_mbs_minus1 over 1054")?) + 1;
    let height_in_map_units = u64::from(bits.ue_max(
        MAX_PIC_SIDE_IN_MBS - 1,
        "pic_height_in_map_units_minus1 over 1054",
    )?) + 1;
    let frame_mbs_only = bits.flag()?;
    if !frame_mbs_only {
        bits.flag()?; // mb_adaptive_frame_field_flag
    }
    bits.flag()?; // direct_8x8_inference_flag
    let crop = if bits.flag()? {
        // Bounded together with the picture below, 7.4.2.1.1.
        let mut offsets = [0u32; 4];
        for offset in &mut offsets {
            // The reader refuses first, ue(v) stopping at 2^32 - 2.
            *offset = bits.ue_max(u32::MAX - 1, "a frame_crop offset over 2^32 - 2")?;
        }
        offsets
    } else {
        [0; 4]
    };
    let signal = if bits.flag()? {
        vui_signal_type(&mut bits)?
    } else {
        None
    };

    // 7-13 to 7-18: PicWidthInSamplesL = PicWidthInMbs * 16 and
    // FrameHeightInMbs = ( 2 - frame_mbs_only_flag ) * PicHeightInMapUnits.
    let field_factor: u64 = if frame_mbs_only { 1 } else { 2 };
    let frame_height_in_mbs = height_in_map_units * field_factor;
    // A.3.1 items e to g at the largest MaxFS of Table A-1, level 6.2.
    if frame_height_in_mbs > u64::from(MAX_PIC_SIDE_IN_MBS)
        || width_in_mbs * frame_height_in_mbs > MAX_FRAME_SIZE_IN_MBS
    {
        return Err(bits.malformed("a picture larger than any level of Table A-1 allows"));
    }
    let coded_size = FrameSize {
        width: dimension(width_in_mbs * 16)?,
        height: dimension(frame_height_in_mbs * 16)?,
    };
    // 7-19 to 7-24, with SubWidthC and SubHeightC from Table 6-1.
    let chroma_array_type = if separate_colour_plane {
        0
    } else {
        chroma_format_idc
    };
    let (crop_x, crop_y): (u64, u64) = match chroma_array_type {
        0 => (1, field_factor),
        1 => (2, 2 * field_factor),
        2 => (2, field_factor),
        _ => (1, field_factor),
    };
    let [left, right, top, bottom] = crop.map(u64::from);
    // 7.4.2.1.1: frame_crop_left_offset is at most
    // ( PicWidthInSamplesL / CropUnitX ) - ( frame_crop_right_offset + 1 ) and
    // frame_crop_top_offset at most
    // ( 16 * FrameHeightInMbs / CropUnitY ) - ( frame_crop_bottom_offset + 1 ).
    if left + right + 1 > u64::from(coded_size.width) / crop_x
        || top + bottom + 1 > u64::from(coded_size.height) / crop_y
    {
        return Err(bits.malformed("a cropping window outside the picture"));
    }
    let visible_size = FrameSize {
        width: cropped(coded_size.width, crop_x * (left + right))?,
        height: cropped(coded_size.height, crop_y * (top + bottom))?,
    };
    let id = super::bits::to_usize(id, "H.264 sequence parameter set")?;
    Ok((
        id,
        Sps {
            profile_idc,
            constraint_flags,
            level_idc,
            separate_colour_plane,
            chroma_array_type,
            bit_depth_luma_minus8,
            log2_max_frame_num: small(log2_max_frame_num)?,
            max_num_ref_frames,
            poc,
            frame_mbs_only,
            width_in_mbs,
            frame_height_in_mbs,
            coded_size,
            visible_size,
            signal,
        },
    ))
}

fn small(value: u32) -> Result<u8, VideoError> {
    u8::try_from(value).map_err(|_| VideoError::Malformed {
        structure: "H.264 sequence parameter set",
        reason: "a bit width over 255",
    })
}

pub(crate) fn dimension(value: u64) -> Result<u32, VideoError> {
    u32::try_from(value).map_err(|_| VideoError::Malformed {
        structure: "sequence parameter set",
        reason: "a picture size over 2^32 samples",
    })
}

pub(crate) fn cropped(size: u32, crop: u64) -> Result<u32, VideoError> {
    u64::from(size)
        .checked_sub(crop)
        .filter(|visible| *visible > 0)
        .and_then(|visible| u32::try_from(visible).ok())
        .ok_or(VideoError::Malformed {
            structure: "sequence parameter set",
            reason: "a cropping window as large as the picture",
        })
}

/// 7.3.2.1.1.1, scaling_list, read only for its length.
fn skip_scaling_list(bits: &mut Bits<'_>, size: u32) -> Result<(), VideoError> {
    let mut last = 8i32;
    let mut next = 8i32;
    for _ in 0..size {
        if next != 0 {
            // 7.4.2.1.1.1: delta_scale is -128 to +127.
            let delta = bits.se_range(-128, 127, "delta_scale outside -128 to 127")?;
            next = (last + delta).rem_euclid(256);
        }
        if next != 0 {
            last = next;
        }
    }
    Ok(())
}

/// E.1.1 up to the video signal type, which is all this module needs.
fn vui_signal_type(bits: &mut Bits<'_>) -> Result<Option<SignalType>, VideoError> {
    if bits.flag()? && bits.byte()? == EXTENDED_SAR {
        // aspect_ratio_info_present_flag, aspect_ratio_idc, sar_width and
        // sar_height.
        bits.skip(32)?;
    }
    if bits.flag()? {
        bits.flag()?; // overscan_appropriate_flag
    }
    if !bits.flag()? {
        return Ok(None);
    }
    bits.bits(3)?; // video_format
    let full_range = bits.flag()?;
    let description = if bits.flag()? {
        Some((bits.byte()?, bits.byte()?, bits.byte()?))
    } else {
        None
    };
    Ok(Some(SignalType {
        full_range,
        description,
    }))
}

/// The fields of one picture parameter set this module uses, 7.3.2.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pps {
    sps_id: usize,
    bottom_field_pic_order_in_frame_present: bool,
    num_ref_idx_l0_default: u32,
    num_ref_idx_l1_default: u32,
    weighted_pred: bool,
    weighted_bipred_idc: u32,
    pic_init_qp_minus26: i32,
    redundant_pic_cnt_present: bool,
}

fn parse_pps(nal: &[u8]) -> Result<(usize, Pps), VideoError> {
    let mut bits = Bits::rbsp(
        nal.get(1..).unwrap_or_default(),
        "H.264 picture parameter set",
    );
    let id = bits.ue_max(255, "pic_parameter_set_id over 255")?;
    let sps_id = bits.ue_max(31, "seq_parameter_set_id over 31")?;
    bits.flag()?; // entropy_coding_mode_flag
    let bottom_field_pic_order_in_frame_present = bits.flag()?;
    // A.2.1 allows num_slice_groups_minus1 up to 7 in the profiles that use
    // slice groups, and A.2.4 allows the High profile 0 only.
    if bits.ue_max(7, "num_slice_groups_minus1 over 7")? > 0 {
        return Err(VideoError::Unsupported {
            feature: "an H.264 stream with slice groups",
        });
    }
    let num_ref_idx_l0_default =
        bits.ue_max(MAX_REF_IDX, "num_ref_idx_l0_default_active_minus1 over 31")?;
    let num_ref_idx_l1_default =
        bits.ue_max(MAX_REF_IDX, "num_ref_idx_l1_default_active_minus1 over 31")?;
    let weighted_pred = bits.flag()?;
    let weighted_bipred_idc = bits.bits(2)?;
    // 7.4.2.2: pic_init_qp_minus26 is -( 26 + QpBdOffsetY ) to +25, with
    // QpBdOffsetY = 6 * bit_depth_luma_minus8 by 7-4. QpBdOffsetY belongs to
    // the SPS this PPS activates, which 7.4.1.2.1 does not require to have
    // arrived yet and which may be replaced before activation, so the PPS is
    // read against the widest range, bit_depth_luma_minus8 6 giving -62, and
    // the slice checks it against the SPS it activates.
    // pic_init_qs_minus26 is -26 to +25 and chroma_qp_index_offset -12 to +12.
    let pic_init_qp_minus26 = bits.se_range(
        -(26 + MAX_QP_BD_OFFSET),
        25,
        "pic_init_qp_minus26 out of range",
    )?;
    bits.se_range(-26, 25, "pic_init_qs_minus26 out of range")?;
    bits.se_range(-12, 12, "chroma_qp_index_offset out of range")?;
    bits.flag()?; // deblocking_filter_control_present_flag
    bits.flag()?; // constrained_intra_pred_flag
    let redundant_pic_cnt_present = bits.flag()?;
    Ok((
        super::bits::to_usize(id, "H.264 picture parameter set")?,
        Pps {
            sps_id: super::bits::to_usize(sps_id, "H.264 picture parameter set")?,
            bottom_field_pic_order_in_frame_present,
            num_ref_idx_l0_default,
            num_ref_idx_l1_default,
            weighted_pred,
            weighted_bipred_idc,
            pic_init_qp_minus26,
            redundant_pic_cnt_present,
        },
    ))
}

/// The slice header fields 7.4.1.2.4 compares and 8.2.1 needs, 7.3.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slice {
    nal_ref_idc: u8,
    idr: bool,
    pps_id: usize,
    frame_num: u32,
    idr_pic_id: u32,
    poc_lsb: u32,
    delta_poc_bottom: i32,
    delta_poc: [i32; 2],
    redundant_pic_cnt: u32,
    mmco5: bool,
}

/// Table 7-6, slice_type modulo 5.
const P: u32 = 0;
const B: u32 = 1;
const I: u32 = 2;
const SP: u32 = 3;
const SI: u32 = 4;

fn parse_slice(
    nal: &[u8],
    sps_table: &[Option<Sps>],
    pps_table: &[Option<Pps>],
) -> Result<Slice, VideoError> {
    let header = *nal.first().ok_or(VideoError::Truncated {
        structure: "H.264 slice header",
    })?;
    let nal_ref_idc = (header >> 5) & 0x03;
    let idr = header & 0x1F == nal::IDR;
    let mut bits = Bits::rbsp(nal.get(1..).unwrap_or_default(), "H.264 slice header");
    // Bounded by PicSizeInMbs once the parameter sets are known. The bound
    // here never refuses, ue(v) stopping at 2^32 - 2.
    let first_mb_in_slice = bits.ue_max(u32::MAX - 1, "first_mb_in_slice over 2^32 - 2")?;
    let slice_type = bits.ue_max(9, "slice_type over 9")? % 5;
    let pps_id = super::bits::to_usize(
        bits.ue_max(255, "pic_parameter_set_id over 255")?,
        "H.264 slice header",
    )?;
    let pps =
        pps_table.get(pps_id).copied().flatten().ok_or_else(|| {
            bits.malformed("a slice refers to a picture parameter set not yet seen")
        })?;
    let sps = sps_table
        .get(pps.sps_id)
        .and_then(Option::as_ref)
        .ok_or_else(|| {
            bits.malformed(
                "a picture parameter set refers to a sequence parameter set not yet seen",
            )
        })?;
    // 7.4.3: first_mb_in_slice is 0 to PicSizeInMbs - 1, which for a frame is
    // PicWidthInMbs * FrameHeightInMbs by 7-28 and 7-31.
    if u64::from(first_mb_in_slice) >= sps.width_in_mbs * sps.frame_height_in_mbs {
        return Err(bits.malformed("first_mb_in_slice past the last macroblock"));
    }
    // 7.4.2.2 against the SPS this slice activates, 7.4.1.2.1: the lower
    // bound of pic_init_qp_minus26 is -( 26 + 6 * bit_depth_luma_minus8 ).
    let qp_bd_offset = i32::try_from(6 * sps.bit_depth_luma_minus8)
        .map_err(|_| bits.malformed("a bit depth out of range"))?;
    if pps.pic_init_qp_minus26 < -(26 + qp_bd_offset) {
        return Err(bits.malformed(
            "pic_init_qp_minus26 below -( 26 + QpBdOffsetY ) of the active sequence parameter set",
        ));
    }
    if sps.separate_colour_plane {
        bits.bits(2)?; // colour_plane_id
    }
    let frame_num = bits.bits(sps.log2_max_frame_num)?;
    let max_frame_num = 1u32 << sps.log2_max_frame_num;
    if !sps.frame_mbs_only && bits.flag()? {
        // field_pic_flag. A field is half a displayed frame, and pairing the
        // two fields of a frame is not done here.
        return Err(VideoError::Unsupported {
            feature: "an H.264 stream of field-coded pictures outside MP4",
        });
    }
    // 7.4.3: idr_pic_id is 0 to 65535.
    let idr_pic_id = if idr {
        bits.ue_max(65_535, "idr_pic_id over 65535")?
    } else {
        0
    };
    let mut poc_lsb = 0;
    let mut delta_poc_bottom = 0;
    let mut delta_poc = [0; 2];
    match &sps.poc {
        PocType::Zero { log2_max_lsb } => {
            poc_lsb = bits.bits(*log2_max_lsb)?;
            if pps.bottom_field_pic_order_in_frame_present {
                // 7.4.3, with the tighter lower bound of a picture holding a
                // memory_management_control_operation 5 checked below.
                delta_poc_bottom = bits.se_range(
                    -SE_LIMIT,
                    SE_LIMIT,
                    "delta_pic_order_cnt_bottom out of range",
                )?;
            }
        }
        PocType::One {
            delta_always_zero: false,
            ..
        } => {
            // 7.4.3: each is -2^31 + 1 to 2^31 - 1.
            delta_poc[0] =
                bits.se_range(-SE_LIMIT, SE_LIMIT, "delta_pic_order_cnt[ 0 ] out of range")?;
            if pps.bottom_field_pic_order_in_frame_present {
                delta_poc[1] =
                    bits.se_range(-SE_LIMIT, SE_LIMIT, "delta_pic_order_cnt[ 1 ] out of range")?;
            }
        }
        PocType::One { .. } | PocType::Two => {}
    }
    // 7.4.3: redundant_pic_cnt is 0 to 127.
    let redundant_pic_cnt = if pps.redundant_pic_cnt_present {
        bits.ue_max(127, "redundant_pic_cnt over 127")?
    } else {
        0
    };
    if slice_type == B {
        bits.flag()?; // direct_spatial_mv_pred_flag
    }
    let mut l0 = pps.num_ref_idx_l0_default;
    let mut l1 = pps.num_ref_idx_l1_default;
    // 7.4.3: for a frame, field_pic_flag 0, the override is 0 to 15. Field
    // pictures were refused above.
    if matches!(slice_type, P | SP | B) && bits.flag()? {
        l0 = bits.ue_max(MAX_FRAME_REF_IDX, "num_ref_idx_l0_active_minus1 over 15")?;
        if slice_type == B {
            l1 = bits.ue_max(MAX_FRAME_REF_IDX, "num_ref_idx_l1_active_minus1 over 15")?;
        }
    }
    let limits = Limits {
        max_frame_num,
        max_num_ref_frames: sps.max_num_ref_frames,
    };
    // 7.3.3.1, ref_pic_list_modification.
    if slice_type != I && slice_type != SI {
        skip_list_modification(&mut bits, limits)?;
    }
    if slice_type == B {
        skip_list_modification(&mut bits, limits)?;
    }
    if (pps.weighted_pred && matches!(slice_type, P | SP))
        || (pps.weighted_bipred_idc == 1 && slice_type == B)
    {
        skip_pred_weight_table(
            &mut bits,
            sps.chroma_array_type != 0,
            l0,
            (slice_type == B).then_some(l1),
        )?;
    }
    let mut mmco5 = false;
    if nal_ref_idc != 0 {
        // 7.3.3.3, dec_ref_pic_marking.
        if idr {
            bits.skip(2)?; // no_output_of_prior_pics_flag, long_term_reference_flag
        } else if bits.flag()? {
            // Table 7-9 operations, with the fields of 7.4.3.3.
            loop {
                match bits.ue_max(6, "memory_management_control_operation over 6")? {
                    0 => break,
                    1 => limits.difference_of_pic_nums(&mut bits)?,
                    2 => limits.long_term_number(&mut bits)?,
                    3 => {
                        limits.difference_of_pic_nums(&mut bits)?;
                        limits.long_term_number(&mut bits)?;
                    }
                    4 => {
                        // max_long_term_frame_idx_plus1 is 0 to
                        // max_num_ref_frames.
                        bits.ue_max(
                            limits.max_num_ref_frames,
                            "max_long_term_frame_idx_plus1 over max_num_ref_frames",
                        )?;
                    }
                    5 => mmco5 = true,
                    _ => limits.long_term_number(&mut bits)?,
                }
            }
        }
    }
    // 7.4.3: with a memory_management_control_operation 5,
    // delta_pic_order_cnt_bottom is at least 1 - MaxPicOrderCntLsb.
    if let PocType::Zero { log2_max_lsb } = &sps.poc
        && mmco5
        && i64::from(delta_poc_bottom) < 1 - (1i64 << log2_max_lsb)
    {
        return Err(bits.malformed("delta_pic_order_cnt_bottom out of range"));
    }
    Ok(Slice {
        nal_ref_idc,
        idr,
        pps_id,
        frame_num,
        idr_pic_id,
        poc_lsb,
        delta_poc_bottom,
        delta_poc,
        redundant_pic_cnt,
        mmco5,
    })
}

/// The sequence limits the reference picture fields of a frame are bounded by.
#[derive(Debug, Clone, Copy)]
struct Limits {
    /// MaxFrameNum, 7-10, which is MaxPicNum for a frame, 7.4.3.
    max_frame_num: u32,
    max_num_ref_frames: u32,
}

impl Limits {
    /// abs_diff_pic_num_minus1 or difference_of_pic_nums_minus1. For a frame
    /// the first is 0 to MaxPicNum - 1, 7.4.3.1, with MaxPicNum equal to
    /// MaxFrameNum. The second must reach a short-term reference frame, whose
    /// PicNum is a FrameNumWrap above frame_num - MaxFrameNum by 8-27 and 8-28,
    /// so MaxPicNum - 1 bounds it too, one more than the tightest bound.
    fn difference_of_pic_nums(self, bits: &mut Bits<'_>) -> Result<(), VideoError> {
        bits.ue_max(
            self.max_frame_num - 1,
            "a picture number difference of MaxPicNum or more",
        )?;
        Ok(())
    }

    /// long_term_pic_num or long_term_frame_idx. For a frame
    /// LongTermPicNum equals LongTermFrameIdx, 8.2.4.1, which 7.4.3.3 bounds
    /// by MaxLongTermFrameIdx, at most max_num_ref_frames - 1 because
    /// max_long_term_frame_idx_plus1 is at most max_num_ref_frames.
    fn long_term_number(self, bits: &mut Bits<'_>) -> Result<(), VideoError> {
        let Some(max) = self.max_num_ref_frames.checked_sub(1) else {
            return Err(
                bits.malformed("a long-term reference in a sequence with max_num_ref_frames 0")
            );
        };
        bits.ue_max(max, "a long-term index of max_num_ref_frames or more")?;
        Ok(())
    }
}

/// One list of 7.3.3.1, from its flag to modification_of_pic_nums_idc 3.
fn skip_list_modification(bits: &mut Bits<'_>, limits: Limits) -> Result<(), VideoError> {
    if bits.flag()? {
        loop {
            // Table 7-7 gives 0 to 3. Values 4 and 5 belong to the MVC list
            // modification of Annex H, whose NAL unit types this demuxer does
            // not read, so they are refused by name.
            match bits.ue_max(5, "modification_of_pic_nums_idc over 5")? {
                0 | 1 => limits.difference_of_pic_nums(bits)?,
                2 => limits.long_term_number(bits)?,
                3 => break,
                _ => {
                    return Err(VideoError::Unsupported {
                        feature: "modification_of_pic_nums_idc 4 or 5, the MVC values,",
                    });
                }
            }
        }
    }
    Ok(())
}

/// 7.3.3.2, pred_weight_table, read only for its length.
fn skip_pred_weight_table(
    bits: &mut Bits<'_>,
    chroma: bool,
    l0: u32,
    l1: Option<u32>,
) -> Result<(), VideoError> {
    // 7.4.3.2: each denominator is 0 to 7, and each weight and offset is
    // -128 to 127.
    bits.ue_max(7, "luma_log2_weight_denom over 7")?;
    if chroma {
        bits.ue_max(7, "chroma_log2_weight_denom over 7")?;
    }
    let weight = |bits: &mut Bits<'_>| {
        bits.se_range(
            -128,
            127,
            "a prediction weight or offset outside -128 to 127",
        )
    };
    for count in std::iter::once(l0).chain(l1) {
        for _ in 0..=count {
            if bits.flag()? {
                weight(bits)?;
                weight(bits)?;
            }
            if chroma && bits.flag()? {
                for _ in 0..4 {
                    weight(bits)?;
                }
            }
        }
    }
    Ok(())
}

/// 7.4.1.2.4: whether `next` is the first VCL NAL unit of a new primary
/// picture after a picture whose first slice is `previous`.
fn starts_new_picture(
    previous: &Slice,
    next: &Slice,
    poc_type_zero: bool,
    poc_type_one: bool,
) -> bool {
    previous.frame_num != next.frame_num
        || previous.pps_id != next.pps_id
        || (previous.nal_ref_idc != next.nal_ref_idc
            && (previous.nal_ref_idc == 0 || next.nal_ref_idc == 0))
        || (poc_type_zero
            && (previous.poc_lsb != next.poc_lsb
                || previous.delta_poc_bottom != next.delta_poc_bottom))
        || (poc_type_one && previous.delta_poc != next.delta_poc)
        || previous.idr != next.idr
        || (previous.idr && next.idr && previous.idr_pic_id != next.idr_pic_id)
}

/// The state 8.2.1 carries from picture to picture, for frames.
#[derive(Debug, Default)]
struct PocState {
    /// The previous reference picture's PicOrderCntMsb and pic_order_cnt_lsb,
    /// or after a memory_management_control_operation 5, 0 and its
    /// TopFieldOrderCnt, 8.2.1.1.
    prev_msb: i64,
    prev_lsb: i64,
    /// The previous picture's frame_num and FrameNumOffset, 8.2.1.2 and
    /// 8.2.1.3, with both 0 after a memory_management_control_operation 5.
    prev_frame_num: i64,
    prev_frame_num_offset: i64,
    /// The output segment. An IDR picture or one with a
    /// memory_management_control_operation 5 empties the decoded picture
    /// buffer by bumping before it is stored, C.4.4, so every earlier picture
    /// is output before it and it starts a new segment. An IDR picture with
    /// no_output_of_prior_pics_flag 1 discards those pictures instead, and
    /// this table still lists them, as an MP4 sample table would.
    segment: u32,
    started: bool,
}

fn overflow() -> VideoError {
    VideoError::Malformed {
        structure: "H.264 picture order count",
        reason: "a count outside the range of 8.2.1",
    }
}

fn add(left: i64, right: i64) -> Result<i64, VideoError> {
    left.checked_add(right).ok_or_else(overflow)
}

impl PocState {
    /// The output position (segment, PicOrderCnt) of one frame, 8.2.1.
    fn next(&mut self, sps: &Sps, slice: &Slice) -> Result<(u32, i64), VideoError> {
        if self.started && (slice.idr || slice.mmco5) {
            self.segment = self.segment.checked_add(1).ok_or_else(overflow)?;
        }
        self.started = true;
        let max_frame_num = 1i64 << sps.log2_max_frame_num;
        let frame_num = i64::from(slice.frame_num);
        let reference = slice.nal_ref_idc != 0;
        let frame_num_offset = if slice.idr {
            0
        } else if self.prev_frame_num > frame_num {
            add(self.prev_frame_num_offset, max_frame_num)?
        } else {
            self.prev_frame_num_offset
        };
        let (top, bottom, msb) = match &sps.poc {
            PocType::Zero { log2_max_lsb } => {
                let max_lsb = 1i64 << log2_max_lsb;
                let (prev_msb, prev_lsb) = if slice.idr {
                    (0, 0)
                } else {
                    (self.prev_msb, self.prev_lsb)
                };
                let lsb = i64::from(slice.poc_lsb);
                // 8-3.
                let msb = if lsb < prev_lsb && prev_lsb - lsb >= max_lsb / 2 {
                    add(prev_msb, max_lsb)?
                } else if lsb > prev_lsb && lsb - prev_lsb > max_lsb / 2 {
                    add(prev_msb, -max_lsb)?
                } else {
                    prev_msb
                };
                // 8-4 and 8-5 for a frame.
                let top = add(msb, lsb)?;
                let bottom = add(top, slice.delta_poc_bottom.into())?;
                (top, bottom, msb)
            }
            PocType::One {
                offset_for_non_ref_pic,
                offset_for_top_to_bottom_field,
                offset_for_ref_frame,
                ..
            } => {
                // 8-7 to 8-10 for a frame.
                let cycle = i64::try_from(offset_for_ref_frame.len()).map_err(|_| overflow())?;
                let mut abs_frame_num = if cycle != 0 {
                    add(frame_num_offset, frame_num)?
                } else {
                    0
                };
                if !reference && abs_frame_num > 0 {
                    abs_frame_num -= 1;
                }
                let mut expected = 0i64;
                if abs_frame_num > 0 {
                    let per_cycle: i64 = offset_for_ref_frame
                        .iter()
                        .map(|value| i64::from(*value))
                        .sum();
                    let cycle_count = (abs_frame_num - 1) / cycle;
                    let in_cycle =
                        usize::try_from((abs_frame_num - 1) % cycle).map_err(|_| overflow())?;
                    expected = cycle_count.checked_mul(per_cycle).ok_or_else(overflow)?;
                    for offset in offset_for_ref_frame.iter().take(in_cycle + 1) {
                        expected = add(expected, (*offset).into())?;
                    }
                }
                if !reference {
                    expected = add(expected, (*offset_for_non_ref_pic).into())?;
                }
                let top = add(expected, slice.delta_poc[0].into())?;
                let bottom = add(
                    add(top, (*offset_for_top_to_bottom_field).into())?,
                    slice.delta_poc[1].into(),
                )?;
                (top, bottom, 0)
            }
            PocType::Two => {
                // 8-12, with TopFieldOrderCnt and BottomFieldOrderCnt both
                // tempPicOrderCnt for a frame.
                let temp = if slice.idr {
                    0
                } else {
                    let doubled = add(frame_num_offset, frame_num)?
                        .checked_mul(2)
                        .ok_or_else(overflow)?;
                    if reference {
                        doubled
                    } else {
                        add(doubled, -1)?
                    }
                };
                (temp, temp, 0)
            }
        };
        // 8-1, PicOrderCnt of a frame.
        let mut poc = top.min(bottom);
        if slice.mmco5 {
            // 8.2.1: after decoding, tempPicOrderCnt = PicOrderCnt( CurrPic )
            // is subtracted from both field counts, so this frame is 0 in
            // its new segment and TopFieldOrderCnt becomes top - poc.
            if reference {
                self.prev_msb = 0;
                self.prev_lsb = top - poc;
            }
            self.prev_frame_num = 0;
            self.prev_frame_num_offset = 0;
            poc = 0;
        } else {
            if reference {
                self.prev_msb = msb;
                self.prev_lsb = i64::from(slice.poc_lsb);
            }
            self.prev_frame_num = frame_num;
            self.prev_frame_num_offset = frame_num_offset;
        }
        Ok((self.segment, poc))
    }
}

/// Splits an Annex B byte stream into access units, 7.4.1.2.3, with the
/// output position of each from 8.2.1 and the stream description from the
/// first sequence parameter set.
pub(crate) fn access_units(es: &[u8]) -> Result<(Vec<AccessUnit>, StreamInfo), VideoError> {
    let mut sps_table: Vec<Option<Sps>> = vec![None; SPS_IDS];
    let mut pps_table: Vec<Option<Pps>> = vec![None; PPS_IDS];
    let mut info: Option<StreamInfo> = None;
    let mut found: Vec<AccessUnit> = Vec::new();
    let mut poc = PocState::default();
    // The current access unit: its first byte and start code position, the
    // first slice of its primary picture, and whether that picture's last
    // VCL NAL unit may already have passed.
    let mut open: Option<(usize, usize)> = None;
    let mut picture: Option<(Slice, (u32, i64))> = None;
    let mut after_vcl = false;
    let mut last_end = 0usize;
    for unit in units(es) {
        let nal = es.get(unit.payload..unit.end).unwrap_or_default();
        let Some(header) = nal.first().copied() else {
            continue;
        };
        if header & 0x80 != 0 {
            return Err(VideoError::Malformed {
                structure: "H.264 NAL unit header",
                reason: "forbidden_zero_bit is 1",
            });
        }
        let kind = header & 0x1F;
        let prefix = unit.payload - 3;
        let starts_unit = matches!(kind, nal::SEI | nal::SPS | nal::PPS | nal::AUD)
            || (nal::PREFIX_FIRST..=nal::PREFIX_LAST).contains(&kind);
        if starts_unit && after_vcl {
            close(&mut found, &mut open, &mut picture, last_end)?;
            after_vcl = false;
        }
        if open.is_none() {
            open = Some((unit.start, prefix));
        }
        match kind {
            nal::SPS => {
                let (id, sps) = parse_sps(nal)?;
                if info.is_none() {
                    info = Some(sps.stream_info("avc1"));
                }
                if let Some(slot) = sps_table.get_mut(id) {
                    *slot = Some(sps);
                }
            }
            nal::PPS => {
                let (id, pps) = parse_pps(nal)?;
                if let Some(slot) = pps_table.get_mut(id) {
                    *slot = Some(pps);
                }
            }
            nal::PARTITION_A..=nal::PARTITION_C => {
                // A.2.4: the High profile allows no nal_unit_type 2 to 4.
                return Err(VideoError::Unsupported {
                    feature: "an H.264 stream with data partitioning",
                });
            }
            nal::SLICE | nal::IDR => {
                let slice = parse_slice(nal, &sps_table, &pps_table)?;
                let sps = pps_table
                    .get(slice.pps_id)
                    .copied()
                    .flatten()
                    .and_then(|pps| sps_table.get(pps.sps_id).and_then(Option::as_ref))
                    .ok_or(VideoError::Malformed {
                        structure: "H.264 slice header",
                        reason: "a slice refers to a parameter set not yet seen",
                    })?;
                let zero = matches!(sps.poc, PocType::Zero { .. });
                let one = matches!(sps.poc, PocType::One { .. });
                let new_picture = match &picture {
                    None => true,
                    Some((first, _)) => {
                        slice.redundant_pic_cnt == 0 && starts_new_picture(first, &slice, zero, one)
                    }
                };
                if new_picture {
                    if picture.is_some() {
                        close(&mut found, &mut open, &mut picture, last_end)?;
                        open = Some((unit.start, prefix));
                    }
                    let order = poc.next(sps, &slice)?;
                    picture = Some((slice, order));
                }
                after_vcl = true;
            }
            _ => {}
        }
        last_end = unit.end;
    }
    if picture.is_some() {
        close(&mut found, &mut open, &mut picture, es.len())?;
    }
    let info = info.ok_or(VideoError::Malformed {
        structure: "H.264 byte stream",
        reason: "no sequence parameter set",
    })?;
    Ok((found, info))
}

fn close(
    found: &mut Vec<AccessUnit>,
    open: &mut Option<(usize, usize)>,
    picture: &mut Option<(Slice, (u32, i64))>,
    end: usize,
) -> Result<(), VideoError> {
    let (start, anchor) = open.take().ok_or(VideoError::Malformed {
        structure: "H.264 byte stream",
        reason: "an access unit without a start",
    })?;
    let Some((slice, order)) = picture.take() else {
        return Err(VideoError::Malformed {
            structure: "H.264 byte stream",
            reason: "an access unit without a primary coded picture",
        });
    };
    let range: Range<usize> = start..end;
    found.push(AccessUnit {
        range,
        anchor,
        order,
        // An IDR picture is where 8.2.1 and the decoded reference picture
        // marking restart, so decoding may begin there.
        random_access: slice.idr,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PocState, PocType, Slice, Sps, access_units, parse_sps, starts_new_picture};
    use crate::video::{
        FrameSize, VideoError,
        bits::{Writer, annex_b},
    };

    fn frame_sps(poc: PocType, log2_max_frame_num: u8) -> Sps {
        Sps {
            profile_idc: 100,
            constraint_flags: 0,
            level_idc: 30,
            separate_colour_plane: false,
            chroma_array_type: 1,
            bit_depth_luma_minus8: 0,
            log2_max_frame_num,
            max_num_ref_frames: 4,
            poc,
            frame_mbs_only: true,
            width_in_mbs: 1,
            frame_height_in_mbs: 1,
            coded_size: FrameSize {
                width: 16,
                height: 16,
            },
            visible_size: FrameSize {
                width: 16,
                height: 16,
            },
            signal: None,
        }
    }

    /// A frame's slice with the fields 8.2.1 reads.
    fn slice(idr: bool, reference: bool, frame_num: u32, poc_lsb: u32) -> Slice {
        Slice {
            nal_ref_idc: if reference { 2 } else { 0 },
            idr,
            pps_id: 0,
            frame_num,
            idr_pic_id: 0,
            poc_lsb,
            delta_poc_bottom: 0,
            delta_poc: [0; 2],
            redundant_pic_cnt: 0,
            mmco5: false,
        }
    }

    fn run(sps: &Sps, slices: &[Slice]) -> Result<Vec<(u32, i64)>, VideoError> {
        let mut state = PocState::default();
        slices.iter().map(|slice| state.next(sps, slice)).collect()
    }

    /// 8.2.1.2, pic_order_cnt_type 1, with offset_for_ref_frame [4, 2], so
    /// ExpectedDeltaPerPicOrderCntCycle is 6 by 7-12, offset_for_non_ref_pic
    /// -2, offset_for_top_to_bottom_field 1 and MaxFrameNum 16. By 8-6 to 8-10
    /// and 8-1, PicOrderCnt = Min( TopFieldOrderCnt, BottomFieldOrderCnt ):
    ///
    /// - IDR, frame_num 0: absFrameNum 0, expected 0, top 0, bottom 1, POC 0.
    /// - reference, frame_num 1: absFrameNum 1, cycle 0, position 0, expected
    ///   4, POC 4.
    /// - non-reference, frame_num 2: absFrameNum 2 - 1 = 1, expected 4, plus
    ///   -2 is 2, POC 2.
    /// - reference, frame_num 2: absFrameNum 2, cycle 0, position 1, expected
    ///   4 + 2 = 6, POC 6.
    /// - reference, frame_num 3, delta_pic_order_cnt -1 and 3: absFrameNum 3,
    ///   cycle 1, position 0, expected 6 + 4 = 10, top 9, bottom 9 + 1 + 3 =
    ///   13, POC 9.
    /// - reference, frame_num 0 after 3: FrameNumOffset 16 by 8-6, absFrameNum
    ///   16, cycle 7, position 1, expected 7 * 6 + 4 + 2 = 48, POC 48.
    #[test]
    fn poc_type_1_follows_8_2_1_2() -> Result<(), VideoError> {
        let sps = frame_sps(
            PocType::One {
                delta_always_zero: false,
                offset_for_non_ref_pic: -2,
                offset_for_top_to_bottom_field: 1,
                offset_for_ref_frame: vec![4, 2],
            },
            4,
        );
        let mut fourth = slice(false, true, 3, 0);
        fourth.delta_poc = [-1, 3];
        let slices = [
            slice(true, true, 0, 0),
            slice(false, true, 1, 0),
            slice(false, false, 2, 0),
            slice(false, true, 2, 0),
            fourth,
            slice(false, true, 0, 0),
        ];
        assert_eq!(
            run(&sps, &slices)?,
            [(0, 0), (0, 4), (0, 2), (0, 6), (0, 9), (0, 48)]
        );
        Ok(())
    }

    /// 8.2.1.3, pic_order_cnt_type 2, 8-12: tempPicOrderCnt is
    /// 2 * ( FrameNumOffset + frame_num ) for a reference picture and one less
    /// for a non-reference picture. IDR 0, reference frame_num 1 gives 2,
    /// non-reference frame_num 2 gives 3, reference frame_num 2 gives 4.
    #[test]
    fn poc_type_2_non_reference_arm() -> Result<(), VideoError> {
        let sps = frame_sps(PocType::Two, 4);
        let slices = [
            slice(true, true, 0, 0),
            slice(false, true, 1, 0),
            slice(false, false, 2, 0),
            slice(false, true, 2, 0),
        ];
        assert_eq!(run(&sps, &slices)?, [(0, 0), (0, 2), (0, 3), (0, 4)]);
        Ok(())
    }

    /// 8.2.1 and 8.2.1.1 with a memory_management_control_operation 5. With
    /// MaxPicOrderCntLsb 16: IDR lsb 0 is POC 0, a reference lsb 4 is POC 4,
    /// and a reference lsb 8 with the operation is POC 8, then reset to 0,
    /// which C.4.4 outputs after every earlier picture, so it opens segment 1.
    /// prevPicOrderCntMsb becomes 0 and prevPicOrderCntLsb its
    /// TopFieldOrderCnt after the reset, 8 - 8 = 0, so a following reference
    /// lsb 2 is POC 2 in segment 1, after the reset picture, not before the
    /// POC 4 picture.
    #[test]
    fn mmco5_resets_the_count_and_opens_a_segment() -> Result<(), VideoError> {
        let sps = frame_sps(PocType::Zero { log2_max_lsb: 4 }, 4);
        let mut reset = slice(false, true, 2, 8);
        reset.mmco5 = true;
        let slices = [
            slice(true, true, 0, 0),
            slice(false, true, 1, 4),
            reset,
            slice(false, true, 1, 2),
        ];
        assert_eq!(run(&sps, &slices)?, [(0, 0), (0, 4), (1, 0), (1, 2)]);
        Ok(())
    }

    /// 8.2.1.3 with a memory_management_control_operation 5: prevFrameNumOffset
    /// is 0 after it, and the frame_num it carried is taken as 0 for what
    /// follows, 7.4.1.2.4 NOTE 1. A reference frame_num 5 with the operation
    /// resets to 0. The next reference frame_num 1 then has FrameNumOffset 0
    /// and POC 2. Comparing frame_num 1 with the 5 it carried would add
    /// MaxFrameNum 16 and give 34.
    #[test]
    fn mmco5_resets_the_frame_num_offset() -> Result<(), VideoError> {
        let sps = frame_sps(PocType::Two, 4);
        let mut reset = slice(false, true, 5, 0);
        reset.mmco5 = true;
        let slices = [slice(true, true, 0, 0), reset, slice(false, true, 1, 0)];
        assert_eq!(run(&sps, &slices)?, [(0, 0), (1, 0), (1, 2)]);
        Ok(())
    }

    /// 8-3 at both boundaries, MaxPicOrderCntLsb 16 so half is 8. Reference
    /// lsb 6 then 12 are 6 apart, so both keep PicOrderCntMsb 0. From lsb 12,
    /// lsb 4 is 8 below, which is >= 8, so PicOrderCntMsb gains 16 and POC is
    /// 20. From a reference lsb 4, lsb 12 is 8 above, which is not > 8, so
    /// PicOrderCntMsb stays 0 and POC is 12.
    #[test]
    fn poc_lsb_wrap_boundaries_of_8_3() -> Result<(), VideoError> {
        let sps = frame_sps(PocType::Zero { log2_max_lsb: 4 }, 4);
        let up = [
            slice(true, true, 0, 0),
            slice(false, true, 1, 6),
            slice(false, true, 2, 12),
            slice(false, true, 3, 4),
        ];
        assert_eq!(run(&sps, &up)?, [(0, 0), (0, 6), (0, 12), (0, 20)]);
        let flat = [
            slice(true, true, 0, 0),
            slice(false, true, 1, 4),
            slice(false, true, 2, 12),
        ];
        assert_eq!(run(&sps, &flat)?, [(0, 0), (0, 4), (0, 12)]);
        Ok(())
    }

    /// 7.4.1.2.4: a different pic_parameter_set_id alone starts a new picture,
    /// and a slice equal in every compared field does not.
    #[test]
    fn a_new_pps_id_starts_a_new_picture() {
        let first = slice(false, true, 1, 2);
        let mut other = first;
        other.pps_id = 1;
        assert!(starts_new_picture(&first, &other, true, false));
        assert!(!starts_new_picture(&first, &first, true, false));
    }

    /// A Baseline SPS, 7.3.2.1.1, for a 16 by 16 picture with
    /// pic_order_cnt_type 0 and 4-bit frame_num and pic_order_cnt_lsb.
    fn baseline_sps() -> Vec<u8> {
        let mut w = Writer::default();
        w.u(8, 66).u(8, 0).u(8, 30).ue(0); // profile_idc, flags, level_idc, id
        w.ue(0).ue(0).ue(0); // log2_max_frame_num_minus4, poc type 0, lsb_minus4
        w.ue(1).flag(false); // max_num_ref_frames, gaps
        w.ue(0).ue(0).flag(true).flag(true).flag(false).flag(false); // 1 by 1 macroblock
        w.nal(&[0x67])
    }

    /// 7.3.2.2, with every flag clear and every value 0.
    fn baseline_pps() -> Vec<u8> {
        let mut w = Writer::default();
        w.ue(0).ue(0).flag(false).flag(false).ue(0).ue(0).ue(0);
        w.flag(false)
            .u(2, 0)
            .se(0)
            .se(0)
            .se(0)
            .flag(false)
            .flag(false)
            .flag(false);
        w.nal(&[0x68])
    }

    /// An I slice of an IDR picture or a P slice, 7.3.3, up to
    /// dec_ref_pic_marking, both reference pictures.
    fn slice_nal(idr: bool, frame_num: u64, poc_lsb: u64) -> Vec<u8> {
        let mut w = Writer::default();
        w.ue(0).ue(if idr { 7 } else { 5 }).ue(0).u(4, frame_num);
        if idr {
            w.ue(0); // idr_pic_id
        }
        w.u(4, poc_lsb);
        if !idr {
            w.flag(false).flag(false); // num_ref_idx override, list modification
        }
        w.flag(false); // no_output_of_prior_pics or adaptive marking
        if idr {
            w.flag(false); // long_term_reference_flag
        }
        w.nal(&[if idr { 0x65 } else { 0x41 }])
    }

    /// 7.4.1.2.3: an SEI NAL unit after the last VCL NAL unit of a picture
    /// starts the next access unit, so the second access unit begins at the
    /// SEI's start code and not at the P slice after it.
    #[test]
    fn an_sei_opens_the_next_access_unit() -> Result<(), VideoError> {
        let sei = vec![0x06, 0x05, 0x01, 0x00, 0x80];
        let units = [
            baseline_sps(),
            baseline_pps(),
            slice_nal(true, 0, 0),
            sei,
            slice_nal(false, 1, 2),
        ];
        let stream = annex_b(&units);
        let sei_start: usize = units.iter().take(3).map(|unit| unit.len() + 4).sum();
        let (found, _) = access_units(&stream)?;
        let starts: Vec<usize> = found.iter().map(|unit| unit.range.start).collect();
        assert_eq!(starts, [0, sei_start]);
        Ok(())
    }

    /// 7.3.3.3 with adaptive_ref_pic_marking_mode_flag 1 and the operations
    /// of Table 7-9: 1 with difference_of_pic_nums_minus1 0, 3 with
    /// difference_of_pic_nums_minus1 0 and long_term_frame_idx 0, then 5,
    /// then 0. Reading every field at its width finds the operation 5, so the
    /// picture opens segment 1 at POC 0 and a following reference lsb 2 is
    /// (1, 2), where a misread would leave the three pictures in segment 0.
    #[test]
    fn memory_management_operations_are_parsed() -> Result<(), VideoError> {
        let mut w = Writer::default();
        w.ue(0).ue(5).ue(0).u(4, 1).u(4, 4).flag(false).flag(false);
        w.flag(true).ue(1).ue(0).ue(3).ue(0).ue(0).ue(5).ue(0);
        let marked = w.nal(&[0x41]);
        let stream = annex_b(&[
            baseline_sps(),
            baseline_pps(),
            slice_nal(true, 0, 0),
            marked,
            slice_nal(false, 1, 2),
        ]);
        let (found, _) = access_units(&stream)?;
        let orders: Vec<(u32, i64)> = found.iter().map(|unit| unit.order).collect();
        assert_eq!(orders, [(0, 0), (1, 0), (1, 2)]);
        Ok(())
    }

    /// Table 7-7 gives modification_of_pic_nums_idc 0 to 3. Values 4 and 5
    /// are the MVC list modification of Annex H, refused by name. A P slice
    /// whose list 0 modification opens with 4 is refused, and the same slice
    /// with 3, the end of the loop, reads.
    #[test]
    fn mvc_list_modification_values_are_refused() -> Result<(), VideoError> {
        let p_slice = |idc: u64| {
            let mut w = Writer::default();
            w.ue(0).ue(5).ue(0).u(4, 1).u(4, 2).flag(false);
            w.flag(true).ue(idc);
            if idc == 4 {
                w.ue(0);
            }
            w.ue(3).flag(false);
            w.nal(&[0x41])
        };
        let stream = |idc| {
            annex_b(&[
                baseline_sps(),
                baseline_pps(),
                slice_nal(true, 0, 0),
                p_slice(idc),
            ])
        };
        assert_eq!(access_units(&stream(3))?.0.len(), 2);
        assert_eq!(
            access_units(&stream(4)).err(),
            Some(VideoError::Unsupported {
                feature: "modification_of_pic_nums_idc 4 or 5, the MVC values,",
            })
        );
        Ok(())
    }

    /// E.1.1: aspect_ratio_idc 255, Extended_SAR in Table E-1, is followed by
    /// 16-bit sar_width and sar_height, then the video signal type. The
    /// colour code points 5, 1, 6 and the full range flag read back only if
    /// all 32 bits were skipped.
    #[test]
    fn extended_sar_is_skipped_before_the_signal_type() -> Result<(), VideoError> {
        let mut w = Writer::default();
        w.u(8, 66)
            .u(8, 0)
            .u(8, 30)
            .ue(0)
            .ue(0)
            .ue(0)
            .ue(0)
            .ue(1)
            .flag(false);
        w.ue(0).ue(0).flag(true).flag(true).flag(false);
        w.flag(true); // vui_parameters_present_flag
        w.flag(true).u(8, 255).u(16, 0xFFFF).u(16, 0x0001); // Extended_SAR
        w.flag(false); // overscan_info_present_flag
        w.flag(true)
            .u(3, 5)
            .flag(true)
            .flag(true)
            .u(8, 5)
            .u(8, 1)
            .u(8, 6);
        let (_, sps) = parse_sps(&w.nal(&[0x67]))?;
        let signal = sps.signal.ok_or(VideoError::Empty)?;
        assert!(signal.full_range);
        assert_eq!(signal.description, Some((5, 1, 6)));
        Ok(())
    }

    /// 7.3.2.1.1: a 4:2:0 High profile SPS has 8 seq_scaling_list_present_flag
    /// bits, not 12. With all 8 clear and pic_width_in_mbs_minus1 3 after
    /// them, the width reads 64. Reading 12 flags would move every later
    /// field by four bits.
    #[test]
    fn a_420_sps_has_eight_scaling_lists() -> Result<(), VideoError> {
        let mut w = Writer::default();
        w.u(8, 100)
            .u(8, 0)
            .u(8, 30)
            .ue(0)
            .ue(1)
            .ue(0)
            .ue(0)
            .flag(false);
        w.flag(true).u(8, 0); // seq_scaling_matrix_present_flag and 8 lists absent
        w.ue(0).ue(0).ue(0).ue(1).flag(false);
        w.ue(3).ue(2).flag(true).flag(true).flag(false).flag(false);
        let (_, sps) = parse_sps(&w.nal(&[0x67]))?;
        assert_eq!(
            sps.coded_size,
            FrameSize {
                width: 64,
                height: 48
            }
        );
        Ok(())
    }

    /// 7.4.2.1.1.1: delta_scale is -128 to +127. An SPS whose first scaling
    /// list starts with a delta_scale at a bound reads, and one a step past
    /// either bound, or at the review's +-(2^31 - 1), is refused by name
    /// rather than overflowing lastScale + delta_scale.
    #[test]
    fn delta_scale_is_bounded_by_7_4_2_1_1_1() {
        let sps_with = |delta: i64| {
            let mut w = Writer::default();
            w.u(8, 100)
                .u(8, 0)
                .u(8, 10)
                .ue(0)
                .ue(1)
                .ue(0)
                .ue(0)
                .flag(false);
            w.flag(true).flag(true).se(delta);
            for _ in 0..15 {
                w.se(0);
            }
            for _ in 0..7 {
                w.flag(false);
            }
            w.ue(0)
                .ue(0)
                .ue(0)
                .ue(1)
                .flag(false)
                .ue(0)
                .ue(0)
                .flag(true)
                .flag(true)
                .flag(false)
                .flag(false);
            w.nal(&[0x67])
        };
        for delta in [-128, 127] {
            assert!(parse_sps(&sps_with(delta)).is_ok(), "{delta}");
        }
        for delta in [-129, 128, (1 << 31) - 1, -((1 << 31) - 1)] {
            assert_eq!(
                parse_sps(&sps_with(delta)).err(),
                Some(VideoError::Malformed {
                    structure: "H.264 sequence parameter set",
                    reason: "delta_scale outside -128 to 127",
                }),
                "{delta}"
            );
        }
    }

    /// One crafted H.264 stream, an SPS, a PPS, an IDR I slice and a P or B slice,
    /// written from 7.3.2.1.1, 7.3.2.2 and 7.3.3 with every field the
    /// parsers bound settable. The defaults are a 16 by 16 High profile
    /// picture with pic_order_cnt_type 0, 4-bit frame_num and lsb, 4
    /// reference frames and every optional field absent. Review pass 2's
    /// f241-p2-craft.py is the row list.
    #[derive(Clone)]
    struct Craft {
        profile: u64,
        chroma: u64,
        bit_depth: u64,
        scaling: Option<Vec<i64>>,
        log2_frame_num: u64,
        poc_type: u64,
        log2_lsb: u64,
        type_1: Option<(i64, i64, Vec<i64>)>,
        always_zero: bool,
        refs: u64,
        width: u64,
        height: u64,
        frame_mbs_only: bool,
        crop: Option<[u64; 4]>,
        pps_first: bool,
        slice_groups: u64,
        weighted: bool,
        qp: i64,
        qs: i64,
        chroma_qp: i64,
        redundant: bool,
        bottom: bool,
        first_mb: u64,
        idr_id: u64,
        override_l0: Option<u64>,
        override_l1: Option<u64>,
        b_slice: bool,
        b_reference: bool,
        bipred: bool,
        kind: Option<u64>,
        pps_id: u64,
        slice_pps_id: Option<u64>,
        separate_planes: Option<u64>,
        idr_no_output: bool,
        idr_qp_delta: i64,
        qp_delta: i64,
        ref_idc: Option<u8>,
        redundant_cnt: u64,
        delta_bottom: i64,
        delta: [i64; 2],
        modification: Option<Vec<(u64, Option<u64>)>>,
        modification_l1: Option<Vec<(u64, Option<u64>)>>,
        weights: (u64, u64, Vec<(i64, i64)>),
        weights_l1: Vec<(i64, i64)>,
        mmco: Option<Vec<Vec<u64>>>,
    }

    impl Default for Craft {
        fn default() -> Self {
            Self {
                profile: 100,
                chroma: 1,
                bit_depth: 0,
                scaling: None,
                log2_frame_num: 0,
                poc_type: 0,
                log2_lsb: 0,
                type_1: None,
                always_zero: false,
                refs: 4,
                width: 0,
                height: 0,
                frame_mbs_only: true,
                crop: None,
                pps_first: false,
                slice_groups: 0,
                weighted: false,
                qp: 0,
                qs: 0,
                chroma_qp: 0,
                redundant: false,
                bottom: false,
                first_mb: 0,
                idr_id: 0,
                override_l0: None,
                override_l1: None,
                b_slice: false,
                b_reference: false,
                bipred: false,
                kind: None,
                pps_id: 0,
                slice_pps_id: None,
                separate_planes: None,
                idr_no_output: false,
                idr_qp_delta: 0,
                qp_delta: 0,
                ref_idc: None,
                redundant_cnt: 0,
                delta_bottom: 0,
                delta: [0; 2],
                modification: None,
                modification_l1: None,
                weights: (0, 0, Vec::new()),
                weights_l1: Vec::new(),
                mmco: None,
            }
        }
    }

    impl Craft {
        fn sps(&self) -> Vec<u8> {
            let mut w = Writer::default();
            w.u(8, self.profile).u(8, 0).u(8, 40).ue(0);
            if self.profile == 100 {
                w.ue(self.chroma);
                if self.chroma == 3 {
                    w.flag(self.separate_planes.is_some()); // separate_colour_plane_flag
                }
                w.ue(self.bit_depth).ue(self.bit_depth).flag(false);
                match &self.scaling {
                    None => {
                        w.flag(false);
                    }
                    Some(deltas) => {
                        w.flag(true).flag(true);
                        for delta in deltas {
                            w.se(*delta);
                        }
                        // The other lists absent, 8 in all, or 12 for 4:4:4.
                        w.u(if self.chroma == 3 { 11 } else { 7 }, 0);
                    }
                }
            }
            w.ue(self.log2_frame_num).ue(self.poc_type);
            match (self.poc_type, &self.type_1) {
                (0, _) => {
                    w.ue(self.log2_lsb);
                }
                (1, Some((non_ref, top_to_bottom, offsets))) => {
                    w.flag(self.always_zero).se(*non_ref).se(*top_to_bottom);
                    w.ue(u64::try_from(offsets.len()).unwrap_or_default());
                    for offset in offsets {
                        w.se(*offset);
                    }
                }
                _ => {}
            }
            w.ue(self.refs)
                .flag(false)
                .ue(self.width)
                .ue(self.height)
                .flag(self.frame_mbs_only);
            if !self.frame_mbs_only {
                w.flag(false);
            }
            w.flag(true);
            match self.crop {
                None => {
                    w.flag(false);
                }
                Some(crop) => {
                    w.flag(true);
                    for offset in crop {
                        w.ue(offset);
                    }
                }
            }
            w.flag(false);
            w.nal(&[0x67])
        }

        fn pps(&self) -> Vec<u8> {
            let mut w = Writer::default();
            w.ue(self.pps_id)
                .ue(0)
                .flag(false)
                .flag(self.bottom)
                .ue(self.slice_groups);
            if self.slice_groups > 0 {
                w.ue(0);
                for _ in 0..=self.slice_groups {
                    w.ue(0);
                }
            }
            w.ue(0)
                .ue(0)
                .flag(self.weighted)
                .u(2, u64::from(self.bipred))
                .se(self.qp)
                .se(self.qs)
                .se(self.chroma_qp);
            w.flag(false).flag(false).flag(self.redundant);
            w.nal(&[0x68])
        }

        fn slice(&self, idr: bool) -> Vec<u8> {
            let mut w = Writer::default();
            // Table 7-6: slice_type 7 is I, 5 is P and 6 is B, all slices of
            // the picture alike, and `kind` sets the second picture's.
            let slice_type = if idr {
                7
            } else {
                self.kind.unwrap_or(if self.b_slice { 6 } else { 5 })
            };
            let class = slice_type % 5;
            let (p, b, i, sp, si) = (class == 0, class == 1, class == 2, class == 3, class == 4);
            w.ue(self.first_mb)
                .ue(slice_type)
                .ue(self.slice_pps_id.unwrap_or(self.pps_id));
            if let Some(plane) = self.separate_planes {
                w.u(2, plane); // colour_plane_id
            }
            w.u(
                u32::try_from(self.log2_frame_num + 4).unwrap_or(4),
                u64::from(!idr),
            );
            if !self.frame_mbs_only {
                w.flag(false);
            }
            if idr {
                w.ue(self.idr_id);
            }
            // 8.2.1: Min( TopFieldOrderCnt, BottomFieldOrderCnt ) of an IDR
            // frame is 0, so the IDR carries deltas that give both counts 0
            // and the rows' deltas go in the second picture only. For type 1
            // the IDR's expectedPicOrderCnt is 0 by 8-6, its
            // TopFieldOrderCnt is delta_pic_order_cnt[ 0 ] and its
            // BottomFieldOrderCnt adds offset_for_top_to_bottom_field and
            // delta_pic_order_cnt[ 1 ], 8-7 and 8-8.
            if self.poc_type == 0 {
                w.u(
                    u32::try_from(self.log2_lsb + 4).unwrap_or(4),
                    if idr { 0 } else { 2 },
                );
                if self.bottom {
                    w.se(if idr { 0 } else { self.delta_bottom });
                }
            } else if self.poc_type == 1 && !self.always_zero {
                let top_to_bottom = self.type_1.as_ref().map_or(0, |(_, offset, _)| *offset);
                w.se(if idr { 0 } else { self.delta[0] });
                if self.bottom {
                    w.se(if idr {
                        -top_to_bottom.min(0)
                    } else {
                        self.delta[1]
                    });
                }
            }
            if self.redundant {
                w.ue(self.redundant_cnt);
            }
            if b {
                w.flag(true); // direct_spatial_mv_pred_flag
            }
            if p || sp || b {
                if self.override_l0.is_some() || self.override_l1.is_some() {
                    w.flag(true).ue(self.override_l0.unwrap_or(0));
                    if b {
                        w.ue(self.override_l1.unwrap_or(0));
                    }
                } else {
                    w.flag(false);
                }
            }
            // 7.3.3.1: list 0's modification for every slice but I and SI,
            // then list 1's for a B slice.
            let lists = match (i || si, b) {
                (true, _) => vec![],
                (false, true) => vec![&self.modification, &self.modification_l1],
                (false, false) => vec![&self.modification],
            };
            for list in lists {
                match list {
                    None => {
                        w.flag(false);
                    }
                    Some(entries) => {
                        w.flag(true);
                        for (idc, value) in entries {
                            w.ue(*idc);
                            if let Some(value) = value {
                                w.ue(*value);
                            }
                        }
                        w.ue(3);
                    }
                }
            }
            // 7.3.3: a P or SP slice has pred_weight_table when
            // weighted_pred_flag is 1, and a B slice when weighted_bipred_idc
            // is 1. 7.3.3.2 then gives list 0's entries, and list 1's for a B
            // slice. ChromaArrayType 0 has no chroma fields.
            if (self.weighted && (p || sp)) || (self.bipred && b) {
                let (luma, chroma, entries) = &self.weights;
                w.ue(*luma);
                if self.chroma != 0 {
                    w.ue(*chroma);
                }
                let l1: &[(i64, i64)] = if b { &self.weights_l1 } else { &[] };
                for (weight, offset) in entries.iter().chain(l1) {
                    w.flag(true).se(*weight).se(*offset);
                    if self.chroma != 0 {
                        w.flag(false);
                    }
                }
            }
            // 7.3.3.3, present when nal_ref_idc is not 0, which every slice
            // here has except a B slice not marked `b_reference`.
            if idr {
                w.flag(self.idr_no_output).flag(false);
            } else if b && !self.b_reference {
                // No dec_ref_pic_marking.
            } else if let Some(operations) = &self.mmco {
                w.flag(true);
                for operation in operations {
                    for value in operation {
                        w.ue(*value);
                    }
                }
                w.ue(0);
            } else {
                w.flag(false);
            }
            // slice_qp_delta, then sp_for_switch_flag for SP and
            // slice_qs_delta for SP and SI, then a byte of slice data. CAVLC
            // and no deblocking control, so nothing else of 7.3.3 is present.
            w.se(if idr {
                self.idr_qp_delta
            } else {
                self.qp_delta
            });
            if sp {
                w.flag(false);
            }
            if sp || si {
                w.se(0);
            }
            w.u(8, 0x55);
            w.nal(&[match (idr, b) {
                (true, _) => 0x65,
                (false, true) if self.b_reference => 0x21,
                (false, true) => 0x01,
                (false, false) => (self.ref_idc.unwrap_or(2) << 5) | 0x01,
            }])
        }

        fn stream(&self) -> Vec<u8> {
            let (first, second) = if self.pps_first {
                (self.pps(), self.sps())
            } else {
                (self.sps(), self.pps())
            };
            annex_b(&[first, second, self.slice(true), self.slice(false)])
        }
    }

    /// Review pass 2 S1: every range the H.264 parsers check, at its bound,
    /// accepted, and one step past it, refused by its reason. A row's
    /// expectation is `None` for accepted or the text the refusal carries.
    /// The clause of each bound is named in its row.
    #[test]
    fn every_h264_bound_is_exact() {
        let big = (1i64 << 31) - 1;
        let base = Craft::default;
        let reference_b = || Craft {
            b_slice: true,
            b_reference: true,
            bipred: true,
            modification_l1: Some(vec![(0, Some(3))]),
            weights: (2, 1, vec![(3, -2)]),
            weights_l1: vec![(-4, 5)],
            bottom: true,
            delta_bottom: -15,
            mmco: Some(vec![vec![5]]),
            ..Craft::default()
        };
        let rows: Vec<(&str, Craft, Option<&str>)> = vec![
            ("base", base(), None),
            // 7.4.2.1.1: bit_depth_luma_minus8 0 to 6.
            (
                "bit depth 6",
                Craft {
                    bit_depth: 6,
                    ..base()
                },
                None,
            ),
            (
                "bit depth 7",
                Craft {
                    bit_depth: 7,
                    ..base()
                },
                Some("bit_depth_luma_minus8 over 6"),
            ),
            // 7.4.2.1.1 with A.3.1 h: max_num_ref_frames 0 to 16.
            ("refs 16", Craft { refs: 16, ..base() }, None),
            (
                "refs 17",
                Craft { refs: 17, ..base() },
                Some("max_num_ref_frames over 16"),
            ),
            // A.3.1 e to g at MaxFS 139 264: each side 1 055, area 139 264.
            (
                "width 1055",
                Craft {
                    width: 1054,
                    ..base()
                },
                None,
            ),
            (
                "width 1056",
                Craft {
                    width: 1055,
                    ..base()
                },
                Some("pic_width_in_mbs_minus1 over 1054"),
            ),
            (
                "height 1055",
                Craft {
                    height: 1054,
                    ..base()
                },
                None,
            ),
            (
                "height 1056",
                Craft {
                    height: 1055,
                    ..base()
                },
                Some("pic_height_in_map_units_minus1 over 1054"),
            ),
            // A.3.1 e: the product at exactly MaxFS, 1 024 by 136.
            (
                "area 139 264",
                Craft {
                    width: 1023,
                    height: 135,
                    ..base()
                },
                None,
            ),
            (
                "area 140 315",
                Craft {
                    width: 1054,
                    height: 132,
                    ..base()
                },
                Some("larger than any level"),
            ),
            (
                "field height 1054",
                Craft {
                    frame_mbs_only: false,
                    height: 526,
                    ..base()
                },
                None,
            ),
            (
                "field height 1056",
                Craft {
                    frame_mbs_only: false,
                    height: 527,
                    ..base()
                },
                Some("larger than any level"),
            ),
            // 7.4.2.1.1: the crop window leaves at least one crop unit, CropUnit
            // 2 for 4:2:0, so a 32 by 32 picture keeps 15 + 0 + 1 = 16 units.
            (
                "crop 15",
                Craft {
                    width: 1,
                    height: 1,
                    crop: Some([15, 0, 15, 0]),
                    ..base()
                },
                None,
            ),
            (
                "crop 16",
                Craft {
                    width: 1,
                    height: 1,
                    crop: Some([16, 0, 0, 0]),
                    ..base()
                },
                Some("cropping window outside"),
            ),
            (
                "crop 7 and 8",
                Craft {
                    width: 1,
                    height: 1,
                    crop: Some([7, 8, 7, 8]),
                    ..base()
                },
                None,
            ),
            (
                "crop 8 and 8",
                Craft {
                    width: 1,
                    height: 1,
                    crop: Some([8, 8, 0, 0]),
                    ..base()
                },
                Some("cropping window outside"),
            ),
            (
                "crop 2^32 - 2",
                Craft {
                    crop: Some([u64::from(u32::MAX) - 1, 0, 0, 0]),
                    ..base()
                },
                Some("cropping window outside"),
            ),
            (
                "crop 2^32 - 1",
                Craft {
                    crop: Some([u64::from(u32::MAX), 0, 0, 0]),
                    ..base()
                },
                Some("longer than 32 bits"),
            ),
            // 7.4.2.1.1.1: delta_scale -128 to 127.
            (
                "delta_scale -128",
                Craft {
                    scaling: Some([vec![-128], vec![0; 15]].concat()),
                    ..base()
                },
                None,
            ),
            (
                "delta_scale 127",
                Craft {
                    scaling: Some([vec![127], vec![0; 15]].concat()),
                    ..base()
                },
                None,
            ),
            (
                "delta_scale -129",
                Craft {
                    scaling: Some([vec![-129], vec![0; 15]].concat()),
                    ..base()
                },
                Some("delta_scale outside"),
            ),
            (
                "delta_scale 128",
                Craft {
                    scaling: Some([vec![128], vec![0; 15]].concat()),
                    ..base()
                },
                Some("delta_scale outside"),
            ),
            // 7.4.2.1.1 and 7.4.3: the POC type 1 offsets and deltas are
            // -2^31 + 1 to 2^31 - 1, which is all se(v) gives, so 2^31 is
            // refused by the reader.
            (
                "type 1 at 2^31 - 1",
                Craft {
                    poc_type: 1,
                    type_1: Some((big, big, vec![big, big])),
                    bottom: true,
                    delta: [big, big],
                    ..base()
                },
                None,
            ),
            (
                "type 1 at -(2^31 - 1)",
                Craft {
                    poc_type: 1,
                    type_1: Some((-big, -big, vec![-big; 255])),
                    bottom: true,
                    delta: [-big, -big],
                    ..base()
                },
                None,
            ),
            (
                "type 1 at 2^31",
                Craft {
                    poc_type: 1,
                    type_1: Some((big + 1, 0, vec![1])),
                    ..base()
                },
                Some("longer than 32 bits"),
            ),
            // 7.4.2.2: pic_init_qp_minus26 -( 26 + 6 * bit_depth_luma_minus8 )
            // to 25 against the active SPS, pic_init_qs_minus26 -26 to 25 and
            // chroma_qp_index_offset -12 to 12.
            ("qp -26 at 8 bits", Craft { qp: -26, ..base() }, None),
            (
                "qp -27 at 8 bits",
                Craft { qp: -27, ..base() },
                Some("pic_init_qp_minus26 below"),
            ),
            (
                "qp -38 at 10 bits",
                Craft {
                    bit_depth: 2,
                    qp: -38,
                    ..base()
                },
                None,
            ),
            (
                "qp -39 at 10 bits",
                Craft {
                    bit_depth: 2,
                    qp: -39,
                    ..base()
                },
                Some("pic_init_qp_minus26 below"),
            ),
            (
                "qp -62 at 14 bits",
                Craft {
                    bit_depth: 6,
                    qp: -62,
                    ..base()
                },
                None,
            ),
            (
                "qp -63 at 14 bits",
                Craft {
                    bit_depth: 6,
                    qp: -63,
                    ..base()
                },
                Some("pic_init_qp_minus26 out of range"),
            ),
            ("qp 25", Craft { qp: 25, ..base() }, None),
            (
                "qp 26",
                Craft { qp: 26, ..base() },
                Some("pic_init_qp_minus26 out of range"),
            ),
            ("qs -26", Craft { qs: -26, ..base() }, None),
            ("qs 25", Craft { qs: 25, ..base() }, None),
            (
                "qs -27",
                Craft { qs: -27, ..base() },
                Some("pic_init_qs_minus26 out of range"),
            ),
            (
                "qs 26",
                Craft { qs: 26, ..base() },
                Some("pic_init_qs_minus26 out of range"),
            ),
            (
                "chroma qp 12",
                Craft {
                    chroma_qp: 12,
                    ..base()
                },
                None,
            ),
            (
                "chroma qp -12",
                Craft {
                    chroma_qp: -12,
                    ..base()
                },
                None,
            ),
            (
                "chroma qp 13",
                Craft {
                    chroma_qp: 13,
                    ..base()
                },
                Some("chroma_qp_index_offset out of range"),
            ),
            (
                "chroma qp -13",
                Craft {
                    chroma_qp: -13,
                    ..base()
                },
                Some("chroma_qp_index_offset out of range"),
            ),
            // A.2.1 and A.2.4: num_slice_groups_minus1 0 to 7, then refused by
            // name as a High profile excludes it.
            (
                "slice groups 7",
                Craft {
                    profile: 66,
                    slice_groups: 7,
                    ..base()
                },
                Some("slice groups is not read"),
            ),
            (
                "slice groups 8",
                Craft {
                    profile: 66,
                    slice_groups: 8,
                    ..base()
                },
                Some("num_slice_groups_minus1 over 7"),
            ),
            // 7.4.1.2.1: a PPS may arrive before its SPS.
            (
                "PPS before SPS",
                Craft {
                    pps_first: true,
                    ..base()
                },
                None,
            ),
            // 7.4.3: first_mb_in_slice below PicSizeInMbs, 4 for 32 by 32.
            (
                "first_mb 3",
                Craft {
                    width: 1,
                    height: 1,
                    first_mb: 3,
                    ..base()
                },
                None,
            ),
            (
                "first_mb 4",
                Craft {
                    width: 1,
                    height: 1,
                    first_mb: 4,
                    ..base()
                },
                Some("first_mb_in_slice past"),
            ),
            // 7.4.3: idr_pic_id 0 to 65 535, redundant_pic_cnt 0 to 127,
            // num_ref_idx_l0_active_minus1 0 to 15 for a frame.
            (
                "idr_pic_id 65535",
                Craft {
                    idr_id: 65_535,
                    ..base()
                },
                None,
            ),
            (
                "idr_pic_id 65536",
                Craft {
                    idr_id: 65_536,
                    ..base()
                },
                Some("idr_pic_id over 65535"),
            ),
            (
                "redundant 127",
                Craft {
                    profile: 66,
                    redundant: true,
                    redundant_cnt: 127,
                    ..base()
                },
                None,
            ),
            (
                "redundant 128",
                Craft {
                    profile: 66,
                    redundant: true,
                    redundant_cnt: 128,
                    ..base()
                },
                Some("redundant_pic_cnt over 127"),
            ),
            (
                "override 15",
                Craft {
                    override_l0: Some(15),
                    ..base()
                },
                None,
            ),
            (
                "override 16",
                Craft {
                    override_l0: Some(16),
                    ..base()
                },
                Some("num_ref_idx_l0_active_minus1 over 15"),
            ),
            // The same bound for list 1, in a B slice, 7.3.3.
            (
                "override l1 15",
                Craft {
                    b_slice: true,
                    override_l1: Some(15),
                    ..base()
                },
                None,
            ),
            (
                "override l1 16",
                Craft {
                    b_slice: true,
                    override_l1: Some(16),
                    ..base()
                },
                Some("num_ref_idx_l1_active_minus1 over 15"),
            ),
            (
                "override l0 16 in a B slice",
                Craft {
                    b_slice: true,
                    override_l0: Some(16),
                    ..base()
                },
                Some("num_ref_idx_l0_active_minus1 over 15"),
            ),
            // The pass 5 sweep. 7.4.3: slice_type is 0 to 9, and 9 is SI,
            // which the Extended profile carries. pic_parameter_set_id is 0 to
            // 255, in the slice as in the PPS.
            (
                "slice_type 9",
                Craft {
                    profile: 88,
                    kind: Some(9),
                    ..base()
                },
                None,
            ),
            (
                "slice_type 10",
                Craft {
                    profile: 88,
                    kind: Some(10),
                    ..base()
                },
                Some("slice_type over 9"),
            ),
            (
                "pps id 255",
                Craft {
                    pps_id: 255,
                    ..base()
                },
                None,
            ),
            (
                "slice pps id 256",
                Craft {
                    pps_id: 255,
                    slice_pps_id: Some(256),
                    ..base()
                },
                Some("pic_parameter_set_id over 255"),
            ),
            // S1 of review pass 4: list 1's modification, 7.3.3.1, with
            // abs_diff_pic_num_minus1 0 to MaxPicNum - 1 by 7.4.3.1, here 15.
            // Each S1 row is the reference B slice below, so both weight
            // loops and the marking follow the modification.
            (
                "l1 abs_diff 15",
                Craft {
                    modification_l1: Some(vec![(0, Some(15))]),
                    ..reference_b()
                },
                None,
            ),
            (
                "l1 abs_diff 16",
                Craft {
                    modification_l1: Some(vec![(0, Some(16))]),
                    ..reference_b()
                },
                Some("picture number difference"),
            ),
            // 7.3.3.2's list 1 loop under weighted_bipred_idc 1, each weight
            // and offset -128 to 127 by 7.4.3.2.
            (
                "l1 weight 127",
                Craft {
                    weights_l1: vec![(127, -128)],
                    ..reference_b()
                },
                None,
            ),
            (
                "l1 weight 128",
                Craft {
                    weights_l1: vec![(128, 0)],
                    ..reference_b()
                },
                Some("prediction weight"),
            ),
            (
                "l1 offset -129",
                Craft {
                    weights_l1: vec![(0, -129)],
                    ..reference_b()
                },
                Some("prediction weight"),
            ),
            // A reference B slice, nal_ref_idc 1, whose dec_ref_pic_marking
            // follows both list modifications and both weight loops. Its
            // memory_management_control_operation 5 sets the bound of 7.4.3
            // on delta_pic_order_cnt_bottom, 1 - 16, so the refusal at -16 is
            // reached only through a field for field read.
            ("reference B bottom -15 with mmco 5", reference_b(), None),
            // S1 of review pass 5: 7.3.3.2 runs list 1's loop
            // num_ref_idx_l1_active_minus1 + 1 times, here 2 against list 0's
            // 1, and the second l1 entry carries the refusal.
            (
                "reference B l1 count 2",
                Craft {
                    override_l0: Some(0),
                    override_l1: Some(1),
                    weights_l1: vec![(-4, 5), (6, -7)],
                    ..reference_b()
                },
                None,
            ),
            (
                "reference B l1 count 2, weight 128",
                Craft {
                    override_l0: Some(0),
                    override_l1: Some(1),
                    weights_l1: vec![(-4, 5), (128, 0)],
                    ..reference_b()
                },
                Some("prediction weight"),
            ),
            (
                "reference B bottom -16 with mmco 5",
                Craft {
                    delta_bottom: -16,
                    ..reference_b()
                },
                Some("delta_pic_order_cnt_bottom out of range"),
            ),
            // N2 of review pass 3: separate_colour_plane_flag for 4:4:4 with
            // 12 scaling lists, and no chroma weight fields for monochrome.
            (
                "4:4:4 with scaling lists",
                Craft {
                    chroma: 3,
                    scaling: Some([vec![127], vec![0; 15]].concat()),
                    ..base()
                },
                None,
            ),
            (
                "monochrome weighted",
                Craft {
                    chroma: 0,
                    weighted: true,
                    weights: (7, 0, vec![(-128, 127)]),
                    ..base()
                },
                None,
            ),
            // 7.4.3: delta_pic_order_cnt_bottom is at least 1 - MaxPicOrderCntLsb
            // with a memory_management_control_operation 5, here 1 - 16.
            (
                "bottom -15 with mmco 5",
                Craft {
                    bottom: true,
                    delta_bottom: -15,
                    mmco: Some(vec![vec![5]]),
                    ..base()
                },
                None,
            ),
            (
                "bottom -16 with mmco 5",
                Craft {
                    bottom: true,
                    delta_bottom: -16,
                    mmco: Some(vec![vec![5]]),
                    ..base()
                },
                Some("delta_pic_order_cnt_bottom out of range"),
            ),
            (
                "bottom -16 without",
                Craft {
                    bottom: true,
                    delta_bottom: -16,
                    ..base()
                },
                None,
            ),
            (
                "bottom at -(2^31 - 1)",
                Craft {
                    bottom: true,
                    delta_bottom: -big,
                    ..base()
                },
                None,
            ),
            // 7.4.3.1: abs_diff_pic_num_minus1 0 to MaxPicNum - 1, here 15.
            (
                "abs_diff 15",
                Craft {
                    modification: Some(vec![(0, Some(15))]),
                    ..base()
                },
                None,
            ),
            (
                "abs_diff 15, idc 1",
                Craft {
                    modification: Some(vec![(1, Some(15))]),
                    ..base()
                },
                None,
            ),
            (
                "abs_diff 16",
                Craft {
                    modification: Some(vec![(0, Some(16))]),
                    ..base()
                },
                Some("picture number difference"),
            ),
            (
                "abs_diff 65535 at 16 bits",
                Craft {
                    log2_frame_num: 12,
                    modification: Some(vec![(0, Some(65_535))]),
                    ..base()
                },
                None,
            ),
            (
                "abs_diff 65536 at 16 bits",
                Craft {
                    log2_frame_num: 12,
                    modification: Some(vec![(0, Some(65_536))]),
                    ..base()
                },
                Some("picture number difference"),
            ),
            // 8.2.4.1 and 7.4.3.3: a long-term number is below max_num_ref_frames.
            (
                "long_term_pic_num 3",
                Craft {
                    modification: Some(vec![(2, Some(3))]),
                    ..base()
                },
                None,
            ),
            (
                "long_term_pic_num 4",
                Craft {
                    modification: Some(vec![(2, Some(4))]),
                    ..base()
                },
                Some("long-term index"),
            ),
            (
                "long-term with 0 refs",
                Craft {
                    refs: 0,
                    modification: Some(vec![(2, Some(0))]),
                    ..base()
                },
                Some("max_num_ref_frames 0"),
            ),
            // Table 7-7: idc 4 and 5 are MVC, over 5 is malformed.
            (
                "idc 4",
                Craft {
                    modification: Some(vec![(4, Some(0))]),
                    ..base()
                },
                Some("the MVC values"),
            ),
            (
                "idc 6",
                Craft {
                    modification: Some(vec![(6, Some(0))]),
                    ..base()
                },
                Some("modification_of_pic_nums_idc over 5"),
            ),
            // 7.4.3.2: denominators 0 to 7, weights and offsets -128 to 127.
            (
                "weights at bounds",
                Craft {
                    weighted: true,
                    weights: (7, 7, vec![(-128, 127)]),
                    ..base()
                },
                None,
            ),
            (
                "luma denominator 8",
                Craft {
                    weighted: true,
                    weights: (8, 0, vec![(0, 0)]),
                    ..base()
                },
                Some("luma_log2_weight_denom over 7"),
            ),
            (
                "chroma denominator 8",
                Craft {
                    weighted: true,
                    weights: (0, 8, vec![(0, 0)]),
                    ..base()
                },
                Some("chroma_log2_weight_denom over 7"),
            ),
            (
                "weight 128",
                Craft {
                    weighted: true,
                    weights: (0, 0, vec![(128, 0)]),
                    ..base()
                },
                Some("prediction weight"),
            ),
            (
                "offset -129",
                Craft {
                    weighted: true,
                    weights: (0, 0, vec![(0, -129)]),
                    ..base()
                },
                Some("prediction weight"),
            ),
            (
                "weight -129",
                Craft {
                    weighted: true,
                    weights: (0, 0, vec![(-129, 0)]),
                    ..base()
                },
                Some("prediction weight"),
            ),
            (
                "offset 128",
                Craft {
                    weighted: true,
                    weights: (0, 0, vec![(0, 128)]),
                    ..base()
                },
                Some("prediction weight"),
            ),
            // Table 7-9 and 7.4.3.3.
            (
                "mmco 1 at 15",
                Craft {
                    mmco: Some(vec![vec![1, 15]]),
                    ..base()
                },
                None,
            ),
            (
                "mmco 1 at 16",
                Craft {
                    mmco: Some(vec![vec![1, 16]]),
                    ..base()
                },
                Some("picture number difference"),
            ),
            (
                "mmco 3 at 15 and 3",
                Craft {
                    mmco: Some(vec![vec![3, 15, 3]]),
                    ..base()
                },
                None,
            ),
            (
                "mmco 3 at 15 and 4",
                Craft {
                    mmco: Some(vec![vec![3, 15, 4]]),
                    ..base()
                },
                Some("long-term index"),
            ),
            (
                "mmco 4 at 4",
                Craft {
                    mmco: Some(vec![vec![4, 4]]),
                    ..base()
                },
                None,
            ),
            (
                "mmco 4 at 5",
                Craft {
                    mmco: Some(vec![vec![4, 5]]),
                    ..base()
                },
                Some("max_long_term_frame_idx_plus1"),
            ),
            (
                "mmco 6 at 3",
                Craft {
                    mmco: Some(vec![vec![6, 3]]),
                    ..base()
                },
                None,
            ),
            (
                "mmco 6 at 4",
                Craft {
                    mmco: Some(vec![vec![6, 4]]),
                    ..base()
                },
                Some("long-term index"),
            ),
            (
                "mmco 2 with 0 refs",
                Craft {
                    refs: 0,
                    mmco: Some(vec![vec![2, 0]]),
                    ..base()
                },
                Some("max_num_ref_frames 0"),
            ),
            (
                "mmco 7",
                Craft {
                    mmco: Some(vec![vec![7, 0]]),
                    ..base()
                },
                Some("memory_management_control_operation over 6"),
            ),
        ];
        let mut failures = Vec::new();
        for (name, craft, expected) in rows {
            crate::video::bits::dump_row("h264", name, &craft.stream());
            let outcome = access_units(&craft.stream())
                .map(|_| ())
                .map_err(|error| error.to_string());
            let matches = match (&outcome, expected) {
                (Ok(()), None) => true,
                (Err(text), Some(reason)) => text.contains(reason),
                _ => false,
            };
            if !matches {
                failures.push(format!("{name}: {outcome:?}, expected {expected:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// 7.3.3.2: with ChromaArrayType 0 a pred_weight_table has no
    /// chroma_log2_weight_denom and no chroma weight flags. A monochrome P
    /// slice with one luma weight, then dec_ref_pic_marking carrying a
    /// memory_management_control_operation 5, is read field for field only
    /// if no chroma field is read, and the operation 5 then puts the P picture
    /// in segment 1 at POC 0, 8.2.1. The same slice in 4:2:0, with the chroma
    /// fields written, reads the same way.
    #[test]
    fn monochrome_weights_have_no_chroma_fields() -> Result<(), VideoError> {
        for chroma in [0, 1] {
            let craft = Craft {
                chroma,
                weighted: true,
                weights: (3, 2, vec![(5, -3)]),
                mmco: Some(vec![vec![5]]),
                ..Craft::default()
            };
            let (found, _) = access_units(&craft.stream())?;
            let orders: Vec<(u32, i64)> = found.iter().map(|unit| unit.order).collect();
            assert_eq!(orders, [(0, 0), (1, 0)], "chroma_format_idc {chroma}");
        }
        Ok(())
    }

    /// Review pass 4 S1, 8.2.1: a reference B slice, nal_ref_idc 1, carrying
    /// a list 1 modification, both weight loops of 7.3.3.2 under
    /// weighted_bipred_idc 1 and then a memory_management_control_operation
    /// 5. Its pic_order_cnt_lsb is 2, so with the operation read it starts
    /// segment 1 at POC 0, and a read that misses it leaves it in segment 0
    /// at POC 2.
    #[test]
    fn reference_b_slice_reads_through_to_its_marking() -> Result<(), VideoError> {
        let craft = Craft {
            b_slice: true,
            b_reference: true,
            bipred: true,
            modification_l1: Some(vec![(0, Some(3)), (1, Some(7))]),
            weights: (2, 1, vec![(3, -2)]),
            weights_l1: vec![(-4, 5)],
            mmco: Some(vec![vec![5]]),
            ..Craft::default()
        };
        let (found, _) = access_units(&craft.stream())?;
        let orders: Vec<(u32, i64)> = found.iter().map(|unit| unit.order).collect();
        assert_eq!(orders, [(0, 0), (1, 0)]);
        let without = Craft {
            mmco: None,
            ..craft
        };
        let (found, _) = access_units(&without.stream())?;
        let orders: Vec<(u32, i64)> = found.iter().map(|unit| unit.order).collect();
        assert_eq!(orders, [(0, 0), (0, 2)]);
        Ok(())
    }

    /// The pass 5 sweep: every presence condition of 7.3.3 to 7.3.3.3 that
    /// no other row turns on, each in a stream whose second picture ends its
    /// dec_ref_pic_marking with a memory_management_control_operation 5.
    /// That operation is the last field read, so it is found only when every
    /// earlier field is read at its own length, and it puts the picture at
    /// segment 1 and POC 0, 8.2.1. Each row names the condition it turns on.
    #[test]
    fn every_presence_condition_reaches_the_marking() {
        let five = || Craft {
            mmco: Some(vec![vec![5]]),
            ..Craft::default()
        };
        let rows: Vec<(&str, Craft)> = vec![
            ("base", five()),
            // separate_colour_plane_flag 1 puts colour_plane_id u(2) before
            // frame_num.
            (
                "colour_plane_id 2",
                Craft {
                    chroma: 3,
                    separate_planes: Some(2),
                    ..five()
                },
            ),
            // frame_mbs_only_flag 0 puts field_pic_flag after frame_num.
            (
                "field_pic_flag 0",
                Craft {
                    frame_mbs_only: false,
                    ..five()
                },
            ),
            // pic_order_cnt_type 1 with delta_pic_order_always_zero_flag 0
            // reads delta_pic_order_cnt[ 0 ], and [ 1 ] with
            // bottom_field_pic_order_in_frame_present_flag.
            (
                "delta_pic_order_cnt 3 and -2",
                Craft {
                    poc_type: 1,
                    type_1: Some((0, 0, vec![2])),
                    bottom: true,
                    delta: [3, -2],
                    ..five()
                },
            ),
            // Review pass 6 S1, the other direction of the two type 1
            // conditions: with bottom_field_pic_order_in_frame_present_flag 0
            // only delta_pic_order_cnt[ 0 ] is present, and with
            // delta_pic_order_always_zero_flag 1 neither is.
            (
                "delta_pic_order_cnt 3, no bottom",
                Craft {
                    poc_type: 1,
                    type_1: Some((0, 0, vec![2])),
                    delta: [3, 0],
                    ..five()
                },
            ),
            (
                "delta_pic_order_always_zero_flag 1",
                Craft {
                    poc_type: 1,
                    type_1: Some((0, 0, vec![2])),
                    always_zero: true,
                    bottom: true,
                    ..five()
                },
            ),
            // An SP slice, slice_type 8, reads the override and
            // pred_weight_table as a P slice does.
            (
                "SP slice, override and weights",
                Craft {
                    profile: 88,
                    kind: Some(8),
                    weighted: true,
                    override_l0: Some(1),
                    weights: (2, 1, vec![(3, -2), (4, 1)]),
                    ..five()
                },
            ),
            // An SI slice, slice_type 9, has no override, no list
            // modification and no pred_weight_table.
            (
                "SI slice",
                Craft {
                    profile: 88,
                    kind: Some(9),
                    ..five()
                },
            ),
            // An I slice outside an IDR picture, slice_type 7, has no list
            // modification and carries adaptive marking.
            (
                "I slice, not IDR",
                Craft {
                    kind: Some(7),
                    ..five()
                },
            ),
            // no_output_of_prior_pics_flag 1 in the IDR. A reader that took
            // it for adaptive_ref_pic_marking_mode_flag would read
            // long_term_reference_flag 0, slice_qp_delta 2, which is 00100,
            // and a 0 of slice data as memory_management_control_operation
            // 0001000, which is 7 and refused.
            (
                "no_output_of_prior_pics_flag 1",
                Craft {
                    idr_no_output: true,
                    idr_qp_delta: 2,
                    ..five()
                },
            ),
            (
                "pps id 255",
                Craft {
                    pps_id: 255,
                    ..five()
                },
            ),
            // Operation 5 has no field of its own, Table 7-9. After its
            // operation 0 comes slice_qp_delta 4, codeNum 7 by Table 9-3, so a
            // reader that took a field for operation 5 would read 0 there
            // and then 7 as a further operation, which is refused.
            (
                "mmco 5 then slice_qp_delta 4",
                Craft {
                    qp_delta: 4,
                    ..five()
                },
            ),
            // nal_ref_idc 3 in a non-IDR P slice, nal_unit_type 1. IdrPicFlag
            // comes from nal_unit_type alone, 7.4.1.
            (
                "nal_ref_idc 3, not IDR",
                Craft {
                    ref_idc: Some(3),
                    ..five()
                },
            ),
        ];
        let mut failures = Vec::new();
        for (name, craft) in rows {
            crate::video::bits::dump_row("h264", &format!("marking {name}"), &craft.stream());
            let outcome = access_units(&craft.stream())
                .map(|(found, _)| found.iter().map(|unit| unit.order).collect::<Vec<_>>());
            if !matches!(&outcome, Ok(orders) if orders == &[(0, 0), (1, 0)]) {
                failures.push(format!("{name}: {outcome:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// Review pass 2 D1's second stream, 7.4.1.2.1: a PPS with
    /// pic_init_qp_minus26 -38 sent after an 8-bit SPS and before a 10-bit SPS
    /// with the same id. The IDR slice activates the 10-bit SPS, for which
    /// -38 is the lower bound, so the stream is accepted. With an 8-bit SPS in
    /// that place the slice refuses it by name.
    #[test]
    fn pic_init_qp_follows_the_sps_the_slice_activates() -> Result<(), VideoError> {
        let eight = Craft {
            qp: -38,
            ..Craft::default()
        };
        let ten = Craft {
            bit_depth: 2,
            ..eight.clone()
        };
        let stream = annex_b(&[eight.sps(), eight.pps(), ten.sps(), ten.slice(true)]);
        assert_eq!(access_units(&stream)?.0.len(), 1);
        let stream = annex_b(&[ten.sps(), eight.pps(), eight.sps(), eight.slice(true)]);
        let error = access_units(&stream).err().ok_or(VideoError::Empty)?;
        assert!(
            error.to_string().contains("pic_init_qp_minus26 below"),
            "{error}"
        );
        Ok(())
    }
}
