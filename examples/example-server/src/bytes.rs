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

/// Three synthetic objects.
///
/// The design text says the button sends two objects. Two groups and an axial
/// order of two slices need three, so the button sends three. The axial pair
/// is ordered against instance number: z = 3 is first in the body and z = 0
/// is second. The third object is sagittal.
pub(crate) fn synthetic_series_parts() -> Result<Vec<Vec<u8>>, &'static str> {
    Ok(vec![
        series_slice(
            b"2.25.21",
            "1",
            "0\\0\\3",
            "1\\0\\0\\0\\1\\0",
            None,
            Some("30"),
            None,
        )?,
        series_slice(
            b"2.25.20",
            "2",
            "0\\0\\0",
            "1\\0\\0\\0\\1\\0",
            Some("100"),
            None,
            None,
        )?,
        series_slice(
            b"2.25.22",
            "1",
            "0\\0\\0",
            "0\\1\\0\\0\\0\\-1",
            None,
            None,
            Some(1),
        )?,
    ])
}

fn series_slice(
    sop: &[u8],
    instance: &str,
    position: &str,
    orientation: &str,
    frame_time: Option<&str>,
    recommended_rate: Option<&str>,
    playback: Option<u16>,
) -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0008, 0x0018), *b"UI", &ui(sop))?);
    if let Some(rate) = recommended_rate {
        data.extend(short_le((0x0008, 0x2144), *b"IS", &text_pad(rate))?);
    }
    if let Some(time) = frame_time {
        data.extend(short_le((0x0018, 0x1063), *b"DS", &text_pad(time))?);
    }
    if let Some(playback) = playback {
        // PS3.6 (0018,1244) is US. Value 1 is sweeping.
        data.extend(short_le((0x0018, 0x1244), *b"US", &playback.to_le_bytes())?);
    }
    data.extend(short_le((0x0020, 0x000E), *b"UI", &ui(b"2.25.10"))?);
    data.extend(short_le((0x0020, 0x0013), *b"IS", &text_pad(instance))?);
    data.extend(short_le((0x0020, 0x0032), *b"DS", &text_pad(position))?);
    data.extend(short_le((0x0020, 0x0037), *b"DS", &text_pad(orientation))?);
    data.extend(short_le((0x0020, 0x0052), *b"UI", &ui(b"2.25.11"))?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0030), *b"DS", &text_pad("1\\1"))?);
    part10(data)
}

/// One axial slice at z = 0 whose Frame Time is 0.
///
/// Assembly refuses that value as cine and does not fall through.
#[cfg(test)]
pub(crate) fn frame_time_zero_part() -> Result<Vec<u8>, &'static str> {
    series_slice(
        b"2.25.23",
        "3",
        "0\\0\\0",
        "1\\0\\0\\0\\1\\0",
        Some("0"),
        None,
        None,
    )
}

/// One 4 by 4 object.
///
/// Overlay group `6000` has origin `1\1` and the first bit set. The rectangular
/// shutter opens from column 2, so image `(1, 1)` is outside the opening.
/// Shutter Presentation Value is absent.
pub(crate) fn synthetic_overlay_object() -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0008, 0x0018), *b"UI", &ui(b"2.25.40"))?);
    data.extend(short_le(
        (0x0018, 0x1600),
        *b"CS",
        &text_pad("RECTANGULAR"),
    )?);
    data.extend(short_le((0x0018, 0x1602), *b"IS", &text_pad("2"))?);
    data.extend(short_le((0x0018, 0x1604), *b"IS", &text_pad("4"))?);
    data.extend(short_le((0x0018, 0x1606), *b"IS", &text_pad("1"))?);
    data.extend(short_le((0x0018, 0x1608), *b"IS", &text_pad("4"))?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &4_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &4_u16.to_le_bytes())?);
    data.extend(short_le((0x6000, 0x0010), *b"US", &4_u16.to_le_bytes())?);
    data.extend(short_le((0x6000, 0x0011), *b"US", &4_u16.to_le_bytes())?);
    data.extend(short_le((0x6000, 0x0040), *b"CS", &text_pad("G"))?);
    let mut origin = Vec::new();
    origin.extend_from_slice(&1_i16.to_le_bytes());
    origin.extend_from_slice(&1_i16.to_le_bytes());
    data.extend(short_le((0x6000, 0x0050), *b"SS", &origin)?);
    data.extend(short_le((0x6000, 0x0100), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x6000, 0x0102), *b"US", &0_u16.to_le_bytes())?);
    data.extend(long_le((0x6000, 0x3000), *b"OB", &[0x01, 0x00])?);
    part10(data)
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
