//! ICC.1:2022 matrix and tone-curve fixtures and the IEC 61966-2-1 sRGB
//! encoding.
//!
//! F-240. Every expected value was computed by hand from the two published
//! texts and none of it was read out of this repository's own output, HLD
//! 27.2 R2. The editions read, before any fixture was written:
//!
//! - ICC.1:2022-05, profile version 4.4.0.0 (color.org), for the header
//!   layout of 7.2 Table 17, the tag table of 7.3, `XYZType` of 10.31
//!   Table 85, `curveType` of 10.6 Table 35, `parametricCurveType` of 10.18
//!   Tables 67 and 68, the number encodings of 4.6 and 4.9, the profile
//!   classes of Table 18, the colour spaces of Table 19, the rendering
//!   intents of Table 23 and the linearized Bradford cone matrix of Annex E.
//! - IEC 61966-2-1:1999 as amended, read through the ICC's sRGB
//!   characterisation note at registry.color.org (updated April 2015), for
//!   the transfer function `12.92 L` at or below `0.0031308` and
//!   `1.055 L^(1/2.4) - 0.055` above it, and for the sRGB D50 colourants the
//!   note publishes for matrix/TRC profiles.
//!
//! **The one tolerance.** The profile's colourants are s15Fixed16, so the
//! product of the PCS to sRGB matrix and those colourants is not exactly the
//! identity. For the note's colourants as these fixtures encode them, each
//! entry of the product is within `1.34e-5` of the identity's entry, and the
//! largest linear move the product gives a device triple in
//! the unit cube is `2.66e-5`, at the white vertex, green channel. Encoded,
//! the gap between the asserted hand values and the pipeline's replies
//! reaches at most `0.0029` of 255 over the sRGB-colourant rows these
//! fixtures assert: `0.0026` where the only quantisation is the colourants,
//! and `0.0029` on the parametric rows, whose curve parameters are
//! s15Fixed16 too. The wide-gamut Adobe rows carry more, `0.0054` of 255 at
//! the red row's green channel: the quantisation moves that channel by only
//! `1.6e-6` linear against the printed decimals, but it answers at `1.1e-4`
//! from zero, where the `12.92` slope is `3294.6` codes per linear unit.
//! The unit cube's worst case, a channel at black beside two saturated
//! ones, reaches about `0.044` of 255, the `12.92` slope amplifying the
//! residue. The tolerance of `0.05` bounds all of these and still dies
//! against every wrong implementation named below, the smallest of which
//! moves a row by about `0.31`.

use ocelli_core::{Stored, exact_f32_integer};
use ocelli_pixel::{
    ColorTransform, ColourWindow, DecodedLayout, DecodedPhotometric, DisplayScale, IccError,
    IccProfile, IccTransform, PhotometricInterpretation, PixelDataEncoding, PixelError,
    PlanarConfiguration, Rgb, SampleLayout,
};

/// See the module comment. The smallest wrong implementation named below
/// moves a row by about `0.31`, six times this bound.
const TOLERANCE: f32 = 0.05;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok(), "{result:?}");
        let Ok(value) = result else { return };
        value
    }};
}

/// The sRGB D50 colourants of the ICC's sRGB characterisation note, as a
/// matrix/TRC profile encodes them. Each value is the decimal published,
/// times 65,536, rounded to the nearest whole step. The three colourants sum
/// to the D50 media white, so a neutral device code stays neutral and the
/// rows below do not depend on the exact PCS to sRGB matrix beyond its
/// mapping of D50 to `1 1 1`.
///
/// ```text
/// rXYZ = (0.436030342570117, 0.222438466210245, 0.013897440074263)
/// gXYZ = (0.385101860087134, 0.716942745571917, 0.097076381494207)
/// bXYZ = (0.143067806654203, 0.060618777416563, 0.713926257896652)
/// ```
const R_XYZ: [u32; 3] = [0x0000_6FA0, 0x0000_38F2, 0x0000_038F];
const G_XYZ: [u32; 3] = [0x0000_6296, 0x0000_B78A, 0x0000_18DA];
const B_XYZ: [u32; 3] = [0x0000_24A0, 0x0000_0F85, 0x0000_B6C4];

/// The tier C byte rule, `docs/lld/render-graph.md`. A value at or below 0
/// stores 0, a value at or above 255 stores 255, and otherwise the nearest
/// code stores with a tie to the even code. The display value itself is not
/// clamped. The clamp here is the store's, not the arithmetic's.
fn reply(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    if value >= 255.0 {
        return 255;
    }
    let floor = value.floor();
    let Ok(exact) = exact_f32_integer(floor) else {
        return 0;
    };
    let frac = value - floor;
    let floor_is_even = exact.magnitude % 2 == 0;
    let tie = frac.to_bits() == 0.5_f32.to_bits();
    let up = frac > 0.5 || (tie && !floor_is_even);
    let code = exact.magnitude + u64::from(up);
    u8::try_from(code).unwrap_or(255)
}

fn replies(rgb: Rgb) -> [u8; 3] {
    [reply(rgb.r), reply(rgb.g), reply(rgb.b)]
}

/// One mapped pixel, with the index proved before the read.
#[track_caller]
fn row(rows: &[Rgb], index: usize) -> Rgb {
    assert!(index < rows.len(), "row {index} of {}", rows.len());
    *rows.get(index).unwrap_or(&Rgb::BLACK)
}

#[track_caller]
fn assert_close(actual: Rgb, expected: (f32, f32, f32)) {
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

// ---------------------------------------------------------------------------
// Profile assembly, ICC.1:2022 7.2 and 7.3
// ---------------------------------------------------------------------------

/// One tag's signature and its payload bytes.
type Tag = ([u8; 4], Vec<u8>);

/// Assemble a profile: the 7.2 header over the 7.3 tag table, each tag
/// padded to its four-byte boundary, and the size field written last. Every
/// payload is built from the encodings its own site derives.
fn profile(class: &[u8; 4], space: &[u8; 4], pcs: &[u8; 4], tags: &[Tag]) -> Vec<u8> {
    let mut out = vec![0_u8; 128];
    // 7.2.4: version 4.4.0.0. 7.2.9: the profile file signature. 7.2.15: a
    // perceptual intent, which the transform records and never applies.
    // 7.2.16: the PCS illuminant, D50 as 0xF6D6, 0x10000, 0xD32D.
    out.get_mut(8..12)
        .unwrap_or_default()
        .copy_from_slice(&[0x04, 0x40, 0x00, 0x00]);
    out.get_mut(12..16)
        .unwrap_or_default()
        .copy_from_slice(class);
    out.get_mut(16..20)
        .unwrap_or_default()
        .copy_from_slice(space);
    out.get_mut(20..24).unwrap_or_default().copy_from_slice(pcs);
    out.get_mut(36..40)
        .unwrap_or_default()
        .copy_from_slice(b"acsp");
    out.get_mut(68..80).unwrap_or_default().copy_from_slice(&[
        0x00, 0x00, 0xF6, 0xD6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xD3, 0x2D,
    ]);
    let mut entries = Vec::new();
    entries.extend_from_slice(&u32::try_from(tags.len()).unwrap_or(0).to_be_bytes());
    let mut offset = 128 + 4 + tags.len() * 12;
    for (signature, payload) in tags {
        entries.extend_from_slice(signature);
        entries.extend_from_slice(&u32::try_from(offset).unwrap_or(u32::MAX).to_be_bytes());
        entries.extend_from_slice(
            &u32::try_from(payload.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes(),
        );
        offset += payload.len();
        offset = offset.div_ceil(4) * 4;
    }
    out.extend_from_slice(&entries);
    for (_signature, payload) in tags {
        out.extend_from_slice(payload);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }
    let size = u32::try_from(out.len()).unwrap_or(u32::MAX);
    out.get_mut(0..4)
        .unwrap_or_default()
        .copy_from_slice(&size.to_be_bytes());
    out
}

/// An `XYZType` payload, 10.31 Table 85. Four reserved bytes, then one
/// XYZNumber triple of s15Fixed16Number values, big-endian.
fn xyz(value: [u32; 3]) -> Vec<u8> {
    let mut out = vec![0_u8; 20];
    out.get_mut(0..4)
        .unwrap_or_default()
        .copy_from_slice(b"XYZ ");
    for (index, word) in value.iter().enumerate() {
        let at = 8 + index * 4;
        out.get_mut(at..at + 4)
            .unwrap_or_default()
            .copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// A `curveType` payload, 10.6 Table 35. Four reserved bytes, a uInt32Number
/// count, then the entries as uInt16Number values, big-endian.
fn curv(entries: &[u16]) -> Vec<u8> {
    let mut out = vec![0_u8; 12 + entries.len() * 2];
    out.get_mut(0..4)
        .unwrap_or_default()
        .copy_from_slice(b"curv");
    let count = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    out.get_mut(8..12)
        .unwrap_or_default()
        .copy_from_slice(&count.to_be_bytes());
    for (index, entry) in entries.iter().enumerate() {
        let at = 12 + index * 2;
        out.get_mut(at..at + 2)
            .unwrap_or_default()
            .copy_from_slice(&entry.to_be_bytes());
    }
    out
}

/// A `parametricCurveType` payload, 10.18 Table 67. Four reserved bytes, the
/// uInt16Number function type at 8, two reserved bytes, then the
/// s15Fixed16Number parameters at 12.
fn para(function: u16, parameters: &[u32]) -> Vec<u8> {
    let mut out = vec![0_u8; 12 + parameters.len() * 4];
    out.get_mut(0..4)
        .unwrap_or_default()
        .copy_from_slice(b"para");
    out.get_mut(8..10)
        .unwrap_or_default()
        .copy_from_slice(&function.to_be_bytes());
    for (index, parameter) in parameters.iter().enumerate() {
        let at = 12 + index * 4;
        out.get_mut(at..at + 4)
            .unwrap_or_default()
            .copy_from_slice(&parameter.to_be_bytes());
    }
    out
}

/// The six-tag matrix profile with the tone curves given.
fn srgb_profile(red: Vec<u8>, green: Vec<u8>, blue: Vec<u8>) -> Vec<u8> {
    profile(
        b"mntr",
        b"RGB ",
        b"XYZ ",
        &[
            (*b"rXYZ", xyz(R_XYZ)),
            (*b"gXYZ", xyz(G_XYZ)),
            (*b"bXYZ", xyz(B_XYZ)),
            (*b"rTRC", red),
            (*b"gTRC", green.clone()),
            (*b"bTRC", blue),
        ],
    )
}

fn stored(values: &[f32]) -> Vec<Stored> {
    values.iter().copied().map(Stored).collect()
}

/// One interleaved RGB frame, mapped through the given transform.
fn mapped(transform: &ColorTransform, samples: &[f32]) -> Vec<Rgb> {
    let pixels = samples.len() / 3;
    let mut destination = vec![Rgb::BLACK; pixels];
    assert!(
        transform
            .map_into(&stored(samples), &mut destination, pixels)
            .is_ok()
    );
    destination
}

/// The colour route for an interleaved native RGB frame of the given depth.
fn rgb_route(bits: u16) -> Result<ColorTransform, PixelError> {
    let layout = SampleLayout::new(
        3,
        PhotometricInterpretation::Rgb,
        Some(PlanarConfiguration::Interleaved),
    )?;
    let transform = ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    )?;
    if bits == 8 {
        return Ok(transform);
    }
    let scale = DisplayScale::from_bits(bits)?;
    transform.with_sample_scale(scale)
}

/// The colour route with the profile parsed and attached. `with_sample_scale`
/// goes first for a frame above 8 bits, because the device depth the profile
/// normalises by is the sample scale's own.
fn icc_route(profile_bytes: &[u8], bits: u16) -> Result<ColorTransform, PixelError> {
    let parsed = IccProfile::parse(profile_bytes);
    assert!(parsed.is_ok(), "{parsed:?}");
    let Ok(parsed) = parsed else {
        return rgb_route(bits);
    };
    rgb_route(bits).map(|route| route.with_icc(IccTransform::from(parsed)))
}

// ---------------------------------------------------------------------------
// AC1, AC2, AC4, AC8. The tone curves and the encoding.
// ---------------------------------------------------------------------------

/// AC1. Gamma `0x0100` is the u8Fixed8Number 1.0, ICC.1:2022 Table 7, so the
/// TRC is the identity and the linear value equals the device code over 255.
/// The sRGB encoding of IEC 61966-2-1 then gives, per channel, about `38.25`,
/// `137.2066`, `187.845`, `224.9999` and `255`. Hand computation of the
/// first row: `64 / 255 = 0.250980`, which is above the `0.0031308` knee, so
/// `1.055 * 0.250980^(1/2.4) - 0.055 = 0.537872`, times 255 is `137.2066`.
/// The row at device `5` sits between the two knees of the transfer
/// function, `0.0031308 < 0.0196078 <= 0.04045`, so its linear value takes
/// the power branch: `1.055 * 0.0196078^(1/2.4) - 0.055 = 0.150008`, times
/// 255 about `38.25`. Reading the knee as the decode threshold's `0.04045`
/// would answer `12.92 * 0.0196078 = 0.253333` there, times 255 `64.6`,
/// which no row here is within the tolerance of.
#[test]
fn gamma_one_profile_encodes_linear_as_srgb() {
    let transform = require_ok!(icc_route(
        &srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100])),
        8
    ));
    let rows = mapped(
        &transform,
        &[
            5.0, 5.0, 5.0, 64.0, 64.0, 64.0, 128.0, 128.0, 128.0, 192.0, 192.0, 192.0, 255.0,
            255.0, 255.0,
        ],
    );
    assert_close(row(&rows, 0), (38.2513, 38.2513, 38.2513));
    assert_close(row(&rows, 1), (137.2066, 137.2066, 137.2066));
    assert_close(row(&rows, 2), (187.845, 187.845, 187.845));
    assert_close(row(&rows, 3), (224.9999, 224.9999, 224.9999));
    assert_close(row(&rows, 4), (255.0, 255.0, 255.0));
    assert_eq!(replies(row(&rows, 0)), [38, 38, 38]);
    assert_eq!(replies(row(&rows, 1)), [137, 137, 137]);
    assert_eq!(replies(row(&rows, 2)), [188, 188, 188]);
    assert_eq!(replies(row(&rows, 3)), [225, 225, 225]);
    assert_eq!(replies(row(&rows, 4)), [255, 255, 255]);
}

/// AC2. Gamma `0x0233` is the u8Fixed8Number `563 / 256 = 2.19921875`, so the
/// linear value is the device code to that power before the encoding. Hand
/// computation of the first row: `0.250980^2.19921875 = 0.047825`, and
/// `1.055 * 0.047825^(1/2.4) - 0.055 = 0.242278`, times 255 is `61.773`.
/// Reading only the integer byte as the gamma would give `71 137 198`, which
/// no row here is within the tolerance of.
#[test]
fn u8_fixed8_gamma_is_read_as_two_point_two() {
    let transform = require_ok!(icc_route(
        &srgb_profile(curv(&[0x0233]), curv(&[0x0233]), curv(&[0x0233])),
        8
    ));
    let rows = mapped(
        &transform,
        &[64.0, 64.0, 64.0, 128.0, 128.0, 128.0, 192.0, 192.0, 192.0],
    );
    assert_close(row(&rows, 0), (61.773, 61.773, 61.773));
    assert_close(row(&rows, 1), (129.0303, 129.0303, 129.0303));
    assert_close(row(&rows, 2), (193.4013, 193.4013, 193.4013));
    assert_eq!(replies(row(&rows, 0)), [62, 62, 62]);
    assert_eq!(replies(row(&rows, 1)), [129, 129, 129]);
    assert_eq!(replies(row(&rows, 2)), [193, 193, 193]);
}

/// AC3. A `para` type 3 carrying the sRGB parameters of the plan. The curve
/// is the IEC decode function, so a grey device code round trips to itself
/// within the quantisation. The plan's grey rows are all above `d`, where
/// both branches of the function agree, so the below-threshold rows `5` and
/// `0` are added. They are what makes the named wrong implementation, using
/// the `X >= d` branch below `d`, red: on device `5` it answers `5.71`
/// against `5.00`, and on device `0` it answers `2.75` against `0.00`.
#[test]
fn parametric_type_three_srgb_is_the_identity() {
    // g, a, b, c and d as s15Fixed16Number values.
    let curve = para(
        3,
        &[
            0x0002_6666,
            0x0000_F2A7,
            0x0000_0D59,
            0x0000_13D0,
            0x0000_0A5B,
        ],
    );
    let transform = require_ok!(icc_route(
        &srgb_profile(curve.clone(), curve.clone(), curve),
        8
    ));
    let rows = mapped(
        &transform,
        &[
            64.0, 64.0, 64.0, 128.0, 128.0, 128.0, 192.0, 192.0, 192.0, 5.0, 5.0, 5.0, 0.0, 0.0,
            0.0,
        ],
    );
    assert_close(row(&rows, 0), (64.0, 64.0, 64.0));
    assert_close(row(&rows, 1), (128.0, 128.0, 128.0));
    assert_close(row(&rows, 2), (192.0, 192.0, 192.0));
    assert_close(row(&rows, 3), (5.0, 5.0, 5.0));
    assert_close(row(&rows, 4), (0.0, 0.0, 0.0));
    assert_eq!(replies(row(&rows, 0)), [64, 64, 64]);
    assert_eq!(replies(row(&rows, 1)), [128, 128, 128]);
    assert_eq!(replies(row(&rows, 2)), [192, 192, 192]);
    assert_eq!(replies(row(&rows, 3)), [5, 5, 5]);
    assert_eq!(replies(row(&rows, 4)), [0, 0, 0]);
}

/// AC4. Two entries, `0` and `65535`, span the input range exactly, so
/// linear interpolation reproduces the input and the rows are AC1's. An
/// implementation that indexes the table without interpolating reads entry
/// `0` for every code below 255 and answers black.
#[test]
fn a_two_entry_table_is_linear() {
    let transform = require_ok!(icc_route(
        &srgb_profile(
            curv(&[0x0000, 0xFFFF]),
            curv(&[0x0000, 0xFFFF]),
            curv(&[0x0000, 0xFFFF])
        ),
        8
    ));
    let rows = mapped(
        &transform,
        &[
            64.0, 64.0, 64.0, 128.0, 128.0, 128.0, 192.0, 192.0, 192.0, 255.0, 255.0, 255.0,
        ],
    );
    assert_close(row(&rows, 0), (137.2066, 137.2066, 137.2066));
    assert_close(row(&rows, 1), (187.845, 187.845, 187.845));
    assert_close(row(&rows, 2), (224.9999, 224.9999, 224.9999));
    assert_close(row(&rows, 3), (255.0, 255.0, 255.0));
    assert_eq!(replies(row(&rows, 0)), [137, 137, 137]);
    assert_eq!(replies(row(&rows, 3)), [255, 255, 255]);
}

/// AC8. A 12-bit frame whose codes are normalised by `2^12 - 1 = 4095`, not
/// shifted to 8 bits first. Hand computation of the middle row:
/// `2048 / 4095 = 0.500122`, and `1.055 * 0.500122^(1/2.4) - 0.055 =
/// 0.735438`, times 255 is `187.5365`. Applying the 8-bit display scaling
/// before the profile would answer the encoding of `128 / 255 = 0.501961`,
/// which is `187.845` on this row, `0.31` away and outside the tolerance.
#[test]
fn icc_normalises_by_bits_stored() {
    let transform = require_ok!(icc_route(
        &srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100])),
        12
    ));
    let rows = mapped(
        &transform,
        &[
            4095.0, 4095.0, 4095.0, 2048.0, 2048.0, 2048.0, 0.0, 0.0, 0.0,
        ],
    );
    assert_close(row(&rows, 0), (255.0, 255.0, 255.0));
    assert_close(row(&rows, 1), (187.5365, 187.5365, 187.5365));
    assert_close(row(&rows, 2), (0.0, 0.0, 0.0));
    assert_eq!(replies(row(&rows, 0)), [255, 255, 255]);
    assert_eq!(replies(row(&rows, 1)), [188, 188, 188]);
    assert_eq!(replies(row(&rows, 2)), [0, 0, 0]);
}

// ---------------------------------------------------------------------------
// D1. An out-of-gamut device colour clips to the display range, A.7.
// ---------------------------------------------------------------------------

/// The Adobe RGB 1998 D50 colourants as the reference matrix/TRC profile
/// carries them. Each value is the decimal published, times 65,536, rounded
/// to the nearest whole step.
///
/// ```text
/// rXYZ = (0.60974, 0.31111, 0.01947)
/// gXYZ = (0.20528, 0.62567, 0.06087)
/// bXYZ = (0.14919, 0.06322, 0.74457)
/// ```
const ADOBE_R_XYZ: [u32; 3] = [0x0000_9C18, 0x0000_4FA5, 0x0000_04FC];
const ADOBE_G_XYZ: [u32; 3] = [0x0000_348D, 0x0000_A02C, 0x0000_0F95];
const ADOBE_B_XYZ: [u32; 3] = [0x0000_2631, 0x0000_102F, 0x0000_BE9C];

/// D1. An Adobe RGB 1998 green is outside the sRGB gamut, and the reply
/// clips to the display codes sRGB has. The TRC is the Adobe gamma
/// `0x0233 = 563/256 = 2.19921875`, which at code 255 is `1.0`.
///
/// Hand computation of device `0 255 0`: the normalised input `0 1 0` meets
/// the product of the PCS to sRGB matrix and the green colourant. The red
/// row is `3.1339235 * 0.20528 - 1.616923 * 0.62567 - 0.4907337 * 0.06087 =
/// 0.64333182236 - 1.01166019751 - 0.02987096110 = -0.39819933626`, and the blue row is
/// `0.0720355 * 0.20528 - 0.22903204 * 0.62567 + 1.4057162 * 0.06087 =
/// 0.01478745486 - 0.14329847624 + 0.08556594393 = -0.04294507745`. A.7 of the ICC's sRGB
/// note clips both to 0, and the encoding of 0 is 0. Green maps to
/// `0.9998680` and encodes to `1.055 * 0.9998680^(1/2.4) - 0.055 =
/// 0.9999420`, times 255 `254.9852`. Without the clip the red channel would
/// answer `12.92 * -0.3981993 * 255 = -1311.9`: the byte rule would still
/// store 0, but the D-29 colour window would window it to 0 instead of the
/// clipped `64`, and the readout would report a value sRGB does not have.
/// Device `255 0 0` clips the other way: red maps to `1.3982830`, above 1,
/// clipped to `1.0` and encoded `255.0`. The same red column carries green
/// `-0.97842103 * 0.60974 + 1.9158427 * 0.31111 + 0.033399127 * 0.01947 =
/// 0.000105665` and blue `0.072035536 * 0.60974 - 0.22903204 * 0.31111 +
/// 1.4057162 * 0.01947 = 0.0000380842`, both inside the knee, so the
/// `12.92` branch encodes them to `0.3481` and `0.1255`.
#[test]
fn an_out_of_gamut_device_colour_clips_to_the_display_range() {
    let adobe = profile(
        b"mntr",
        b"RGB ",
        b"XYZ ",
        &[
            (*b"rXYZ", xyz(ADOBE_R_XYZ)),
            (*b"gXYZ", xyz(ADOBE_G_XYZ)),
            (*b"bXYZ", xyz(ADOBE_B_XYZ)),
            (*b"rTRC", curv(&[0x0233])),
            (*b"gTRC", curv(&[0x0233])),
            (*b"bTRC", curv(&[0x0233])),
        ],
    );
    let transform = require_ok!(icc_route(&adobe, 8));
    let rows = mapped(&transform, &[0.0, 255.0, 0.0, 255.0, 0.0, 0.0]);
    // The clipped channels are exactly 0, not merely inside the tolerance.
    let green = row(&rows, 0);
    assert_eq!(green.r.to_bits(), 0.0_f32.to_bits());
    assert_eq!(green.b.to_bits(), 0.0_f32.to_bits());
    assert_close(green, (0.0, 254.9852, 0.0));
    let red = row(&rows, 1);
    assert_close(red, (255.0, 0.3481, 0.1255));
    assert_eq!(replies(green), [0, 255, 0]);
    assert_eq!(replies(red), [255, 0, 0]);
    // The D-29 colour window acts on the clipped values. Its lower end is
    // below 0, so the stored 0 windows to 64. The unclipped -1311.9 stores
    // as the same byte 0 but windows to 0, 64 codes away from the clipped
    // value, a difference the byte rule cannot see because both store 0.
    let window = require_ok!(ColourWindow::new(64.0, 256.0, false));
    assert!(!window.is_identity());
    let mut windowed = [Rgb::BLACK; 2];
    assert!(window.map_into(&[green, red], &mut windowed).is_ok());
    assert_eq!(replies(windowed[0]), [64, 255, 64]);
    assert_eq!(replies(windowed[1]), [255, 64, 64]);
}

// ---------------------------------------------------------------------------
// AC5. The colourants are columns, ICC.1:2022 Annex F.3.
// ---------------------------------------------------------------------------

/// AC5. With `rXYZ` and `gXYZ` exchanged, device red leaves through the green
/// colourant and replies green, and device green replies red. Hand computed
/// on device `255 0 0`: the normalised input `1 0 0` meets the matrix, whose
/// product with the swapped colourants maps the second colourant column to
/// the second unit vector, so the encoded reply is `0 255 0`. The transposed
/// wrong implementation, which builds the colourants as rows, answers the
/// dot products of the constant's rows with the first components
/// `0.3851019, 0.4360303, 0.1430678` on this row: `3.1339235 * 0.3851019 -
/// 1.616923 * 0.4360303 - 0.4907337 * 0.1430678 = 0.4316441`, encoded
/// `175.5416`, and `0.4633521` and `0.1289888` the same way, encoded
/// `181.2241` and `100.5762`, bytes `176 181 101`. No row here is within the
/// tolerance of any of those.
#[test]
fn colourants_are_columns_not_rows() {
    let swapped = profile(
        b"mntr",
        b"RGB ",
        b"XYZ ",
        &[
            (*b"rXYZ", xyz(G_XYZ)),
            (*b"gXYZ", xyz(R_XYZ)),
            (*b"bXYZ", xyz(B_XYZ)),
            (*b"rTRC", curv(&[0x0100])),
            (*b"gTRC", curv(&[0x0100])),
            (*b"bTRC", curv(&[0x0100])),
        ],
    );
    let transform = require_ok!(icc_route(&swapped, 8));
    let rows = mapped(&transform, &[255.0, 0.0, 0.0, 0.0, 255.0, 0.0]);
    assert_eq!(replies(row(&rows, 0)), [0, 255, 0]);
    assert_eq!(replies(row(&rows, 1)), [255, 0, 0]);
    // The strong channel of each row, against its ideal of 255. The weak
    // channels are quantisation noise around zero, far inside one code and
    // far from the transposed implementation's `175.5416`.
    assert!((row(&rows, 0).g - 255.0).abs() < TOLERANCE);
    assert!(row(&rows, 0).r.abs() < 1.0);
    assert!(row(&rows, 0).b.abs() < 1.0);
    assert!((row(&rows, 1).r - 255.0).abs() < TOLERANCE);
    assert!(row(&rows, 1).g.abs() < 1.0);
    assert!(row(&rows, 1).b.abs() < 1.0);
}

// ---------------------------------------------------------------------------
// AC6. Every shape outside the subset is refused by name.
// ---------------------------------------------------------------------------

/// AC6. Each named refusal, one profile per row of the plan. A profile that
/// carries a lookup-table tag is refused even when the six matrix tags are
/// present, the operator's answer to question 2, because a profile with a
/// lookup table asks for the lookup-table answer.
#[test]
fn icc_profiles_outside_the_subset_are_named_and_not_applied() {
    let matrix_curves = (curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100]));
    let six = |extra: Option<Tag>| {
        let mut tags = vec![
            (*b"rXYZ", xyz(R_XYZ)),
            (*b"gXYZ", xyz(G_XYZ)),
            (*b"bXYZ", xyz(B_XYZ)),
            (*b"rTRC", matrix_curves.0.clone()),
            (*b"gTRC", matrix_curves.1.clone()),
            (*b"bTRC", matrix_curves.2.clone()),
        ];
        if let Some(tag) = extra {
            tags.push(tag);
        }
        profile(b"mntr", b"RGB ", b"XYZ ", &tags)
    };

    // Another profile class, Table 18.
    for class in [b"prtr", b"link", b"spac", b"abst", b"nmcl"] {
        let bytes = profile(class, b"RGB ", b"XYZ ", &[]);
        assert_eq!(
            IccProfile::parse(&bytes),
            Err(IccError::UnsupportedClass),
            "{class:?}"
        );
    }
    // A Lab PCS, Table 19, and a GRAY device side.
    let lab = profile(b"mntr", b"RGB ", b"Lab ", &[]);
    assert_eq!(IccProfile::parse(&lab), Err(IccError::UnsupportedPcs));
    let gray = profile(b"mntr", b"GRAY", b"XYZ ", &[]);
    assert_eq!(
        IccProfile::parse(&gray),
        Err(IccError::UnsupportedColourSpace)
    );
    // A missing required tag.
    let missing = profile(
        b"mntr",
        b"RGB ",
        b"XYZ ",
        &[
            (*b"rXYZ", xyz(R_XYZ)),
            (*b"gXYZ", xyz(G_XYZ)),
            (*b"rTRC", curv(&[0x0100])),
            (*b"gTRC", curv(&[0x0100])),
            (*b"bTRC", curv(&[0x0100])),
        ],
    );
    assert_eq!(IccProfile::parse(&missing), Err(IccError::MissingTag));
    // A curv count whose entries do not fit the tag, 10.6.
    let mut truncated = vec![0_u8; 16];
    truncated
        .get_mut(0..4)
        .unwrap_or_default()
        .copy_from_slice(b"curv");
    truncated
        .get_mut(8..12)
        .unwrap_or_default()
        .copy_from_slice(&3_u32.to_be_bytes());
    let short_table = profile(
        b"mntr",
        b"RGB ",
        b"XYZ ",
        &[
            (*b"rXYZ", xyz(R_XYZ)),
            (*b"gXYZ", xyz(G_XYZ)),
            (*b"bXYZ", xyz(B_XYZ)),
            (*b"rTRC", truncated),
            (*b"gTRC", curv(&[0x0100])),
            (*b"bTRC", curv(&[0x0100])),
        ],
    );
    assert_eq!(IccProfile::parse(&short_table), Err(IccError::Malformed));
    // A profile above the size constant. It carries the six matrix tags, so
    // the size is the only thing it can be refused for, with a payload of
    // filler past them.
    let filler = vec![0_u8; IccProfile::max_bytes()];
    let large = six(Some((*b"desc", filler)));
    assert_eq!(IccProfile::parse(&large), Err(IccError::TooLarge));
    // A lookup-table profile above the size constant is named for what it
    // is and not for its size: the header and the tag table are read from
    // any length, and the refusal comes before any payload is read.
    let lut_filler = vec![0_u8; IccProfile::max_bytes()];
    let large_lut = profile(b"mntr", b"RGB ", b"XYZ ", &[(*b"A2B0", lut_filler)]);
    assert_eq!(
        IccProfile::parse(&large_lut),
        Err(IccError::LookupTableProfile)
    );
    // A lookup-table tag alone, and beside all six matrix tags. Each of the
    // seven lookup-table tags names the same refusal, so dropping one from
    // the parser's list would be visible here.
    for lut_tag in [
        *b"A2B0", *b"A2B1", *b"A2B2", *b"D2B0", *b"D2B1", *b"D2B2", *b"D2B3",
    ] {
        let lut_alone = profile(b"mntr", b"RGB ", b"XYZ ", &[(lut_tag, curv(&[0x0100]))]);
        assert_eq!(
            IccProfile::parse(&lut_alone),
            Err(IccError::LookupTableProfile),
            "{lut_tag:?}"
        );
    }
    let lut_beside = six(Some((*b"A2B0", curv(&[0x0100]))));
    assert_eq!(
        IccProfile::parse(&lut_beside),
        Err(IccError::LookupTableProfile)
    );
    // A colourant tag that is not an `XYZType`, 10.31 Table 85. The payload
    // is a `curv` of 20 bytes, so the tag size alone does not refuse it and
    // the type signature is the only guard that can.
    let mut not_xyz = vec![0_u8; 20];
    not_xyz
        .get_mut(0..4)
        .unwrap_or_default()
        .copy_from_slice(b"curv");
    not_xyz
        .get_mut(8..12)
        .unwrap_or_default()
        .copy_from_slice(&1_u32.to_be_bytes());
    not_xyz
        .get_mut(12..14)
        .unwrap_or_default()
        .copy_from_slice(&0x0100_u16.to_be_bytes());
    let wrong_colourant = profile(
        b"mntr",
        b"RGB ",
        b"XYZ ",
        &[
            (*b"rXYZ", not_xyz),
            (*b"gXYZ", xyz(G_XYZ)),
            (*b"bXYZ", xyz(B_XYZ)),
            (*b"rTRC", curv(&[0x0100])),
            (*b"gTRC", curv(&[0x0100])),
            (*b"bTRC", curv(&[0x0100])),
        ],
    );
    assert_eq!(
        IccProfile::parse(&wrong_colourant),
        Err(IccError::Malformed)
    );
    // A parametric function type outside 0 to 4, Table 68.
    let unknown_function = para(5, &[0x0001_0000]);
    let unsupported = srgb_profile(
        unknown_function.clone(),
        unknown_function.clone(),
        unknown_function,
    );
    assert_eq!(
        IccProfile::parse(&unsupported),
        Err(IccError::UnsupportedCurve)
    );
    // A tag table whose count claims entries the profile does not hold.
    let mut short_count = srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100]));
    short_count
        .get_mut(128..132)
        .unwrap_or_default()
        .copy_from_slice(&9_u32.to_be_bytes());
    assert_eq!(IccProfile::parse(&short_count), Err(IccError::Malformed));
    // A header that is not the profile's own size.
    let mut wrong_size = srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100]));
    if let Some(slot) = wrong_size.get_mut(0..1) {
        slot.copy_from_slice(&[0x10]);
    }
    assert_eq!(IccProfile::parse(&wrong_size), Err(IccError::Malformed));
    // A header without the profile file signature of 7.2.9.
    let mut no_signature = srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100]));
    if let Some(slot) = no_signature.get_mut(36..37) {
        slot.copy_from_slice(b"x");
    }
    assert_eq!(IccProfile::parse(&no_signature), Err(IccError::Malformed));
}

// ---------------------------------------------------------------------------
// D7. The guards the parser and the transform carry, each observed firing.
// ---------------------------------------------------------------------------

/// D7. A `para` type 1 whose `b` arrives as a negative s15Fixed16Number, the
/// normal way to write a CIE 122-1966 curve with a foot. The parameters are
/// `g = 2.0` as `0x00020000`, `a = 1.0` as `0x00010000` and `b = -0.5` as
/// `0xFFFF8000`, so the power branch runs at and above `-b/a = 0.5` and the
/// answer is 0 below it, Table 68. A sign dropped in the s15Fixed16 decode
/// would read `b` as `+0.5` and answer `(x + 0.5)^2` everywhere.
///
/// Hand computation: device `191` is `191/255 = 0.7490196`, above the
/// boundary, so the curve answers `(0.7490196 - 0.5)^2 = 0.0620108` and the
/// sRGB encoding gives `1.055 * 0.0620108^(1/2.4) - 0.055 = 0.2762183`,
/// times 255 `70.4356`. Device `128` is `0.5019608`, just above the
/// boundary: `(0.0019608)^2 = 0.0000038`, which encodes through the `12.92`
/// branch to `0.0127`. Devices `127`, `0.4980392`, and `64` are below the
/// boundary and encode 0 exactly.
#[test]
fn a_negative_s15_fixed16_parameter_keeps_its_sign() {
    let curve = para(1, &[0x0002_0000, 0x0001_0000, 0xFFFF_8000]);
    let transform = require_ok!(icc_route(
        &srgb_profile(curve.clone(), curve.clone(), curve),
        8
    ));
    let rows = mapped(
        &transform,
        &[
            191.0, 191.0, 191.0, 128.0, 128.0, 128.0, 127.0, 127.0, 127.0, 64.0, 64.0, 64.0,
        ],
    );
    assert_close(row(&rows, 0), (70.4356, 70.4356, 70.4356));
    assert_close(row(&rows, 1), (0.0127, 0.0127, 0.0127));
    // The two below-boundary rows are exactly black.
    assert_eq!(row(&rows, 2).r.to_bits(), 0.0_f32.to_bits());
    assert_eq!(row(&rows, 3).r.to_bits(), 0.0_f32.to_bits());
    assert_eq!(replies(row(&rows, 0)), [70, 70, 70]);
    assert_eq!(replies(row(&rows, 2)), [0, 0, 0]);
    assert_eq!(replies(row(&rows, 3)), [0, 0, 0]);
}

/// D7. A `para` type 2 whose value exceeds 1 at the top, which is what the
/// 10.6 and 10.18 range clip is for. The profile is the Adobe RGB 1998
/// colourant fixture above with the type 2 curve `g = 2.0` as
/// `0x00020000`, `a = 1.0`, `b = 0.0` and `c = 0.1` as `0x0000199A`, which
/// decodes to `6554/65536 = 0.1000061`, on all three channels: the curve is
/// `x^2 + 0.1000061`, so device `255` asks it for `1.1000061`.
///
/// Hand computation of device `0 255 0`: the curve outputs are
/// `0.1000061` red, `1.1000061` green clipped to `1.0` and `0.1000061`
/// blue. The product matrix's columns, from the printed values, are
/// `1.3982830 0.0001056 0.0000381` red, `-0.3981993 0.9998680 -0.0429451`
/// green and the blue column
/// `3.1339235 * 0.14919 - 1.616923 * 0.06322 - 0.4907337 * 0.74457 =
/// 0.46755005153 - 0.10222187045 - 0.36538560062 = -0.00005741955` red,
/// `-0.97842103 * 0.14919 + 1.9158427 * 0.06322 + 0.033399127 * 0.74457 =
/// -0.14597063383 + 0.12111957248 + 0.02486798831 = 0.00001692696` green and
/// `0.072035536 * 0.14919 - 0.22903204 * 0.06322 + 1.4057162 * 0.74457 =
/// 0.01074698164 - 0.01447940555 + 1.04665409674 = 1.04292167283` blue. The linear value is
/// `0.1000061` times the red column, `1.0` times the green column and
/// `0.1000061` times the blue column: `-0.2583682, 0.9998803, 0.0613573`.
/// A.7 clips the red to 0, which encodes to exactly 0. Green encodes to
/// `1.055 * 0.9998803^(1/2.4) - 0.055 = 0.9999474`, times 255 `254.9866`.
/// Blue encodes to `1.055 * 0.0613573^(1/2.4) - 0.055 = 0.2747590`, times
/// 255 `70.0635`. Without the range clip the green curve output stays
/// `1.1000061`, the linear blue becomes `0.0570625` and encodes to
/// `67.5591`, which no row here is within the tolerance of: the sRGB
/// colourant fixture cannot see this clip, because its product is near the
/// identity and the wide-gamut columns are what carry the difference.
#[test]
fn a_curve_value_above_one_clips_to_the_encoding_range() {
    let curve = para(2, &[0x0002_0000, 0x0001_0000, 0x0000_0000, 0x0000_199A]);
    let adobe = profile(
        b"mntr",
        b"RGB ",
        b"XYZ ",
        &[
            (*b"rXYZ", xyz(ADOBE_R_XYZ)),
            (*b"gXYZ", xyz(ADOBE_G_XYZ)),
            (*b"bXYZ", xyz(ADOBE_B_XYZ)),
            (*b"rTRC", curve.clone()),
            (*b"gTRC", curve.clone()),
            (*b"bTRC", curve),
        ],
    );
    let transform = require_ok!(icc_route(&adobe, 8));
    let rows = mapped(&transform, &[0.0, 255.0, 0.0]);
    assert_eq!(row(&rows, 0).r.to_bits(), 0.0_f32.to_bits());
    assert_close(row(&rows, 0), (0.0, 254.9866, 70.0635));
    assert_eq!(replies(row(&rows, 0)), [0, 255, 70]);
}

// ---------------------------------------------------------------------------
// AC7. A frame without (0028,2000) keeps F-239's route byte for byte.
// ---------------------------------------------------------------------------

/// AC7. No profile attached. An 8-bit RGB frame leaves stage 4 as its own
/// samples, and a 12-bit frame leaves through F-239's display scale, the
/// exact floor of the code over `2^(n - 8)`. Both are the values the colour
/// stage already published, hand computed from PS3.3 C.7.6.3.1 and F-239's
/// shift rule, not from running the profile path.
#[test]
fn no_profile_is_byte_identical_to_f_239() {
    let eight = require_ok!(rgb_route(8));
    let rows = mapped(&eight, &[64.0, 128.0, 192.0, 10.0, 20.0, 30.0]);
    assert_eq!(row(&rows, 0).r.to_bits(), 64.0_f32.to_bits());
    assert_eq!(row(&rows, 0).g.to_bits(), 128.0_f32.to_bits());
    assert_eq!(row(&rows, 0).b.to_bits(), 192.0_f32.to_bits());
    assert_eq!(row(&rows, 1).r.to_bits(), 10.0_f32.to_bits());
    let twelve = require_ok!(rgb_route(12));
    let rows = mapped(&twelve, &[4095.0, 2048.0, 0.0]);
    assert_eq!(row(&rows, 0).r.to_bits(), 255.0_f32.to_bits());
    assert_eq!(row(&rows, 0).g.to_bits(), 128.0_f32.to_bits());
    assert_eq!(row(&rows, 0).b.to_bits(), 0.0_f32.to_bits());
}

// ---------------------------------------------------------------------------
// Palette colour, whose device values are the entries at their own depth.
// ---------------------------------------------------------------------------

/// A palette frame with the AC1 profile. The device values are the raw
/// entries, normalised by each channel's declared depth of 16. Two hand
/// computations of the second row, where red carries 65,535, green 32,767
/// and blue 16,384.
///
/// The unquantised reference, from the sRGB encoding formula alone:
/// `65535 / 65535 = 1.0` encodes to `255`. `32767 / 65535 = 0.4999924` is
/// above the `0.0031308` knee, so it encodes to
/// `1.055 * 0.4999924^(1/2.4) - 0.055 = 0.7353520`, times 255 `187.5147`.
/// `16384 / 65535 = 0.2500038` encodes to
/// `1.055 * 0.2500038^(1/2.4) - 0.055 = 0.5371025`, times 255 `136.9611`.
///
/// The assert, from the profile's own s15Fixed16 colourant words, ICC.1
/// Table 4, times the printed inverse. The words over 65,536 are
/// `0.43603516, 0.22244263, 0.01390076` red, `0.38510132, 0.71694946,
/// 0.09707642` green and `0.14306641, 0.06062317, 0.71392822` blue, each at
/// most `6.7e-6` from the printed decimal. Their product with the printed
/// inverse maps the normalised row to linear `0.99999733, 0.50000491,
/// 0.25000752` in `f64`, and to `0.99999738, 0.50000489, 0.25000751` in the
/// `f32` the pipeline computes, which encodes to the reply
/// `254.99971, 187.51683, 136.96205`. The `f64` values encode to
/// `254.9997`, `187.5169` and `136.9621`, the values the assert names. The
/// `1e-3` bound is about thirteen times the largest `f32` gap between the
/// reply and the assert's literal on this row, `7.6e-5` at the green, of
/// which `2.4e-5` is the pipeline's own against the unrounded `f64`
/// encoding `187.51685` and the rest is that literal's rounding of the hand
/// value. Normalising by `2^16` rather than `2^16 - 1` moves the green by
/// `1.3e-3`, which the former
/// `1e-2` bound tolerated and this one refuses. The display scale that
/// would have shifted these entries by 8 bits is not applied: the profile
/// replaces it.
const PALETTE_TOLERANCE: f32 = 1e-3;
#[test]
fn palette_entries_are_normalised_by_their_declared_depth() {
    let red = require_ok!(ocelli_pixel::LutDescriptor::new(
        2,
        0,
        16,
        vec![0.0, 65_535.0]
    ));
    let green = require_ok!(ocelli_pixel::LutDescriptor::new(
        2,
        0,
        16,
        vec![0.0, 32_767.0]
    ));
    let blue = require_ok!(ocelli_pixel::LutDescriptor::new(
        2,
        0,
        16,
        vec![0.0, 16_384.0]
    ));
    let palette = require_ok!(ocelli_pixel::PaletteColorLut::with_entry_bits(
        red,
        green,
        blue,
        [16, 16, 16]
    ));
    let layout = require_ok!(SampleLayout::new(
        1,
        PhotometricInterpretation::PaletteColor,
        None
    ));
    let resolved = require_ok!(ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        Some(palette),
    ));
    let parsed = IccProfile::parse(&srgb_profile(
        curv(&[0x0100]),
        curv(&[0x0100]),
        curv(&[0x0100]),
    ));
    assert!(parsed.is_ok(), "{parsed:?}");
    let Ok(parsed) = parsed else { return };
    let transform = resolved.with_icc(IccTransform::from(parsed));
    // One sample per pixel, so the destination is one RGB per sample.
    let source = stored(&[0.0, 1.0]);
    let mut destination = vec![Rgb::BLACK; 2];
    assert!(transform.map_into(&source, &mut destination, 2).is_ok());
    let rows = destination;
    // Entry 0 is black through the encoding, and each channel answers its
    // own entry over its own encoding range, against the hand values above.
    let close = |actual: Rgb, expected: (f32, f32, f32)| {
        let (r, g, b) = expected;
        assert!(
            (actual.r - r).abs() < PALETTE_TOLERANCE
                && (actual.g - g).abs() < PALETTE_TOLERANCE
                && (actual.b - b).abs() < PALETTE_TOLERANCE,
            "was ({}, {}, {}), wanted ({r}, {g}, {b})",
            actual.r,
            actual.g,
            actual.b
        );
    };
    close(row(&rows, 0), (0.0, 0.0, 0.0));
    close(row(&rows, 1), (254.9997, 187.5169, 136.9621));
    assert_eq!(replies(row(&rows, 0)), [0, 0, 0]);
    assert_eq!(replies(row(&rows, 1)), [255, 188, 137]);
}

/// A palette whose wide channels hold only entries of at most 255 is read as
/// 8-bit by ruling R1, so the profile normalises by 255 and an entry of 255
/// is white. Normalising by the declared 16 instead would answer the
/// encoding of `255 / 65535 = 0.003893`, about `12.6` on this row, which no
/// row here is within the tolerance of.
#[test]
fn a_palette_read_as_8_bit_normalises_by_255() {
    let red = require_ok!(ocelli_pixel::LutDescriptor::new(2, 0, 16, vec![0.0, 255.0]));
    let green = require_ok!(ocelli_pixel::LutDescriptor::new(2, 0, 16, vec![0.0, 255.0]));
    let blue = require_ok!(ocelli_pixel::LutDescriptor::new(2, 0, 16, vec![0.0, 128.0]));
    let palette = require_ok!(ocelli_pixel::PaletteColorLut::with_entry_bits(
        red,
        green,
        blue,
        [16, 16, 16]
    ));
    assert!(palette.entries_read_as_8_bit());
    let layout = require_ok!(SampleLayout::new(
        1,
        PhotometricInterpretation::PaletteColor,
        None
    ));
    let resolved = require_ok!(ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        Some(palette),
    ));
    let parsed = IccProfile::parse(&srgb_profile(
        curv(&[0x0100]),
        curv(&[0x0100]),
        curv(&[0x0100]),
    ));
    assert!(parsed.is_ok(), "{parsed:?}");
    let Ok(parsed) = parsed else { return };
    let transform = resolved.with_icc(IccTransform::from(parsed));
    let source = stored(&[0.0, 1.0]);
    let mut destination = vec![Rgb::BLACK; 2];
    assert!(transform.map_into(&source, &mut destination, 2).is_ok());
    // Entry 0 is black, entry 255 is white in red and green and the encode
    // of 128 / 255 = 0.501961 in blue, which is `187.845`.
    assert_close(row(&destination, 0), (0.0, 0.0, 0.0));
    assert_close(row(&destination, 1), (255.0, 255.0, 187.845));
    assert_eq!(replies(row(&destination, 0)), [0, 0, 0]);
    assert_eq!(replies(row(&destination, 1)), [255, 255, 188]);
}

// ---------------------------------------------------------------------------
// D8. The three YCbCr routes with a profile attached, PS3.3 C.11.15.1.1.
// ---------------------------------------------------------------------------

/// The colour route for a YBR frame. The layout is interleaved, which is
/// what a decoder's own output is.
fn ybr_route(photometric: PhotometricInterpretation) -> Result<ColorTransform, PixelError> {
    let layout = SampleLayout::new(3, photometric, Some(PlanarConfiguration::Interleaved))?;
    ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    )
}

/// The YBR colour route with the profile parsed and attached. The device
/// depths are 8, `with_icc`'s rule: the YCbCr matrices consume 8-bit samples
/// and return 8-bit device RGB.
fn ybr_with_profile(
    photometric: PhotometricInterpretation,
    profile_bytes: &[u8],
) -> Result<ColorTransform, PixelError> {
    let parsed = IccProfile::parse(profile_bytes);
    assert!(parsed.is_ok(), "{parsed:?}");
    let Ok(parsed) = parsed else {
        return ybr_route(photometric);
    };
    ybr_route(photometric).map(|route| route.with_icc(IccTransform::from(parsed)))
}

/// D8. A `YBR_FULL` frame with the AC1 profile. The profile's input is RGB
/// after the YCbCr conversion, PS3.3 C.11.15.1.1: it is applied "regardless
/// of the Photometric Interpretation of the image pixel data prior to
/// decompression". Dropping the profile from the route would answer the raw
/// YCbCr conversion, `204.8648 148.1603 103.2924`, which no row here is
/// within the tolerance of.
///
/// Hand computation on samples `Y = 160, Cb = 96, Cr = 160`, through the
/// exact rational inverse of PS3.3 C.7.6.3.1.2 that `color.rs` carries, with
/// `Cb' = 96 - 128 = -32` and `Cr' = 160 - 128 = 32`:
///
/// ```text
/// R = 160 - 0.000036820 * (-32) + 1.401987577 * 32  = 204.8647807
/// G = 160 - 0.344113281 * (-32) - 0.714103821 * 32  = 148.1603027
/// B = 160 + 1.771978117 * (-32) - 0.000134583 * 32  = 103.2923937
/// ```
///
/// The AC1 curve is the identity, so each device value normalises by 255
/// and encodes: `204.8647807 / 255 = 0.8033913` encodes to `0.9080277`,
/// times 255 `231.5471`. `148.1603027 / 255 = 0.5810208` encodes to
/// `0.7863932`, times 255 `200.5303`. And `103.2923937 / 255 = 0.4050682`
/// encodes to `0.6689733`, times 255 `170.5882`.
#[test]
fn a_ybr_full_frame_with_a_profile_converts_then_profiles() {
    let transform = require_ok!(ybr_with_profile(
        PhotometricInterpretation::YbrFull,
        &srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100])),
    ));
    let rows = mapped(&transform, &[160.0, 96.0, 160.0]);
    assert_close(row(&rows, 0), (231.5471, 200.5303, 170.5882));
    assert_eq!(replies(row(&rows, 0)), [232, 201, 171]);
}

/// D8. A `YBR_FULL_422` frame with the AC1 profile. Each pair of pixels is
/// stored `Y1 Y2 Cb Cr`, PS3.3 C.7.6.3.1.2, and both share the one chroma
/// sample, so the stored frame is `160 96 96 160` for two pixels.
///
/// Hand computation of the second pixel, `Y = 96` with the same chroma as
/// the first: `R = 96 + 0.0011782 + 44.8636025 = 140.8647807`, `G = 96 +
/// 11.0116250 - 22.8513223 = 84.1603027` and `B = 96 - 56.7032997 -
/// 0.0043067 = 39.2923937`. Encoded: `140.8647807 / 255 = 0.5524109`
/// encodes to `0.7688758`, times 255 `196.0633`. `84.1603027 / 255 =
/// 0.3300404` encodes to `0.6097455`, times 255 `155.4851`. And
/// `39.2923937 / 255 = 0.1540878` encodes to `0.4289746`, times 255
/// `109.3885`. The first pixel is the `YBR_FULL` row above.
#[test]
fn a_ybr_full_422_frame_with_a_profile_converts_then_profiles() {
    let transform = require_ok!(ybr_with_profile(
        PhotometricInterpretation::YbrFull422,
        &srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100])),
    ));
    // Two pixels, stored `Y1 Y2 Cb Cr`, two samples per pixel.
    let source = stored(&[160.0, 96.0, 96.0, 160.0]);
    let mut destination = vec![Rgb::BLACK; 2];
    assert!(transform.map_into(&source, &mut destination, 2).is_ok());
    assert_close(row(&destination, 0), (231.5471, 200.5303, 170.5882));
    assert_close(row(&destination, 1), (196.0633, 155.4851, 109.3885));
    assert_eq!(replies(row(&destination, 0)), [232, 201, 171]);
    assert_eq!(replies(row(&destination, 1)), [196, 155, 109]);
}

/// D8. A `YBR_PARTIAL_422` frame with the AC1 profile, through the
/// partial-range inverse whose `+ 16` offset `color.rs` carries: `Y' = Y -
/// 16`, the same `Cb` and `Cr` offsets as the full range.
///
/// Hand computation of the first pixel, `Y = 160, Cb = 96, Cr = 160`, so
/// `Y' = 144`: `R = 1.164415463 * 144 - 0.000095036 * (-32) +
/// 1.596001878 * 32 = 218.7509280`, `G = 1.164415463 * 144 -
/// 0.391724564 * (-32) - 0.813013368 * 32 = 154.1945849` and `B =
/// 1.164415463 * 144 + 2.017290682 * (-32) - 0.000135273 * 32 =
/// 103.1181961`. Encoded: `218.7509280 / 255 = 0.8578468` encodes to
/// `0.9347067`, times 255 `238.3502`. `154.1945849 / 255 = 0.6046846`
/// encodes to `0.8005056`, times 255 `204.1289`. And `103.1181961 / 255 =
/// 0.4043851` encodes to `0.6684643`, times 255 `170.4584`. The second
/// pixel, `Y' = 80`: `144.2283384`, `79.6719953` and `28.5956066`, encoding
/// to `198.1391`, `151.6581` and `94.0833`. An implementation omitting the
/// `+ 16` would read the first pixel's Y through an offset of 0 and answer
/// `237.3816 172.8252 121.7488`, encoding to `247.0932`, `214.7475` and
/// `183.6771`, which no row here is within the tolerance of.
#[test]
fn a_ybr_partial_422_frame_with_a_profile_converts_then_profiles() {
    let transform = require_ok!(ybr_with_profile(
        PhotometricInterpretation::YbrPartial422,
        &srgb_profile(curv(&[0x0100]), curv(&[0x0100]), curv(&[0x0100])),
    ));
    // Two pixels, stored `Y1 Y2 Cb Cr`, two samples per pixel.
    let source = stored(&[160.0, 96.0, 96.0, 160.0]);
    let mut destination = vec![Rgb::BLACK; 2];
    assert!(transform.map_into(&source, &mut destination, 2).is_ok());
    assert_close(row(&destination, 0), (238.3502, 204.1289, 170.4584));
    assert_close(row(&destination, 1), (198.1391, 151.6581, 94.0833));
    assert_eq!(replies(row(&destination, 0)), [238, 204, 170]);
    assert_eq!(replies(row(&destination, 1)), [198, 152, 94]);
}
