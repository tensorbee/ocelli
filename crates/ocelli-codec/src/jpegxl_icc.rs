//! Refusal of embedded ICC profiles before `jxl-oxide` parses a codestream.
//!
//! Every codestream whose header declares an embedded ICC profile refuses
//! with [`CodecError::UnsupportedPixelFormat`] (AC13), because Ocelli
//! implements no ICC. [`check_before_parse`] issues that refusal before
//! `jxl-oxide` sees a byte. Left to the dependency, a stream whose ICC
//! entropy data does not decode is refused [`CodecError::InvalidCodestream`],
//! which reports a corrupt file rather than an unavailable feature, and a
//! stream that declares absurd encoded sizes is worse: `jxl-color` 0.11.0
//! `icc/decode.rs` resizes its encoded ICC buffer to the codestream's own
//! `enc_size` field, up to `1 << 28` bytes, before it reads the buffer, and
//! that resize is outside `jxl-grid`'s `AllocTracker`. A 26 byte codestream
//! declaring `1 << 28` bytes measured a peak resident set of 270811136 bytes
//! on 2026-11-06 through an unguarded `try_init`, and so did its 29, 46 and
//! 56 byte variants.
//!
//! The header is read by the dependency's own parser, in the order
//! `jxl-oxide` itself reads it, so there is no hand-written walk here to
//! desynchronise from the parser it protects. [`jxl_bitstream::ContainerParser`]
//! collects the codestream bytes a container carries, the same
//! `ParseEvent::Codestream` events `UninitializedJxlImage::feed_bytes`
//! collects into the buffer `try_init` parses. [`jxl_oxide::ImageHeader`]
//! parses the header, the same parse `try_init` runs, and it reads no ICC
//! byte: parsing the header and reading `enc_size` on the crafted streams
//! above peaked at 2260992 bytes of resident set on 2026-11-06, against
//! 270811136 for the unguarded `try_init` on the same inputs.
//!
//! The check fails closed. A header it cannot read to the ICC flag is
//! refused with [`CodecError::InvalidCodestream`] before any parsing, the
//! same refusal `read_header` returns for a truncated header, including when
//! a container box's declared size outruns the input and cuts the header
//! short. Collection stops once the header has been read, so the collected
//! prefix is bounded by the header length rather than by the input, and each
//! probe doubles the window it will read, so the total probe work is linear
//! in the header length. The one probe over everything collected happens
//! only when the input is exhausted short of a decision, which is the
//! refusal path.

use jxl_bitstream::{Bitstream, ContainerParser, ParseEvent};
use jxl_oxide::ImageHeader;
use jxl_oxide_common::Bundle;

use crate::CodecError;

/// The codestream length of the first header probe, and the factor each
/// later probe grows by. Real headers end well inside the first window.
const FIRST_PROBE_BYTES: usize = 64;

/// Refuses a codestream before `jxl-oxide` parses it.
///
/// [`CodecError::UnsupportedPixelFormat`] when the header declares an
/// embedded ICC profile. [`CodecError::InvalidCodestream`] when the header
/// cannot be read to the ICC flag, including when a container box's declared
/// size outruns the input and cuts the header short, and when the container
/// structure itself is invalid. A header that reads with the flag clear
/// returns `Ok(())`, and the decode proceeds unchanged.
///
/// # Errors
///
/// Returns the refusal described above. The refusal kinds are the ones the
/// decode itself returns, so only the time of the refusal changes, never the
/// policy.
pub fn check_before_parse(src: &[u8]) -> Result<(), CodecError> {
    let mut parser = ContainerParser::new();
    let mut slices: Vec<&[u8]> = Vec::new();
    let mut total = 0usize;
    let mut probe_at = FIRST_PROBE_BYTES;
    let mut decided: Option<bool> = None;
    for event in parser.feed_bytes(src) {
        let payload = match event.map_err(|_| CodecError::InvalidCodestream)? {
            ParseEvent::Codestream(payload) => payload,
            _ => continue,
        };
        total = total.saturating_add(payload.len());
        slices.push(payload);
        if total >= probe_at {
            match probe(&slices, probe_at, total) {
                Ok(want_icc) => {
                    decided = Some(want_icc);
                    break;
                }
                Err(ProbeError::NeedMoreHeader) => probe_at = probe_at.saturating_mul(2),
                Err(ProbeError::UnreadableHeader) => return Err(CodecError::InvalidCodestream),
            }
        }
    }
    let want_icc = match decided {
        Some(want_icc) => want_icc,
        None => probe(&slices, usize::MAX, total).map_err(|_| CodecError::InvalidCodestream)?,
    };
    if want_icc {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    Ok(())
}

/// Why a header probe did not reach the ICC flag.
enum ProbeError {
    /// The input ended inside the header, and more codestream may complete it.
    NeedMoreHeader,
    /// The header is definitively invalid, whatever follows it.
    UnreadableHeader,
}

/// Parses a header over the first `limit` bytes of the collected codestream.
///
/// An incomplete parse is [`ProbeError::NeedMoreHeader`], because a parse
/// error in a prefix is always an input shortage: the parser reads forward
/// only, so a defect in the bits it did read is reported whatever follows.
fn probe(slices: &[&[u8]], limit: usize, total: usize) -> Result<bool, ProbeError> {
    let mut buf = Vec::with_capacity(limit.min(total));
    for slice in slices {
        if buf.len() >= limit {
            break;
        }
        let take = limit.saturating_sub(buf.len()).min(slice.len());
        let Some(prefix) = slice.get(..take) else {
            // Unreachable while `take` is at most the slice length. A short
            // window here fails closed as a further `NeedMoreHeader`.
            break;
        };
        buf.extend_from_slice(prefix);
    }
    let mut bitstream = Bitstream::new(&buf);
    let header = <ImageHeader as Bundle<()>>::parse(&mut bitstream, ()).map_err(|error| {
        if error.unexpected_eof() {
            ProbeError::NeedMoreHeader
        } else {
            ProbeError::UnreadableHeader
        }
    })?;
    Ok(header.metadata.colour_encoding.want_icc())
}

#[cfg(test)]
mod tests {
    use super::check_before_parse;
    use crate::CodecError;

    #[test]
    fn unreadable_input_refuses_before_parse() {
        // Fails closed. The empty input and a non JPEG XL signature are
        // refused rather than passed to the dependency, which would reach the
        // same refusal after feeding the bytes one at a time.
        assert_eq!(check_before_parse(b""), Err(CodecError::InvalidCodestream));
        assert_eq!(
            check_before_parse(b"not a jxl stream at all"),
            Err(CodecError::InvalidCodestream)
        );
    }
}
