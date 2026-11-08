//! HEVC parameter sets, slice segment headers, picture order count and
//! access units.
//!
//! Every syntax table and field width here is from Rec. ITU-T H.265 (V10)
//! (07/2024), the clause named beside each parser. Only base layer NAL units,
//! nuh_layer_id 0, are parsed, as 7.4.2.4.4 NOTE 3 says a decoder of the
//! Annex A profiles does.

use super::{
    FrameSize, VideoError,
    bits::{Bits, to_usize, units},
    h264::{SignalType, cropped, dimension},
    table::{AccessUnit, StreamInfo},
};

/// 7.4.3.2.1, sps_seq_parameter_set_id is 0 to 15.
const SPS_IDS: usize = 16;
/// 7.4.3.3.1, pps_pic_parameter_set_id is 0 to 63.
const PPS_IDS: usize = 64;
/// 7.4.3.2.1, num_short_term_ref_pic_sets is 0 to 64 and
/// num_long_term_ref_pics_sps is 0 to 32.
const MAX_SHORT_TERM_SETS: u32 = 64;
const MAX_LONG_TERM_SPS: u32 = 32;
/// A.4.2: when the level is not level 8.5, A-2 gives MaxDpbSize, and every
/// branch of A-2 is at most 16, its last being maxDpbPicBuf, which is 6 or 7.
/// At level 8.5 A.4.2 sets no MaxDpbSize, so the field has only the reader's
/// own limit.
const MAX_DPB_SIZE: u32 = 16;
/// A.4.1 items b and c with Table A.8's largest MaxLumaPs, 142 606 336 at
/// level 7.2: Sqrt( MaxLumaPs * 8 ) is 33 776.
const MAX_PIC_SIDE: u32 = 33_776;
/// Level 8.5, which A.4.1 leaves without size limits, is general_level_idc
/// 255, thirty times the level number by A.4.1.
const LEVEL_8_5: u8 = 255;
/// Table E.1 via E.3.1, aspect_ratio_idc 255 is EXTENDED_SAR.
const EXTENDED_SAR: u8 = 255;

/// Table 7-1 nal_unit_type values.
mod nal {
    pub(super) const RADL_N: u8 = 6;
    pub(super) const RASL_R: u8 = 9;
    pub(super) const RSV_VCL_N10: u8 = 10;
    pub(super) const RSV_VCL_N14: u8 = 14;
    pub(super) const RSV_VCL_R15: u8 = 15;
    pub(super) const RSV_IRAP_22: u8 = 22;
    pub(super) const BLA_W_LP: u8 = 16;
    pub(super) const BLA_N_LP: u8 = 18;
    pub(super) const IDR_W_RADL: u8 = 19;
    pub(super) const IDR_N_LP: u8 = 20;
    pub(super) const CRA: u8 = 21;
    pub(super) const RSV_IRAP_23: u8 = 23;
    pub(super) const VCL_LAST: u8 = 31;
    pub(super) const VPS: u8 = 32;
    pub(super) const SPS: u8 = 33;
    pub(super) const PPS: u8 = 34;
    pub(super) const AUD: u8 = 35;
    pub(super) const EOS: u8 = 36;
    pub(super) const PREFIX_SEI: u8 = 39;
    pub(super) const RSV_NVCL41: u8 = 41;
    pub(super) const RSV_NVCL44: u8 = 44;
    pub(super) const UNSPEC48: u8 = 48;
    pub(super) const UNSPEC55: u8 = 55;
    pub(super) const RASL_N: u8 = 8;
}

/// The general_profile_tier_level fields of 7.3.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeneralProfile {
    profile_space: u8,
    tier: bool,
    profile_idc: u8,
    compatibility: u32,
    /// general_progressive_source_flag through the reserved bits and
    /// general_inbld_flag, 48 bits in all, as six bytes.
    constraints: [u8; 6],
    level_idc: u8,
}

impl GeneralProfile {
    fn progressive_source(&self) -> bool {
        self.constraints[0] & 0x80 != 0
    }

    fn interlaced_source(&self) -> bool {
        self.constraints[0] & 0x40 != 0
    }

    /// The dotted HEVC codec string the WebCodecs HEVC registration takes
    /// from ISO/IEC 14496-15 Annex E. That annex was not available to read.
    /// This is the published form: the profile space as nothing, A, B or C
    /// with profile_idc in decimal, the compatibility flags in reverse bit
    /// order in hexadecimal, L or H for the tier with level_idc in decimal,
    /// then the constraint bytes in hexadecimal with trailing zero bytes left
    /// out.
    fn codec_string(&self, prefix: &str) -> String {
        let space = match self.profile_space {
            1 => "A",
            2 => "B",
            3 => "C",
            _ => "",
        };
        let tier = if self.tier { 'H' } else { 'L' };
        let mut text = format!(
            "{prefix}.{space}{}.{:X}.{tier}{}",
            self.profile_idc,
            self.compatibility.reverse_bits(),
            self.level_idc
        );
        let kept = self
            .constraints
            .iter()
            .rposition(|value| *value != 0)
            .map_or(0, |last| last + 1);
        for value in self.constraints.iter().take(kept) {
            text.push_str(&format!(".{value:02X}"));
        }
        text
    }
}

/// The fields of one sequence parameter set this module uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sps {
    profile: GeneralProfile,
    separate_colour_plane: bool,
    log2_max_poc_lsb: u8,
    coded_size: FrameSize,
    visible_size: FrameSize,
    signal: Option<SignalType>,
    field_seq: bool,
}

impl Sps {
    pub(crate) fn stream_info(&self, prefix: &str) -> StreamInfo {
        StreamInfo {
            codec_string: Some(self.profile.codec_string(prefix)),
            coded_size: self.coded_size,
            visible_size: self.visible_size,
            colour: self.signal.map(SignalType::colour),
            code_points_signalled: self
                .signal
                .is_some_and(|signal| signal.description.is_some()),
            // E.3.1: field_seq_flag 1 means each picture is a field. 7.4.4:
            // general_progressive_source_flag 0 with
            // general_interlaced_source_flag 1 says the source is interlaced.
            interlaced: self.field_seq
                || (self.profile.interlaced_source() && !self.profile.progressive_source()),
        }
    }
}

/// 7.3.3, profile_tier_level( 1, sps_max_sub_layers_minus1 ).
fn profile_tier_level(
    bits: &mut Bits<'_>,
    sub_layers_minus1: u32,
) -> Result<GeneralProfile, VideoError> {
    let first = bits.byte()?;
    let compatibility = bits.bits(32)?;
    let mut constraints = [0u8; 6];
    for value in &mut constraints {
        *value = bits.byte()?;
    }
    let level_idc = bits.byte()?;
    let mut present = Vec::new();
    for _ in 0..sub_layers_minus1 {
        present.push((bits.flag()?, bits.flag()?));
    }
    if sub_layers_minus1 > 0 {
        bits.skip(2 * (8 - sub_layers_minus1))?; // reserved_zero_2bits
    }
    for (profile_present, level_present) in present {
        if profile_present {
            // sub_layer profile space to sub_layer_inbld_flag: 2 + 1 + 5 + 32
            // + 4 + 43 + 1 bits.
            bits.skip(88)?;
        }
        if level_present {
            bits.skip(8)?; // sub_layer_level_idc
        }
    }
    Ok(GeneralProfile {
        profile_space: first >> 6,
        tier: first & 0x20 != 0,
        profile_idc: first & 0x1F,
        compatibility,
        constraints,
        level_idc,
    })
}

/// 7.3.4, scaling_list_data, read only for its length, with the ranges of
/// 7.4.5.
fn skip_scaling_list_data(bits: &mut Bits<'_>) -> Result<(), VideoError> {
    for size_id in 0..4u32 {
        let step = if size_id == 3 { 3 } else { 1 };
        for matrix_id in (0..6u32).step_by(step) {
            if bits.flag()? {
                let coefficients = 64.min(1u32 << (4 + (size_id << 1)));
                if size_id > 1 {
                    bits.se_range(-7, 247, "scaling_list_dc_coef_minus8 outside -7 to 247")?;
                }
                for _ in 0..coefficients {
                    bits.se_range(-128, 127, "scaling_list_delta_coef outside -128 to 127")?;
                }
            } else {
                // 0 to matrixId for sizeId 0 to 2, and 0 to matrixId / 3 for
                // sizeId 3.
                let max = if size_id == 3 {
                    matrix_id / 3
                } else {
                    matrix_id
                };
                bits.ue_max(max, "scaling_list_pred_matrix_id_delta past matrixId")?;
            }
        }
    }
    Ok(())
}

/// One candidate short-term RPS, as the DeltaPocS0 and DeltaPocS1 lists of
/// 7.4.8, which st_ref_pic_set needs to read the sets predicted from it.
#[derive(Debug, Clone, Default)]
struct ShortTermSet {
    negative: Vec<i64>,
    positive: Vec<i64>,
}

/// 7.3.7, st_ref_pic_set( stRpsIdx ), in an SPS, with 7-59 to 7-70.
fn short_term_set(
    bits: &mut Bits<'_>,
    index: usize,
    sets: &[ShortTermSet],
    max_dec_pic_buffering_minus1: u32,
) -> Result<ShortTermSet, VideoError> {
    if index != 0 && bits.flag()? {
        // inter_ref_pic_set_prediction_flag. In an SPS stRpsIdx is below
        // num_short_term_ref_pic_sets, so delta_idx_minus1 is absent and 0.
        let reference = sets
            .get(index - 1)
            .ok_or_else(|| bits.malformed("RefRpsIdx outside the candidate sets"))?;
        let sign = bits.flag()?;
        let magnitude = i64::from(bits.ue_max(0x7FFF, "abs_delta_rps_minus1 over 2^15 - 1")?) + 1;
        let delta_rps = if sign { -magnitude } else { magnitude };
        let count = reference.negative.len() + reference.positive.len();
        let mut use_delta = Vec::with_capacity(count + 1);
        for _ in 0..=count {
            let used = bits.flag()?;
            // use_delta_flag is inferred 1 when absent.
            use_delta.push(used || bits.flag()?);
        }
        let used = |at: usize| use_delta.get(at).copied().unwrap_or(false);
        let neg = reference.negative.len();
        let mut set = ShortTermSet::default();
        // 7-61.
        for (j, value) in reference.positive.iter().enumerate().rev() {
            let d = value + delta_rps;
            if d < 0 && used(neg + j) {
                set.negative.push(d);
            }
        }
        if delta_rps < 0 && used(count) {
            set.negative.push(delta_rps);
        }
        for (j, value) in reference.negative.iter().enumerate() {
            let d = value + delta_rps;
            if d < 0 && used(j) {
                set.negative.push(d);
            }
        }
        // 7-62.
        for (j, value) in reference.negative.iter().enumerate().rev() {
            let d = value + delta_rps;
            if d > 0 && used(j) {
                set.positive.push(d);
            }
        }
        if delta_rps > 0 && used(count) {
            set.positive.push(delta_rps);
        }
        for (j, value) in reference.positive.iter().enumerate() {
            let d = value + delta_rps;
            if d > 0 && used(neg + j) {
                set.positive.push(d);
            }
        }
        return Ok(set);
    }
    // 7.4.8: num_negative_pics is 0 to sps_max_dec_pic_buffering_minus1 and
    // num_positive_pics is 0 to that less num_negative_pics.
    let negative_count = bits.ue_max(
        max_dec_pic_buffering_minus1,
        "num_negative_pics over sps_max_dec_pic_buffering_minus1",
    )?;
    let positive_count = bits.ue_max(
        max_dec_pic_buffering_minus1 - negative_count,
        "num_negative_pics and num_positive_pics over sps_max_dec_pic_buffering_minus1",
    )?;
    let mut set = ShortTermSet::default();
    // 7-67 to 7-70: each delta is cumulative from the current picture.
    let mut poc = 0i64;
    for _ in 0..negative_count {
        poc -= i64::from(bits.ue_max(0x7FFF, "delta_poc_s0_minus1 over 2^15 - 1")?) + 1;
        bits.flag()?; // used_by_curr_pic_s0_flag
        set.negative.push(poc);
    }
    poc = 0;
    for _ in 0..positive_count {
        poc += i64::from(bits.ue_max(0x7FFF, "delta_poc_s1_minus1 over 2^15 - 1")?) + 1;
        bits.flag()?; // used_by_curr_pic_s1_flag
        set.positive.push(poc);
    }
    Ok(set)
}

/// 7.3.2.2.1, from a NAL unit including its two byte header.
pub(crate) fn parse_sps(nal: &[u8]) -> Result<(usize, Sps), VideoError> {
    let mut bits = Bits::rbsp(
        nal.get(2..).unwrap_or_default(),
        "HEVC sequence parameter set",
    );
    bits.bits(4)?; // sps_video_parameter_set_id
    let sub_layers_minus1 = bits.bits(3)?;
    if sub_layers_minus1 > 6 {
        return Err(bits.malformed("sps_max_sub_layers_minus1 over 6"));
    }
    bits.flag()?; // sps_temporal_id_nesting_flag
    let profile = profile_tier_level(&mut bits, sub_layers_minus1)?;
    let id = bits.ue_max(15, "sps_seq_parameter_set_id over 15")?;
    let chroma_format_idc = bits.ue_max(3, "chroma_format_idc over 3")?;
    let separate_colour_plane = chroma_format_idc == 3 && bits.flag()?;
    // A.4.1 items b and c bound each side by Sqrt( MaxLumaPs * 8 ), and the
    // largest MaxLumaPs of Table A.8, level 7.2, makes that 33 776. Level 8.5
    // has no limit.
    let side_limit = if profile.level_idc == LEVEL_8_5 {
        u32::MAX - 1
    } else {
        MAX_PIC_SIDE
    };
    let width = bits.ue_max(
        side_limit,
        "pic_width_in_luma_samples over every level's limit",
    )?;
    let height = bits.ue_max(
        side_limit,
        "pic_height_in_luma_samples over every level's limit",
    )?;
    let window = if bits.flag()? {
        // Bounded together with the picture below, 7.4.3.2.1.
        let mut offsets = [0u32; 4];
        for offset in &mut offsets {
            // The reader refuses first, ue(v) stopping at 2^32 - 2.
            *offset = bits.ue_max(u32::MAX - 1, "a conf_win offset over 2^32 - 2")?;
        }
        offsets
    } else {
        [0; 4]
    };
    let bit_depth_luma = bits.ue_max(8, "bit_depth_luma_minus8 over 8")? + 8;
    let bit_depth_chroma = bits.ue_max(8, "bit_depth_chroma_minus8 over 8")? + 8;
    let log2_max_poc_lsb = bits.ue_max(12, "log2_max_pic_order_cnt_lsb_minus4 over 12")? + 4;
    let ordering_all = bits.flag()?;
    let first_layer = if ordering_all { 0 } else { sub_layers_minus1 };
    let dpb_limit = if profile.level_idc == LEVEL_8_5 {
        u32::MAX
    } else {
        MAX_DPB_SIZE
    };
    let mut max_dec_pic_buffering_minus1 = 0;
    for _ in first_layer..=sub_layers_minus1 {
        // 7.4.3.2.1: sps_max_dec_pic_buffering_minus1 is 0 to MaxDpbSize - 1,
        // and A.4.2 gives MaxDpbSize by A-2, at most 16, except at level 8.5,
        // where it gives none. sps_max_num_reorder_pics is 0 to
        // sps_max_dec_pic_buffering_minus1, and
        // sps_max_latency_increase_plus1 is 0 to 2^32 - 2, which the reader
        // enforces before the check because ue(v) cannot exceed 2^32 - 2.
        max_dec_pic_buffering_minus1 = bits.ue_max(
            dpb_limit - 1,
            "sps_max_dec_pic_buffering_minus1 over MaxDpbSize - 1",
        )?;
        bits.ue_max(
            max_dec_pic_buffering_minus1,
            "sps_max_num_reorder_pics over sps_max_dec_pic_buffering_minus1",
        )?;
        bits.ue_max(u32::MAX - 1, "sps_max_latency_increase_plus1 over 2^32 - 2")?;
    }
    // 7-10 and 7-11, with the A.3 profiles keeping CtbLog2SizeY in 4 to 6.
    let min_cb_log2 = bits.ue_max(3, "log2_min_luma_coding_block_size_minus3 over 3")? + 3;
    let ctb_log2 =
        min_cb_log2 + bits.ue_max(6 - min_cb_log2, "a coding tree block larger than 64")?;
    if ctb_log2 < 4 {
        return Err(bits.malformed("a coding tree block smaller than 16"));
    }
    // 7.4.3.2.1: MinTbLog2SizeY is below MinCbLog2SizeY, MaxTbLog2SizeY is at
    // most Min( CtbLog2SizeY, 5 ), and each transform hierarchy depth is at
    // most CtbLog2SizeY - MinTbLog2SizeY.
    let min_tb_log2 = bits.ue_max(min_cb_log2 - 3, "MinTbLog2SizeY not below MinCbLog2SizeY")? + 2;
    bits.ue_max(
        ctb_log2.min(5) - min_tb_log2,
        "MaxTbLog2SizeY over Min( CtbLog2SizeY, 5 )",
    )?;
    bits.ue_max(
        ctb_log2 - min_tb_log2,
        "max_transform_hierarchy_depth_inter too deep",
    )?;
    bits.ue_max(
        ctb_log2 - min_tb_log2,
        "max_transform_hierarchy_depth_intra too deep",
    )?;
    if bits.flag()? && bits.flag()? {
        // scaling_list_enabled_flag and sps_scaling_list_data_present_flag.
        skip_scaling_list_data(&mut bits)?;
    }
    bits.flag()?; // amp_enabled_flag
    bits.flag()?; // sample_adaptive_offset_enabled_flag
    if bits.flag()? {
        // pcm_enabled_flag. 7.4.3.2.1: PcmBitDepthY and PcmBitDepthC are at
        // most BitDepthY and BitDepthC, Log2MinIpcmCbSizeY is
        // Min( MinCbLog2SizeY, 5 ) to Min( CtbLog2SizeY, 5 ), and
        // Log2MaxIpcmCbSizeY is at most Min( CtbLog2SizeY, 5 ).
        let pcm_luma = bits.bits(4)? + 1;
        let pcm_chroma = bits.bits(4)? + 1;
        if pcm_luma > bit_depth_luma || pcm_chroma > bit_depth_chroma {
            return Err(bits.malformed("a PCM sample deeper than the picture"));
        }
        let pcm_min = bits.ue_max(ctb_log2.min(5) - 3, "Log2MinIpcmCbSizeY too large")? + 3;
        if pcm_min < min_cb_log2.min(5) {
            return Err(bits.malformed("Log2MinIpcmCbSizeY too small"));
        }
        bits.ue_max(ctb_log2.min(5) - pcm_min, "Log2MaxIpcmCbSizeY too large")?;
        bits.flag()?; // pcm_loop_filter_disabled_flag
    }
    let set_count = bits.ue_max(MAX_SHORT_TERM_SETS, "num_short_term_ref_pic_sets over 64")?;
    let mut sets: Vec<ShortTermSet> = Vec::new();
    for index in 0..to_usize(set_count, "HEVC sequence parameter set")? {
        let set = short_term_set(&mut bits, index, &sets, max_dec_pic_buffering_minus1)?;
        sets.push(set);
    }
    if bits.flag()? {
        // long_term_ref_pics_present_flag.
        let count = bits.ue_max(MAX_LONG_TERM_SPS, "num_long_term_ref_pics_sps over 32")?;
        for _ in 0..count {
            bits.bits(
                u8::try_from(log2_max_poc_lsb)
                    .map_err(|_| bits.malformed("a bit width over 255"))?,
            )?;
            bits.flag()?; // used_by_curr_pic_lt_sps_flag
        }
    }
    bits.flag()?; // sps_temporal_mvp_enabled_flag
    bits.flag()?; // strong_intra_smoothing_enabled_flag
    let (signal, field_seq) = if bits.flag()? {
        vui(&mut bits)?
    } else {
        (None, false)
    };
    // Table 6-1, SubWidthC and SubHeightC, and 7.4.3.2.1's conformance
    // window in units of them.
    let (sub_width, sub_height) = match (chroma_format_idc, separate_colour_plane) {
        (1, _) => (2u64, 2u64),
        (2, _) => (2, 1),
        _ => (1, 1),
    };
    // 7.4.3.2.1: each side is not 0 and is a multiple of MinCbSizeY, and
    // SubWidthC * ( conf_win_left_offset + conf_win_right_offset ) is below
    // pic_width_in_luma_samples, with the same for the height.
    let min_cb = 1u32 << min_cb_log2;
    if width == 0 || height == 0 || width % min_cb != 0 || height % min_cb != 0 {
        return Err(bits.malformed("a picture side of 0 or not a multiple of MinCbSizeY"));
    }
    let coded_size = FrameSize {
        width: dimension(width.into())?,
        height: dimension(height.into())?,
    };
    let [left, right, top, bottom] = window.map(u64::from);
    let visible_size = FrameSize {
        width: cropped(coded_size.width, sub_width * (left + right))?,
        height: cropped(coded_size.height, sub_height * (top + bottom))?,
    };
    Ok((
        to_usize(id, "HEVC sequence parameter set")?,
        Sps {
            profile,
            separate_colour_plane,
            log2_max_poc_lsb: u8::try_from(log2_max_poc_lsb)
                .map_err(|_| bits.malformed("a bit width over 255"))?,
            coded_size,
            visible_size,
            signal,
            field_seq,
        },
    ))
}

/// E.2.1 up to field_seq_flag.
fn vui(bits: &mut Bits<'_>) -> Result<(Option<SignalType>, bool), VideoError> {
    if bits.flag()? && bits.byte()? == EXTENDED_SAR {
        bits.skip(32)?; // sar_width and sar_height
    }
    if bits.flag()? {
        bits.flag()?; // overscan_appropriate_flag
    }
    let signal = if bits.flag()? {
        bits.bits(3)?; // video_format
        let full_range = bits.flag()?;
        let description = if bits.flag()? {
            Some((bits.byte()?, bits.byte()?, bits.byte()?))
        } else {
            None
        };
        Some(SignalType {
            full_range,
            description,
        })
    } else {
        None
    };
    if bits.flag()? {
        // E.3.1: each chroma_sample_loc_type is 0 to 5.
        bits.ue_max(5, "chroma_sample_loc_type_top_field over 5")?;
        bits.ue_max(5, "chroma_sample_loc_type_bottom_field over 5")?;
    }
    bits.flag()?; // neutral_chroma_indication_flag
    let field_seq = bits.flag()?;
    Ok((signal, field_seq))
}

/// The fields of 7.3.2.3.1 a slice segment header needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pps {
    sps_id: usize,
    output_flag_present: bool,
    num_extra_slice_header_bits: u32,
}

fn parse_pps(nal: &[u8]) -> Result<(usize, Pps), VideoError> {
    let mut bits = Bits::rbsp(
        nal.get(2..).unwrap_or_default(),
        "HEVC picture parameter set",
    );
    let id = bits.ue_max(63, "pps_pic_parameter_set_id over 63")?;
    let sps_id = bits.ue_max(15, "pps_seq_parameter_set_id over 15")?;
    bits.flag()?; // dependent_slice_segments_enabled_flag
    let output_flag_present = bits.flag()?;
    let num_extra_slice_header_bits = bits.bits(3)?;
    Ok((
        to_usize(id, "HEVC picture parameter set")?,
        Pps {
            sps_id: to_usize(sps_id, "HEVC picture parameter set")?,
            output_flag_present,
            num_extra_slice_header_bits,
        },
    ))
}

/// 7.3.6.1 up to slice_pic_order_cnt_lsb, for the first slice segment of a
/// picture. Returns None for any later slice segment.
fn first_slice_poc_lsb(
    nal: &[u8],
    kind: u8,
    sps_table: &[Option<Sps>],
    pps_table: &[Option<Pps>],
) -> Result<Option<(u32, u8)>, VideoError> {
    let mut bits = Bits::rbsp(
        nal.get(2..).unwrap_or_default(),
        "HEVC slice segment header",
    );
    if !bits.flag()? {
        return Ok(None); // first_slice_segment_in_pic_flag
    }
    if (nal::BLA_W_LP..=nal::RSV_IRAP_23).contains(&kind) {
        bits.flag()?; // no_output_of_prior_pics_flag
    }
    let pps_id = to_usize(
        bits.ue_max(63, "slice_pic_parameter_set_id over 63")?,
        "HEVC slice segment header",
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
    bits.skip(pps.num_extra_slice_header_bits)?; // slice_reserved_flag
    bits.ue_max(2, "slice_type over 2")?;
    if pps.output_flag_present && !bits.flag()? {
        // pic_output_flag 0: the picture is decoded and never shown, so it is
        // not a displayed frame and this table has no row for it.
        return Err(VideoError::Unsupported {
            feature: "an HEVC picture with pic_output_flag 0 outside MP4",
        });
    }
    if sps.separate_colour_plane {
        bits.bits(2)?; // colour_plane_id
    }
    let lsb = if kind == nal::IDR_W_RADL || kind == nal::IDR_N_LP {
        0
    } else {
        bits.bits(sps.log2_max_poc_lsb)?
    };
    Ok(Some((lsb, sps.log2_max_poc_lsb)))
}

/// The state 8.3.1 carries from picture to picture.
#[derive(Debug)]
struct PocState {
    /// prevTid0Pic's PicOrderCntMsb and slice_pic_order_cnt_lsb.
    prev_msb: i64,
    prev_lsb: i64,
    /// The coded video sequence, 3.31, which output order never crosses.
    segment: u32,
    first_picture: bool,
    after_end_of_sequence: bool,
    /// Whether the last IRAP picture began with NoRaslOutputFlag 1 and can
    /// have RASL pictures, which are then not output.
    rasl_skipped: bool,
}

impl Default for PocState {
    fn default() -> Self {
        Self {
            prev_msb: 0,
            prev_lsb: 0,
            segment: 0,
            first_picture: true,
            after_end_of_sequence: false,
            rasl_skipped: false,
        }
    }
}

impl PocState {
    /// An end of sequence NAL unit, after which a CRA picture has
    /// NoRaslOutputFlag 1, 3.31 NOTE.
    fn end_of_sequence(&mut self) {
        self.after_end_of_sequence = true;
    }

    /// The output position (coded video sequence, PicOrderCntVal) of one
    /// picture, 8.3.1, from its nal_unit_type, TemporalId and
    /// slice_pic_order_cnt_lsb.
    fn next(
        &mut self,
        kind: u8,
        temporal_id: u8,
        lsb: u32,
        log2_max_lsb: u8,
    ) -> Result<(u32, i64), VideoError> {
        let irap = (nal::BLA_W_LP..=nal::RSV_IRAP_23).contains(&kind);
        // 3.31 NOTE: NoRaslOutputFlag is 1 for IDR and BLA pictures, and for
        // a CRA picture first in the stream or first after an end of sequence
        // NAL unit.
        let no_rasl_output =
            irap && (kind != nal::CRA || self.first_picture || self.after_end_of_sequence);
        if no_rasl_output {
            if !self.first_picture {
                self.segment = self.segment.checked_add(1).ok_or(VideoError::Malformed {
                    structure: "HEVC byte stream",
                    reason: "too many coded video sequences",
                })?;
            }
            self.rasl_skipped = kind == nal::CRA || (nal::BLA_W_LP..=nal::BLA_N_LP).contains(&kind);
        } else if irap {
            self.rasl_skipped = false;
        }
        if self.rasl_skipped && (nal::RASL_N..=nal::RASL_R).contains(&kind) {
            return Err(VideoError::Unsupported {
                feature: "an HEVC stream opening with RASL pictures that are never output",
            });
        }
        self.first_picture = false;
        self.after_end_of_sequence = false;
        let lsb = i64::from(lsb);
        let max_lsb = 1i64 << log2_max_lsb;
        let (prev_msb, prev_lsb) = (self.prev_msb, self.prev_lsb);
        // 8-1 and 8-2.
        let msb = if no_rasl_output {
            0
        } else if lsb < prev_lsb && prev_lsb - lsb >= max_lsb / 2 {
            prev_msb + max_lsb
        } else if lsb > prev_lsb && lsb - prev_lsb > max_lsb / 2 {
            prev_msb - max_lsb
        } else {
            prev_msb
        };
        // prevTid0Pic: TemporalId 0 and not RASL, RADL or SLNR. SLNR types are
        // the even values up to 14, Table 7-1.
        let leading = (nal::RADL_N..=nal::RASL_R).contains(&kind);
        let sub_layer_non_reference = kind <= nal::RSV_VCL_N14 && kind.is_multiple_of(2);
        if temporal_id == 0 && !leading && !sub_layer_non_reference {
            self.prev_msb = msb;
            self.prev_lsb = lsb;
        }
        Ok((self.segment, msb + lsb))
    }
}

/// Table 7-1's reserved VCL nal_unit_type values, RSV_VCL_N10 to RSV_VCL_R15
/// and RSV_IRAP_VCL22 to RSV_VCL31.
fn reserved_vcl(kind: u8) -> bool {
    (nal::RSV_VCL_N10..=nal::RSV_VCL_R15).contains(&kind)
        || (nal::RSV_IRAP_22..=nal::VCL_LAST).contains(&kind)
}

/// Splits an HEVC byte stream into access units, 7.4.2.4.4, with the output
/// position of each from 8.3.1 and the stream description from the first
/// sequence parameter set.
pub(crate) fn access_units(es: &[u8]) -> Result<(Vec<AccessUnit>, StreamInfo), VideoError> {
    let mut sps_table: Vec<Option<Sps>> = vec![None; SPS_IDS];
    let mut pps_table: Vec<Option<Pps>> = vec![None; PPS_IDS];
    let mut info: Option<StreamInfo> = None;
    let mut found = Vec::new();
    // The open access unit: first byte, start code position, its output
    // position and whether it is an IRAP picture.
    let mut open: Option<(usize, usize)> = None;
    let mut picture: Option<((u32, i64), bool)> = None;
    let mut after_vcl = false;
    let mut last_end = 0usize;
    let mut poc = PocState::default();
    for unit in units(es) {
        let nal = es.get(unit.payload..unit.end).unwrap_or_default();
        let (Some(first), Some(second)) = (nal.first().copied(), nal.get(1).copied()) else {
            continue;
        };
        if first & 0x80 != 0 {
            return Err(VideoError::Malformed {
                structure: "HEVC NAL unit header",
                reason: "forbidden_zero_bit is 1",
            });
        }
        // 7.3.1.2: nal_unit_type u(6), nuh_layer_id u(6) and
        // nuh_temporal_id_plus1 u(3).
        let kind = (first >> 1) & 0x3F;
        let layer = ((first & 0x01) << 5) | (second >> 3);
        let temporal_id = (second & 0x07)
            .checked_sub(1)
            .ok_or(VideoError::Malformed {
                structure: "HEVC NAL unit header",
                reason: "nuh_temporal_id_plus1 is 0",
            })?;
        let prefix = unit.payload - 3;
        let starts_unit = layer == 0
            && (matches!(
                kind,
                nal::VPS | nal::SPS | nal::PPS | nal::AUD | nal::PREFIX_SEI
            ) || (nal::RSV_NVCL41..=nal::RSV_NVCL44).contains(&kind)
                || (nal::UNSPEC48..=nal::UNSPEC55).contains(&kind));
        if starts_unit && after_vcl {
            close(&mut found, &mut open, &mut picture, last_end)?;
            after_vcl = false;
        }
        if open.is_none() {
            open = Some((unit.start, prefix));
        }
        if layer == 0 {
            match kind {
                nal::SPS => {
                    let (id, sps) = parse_sps(nal)?;
                    if info.is_none() {
                        info = Some(sps.stream_info("hvc1"));
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
                nal::EOS => poc.end_of_sequence(),
                // 7.4.2.2: a NAL unit of a reserved nal_unit_type is ignored,
                // and Table 7-1 reserves VCL types 10 to 15 and 22 to 31, so
                // such a unit neither starts nor ends a picture.
                kind if reserved_vcl(kind) => {}
                0..=nal::VCL_LAST => {
                    if let Some((lsb, log2_max_lsb)) =
                        first_slice_poc_lsb(nal, kind, &sps_table, &pps_table)?
                    {
                        if picture.is_some() {
                            close(&mut found, &mut open, &mut picture, last_end)?;
                            open = Some((unit.start, prefix));
                        }
                        let irap = (nal::BLA_W_LP..=nal::RSV_IRAP_23).contains(&kind);
                        let order = poc.next(kind, temporal_id, lsb, log2_max_lsb)?;
                        picture = Some((order, irap));
                    }
                }
                _ => {}
            }
        }
        // 7.4.2.4.4 NOTE 1: the last VCL NAL unit before the next picture may
        // have nuh_layer_id above 0, and it still ends the picture's VCL run.
        if kind <= nal::VCL_LAST && picture.is_some() {
            after_vcl = true;
        }
        last_end = unit.end;
    }
    if picture.is_some() {
        close(&mut found, &mut open, &mut picture, es.len())?;
    }
    let info = info.ok_or(VideoError::Malformed {
        structure: "HEVC byte stream",
        reason: "no sequence parameter set",
    })?;
    Ok((found, info))
}

fn close(
    found: &mut Vec<AccessUnit>,
    open: &mut Option<(usize, usize)>,
    picture: &mut Option<((u32, i64), bool)>,
    end: usize,
) -> Result<(), VideoError> {
    let (start, anchor) = open.take().ok_or(VideoError::Malformed {
        structure: "HEVC byte stream",
        reason: "an access unit without a start",
    })?;
    let ((order, irap), range) =
        picture
            .take()
            .map(|value| (value, start..end))
            .ok_or(VideoError::Malformed {
                structure: "HEVC byte stream",
                reason: "an access unit without a coded picture",
            })?;
    found.push(AccessUnit {
        range,
        anchor,
        order,
        // An IRAP picture, IDR, CRA or BLA, is where decoding may begin, as
        // the WebCodecs HEVC registration also takes a key chunk to be.
        random_access: irap,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PocState, access_units, nal, parse_sps, profile_tier_level};
    use crate::video::{
        VideoError,
        bits::{Bits, Writer},
        table::AccessUnit,
    };

    /// Table 7-1 values used below.
    const TRAIL_N: u8 = 0;
    const TRAIL_R: u8 = 1;
    const RADL_R: u8 = 7;

    /// Every picture below has log2_max_pic_order_cnt_lsb 4, so
    /// MaxPicOrderCntLsb is 16 and half of it 8 in 8-1.
    fn run(
        state: &mut PocState,
        pictures: &[(u8, u8, u32)],
    ) -> Result<Vec<(u32, i64)>, VideoError> {
        pictures
            .iter()
            .map(|&(kind, temporal_id, lsb)| state.next(kind, temporal_id, lsb, 4))
            .collect()
    }

    /// 3.31 NOTE: a CRA picture after an end of sequence NAL unit has
    /// NoRaslOutputFlag 1, so it opens a coded video sequence with
    /// PicOrderCntMsb 0. IDR 0 and TRAIL_R 3, then end of sequence, then CRA
    /// lsb 1 and TRAIL_R lsb 2, give (0, 0), (0, 3), (1, 1), (1, 2). Without
    /// the end of sequence the CRA continues the sequence, and lsb 1 and 2
    /// are POC 1 and 2 of sequence 0, before the TRAIL_R of POC 3.
    #[test]
    fn end_of_sequence_before_a_cra_opens_a_sequence() -> Result<(), VideoError> {
        let mut state = PocState::default();
        let mut found = run(&mut state, &[(nal::IDR_W_RADL, 0, 0), (TRAIL_R, 0, 3)])?;
        state.end_of_sequence();
        found.extend(run(&mut state, &[(nal::CRA, 0, 1), (TRAIL_R, 0, 2)])?);
        assert_eq!(found, [(0, 0), (0, 3), (1, 1), (1, 2)]);
        let mut state = PocState::default();
        let found = run(
            &mut state,
            &[
                (nal::IDR_W_RADL, 0, 0),
                (TRAIL_R, 0, 3),
                (nal::CRA, 0, 1),
                (TRAIL_R, 0, 2),
            ],
        )?;
        assert_eq!(found, [(0, 0), (0, 3), (0, 1), (0, 2)]);
        Ok(())
    }

    /// 8.3.1: prevTid0Pic has TemporalId 0 and is not RASL, RADL or SLNR.
    /// After IDR 0 and TRAIL_R 6, a picture of lsb 13 is POC 13, 7 above 6.
    /// When it is a TRAIL_N, which Table 7-1 makes SLNR, or a RADL_R, or has
    /// TemporalId 1, prevTid0Pic stays the lsb 6 picture, so a following
    /// TRAIL_R lsb 2 is 4 below 6 and POC 2. Taking lsb 13 instead would put
    /// it 11 below, at or past half of 16, and make it POC 18.
    #[test]
    fn prev_tid0_pic_skips_slnr_leading_and_higher_sub_layers() -> Result<(), VideoError> {
        for (kind, temporal_id) in [(TRAIL_N, 0), (RADL_R, 0), (TRAIL_R, 1)] {
            let mut state = PocState::default();
            let found = run(
                &mut state,
                &[
                    (nal::IDR_W_RADL, 0, 0),
                    (TRAIL_R, 0, 6),
                    (kind, temporal_id, 13),
                    (TRAIL_R, 0, 2),
                ],
            )?;
            assert_eq!(
                found,
                [(0, 0), (0, 6), (0, 13), (0, 2)],
                "{kind} {temporal_id}"
            );
        }
        let mut state = PocState::default();
        let found = run(
            &mut state,
            &[
                (nal::IDR_W_RADL, 0, 0),
                (TRAIL_R, 0, 6),
                (TRAIL_R, 0, 13),
                (TRAIL_R, 0, 2),
            ],
        )?;
        assert_eq!(found, [(0, 0), (0, 6), (0, 13), (0, 18)]);
        Ok(())
    }

    /// 7.3.3 with sps_max_sub_layers_minus1 2: two pairs of sub-layer present
    /// flags, (1, 1) and (0, 1), then reserved_zero_2bits for i of 2 to 7, 12
    /// bits, then 88 profile bits and 8 level bits for sub-layer 0 and 8
    /// level bits for sub-layer 1. A ue(v) of 5 after them reads back only if
    /// every skip is that long, and the general fields read as written.
    #[test]
    fn sub_layer_profile_tier_level_is_skipped() -> Result<(), VideoError> {
        let mut w = Writer::default();
        w.u(8, 0x01).u(32, 0x6000_0000).u(8, 0x90).u(40, 0).u(8, 93);
        w.flag(true).flag(true).flag(false).flag(true);
        w.u(12, 0);
        w.u(44, 0x0FFF_FFFF_FFFF).u(44, 0x0FFF_FFFF_FFFF).u(8, 0xFF);
        w.u(8, 0xFF);
        w.ue(5);
        let nal = w.nal(&[]);
        let mut bits = Bits::rbsp(&nal, "test");
        let profile = profile_tier_level(&mut bits, 2)?;
        assert_eq!(profile.profile_idc, 1);
        assert_eq!(profile.compatibility, 0x6000_0000);
        assert_eq!(profile.constraints, [0x90, 0, 0, 0, 0, 0]);
        assert_eq!(profile.level_idc, 93);
        assert_eq!(bits.ue_max(31, "marker")?, 5);
        Ok(())
    }

    /// The scaling_list_data of 7.3.4 with every matrix predicted, delta 0,
    /// except sizeId 2 matrix 0, which carries `dc` and `delta` when given,
    /// and the one matrix `pred` names, which carries its delta.
    #[derive(Clone, Copy, Default)]
    struct Scaling {
        dc: Option<i64>,
        delta: Option<i64>,
        pred: Option<((u32, u32), u64)>,
    }

    /// One crafted HEVC SPS, 7.3.2.2.1 with profile_tier_level 7.3.3 for
    /// Main, general_level_idc `level`, and every field the parser bounds
    /// settable. Review pass 2's f241-p2-craft.py is the row list.
    #[derive(Clone)]
    struct Sps {
        level: u64,
        chroma: u64,
        width: u64,
        height: u64,
        conformance: Option<[u64; 4]>,
        bit_depth: u64,
        bit_depth_chroma: u64,
        lsb: u64,
        dpb: u64,
        reorder: u64,
        latency: u64,
        min_cb: u64,
        diff_cb: u64,
        min_tb: u64,
        diff_tb: u64,
        depth_inter: u64,
        depth_intra: u64,
        scaling: Option<Scaling>,
        pcm: Option<(u64, u64, u64, u64)>,
        sets: Vec<(u64, u64)>,
        chroma_loc: Option<(u64, u64)>,
        separate_planes: bool,
        id: u64,
    }

    impl Default for Sps {
        fn default() -> Self {
            Self {
                level: 93,
                chroma: 1,
                width: 64,
                height: 64,
                conformance: None,
                bit_depth: 0,
                bit_depth_chroma: 0,
                lsb: 4,
                dpb: 4,
                reorder: 2,
                latency: 0,
                min_cb: 0,
                diff_cb: 2,
                min_tb: 0,
                diff_tb: 3,
                depth_inter: 0,
                depth_intra: 0,
                scaling: None,
                pcm: None,
                sets: vec![(1, 0)],
                chroma_loc: None,
                separate_planes: false,
                id: 0,
            }
        }
    }

    impl Sps {
        fn nal(&self) -> Vec<u8> {
            let mut w = Writer::default();
            w.u(4, 0).u(3, 0).flag(true);
            w.u(2, 0)
                .flag(false)
                .u(5, 1)
                .u(32, 0x6000_0000)
                .u(4, 0b1001)
                .u(44, 0)
                .u(8, self.level);
            w.ue(self.id).ue(self.chroma);
            if self.chroma == 3 {
                w.flag(self.separate_planes); // separate_colour_plane_flag
            }
            w.ue(self.width).ue(self.height);
            match self.conformance {
                None => {
                    w.flag(false);
                }
                Some(window) => {
                    w.flag(true);
                    for offset in window {
                        w.ue(offset);
                    }
                }
            }
            w.ue(self.bit_depth)
                .ue(self.bit_depth_chroma)
                .ue(self.lsb)
                .flag(true);
            w.ue(self.dpb).ue(self.reorder).ue(self.latency);
            w.ue(self.min_cb)
                .ue(self.diff_cb)
                .ue(self.min_tb)
                .ue(self.diff_tb);
            w.ue(self.depth_inter).ue(self.depth_intra);
            match self.scaling {
                None => {
                    w.flag(false);
                }
                Some(scaling) => {
                    w.flag(true).flag(true);
                    for size_id in 0..4u32 {
                        let step = if size_id == 3 { 3 } else { 1 };
                        for matrix_id in (0..6u32).step_by(step) {
                            if let Some((place, delta)) = scaling.pred
                                && place == (size_id, matrix_id)
                            {
                                w.flag(false).ue(delta);
                            } else if (scaling.dc.is_some() || scaling.delta.is_some())
                                && (size_id, matrix_id) == (2, 0)
                            {
                                w.flag(true)
                                    .se(scaling.dc.unwrap_or(0))
                                    .se(scaling.delta.unwrap_or(0));
                                for _ in 0..63 {
                                    w.se(0);
                                }
                            } else {
                                w.flag(false).ue(0);
                            }
                        }
                    }
                }
            }
            w.flag(false).flag(false);
            match self.pcm {
                None => {
                    w.flag(false);
                }
                Some((luma, chroma, min, diff)) => {
                    w.flag(true)
                        .u(4, luma - 1)
                        .u(4, chroma - 1)
                        .ue(min)
                        .ue(diff)
                        .flag(false);
                }
            }
            w.ue(u64::try_from(self.sets.len()).unwrap_or_default());
            for (negative, positive) in &self.sets {
                w.ue(*negative).ue(*positive);
                for _ in 0..negative + positive {
                    w.ue(0).flag(true);
                }
            }
            w.flag(false).flag(false).flag(false);
            match self.chroma_loc {
                None => {
                    w.flag(false);
                }
                Some((top, bottom)) => {
                    w.flag(true);
                    w.flag(false)
                        .flag(false)
                        .flag(false)
                        .flag(true)
                        .ue(top)
                        .ue(bottom);
                    // E.2.1: neutral_chroma_indication_flag, field_seq_flag,
                    // frame_field_info_present_flag,
                    // default_display_window_flag,
                    // vui_timing_info_present_flag and
                    // bitstream_restriction_flag.
                    for _ in 0..6 {
                        w.flag(false);
                    }
                }
            }
            w.flag(false);
            w.nal(&[0x42, 0x01])
        }
    }

    /// A crafted HEVC PPS and slice segments, 7.3.2.3.1 and 7.3.6.1, with the
    /// fields the slice segment header reads before slice_pic_order_cnt_lsb
    /// settable. The SPS is `Sps`, 64 by 64 with CTB 32, so PicSizeInCtbsY
    /// is 4 and slice_segment_address is u(2), and log2_max_pic_order_cnt_lsb
    /// is 8.
    #[derive(Clone, Default)]
    struct Segments {
        sps: Sps,
        pps_id: u64,
        slice_pps_id: Option<u64>,
        pps_sps_id: Option<u64>,
        dependent_enabled: bool,
        output_flag_present: bool,
        pic_output: bool,
        extra_bits: u32,
        colour_plane: Option<u64>,
    }

    impl Segments {
        fn pps(&self) -> Vec<u8> {
            let mut w = Writer::default();
            w.ue(self.pps_id)
                .ue(self.pps_sps_id.unwrap_or(self.sps.id))
                .flag(self.dependent_enabled)
                .flag(self.output_flag_present)
                .u(3, u64::from(self.extra_bits));
            // sign_data_hiding_enabled_flag to transform_skip_enabled_flag,
            // cu_qp_delta_enabled_flag 0, both chroma offsets, and every flag
            // after them 0 with log2_parallel_merge_level_minus2 0.
            w.flag(false)
                .flag(false)
                .ue(0)
                .ue(0)
                .se(0)
                .flag(false)
                .flag(false);
            w.flag(false).se(0).se(0);
            for _ in 0..10 {
                w.flag(false);
            }
            w.ue(0).flag(false).flag(false);
            w.nal(&[0x44, 0x01])
        }

        /// One slice segment of a picture of `kind`, its header complete
        /// for this SPS and PPS and its slice data one byte. A later segment is
        /// dependent when the PPS allows it, and carries only its address,
        /// and is otherwise independent with every field. Each
        /// slice_reserved_flag is 1. An IDR's header ends in slice_qp_delta 0
        /// and byte_alignment( ), 1 then 1 and zeros, so a reader that took
        /// an 8-bit slice_pic_order_cnt_lsb from it would not read 0.
        fn segment(&self, kind: u8, first: bool, lsb: u64) -> Vec<u8> {
            let irap = (nal::BLA_W_LP..=nal::RSV_IRAP_23).contains(&kind);
            let idr = kind == nal::IDR_W_RADL || kind == nal::IDR_N_LP;
            let dependent = !first && self.dependent_enabled;
            let mut w = Writer::default();
            w.flag(first);
            if irap {
                w.flag(false); // no_output_of_prior_pics_flag
            }
            w.ue(self.slice_pps_id.unwrap_or(self.pps_id));
            if !first {
                if self.dependent_enabled {
                    w.flag(true); // dependent_slice_segment_flag
                }
                w.u(2, 1); // slice_segment_address
            }
            if !dependent {
                w.u(self.extra_bits, (1 << self.extra_bits) - 1);
                // slice_type 2, I, for an IRAP picture and 1, P, otherwise.
                w.ue(if irap { 2 } else { 1 });
                if self.output_flag_present {
                    w.flag(self.pic_output);
                }
                if let Some(plane) = self.colour_plane {
                    w.u(2, plane);
                }
                if !idr {
                    w.u(8, lsb);
                    if irap {
                        // A CRA or BLA picture has NumPicTotalCurr 0, 7.4.7.1,
                        // so its st_ref_pic_set( 1 ) is written in the header:
                        // inter_ref_pic_set_prediction_flag 0 and no pictures.
                        w.flag(false).flag(false).ue(0).ue(0);
                    } else {
                        // short_term_ref_pic_set_sps_flag 1, the SPS's one set.
                        w.flag(true);
                    }
                }
                if !irap {
                    // num_ref_idx_active_override_flag 0 and
                    // five_minus_max_num_merge_cand 0. No other P field is
                    // present for this SPS and PPS.
                    w.flag(false).ue(0);
                }
                w.se(0); // slice_qp_delta
            }
            // byte_alignment( ), then a byte of slice data.
            w.align().u(8, 0x55);
            w.nal(&[kind << 1, 0x01])
        }

        /// IDR_W_RADL, TRAIL_R lsb 3 in two slice segments that both carry
        /// it, as 7.4.7.1 requires of an independent segment, CRA lsb 6, BLA_W_LP
        /// lsb 2, TRAIL_R lsb 4, IDR_N_LP and TRAIL_R lsb 1.
        fn stream(&self) -> Vec<u8> {
            crate::video::bits::annex_b(&self.units())
        }

        fn units(&self) -> Vec<Vec<u8>> {
            vec![
                self.sps.nal(),
                self.pps(),
                self.segment(nal::IDR_W_RADL, true, 0),
                self.segment(TRAIL_R, true, 3),
                self.segment(TRAIL_R, false, 3),
                self.segment(nal::CRA, true, 6),
                self.segment(nal::BLA_W_LP, true, 2),
                self.segment(TRAIL_R, true, 4),
                self.segment(nal::IDR_N_LP, true, 0),
                self.segment(TRAIL_R, true, 1),
            ]
        }
    }

    /// 7.4.2.4.4: after the last VCL NAL unit of a picture, the first NAL
    /// unit with nuh_layer_id 0 of type RSV_NVCL41 to RSV_NVCL44, among
    /// others, starts the next access unit. Types 45 to 47 are not in that
    /// list, and neither is any of these types at nuh_layer_id above 0. One
    /// such unit goes between the two-segment TRAIL_R and the CRA of
    /// `Segments::stream`. A starter opens the CRA's access unit, which then
    /// begins at the starter and grows by its length. Any other unit joins
    /// the TRAIL_R's access unit, as a reserved VCL unit does. The output
    /// positions and random access flags do not change either way.
    #[test]
    fn reserved_non_vcl_41_to_44_start_an_access_unit_at_layer_0() -> Result<(), VideoError> {
        let segments = Segments::default();
        let (plain, _) = access_units(&segments.stream())?;
        for (kind, layer, starts) in [
            (41u8, 0u8, true),
            (44, 0, true),
            (45, 0, false),
            (41, 1, false),
            (44, 1, false),
        ] {
            let mut units = segments.units();
            let unit =
                Writer::default().nal(&[(kind << 1) | (layer >> 5), ((layer & 0x1F) << 3) | 1]);
            let grown = unit.len() + 4;
            let at = plain.get(1).map_or(0, |trail| trail.range.end);
            units.insert(5, unit);
            let (found, _) = access_units(&crate::video::bits::annex_b(&units))?;
            let expected: Vec<_> = plain
                .iter()
                .enumerate()
                .map(|(index, unit)| {
                    let shift = |offset: usize| if index > 1 { offset + grown } else { offset };
                    let (start, anchor, end) = match (index, starts) {
                        (1, false) => (unit.range.start, unit.anchor, unit.range.end + grown),
                        (2, true) => (at, at + 1, unit.range.end + grown),
                        _ => (
                            shift(unit.range.start),
                            shift(unit.anchor),
                            shift(unit.range.end),
                        ),
                    };
                    AccessUnit {
                        range: start..end,
                        anchor,
                        ..unit.clone()
                    }
                })
                .collect();
            assert_eq!(
                found, expected,
                "nal_unit_type {kind} at nuh_layer_id {layer}"
            );
        }
        Ok(())
    }

    /// 7.4.2.2: decoders ignore, remove and discard, every NAL unit of a
    /// reserved nal_unit_type, and Table 7-1 reserves the VCL types 10 to 15
    /// and 22 to 31. A reserved unit written as a first slice segment, lsb 9,
    /// between the two-segment TRAIL_R and the CRA of `Segments::stream`
    /// starts no picture, so the access units keep their output positions
    /// and random access flags. Its bytes are discarded inside the access
    /// unit it follows, which grows by its length, and every later access
    /// unit moves by that length.
    #[test]
    fn a_reserved_vcl_unit_is_ignored() -> Result<(), VideoError> {
        let segments = Segments::default();
        let (plain, _) = access_units(&segments.stream())?;
        for kind in [10, 13, 15, 22, 23, 24, 31] {
            let mut units = segments.units();
            let reserved = segments.segment(kind, true, 9);
            let grown = reserved.len() + 4;
            units.insert(5, reserved);
            let (found, _) = access_units(&crate::video::bits::annex_b(&units))?;
            let expected: Vec<_> = plain
                .iter()
                .enumerate()
                .map(|(index, unit)| {
                    let shift = |at: usize| if index > 1 { at + grown } else { at };
                    let end = if index >= 1 {
                        unit.range.end + grown
                    } else {
                        unit.range.end
                    };
                    AccessUnit {
                        range: shift(unit.range.start)..end,
                        anchor: shift(unit.anchor),
                        ..unit.clone()
                    }
                })
                .collect();
            assert_eq!(found, expected, "nal_unit_type {kind}");
        }
        Ok(())
    }

    /// The pass 5 sweep of 7.3.6.1 up to slice_pic_order_cnt_lsb. Under 8.3.1
    /// the stream of `Segments::stream` is seven pictures: the IDR at POC 0,
    /// the two-segment TRAIL_R at 3 and the CRA, which neither opens the
    /// stream nor follows an end of sequence, at 6, all in sequence 0. The
    /// BLA opens sequence 1 at 2, then 4, and the IDR_N_LP opens sequence 2
    /// at 0, then 1. Each row turns on one condition of the header, and the
    /// two refusals sit one past a bound or on pic_output_flag 0.
    #[test]
    fn every_slice_header_field_before_the_lsb_is_read() {
        let expected = [(0, 0), (0, 3), (0, 6), (1, 2), (1, 4), (2, 0), (2, 1)];
        let rows: Vec<(&str, Segments, Option<&str>)> = vec![
            ("base", Segments::default(), None),
            (
                "pps 63, dependent slices, pic_output_flag 1, 2 extra bits",
                Segments {
                    pps_id: 63,
                    dependent_enabled: true,
                    output_flag_present: true,
                    pic_output: true,
                    extra_bits: 2,
                    ..Segments::default()
                },
                None,
            ),
            (
                "7 extra bits",
                Segments {
                    extra_bits: 7,
                    ..Segments::default()
                },
                None,
            ),
            (
                "colour_plane_id 2",
                Segments {
                    sps: Sps {
                        chroma: 3,
                        separate_planes: true,
                        ..Sps::default()
                    },
                    colour_plane: Some(2),
                    ..Segments::default()
                },
                None,
            ),
            (
                "slice pps id 64",
                Segments {
                    pps_id: 63,
                    slice_pps_id: Some(64),
                    ..Segments::default()
                },
                Some("slice_pic_parameter_set_id over 63"),
            ),
            // 7.4.3.3.1: pps_seq_parameter_set_id is 0 to 15.
            (
                "sps id 15",
                Segments {
                    sps: Sps {
                        id: 15,
                        ..Sps::default()
                    },
                    ..Segments::default()
                },
                None,
            ),
            (
                "pps sps id 16",
                Segments {
                    sps: Sps {
                        id: 15,
                        ..Sps::default()
                    },
                    pps_sps_id: Some(16),
                    ..Segments::default()
                },
                Some("pps_seq_parameter_set_id over 15"),
            ),
            (
                "pic_output_flag 0",
                Segments {
                    output_flag_present: true,
                    pic_output: false,
                    ..Segments::default()
                },
                Some("pic_output_flag 0"),
            ),
        ];
        let mut failures = Vec::new();
        for (name, segments, refusal) in rows {
            let stream = segments.stream();
            crate::video::bits::dump_row("h265", &format!("segments {name}"), &stream);
            let outcome = access_units(&stream)
                .map(|(found, _)| found.iter().map(|unit| unit.order).collect::<Vec<_>>())
                .map_err(|error| error.to_string());
            let matches = match (&outcome, refusal) {
                (Ok(orders), None) => orders == &expected,
                (Err(text), Some(reason)) => text.contains(reason),
                _ => false,
            };
            if !matches {
                failures.push(format!("{name}: {outcome:?}, expected {refusal:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// Review pass 2 S1: every range the HEVC SPS parser checks, at its
    /// bound, accepted, and one step past it, refused by its reason, the
    /// clause named per row.
    #[test]
    fn every_hevc_bound_is_exact() {
        let base = Sps::default;
        // CTB 16 so that MaxTbLog2SizeY can sit at Min( CtbLog2SizeY, 5 ).
        let small = || Sps {
            min_cb: 0,
            diff_cb: 1,
            diff_tb: 2,
            ..base()
        };
        let ctb64 = || Sps {
            min_cb: 0,
            diff_cb: 3,
            min_tb: 0,
            diff_tb: 3,
            ..base()
        };
        let scaling = |scaling: Scaling| Sps {
            scaling: Some(scaling),
            ..base()
        };
        let rows: Vec<(&str, Sps, Option<&str>)> = vec![
            ("base", base(), None),
            // 7.4.3.2.1: bit depths 0 to 8.
            (
                "bit depth 8",
                Sps {
                    bit_depth: 8,
                    bit_depth_chroma: 8,
                    ..base()
                },
                None,
            ),
            (
                "bit depth 9",
                Sps {
                    bit_depth: 9,
                    ..base()
                },
                Some("bit_depth_luma_minus8 over 8"),
            ),
            (
                "chroma bit depth 9",
                Sps {
                    bit_depth_chroma: 9,
                    ..base()
                },
                Some("bit_depth_chroma_minus8 over 8"),
            ),
            // A.4.1 b and c at level 7.2: each side at most 33 776, the
            // largest multiple of MinCbSizeY 8 being 33 776 itself.
            (
                "width 33776",
                Sps {
                    width: 33_776,
                    height: 8,
                    ..small()
                },
                None,
            ),
            (
                "width 33784",
                Sps {
                    width: 33_784,
                    height: 8,
                    ..small()
                },
                Some("pic_width_in_luma_samples over"),
            ),
            (
                "height 33784",
                Sps {
                    width: 8,
                    height: 33_784,
                    ..small()
                },
                Some("pic_height_in_luma_samples over"),
            ),
            (
                "width 65536 at 8.5",
                Sps {
                    level: 255,
                    width: 65_536,
                    height: 8,
                    ..small()
                },
                None,
            ),
            // 7.4.3.2.1: a side not 0 and a multiple of MinCbSizeY.
            (
                "width 0",
                Sps { width: 0, ..base() },
                Some("picture side of 0"),
            ),
            (
                "width 60 at MinCb 16",
                Sps {
                    width: 60,
                    min_cb: 1,
                    diff_cb: 1,
                    ..base()
                },
                Some("multiple of MinCbSizeY"),
            ),
            (
                "width 56 at MinCb 8",
                Sps {
                    width: 56,
                    ..base()
                },
                None,
            ),
            // 7.4.3.2.1: SubWidthC * ( left + right ) below the width.
            (
                "window 31 units",
                Sps {
                    conformance: Some([15, 16, 0, 0]),
                    ..base()
                },
                None,
            ),
            (
                "window 32 units",
                Sps {
                    conformance: Some([16, 16, 0, 0]),
                    ..base()
                },
                Some("cropping window"),
            ),
            ("lsb 12", Sps { lsb: 12, ..base() }, None),
            (
                "lsb 13",
                Sps { lsb: 13, ..base() },
                Some("log2_max_pic_order_cnt_lsb_minus4 over 12"),
            ),
            // A.4.2 by A-2: sps_max_dec_pic_buffering_minus1 below 16, except at
            // level 8.5, where a DPB of 17 is accepted.
            (
                "DPB 16",
                Sps {
                    dpb: 15,
                    reorder: 15,
                    ..base()
                },
                None,
            ),
            (
                "DPB 17",
                Sps {
                    dpb: 16,
                    reorder: 0,
                    ..base()
                },
                Some("sps_max_dec_pic_buffering_minus1 over"),
            ),
            (
                "DPB 17 at 8.5",
                Sps {
                    level: 255,
                    dpb: 16,
                    reorder: 0,
                    ..base()
                },
                None,
            ),
            (
                "DPB 17 at 6.2",
                Sps {
                    level: 186,
                    dpb: 16,
                    reorder: 0,
                    ..base()
                },
                Some("sps_max_dec_pic_buffering_minus1 over"),
            ),
            (
                "reorder 5 of 4",
                Sps {
                    dpb: 4,
                    reorder: 5,
                    ..base()
                },
                Some("sps_max_num_reorder_pics over"),
            ),
            (
                "reorder 4 of 4",
                Sps {
                    dpb: 4,
                    reorder: 4,
                    ..base()
                },
                None,
            ),
            (
                "latency 2^32 - 2",
                Sps {
                    latency: u64::from(u32::MAX) - 1,
                    ..base()
                },
                None,
            ),
            (
                "latency 2^32 - 1",
                Sps {
                    latency: u64::from(u32::MAX),
                    ..base()
                },
                Some("longer than 32 bits"),
            ),
            // 7-10, 7-11 and the A.3 profiles: CtbLog2SizeY 4 to 6.
            ("CTB 64", ctb64(), None),
            (
                "CTB 128",
                Sps {
                    min_cb: 0,
                    diff_cb: 4,
                    ..base()
                },
                Some("coding tree block larger than 64"),
            ),
            (
                "MinCb 64",
                Sps {
                    min_cb: 3,
                    diff_cb: 0,
                    min_tb: 0,
                    diff_tb: 3,
                    ..base()
                },
                None,
            ),
            (
                "MinCb 128",
                Sps {
                    min_cb: 4,
                    diff_cb: 0,
                    ..base()
                },
                Some("log2_min_luma_coding_block_size_minus3 over 3"),
            ),
            (
                "CTB 8",
                Sps {
                    min_cb: 0,
                    diff_cb: 0,
                    diff_tb: 1,
                    ..base()
                },
                Some("smaller than 16"),
            ),
            ("CTB 16", small(), None),
            // 7.4.3.2.1: MinTbLog2SizeY below MinCbLog2SizeY, MaxTbLog2SizeY at
            // most Min( CtbLog2SizeY, 5 ), depths at most CtbLog2SizeY -
            // MinTbLog2SizeY.
            (
                "MinTb at MinCb",
                Sps {
                    min_cb: 0,
                    diff_cb: 3,
                    min_tb: 1,
                    diff_tb: 0,
                    ..base()
                },
                Some("MinTbLog2SizeY not below"),
            ),
            (
                "MinTb 32 under MinCb 64",
                Sps {
                    min_cb: 3,
                    diff_cb: 0,
                    min_tb: 3,
                    diff_tb: 0,
                    ..base()
                },
                None,
            ),
            (
                "MaxTb 64",
                Sps {
                    diff_tb: 4,
                    ..ctb64()
                },
                Some("MaxTbLog2SizeY over"),
            ),
            (
                "MaxTb 32 at CTB 16",
                Sps {
                    diff_tb: 3,
                    ..small()
                },
                Some("MaxTbLog2SizeY over"),
            ),
            (
                "depths 4",
                Sps {
                    depth_inter: 4,
                    depth_intra: 4,
                    ..ctb64()
                },
                None,
            ),
            (
                "inter depth 5",
                Sps {
                    depth_inter: 5,
                    ..ctb64()
                },
                Some("max_transform_hierarchy_depth_inter"),
            ),
            (
                "intra depth 5",
                Sps {
                    depth_intra: 5,
                    ..ctb64()
                },
                Some("max_transform_hierarchy_depth_intra"),
            ),
            // 7.4.3.2.1 PCM: depths at most the picture's, Log2MinIpcmCbSizeY
            // Min( MinCbLog2SizeY, 5 ) to Min( CtbLog2SizeY, 5 ), and
            // Log2MaxIpcmCbSizeY at most Min( CtbLog2SizeY, 5 ).
            (
                "PCM 8 to 32",
                Sps {
                    pcm: Some((8, 8, 0, 2)),
                    ..base()
                },
                None,
            ),
            (
                "PCM luma 9 bits",
                Sps {
                    pcm: Some((9, 8, 0, 0)),
                    ..base()
                },
                Some("PCM sample deeper"),
            ),
            (
                "PCM chroma 9 bits",
                Sps {
                    pcm: Some((8, 9, 0, 0)),
                    ..base()
                },
                Some("PCM sample deeper"),
            ),
            (
                "PCM 10 bits at 10",
                Sps {
                    bit_depth: 2,
                    bit_depth_chroma: 2,
                    pcm: Some((10, 10, 0, 0)),
                    ..base()
                },
                None,
            ),
            (
                "PCM max 64",
                Sps {
                    pcm: Some((8, 8, 0, 3)),
                    ..base()
                },
                Some("Log2MaxIpcmCbSizeY too large"),
            ),
            (
                "PCM min 64",
                Sps {
                    pcm: Some((8, 8, 3, 0)),
                    ..base()
                },
                Some("Log2MinIpcmCbSizeY too large"),
            ),
            (
                "PCM min 8 under MinCb 16",
                Sps {
                    min_cb: 1,
                    diff_cb: 1,
                    pcm: Some((8, 8, 0, 1)),
                    ..base()
                },
                Some("Log2MinIpcmCbSizeY too small"),
            ),
            (
                "PCM min 16 at MinCb 16",
                Sps {
                    min_cb: 1,
                    diff_cb: 1,
                    pcm: Some((8, 8, 1, 0)),
                    ..base()
                },
                None,
            ),
            (
                "PCM min 32 at MinCb 64",
                Sps {
                    min_cb: 3,
                    diff_cb: 0,
                    min_tb: 0,
                    diff_tb: 3,
                    pcm: Some((8, 8, 2, 0)),
                    ..base()
                },
                None,
            ),
            // 7.4.8: num_negative_pics at most sps_max_dec_pic_buffering_minus1,
            // and num_positive_pics at most what is left.
            (
                "15 negative of 15",
                Sps {
                    dpb: 15,
                    reorder: 0,
                    sets: vec![(15, 0)],
                    ..base()
                },
                None,
            ),
            (
                "5 negative of 4",
                Sps {
                    dpb: 4,
                    reorder: 0,
                    sets: vec![(5, 0)],
                    ..base()
                },
                Some("num_negative_pics over"),
            ),
            (
                "2 and 2 of 4",
                Sps {
                    dpb: 4,
                    reorder: 0,
                    sets: vec![(2, 2)],
                    ..base()
                },
                None,
            ),
            (
                "2 and 3 of 4",
                Sps {
                    dpb: 4,
                    reorder: 0,
                    sets: vec![(2, 3)],
                    ..base()
                },
                Some("num_positive_pics over"),
            ),
            // 7.4.5: dc -7 to 247, delta -128 to 127, pred_matrix_id_delta 0 to
            // matrixId, or matrixId / 3 for sizeId 3.
            (
                "dc -7",
                scaling(Scaling {
                    dc: Some(-7),
                    ..Scaling::default()
                }),
                None,
            ),
            (
                "dc 247",
                scaling(Scaling {
                    dc: Some(247),
                    ..Scaling::default()
                }),
                None,
            ),
            (
                "dc -8",
                scaling(Scaling {
                    dc: Some(-8),
                    ..Scaling::default()
                }),
                Some("scaling_list_dc_coef_minus8"),
            ),
            (
                "dc 248",
                scaling(Scaling {
                    dc: Some(248),
                    ..Scaling::default()
                }),
                Some("scaling_list_dc_coef_minus8"),
            ),
            (
                "delta -128",
                scaling(Scaling {
                    delta: Some(-128),
                    ..Scaling::default()
                }),
                None,
            ),
            (
                "delta 127",
                scaling(Scaling {
                    delta: Some(127),
                    ..Scaling::default()
                }),
                None,
            ),
            (
                "delta 128",
                scaling(Scaling {
                    delta: Some(128),
                    ..Scaling::default()
                }),
                Some("scaling_list_delta_coef"),
            ),
            (
                "delta -129",
                scaling(Scaling {
                    delta: Some(-129),
                    ..Scaling::default()
                }),
                Some("scaling_list_delta_coef"),
            ),
            (
                "pred 5 of 5",
                scaling(Scaling {
                    pred: Some(((1, 5), 5)),
                    ..Scaling::default()
                }),
                None,
            ),
            (
                "pred 6 of 5",
                scaling(Scaling {
                    pred: Some(((1, 5), 6)),
                    ..Scaling::default()
                }),
                Some("scaling_list_pred_matrix_id_delta"),
            ),
            (
                "pred 1 of sizeId 3 matrix 3",
                scaling(Scaling {
                    pred: Some(((3, 3), 1)),
                    ..Scaling::default()
                }),
                None,
            ),
            (
                "pred 2 of sizeId 3 matrix 3",
                scaling(Scaling {
                    pred: Some(((3, 3), 2)),
                    ..Scaling::default()
                }),
                Some("scaling_list_pred_matrix_id_delta"),
            ),
            // E.3.1: chroma_sample_loc_type 0 to 5.
            (
                "chroma loc 5",
                Sps {
                    chroma_loc: Some((5, 5)),
                    ..base()
                },
                None,
            ),
            (
                "chroma loc top 6",
                Sps {
                    chroma_loc: Some((6, 0)),
                    ..base()
                },
                Some("chroma_sample_loc_type_top_field over 5"),
            ),
            (
                "chroma loc bottom 6",
                Sps {
                    chroma_loc: Some((0, 6)),
                    ..base()
                },
                Some("chroma_sample_loc_type_bottom_field over 5"),
            ),
        ];
        let mut failures = Vec::new();
        for (name, sps, expected) in rows {
            crate::video::bits::dump_row(
                "h265",
                name,
                &[[0, 0, 0, 1].as_slice(), &sps.nal()].concat(),
            );
            let outcome = parse_sps(&sps.nal())
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

    /// N1 of review pass 2, 8-1 at both boundaries, MaxPicOrderCntLsb 16 so
    /// half is 8. TRAIL_R lsb 6 then 12 keep PicOrderCntMsb 0. From lsb 12,
    /// lsb 4 is 8 below, which is >= 8, so PicOrderCntMsb gains 16 and POC is
    /// 20. From lsb 4, lsb 12 is 8 above, which is not > 8, so POC is 12.
    #[test]
    fn poc_lsb_wrap_boundaries_of_8_1() -> Result<(), VideoError> {
        let mut state = PocState::default();
        let found = run(
            &mut state,
            &[
                (nal::IDR_W_RADL, 0, 0),
                (TRAIL_R, 0, 6),
                (TRAIL_R, 0, 12),
                (TRAIL_R, 0, 4),
            ],
        )?;
        assert_eq!(found, [(0, 0), (0, 6), (0, 12), (0, 20)]);
        let mut state = PocState::default();
        let found = run(
            &mut state,
            &[(nal::IDR_W_RADL, 0, 0), (TRAIL_R, 0, 4), (TRAIL_R, 0, 12)],
        )?;
        assert_eq!(found, [(0, 0), (0, 4), (0, 12)]);
        Ok(())
    }
}
