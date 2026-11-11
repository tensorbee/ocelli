//! Bare start code streams: an H.264 or HEVC Annex B byte stream and an
//! MPEG-2 video elementary stream.
//!
//! Neither carries presentation times, so the table's order is the codec's
//! own output order, picture order count for H.264 and HEVC and
//! temporal_reference for MPEG-2, and each sample's pts is its presentation
//! position with no timescale.

use super::{
    CodecConfig, SampleFormat, SampleTable, VideoCodec, VideoContainer, VideoError, h264, h265,
    mpeg2,
    table::{Decoded, presentation_order},
};

pub(crate) fn demux(
    data: &[u8],
    codec: VideoCodec,
    container: VideoContainer,
) -> Result<SampleTable, VideoError> {
    let (units, info) = match codec {
        VideoCodec::H264 => h264::access_units(data)?,
        VideoCodec::H265 => h265::access_units(data)?,
        VideoCodec::Mpeg2 => mpeg2::access_units(data)?,
    };
    let decoded = units
        .into_iter()
        .map(|unit| Decoded {
            range: unit.range,
            key: unit.order,
            pts: None,
            random_access: unit.random_access,
        })
        .collect();
    let (samples, timescale) = presentation_order(decoded, None)?;
    Ok(SampleTable {
        container,
        samples,
        timescale,
        reassembled: None,
        config: CodecConfig {
            codec_string: info.codec_string,
            description: None,
            sample_format: SampleFormat::StartCodes,
            coded_size: info.coded_size,
            visible_size: info.visible_size,
            colour: info.colour,
            has_audio: false,
            interlaced: info.interlaced,
        },
    })
}
