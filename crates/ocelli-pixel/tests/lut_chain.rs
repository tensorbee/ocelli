//! DICOM PS3.3 C.11 LUT chain fixtures, stages 1 to 3.
//!
//! Every expected value below is hand-computed from the PS3.3 section named
//! beside it, never from reading `ocelli-pixel`. HLD 27.2 R2.

use ocelli_core::{Display, Stored};
use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation, PixelError,
    PresentationLutEvidence, PresentationLutShape, PresentationTransform, VoiFunction,
    VoiTransform,
};

const TOLERANCE: f32 = 0.001;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(
            result.is_ok(),
            "expected Ok, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return };
        value
    }};
}

#[track_caller]
fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "was {actual}, wanted {expected}"
    );
}

/// Rescale slope 1 and intercept 0, so stage 1 is the identity and a fixture
/// reads as its own Modality value.
///
/// The helpers below return `Result` so their call sites can use
/// `require_ok!`. `unwrap_used`, `expect_used` and `panic` are all denied at
/// the workspace, tests included, so a helper that returns the value directly
/// has no legal way to report its own failure.
fn identity_modality() -> Result<ModalityTransform, PixelError> {
    ModalityTransform::new(None, Some(1.0), Some(0.0))
}

/// Centre 40, width 400, which is HLD section 18.3's soft-tissue CT window.
fn window(function: VoiFunction, ymin: f32, ymax: f32) -> Result<VoiTransform, PixelError> {
    VoiTransform::new(None, &[40.0], &[400.0], 0, function, ymin, ymax)
}

/// Stages 1 to 3 over the section 18.3 window with an identity rescale.
fn chain(
    function: VoiFunction,
    ymin: f32,
    ymax: f32,
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
) -> Result<LutChain, PixelError> {
    LutChain::new(
        identity_modality()?,
        window(function, ymin, ymax)?,
        photometric,
        evidence,
    )
}

#[test]
fn hld_section_18_3_rows_survive_composition_through_the_chain() {
    // The four section 18.3 rows, with D-13 applied to LINEAR_EXACT(-160).
    // Composing stages 1 to 3 with an identity rescale and an IDENTITY
    // presentation must not move any of them.
    for (function, rows) in [
        (
            VoiFunction::Linear,
            [
                (-160.0, 0.0),
                (40.0, 127.819_55),
                (240.0, 255.0),
                (-60.0, 63.909_775),
            ],
        ),
        (
            VoiFunction::LinearExact,
            [(-160.0, 0.0), (40.0, 127.5), (240.0, 255.0), (-60.0, 63.75)],
        ),
    ] {
        let chain = require_ok!(chain(
            function,
            0.0,
            255.0,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
        ));
        assert!(!chain.inverts());
        for (input, expected) in rows {
            assert_close(chain.apply(Stored(input)).0, expected);
        }
    }
}

#[test]
fn monochrome1_inverts_exactly_once_at_the_window_centre() {
    // PS3.3 C.11.2.1.2 at x = 40, c = 40, w = 400, output 0 to 255:
    //   c' = 39.5, w' = 399
    //   y = ((40 - 39.5) / 399 + 0.5) * 255 = 127.819548...
    // PS3.3 C.11.6.1.2 INVERSE reflects within the output range:
    //   y' = ymin + ymax - y = 0 + 255 - 127.819548... = 127.180451...
    // Applying the reflection twice returns 127.819548..., which is a
    // different number, so this row separates one inversion from two.
    const ONCE: f32 = 127.180_45;
    const TWICE: f32 = 127.819_55;

    let chain = require_ok!(chain(
        VoiFunction::Linear,
        0.0,
        255.0,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));

    assert!(chain.inverts());
    assert_close(chain.apply(Stored(40.0)).0, ONCE);
    assert!((ONCE - TWICE).abs() > TOLERANCE);
}

#[test]
fn monochrome1_inverts_exactly_once_away_from_the_window_centre() {
    // The row that separates one inversion from two for BOTH VOI functions.
    // A reflection fixes the midpoint of the output range, here 127.5, and
    // PS3.3 C.11.2.1.3.2 maps the window centre exactly onto it, so a
    // LINEAR_EXACT centre fixture would pass under a double inversion.
    // C.11.2.1.2's `c - 0.5` and `w - 1` move LINEAR off that fixed point,
    // which is why the fixture above works for LINEAR. An input away from the
    // centre avoids the fixed point for both.
    // PS3.3 C.11.2.1.2 at x = -60:
    //   y = ((-60 - 39.5) / 399 + 0.5) * 255 = 63.909774...
    // PS3.3 C.11.6.1.2:
    //   y' = 0 + 255 - 63.909774... = 191.090225...
    const UNINVERTED: f32 = 63.909_775;
    const ONCE: f32 = 191.090_23;

    let chain = require_ok!(chain(
        VoiFunction::Linear,
        0.0,
        255.0,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));

    assert_close(chain.apply(Stored(-60.0)).0, ONCE);
    assert!((ONCE - UNINVERTED).abs() > TOLERANCE);
}

#[test]
fn an_explicit_presentation_shape_decides_alone() {
    // The S09 design round's rule. An explicit Presentation LUT Shape is an
    // override, never a second inversion composed with MONOCHROME1.
    //
    // A reviewer expecting MONOCHROME1 to always invert will read the first
    // case as a bug. It is the presentation state being honoured, which is
    // HLD section 18's stage table, row 3.
    let uninverted = 63.909_775_f32;
    let inverted = 191.090_23_f32;

    for (photometric, evidence, expected, inverts) in [
        (
            PhotometricInterpretation::Monochrome1,
            PresentationLutEvidence::Shape(PresentationLutShape::Identity),
            uninverted,
            false,
        ),
        (
            PhotometricInterpretation::Monochrome1,
            PresentationLutEvidence::Shape(PresentationLutShape::Inverse),
            inverted,
            true,
        ),
        (
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Shape(PresentationLutShape::Inverse),
            inverted,
            true,
        ),
        (
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Shape(PresentationLutShape::Identity),
            uninverted,
            false,
        ),
    ] {
        let chain = require_ok!(chain(
            VoiFunction::Linear,
            0.0,
            255.0,
            photometric,
            evidence,
        ));
        assert_eq!(chain.inverts(), inverts);
        assert_close(chain.apply(Stored(-60.0)).0, expected);
    }
}

#[test]
fn inversion_reflects_within_a_non_zero_output_range() {
    // PS3.3 C.11.2.1.3.2 LINEAR_EXACT at x = -60, c = 40, w = 400,
    // output 16 to 236, so the range is 220:
    //   y = ((-60 - 40) / 400 + 0.5) * 220 + 16
    //     = (-0.25 + 0.5) * 220 + 16 = 0.25 * 220 + 16 = 71
    // PS3.3 C.11.6.1.2:
    //   y' = ymin + ymax - y = 16 + 236 - 71 = 181
    //
    // The common wrong form, `ymax - y`, gives 236 - 71 = 165 here and gives
    // the right answer whenever ymin is zero. That is why this fixture uses a
    // non-zero ymin and an input a quarter of the way through the window.
    const UNINVERTED: f32 = 71.0;
    const INVERTED: f32 = 181.0;
    const YMAX_ONLY: f32 = 165.0;

    let plain = require_ok!(chain(
        VoiFunction::LinearExact,
        16.0,
        236.0,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));
    assert_close(plain.apply(Stored(-60.0)).0, UNINVERTED);

    let inverted = require_ok!(chain(
        VoiFunction::LinearExact,
        16.0,
        236.0,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));
    assert_close(inverted.apply(Stored(-60.0)).0, INVERTED);
    assert!((INVERTED - YMAX_ONLY).abs() > TOLERANCE);
}

#[test]
fn a_voi_lut_sequence_inverts_about_its_descriptor_range() {
    // PS3.3 C.11.2.1.1: a VOI LUT's output range is given by the descriptor's
    // third value, bits per entry. For 16 bits that is 0 to 65535.
    // A stored value of 0 selects the single entry, 1000.
    //   y' = 0 + 65535 - 1000 = 64535
    let lut = require_ok!(LutDescriptor::new(1, 0, 16, vec![1000.0]));
    let voi = require_ok!(VoiTransform::new(
        Some(lut),
        &[40.0],
        &[400.0],
        0,
        VoiFunction::Linear,
        0.0,
        255.0
    ));
    assert_eq!(voi.output_range(), (0.0, 65_535.0));

    let chain = require_ok!(LutChain::new(
        require_ok!(identity_modality()),
        voi,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));
    assert_close(chain.apply(Stored(0.0)).0, 64_535.0);
}

#[test]
fn an_eight_bit_voi_lut_descriptor_inverts_about_255() {
    // PS3.3 C.11.2.1.1 with bits per entry 8, so the range is 0 to 255.
    let lut = require_ok!(LutDescriptor::new(1, 0, 8, vec![10.0]));
    let voi = require_ok!(VoiTransform::new(
        Some(lut),
        &[40.0],
        &[400.0],
        0,
        VoiFunction::Linear,
        0.0,
        255.0
    ));
    assert_eq!(voi.output_range(), (0.0, 255.0));

    let chain = require_ok!(LutChain::new(
        require_ok!(identity_modality()),
        voi,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));
    assert_close(chain.apply(Stored(0.0)).0, 245.0);
}

#[test]
fn the_modality_stage_still_runs_before_the_voi_stage() {
    // PS3.3 C.11.1 then C.11.2.1.3.2, in that order. Stored 2048 with
    // slope 1 and intercept -1024 is 1024 HU, and LINEAR_EXACT at
    // c = 40, w = 400 clamps above c + w/2 = 240, so the result is ymax.
    // Reversing the two stages would give a completely different number,
    // which is what this row is for.
    let modality = require_ok!(ModalityTransform::new(None, Some(1.0), Some(-1024.0)));
    let chain = require_ok!(LutChain::new(
        modality,
        require_ok!(window(VoiFunction::LinearExact, 0.0, 255.0)),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));
    assert_close(chain.apply(Stored(2048.0)).0, 255.0);
    // Stored 1064 is 40 HU, the window centre, which is 127.5 exactly.
    assert_close(chain.apply(Stored(1064.0)).0, 127.5);
}

#[test]
fn colour_photometric_interpretations_refuse_the_presentation_stage() {
    // PS3.3 C.11.6 applies to the greyscale path. A colour frame reaches RGB
    // through stage 4, which this story does not implement, so the honest
    // answer is a refusal rather than an identity that looks like success.
    for photometric in [
        PhotometricInterpretation::PaletteColor,
        PhotometricInterpretation::Rgb,
        PhotometricInterpretation::YbrFull,
        PhotometricInterpretation::YbrFull422,
        PhotometricInterpretation::YbrPartial422,
        PhotometricInterpretation::YbrIct,
        PhotometricInterpretation::YbrRct,
    ] {
        assert_eq!(
            chain(
                VoiFunction::Linear,
                0.0,
                255.0,
                photometric,
                PresentationLutEvidence::Absent,
            ),
            Err(PixelError::PresentationLutNotApplicable)
        );
    }
}

#[test]
fn a_presentation_lut_sequence_reports_unsupported_rather_than_falling_back() {
    // PS3.3 C.11.6 permits an explicit Presentation LUT Sequence (2050,0010).
    // Nothing in the corpus carries one, so it reports unavailable instead of
    // silently applying a shape nobody declared. HLD section 31's rule.
    assert_eq!(
        chain(
            VoiFunction::Linear,
            0.0,
            255.0,
            PhotometricInterpretation::Monochrome1,
            PresentationLutEvidence::Sequence,
        ),
        Err(PixelError::PresentationLutSequenceUnsupported)
    );
}

#[test]
fn the_presentation_stage_refuses_a_malformed_output_range_on_its_own() {
    // `LutChain` cannot reach this, because `VoiTransform::output_range` only
    // returns a validated window range or a descriptor range. The public
    // constructor can, so the refusal is exercised through it rather than left
    // present and unreached.
    for (ymin, ymax) in [
        (255.0_f32, 0.0_f32),
        (f32::NAN, 255.0),
        (0.0, f32::INFINITY),
    ] {
        assert_eq!(
            PresentationTransform::new(
                PresentationLutEvidence::Absent,
                PhotometricInterpretation::Monochrome2,
                ymin,
                ymax,
            ),
            Err(PixelError::InvalidDisplayRange)
        );
    }
}

#[test]
fn the_presentation_stage_reports_the_range_before_the_evidence() {
    // The order of the three refusals is a choice, so it is asserted rather
    // than left to whichever check happens to run first. A malformed range is
    // reported ahead of a colour interpretation, and a colour interpretation
    // ahead of an unsupported sequence, because each is a stronger statement
    // about why this stage cannot run.
    assert_eq!(
        PresentationTransform::new(
            PresentationLutEvidence::Sequence,
            PhotometricInterpretation::Rgb,
            255.0,
            0.0,
        ),
        Err(PixelError::InvalidDisplayRange)
    );
    assert_eq!(
        PresentationTransform::new(
            PresentationLutEvidence::Sequence,
            PhotometricInterpretation::Rgb,
            0.0,
            255.0,
        ),
        Err(PixelError::PresentationLutNotApplicable)
    );
    assert_eq!(
        PresentationTransform::new(
            PresentationLutEvidence::Sequence,
            PhotometricInterpretation::Monochrome2,
            0.0,
            255.0,
        ),
        Err(PixelError::PresentationLutSequenceUnsupported)
    );
}

#[test]
fn map_into_refuses_a_mismatched_destination_before_writing() {
    let chain = require_ok!(chain(
        VoiFunction::Linear,
        0.0,
        255.0,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));

    let mut destination = [Display(77.0)];
    assert_eq!(
        chain.map_into(&[Stored(40.0), Stored(-60.0)], &mut destination),
        Err(PixelError::DestinationLength)
    );
    assert_eq!(destination[0].0.to_bits(), 77.0_f32.to_bits());
}

#[test]
fn map_into_matches_scalar_apply_for_every_element() {
    let chain = require_ok!(chain(
        VoiFunction::Linear,
        0.0,
        255.0,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));

    let source = [
        Stored(-160.0),
        Stored(-60.0),
        Stored(40.0),
        Stored(240.0),
        Stored(1000.0),
    ];
    let mut destination = [Display(0.0); 5];
    assert!(chain.map_into(&source, &mut destination).is_ok());
    for (input, output) in source.iter().zip(destination.iter()) {
        assert_eq!(output.0.to_bits(), chain.apply(*input).0.to_bits());
    }
}

#[test]
fn identity_presentation_leaves_the_voi_result_bit_identical() {
    // The composition adds nothing when the resolved shape is IDENTITY. Not an
    // epsilon comparison: stage 3 must be a no-op rather than an arithmetic
    // round trip that happens to land close.
    let voi = require_ok!(window(VoiFunction::Sigmoid, 0.0, 255.0));
    let chain = require_ok!(LutChain::new(
        require_ok!(identity_modality()),
        voi.clone(),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));

    let modality = require_ok!(identity_modality());
    let mut input = -400.0_f32;
    while input <= 600.0 {
        let staged = voi.apply(modality.apply(Stored(input)));
        assert_eq!(chain.apply(Stored(input)).0.to_bits(), staged.0.to_bits());
        input += 3.5;
    }
}

#[test]
fn inverse_presentation_applied_twice_is_the_identity() {
    // The defining property of a reflection, and the property a double
    // inversion defect would satisfy while a single inversion does not.
    let voi = require_ok!(window(VoiFunction::Linear, 16.0, 236.0));
    let inverted = require_ok!(LutChain::new(
        require_ok!(identity_modality()),
        voi.clone(),
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    ));
    let plain = require_ok!(LutChain::new(
        require_ok!(identity_modality()),
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ));

    let mut input = -400.0_f32;
    while input <= 600.0 {
        let once = inverted.apply(Stored(input)).0;
        let straight = plain.apply(Stored(input)).0;
        // Reflecting `once` again must return `straight`.
        assert_close(16.0 + 236.0 - once, straight);
        input += 7.0;
    }
}
