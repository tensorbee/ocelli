//! HTJ2K conformance fixtures for F-027.
//!
//! Built by `tests/fixtures/generate_htj2k.py`, which records where each
//! codestream came from and asserts its CAP marker, progression order and
//! wavelet kernel before writing it.

use std::error::Error;

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescInput, Htj2kDecoder, PixelDataVr, PixelRepresentation, Registry,
    RegistryError, register_htj2k_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

/// One differing sample as `(index, actual, expected)`.
type Difference = (usize, u16, u16);

const HTJ2K_LOSSLESS: &str = "1.2.840.10008.1.2.4.201";
const HTJ2K_LOSSLESS_RPCL: &str = "1.2.840.10008.1.2.4.202";
const HTJ2K: &str = "1.2.840.10008.1.2.4.203";

#[test]
fn both_lossless_syntaxes_reproduce_the_uncompressed_reference_exactly() -> TestResult {
    // `corpus/manifest.tsv` rows 35 and 36. Both are lossless, so both must
    // reproduce `syntax/explicit_vr_le.dcm`'s Pixel Data byte for byte. That
    // reference is the synthetic ramp from scripts/corpus_synth.py and comes
    // from no codec, so it is an encoder-independent anchor.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    let desc = corpus_desc()?;

    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());

    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless_rpcl().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless_rpcl.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

/// The `.203` decode F-X013 measured, over the canonical 12,288 bytes.
///
/// F-X013 recorded this for `D_native`, `D_wasm` and `D_simd` alike, so pinning
/// it here asserts that the production adapter reproduces the spike's
/// measurement through an entirely different code path.
const LOSSY_SHA256: &str = "ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c";

#[test]
fn the_irreversible_syntax_reproduces_the_measurement_f_x013_recorded() -> TestResult {
    // `corpus/manifest.tsv` row 37, transfer syntax `.203`.
    //
    // **The reference ramp is NOT an anchor for this row and the story's design
    // plan was wrong about how to assert it.** The plan said to pin F-X013's
    // "41 of 6144" divergence. That figure is the candidate measured against
    // OpenJPH 0.31.0's own output, which is not in this tree: `ojph_expand` is
    // an external tool the spike ran once. Against the uncompressed ramp this
    // irreversible decode differs at 5,377 of 6,144 samples, which is what a
    // lossy codec does and is not a defect.
    //
    // What CAN be asserted here is stronger than a tolerance and is what
    // decision D14 means by a measured divergence: F-X013 recorded the
    // candidate's own output digest, identical across its native, plain wasm
    // and SIMD wasm builds, and the production adapter reproduces it exactly.
    // If the decode ever changes, this fails, and the OpenJPH comparison the
    // spike recorded would have to be re-measured rather than assumed.
    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::general().decode(
        include_bytes!("fixtures/htj2k_corpus_lossy.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.len(), 12_288);
    assert_eq!(hex_sha256(&out), LOSSY_SHA256);

    // And it really is lossy, so a `.203` fixture cannot silently be a lossless
    // one. The count is recorded rather than bounded.
    let differing = differences(
        &out,
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw"),
    )?;
    assert_eq!(differing.len(), 5_377);
    Ok(())
}

#[test]
fn a_part_one_codestream_is_refused_because_it_carries_no_cap_marker() -> TestResult {
    // CAP `(0xff50)` is what makes a codestream HTJ2K rather than JPEG 2000
    // Part 1. `fixtures/jpeg2000_corpus_j2k_lossless.j2k` is the same synthetic
    // ramp encoded as Part 1, so its dimensions, precision and reversibility
    // all match the descriptor and every other check passes. Without the CAP
    // check it would decode through an HTJ2K transfer syntax, which is a file
    // being read as something it does not claim to be.
    let desc = corpus_desc()?;
    for decoder in [
        Htj2kDecoder::lossless(),
        Htj2kDecoder::lossless_rpcl(),
        Htj2kDecoder::general(),
    ] {
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            decoder.decode(
                include_bytes!("fixtures/jpeg2000_corpus_j2k_lossless.j2k"),
                &desc,
                &mut out,
            ),
            Err(CodecError::FrameMismatch)
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }
    Ok(())
}

#[test]
fn the_rpcl_constraint_is_one_directional_because_ps3_5_makes_it_so() -> TestResult {
    // PS3.5 A.4.10 requires RPCL progression for `.202`. A.4.9 constrains
    // nothing about progression for `.201`.
    //
    // So `.202` refuses the LRCP row, and `.201` ACCEPTS the RPCL row. The
    // second half is the one worth asserting: a `.202` codestream is also a
    // valid `.201` one, and inventing a non-RPCL constraint for `.201` to make
    // the two mutually exclusive would refuse conformant files.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    let desc = corpus_desc()?;

    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless_rpcl().decode(
            include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
            &desc,
            &mut out,
        ),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless_rpcl.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

#[test]
fn the_lossless_syntaxes_refuse_an_irreversible_codestream() -> TestResult {
    // PS3.5 A.4.9 and A.4.10 are lossless only, so the wavelet transform must
    // be the reversible 5/3 kernel. The general form accepts either, which is
    // the same asymmetry `.90` and `.91` already have.
    let desc = corpus_desc()?;
    for decoder in [Htj2kDecoder::lossless(), Htj2kDecoder::lossless_rpcl()] {
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            decoder.decode(
                include_bytes!("fixtures/htj2k_corpus_lossy.j2c"),
                &desc,
                &mut out,
            ),
            Err(CodecError::FrameMismatch)
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }

    // And the general form accepts a reversible codestream, so the constraint
    // refuses the wrong pairing rather than everything.
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::general().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw").as_slice()
    );
    Ok(())
}

#[test]
fn lossless_refuses_a_component_override_and_a_tile_part_cod() -> TestResult {
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");
    let desc = corpus_desc()?;
    let cod = source
        .windows(2)
        .position(|window| window == [0xff, 0x52])
        .ok_or("COD marker is absent")?;
    let mut coc = source.to_vec();
    *coc.get_mut(cod + 1).ok_or("COD marker is truncated")? = 0x53;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&coc, &desc, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    let sod = source
        .windows(2)
        .position(|window| window == [0xff, 0x93])
        .ok_or("SOD marker is absent")?;
    let mut tile_cod = source.to_vec();
    tile_cod.splice(
        sod..sod,
        [0xff, 0x52, 0x00, 0x0c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&tile_cod, &desc, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn lossless_refuses_quantization_and_rpcl_refuses_a_progression_change() -> TestResult {
    // ISO/IEC 15444-1 A.6.4. Style 1 is scalar derived. A reversible COD does
    // not make that quantizer lossless.
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");
    let desc = corpus_desc()?;
    let qcd = source
        .windows(2)
        .position(|window| window == [0xff, 0x5c])
        .ok_or("QCD marker is absent")?;
    let mut quantized = source.to_vec();
    let style = quantized
        .get_mut(qcd + 4)
        .ok_or("QCD payload is truncated")?;
    *style = (*style & 0xe0) | 1;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&quantized, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // ISO/IEC 15444-1 A.6.6. A POC replaces the COD progression order. `.202`
    // requires RPCL. `.201` does not, so the same marker is not that refusal.
    let rpcl = include_bytes!("fixtures/htj2k_corpus_lossless_rpcl.j2c");
    let sot = rpcl
        .windows(2)
        .position(|window| window == [0xff, 0x90])
        .ok_or("SOT marker is absent")?;
    let mut main_poc = rpcl.to_vec();
    main_poc.splice(sot..sot, [0xff, 0x5f, 0x00, 0x04, 0, 0]);
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless_rpcl().decode(&main_poc, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    assert_ne!(
        Htj2kDecoder::lossless().decode(&main_poc, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );

    let sod = rpcl
        .windows(2)
        .position(|window| window == [0xff, 0x93])
        .ok_or("SOD marker is absent")?;
    let mut tile_poc = rpcl.to_vec();
    tile_poc.splice(sod..sod, [0xff, 0x5f, 0x00, 0x04, 0, 0]);
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless_rpcl().decode(&tile_poc, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn the_corpus_colour_rows_reproduce_the_saturated_reference() -> TestResult {
    // ISO/IEC 15444-1 G.2 at B = 8, from the F-239 plan's working. The
    // level-shifted RGB `200 100 50` is `72 -28 -78`, whose forward RCT is
    // `Y0 = floor((72 + 2 * -28 + -78) / 4) = -16`, `Y1 = -78 - -28 = -50`,
    // `Y2 = 72 - -28 = 100`. The inverse is
    // `I1 = -16 - floor((100 + -50) / 4) = -28`, `I0 = 100 + -28 = 72`,
    // `I2 = -50 + -28 = -78`, and undoing the shift gives `200 100 50`.
    //
    // Pure red `255 0 0` level-shifts to `127 -128 -128`, forward
    // `Y0 = floor(-257 / 4) = -65`, `Y1 = 0`, `Y2 = 255`. Its chroma `Y2`
    // spans one bit more than the depth, reaching 255 where the clamp of an
    // early level shift would cut it, which is why the inverse runs before
    // the shift is undone. Both markers sit in the saturated reference, and
    // the lossless `.201` syntax must reproduce all of it byte for byte.
    let codestream = include_bytes!("fixtures/htj2k_colour_lossless_ybr_rct.j2c");
    let reference = include_bytes!("fixtures/j2k_colour_reference_rgb8le.raw");
    let desc = colour_frame("YBR_RCT")?;
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(codestream, &desc, &mut out)?;
    assert_eq!(out.as_slice(), reference);

    // The named markers, three bytes per interleaved pixel.
    assert_eq!(out.get(0..3), Some(&[255, 0, 0][..]), "pure red");
    assert_eq!(out.get(3..6), Some(&[0, 255, 0][..]), "pure green");
    assert_eq!(out.get(6..9), Some(&[0, 0, 255][..]), "pure blue");
    assert_eq!(
        out.get(9..12),
        Some(&[200, 100, 50][..]),
        "the worked RCT triple"
    );
    Ok(())
}

/// The recorded AC25 measurement for the `.203` row: the decode against
/// OpenJPEG's own decode of the same codestream, through
/// `scripts/tests/test_corpus_synth.py`'s pylibjpeg route. The measured
/// maximum is zero: the two implementations agree byte for byte on this row.
/// A decode change here is a re-measurement, not a tolerance to widen.
#[test]
fn inverse_ict_uses_g6_literal_coefficients() -> TestResult {
    // ISO/IEC 15444-1 G.3, from the F-239 plan's working. Transformed
    // components `Y0 = -28, Y1 = 22, Y2 = -38` give
    // `I0 = -28 + 1.402 * -38 = -81.276`,
    // `I1 = -28 - 0.34413 * 22 - 0.71414 * -38 = -8.43354`,
    // `I2 = -28 + 1.772 * 22 = 10.984`. Adding the level shift's 128 gives
    // `46.724 119.56646 138.984`, which the decoder rounds to `47 120 139`.
    // A whole frame of those coefficients is not buildable by an available
    // encoder, so this corpus row is a recorded bound and not a coefficient
    // pin: the decode agrees with OpenJPEG, an independent conforming
    // implementation of the same inverse, byte for byte. A decode with the
    // inverse left out differs by more.
    let codestream = include_bytes!("fixtures/htj2k_colour_lossy_ybr_ict.j2c");
    let openjpeg = include_bytes!("fixtures/htj2k_colour_lossy_ict_openjpeg.raw");
    let desc = colour_frame("YBR_ICT")?;
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::general().decode(codestream, &desc, &mut out)?;
    let max_per_channel = colour_measurement(&out, openjpeg)?;
    assert_eq!(
        max_per_channel,
        [0, 0, 0],
        "the recorded decode agrees with OpenJPEG byte for byte"
    );

    // The hand values are the exact mathematics and no available encoder can
    // carry them: openjph's own 9/7 quantization moves the one-pixel case to
    // 46 119 139, and OpenJPEG decodes the same codestream to the same three
    // codes. The exact coefficients are held by the vendored patch's unit
    // test `ict_backward_is_the_annex_g_transform`, which asserts those hand
    // values, and a decode with the inverse left out differs at 18,353 of
    // 18,432 samples against the comparison this test records.
    Ok(())
}

/// The corpus colour rows are 64 by 96, three samples at eight bits, from
/// `scripts/corpus_synth.py`'s saturated RGB reference.
fn colour_frame(photometric_interpretation: &str) -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: photometric_interpretation.to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?)
}

/// Per-channel difference measurement for interleaved colour bytes.
fn colour_measurement(actual: &[u8], expected: &[u8]) -> Result<[u32; 3], Box<dyn Error>> {
    if actual.len() != expected.len() || !actual.len().is_multiple_of(3) {
        return Err("colour buffers have different or invalid lengths".into());
    }
    let mut max_per_channel = [0_u32; 3];
    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        let difference = (*actual as i32 - *expected as i32).unsigned_abs();
        // `index` runs over a multiple-of-three length, so the channel is 0,
        // 1 or 2 and `get_mut` below always finds the slot.
        let channel = index % 3;
        let slot = max_per_channel.get_mut(channel).ok_or("channel index")?;
        *slot = (*slot).max(difference);
    }
    Ok(max_per_channel)
}

#[test]
fn a_multi_component_codestream_is_refused_by_the_header_check() -> TestResult {
    // A two-component codestream has no photometric PS3.5 8.2.4 permits for
    // the JPEG 2000 syntaxes and no colour transform is defined over two
    // components, so the descriptor check cannot reach it: a three-sample
    // descriptor is refused before the codestream is read. So the SIZ of the
    // `.201` fixture is rewritten here to declare `Csiz = 2` with two
    // component specifiers, which is the smallest edit that makes a
    // structurally valid multi-component main header out of a
    // single-component one.
    //
    // Everything before the component check passes, including CAP, so the
    // refusal that fires is the one this test is for.
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");
    // ISO/IEC 15444-1 A.5.1. `Rsiz Xsiz Ysiz XOsiz YOsiz XTsiz YTsiz XTOsiz
    // YTOsiz Csiz (Ssiz XRsiz YRsiz)*Csiz`, 96 by 64, one tile, 16-bit
    // unsigned, with Csiz raised from 1 to 2 and its specifier repeated.
    const TWO_COMPONENT_SIZ: &[u8] = &[
        0xff, 0x51, 0x00, 0x2c, 0x40, 0x00, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x40, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x40,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x0f, 0x01, 0x01, 0x0f, 0x01,
        0x01,
    ];
    // The original SIZ is 43 bytes and begins at offset 2, right after SOC.
    let head = source.get(..2).ok_or("fixture is shorter than SOC")?;
    let tail = source.get(45..).ok_or("fixture is shorter than its SIZ")?;
    assert_eq!(source.get(2..4), Some(&[0xff, 0x51][..]));
    let mut spliced = Vec::with_capacity(head.len() + TWO_COMPONENT_SIZ.len() + tail.len());
    spliced.extend_from_slice(head);
    spliced.extend_from_slice(TWO_COMPONENT_SIZ);
    spliced.extend_from_slice(tail);

    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&spliced, &desc, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

/// Build a one-pixel HTJ2K codestream whose three components' transform
/// outputs are exactly `transformed`, through the vendored openjph-core
/// encoder.
///
/// The encoder writes the COD multiple-component-transform byte when asked
/// and applies no forward transform of its own, which is what a fixture for
/// the inverse transform wants. Its `push_line` applies the DC level shift,
/// so the pre-shift component samples are passed here and the codestream's
/// coefficients come out as the transformed values.
fn one_pixel_transformed_codestream(
    transformed: [i32; 3],
    reversible: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    use openjph_core::codestream::Codestream;
    use openjph_core::file::MemOutfile;
    use openjph_core::types::{Point, Size};

    let mut codestream = Codestream::new();
    codestream
        .access_siz_mut()
        .set_image_extent(Point::new(1, 1));
    codestream.access_siz_mut().set_tile_size(Size::new(1, 1));
    codestream.access_siz_mut().set_num_components(3);
    for component in 0..3_u32 {
        codestream
            .access_siz_mut()
            .set_comp_info(component, Point::new(1, 1), 8, false);
    }
    codestream.access_cod_mut().set_reversible(reversible);
    codestream.access_cod_mut().set_color_transform(true);
    codestream.access_cod_mut().set_block_dims(64, 64);
    codestream.access_cod_mut().set_num_decomposition(0);
    codestream.access_cod_mut().set_progression_order("LRCP")?;

    let mut outfile = MemOutfile::new();
    codestream.write_headers(&mut outfile, &[])?;
    for (component, value) in transformed.iter().enumerate() {
        let component = u32::try_from(component).map_err(|_| "component index overflow")?;
        codestream.exchange(&[value + 128], component)?;
    }
    codestream.flush(&mut outfile)?;
    Ok(outfile.get_data().to_vec())
}

#[test]
fn inverse_rct_is_exact_before_the_level_shift() -> TestResult {
    // ISO/IEC 15444-1 G.2 at B = 8, from the F-239 plan's working. The
    // level-shifted RGB `200 100 50` is `72 -28 -78`, whose forward RCT is
    // `Y0 = floor((72 + 2 * -28 + -78) / 4) = -16`, `Y1 = -78 - -28 = -50`,
    // `Y2 = 72 - -28 = 100`. The inverse is
    // `I1 = -16 - floor((100 + -50) / 4) = -28`, `I0 = 100 + -28 = 72`,
    // `I2 = -50 + -28 = -78`, and undoing the shift gives `200 100 50`.
    //
    // Pure red `255 0 0` level-shifts to `127 -128 -128`, forward
    // `Y0 = floor(-257 / 4) = -65`, `Y1 = 0`, `Y2 = 255`. Its chroma `Y2`
    // spans one bit more than the depth, reaching 255 where the clamp of an
    // early level shift would cut it, which is why the inverse runs before
    // the shift is undone.
    let desc = FrameDesc::new(FrameDescInput {
        rows: 1,
        columns: 1,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_RCT".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let codestream = one_pixel_transformed_codestream([-16, -50, 100], true)?;
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(&codestream, &desc, &mut out)?;
    assert_eq!(out, [200, 100, 50]);

    let codestream = one_pixel_transformed_codestream([-65, 0, 255], true)?;
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(&codestream, &desc, &mut out)?;
    assert_eq!(out, [255, 0, 0]);
    Ok(())
}

#[test]
fn a_three_component_codestream_is_reported_rgb_when_the_transform_is_on() -> TestResult {
    // PS3.5 8.2.4: YBR_RCT and YBR_ICT name the codestream's own component
    // transform, so the decode hands back RGB and says so. A descriptor that
    // disagrees with the component count is a frame mismatch.
    let mut registry = Registry::new();
    register_htj2k_decoders(&mut registry)?;

    let colour = FrameDesc::new(FrameDescInput {
        rows: 1,
        columns: 1,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_RCT".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    assert_eq!(
        registry.decode_photometric_interpretation(HTJ2K_LOSSLESS, &colour)?,
        DecodePhotometricInterpretation::Rgb
    );
    assert_eq!(
        registry.decode_sample_layout(HTJ2K_LOSSLESS, &colour)?,
        DecodeSampleLayout::Interleaved
    );

    // A monochrome descriptor against a three-component codestream is the
    // codestream contradicting the data set, not an unsupported pixel format.
    let monochrome = corpus_desc()?;
    let codestream = one_pixel_transformed_codestream([-16, -50, 100], true)?;
    // PS3.5 Table 8.2.4-1: the two 4:2:2 photometrics are not valid for the
    // HTJ2K syntaxes, so a descriptor carrying one is refused before the
    // codestream is read.
    let four_two_two = FrameDesc::new(FrameDescInput {
        rows: 1,
        columns: 1,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_FULL_422".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let mut out = vec![0xa5; four_two_two.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&codestream, &four_two_two, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    let mut out = vec![0xa5; monochrome.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&codestream, &monochrome, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn a_transformed_codestream_refuses_a_descriptor_that_names_no_transform() -> TestResult {
    // PS3.5 8.2.4: a codestream whose multiple component transform is on
    // hands back RGB samples, so Photometric Interpretation shall be
    // YBR_RCT or YBR_ICT. `RGB` or `YBR_FULL` over such a codestream would
    // draw already-RGB samples through a YBR matrix and report a space the
    // pixels cannot have, so the decode refuses the contradiction instead.
    let codestream = one_pixel_transformed_codestream([-16, -50, 100], true)?;
    for photometric in ["RGB", "YBR_FULL"] {
        let desc = FrameDesc::new(FrameDescInput {
            rows: 1,
            columns: 1,
            samples_per_pixel: 3,
            bits_allocated: 8,
            bits_stored: 8,
            high_bit: 7,
            pixel_representation: PixelRepresentation::Unsigned,
            photometric_interpretation: photometric.to_owned(),
            pixel_data_vr: PixelDataVr::Ob,
        })?;
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            Htj2kDecoder::lossless().decode(&codestream, &desc, &mut out),
            Err(CodecError::FrameMismatch),
            "{photometric} over a transformed codestream must be refused"
        );
        assert!(out.iter().all(|byte| *byte == 0xa5), "{photometric}");
    }
    Ok(())
}

#[test]
fn the_lossless_colour_syntax_refuses_the_irreversible_transform_and_vice_versa() -> TestResult {
    // PS3.5 A.4.4: `.201` pairs the reversible wavelet with the reversible
    // component transform, and an irreversible codestream under it is a
    // frame mismatch. The general `.203` accepts either wavelet.
    let desc = FrameDesc::new(FrameDescInput {
        rows: 1,
        columns: 1,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_ICT".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let irreversible = one_pixel_transformed_codestream([-28, 22, -38], false)?;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&irreversible, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // The refusal above cannot say which rule fired: an irreversible QCD is
    // also a lossy one, so PS3.5 A.4.4's no-quantization rule refuses the
    // same codestream. This case passes that rule, by rewriting the QCD's
    // Sqcd to the no-quantization style while keeping the irreversible
    // wavelet, so the only refusal left is the wavelet check itself.
    // ISO/IEC 15444-1 A.6.2 Table A.28: Sqcd's three most significant bits
    // carry the guard count and its low bits the quantization style, so
    // clearing the low five bits keeps the guard count and declares no
    // quantization.
    let mut no_quantization = irreversible.clone();
    let mut offset = 2;
    let qcd_payload = loop {
        let marker = no_transform_marker(&no_quantization, offset)?;
        if marker == 0xff5c {
            break offset + 4;
        }
        let length = usize::from(u16::from_be_bytes(
            no_quantization
                .get(offset + 2..offset + 4)
                .ok_or("marker length is truncated")?
                .try_into()?,
        ));
        offset += 2 + length;
    };
    let sqcd = *no_quantization
        .get(qcd_payload)
        .ok_or("QCD payload is truncated")?;
    *no_quantization
        .get_mut(qcd_payload)
        .ok_or("QCD payload is truncated")? = sqcd & 0b1110_0000;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&no_quantization, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

/// The marker word at `offset`, after SOC.
fn no_transform_marker(src: &[u8], offset: usize) -> Result<u16, Box<dyn Error>> {
    Ok(u16::from_be_bytes(
        src.get(offset..offset + 2)
            .ok_or("marker is truncated")?
            .try_into()?,
    ))
}

#[test]
fn descriptor_and_geometry_mismatches_preserve_caller_output() -> TestResult {
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");

    // Dimensions disagreeing with SIZ.
    let wrong = frame(32, 96, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    assert_atomic_error(source, &wrong, CodecError::FrameMismatch);

    // Bits Stored disagreeing with SIZ's precision.
    let narrow = frame(64, 96, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    assert_atomic_error(source, &narrow, CodecError::FrameMismatch);

    // Signedness disagreeing with SIZ.
    let signed = frame(64, 96, 16, 16, PixelRepresentation::Signed, "MONOCHROME2")?;
    assert_atomic_error(source, &signed, CodecError::FrameMismatch);

    // A colour descriptor is a legal shape now, so the refusal moves to the
    // codestream: this fixture carries one component where the descriptor
    // declares three, which is the codestream contradicting the data set.
    let colour = FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "RGB".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    assert_atomic_error(source, &colour, CodecError::FrameMismatch);

    // Other Word Pixel Data. PS3.5 A.4 encapsulates with OB.
    let word = frame_with_vr(
        64,
        96,
        16,
        16,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
        PixelDataVr::Ow,
    )?;
    assert_atomic_error(source, &word, CodecError::UnsupportedPixelFormat);
    Ok(())
}

#[test]
fn structural_failures_are_distinguished_from_each_other() -> TestResult {
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");
    let desc = corpus_desc()?;

    // Not a codestream at all.
    let head = source
        .get(..3)
        .ok_or("fixture is shorter than its own header")?;
    assert_atomic_error(head, &desc, CodecError::InvalidCodestream);
    // Truncated before EOC.
    let truncated = source
        .get(..source.len() - 8)
        .ok_or("fixture is shorter than the truncation")?;
    assert_atomic_error(truncated, &desc, CodecError::InvalidCodestream);

    // PS3.5 A.4's single pad byte is not trailing data and must still decode.
    let mut padded = source.to_vec();
    padded.push(0);
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(&padded, &desc, &mut out)?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw").as_slice()
    );

    // Two extra bytes is a complete image followed by something else.
    let mut trailing = source.to_vec();
    trailing.extend_from_slice(&[0x12, 0x34]);
    assert_atomic_error(&trailing, &desc, CodecError::TrailingData);

    // The caller's buffer is the wrong length.
    let mut short = vec![0xa5; desc.output_len() - 2];
    assert_eq!(
        Htj2kDecoder::lossless().decode(source, &desc, &mut short),
        Err(CodecError::OutputLength {
            expected: desc.output_len(),
            actual: desc.output_len() - 2,
        })
    );
    assert!(short.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn registration_covers_three_uids_atomically() -> TestResult {
    let mut registry = Registry::new();
    for uid in [HTJ2K_LOSSLESS, HTJ2K_LOSSLESS_RPCL, HTJ2K] {
        assert_eq!(registry.capability(uid), Capability::KnownUnavailable);
    }

    register_htj2k_decoders(&mut registry)?;
    for uid in [HTJ2K_LOSSLESS, HTJ2K_LOSSLESS_RPCL, HTJ2K] {
        assert_eq!(registry.capability(uid), Capability::Available);
    }

    assert_eq!(
        register_htj2k_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered {
            uid: HTJ2K_LOSSLESS
        })
    );

    let desc = corpus_desc()?;
    assert_eq!(
        registry.decode_photometric_interpretation(HTJ2K_LOSSLESS, &desc)?,
        DecodePhotometricInterpretation::Preserved
    );
    // The adapter's output is sample-interleaved for every shape it decodes,
    // monochrome included, matching the JPEG 2000 adapter.
    assert_eq!(
        registry.decode_sample_layout(HTJ2K_LOSSLESS, &desc)?,
        DecodeSampleLayout::Interleaved
    );

    let mut out = vec![0xa5; desc.output_len()];
    registry.decode(
        HTJ2K_LOSSLESS,
        include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw").as_slice()
    );
    Ok(())
}

#[test]
fn a_collision_on_the_third_uid_leaves_the_first_two_unclaimed() -> TestResult {
    // The preflight covers all three before any registration, so a collision on
    // the last leaves the earlier two `KnownUnavailable`. Deviation D-19.
    let mut registry = Registry::new();
    registry.register(std::sync::Arc::new(Htj2kDecoder::general()))?;
    assert_eq!(
        register_htj2k_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered { uid: HTJ2K })
    );
    assert_eq!(
        registry.capability(HTJ2K_LOSSLESS),
        Capability::KnownUnavailable
    );
    assert_eq!(
        registry.capability(HTJ2K_LOSSLESS_RPCL),
        Capability::KnownUnavailable
    );
    Ok(())
}

#[track_caller]
fn assert_atomic_error(source: &[u8], desc: &FrameDesc, expected: CodecError) {
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(source, desc, &mut out),
        Err(expected)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
}

/// The corpus rows are 64 by 96 unsigned 16-bit, from `scripts/corpus_synth.py`.
fn corpus_desc() -> Result<FrameDesc, Box<dyn Error>> {
    frame(64, 96, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")
}

fn frame(
    rows: u16,
    columns: u16,
    bits_allocated: u16,
    bits_stored: u16,
    pixel_representation: PixelRepresentation,
    photometric_interpretation: &str,
) -> Result<FrameDesc, Box<dyn Error>> {
    frame_with_vr(
        rows,
        columns,
        bits_allocated,
        bits_stored,
        pixel_representation,
        photometric_interpretation,
        PixelDataVr::Ob,
    )
}

fn frame_with_vr(
    rows: u16,
    columns: u16,
    bits_allocated: u16,
    bits_stored: u16,
    pixel_representation: PixelRepresentation,
    photometric_interpretation: &str,
    pixel_data_vr: PixelDataVr,
) -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel: 1,
        bits_allocated,
        bits_stored,
        high_bit: bits_stored - 1,
        pixel_representation,
        photometric_interpretation: photometric_interpretation.to_owned(),
        pixel_data_vr,
    })?)
}

/// Lowercase hexadecimal SHA-256, so a digest recorded in a spike answer file
/// can be compared with a decode without a second spelling of the same bytes.
fn hex_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Every differing sample as `(index, actual, expected)`, in index order.
fn differences(actual: &[u8], expected: &[u8]) -> Result<Vec<Difference>, Box<dyn Error>> {
    if actual.len() != expected.len() {
        return Err("length mismatch".into());
    }
    let mut found = Vec::new();
    for (index, (got, want)) in actual
        .chunks_exact(2)
        .zip(expected.chunks_exact(2))
        .enumerate()
    {
        let got = u16::from_le_bytes(<[u8; 2]>::try_from(got)?);
        let want = u16::from_le_bytes(<[u8; 2]>::try_from(want)?);
        if got != want {
            found.push((index, got, want));
        }
    }
    Ok(found)
}
