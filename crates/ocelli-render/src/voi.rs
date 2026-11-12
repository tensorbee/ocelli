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
use ocelli_core::Stored;
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

/// Texels per folded modality table row.
///
/// The fold is the width the pinned wgpu limits force: a 16-bit container is
/// 65,536 entries, a single row that wide exceeds every named
/// `max_texture_dimension_2d`, and 256 fits the 2048 downlevel floor with
/// 1024 bytes per `R32Float` row, already `COPY_BYTES_PER_ROW_ALIGNMENT`.
pub const MODALITY_FOLD: u32 = 256;

/// How a viewport's stage 1 reaches the shader.
///
/// `Tabled` is only legal for a whole integer container, because the
/// table's index is the container's own value. A legal range covers every
/// value the container can hold: `count` at least 1 and at most 65,536, and every covered
/// value inside the union of the signed and unsigned 16-bit domains,
/// -32,768 to 65,535. That expresses every integer DICOM container, 8, 12
/// or 16 bits, signed or unsigned, and keeps the builder's conversion
/// through `u16` or `i16` exact, since every accepted value fits one of
/// them. The two ranges the volume ray derives from `brick_format`, `(0, 256)`
/// for `R8Uint` and `(0, 65536)` for `R16Uint`, are the common cases, and
/// a 12-bit stack declares `(0, 4096)`. Any other range is refused at
/// registration, including `count` 0, which the shared check's decision
/// unit pins because `div_ceil(0)` would otherwise reach `create_texture`
/// as a zero extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredRoute {
    /// Integer stored data. `first` is the container's smallest value and
    /// `count` its width. The graph builds and owns the folded table. The
    /// table is `fold` texels wide and `count.div_ceil(fold)` high, so a
    /// count that is not a multiple of the fold is accepted and its last
    /// row is padded to the fold with entry values the validated index
    /// range never reaches.
    Tabled {
        /// The container's smallest stored value.
        first: i32,
        /// The container's width in values.
        count: u32,
    },
    /// Everything else, including F32, any width above 16 bits, and every
    /// interpolated route. Stage 1 is applied on the CPU before the bytes
    /// are written, and the uniform's rescale is written as identity.
    PreApplied,
}

/// The 16-byte route uniform, `voi.wgsl`'s `ModalityRoute`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ModalityRoute {
    /// 1 reads the folded table, 0 returns its input unchanged.
    pub tabled: u32,
    /// The container's smallest stored value.
    pub first: i32,
    /// Texels per table row. The fold width, 256.
    pub fold: u32,
    pub pad: u32,
}

/// Why a tabled declaration or fill was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RouteError {
    /// The declared count is zero, above 65,536, or its covered values
    /// leave the union of the signed and unsigned 16-bit domains.
    #[error("the declared container range is empty or leaves the accepted 16-bit union")]
    RangeOutside,
    /// The pass stages its own pre-applied samples or composes no VOI
    /// text, so a whole-integer container has nothing to serve.
    #[error("this pass cannot serve a tabled declaration")]
    PassRefusesTable,
    /// The route declares no container, so there is nothing to fill.
    #[error("a pre-applied route has no container to fill a table from")]
    NotTabled,
    /// The destination buffer is shorter than the declared container.
    #[error("the table buffer is shorter than the declared container")]
    BufferShort,
}

/// The largest container width a table may cover, `stored_sample_table`'s
/// own limit: more than 65,536 entries is refused.
const TABLE_MAX_ENTRIES: u32 = 65_536;

/// The union of the signed and unsigned 16-bit domains' smallest value.
const UNION_FIRST: i32 = -32_768;

/// The union of the signed and unsigned 16-bit domains' largest value.
const UNION_LAST: i32 = 65_535;

/// Whether a `Tabled` declaration covers a legal whole-integer container.
///
/// `count` 0 is refused, because `count.div_ceil(fold)` of 0 is a zero
/// texture extent and `create_texture` refuses that. A count above
/// 65,536 is refused on `stored_sample_table`'s own limit. A covered
/// value outside -32,768 to 65,535 is refused, because the builder's
/// conversion through `u16` or `i16` must stay exact. A `PreApplied`
/// route declares no container and is legal everywhere.
///
/// # Errors
///
/// [`RouteError::RangeOutside`] for an illegal container.
pub(crate) fn check_container(route: StoredRoute) -> Result<(), RouteError> {
    let StoredRoute::Tabled { first, count } = route else {
        return Ok(());
    };
    if count == 0 || count > TABLE_MAX_ENTRIES {
        return Err(RouteError::RangeOutside);
    }
    let span = i64::from(count) - 1;
    let last = i64::from(first) + span;
    if first < UNION_FIRST || last > i64::from(UNION_LAST) {
        return Err(RouteError::RangeOutside);
    }
    Ok(())
}

/// Fill a folded modality table with `ocelli_pixel::modality`'s own outputs.
///
/// Entry `i` of `out` receives `modality(Stored(container_value), slope,
/// intercept).0` for the container value `first + i`, walked as `i32`.
/// The conversion to `f32` goes through the 16-bit types,
/// `f32::from(u16::try_from(value))` at zero and above and
/// `f32::from(i16::try_from(value))` below, exact for every value in the
/// accepted union: `std` has no `From<i32>` for `f32`, and a direct `as`
/// is a denied truncation cast. The buffer's tail beyond `count`, the
/// padded last row's unused texels when `count` is not a multiple of the
/// fold, is filled with `0.0` and is reachable by no validated index.
/// The builder calls `ocelli_pixel::modality`, it does not re-derive the
/// formula.
///
/// # Errors
///
/// [`RouteError::NotTabled`] for a `PreApplied` route, which has no
/// container to walk. [`RouteError::RangeOutside`] for a container
/// outside the accepted union. [`RouteError::BufferShort`] when `out`
/// cannot hold the declared count.
pub fn fill_modality_table(
    route: StoredRoute,
    slope: f32,
    intercept: f32,
    out: &mut [f32],
) -> Result<(), RouteError> {
    let StoredRoute::Tabled { first, count } = route else {
        return Err(RouteError::NotTabled);
    };
    check_container(route)?;
    let count_us = usize::try_from(count).map_err(|_| RouteError::BufferShort)?;
    if out.len() < count_us {
        return Err(RouteError::BufferShort);
    }
    for (index, slot) in out.iter_mut().enumerate().take(count_us) {
        let offset = i32::try_from(index).map_err(|_| RouteError::RangeOutside)?;
        let value = first.checked_add(offset).ok_or(RouteError::RangeOutside)?;
        let container = if value >= 0 {
            f32::from(u16::try_from(value).map_err(|_| RouteError::RangeOutside)?)
        } else {
            f32::from(i16::try_from(value).map_err(|_| RouteError::RangeOutside)?)
        };
        *slot = ocelli_pixel::modality(Stored(container), slope, intercept).0;
    }
    for slot in out.iter_mut().skip(count_us) {
        *slot = 0.0;
    }
    Ok(())
}

impl VoiParams {
    /// The same thirty-two bytes with the rescale written as identity.
    ///
    /// A viewport whose texture already holds modality values draws with
    /// this uniform: the shader's stage 1 returns its input unchanged, so
    /// a surviving rescale would apply stage 1 twice. `center`, `width`,
    /// `ymin`, `ymax`, `fn_kind` and `invert` are untouched, because a
    /// pre-applied upload changes nothing about the window.
    #[must_use]
    pub fn with_identity_rescale(self) -> Self {
        Self {
            slope: 1.0,
            intercept: 0.0,
            ..self
        }
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

    /// **The shader carries no entry point, and reserves exactly bindings
    /// 0, 1 and 2.**
    ///
    /// Its header says so and this is the assertion behind that sentence. An
    /// entry point added here would make the file compile on its own and stop
    /// composing, and a second binding beyond the reserved set would collide
    /// with the consumer's. Binding 0 is section 18.4's uniform. Bindings 1
    /// and 2 are the route uniform and the folded modality table, which the
    /// header documents as reserved by this file. Bindings 3 upward belong to
    /// the consumer.
    #[test]
    fn the_shader_is_composable_and_reserves_bindings_zero_one_and_two() {
        assert!(
            !VOI_WGSL.contains("@vertex")
                && !VOI_WGSL.contains("@fragment")
                && !VOI_WGSL.contains("@compute"),
            "voi.wgsl grew an entry point, so it no longer composes"
        );
        assert!(!VOI_WGSL.contains("var<storage"));
        // No binding above the reserved set, not merely no binding 3: a
        // `@binding(4)` would collide with the consumer just the same. The
        // reserved set is 0, the section 18.4 uniform, and 1 and 2, the
        // route uniform and the folded table.
        let highest = VOI_WGSL
            .match_indices("@binding(")
            .filter_map(|(at, _)| {
                let rest = VOI_WGSL.get(at + "@binding(".len()..)?;
                let end = rest.find(')')?;
                rest[..end].parse::<u32>().ok()
            })
            .max()
            .unwrap_or(0);
        assert!(
            highest <= 2,
            "voi.wgsl binds {highest}, past the reserved 0, 1 and 2"
        );
        // The DECLARATIONS, not the binding attributes, which the file's own
        // header also names in prose when it explains what it reserves.
        assert_eq!(
            VOI_WGSL.matches("var<uniform> voi : VoiParams;").count(),
            1,
            "voi.wgsl declares the section 18.4 uniform other than once"
        );
        assert_eq!(
            VOI_WGSL
                .matches("var<uniform> modality_route : ModalityRoute;")
                .count(),
            1,
            "voi.wgsl declares the route uniform other than once"
        );
        assert_eq!(
            VOI_WGSL
                .matches("var modality_table : texture_2d<f32>;")
                .count(),
            1,
            "voi.wgsl declares the folded table other than once"
        );
    }

    /// **The count-0 refusal, and the accepted union's edges.**
    ///
    /// `count` 0 must refuse before `div_ceil` reaches `create_texture` as a
    /// zero extent, which wgpu refuses with `TextureDimensionError::Zero`
    /// rather than clipping. The union is -32,768 to 65,535, and a covered
    /// value outside it would make the builder's `u16` or `i16` conversion
    /// inexact.
    #[test]
    fn the_container_check_refuses_count_zero_and_ranges_outside_the_union() {
        use super::{StoredRoute, check_container};
        assert_eq!(
            check_container(StoredRoute::Tabled { first: 0, count: 0 }),
            Err(super::RouteError::RangeOutside),
            "count 0 would reach create_texture as a zero extent"
        );
        assert_eq!(
            check_container(StoredRoute::Tabled {
                first: 0,
                count: 65_537
            }),
            Err(super::RouteError::RangeOutside),
            "more than 65,536 entries is stored_sample_table's own refusal"
        );
        assert_eq!(
            check_container(StoredRoute::Tabled {
                first: -32_769,
                count: 1
            }),
            Err(super::RouteError::RangeOutside),
            "one step below the signed 16-bit domain"
        );
        assert_eq!(
            check_container(StoredRoute::Tabled {
                first: 65_536,
                count: 1
            }),
            Err(super::RouteError::RangeOutside),
            "one step above the unsigned 16-bit domain"
        );
        assert_eq!(
            check_container(StoredRoute::Tabled {
                first: i32::MAX - 1,
                count: 4
            }),
            Err(super::RouteError::RangeOutside),
            "a covered last value that would overflow the walk is refused"
        );
        // The accepted edges and the common shapes.
        assert!(
            check_container(StoredRoute::Tabled {
                first: -32_768,
                count: 65_536
            })
            .is_ok()
        );
        assert!(
            check_container(StoredRoute::Tabled {
                first: 0,
                count: 65_536
            })
            .is_ok()
        );
        assert!(
            check_container(StoredRoute::Tabled {
                first: 0,
                count: 4_095
            })
            .is_ok()
        );
        assert!(
            check_container(StoredRoute::Tabled {
                first: 0,
                count: 16
            })
            .is_ok()
        );
        assert!(
            check_container(StoredRoute::Tabled {
                first: -32_768,
                count: 1
            })
            .is_ok()
        );
        assert!(check_container(StoredRoute::PreApplied).is_ok());
    }

    /// **AC17's builder entry, pinned as bits.**
    ///
    /// The pair-B table: slope `0x3D47EF8C`, intercept `0xC52D91AC`. Entry
    /// 2371 holds `0xC52655EC`, `ocelli_pixel::modality`'s own two-rounding
    /// output, and not pair A's 3718.0, which is what a builder that reused
    /// the old pair would leave. PS3.3 C.11.1: output units are m*SV + b,
    /// applied as multiply then add. Hand-computed with struct pack
    /// round-trips: 2371.0 * 0x3D47EF8C is 0x42E777F4, and
    /// 0x42E777F4 + 0xC52D91AC is 0xC52655EC. Entry 459 is where the two
    /// roundings and a fused multiply-add part company: the two-rounding
    /// value is 0xC52C2B32 and the fused value is 0xC52C2B31, so a builder
    /// that contracted would leave the wrong bits there.
    #[test]
    fn the_builder_fills_pair_bs_table_entry_from_ocelli_pixel() {
        use super::{StoredRoute, fill_modality_table};
        let route = StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        };
        let mut table = vec![0.0_f32; 65_536];
        let filled = fill_modality_table(
            route,
            f32::from_bits(0x3D47EF8C),
            f32::from_bits(0xC52D91AC),
            &mut table,
        );
        assert!(filled.is_ok(), "the 16-bit container is legal: {filled:?}");
        let entry = table.get(2371).copied().unwrap_or(f32::NAN);
        assert_eq!(
            entry.to_bits(),
            0xC52655EC,
            "index 2371 is modality(2371) under pair B, not pair A's 3718.0"
        );
        // Entry 0 is the intercept itself: modality(0) = 0 * slope + b.
        let first_entry = table.first().copied().unwrap_or(f32::NAN);
        assert_eq!(
            first_entry.to_bits(),
            f32::from_bits(0xC52D91AC).to_bits(),
            "entry 0 is the intercept under the two-rounding order"
        );
        // The fused-distinguishing entry: two roundings give ...32, a
        // contraction gives ...31.
        let distinguishing = table.get(459).copied().unwrap_or(f32::NAN);
        assert_eq!(
            distinguishing.to_bits(),
            0xC52C2B32,
            "entry 459 is the two-rounding value, not the fused 0xC52C2B31"
        );
    }

    /// **The padded tail and the refusals.**
    ///
    /// A count that is not a multiple of the fold pads its last row with
    /// `0.0`, and no validated index reaches the padding. A short buffer,
    /// an illegal container and a pre-applied route are refused without
    /// writing. Entry 4094 is modality(4094, 2.0, -1024.0) = 7164.0, hand
    /// computed from PS3.3 C.11.1.
    #[test]
    fn the_builder_pads_the_tail_and_refuses_the_rest() {
        use super::{StoredRoute, fill_modality_table};
        let route = StoredRoute::Tabled {
            first: 0,
            count: 4_095,
        };
        let mut table = vec![7.5_f32; 4_096];
        let filled = fill_modality_table(route, 2.0, -1024.0, &mut table);
        assert!(filled.is_ok(), "4095 fits the union: {filled:?}");
        assert_eq!(
            table.get(4_094).copied().unwrap_or(f32::NAN).to_bits(),
            7_164.0_f32.to_bits(),
            "the last real entry is modality(4094) under slope 2, intercept -1024"
        );
        assert_eq!(
            table.get(4_095).copied().unwrap_or(f32::NAN).to_bits(),
            0.0_f32.to_bits(),
            "the padded texel is zero"
        );
        assert_eq!(
            fill_modality_table(
                StoredRoute::Tabled {
                    first: 0,
                    count: 4_095
                },
                2.0,
                -1024.0,
                &mut [0.0_f32; 4_094]
            ),
            Err(super::RouteError::BufferShort)
        );
        assert_eq!(
            fill_modality_table(
                StoredRoute::Tabled { first: 0, count: 0 },
                2.0,
                -1024.0,
                &mut []
            ),
            Err(super::RouteError::RangeOutside)
        );
        assert_eq!(
            fill_modality_table(StoredRoute::PreApplied, 2.0, -1024.0, &mut []),
            Err(super::RouteError::NotTabled)
        );
    }

    /// **The route uniform is 16 bytes and `with_identity_rescale` moves
    /// only the rescale.**
    #[test]
    fn modality_route_is_sixteen_bytes_and_identity_rescale_keeps_the_window() {
        use super::ModalityRoute;
        assert_eq!(std::mem::size_of::<ModalityRoute>(), 16);
        let voi = super::VoiParams {
            center: 40.0,
            width: 400.0,
            slope: 2.0,
            intercept: -1024.0,
            ymin: 0.0,
            ymax: 255.0,
            fn_kind: 1,
            invert: 1,
        };
        let identity = voi.with_identity_rescale();
        assert_eq!(identity.slope.to_bits(), 1.0_f32.to_bits());
        assert_eq!(identity.intercept.to_bits(), 0.0_f32.to_bits());
        assert_eq!(identity.center.to_bits(), voi.center.to_bits());
        assert_eq!(identity.width.to_bits(), voi.width.to_bits());
        assert_eq!(identity.ymin.to_bits(), voi.ymin.to_bits());
        assert_eq!(identity.ymax.to_bits(), voi.ymax.to_bits());
        assert_eq!(identity.fn_kind, voi.fn_kind);
        assert_eq!(identity.invert, voi.invert);
        assert_eq!(std::mem::size_of::<super::VoiParams>(), 32);
    }

    /// **Tier B legality, as far as a text check can carry it.**
    ///
    /// HLD section 7 gives tier B fragment shaders only, no compute and no
    /// storage buffers. This file has none of those. The text check does not
    /// execute the shader. The WebGL2 execution is the Playwright probe
    /// recorded in `docs/lld/render-graph.md`.
    #[test]
    fn the_shader_uses_nothing_tier_b_lacks() {
        assert!(!VOI_WGSL.contains("@compute"));
        assert!(!VOI_WGSL.contains("var<storage"));
        assert!(!VOI_WGSL.contains("atomic"));
        assert!(!VOI_WGSL.contains("workgroup"));
    }

    /// The fragment variant includes `VOI_WGSL` unchanged.
    ///
    /// One definition of each window function. The entry point samples with
    /// `textureLoad`. There is no compute entry and no storage buffer, and
    /// the three functions are not re-typed in the graph module.
    #[test]
    fn the_tier_b_module_includes_voi_unchanged() {
        let module = crate::graph::tier_b_fragment_module();
        assert!(module.starts_with(VOI_WGSL));
        assert_eq!(module.matches("fn voi_linear(").count(), 1);
        assert_eq!(module.matches("fn voi_linear_exact(").count(), 1);
        assert_eq!(module.matches("fn voi_sigmoid(").count(), 1);
        assert!(module.contains("@fragment"));
        assert!(module.contains("textureLoad"));
        assert!(!module.contains("@compute"));
        assert!(!module.contains("var<storage"));
    }
}
