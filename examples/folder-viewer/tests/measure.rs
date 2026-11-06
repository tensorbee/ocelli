//! Measurements on a held frame. Every expected number is worked by hand
//! from the local rules of `.claude/plans/F-211-design.md` and the DICOM
//! sections each test names, not from a previous run of this route.
//!
//! Spacing: PS3.3 C.7.6.2, Pixel Spacing is "adjacent row spacing
//! (delimiter) adjacent column spacing in mm", and PS3.3 C.7.6.2.1.1 scales
//! the column index by the column spacing and the row index by the row
//! spacing. A page point `(x, y)` is index `(x - 0.5, y - 0.5)` and the
//! plane-local millimetre point `((x - 0.5) * column, (y - 0.5) * row)`.
//! Modality values: PS3.3 C.11.1, `m * SV + b`. Statistics: the F-075
//! population form. Areas: F-074 rectangle and ellipse, F-076 shoelace.
//! Calibration: F-195, PS3.3 C.8.11.4 and C.8.7.5.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use ocelli_folder_viewer::{
    ImagePoint, MeasureShape, PreviewError, Viewer, ViewerConfig, measure_frame, serve_on, slice,
};

// ---------------------------------------------------------------------------
// Builders. Elements are sorted by tag before they are written.

struct Spec {
    rows: u16,
    columns: u16,
    signed: bool,
    modality: &'static str,
    rescale: Option<(&'static str, &'static str)>,
    extra: Vec<((u16, u16), Vec<u8>)>,
    pixels: Vec<u8>,
}

impl Spec {
    fn grid(columns: u16, rows: u16, pixels: &[u16]) -> Self {
        let mut bytes = Vec::new();
        for value in pixels {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Self {
            rows,
            columns,
            signed: false,
            modality: "OT",
            rescale: None,
            extra: Vec::new(),
            pixels: bytes,
        }
    }

    fn text(&mut self, tag: (u16, u16), vr: [u8; 2], text: &str) -> Result<(), String> {
        self.extra.push((tag, short_le(tag, vr, &text_pad(text))?));
        Ok(())
    }
}

fn build(spec: Spec) -> Result<Vec<u8>, String> {
    let mut elements: Vec<((u16, u16), Vec<u8>)> = Vec::new();
    let sop = ui(b"1.2.840.10008.5.1.4.1.1.7");
    elements.push(((0x0008, 0x0016), short_le((0x0008, 0x0016), *b"UI", &sop)?));
    elements.push((
        (0x0008, 0x0060),
        short_le((0x0008, 0x0060), *b"CS", &text_pad(spec.modality))?,
    ));
    let unsigned = [
        ((0x0028, 0x0002), 1),
        ((0x0028, 0x0010), spec.rows),
        ((0x0028, 0x0011), spec.columns),
        ((0x0028, 0x0100), 16),
        ((0x0028, 0x0101), 16),
        ((0x0028, 0x0102), 15),
        ((0x0028, 0x0103), u16::from(spec.signed)),
    ];
    for (tag, value) in unsigned {
        elements.push((tag, short_le(tag, *b"US", &value.to_le_bytes())?));
    }
    elements.push((
        (0x0028, 0x0004),
        short_le((0x0028, 0x0004), *b"CS", &text_pad("MONOCHROME2"))?,
    ));
    if let Some((slope, intercept)) = spec.rescale {
        elements.push((
            (0x0028, 0x1052),
            short_le((0x0028, 0x1052), *b"DS", &text_pad(intercept))?,
        ));
        elements.push((
            (0x0028, 0x1053),
            short_le((0x0028, 0x1053), *b"DS", &text_pad(slope))?,
        ));
    }
    elements.extend(spec.extra);
    elements.push((
        (0x7FE0, 0x0010),
        long_le((0x7FE0, 0x0010), *b"OW", &spec.pixels)?,
    ));
    elements.sort_by_key(|(tag, _)| *tag);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta()?);
    for (_, element) in elements {
        bytes.extend(element);
    }
    Ok(bytes)
}

fn file_meta() -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(short_le(
        (0x0002, 0x0002),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.7"),
    )?);
    body.extend(short_le((0x0002, 0x0003), *b"UI", &ui(b"2.25.1"))?);
    body.extend(short_le(
        (0x0002, 0x0010),
        *b"UI",
        &ui(b"1.2.840.10008.1.2.1"),
    )?);
    body.extend(short_le((0x0002, 0x0012), *b"UI", &ui(b"2.25.2"))?);
    let length = u32::try_from(body.len()).map_err(|_| "meta length".to_owned())?;
    let mut meta = short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes())?;
    meta.extend(body);
    Ok(meta)
}

fn ui(text: &[u8]) -> Vec<u8> {
    let mut value = text.to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    value
}

fn text_pad(text: &str) -> Vec<u8> {
    let mut value = text.as_bytes().to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(b' ');
    }
    value
}

fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u16::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u32::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

/// The 5 by 5 grid `v(c, r) = c + 10 * r`, identity rescale, no spacing.
fn value_grid() -> Result<Vec<u8>, String> {
    let mut pixels = Vec::new();
    for row in 0..5_u16 {
        for column in 0..5_u16 {
            pixels.push(column + 10 * row);
        }
    }
    build(Spec::grid(5, 5, &pixels))
}

fn at(x: f64, y: f64) -> ImagePoint {
    ImagePoint { x, y }
}

fn measured(
    file: &[u8],
    shape: MeasureShape,
    points: &[ImagePoint],
    name: &str,
) -> Result<String, String> {
    measure_frame(file, 0, shape, points)
        .map_err(|error| format!("{name}: refused with {}", error.code()))
}

fn expect(got: Result<String, String>, want: &str, name: &str) -> Result<(), String> {
    let got = got?;
    if got == want {
        Ok(())
    } else {
        Err(format!("{name}: got {got}, wanted {want}"))
    }
}

fn expect_refusal(
    file: &[u8],
    shape: MeasureShape,
    points: &[ImagePoint],
    want: PreviewError,
    name: &str,
) -> Result<(), String> {
    match measure_frame(file, 0, shape, points) {
        Err(error) if error == want => Ok(()),
        Err(error) => Err(format!("{name}: refused with {}", error.code())),
        Ok(json) => Err(format!("{name}: answered {json}")),
    }
}

fn contains(json: &str, part: &str, name: &str) -> Result<(), String> {
    if json.contains(part) {
        Ok(())
    } else {
        Err(format!("{name}: {json} has no {part}"))
    }
}

// ---------------------------------------------------------------------------
// The region rule through `measure_frame`, the composition the route uses.

#[test]
fn measure_rectangle_half_pixel() -> Result<(), String> {
    // AC1. Page (1, 1) and (3, 3) are index (0.5, 0.5) and (2.5, 2.5), so
    // columns and rows 1 and 2: 11, 12, 21, 22. Mean 66 / 4 = 16.5. SD
    // sqrt(101 / 4) = 5.024937810560445, text 5.0. No spacing, so the area
    // is 2 * 2 = 4 px². OT with no Rescale Type names no unit, F-210.
    let file = value_grid()?;
    expect(
        measured(
            &file,
            MeasureShape::Rectangle,
            &[at(1.0, 1.0), at(3.0, 3.0)],
            "rectangle",
        ),
        "{\"shape\":\"rectangle\",\"frame\":0,\"calibration\":{\"source\":\"uncalibrated\"},\
         \"area\":{\"value\":4,\"unit\":\"px²\",\"text\":\"4.00 px²\"},\
         \"stats\":{\"count\":4,\"padding_excluded\":0,\"unit\":\"unspecified\",\"mean\":16.5,\
         \"sd\":5.024937810560445,\"min\":11,\"max\":22,\
         \"text\":\"mean 16.5 unspecified, SD 5.0 unspecified, min 11 unspecified, max 22 unspecified\"}}",
        "rectangle",
    )?;
    // AC2. Page box (0, 0) to (5, 5) is index centre (2, 2), radii 2.5.
    // 21 samples, mean 462 / 21 = 22, SD sqrt(3434 / 21) =
    // 12.787642844707914, min 1, max 43, area PI * 6.25 = 19.634954084936208.
    expect(
        measured(
            &file,
            MeasureShape::Ellipse,
            &[at(0.0, 0.0), at(5.0, 5.0)],
            "ellipse",
        ),
        "{\"shape\":\"ellipse\",\"frame\":0,\"calibration\":{\"source\":\"uncalibrated\"},\
         \"area\":{\"value\":19.634954084936208,\"unit\":\"px²\",\"text\":\"19.63 px²\"},\
         \"stats\":{\"count\":21,\"padding_excluded\":0,\"unit\":\"unspecified\",\"mean\":22,\
         \"sd\":12.787642844707914,\"min\":1,\"max\":43,\
         \"text\":\"mean 22.0 unspecified, SD 12.8 unspecified, min 1 unspecified, max 43 unspecified\"}}",
        "ellipse",
    )?;
    // AC3. Page (0.5, 0.5), (4.5, 0.5), (0.5, 4.5) is index (0, 0), (4, 0),
    // (0, 4): 15 samples with c + r <= 4, sum 220, mean 14.666666666666666,
    // SD 11.897712198383164, min 0, max 40, shoelace area 8 px².
    expect(
        measured(
            &file,
            MeasureShape::Freeform,
            &[at(0.5, 0.5), at(4.5, 0.5), at(0.5, 4.5)],
            "freeform",
        ),
        "{\"shape\":\"freeform\",\"frame\":0,\"calibration\":{\"source\":\"uncalibrated\"},\
         \"area\":{\"value\":8,\"unit\":\"px²\",\"text\":\"8.00 px²\"},\
         \"stats\":{\"count\":15,\"padding_excluded\":0,\"unit\":\"unspecified\",\
         \"mean\":14.666666666666666,\"sd\":11.897712198383164,\"min\":0,\"max\":40,\
         \"text\":\"mean 14.7 unspecified, SD 11.9 unspecified, min 0 unspecified, max 40 unspecified\"}}",
        "freeform",
    )
}

#[test]
fn measure_refuses_a_region_off_the_image() -> Result<(), String> {
    // AC5 through the route function. A rectangle right of the 5 by 5 grid
    // holds no sample, which is `degenerate`, never a mean of 0. A zero
    // width ellipse is `degenerate` from the F-074 area.
    let file = value_grid()?;
    expect_refusal(
        &file,
        MeasureShape::Rectangle,
        &[at(6.0, 1.0), at(9.0, 3.0)],
        PreviewError::Degenerate,
        "outside",
    )?;
    expect_refusal(
        &file,
        MeasureShape::Ellipse,
        &[at(2.0, 1.0), at(2.0, 4.0)],
        PreviewError::Degenerate,
        "flat ellipse",
    )
}

// ---------------------------------------------------------------------------
// The synthetic study: stored HU + 1024, intercept -1024, Pixel Spacing
// `0.7\0.5`, so a row step is 0.7 mm and a column step 0.5 mm.

fn synthetic() -> Result<Vec<u8>, String> {
    slice(0).ok_or_else(|| "no synthetic slice".to_owned())
}

/// The AC6 reply. Page (69, 48) to (71, 49) is index (68.5, 47.5) to
/// (70.5, 48.5): columns 69 and 70 of row 48. Column 69 is 5 from the body
/// centre (64, 48), inside the body and outside the 36 to 144 bone ring, so
/// 40 HU. Column 70 is 6 away, 36, on the ring, 700 HU. Mean 370, SD 330.
/// The mm corners are (68.5 * 0.5, 47.5 * 0.7) and (70.5 * 0.5, 48.5 * 0.7).
/// The width is 35.25 - 34.25 = 1.0 exactly. The height is the f64
/// difference 33.949999999999996 - 33.25 = 0.6999999999999957, so the area
/// value is 0.6999999999999957 and its two-decimal text 0.70 mm².
const SYNTHETIC_RECTANGLE: &str = "{\"shape\":\"rectangle\",\"frame\":0,\
    \"calibration\":{\"source\":\"pixel-spacing\",\"row_mm\":0.7,\"column_mm\":0.5},\
    \"area\":{\"value\":0.6999999999999957,\"unit\":\"mm²\",\"text\":\"0.70 mm²\"},\
    \"stats\":{\"count\":2,\"padding_excluded\":0,\"unit\":\"HU\",\"mean\":370,\"sd\":330,\
    \"min\":40,\"max\":700,\"text\":\"mean 370.0 HU, SD 330.0 HU, min 40 HU, max 700 HU\"}}";

#[test]
fn measure_angle_uses_physical_spacing() -> Result<(), String> {
    // AC7. Arm (14, 10), vertex (10, 10), arm (14, 14). The mm arms are
    // (4 * 0.5, 0) = (2, 0) and (2, 4 * 0.7) = (2, 2.8). The angle is
    // atan2(|2 * 2.8|, 2 * 2) = atan(1.4) = 54.46232220802562 degrees, text
    // 54.5°. In pixels the arms (4, 0) and (4, 4) would give 45°.
    let file = synthetic()?;
    let json = measured(
        &file,
        MeasureShape::Angle,
        &[at(14.0, 10.0), at(10.0, 10.0), at(14.0, 14.0)],
        "angle",
    )?;
    contains(
        &json,
        "\"angle\":{\"value\":54.46232220802562,\"unit\":\"°\",\"text\":\"54.5°\"}",
        "angle",
    )?;
    contains(&json, "\"source\":\"pixel-spacing\"", "angle")
}

#[test]
fn measure_length_is_row_then_column() -> Result<(), String> {
    // AC8. PS3.3 C.7.6.2, row then column. (10, 10) to (14, 14) is
    // hypot(4 * 0.5, 4 * 0.7) = hypot(2, 2.8) = 3.4409301068170506 mm, text
    // 3.4 mm. Horizontal (10, 10) to (14, 10) is 4 columns, 2.0 mm, and
    // vertical (10, 10) to (10, 14) is 4 rows, 2.8 mm. Swapped spacing
    // would give 2.8 mm for the horizontal segment.
    let file = synthetic()?;
    let cases = [
        (
            [at(10.0, 10.0), at(14.0, 14.0)],
            "\"length\":{\"value\":3.4409301068170506,\"unit\":\"mm\",\"text\":\"3.4 mm\"}",
        ),
        (
            [at(10.0, 10.0), at(14.0, 10.0)],
            "\"length\":{\"value\":2,\"unit\":\"mm\",\"text\":\"2.0 mm\"}",
        ),
        (
            [at(10.0, 10.0), at(10.0, 14.0)],
            "\"length\":{\"value\":2.8,\"unit\":\"mm\",\"text\":\"2.8 mm\"}",
        ),
    ];
    for (points, want) in cases {
        let json = measured(&file, MeasureShape::Length, &points, "length")?;
        contains(&json, want, "length")?;
    }
    Ok(())
}

#[test]
fn measure_cobb_acute() -> Result<(), String> {
    // AC9. Lines (10, 10) to (14, 10) and (20, 20) to (24, 16). The mm
    // vectors are (2, 0) and (2, -2.8). The acute angle is
    // atan2(|2 * -2.8|, |2 * 2|) = atan(1.4), text 54.5°, and not the
    // obtuse 180 - 54.5 = 125.5°.
    let file = synthetic()?;
    let json = measured(
        &file,
        MeasureShape::Cobb,
        &[
            at(10.0, 10.0),
            at(14.0, 10.0),
            at(20.0, 20.0),
            at(24.0, 16.0),
        ],
        "cobb",
    )?;
    contains(&json, "\"unit\":\"°\",\"text\":\"54.5°\"}", "cobb")
}

#[test]
fn measure_bidirectional_lengths() -> Result<(), String> {
    // AC10. Long axis (10, 10) to (18, 10), 8 columns, 8 * 0.5 = 4.0 mm.
    // Short axis (14, 8) to (14, 12), 4 rows, 4 * 0.7 = 2.8 mm. The longer
    // is `length` and the shorter `width`, whichever order they arrive in.
    let file = synthetic()?;
    for (name, points) in [
        (
            "long first",
            [
                at(10.0, 10.0),
                at(18.0, 10.0),
                at(14.0, 8.0),
                at(14.0, 12.0),
            ],
        ),
        (
            "short first",
            [
                at(14.0, 8.0),
                at(14.0, 12.0),
                at(10.0, 10.0),
                at(18.0, 10.0),
            ],
        ),
    ] {
        let json = measured(&file, MeasureShape::Bidirectional, &points, name)?;
        contains(
            &json,
            "\"length\":{\"value\":4,\"unit\":\"mm\",\"text\":\"4.0 mm\"}",
            name,
        )?;
        // After the half-pixel shift the rows are 11.5 and 7.5. In f64,
        // 11.5 * 0.7 = 8.049999999999999 and 7.5 * 0.7 = 5.25, so the width
        // is 2.799999999999999, which still rounds to the text 2.8 mm.
        contains(
            &json,
            "\"width\":{\"value\":2.799999999999999,\"unit\":\"mm\",\"text\":\"2.8 mm\"}",
            name,
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Calibration provenance, PS3.3 C.8.11.4 and C.8.7.5, the F-195 order.

fn spaced(attributes: &[((u16, u16), &str)]) -> Result<Vec<u8>, String> {
    let mut spec = Spec::grid(8, 8, &[0; 64]);
    for &(tag, text) in attributes {
        spec.text(tag, *b"DS", text)?;
    }
    build(spec)
}

const PIXEL_SPACING: (u16, u16) = (0x0028, 0x0030);
const IMAGER_SPACING: (u16, u16) = (0x0018, 0x1164);
const MAGNIFICATION: (u16, u16) = (0x0018, 0x1114);

#[test]
fn measure_calibration_provenance() -> Result<(), String> {
    // AC11. A length of 4 columns, page (1, 1) to (5, 1).
    // Imager Pixel Spacing 0.2\0.2 is at the detector and not corrected for
    // magnification (C.8.11.4). The factor 2 is SID over SOD (C.8.7.5), so
    // the patient spacing is 0.2 / 2 = 0.1 mm and the length 4 * 0.1, text
    // 0.4 mm. Multiplying by the factor would give 1.6 mm.
    let segment = [at(1.0, 1.0), at(5.0, 1.0)];
    let imager = spaced(&[(IMAGER_SPACING, "0.2\\0.2"), (MAGNIFICATION, "2")])?;
    let json = measured(&imager, MeasureShape::Length, &segment, "imager")?;
    contains(
        &json,
        "\"calibration\":{\"source\":\"imager-spacing\",\"row_mm\":0.1,\"column_mm\":0.1}",
        "imager",
    )?;
    contains(&json, "\"text\":\"0.4 mm\"", "imager")?;
    // Without the factor, the raw detector spacing 0.2, 4 * 0.2 = 0.8.
    let detector = spaced(&[(IMAGER_SPACING, "0.2\\0.2")])?;
    let json = measured(&detector, MeasureShape::Length, &segment, "detector")?;
    contains(
        &json,
        "\"calibration\":{\"source\":\"detector\",\"row_mm\":0.2,\"column_mm\":0.2}",
        "detector",
    )?;
    contains(
        &json,
        "\"unit\":\"mm at the detector\",\"text\":\"0.8 mm at the detector\"",
        "detector",
    )?;
    // No spacing at all: 4 px, and an angle says square pixels are assumed.
    // Arm (5, 1), vertex (1, 1), arm (5, 5): arms (4, 0) and (4, 4), 45°.
    let bare = spaced(&[])?;
    let json = measured(&bare, MeasureShape::Length, &segment, "uncalibrated")?;
    contains(
        &json,
        "\"calibration\":{\"source\":\"uncalibrated\"}",
        "uncalibrated",
    )?;
    contains(&json, "\"unit\":\"px\",\"text\":\"4.0 px\"", "uncalibrated")?;
    let json = measured(
        &bare,
        MeasureShape::Angle,
        &[at(5.0, 1.0), at(1.0, 1.0), at(5.0, 5.0)],
        "uncalibrated angle",
    )?;
    contains(
        &json,
        "\"text\":\"45.0°, square pixels assumed\"",
        "uncalibrated angle",
    )?;
    // A present Pixel Spacing of 0\0.5 is not strictly positive. It refuses
    // and does not fall through to the usable imager pair beside it.
    for (name, file) in [
        ("zero row", spaced(&[(PIXEL_SPACING, "0\\0.5")])?),
        (
            "zero row beside an imager pair",
            spaced(&[
                (PIXEL_SPACING, "0\\0.5"),
                (IMAGER_SPACING, "0.2\\0.2"),
                (MAGNIFICATION, "2"),
            ])?,
        ),
        ("one value", spaced(&[(PIXEL_SPACING, "0.5")])?),
        (
            "an unusable factor",
            spaced(&[(IMAGER_SPACING, "0.2\\0.2"), (MAGNIFICATION, "two")])?,
        ),
        (
            "a zero factor",
            spaced(&[(IMAGER_SPACING, "0.2\\0.2"), (MAGNIFICATION, "0")])?,
        ),
    ] {
        expect_refusal(
            &file,
            MeasureShape::Length,
            &segment,
            PreviewError::Calibration,
            name,
        )?;
    }
    // F-211 review pass 1, D2. F-195 takes the first present source. A
    // usable Pixel Spacing 0.5\0.5 is chosen, so a later source that cannot
    // be used is never read and cannot refuse it. Four columns are
    // 4 * 0.5 = 2.0 mm.
    for (name, file) in [
        (
            "an unusable factor after Pixel Spacing",
            spaced(&[
                (PIXEL_SPACING, "0.5\\0.5"),
                (IMAGER_SPACING, "0.2\\0.2"),
                (MAGNIFICATION, "two"),
            ])?,
        ),
        (
            "a one-value imager pair after Pixel Spacing",
            spaced(&[(PIXEL_SPACING, "0.5\\0.5"), (IMAGER_SPACING, "0.2")])?,
        ),
    ] {
        let json = measured(&file, MeasureShape::Length, &segment, name)?;
        contains(
            &json,
            "\"calibration\":{\"source\":\"pixel-spacing\",\"row_mm\":0.5,\"column_mm\":0.5}",
            name,
        )?;
        contains(&json, "\"text\":\"2.0 mm\"", name)?;
    }
    // The operator answer after review pass 1, PS3.3 10.7.1.1. Pixel
    // Spacing equal to Imager Pixel Spacing has not been corrected for
    // magnification, so the imager path decides. Four columns:
    // with factor 2, 4 * 0.2 / 2 = 0.4 mm. With no factor, the detector
    // plane 4 * 0.2 = 0.8 mm at the detector. A pair that differs is Pixel
    // Spacing, 4 * 0.5 = 2.0 mm. A present factor that cannot be used
    // refuses on the imager path, beside an equal pair as without Pixel
    // Spacing, by the lead ruling after review pass 1.
    let equal = "0.2\\0.2";
    for (name, file, calibration, text) in [
        (
            "equal with a factor",
            spaced(&[
                (PIXEL_SPACING, equal),
                (IMAGER_SPACING, equal),
                (MAGNIFICATION, "2"),
            ])?,
            "\"calibration\":{\"source\":\"imager-spacing\",\"row_mm\":0.1,\"column_mm\":0.1}",
            "\"text\":\"0.4 mm\"",
        ),
        (
            "equal without a factor",
            spaced(&[(PIXEL_SPACING, equal), (IMAGER_SPACING, equal)])?,
            "\"calibration\":{\"source\":\"detector\",\"row_mm\":0.2,\"column_mm\":0.2}",
            "\"text\":\"0.8 mm at the detector\"",
        ),
        (
            "unequal",
            spaced(&[
                (PIXEL_SPACING, "0.5\\0.5"),
                (IMAGER_SPACING, equal),
                (MAGNIFICATION, "2"),
            ])?,
            "\"calibration\":{\"source\":\"pixel-spacing\",\"row_mm\":0.5,\"column_mm\":0.5}",
            "\"text\":\"2.0 mm\"",
        ),
    ] {
        let json = measured(&file, MeasureShape::Length, &segment, name)?;
        contains(&json, calibration, name)?;
        contains(&json, text, name)?;
    }
    // F-211 review pass 2. A pair that differs only in the row is a
    // corrected Pixel Spacing, 4 columns * 0.2 = 0.8 mm. A pair inside the
    // one spacing tolerance, 0.1430000001 against 0.143 (about 1e-10 mm), is
    // equal and with no factor is the detector plane, 4 * 0.143 = 0.572 mm,
    // text 0.6 mm at the detector.
    let row_only = spaced(&[(PIXEL_SPACING, "0.18\\0.2"), (IMAGER_SPACING, equal)])?;
    let json = measured(
        &row_only,
        MeasureShape::Length,
        &segment,
        "row only differs",
    )?;
    contains(
        &json,
        "\"calibration\":{\"source\":\"pixel-spacing\",\"row_mm\":0.18,\"column_mm\":0.2}",
        "row only differs",
    )?;
    let near = spaced(&[
        (PIXEL_SPACING, "0.1430000001\\0.1430000001"),
        (IMAGER_SPACING, "0.143\\0.143"),
    ])?;
    let json = measured(
        &near,
        MeasureShape::Length,
        &segment,
        "within the tolerance",
    )?;
    contains(
        &json,
        "\"calibration\":{\"source\":\"detector\",\"row_mm\":0.143,\"column_mm\":0.143}",
        "within the tolerance",
    )?;
    contains(
        &json,
        "\"text\":\"0.6 mm at the detector\"",
        "within the tolerance",
    )?;
    for factor in ["two", "0", "-2"] {
        let file = spaced(&[
            (PIXEL_SPACING, equal),
            (IMAGER_SPACING, equal),
            (MAGNIFICATION, factor),
        ])?;
        expect_refusal(
            &file,
            MeasureShape::Length,
            &segment,
            PreviewError::Calibration,
            &format!("equal with factor {factor}"),
        )?;
    }
    if PreviewError::Calibration.code() != "calibration" {
        return Err("the calibration refusal has the wrong code".to_owned());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Padding, PS3.3 C.7.5.1.1.2.

fn padded(stored: [i16; 4]) -> Result<Vec<u8>, String> {
    let mut spec = Spec::grid(2, 2, &[]);
    spec.pixels = stored
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    spec.signed = true;
    spec.modality = "CT";
    spec.rescale = Some(("1", "-1024"));
    spec.extra.push((
        (0x0028, 0x0120),
        short_le((0x0028, 0x0120), *b"SS", &(-2000_i16).to_le_bytes())?,
    ));
    build(spec)
}

#[test]
fn measure_excludes_padding() -> Result<(), String> {
    // AC12. Pixel Padding Value -2000 in stored space. Stored -2000, 0,
    // -2000, 100: two samples are padding and have no modality value. The
    // other two are 0 - 1024 = -1024 and 100 - 1024 = -924 HU. Mean -974,
    // SD 50. Including padding as -3024 HU would give a mean of -1999.
    let whole = [at(0.0, 0.0), at(2.0, 2.0)];
    let file = padded([-2000, 0, -2000, 100])?;
    let json = measured(&file, MeasureShape::Rectangle, &whole, "padding")?;
    contains(
        &json,
        "\"stats\":{\"count\":2,\"padding_excluded\":2,\"unit\":\"HU\",\"mean\":-974,\"sd\":50,\
         \"min\":-1024,\"max\":-924,\"text\":\"mean -974.0 HU, SD 50.0 HU, min -1024 HU, max -924 HU\"}",
        "padding",
    )?;
    let all = padded([-2000; 4])?;
    expect_refusal(
        &all,
        MeasureShape::Rectangle,
        &whole,
        PreviewError::Degenerate,
        "all padding",
    )
}

// ---------------------------------------------------------------------------
// Refusals of shape and size.

#[test]
fn measure_refuses_a_crossing_outline() -> Result<(), String> {
    // AC13. Page (0.5, 0.5), (4.5, 4.5), (4.5, 0.5), (0.5, 4.5) is index
    // (0, 0), (4, 4), (4, 0), (0, 4): two edges cross at (2, 2). The
    // shoelace area is 0 there while the even-odd rule keeps samples.
    let file = value_grid()?;
    expect_refusal(
        &file,
        MeasureShape::Freeform,
        &[at(0.5, 0.5), at(4.5, 4.5), at(4.5, 0.5), at(0.5, 4.5)],
        PreviewError::SelfIntersecting,
        "figure of eight",
    )?;
    if PreviewError::SelfIntersecting.code() != "self-intersecting"
        || PreviewError::SelfIntersecting.to_string()
            != "This outline crosses itself, so its area is not measured."
    {
        return Err("the crossing refusal has the wrong code or sentence".to_owned());
    }
    // F-211 review pass 1, D1. Index (0, 0), (2, 2), (4, 4), (4, 0),
    // (0, 4): the edge (4, 0) to (0, 4) passes through the vertex (2, 2).
    // Its shoelace area is 0 while the even-odd rule keeps 17 samples, so it
    // is refused like any other crossing.
    expect_refusal(
        &file,
        MeasureShape::Freeform,
        &[
            at(0.5, 0.5),
            at(2.5, 2.5),
            at(4.5, 4.5),
            at(4.5, 0.5),
            at(0.5, 4.5),
        ],
        PreviewError::SelfIntersecting,
        "an edge through a vertex",
    )?;
    // The convex square through the same corners in ring order is measured.
    // Index (0, 0) to (4, 4) holds all 25 samples, area 16 px².
    let json = measured(
        &file,
        MeasureShape::Freeform,
        &[at(0.5, 0.5), at(4.5, 0.5), at(4.5, 4.5), at(0.5, 4.5)],
        "square",
    )?;
    contains(&json, "\"area\":{\"value\":16,", "square")?;
    contains(&json, "\"count\":25,", "square")
}

/// A ring on the border of the page box `(0, 0)` to `(side, side)`, with
/// `vertices` points spread over its four edges. Collinear points on one
/// edge never cross, so the ring is simple.
fn border_ring(side: f64, vertices: usize) -> Vec<ImagePoint> {
    let per_edge = vertices / 4;
    let extra = vertices % 4;
    let mut points = Vec::with_capacity(vertices);
    let corners = [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)];
    for (edge, &(x0, y0)) in corners.iter().enumerate() {
        let (x1, y1) = corners.get((edge + 1) % 4).copied().unwrap_or((0.0, 0.0));
        let steps = per_edge + usize::from(edge < extra);
        let count = f64::from(u32::try_from(steps).unwrap_or(1));
        for step in 0..steps {
            let t = f64::from(u32::try_from(step).unwrap_or(0)) / count;
            points.push(at(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t));
        }
    }
    points
}

#[test]
fn measure_bounds_the_region() -> Result<(), String> {
    // AC14. A 4097 by 4096 image. The page box (0, 0) to (4097, 4096)
    // holds 4097 * 4096 = 16781312 samples, above 4096 * 4096 = 16777216,
    // so it is refused `too-large` before any sample is read.
    let big = build(Spec::grid(4097, 4096, &vec![0; 4097 * 4096]))?;
    expect_refusal(
        &big,
        MeasureShape::Rectangle,
        &[at(0.0, 0.0), at(4097.0, 4096.0)],
        PreviewError::TooLarge,
        "box",
    )?;
    // A 260 by 260 image. A freeform on its border has the clipped box
    // 260 * 260 = 67600 samples. 67600 * 3971 = 268439600 > 2^28 =
    // 268435456 is refused. 67600 * 3970 = 268372000 is within the cap, but
    // walking it is slow in a test, so the accepted side is a small ring.
    let wide = build(Spec::grid(260, 260, &vec![0; 260 * 260]))?;
    expect_refusal(
        &wide,
        MeasureShape::Freeform,
        &border_ring(260.0, 3971),
        PreviewError::TooLarge,
        "freeform",
    )?;
    let json = measured(
        &wide,
        MeasureShape::Freeform,
        &border_ring(10.0, 40),
        "small ring",
    )?;
    // Page (0, 0) to (10, 10) is index -0.5 to 9.5, so 10 * 10 samples.
    contains(&json, "\"count\":100,", "small ring")
}

// ---------------------------------------------------------------------------
// The HTTP route.

fn exchange(addr: SocketAddr, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(request)
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| error.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| error.to_string())?;
    Ok(response)
}

fn split_http(response: &[u8]) -> Result<(String, String), String> {
    let end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| start + 4)
        .ok_or_else(|| "the response has no header".to_owned())?;
    let header = response
        .get(..end)
        .ok_or_else(|| "the response header is short".to_owned())?;
    let body = response
        .get(end..)
        .ok_or_else(|| "the response body is short".to_owned())?;
    Ok((
        String::from_utf8_lossy(header).into_owned(),
        String::from_utf8_lossy(body).into_owned(),
    ))
}

fn header_line(header: &str, name: &str) -> Option<String> {
    header.split("\r\n").find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.eq_ignore_ascii_case(name) {
            Some(value.trim().to_owned())
        } else {
            None
        }
    })
}

fn start(viewer: Viewer) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(viewer);
    std::thread::spawn(move || {
        let _ = serve_on(listener, viewer);
    });
    Ok(addr)
}

/// The direct viewer and the process server, which decodes on its workers.
fn both_viewers() -> Result<[(&'static str, SocketAddr); 2], String> {
    let config = || ViewerConfig {
        cache_bytes: 8 * 1024 * 1024,
    };
    Ok([
        ("direct", start(Viewer::new(config()))?),
        ("server", start(Viewer::server(config()))?),
    ])
}

fn post_file(addr: SocketAddr, file: &[u8]) -> Result<String, String> {
    let mut request = format!(
        "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {len}\r\n\r\n",
        len = file.len()
    )
    .into_bytes();
    request.extend_from_slice(file);
    let (header, _) = split_http(&exchange(addr, &request)?)?;
    header_line(&header, "X-Ocelli-File").ok_or_else(|| format!("no file id in {header}"))
}

fn measure_request(head: &str, body: &[u8]) -> Vec<u8> {
    let mut request = format!(
        "POST /measure HTTP/1.1\r\nHost: 127.0.0.1\r\n{head}Content-Length: {len}\r\n\r\n",
        len = body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    request
}

fn ask(
    addr: SocketAddr,
    id: &str,
    shape: &str,
    extra: &str,
    body: &[u8],
) -> Result<(String, String), String> {
    let head =
        format!("X-Ocelli-File: {id}\r\nX-Ocelli-Frame: 0\r\nX-Ocelli-Shape: {shape}\r\n{extra}");
    split_http(&exchange(addr, &measure_request(&head, body))?)
}

fn refused_with(reply: &(String, String), status: &str, code: &str) -> bool {
    reply.0.starts_with(&format!("HTTP/1.1 {status} "))
        && header_line(&reply.0, "X-Ocelli-Error").as_deref() == Some(code)
}

#[test]
fn measure_route_synthetic_rectangle() -> Result<(), String> {
    // AC6, the reply of `SYNTHETIC_RECTANGLE`, through the route and
    // through the function the route composes.
    let file = synthetic()?;
    expect(
        measured(
            &file,
            MeasureShape::Rectangle,
            &[at(69.0, 48.0), at(71.0, 49.0)],
            "function",
        ),
        SYNTHETIC_RECTANGLE,
        "function",
    )?;
    for (name, addr) in both_viewers()? {
        let id = post_file(addr, &file)?;
        let reply = ask(addr, &id, "rectangle", "", b"69,48 71,49")?;
        if !reply.0.starts_with("HTTP/1.1 200 ") || reply.1 != SYNTHETIC_RECTANGLE {
            return Err(format!("{name}: {} {}", reply.0, reply.1));
        }
    }
    Ok(())
}

#[test]
fn measure_points_body_is_bounded() -> Result<(), String> {
    // AC15. The body is ASCII `x,y` pairs separated by one space. Anything
    // else, or the wrong count for the shape, is 400 `points`.
    let file = synthetic()?;
    let too_many: Vec<String> = (0..4001).map(|index| format!("{index},1")).collect();
    let too_many = too_many.join(" ");
    for (name, addr) in both_viewers()? {
        let id = post_file(addr, &file)?;
        let cases: [(&str, &str, Vec<u8>); 12] = [
            ("empty", "length", Vec::new()),
            ("one pair for a length", "length", b"1,1".to_vec()),
            (
                "three pairs for a length",
                "length",
                b"1,1 2,2 3,3".to_vec(),
            ),
            ("two pairs for an angle", "angle", b"1,1 2,2".to_vec()),
            ("three pairs for a Cobb", "cobb", b"1,1 2,2 3,3".to_vec()),
            ("two pairs for a freeform", "freeform", b"1,1 2,2".to_vec()),
            (
                "4001 pairs for a freeform",
                "freeform",
                too_many.clone().into_bytes(),
            ),
            ("a half pair", "length", b"1,1 2,".to_vec()),
            ("three numbers", "length", b"1,1 2,2,2".to_vec()),
            ("not a number", "length", b"1,1 NaN,2".to_vec()),
            ("infinity", "length", b"1,1 inf,2".to_vec()),
            (
                "a non-ASCII byte",
                "length",
                "1,1 2,\u{00e9}".as_bytes().to_vec(),
            ),
        ];
        for (case, shape, body) in cases {
            let reply = ask(addr, &id, shape, "", &body)?;
            if !refused_with(&reply, "400", "points") {
                return Err(format!("{name} {case}: {}", reply.0));
            }
        }
        // A Part 10 file sent as the points is not a point list.
        let reply = ask(addr, &id, "length", "", &file)?;
        if !refused_with(&reply, "400", "points") {
            return Err(format!("{name}: a file was read as points: {}", reply.0));
        }
        // A declared body over 256 KiB is refused before it is read.
        let head = format!(
            "POST /measure HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nX-Ocelli-Shape: freeform\r\nContent-Length: {len}\r\n\r\n",
            len = 256 * 1024 + 1
        );
        let reply = split_http(&exchange(addr, head.as_bytes())?)?;
        if !refused_with(&reply, "400", "points") {
            return Err(format!("{name}: a long body was read: {}", reply.0));
        }
        // An unknown or missing shape is a header refusal.
        let reply = ask(addr, &id, "circle", "", b"1,1 2,2")?;
        if !refused_with(&reply, "400", "header") {
            return Err(format!("{name}: an unknown shape: {}", reply.0));
        }
        let head = format!("X-Ocelli-File: {id}\r\n");
        let reply = split_http(&exchange(addr, &measure_request(&head, b"1,1 2,2"))?)?;
        if !refused_with(&reply, "400", "header") {
            return Err(format!("{name}: no shape: {}", reply.0));
        }
        // The control: the same id answers a well-formed list.
        let reply = ask(addr, &id, "length", "", b"10,10 14,14")?;
        if !reply.0.starts_with("HTTP/1.1 200 ") || !reply.1.contains("\"text\":\"3.4 mm\"") {
            return Err(format!(
                "{name}: a good list was refused: {} {}",
                reply.0, reply.1
            ));
        }
    }
    Ok(())
}

#[test]
fn measure_ignores_window_headers() -> Result<(), String> {
    // AC16. Statistics are modality values, so a window, a preset or invert
    // on the request does not move them.
    let file = synthetic()?;
    for (name, addr) in both_viewers()? {
        let id = post_file(addr, &file)?;
        let plain = ask(addr, &id, "rectangle", "", b"69,48 71,49")?;
        let windowed = ask(
            addr,
            &id,
            "rectangle",
            "X-Ocelli-Window-Center: 500\r\nX-Ocelli-Window-Width: 10\r\nX-Ocelli-Window-Function: SIGMOID\r\nX-Ocelli-Invert: 1\r\n",
            b"69,48 71,49",
        )?;
        let preset = ask(
            addr,
            &id,
            "rectangle",
            "X-Ocelli-Window-Preset: bone\r\n",
            b"69,48 71,49",
        )?;
        if plain.1 != SYNTHETIC_RECTANGLE || windowed.1 != plain.1 || preset.1 != plain.1 {
            return Err(format!(
                "{name}: a window changed the statistics: {} {} {}",
                plain.1, windowed.1, preset.1
            ));
        }
    }
    Ok(())
}

#[test]
fn measure_route_takes_only_a_held_id() -> Result<(), String> {
    // AC20. The file is posted once and named by id.
    let file = synthetic()?;
    let viewers = both_viewers()?;
    for (name, addr) in viewers {
        let reply = split_http(&exchange(
            addr,
            &measure_request("X-Ocelli-Shape: length\r\n", b"10,10 14,14"),
        )?)?;
        if !refused_with(&reply, "400", "header") {
            return Err(format!("{name}: no id was not refused: {}", reply.0));
        }
        let id = post_file(addr, &file)?;
        let first = ask(addr, &id, "length", "", b"10,10 14,14")?;
        let second = ask(addr, &id, "length", "", b"10,10 14,14")?;
        let sources = [
            header_line(&first.0, "X-Ocelli-Source"),
            header_line(&second.0, "X-Ocelli-Source"),
        ];
        if sources != [Some("decoded".to_owned()), Some("held".to_owned())] {
            return Err(format!("{name}: sources were {sources:?}"));
        }
        // A repeated request with the same points returns the same reply.
        if first.1 != second.1 || !first.1.contains("3.4 mm") {
            return Err(format!(
                "{name}: a repeated measure differed: {} {}",
                first.1, second.1
            ));
        }
    }
    // An id from another viewer.
    let id = post_file(viewers[0].1, &file)?;
    let reply = ask(viewers[1].1, &id, "length", "", b"10,10 14,14")?;
    if !refused_with(&reply, "410", "file-gone") {
        return Err(format!("another viewer's id was served: {}", reply.0));
    }
    // An evicted id. The budget holds one decoded synthetic frame at most,
    // so posting more files pushes the first one out.
    let small = start(Viewer::new(ViewerConfig {
        cache_bytes: file.len() + 1024,
    }))?;
    let first = post_file(small, &file)?;
    for _ in 0..3 {
        let _ = post_file(small, &file)?;
    }
    let reply = ask(small, &first, "length", "", b"10,10 14,14")?;
    if !refused_with(&reply, "410", "file-gone") {
        return Err(format!("an evicted id was served: {}", reply.0));
    }
    Ok(())
}
