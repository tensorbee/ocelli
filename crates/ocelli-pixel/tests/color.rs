//! DICOM PS3.3 C.7.6.3.1.2 and C.7.6.3.1.3 colour fixtures.
//!
//! Every expected value here was computed in exact rational arithmetic from
//! the standard's stated `RGB -> YBR` forward equations, inverted, and none of
//! it was read out of this repository's own output. HLD 27.2 R2.
//!
//! **The fixtures state YBR and assert RGB, never the reverse.** The standard's
//! coefficients are rounded to four decimals and the resulting matrix is not
//! exactly normalised, so a saturated primary encodes forward to `Cb = 255.5`.
//! Building an expected value through that intermediate would assert something
//! the wire cannot hold.

use ocelli_core::Stored;
use ocelli_pixel::{
    ByteOrder, ColorTransform, DecodedLayout, DecodedPhotometric, ImageDimensions,
    PhotometricInterpretation, PixelDataEncoding, PixelError, PixelRepresentation,
    PlanarConfiguration, Rgb, SampleLayout, StoredBits, StoredPixelDescription,
};

/// Ten times tighter than the f32 accumulation this arithmetic can produce and
/// twenty times tighter than the 0.020028 divergence between the inverse of
/// PS3.3's stated matrix and the textbook BT.601 inverse. So substituting
/// BT.601 constants for the transcribed ones makes every YBR row below red,
/// which is the mutation this tolerance exists to stay sensitive to.
const TOLERANCE: f32 = 0.001;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok(), "{result:?}");
        let Ok(value) = result else { return };
        value
    }};
}

#[track_caller]
fn assert_rgb_close(actual: Rgb, expected: (f32, f32, f32)) {
    let (r, g, b) = expected;
    assert!(
        (actual.r - r).abs() < TOLERANCE
            && (actual.g - g).abs() < TOLERANCE
            && (actual.b - b).abs() < TOLERANCE,
        "was ({}, {}, {}), wanted ({r}, {g}, {b})",
        actual.r,
        actual.g,
        actual.b
    );
}

/// Samples per Pixel as a real header would carry it.
///
/// Spelled out here rather than asked of `PhotometricInterpretation`, because
/// `SampleLayout::new` exists to check the header's Samples per Pixel against
/// the photometric interpretation. A test that derived one from the other
/// would hand the validator its own answer.
fn samples_for(photometric: PhotometricInterpretation) -> u8 {
    match photometric {
        PhotometricInterpretation::Monochrome1
        | PhotometricInterpretation::Monochrome2
        | PhotometricInterpretation::PaletteColor => 1,
        PhotometricInterpretation::Rgb
        | PhotometricInterpretation::YbrFull
        | PhotometricInterpretation::YbrFull422
        | PhotometricInterpretation::YbrPartial422
        | PhotometricInterpretation::YbrIct
        | PhotometricInterpretation::YbrRct
        | PhotometricInterpretation::Xyb => 3,
    }
}

fn layout(
    photometric: PhotometricInterpretation,
    planar: Option<PlanarConfiguration>,
) -> Result<SampleLayout, PixelError> {
    SampleLayout::new(samples_for(photometric), photometric, planar)
}

/// One frame of three-sample interleaved evidence, resolved with no decoder
/// claim at all, which is the native uncompressed case.
fn native_interleaved(
    photometric: PhotometricInterpretation,
) -> Result<ColorTransform, PixelError> {
    ColorTransform::resolve(
        layout(photometric, Some(PlanarConfiguration::Interleaved))?,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    )
}

fn stored(values: &[f32]) -> Vec<Stored> {
    values.iter().copied().map(Stored).collect()
}

// ---------------------------------------------------------------------------
// YBR_FULL, PS3.3 C.7.6.3.1.2
// ---------------------------------------------------------------------------

/// The eight rows, each computed from the exact inverse of the stated matrix.
///
/// ```text
/// R = 1.000000000 Y - 0.000036820 Cb' + 1.401987577 Cr'
/// G = 1.000000000 Y - 0.344113281 Cb' - 0.714103821 Cr'
/// B = 1.000000000 Y + 1.771978117 Cb' - 0.000134583 Cr'
/// ```
///
/// with `Cb' = Cb - 128` and `Cr' = Cr - 128`.
const YBR_FULL_ROWS: [([f32; 3], (f32, f32, f32)); 8] = [
    // Neutral chroma must reproduce the luma exactly on all three channels.
    // A dropped or mistyped luma coefficient moves these three rows and
    // nothing else would.
    ([0.0, 128.0, 128.0], (0.0, 0.0, 0.0)),
    ([255.0, 128.0, 128.0], (255.0, 255.0, 255.0)),
    ([128.0, 128.0, 128.0], (128.0, 128.0, 128.0)),
    // Near each primary. These are the rows a transposed Cb and Cr moves.
    ([76.0, 85.0, 255.0], (254.054_01, 0.105_685_8, -0.212_151_1)),
    ([150.0, 44.0, 21.0], (-0.009_577_9, 255.314_62, 1.168_238_6)),
    (
        [29.0, 255.0, 107.0],
        (-0.446_415_3, 0.293_793_5, 254.044_05),
    ),
    // An arbitrary interior colour, which no boundary convention reaches.
    ([79.0, 101.0, 163.0], (128.070_56, 63.297_425, 31.151_88)),
    // A studio-swing-looking triple read as full range. It is the row that
    // goes red if the two YBR arms are ever swapped.
    ([16.0, 16.0, 240.0], (173.026_73, -25.438_94, -182.476_62)),
];

#[test]
fn ybr_full_inverts_the_ps3_3_c_7_6_3_1_2_matrix() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull));
    for (ybr, expected) in YBR_FULL_ROWS {
        let source = stored(&ybr);
        let mut destination = [Rgb::BLACK];
        assert_eq!(transform.map_into(&source, &mut destination, 1), Ok(()));
        assert_rgb_close(destination[0], expected);
    }
}

/// Nothing is clamped to `[0, 255]`.
///
/// Three of the eight rows above are negative on at least one channel, which
/// is the rounded forward matrix's own residue rather than a defect. A stage
/// that clamped would hide it, and clamping belongs with the rounding decision
/// at the render or export boundary. HLD 27.3 makes that a review item
/// wherever it lands, so it is not made silently here.
#[test]
fn out_of_gamut_results_are_reported_rather_than_clamped() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull));
    let source = stored(&[16.0, 16.0, 240.0]);
    let mut destination = [Rgb::BLACK];
    assert_eq!(transform.map_into(&source, &mut destination, 1), Ok(()));
    assert!(destination[0].b < -182.0);
    assert!(destination[0].r > 173.0);
}

// ---------------------------------------------------------------------------
// YBR_PARTIAL_422, PS3.3 C.7.6.3.1.2
// ---------------------------------------------------------------------------

/// ```text
/// R = 1.164415463 Y' - 0.000095036 Cb' + 1.596001878 Cr'
/// G = 1.164415463 Y' - 0.391724564 Cb' - 0.813013368 Cr'
/// B = 1.164415463 Y' + 2.017290682 Cb' - 0.000135273 Cr'
/// ```
///
/// with `Y' = Y - 16`, which is the offset a full-range implementation omits.
const YBR_PARTIAL_ROWS: [([f32; 3], (f32, f32, f32)); 5] = [
    // Studio black. THIS IS THE ROW THAT PROVES THE +16 OFFSET. Omitting it
    // gives 18.630647 on every channel, which is 18.6 of 255 and cannot hide
    // inside any tolerance this suite would accept.
    ([16.0, 128.0, 128.0], (0.0, 0.0, 0.0)),
    ([235.0, 128.0, 128.0], (255.006_99, 255.006_99, 255.006_99)),
    // Below studio black is legal on the wire and stays negative. A stage that
    // clamped the input to the studio range would move this row to zero.
    ([0.0, 128.0, 128.0], (-18.630_647, -18.630_647, -18.630_647)),
    ([126.0, 128.0, 128.0], (128.085_7, 128.085_7, 128.085_7)),
    (
        [81.0, 90.0, 240.0],
        (254.442_83, -0.484_958_7, -0.985_191_4),
    ),
];

#[test]
fn ybr_partial_422_applies_the_studio_swing_offset_and_gain() {
    let transform = require_ok!(ColorTransform::resolve(
        require_ok!(layout(
            PhotometricInterpretation::YbrPartial422,
            Some(PlanarConfiguration::Interleaved)
        )),
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    ));
    // Two pixels, because 4:2:2 is stored in pairs. Both carry the same
    // chroma, so each row below is asserted twice and the pair alignment is
    // exercised at the same time.
    for (ybr, expected) in YBR_PARTIAL_ROWS {
        let [y, cb, cr] = ybr;
        let source = stored(&[y, y, cb, cr]);
        let mut destination = [Rgb::BLACK; 2];
        assert_eq!(transform.map_into(&source, &mut destination, 2), Ok(()));
        assert_rgb_close(destination[0], expected);
        assert_rgb_close(destination[1], expected);
    }
}

/// The partial-range arm is not the full-range arm with different numbers.
///
/// Feeding the same triple to both must disagree, or the two `match` arms
/// could be collapsed by a later reader without any test noticing.
#[test]
fn the_two_ybr_ranges_do_not_agree_on_the_same_triple() {
    let full = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull));
    let source = stored(&[16.0, 128.0, 128.0]);
    let mut destination = [Rgb::BLACK];
    assert_eq!(full.map_into(&source, &mut destination, 1), Ok(()));
    // Full range reads studio black as 16, partial range reads it as 0.
    assert_rgb_close(destination[0], (16.0, 16.0, 16.0));
}

// ---------------------------------------------------------------------------
// YBR_FULL_422 chroma replication, PS3.3 C.7.6.3.1.2
// ---------------------------------------------------------------------------

/// One wire pair `Y1 Y2 Cb Cr` becomes two pixels sharing the chroma.
///
/// Replication rather than interpolation, because PS3.3 names no filter. See
/// the design plan's "What the specification does not cover" item 5.
#[test]
fn ybr_full_422_replicates_chroma_across_the_pair() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull422));
    // Y1 = 60, Y2 = 200, Cb = 90, Cr = 170.
    let source = stored(&[60.0, 200.0, 90.0, 170.0]);
    let mut destination = [Rgb::BLACK; 2];
    assert_eq!(transform.map_into(&source, &mut destination, 2), Ok(()));
    // Exact: (118.8848774, 43.0839442, -7.3408209) and
    //        (258.8848774, 183.0839442, 132.6591791). Written as the shortest
    //        decimals that round-trip to the nearest f32, per
    //        clippy::excessive_precision.
    assert_rgb_close(destination[0], (118.884_88, 43.083_942, -7.340_821));
    assert_rgb_close(destination[1], (258.884_9, 183.083_94, 132.659_18));
    // The two luma values differ by exactly 140, and so does every channel,
    // which is what shared chroma means and what an interpolating upsample
    // would break.
    assert!((destination[1].r - destination[0].r - 140.0).abs() < TOLERANCE);
    assert!((destination[1].g - destination[0].g - 140.0).abs() < TOLERANCE);
    assert!((destination[1].b - destination[0].b - 140.0).abs() < TOLERANCE);
}

/// A 4:2:2 frame is two bytes per pixel and not three.
///
/// `scripts/tests/test_corpus_synth.py::test_ybr_full_422_frame_is_two_bytes_per_pixel`
/// asserts this about the corpus. This is the counterpart asserting it about
/// the unpacker, which until F-030 demanded three bytes per pixel from a
/// source that holds two and so could not read the corpus row at all.
#[test]
fn a_4_2_2_frame_is_two_stored_samples_per_pixel() {
    let dimensions = require_ok!(ImageDimensions::new(12, 20));
    let description = StoredPixelDescription::new(
        dimensions,
        require_ok!(layout(
            PhotometricInterpretation::YbrFull422,
            Some(PlanarConfiguration::Interleaved)
        )),
        require_ok!(StoredBits::new(8, 8, 7, PixelRepresentation::Unsigned)),
    );
    assert_eq!(description.sample_count(), Ok(12 * 20 * 2));
    assert_eq!(description.sample_count(), Ok(480));

    // The same frame as RGB is three per pixel, so the 4:2:2 answer is not
    // simply what every three-sample layout returns.
    let rgb = StoredPixelDescription::new(
        dimensions,
        require_ok!(layout(
            PhotometricInterpretation::Rgb,
            Some(PlanarConfiguration::Interleaved)
        )),
        require_ok!(StoredBits::new(8, 8, 7, PixelRepresentation::Unsigned)),
    );
    assert_eq!(rgb.sample_count(), Ok(12 * 20 * 3));
}

/// An odd `Columns` cannot hold whole chroma pairs, so it is refused.
#[test]
fn odd_columns_are_refused_for_subsampled_chroma() {
    let description = StoredPixelDescription::new(
        require_ok!(ImageDimensions::new(12, 19)),
        require_ok!(layout(
            PhotometricInterpretation::YbrFull422,
            Some(PlanarConfiguration::Interleaved)
        )),
        require_ok!(StoredBits::new(8, 8, 7, PixelRepresentation::Unsigned)),
    );
    assert_eq!(
        description.sample_count(),
        Err(PixelError::SubsampledChromaAlignment)
    );
    // And the refusal reaches `unpack`, which calls it before any length
    // arithmetic, so no buffer is sized from a count that does not exist.
    let mut destination = [Stored(77.0)];
    assert_eq!(
        description.unpack(&[0], ByteOrder::LittleEndian, &mut destination),
        Err(PixelError::SubsampledChromaAlignment)
    );
    assert_eq!(destination[0].0.to_bits(), 77.0_f32.to_bits());
}

// ---------------------------------------------------------------------------
// Planar Configuration, PS3.3 C.7.6.3.1.3
// ---------------------------------------------------------------------------

/// Three RGB pixels, red-ish, green-ish and blue-ish, in both wire layouts.
const INTERLEAVED: [f32; 9] = [200.0, 30.0, 40.0, 50.0, 210.0, 60.0, 70.0, 80.0, 220.0];
const PLANAR: [f32; 9] = [200.0, 50.0, 70.0, 30.0, 210.0, 80.0, 40.0, 60.0, 220.0];

#[test]
fn the_same_image_reads_identically_in_both_planar_layouts() {
    let interleaved = require_ok!(ColorTransform::resolve(
        require_ok!(layout(
            PhotometricInterpretation::Rgb,
            Some(PlanarConfiguration::Interleaved)
        )),
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    ));
    let planar = require_ok!(ColorTransform::resolve(
        require_ok!(layout(
            PhotometricInterpretation::Rgb,
            Some(PlanarConfiguration::Planar)
        )),
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    ));

    let mut from_interleaved = [Rgb::BLACK; 3];
    let mut from_planar = [Rgb::BLACK; 3];
    assert_eq!(
        interleaved.map_into(&stored(&INTERLEAVED), &mut from_interleaved, 3),
        Ok(())
    );
    assert_eq!(
        planar.map_into(&stored(&PLANAR), &mut from_planar, 3),
        Ok(())
    );

    for (interleaved_pixel, planar_pixel) in from_interleaved.iter().zip(from_planar.iter()) {
        assert_rgb_close(
            *interleaved_pixel,
            (planar_pixel.r, planar_pixel.g, planar_pixel.b),
        );
    }
    // And they are the pixels intended, not merely equal to each other. Two
    // identically wrong readings would satisfy the loop above on its own.
    assert_rgb_close(from_interleaved[0], (200.0, 30.0, 40.0));
    assert_rgb_close(from_interleaved[1], (50.0, 210.0, 60.0));
    assert_rgb_close(from_interleaved[2], (70.0, 80.0, 220.0));
}

/// PS3.3 C.7.6.3.1.3: Planar Configuration "is required to be 0 when the
/// Pixel Data is encapsulated". A header saying 1 for an encapsulated frame is
/// ignored rather than honoured, and the same header is honoured for a native
/// one. Both directions, because asserting only the first cannot tell
/// "ignored correctly" from "never read at all".
#[test]
fn planar_configuration_is_honoured_natively_and_ignored_when_encapsulated() {
    let declared_planar = require_ok!(layout(
        PhotometricInterpretation::Rgb,
        Some(PlanarConfiguration::Planar)
    ));

    let native = require_ok!(ColorTransform::resolve(
        declared_planar,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    ));
    let encapsulated = require_ok!(ColorTransform::resolve(
        declared_planar,
        PixelDataEncoding::Encapsulated,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    ));

    let mut native_out = [Rgb::BLACK; 3];
    let mut encapsulated_out = [Rgb::BLACK; 3];
    assert_eq!(
        native.map_into(&stored(&PLANAR), &mut native_out, 3),
        Ok(())
    );
    assert_eq!(
        encapsulated.map_into(&stored(&PLANAR), &mut encapsulated_out, 3),
        Ok(())
    );

    // Native honours the 1 and reads the planes.
    assert_rgb_close(native_out[0], (200.0, 30.0, 40.0));
    // Encapsulated ignores it and reads the same bytes interleaved, so the
    // first pixel is the first three wire samples.
    assert_rgb_close(encapsulated_out[0], (200.0, 50.0, 70.0));
}

// ---------------------------------------------------------------------------
// The resolution, and the double-conversion guard
// ---------------------------------------------------------------------------

/// A decoder that already produced RGB must not have a second conversion
/// applied on top of its own.
///
/// This is the defect `docs/sprints/CURRENT_SPRINT.md` names first and the
/// F-024 AS_BUILT entry's closing note left due. A JPEG frame whose DICOM
/// header still reads `YBR_FULL_422` is RGB in the buffer, and converting it
/// again darkens and shifts hue on an image that still looks like an image.
///
/// **A greyscale fixture cannot show this**, which is why the same samples are
/// asserted both ways in one test.
#[test]
fn a_decoder_converted_frame_is_not_converted_a_second_time() {
    let header = require_ok!(layout(
        PhotometricInterpretation::YbrFull422,
        Some(PlanarConfiguration::Interleaved)
    ));
    let samples = [200.0, 30.0, 40.0, 50.0, 210.0, 60.0];

    let already_rgb = require_ok!(ColorTransform::resolve(
        header,
        PixelDataEncoding::Encapsulated,
        DecodedPhotometric::Rgb,
        DecodedLayout::Interleaved,
        None,
    ));
    let mut untouched = [Rgb::BLACK; 2];
    assert_eq!(
        already_rgb.map_into(&stored(&samples), &mut untouched, 2),
        Ok(())
    );
    assert_rgb_close(untouched[0], (200.0, 30.0, 40.0));
    assert_rgb_close(untouched[1], (50.0, 210.0, 60.0));

    // The identical samples, with the decoder claiming nothing, DO convert.
    // Without this half the test would also pass against an implementation
    // that never converts anything at all.
    let preserved = require_ok!(ColorTransform::resolve(
        header,
        PixelDataEncoding::Encapsulated,
        DecodedPhotometric::Preserved,
        DecodedLayout::Interleaved,
        None,
    ));
    let mut converted = [Rgb::BLACK; 2];
    assert_eq!(
        preserved.map_into(&stored(&samples[0..4]), &mut converted, 2),
        Ok(())
    );
    assert!((converted[0].r - 200.0).abs() > 1.0);
}

/// PS3.3 permits `YBR_ICT` and `YBR_RCT` only with JPEG 2000, where the
/// codestream's multiple component transform carries them and the decoder
/// performs the inverse. `crates/ocelli-codec/src/jpeg2000.rs` refuses
/// `mct != 0`, so neither can reach this stage still in that space.
///
/// RCT is an integer lifting transform and not a matrix at all, so applying
/// the full-range matrix to it would produce a plausible image in the wrong
/// colours. Refusing is HLD section 31's rule.
#[test]
fn the_two_jpeg_2000_transform_spaces_are_the_codec_s_to_invert() {
    for photometric in [
        PhotometricInterpretation::YbrIct,
        PhotometricInterpretation::YbrRct,
    ] {
        assert_eq!(
            native_interleaved(photometric),
            Err(PixelError::CodecOwnedColorTransform)
        );
        // Under a decoder that reports RGB the question does not arise,
        // because the resolution has already replaced the space.
        let resolved = ColorTransform::resolve(
            require_ok!(layout(photometric, Some(PlanarConfiguration::Interleaved))),
            PixelDataEncoding::Encapsulated,
            DecodedPhotometric::Rgb,
            DecodedLayout::Interleaved,
            None,
        );
        assert!(resolved.is_ok(), "{resolved:?}");
    }
}

/// PS3.3 C.7.6.3.1.2 names `XYB` and gives no conversion matrix. The JPEG XL
/// decoder owns the conversion. A preserved report must not invent one.
#[test]
fn xyb_is_codec_owned() {
    assert_eq!(
        native_interleaved(PhotometricInterpretation::Xyb),
        Err(PixelError::CodecOwnedColorTransform)
    );
    let resolved = require_ok!(ColorTransform::resolve(
        require_ok!(layout(
            PhotometricInterpretation::Xyb,
            Some(PlanarConfiguration::Interleaved)
        )),
        PixelDataEncoding::Encapsulated,
        DecodedPhotometric::Rgb,
        DecodedLayout::Interleaved,
        None,
    ));
    // Stored 10, 20, 30 stay 10, 20, 30. There is no matrix to apply.
    let mut out = [Rgb::BLACK; 1];
    assert_eq!(
        resolved.map_into(&stored(&[10.0, 20.0, 30.0]), &mut out, 1),
        Ok(())
    );
    let Some(pixel) = out.first().copied() else {
        return;
    };
    assert_rgb_close(pixel, (10.0, 20.0, 30.0));
}

/// Stage 3 and stage 4 partition the photometric interpretations exactly.
///
/// `PresentationTransform::new` refuses every colour space with
/// `PresentationLutNotApplicable`. `ColorTransform::resolve` refuses both
/// monochrome ones with `ColorTransformNotApplicable`. Together they mean a
/// frame reaches exactly one of the two stages, never both and never neither.
#[test]
fn monochrome_frames_belong_to_stage_three_and_are_refused_here() {
    for photometric in [
        PhotometricInterpretation::Monochrome1,
        PhotometricInterpretation::Monochrome2,
    ] {
        assert_eq!(
            ColorTransform::resolve(
                require_ok!(layout(photometric, None)),
                PixelDataEncoding::Native,
                DecodedPhotometric::Preserved,
                DecodedLayout::Preserved,
                None,
            ),
            Err(PixelError::ColorTransformNotApplicable)
        );
    }
}

// ---------------------------------------------------------------------------
// The caller-buffer contract
// ---------------------------------------------------------------------------

/// The crate's standing rule: both lengths are checked before the first
/// destination element is written.
#[test]
fn mapping_refuses_a_length_mismatch_before_writing() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::Rgb));
    let mut destination = [Rgb {
        r: 77.0,
        g: 77.0,
        b: 77.0,
    }];

    // One pixel of RGB is three stored samples, so two is short and four is
    // long. Both are refused and neither writes.
    assert_eq!(
        transform.map_into(&stored(&[1.0, 2.0]), &mut destination, 1),
        Err(PixelError::SourceLength)
    );
    assert_eq!(destination[0].r.to_bits(), 77.0_f32.to_bits());
    assert_eq!(
        transform.map_into(&stored(&[1.0, 2.0, 3.0, 4.0]), &mut destination, 1,),
        Err(PixelError::SourceLength)
    );
    assert_eq!(destination[0].r.to_bits(), 77.0_f32.to_bits());
}

/// PS3.3 C.7.6.3.1.2 pairs pixels horizontally. Three columns and two rows
/// is six pixels, which is even, and the third pixel still shares chroma
/// with the first pixel of the next row.
#[test]
fn odd_columns_are_refused_even_when_the_pixel_count_is_even() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull422));
    let mut destination = [Rgb::BLACK; 6];
    assert_eq!(
        transform.map_into(
            &stored(&[
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
            ]),
            &mut destination,
            3,
        ),
        Err(PixelError::SubsampledChromaAlignment)
    );
    assert_eq!(destination[0].r.to_bits(), 0.0_f32.to_bits());
}

/// A 4:2:2 destination holding an odd number of pixels splits a chroma pair.
#[test]
fn an_odd_pixel_count_is_refused_for_subsampled_chroma() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull422));
    let mut destination = [Rgb::BLACK; 3];
    assert_eq!(
        transform.map_into(
            &stored(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            &mut destination,
            2,
        ),
        Err(PixelError::SubsampledChromaAlignment)
    );
}

// ---------------------------------------------------------------------------
// Display scaling of colour samples, F-239 item 11 and AC11
// ---------------------------------------------------------------------------

use ocelli_pixel::{ColourWindow, DisplayScale};

/// PS3.3 C.7.6.3.1 gives Bits Stored as the range of each sample, so a
/// sample of `n` bits is shifted right by `n - 8` for `9 <= n <= 16` before
/// stage 4, and taken as it is at 8. The operator's answer to F-239's
/// question 8.
///
/// ```text
/// Bits Stored 12, shift 4:  4095 >> 4 = 255   2048 >> 4 = 128   15 >> 4 = 0
/// Bits Stored 16, shift 8:  4095 >> 8 = 15    2048 >> 8 = 8     15 >> 8 = 0
/// ```
///
/// Shifting by Bits Allocated 16 instead of Bits Stored 12 gives the second
/// row for the first frame, which is the mutation this fixture rejects.
/// cornerstone3D's saturation gives `255 255 15`.
#[test]
fn colour_above_8_bits_is_shifted_by_bits_stored() {
    let twelve = require_ok!(DisplayScale::from_bits(12));
    let sixteen = require_ok!(DisplayScale::from_bits(16));
    let rgb = require_ok!(native_interleaved(PhotometricInterpretation::Rgb));

    let at_twelve = require_ok!(rgb.clone().with_sample_scale(twelve));
    let mut destination = [Rgb::BLACK];
    assert_eq!(
        at_twelve.map_into(&stored(&[4095.0, 2048.0, 15.0]), &mut destination, 1),
        Ok(())
    );
    assert_eq!(
        [destination[0].r, destination[0].g, destination[0].b].map(f32::to_bits),
        [255.0_f32, 128.0, 0.0].map(f32::to_bits)
    );

    let at_sixteen = require_ok!(rgb.with_sample_scale(sixteen));
    assert_eq!(
        at_sixteen.map_into(&stored(&[4095.0, 2048.0, 15.0]), &mut destination, 1),
        Ok(())
    );
    assert_eq!(
        [destination[0].r, destination[0].g, destination[0].b].map(f32::to_bits),
        [15.0_f32, 8.0, 0.0].map(f32::to_bits)
    );
}

/// The shift happens before stage 4, so the 8-bit YCbCr matrix and its
/// `+ 128` chroma offset meet an 8-bit value.
///
/// ```text
/// 1600 >> 4 = 100   2400 >> 4 = 150   1440 >> 4 = 90
/// ```
///
/// and `100 150 90` is the AC3 triple, `Cb' = 22`, `Cr' = -38`:
///
/// ```text
/// R = 100 - 0.00003682 * 22 + 1.4019876 * -38 = 46.72366
/// G = 100 - 0.3441133 * 22 - 0.7141038 * -38  = 119.56544
/// B = 100 + 1.7719781 * 22 - 0.000134583 * -38 = 138.98863
/// ```
///
/// Shifting after the matrix would subtract 128 from a 12-bit chroma and give
/// a different colour entirely.
#[test]
fn a_ybr_sample_is_shifted_before_the_matrix() {
    let transform = require_ok!(
        require_ok!(native_interleaved(PhotometricInterpretation::YbrFull))
            .with_sample_scale(require_ok!(DisplayScale::from_bits(12)))
    );
    let mut destination = [Rgb::BLACK];
    assert_eq!(
        transform.map_into(&stored(&[1600.0, 2400.0, 1440.0]), &mut destination, 1),
        Ok(())
    );
    assert_rgb_close(destination[0], (46.723_66, 119.565_44, 138.988_63));
}

/// Eight bits is the identity, nine to sixteen shift, and everything else is
/// refused by name rather than clamped into range.
#[test]
fn a_display_scale_outside_8_to_16_bits_is_refused() {
    for bits in [0_u16, 1, 7, 17, 32] {
        assert_eq!(
            DisplayScale::from_bits(bits),
            Err(PixelError::UnsupportedColourDepth),
            "{bits}"
        );
    }
    for (bits, shift) in [(8_u16, 0_u8), (9, 1), (10, 2), (12, 4), (15, 7), (16, 8)] {
        assert_eq!(require_ok!(DisplayScale::from_bits(bits)).shift(), shift);
    }
    // A truncating shift, not a rounding one. 255 >> 8 is 0, where rounding
    // 255 / 256 would give 1. 65535 >> 8 is 255, 32768 >> 8 is 128.
    let sixteen = require_ok!(DisplayScale::from_bits(16));
    for (value, expected) in [
        (255.0_f32, 0.0_f32),
        (256.0, 1.0),
        (32_768.0, 128.0),
        (65_535.0, 255.0),
    ] {
        assert_eq!(
            sixteen.apply(value).to_bits(),
            expected.to_bits(),
            "{value}"
        );
    }
    let eight = require_ok!(DisplayScale::from_bits(8));
    assert_eq!(eight.apply(200.0).to_bits(), 200.0_f32.to_bits());
}

/// A palette frame's stored value is an index, PS3.3 C.7.6.3.1.5, so it is
/// never scaled. Its entries are scaled by `PaletteColorLut` instead.
#[test]
fn a_sample_scale_is_refused_on_a_palette_frame() {
    let red = require_ok!(ocelli_pixel::LutDescriptor::new(1, 0, 8, vec![1.0]));
    let green = require_ok!(ocelli_pixel::LutDescriptor::new(1, 0, 8, vec![2.0]));
    let blue = require_ok!(ocelli_pixel::LutDescriptor::new(1, 0, 8, vec![3.0]));
    let palette = require_ok!(ocelli_pixel::PaletteColorLut::new(red, green, blue));
    let transform = require_ok!(ColorTransform::resolve(
        require_ok!(layout(PhotometricInterpretation::PaletteColor, None)),
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        Some(palette),
    ));
    assert_eq!(
        transform.with_sample_scale(require_ok!(DisplayScale::from_bits(16))),
        Err(PixelError::SampleScaleNotApplicable)
    );
}

// ---------------------------------------------------------------------------
// Colour window and invert, deviation D-29 and AC19
// ---------------------------------------------------------------------------

/// cornerstone3D 5.8.2's colour window, `getVOILut.js:3`:
///
/// ```text
/// ((x - (c - 0.5)) / (w - 1) + 0.5) * 255, clamped to [0, 255]
/// ```
///
/// which is PS3.3 C.11.2.1.2's LINEAR with output range `0 ..= 255`, applied
/// to each channel. `generateColorLUT.js:13` to `:21` inverts after it as
/// `255 - y`. Every row is computed in `f32`:
///
/// ```text
/// w 100, c 128: lower = 127.5 - 49.5 = 78, upper = 177
///   100 -> ((100 - 127.5) / 99 + 0.5) * 255 = 56.666664
///   30  -> at or below 78, 0
///   200 -> above 177, 255
///   128 -> (0.5 / 99 + 0.5) * 255 = 128.78787
/// w 10, c 128: lower = 123, upper = 132
///   125 -> (-2.5 / 9 + 0.5) * 255 = 56.666664
///   128 -> ( 0.5 / 9 + 0.5) * 255 = 141.66667
///   130 -> ( 2.5 / 9 + 0.5) * 255 = 198.33334
/// w 2, c 128: lower = 127, upper = 128
///   127 -> 0, 128 -> (0.5 / 1 + 0.5) * 255 = 255, 0 -> 0
/// ```
///
/// **The width 10 rows exist to reject `w` in place of `w - 1`**, which gives
/// `63.75 140.25 191.25` there, while the width 100 rows alone would move by
/// less than a byte on two of their three channels. LINEAR_EXACT's `c` and `w`
/// give `51 127.5 178.5`.
#[test]
fn colour_window_is_linear_per_channel_and_inverted_after() {
    let rows: [(f32, f32, [f32; 3], [f32; 3]); 4] = [
        (128.0, 100.0, [100.0, 30.0, 200.0], [56.666_664, 0.0, 255.0]),
        (128.0, 100.0, [128.0, 128.0, 128.0], [128.787_87; 3]),
        (
            128.0,
            10.0,
            [125.0, 128.0, 130.0],
            [56.666_664, 141.666_67, 198.333_34],
        ),
        (128.0, 2.0, [127.0, 128.0, 0.0], [0.0, 255.0, 0.0]),
    ];
    for (center, width, input, expected) in rows {
        let window = require_ok!(ColourWindow::new(center, width, false));
        let source = [Rgb {
            r: input[0],
            g: input[1],
            b: input[2],
        }];
        let mut destination = [Rgb::BLACK];
        assert_eq!(window.map_into(&source, &mut destination), Ok(()));
        assert_rgb_close(destination[0], (expected[0], expected[1], expected[2]));

        // Inverted after the window, `0 + 255 - y`, so the same rows reflect.
        let inverted = require_ok!(ColourWindow::new(center, width, true));
        assert_eq!(inverted.map_into(&source, &mut destination), Ok(()));
        assert_rgb_close(
            destination[0],
            (
                255.0 - expected[0],
                255.0 - expected[1],
                255.0 - expected[2],
            ),
        );
    }
}

/// Inverting BEFORE the window is a different image, but only off centre.
///
/// The invert reflects about 127.5 and a window at centre 128 is symmetric
/// about 127.5 too, so at that centre the two orders give the same value and
/// every row above would pass either way. At centre 100 and width 100 the
/// window spans `lower = 99.5 - 49.5 = 50` to `upper = 149`:
///
/// ```text
/// window then invert: ((60 - 99.5) / 99 + 0.5) * 255 = 25.75758, 255 - that = 229.24242
/// invert then window: 255 - 60 = 195, above 149, so 255
/// ```
#[test]
fn the_colour_invert_follows_the_window_and_does_not_precede_it() {
    let inverted = require_ok!(ColourWindow::new(100.0, 100.0, true));
    let mut destination = [Rgb::BLACK];
    assert_eq!(
        inverted.map_into(
            &[Rgb {
                r: 60.0,
                g: 60.0,
                b: 60.0
            }],
            &mut destination
        ),
        Ok(())
    );
    assert_rgb_close(destination[0], (229.242_42, 229.242_42, 229.242_42));
}

/// `renderColorImage.js:30` to `:36` returns the unwindowed canvas when the
/// width is 255 or 256, the centre is 127 or 128 and invert is off. At width
/// 255 and centre 127 the formula would give
/// `((100 - 126.5) / 254 + 0.5) * 255 = 100.89567`, so applying it there is a
/// one-byte difference from the reference. The initial window is width 256
/// and centre 128, `createImage.js:321` to `:324`, which is the identity.
#[test]
fn the_identity_colour_window_is_skipped_as_the_reference_skips_it() {
    let source = [Rgb {
        r: 100.0,
        g: 10.0,
        b: 200.0,
    }];
    for (center, width) in [
        (127.0_f32, 255.0_f32),
        (128.0, 255.0),
        (127.0, 256.0),
        (128.0, 256.0),
    ] {
        let window = require_ok!(ColourWindow::new(center, width, false));
        assert!(window.is_identity(), "{center} {width}");
        let mut destination = [Rgb::BLACK];
        assert_eq!(window.map_into(&source, &mut destination), Ok(()));
        assert_eq!(destination[0].r.to_bits(), 100.0_f32.to_bits());
        assert_eq!(destination[0].g.to_bits(), 10.0_f32.to_bits());
        assert_eq!(destination[0].b.to_bits(), 200.0_f32.to_bits());
    }
    assert!(ColourWindow::initial().is_identity());
    // Invert on, or any other window, is not skipped.
    assert!(!require_ok!(ColourWindow::new(128.0, 256.0, true)).is_identity());
    assert!(!require_ok!(ColourWindow::new(129.0, 256.0, false)).is_identity());
    let applied = require_ok!(ColourWindow::new(127.0, 254.0, false));
    let mut destination = [Rgb::BLACK];
    assert_eq!(applied.map_into(&source, &mut destination), Ok(()));
    assert!(destination[0].r.to_bits() != 100.0_f32.to_bits());
}

/// AC21's case, an exact tie in `f32`.
///
/// ```text
/// w 511, c 128.5: ((130 - 128) / 510 + 0.5) * 255 = 1 + 127.5 = 128.5
/// inverted: 255 - 128.5 = 126.5
/// ```
///
/// Both are exactly representable and both are reached exactly in `f32`, so
/// the byte the render boundary stores for them is decided by its tie rule
/// alone. `crates/ocelli-render` asserts that byte.
#[test]
fn a_colour_window_can_land_exactly_on_a_tie() {
    let gray = [Rgb {
        r: 130.0,
        g: 130.0,
        b: 130.0,
    }];
    let mut destination = [Rgb::BLACK];
    let window = require_ok!(ColourWindow::new(128.5, 511.0, false));
    assert_eq!(window.map_into(&gray, &mut destination), Ok(()));
    assert_eq!(destination[0].r.to_bits(), 128.5_f32.to_bits());
    let inverted = require_ok!(ColourWindow::new(128.5, 511.0, true));
    assert_eq!(inverted.map_into(&gray, &mut destination), Ok(()));
    assert_eq!(destination[0].b.to_bits(), 126.5_f32.to_bits());
}

/// The existing LINEAR refusal, PS3.3 C.11.2.1.2's `width >= 1`, and the
/// length check before the first write.
#[test]
fn a_colour_window_refuses_what_the_linear_window_refuses() {
    assert_eq!(
        ColourWindow::new(128.0, 0.5, false),
        Err(PixelError::InvalidWindow)
    );
    assert_eq!(
        ColourWindow::new(f32::NAN, 100.0, false),
        Err(PixelError::InvalidWindow)
    );
    let window = require_ok!(ColourWindow::new(128.0, 100.0, false));
    let mut destination = [Rgb {
        r: 7.0,
        g: 7.0,
        b: 7.0,
    }];
    assert_eq!(
        window.map_into(&[Rgb::BLACK, Rgb::BLACK], &mut destination),
        Err(PixelError::DestinationLength)
    );
    assert_eq!(destination[0].r.to_bits(), 7.0_f32.to_bits());
}

/// Exact inverse basis values, PS3.3 2026d C.7.6.3.1.2 for FULL and
/// PS3.3 2017b C.7.6.3.1.2 for the retired PARTIAL_422 equations.
/// Exact rational inversion gives -870/23628469, -3180/23628469 and
/// -310000/3261922071. Nearest binary32 bits were independently selected
/// by exact distances, without reading product outputs. Zeroing the other
/// shifted components isolates each coefficient without accumulation error.
#[test]
fn ybr_inverse_basis_uses_one_rounding_from_exact_rationals() {
    let mut actual_bits = Vec::new();
    let mut expected_bits = Vec::new();
    for (space, samples, pixels, component, expected) in [
        (
            PhotometricInterpretation::YbrFull,
            &[0.0, 129.0, 128.0][..],
            1,
            0,
            0xb81a_6f2a,
        ),
        (
            PhotometricInterpretation::YbrFull,
            &[0.0, 128.0, 129.0][..],
            1,
            2,
            0xb90d_1ef6,
        ),
        (
            PhotometricInterpretation::YbrPartial422,
            &[16.0, 16.0, 129.0, 128.0][..],
            2,
            0,
            0xb8c7_4e0f,
        ),
    ] {
        let transform = require_ok!(native_interleaved(space));
        let mut output = vec![Rgb::BLACK; pixels];
        assert_eq!(
            transform.map_into(&stored(samples), &mut output, pixels),
            Ok(())
        );
        let rgb = require_ok!(output.first().ok_or("the basis fixture has no output"));
        let actual = require_ok!(
            [rgb.r, rgb.g, rgb.b]
                .get(component)
                .copied()
                .ok_or("the basis fixture names no channel")
        );
        actual_bits.push(actual.to_bits());
        expected_bits.push(expected);
    }
    assert_eq!(actual_bits, expected_bits);
}

/// Independent binary32 calculation: for YBR [0,128,0], blue is
/// 4624251/268435456 after the nearest inverse coefficient is multiplied
/// by -128. The stated centre minus one half lies below that value, so the
/// approved width-one LINEAR rule maps it to 255. The old rounded-summary
/// literal instead gives 9248473/536870912, below the threshold.
#[test]
fn ybr_exact_inverse_keeps_a_narrow_window_on_the_correct_side() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull));
    let mut output = [Rgb::BLACK];
    assert_eq!(
        transform.map_into(&stored(&[0.0, 128.0, 0.0]), &mut output, 1),
        Ok(())
    );
    let window = require_ok!(ocelli_pixel::ColourWindow::new(
        f32::from_bits(0x3f04_68f7),
        1.0,
        false
    ));
    assert_eq!(window.apply(output[0]).b.to_bits(), 255.0_f32.to_bits());
}
