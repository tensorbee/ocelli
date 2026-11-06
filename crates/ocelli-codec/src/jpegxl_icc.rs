//! Preparation of a JPEG XL source before `jxl-oxide` parses any of it.
//!
//! Every codestream whose header declares an embedded ICC profile refuses
//! with [`CodecError::UnsupportedPixelFormat`] (AC13), because Ocelli
//! implements no ICC. Every codestream whose header declares extra channels
//! refuses with the same kind, because [`crate::jpegxl::validate_header`]
//! refuses them later anyway and the pre-check refusal is what keeps the
//! decode off a header whose length grows with its own declarations. Left to
//! the dependency, a stream whose ICC entropy data does not decode is refused
//! [`CodecError::InvalidCodestream`], which reports a corrupt file rather
//! than an unavailable feature, and a stream that declares absurd encoded
//! sizes is worse: `jxl-color` 0.11.0 `icc/decode.rs` resizes its encoded ICC
//! buffer to the codestream's own `enc_size` field, up to `1 << 28` bytes,
//! before it reads the buffer, and that resize is outside `jxl-grid`'s
//! `AllocTracker`. A 26 byte codestream declaring `1 << 28` bytes measured a
//! peak resident set of 270811136 bytes on 2026-11-06 through an unguarded
//! `try_init`, and so did its 29, 46 and 56 byte variants.
//!
//! A container is stripped to its codestream before `jxl-oxide` sees any of
//! it. `jxl-oxide` 0.12.6 `aux_box.rs:72` inflates a `brob` box into an
//! unbounded `Vec` through `brotli_decompressor`, outside the `AllocTracker`
//! that the module documentation of [`crate::jpegxl`] cites as the bound, and
//! `jxl-jbr` 0.2.1 `lib.rs:59` does the same for a `jbrd` box. A container of
//! 1129 bytes whose `brob` box carries 841 bytes of Brotli naming 1 GiB of
//! zeros measured a peak resident set of 1095647232 bytes on 2026-11-06
//! through [`crate::jpegxl::JpegXlDecoder::decode`], and it decoded. So
//! [`prepare`] collects the `jxlc` payload or the `jxlp` payloads in index
//! order and hands `jxl-oxide` that codestream alone. No other box, `brob`,
//! `jbrd`, `Exif`, `xml `, `jumb` or any other, is copied or parsed. A
//! container with no codestream, two `jxlc` boxes, a `jxlc` beside `jxlp`
//! boxes, `jxlp` boxes out of order, or `jxlp` boxes without the last flag is
//! refused before the decode.
//!
//! The header is read by the dependency's own parser, in the order
//! `jxl-oxide` itself reads it, so there is no hand-written walk over the
//! header bits to desynchronise from the parser it protects. The box walk
//! above is byte aligned and fails closed. [`jxl_oxide::ImageHeader`] parses
//! the header, the same parse `try_init` runs, and it reads no ICC byte. On
//! the crafted streams above, a probe that parsed the header and then read
//! the encoded ICC size behind the flag peaked at 2260992 bytes of resident
//! set on 2026-11-06, against 270811136 for the unguarded `try_init` on the
//! same inputs. The probe here does the parse and stops at the flag.
//!
//! Allocation bounds, measured rather than asserted. The extracted
//! codestream of a container is one exact allocation of the codestream's own
//! length, at most the input length, and a bare codestream is borrowed with
//! no copy at all. The header probes allocate nothing. Each probe slices a
//! window of the codestream and each later window doubles, so the probe work
//! is linear in the header length. One constant bound sits outside
//! `AllocTracker` and outside this module's control: `jxl-image` 0.13.0
//! grows the extra channel vector and each channel name while it parses, at
//! most 4096 entries of a name of 1071 bytes, and a 540 byte header
//! declaring 4096 channels measured a 262144 byte block from it before the
//! dependency refused `num_extra too large`. That is constant bounded by the
//! format's own limits, and the refusal kinds it produces are the ones the
//! decode returns.
//!
//! The check fails closed. A header it cannot read to the ICC flag and the
//! extra channel count is refused with [`CodecError::InvalidCodestream`]
//! before any parsing, the same refusal `read_header` returns for a truncated
//! header, including when a container box's declared size outruns the input
//! and cuts the header short. Bytes of a container after the last complete
//! box are not part of the codestream and are refused as
//! [`CodecError::TrailingData`] after the decode, the same rule a bare
//! codestream is under.

use std::borrow::Cow;

use jxl_bitstream::Bitstream;
use jxl_oxide::ImageHeader;
use jxl_oxide_common::Bundle;

use crate::CodecError;

/// The codestream length of the first header probe, and the factor each
/// later probe grows by. Real headers end well inside the first window.
const FIRST_PROBE_BYTES: usize = 64;

/// The two byte signature of a bare JPEG XL codestream.
const CODESTREAM_SIGNATURE: [u8; 2] = [0xff, 0x0a];

/// The twelve byte signature box of a JPEG XL container.
const CONTAINER_SIGNATURE: [u8; 12] = *b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a";

/// Box types carrying codestream. Every other type is skipped.
const CODESTREAM_BOX: [u8; 4] = *b"jxlc";
const PARTIAL_CODESTREAM_BOX: [u8; 4] = *b"jxlp";

/// A source reduced to what `jxl-oxide` may parse.
pub(crate) struct Prepared<'src> {
    /// The codestream alone. Borrowed from the source for a bare codestream,
    /// copied once for a container.
    codestream: Cow<'src, [u8]>,
    /// The bytes of the source after the last complete container box, with
    /// the number of source bytes before them. `None` for a bare codestream,
    /// whose trailing bytes the decode's trailing rule already sees.
    container_tail: Option<(&'src [u8], usize)>,
}

impl<'src> Prepared<'src> {
    pub(crate) fn codestream(&self) -> &[u8] {
        &self.codestream
    }

    pub(crate) fn container_tail(&self) -> Option<(&'src [u8], usize)> {
        self.container_tail
    }
}

/// Reduces a source to its codestream and refuses a header Ocelli cannot
/// decode before `jxl-oxide` parses any of it.
///
/// [`CodecError::UnsupportedPixelFormat`] when the header declares an
/// embedded ICC profile or any extra channel. [`CodecError::InvalidCodestream`]
/// when the header cannot be read to those fields, including when a container
/// box's declared size outruns the input and cuts the header short, and when
/// the container breaks the extraction rules listed in the module
/// documentation.
///
/// # Errors
///
/// Returns the refusals described above. The refusal kind can differ from the
/// one the decode alone would return for the same bytes, which is how
/// `declared_icc_refuses_unsupported_before_any_parse` detects this call's
/// removal.
pub(crate) fn prepare(src: &[u8]) -> Result<Prepared<'_>, CodecError> {
    let (codestream, container_tail) = if src.starts_with(&CODESTREAM_SIGNATURE) {
        (Cow::Borrowed(src), None)
    } else if src.starts_with(&CONTAINER_SIGNATURE) {
        let mut total = 0usize;
        let consumed = walk_container(src, |chunk| {
            total = total.saturating_add(chunk.len());
            Ok(())
        })?;
        let mut codestream = Vec::new();
        codestream
            .try_reserve_exact(total)
            .map_err(|_| CodecError::DecoderFailure)?;
        walk_container(src, |chunk| {
            codestream.extend_from_slice(chunk);
            Ok(())
        })?;
        let tail = src.get(consumed..).ok_or(CodecError::DecoderFailure)?;
        (Cow::Owned(codestream), Some((tail, consumed)))
    } else {
        return Err(CodecError::InvalidCodestream);
    };
    check_header(&codestream)?;
    Ok(Prepared {
        codestream,
        container_tail,
    })
}

/// Collects the codestream chunks of a container and validates its structure.
///
/// Returns the number of source bytes the walk consumed, which is the input
/// length unless the input ends inside a box header. The sink receives each
/// codestream chunk in order. Every allocation the caller makes from the
/// chunks is bounded by the input length, because each chunk is a slice of
/// the input and the chunks never overlap.
fn walk_container<'a>(
    src: &'a [u8],
    mut sink: impl FnMut(&'a [u8]) -> Result<(), CodecError>,
) -> Result<usize, CodecError> {
    let mut pos = CONTAINER_SIGNATURE.len();
    let mut seen_codestream_box = false;
    let mut seen_partial_box = false;
    let mut partial_finished = false;
    let mut next_partial_index = 0u32;
    let mut any_codestream = false;
    while pos < src.len() {
        let rest = src.get(pos..).ok_or(CodecError::DecoderFailure)?;
        let Some(head) = rest.get(..8) else {
            // Not enough input for a box header. The dependency holds the
            // same bytes and waits for more, and there is no more, so the
            // remainder is the container tail.
            break;
        };
        // Copied and destructured rather than indexed, because the workspace
        // runs clippy with -D warnings and indexing_slicing is a warning.
        let head: [u8; 8] = head.try_into().map_err(|_| CodecError::DecoderFailure)?;
        let [s0, s1, s2, s3, t0, t1, t2, t3] = head;
        let declared_size = u32::from_be_bytes([s0, s1, s2, s3]);
        let box_type = [t0, t1, t2, t3];
        let (header_len, declared): (usize, Option<u64>) = if declared_size == 1 {
            // Extended size: the type is followed by a 64 bit size covering
            // the whole box.
            let Some(xl) = rest.get(8..16) else {
                break;
            };
            let xl: [u8; 8] = xl.try_into().map_err(|_| CodecError::DecoderFailure)?;
            let whole = u64::from_be_bytes(xl);
            let payload = whole.checked_sub(16).ok_or(CodecError::InvalidCodestream)?;
            (16, Some(payload))
        } else if declared_size == 0 {
            // The box runs to the end of the input.
            (8, None)
        } else {
            let payload = u64::from(declared_size)
                .checked_sub(8)
                .ok_or(CodecError::InvalidCodestream)?;
            (8, Some(payload))
        };
        let available = rest.len() - header_len;
        let payload_len = match declared {
            None => available,
            // A size past the input truncates the box, as the dependency's
            // own reader truncates it. The walk ends at the input.
            Some(n) => usize::try_from(n).unwrap_or(available).min(available),
        };
        let payload = rest
            .get(header_len..header_len + payload_len)
            .ok_or(CodecError::DecoderFailure)?;
        match box_type {
            CODESTREAM_BOX => {
                if seen_codestream_box {
                    // Two jxlc boxes.
                    return Err(CodecError::InvalidCodestream);
                }
                if seen_partial_box {
                    // A jxlc beside jxlp boxes.
                    return Err(CodecError::InvalidCodestream);
                }
                seen_codestream_box = true;
                any_codestream = true;
                sink(payload)?;
            }
            PARTIAL_CODESTREAM_BOX => {
                if seen_codestream_box {
                    // A jxlp beside a jxlc box.
                    return Err(CodecError::InvalidCodestream);
                }
                if partial_finished {
                    // A jxlp after the final one.
                    return Err(CodecError::InvalidCodestream);
                }
                let (index_bytes, chunk) = payload.split_at_checked(4).ok_or(
                    // Too short to carry its index.
                    CodecError::InvalidCodestream,
                )?;
                let index_bytes: [u8; 4] = index_bytes
                    .try_into()
                    .map_err(|_| CodecError::DecoderFailure)?;
                let raw = u32::from_be_bytes(index_bytes);
                let is_last = raw & 0x8000_0000 != 0;
                let index = raw & 0x7FFF_FFFF;
                if index != next_partial_index {
                    // Out of order.
                    return Err(CodecError::InvalidCodestream);
                }
                next_partial_index = next_partial_index
                    .checked_add(1)
                    .ok_or(CodecError::DecoderFailure)?;
                if is_last {
                    partial_finished = true;
                }
                seen_partial_box = true;
                any_codestream = true;
                sink(chunk)?;
            }
            // brob, jbrd, Exif, xml , jumb, ftyp, jxll and every unknown
            // type are skipped whole. None of their bytes are copied and
            // none reach the sink.
            _ => {}
        }
        pos += header_len + payload_len;
    }
    if !any_codestream {
        // A container with no codestream.
        return Err(CodecError::InvalidCodestream);
    }
    if seen_partial_box && !partial_finished {
        // jxlp boxes without the last flag.
        return Err(CodecError::InvalidCodestream);
    }
    Ok(pos)
}

/// Why a header probe did not reach the fields the refusal needs.
enum Probe {
    /// The header parsed with no ICC profile and no extra channel.
    Clear,
    /// The header parsed and declares one of the two.
    Refuse,
    /// The input ended inside the header, and more codestream may complete it.
    NeedMoreHeader,
    /// The header is definitively invalid, whatever follows it.
    UnreadableHeader,
}

/// Refuses a header that declares an ICC profile or extra channels.
///
/// Each probe doubles the window it reads, so the total probe work is linear
/// in the header length. A parse over a window that ends inside the header is
/// [`Probe::NeedMoreHeader`], because a parse error in a prefix is always an
/// input shortage: the parser reads forward only, so a defect in the bits it
/// did read is reported whatever follows. When the window covers the whole
/// codestream and the header is still incomplete, the input is exhausted and
/// the refusal is [`CodecError::InvalidCodestream`].
fn check_header(codestream: &[u8]) -> Result<(), CodecError> {
    let mut window = FIRST_PROBE_BYTES;
    loop {
        match probe(codestream, window) {
            Probe::Clear => return Ok(()),
            Probe::Refuse => return Err(CodecError::UnsupportedPixelFormat),
            Probe::NeedMoreHeader => {
                if window >= codestream.len() {
                    return Err(CodecError::InvalidCodestream);
                }
                window = window.saturating_mul(2);
            }
            Probe::UnreadableHeader => return Err(CodecError::InvalidCodestream),
        }
    }
}

/// Parses a header over the first `limit` bytes of the codestream.
fn probe(codestream: &[u8], limit: usize) -> Probe {
    let window = limit.min(codestream.len());
    let Some(prefix) = codestream.get(..window) else {
        return Probe::NeedMoreHeader;
    };
    let mut bitstream = Bitstream::new(prefix);
    let header = match <ImageHeader as Bundle<()>>::parse(&mut bitstream, ()) {
        Ok(header) => header,
        Err(error) => {
            return if error.unexpected_eof() {
                Probe::NeedMoreHeader
            } else {
                Probe::UnreadableHeader
            };
        }
    };
    if header.metadata.colour_encoding.want_icc() || !header.metadata.ec_info.is_empty() {
        Probe::Refuse
    } else {
        Probe::Clear
    }
}

#[cfg(test)]
mod tests {
    use super::prepare;
    use crate::CodecError;

    #[test]
    fn unreadable_input_refuses_before_parse() {
        // Fails closed. The empty input and a non JPEG XL signature are
        // refused rather than passed to the dependency, which would reach the
        // same refusal after feeding the bytes one at a time.
        assert!(matches!(prepare(b""), Err(CodecError::InvalidCodestream)));
        assert!(matches!(
            prepare(b"not a jxl stream at all"),
            Err(CodecError::InvalidCodestream)
        ));
    }

    #[test]
    fn container_signature_needs_a_codestream() {
        // The signature box alone is a container with no codestream.
        let src = *b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a";
        assert!(matches!(prepare(&src), Err(CodecError::InvalidCodestream)));
    }
}
