//! Release-only JPEG-LS decode benchmark subject.
//!
//! The lossless corpus row, because it is the one with an encoder-independent
//! anchor. The near-lossless row is a different amount of work and is not
//! averaged with it, which is what `decode.transfer_syntax.jpegls`'s definition
//! means by reporting per syntax.

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
    let source = include_bytes!("../tests/fixtures/jpegls_corpus_lossless.jls");
    let desc = FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 16,
        high_bit: 15,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let decoder = JpegLsDecoder::lossless();
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
