//! HLD section 18.4's uniform, and the parameters it reads off a resolved
//! chain.
//!
//! Section 18's instruction is to "implement it once, in ocelli-pixel, and let
//! the shader read the parameters". This module is the reading.
//!
//! **The Rust here computes no LUT value**: every field of [`VoiParams`] is
//! taken from a [`LutChain`] that already resolved it. **The WGSL here does
//! evaluate the three window formulas**, because section 18.4's uniform hands a
//! shader `center`, `width` and `fn_kind`. What neither half does is make a LUT
//! DECISION, and the shader is handed no input from which it could re-make one.
//! `shaders/voi.wgsl` sets that out at the line.
//!
//! **The layout lives here and not in `ocelli-pixel`.** A `#[repr(C)]` uniform
//! is a rendering concern, and `ocelli-pixel` is `no_std`, holds no GPU type
//! and should not learn what a binding is. The division is the one section 18
//! already draws: values there, layout here.

use bytemuck::{Pod, Zeroable};
use ocelli_pixel::{LutChain, VoiFunction};

/// The WGSL this uniform is declared in, composed by a consumer with its own
/// entry point.
///
/// See the file's own header for why it carries no entry point. Exposed so
/// `tests/voi_shader.rs` can append a harness today and F-038 can append a
/// fragment pass later, both over the same text.
pub const VOI_WGSL: &str = include_str!("../shaders/voi.wgsl");

/// Why a resolved chain has no section 18.4 uniform.
///
/// Both arms are the same rule from HLD section 31, generalised by deviation
/// D-07: a feature that cannot run on the resolved path **reports unavailable**
/// and never silently produces a different answer. They are two variants rather
/// than one because "which stage cannot be expressed" is what a caller
/// reporting the feature unavailable has to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VoiParamsError {
    /// Stage 1 is driven by a Modality LUT Sequence.
    ///
    /// Section 18's stage table says a sequence takes precedence over rescale,
    /// and section 18.4's uniform has `slope` and `intercept` and no field for
    /// a sequence. Substituting the rescale values a sequence overrode would
    /// answer a different question with numbers that look right.
    #[error("a Modality LUT Sequence cannot be expressed in HLD section 18.4's uniform")]
    ModalitySequenceNotExpressible,

    /// Stage 2 is driven by a VOI LUT Sequence. The same rule, the other stage.
    #[error("a VOI LUT Sequence cannot be expressed in HLD section 18.4's uniform")]
    VoiSequenceNotExpressible,
}

/// HLD section 18.4's uniform, field for field and in its order.
///
/// ```wgsl
/// struct VoiParams {
///     center : f32,
///     width : f32,
///     slope : f32,
///     intercept : f32,
///     ymin : f32,
///     ymax : f32,
///     fn_kind : u32, // 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID
///     invert : u32,
/// };
/// ```
///
/// **Thirty-two bytes**, which is HLD section 26's claim in the form it can be
/// asserted: "Prefer a uniform update to a texture update. Window/level is
/// thirty-two bytes, not a re-upload." A window-level drag writes this struct
/// and nothing else.
///
/// `Pod` through `bytemuck`, which is HLD section 20's third bullet by name:
/// reinterpret pixel buffers with `bytemuck::cast_slice`, because a
/// hand-written transmute is the construct HLD section 27.2 R5 allow-lists to
/// exactly two files, and neither of them is this one. The derive is how the
/// bytes reach a buffer without one.
///
/// **Every field is public and that is a hazard worth naming.** This type is a
/// transparent mirror of a WGSL block, so private fields with eight accessors
/// would be a wrapper that only forwards, which `AGENTS.md` refuses. The cost
/// is that `VoiParams { width: 0.0, ..built }` compiles and hands the shader a
/// width PS3.3 C.11.2.1.2 forbids.
///
/// **What that produces is the bad kind of wrong.** Measured: at `w = 0` under
/// LINEAR there is no division by zero at all, because `w' = w - 1 = -1`. The
/// bounds invert, `lower` becomes greater than `upper`, and every input clamps,
/// to `ymin` at or below the centre and to `ymax` above it. The frame is a
/// clean two-tone threshold image. It looks like a deliberate rendering choice
/// rather than a defect, which is this project's whole defect class. A NaN
/// would at least be loud, and at `w = 0` only SIGMOID gives one, at exactly
/// `x == c`.
///
/// A NEGATIVE width is the quietest of all. Measured at `w = -400` under
/// SIGMOID, the shader renders a complete, smooth, INVERTED sigmoid: 253.3 at
/// one end, 127.5 at the centre, 1.7 at the other. There is no NaN, no clamp
/// and no discontinuity, and the frame is a photographic negative of the
/// correct one. That is arithmetically exact rather than a loose description:
/// the pairwise sums of the `w = +400` and `w = -400` frames are 255 to within
/// 2e-5. Nothing about it looks like a failure.
///
/// **None of these is reachable through [`VoiParams::from_chain`]**, because
/// `VoiTransform::new` refuses every width outside its function's domain before
/// a chain exists. That is the whole of the mitigation: the type cannot stop a
/// caller assembling a uniform field by field, and the constructor that reads a
/// validated chain is the one every test and every caller uses. Constructing
/// one field by field is constructing a uniform nothing validated.
///
/// The WGSL uniform address space rounds a struct's size up to a multiple of
/// 16. Thirty-two already is one, so the Rust and WGSL layouts agree with no
/// padding field, and
/// `tests/voi_params.rs::voi_params_is_thirty_two_bytes_in_section_18_4_field_order`
/// asserts the eight offsets rather than trusting that.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct VoiParams {
    /// Window Center, the one pair `VoiTransform` selected.
    pub center: f32,
    /// Window Width, validated against its function's domain.
    pub width: f32,
    /// Rescale Slope.
    pub slope: f32,
    /// Rescale Intercept.
    pub intercept: f32,
    /// The VOI stage's declared minimum output value.
    pub ymin: f32,
    /// The VOI stage's declared maximum output value.
    pub ymax: f32,
    /// Section 18.4's own numbering: 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID.
    pub fn_kind: u32,
    /// The single resolved inversion flag, from [`LutChain::inverts`].
    ///
    /// **Resolved on the CPU, exactly once.** The shader receives this and no
    /// Photometric Interpretation, so it cannot recompute inversion and cannot
    /// compose a second one. That is the double-inversion defect F-029 spent a
    /// story preventing, made unreachable by what the uniform does not carry.
    pub invert: u32,
}

impl VoiParams {
    /// Read section 18.4's eight parameters off a resolved chain.
    ///
    /// **Computes nothing.** Every value is taken from state the chain already
    /// holds, through accessors that add no arithmetic.
    ///
    /// `ymin` and `ymax` come from the VOI stage's own `output_range`, which is
    /// the same range [`LutChain::new`] built the presentation stage with. That
    /// is what stops the uniform carrying one range while `invert` was resolved
    /// about another, which would make the shader's reflection land somewhere
    /// the CPU's does not.
    ///
    /// # Errors
    ///
    /// [`VoiParamsError`], when either stage is driven by a LUT Sequence.
    pub fn from_chain(chain: &LutChain) -> Result<Self, VoiParamsError> {
        let (slope, intercept) = chain
            .modality()
            .rescale()
            .ok_or(VoiParamsError::ModalitySequenceNotExpressible)?;
        let (center, width, function) = chain
            .voi()
            .window()
            .ok_or(VoiParamsError::VoiSequenceNotExpressible)?;
        let (ymin, ymax) = chain.voi().output_range();

        Ok(Self {
            center,
            width,
            slope,
            intercept,
            ymin,
            ymax,
            fn_kind: fn_kind(function),
            invert: u32::from(chain.inverts()),
        })
    }
}

/// Section 18.4's own numbering, from its comment on the field.
///
/// **A total match with no wildcard arm.** A fourth `VoiFunction` would stop
/// this compiling, which is the outcome this project wants: a wildcard would
/// give a new function LINEAR's number and the shader would evaluate the wrong
/// formula, which is the quietly-wrong-pixel defect rather than a crash.
const fn fn_kind(function: VoiFunction) -> u32 {
    match function {
        VoiFunction::Linear => 0,
        VoiFunction::LinearExact => 1,
        VoiFunction::Sigmoid => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{VOI_WGSL, fn_kind};
    use ocelli_pixel::VoiFunction;

    /// The numbering, asserted against section 18.4's comment rather than
    /// against the shader's own branch order.
    #[test]
    fn fn_kind_is_zero_one_two_in_section_18_4s_order() {
        assert_eq!(fn_kind(VoiFunction::Linear), 0);
        assert_eq!(fn_kind(VoiFunction::LinearExact), 1);
        assert_eq!(fn_kind(VoiFunction::Sigmoid), 2);
    }

    /// **The shader carries no entry point and no binding but section 18.4's.**
    ///
    /// Its header says so and this is the assertion behind that sentence. An
    /// entry point added here would make the file compile on its own and stop
    /// composing, and a second binding would collide with the consumer's.
    #[test]
    fn the_shader_is_composable_and_reserves_only_binding_zero() {
        assert!(
            !VOI_WGSL.contains("@vertex")
                && !VOI_WGSL.contains("@fragment")
                && !VOI_WGSL.contains("@compute"),
            "voi.wgsl grew an entry point, so it no longer composes"
        );
        assert!(!VOI_WGSL.contains("@binding(1)"));
        assert!(!VOI_WGSL.contains("var<storage"));
        // The DECLARATION, not the binding attribute, which the file's own
        // header also names in prose when it explains what it reserves.
        assert_eq!(
            VOI_WGSL.matches("var<uniform> voi : VoiParams;").count(),
            1,
            "voi.wgsl declares the section 18.4 uniform other than once"
        );
    }

    /// **Tier B legality, as far as a text check can carry it.**
    ///
    /// HLD section 7 gives tier B "fragment shaders only, no compute, no
    /// storage buffers". This file has none of the three. It is a weak check
    /// and it is the only one available: nothing in this repository has ever
    /// run on tier B, which `docs/sprints/CURRENT_SPRINT.md` names as this
    /// sprint's exposure, and a text assertion is not a downlevel adapter.
    #[test]
    fn the_shader_uses_nothing_tier_b_lacks() {
        assert!(!VOI_WGSL.contains("@compute"));
        assert!(!VOI_WGSL.contains("var<storage"));
        assert!(!VOI_WGSL.contains("atomic"));
        assert!(!VOI_WGSL.contains("workgroup"));
    }
}
