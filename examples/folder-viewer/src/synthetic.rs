//! One synthetic CT series the folder viewer can open without a file.
//!
//! Twelve axial slices, explicit VR little endian, no patient name and no
//! patient id. Pixel Spacing is row then column, `0.7\0.5`, so a swap is
//! visible. The file window is centre 40 and width 400, `LINEAR`.

/// Slices in the study.
pub const SLICES: u32 = 12;

const ROWS: i32 = 96;
const COLUMNS: i32 = 128;

/// Part 10 bytes for one zero-based slice.
///
/// `None` when the index is outside the study.
#[must_use]
pub fn slice(index: u32) -> Option<Vec<u8>> {
    if index >= SLICES {
        return None;
    }
    let slice_index = i32::try_from(index).ok()?;
    let mut data = Vec::new();
    data.extend(element_ui((0x0008, 0x0016), b"1.2.840.10008.5.1.4.1.1.2")?);
    data.extend(element_ui((0x0008, 0x0018), sop_uid(index).as_bytes())?);
    data.extend(element_text((0x0008, 0x0060), "CT")?);
    data.extend(element_text((0x0008, 0x1030), "Synthetic phantom")?);
    data.extend(element_text((0x0008, 0x103E), "Synthetic phantom")?);
    data.extend(element_ui((0x0020, 0x000D), b"2.25.43000")?);
    data.extend(element_ui((0x0020, 0x000E), b"2.25.42000")?);
    data.extend(element_text((0x0020, 0x0010), "1")?);
    data.extend(element_text((0x0020, 0x0011), "1")?);
    data.extend(element_text((0x0020, 0x0013), &format!("{}", index + 1))?);
    data.extend(element_text(
        (0x0020, 0x0032),
        &format!("0\\0\\{}", z_text(index)),
    )?);
    data.extend(element_text((0x0020, 0x0037), "1\\0\\0\\0\\1\\0")?);
    data.extend(element_ui((0x0020, 0x0052), b"2.25.44000")?);
    data.extend(element_us((0x0028, 0x0002), 1)?);
    data.extend(element_text((0x0028, 0x0004), "MONOCHROME2")?);
    data.extend(element_us((0x0028, 0x0010), u16::try_from(ROWS).ok()?)?);
    data.extend(element_us((0x0028, 0x0011), u16::try_from(COLUMNS).ok()?)?);
    data.extend(element_text((0x0028, 0x0030), "0.7\\0.5")?);
    data.extend(element_us((0x0028, 0x0100), 16)?);
    data.extend(element_us((0x0028, 0x0101), 16)?);
    data.extend(element_us((0x0028, 0x0102), 15)?);
    data.extend(element_us((0x0028, 0x0103), 1)?);
    data.extend(element_text((0x0028, 0x1050), "40")?);
    data.extend(element_text((0x0028, 0x1051), "400")?);
    data.extend(element_text((0x0028, 0x1052), "-1024")?);
    data.extend(element_text((0x0028, 0x1053), "1")?);
    data.extend(element_text((0x0028, 0x1056), "LINEAR")?);
    data.extend(element_ow((0x7FE0, 0x0010), &pixels(slice_index))?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta(sop_uid(index).as_bytes())?);
    bytes.extend(data);
    Some(bytes)
}

fn sop_uid(index: u32) -> String {
    format!("2.25.41{index:03}")
}

fn z_text(index: u32) -> String {
    if index.is_multiple_of(2) {
        format!("{}", index / 2 * 5)
    } else {
        format!("{}.5", index / 2 * 5 + 2)
    }
}

/// Hounsfield value. Air is -1000, the body ellipse is 40, the two lung
/// ellipses are -800, and the ring is bone at 700.
fn sample_hu(column: i32, row: i32, slice_index: i32) -> i16 {
    let radius_x = 40 + slice_index;
    let radius_y = 28 + slice_index;
    if !inside(column, row, 64, 48, radius_x, radius_y) {
        return -1000;
    }
    let dx = column - 64;
    let dy = row - 48;
    let distance = dx * dx + dy * dy;
    if (36..=144).contains(&distance) {
        return 700;
    }
    if inside(column, row, 46, 48, 8, 12) || inside(column, row, 82, 48, 8, 12) {
        return -800;
    }
    40
}

fn inside(column: i32, row: i32, cx: i32, cy: i32, rx: i32, ry: i32) -> bool {
    if rx <= 0 || ry <= 0 {
        return false;
    }
    let dx = column - cx;
    let dy = row - cy;
    dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry
}

fn pixels(slice_index: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let count = usize::try_from(ROWS * COLUMNS).unwrap_or(0);
    bytes.reserve(count.saturating_mul(2));
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let stored = sample_hu(column, row, slice_index) + 1024;
            bytes.extend_from_slice(&stored.to_le_bytes());
        }
    }
    bytes
}

fn file_meta(sop: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    body.extend(element_ob((0x0002, 0x0001), &[0, 1])?);
    body.extend(element_ui((0x0002, 0x0002), b"1.2.840.10008.5.1.4.1.1.2")?);
    body.extend(element_ui((0x0002, 0x0003), sop)?);
    body.extend(element_ui((0x0002, 0x0010), b"1.2.840.10008.1.2.1")?);
    body.extend(element_ui((0x0002, 0x0012), b"2.25.9")?);
    let length = u32::try_from(body.len()).ok()?;
    let mut meta = element_ul((0x0002, 0x0000), length)?;
    meta.extend(body);
    Some(meta)
}

fn element_us(tag: (u16, u16), value: u16) -> Option<Vec<u8>> {
    short(tag, *b"US", &value.to_le_bytes())
}

fn element_ul(tag: (u16, u16), value: u32) -> Option<Vec<u8>> {
    short(tag, *b"UL", &value.to_le_bytes())
}

fn element_ob(tag: (u16, u16), value: &[u8]) -> Option<Vec<u8>> {
    long(tag, *b"OB", value)
}

fn element_ui(tag: (u16, u16), text: &[u8]) -> Option<Vec<u8>> {
    let mut value = text.to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    short(tag, *b"UI", &value)
}

fn element_text(tag: (u16, u16), text: &str) -> Option<Vec<u8>> {
    let mut value = text.as_bytes().to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(b' ');
    }
    let vr = match tag {
        (0x0008, 0x1030) | (0x0008, 0x103E) => *b"LO",
        (0x0020, 0x0032)
        | (0x0020, 0x0037)
        | (0x0028, 0x0030)
        | (0x0028, 0x1050)
        | (0x0028, 0x1051)
        | (0x0028, 0x1052)
        | (0x0028, 0x1053) => *b"DS",
        (0x0028, 0x1056) | (0x0008, 0x0060) | (0x0028, 0x0004) => *b"CS",
        (0x0020, 0x0010) => *b"SH",
        (0x0020, 0x0011) | (0x0020, 0x0013) => *b"IS",
        _ => return None,
    };
    short(tag, vr, &value)
}

fn element_ow(tag: (u16, u16), value: &[u8]) -> Option<Vec<u8>> {
    long(tag, *b"OW", value)
}

fn short(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let length = u16::try_from(value.len()).ok()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Some(bytes)
}

fn long(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let length = u32::try_from(value.len()).ok()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Some(bytes)
}
