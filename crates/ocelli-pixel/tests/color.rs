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
        | PhotometricInterpretation::YbrRct => 3,
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
        assert_eq!(transform.map_into(&source, &mut destination), Ok(()));
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
    assert_eq!(transform.map_into(&source, &mut destination), Ok(()));
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
        assert_eq!(transform.map_into(&source, &mut destination), Ok(()));
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
    assert_eq!(full.map_into(&source, &mut destination), Ok(()));
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
    assert_eq!(transform.map_into(&source, &mut destination), Ok(()));
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
        interleaved.map_into(&stored(&INTERLEAVED), &mut from_interleaved),
        Ok(())
    );
    assert_eq!(planar.map_into(&stored(&PLANAR), &mut from_planar), Ok(()));

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
    assert_eq!(native.map_into(&stored(&PLANAR), &mut native_out), Ok(()));
    assert_eq!(
        encapsulated.map_into(&stored(&PLANAR), &mut encapsulated_out),
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
        already_rgb.map_into(&stored(&samples), &mut untouched),
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
        preserved.map_into(&stored(&samples[0..4]), &mut converted),
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
        transform.map_into(&stored(&[1.0, 2.0]), &mut destination),
        Err(PixelError::SourceLength)
    );
    assert_eq!(destination[0].r.to_bits(), 77.0_f32.to_bits());
    assert_eq!(
        transform.map_into(&stored(&[1.0, 2.0, 3.0, 4.0]), &mut destination),
        Err(PixelError::SourceLength)
    );
    assert_eq!(destination[0].r.to_bits(), 77.0_f32.to_bits());
}

/// A 4:2:2 destination holding an odd number of pixels splits a chroma pair.
#[test]
fn an_odd_pixel_count_is_refused_for_subsampled_chroma() {
    let transform = require_ok!(native_interleaved(PhotometricInterpretation::YbrFull422));
    let mut destination = [Rgb::BLACK; 3];
    assert_eq!(
        transform.map_into(&stored(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]), &mut destination),
        Err(PixelError::SubsampledChromaAlignment)
    );
}
