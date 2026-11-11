//! The MP4 sample table.
//!
//! ISO/IEC 14496-12 was not available to read, because its free copy sits
//! behind a licence agreement this work did not accept. The box layouts here
//! are from Apple's QuickTime File Format, the documentation's "Atoms" page
//! and its pages for each sample table atom, which that document says are
//! functionally identical to the ISO boxes. The `colr` box of type `nclx`,
//! the avcC and hvcC records of ISO/IEC 14496-15 and the esds descriptors of
//! ISO/IEC 14496-1 were not available either, and each is marked where it is
//! read.

use std::ops::Range;

use super::{
    CodecConfig, ColourDescription, ColourSource, SampleFormat, SampleTable, VideoCodec,
    VideoContainer, VideoError,
    bits::{be, be_u8, be_u16, be_u32, be_u64, to_usize},
    h264, h265, mpeg2,
    table::{Decoded, MAX_SAMPLES, StreamInfo, presentation_order},
};

/// The fixed fields of a visual sample entry before its child boxes: six
/// reserved bytes and a 16-bit data reference index, then 70 bytes of image
/// description, QuickTime File Format "Video sample description".
const VISUAL_SAMPLE_ENTRY: usize = 78;
/// Version and flags of a full box.
const FULL_BOX: usize = 4;

/// One box: its type and the range of its body.
#[derive(Debug, Clone, Copy)]
struct Mp4Box {
    kind: [u8; 4],
    body: usize,
    end: usize,
}

impl Mp4Box {
    fn range(&self) -> Range<usize> {
        self.body..self.end
    }

    /// The box's own body. Every field is read from this slice, so a count
    /// that runs past the box end is `Truncated` rather than a read of the
    /// next box. `children` checked the range against the file.
    fn bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        data.get(self.range()).unwrap_or_default()
    }
}

/// The boxes inside `range`, "Atoms": a 32-bit size counting the header and
/// a type, a size of 1 meaning a 64-bit size follows the type, and a size of
/// 0 meaning the box runs to the end of its parent. A `uuid` box carries a
/// 16-byte extended type after that.
fn children(data: &[u8], range: Range<usize>) -> Result<Vec<Mp4Box>, VideoError> {
    let mut found = Vec::new();
    let mut position = range.start;
    while position < range.end {
        let size32 = be_u32(data, position, "MP4 box header")?;
        let kind = be::<4>(data, position + 4, "MP4 box header")?;
        let (mut header, size) = match size32 {
            0 => (8, range.end - position),
            1 => {
                let size = be_u64(data, position + 8, "MP4 box header")?;
                (16, usize::try_from(size).map_err(|_| truncated("MP4 box"))?)
            }
            other => (8, to_usize(other, "MP4 box header")?),
        };
        if &kind == b"uuid" {
            header += 16;
        }
        let end = position
            .checked_add(size)
            .ok_or_else(|| truncated("MP4 box"))?;
        if size < header || end > range.end {
            return Err(truncated("MP4 box"));
        }
        found.push(Mp4Box {
            kind,
            body: position + header,
            end,
        });
        position = end;
    }
    Ok(found)
}

fn truncated(structure: &'static str) -> VideoError {
    VideoError::Truncated { structure }
}

fn malformed(structure: &'static str, reason: &'static str) -> VideoError {
    VideoError::Malformed { structure, reason }
}

fn find(data: &[u8], parent: &Mp4Box, kind: &[u8; 4]) -> Result<Option<Mp4Box>, VideoError> {
    Ok(children(data, parent.range())?
        .into_iter()
        .find(|candidate| &candidate.kind == kind))
}

fn require(
    data: &[u8],
    parent: &Mp4Box,
    kind: &[u8; 4],
    structure: &'static str,
) -> Result<Mp4Box, VideoError> {
    find(data, parent, kind)?.ok_or_else(|| malformed(structure, "a required box is missing"))
}

/// The track handler type, "Handler reference atom": version and flags,
/// then a 32-bit component type and the 32-bit subtype that names the media.
fn handler(data: &[u8], trak: &Mp4Box) -> Result<[u8; 4], VideoError> {
    let mdia = require(data, trak, b"mdia", "MP4 track")?;
    let hdlr = require(data, &mdia, b"hdlr", "MP4 track")?;
    be::<4>(hdlr.bytes(data), FULL_BOX + 4, "MP4 handler box")
}

/// Reads the first video track of an MP4 file.
pub(crate) fn demux(data: &[u8], codec: VideoCodec) -> Result<SampleTable, VideoError> {
    let top = children(data, 0..data.len())?;
    if top.iter().any(|candidate| &candidate.kind == b"moof") {
        return Err(fragmented());
    }
    let moov = top
        .iter()
        .find(|candidate| &candidate.kind == b"moov")
        .copied()
        .ok_or_else(|| malformed("MP4 file", "no moov box"))?;
    // A movie extends box only says fragments may follow. The samples of a
    // file without any movie fragment box are all in the sample tables, so it
    // is read like any other.
    let tracks = children(data, moov.range())?;
    let mut video = None;
    let mut has_audio = false;
    for trak in tracks.iter().filter(|candidate| &candidate.kind == b"trak") {
        match &handler(data, trak)? {
            b"vide" if video.is_none() => video = Some(*trak),
            b"soun" => has_audio = true,
            _ => {}
        }
    }
    let trak = video.ok_or(VideoError::NoVideoStream)?;
    let mdia = require(data, &trak, b"mdia", "MP4 track")?;
    let timescale = media_timescale(data, &require(data, &mdia, b"mdhd", "MP4 track")?)?;
    let minf = require(data, &mdia, b"minf", "MP4 track")?;
    let stbl = require(data, &minf, b"stbl", "MP4 track")?;
    let entry = SampleEntry::read(data, &require(data, &stbl, b"stsd", "MP4 sample table")?)?;
    if entry.codec != codec {
        return Err(VideoError::CodecMismatch {
            expected: codec,
            found: entry.codec,
        });
    }
    let decoded = sample_table(data, &stbl)?;
    let (info, description, sample_format) = entry.configuration(data, &decoded)?;
    let (samples, timescale) = presentation_order(decoded, Some(timescale))?;
    let colour = merge_colour(info.colour, info.code_points_signalled, entry.colour);
    Ok(SampleTable {
        container: VideoContainer::Mp4,
        samples,
        timescale,
        reassembled: None,
        config: CodecConfig {
            codec_string: info.codec_string.map(|text| match &entry.kind {
                // The string's prefix is the sample entry type, RFC 6381 3.3.
                b"avc3" => text.replacen("avc1", "avc3", 1),
                b"hev1" => text.replacen("hvc1", "hev1", 1),
                _ => text,
            }),
            description,
            sample_format,
            coded_size: info.coded_size,
            visible_size: info.visible_size,
            colour,
            has_audio,
            interlaced: info.interlaced,
        },
    })
}

/// PS3.5 8.2.8 gives the stream's own characteristics control, field by
/// field, the F-241 lead ruling on review pass 1. The range is the stream's
/// whenever the stream signals one, because the VUI's video_full_range_flag
/// is present whenever video_signal_type_present_flag is set. The code points
/// are the stream's colour description when it has one and the colr box's
/// otherwise, and without either the VUI's inferred 2, unspecified.
fn merge_colour(
    stream: Option<ColourDescription>,
    code_points_signalled: bool,
    colr: Option<ColourDescription>,
) -> Option<ColourDescription> {
    match (stream, colr) {
        (Some(stream), Some(colr)) if !code_points_signalled => Some(ColourDescription {
            code_point_source: colr.code_point_source,
            colour_primaries: colr.colour_primaries,
            transfer_characteristics: colr.transfer_characteristics,
            matrix_coefficients: colr.matrix_coefficients,
            ..stream
        }),
        (Some(stream), _) => Some(stream),
        (None, colr) => colr,
    }
}

fn fragmented() -> VideoError {
    VideoError::Unsupported {
        feature: "a fragmented MP4 file, whose samples are in movie fragments,",
    }
}

/// "Media header atom": version and flags, then two 32-bit times before the
/// 32-bit time scale, the only version that document gives. Version 1, with
/// the two times widened to 64 bits, is the ISO/IEC 14496-12 form, which was
/// not available to read.
fn media_timescale(data: &[u8], mdhd: &Mp4Box) -> Result<std::num::NonZeroU32, VideoError> {
    let offset = match be_u8(mdhd.bytes(data), 0, "MP4 media header")? {
        0 => FULL_BOX + 8,
        1 => FULL_BOX + 16,
        _ => return Err(malformed("MP4 media header", "a version other than 0 or 1")),
    };
    std::num::NonZeroU32::new(be_u32(mdhd.bytes(data), offset, "MP4 media header")?)
        .ok_or_else(|| malformed("MP4 media header", "a time scale of zero"))
}

/// The first sample description of the track.
struct SampleEntry {
    kind: [u8; 4],
    codec: VideoCodec,
    /// The avcC or hvcC body.
    record: Option<Range<usize>>,
    colour: Option<ColourDescription>,
}

impl SampleEntry {
    /// "Sample description atom": version and flags, a 32-bit entry count,
    /// then the entries, each a box whose type names the format.
    fn read(data: &[u8], stsd: &Mp4Box) -> Result<Self, VideoError> {
        let count = be_u32(stsd.bytes(data), FULL_BOX, "MP4 sample description")?;
        if count == 0 {
            return Err(malformed("MP4 sample description", "no sample entry"));
        }
        let entries = children(data, stsd.body + FULL_BOX + 4..stsd.end)?;
        let entry = entries
            .first()
            .ok_or_else(|| malformed("MP4 sample description", "no sample entry"))?;
        if entry.bytes(data).len() < VISUAL_SAMPLE_ENTRY {
            return Err(truncated("MP4 visual sample entry"));
        }
        let boxes = children(data, entry.body + VISUAL_SAMPLE_ENTRY..entry.end)?;
        let child = |kind: &[u8; 4]| {
            boxes
                .iter()
                .find(|candidate| &candidate.kind == kind)
                .copied()
        };
        let (codec, record) = match &entry.kind {
            b"avc1" | b"avc3" => (
                VideoCodec::H264,
                Some(child(b"avcC").ok_or_else(|| malformed("MP4 sample entry", "no avcC box"))?),
            ),
            b"hvc1" | b"hev1" => (
                VideoCodec::H265,
                Some(child(b"hvcC").ok_or_else(|| malformed("MP4 sample entry", "no hvcC box"))?),
            ),
            b"mp4v" => {
                let esds =
                    child(b"esds").ok_or_else(|| malformed("MP4 sample entry", "no esds box"))?;
                (mp4v_codec(data, &esds)?, None)
            }
            b"encv" => {
                return Err(VideoError::Unsupported {
                    feature: "an encrypted MP4 video track",
                });
            }
            _ => {
                return Err(VideoError::Unsupported {
                    feature: "an MP4 video sample entry other than avc1, avc3, hvc1, hev1 or mp4v",
                });
            }
        };
        let colour = child(b"colr")
            .map(|colr| colr_box(data, &colr))
            .transpose()?
            .flatten();
        Ok(Self {
            kind: entry.kind,
            codec,
            record: record.map(|found| found.range()),
            colour,
        })
    }

    /// The decoder configuration from the record, or for MPEG-2 from the
    /// first sample's sequence header.
    fn configuration(
        &self,
        data: &[u8],
        decoded: &[Decoded],
    ) -> Result<(StreamInfo, Option<Vec<u8>>, SampleFormat), VideoError> {
        let record = self
            .record
            .clone()
            .and_then(|range| data.get(range))
            .unwrap_or_default();
        match self.codec {
            VideoCodec::H264 => {
                // AVCDecoderConfigurationRecord, ISO/IEC 14496-15 5.3.3.1,
                // not available to read: configurationVersion,
                // AVCProfileIndication, profile_compatibility and
                // AVCLevelIndication bytes, lengthSizeMinusOne in the low two
                // bits of byte 4, numOfSequenceParameterSets in the low five
                // bits of byte 5, then each SPS behind a 16-bit length.
                let length_size = length_size(be_u8(record, 4, "avcC record")?)?;
                if be_u8(record, 5, "avcC record")? & 0x1F == 0 {
                    return Err(malformed("avcC record", "no sequence parameter set"));
                }
                let sps_length = usize::from(be_u16(record, 6, "avcC record")?);
                let sps = record
                    .get(8..8 + sps_length)
                    .ok_or_else(|| truncated("avcC record"))?;
                let (_, sps) = h264::parse_sps(sps)?;
                Ok((sps.stream_info("avc1"), Some(record.to_vec()), length_size))
            }
            VideoCodec::H265 => {
                // HEVCDecoderConfigurationRecord, ISO/IEC 14496-15 8.3.3.1,
                // not available to read: lengthSizeMinusOne in the low two
                // bits of byte 21 and numOfArrays at byte 22, then each array
                // as a byte with NAL_unit_type in its low six bits, a 16-bit
                // count and each unit behind a 16-bit length.
                let length_size = length_size(be_u8(record, 21, "hvcC record")?)?;
                let sps = hvcc_sps(record)?;
                let (_, sps) = h265::parse_sps(sps)?;
                Ok((sps.stream_info("hvc1"), Some(record.to_vec()), length_size))
            }
            VideoCodec::Mpeg2 => {
                // The sequence header travels in the samples, so the first
                // sample in decode order describes the stream.
                let first = decoded
                    .first()
                    .and_then(|sample| data.get(sample.range.clone()))
                    .ok_or(VideoError::Empty)?;
                let (_, info) = mpeg2::access_units(first)?;
                Ok((info, None, SampleFormat::StartCodes))
            }
        }
    }
}

fn length_size(byte: u8) -> Result<SampleFormat, VideoError> {
    match (byte & 0x03) + 1 {
        3 => Err(malformed(
            "decoder configuration record",
            "a NAL unit length field of three bytes",
        )),
        size => Ok(SampleFormat::LengthPrefixed { length_size: size }),
    }
}

/// The first SPS NAL unit, nal_unit_type 33, of the hvcC arrays.
fn hvcc_sps(record: &[u8]) -> Result<&[u8], VideoError> {
    let arrays = be_u8(record, 22, "hvcC record")?;
    let mut position = 23usize;
    for _ in 0..arrays {
        let kind = be_u8(record, position, "hvcC record")? & 0x3F;
        let count = be_u16(record, position + 1, "hvcC record")?;
        position += 3;
        for _ in 0..count {
            let length = usize::from(be_u16(record, position, "hvcC record")?);
            let unit = record
                .get(position + 2..position + 2 + length)
                .ok_or_else(|| truncated("hvcC record"))?;
            if kind == 33 {
                return Ok(unit);
            }
            position += 2 + length;
        }
    }
    Err(malformed("hvcC record", "no sequence parameter set"))
}

/// The codec of an mp4v entry from its esds box: version and flags, then an
/// ES_Descriptor, tag 3, holding a DecoderConfigDescriptor, tag 4, whose
/// first byte is objectTypeIndication. ISO/IEC 14496-1 7.2.6.5 and 7.2.6.6
/// were not available to read. The MP4 registration authority's object type
/// list gives 0x60 to 0x65 as ISO/IEC 13818-2 video.
fn mp4v_codec(data: &[u8], esds: &Mp4Box) -> Result<VideoCodec, VideoError> {
    let body = data.get(esds.body + FULL_BOX..esds.end).unwrap_or_default();
    let (tag, mut position) = descriptor(body, 0)?;
    if tag != 3 {
        return Err(malformed("esds box", "no ES_Descriptor"));
    }
    // ES_ID, then a flags byte: streamDependenceFlag, URL_Flag and
    // OCRstreamFlag in its top three bits.
    let flags = be_u8(body, position + 2, "esds box")?;
    position += 3;
    if flags & 0x80 != 0 {
        position += 2; // dependsOn_ES_ID
    }
    if flags & 0x40 != 0 {
        position += 1 + usize::from(be_u8(body, position, "esds box")?); // URLlength and URLstring
    }
    if flags & 0x20 != 0 {
        position += 2; // OCR_ES_Id
    }
    let (tag, position) = descriptor(body, position)?;
    if tag != 4 {
        return Err(malformed("esds box", "no DecoderConfigDescriptor"));
    }
    match be_u8(body, position, "esds box")? {
        0x60..=0x65 => Ok(VideoCodec::Mpeg2),
        _ => Err(VideoError::Unsupported {
            feature: "an mp4v track that is not ISO/IEC 13818-2 video",
        }),
    }
}

/// A descriptor tag and the position of its body, after a size of one to
/// four bytes, each carrying seven bits and a continuation bit.
fn descriptor(body: &[u8], position: usize) -> Result<(u8, usize), VideoError> {
    let tag = be_u8(body, position, "esds box")?;
    let mut at = position + 1;
    for _ in 0..4 {
        let byte = be_u8(body, at, "esds box")?;
        at += 1;
        if byte & 0x80 == 0 {
            return Ok((tag, at));
        }
    }
    Err(malformed(
        "esds box",
        "a descriptor size longer than four bytes",
    ))
}

/// The `colr` box. Type `nclc`, "Color parameter atom", holds three 16-bit
/// indexes. Type `nclx`, ISO/IEC 14496-12 12.1.5, not available to read, holds
/// the same three Rec. ITU-T H.273 code points and then a byte whose top bit
/// is full_range_flag. Any other type, an ICC profile, carries no code points.
fn colr_box(data: &[u8], colr: &Mp4Box) -> Result<Option<ColourDescription>, VideoError> {
    let kind = be::<4>(colr.bytes(data), 0, "colr box")?;
    let full_range = match &kind {
        b"nclx" => be_u8(colr.bytes(data), 10, "colr box")? & 0x80 != 0,
        b"nclc" => false,
        _ => return Ok(None),
    };
    Ok(Some(ColourDescription {
        code_point_source: ColourSource::ColrBox,
        range_source: ColourSource::ColrBox,
        colour_primaries: be_u16(colr.bytes(data), 4, "colr box")?,
        transfer_characteristics: be_u16(colr.bytes(data), 6, "colr box")?,
        matrix_coefficients: be_u16(colr.bytes(data), 8, "colr box")?,
        full_range,
    }))
}

/// Every sample of the track in decode order with its file range, its
/// composition time and its sync flag.
fn sample_table(data: &[u8], stbl: &Mp4Box) -> Result<Vec<Decoded>, VideoError> {
    let boxes = children(data, stbl.range())?;
    let child = |kind: &[u8; 4]| {
        boxes
            .iter()
            .find(|candidate| &candidate.kind == kind)
            .copied()
    };
    let stsz = match child(b"stsz") {
        Some(found) => found,
        None if child(b"stz2").is_some() => {
            return Err(VideoError::Unsupported {
                feature: "the compact sample size box stz2",
            });
        }
        None => return Err(malformed("MP4 sample table", "no sample size box")),
    };
    let sizes = SampleSizes::read(data, &stsz)?;
    let count = sizes.count;
    let mut decoded: Vec<Decoded> = Vec::with_capacity(count);

    // "Time-to-sample atom": runs of a 32-bit sample count and a 32-bit
    // delta. The decode time of a sample is the sum of the deltas before it.
    let stts =
        child(b"stts").ok_or_else(|| malformed("MP4 sample table", "no time-to-sample box"))?;
    let mut time = 0i64;
    for (run_count, delta) in runs(data, &stts, "MP4 time-to-sample box")? {
        for _ in 0..run_count {
            if decoded.len() == count {
                return Err(malformed(
                    "MP4 time-to-sample box",
                    "more samples than the sample size box",
                ));
            }
            decoded.push(Decoded {
                range: 0..0,
                key: (0, time),
                pts: Some(time),
                random_access: true,
            });
            time = time
                .checked_add(i64::from(delta))
                .ok_or_else(|| malformed("MP4 time-to-sample box", "a decode time over 2^63"))?;
        }
    }
    if decoded.len() != count {
        return Err(malformed(
            "MP4 time-to-sample box",
            "fewer samples than the sample size box",
        ));
    }

    // "Composition offset atom": runs of a sample count and an offset with
    // CT(n) = DT(n) + CTTS(n). That document allows a negative offset in any
    // version. Here version 1 is signed, the ISO/IEC 14496-12 rule, which was
    // not available to read. A version 0 offset with its top bit set is
    // refused, because the two documents read it differently.
    if let Some(ctts) = child(b"ctts") {
        let signed = be_u8(ctts.bytes(data), 0, "MP4 composition offset box")? == 1;
        let mut samples = decoded.iter_mut();
        for (run_count, raw) in runs(data, &ctts, "MP4 composition offset box")? {
            let offset = if signed {
                i64::from(i32::from_be_bytes(raw.to_be_bytes()))
            } else if raw > 0x7FFF_FFFF {
                return Err(malformed(
                    "MP4 composition offset box",
                    "a version 0 offset of 2^31 or more, which QuickTime reads as negative",
                ));
            } else {
                i64::from(raw)
            };
            for _ in 0..run_count {
                let sample = samples.next().ok_or_else(|| {
                    malformed("MP4 composition offset box", "more samples than the track")
                })?;
                let pts =
                    sample.key.1.checked_add(offset).ok_or_else(|| {
                        malformed("MP4 composition offset box", "a time over 2^63")
                    })?;
                sample.key = (0, pts);
                sample.pts = Some(pts);
            }
        }
        if samples.next().is_some() {
            return Err(malformed(
                "MP4 composition offset box",
                "fewer samples than the track",
            ));
        }
    }

    // "Sync sample atom": one-based sample numbers in increasing order.
    // Absent, every sample is a sync sample.
    if let Some(stss) = child(b"stss") {
        decoded
            .iter_mut()
            .for_each(|sample| sample.random_access = false);
        let entries = be_u32(stss.bytes(data), FULL_BOX, "MP4 sync sample box")?;
        let mut previous = 0u32;
        for entry in 0..to_usize(entries, "MP4 sync sample box")? {
            let number = be_u32(
                stss.bytes(data),
                FULL_BOX + 4 + entry * 4,
                "MP4 sync sample box",
            )?;
            if number <= previous {
                return Err(malformed(
                    "MP4 sync sample box",
                    "a sample number of 0 or out of order",
                ));
            }
            previous = number;
            let sample = decoded
                .get_mut(to_usize(number - 1, "MP4 sync sample box")?)
                .ok_or_else(|| {
                    malformed(
                        "MP4 sync sample box",
                        "a sample number past the last sample",
                    )
                })?;
            sample.random_access = true;
        }
    }

    // "Sample-to-chunk atom" with "Chunk offset atom": chunks of samples at
    // file offsets, each sample following the one before it in its chunk.
    let offsets = ChunkOffsets::read(data, &boxes)?;
    let stsc =
        child(b"stsc").ok_or_else(|| malformed("MP4 sample table", "no sample-to-chunk box"))?;
    let stsc_entries = to_usize(
        be_u32(stsc.bytes(data), FULL_BOX, "MP4 sample-to-chunk box")?,
        "MP4 sample-to-chunk box",
    )?;
    let stsc_body = stsc.bytes(data);
    let mut samples = decoded.iter_mut().enumerate();
    for entry in 0..stsc_entries {
        let at = FULL_BOX + 4 + entry * 12;
        let first_chunk = be_u32(stsc_body, at, "MP4 sample-to-chunk box")?;
        let per_chunk = be_u32(stsc_body, at + 4, "MP4 sample-to-chunk box")?;
        if be_u32(stsc_body, at + 8, "MP4 sample-to-chunk box")? != 1 {
            return Err(VideoError::Unsupported {
                feature: "an MP4 track with more than one sample description",
            });
        }
        let next_chunk = if entry + 1 < stsc_entries {
            be_u32(stsc_body, at + 12, "MP4 sample-to-chunk box")?
        } else {
            offsets
                .count
                .checked_add(1)
                .ok_or_else(|| malformed("MP4 chunk offset box", "too many chunks"))?
        };
        if first_chunk == 0 || next_chunk <= first_chunk {
            return Err(malformed(
                "MP4 sample-to-chunk box",
                "chunk numbers not one-based and increasing",
            ));
        }
        for chunk in first_chunk..next_chunk {
            let mut position = offsets.get(data, chunk - 1)?;
            for _ in 0..per_chunk {
                let (index, sample) = samples.next().ok_or_else(|| {
                    malformed("MP4 sample-to-chunk box", "more samples than the track")
                })?;
                let size = sizes.get(data, index)?;
                let end = position
                    .checked_add(size)
                    .filter(|end| *end <= data.len())
                    .ok_or_else(|| truncated("MP4 sample data"))?;
                sample.range = position..end;
                position = end;
            }
        }
    }
    if samples.next().is_some() {
        return Err(malformed(
            "MP4 sample-to-chunk box",
            "fewer samples than the track",
        ));
    }
    Ok(decoded)
}

/// The (count, value) pairs of a box laid out as version and flags, a
/// 32-bit entry count and pairs of 32-bit fields.
fn runs<'a>(
    data: &'a [u8],
    table: &Mp4Box,
    structure: &'static str,
) -> Result<impl Iterator<Item = (u32, u32)> + 'a, VideoError> {
    let body = table.bytes(data);
    let entries = to_usize(be_u32(body, FULL_BOX, structure)?, structure)?;
    let start = FULL_BOX + 4;
    let bytes = entries.checked_mul(8).ok_or_else(|| truncated(structure))?;
    let pairs = start
        .checked_add(bytes)
        .and_then(|end| body.get(start..end))
        .ok_or_else(|| truncated(structure))?;
    Ok(pairs.chunks_exact(8).map(|pair| {
        let mut count = [0u8; 4];
        let mut value = [0u8; 4];
        count.copy_from_slice(pair.get(..4).unwrap_or(&[0; 4]));
        value.copy_from_slice(pair.get(4..).unwrap_or(&[0; 4]));
        (u32::from_be_bytes(count), u32::from_be_bytes(value))
    }))
}

/// "Sample size atom": version and flags, a 32-bit constant size, a 32-bit
/// count, and when the constant is 0 one 32-bit size per sample.
struct SampleSizes {
    constant: usize,
    stsz: Mp4Box,
    count: usize,
}

/// The first per-sample size in a sample size box body.
const SAMPLE_SIZE_TABLE: usize = FULL_BOX + 8;

impl SampleSizes {
    fn read(data: &[u8], stsz: &Mp4Box) -> Result<Self, VideoError> {
        let constant = to_usize(
            be_u32(stsz.bytes(data), FULL_BOX, "MP4 sample size box")?,
            "MP4 sample size box",
        )?;
        let count = to_usize(
            be_u32(stsz.bytes(data), FULL_BOX + 4, "MP4 sample size box")?,
            "MP4 sample size box",
        )?;
        if count > MAX_SAMPLES {
            return Err(VideoError::TooManySamples);
        }
        // Each count is bounded by bytes that exist before anything is
        // allocated for it: a table of four bytes a sample, or samples of a
        // constant size that must fit the file.
        let backed = if constant == 0 {
            count
                .checked_mul(4)
                .and_then(|bytes| SAMPLE_SIZE_TABLE.checked_add(bytes))
                .is_some_and(|end| end <= stsz.bytes(data).len())
        } else {
            count
                .checked_mul(constant)
                .is_some_and(|bytes| bytes <= data.len())
        };
        if !backed {
            return Err(truncated("MP4 sample size box"));
        }
        Ok(Self {
            constant,
            stsz: *stsz,
            count,
        })
    }

    fn get(&self, data: &[u8], index: usize) -> Result<usize, VideoError> {
        if self.constant != 0 {
            return Ok(self.constant);
        }
        to_usize(
            be_u32(
                self.stsz.bytes(data),
                SAMPLE_SIZE_TABLE + index * 4,
                "MP4 sample size box",
            )?,
            "MP4 sample size box",
        )
    }
}

/// "Chunk offset atom", 32-bit offsets in stco or 64-bit ones in co64.
struct ChunkOffsets {
    table: Mp4Box,
    wide: bool,
    count: u32,
}

/// The first offset in a chunk offset box body.
const CHUNK_OFFSET_TABLE: usize = FULL_BOX + 4;

impl ChunkOffsets {
    fn read(data: &[u8], boxes: &[Mp4Box]) -> Result<Self, VideoError> {
        let (found, wide) = match boxes.iter().find(|candidate| &candidate.kind == b"stco") {
            Some(stco) => (*stco, false),
            None => (
                boxes
                    .iter()
                    .find(|candidate| &candidate.kind == b"co64")
                    .copied()
                    .ok_or_else(|| malformed("MP4 sample table", "no chunk offset box"))?,
                true,
            ),
        };
        let count = be_u32(found.bytes(data), FULL_BOX, "MP4 chunk offset box")?;
        let width = if wide { 8 } else { 4 };
        let end = to_usize(count, "MP4 chunk offset box")?
            .checked_mul(width)
            .and_then(|bytes| bytes.checked_add(CHUNK_OFFSET_TABLE));
        if end.is_none_or(|end| end > found.bytes(data).len()) {
            return Err(truncated("MP4 chunk offset box"));
        }
        Ok(Self {
            table: found,
            wide,
            count,
        })
    }

    /// The offset of the zero-based chunk `index`.
    fn get(&self, data: &[u8], index: u32) -> Result<usize, VideoError> {
        if index >= self.count {
            return Err(malformed(
                "MP4 sample-to-chunk box",
                "a chunk number past the last chunk",
            ));
        }
        let index = to_usize(index, "MP4 chunk offset box")?;
        if self.wide {
            usize::try_from(be_u64(
                self.table.bytes(data),
                CHUNK_OFFSET_TABLE + index * 8,
                "MP4 chunk offset box",
            )?)
            .map_err(|_| truncated("MP4 sample data"))
        } else {
            to_usize(
                be_u32(
                    self.table.bytes(data),
                    CHUNK_OFFSET_TABLE + index * 4,
                    "MP4 chunk offset box",
                )?,
                "MP4 chunk offset box",
            )
        }
    }
}
