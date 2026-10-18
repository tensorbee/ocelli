//! F-046 window presets, invert and colormap.
//!
//! The four soft-tissue rows are PS3.3 C.11.2.1.2 at centre 40 and width
//! 400, output 0 to 255. Deviation D-13 keeps LINEAR_EXACT at -160 on ymin.
//! The literals are the ones `ocelli-pixel` already locks. This file does
//! not recompute them.

use ocelli_core::{Display, Stored};
use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence,
    PresentationLutShape, VoiFunction, VoiTransform,
};
use ocelli_viewport::presentation::{Colormap, Presentation, PresentationError, WindowPreset};

const TOLERANCE: f32 = 0.001;

fn assert_close(actual: f32, expected: f32) {
    let difference = (actual - expected).abs();
    assert!(
        difference < TOLERANCE,
        "was {actual}, wanted {expected}, difference {difference}"
    );
}

fn rescale() -> ModalityTransform {
    let built = ModalityTransform::new(None, Some(1.0), Some(0.0));
    assert!(built.is_ok());
    match built {
        Ok(modality) => modality,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

fn window(center: f32, width: f32, function: VoiFunction, ymin: f32, ymax: f32) -> VoiTransform {
    let built = VoiTransform::new(None, &[center], &[width], 0, function, ymin, ymax);
    assert!(built.is_ok());
    match built {
        Ok(voi) => voi,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

fn chain(
    modality: ModalityTransform,
    voi: VoiTransform,
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
) -> LutChain {
    let built = LutChain::new(modality, voi, photometric, evidence);
    assert!(built.is_ok());
    match built {
        Ok(chain) => chain,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

fn presentation_of(
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
) -> Presentation {
    let voi = window(0.0, 1.0, VoiFunction::Linear, 0.0, 255.0);
    Presentation::new(
        chain(rescale(), voi, photometric, evidence),
        photometric,
        evidence,
        Colormap::Grey,
    )
}

fn display_of(presentation: &Presentation, stored: f32) -> f32 {
    presentation.chain().apply(Stored(stored)).0
}

#[test]
fn soft_tissue_matches_the_section_18_3_linear_rows() {
    let mut presentation = presentation_of(
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(presentation.apply_preset(WindowPreset::SoftTissue).is_ok());
    // PS3.3 C.11.2.1.2. Input -160 is the LINEAR lower bound, so the
    // comparison clamps it. D-13 keeps the exact row at 0, and this preset
    // is LINEAR, so 40 is not 127.5.
    assert_close(display_of(&presentation, -160.0), 0.0);
    assert_close(display_of(&presentation, 40.0), 127.819_55);
    assert_close(display_of(&presentation, 240.0), 255.0);
    assert_close(display_of(&presentation, -60.0), 63.909_775);
    let centre = display_of(&presentation, 40.0);
    assert!(
        (centre - 127.5).abs() > 0.2,
        "soft tissue at 40 was {centre}, which is the exact row"
    );
    let reported = presentation.chain().voi().window();
    assert_eq!(reported, Some((40.0, 400.0, VoiFunction::Linear)));
}

#[test]
fn lung_and_bone_report_their_plan_windows() {
    let mut lung = Presentation::new(
        chain(
            rescale(),
            window(0.0, 1.0, VoiFunction::Linear, 10.0, 20.0),
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
        ),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    assert!(lung.apply_preset(WindowPreset::Lung).is_ok());
    assert_eq!(
        lung.chain().voi().window(),
        Some((-600.0, 1500.0, VoiFunction::Linear))
    );
    assert_eq!(lung.chain().voi().output_range(), (10.0, 20.0));
    let mut bone = presentation_of(
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(bone.apply_preset(WindowPreset::Bone).is_ok());
    assert_eq!(
        bone.chain().voi().window(),
        Some((300.0, 1500.0, VoiFunction::Linear))
    );
}

#[test]
fn a_sequence_preset_is_refused_and_the_chain_stays() {
    let lut = LutDescriptor::new(3, 0, 8, vec![5.0, 15.0, 25.0]);
    assert!(lut.is_ok());
    let Ok(lut) = lut else {
        unreachable!("asserted immediately above")
    };
    let voi = VoiTransform::new(
        Some(lut),
        &[40.0],
        &[400.0],
        0,
        VoiFunction::Linear,
        0.0,
        255.0,
    );
    assert!(voi.is_ok());
    let Ok(voi) = voi else {
        unreachable!("asserted immediately above")
    };
    let original = chain(
        rescale(),
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    let mut presentation = Presentation::new(
        original.clone(),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    assert!(matches!(
        presentation.apply_preset(WindowPreset::SoftTissue),
        Err(PresentationError::Sequence)
    ));
    assert_eq!(presentation.chain(), &original);
    let modality = ModalityTransform::new(Some(lut_again()), Some(1.0), Some(0.0));
    assert!(modality.is_ok());
    let Ok(modality) = modality else {
        unreachable!("asserted immediately above")
    };
    let original = chain(
        modality,
        window(40.0, 400.0, VoiFunction::Linear, 0.0, 255.0),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    let mut presentation = Presentation::new(
        original.clone(),
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    assert!(matches!(
        presentation.apply_preset(WindowPreset::SoftTissue),
        Err(PresentationError::Sequence)
    ));
    assert_eq!(presentation.chain(), &original);
}

fn lut_again() -> ocelli_pixel::LutDescriptor {
    let built = LutDescriptor::new(3, 0, 8, vec![1.0, 2.0, 3.0]);
    assert!(built.is_ok());
    match built {
        Ok(lut) => lut,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

#[test]
fn invert_is_an_explicit_shape_and_does_not_xor() {
    let mut mono1 = presentation_of(
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    );
    assert!(mono1.chain().inverts());
    assert!(mono1.set_invert(false).is_ok());
    assert!(!mono1.chain().inverts());
    let mut mono2 = presentation_of(
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Shape(PresentationLutShape::Identity),
    );
    assert!(!mono2.chain().inverts());
    assert!(mono2.set_invert(true).is_ok());
    assert!(mono2.chain().inverts());
}

fn stored_window(presentation: &Presentation) -> Option<(f32, f32, VoiFunction)> {
    presentation.chain().voi().window()
}

#[test]
fn set_window_uses_the_voi_domain() {
    let mut presentation = presentation_of(
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    // Width 0.5 is outside LINEAR and inside LINEAR_EXACT and SIGMOID.
    // PS3.3 C.11.2.1.2 requires LINEAR width at least 1. C.11.2.1.3
    // requires a positive width. A refusal leaves the stored window.
    assert_eq!(
        stored_window(&presentation),
        Some((0.0, 1.0, VoiFunction::Linear))
    );
    assert!(matches!(
        presentation.set_window(40.0, 0.0, VoiFunction::Linear),
        Err(PresentationError::InvalidWindow)
    ));
    assert_eq!(
        stored_window(&presentation),
        Some((0.0, 1.0, VoiFunction::Linear))
    );
    assert!(matches!(
        presentation.set_window(40.0, 0.5, VoiFunction::Linear),
        Err(PresentationError::InvalidWindow)
    ));
    assert_eq!(
        stored_window(&presentation),
        Some((0.0, 1.0, VoiFunction::Linear))
    );
    assert!(
        presentation
            .set_window(40.0, 1.0, VoiFunction::Linear)
            .is_ok()
    );
    assert_eq!(
        stored_window(&presentation),
        Some((40.0, 1.0, VoiFunction::Linear))
    );
    assert!(
        presentation
            .set_window(40.0, 1.0, VoiFunction::LinearExact)
            .is_ok()
    );
    assert_eq!(
        stored_window(&presentation),
        Some((40.0, 1.0, VoiFunction::LinearExact))
    );
    assert!(
        presentation
            .set_window(40.0, 0.5, VoiFunction::LinearExact)
            .is_ok()
    );
    assert_eq!(
        stored_window(&presentation),
        Some((40.0, 0.5, VoiFunction::LinearExact))
    );
    assert!(matches!(
        presentation.set_window(40.0, 0.0, VoiFunction::LinearExact),
        Err(PresentationError::InvalidWindow)
    ));
    assert_eq!(
        stored_window(&presentation),
        Some((40.0, 0.5, VoiFunction::LinearExact))
    );
    assert!(
        presentation
            .set_window(40.0, 0.5, VoiFunction::Sigmoid)
            .is_ok()
    );
    assert_eq!(
        stored_window(&presentation),
        Some((40.0, 0.5, VoiFunction::Sigmoid))
    );
    assert!(matches!(
        presentation.set_window(40.0, 0.0, VoiFunction::Sigmoid),
        Err(PresentationError::InvalidWindow)
    ));
    assert_eq!(
        stored_window(&presentation),
        Some((40.0, 0.5, VoiFunction::Sigmoid))
    );
}

#[test]
fn a_window_change_keeps_the_colormap() {
    let mut presentation = presentation_of(
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    presentation.set_colormap(Colormap::Stops);
    assert!(
        presentation
            .set_window(40.0, 400.0, VoiFunction::Linear)
            .is_ok()
    );
    assert_eq!(presentation.colormap(), Colormap::Stops);
}

#[test]
fn the_preset_rows_are_not_copied_from_a_readback() {
    // Modality input 40 through slope 1 is still 40. The display is the
    // VOI stage, not a stored shader byte.
    let mut presentation = presentation_of(
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(presentation.apply_preset(WindowPreset::SoftTissue).is_ok());
    let modality = presentation.chain().modality().apply(Stored(40.0));
    // `Modality` has no `PartialEq`. Slope 1 and intercept 0 keep the bits.
    assert_eq!(modality.0.to_bits(), 40.0_f32.to_bits());
    let displayed = presentation.chain().voi().apply(modality);
    assert_close(displayed.0, 127.819_55);
    let _kept: Display = presentation.chain().apply(Stored(40.0));
}
