//! Explicit VR little endian builders for the built-in frame-open fixture.
//!
//! The bytes are synthetic. They carry no names and no pixel payload.

pub(crate) fn synthetic_part10() -> Result<Vec<u8>, &'static str> {
    let mut data = image_module()?;
    data.extend(window_exact()?);
    data.extend(rescale_one()?);
    data.extend(short_le((0x0028, 0x0120), *b"SS", &0_i16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0121), *b"SS", &0_i16.to_le_bytes())?);
    part10(data)
}

#[cfg(test)]
pub(crate) fn plain_part10() -> Result<Vec<u8>, &'static str> {
    let mut data = image_module()?;
    data.extend(window_exact()?);
    data.extend(rescale_one()?);
    part10(data)
}

fn image_module() -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    Ok(data)
}

fn window_exact() -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le(
        (0x0028, 0x1056),
        *b"CS",
        &text_pad("LINEAR_EXACT"),
    )?);
    Ok(data)
}

fn rescale_one() -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    Ok(data)
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
    body.extend(short_le(
        (0x0002, 0x0002),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    body.extend(short_le((0x0002, 0x0003), *b"UI", &ui(b"2.25.1"))?);
    body.extend(short_le(
        (0x0002, 0x0010),
        *b"UI",
        &ui(b"1.2.840.10008.1.2.1"),
    )?);
    body.extend(short_le((0x0002, 0x0012), *b"UI", &ui(b"2.25.2"))?);
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
