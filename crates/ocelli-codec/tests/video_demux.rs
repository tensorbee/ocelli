//! Sample tables of the video demuxer, AC3, AC4 and the demux half of AC5 of
//! the F-241 plan, with the codec configuration and the refusals by name.
//!
//! No expected value here comes from running `demux`. Each one comes from one
//! of two places. The first is an independent reader written in this file
//! from the published container text: the MP4 sample table boxes as Apple's
//! QuickTime File Format documents them, and the transport stream packet, PSI
//! and PES syntax of Rec. ITU-T H.222.0 (08/2023). The second is the
//! construction of the clip in `fixtures/video/generate_video.py`, which names
//! the ffmpeg arguments that fixed the value, a group of 12 or a BT.709 matrix.
//!
//! The readers are deliberately simpler than the demuxer. They materialise one
//! row per sample from the boxes, and they compare elementary streams by the
//! bytes of their coded pictures, so a sample from MP4 and the same sample from
//! a transport stream are matched by content rather than by position.

use std::{error::Error, num::NonZeroU32, path::Path};

use ocelli_codec::video::{
    ColourDescription, ColourSource, FrameSize, Sample, SampleFormat, SampleTable, VideoCodec,
    VideoContainer, VideoError, demux,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn fixture(name: &str) -> TestResult<Vec<u8>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/video")
        .join(name);
    Ok(std::fs::read(path)?)
}

fn bytes_at(bytes: &[u8], offset: usize, length: usize) -> TestResult<&[u8]> {
    let end = offset.checked_add(length).ok_or("offset overflow")?;
    bytes
        .get(offset..end)
        .ok_or_else(|| format!("{length} bytes at {offset} are out of range").into())
}

fn be16(bytes: &[u8], offset: usize) -> TestResult<u16> {
    let field = bytes_at(bytes, offset, 2)?;
    Ok(u16::from_be_bytes(field.try_into()?))
}

fn be32(bytes: &[u8], offset: usize) -> TestResult<u32> {
    let field = bytes_at(bytes, offset, 4)?;
    Ok(u32::from_be_bytes(field.try_into()?))
}

fn be64(bytes: &[u8], offset: usize) -> TestResult<u64> {
    let field = bytes_at(bytes, offset, 8)?;
    Ok(u64::from_be_bytes(field.try_into()?))
}

fn byte(bytes: &[u8], offset: usize) -> TestResult<u8> {
    bytes
        .get(offset)
        .copied()
        .ok_or_else(|| format!("byte {offset} is out of range").into())
}

fn index(value: u32) -> TestResult<usize> {
    Ok(usize::try_from(value)?)
}

// ---------------------------------------------------------------------------
// Independent MP4 reader. Box layout from Apple's QuickTime File Format,
// "Atoms": a 32-bit size counting the header, a four character type, a size
// of 1 meaning a 64-bit size follows the type, and a size of 0 meaning the box
// runs to the end of its parent. Sample table layouts from the same document's
// pages for stts, ctts, stss, stsc, stsz, stco and co64.
// ---------------------------------------------------------------------------

/// One box, its type and the byte range of its body.
#[derive(Clone, Copy, Debug)]
struct Mp4Box {
    kind: [u8; 4],
    body: usize,
    end: usize,
}

fn boxes(bytes: &[u8], start: usize, end: usize) -> TestResult<Vec<Mp4Box>> {
    let mut found = Vec::new();
    let mut position = start;
    while position < end {
        let size32 = be32(bytes, position)?;
        let kind: [u8; 4] = bytes_at(bytes, position + 4, 4)?.try_into()?;
        let (header, size) = match size32 {
            0 => (8, end - position),
            1 => (16, usize::try_from(be64(bytes, position + 8)?)?),
            other => (8, index(other)?),
        };
        let box_end = position.checked_add(size).ok_or("box size overflow")?;
        if size < header || box_end > end {
            return Err(format!("box {kind:?} at {position} overruns its parent").into());
        }
        found.push(Mp4Box {
            kind,
            body: position + header,
            end: box_end,
        });
        position = box_end;
    }
    Ok(found)
}

fn child(bytes: &[u8], parent: Mp4Box, kind: &[u8; 4]) -> TestResult<Option<Mp4Box>> {
    Ok(boxes(bytes, parent.body, parent.end)?
        .into_iter()
        .find(|candidate| &candidate.kind == kind))
}

fn require(bytes: &[u8], parent: Mp4Box, kind: &[u8; 4]) -> TestResult<Mp4Box> {
    child(bytes, parent, kind)?.ok_or_else(|| format!("no {kind:?} box").into())
}

/// One sample of the reference table, in decode order.
#[derive(Clone, Copy, Debug)]
struct ReferenceSample {
    offset: usize,
    size: usize,
    composition_time: i64,
    sync: bool,
}

struct ReferenceTrack {
    timescale: u32,
    entry: [u8; 4],
    /// The body of the sample entry's configuration box, avcC or hvcC.
    configuration: Option<Vec<u8>>,
    samples: Vec<ReferenceSample>,
}

/// The first track whose handler is `vide`, read box by box.
fn reference_mp4(bytes: &[u8]) -> TestResult<ReferenceTrack> {
    let top = boxes(bytes, 0, bytes.len())?;
    let moov = top
        .into_iter()
        .find(|candidate| &candidate.kind == b"moov")
        .ok_or("no moov box")?;
    for trak in boxes(bytes, moov.body, moov.end)? {
        if &trak.kind != b"trak" {
            continue;
        }
        let mdia = require(bytes, trak, b"mdia")?;
        let hdlr = require(bytes, mdia, b"hdlr")?;
        // Version and flags, then a 32-bit pre_defined or component type,
        // then the 32-bit handler type.
        if bytes_at(bytes, hdlr.body + 8, 4)? != b"vide" {
            continue;
        }
        let mdhd = require(bytes, mdia, b"mdhd")?;
        // Version 0 has two 32-bit times before the time scale, version 1 two
        // 64-bit times.
        let timescale = match byte(bytes, mdhd.body)? {
            0 => be32(bytes, mdhd.body + 12)?,
            _ => be32(bytes, mdhd.body + 20)?,
        };
        let stbl = require(bytes, require(bytes, mdia, b"minf")?, b"stbl")?;
        let (entry, configuration) = sample_entry(bytes, require(bytes, stbl, b"stsd")?)?;
        let samples = reference_samples(bytes, stbl)?;
        return Ok(ReferenceTrack {
            timescale,
            entry,
            configuration,
            samples,
        });
    }
    Err("no video track".into())
}

fn sample_entry(bytes: &[u8], stsd: Mp4Box) -> TestResult<([u8; 4], Option<Vec<u8>>)> {
    // Version, flags and a 32-bit entry count precede the entries.
    let entries = boxes(bytes, stsd.body + 8, stsd.end)?;
    let entry = entries.first().ok_or("empty stsd")?;
    // A visual sample entry has 78 bytes of fixed fields: six reserved bytes
    // and a data reference index, then 70 bytes of image description.
    let children = boxes(bytes, entry.body + 78, entry.end)?;
    let configuration = children
        .iter()
        .find(|candidate| &candidate.kind == b"avcC" || &candidate.kind == b"hvcC")
        .map(|found| bytes_at(bytes, found.body, found.end - found.body).map(<[u8]>::to_vec))
        .transpose()?;
    Ok((entry.kind, configuration))
}

fn entry_count(bytes: &[u8], table: Mp4Box) -> TestResult<usize> {
    index(be32(bytes, table.body + 4)?)
}

fn reference_samples(bytes: &[u8], stbl: Mp4Box) -> TestResult<Vec<ReferenceSample>> {
    // stts: runs of (sample count, sample delta), decode times summed.
    let stts = require(bytes, stbl, b"stts")?;
    let mut decode_times = Vec::new();
    let mut time = 0i64;
    for run in 0..entry_count(bytes, stts)? {
        let at = stts.body + 8 + run * 8;
        for _ in 0..be32(bytes, at)? {
            decode_times.push(time);
            time += i64::from(be32(bytes, at + 4)?);
        }
    }
    // ctts: runs of (sample count, offset), unsigned in version 0 and signed
    // in version 1. Absent means presentation follows decode order.
    let mut offsets = vec![0i64; decode_times.len()];
    if let Some(ctts) = child(bytes, stbl, b"ctts")? {
        let signed = byte(bytes, ctts.body)? == 1;
        let mut sample = 0usize;
        for run in 0..entry_count(bytes, ctts)? {
            let at = ctts.body + 8 + run * 8;
            let raw = be32(bytes, at + 4)?;
            let offset = if signed {
                i64::from(i32::from_be_bytes(raw.to_be_bytes()))
            } else {
                i64::from(raw)
            };
            for _ in 0..be32(bytes, at)? {
                *offsets
                    .get_mut(sample)
                    .ok_or("ctts covers too many samples")? = offset;
                sample += 1;
            }
        }
    }
    // stss: one-based sample numbers. Absent means every sample is a sync
    // sample.
    let mut sync = vec![true; decode_times.len()];
    if let Some(stss) = child(bytes, stbl, b"stss")? {
        sync.fill(false);
        for entry in 0..entry_count(bytes, stss)? {
            let number = index(be32(bytes, stss.body + 8 + entry * 4)?)?;
            let zero_based = number.checked_sub(1).ok_or("stss number 0")?;
            *sync.get_mut(zero_based).ok_or("stss number out of range")? = true;
        }
    }
    // stsz: a constant size, or zero followed by one size per sample.
    let stsz = require(bytes, stbl, b"stsz")?;
    let constant = index(be32(bytes, stsz.body + 4)?)?;
    let count = index(be32(bytes, stsz.body + 8)?)?;
    let mut sizes = Vec::with_capacity(count);
    for sample in 0..count {
        sizes.push(if constant == 0 {
            index(be32(bytes, stsz.body + 12 + sample * 4)?)?
        } else {
            constant
        });
    }
    // stco or co64: one file offset per chunk.
    let mut chunks = Vec::new();
    if let Some(stco) = child(bytes, stbl, b"stco")? {
        for chunk in 0..entry_count(bytes, stco)? {
            chunks.push(index(be32(bytes, stco.body + 8 + chunk * 4)?)?);
        }
    } else {
        let co64 = require(bytes, stbl, b"co64")?;
        for chunk in 0..entry_count(bytes, co64)? {
            chunks.push(usize::try_from(be64(bytes, co64.body + 8 + chunk * 8)?)?);
        }
    }
    // stsc: runs of (first chunk, samples per chunk, description index),
    // with one-based chunk numbers. Every chunk is expanded on its own.
    let stsc = require(bytes, stbl, b"stsc")?;
    let runs = entry_count(bytes, stsc)?;
    let mut per_chunk = vec![0usize; chunks.len()];
    for run in 0..runs {
        let at = stsc.body + 8 + run * 12;
        let first = index(be32(bytes, at)?)?;
        let next = if run + 1 < runs {
            index(be32(bytes, at + 12)?)?
        } else {
            chunks.len() + 1
        };
        for chunk in first..next {
            *per_chunk
                .get_mut(chunk.checked_sub(1).ok_or("chunk 0")?)
                .ok_or("stsc chunk out of range")? = index(be32(bytes, at + 4)?)?;
        }
    }
    let mut samples = Vec::with_capacity(count);
    for (chunk, start) in chunks.iter().enumerate() {
        let mut offset = *start;
        for _ in 0..*per_chunk.get(chunk).ok_or("chunk out of range")? {
            let decode_index = samples.len();
            let size = *sizes
                .get(decode_index)
                .ok_or("stsc covers too many samples")?;
            samples.push(ReferenceSample {
                offset,
                size,
                composition_time: decode_times.get(decode_index).ok_or("stts too short")?
                    + offsets.get(decode_index).ok_or("ctts too short")?,
                sync: *sync.get(decode_index).ok_or("stss too short")?,
            });
            offset += size;
        }
    }
    if samples.len() != count || decode_times.len() != count {
        return Err("stts, stsz and stsc disagree on the sample count".into());
    }
    Ok(samples)
}

/// The reference samples in presentation order, each with its decode index.
fn presentation(track: &ReferenceTrack) -> Vec<(usize, ReferenceSample)> {
    let mut ordered: Vec<(usize, ReferenceSample)> =
        track.samples.iter().copied().enumerate().collect();
    ordered.sort_by_key(|(_, sample)| sample.composition_time);
    ordered
}

// ---------------------------------------------------------------------------
// Coded picture identity. A sample's identity is the list of its coded slice
// NAL units, without start codes or length fields and without the access unit
// delimiters and parameter sets a muxer may add. Rec. ITU-T H.264 (08/2024)
// Table 7-1 gives nal_unit_type 1 to 5 as coded slices in the low five bits of
// the first byte, and Rec. ITU-T H.265 (07/2024) Table 7-1 gives 0 to 31 as
// VCL types in bits 1 to 6. For MPEG-2 the identity is the bytes from the
// first slice start code of the picture, Rec. ITU-T H.262 (02/2000) Table 6-1.
// ---------------------------------------------------------------------------

fn is_vcl(codec: VideoCodec, header: u8) -> bool {
    match codec {
        VideoCodec::H264 => (1..=5).contains(&(header & 0x1F)),
        VideoCodec::H265 => (header >> 1) & 0x3F <= 31,
        VideoCodec::Mpeg2 => false,
    }
}

fn length_prefixed_slices(
    data: &[u8],
    length_size: usize,
    codec: VideoCodec,
) -> TestResult<Vec<Vec<u8>>> {
    let mut slices = Vec::new();
    let mut position = 0usize;
    while position < data.len() {
        let mut length = 0usize;
        for byte_index in 0..length_size {
            length = length << 8 | usize::from(byte(data, position + byte_index)?);
        }
        let nal = bytes_at(data, position + length_size, length)?;
        if is_vcl(codec, byte(nal, 0)?) {
            slices.push(nal.to_vec());
        }
        position += length_size + length;
    }
    Ok(slices)
}

/// NAL units of a start code stream. A unit runs to the next 0x000001 and
/// loses the zero bytes before it, which belong to the next start code.
fn start_code_units(data: &[u8]) -> Vec<&[u8]> {
    let mut starts = Vec::new();
    let mut position = 0usize;
    while let Some(found) = data
        .get(position..)
        .and_then(|rest| rest.windows(3).position(|window| window == [0, 0, 1]))
    {
        starts.push(position + found + 3);
        position += found + 3;
    }
    let mut units = Vec::new();
    for (number, start) in starts.iter().enumerate() {
        let mut end = starts.get(number + 1).map_or(data.len(), |next| next - 3);
        while end > *start && data.get(end - 1) == Some(&0) {
            end -= 1;
        }
        if let Some(unit) = data.get(*start..end) {
            units.push(unit);
        }
    }
    units
}

fn start_code_slices(data: &[u8], codec: VideoCodec) -> Vec<Vec<u8>> {
    start_code_units(data)
        .into_iter()
        .filter(|unit| unit.first().is_some_and(|header| is_vcl(codec, *header)))
        .map(<[u8]>::to_vec)
        .collect()
}

fn mpeg2_picture(data: &[u8]) -> TestResult<Vec<u8>> {
    // The first slice_start_code, 01 to AF in Table 6-1, after the picture
    // header. Starting at the slices leaves temporal_reference out, which
    // `mpeg2_tr_wrap.m2v` renumbers.
    let picture = data
        .windows(4)
        .position(|window| window == [0, 0, 1, 0])
        .ok_or("no picture start code")?;
    let start = picture
        + data
            .get(picture..)
            .ok_or("picture out of range")?
            .windows(4)
            .position(|window| matches!(window, [0, 0, 1, 0x01..=0xAF]))
            .ok_or("no slice start code")?;
    let mut end = data.len();
    while end > start && data.get(end - 1) == Some(&0) {
        end -= 1;
    }
    Ok(bytes_at(data, start, end - start)?.to_vec())
}

fn identity(data: &[u8], format: SampleFormat, codec: VideoCodec) -> TestResult<Vec<Vec<u8>>> {
    if codec == VideoCodec::Mpeg2 {
        return Ok(vec![mpeg2_picture(data)?]);
    }
    match format {
        SampleFormat::LengthPrefixed { length_size } => {
            length_prefixed_slices(data, usize::from(length_size), codec)
        }
        SampleFormat::StartCodes => Ok(start_code_slices(data, codec)),
    }
}

fn sample_bytes<'a>(
    table: &'a SampleTable,
    input: &'a [u8],
    sample: &Sample,
) -> TestResult<&'a [u8]> {
    table
        .sample_data(input, sample)
        .ok_or_else(|| "sample range outside its buffer".into())
}

/// The length field size of an avcC or hvcC record. Both keep
/// lengthSizeMinusOne in the low two bits of a fixed byte, 4 in an
/// AVCDecoderConfigurationRecord and 21 in an HEVCDecoderConfigurationRecord,
/// ISO/IEC 14496-15 5.3.3.1 and 8.3.3.1. Those clauses were not available to
/// read, so the offsets are a shared transcription a reviewer checks.
fn reference_length_size(track: &ReferenceTrack) -> TestResult<SampleFormat> {
    let Some(record) = &track.configuration else {
        return Ok(SampleFormat::StartCodes);
    };
    let at = if &track.entry == b"hvc1" || &track.entry == b"hev1" {
        21
    } else {
        4
    };
    Ok(SampleFormat::LengthPrefixed {
        length_size: (byte(record, at)? & 0x03) + 1,
    })
}

// ---------------------------------------------------------------------------
// Comparisons
// ---------------------------------------------------------------------------

/// AC3's rule: every sample of the table is the reference sample at the same
/// presentation position, with its byte range, its composition time, its
/// decode index and its sync flag read from the boxes.
fn assert_matches_boxes(
    input: &[u8],
    codec: VideoCodec,
) -> TestResult<(SampleTable, ReferenceTrack)> {
    let track = reference_mp4(input)?;
    let table = demux(input, codec)?;
    assert_eq!(table.container(), VideoContainer::Mp4);
    assert_eq!(table.timescale(), NonZeroU32::new(track.timescale));
    let expected = presentation(&track);
    assert_eq!(table.samples().len(), expected.len());
    for (position, (sample, (decode_index, reference))) in
        table.samples().iter().zip(&expected).enumerate()
    {
        assert_eq!(
            sample.range,
            reference.offset..reference.offset + reference.size,
            "byte range of presentation sample {position}"
        );
        assert_eq!(sample.pts, reference.composition_time, "pts of {position}");
        assert_eq!(
            sample.decode_index, *decode_index,
            "decode index of {position}"
        );
        assert_eq!(
            sample.random_access, reference.sync,
            "sync flag of {position}"
        );
    }
    Ok((table, track))
}

/// AC4's rule: a table from another container holds the same coded pictures
/// in the same presentation order, with the same decode order and random
/// access points, and presentation times that differ from the MP4 ones only by
/// a constant and the time scale. A bare elementary stream has no times, so
/// its pts counts frames.
fn assert_same_table(
    reference_input: &[u8],
    other_input: &[u8],
    codec: VideoCodec,
    container: VideoContainer,
) -> TestResult<SampleTable> {
    let track = reference_mp4(reference_input)?;
    let reference_format = reference_length_size(&track)?;
    let expected = presentation(&track);
    let table = demux(other_input, codec)?;
    assert_eq!(table.container(), container);
    assert_eq!(table.samples().len(), expected.len());
    let first_reference = expected
        .first()
        .ok_or("empty reference")?
        .1
        .composition_time;
    let first = table.samples().first().ok_or("empty table")?.pts;
    for (position, (sample, (decode_index, reference))) in
        table.samples().iter().zip(&expected).enumerate()
    {
        let reference_data = bytes_at(reference_input, reference.offset, reference.size)?;
        assert_eq!(
            identity(
                sample_bytes(&table, other_input, sample)?,
                table.codec_config().sample_format,
                codec
            )?,
            identity(reference_data, reference_format, codec)?,
            "coded picture at presentation position {position}"
        );
        assert_eq!(
            sample.decode_index, *decode_index,
            "decode index of {position}"
        );
        assert_eq!(
            sample.random_access, reference.sync,
            "random access of {position}"
        );
        match table.timescale() {
            Some(timescale) => {
                // Cross-multiplied, so no division rounds.
                let here = i128::from(sample.pts - first) * i128::from(track.timescale);
                let there = i128::from(reference.composition_time - first_reference)
                    * i128::from(timescale.get());
                assert_eq!(here, there, "presentation time of {position}");
            }
            None => assert_eq!(
                sample.pts,
                i64::try_from(position)?,
                "frame count of {position}"
            ),
        }
    }
    assert_access_unit_structure(&table, other_input, codec)?;
    Ok(table)
}

/// A start code stream's samples, taken in decode order, cover its bytes
/// with no gap and no overlap, and each is one access unit as Rec. ITU-T H.264
/// (08/2024) 7.4.1.2.3 and Rec. ITU-T H.265 (07/2024) 7.4.2.4.4 order one: no
/// access unit delimiter, parameter set or prefix SEI follows the last coded
/// slice of the picture, because each of those starts the next access unit.
fn assert_access_unit_structure(
    table: &SampleTable,
    input: &[u8],
    codec: VideoCodec,
) -> TestResult {
    if table.codec_config().sample_format != SampleFormat::StartCodes {
        return Ok(());
    }
    let mut by_decode: Vec<&Sample> = table.samples().iter().collect();
    by_decode.sort_by_key(|sample| sample.decode_index);
    for pair in by_decode.windows(2) {
        if let [earlier, later] = pair {
            assert_eq!(
                earlier.range.end, later.range.start,
                "samples abut in decode order"
            );
        }
    }
    if codec == VideoCodec::Mpeg2 {
        return Ok(());
    }
    for sample in by_decode {
        let data = sample_bytes(table, input, sample)?;
        let mut seen_slice = false;
        for unit in start_code_units(data) {
            let header = byte(unit, 0)?;
            let opens_unit = match codec {
                // Table 7-1: 6 SEI, 7 SPS, 8 PPS, 9 access unit delimiter and
                // 14 to 18.
                VideoCodec::H264 => matches!(header & 0x1F, 6..=9 | 14..=18),
                // Table 7-1: 32 to 35 VPS, SPS, PPS and access unit delimiter,
                // 39 prefix SEI, 41 to 44 and 48 to 55, for nuh_layer_id 0.
                _ => {
                    let kind = (header >> 1) & 0x3F;
                    let layer_zero = header & 0x01 == 0 && byte(unit, 1)? >> 3 == 0;
                    layer_zero && matches!(kind, 32..=35 | 39 | 41..=44 | 48..=55)
                }
            };
            assert!(
                !(seen_slice && opens_unit),
                "a unit that opens an access unit follows a slice"
            );
            seen_slice |= is_vcl(codec, header);
        }
        assert!(seen_slice, "every access unit holds a coded picture");
    }
    Ok(())
}

fn presentation_raps(table: &SampleTable) -> Vec<usize> {
    table
        .samples()
        .iter()
        .enumerate()
        .filter(|(_, sample)| sample.random_access)
        .map(|(position, _)| position)
        .collect()
}

// ---------------------------------------------------------------------------
// AC3, MP4
// ---------------------------------------------------------------------------

/// AC3. 60 frames from `-frames:v 60`, a group of 12 from `keyint=12` with
/// `min-keyint=12` and `scenecut=0`, so the IDR pictures are frames 0, 12, 24,
/// 36 and 48 in both orders, and two B-frames from `bframes=2`, so the
/// presentation order is not the decode order.
#[test]
fn ac3_mp4_h264_table_follows_its_boxes() -> TestResult {
    let input = fixture("h264_gop12.mp4")?;
    let (table, track) = assert_matches_boxes(&input, VideoCodec::H264)?;
    assert_eq!(table.samples().len(), 60);
    assert_eq!(presentation_raps(&table), [0, 12, 24, 36, 48]);
    let decode_raps: Vec<usize> = track
        .samples
        .iter()
        .enumerate()
        .filter(|(_, sample)| sample.sync)
        .map(|(decode_index, _)| decode_index)
        .collect();
    assert_eq!(decode_raps, [0, 12, 24, 36, 48]);
    let reordered = table
        .samples()
        .iter()
        .enumerate()
        .filter(|(position, sample)| sample.decode_index != *position)
        .count();
    assert!(reordered > 0, "B-frames reorder some samples");
    Ok(())
}

/// The audio track of `h264_audio.mp4` interleaves its chunks with the video
/// chunks, so the chunk walk of stsc and stco is exercised across tracks.
#[test]
fn ac3_mp4_with_an_audio_track_keeps_the_video_table() -> TestResult {
    let input = fixture("h264_audio.mp4")?;
    let (table, _) = assert_matches_boxes(&input, VideoCodec::H264)?;
    assert_eq!(table.samples().len(), 24);
    assert!(table.codec_config().has_audio);
    Ok(())
}

#[test]
fn ac3_mp4_hevc_table_follows_its_boxes() -> TestResult {
    let input = fixture("hevc_main.mp4")?;
    let (table, _) = assert_matches_boxes(&input, VideoCodec::H265)?;
    // 16 frames with keyint=8, min-keyint=8 and a closed group.
    assert_eq!(table.samples().len(), 16);
    assert_eq!(presentation_raps(&table), [0, 8]);
    Ok(())
}

// ---------------------------------------------------------------------------
// AC4, MPEG-TS and Annex B
// ---------------------------------------------------------------------------

/// AC4. `h264_gop12.m2t` is `h264_gop12.mp4` copied into a transport stream.
#[test]
fn ac4_mpeg_ts_gives_the_mp4_table() -> TestResult {
    let reference = fixture("h264_gop12.mp4")?;
    let input = fixture("h264_gop12.m2t")?;
    let table = assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::MpegTs)?;
    // Rec. ITU-T H.222.0 (08/2023) 2.4.3.7, PTS counts a 90 kHz clock.
    assert_eq!(table.timescale(), NonZeroU32::new(90_000));
    assert_eq!(table.samples().len(), 60);
    Ok(())
}

/// AC4. The same stream copied with an offset that puts the 33-bit PTS wrap
/// thirteen frames in. A reader that does not unwrap the clock orders the
/// frames after the wrap before the ones ahead of it.
#[test]
fn ac4_mpeg_ts_across_a_pts_wrap_gives_the_mp4_table() -> TestResult {
    let reference = fixture("h264_gop12.mp4")?;
    let input = fixture("h264_gop12_wrap.m2t")?;
    let pts = reference_ts_pts(&input)?;
    let wraps = pts
        .windows(2)
        .filter(|pair| matches!(pair, [a, b] if b < a && a - b > 1 << 32))
        .count();
    assert_eq!(wraps, 1, "the clip wraps the PTS exactly once");
    let table = assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::MpegTs)?;
    assert_eq!(table.timescale(), NonZeroU32::new(90_000));
    Ok(())
}

/// AC4. `h264_gop12.h264` is the same stream written as an Annex B byte
/// stream, which carries no presentation times.
#[test]
fn ac4_annex_b_gives_the_mp4_table() -> TestResult {
    let reference = fixture("h264_gop12.mp4")?;
    let input = fixture("h264_gop12.h264")?;
    let table = assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::AnnexB)?;
    assert_eq!(table.timescale(), None);
    assert_eq!(table.codec_config().sample_format, SampleFormat::StartCodes);
    assert_eq!(table.codec_config().description, None);
    Ok(())
}

#[test]
fn ac4_hevc_mpeg_ts_and_annex_b_give_the_mp4_table() -> TestResult {
    let reference = fixture("hevc_main.mp4")?;
    let ts = fixture("hevc_main.m2t")?;
    assert_same_table(&reference, &ts, VideoCodec::H265, VideoContainer::MpegTs)?;
    let annex_b = fixture("hevc_main.h265")?;
    assert_same_table(
        &reference,
        &annex_b,
        VideoCodec::H265,
        VideoContainer::AnnexB,
    )?;
    Ok(())
}

#[test]
fn ac4_mpeg_ts_with_an_audio_stream_names_it() -> TestResult {
    let reference = fixture("h264_audio.mp4")?;
    let input = fixture("h264_audio.m2t")?;
    let table = assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::MpegTs)?;
    assert!(table.codec_config().has_audio);
    Ok(())
}

/// AC4 with pictures of three slices and one IDR in 100 frames, so an access
/// unit spans several coded slices and x264's 5-bit pic_order_cnt_lsb, two a
/// frame, wraps six times, Rec. ITU-T H.264 (08/2024) 7.4.1.2.4 and 8.2.1.1.
/// A splitter that starts a picture at every slice, or that loses
/// PicOrderCntMsb, gives a different table.
#[test]
fn ac4_multi_slice_pictures_and_a_wrapping_poc() -> TestResult {
    let reference = fixture("h264_slices.mp4")?;
    let annex_b = fixture("h264_slices.h264")?;
    let table = assert_same_table(
        &reference,
        &annex_b,
        VideoCodec::H264,
        VideoContainer::AnnexB,
    )?;
    assert_eq!(table.samples().len(), 100);
    assert_eq!(presentation_raps(&table), [0]);
    let ts = fixture("h264_slices.m2t")?;
    assert_same_table(&reference, &ts, VideoCodec::H264, VideoContainer::MpegTs)?;
    Ok(())
}

/// AC4 with a B-frame pyramid, review pass 4 S1. The middle B-frame of each
/// three is a reference picture, nal_ref_idc above 0, so its slices carry
/// dec_ref_pic_marking after the list 1 half of ref_pic_list_modification,
/// Rec. ITU-T H.264 (08/2024) 7.3.3, and x264 writes adaptive marking there.
/// Every route gives the MP4 table.
#[test]
fn ac4_b_frame_pyramid_gives_the_mp4_table() -> TestResult {
    let reference = fixture("h264_bpyramid.mp4")?;
    let annex_b = fixture("h264_bpyramid.h264")?;
    let table = assert_same_table(
        &reference,
        &annex_b,
        VideoCodec::H264,
        VideoContainer::AnnexB,
    )?;
    assert_eq!(table.samples().len(), 48);
    assert_eq!(presentation_raps(&table), [0, 24]);
    let ts = fixture("h264_bpyramid.m2t")?;
    assert_same_table(&reference, &ts, VideoCodec::H264, VideoContainer::MpegTs)?;
    let (mp4, _) = assert_matches_boxes(&reference, VideoCodec::H264)?;
    assert_eq!(mp4.samples().len(), 48);
    Ok(())
}

/// AC4 for pic_order_cnt_type 2, which x264 writes with no B-frames, with a
/// 4-bit frame_num that wraps in 40 frames, 8.2.1.3. Every picture of this
/// clip is a reference picture, so it exercises 8-11 and the reference arm of
/// 8-12. The non-reference arm is `poc_type_2_non_reference_arm` in h264.rs.
#[test]
fn ac4_poc_type_2_follows_frame_num() -> TestResult {
    let reference = fixture("h264_poc2.mp4")?;
    let input = fixture("h264_poc2.h264")?;
    let table = assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::AnnexB)?;
    assert_eq!(table.samples().len(), 40);
    Ok(())
}

/// AC4 for HEVC with one IRAP picture in 80 frames and a 6-bit
/// slice_pic_order_cnt_lsb of one a frame, so PicOrderCntMsb has to carry the
/// count across the wrap at frame 64, Rec. ITU-T H.265 (07/2024) 8.3.1.
#[test]
fn ac4_hevc_wrapping_poc() -> TestResult {
    let reference = fixture("hevc_poc.mp4")?;
    let input = fixture("hevc_poc.h265")?;
    let table = assert_same_table(&reference, &input, VideoCodec::H265, VideoContainer::AnnexB)?;
    assert_eq!(table.samples().len(), 80);
    assert_eq!(presentation_raps(&table), [0]);
    Ok(())
}

/// AC4 with a trailing_zero_8bits byte, Rec. ITU-T H.264 (08/2024) B.1.1, at
/// the end of every PES packet. The zero is not part of the next access
/// unit's start code prefix, so each access unit still commences in its own
/// PES packet and keeps its PTS, Rec. ITU-T H.222.0 (08/2023) 2.4.3.7.
#[test]
fn ac4_mpeg_ts_with_trailing_zero_bytes_gives_the_mp4_table() -> TestResult {
    let reference = fixture("h264_gop12.mp4")?;
    let input = fixture("h264_gop12_trailing_zero.m2t")?;
    assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::MpegTs)?;
    let reference = fixture("hevc_main.mp4")?;
    let input = fixture("hevc_main_trailing_zero.m2t")?;
    assert_same_table(&reference, &input, VideoCodec::H265, VideoContainer::MpegTs)?;
    Ok(())
}

/// The 188-byte packets of a transport stream with their PID, Rec. ITU-T
/// H.222.0 (08/2023) Table 2-2.
fn ts_packets(input: &[u8]) -> TestResult<Vec<(u16, &[u8])>> {
    input
        .chunks(188)
        .map(|packet| Ok((be16(packet, 1)? & 0x1FFF, packet)))
        .collect()
}

/// The PID whose first PES packet opens with a video stream_id, Table 2-22.
fn video_pid(input: &[u8]) -> TestResult<u16> {
    for (pid, packet) in ts_packets(input)? {
        if byte(packet, 1)? & 0x40 != 0 && bytes_at(packet, 4, 4)? == [0, 0, 1, 0xE0] {
            return Ok(pid);
        }
    }
    Err("no video PID".into())
}

/// 2.4.3.3: a duplicate packet repeats the one before it with the same
/// continuity_counter, and a decoder keeps one copy, so the table is the
/// same. A missing packet breaks the count, so the stream is refused.
#[test]
fn duplicate_and_missing_transport_packets() -> TestResult {
    let input = fixture("h264_gop12.m2t")?;
    let pid = video_pid(&input)?;
    let packets = ts_packets(&input)?;
    // A video packet that continues a PES packet, so it starts no unit.
    let target = packets
        .iter()
        .position(|(packet_pid, packet)| {
            *packet_pid == pid && byte(packet, 1).is_ok_and(|flags| flags & 0x40 == 0)
        })
        .ok_or("no continuation packet")?;
    let mut duplicated = Vec::new();
    let mut dropped = Vec::new();
    for (number, (_, packet)) in packets.iter().enumerate() {
        duplicated.extend_from_slice(packet);
        if number == target {
            duplicated.extend_from_slice(packet);
        } else {
            dropped.extend_from_slice(packet);
        }
    }
    let original = demux(&input, VideoCodec::H264)?;
    let with_duplicate = demux(&duplicated, VideoCodec::H264)?;
    assert_eq!(with_duplicate.samples(), original.samples());
    for sample in original.samples() {
        assert_eq!(
            with_duplicate.sample_data(&duplicated, sample),
            original.sample_data(&input, sample)
        );
    }
    let error = demux(&dropped, VideoCodec::H264)
        .err()
        .ok_or("a missing packet was accepted")?;
    assert!(matches!(error, VideoError::Malformed { .. }), "{error:?}");
    assert!(error.to_string().contains("continuity_counter"), "{error}");
    Ok(())
}

/// Annex A: a PSI section whose CRC_32 does not check is refused. In the
/// first PID 0 packet, with no adaptation field and a pointer_field of 0, the
/// PAT's program_number follows the four byte packet header, the pointer
/// field and the eight byte section header, at byte 4 + 1 + 8.
#[test]
fn a_section_failing_its_crc_is_refused() -> TestResult {
    let mut input = fixture("h264_gop12.m2t")?;
    let pat = ts_packets(&input)?
        .iter()
        .position(|(pid, _)| *pid == 0)
        .ok_or("no PAT packet")?
        * 188;
    assert_eq!(byte(&input, pat + 3)? & 0x30, 0x10, "payload only");
    assert_eq!(byte(&input, pat + 4)?, 0, "pointer_field 0");
    if let Some(value) = input.get_mut(pat + 4 + 1 + 8) {
        *value ^= 0x01;
    }
    let error = demux(&input, VideoCodec::H264)
        .err()
        .ok_or("a bad CRC was accepted")?;
    assert!(error.to_string().contains("CRC_32"), "{error}");
    Ok(())
}

/// A version 0 ctts offset with its top bit set is unsigned under ISO/IEC
/// 14496-12 and negative under the QuickTime File Format, so the two read a
/// different frame order and the demuxer refuses rather than choose.
#[test]
fn an_ambiguous_composition_offset_is_refused() -> TestResult {
    let mut input = fixture("h264_gop12.mp4")?;
    let top = boxes(&input, 0, input.len())?;
    let moov = top
        .into_iter()
        .find(|candidate| &candidate.kind == b"moov")
        .ok_or("no moov")?;
    let trak = require(&input, moov, b"trak")?;
    let stbl = require(
        &input,
        require(&input, require(&input, trak, b"mdia")?, b"minf")?,
        b"stbl",
    )?;
    let ctts = require(&input, stbl, b"ctts")?;
    // Version 0, then the first run's offset at body + 8 + 4.
    let edits = [
        (ctts.body, 0u8),
        (ctts.body + 12, 0x80),
        (ctts.body + 13, 0),
        (ctts.body + 14, 0),
        (ctts.body + 15, 0),
    ];
    for (at, value) in edits {
        if let Some(slot) = input.get_mut(at) {
            *slot = value;
        }
    }
    let error = demux(&input, VideoCodec::H264)
        .err()
        .ok_or("an ambiguous offset was accepted")?;
    assert!(error.to_string().contains("QuickTime"), "{error}");
    Ok(())
}

/// Two samples at one presentation time have no order. `h264_poc2.mp4` has no
/// ctts box, so setting every stts delta to 0 gives all 40 samples time 0.
#[test]
fn samples_sharing_a_presentation_time_are_refused() -> TestResult {
    let mut input = fixture("h264_poc2.mp4")?;
    let top = boxes(&input, 0, input.len())?;
    let moov = top
        .into_iter()
        .find(|candidate| &candidate.kind == b"moov")
        .ok_or("no moov")?;
    let trak = require(&input, moov, b"trak")?;
    let stbl = require(
        &input,
        require(&input, require(&input, trak, b"mdia")?, b"minf")?,
        b"stbl",
    )?;
    assert!(child(&input, stbl, b"ctts")?.is_none());
    let stts = require(&input, stbl, b"stts")?;
    for run in 0..entry_count(&input, stts)? {
        let delta = stts.body + 8 + run * 8 + 4;
        for at in delta..delta + 4 {
            if let Some(value) = input.get_mut(at) {
                *value = 0;
            }
        }
    }
    assert_eq!(
        demux(&input, VideoCodec::H264).err(),
        Some(VideoError::DuplicatePresentationTime { pts: 0 })
    );
    Ok(())
}

/// Rec. ITU-T H.222.0 (08/2023) 2.4.3.7: a PTS refers to the first access unit
/// that commences in its PES packet. With two access units a packet, the
/// second, decode index 1, has none, and the table refuses rather than
/// inventing one.
#[test]
fn an_access_unit_without_a_pts_is_refused() -> TestResult {
    let input = fixture("h264_gop12_two_per_pes.m2t")?;
    assert_eq!(
        demux(&input, VideoCodec::H264).err(),
        Some(VideoError::MissingTimestamp { decode_index: 1 })
    );
    Ok(())
}

// Independent transport stream reader, Rec. ITU-T H.222.0 (08/2023) Table 2-2
// for the packet header, Table 2-6 for the adaptation field length and Table
// 2-21 for the PES header. It returns the PTS of every PES packet that starts
// on a PID whose payload opens with a video stream_id, 0xE0 to 0xEF in Table
// 2-22.
fn reference_ts_pts(input: &[u8]) -> TestResult<Vec<u64>> {
    let mut pts = Vec::new();
    for packet in input.chunks(188) {
        let flags = byte(packet, 1)?;
        if flags & 0x40 == 0 {
            continue;
        }
        let control = (byte(packet, 3)? >> 4) & 0x03;
        let start = if control & 0x02 != 0 {
            5 + usize::from(byte(packet, 4)?)
        } else {
            4
        };
        let Some(payload) = packet.get(start..) else {
            continue;
        };
        if payload.get(0..3) != Some(&[0, 0, 1][..]) || !(0xE0..=0xEF).contains(&byte(payload, 3)?)
        {
            continue;
        }
        if byte(payload, 7)? & 0x80 == 0 {
            return Err("a video PES without a PTS".into());
        }
        // PTS[32..30], PTS[29..15] and PTS[14..0], each followed by a marker.
        let high = u64::from(byte(payload, 9)? >> 1) & 0x07;
        let middle = u64::from(be16(payload, 10)? >> 1);
        let low = u64::from(be16(payload, 12)? >> 1);
        pts.push(high << 30 | middle << 15 | low);
    }
    Ok(pts)
}

// ---------------------------------------------------------------------------
// AC5, MPEG-2, and the refusals that name F-242
// ---------------------------------------------------------------------------

/// AC5. 12 frames with `-g 4 -bf 2`, so the table has B-frames and an open
/// group, in MP4, in a transport stream and as a bare elementary stream.
#[test]
fn ac5_mpeg2_tables_in_mp4_ts_and_elementary_stream() -> TestResult {
    let reference = fixture("mpeg2_gop4.mp4")?;
    let (table, _) = assert_matches_boxes(&reference, VideoCodec::Mpeg2)?;
    assert_eq!(table.samples().len(), 12);
    assert_eq!(table.codec_config().sample_format, SampleFormat::StartCodes);
    let ts = fixture("mpeg2_gop4.m2t")?;
    assert_same_table(&reference, &ts, VideoCodec::Mpeg2, VideoContainer::MpegTs)?;
    let es = fixture("mpeg2_gop4.m2v")?;
    let es_table = assert_same_table(&reference, &es, VideoCodec::Mpeg2, VideoContainer::Mpeg2Es)?;
    assert_eq!(es_table.timescale(), None);
    Ok(())
}

/// AC5. With no group of pictures header, 6.2.2, the display order is
/// temporal_reference counted modulo 1024, 6.3.9. `mpeg2_tr_wrap.m2v` numbers
/// the twelve frames of `mpeg2_gop4.m2v` from 1020, so the count passes 1023.
#[test]
fn ac5_mpeg2_temporal_reference_wraps_without_group_headers() -> TestResult {
    let reference = fixture("mpeg2_gop4.mp4")?;
    let input = fixture("mpeg2_tr_wrap.m2v")?;
    assert!(
        !input.windows(4).any(|window| window == [0, 0, 1, 0xB8]),
        "no group header"
    );
    assert_same_table(
        &reference,
        &input,
        VideoCodec::Mpeg2,
        VideoContainer::Mpeg2Es,
    )?;
    Ok(())
}

/// AC5. PS3.5 8.2.5 allows MPEG-PS and a bare PES for MPEG-2, and this build
/// reads neither. Each is refused by name, and the sentence names F-242.
#[test]
fn ac5_program_stream_and_bare_pes_are_refused_naming_f242() -> TestResult {
    let program = fixture("mpeg2_gop4.mpg")?;
    let error = demux(&program, VideoCodec::Mpeg2)
        .err()
        .ok_or("program stream accepted")?;
    assert_eq!(error, VideoError::ProgramStream);
    assert!(error.to_string().contains("F-242"), "{error}");
    assert!(error.to_string().contains("program stream"), "{error}");

    let pes = fixture("mpeg2_gop4.pes")?;
    let error = demux(&pes, VideoCodec::Mpeg2)
        .err()
        .ok_or("bare PES accepted")?;
    assert_eq!(error, VideoError::BarePes);
    assert!(error.to_string().contains("F-242"), "{error}");
    assert!(error.to_string().contains("PES"), "{error}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Codec configuration
// ---------------------------------------------------------------------------

/// RFC 6381 section 3.3: avc1 followed by the hexadecimal profile_idc,
/// constraint flags byte and level_idc, the first three bytes after the NAL
/// header of the SPS, Rec. ITU-T H.264 (08/2024) 7.3.2.1.1. The record is the
/// avcC body, read here by the box walker. 64 by 64 is `size=64x64`.
#[test]
fn h264_codec_config_from_the_mp4_record() -> TestResult {
    let input = fixture("h264_gop12.mp4")?;
    let track = reference_mp4(&input)?;
    let record = track.configuration.clone().ok_or("no avcC")?;
    let table = demux(&input, VideoCodec::H264)?;
    let config = table.codec_config();
    // The record repeats those three bytes at offsets 1 to 3.
    let expected = format!(
        "avc1.{:02X}{:02X}{:02X}",
        byte(&record, 1)?,
        byte(&record, 2)?,
        byte(&record, 3)?
    );
    assert_eq!(config.codec_string.as_deref(), Some(expected.as_str()));
    // `-profile:v high` is profile_idc 100, Rec. ITU-T H.264 (08/2024) A.2.4.
    assert!(expected.starts_with("avc1.64"), "{expected}");
    assert_eq!(config.description.as_deref(), Some(record.as_slice()));
    assert_eq!(config.sample_format, reference_length_size(&track)?);
    assert_eq!(
        config.coded_size,
        FrameSize {
            width: 64,
            height: 64
        }
    );
    assert_eq!(
        config.visible_size,
        FrameSize {
            width: 64,
            height: 64
        }
    );
    assert!(!config.has_audio);
    assert!(!config.interlaced);
    assert_eq!(
        config.colour, None,
        "no colour arguments, so no description"
    );
    Ok(())
}

/// The same string from the SPS of the Annex B stream, read by start code.
#[test]
fn h264_codec_string_from_an_annex_b_sps() -> TestResult {
    let input = fixture("h264_gop12.h264")?;
    let sps = start_code_units(&input)
        .into_iter()
        .find(|unit| unit.first().is_some_and(|header| header & 0x1F == 7))
        .ok_or("no SPS")?;
    let expected = format!(
        "avc1.{:02X}{:02X}{:02X}",
        byte(sps, 1)?,
        byte(sps, 2)?,
        byte(sps, 3)?
    );
    let table = demux(&input, VideoCodec::H264)?;
    assert_eq!(
        table.codec_config().codec_string.as_deref(),
        Some(expected.as_str())
    );
    Ok(())
}

/// The HEVC string, written from the general profile, tier and level of the
/// SPS, Rec. ITU-T H.265 (07/2024) 7.3.3, in the dotted form of ISO/IEC
/// 14496-15 Annex E as the WebCodecs HEVC registration cites it. That annex
/// was not available to read, so the form here is the commonly published one
/// and is a review item: the profile space letter and profile_idc, the 32
/// compatibility flags bit reversed in hexadecimal, L or H and the level_idc,
/// then the six constraint bytes in hexadecimal with trailing zero bytes left
/// out.
#[test]
fn hevc_codec_string_from_the_sps_profile_tier_level() -> TestResult {
    for (name, profile_idc) in [("hevc_main.mp4", 1u8), ("hevc_main10.mp4", 2u8)] {
        let input = fixture(name)?;
        let table = demux(&input, VideoCodec::H265)?;
        let track = reference_mp4(&input)?;
        let format = reference_length_size(&track)?;
        let record = track.configuration.ok_or("no hvcC")?;
        let ptl = hevc_profile_tier_level(&record)?;
        // Main is profile_idc 1 and Main 10 is 2, Rec. ITU-T H.265 (07/2024)
        // A.3.2 and A.3.3, from `-pix_fmt yuv420p` and `yuv420p10le`.
        assert_eq!(ptl.profile_idc, profile_idc, "{name}");
        assert_eq!(
            table.codec_config().codec_string.as_deref(),
            Some(ptl.codec_string().as_str()),
            "{name}"
        );
        assert_eq!(
            table.codec_config().description.as_deref(),
            Some(record.as_slice()),
            "{name}"
        );
        assert_eq!(table.codec_config().sample_format, format, "{name}");
    }
    Ok(())
}

struct ProfileTierLevel {
    profile_space: u8,
    tier: bool,
    profile_idc: u8,
    compatibility: u32,
    constraints: [u8; 6],
    level_idc: u8,
}

impl ProfileTierLevel {
    fn codec_string(&self) -> String {
        let space = match self.profile_space {
            1 => "A",
            2 => "B",
            3 => "C",
            _ => "",
        };
        let mut text = format!(
            "hvc1.{space}{}.{:X}.{}{}",
            self.profile_idc,
            self.compatibility.reverse_bits(),
            if self.tier { 'H' } else { 'L' },
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

/// The SPS from the hvcC arrays, then the general profile_tier_level fields
/// after its two byte NAL header and the byte holding the vps id, sub-layer
/// count and nesting flag, Rec. ITU-T H.265 (07/2024) 7.3.2.2.1 and 7.3.3.
/// Emulation prevention bytes are removed first, 7.3.1.1. The arrays start
/// with numOfArrays at byte 22, and each holds a byte with NAL_unit_type in
/// its low six bits, a 16-bit count and each unit behind a 16-bit length.
/// That is the HEVCDecoderConfigurationRecord of ISO/IEC 14496-15 8.3.3.1,
/// which was not available to read, so this reader and the demuxer share one
/// unchecked transcription of it and a reviewer checks it against the text.
fn hevc_profile_tier_level(record: &[u8]) -> TestResult<ProfileTierLevel> {
    let arrays = usize::from(byte(record, 22)?);
    let mut position = 23usize;
    for _ in 0..arrays {
        let nal_type = byte(record, position)? & 0x3F;
        let count = usize::from(be16(record, position + 1)?);
        position += 3;
        for _ in 0..count {
            let length = usize::from(be16(record, position)?);
            let nal = bytes_at(record, position + 2, length)?;
            position += 2 + length;
            if nal_type != 33 {
                continue;
            }
            let mut rbsp = Vec::new();
            let mut zeros = 0usize;
            for value in nal.iter().skip(2) {
                if zeros >= 2 && *value == 3 {
                    zeros = 0;
                    continue;
                }
                zeros = if *value == 0 { zeros + 1 } else { 0 };
                rbsp.push(*value);
            }
            let first = byte(&rbsp, 1)?;
            return Ok(ProfileTierLevel {
                profile_space: first >> 6,
                tier: first & 0x20 != 0,
                profile_idc: first & 0x1F,
                compatibility: be32(&rbsp, 2)?,
                constraints: bytes_at(&rbsp, 6, 6)?.try_into()?,
                level_idc: byte(&rbsp, 12)?,
            });
        }
    }
    Err("no SPS in the hvcC arrays".into())
}

/// Item 6 of the plan, per field as the lead ruled on review pass 1, PS3.5
/// 8.2.8: the range is the stream's whenever video_signal_type_present_flag
/// is set, and the code points are the stream's colour description when it
/// has one and the colr box's otherwise. Each expected code point is the Rec.
/// ITU-T H.273 value of the name the generator gave ffmpeg, `bt470bg` 5,
/// `bt709` 1 and `smpte170m` 6, and range `pc` is a full range flag of 1. The
/// three code points of each description differ, so a reader that swaps two
/// fields fails here.
#[test]
fn colour_description_per_field_from_the_stream_then_colr() -> TestResult {
    use ColourSource::{ColrBox, SequenceDisplayExtension, Vui};
    // (clip, codec, code point source, code points, range source, full range)
    let cases = [
        (
            "h264_vui.mp4",
            VideoCodec::H264,
            Some((Vui, [5, 1, 6], Vui, false)),
        ),
        (
            "h264_vui.h264",
            VideoCodec::H264,
            Some((Vui, [5, 1, 6], Vui, false)),
        ),
        (
            "h264_vui_full.mp4",
            VideoCodec::H264,
            Some((Vui, [1, 6, 5], Vui, true)),
        ),
        (
            "h264_colr.mp4",
            VideoCodec::H264,
            Some((ColrBox, [1, 6, 5], ColrBox, false)),
        ),
        (
            "h264_colr_full.mp4",
            VideoCodec::H264,
            Some((ColrBox, [1, 6, 5], ColrBox, true)),
        ),
        // The stream says 5, 1, 6 and the colr box says 1, 6, 5.
        (
            "h264_vui_and_colr.mp4",
            VideoCodec::H264,
            Some((Vui, [5, 1, 6], Vui, false)),
        ),
        ("h264_plain.mp4", VideoCodec::H264, None),
        (
            "hevc_main.mp4",
            VideoCodec::H265,
            Some((Vui, [5, 1, 6], Vui, false)),
        ),
        (
            "hevc_main.h265",
            VideoCodec::H265,
            Some((Vui, [5, 1, 6], Vui, false)),
        ),
        // The review's clip: the VUI signals range 0 and no code points, and
        // the colr box says 1, 6, 5 and full range. The code points come from
        // colr and the range stays the stream's, so MP4 and the bare stream
        // agree on it. The bare stream's code points are E.3.1's inferred 2.
        (
            "hevc_main10_colr_full.mp4",
            VideoCodec::H265,
            Some((ColrBox, [1, 6, 5], Vui, false)),
        ),
        (
            "hevc_main10.h265",
            VideoCodec::H265,
            Some((Vui, [2, 2, 2], Vui, false)),
        ),
        (
            "mpeg2_gop4.m2v",
            VideoCodec::Mpeg2,
            Some((
                SequenceDisplayExtension,
                [5, 1, 6],
                SequenceDisplayExtension,
                false,
            )),
        ),
        (
            "mpeg2_gop4.mp4",
            VideoCodec::Mpeg2,
            Some((
                SequenceDisplayExtension,
                [5, 1, 6],
                SequenceDisplayExtension,
                false,
            )),
        ),
    ];
    for (name, codec, expected) in cases {
        let table = demux(&fixture(name)?, codec)?;
        let expected = expected.map(
            |(code_point_source, [primaries, transfer, matrix], range_source, full_range)| {
                ColourDescription {
                    code_point_source,
                    range_source,
                    colour_primaries: primaries,
                    transfer_characteristics: transfer,
                    matrix_coefficients: matrix,
                    full_range,
                }
            },
        );
        assert_eq!(table.codec_config().colour, expected, "{name}");
    }
    Ok(())
}

/// Cropping. Rec. ITU-T H.264 (08/2024) 7-13 to 7-18 code whole 16 by 16
/// macroblocks, so a 56 by 56 picture is coded 64 by 64 and cropped by 7-19
/// to 7-24 in both directions. Rec. ITU-T H.262 (02/2000) 6.3.3 gives mb_width
/// as ( horizontal_size + 15 ) / 16 and, for a progressive sequence, mb_height
/// as ( vertical_size + 15 ) / 16. Rec. ITU-T H.265 (07/2024) leaves the coded
/// size to the encoder, so only the visible size, the generator's
/// `size=60x60`, is checked there.
#[test]
fn cropping_gives_the_visible_size() -> TestResult {
    let square = |side| FrameSize {
        width: side,
        height: side,
    };
    let h264 = demux(&fixture("h264_crop.mp4")?, VideoCodec::H264)?;
    assert_eq!(h264.codec_config().visible_size, square(56));
    assert_eq!(h264.codec_config().coded_size, square(64));
    let mpeg2 = demux(&fixture("mpeg2_crop.m2v")?, VideoCodec::Mpeg2)?;
    assert_eq!(mpeg2.codec_config().visible_size, square(56));
    assert_eq!(mpeg2.codec_config().coded_size, square(64));
    let hevc = demux(&fixture("hevc_crop.mp4")?, VideoCodec::H265)?;
    assert_eq!(hevc.codec_config().visible_size, square(60));
    Ok(())
}

/// A movie extends box only announces that fragments may follow. With no
/// movie fragment box the samples are all in the sample tables, so adding an
/// empty `mvex` to `h264_plain.mp4`'s moov, its last box, changes no sample.
#[test]
fn a_movie_extends_box_without_fragments_is_read() -> TestResult {
    let input = fixture("h264_plain.mp4")?;
    let top = boxes(&input, 0, input.len())?;
    let moov = *top.last().ok_or("no boxes")?;
    assert_eq!(&moov.kind, b"moov");
    assert_eq!(moov.end, input.len());
    let mut extended = input.clone();
    extended.extend_from_slice(&[0, 0, 0, 8]);
    extended.extend_from_slice(b"mvex");
    let size = u32::try_from(moov.end - (moov.body - 8) + 8)?;
    let header = moov.body - 8;
    for (offset, value) in size.to_be_bytes().iter().enumerate() {
        if let Some(slot) = extended.get_mut(header + offset) {
            *slot = *value;
        }
    }
    let original = demux(&input, VideoCodec::H264)?;
    assert_eq!(
        demux(&extended, VideoCodec::H264)?.samples(),
        original.samples()
    );
    Ok(())
}

/// `interlaced=1:tff=1` codes with frame_mbs_only_flag 0, Rec. ITU-T H.264
/// (08/2024) 7.4.2.1.1. Every other H.264 clip is progressive.
#[test]
fn interlace_is_named() -> TestResult {
    let table = demux(&fixture("h264_interlaced.mp4")?, VideoCodec::H264)?;
    assert!(table.codec_config().interlaced);
    assert_eq!(table.samples().len(), 4);
    let table = demux(&fixture("h264_plain.mp4")?, VideoCodec::H264)?;
    assert!(!table.codec_config().interlaced);
    Ok(())
}

/// MPEG-2 has no WebCodecs registration, so there is no codec string, and the
/// sequence header carries the size, Rec. ITU-T H.262 (02/2000) 6.2.2.1.
#[test]
fn mpeg2_codec_config() -> TestResult {
    for name in ["mpeg2_gop4.mp4", "mpeg2_gop4.m2t", "mpeg2_gop4.m2v"] {
        let table = demux(&fixture(name)?, VideoCodec::Mpeg2)?;
        let config = table.codec_config();
        assert_eq!(config.codec_string, None, "{name}");
        assert_eq!(config.description, None, "{name}");
        assert_eq!(
            config.visible_size,
            FrameSize {
                width: 64,
                height: 64
            },
            "{name}"
        );
        assert_eq!(
            config.coded_size,
            FrameSize {
                width: 64,
                height: 64
            },
            "{name}"
        );
        assert!(!config.interlaced, "{name}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_container_is_refused_by_name() -> TestResult {
    let input = fixture("garbage.bin")?;
    for codec in [VideoCodec::H264, VideoCodec::H265, VideoCodec::Mpeg2] {
        assert_eq!(
            demux(&input, codec).err(),
            Some(VideoError::UnknownContainer)
        );
    }
    assert_eq!(
        demux(&[], VideoCodec::H264).err(),
        Some(VideoError::UnknownContainer)
    );
    Ok(())
}

#[test]
fn a_fragmented_mp4_is_refused_by_name() -> TestResult {
    let error = demux(&fixture("h264_fragmented.mp4")?, VideoCodec::H264)
        .err()
        .ok_or("fragmented MP4 accepted")?;
    assert!(matches!(error, VideoError::Unsupported { .. }), "{error:?}");
    assert!(error.to_string().contains("fragmented"), "{error}");
    Ok(())
}

/// The transfer syntax names the codec and the container names it again. A
/// disagreement is refused with both named, rather than handing the wrong
/// codec's samples to a decoder.
#[test]
fn a_codec_that_disagrees_with_the_transfer_syntax_is_refused() -> TestResult {
    let cases = [
        ("h264_gop12.mp4", VideoCodec::H265, VideoCodec::H264),
        ("hevc_main.m2t", VideoCodec::H264, VideoCodec::H265),
        ("mpeg2_gop4.mp4", VideoCodec::H264, VideoCodec::Mpeg2),
        ("mpeg2_gop4.m2v", VideoCodec::H264, VideoCodec::Mpeg2),
        ("h264_gop12.m2t", VideoCodec::Mpeg2, VideoCodec::H264),
    ];
    for (name, expected, found) in cases {
        let error = demux(&fixture(name)?, expected)
            .err()
            .ok_or("mismatch accepted")?;
        assert_eq!(
            error,
            VideoError::CodecMismatch { expected, found },
            "{name}"
        );
    }
    Ok(())
}

/// A truncated or damaged file returns an error or a shorter table and never
/// panics, because a panic poisons the wasm instance, HLD section 23. This
/// sweep covers only what it generates: about 61 truncations and 61 one-byte
/// corruptions of each clip listed, each read as all three codecs. It is not
/// evidence about crafted fields, which is why the range checks of each
/// parser have their own tests, `delta_scale_outside_its_range_is_refused`
/// among them.
#[test]
fn damaged_input_never_panics() -> TestResult {
    let names = [
        "h264_gop12.mp4",
        "h264_gop12.m2t",
        "h264_gop12.h264",
        "hevc_main.mp4",
        "hevc_main.h265",
        "mpeg2_gop4.m2v",
        "mpeg2_gop4.m2t",
    ];
    for name in names {
        let input = fixture(name)?;
        let step = (input.len() / 61).max(1);
        for cut in (0..input.len()).step_by(step) {
            let prefix = input.get(..cut).ok_or("cut out of range")?;
            let _ = demux(prefix, VideoCodec::H264);
            let _ = demux(prefix, VideoCodec::H265);
            let _ = demux(prefix, VideoCodec::Mpeg2);
            let mut flipped = input.clone();
            if let Some(value) = flipped.get_mut(cut) {
                *value ^= 0xA5;
            }
            let _ = demux(&flipped, VideoCodec::H264);
            let _ = demux(&flipped, VideoCodec::H265);
            let _ = demux(&flipped, VideoCodec::Mpeg2);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Crafted MP4 files, the lead rulings on review pass 1. A small box tree
// rewriter, written from the same "Atoms" layout as the reader above, edits a
// clip whose moov box is its last box, so no chunk offset into mdat moves.
// ---------------------------------------------------------------------------

/// A box, either raw bytes or a container with a fixed prefix before its
/// children: none for moov, trak, mdia, minf and stbl, the version, flags and
/// entry count for stsd, and the 78 fixed bytes of a visual sample entry.
#[derive(Clone, Debug)]
enum Node {
    Leaf([u8; 4], Vec<u8>),
    Container([u8; 4], Vec<u8>, Vec<Node>),
}

fn prefix_length(kind: &[u8; 4]) -> Option<usize> {
    match kind {
        b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" => Some(0),
        b"stsd" => Some(8),
        b"avc1" | b"avc3" | b"hvc1" | b"hev1" | b"mp4v" => Some(78),
        _ => None,
    }
}

fn parse_nodes(bytes: &[u8], start: usize, end: usize) -> TestResult<Vec<Node>> {
    let mut nodes = Vec::new();
    for found in boxes(bytes, start, end)? {
        let body = bytes_at(bytes, found.body, found.end - found.body)?;
        nodes.push(match prefix_length(&found.kind) {
            Some(prefix) => Node::Container(
                found.kind,
                bytes_at(body, 0, prefix)?.to_vec(),
                parse_nodes(bytes, found.body + prefix, found.end)?,
            ),
            None => Node::Leaf(found.kind, body.to_vec()),
        });
    }
    Ok(nodes)
}

fn serialise(nodes: &[Node]) -> TestResult<Vec<u8>> {
    let mut out = Vec::new();
    for node in nodes {
        let (kind, body) = match node {
            Node::Leaf(kind, body) => (kind, body.clone()),
            Node::Container(kind, prefix, children) => {
                (kind, [prefix.clone(), serialise(children)?].concat())
            }
        };
        out.extend_from_slice(&u32::try_from(body.len() + 8)?.to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(&body);
    }
    Ok(out)
}

/// Applies `edit` to the first leaf of type `kind`, depth first, and lets it
/// return boxes to insert after that leaf.
fn edit_leaf(
    nodes: &mut Vec<Node>,
    kind: &[u8; 4],
    edit: &mut dyn FnMut(&mut Vec<u8>) -> Vec<Node>,
) -> bool {
    for position in 0..nodes.len() {
        let inserted = match nodes.get_mut(position) {
            Some(Node::Leaf(found, body)) if found == kind => Some(edit(body)),
            Some(Node::Container(_, _, children)) => {
                if edit_leaf(children, kind, edit) {
                    return true;
                }
                None
            }
            _ => None,
        };
        if let Some(inserted) = inserted {
            for (offset, node) in inserted.into_iter().enumerate() {
                nodes.insert(position + 1 + offset, node);
            }
            return true;
        }
    }
    false
}

/// `name` with the first `kind` box edited, the moov box rebuilt with sizes
/// that agree, and every byte before the moov box unchanged.
fn rewritten(
    name: &str,
    kind: &[u8; 4],
    mut edit: impl FnMut(&mut Vec<u8>) -> Vec<Node>,
) -> TestResult<Vec<u8>> {
    let input = fixture(name)?;
    let top = boxes(&input, 0, input.len())?;
    let moov = *top.last().ok_or("no boxes")?;
    if &moov.kind != b"moov" || moov.end != input.len() {
        return Err("the moov box is not last".into());
    }
    let mut nodes = parse_nodes(&input, moov.body - 8, moov.end)?;
    if !edit_leaf(&mut nodes, kind, &mut edit) {
        return Err(format!("no {kind:?} box").into());
    }
    let prefix = bytes_at(&input, 0, moov.body - 8)?;
    Ok([prefix.to_vec(), serialise(&nodes)?].concat())
}

fn add_to_be32(body: &mut [u8], at: usize, step: u32) {
    let value = body
        .get(at..at + 4)
        .and_then(|field| <[u8; 4]>::try_from(field).ok())
        .map(u32::from_be_bytes)
        .unwrap_or_default();
    for (offset, byte) in (value + step).to_be_bytes().iter().enumerate() {
        if let Some(slot) = body.get_mut(at + offset) {
            *slot = *byte;
        }
    }
}

/// The rewriter changes nothing it is not asked to: an edit that leaves the
/// box alone reproduces the clip byte for byte, so the cases below differ from
/// their clip only by their edit.
#[test]
fn the_box_rewriter_is_exact() -> TestResult {
    for name in ["h264_gop12.mp4", "h264_colr.mp4"] {
        assert_eq!(
            rewritten(name, b"stss", |_| Vec::new())?,
            fixture(name)?,
            "{name}"
        );
    }
    Ok(())
}

/// Review pass 1 D2: an `stss` entry count one too large, with a `free` box
/// after it whose bytes would read as a sixth sync sample number. The count
/// runs past the box, so the table is refused rather than given a random
/// access point that is not one.
#[test]
fn a_sync_sample_count_past_its_box_is_refused() -> TestResult {
    let input = rewritten("h264_gop12.mp4", b"stss", |body| {
        add_to_be32(body, 4, 1);
        vec![Node::Leaf(*b"free", vec![0; 48])]
    })?;
    assert_eq!(
        demux(&input, VideoCodec::H264).err(),
        Some(VideoError::Truncated {
            structure: "MP4 sync sample box"
        })
    );
    Ok(())
}

/// The same for an `stsc` entry count one too large.
#[test]
fn a_sample_to_chunk_count_past_its_box_is_refused() -> TestResult {
    let input = rewritten("h264_gop12.mp4", b"stsc", |body| {
        add_to_be32(body, 4, 1);
        Vec::new()
    })?;
    assert_eq!(
        demux(&input, VideoCodec::H264).err(),
        Some(VideoError::Truncated {
            structure: "MP4 sample-to-chunk box"
        })
    );
    Ok(())
}

/// A body cut short ends inside the field the reader needs, and each read
/// stops at its own box, so the box is named as ending early. hdlr needs 12
/// bytes for its handler type, version 0 mdhd 16 for its time scale, colr 11
/// for an nclx full range flag, and stss and stsc one whole entry more than
/// they keep.
#[test]
fn a_box_cut_short_is_refused_by_name() -> TestResult {
    let cases: [(&str, &[u8; 4], usize, &str); 5] = [
        ("h264_gop12.mp4", b"hdlr", 8, "MP4 handler box"),
        ("h264_gop12.mp4", b"mdhd", 12, "MP4 media header"),
        ("h264_colr.mp4", b"colr", 10, "colr box"),
        ("h264_gop12.mp4", b"stss", 4, "MP4 sync sample box"),
        ("h264_gop12.mp4", b"stsc", 4, "MP4 sample-to-chunk box"),
    ];
    for (name, kind, cut, structure) in cases {
        let input = rewritten(name, kind, |body| {
            // hdlr, mdhd and colr keep `cut` bytes, stss and stsc lose `cut`.
            let keep = if matches!(kind, b"stss" | b"stsc") {
                body.len() - cut
            } else {
                cut
            };
            body.truncate(keep);
            Vec::new()
        })?;
        assert_eq!(
            demux(&input, VideoCodec::H264).err(),
            Some(VideoError::Truncated { structure }),
            "{kind:?}"
        );
    }
    Ok(())
}

/// ISO/IEC 14496-12 mdhd version 1 widens creation, modification and
/// duration to 64 bits and keeps the 32-bit time scale after the two times.
/// That layout was not available to read and is a review item. A clip's mdhd
/// rewritten to version 1 with the same values gives the same table.
#[test]
fn a_version_1_media_header_gives_the_same_table() -> TestResult {
    let input = rewritten("h264_gop12.mp4", b"mdhd", |body| {
        let field = |at: usize| body.get(at..at + 4).map(<[u8]>::to_vec).unwrap_or_default();
        let widen = |at: usize| [vec![0; 4], field(at)].concat();
        let rest = body.get(20..).map(<[u8]>::to_vec).unwrap_or_default();
        let flags = body.get(1..4).map(<[u8]>::to_vec).unwrap_or_default();
        *body = [
            vec![1],
            flags,
            widen(4),
            widen(8),
            field(12),
            widen(16),
            rest,
        ]
        .concat();
        Vec::new()
    })?;
    let original = demux(&fixture("h264_gop12.mp4")?, VideoCodec::H264)?;
    let table = demux(&input, VideoCodec::H264)?;
    assert_eq!(table.timescale(), original.timescale());
    assert_eq!(table.samples(), original.samples());
    Ok(())
}

/// Review pass 1 D1 through the public path: the generator's two Annex B
/// SPS clips carry delta_scale 2^31 - 1 and -(2^31 - 1), outside 7.4.2.1.1.1's
/// -128 to +127, and are refused by name.
#[test]
fn delta_scale_outside_its_range_is_refused() -> TestResult {
    for name in ["h264_delta_scale_high.h264", "h264_delta_scale_low.h264"] {
        assert_eq!(
            demux(&fixture(name)?, VideoCodec::H264).err(),
            Some(VideoError::Malformed {
                structure: "H.264 sequence parameter set",
                reason: "delta_scale outside -128 to 127",
            }),
            "{name}"
        );
    }
    Ok(())
}

/// Rec. ITU-T H.222.0 (08/2023) 2.4.3.3: a continuity_counter jump is allowed
/// where discontinuity_indicator is set, Table 2-6. From the first video
/// packet carrying an adaptation field with flags, every video
/// continuity_counter is raised by 5 modulo 16 and that packet's
/// discontinuity_indicator set, and the table is the clip's own.
#[test]
fn a_flagged_continuity_jump_is_accepted() -> TestResult {
    let input = fixture("h264_gop12.m2t")?;
    let pid = video_pid(&input)?;
    let mut jumped = input.clone();
    let mut found = false;
    let mut seen_video = false;
    let mut jump_at = 0usize;
    for (number, (packet_pid, packet)) in ts_packets(&input)?.into_iter().enumerate() {
        if packet_pid != pid {
            continue;
        }
        let at = number * 188;
        let control = byte(packet, 3)?;
        // The jump needs a video packet before it to jump from.
        let flagged = seen_video && control & 0x20 != 0 && byte(packet, 4)? > 0;
        seen_video = true;
        if !found && !flagged {
            continue;
        }
        if !found {
            jump_at = at;
        }
        if !found {
            found = true;
            if let Some(flags) = jumped.get_mut(at + 5) {
                *flags |= 0x80;
            }
        }
        if let Some(slot) = jumped.get_mut(at + 3) {
            *slot = (control & 0xF0) | (((control & 0x0F) + 5) % 16);
        }
    }
    assert!(
        found,
        "the clip has a video packet with adaptation field flags"
    );
    let original = demux(&input, VideoCodec::H264)?;
    assert_eq!(
        demux(&jumped, VideoCodec::H264)?.samples(),
        original.samples()
    );
    // Without the indicator the same jump is a missing packet.
    let mut unflagged = jumped.clone();
    if let Some(flags) = unflagged.get_mut(jump_at + 5) {
        *flags &= 0x7F;
    }
    let error = demux(&unflagged, VideoCodec::H264)
        .err()
        .ok_or("an unflagged jump was accepted")?;
    assert!(error.to_string().contains("continuity_counter"), "{error}");
    Ok(())
}

/// Rec. ITU-T H.265 (07/2024) 7.4.2.4.4 NOTE 1: the last VCL NAL unit before
/// a picture may have nuh_layer_id above 0. Built from `hevc_main.h265`: the
/// first picture's units, a copy of its slice with nuh_layer_id 1, the PPS
/// again, then the second picture's slice. The PPS follows a VCL NAL unit, so
/// it opens the second access unit, and the layer 1 slice stays in the first.
#[test]
fn a_layer_1_slice_stays_in_its_access_unit() -> TestResult {
    let input = fixture("hevc_main.h265")?;
    let units = start_code_units(&input);
    let kind = |unit: &&[u8]| unit.first().map(|header| (header >> 1) & 0x3F);
    let first_vcl = units
        .iter()
        .position(|unit| kind(unit).is_some_and(|k| k <= 31))
        .ok_or("no slice")?;
    let second_vcl = units
        .iter()
        .skip(first_vcl + 1)
        .position(|unit| kind(unit).is_some_and(|k| k <= 31))
        .ok_or("no second slice")?
        + first_vcl
        + 1;
    let pps = units
        .iter()
        .find(|unit| kind(unit) == Some(34))
        .ok_or("no PPS")?;
    let slice = units.get(first_vcl).ok_or("no slice")?;
    // 7.3.1.2: nuh_layer_id is the low bit of byte 0 and the top five bits of
    // byte 1, so layer 1 is 0b00001 in byte 1's top bits.
    let mut layered = slice.to_vec();
    if let Some(second) = layered.get_mut(1) {
        *second = (1 << 3) | (*second & 0x07);
    }
    let mut chosen: Vec<Vec<u8>> = units
        .iter()
        .take(first_vcl + 1)
        .map(|unit| unit.to_vec())
        .collect();
    chosen.push(layered);
    chosen.push(pps.to_vec());
    chosen.push(units.get(second_vcl).ok_or("no second slice")?.to_vec());
    let stream: Vec<u8> = chosen
        .iter()
        .flat_map(|unit| [0u8, 0, 0, 1].into_iter().chain(unit.iter().copied()))
        .collect();
    let pps_start: usize = chosen
        .iter()
        .take(first_vcl + 2)
        .map(|unit| unit.len() + 4)
        .sum();
    let table = demux(&stream, VideoCodec::H265)?;
    let mut starts: Vec<(usize, usize)> = table
        .samples()
        .iter()
        .map(|sample| (sample.decode_index, sample.range.start))
        .collect();
    starts.sort_unstable();
    assert_eq!(starts, [(0, 0), (1, pps_start)]);
    Ok(())
}

/// Annex A's CRC_32: polynomial 0x04C11DB7, registers starting at 1, bits
/// most significant first, over the section up to its CRC_32 field.
fn crc32_mpeg(bytes: &[u8]) -> u32 {
    let mut register = u32::MAX;
    for byte in bytes {
        register ^= u32::from(*byte) << 24;
        for _ in 0..8 {
            register = if register & 0x8000_0000 != 0 {
                (register << 1) ^ 0x04C1_1DB7
            } else {
                register << 1
            };
        }
    }
    register
}

/// Rec. ITU-T H.222.0 (08/2023) 2.4.4.6: a PAT with current_next_indicator 0
/// is not yet applicable. With the indicator cleared in every PAT of the clip,
/// and each CRC_32 recomputed, no program is ever current, so there is no
/// video stream to read. In each PAT packet, with no adaptation field and a
/// pointer_field of 0, the indicator is the low bit of section byte 5, at
/// packet byte 4 + 1 + 5, and the 13-byte section's CRC_32 ends at byte 21.
#[test]
fn a_table_not_yet_current_is_ignored() -> TestResult {
    let mut input = fixture("h264_gop12.m2t")?;
    let pats: Vec<usize> = ts_packets(&input)?
        .iter()
        .enumerate()
        .filter(|(_, (pid, _))| *pid == 0)
        .map(|(number, _)| number * 188)
        .collect();
    assert!(!pats.is_empty());
    for at in pats {
        assert_eq!(byte(&input, at + 3)? & 0x30, 0x10, "payload only");
        assert_eq!(byte(&input, at + 4)?, 0, "pointer_field 0");
        let length = usize::from(be16(&input, at + 6)? & 0x0FFF);
        assert_eq!(length, 13, "one program");
        if let Some(flags) = input.get_mut(at + 10) {
            *flags &= 0xFE;
        }
        let crc = crc32_mpeg(bytes_at(&input, at + 5, 3 + length - 4)?);
        for (offset, value) in crc.to_be_bytes().iter().enumerate() {
            if let Some(slot) = input.get_mut(at + 5 + 3 + length - 4 + offset) {
                *slot = *value;
            }
        }
    }
    assert_eq!(
        demux(&input, VideoCodec::H264).err(),
        Some(VideoError::NoVideoStream)
    );
    Ok(())
}

/// Review pass 2 D1, Rec. ITU-T H.264 (08/2024) 7.4.1.2.1: a PPS may arrive
/// before the SPS it names, because each is required only before its
/// activation. `h264_gop12_pps_first.h264` is `h264_gop12.h264` with its first
/// SPS and PPS exchanged, and gives the AC3 table.
#[test]
fn a_pps_before_its_sps_gives_the_mp4_table() -> TestResult {
    let reference = fixture("h264_gop12.mp4")?;
    let input = fixture("h264_gop12_pps_first.h264")?;
    let table = assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::AnnexB)?;
    assert_eq!(table.samples().len(), 60);
    Ok(())
}

/// Review pass 2 S1: x264 High 10 at QP 1 writes pic_init_qp_minus26 -37,
/// below the -26 an 8-bit SPS allows and inside the -38 that 7.4.2.2 gives a
/// 10-bit one. A real encoder's stream crosses the bit-depth-dependent bound
/// and is accepted, as Annex B and against its own MP4 table.
#[test]
fn a_10_bit_low_qp_stream_is_accepted() -> TestResult {
    let reference = fixture("h264_high10_qp1.mp4")?;
    let input = fixture("h264_high10_qp1.h264")?;
    let table = assert_same_table(&reference, &input, VideoCodec::H264, VideoContainer::AnnexB)?;
    assert_eq!(table.samples().len(), 4);
    assert!(
        table
            .codec_config()
            .codec_string
            .as_deref()
            .is_some_and(|text| text.starts_with("avc1.6E"))
    );
    Ok(())
}

/// A visual sample entry is 78 fixed bytes before its child boxes, QuickTime
/// File Format "Video sample description". An avc1 entry cut to 40 bytes is
/// refused as ending early, not as missing its avcC box.
#[test]
fn a_visual_sample_entry_cut_short_is_refused() -> TestResult {
    let input = fixture("h264_gop12.mp4")?;
    let top = boxes(&input, 0, input.len())?;
    let moov = *top.last().ok_or("no boxes")?;
    let mut nodes = parse_nodes(&input, moov.body - 8, moov.end)?;
    assert!(cut_container(&mut nodes, b"avc1", 40), "an avc1 entry");
    let prefix = bytes_at(&input, 0, moov.body - 8)?;
    let cut = [prefix.to_vec(), serialise(&nodes)?].concat();
    assert_eq!(
        demux(&cut, VideoCodec::H264).err(),
        Some(VideoError::Truncated {
            structure: "MP4 visual sample entry"
        })
    );
    Ok(())
}

/// Replaces the first container box of type `kind` with a leaf holding its
/// first `keep` body bytes.
fn cut_container(nodes: &mut [Node], kind: &[u8; 4], keep: usize) -> bool {
    for node in nodes.iter_mut() {
        if let Node::Container(found, prefix, children) = node {
            if found == kind {
                let body = [prefix.clone(), serialise(children).unwrap_or_default()].concat();
                *node = Node::Leaf(*kind, body.into_iter().take(keep).collect());
                return true;
            }
            if cut_container(children, kind, keep) {
                return true;
            }
        }
    }
    false
}
