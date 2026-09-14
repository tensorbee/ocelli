//! HLD section 18.4's uniform, as a host-side type.
//!
//! These need no GPU. They assert the LAYOUT and the parameter extraction, and
//! `tests/voi_shader.rs` asserts that a shader reading that layout computes what
//! `ocelli-pixel` computes.
//!
//! Section 18.4, transcribed:
//!
//! ```wgsl
//! struct VoiParams {
//!     center : f32,
//!     width : f32,
//!     slope : f32,
//!     intercept : f32,
//!     ymin : f32,
//!     ymax : f32,
//!     fn_kind : u32, // 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID
//!     invert : u32,
//! };
//! @group(0) @binding(0) var<uniform> voi : VoiParams;
//! ```

use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence,
    PresentationLutShape, VoiFunction, VoiTransform,
};
use ocelli_render::{VOI_WGSL, VoiParams, VoiParamsError};

/// The soft-tissue CT chain HLD section 18.3 is written against, with a
/// rescale that is not the identity so `slope` and `intercept` are separable.
fn soft_tissue(photometric: PhotometricInterpretation, function: VoiFunction) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(-1024.0));
    assert!(modality.is_ok());
    let Ok(modality) = modality else {
        unreachable!("checked immediately above")
    };
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, function, 0.0, 255.0);
    assert!(voi.is_ok());
    let Ok(voi) = voi else {
        unreachable!("checked immediately above")
    };
    let chain = LutChain::new(modality, voi, photometric, PresentationLutEvidence::Absent);
    assert!(chain.is_ok());
    match chain {
        Ok(chain) => chain,
        Err(_) => unreachable!("checked immediately above"),
    }
}

/// **Thirty-two bytes, and the eight offsets section 18.4's field order
/// implies.**
///
/// HLD section 26: "Prefer a uniform update to a texture update. Window/level
/// is thirty-two bytes, not a re-upload." That sentence is only true if this
/// struct is thirty-two bytes, so the performance claim and the layout are the
/// same assertion.
///
/// The offsets are written as literals rather than computed, because computing
/// them from the struct would assert the layout against itself.
#[test]
fn voi_params_is_thirty_two_bytes_in_section_18_4_field_order() {
    assert_eq!(core::mem::size_of::<VoiParams>(), 32);
    assert_eq!(core::mem::align_of::<VoiParams>(), 4);

    let probe = VoiParams {
        center: 1.0,
        width: 2.0,
        slope: 3.0,
        intercept: 4.0,
        ymin: 5.0,
        ymax: 6.0,
        fn_kind: 7,
        invert: 8,
    };
    let bytes: &[u8] = bytemuck::bytes_of(&probe);
    assert_eq!(bytes.len(), 32);

    // Raw bytes rather than decoded floats, so this is an exact layout
    // assertion with no float comparison in it at all. `float_cmp` is denied at
    // the workspace and it is right to be: a layout test that compared decoded
    // values would be asking an equality question about arithmetic when what it
    // means to ask is a question about offsets.
    let at = |offset: usize| -> Option<&[u8]> { bytes.get(offset..offset + 4) };
    assert_eq!(
        at(0),
        Some(&1.0_f32.to_ne_bytes()[..]),
        "center is not at 0"
    );
    assert_eq!(at(4), Some(&2.0_f32.to_ne_bytes()[..]), "width is not at 4");
    assert_eq!(at(8), Some(&3.0_f32.to_ne_bytes()[..]), "slope is not at 8");
    assert_eq!(
        at(12),
        Some(&4.0_f32.to_ne_bytes()[..]),
        "intercept is not at 12"
    );
    assert_eq!(
        at(16),
        Some(&5.0_f32.to_ne_bytes()[..]),
        "ymin is not at 16"
    );
    assert_eq!(
        at(20),
        Some(&6.0_f32.to_ne_bytes()[..]),
        "ymax is not at 20"
    );
    assert_eq!(
        at(24),
        Some(&7_u32.to_ne_bytes()[..]),
        "fn_kind is not at 24"
    );
    assert_eq!(
        at(28),
        Some(&8_u32.to_ne_bytes()[..]),
        "invert is not at 28"
    );
}

/// `fn_kind` is section 18.4's own numbering, taken from its comment:
/// `// 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID`.
///
/// All three named. A mapping that returned a constant satisfies any one of
/// them alone.
#[test]
fn fn_kind_is_section_18_4s_own_numbering() {
    let kind = |function| {
        VoiParams::from_chain(&soft_tissue(
            PhotometricInterpretation::Monochrome2,
            function,
        ))
        .map(|params| params.fn_kind)
    };
    assert_eq!(kind(VoiFunction::Linear), Ok(0));
    assert_eq!(kind(VoiFunction::LinearExact), Ok(1));
    assert_eq!(kind(VoiFunction::Sigmoid), Ok(2));
}

/// **Every field of the uniform, hand-written from the chain that produced
/// it.**
///
/// Centre 40, width 400, rescale slope 1 and intercept -1024, `MONOCHROME2`,
/// output range 0 to 255, `LINEAR`. Nothing here is read back from
/// `ocelli-pixel`: each figure is what the chain was constructed with, so a
/// `from_chain` that read the wrong field is caught by the value being a
/// different one of the six.
///
/// The six scalars are deliberately all distinct, which is why the intercept is
/// -1024 rather than 0: a transposition of two fields both holding 0 is
/// invisible.
#[test]
fn from_chain_reads_section_18_4s_eight_parameters() {
    let params = VoiParams::from_chain(&soft_tissue(
        PhotometricInterpretation::Monochrome2,
        VoiFunction::Linear,
    ));
    assert_eq!(
        params,
        Ok(VoiParams {
            center: 40.0,
            width: 400.0,
            slope: 1.0,
            intercept: -1024.0,
            ymin: 0.0,
            ymax: 255.0,
            fn_kind: 0,
            invert: 0,
        })
    );
}

/// **`MONOCHROME1` changes `invert` and CHANGES NOTHING ELSE.**
///
/// This is the whole of what F-029 built and the reason the shader cannot
/// double-invert: inversion reaches the uniform through one resolved `u32` and
/// through no other field. A `from_chain` that also flipped `ymin` and `ymax`,
/// or negated the slope, would be a second expression of the same intent, and
/// the two would compose.
///
/// PS3.3 C.7.6.3.1.2 and C.11.6, through `LutChain::inverts`.
#[test]
fn monochrome1_sets_invert_and_moves_no_other_field() {
    let two = VoiParams::from_chain(&soft_tissue(
        PhotometricInterpretation::Monochrome2,
        VoiFunction::Linear,
    ));
    let one = VoiParams::from_chain(&soft_tissue(
        PhotometricInterpretation::Monochrome1,
        VoiFunction::Linear,
    ));

    assert_eq!(two.map(|p| p.invert), Ok(0));
    assert_eq!(one.map(|p| p.invert), Ok(1));

    let (Ok(two), Ok(one)) = (two, one) else {
        unreachable!("both asserted Ok immediately above")
    };
    assert_eq!(
        VoiParams { invert: 0, ..one },
        two,
        "MONOCHROME1 moved a field other than invert"
    );
}

/// An explicit `IDENTITY` on a `MONOCHROME1` frame does not invert.
///
/// F-029's resolution rule is an override rather than a composition, and this
/// is that decision arriving at the uniform. A reviewer who expects
/// `MONOCHROME1` to always invert will read it as a bug, so it is asserted
/// here rather than left to the CPU's own suite.
#[test]
fn an_explicit_identity_on_monochrome1_does_not_invert() {
    let modality = ModalityTransform::new(None, Some(1.0), Some(-1024.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 0.0, 255.0);
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let chain = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Shape(PresentationLutShape::Identity),
    );
    assert!(chain.is_ok());
    let Ok(chain) = chain else { return };

    assert_eq!(VoiParams::from_chain(&chain).map(|p| p.invert), Ok(0));
}

/// **A Modality LUT Sequence has no uniform to be expressed in, so it reports
/// unavailable.**
///
/// Section 18.4's struct carries `slope` and `intercept` and has no field for a
/// sequence, and section 18's stage table says a sequence TAKES PRECEDENCE over
/// rescale. Substituting the rescale values would answer a different question
/// with numbers that look right, which is HLD section 31's rule generalised by
/// D-07.
#[test]
fn a_modality_lut_sequence_is_not_expressible_in_the_uniform() {
    let lut = LutDescriptor::new(3, -1, 16, vec![5.0, 15.0, 25.0]);
    assert!(lut.is_ok());
    let Ok(lut) = lut else { return };
    let modality = ModalityTransform::new(Some(lut), Some(1.0), Some(-1024.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 0.0, 255.0);
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let chain = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(chain.is_ok());
    let Ok(chain) = chain else { return };

    assert_eq!(
        VoiParams::from_chain(&chain),
        Err(VoiParamsError::ModalitySequenceNotExpressible)
    );
}

/// A VOI LUT Sequence, the same rule on the other stage.
///
/// A separate error variant from the modality one, because "which stage cannot
/// be expressed" is what a caller reporting the feature unavailable has to say.
#[test]
fn a_voi_lut_sequence_is_not_expressible_in_the_uniform() {
    let lut = LutDescriptor::new(3, -1, 16, vec![5.0, 15.0, 25.0]);
    assert!(lut.is_ok());
    let Ok(lut) = lut else { return };
    let modality = ModalityTransform::new(None, Some(1.0), Some(-1024.0));
    let voi = VoiTransform::new(
        Some(lut),
        &[40.0],
        &[400.0],
        0,
        VoiFunction::Linear,
        0.0,
        255.0,
    );
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let chain = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(chain.is_ok());
    let Ok(chain) = chain else { return };

    assert_eq!(
        VoiParams::from_chain(&chain),
        Err(VoiParamsError::VoiSequenceNotExpressible)
    );
}

/// The shader declares section 18.4's struct, its eight fields in order, and
/// its binding.
///
/// **The search is confined to the struct BODY**, and the first version of this
/// test was not. It scanned the whole file with a moving cursor, and the file's
/// prose header names every field before the struct does, so reordering
/// `ymin` and `ymax` in the declaration left it green. The F-041 review's first
/// pass measured that on three different swaps. Under deviation D-04 the CI
/// floor has no adapter, so this is the only check on the WGSL that runs there,
/// and a layout guard defeated by a comment is worse than none.
///
/// It is still weak: it reads text and compiles nothing. The real evidence is
/// `tests/voi_shader.rs`, where a reorder is red on a real device.
#[test]
fn the_shader_declares_section_18_4s_uniform() {
    assert!(VOI_WGSL.contains("struct VoiParams"));
    assert!(VOI_WGSL.contains("@group(0) @binding(0) var<uniform> voi : VoiParams;"));

    let opened = VOI_WGSL.find("struct VoiParams {");
    assert!(opened.is_some(), "the shader declares no VoiParams struct");
    let Some(opened) = opened else { return };
    let rest = VOI_WGSL.get(opened..);
    assert!(rest.is_some());
    let Some(rest) = rest else { return };
    let closed = rest.find('}');
    assert!(closed.is_some(), "the VoiParams struct is not closed");
    let Some(closed) = closed else { return };
    let body = rest.get(..closed);
    assert!(body.is_some());
    let Some(body) = body else { return };

    // The eight fields, in section 18.4's order, from a cursor that only moves
    // forward, so a declaration in a different order fails even though every
    // name is present.
    let mut cursor = 0;
    for field in [
        "center",
        "width",
        "slope",
        "intercept",
        "ymin",
        "ymax",
        "fn_kind",
        "invert",
    ] {
        let remaining = body.get(cursor..);
        assert!(remaining.is_some());
        let Some(remaining) = remaining else { return };
        let found = remaining.find(field).map(|offset| cursor + offset);
        assert!(
            found.is_some(),
            "the struct body does not declare {field} in section 18.4's order"
        );
        if let Some(found) = found {
            cursor = found + field.len();
        }
    }
}

/// **`ymin` and `ymax` come from the VOI stage's own range, not from a
/// constant.**
///
/// Every other test here uses a `[0, 255]` range, so hardcoding `(0.0, 255.0)`
/// inside `from_chain` leaves them all green. The F-041 review's first pass
/// measured that. PS3.3 C.11.2.1.1 lets a VOI LUT Sequence declare
/// `0 ..= 2^bits - 1`, and a window declares whatever it was validated with, so
/// a constant here would be wrong for both.
///
/// It matters beyond the two fields: `ymin` and `ymax` are also what the
/// shader's inversion reflects about, so a uniform carrying the wrong range
/// inverts about the wrong midpoint.
#[test]
fn ymin_and_ymax_come_from_the_voi_stages_declared_range() {
    let modality = ModalityTransform::new(None, Some(1.0), Some(-1024.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 16.0, 235.0);
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let chain = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(chain.is_ok());
    let Ok(chain) = chain else { return };

    let params = VoiParams::from_chain(&chain);
    assert_eq!(params.map(|p| p.ymin.to_bits()), Ok(16.0_f32.to_bits()));
    assert_eq!(params.map(|p| p.ymax.to_bits()), Ok(235.0_f32.to_bits()));
}
