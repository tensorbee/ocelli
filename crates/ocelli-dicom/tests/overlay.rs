//! Overlay and shutter fixtures.
//!
//! Overlay bits cite DICOM PS3.3 C.9.2 and PS3.5 bit packing, low bit first.
//! A 4 by 4 plane with only overlay pixel (1, 1) set is the two bytes `01 00`.
//! Shutters cite PS3.3 C.7.6.11 and C.7.6.15. No patient data.

use ocelli_dicom::{
    OverlayError, OverlayReason, OverlaySet, OverlayType, ShutterShape, parse_part10, read_overlays,
};

#[test]
fn overlay_origin_one_sets_the_image_pixel() {
    let set = opened(OverlayCase::hit());
    let plane = only_plane(&set);
    assert_eq!(plane.overlay_type(), OverlayType::Graphics);
    assert!(plane.bit(1, 1));
    assert_eq!(plane.outside_count(), 0);
}

#[test]
fn overlay_origin_zero_puts_the_bit_outside() {
    let mut case = OverlayCase::hit();
    case.origin = (0, 0);
    let set = opened(case);
    let plane = only_plane(&set);
    assert!(!plane.bit(1, 1));
    assert_eq!(plane.outside_count(), 1);
}

#[test]
fn overlay_rectangular_edge_is_inclusive() {
    let shutter = shutter_of(OverlayCase::hit());
    assert!(shutter.hides(1, 1));
    assert!(!shutter.hides(1, 2));
}

#[test]
fn overlay_bitmap_shutter_is_not_also_graphic() {
    let mut case = OverlayCase::hit();
    case.shutter = ShutterCase::Bitmap;
    let set = opened(case);
    assert!(set.planes().is_empty());
    let shutter = shutter_of_set(&set);
    assert!(matches!(
        shutter.shapes().first(),
        Some(ShutterShape::Bitmap { group: 0x6000 })
    ));
    assert!(shutter.hides(1, 1));
    assert!(!shutter.hides(1, 2));
}

#[test]
fn overlay_absent_presentation_stays_absent() {
    assert_eq!(opened(OverlayCase::hit()).presentation_value(), None);
}

#[test]
fn overlay_presentation_zero_is_present() {
    let mut case = OverlayCase::hit();
    case.presentation = Some(0);
    assert_eq!(opened(case).presentation_value(), Some(0));
}

#[test]
fn overlay_bits_allocated_eight_is_an_error() {
    let mut case = OverlayCase::hit();
    case.allocated = 8;
    refused(case, OverlayReason::BitsAllocated);
}

#[test]
fn overlay_crossing_polygon_is_an_error() {
    let mut case = OverlayCase::plain();
    case.shutter = ShutterCase::Polygon("1\\1\\4\\1\\1\\4\\4\\4");
    refused(case, OverlayReason::Polygon);
}

#[test]
fn overlay_simple_polygon_keeps_the_interior() {
    let mut case = OverlayCase::plain();
    case.shutter = ShutterCase::Polygon("1\\1\\1\\4\\4\\4\\4\\1");
    let shutter = shutter_of(case);
    assert!(!shutter.hides(2, 2));
    assert!(!shutter.hides(1, 2));
    assert!(shutter.hides(5, 2));
}

#[test]
fn overlay_circle_on_the_radius_is_inside() {
    let mut case = OverlayCase::plain();
    case.shutter = ShutterCase::Circle("2\\2", "1");
    let shutter = shutter_of(case);
    assert!(!shutter.hides(2, 2));
    assert!(!shutter.hides(2, 3));
    assert!(shutter.hides(2, 4));
}

#[test]
fn overlay_origin_column_is_not_the_row() {
    let mut case = OverlayCase::hit();
    case.image_rows = 2;
    case.image_columns = 3;
    case.rows = 1;
    case.columns = 1;
    case.origin = (1, 2);
    case.shutter = ShutterCase::None;
    let set = opened(case);
    let plane = only_plane(&set);
    assert!(plane.bit(1, 2));
    assert!(!plane.bit(2, 1));
    assert!(!plane.bit(1, 1));
    assert_eq!(plane.outside_count(), 0);
}

#[test]
fn overlay_bit_past_the_image_stays_outside() {
    let mut case = OverlayCase::hit();
    case.image_rows = 1;
    case.image_columns = 1;
    case.rows = 1;
    case.columns = 2;
    case.data = Data::Packed(&[0x03, 0x00]);
    case.shutter = ShutterCase::None;
    let set = opened(case);
    let plane = only_plane(&set);
    assert!(plane.bit(1, 1));
    assert!(!plane.bit(1, 2));
    assert_eq!(plane.outside_count(), 1);
}

#[test]
fn overlay_one_bit_ow_is_packed_low_bit_first() {
    let mut case = OverlayCase::hit();
    case.image_rows = 2;
    case.image_columns = 2;
    case.rows = 2;
    case.columns = 2;
    case.allocated = 1;
    case.data = Data::Word(0x0002);
    case.shutter = ShutterCase::None;
    let set = opened(case);
    let plane = only_plane(&set);
    assert!(plane.bit(1, 2));
    assert!(!plane.bit(1, 1));
    assert!(!plane.bit(2, 1));
}

#[test]
fn overlay_bitmap_ow_follows_the_low_bit() {
    let mut case = OverlayCase::hit();
    case.image_rows = 1;
    case.image_columns = 1;
    case.rows = 1;
    case.columns = 1;
    case.allocated = 1;
    case.data = Data::Word(1);
    case.shutter = ShutterCase::Bitmap;
    let set = opened(case);
    assert!(set.planes().is_empty());
    let shutter = shutter_of_set(&set);
    assert!(shutter.hides(1, 1));
    assert!(shutter.hides(1, 2));
}

#[test]
fn overlay_circle_centre_is_row_then_column() {
    let mut case = OverlayCase::plain();
    case.shutter = ShutterCase::Circle("1\\3", "1");
    let shutter = shutter_of(case);
    assert!(!shutter.hides(1, 3));
    assert!(!shutter.hides(1, 4));
    assert!(shutter.hides(1, 5));
    assert!(shutter.hides(3, 3));
}

#[test]
fn overlay_asymmetric_polygon_keeps_one_side() {
    let mut case = OverlayCase::plain();
    case.shutter = ShutterCase::Polygon("1\\1\\1\\4\\3\\1");
    let shutter = shutter_of(case);
    assert!(!shutter.hides(2, 2));
    assert!(shutter.hides(2, 3));
    assert!(!shutter.hides(1, 2));
    assert!(!shutter.hides(1, 4));
    assert!(shutter.hides(4, 1));
}

#[test]
fn overlay_zero_length_edge_is_an_error() {
    let mut case = OverlayCase::plain();
    case.shutter = ShutterCase::Polygon("1\\1\\1\\1\\3\\3");
    refused(case, OverlayReason::Polygon);
}

#[test]
fn overlay_folded_adjacent_edge_is_an_error() {
    let mut case = OverlayCase::plain();
    case.shutter = ShutterCase::Polygon("1\\1\\1\\4\\1\\2");
    refused(case, OverlayReason::Polygon);
}

#[test]
fn overlay_second_frame_uses_its_own_bits() {
    let mut case = OverlayCase::hit();
    case.image_rows = 1;
    case.image_columns = 2;
    case.rows = 1;
    case.columns = 2;
    case.data = Data::Packed(&[0x04, 0x00]);
    case.shutter = ShutterCase::None;
    case.overlay_frames = Some(2);
    case.number_of_frames = Some(2);
    let parsed = parsed(case);
    let first = read_overlays(&parsed, 0);
    let second = read_overlays(&parsed, 1);
    assert!(first.is_ok());
    assert!(second.is_ok());
    let Ok(first) = first else {
        unreachable!("read");
    };
    let Ok(second) = second else {
        unreachable!("read");
    };
    assert!(!only_plane(&first).bit(1, 1));
    assert!(!only_plane(&first).bit(1, 2));
    assert!(only_plane(&second).bit(1, 1));
    assert!(!only_plane(&second).bit(1, 2));
}

#[test]
fn overlay_frame_before_the_overlay_origin_is_empty() {
    let mut case = OverlayCase::hit();
    case.image_rows = 1;
    case.image_columns = 1;
    case.rows = 1;
    case.columns = 1;
    case.shutter = ShutterCase::None;
    case.frame_origin = Some(2);
    case.number_of_frames = Some(2);
    let parsed = parsed(case);
    let first = read_overlays(&parsed, 0);
    let second = read_overlays(&parsed, 1);
    assert!(first.is_ok());
    assert!(second.is_ok());
    let Ok(first) = first else {
        unreachable!("read");
    };
    let Ok(second) = second else {
        unreachable!("read");
    };
    assert!(first.planes().is_empty());
    assert!(only_plane(&second).bit(1, 1));
}

#[test]
fn overlay_sparse_group_continues() {
    let mut case = OverlayCase::hit();
    case.group = 0x6002;
    case.shutter = ShutterCase::None;
    assert_eq!(only_plane(&opened(case)).group(), 0x6002);
}

#[test]
fn overlay_frame_past_the_end_is_an_error() {
    expect_reason(
        read_overlays(&parsed(OverlayCase::hit()), 1),
        OverlayReason::FrameOutOfRange,
    );
}

#[test]
fn overlay_per_frame_group_is_unsupported() {
    let bytes = per_frame_overlay();
    assert!(bytes.is_ok());
    let Ok(bytes) = bytes else {
        unreachable!("fixture");
    };
    let parsed = parse_part10(&bytes);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        unreachable!("parse");
    };
    expect_reason(
        read_overlays(&parsed, 0),
        OverlayReason::FrameOverlayUnsupported,
    );
}

#[test]
fn overlay_sixteen_bit_position_selects_one_bit() {
    let mut low = OverlayCase::word();
    low.bit_position = 0;
    assert!(only_plane(&opened(low)).bit(1, 1));

    let mut high = OverlayCase::word();
    high.bit_position = 1;
    assert!(!only_plane(&opened(high)).bit(1, 1));
}

#[test]
fn overlay_second_read_matches() {
    let parsed = parsed(OverlayCase::hit());
    let first = read_overlays(&parsed, 0);
    let second = read_overlays(&parsed, 0);
    assert!(first.is_ok());
    assert!(second.is_ok());
    let Ok(first) = first else {
        unreachable!("read");
    };
    let Ok(second) = second else {
        unreachable!("read");
    };
    assert_eq!(first, second);
}

fn only_plane(set: &OverlaySet) -> &ocelli_dicom::OverlayPlane {
    assert_eq!(set.planes().len(), 1);
    let Some(plane) = set.planes().first() else {
        unreachable!("the length check passed");
    };
    plane
}

fn expect_reason(result: Result<OverlaySet, OverlayError>, reason: OverlayReason) {
    match result {
        Err(error) => assert_eq!(error.reason(), reason),
        Ok(_) => unreachable!("the fixture is refused"),
    }
}

fn parsed(case: OverlayCase) -> ocelli_dicom::ParsedDicom {
    let bytes = object(case);
    assert!(bytes.is_ok());
    let Ok(bytes) = bytes else {
        unreachable!("fixture");
    };
    let parsed = parse_part10(&bytes);
    assert!(parsed.is_ok());
    let Ok(parsed) = parsed else {
        unreachable!("parse");
    };
    parsed
}

fn opened(case: OverlayCase) -> OverlaySet {
    let set = read_overlays(&parsed(case), 0);
    assert!(set.is_ok());
    let Ok(set) = set else {
        unreachable!("read");
    };
    set
}

fn shutter_of(case: OverlayCase) -> ocelli_dicom::Shutter {
    let set = opened(case);
    shutter_of_set(&set).clone()
}

fn shutter_of_set(set: &OverlaySet) -> &ocelli_dicom::Shutter {
    assert!(set.shutter().is_some());
    let Some(shutter) = set.shutter() else {
        unreachable!("shutter");
    };
    shutter
}

fn refused(case: OverlayCase, reason: OverlayReason) {
    expect_reason(read_overlays(&parsed(case), 0), reason);
}

struct OverlayCase {
    group: u16,
    image_rows: u16,
    image_columns: u16,
    rows: u16,
    columns: u16,
    origin: (i16, i16),
    allocated: u16,
    bit_position: u16,
    data: Data,
    shutter: ShutterCase,
    presentation: Option<u16>,
    overlay_frames: Option<u32>,
    frame_origin: Option<u16>,
    number_of_frames: Option<u32>,
}

enum Data {
    Packed(&'static [u8]),
    Word(u16),
}

enum ShutterCase {
    None,
    Rectangle,
    Bitmap,
    Circle(&'static str, &'static str),
    Polygon(&'static str),
}

impl OverlayCase {
    fn hit() -> Self {
        Self {
            group: 0x6000,
            image_rows: 4,
            image_columns: 4,
            rows: 4,
            columns: 4,
            origin: (1, 1),
            allocated: 1,
            bit_position: 0,
            data: Data::Packed(&[0x01, 0x00]),
            shutter: ShutterCase::Rectangle,
            presentation: None,
            overlay_frames: None,
            frame_origin: None,
            number_of_frames: None,
        }
    }

    fn plain() -> Self {
        Self {
            shutter: ShutterCase::None,
            data: Data::Packed(&[0x00, 0x00]),
            ..Self::hit()
        }
    }

    fn word() -> Self {
        Self {
            image_rows: 1,
            image_columns: 1,
            rows: 1,
            columns: 1,
            allocated: 16,
            data: Data::Word(1),
            shutter: ShutterCase::None,
            ..Self::hit()
        }
    }
}

fn object(case: OverlayCase) -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    data.extend(ui_element((0x0008, 0x0016), "1.2.840.10008.5.1.4.1.1.2")?);
    data.extend(ui_element((0x0008, 0x0018), "2.25.40")?);
    if let Some(frames) = case.number_of_frames {
        data.extend(text_element((0x0028, 0x0008), *b"IS", &frames.to_string())?);
    }
    data.extend(short_le(
        (0x0028, 0x0010),
        *b"US",
        &case.image_rows.to_le_bytes(),
    )?);
    data.extend(short_le(
        (0x0028, 0x0011),
        *b"US",
        &case.image_columns.to_le_bytes(),
    )?);
    let group = case.group;
    data.extend(short_le((group, 0x0010), *b"US", &case.rows.to_le_bytes())?);
    data.extend(short_le(
        (group, 0x0011),
        *b"US",
        &case.columns.to_le_bytes(),
    )?);
    if let Some(frames) = case.overlay_frames {
        data.extend(text_element((group, 0x0015), *b"IS", &frames.to_string())?);
    }
    data.extend(text_element((group, 0x0040), *b"CS", "G")?);
    let mut origin = Vec::new();
    origin.extend_from_slice(&case.origin.0.to_le_bytes());
    origin.extend_from_slice(&case.origin.1.to_le_bytes());
    data.extend(short_le((group, 0x0050), *b"SS", &origin)?);
    if let Some(origin_frame) = case.frame_origin {
        data.extend(short_le(
            (group, 0x0051),
            *b"US",
            &origin_frame.to_le_bytes(),
        )?);
    }
    data.extend(short_le(
        (group, 0x0100),
        *b"US",
        &case.allocated.to_le_bytes(),
    )?);
    data.extend(short_le(
        (group, 0x0102),
        *b"US",
        &case.bit_position.to_le_bytes(),
    )?);
    match case.data {
        Data::Packed(bytes) => data.extend(long_le((group, 0x3000), *b"OB", bytes)?),
        Data::Word(word) => data.extend(long_le((group, 0x3000), *b"OW", &word.to_le_bytes())?),
    }
    match case.shutter {
        ShutterCase::None => {}
        ShutterCase::Rectangle => {
            data.extend(text_element((0x0018, 0x1600), *b"CS", "RECTANGULAR")?);
            data.extend(text_element((0x0018, 0x1602), *b"IS", "2")?);
            data.extend(text_element((0x0018, 0x1604), *b"IS", "4")?);
            data.extend(text_element((0x0018, 0x1606), *b"IS", "1")?);
            data.extend(text_element((0x0018, 0x1608), *b"IS", "4")?);
        }
        ShutterCase::Bitmap => {
            data.extend(text_element((0x0018, 0x1600), *b"CS", "BITMAP")?);
            data.extend(short_le((0x0018, 0x1623), *b"US", &group.to_le_bytes())?);
        }
        ShutterCase::Circle(center, radius) => {
            data.extend(text_element((0x0018, 0x1600), *b"CS", "CIRCULAR")?);
            data.extend(text_element((0x0018, 0x1610), *b"IS", center)?);
            data.extend(text_element((0x0018, 0x1612), *b"IS", radius)?);
        }
        ShutterCase::Polygon(vertices) => {
            data.extend(text_element((0x0018, 0x1600), *b"CS", "POLYGONAL")?);
            data.extend(text_element((0x0018, 0x1620), *b"IS", vertices)?);
        }
    }
    if let Some(value) = case.presentation {
        data.extend(short_le((0x0018, 0x1622), *b"US", &value.to_le_bytes())?);
    }
    part10(data)
}

fn per_frame_overlay() -> Result<Vec<u8>, &'static str> {
    let mut item = Vec::new();
    item.extend(long_le((0x6000, 0x3000), *b"OB", &[0x01, 0x00])?);
    let mut data = Vec::new();
    data.extend(ui_element((0x0008, 0x0016), "1.2.840.10008.5.1.4.1.1.2")?);
    data.extend(ui_element((0x0008, 0x0018), "2.25.41")?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &4_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &4_u16.to_le_bytes())?);
    data.extend(sequence((0x5200, 0x9230), &[item])?);
    part10(data)
}

fn ui_element(tag: (u16, u16), text: &str) -> Result<Vec<u8>, &'static str> {
    short_le(tag, *b"UI", &ui(text.as_bytes()))
}

fn text_element(tag: (u16, u16), vr: [u8; 2], text: &str) -> Result<Vec<u8>, &'static str> {
    short_le(tag, vr, &text_pad(text))
}

fn sequence(tag: (u16, u16), items: &[Vec<u8>]) -> Result<Vec<u8>, &'static str> {
    let mut payload = Vec::new();
    for item in items {
        payload.extend(item_bytes(item)?);
    }
    long_le(tag, *b"SQ", &payload)
}

fn item_bytes(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let length = u32::try_from(data.len()).map_err(|_| "item length")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(data);
    Ok(bytes)
}

fn part10(data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta()?);
    bytes.extend(data);
    Ok(bytes)
}

fn file_meta() -> Result<Vec<u8>, &'static str> {
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(ui_element((0x0002, 0x0002), "1.2.840.10008.5.1.4.1.1.2")?);
    body.extend(ui_element((0x0002, 0x0003), "2.25.1")?);
    body.extend(ui_element((0x0002, 0x0010), "1.2.840.10008.1.2.1")?);
    body.extend(ui_element((0x0002, 0x0012), "2.25.2")?);
    let length = u32::try_from(body.len()).map_err(|_| "meta length")?;
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

fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, &'static str> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element");
    }
    let length = u16::try_from(value.len()).map_err(|_| "element length")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, &'static str> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element");
    }
    let length = u32::try_from(value.len()).map_err(|_| "element length")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}
