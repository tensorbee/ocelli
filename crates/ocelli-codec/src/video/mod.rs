//! Video stream demuxing for the DICOM video transfer syntaxes, F-241.
//!
//! PS3.5 8.2.5 to 8.2.11 put an MPEG-2, H.264 or HEVC stream in a container:
//! MP4 or MPEG-TS for H.264 and HEVC, and for MPEG-2 also a bare elementary
//! stream, a program stream or a bare PES. [`demux`] reads MP4, MPEG-TS, an
//! Annex B byte stream and an MPEG-2 elementary stream into one
//! [`SampleTable`], a sample per displayed frame in presentation order. It
//! refuses a program stream and a bare PES by name, because F-242 adds them.
//!
//! Frame N of a video is `samples()[N]`, the N-th sample in presentation
//! order. It is never a time multiplied by a frame rate.

use std::{error::Error, fmt, num::NonZeroU32, ops::Range};

mod annexb;
mod bits;
mod h264;
mod h265;
mod mp4;
mod mpeg2;
mod table;
mod ts;

/// The codec a DICOM transfer syntax names, PS3.6 Table A-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    /// Rec. ITU-T H.262, MPEG-2 video.
    Mpeg2,
    /// Rec. ITU-T H.264, MPEG-4 AVC.
    H264,
    /// Rec. ITU-T H.265, HEVC.
    H265,
}

/// The container a stream was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoContainer {
    /// An ISO base media file, MP4.
    Mp4,
    /// An MPEG-2 transport stream, Rec. ITU-T H.222.0.
    MpegTs,
    /// An H.264 or HEVC byte stream, Annex B of Rec. ITU-T H.264 or H.265.
    AnnexB,
    /// A bare MPEG-2 video elementary stream, Rec. ITU-T H.262.
    Mpeg2Es,
}

/// How the NAL units or start codes of one sample are framed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    /// Each NAL unit follows a big-endian length field of this many bytes,
    /// the MP4 form a WebCodecs `description` record goes with.
    LengthPrefixed {
        /// The length field size in bytes, 1, 2 or 4.
        length_size: u8,
    },
    /// Start code prefixed units, an Annex B byte stream or MPEG-2 video.
    StartCodes,
}

/// A picture size in luma samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameSize {
    /// Width in luma samples.
    pub width: u32,
    /// Height in luma samples.
    pub height: u32,
}

/// Where a colour description was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColourSource {
    /// The VUI of the H.264 or HEVC sequence parameter set.
    Vui,
    /// The sequence display extension of an MPEG-2 stream.
    SequenceDisplayExtension,
    /// The `colr` box of the MP4 sample entry.
    ColrBox,
}

/// A colour description as the stream or container signals it, the code
/// points of Rec. ITU-T H.273. The demuxer reports the values and chooses
/// nothing. The conversion selects the matrix and range from them.
///
/// The precedence is per field, because PS3.5 8.2.8 gives the stream's own
/// characteristics control. The range is the stream's whenever the stream
/// signals one, and the code points are the stream's when it signals them and
/// the `colr` box's otherwise, so the two fields can come from two places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColourDescription {
    /// Where the three code points were read.
    pub code_point_source: ColourSource,
    /// Where `full_range` was read.
    pub range_source: ColourSource,
    /// colour_primaries.
    pub colour_primaries: u16,
    /// transfer_characteristics.
    pub transfer_characteristics: u16,
    /// matrix_coefficients.
    pub matrix_coefficients: u16,
    /// video_full_range_flag, or the colr box full range flag. MPEG-2 has no
    /// such flag, so it is false there.
    pub full_range: bool,
}

/// What a decoder needs to be configured, and what the inspector names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecConfig {
    /// The WebCodecs codec string. None for MPEG-2, which has no WebCodecs
    /// registration.
    pub codec_string: Option<String>,
    /// The avcC or hvcC record body for a WebCodecs `description`, present
    /// only for MP4, where the samples are length prefixed.
    pub description: Option<Vec<u8>>,
    /// How each sample is framed.
    pub sample_format: SampleFormat,
    /// The coded picture size, before any cropping.
    pub coded_size: FrameSize,
    /// The displayed size after the conformance or cropping window.
    pub visible_size: FrameSize,
    /// The colour description, or None when neither the stream nor the
    /// container signals one.
    pub colour: Option<ColourDescription>,
    /// Whether the container carries an audio track.
    pub has_audio: bool,
    /// Whether the stream signals interlaced coding.
    pub interlaced: bool,
}

/// One displayed frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sample {
    /// The sample's bytes, within the buffer [`SampleTable::sample_data`]
    /// chooses.
    pub range: Range<usize>,
    /// The presentation time in units of [`SampleTable::timescale`], or the
    /// frame's presentation position when the timescale is None. For MP4 it
    /// is the composition time, decode time plus composition offset, without
    /// the edit list, so it can differ from a player's time by the edit list's
    /// constant shift. For a transport stream it is the PTS, unwrapped.
    pub pts: i64,
    /// The position of this sample in decode order.
    pub decode_index: usize,
    /// Whether decoding may start at this sample.
    pub random_access: bool,
}

/// The demuxed table, a sample per displayed frame in presentation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleTable {
    container: VideoContainer,
    samples: Vec<Sample>,
    timescale: Option<NonZeroU32>,
    reassembled: Option<Vec<u8>>,
    config: CodecConfig,
}

impl SampleTable {
    /// The container the stream was read from.
    #[must_use]
    pub fn container(&self) -> VideoContainer {
        self.container
    }

    /// Every sample in presentation order. Frame N is `samples()[N]`.
    #[must_use]
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }

    /// Ticks per second of [`Sample::pts`]. None for a bare elementary
    /// stream, which carries no presentation times.
    #[must_use]
    pub fn timescale(&self) -> Option<NonZeroU32> {
        self.timescale
    }

    /// The decoder configuration.
    #[must_use]
    pub fn codec_config(&self) -> &CodecConfig {
        &self.config
    }

    /// The bytes of one sample. A transport stream's samples lie in the
    /// elementary stream reassembled from its packets, and every other
    /// container's lie in `input`, the bytes given to [`demux`].
    #[must_use]
    pub fn sample_data<'a>(&'a self, input: &'a [u8], sample: &Sample) -> Option<&'a [u8]> {
        self.reassembled
            .as_deref()
            .unwrap_or(input)
            .get(sample.range.clone())
    }
}

/// Why a stream was not demuxed. Every refusal names its reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoError {
    /// The bytes open no container this build reads.
    UnknownContainer,
    /// An MPEG-2 program stream, which PS3.5 8.2.5 allows and F-242 reads.
    ProgramStream,
    /// A bare PES stream, which PS3.5 8.2.5 allows and F-242 reads.
    BarePes,
    /// The container's stream is not the codec the transfer syntax names.
    CodecMismatch {
        /// The codec the transfer syntax names.
        expected: VideoCodec,
        /// The codec the container holds.
        found: VideoCodec,
    },
    /// The container holds no video stream.
    NoVideoStream,
    /// A structure ends before its declared length.
    Truncated {
        /// The structure that ends early.
        structure: &'static str,
    },
    /// A structure's fields contradict the specification or each other.
    Malformed {
        /// The structure.
        structure: &'static str,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A legal feature this demuxer does not read.
    Unsupported {
        /// The feature.
        feature: &'static str,
    },
    /// More samples than PS3.5 8.2.7's limit of 2^31 - 1 frames.
    TooManySamples,
    /// Two samples share one presentation time, so their order is unknown.
    DuplicatePresentationTime {
        /// The shared presentation time.
        pts: i64,
    },
    /// A transport stream access unit has no PTS of its own.
    MissingTimestamp {
        /// The access unit's position in decode order.
        decode_index: usize,
    },
    /// The stream holds no displayable sample.
    Empty,
}

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownContainer => write!(
                f,
                "the video is in no container this build reads, which are MP4, MPEG-2 transport \
                 stream, an H.264 or HEVC Annex B stream and an MPEG-2 elementary stream"
            ),
            Self::ProgramStream => write!(
                f,
                "the video is an MPEG-2 program stream, which this build does not demux, and F-242 \
                 adds it"
            ),
            Self::BarePes => write!(
                f,
                "the video is a bare MPEG-2 PES stream, which this build does not demux, and F-242 \
                 adds it"
            ),
            Self::CodecMismatch { expected, found } => write!(
                f,
                "the transfer syntax names {} video but the stream is {}",
                codec_name(*expected),
                codec_name(*found)
            ),
            Self::NoVideoStream => write!(f, "the container holds no video stream"),
            Self::Truncated { structure } => write!(f, "the {structure} ends early"),
            Self::Malformed { structure, reason } => {
                write!(f, "the {structure} does not parse: {reason}")
            }
            Self::Unsupported { feature } => write!(f, "{feature} is not read by this build"),
            Self::TooManySamples => {
                write!(
                    f,
                    "the stream has more than 2^31 - 1 frames, the PS3.5 8.2.7 limit"
                )
            }
            Self::DuplicatePresentationTime { pts } => write!(
                f,
                "two samples share the presentation time {pts}, so their order is unknown"
            ),
            Self::MissingTimestamp { decode_index } => write!(
                f,
                "access unit {decode_index} in decode order has no presentation time stamp"
            ),
            Self::Empty => write!(f, "the stream holds no frame"),
        }
    }
}

impl Error for VideoError {}

fn codec_name(codec: VideoCodec) -> &'static str {
    match codec {
        VideoCodec::Mpeg2 => "MPEG-2",
        VideoCodec::H264 => "H.264",
        VideoCodec::H265 => "HEVC",
    }
}

/// Demux one video stream into its sample table.
///
/// # Errors
///
/// Returns [`VideoError`] naming why the stream was not demuxed.
pub fn demux(input: &[u8], codec: VideoCodec) -> Result<SampleTable, VideoError> {
    match detect(input, codec)? {
        VideoContainer::Mp4 => mp4::demux(input, codec),
        VideoContainer::MpegTs => ts::demux(input, codec),
        container => annexb::demux(input, codec, container),
    }
}

/// Box types that open an MP4 file in practice: the file type box, the
/// movie and media data boxes, and the free space and extension boxes a
/// writer may put first.
const MP4_FIRST_BOXES: [&[u8; 4]; 9] = [
    b"ftyp", b"moov", b"mdat", b"free", b"skip", b"wide", b"pdin", b"uuid", b"styp",
];

/// Start code values after 0x000001 that belong to the systems layer, Rec.
/// ITU-T H.262 (02/2000) Table 6-1 "system start codes" and Rec. ITU-T H.222.0
/// (08/2023) Table 2-22: MPEG_program_end_code 0xB9, pack_start_code 0xBA
/// and system_header_start_code 0xBB open a program stream, and 0xBC and up
/// are PES stream_id values.
const PROGRAM_END: u8 = 0xB9;
const SYSTEM_HEADER: u8 = 0xBB;
const FIRST_STREAM_ID: u8 = 0xBC;
/// Rec. ITU-T H.262 (02/2000) Table 6-1, sequence_header_code, the first
/// start code of a video sequence by 6.2.2.
const SEQUENCE_HEADER: u8 = 0xB3;

fn detect(input: &[u8], codec: VideoCodec) -> Result<VideoContainer, VideoError> {
    if input
        .get(4..8)
        .is_some_and(|kind| MP4_FIRST_BOXES.iter().any(|first| first.as_slice() == kind))
    {
        return Ok(VideoContainer::Mp4);
    }
    // Rec. ITU-T H.222.0 (08/2023) 2.4.3.2: every packet opens with
    // sync_byte 0x47, so the first two packets do.
    if input.first() == Some(&ts::SYNC_BYTE)
        && input.len() >= ts::PACKET
        && (input.len() == ts::PACKET || input.get(ts::PACKET) == Some(&ts::SYNC_BYTE))
    {
        return Ok(VideoContainer::MpegTs);
    }
    // A start code stream opens with zero bytes and 0x01, at least two zeros.
    let zeros = input.iter().take_while(|byte| **byte == 0).count();
    if zeros < 2 || input.get(zeros) != Some(&1) {
        return Err(VideoError::UnknownContainer);
    }
    let value = *input.get(zeros + 1).ok_or(VideoError::UnknownContainer)?;
    match value {
        PROGRAM_END..=SYSTEM_HEADER => Err(VideoError::ProgramStream),
        FIRST_STREAM_ID..=0xFF => Err(VideoError::BarePes),
        SEQUENCE_HEADER if codec == VideoCodec::Mpeg2 => Ok(VideoContainer::Mpeg2Es),
        SEQUENCE_HEADER => Err(VideoError::CodecMismatch {
            expected: codec,
            found: VideoCodec::Mpeg2,
        }),
        // Rec. ITU-T H.264 (08/2024) 7.3.1 and Rec. ITU-T H.265 (07/2024)
        // 7.3.1.2: a NAL unit header opens with forbidden_zero_bit 0.
        header if header & 0x80 == 0 && codec != VideoCodec::Mpeg2 => Ok(VideoContainer::AnnexB),
        _ => Err(VideoError::UnknownContainer),
    }
}
