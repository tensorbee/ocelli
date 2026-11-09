//! Release-only JPEG-LS decode benchmark subject.
//!
//! The lossless corpus row, because it is the one with an encoder-independent
//! anchor. The near-lossless row is a different amount of work and is not
//! averaged with it, which is what `decode.transfer_syntax.jpegls`'s definition
//! means by reporting per syntax.
//!
//! An optional argument names another monochrome corpus row for a one-off
//! comparison, F-243: `lossless`, the default and the subject's only
//! invocation, `near_lossless`, `banded_lossless` or `banded_near_lossless`.

use std::{error::Error, hint::black_box, time::Instant};

use ocelli_codec::{
    Decoder, FrameDesc, FrameDescInput, JpegLsDecoder, PixelDataVr, PixelRepresentation,
};

const ITERATIONS: usize = 31;
const DECODES_PER_SAMPLE: usize = 4;

fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(debug_assertions) {
        return Err("decode_jpegls must use the release profile".into());
    }
    let row = std::env::args().nth(1);
    let (source, rows, columns, decoder): (&[u8], u16, u16, JpegLsDecoder) =
        match row.as_deref().unwrap_or("lossless") {
            "lossless" => (
                include_bytes!("../tests/fixtures/jpegls_corpus_lossless.jls"),
                64,
                96,
                JpegLsDecoder::lossless(),
            ),
            "near_lossless" => (
                include_bytes!("../tests/fixtures/jpegls_corpus_near_lossless.jls"),
                64,
                96,
                JpegLsDecoder::near_lossless(),
            ),
            "banded_lossless" => (
                include_bytes!("../tests/fixtures/jpegls_corpus_banded_lossless.jls"),
                32,
                32,
                JpegLsDecoder::lossless(),
            ),
            "banded_near_lossless" => (
                include_bytes!("../tests/fixtures/jpegls_corpus_banded_near3.jls"),
                32,
                32,
                JpegLsDecoder::near_lossless(),
            ),
            other => return Err(format!("unknown corpus row {other}").into()),
        };
    let desc = FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 16,
        high_bit: 15,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let mut out = vec![0_u8; desc.output_len()];
    decoder.decode(black_box(source), black_box(&desc), black_box(&mut out))?;
    let mut durations = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        for _ in 0..DECODES_PER_SAMPLE {
            decoder.decode(black_box(source), black_box(&desc), black_box(&mut out))?;
        }
        durations.push(started.elapsed().as_secs_f64() * 1_000.0 / 4.0);
    }
    durations.sort_by(f64::total_cmp);
    let median = durations.get(ITERATIONS / 2).ok_or("no median")?;
    let minimum = durations.first().ok_or("no minimum")?;
    let maximum = durations.last().ok_or("no maximum")?;
    let checksum = out
        .iter()
        .fold(0_u64, |sum, byte| sum.wrapping_add(u64::from(*byte)));
    println!(
        "{{\"value\":{median:.4},\"iterations\":{ITERATIONS},\"warmup_iterations\":1,\"decodes_per_sample\":{DECODES_PER_SAMPLE},\"range_ms\":[{minimum:.4},{maximum:.4}],\"checksum\":{checksum}}}"
    );
    Ok(())
}
