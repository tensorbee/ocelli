//! Release-only `decode.frame` benchmark subject.

use std::{error::Error, hint::black_box, time::Instant};

use ocelli_codec::{
    Decoder, FrameDesc, FrameDescInput, JpegDecoder, PixelDataVr, PixelRepresentation,
};

const ITERATIONS: usize = 31;
const WARMUP_ITERATIONS: usize = 1;

fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(debug_assertions) {
        return Err("decode.frame must be built with the release profile".into());
    }

    let source = include_bytes!("../tests/fixtures/jpeg_extended_12bit.jpg");
    let desc = FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 12,
        high_bit: 11,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let decoder = JpegDecoder::extended();
    let mut out = vec![0_u8; desc.output_len()];

    for _ in 0..WARMUP_ITERATIONS {
        decoder.decode(black_box(source), black_box(&desc), black_box(&mut out))?;
    }

    let mut durations = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        decoder.decode(black_box(source), black_box(&desc), black_box(&mut out))?;
        durations.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    durations.sort_by(f64::total_cmp);
    let median_ms = *durations
        .get(ITERATIONS / 2)
        .ok_or("decode.frame produced no median")?;
    let minimum_ms = *durations
        .first()
        .ok_or("decode.frame produced no minimum")?;
    let maximum_ms = *durations.last().ok_or("decode.frame produced no maximum")?;
    let checksum = out
        .iter()
        .fold(0_u64, |sum, byte| sum.wrapping_add(u64::from(*byte)));

    println!(
        "{{\"value\":{median_ms:.4},\"iterations\":{ITERATIONS},\"warmup_iterations\":{WARMUP_ITERATIONS},\"range_ms\":[{minimum_ms:.4},{maximum_ms:.4}],\"checksum\":{checksum}}}"
    );
    Ok(())
}
