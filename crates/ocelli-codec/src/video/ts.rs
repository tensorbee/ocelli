//! The MPEG-2 transport stream.
//!
//! Every syntax table and field width here is from Rec. ITU-T H.222.0 v9
//! (08/2023), which is ISO/IEC 13818-1:2023, the clause named beside each
//! parser. The video elementary stream is reassembled from its PES packets
//! and split into access units by its codec, and each access unit takes the
//! PTS of the PES packet it commences in.

use std::num::NonZeroU32;

use super::{
    CodecConfig, SampleFormat, SampleTable, VideoCodec, VideoContainer, VideoError,
    bits::{be_u8, be_u16},
    h264, h265, mpeg2,
    table::{AccessUnit, Decoded, StreamInfo, presentation_order},
};

/// 2.4.3.2, a transport packet is 188 bytes and starts with sync_byte 0x47.
pub(crate) const PACKET: usize = 188;
pub(crate) const SYNC_BYTE: u8 = 0x47;
/// Table 2-3, the program association table's PID.
const PAT_PID: u16 = 0x0000;
/// Table 2-31, table_id values.
const PAT_TABLE_ID: u8 = 0x00;
const PMT_TABLE_ID: u8 = 0x02;
/// 2.4.4.6 and 2.4.4.10, section_length shall not exceed 1021.
const MAX_SECTION_LENGTH: usize = 1021;
/// 2.4.3.7, equation 2-11: the PTS counts a 90 kHz clock modulo 2^33.
const CLOCK: u32 = 90_000;
const PTS_MODULUS: i64 = 1 << 33;
/// Table 2-22 stream_id values whose PES packets carry no optional header:
/// program_stream_map, padding_stream, private_stream_2, ECM, EMM,
/// program_stream_directory, DSMCC_stream and H.222.1 type E.
const NO_HEADER_STREAMS: [u8; 8] = [0xBC, 0xBE, 0xBF, 0xF0, 0xF1, 0xFF, 0xF2, 0xF8];
/// Table 2-34, the stream_type values of the three video codecs.
const MPEG2_VIDEO: u8 = 0x02;
const AVC_VIDEO: u8 = 0x1B;
const HEVC_VIDEO: u8 = 0x24;
/// Table 2-34, stream_type values that are audio: ISO/IEC 11172-3,
/// 13818-3, 13818-7 ADTS, 14496-3 LATM, 14496-3 without transport syntax and
/// 23008-3 MHAS main and auxiliary.
const AUDIO_STREAMS: [u8; 7] = [0x03, 0x04, 0x0F, 0x11, 0x1C, 0x2D, 0x2E];

const STRUCTURE: &str = "MPEG-2 transport stream";

fn malformed(reason: &'static str) -> VideoError {
    VideoError::Malformed {
        structure: STRUCTURE,
        reason,
    }
}

/// One transport packet's header fields and payload, Table 2-2 and Table
/// 2-6.
struct Packet<'a> {
    payload_unit_start: bool,
    pid: u16,
    continuity_counter: u8,
    discontinuity: bool,
    payload: Option<&'a [u8]>,
}

fn packet(bytes: &[u8]) -> Result<Packet<'_>, VideoError> {
    if bytes.first() != Some(&SYNC_BYTE) {
        return Err(malformed("a packet without sync_byte 0x47"));
    }
    let flags = be_u16(bytes, 1, STRUCTURE)?;
    if flags & 0x8000 != 0 {
        return Err(malformed("a packet with transport_error_indicator set"));
    }
    let last = be_u8(bytes, 3, STRUCTURE)?;
    if last >> 6 != 0 {
        return Err(VideoError::Unsupported {
            feature: "a scrambled transport stream",
        });
    }
    let control = (last >> 4) & 0x03;
    let mut payload_start = 4usize;
    let mut discontinuity = false;
    if control & 0x02 != 0 {
        // adaptation_field_length, then discontinuity_indicator in the top
        // bit of the flags byte when the length is not zero.
        let length = usize::from(be_u8(bytes, 4, STRUCTURE)?);
        if length > 0 {
            discontinuity = be_u8(bytes, 5, STRUCTURE)? & 0x80 != 0;
        }
        payload_start = 5 + length;
        if payload_start > PACKET {
            return Err(malformed("an adaptation field longer than its packet"));
        }
    }
    // Table 2-5: '00' is reserved and its packets are discarded, '10' has no
    // payload.
    let payload = (control & 0x01 != 0)
        .then(|| bytes.get(payload_start..))
        .flatten();
    Ok(Packet {
        payload_unit_start: flags & 0x4000 != 0,
        pid: flags & 0x1FFF,
        continuity_counter: last & 0x0F,
        discontinuity,
        payload,
    })
}

/// A PSI section being gathered across packets.
#[derive(Default)]
struct Section {
    bytes: Vec<u8>,
    open: bool,
}

impl Section {
    /// Adds a packet's payload and returns a complete section when one ends.
    fn push(&mut self, packet: &Packet<'_>) -> Result<Option<Vec<u8>>, VideoError> {
        let Some(payload) = packet.payload else {
            return Ok(None);
        };
        if packet.payload_unit_start {
            // 2.4.4.2, pointer_field counts the bytes before the section.
            let pointer = usize::from(be_u8(payload, 0, STRUCTURE)?);
            self.bytes.clear();
            self.bytes.extend_from_slice(
                payload
                    .get(1 + pointer..)
                    .ok_or_else(|| malformed("a pointer_field past its packet"))?,
            );
            self.open = true;
        } else if self.open {
            self.bytes.extend_from_slice(payload);
        }
        if !self.open || self.bytes.len() < 3 {
            return Ok(None);
        }
        let length = usize::from(be_u16(&self.bytes, 1, STRUCTURE)? & 0x0FFF);
        if length > MAX_SECTION_LENGTH {
            return Err(malformed("a section_length over 1021"));
        }
        if self.bytes.len() < 3 + length {
            return Ok(None);
        }
        self.open = false;
        let section = self.bytes.get(..3 + length).unwrap_or_default().to_vec();
        if crc32(&section) != 0 {
            return Err(malformed("a section whose CRC_32 does not check"));
        }
        Ok(Some(section))
    }
}

/// Annex A: the CRC of polynomial (A-1), registers starting at 1, bits most
/// significant first. Over a section including its CRC_32 the result is 0.
fn crc32(bytes: &[u8]) -> u32 {
    const POLYNOMIAL: u32 = 0x04C1_1DB7;
    let mut register = u32::MAX;
    for byte in bytes {
        register ^= u32::from(*byte) << 24;
        for _ in 0..8 {
            register = if register & 0x8000_0000 != 0 {
                (register << 1) ^ POLYNOMIAL
            } else {
                register << 1
            };
        }
    }
    register
}

/// 2.4.4.6 and 2.4.4.10: current_next_indicator, the low bit of byte 5 of a
/// PAT or PMT section, is 1 for the table that applies now and 0 for the next
/// one, which is not used yet.
fn is_current(section: &[u8]) -> Result<bool, VideoError> {
    Ok(be_u8(section, 5, STRUCTURE)? & 0x01 == 1)
}

/// 2.4.4.4, the PMT PID of the first program, skipping network_PID entries.
fn program_map_pid(section: &[u8]) -> Result<Option<u16>, VideoError> {
    if be_u8(section, 0, STRUCTURE)? != PAT_TABLE_ID {
        return Ok(None);
    }
    // The loop runs from byte 8 to the four CRC_32 bytes.
    let end = section.len().saturating_sub(4);
    let mut position = 8usize;
    while position + 4 <= end {
        let number = be_u16(section, position, STRUCTURE)?;
        let pid = be_u16(section, position + 2, STRUCTURE)? & 0x1FFF;
        if number != 0 {
            return Ok(Some(pid));
        }
        position += 4;
    }
    Ok(None)
}

/// 2.4.4.9, the first video stream of the program and whether any audio
/// stream is present.
fn program_streams(section: &[u8]) -> Result<Option<(VideoCodec, u16, bool)>, VideoError> {
    if be_u8(section, 0, STRUCTURE)? != PMT_TABLE_ID {
        return Ok(None);
    }
    let info_length = usize::from(be_u16(section, 10, STRUCTURE)? & 0x0FFF);
    let end = section.len().saturating_sub(4);
    let mut position = 12 + info_length;
    let mut video = None;
    let mut audio = false;
    while position + 5 <= end {
        let stream_type = be_u8(section, position, STRUCTURE)?;
        let pid = be_u16(section, position + 1, STRUCTURE)? & 0x1FFF;
        let es_info_length = usize::from(be_u16(section, position + 3, STRUCTURE)? & 0x0FFF);
        let codec = match stream_type {
            MPEG2_VIDEO => Some(VideoCodec::Mpeg2),
            AVC_VIDEO => Some(VideoCodec::H264),
            HEVC_VIDEO => Some(VideoCodec::H265),
            _ => None,
        };
        if let Some(codec) = codec
            && video.is_none()
        {
            video = Some((codec, pid));
        }
        audio |= AUDIO_STREAMS.contains(&stream_type);
        position += 5 + es_info_length;
    }
    Ok(video.map(|(codec, pid)| (codec, pid, audio)))
}

/// One PES packet's place in the reassembled elementary stream and its PTS.
struct Stamp {
    start: usize,
    pts: Option<i64>,
}

/// 2.4.3.6 and 2.4.3.7: appends the payload of one PES packet to `es` and
/// returns its PTS.
fn pes(packet: &[u8], es: &mut Vec<u8>) -> Result<Option<i64>, VideoError> {
    if packet.get(..3) != Some(&[0, 0, 1][..]) {
        return Err(malformed("a PES packet without packet_start_code_prefix"));
    }
    let stream_id = be_u8(packet, 3, STRUCTURE)?;
    if NO_HEADER_STREAMS.contains(&stream_id) {
        return Err(malformed(
            "a video PES packet of a stream_id without a header",
        ));
    }
    let length = usize::from(be_u16(packet, 4, STRUCTURE)?);
    let flags = be_u16(packet, 6, STRUCTURE)?;
    if flags >> 14 != 0b10 {
        return Err(malformed("a PES header without its '10' marker bits"));
    }
    if (flags >> 12) & 0x03 != 0 {
        return Err(VideoError::Unsupported {
            feature: "a scrambled PES packet",
        });
    }
    let pts_dts = (flags >> 6) & 0x03;
    if pts_dts == 0b01 {
        return Err(malformed("PTS_DTS_flags '01', which is forbidden"));
    }
    let header_length = usize::from(be_u8(packet, 8, STRUCTURE)?);
    let pts = if pts_dts & 0b10 != 0 {
        Some(timestamp(packet.get(9..14).unwrap_or_default())?)
    } else {
        None
    };
    let start = 9 + header_length;
    // PES_packet_length counts the bytes after itself, and 0 is allowed for
    // video in a transport stream to mean unbounded.
    let end = if length == 0 {
        packet.len()
    } else {
        6 + length
    };
    let payload = packet.get(start..end).ok_or(VideoError::Truncated {
        structure: "PES packet",
    })?;
    es.extend_from_slice(payload);
    Ok(pts)
}

/// PTS[32..30], PTS[29..15] and PTS[14..0], each followed by a marker_bit,
/// behind a four bit prefix, Table 2-21.
fn timestamp(field: &[u8]) -> Result<i64, VideoError> {
    let first = be_u8(field, 0, STRUCTURE)?;
    let middle = be_u16(field, 1, STRUCTURE)?;
    let last = be_u16(field, 3, STRUCTURE)?;
    if first & 0x01 == 0 || middle & 0x01 == 0 || last & 0x01 == 0 {
        return Err(malformed("a PTS without its marker bits"));
    }
    Ok(i64::from((first >> 1) & 0x07) << 30 | i64::from(middle >> 1) << 15 | i64::from(last >> 1))
}

/// The difference of two 33-bit counts taken modulo 2^33 into
/// -2^32 to 2^32 - 1, so a count that wraps keeps increasing.
fn wrapped_step(from: i64, to: i64) -> i64 {
    let half = PTS_MODULUS / 2;
    (to - from + half).rem_euclid(PTS_MODULUS) - half
}

/// Reads the first video stream of the first program.
pub(crate) fn demux(data: &[u8], codec: VideoCodec) -> Result<SampleTable, VideoError> {
    if !data.len().is_multiple_of(PACKET) {
        return Err(VideoError::Truncated {
            structure: "transport stream packet",
        });
    }
    let mut pat = Section::default();
    let mut pmt = Section::default();
    let mut pmt_pid: Option<u16> = None;
    let mut program: Option<(VideoCodec, u16, bool)> = None;
    let mut es: Vec<u8> = Vec::new();
    let mut stamps: Vec<Stamp> = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut last_counter: Option<u8> = None;
    for bytes in data.chunks_exact(PACKET) {
        let packet = packet(bytes)?;
        if packet.pid == PAT_PID && pmt_pid.is_none() {
            if let Some(section) = pat.push(&packet)?
                && is_current(&section)?
            {
                pmt_pid = program_map_pid(&section)?;
            }
            continue;
        }
        if Some(packet.pid) == pmt_pid && program.is_none() {
            if let Some(section) = pmt.push(&packet)?
                && is_current(&section)?
            {
                program = program_streams(&section)?;
                if program.is_none() {
                    return Err(VideoError::NoVideoStream);
                }
            }
            continue;
        }
        let Some((_, video_pid, _)) = program else {
            continue;
        };
        if packet.pid != video_pid {
            continue;
        }
        let Some(payload) = packet.payload else {
            continue;
        };
        // 2.4.3.3: the counter increments by one per packet with a payload,
        // a duplicate repeats it, and a discontinuity_indicator excuses a
        // jump.
        if let Some(previous) = last_counter {
            if packet.continuity_counter == previous {
                continue;
            }
            if packet.continuity_counter != (previous + 1) % 16 && !packet.discontinuity {
                return Err(malformed(
                    "a continuity_counter jump, so packets are missing",
                ));
            }
        }
        last_counter = Some(packet.continuity_counter);
        if packet.payload_unit_start {
            if !current.is_empty() {
                stamps.push(Stamp {
                    start: es.len(),
                    pts: pes(&current, &mut es)?,
                });
            }
            current.clear();
            current.extend_from_slice(payload);
        } else if !current.is_empty() {
            current.extend_from_slice(payload);
        }
    }
    if !current.is_empty() {
        stamps.push(Stamp {
            start: es.len(),
            pts: pes(&current, &mut es)?,
        });
    }
    let (found, has_audio) = match program {
        Some((found, _, audio)) => (found, audio),
        None => return Err(VideoError::NoVideoStream),
    };
    if found != codec {
        return Err(VideoError::CodecMismatch {
            expected: codec,
            found,
        });
    }
    let (units, info): (Vec<AccessUnit>, StreamInfo) = match codec {
        VideoCodec::H264 => h264::access_units(&es)?,
        VideoCodec::H265 => h265::access_units(&es)?,
        VideoCodec::Mpeg2 => mpeg2::access_units(&es)?,
    };
    let decoded = timestamps(units, &stamps)?;
    let (samples, timescale) = presentation_order(decoded, NonZeroU32::new(CLOCK))?;
    Ok(SampleTable {
        container: VideoContainer::MpegTs,
        samples,
        timescale,
        reassembled: Some(es),
        config: CodecConfig {
            codec_string: info.codec_string,
            description: None,
            sample_format: SampleFormat::StartCodes,
            coded_size: info.coded_size,
            visible_size: info.visible_size,
            colour: info.colour,
            has_audio,
            interlaced: info.interlaced,
        },
    })
}

/// 2.4.3.7: a PTS refers to the first access unit that commences in its PES
/// packet, for MPEG-2 the one whose picture start code commences there. An
/// access unit that is not first in its packet has no PTS of its own, and
/// this table does not interpolate one.
fn timestamps(units: Vec<AccessUnit>, stamps: &[Stamp]) -> Result<Vec<Decoded>, VideoError> {
    let mut decoded = Vec::with_capacity(units.len());
    let mut previous_packet: Option<usize> = None;
    let mut previous: Option<(i64, i64)> = None;
    for (decode_index, unit) in units.into_iter().enumerate() {
        let packet = stamps
            .partition_point(|stamp| stamp.start <= unit.anchor)
            .checked_sub(1);
        let raw = match packet {
            Some(index) if previous_packet != Some(index) => {
                stamps.get(index).and_then(|stamp| stamp.pts)
            }
            _ => None,
        }
        .ok_or(VideoError::MissingTimestamp { decode_index })?;
        previous_packet = packet;
        let pts = match previous {
            None => raw,
            Some((previous_raw, previous_pts)) => previous_pts + wrapped_step(previous_raw, raw),
        };
        previous = Some((raw, pts));
        decoded.push(Decoded {
            range: unit.range,
            key: (0, pts),
            pts: Some(pts),
            random_access: unit.random_access,
        });
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::{crc32, wrapped_step};

    /// Annex A over a section including its own CRC_32 gives 0. This PAT is
    /// table_id 0, section_length 13, transport_stream_id 1, version 0,
    /// current, sections 0 of 0, program 1 on PMT PID 0x1000, then the CRC
    /// that Annex A's encoder appends. The CRC bytes come from `figure_a1`
    /// below, the bit-serial register of Figure A.1 written out longhand,
    /// rather than from `crc32`.
    #[test]
    fn crc_of_a_section_with_its_crc_is_zero() {
        let mut section = vec![
            0x00, 0xB0, 0x0D, 0x00, 0x01, 0xC1, 0x00, 0x00, 0x00, 0x01, 0xF0, 0x00,
        ];
        let crc = figure_a1(&section);
        section.extend_from_slice(&crc.to_be_bytes());
        assert_eq!(crc32(&section), 0);
        // A flipped bit fails the check.
        if let Some(byte) = section.get_mut(9) {
            *byte ^= 0x01;
        }
        assert_ne!(crc32(&section), 0);
    }

    /// Figure A.1 as drawn: 32 delay elements starting at 1, input added to
    /// z(31), the sum fed to z(0) and to the adders after z(i) for i in 0, 1,
    /// 3, 4, 6, 7, 9, 10, 11, 15, 21, 22 and 25.
    fn figure_a1(bytes: &[u8]) -> u32 {
        let taps = [0usize, 1, 3, 4, 6, 7, 9, 10, 11, 15, 21, 22, 25];
        let mut z = vec![true; 32];
        for byte in bytes {
            for bit in (0..8).rev() {
                let input = (byte >> bit) & 1 == 1;
                let feedback = input ^ z.last().copied().unwrap_or(false);
                let mut next = Vec::with_capacity(32);
                next.push(feedback);
                for (i, value) in z.iter().take(31).enumerate() {
                    next.push(if taps.contains(&i) {
                        value ^ feedback
                    } else {
                        *value
                    });
                }
                z = next;
            }
        }
        z.iter()
            .rev()
            .fold(0u32, |value, bit| value << 1 | u32::from(*bit))
    }

    #[test]
    fn crc_register_matches_the_figure() {
        for sample in [&b"123456789"[..], &[0x00, 0xB0, 0x0D][..], &[][..]] {
            assert_eq!(crc32(sample), figure_a1(sample));
        }
    }

    /// 2.4.3.7: the PTS is modulo 2^33, so 2^33 - 1 followed by 0 is one tick
    /// later and 0 followed by 2^33 - 1 is one tick earlier.
    #[test]
    fn pts_steps_across_the_wrap() {
        let top = (1i64 << 33) - 1;
        assert_eq!(wrapped_step(top, 0), 1);
        assert_eq!(wrapped_step(0, top), -1);
        assert_eq!(wrapped_step(100, 3700), 3600);
    }
}
