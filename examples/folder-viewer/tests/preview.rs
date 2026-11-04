//! Synthetic Part 10 frames. The expected grey bytes are computed from
//! PS3.3 C.11.2.1.3.2, not from a previous run of this preview.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use ocelli_folder_viewer::{
    FrameRequest, Preview, PreviewError, handle_connection, headers_json, inspect_json,
    preview_first_frame, preview_frame,
};
use ocelli_pixel::VoiFunction;

#[test]
fn an_enhanced_frame_uses_its_functional_group_window() -> Result<(), String> {
    // No top-level rescale or window. Shared functional groups carry slope 1
    // and intercept 0. Frame 0 has LINEAR_EXACT center 100 width 200.
    // Frame 1 has LINEAR_EXACT center 50 width 1. Both frames store 50.
    // Frame 0: ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    // Frame 1: ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    // Ignoring the groups would full-range both frames to grey 128.
    let file = enhanced_frames()?;
    let first = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the first enhanced frame was refused".to_owned())?;
    if first.full_range {
        return Err("the enhanced frame was labelled full range".to_owned());
    }
    if only_byte(&first)? != 64 {
        return Err("the first enhanced frame was not grey 64".to_owned());
    }
    let second = preview_frame(
        &file,
        FrameRequest {
            frame: 1,
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "the second enhanced frame was refused".to_owned())?;
    if only_byte(&second)? != 128 {
        return Err("the second enhanced frame was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn a_linear_exact_window_maps_stored_50_to_grey_64() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0, LINEAR_EXACT center 100, width 200.
    // ymin 0, ymax 255. x = 50 lies inside [0, 200].
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75.
    // `display_index` adds 0.5 in f32 and lands on table address 64.
    let preview = preview_first_frame(&mono_frame(true)?)
        .map_err(|_| "the synthetic windowed frame was refused".to_owned())?;
    assert_eq!(preview.width, 1);
    assert_eq!(preview.height, 1);
    assert_eq!(preview.frames, 1);
    assert!(!preview.full_range);
    assert_eq!(only_byte(&preview)?, 64);
    Ok(())
}

#[test]
fn a_zero_length_window_is_full_range() -> Result<(), String> {
    // Stored 50. Slope 1 and intercept 0 are present. Window Center and
    // Window Width are present with length 0, so they carry no number.
    // That is the same as an absent window. One modality value gives
    // LINEAR_EXACT center 50 and width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let preview = preview_first_frame(&mono_zero_window()?)
        .map_err(|_| "a zero-length window was refused".to_owned())?;
    if !preview.full_range {
        return Err("a zero-length window was not full range".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("a zero-length window was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn a_zero_length_rescale_keeps_the_explicit_window() -> Result<(), String> {
    // Stored 50. Rescale Slope and Rescale Intercept are present with
    // length 0, so they carry no number. Absent rescale is slope 1 and
    // intercept 0. The window is LINEAR_EXACT center 100 and width 200.
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    let preview = preview_first_frame(&mono_zero_rescale()?)
        .map_err(|_| "a zero-length rescale was refused".to_owned())?;
    if preview.full_range {
        return Err("a zero-length rescale was labelled full range".to_owned());
    }
    if only_byte(&preview)? != 64 {
        return Err("a zero-length rescale was not grey 64".to_owned());
    }
    Ok(())
}

#[test]
fn a_zero_length_voi_function_uses_linear() -> Result<(), String> {
    // Window Center 100 and Window Width 200 are present. VOI LUT Function
    // is present with length 0, so it carries no token. An absent function
    // is LINEAR. The frame is not full range.
    let preview = preview_first_frame(&mono_zero_function()?)
        .map_err(|_| "a zero-length function was refused".to_owned())?;
    if preview.full_range {
        return Err("a zero-length function was labelled full range".to_owned());
    }
    if preview.window_function != "LINEAR" {
        return Err("a zero-length function was not LINEAR".to_owned());
    }
    Ok(())
}

#[test]
fn a_missing_window_is_labelled_full_range() -> Result<(), String> {
    // One stored sample 50. Absent rescale is slope 1 and intercept 0.
    // Absent center and width use LINEAR_EXACT over that single modality
    // value, so the width is 1 and the center is 50.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let preview = preview_first_frame(&mono_frame(false)?)
        .map_err(|_| "the synthetic full-range frame was refused".to_owned())?;
    assert!(preview.full_range);
    assert_eq!(only_byte(&preview)?, 128);
    Ok(())
}

#[test]
fn bytes_without_dicm_are_refused() -> Result<(), String> {
    let bytes = vec![0_u8; 200];
    match preview_first_frame(&bytes) {
        Err(PreviewError::NotPart10) => Ok(()),
        Ok(_) => Err("bytes without DICM were accepted".to_owned()),
        Err(error) => Err(format!("bytes without DICM were refused as {error}")),
    }
}

#[test]
fn the_page_posts_a_frame_and_returns_its_grey_byte() -> Result<(), String> {
    let file = mono_frame(true)?;
    let request = http_post(&file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the preview response was not 200".to_owned());
    }
    if !header.contains("X-Ocelli-Width: 1") || !header.contains("X-Ocelli-Height: 1") {
        return Err("the preview response omitted the frame size".to_owned());
    }
    if !header.contains("X-Ocelli-Full-Range: 0") {
        return Err("a windowed frame was labelled full range".to_owned());
    }
    if body.len() != 1 {
        return Err("the preview response was not one byte".to_owned());
    }
    let grey = body
        .first()
        .copied()
        .ok_or_else(|| "the preview response was empty".to_owned())?;
    if grey != 64 {
        return Err("the preview response was not grey 64".to_owned());
    }
    Ok(())
}

#[test]
fn the_page_offers_a_folder_picker() -> Result<(), String> {
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the page response was not 200".to_owned());
    }
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    if !page.contains("webkitdirectory") {
        return Err("the page has no folder picker".to_owned());
    }
    if !page.contains("id=\"series\"") || !page.contains(">Headers<") {
        return Err("the page is not the series viewer".to_owned());
    }
    if !page.contains(">Contour<") || !page.contains(">Text<") {
        return Err("the page has no contour or text tool".to_owned());
    }
    if !page.contains("Series shortened") {
        return Err("a shortened series is not labelled".to_owned());
    }
    Ok(())
}

#[test]
fn a_post_without_a_length_is_refused() -> Result<(), String> {
    let response = one_exchange(b"POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, _) = split_http(&response)?;
    if !header.contains("400") || !header.contains("X-Ocelli-Error: length") {
        return Err("a post without a length was not refused".to_owned());
    }
    Ok(())
}

fn only_byte(preview: &Preview) -> Result<u8, String> {
    if preview.grey.len() != 1 {
        return Err("the frame is not one pixel".to_owned());
    }
    preview
        .grey
        .first()
        .copied()
        .ok_or_else(|| "the frame is empty".to_owned())
}

fn one_exchange(request: &[u8]) -> Result<Vec<u8>, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let server = std::thread::spawn(move || -> Result<(), String> {
        let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
        handle_connection(stream).map_err(|error| error.to_string())
    });
    let response = exchange(addr, request)?;
    match server.join() {
        Ok(Ok(())) => Ok(response),
        Ok(Err(message)) => Err(message),
        Err(_) => Err("the preview server stopped".to_owned()),
    }
}

fn exchange(addr: SocketAddr, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
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

fn split_http(response: &[u8]) -> Result<(&str, &[u8]), String> {
    let end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| start + 4)
        .ok_or_else(|| "the preview response has no header".to_owned())?;
    let header = response
        .get(..end)
        .ok_or_else(|| "the preview response header is short".to_owned())?;
    let body = response
        .get(end..)
        .ok_or_else(|| "the preview response body is short".to_owned())?;
    let text = std::str::from_utf8(header)
        .map_err(|_| "the preview response header is not text".to_owned())?;
    Ok((text, body))
}

fn http_post(body: &[u8]) -> Vec<u8> {
    let mut request = format!(
        "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {len}\r\n\r\n",
        len = body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    request
}

fn enhanced_frames() -> Result<Vec<u8>, String> {
    let rescale = item(&concat_bytes(&[
        &short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?,
        &short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?,
    ]))?;
    let shared = item(&long_le((0x0028, 0x9145), *b"SQ", &rescale)?)?;
    let frame_window = |center: &str, width: &str| -> Result<Vec<u8>, String> {
        let body = concat_bytes(&[
            &short_le((0x0028, 0x1050), *b"DS", &text_pad(center))?,
            &short_le((0x0028, 0x1051), *b"DS", &text_pad(width))?,
            &short_le((0x0028, 0x1056), *b"CS", &text_pad("LINEAR_EXACT"))?,
        ]);
        item(&long_le((0x0028, 0x9132), *b"SQ", &item(&body)?)?)
    };
    let per_frame = concat_bytes(&[&frame_window("100", "200")?, &frame_window("50", "1")?]);
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&50_u16.to_le_bytes());
    pixels.extend_from_slice(&50_u16.to_le_bytes());
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
    data.extend(short_le((0x0028, 0x0008), *b"IS", &text_pad("2"))?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(long_le((0x5200, 0x9229), *b"SQ", &shared)?);
    data.extend(long_le((0x5200, 0x9230), *b"SQ", &per_frame)?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &pixels)?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn item(content: &[u8]) -> Result<Vec<u8>, String> {
    let length = u32::try_from(content.len()).map_err(|_| "item length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(content);
    Ok(bytes)
}

fn concat_bytes(parts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for part in parts {
        bytes.extend_from_slice(part);
    }
    bytes
}

fn mono_frame(window: bool) -> Result<Vec<u8>, String> {
    mono_with(window, &[])
}

fn mono_zero_window() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(image_module()?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &[])?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &[])?);
    data.extend(rescale_one()?);
    data.extend(pixels_50()?);
    part10(data)
}

fn mono_zero_rescale() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(image_module()?);
    data.extend(window_exact()?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &[])?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &[])?);
    data.extend(pixels_50()?);
    part10(data)
}

fn mono_zero_function() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(image_module()?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(rescale_one()?);
    data.extend(short_le((0x0028, 0x1056), *b"CS", &[])?);
    data.extend(pixels_50()?);
    part10(data)
}

fn image_module() -> Result<Vec<u8>, String> {
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

fn rescale_one() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    Ok(data)
}

fn window_exact() -> Result<Vec<u8>, String> {
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

fn pixels_50() -> Result<Vec<u8>, String> {
    long_le((0x7FE0, 0x0010), *b"OW", &50_u16.to_le_bytes())
}

fn part10(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn mono_unusable_window() -> Result<Vec<u8>, String> {
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
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100\\50"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(short_le(
        (0x0028, 0x1056),
        *b"CS",
        &text_pad("NOT_A_FUNCTION"),
    )?);
    data.extend(long_le((0x0028, 0x3010), *b"SQ", &[])?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &50_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn mono_with(window: bool, sequences: &[(u16, u16)]) -> Result<Vec<u8>, String> {
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
    if window {
        data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
        data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    }
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    if window {
        data.extend(short_le(
            (0x0028, 0x1056),
            *b"CS",
            &text_pad("LINEAR_EXACT"),
        )?);
    }
    for tag in sequences {
        data.extend(long_le(*tag, *b"SQ", &[])?);
    }
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &50_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn file_meta(transfer_syntax: &str) -> Result<Vec<u8>, String> {
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
        &ui(transfer_syntax.as_bytes()),
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

#[test]
fn an_invert_flag_flips_grey_64_to_191() -> Result<(), String> {
    // The windowed frame is grey 64. Invert is a display flip after the LUT,
    // 255 - 64 = 191. It is not a second photometric rule.
    let file = mono_frame(true)?;
    let request = http_post_to("/preview", "X-Ocelli-Invert: 1\r\n", &file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") || !header.contains("X-Ocelli-Invert: 1") {
        return Err("the inverted preview was refused".to_owned());
    }
    let grey = body
        .first()
        .copied()
        .ok_or_else(|| "the inverted preview was empty".to_owned())?;
    if grey != 191 {
        return Err("the inverted preview was not grey 191".to_owned());
    }
    Ok(())
}

#[test]
fn a_window_override_maps_stored_50_to_grey_128() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0. Override LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let preview = preview_frame(
        &mono_frame(true)?,
        FrameRequest {
            frame: 0,
            invert: false,
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
        },
    )
    .map_err(|_| "the window override was refused".to_owned())?;
    if preview.full_range {
        return Err("a window override was labelled full range".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("the window override was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn the_second_frame_uses_its_own_stored_value() -> Result<(), String> {
    // Frame 1 stores 0. ((0 - 100) / 200 + 0.5) * 255 = 0.
    // display_index of a non-positive display value is 0.
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&50_u16.to_le_bytes());
    pixels.extend_from_slice(&0_u16.to_le_bytes());
    let file = tagged_frame(2, &pixels)?;
    let request = http_post_to("/preview", "X-Ocelli-Frame: 1\r\n", &file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") || !header.contains("X-Ocelli-Frame: 1") {
        return Err("the second frame was refused".to_owned());
    }
    let grey = body
        .first()
        .copied()
        .ok_or_else(|| "the second frame was empty".to_owned())?;
    if grey != 0 {
        return Err("the second frame was not grey 0".to_owned());
    }
    let first = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the first frame of the pair was refused".to_owned())?;
    if only_byte(&first)? != 64 {
        return Err("the first frame of the pair was not grey 64".to_owned());
    }
    Ok(())
}

#[test]
fn inspect_json_reports_the_series_fields() -> Result<(), String> {
    let file = tagged_frame(1, &50_u16.to_le_bytes())?;
    let request = http_post_to("/inspect", "", &file);
    let response = one_exchange(&request)?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("inspect was refused".to_owned());
    }
    let json = std::str::from_utf8(body).map_err(|_| "inspect is not utf-8".to_owned())?;
    for needle in [
        "\"seriesUid\":\"2.25.9\"",
        "\"modality\":\"CT\"",
        "\"seriesNumber\":\"3\"",
        "\"instanceNumber\":\"2\"",
        "\"patientName\":\"\"",
        "\"monochrome\":true",
        "\"windowFunction\":\"LINEAR_EXACT\"",
    ] {
        if !json.contains(needle) {
            return Err("inspect omitted a series field".to_owned());
        }
    }
    let direct = inspect_json(&file).map_err(|_| "inspect_json refused the file".to_owned())?;
    if direct != json {
        return Err("inspect_json and the page disagree".to_owned());
    }
    Ok(())
}

#[test]
fn headers_json_lists_the_window_and_hides_pixels() -> Result<(), String> {
    let file = tagged_frame(1, &50_u16.to_le_bytes())?;
    let json = headers_json(&file).map_err(|_| "headers_json refused the file".to_owned())?;
    if !json.contains("\"tag\":\"(0028,1056)\"") || !json.contains("\"value\":\"LINEAR_EXACT\"") {
        return Err("the headers omitted the window function".to_owned());
    }
    if !json.contains("\"tag\":\"(7FE0,0010)\"") || !json.contains("\"value\":\"2 bytes\"") {
        return Err("pixel data was not reported as a byte count".to_owned());
    }
    if json.contains("\"value\":\"50\"") {
        return Err("pixel data was copied into the header table".to_owned());
    }
    Ok(())
}

#[test]
fn a_modality_lut_sequence_is_refused() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0, LINEAR_EXACT center 100 width 200
    // would address grey 64 if the Modality LUT Sequence were ignored.
    // PS3.3 C.11.1 says that sequence replaces the rescale. This preview
    // does not apply a sequence, so the file is refused.
    let file = mono_with(true, &[(0x0028, 0x3000)])?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => {
            if PreviewError::Window.code() != "window" {
                return Err("the window refusal has the wrong code".to_owned());
            }
            if PreviewError::Window.to_string() != "the window or presentation could not be applied"
            {
                return Err("the window refusal has the wrong sentence".to_owned());
            }
            Ok(())
        }
        Ok(_) => Err("a modality LUT sequence was drawn".to_owned()),
        Err(error) => Err(format!("a modality LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_window_override_does_not_replace_a_modality_lut() -> Result<(), String> {
    // The same stored 50 with a Modality LUT Sequence. Center 50 and width 1
    // replace a VOI stage only. The modality sequence is still unapplied.
    let file = mono_with(true, &[(0x0028, 0x3000)])?;
    match preview_frame(
        &file,
        FrameRequest {
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a modality LUT sequence was drawn under a window override".to_owned()),
        Err(error) => Err(format!("a modality LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_voi_lut_sequence_without_an_override_is_refused() -> Result<(), String> {
    // No Window Center, and a VOI LUT Sequence is present. Full-range
    // LINEAR_EXACT would address grey 128 for stored 50. PS3.3 C.11.2 says
    // the sequence is the VOI transform. This preview refuses it.
    let file = mono_with(false, &[(0x0028, 0x3010)])?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a VOI LUT sequence with no window was drawn".to_owned()),
        Err(error) => Err(format!("a VOI LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_file_window_does_not_hide_a_voi_lut_sequence() -> Result<(), String> {
    // The file also has LINEAR_EXACT center 100 width 200. Ignoring the
    // VOI LUT Sequence would address grey 64. The sequence takes precedence
    // until the caller replaces the window.
    let file = mono_with(true, &[(0x0028, 0x3010)])?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a VOI LUT sequence was drawn with the file window".to_owned()),
        Err(error) => Err(format!("a VOI LUT sequence was refused as {error}")),
    }
}

#[test]
fn a_window_override_replaces_a_voi_lut_sequence() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0. Override LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let file = mono_with(true, &[(0x0028, 0x3010)])?;
    let preview = preview_frame(
        &file,
        FrameRequest {
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "a window override did not replace the VOI LUT sequence".to_owned())?;
    if preview.full_range {
        return Err("a window override was labelled full range".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("the VOI LUT override was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn a_window_override_ignores_an_unusable_file_window() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0. The file has a VOI LUT Sequence, two
    // centers with one width, and a function token this preview does not know.
    // Without an override that file is refused. An override of LINEAR_EXACT
    // center 50 width 1 does not read those attributes.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let file = mono_unusable_window()?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("an unusable file window was drawn".to_owned()),
        Err(error) => return Err(format!("an unusable file window was refused as {error}")),
    }
    let preview = preview_frame(
        &file,
        FrameRequest {
            window_center: Some(50.0),
            window_width: Some(1.0),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "a window override still read the unusable file window".to_owned())?;
    if preview.full_range {
        return Err("a window override was labelled full range".to_owned());
    }
    if only_byte(&preview)? != 128 {
        return Err("the override of an unusable window was not grey 128".to_owned());
    }
    Ok(())
}

fn http_post_to(path: &str, extra: &str, body: &[u8]) -> Vec<u8> {
    let mut request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{extra}Content-Length: {len}\r\n\r\n",
        len = body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    request
}

fn tagged_frame(frames: u16, pixels: &[u8]) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0008, 0x0018), *b"UI", &ui(b"2.25.1"))?);
    data.extend(short_le((0x0008, 0x0020), *b"DA", &text_pad("20200101"))?);
    data.extend(short_le((0x0008, 0x0050), *b"SH", &text_pad("A1"))?);
    data.extend(short_le((0x0008, 0x0060), *b"CS", &text_pad("CT"))?);
    data.extend(short_le((0x0008, 0x1030), *b"LO", &text_pad("SYNTH"))?);
    data.extend(short_le((0x0008, 0x103E), *b"LO", &text_pad("SYNTH"))?);
    data.extend(short_le((0x0020, 0x000E), *b"UI", &ui(b"2.25.9"))?);
    data.extend(short_le((0x0020, 0x0011), *b"IS", &text_pad("3"))?);
    data.extend(short_le((0x0020, 0x0013), *b"IS", &text_pad("2"))?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    if frames > 1 {
        data.extend(short_le(
            (0x0028, 0x0008),
            *b"IS",
            &text_pad(&frames.to_string()),
        )?);
    }
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(short_le(
        (0x0028, 0x1056),
        *b"CS",
        &text_pad("LINEAR_EXACT"),
    )?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", pixels)?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}
