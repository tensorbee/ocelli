//! The shared table builder: decode ordered samples in, a presentation
//! ordered table out.

use std::{num::NonZeroU32, ops::Range};

use super::{ColourDescription, FrameSize, Sample, VideoError};

/// PS3.5 8.2.7, at most 2^31 - 1 frames.
pub(crate) const MAX_SAMPLES: usize = 0x7FFF_FFFF;

/// One access unit of an elementary stream, as a codec splitter finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccessUnit {
    /// Its bytes in the elementary stream.
    pub(crate) range: Range<usize>,
    /// Where the access unit commences for Rec. ITU-T H.222.0 (08/2023)
    /// 2.4.3.7, which gives a transport stream PTS to the first access unit
    /// commencing in its PES packet: the 0x000001 prefix of its first NAL
    /// unit for H.264 and HEVC, and of its picture start code for MPEG-2.
    /// Zero bytes before the prefix are not counted, because a
    /// trailing_zero_8bits byte may end the previous PES packet.
    pub(crate) anchor: usize,
    /// The output position the codec gives it, a segment that output order
    /// never crosses and a count within it.
    pub(crate) order: (u32, i64),
    pub(crate) random_access: bool,
}

/// What a codec splitter reads from the stream's own headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamInfo {
    pub(crate) codec_string: Option<String>,
    pub(crate) coded_size: FrameSize,
    pub(crate) visible_size: FrameSize,
    pub(crate) colour: Option<ColourDescription>,
    /// Whether `colour` holds code points the stream signalled, rather than
    /// a range alone with the code points inferred unspecified. `colour` is
    /// present only when the stream signals a range.
    pub(crate) code_points_signalled: bool,
    pub(crate) interlaced: bool,
}

/// One sample in decode order, with the key that orders it for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decoded {
    pub(crate) range: Range<usize>,
    /// The presentation order key. A container time is `(0, pts)`.
    pub(crate) key: (u32, i64),
    /// The presentation time, or None when the stream carries none and the
    /// table counts frames instead.
    pub(crate) pts: Option<i64>,
    pub(crate) random_access: bool,
}

/// Sorts decode ordered samples into presentation order.
///
/// Two samples with one key have no defined order, so the table refuses
/// them rather than choosing one. A stream without times gets each sample's
/// presentation position as its pts.
pub(crate) fn presentation_order(
    decoded: Vec<Decoded>,
    timescale: Option<NonZeroU32>,
) -> Result<(Vec<Sample>, Option<NonZeroU32>), VideoError> {
    if decoded.is_empty() {
        return Err(VideoError::Empty);
    }
    if decoded.len() > MAX_SAMPLES {
        return Err(VideoError::TooManySamples);
    }
    let mut indexed: Vec<(usize, Decoded)> = decoded.into_iter().enumerate().collect();
    indexed.sort_by_key(|(_, sample)| sample.key);
    if let Some(pair) = indexed.windows(2).find(|pair| match pair {
        [a, b] => a.1.key == b.1.key,
        _ => false,
    }) && let Some((_, first)) = pair.first()
    {
        return Err(VideoError::DuplicatePresentationTime { pts: first.key.1 });
    }
    let mut samples = Vec::with_capacity(indexed.len());
    for (position, (decode_index, sample)) in indexed.into_iter().enumerate() {
        let pts = match sample.pts {
            Some(pts) => pts,
            None => i64::try_from(position).map_err(|_| VideoError::TooManySamples)?,
        };
        samples.push(Sample {
            range: sample.range,
            pts,
            decode_index,
            random_access: sample.random_access,
        });
    }
    Ok((samples, timescale))
}
