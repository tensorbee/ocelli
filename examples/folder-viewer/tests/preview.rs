//! Synthetic Part 10 frames. The expected grey bytes are computed from
//! PS3.3 C.11.2.1.3.2, not from a previous run of this preview.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use ocelli_folder_viewer::{
    FrameRequest, Preview, PreviewError, SLICES, Viewer, ViewerConfig, handle_connection,
    headers_json, inspect_json, preview_first_frame, preview_frame, serve_on, slice,
};
use ocelli_pixel::VoiFunction;
use ocelli_viewport::presentation::WindowPreset;

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
    if !page.contains(">Freeform<") || !page.contains(">Text<") {
        return Err("the page has no freeform or text tool".to_owned());
    }
    if !page.contains(">Rectangle<") || !page.contains(">Ellipse<") || !page.contains(">Remove<") {
        return Err("the page has no rectangle, ellipse, or remove control".to_owned());
    }
    if !page.contains("system-ui") || !page.contains("#141414") {
        return Err("the page is not the sans gray layout".to_owned());
    }
    if page.contains("Georgia") || page.contains("#16130f") {
        return Err("the page still uses the old type or the warm ground".to_owned());
    }
    if !page.contains("if (event.ctrlKey) {\n    event.preventDefault();") {
        return Err("a pinch does not call preventDefault".to_owned());
    }
    if !page.contains("if (Math.abs(wheelAcc) >= 40) {\n    wheelAcc = 0;\n    queueSlice(sign);") {
        return Err("a swipe still steps once per event".to_owned());
    }
    if !page.contains("not DICOM SR") || !page.contains("gone when you reload") {
        return Err("the page does not say what a mark is".to_owned());
    }
    let wrap_at = page
        .find("id=\"wrap\"")
        .ok_or_else(|| "the page has no viewport".to_owned())?;
    let note_at = page
        .find("id=\"note\"")
        .ok_or_else(|| "the page has no note field".to_owned())?;
    let inspector_at = page
        .find("id=\"inspector\"")
        .ok_or_else(|| "the page has no inspector".to_owned())?;
    if wrap_at > note_at || note_at > inspector_at {
        return Err("the note field is outside the viewport".to_owned());
    }
    if !page.contains("id=\"draft-under\"") && !page.contains("\"draft-under\"") {
        return Err("a draft stroke has no underlay".to_owned());
    }
    if !page.contains("Series shortened") {
        return Err("a shortened series is not labelled".to_owned());
    }
    // PS3.3 10.7.1.3: the first Pixel Spacing value is the row spacing, the
    // vertical distance between rows, and the second is the column spacing.
    // The display stretches x by the column spacing and y by the row spacing.
    if !page.contains("return { x: facts.column / least, y: facts.row / least };") {
        return Err("the display aspect does not follow row and column spacing".to_owned());
    }
    for used in [
        "const pictureW = Math.max(1, imageWidth * scale * aspectX);",
        "const pictureH = Math.max(1, imageHeight * scale * aspectY);",
        "node.dataset.aspect = String(aspect.y / aspect.x);",
    ] {
        if !page.contains(used) {
            return Err(format!(
                "the display aspect is not applied where it is used: {used}"
            ));
        }
    }
    Ok(())
}

#[test]
fn every_page_carries_a_well_formed_icon() -> Result<(), String> {
    for path in ["/", "/more", "/use"] {
        let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let response = one_exchange(request.as_bytes())?;
        let (_, body) = split_http(&response)?;
        let page = std::str::from_utf8(body).map_err(|_| format!("{path} is not utf-8"))?;
        let start = "<link rel=\"icon\" type=\"image/svg+xml\" href=\"data:image/svg+xml,";
        let at = page
            .find(start)
            .ok_or_else(|| format!("{path} has no icon"))?;
        let rest = page
            .get(at + start.len()..)
            .ok_or_else(|| format!("{path} icon is cut short"))?;
        let end = rest
            .find('"')
            .ok_or_else(|| format!("{path} icon href never closes"))?;
        let href = rest.get(..end).unwrap_or_default();
        // A raw double quote inside the data URI ends the attribute early and
        // the browser reads a truncated href, so the whole mark must arrive
        // inside the attribute, closing tag included.
        if !href.ends_with("%3C/svg%3E") || rest.get(end..end + 2) != Some("\">") {
            return Err(format!("{path} icon href is truncated by a raw quote"));
        }
    }
    Ok(())
}

#[test]
fn more_and_use_are_static_pages() -> Result<(), String> {
    for path in ["/more", "/use"] {
        let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let response = one_exchange(request.as_bytes())?;
        let (header, body) = split_http(&response)?;
        if !header.contains("200") {
            return Err(format!("{path} was not 200"));
        }
        let page = std::str::from_utf8(body).map_err(|_| format!("{path} is not utf-8"))?;
        if page.contains("<canvas") {
            return Err(format!("{path} contains a viewer"));
        }
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
            window_preset: None,
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
        "\"rowSpacing\":null",
        "\"columnSpacing\":null",
        "\"spacingSource\":\"\"",
        "\"edgeLeft\":\"\"",
        "\"edgeRight\":\"\"",
        "\"edgeTop\":\"\"",
        "\"edgeBottom\":\"\"",
    ] {
        if !json.contains(needle) {
            return Err("inspect omitted a series field".to_owned());
        }
    }
    let direct = inspect_json(&file, 0).map_err(|_| "inspect_json refused the file".to_owned())?;
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

#[test]
fn synthetic_study_has_no_patient_name() -> Result<(), String> {
    // The study is a phantom. (0010,0010) and (0010,0020) must not be written.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    if contains_tag(&file, 0x0010, 0x0010) || contains_tag(&file, 0x0010, 0x0020) {
        return Err("the synthetic study wrote a patient tag".to_owned());
    }
    if !file
        .windows(b"Synthetic phantom".len())
        .any(|window| window == b"Synthetic phantom")
    {
        return Err("the series description is not Synthetic phantom".to_owned());
    }
    if !file
        .windows(b"2.25.".len())
        .any(|window| window == b"2.25.")
    {
        return Err("the synthetic study has no 2.25 uid".to_owned());
    }
    let json =
        inspect_json(&file, 0).map_err(|_| "inspect refused the synthetic slice".to_owned())?;
    if !json.contains("\"patientName\":\"\"") || !json.contains("\"patientId\":\"\"") {
        return Err("inspect reported a patient name or id".to_owned());
    }
    if !json.contains("\"seriesDescription\":\"Synthetic phantom\"") {
        return Err("inspect omitted the series description".to_owned());
    }
    Ok(())
}

#[test]
fn every_synthetic_slice_previews() -> Result<(), String> {
    if SLICES != 12 {
        return Err("the study is not 12 slices".to_owned());
    }
    if slice(SLICES).is_some() {
        return Err("an index past the study returned bytes".to_owned());
    }
    for index in 0..SLICES {
        let file = slice(index).ok_or_else(|| format!("slice {index} was not built"))?;
        let preview = preview_frame(&file, FrameRequest::default())
            .map_err(|_| format!("slice {index} was refused"))?;
        if preview.width != 128 || preview.height != 96 || preview.frames != 1 {
            return Err(format!("slice {index} is not 128 by 96"));
        }
        if preview.grey.len() != 128 * 96 {
            return Err(format!("slice {index} did not return one byte per pixel"));
        }
        // Corner is air. Centre of slice 0 is body and stays body as the
        // ellipse grows. Stored values are HU + 1024.
        if pixel_i16(&file, 0, 0)? != 24 {
            return Err(format!("slice {index} corner is not air"));
        }
        if pixel_i16(&file, 64, 48)? != 1064 {
            return Err(format!("slice {index} centre is not body"));
        }
    }
    Ok(())
}

#[test]
fn soft_tissue_matches_the_file_window() -> Result<(), String> {
    // File window is centre 40 width 400 LINEAR. Soft tissue is the same
    // pair from WindowPreset, so the grey bytes match. A second formula
    // would move the body pixel off 128.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    let file_window = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the file window was refused".to_owned())?;
    let preset = preview_frame(
        &file,
        FrameRequest {
            window_preset: Some(WindowPreset::SoftTissue),
            ..FrameRequest::default()
        },
    )
    .map_err(|_| "the soft tissue preset was refused".to_owned())?;
    if file_window.grey != preset.grey {
        return Err("soft tissue disagreed with the file window".to_owned());
    }
    if preset.window_function != "LINEAR" {
        return Err("soft tissue was not LINEAR".to_owned());
    }
    if preset.window_center.to_bits() != WindowPreset::SoftTissue.center().to_bits()
        || preset.window_width.to_bits() != WindowPreset::SoftTissue.width().to_bits()
    {
        return Err("soft tissue did not use the library centre and width".to_owned());
    }
    // Body at column 64 row 48. PS3.3 C.11.2.1.2 LINEAR.
    // x = 40, c' = 39.5, w' = 399.
    // y = ((40 - 39.5) / 399 + 0.5) * 255 = 127.819550 in f32.
    // display_index adds 0.5 in f32 and addresses 128.
    if grey_at(&preset, 64, 48)? != 128 {
        return Err("the body pixel was not grey 128".to_owned());
    }
    Ok(())
}

#[test]
fn preset_grey_matches_ps3_3_c_11() -> Result<(), String> {
    // Named pixels on slice 0, stored = HU + 1024 with intercept -1024.
    // Air (0, 0) is HU -1000. Body (64, 48) is HU 40.
    // Lung (46, 48) is HU -800. Bone (72, 48) is HU 700.
    // PS3.3 C.11.2.1.2 LINEAR. ymin 0, ymax 255.
    // c' = c - 0.5, w' = w - 1.
    // y = ((x - c') / w' + 0.5) * 255 inside the window, else 0 or 255.
    // display_index is the largest n in 0..=255 with f32(n) <= y + 0.5.
    //
    // Soft tissue c = 40, w = 400. Air 0, body 128, lung 0, bone 255.
    // Lung c = -600, w = 1500.
    // Air y = 59.539696 grey 60. Body y = 236.457626 grey 236.
    // Lung y = 93.562378 grey 94. Bone y = 255 grey 255.
    // Bone c = 300, w = 1500.
    // Air 0, body y = 83.355568 grey 83, lung 0, bone y = 195.630417 grey 196.
    // Changing the intercept from -1024 moves body off these codes.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    if pixel_i16(&file, 0, 0)? != 24
        || pixel_i16(&file, 64, 48)? != 1064
        || pixel_i16(&file, 46, 48)? != 224
        || pixel_i16(&file, 72, 48)? != 1724
    {
        return Err("a named pixel is not the planned Hounsfield value".to_owned());
    }
    let cases = [
        (WindowPreset::SoftTissue, [0_u8, 128, 0, 255]),
        (WindowPreset::Lung, [60, 236, 94, 255]),
        (WindowPreset::Bone, [0, 83, 0, 196]),
    ];
    let points = [(0_usize, 0_usize), (64, 48), (46, 48), (72, 48)];
    for (preset, expected) in cases {
        let preview = preview_frame(
            &file,
            FrameRequest {
                window_preset: Some(preset),
                ..FrameRequest::default()
            },
        )
        .map_err(|_| "a preset was refused".to_owned())?;
        if preview.window_center.to_bits() != preset.center().to_bits()
            || preview.window_width.to_bits() != preset.width().to_bits()
        {
            return Err("a preset did not use the library centre and width".to_owned());
        }
        for (point, grey) in points.iter().zip(expected) {
            if grey_at(&preview, point.0, point.1)? != grey {
                return Err("a named pixel missed the hand-computed grey".to_owned());
            }
        }
    }
    Ok(())
}

#[test]
fn spacing_keeps_row_then_column() -> Result<(), String> {
    // PS3.3 C.7.6.2.1.1 stores adjacent row spacing, then adjacent column
    // spacing. The phantom writes 0.7 then 0.5. The tokens stay decimal text.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    let json =
        inspect_json(&file, 0).map_err(|_| "inspect refused the synthetic slice".to_owned())?;
    if !json.contains("\"rowSpacing\":0.7") || !json.contains("\"columnSpacing\":0.5") {
        return Err("spacing was not row 0.7 then column 0.5".to_owned());
    }
    if !json.contains("\"spacingSource\":\"PixelSpacing\"") {
        return Err("pixel spacing was not named".to_owned());
    }
    let preview = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the spaced slice was refused".to_owned())?;
    if preview.facts.row_spacing.as_deref() != Some("0.7")
        || preview.facts.column_spacing.as_deref() != Some("0.5")
        || preview.facts.spacing_source != "PixelSpacing"
    {
        return Err("the preview spacing disagreed with inspect".to_owned());
    }
    let both = spacing_object(Some("0.7\\0.5"), Some("1.5\\1.5"))?;
    let both_json =
        inspect_json(&both, 0).map_err(|_| "inspect refused both spacings".to_owned())?;
    if !both_json.contains("\"spacingSource\":\"PixelSpacing\"")
        || !both_json.contains("\"rowSpacing\":0.7")
    {
        return Err("imager spacing replaced pixel spacing".to_owned());
    }
    let imager = spacing_object(None, Some("0.3\\0.2"))?;
    let imager_json =
        inspect_json(&imager, 0).map_err(|_| "inspect refused imager spacing".to_owned())?;
    if !imager_json.contains("\"spacingSource\":\"ImagerPixelSpacing\"")
        || !imager_json.contains("\"rowSpacing\":0.3")
        || !imager_json.contains("\"columnSpacing\":0.2")
    {
        return Err("imager spacing was not labelled".to_owned());
    }
    // 00.5 is legal DS and not a JSON number. The reply uses 0.5.
    let padded = spacing_object(Some("00.5\\00.5"), None)?;
    let padded_json =
        inspect_json(&padded, 0).map_err(|_| "inspect refused a padded spacing".to_owned())?;
    if !padded_json.contains("\"rowSpacing\":0.5")
        || !padded_json.contains("\"spacingSource\":\"PixelSpacing\"")
        || padded_json.contains("00.5")
    {
        return Err("a padded spacing was not a JSON number".to_owned());
    }
    // -.5 is legal DS. The reply uses -0.5, which is a JSON number.
    let dotted = spacing_object(Some("-.5\\-.5"), None)?;
    let dotted_json =
        inspect_json(&dotted, 0).map_err(|_| "inspect refused a dotted spacing".to_owned())?;
    if !dotted_json.contains("\"rowSpacing\":-0.5")
        || !dotted_json.contains("\"columnSpacing\":-0.5")
    {
        return Err("a leading dot was not rewritten as a JSON number".to_owned());
    }
    // 5E-1 is legal DS. It must stay Pixel Spacing and must not be replaced
    // by the imager pair 0.6.
    let exponent = spacing_object(Some("5E-1\\5E-1"), Some("0.6\\0.6"))?;
    let exponent_json =
        inspect_json(&exponent, 0).map_err(|_| "inspect refused an exponent spacing".to_owned())?;
    if !exponent_json.contains("\"spacingSource\":\"PixelSpacing\"")
        || !exponent_json.contains("\"rowSpacing\":5E-1")
        || exponent_json.contains("0.6")
    {
        return Err("an exponent spacing fell through to the imager pair".to_owned());
    }
    // Three values are not a pair. The imager value must not be substituted.
    let triple = spacing_object(Some("1\\2\\3"), Some("0.6\\0.6"))?;
    let triple_json =
        inspect_json(&triple, 0).map_err(|_| "inspect refused a triple spacing".to_owned())?;
    if !triple_json.contains("\"rowSpacing\":null")
        || !triple_json.contains("\"spacingSource\":\"\"")
        || triple_json.contains("0.6")
    {
        return Err("a triple pixel spacing used the imager pair".to_owned());
    }
    Ok(())
}

#[test]
fn axial_edge_letters() -> Result<(), String> {
    // PS3.3 C.7.6.1.1.1. Row cosine 1\\0\\0 points to the patient's left,
    // which is the right edge of an axial image. Column cosine 0\\1\\0
    // points posterior, which is the bottom edge. The opposite edges are
    // R and A. Left R, right L, top A, bottom P.
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    let json =
        inspect_json(&file, 0).map_err(|_| "inspect refused the synthetic slice".to_owned())?;
    for needle in [
        "\"edgeLeft\":\"R\"",
        "\"edgeRight\":\"L\"",
        "\"edgeTop\":\"A\"",
        "\"edgeBottom\":\"P\"",
    ] {
        if !json.contains(needle) {
            return Err("the axial letters were not R L A P".to_owned());
        }
    }
    let preview = preview_frame(&file, FrameRequest::default())
        .map_err(|_| "the oriented slice was refused".to_owned())?;
    if preview.facts.edge_left != "R"
        || preview.facts.edge_right != "L"
        || preview.facts.edge_top != "A"
        || preview.facts.edge_bottom != "P"
    {
        return Err("the preview letters disagreed with inspect".to_owned());
    }
    // Row cosine 2\\0\\0 is not a unit vector. Letters stay empty and the
    // frame still previews.
    let stretched = oriented_frame("2\\0\\0\\0\\1\\0")?;
    let stretched_json = inspect_json(&stretched, 0)
        .map_err(|_| "inspect refused a non-unit orientation".to_owned())?;
    if !stretched_json.contains("\"edgeLeft\":\"\"")
        || !stretched_json.contains("\"edgeRight\":\"\"")
    {
        return Err("a non-unit orientation still produced letters".to_owned());
    }
    preview_frame(&stretched, FrameRequest::default())
        .map_err(|_| "a non-unit orientation was refused".to_owned())?;
    Ok(())
}

#[test]
fn a_preset_refuses_a_voi_lut_sequence() -> Result<(), String> {
    // The file has a VOI LUT Sequence. A numeric override may replace it.
    // A named preset must not. Lung would otherwise draw a window.
    let file = mono_with(true, &[(0x0028, 0x3010)])?;
    match preview_frame(
        &file,
        FrameRequest {
            window_preset: Some(WindowPreset::Lung),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("a preset replaced a VOI LUT sequence".to_owned()),
        Err(error) => {
            return Err(format!(
                "a preset on a VOI LUT sequence was refused as {error}"
            ));
        }
    }
    // A Modality LUT Sequence is also refused. The preset must not draw
    // the identity slope in its place.
    let modality = mono_with(true, &[(0x0028, 0x3000)])?;
    match preview_frame(
        &modality,
        FrameRequest {
            window_preset: Some(WindowPreset::SoftTissue),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => Ok(()),
        Ok(_) => Err("a preset replaced a modality LUT sequence".to_owned()),
        Err(error) => Err(format!(
            "a preset on a modality LUT sequence was refused as {error}"
        )),
    }
}

#[test]
fn a_preset_with_a_numeric_window_is_refused() -> Result<(), String> {
    let file = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    match preview_frame(
        &file,
        FrameRequest {
            window_center: Some(40.0),
            window_width: Some(400.0),
            window_preset: Some(WindowPreset::Bone),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("a preset was combined with a numeric window".to_owned()),
        Err(error) => {
            return Err(format!("a combined window was refused as {error}"));
        }
    }
    let request = http_post_to(
        "/preview",
        "X-Ocelli-Window-Preset: lung\r\nX-Ocelli-Window-Center: 40\r\n",
        &file,
    );
    let response = one_exchange(&request)?;
    let (header, _) = split_http(&response)?;
    if !header.contains("400") || !header.contains("X-Ocelli-Error: window") {
        return Err("the preset header was combined with a centre".to_owned());
    }
    let lung = http_post_to("/preview", "X-Ocelli-Window-Preset: lung\r\n", &file);
    let drawn = one_exchange(&lung)?;
    let (header, body) = split_http(&drawn)?;
    if !header.contains("200")
        || !header.contains("X-Ocelli-Window-Center: -600")
        || !header.contains("X-Ocelli-Window-Width: 1500")
        || !header.contains("X-Ocelli-Window-Function: LINEAR")
        || !header.contains("X-Ocelli-Row-Spacing: 0.7")
        || !header.contains("X-Ocelli-Column-Spacing: 0.5")
        || !header.contains("X-Ocelli-Spacing-Source: PixelSpacing")
        || !header.contains("X-Ocelli-Edge-Left: R")
        || !header.contains("X-Ocelli-Edge-Right: L")
        || !header.contains("X-Ocelli-Edge-Top: A")
        || !header.contains("X-Ocelli-Edge-Bottom: P")
    {
        return Err("the lung preset response omitted the window or the geometry".to_owned());
    }
    // Lung window, body pixel. y = 236.457626, grey 236.
    let index = 48 * 128 + 64;
    let grey = body
        .get(index)
        .copied()
        .ok_or_else(|| "the lung preview was short".to_owned())?;
    if grey != 236 {
        return Err("the lung preset body pixel was not grey 236".to_owned());
    }
    match preview_frame(
        &file,
        FrameRequest {
            window_preset: Some(WindowPreset::Lung),
            window_function: Some(VoiFunction::LinearExact),
            ..FrameRequest::default()
        },
    ) {
        Err(PreviewError::Window) => {}
        Ok(_) => return Err("a preset kept a different window function".to_owned()),
        Err(error) => return Err(format!("a preset function was refused as {error}")),
    }
    // The body is empty so the window error is written with nothing left
    // unread on the socket.
    let bad = http_post_to("/preview", "X-Ocelli-Window-Preset: soft\r\n", b"");
    let refused = one_exchange(&bad)?;
    let (header, _) = split_http(&refused)?;
    if !header.contains("400") || !header.contains("X-Ocelli-Error: window") {
        return Err("a bad preset token was not the window error".to_owned());
    }
    Ok(())
}

#[test]
fn synthetic_routes_serve_part10() -> Result<(), String> {
    let count = one_exchange(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&count)?;
    if !header.contains("200") || body != br#"{"slices":12}"# {
        return Err("the study count was not 12".to_owned());
    }
    let first = one_exchange(b"GET /synthetic/0 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&first)?;
    let expected = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    if !header.contains("200") || !header.contains("application/octet-stream") || body != expected {
        return Err("GET /synthetic/0 was not the first slice".to_owned());
    }
    let missing = one_exchange(b"GET /synthetic/12 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, _) = split_http(&missing)?;
    if !header.contains("404") {
        return Err("an index past the study was not 404".to_owned());
    }
    let word = one_exchange(b"GET /synthetic/no HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, _) = split_http(&word)?;
    if !header.contains("404") {
        return Err("a non-numeric slice was not 404".to_owned());
    }
    Ok(())
}

fn grey_at(preview: &Preview, column: usize, row: usize) -> Result<u8, String> {
    let index = row
        .checked_mul(128)
        .and_then(|row_start| row_start.checked_add(column))
        .ok_or_else(|| "the pixel index overflowed".to_owned())?;
    preview
        .grey
        .get(index)
        .copied()
        .ok_or_else(|| "the pixel is outside the frame".to_owned())
}

fn pixel_i16(file: &[u8], column: usize, row: usize) -> Result<i16, String> {
    let marker = [0xE0, 0x7F, 0x10, 0x00, b'O', b'W', 0, 0];
    let start = file
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or_else(|| "the slice has no OW pixel data".to_owned())?;
    let length_at = start + marker.len();
    let length_bytes = file
        .get(length_at..length_at + 4)
        .ok_or_else(|| "the pixel length is short".to_owned())?;
    let length_bytes: [u8; 4] = length_bytes
        .try_into()
        .map_err(|_| "the pixel length is short".to_owned())?;
    let length = u32::from_le_bytes(length_bytes);
    let pixels_at = length_at + 4;
    let offset = row
        .checked_mul(128)
        .and_then(|row_start| row_start.checked_add(column))
        .and_then(|index| index.checked_mul(2))
        .ok_or_else(|| "the stored index overflowed".to_owned())?;
    if u32::try_from(offset + 2).map_err(|_| "the stored index overflowed".to_owned())? > length {
        return Err("the stored pixel is outside pixel data".to_owned());
    }
    let at = pixels_at + offset;
    let bytes = file
        .get(at..at + 2)
        .ok_or_else(|| "the stored pixel is short".to_owned())?;
    let bytes: [u8; 2] = bytes
        .try_into()
        .map_err(|_| "the stored pixel is short".to_owned())?;
    Ok(i16::from_le_bytes(bytes))
}

fn contains_tag(bytes: &[u8], group: u16, element: u16) -> bool {
    let mut needle = [0_u8; 4];
    needle[..2].copy_from_slice(&group.to_le_bytes());
    needle[2..].copy_from_slice(&element.to_le_bytes());
    bytes.windows(4).any(|window| window == needle)
}

fn oriented_frame(orientation: &str) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0020, 0x0037), *b"DS", &text_pad(orientation))?);
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
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &0_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

fn spacing_object(pixel: Option<&str>, imager: Option<&str>) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    if let Some(pair) = imager {
        data.extend(short_le((0x0018, 0x1164), *b"DS", &text_pad(pair))?);
    }
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("MONOCHROME2"),
    )?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    if let Some(pair) = pixel {
        data.extend(short_le((0x0028, 0x0030), *b"DS", &text_pad(pair))?);
    }
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("1"))?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &0_u16.to_le_bytes())?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
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

fn viewer_on(bytes: usize) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(Viewer::new(ViewerConfig { cache_bytes: bytes }));
    std::thread::spawn(move || {
        let _ = serve_on(listener, viewer);
    });
    Ok(addr)
}

fn viewer_capped(bytes: usize, cap: usize) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(Viewer::with_connection_cap(
        ViewerConfig { cache_bytes: bytes },
        cap,
    ));
    std::thread::spawn(move || {
        let _ = serve_on(listener, viewer);
    });
    Ok(addr)
}

fn header_line<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split("\r\n").find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.eq_ignore_ascii_case(name) {
            Some(value.trim())
        } else {
            None
        }
    })
}

fn json_number(body: &str, key: &str) -> Result<usize, String> {
    let needle = format!("\"{key}\":");
    let rest = body
        .split_once(&needle)
        .ok_or_else(|| format!("the census omitted {key}"))?
        .1;
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits
        .parse::<usize>()
        .map_err(|_| format!("the census {key} was not a number"))
}

fn held_census(addr: SocketAddr) -> Result<String, String> {
    let response = exchange(addr, b"GET /held HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.starts_with("HTTP/1.1 200 ") {
        return Err(header.to_owned());
    }
    std::str::from_utf8(body)
        .map(str::to_owned)
        .map_err(|_| "the census was not text".to_owned())
}

fn preview_id(id: &str, extra: &str) -> Vec<u8> {
    format!(
        "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\n{extra}Content-Length: 0\r\n\r\n"
    )
    .into_bytes()
}

fn read_one(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        if let Some(end) = buf
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|start| start + 4)
        {
            break end;
        }
        let n = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if n == 0 {
            return Err("the response closed before the header".to_owned());
        }
        let Some(got) = chunk.get(..n) else {
            return Err("the response read was short".to_owned());
        };
        buf.extend_from_slice(got);
    };
    let header = {
        let bytes = buf
            .get(..header_end)
            .ok_or_else(|| "the response header is short".to_owned())?;
        std::str::from_utf8(bytes).map_err(|_| "the response header is not text".to_owned())?
    };
    let length = header_line(header, "Content-Length")
        .ok_or_else(|| "the response has no length".to_owned())?
        .parse::<usize>()
        .map_err(|_| "the response length is not a number".to_owned())?;
    while buf.len() - header_end < length {
        let n = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if n == 0 {
            return Err("the response ended early".to_owned());
        }
        let have = buf.len() - header_end;
        let need = length - have;
        let take = n.min(need);
        let Some(got) = chunk.get(..take) else {
            return Err("the response read was short".to_owned());
        };
        buf.extend_from_slice(got);
    }
    Ok(buf)
}

fn response_parts(bytes: &[u8]) -> Result<Vec<(&str, &[u8])>, String> {
    let mut rest = bytes;
    let mut found = Vec::new();
    while !rest.is_empty() {
        let (header, body) = split_http(rest)?;
        let length = header_line(header, "Content-Length")
            .ok_or_else(|| "a pipelined response has no length".to_owned())?
            .parse::<usize>()
            .map_err(|_| "a pipelined response length is not a number".to_owned())?;
        let taken = body
            .get(..length)
            .ok_or_else(|| "a pipelined response body is short".to_owned())?;
        found.push((header, taken));
        let skip = header
            .len()
            .checked_add(length)
            .ok_or_else(|| "a pipelined response is too long".to_owned())?;
        rest = rest
            .get(skip..)
            .ok_or_else(|| "a pipelined response overran".to_owned())?;
    }
    Ok(found)
}

#[test]
fn held_frames_keep_the_hand_computed_grey() -> Result<(), String> {
    // Stored 50, slope 1, intercept 0, file LINEAR_EXACT center 100 width 200.
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    // Override center 50 width 1: ((50 - 50) / 1 + 0.5) * 255 = 127.5, grey 128.
    // Invert of 64 is 255 - 64 = 191.
    // PS3.3 C.11.2.1.2 and C.11.2.1.3.2. Invert is after the chain.
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    if !header.contains("X-Ocelli-Held: 1") {
        return Err("the file was not held".to_owned());
    }
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let first = exchange(addr, &preview_id(id, ""))?;
    let (first_header, first_body) = split_http(&first)?;
    if !first_header.contains("X-Ocelli-Source: decoded") {
        return Err("the first preview was not decoded".to_owned());
    }
    if first_body != [64] {
        return Err(format!(
            "the held file window was not grey 64: {first_body:?}"
        ));
    }
    let second = exchange(addr, &preview_id(id, ""))?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("X-Ocelli-Source: held") || second_body != [64] {
        return Err("the second preview did not reuse the held frame".to_owned());
    }
    let narrow = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (narrow_header, narrow_body) = split_http(&narrow)?;
    if !narrow_header.contains("X-Ocelli-Source: held") || narrow_body != [128] {
        return Err("the held override was not grey 128".to_owned());
    }
    let flipped = exchange(addr, &preview_id(id, "X-Ocelli-Invert: 1\r\n"))?;
    let (_, flipped_body) = split_http(&flipped)?;
    if flipped_body != [191] {
        return Err(format!(
            "the held invert was not grey 191: {flipped_body:?}"
        ));
    }
    Ok(())
}

#[test]
fn a_held_missing_window_stays_full_range() -> Result<(), String> {
    // Stored 50. Absent rescale is slope 1 and intercept 0. Absent center
    // and width are LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    // A later request with no window must keep that range. The first
    // request's center 100 width 200 must not stick.
    // ((50 - 100) / 200 + 0.5) * 255 = 63.75, which addresses grey 64.
    let file = mono_frame(false)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let overridden = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 100\r\nX-Ocelli-Window-Width: 200\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (over_header, over_body) = split_http(&overridden)?;
    if over_header.contains("X-Ocelli-Full-Range: 1") || over_body != [64] {
        return Err("the first held window was not grey 64".to_owned());
    }
    let plain = exchange(addr, &preview_id(id, ""))?;
    let (plain_header, plain_body) = split_http(&plain)?;
    if !plain_header.contains("X-Ocelli-Full-Range: 1") || plain_body != [128] {
        return Err("the held file lost its full range".to_owned());
    }
    if !plain_header.contains("X-Ocelli-Source: held") {
        return Err("the full range preview decoded again".to_owned());
    }
    Ok(())
}

#[test]
fn a_file_post_matches_inspect_and_holds_once() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let inspected = exchange(addr, &http_post_to("/inspect", "", &file))?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (inspect_header, inspect_body) = split_http(&inspected)?;
    let (file_header, file_body) = split_http(&posted)?;
    if inspect_body != file_body {
        return Err("the file response body differed from inspect".to_owned());
    }
    if !file_header.contains("200") || !inspect_header.contains("200") {
        return Err("inspect and file did not both succeed".to_owned());
    }
    let id = header_line(file_header, "X-Ocelli-File")
        .ok_or_else(|| "the file id was missing".to_owned())?;
    let again = exchange(addr, &http_post_to("/file", "", &file))?;
    let (again_header, _) = split_http(&again)?;
    let other = header_line(again_header, "X-Ocelli-File")
        .ok_or_else(|| "the second id was missing".to_owned())?;
    if id == other {
        return Err("a second post reused an id".to_owned());
    }
    let junk = b"not a dicom file";
    let refused = exchange(addr, &http_post_to("/file", "", junk))?;
    let direct = exchange(addr, &http_post_to("/inspect", "", junk))?;
    let (refused_header, refused_body) = split_http(&refused)?;
    let (direct_header, direct_body) = split_http(&direct)?;
    if refused_body != direct_body
        || !refused_header.contains("X-Ocelli-Error: not-part10")
        || !direct_header.contains("X-Ocelli-Error: not-part10")
    {
        return Err("a non Part 10 file was not the same refusal".to_owned());
    }
    if refused_header.contains("X-Ocelli-File:") {
        return Err("a non Part 10 file was given an id".to_owned());
    }
    Ok(())
}

#[test]
fn an_unknown_id_is_gone_and_a_bad_id_is_a_header_error() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let unknown = exchange(addr, &preview_id("1.1", ""))?;
    let (header, body) = split_http(&unknown)?;
    if !header.contains("410") || !header.contains("X-Ocelli-Error: file-gone") {
        return Err("an unknown id was not gone".to_owned());
    }
    if body != b"the file is no longer held by this process" {
        return Err("the gone sentence differed".to_owned());
    }
    let bad = exchange(addr, &preview_id("A.1", ""))?;
    let (bad_header, _) = split_http(&bad)?;
    if !bad_header.contains("400") || !bad_header.contains("X-Ocelli-Error: header") {
        return Err("an uppercase id was not a header error".to_owned());
    }
    let mixed = "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: 1.1\r\nContent-Length: 4\r\n\r\nnope";
    let both = exchange(addr, mixed.as_bytes())?;
    let (both_header, both_body) = split_http(&both)?;
    if !both_header.contains("400") || both_body != b"a file id and a body cannot be sent together"
    {
        return Err("an id with a body was accepted".to_owned());
    }
    Ok(())
}

#[test]
fn two_viewers_do_not_serve_each_others_ids() -> Result<(), String> {
    let left = viewer_on(8 * 1024 * 1024)?;
    let right = viewer_on(8 * 1024 * 1024)?;
    let file = mono_frame(true)?;
    let left_post = exchange(left, &http_post_to("/file", "", &file))?;
    let right_post = exchange(right, &http_post_to("/file", "", &file))?;
    let (left_header, _) = split_http(&left_post)?;
    let (right_header, _) = split_http(&right_post)?;
    let left_id = header_line(left_header, "X-Ocelli-File")
        .ok_or_else(|| "the left id was missing".to_owned())?;
    let right_id = header_line(right_header, "X-Ocelli-File")
        .ok_or_else(|| "the right id was missing".to_owned())?;
    if left_id == right_id {
        return Err("two viewers issued the same id".to_owned());
    }
    let crossed = exchange(right, &preview_id(left_id, ""))?;
    let (crossed_header, _) = split_http(&crossed)?;
    if !crossed_header.contains("410") {
        return Err("another viewer served a foreign id".to_owned());
    }
    let kept = exchange(left, &preview_id(left_id, ""))?;
    let (kept_header, kept_body) = split_http(&kept)?;
    if !kept_header.contains("200") || kept_body != [64] {
        return Err("the owning viewer lost its file".to_owned());
    }
    Ok(())
}

#[test]
fn a_voi_lut_preset_still_holds_the_frame() -> Result<(), String> {
    // The file has a VOI LUT Sequence. A preset does not replace it, so the
    // first answer is 400 window. The frame is still held. A numeric override
    // then draws grey 128 from the held frame.
    // Stored 50, slope 1, intercept 0, LINEAR_EXACT center 50 width 1.
    // ((50 - 50) / 1 + 0.5) * 255 = 127.5, which addresses grey 128.
    let file = mono_with(true, &[(0x0028, 0x3010)])?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let preset = exchange(
        addr,
        &preview_id(id, "X-Ocelli-Window-Preset: soft_tissue\r\n"),
    )?;
    let (preset_header, _) = split_http(&preset)?;
    if !preset_header.contains("400") || !preset_header.contains("X-Ocelli-Error: window") {
        return Err("the preset did not refuse the VOI LUT".to_owned());
    }
    let drawn = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (drawn_header, drawn_body) = split_http(&drawn)?;
    if !drawn_header.contains("X-Ocelli-Source: held") || drawn_body != [128] {
        return Err("the override after a preset did not reuse the held frame".to_owned());
    }
    Ok(())
}

#[test]
fn single_frame_bytes_drop_and_preview_stays() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let headers = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (headers_header, _) = split_http(&headers)?;
    if !headers_header.contains("200") {
        return Err("headers by id failed before the frame was held".to_owned());
    }
    let _ = exchange(addr, &preview_id(id, ""))?;
    let after = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (after_header, _) = split_http(&after)?;
    if !after_header.contains("410") {
        return Err("single-frame bytes were still held".to_owned());
    }
    let census = held_census(addr)?;
    let used = json_number(&census, "used")?;
    if json_number(&census, "files")? != 0
        || json_number(&census, "frames")? != 1
        || used == 0
        || used >= file.len()
    {
        return Err(format!(
            "held bytes {used} were not the decoded frame alone, file {} census {census}",
            file.len()
        ));
    }
    let again = exchange(addr, &preview_id(id, ""))?;
    let (again_header, again_body) = split_http(&again)?;
    if !again_header.contains("X-Ocelli-Source: held") || again_body != [64] {
        return Err("the held frame was dropped with the bytes".to_owned());
    }
    Ok(())
}

#[test]
fn multi_frame_bytes_drop_only_after_every_frame() -> Result<(), String> {
    let file = enhanced_frames()?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let first = exchange(addr, &preview_id(id, ""))?;
    let (_, first_body) = split_http(&first)?;
    if first_body != [64] {
        return Err("held frame 0 was not grey 64".to_owned());
    }
    let mid = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (mid_header, _) = split_http(&mid)?;
    if !mid_header.contains("200") {
        return Err("multi-frame bytes dropped before every frame was held".to_owned());
    }
    let second = exchange(addr, &preview_id(id, "X-Ocelli-Frame: 1\r\n"))?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("X-Ocelli-Source: decoded") || second_body != [128] {
        return Err("held frame 1 was not grey 128".to_owned());
    }
    let end = exchange(
        addr,
        format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let (end_header, _) = split_http(&end)?;
    if !end_header.contains("410") {
        return Err("multi-frame bytes stayed after every frame was held".to_owned());
    }
    Ok(())
}

#[test]
fn pipelined_bytes_are_the_next_request() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let file = mono_frame(true)?;
    let mut request = http_post_to("/inspect", "", &file);
    request.extend_from_slice(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    let response = exchange(addr, &request)?;
    let parts = response_parts(&response)?;
    let second = parts
        .get(1)
        .ok_or_else(|| "the second response was missing".to_owned())?;
    if !second.0.contains("200") || !second.1.windows(8).any(|window| window == b"\"slices\"") {
        return Err("the pipelined request was not served".to_owned());
    }
    Ok(())
}

#[test]
fn a_full_connection_is_busy_without_taking_the_slot() -> Result<(), String> {
    let addr = viewer_capped(1024, 1)?;
    let mut first = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    first
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let opened = read_one(&mut first)?;
    let (opened_header, _) = split_http(&opened)?;
    if !opened_header.contains("200") || !opened_header.contains("Connection: keep-alive") {
        return Err("the first connection was not kept".to_owned());
    }
    let busy = exchange(addr, b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (busy_header, busy_body) = split_http(&busy)?;
    if !busy_header.contains("503")
        || !busy_header.contains("X-Ocelli-Error: busy")
        || busy_body != b"this process is already serving its connection limit"
    {
        return Err("a second connection was not busy".to_owned());
    }
    drop(first);
    let mut served = false;
    for _ in 0..20 {
        let again = exchange(addr, b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
        let (again_header, _) = split_http(&again)?;
        if again_header.contains("200") {
            served = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if !served {
        return Err("the slot was not released".to_owned());
    }
    Ok(())
}

#[test]
fn held_census_counts_files_and_frames() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let empty = held_census(addr)?;
    if json_number(&empty, "budget")? != 8 * 1024 * 1024 || json_number(&empty, "used")? != 0 {
        return Err(format!("the empty census was {empty}"));
    }
    if json_number(&empty, "inflight")? != 0
        || json_number(&empty, "workspace")? != 0
        || json_number(&empty, "free")? != 0
    {
        return Err(format!("an idle viewer reported live bytes: {empty}"));
    }
    if empty.contains("\"pool\":") {
        return Err(format!("the census named a pool: {empty}"));
    }
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    if !empty.contains(&format!("\"build\":\"{build}\"")) {
        return Err(format!("the build name was not {build}: {empty}"));
    }
    let file = mono_frame(true)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let files = held_census(addr)?;
    if json_number(&files, "files")? != 1 || json_number(&files, "frames")? != 0 {
        return Err(format!("the file census was {files}"));
    }
    let _ = exchange(addr, &preview_id(id, ""))?;
    let frames = held_census(addr)?;
    if json_number(&frames, "files")? != 0 || json_number(&frames, "frames")? != 1 {
        return Err(format!("the frame census was {frames}"));
    }
    if json_number(&frames, "inflight")? != 0 || json_number(&frames, "workspace")? == 0 {
        return Err(format!("the preview census was {frames}"));
    }
    let used = json_number(&frames, "used")?;
    let again = exchange(addr, &preview_id(id, ""))?;
    let _ = split_http(&again)?;
    let census = held_census(addr)?;
    let stable = json_number(&census, "used")?;
    if used != stable || used > 8 * 1024 * 1024 {
        return Err("a second window changed the held bytes".to_owned());
    }
    let tiny = viewer_on(1)?;
    let refused = exchange(tiny, &http_post_to("/file", "", &file))?;
    let (refused_header, _) = split_http(&refused)?;
    if !refused_header.contains("X-Ocelli-Held: 0") || refused_header.contains("X-Ocelli-File:") {
        return Err("a file larger than the budget was held".to_owned());
    }
    let census = held_census(tiny)?;
    if json_number(&census, "used")? != 0
        || json_number(&census, "files")? != 0
        || json_number(&census, "inflight")? != 0
        || json_number(&census, "workspace")? != 0
    {
        return Err(format!("a refused file changed the census: {census}"));
    }
    Ok(())
}

#[test]
fn a_bad_window_does_not_drop_a_held_file() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let bad = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Preset: lung\r\nX-Ocelli-Window-Center: 40\r\n",
        ),
    )?;
    let (bad_header, _) = split_http(&bad)?;
    if !bad_header.contains("400") {
        return Err("a mixed preset was accepted".to_owned());
    }
    let drawn = exchange(addr, &preview_id(id, ""))?;
    let (drawn_header, drawn_body) = split_http(&drawn)?;
    if !drawn_header.contains("200") || drawn_body != [64] {
        return Err("a bad window dropped the file".to_owned());
    }
    Ok(())
}

fn file_id_form(id: &str) -> bool {
    let Some((generation, serial)) = id.split_once('.') else {
        return false;
    };
    let hex = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    };
    hex(generation) && hex(serial) && !id.contains('/')
}

fn status_token(header: &str) -> Result<&str, String> {
    header
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "the status line was short".to_owned())
}

fn ocelli_headers(header: &str) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for line in header.split("\r\n") {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if !key.to_ascii_lowercase().starts_with("x-ocelli-") {
            continue;
        }
        if key.eq_ignore_ascii_case("X-Ocelli-Source") {
            continue;
        }
        rows.push((key.to_ascii_lowercase(), value.trim().to_owned()));
    }
    rows.sort();
    rows
}

fn same_preview(left: &[u8], right: &[u8]) -> Result<(), String> {
    let (left_header, left_body) = split_http(left)?;
    let (right_header, right_body) = split_http(right)?;
    if status_token(left_header)? != status_token(right_header)? || left_body != right_body {
        return Err(format!(
            "status or body differed: {} and {}",
            status_token(left_header)?,
            status_token(right_header)?
        ));
    }
    if ocelli_headers(left_header) != ocelli_headers(right_header) {
        return Err("the X-Ocelli headers differed apart from source".to_owned());
    }
    Ok(())
}

fn same_held_and_body(addr: SocketAddr, file: &[u8], extra: &str) -> Result<(), String> {
    let posted = exchange(addr, &http_post_to("/file", "", file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    if !file_id_form(id) {
        return Err(format!("the file id {id} is not generation.serial"));
    }
    let by_id = exchange(addr, &preview_id(id, extra))?;
    let by_body = exchange(addr, &http_post_to("/preview", extra, file))?;
    same_preview(&by_id, &by_body)
}

#[test]
fn a_posted_file_returns_its_inspect_json_and_an_id() -> Result<(), String> {
    a_file_post_matches_inspect_and_holds_once()?;
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    if !file_id_form(id) {
        return Err(format!("the file id {id} is not generation.serial"));
    }
    Ok(())
}

#[test]
fn a_held_frame_matches_the_posted_frame() -> Result<(), String> {
    let addr = viewer_on(16 * 1024 * 1024)?;
    let mono = mono_frame(true)?;
    same_held_and_body(addr, &mono, "")?;
    same_held_and_body(
        addr,
        &mono,
        "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
    )?;
    same_held_and_body(addr, &mono, "X-Ocelli-Invert: 1\r\n")?;
    let enhanced = enhanced_frames()?;
    same_held_and_body(addr, &enhanced, "")?;
    same_held_and_body(addr, &enhanced, "X-Ocelli-Frame: 1\r\n")?;
    let study = slice(0).ok_or_else(|| "slice 0 was not built".to_owned())?;
    same_held_and_body(addr, &study, "X-Ocelli-Window-Preset: lung\r\n")?;
    let modality = mono_with(true, &[(0x0028, 0x3000)])?;
    same_held_and_body(addr, &modality, "")?;
    let voi = mono_with(true, &[(0x0028, 0x3010)])?;
    same_held_and_body(addr, &voi, "")?;
    same_held_and_body(addr, &enhanced, "X-Ocelli-Frame: 2\r\n")?;
    // A file that is not Part 10 is not held, so it has no id form.
    let junk = b"not a dicom file";
    let posted = exchange(addr, &http_post_to("/file", "", junk))?;
    let (posted_header, _) = split_http(&posted)?;
    if posted_header.contains("X-Ocelli-File:") {
        return Err("a non Part 10 file was given an id".to_owned());
    }
    let left = exchange(addr, &http_post_to("/preview", "", junk))?;
    let right = exchange(addr, &http_post_to("/preview", "", junk))?;
    same_preview(&left, &right)?;
    Ok(())
}

#[test]
fn a_window_change_on_a_held_frame_does_not_decode_again() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let first = exchange(addr, &preview_id(id, ""))?;
    let (first_header, first_body) = split_http(&first)?;
    if !first_header.contains("X-Ocelli-Source: decoded") || first_body != [64] {
        return Err("the first held preview did not decode".to_owned());
    }
    let second = exchange(
        addr,
        &preview_id(
            id,
            "X-Ocelli-Window-Center: 50\r\nX-Ocelli-Window-Width: 1\r\nX-Ocelli-Window-Function: LINEAR_EXACT\r\n",
        ),
    )?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("X-Ocelli-Source: held") || second_body != [128] {
        return Err("the window change decoded again".to_owned());
    }
    Ok(())
}

#[test]
fn an_evicted_file_answers_gone() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(8 * 1024)?;
    let first = exchange(addr, &http_post_to("/file", "", &file))?;
    let (first_header, _) = split_http(&first)?;
    if !first_header.contains("X-Ocelli-Held: 1") {
        return Err("the first file was not held".to_owned());
    }
    let id = header_line(first_header, "X-Ocelli-File")
        .ok_or_else(|| "the file id was missing".to_owned())?;
    let census = held_census(addr)?;
    let used = json_number(&census, "used")?;
    if used == 0 || used > 8 * 1024 {
        return Err(format!("the first file used {used}"));
    }
    // One more file than the budget can hold. Asking early would refresh the
    // first id and keep it.
    let room = (8 * 1024) / used;
    let mut survivor = String::new();
    for _ in 0..room + 2 {
        let posted = exchange(addr, &http_post_to("/file", "", &file))?;
        let (posted_header, _) = split_http(&posted)?;
        if let Some(next) = header_line(posted_header, "X-Ocelli-File") {
            survivor = next.to_owned();
        }
    }
    let answer = exchange(addr, &preview_id(id, ""))?;
    let (answer_header, answer_body) = split_http(&answer)?;
    if !answer_header.contains("410")
        || !answer_header.contains("X-Ocelli-Error: file-gone")
        || answer_body != b"the file is no longer held by this process"
    {
        return Err("the first id was not evicted".to_owned());
    }
    if survivor.is_empty() {
        return Err("no later file was held".to_owned());
    }
    let kept = exchange(addr, &preview_id(&survivor, ""))?;
    let (kept_header, kept_body) = split_http(&kept)?;
    if !kept_header.contains("200") || kept_body != [64] {
        return Err("a later held id stopped answering".to_owned());
    }
    Ok(())
}

#[test]
fn an_id_from_another_process_answers_gone() -> Result<(), String> {
    two_viewers_do_not_serve_each_others_ids()
}

#[test]
fn a_file_larger_than_the_budget_is_not_held() -> Result<(), String> {
    let file = mono_frame(true)?;
    let addr = viewer_on(1)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    if !header.contains("X-Ocelli-Held: 0") || header.contains("X-Ocelli-File:") {
        return Err("a file larger than the budget was held".to_owned());
    }
    let census = held_census(addr)?;
    if json_number(&census, "used")? != 0
        || json_number(&census, "files")? != 0
        || json_number(&census, "inflight")? != 0
        || json_number(&census, "workspace")? != 0
    {
        return Err(format!("a refused file changed the census: {census}"));
    }
    Ok(())
}

#[test]
fn an_id_with_a_body_is_refused() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let mixed = "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: 1.1\r\nContent-Length: 4\r\n\r\nnope";
    let both = exchange(addr, mixed.as_bytes())?;
    let (header, body) = split_http(&both)?;
    if !header.contains("400")
        || !header.contains("X-Ocelli-Error: header")
        || body != b"a file id and a body cannot be sent together"
    {
        return Err("an id with a body was accepted".to_owned());
    }
    Ok(())
}

#[test]
fn a_window_refusal_keeps_the_held_frame() -> Result<(), String> {
    a_voi_lut_preset_still_holds_the_frame()?;
    a_bad_window_does_not_drop_a_held_file()
}

#[test]
fn keep_alive_serves_two_requests() -> Result<(), String> {
    let addr = viewer_on(8 * 1024 * 1024)?;
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(b"GET /held HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let first = read_one(&mut stream)?;
    let (first_header, _) = split_http(&first)?;
    if !first_header.contains("200") || !first_header.contains("Connection: keep-alive") {
        return Err("the first request was not kept".to_owned());
    }
    stream
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let second = read_one(&mut stream)?;
    let (second_header, second_body) = split_http(&second)?;
    if !second_header.contains("200")
        || !second_body.windows(8).any(|window| window == b"\"slices\"")
    {
        return Err("the second request on the same connection was not served".to_owned());
    }
    Ok(())
}

#[test]
fn pipelined_requests_get_two_replies() -> Result<(), String> {
    pipelined_bytes_are_the_next_request()
}

#[test]
fn a_connection_over_the_cap_is_busy() -> Result<(), String> {
    a_full_connection_is_busy_without_taking_the_slot()
}

fn span_frame() -> Result<Vec<u8>, String> {
    // 1 by 4, stored 0, 27, 34 and 200. No Window Center, Window Width or function.
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&0_u16.to_le_bytes());
    pixels.extend_from_slice(&27_u16.to_le_bytes());
    pixels.extend_from_slice(&34_u16.to_le_bytes());
    pixels.extend_from_slice(&200_u16.to_le_bytes());
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
    data.extend(short_le((0x0028, 0x0011), *b"US", &4_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    data.extend(long_le((0x7FE0, 0x0010), *b"OW", &pixels)?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta("1.2.840.10008.1.2.1")?);
    bytes.extend(data);
    Ok(bytes)
}

#[test]
fn a_function_sent_alone_selects_the_full_range_window() -> Result<(), String> {
    // Stored 0, 27, 34 and 200. Rescale slope 1, intercept 0. The full range
    // is center 100 and width 200.
    // PS3.3 C.11.2.1.2 LINEAR uses adjusted center 99.5 and adjusted width 199.
    // Stored 27: ((27 - 99.5) / 199 + 0.5) * 255 = 34.59799 in f32.
    // display_index adds 0.5, getting 35.09799, and keeps the greatest
    // integer at or below that sum, which is grey 35.
    // PS3.3 C.11.2.1.3.2 LINEAR_EXACT uses center 100 and width 200.
    // Stored 27: ((27 - 100) / 200 + 0.5) * 255 = 34.425 in f32.
    // display_index adds 0.5, getting 34.925, and the greatest integer at or
    // below that sum is grey 34.
    // Stored 34, LINEAR: ((34 - 99.5) / 199 + 0.5) * 255 = 43.56784 in f32.
    // display_index adds 0.5, getting 44.06784, and the grey is 44.
    // Stored 34, LINEAR_EXACT: ((34 - 100) / 200 + 0.5) * 255 = 43.35 in f32.
    // display_index adds 0.5, getting 43.85, and the grey is 43.
    // Stored 0 is on the lower edge, so both functions give grey 0.
    // LINEAR's upper edge is 199, so stored 200 is grey 255.
    // LINEAR_EXACT's upper edge is 200. Stored 200 is not above it.
    // ((200 - 100) / 200 + 0.5) * 255 = 255, so grey 255.
    // Stored 50 at this same window is grey 64 for both functions.
    let file = span_frame()?;
    let addr = viewer_on(8 * 1024 * 1024)?;
    let posted = exchange(addr, &http_post_to("/file", "", &file))?;
    let (header, _) = split_http(&posted)?;
    let id =
        header_line(header, "X-Ocelli-File").ok_or_else(|| "the file id was missing".to_owned())?;
    let linear = exchange(
        addr,
        &preview_id(id, "X-Ocelli-Window-Function: LINEAR\r\n"),
    )?;
    let (linear_header, linear_body) = split_http(&linear)?;
    if !linear_header.contains("X-Ocelli-Window-Function: LINEAR")
        || !linear_header.contains("X-Ocelli-Full-Range: 1")
        || !linear_header.contains("X-Ocelli-Window-Center: 100")
        || !linear_header.contains("X-Ocelli-Window-Width: 200")
        || linear_body != [0, 35, 44, 255]
    {
        return Err(format!(
            "LINEAR alone was not grey 0, 35, 44, 255: {linear_header} body {linear_body:?}"
        ));
    }
    let exact = exchange(addr, &preview_id(id, ""))?;
    let (exact_header, exact_body) = split_http(&exact)?;
    if !exact_header.contains("X-Ocelli-Window-Function: LINEAR_EXACT")
        || !exact_header.contains("X-Ocelli-Full-Range: 1")
        || !exact_header.contains("X-Ocelli-Window-Center: 100")
        || !exact_header.contains("X-Ocelli-Window-Width: 200")
        || exact_body != [0, 34, 43, 255]
    {
        return Err(format!(
            "the default full range was not grey 0, 34, 43, 255: {exact_header} body {exact_body:?}"
        ));
    }
    if linear_body == exact_body {
        return Err(format!(
            "LINEAR and LINEAR_EXACT returned the same body: {linear_body:?}"
        ));
    }
    Ok(())
}

#[test]
fn a_busy_post_is_refused_before_its_body() -> Result<(), String> {
    let addr = viewer_capped(1024, 1)?;
    let mut first = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    first
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let opened = read_one(&mut first)?;
    let (opened_header, _) = split_http(&opened)?;
    if !opened_header.contains("200") {
        return Err("the first connection was not served".to_owned());
    }
    let mut second = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    second
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    second
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let started = std::time::Instant::now();
    second
        .write_all(b"POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2000000\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let response = read_one(&mut second)?;
    if started.elapsed() > Duration::from_secs(2) {
        return Err("the busy reply waited for the body".to_owned());
    }
    let (header, body) = split_http(&response)?;
    if !header.contains("503")
        || !header.contains("X-Ocelli-Error: busy")
        || !header.contains("Connection: close")
        || body != b"this process is already serving its connection limit"
    {
        return Err("a header-only post over the cap was not busy".to_owned());
    }
    Ok(())
}

#[test]
fn a_busy_post_with_a_body_is_still_readable() -> Result<(), String> {
    // requestGrey retries only when the status is 503, X-Ocelli-Error is
    // busy, and the body can be read. A reset never reaches that branch.
    let addr = viewer_capped(1024, 1)?;
    let mut first = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    first
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    first
        .write_all(b"GET /synthetic HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .map_err(|error| error.to_string())?;
    let opened = read_one(&mut first)?;
    let (opened_header, _) = split_http(&opened)?;
    if !opened_header.contains("200") {
        return Err("the first connection was not served".to_owned());
    }
    let mut second = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    second
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    second
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    let payload = vec![0xA5_u8; 32 * 1024];
    let mut request = format!(
        "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    )
    .into_bytes();
    request.extend_from_slice(&payload);
    let started = std::time::Instant::now();
    second
        .write_all(&request)
        .map_err(|error| error.to_string())?;
    let response = read_one(&mut second)?;
    if started.elapsed() > Duration::from_secs(2) {
        return Err("the busy reply waited on the whole declared body".to_owned());
    }
    let (header, body) = split_http(&response)?;
    if !header.contains("503")
        || !header.contains("X-Ocelli-Error: busy")
        || !header.contains("Connection: close")
        || body != b"this process is already serving its connection limit"
    {
        return Err(format!(
            "a posted body over the cap was not a readable 503: {header} body {body:?}"
        ));
    }
    // The payload is 32 KiB, under BUSY_DRAIN_CAP. The 503 must be followed by a clean close.
    // A body above the cap is outside this assertion.
    let mut extra = [0_u8; 1];
    match second.read(&mut extra) {
        Ok(0) => {}
        Ok(n) => return Err(format!("bytes followed the 503: {n}")),
        Err(error) => return Err(format!("the 503 was reset: {error}")),
    }
    Ok(())
}

#[test]
fn the_page_names_the_release_build() -> Result<(), String> {
    let response = one_exchange(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")?;
    let (header, body) = split_http(&response)?;
    if !header.contains("200") {
        return Err("the page response was not 200".to_owned());
    }
    let page = std::str::from_utf8(body).map_err(|_| "the page is not utf-8".to_owned())?;
    if !page.contains("cargo run --release -p ocelli-folder-viewer") {
        return Err("the page does not name the release build".to_owned());
    }
    if !page.contains("This is the slow debug build.") {
        return Err("the page does not name the debug build".to_owned());
    }
    Ok(())
}
