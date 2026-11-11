//! ICC profile parsing for the matrix and tone-curve subset, ICC.1:2022,
//! and its conversion to sRGB, IEC 61966-2-1.
//!
//! F-240. HLD section 18's stage 4 names ICC beside the palette, and this is
//! that half for the subset the plan names: a display or input profile with
//! data colour space `RGB `, PCS `XYZ ` and all six of `rXYZ`, `gXYZ`,
//! `bXYZ`, `rTRC`, `gTRC` and `bTRC`, each TRC a `curv` or a `para` of
//! function type 0 to 4. Every other shape is refused by name, and a frame
//! whose profile is refused keeps the colour stage's own route byte for
//! byte.
//!
//! A profile that also carries a lookup-table tag (`A2B0` to `A2B2`,
//! `D2B0` to `D2B2`) is refused even when the six matrix tags are present,
//! the operator's answer to F-240's question 2. A profile with a lookup
//! table asks for the lookup-table answer, and applying the matrix would be
//! a different transform from the one the profile asks for.
//!
//! The edition read, before any fixture was written: ICC.1:2022-05, profile
//! version 4.4.0.0, from color.org, for the header of 7.2, the tag table of
//! 7.3, `XYZType` of 10.31, `curveType` of 10.6, `parametricCurveType` of
//! 10.18 with its corrected function set, the number encodings of 4.6 and
//! 4.9, the profile classes of Table 18, the colour spaces of Table 19, the
//! rendering intents of Table 23 and the linearized Bradford cone matrix of
//! Annex E. The sRGB transfer function and matrix are IEC 61966-2-1:1999 as
//! amended, read through the ICC's sRGB characterisation note at
//! registry.color.org, updated April 2015.

use alloc::vec::Vec;

use glam::Vec2;
use ocelli_core::exact_f32_integer;

use crate::color::Rgb;

/// The largest profile this parser reads, in bytes.
///
/// Set from the largest matrix/TRC profile read on the developer host, the
/// 3,144-byte sRGB profile that ships with macOS, and from the corpus, whose
/// one ICC-bearing file is F-239's 44-byte placeholder in
/// `synthetic/sc_rgb_icc.dcm`, which this parser names [`IccError::Malformed`]
/// because it is shorter than a header and a tag table. Sixteen KiB is the
/// next power of two above five times the macOS profile, room for a
/// text-heavy descriptive tag set and still far below the lookup-table
/// profiles this parser refuses by name. A larger profile is
/// [`IccError::TooLarge`] once its header and tag table have named it,
/// before any tag payload is read.
pub const MAX_PROFILE_BYTES: usize = 16_384;

/// Why a profile outside the subset is not applied.
///
/// Each variant names one kind, so a caller can state what a frame asked for
/// and did not get. The refusals are a property of the bytes alone, so a
/// retry over the same bytes parses to the same refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IccError {
    /// The profile class is neither `mntr` nor `scnr`, ICC.1:2022 Table 18.
    UnsupportedClass,
    /// The data colour space is not `RGB `, ICC.1:2022 Table 19.
    UnsupportedColourSpace,
    /// The PCS is not `XYZ `. A `Lab ` PCS is the named case.
    UnsupportedPcs,
    /// The profile carries a lookup-table tag, so the matrix would not be
    /// the transform it asks for.
    LookupTableProfile,
    /// One of the six required tags is missing.
    MissingTag,
    /// A TRC is neither a `curv` nor a `para` of function type 0 to 4.
    UnsupportedCurve,
    /// The bytes do not parse: a short header, a wrong size field, a missing
    /// file signature, a tag outside the profile, or a curve whose entries
    /// do not fit its tag.
    Malformed,
    /// The profile is larger than [`MAX_PROFILE_BYTES`]. Named only after
    /// the header and the tag table have had their say, so a large
    /// lookup-table profile is named [`IccError::LookupTableProfile`].
    TooLarge,
}

/// The header's rendering intent, ICC.1:2022 Table 23.
///
/// The matrix/TRC model has one transform, so the intent is read and
/// recorded and never changes the arithmetic. The conversion is
/// media-relative colorimetric, the PCS the profile already carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderingIntent {
    /// ICC.1:2022 Table 23, value 0.
    Perceptual,
    /// Value 1.
    MediaRelativeColorimetric,
    /// Value 2.
    Saturation,
    /// Value 3.
    IccAbsoluteColorimetric,
}

/// A parsed profile of the subset.
///
/// The tone curves and the colourants are owned. The product of the PCS to
/// sRGB matrix and the colourant matrix is computed once here, at parse, and
/// [`IccTransform`] carries it.
#[derive(Clone, Debug, PartialEq)]
pub struct IccProfile {
    red: ToneCurve,
    green: ToneCurve,
    blue: ToneCurve,
    /// The product, rows by columns, of [`PCS_TO_SRGB`] and the colourant
    /// matrix.
    matrix: [[f32; 3]; 3],
    /// The header's rendering intent field, raw. The four defined values of
    /// Table 23 map onto [`RenderingIntent`]. Anything else is reserved and
    /// stays raw rather than being named as one of them.
    intent: u32,
}

impl IccProfile {
    /// Parse the subset out of a profile's bytes.
    ///
    /// # Errors
    ///
    /// [`IccError`] names the refusal, one kind per shape outside the subset.
    /// A frame whose profile is refused keeps the colour stage's own route.
    pub fn parse(bytes: &[u8]) -> Result<Self, IccError> {
        // 7.2.1: the header is 128 bytes. 7.3.1: the tag table begins with a
        // four-byte count. Anything shorter cannot hold both.
        if bytes.len() < 132 {
            return Err(IccError::Malformed);
        }
        // 7.2.2: the size field shall be the exact profile size. Every ICC.1
        // tag is padded to four bytes, so a conforming profile is a multiple
        // of four and a DICOM element's own even-length pad adds no byte.
        let declared = u32_at(bytes, 0)?;
        if declared != u32::try_from(bytes.len()).map_err(|_| IccError::Malformed)? {
            return Err(IccError::Malformed);
        }
        // 7.2.9: the profile file signature at 36.
        if bytes.get(36..40) != Some(b"acsp") {
            return Err(IccError::Malformed);
        }
        // 7.2.5, Table 18: the subset is a display or an input profile.
        match bytes.get(12..16) {
            Some(b"mntr") | Some(b"scnr") => {}
            _ => return Err(IccError::UnsupportedClass),
        }
        // 7.2.6, Table 19: the device side is RGB.
        if bytes.get(16..20) != Some(b"RGB ") {
            return Err(IccError::UnsupportedColourSpace);
        }
        // 7.2.7: the PCS is XYZ. A Lab PCS is the named refusal.
        if bytes.get(20..24) != Some(b"XYZ ") {
            return Err(IccError::UnsupportedPcs);
        }
        // 7.2.15: the intent, recorded and never applied.
        let intent = u32_at(bytes, 64)?;

        let count = usize::try_from(u32_at(bytes, 128)?).map_err(|_| IccError::Malformed)?;
        let table_end = 132usize
            .checked_add(count.checked_mul(12).ok_or(IccError::Malformed)?)
            .ok_or(IccError::Malformed)?;
        if table_end > bytes.len() {
            return Err(IccError::Malformed);
        }
        let mut tags = Vec::with_capacity(count);
        for index in 0..count {
            let at = 132 + index * 12;
            let signature: [u8; 4] = bytes
                .get(at..at + 4)
                .ok_or(IccError::Malformed)?
                .try_into()
                .map_err(|_| IccError::Malformed)?;
            let offset =
                usize::try_from(u32_at(bytes, at + 4)?).map_err(|_| IccError::Malformed)?;
            let size = usize::try_from(u32_at(bytes, at + 8)?).map_err(|_| IccError::Malformed)?;
            if offset.checked_add(size).is_none_or(|end| end > bytes.len()) {
                return Err(IccError::Malformed);
            }
            tags.push((signature, offset, size));
        }

        // The operator's answer to question 2, before the matrix tags are
        // even looked for, so a profile carrying both is named a
        // lookup-table profile and not a missing-tag one.
        const LOOKUP_TABLE_TAGS: [[u8; 4]; 7] = [
            *b"A2B0", *b"A2B1", *b"A2B2", *b"D2B0", *b"D2B1", *b"D2B2", *b"D2B3",
        ];
        if tags
            .iter()
            .any(|(signature, _, _)| LOOKUP_TABLE_TAGS.contains(signature))
        {
            return Err(IccError::LookupTableProfile);
        }
        // The size refusal applies to what is actually parsed. The header
        // and the tag table above are read from any length, which is what
        // lets a profile too large for the constant be named for what it is,
        // a lookup-table profile above all, before the size is held against
        // it. Everything below reads tag payloads.
        if bytes.len() > MAX_PROFILE_BYTES {
            return Err(IccError::TooLarge);
        }

        let colourants = [
            xyz_colourant(bytes, required_tag(&tags, b"rXYZ")?)?,
            xyz_colourant(bytes, required_tag(&tags, b"gXYZ")?)?,
            xyz_colourant(bytes, required_tag(&tags, b"bXYZ")?)?,
        ];
        let red = trc_curve(bytes, required_tag(&tags, b"rTRC")?)?;
        let green = trc_curve(bytes, required_tag(&tags, b"gTRC")?)?;
        let blue = trc_curve(bytes, required_tag(&tags, b"bTRC")?)?;

        Ok(Self {
            red,
            green,
            blue,
            matrix: product(&PCS_TO_SRGB, &colourants),
            intent,
        })
    }

    /// The largest profile [`Self::parse`] reads.
    #[must_use]
    pub const fn max_bytes() -> usize {
        MAX_PROFILE_BYTES
    }

    /// The header's rendering intent, when it is one of Table 23's four
    /// defined values. The transform never applies it.
    #[must_use]
    pub const fn rendering_intent(&self) -> Option<RenderingIntent> {
        match self.intent {
            0 => Some(RenderingIntent::Perceptual),
            1 => Some(RenderingIntent::MediaRelativeColorimetric),
            2 => Some(RenderingIntent::Saturation),
            3 => Some(RenderingIntent::IccAbsoluteColorimetric),
            _ => None,
        }
    }
}

/// The executable transform: the three tone curves, the one `f32` matrix and
/// the per-code tables built once per held frame.
#[derive(Clone, Debug, PartialEq)]
pub struct IccTransform {
    red: ToneCurve,
    green: ToneCurve,
    blue: ToneCurve,
    /// The product of [`PCS_TO_SRGB`] and the colourant matrix, computed at
    /// parse.
    matrix: [[f32; 3]; 3],
    tables: Option<TrcTables>,
}

impl From<IccProfile> for IccTransform {
    fn from(profile: IccProfile) -> Self {
        Self {
            red: profile.red,
            green: profile.green,
            blue: profile.blue,
            matrix: profile.matrix,
            tables: None,
        }
    }
}

/// One channel's table, over every `2^depth` device code.
#[derive(Clone, Debug, PartialEq)]
struct ChannelTable {
    depth: u16,
    values: Vec<f32>,
}

/// The per-code tables of a held frame.
#[derive(Clone, Debug, PartialEq)]
struct TrcTables {
    red: Option<ChannelTable>,
    green: Option<ChannelTable>,
    blue: Option<ChannelTable>,
}

impl IccTransform {
    /// Per-code tables for the frame's device depths, built once at hold
    /// time. Each channel's table covers its own depth, because a palette's
    /// declared depth can differ per channel. A depth outside 1 to 16 builds
    /// no table and the curve is evaluated continuously, and the colour
    /// stage never produces one of those.
    ///
    /// At most three tables of 65,536 `f32` entries, 768 KiB, which
    /// [`Self::heap_bytes`] measures.
    pub(crate) fn tabulated(mut self, depths: &[u16; 3]) -> Self {
        let red = tabulate(&self.red, depths[0]);
        let green = tabulate(&self.green, depths[1]);
        let blue = tabulate(&self.blue, depths[2]);
        // The per-channel tables are `Option`s: a palette's declared depth
        // can differ per channel, and an impossible depth builds no table.
        self.tables = Some(TrcTables { red, green, blue });
        self
    }

    /// The heap bytes the tables and the table-form curves hold.
    ///
    /// `ColorTransform::heap_bytes` aggregates these bytes with its palette.
    /// The folder viewer charges that total before admitting a held frame.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        let curve = |curve: &ToneCurve| match curve {
            ToneCurve::Table { entries, .. } => entries.capacity() * size_of::<f32>(),
            _ => 0,
        };
        let table = |table: &Option<TrcTables>| {
            table
                .as_ref()
                .map(|tables| {
                    (tables.red.as_ref().map_or(0, |t| t.values.capacity())
                        + tables.green.as_ref().map_or(0, |t| t.values.capacity())
                        + tables.blue.as_ref().map_or(0, |t| t.values.capacity()))
                        * size_of::<f32>()
                })
                .unwrap_or(0)
        };
        curve(&self.red) + curve(&self.green) + curve(&self.blue) + table(&self.tables)
    }

    /// One device triple through the tone curves, the matrix and the sRGB
    /// encoding. `device` holds raw codes and `depths` the encoding depth of
    /// each channel, from 8 to 16, which the colour stage derives from Bits
    /// Stored, the YCbCr matrix's 8-bit range or a palette descriptor's
    /// third value. The reply is display codes at `0 ..= 255` as `f32`: a
    /// device colour outside the sRGB gamut clips to that range and never
    /// leaves it.
    pub(crate) fn apply(&self, device: Rgb, depths: [u16; 3]) -> Rgb {
        let tables = self.tables.as_ref();
        let normalised = Rgb {
            r: channel_value(
                &self.red,
                device.r,
                depths[0],
                tables.and_then(|t| t.red.as_ref()),
            ),
            g: channel_value(
                &self.green,
                device.g,
                depths[1],
                tables.and_then(|t| t.green.as_ref()),
            ),
            b: channel_value(
                &self.blue,
                device.b,
                depths[2],
                tables.and_then(|t| t.blue.as_ref()),
            ),
        };
        let matrix = &self.matrix;
        let linear = [
            matrix[0][0] * normalised.r + matrix[0][1] * normalised.g + matrix[0][2] * normalised.b,
            matrix[1][0] * normalised.r + matrix[1][1] * normalised.g + matrix[1][2] * normalised.b,
            matrix[2][0] * normalised.r + matrix[2][1] * normalised.g + matrix[2][2] * normalised.b,
        ];
        // A.7 of the ICC's sRGB note: "RL, GL or BL values less than 0 or
        // greater than 1 are clipped to 0 and 1", which is what produces
        // "sRGB digital values with a range 0 to 1". ICC.1:2022 Annex F.3
        // clips the linear value before the inverse TRC on the PCS to device
        // side the same way, F.8 to F.16. A wide-gamut profile such as
        // ADOBERGB, a defined term of PS3.3 C.11.15.1.2, maps device colours
        // outside the sRGB gamut, and the display values the colour window
        // and the readout act on are the ones sRGB has.
        Rgb {
            r: encode_srgb(linear[0].clamp(0.0, 1.0)) * 255.0,
            g: encode_srgb(linear[1].clamp(0.0, 1.0)) * 255.0,
            b: encode_srgb(linear[2].clamp(0.0, 1.0)) * 255.0,
        }
    }
}

/// The PCS D50 to linear sRGB matrix.
///
/// The inverse of the D50 colourant matrix that the ICC's sRGB
/// characterisation note publishes for matrix/TRC profiles, the same printed
/// values `tests/icc.rs` encodes as its `R_XYZ`, `G_XYZ` and `B_XYZ`
/// constants. The note derives and prints the device to PCS direction, the
/// colourants themselves: it combines the recommended D65 to D50 chromatic
/// adaptation with the inverse of A.7's matrix from `XN, YN, ZN` to
/// `RL, GL, BL`.
/// ICC.1:2022 Annex F.3 makes the PCS the colourant matrix times the linear
/// device RGB, so the inverse of the published colourants is the PCS to
/// sRGB direction.
/// Inverting the matrix the profiles actually carry keeps the derivation and
/// the input in one source: a D50 PCS white maps to linear `1 1 1` within
/// the printed matrix's own precision, with no second white point of its
/// own. Computed in `f64` and rounded once to `f32`, as `color.rs` rounds
/// its YCbCr inverse. A unit test recomputes the inverse from the printed
/// values and asserts the constant. The exact `f64` values, for the
/// reviewer:
///
/// ```text
/// 3.133923646337816   -1.616922939273852   -0.490733723087733
/// -0.9784210516720576   1.915842665313229    0.03339912699596239
/// 0.07203553396859233  -0.2290320351702708   1.405716157676996
/// ```
const PCS_TO_SRGB: [[f32; 3]; 3] = [
    [3.133_923_5, -1.616_923, -0.490_733_7],
    [-0.978_421_03, 1.915_842_7, 0.033_399_127],
    [0.072_035_536, -0.229_032_04, 1.405_716_2],
];

/// The IEC 61966-2-1 colour component transfer function, the encode side.
///
/// Not tabulated. Its input is the continuous linear output of the matrix,
/// and a table over it would change the values.
fn encode_srgb(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        // `ocelli-pixel` is `no_std`, where scalar `powf` is not available.
        // glam's libm backend is the one `lut.rs` already uses.
        Vec2::splat(linear).powf(1.0 / 2.4).x * 1.055 - 0.055
    }
}

/// The matrix product with the colourants as columns, ICC.1:2022 Annex F.3.
/// `colourants[c]` is the c-th colourant, so the product's entry at row `r`
/// and column `c` is the dot product of `pcs`'s row `r` with that colourant.
fn product(pcs: &[[f32; 3]; 3], colourants: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0_f32; 3]; 3];
    for (out_row, pcs_row) in out.iter_mut().zip(pcs) {
        for (value, colourant) in out_row.iter_mut().zip(colourants) {
            *value = pcs_row.iter().zip(colourant).map(|(p, c)| p * c).sum();
        }
    }
    out
}

/// One tone reproduction curve, ICC.1:2022 10.6 or 10.18.
#[derive(Clone, Debug, PartialEq)]
enum ToneCurve {
    /// A `curv` with a count of zero, an identity response.
    Identity,
    /// A `curv` with a count of one, the gamma as a u8Fixed8Number.
    Gamma(f32),
    /// A `curv` with a count above one. `steps` is the entry count minus
    /// one, the count of interpolation intervals, kept as the `f32` the
    /// interpolation scales by.
    Table { entries: Vec<f32>, steps: f32 },
    /// A `para` of function type 0 to 4.
    Parametric(Parametric),
}

impl ToneCurve {
    /// The curve at `x`, which the caller has clamped into the domain of
    /// 0.0 to 1.0.
    fn evaluate(&self, x: f32) -> f32 {
        let value = match self {
            ToneCurve::Identity => x,
            ToneCurve::Gamma(gamma) => powf(x, *gamma),
            ToneCurve::Table { entries, steps } => interpolate(entries, *steps, x),
            ToneCurve::Parametric(curve) => curve.evaluate(x),
        };
        // 10.6 and 10.18: the domain and the range are 0.0 to 1.0, and any
        // function value outside the range is clipped to it.
        value.clamp(0.0, 1.0)
    }
}

/// The function set of ICC.1:2022 Table 68, the corrected row set this
/// edition publishes and its NOTE 2 requires.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Parametric {
    /// `Y = X^g`.
    Type0 { g: f32 },
    /// `Y = (aX + b)^g` for `X >= -b/a`, else `0`. CIE 122-1966.
    Type1 { g: f32, a: f32, b: f32 },
    /// `Y = (aX + b)^g + c` for `X >= -b/a`, else `c`. IEC 61966-3.
    Type2 { g: f32, a: f32, b: f32, c: f32 },
    /// `Y = (aX + b)^g` for `X >= d`, else `cX`. IEC 61966-2.1, sRGB.
    Type3 {
        g: f32,
        a: f32,
        b: f32,
        c: f32,
        d: f32,
    },
    /// `Y = (aX + b)^g + e` for `X >= d`, else `cX + f`.
    Type4 {
        g: f32,
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        e: f32,
        f: f32,
    },
}

impl Parametric {
    fn evaluate(&self, x: f32) -> f32 {
        match *self {
            Parametric::Type0 { g } => powf(x, g),
            Parametric::Type1 { g, a, b } => {
                if x >= -b / a {
                    powf(a * x + b, g)
                } else {
                    0.0
                }
            }
            Parametric::Type2 { g, a, b, c } => {
                if x >= -b / a {
                    powf(a * x + b, g) + c
                } else {
                    c
                }
            }
            Parametric::Type3 { g, a, b, c, d } => {
                if x >= d {
                    powf(a * x + b, g)
                } else {
                    c * x
                }
            }
            Parametric::Type4 {
                g,
                a,
                b,
                c,
                d,
                e,
                f,
            } => {
                if x >= d {
                    powf(a * x + b, g) + e
                } else {
                    c * x + f
                }
            }
        }
    }
}

/// `powf` through glam, for the same reason `lut.rs` gives for `exp`.
fn powf(value: f32, exponent: f32) -> f32 {
    Vec2::splat(value).powf(exponent).x
}

/// Linear interpolation over a `curv` table, ICC.1:2022 10.6. The first
/// entry is the input 0.0, the last the input 1.0, and intermediate entries
/// are uniformly spaced at `1.0/(n-1)`.
fn interpolate(entries: &[f32], steps: f32, x: f32) -> f32 {
    let scaled = x * steps;
    let floor = Vec2::splat(scaled).floor().x;
    // A NaN input keeps flowing. No table this parser builds can reach a
    // magnitude of 2^24, the one other failure of the decomposition.
    let Ok(exact) = exact_f32_integer(floor) else {
        return x;
    };
    if exact.negative {
        return entries.first().copied().unwrap_or(0.0);
    }
    let last = entries.len().saturating_sub(1);
    let index = usize::try_from(exact.magnitude).unwrap_or(0).min(last);
    let next = index.saturating_add(1).min(last);
    let lower = entries.get(index).copied().unwrap_or(0.0);
    let upper = entries.get(next).copied().unwrap_or(0.0);
    lower + (upper - lower) * (scaled - floor)
}

/// The required tag's offset and size, or [`IccError::MissingTag`].
fn required_tag(
    tags: &[([u8; 4], usize, usize)],
    signature: &[u8; 4],
) -> Result<(usize, usize), IccError> {
    tags.iter()
        .find(|(tag, _, _)| tag == signature)
        .map(|&(_, offset, size)| (offset, size))
        .ok_or(IccError::MissingTag)
}

/// One `XYZType` colourant, ICC.1:2022 10.31 Table 85: the type signature,
/// four reserved bytes, then the XYZNumber triple.
fn xyz_colourant(bytes: &[u8], (offset, size): (usize, usize)) -> Result<[f32; 3], IccError> {
    if bytes.get(offset..offset.checked_add(4).ok_or(IccError::Malformed)?) != Some(b"XYZ ") {
        return Err(IccError::Malformed);
    }
    // The signature and the reserved field are eight bytes, the triple twelve.
    if size < 20 {
        return Err(IccError::Malformed);
    }
    let mut triple = [0.0_f32; 3];
    for (index, value) in triple.iter_mut().enumerate() {
        *value = s15_fixed16(bytes, offset + 8 + index * 4)?;
    }
    Ok(triple)
}

/// One TRC tag, a `curv` or a `para`.
fn trc_curve(bytes: &[u8], (offset, size): (usize, usize)) -> Result<ToneCurve, IccError> {
    let kind = bytes
        .get(offset..offset.checked_add(4).ok_or(IccError::Malformed)?)
        .ok_or(IccError::Malformed)?;
    match kind {
        b"curv" => {
            // 10.6 Table 35: the count at 8, the entries at 12.
            if size < 12 {
                return Err(IccError::Malformed);
            }
            let count =
                usize::try_from(u32_at(bytes, offset + 8)?).map_err(|_| IccError::Malformed)?;
            let entries_len = count.checked_mul(2).ok_or(IccError::Malformed)?;
            if entries_len.checked_add(12).ok_or(IccError::Malformed)? > size {
                // A count whose entries do not fit the tag is a truncated
                // table, not a curve.
                return Err(IccError::Malformed);
            }
            match count {
                0 => Ok(ToneCurve::Identity),
                1 => {
                    let entry = u16_at(bytes, offset + 12)?;
                    // 10.6: one entry is a gamma, encoded as a u8Fixed8Number.
                    Ok(ToneCurve::Gamma(f32::from(entry) / 256.0))
                }
                _ => {
                    let mut entries = Vec::with_capacity(count);
                    for index in 0..count {
                        let entry = u16_at(bytes, offset + 12 + index * 2)?;
                        entries.push(f32::from(entry) / 65_535.0);
                    }
                    let steps =
                        f32::from(u16::try_from(count - 1).map_err(|_| IccError::Malformed)?);
                    Ok(ToneCurve::Table { entries, steps })
                }
            }
        }
        b"para" => {
            // 10.18 Table 67: the function type at 8, the parameters at 12.
            if size < 12 {
                return Err(IccError::Malformed);
            }
            let function = u16_at(bytes, offset + 8)?;
            // Table 68's parameter lists: one for type 0, three for type 1,
            // four for type 2, five for type 3 and seven for type 4.
            let parameter_count: usize = match function {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => return Err(IccError::UnsupportedCurve),
            };
            let needed = 12usize
                .checked_add(parameter_count.checked_mul(4).ok_or(IccError::Malformed)?)
                .ok_or(IccError::Malformed)?;
            if size < needed {
                return Err(IccError::Malformed);
            }
            let mut parameters = [0.0_f32; 7];
            for (index, parameter) in parameters.iter_mut().take(parameter_count).enumerate() {
                *parameter = s15_fixed16(bytes, offset + 12 + index * 4)?;
            }
            let curve = match function {
                0 => Parametric::Type0 { g: parameters[0] },
                1 => Parametric::Type1 {
                    g: parameters[0],
                    a: parameters[1],
                    b: parameters[2],
                },
                2 => Parametric::Type2 {
                    g: parameters[0],
                    a: parameters[1],
                    b: parameters[2],
                    c: parameters[3],
                },
                3 => Parametric::Type3 {
                    g: parameters[0],
                    a: parameters[1],
                    b: parameters[2],
                    c: parameters[3],
                    d: parameters[4],
                },
                _ => Parametric::Type4 {
                    g: parameters[0],
                    a: parameters[1],
                    b: parameters[2],
                    c: parameters[3],
                    d: parameters[4],
                    e: parameters[5],
                    f: parameters[6],
                },
            };
            Ok(ToneCurve::Parametric(curve))
        }
        _ => Err(IccError::UnsupportedCurve),
    }
}

/// One `s15Fixed16Number`, ICC.1:2022 Table 4, a signed 32-bit with 16
/// fractional bits.
///
/// The split keeps every magnitude up to 256 exact, which is 24 significant
/// bits, the whole `f32` significand. A colourant or a curve parameter never
/// reaches that, and beyond it the 16 fraction bits cannot fit `f32` at all.
fn s15_fixed16(bytes: &[u8], at: usize) -> Result<f32, IccError> {
    let raw = i32::from_be_bytes(u32_at(bytes, at)?.to_be_bytes());
    let magnitude = raw.unsigned_abs();
    // Both halves of a u32 fit a u16 by construction, so neither conversion
    // can fail.
    let high = u16::try_from(magnitude >> 16).map_err(|_| IccError::Malformed)?;
    let low = u16::try_from(magnitude & 0xFFFF).map_err(|_| IccError::Malformed)?;
    let value = f32::from(high) + f32::from(low) / 65_536.0;
    Ok(if raw < 0 { -value } else { value })
}

/// The encoding range `2^depth - 1` as an exact `f32`, for the depths of 1
/// to 16 this transform accepts. The colour stage itself produces 8 to 16.
/// Any other depth has no range here.
fn encoding_range(depth: u16) -> Option<f32> {
    if depth == 0 || depth > 16 {
        return None;
    }
    // The shift is at most 16 and the range at most 65,535, both checked
    // above, so the conversion cannot fail.
    let range = u16::try_from((1_u32 << depth) - 1).ok()?;
    Some(f32::from(range))
}

/// One channel: a table read when the code is a whole device code the
/// channel's own table covers, the curve otherwise.
fn channel_value(curve: &ToneCurve, code: f32, depth: u16, table: Option<&ChannelTable>) -> f32 {
    let Some(max_code) = encoding_range(depth) else {
        // No depth outside 1 to 16 reaches this. The defensive arm draws
        // black rather than guessing a range.
        return 0.0;
    };
    if let Some(table) = table
        && table.depth == depth
        && let Ok(exact) = exact_f32_integer(code)
        && !exact.negative
        && let Ok(index) = usize::try_from(exact.magnitude)
        && let Some(&value) = table.values.get(index)
    {
        return value;
    }
    let normalised = (code / max_code).clamp(0.0, 1.0);
    curve.evaluate(normalised)
}

/// Per-code tables for one curve, or nothing outside the depths of 1 to 16.
fn tabulate(curve: &ToneCurve, depth: u16) -> Option<ChannelTable> {
    // The range and its check are `encoding_range`'s, the one derivation.
    let max_code = encoding_range(depth)?;
    // The depth is at most 16 here, so the shift is within every `usize`
    // this crate targets and the count at most 65,536.
    let count = 1_usize << depth;
    let mut values = Vec::with_capacity(count);
    for code in 0..count {
        let normalised = f32::from(u16::try_from(code).ok()?) / max_code;
        values.push(curve.evaluate(normalised));
    }
    Some(ChannelTable { depth, values })
}

/// A big-endian `u32` at `at`.
fn u32_at(bytes: &[u8], at: usize) -> Result<u32, IccError> {
    let range = at..at.checked_add(4).ok_or(IccError::Malformed)?;
    let four: [u8; 4] = bytes
        .get(range)
        .ok_or(IccError::Malformed)?
        .try_into()
        .map_err(|_| IccError::Malformed)?;
    Ok(u32::from_be_bytes(four))
}

/// A big-endian `u16` at `at`.
fn u16_at(bytes: &[u8], at: usize) -> Result<u16, IccError> {
    let range = at..at.checked_add(2).ok_or(IccError::Malformed)?;
    let two: [u8; 2] = bytes
        .get(range)
        .ok_or(IccError::Malformed)?
        .try_into()
        .map_err(|_| IccError::Malformed)?;
    Ok(u16::from_be_bytes(two))
}

#[cfg(test)]
mod tests {
    use super::{
        IccError, IccProfile, IccTransform, MAX_PROFILE_BYTES, PCS_TO_SRGB, Parametric, ToneCurve,
        encoding_range, powf, s15_fixed16,
    };
    use crate::color::Rgb;

    /// The constant is the inverse of the D50 colourant matrix the ICC's
    /// sRGB note prints, computed here from the printed values alone and
    /// rounded once to `f32`. The note derives and prints the device to PCS
    /// direction, the colourants, and ICC.1:2022 Annex F.3 makes the PCS the
    /// colourant matrix times the linear device RGB, so the inverse of the
    /// printed colourants is the PCS to sRGB direction, and a D50 PCS white
    /// maps to linear `1 1 1` within the printed matrix's own precision.
    #[test]
    fn the_pcs_to_srgb_matrix_is_the_printed_d50_inverse() {
        // The note's printed values. The columns are rXYZ, gXYZ and bXYZ,
        // the same numbers `tests/icc.rs` encodes as its colourant constants.
        let printed = [
            [
                0.436_030_342_570_117_f64,
                0.385_101_860_087_134,
                0.143_067_806_654_203,
            ],
            [
                0.222_438_466_210_245_f64,
                0.716_942_745_571_917,
                0.060_618_777_416_563,
            ],
            [
                0.013_897_440_074_263_f64,
                0.097_076_381_494_207,
                0.713_926_257_896_652,
            ],
        ];
        let [a, b, c] = printed[0];
        let [d, e, f] = printed[1];
        let [g, h, i] = printed[2];
        let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
        // The adjugate over the determinant, each entry the signed cofactor
        // of the transposed position.
        let inverse = [
            [
                (e * i - f * h) / determinant,
                (c * h - b * i) / determinant,
                (b * f - c * e) / determinant,
            ],
            [
                (f * g - d * i) / determinant,
                (a * i - c * g) / determinant,
                (c * d - a * f) / determinant,
            ],
            [
                (d * h - e * g) / determinant,
                (b * g - a * h) / determinant,
                (a * e - b * d) / determinant,
            ],
        ];
        // The one `f64` to `f32` step of the derivation, which is the
        // rounding the constant is defined by: each entry of the inverse to
        // the nearest `f32`. `std` has no conversion that rounds an `f64` to
        // an `f32` without this cast, and the derivation itself is what
        // justifies it.
        #[allow(clippy::cast_possible_truncation)]
        let rounded = inverse.map(|row| row.map(|value| value as f32));
        for (computed, constant_row) in rounded.iter().zip(PCS_TO_SRGB.iter()) {
            for (value, constant) in computed.iter().zip(constant_row) {
                assert_eq!(constant.to_bits(), value.to_bits());
            }
        }
    }

    /// The `s15Fixed16Number` decode, ICC.1:2022 Table 4: two's complement
    /// over 65,536 with 16 fractional bits. The sign and the 65,536 divisor
    /// are pinned on encodings a `para` curve and a colourant actually
    /// carry, including the most negative encoding.
    #[test]
    fn s15_fixed16_decodes_the_printed_encodings() {
        // 2.0, the gamma of a plain power curve.
        assert_eq!(s15_fixed16(&[0x00, 0x02, 0x00, 0x00], 0), Ok(2.0));
        // -0.5, the b of a CIE 122-1966 type 1 curve.
        assert_eq!(s15_fixed16(&[0xFF, 0xFF, 0x80, 0x00], 0), Ok(-0.5));
        // 2.399993896484375, the sRGB gamma's nearest encoding, and its
        // negation, 0xFFFD999A over the same 65,536.
        assert_eq!(s15_fixed16(&[0x00, 0x02, 0x66, 0x66], 0), Ok(2.399_994));
        assert_eq!(s15_fixed16(&[0xFF, 0xFD, 0x99, 0x9A], 0), Ok(-2.399_994));
        // 0x80000000 is -32768, two's complement over 65,536, exact in `f32`.
        assert_eq!(s15_fixed16(&[0x80, 0x00, 0x00, 0x00], 0), Ok(-32_768.0));
    }

    /// The boundary belongs to the power branch: `X >= d`, ICC.1:2022
    /// Table 68. The sRGB parameters are continuous at `d`, so the boundary
    /// is pinned with parameters where the two branches disagree.
    #[test]
    fn the_boundary_d_belongs_to_the_power_branch() {
        let curve = ToneCurve::Parametric(Parametric::Type3 {
            g: 2.0,
            a: 1.0,
            b: 0.0,
            c: 0.9,
            d: 0.5,
        });
        // The power branch gives 0.25 at the boundary, the linear branch 0.45.
        let at_boundary = curve.evaluate(0.5);
        assert_eq!(at_boundary.to_bits(), 0.25_f32.to_bits());
    }

    /// The remaining function types of Table 68, each hand computed from its
    /// own row. Type 0 is `X^g`. Type 1 is CIE 122-1966: `(aX + b)^g` for
    /// `X >= -b/a`, else `0`. Type 2 is IEC 61966-3: the same power branch
    /// plus `c`, else `c`. Type 4 is type 3's shape with the additive `e`
    /// and the `cX + f` branch.
    #[test]
    fn the_other_parametric_functions_follow_table_68() {
        let type0 = ToneCurve::Parametric(Parametric::Type0 { g: 2.4 });
        // 0.5^2.4 = 0.189465.
        let at_half = type0.evaluate(0.5);
        assert!((at_half - 0.189_465).abs() < 1e-5, "{at_half}");

        let type1 = ToneCurve::Parametric(Parametric::Type1 {
            g: 2.0,
            a: 2.0,
            b: -0.5,
        });
        // -b/a is 0.25. At and above it the power branch runs, and below it
        // the answer is zero.
        assert_eq!(type1.evaluate(0.5).to_bits(), 0.25_f32.to_bits());
        assert_eq!(type1.evaluate(0.2).to_bits(), 0.0_f32.to_bits());

        let type2 = ToneCurve::Parametric(Parametric::Type2 {
            g: 2.0,
            a: 2.0,
            b: -0.5,
            c: 0.3,
        });
        // -b/a is 0.25. At and above it the power branch plus c runs, and
        // below it the constant c alone.
        assert_eq!(type2.evaluate(0.5).to_bits(), 0.55_f32.to_bits());
        assert_eq!(type2.evaluate(0.2).to_bits(), 0.3_f32.to_bits());

        let type4 = ToneCurve::Parametric(Parametric::Type4 {
            g: 2.0,
            a: 1.0,
            b: 0.0,
            c: 0.9,
            d: 0.5,
            e: 0.1,
            f: 0.2,
        });
        // At and above d the power branch plus e, below it cX + f.
        assert_eq!(type4.evaluate(0.5).to_bits(), 0.35_f32.to_bits());
        assert_eq!(type4.evaluate(0.25).to_bits(), 0.425_f32.to_bits());
    }

    /// A `curv` with a count of zero is the identity response, ICC.1:2022
    /// 10.6.
    #[test]
    fn a_zero_count_curve_is_the_identity() {
        let curve = ToneCurve::Identity;
        assert_eq!(curve.evaluate(0.25).to_bits(), 0.25_f32.to_bits());
        assert_eq!(curve.evaluate(1.0).to_bits(), 1.0_f32.to_bits());
    }

    /// The type 3 branch rule at an input below `d`, hand computed from the
    /// sRGB parameters of the plan: `c * x` there, not the power branch.
    #[test]
    fn type_three_below_d_is_cx() {
        let curve = ToneCurve::Parametric(Parametric::Type3 {
            g: 2.399_994,
            a: 0.947_861,
            b: 0.052_139,
            c: 0.077_393,
            d: 0.040_451,
        });
        // 5 / 255 = 0.0196078, below d.
        let x = 5.0_f32 / 255.0;
        let expected = 0.077_393 * x;
        let actual = curve.evaluate(x);
        assert!(
            (actual - expected).abs() < 1e-6,
            "{actual} against {expected}"
        );
        // And the power branch above d: 64 / 255.
        let x = 64.0_f32 / 255.0;
        let expected = powf(0.947_861 * x + 0.052_139, 2.399_994);
        let actual = curve.evaluate(x);
        assert!(
            (actual - expected).abs() < 1e-6,
            "{actual} against {expected}"
        );
    }

    /// The tables agree with the continuous curve at every code, which is
    /// what makes the per-draw table read a restatement of the curve and not
    /// a second arithmetic. The matrix and the encoding after it are the
    /// same code, so the two transforms answer bit for bit.
    #[test]
    fn tabulated_codes_match_the_continuous_curve() {
        let bytes = gamma_profile(0x0233);
        let tabulated = {
            let parsed = IccProfile::parse(&bytes);
            assert!(parsed.is_ok(), "{parsed:?}");
            let Ok(parsed) = parsed else { return };
            IccTransform::from(parsed).tabulated(&[8, 8, 8])
        };
        let continuous = {
            let parsed = IccProfile::parse(&bytes);
            assert!(parsed.is_ok(), "{parsed:?}");
            let Ok(parsed) = parsed else { return };
            IccTransform::from(parsed)
        };
        assert!(tabulated.tables.is_some());
        for code in 0..=255_u16 {
            let value = f32::from(code);
            let from_table = tabulated.apply(
                Rgb {
                    r: value,
                    g: value,
                    b: value,
                },
                [8, 8, 8],
            );
            let from_curve = continuous.apply(
                Rgb {
                    r: value,
                    g: value,
                    b: value,
                },
                [8, 8, 8],
            );
            assert_eq!(
                from_table.r.to_bits(),
                from_curve.r.to_bits(),
                "code {code}"
            );
            assert_eq!(
                from_table.g.to_bits(),
                from_curve.g.to_bits(),
                "code {code}"
            );
            assert_eq!(
                from_table.b.to_bits(),
                from_curve.b.to_bits(),
                "code {code}"
            );
        }
        // A depth the tables were not built for falls through to the curve,
        // and answers as an untabulated transform would. The first code is
        // above the 8-bit tables' length, so it falls through on the length
        // alone. The last is below it and would index the 8-bit table if the
        // table's own depth check were gone. Both are the continuous curve's
        // answer only because the check refuses the table.
        let twelve = [12_u16, 12, 12];
        let from_table = tabulated.apply(
            Rgb {
                r: 4095.0,
                g: 2048.0,
                b: 100.0,
            },
            twelve,
        );
        let from_curve = continuous.apply(
            Rgb {
                r: 4095.0,
                g: 2048.0,
                b: 100.0,
            },
            twelve,
        );
        assert_eq!(from_table.r.to_bits(), from_curve.r.to_bits());
        assert_eq!(from_table.g.to_bits(), from_curve.g.to_bits());
        assert_eq!(from_table.b.to_bits(), from_curve.b.to_bits());
    }

    /// A depth outside 1 to 16 has no encoding range.
    #[test]
    fn the_encoding_range_covers_one_to_sixteen_bits() {
        assert!(encoding_range(0).is_none());
        assert!(encoding_range(17).is_none());
        assert_eq!(encoding_range(8), Some(255.0));
        assert_eq!(encoding_range(12), Some(4095.0));
        assert_eq!(encoding_range(16), Some(65_535.0));
    }

    /// A profile too large for the constant is refused once its header and
    /// tag table have had their say, and the constant is what the type
    /// reports. The profile is the unit gamma shape with an unclaimed block
    /// of padding past its last tag, so it is the subset shape and not a
    /// malformed one. A profile whose declared size is wrong is malformed
    /// before its size is held against the constant, at any length.
    #[test]
    fn the_size_constant_is_the_parse_limit() {
        assert_eq!(IccProfile::max_bytes(), MAX_PROFILE_BYTES);
        let mut large = gamma_profile(0x0233);
        large.resize(MAX_PROFILE_BYTES + 1, 0);
        let size = u32::try_from(large.len()).unwrap_or(u32::MAX);
        if let Some(slot) = large.get_mut(0..4) {
            slot.copy_from_slice(&size.to_be_bytes());
        }
        assert_eq!(IccProfile::parse(&large), Err(IccError::TooLarge));
        if let Some(slot) = large.get_mut(0..1) {
            slot.copy_from_slice(&[0x10]);
        }
        assert_eq!(IccProfile::parse(&large), Err(IccError::Malformed));
    }

    /// The smallest profile that can hold a header and a tag count.
    #[test]
    fn a_short_profile_is_malformed() {
        assert_eq!(IccProfile::parse(&[0_u8; 131]), Err(IccError::Malformed));
    }

    /// A minimal conforming gamma profile, built by hand because the fixture
    /// helpers live in the integration test file. The colourants are the same
    /// sRGB D50 s15Fixed16 encodings the integration fixtures derive, so the
    /// product matrix is near the identity and the comparison below is about
    /// the curves and not about zeros.
    fn gamma_profile(gamma: u16) -> Vec<u8> {
        let mut out = vec![0_u8; 128];
        let header = |out: &mut Vec<u8>, at: usize, bytes: &[u8]| {
            if let Some(slot) = out.get_mut(at..at + bytes.len()) {
                slot.copy_from_slice(bytes);
            }
        };
        header(&mut out, 8, &[0x04, 0x40, 0x00, 0x00]);
        header(&mut out, 12, b"mntr");
        header(&mut out, 16, b"RGB ");
        header(&mut out, 20, b"XYZ ");
        header(&mut out, 36, b"acsp");
        let mut table = Vec::new();
        table.extend_from_slice(&6_u32.to_be_bytes());
        let mut offset = 128 + 4 + 6 * 12;
        let mut push = |table: &mut Vec<u8>, signature: [u8; 4], payload: usize| {
            table.extend_from_slice(&signature);
            table.extend_from_slice(&u32::try_from(offset).unwrap_or(u32::MAX).to_be_bytes());
            table.extend_from_slice(&u32::try_from(payload).unwrap_or(u32::MAX).to_be_bytes());
            offset += payload.div_ceil(4) * 4;
        };
        for signature in [*b"rXYZ", *b"gXYZ", *b"bXYZ"] {
            push(&mut table, signature, 20);
        }
        for signature in [*b"rTRC", *b"gTRC", *b"bTRC"] {
            push(&mut table, signature, 14);
        }
        out.extend_from_slice(&table);
        for (_tag, colourant) in [
            (*b"rXYZ", [0x0000_6FA0_u32, 0x0000_38F2, 0x0000_038F]),
            (*b"gXYZ", [0x0000_6296, 0x0000_B78A, 0x0000_18DA]),
            (*b"bXYZ", [0x0000_24A0, 0x0000_0F85, 0x0000_B6C4]),
        ] {
            let mut payload = vec![0_u8; 20];
            if let Some(slot) = payload.get_mut(0..4) {
                slot.copy_from_slice(b"XYZ ");
            }
            for (index, word) in colourant.iter().enumerate() {
                if let Some(slot) = payload.get_mut(8 + index * 4..12 + index * 4) {
                    slot.copy_from_slice(&word.to_be_bytes());
                }
            }
            out.append(&mut payload);
        }
        for _ in 0..3 {
            // 14 bytes of curv, padded to the four-byte boundary the offset
            // above already assumed.
            let mut payload = vec![0_u8; 14];
            if let Some(slot) = payload.get_mut(0..4) {
                slot.copy_from_slice(b"curv");
            }
            if let Some(slot) = payload.get_mut(8..12) {
                slot.copy_from_slice(&1_u32.to_be_bytes());
            }
            if let Some(slot) = payload.get_mut(12..14) {
                slot.copy_from_slice(&gamma.to_be_bytes());
            }
            out.append(&mut payload);
            out.push(0);
            out.push(0);
        }
        let size = u32::try_from(out.len()).unwrap_or(u32::MAX);
        if let Some(slot) = out.get_mut(0..4) {
            slot.copy_from_slice(&size.to_be_bytes());
        }
        out
    }
}
