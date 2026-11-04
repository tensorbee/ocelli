//! Loopback page. The browser chooses the folder and posts each file here.
//!
//! The listener binds `127.0.0.1` and prints only its URL. A request body is
//! not written to a log, and a response does not repeat a path.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use ocelli_pixel::VoiFunction;

use crate::{FrameRequest, headers_json, inspect_json, preview_frame, window_function_from_token};

const HEADER_CAP: usize = 16 * 1024;
/// One posted file. A multi-frame object arrives whole, so the cap is one
/// gibibyte rather than a single decoded frame.
const BODY_CAP: usize = 1024 * 1024 * 1024;
const PAGE: &str = include_str!("page.html");

struct Request {
    method: String,
    path: String,
    body: Vec<u8>,
    frame: FrameRequest,
}

struct Grey {
    width: u32,
    height: u32,
    frames: u32,
    full_range: bool,
    shown_frame: u32,
    window_center: f32,
    window_width: f32,
    window_function: &'static str,
    inverted: bool,
}

struct Reply {
    status: u16,
    content_type: &'static str,
    error: Option<&'static str>,
    grey: Option<Box<Grey>>,
    body: Vec<u8>,
}

enum ReadStop {
    Io(std::io::Error),
    Reply(Reply),
}

/// Bind an ephemeral port on `127.0.0.1` and serve until the process stops.
///
/// # Errors
///
/// Returns the listener error when the socket cannot be opened.
pub fn serve() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    println!("http://{addr}/");
    std::io::stdout().flush()?;
    for incoming in listener.incoming() {
        let Ok(stream) = incoming else {
            continue;
        };
        std::thread::spawn(move || {
            let _ = handle_connection(stream);
        });
    }
    Ok(())
}

/// Read one request and write one response.
///
/// # Errors
///
/// Returns an I/O error when the socket fails before a response is written.
pub fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(60)))?;
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(ReadStop::Io(error)) => return Err(error),
        Err(ReadStop::Reply(reply)) => return write_reply(&mut stream, &reply),
    };
    write_reply(&mut stream, &route(&request))
}

fn route(request: &Request) -> Reply {
    if request.method == "POST" && request.body.len() > BODY_CAP {
        return text_reply(400, Some("body"), "the preview request is too large");
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/" | "/index.html") => page_reply(),
        ("POST", "/preview") => preview_reply(&request.body, request.frame),
        ("POST", "/inspect") => json_or_error(inspect_json(&request.body)),
        ("POST", "/headers") => json_or_error(headers_json(&request.body)),
        _ => text_reply(404, None, "not found"),
    }
}

fn page_reply() -> Reply {
    bare(200, "text/html; charset=utf-8", PAGE.as_bytes().to_vec())
}

fn preview_reply(body: &[u8], frame: FrameRequest) -> Reply {
    match preview_frame(body, frame) {
        Ok(preview) => Reply {
            status: 200,
            content_type: "application/octet-stream",
            error: None,
            grey: Some(Box::new(Grey {
                width: preview.width,
                height: preview.height,
                frames: preview.frames,
                full_range: preview.full_range,
                shown_frame: preview.frame,
                window_center: preview.window_center,
                window_width: preview.window_width,
                window_function: preview.window_function,
                inverted: preview.inverted,
            })),
            body: preview.grey,
        },
        Err(error) => {
            let message = error.to_string();
            text_reply(400, Some(error.code()), &message)
        }
    }
}

fn json_or_error(result: Result<String, crate::PreviewError>) -> Reply {
    match result {
        Ok(json) => bare(200, "application/json; charset=utf-8", json.into_bytes()),
        Err(error) => {
            let message = error.to_string();
            text_reply(400, Some(error.code()), &message)
        }
    }
}

fn bare(status: u16, content_type: &'static str, body: Vec<u8>) -> Reply {
    Reply {
        status,
        content_type,
        error: None,
        grey: None,
        body,
    }
}

fn text_reply(status: u16, error: Option<&'static str>, message: &str) -> Reply {
    let mut reply = bare(
        status,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
    );
    reply.error = error;
    reply
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    }
}

fn format_window(value: f32) -> String {
    if !value.is_finite() {
        return String::new();
    }
    let nearest = value.round();
    let delta = (value - nearest).abs();
    if value.to_bits() == nearest.to_bits() || delta.to_bits() == 0.0_f32.to_bits() {
        format!("{nearest:.0}")
    } else {
        format!("{value}")
    }
}

fn write_reply(stream: &mut TcpStream, reply: &Reply) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {len}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        status = reply.status,
        reason = reason(reply.status),
        ctype = reply.content_type,
        len = reply.body.len(),
    );
    if let Some(code) = reply.error {
        head.push_str("X-Ocelli-Error: ");
        head.push_str(code);
        head.push_str("\r\n");
    }
    if let Some(grey) = reply.grey.as_deref() {
        let range = u8::from(grey.full_range);
        let center = format_window(grey.window_center);
        let width = format_window(grey.window_width);
        let flag = u8::from(grey.inverted);
        head.push_str(&format!(
            "X-Ocelli-Width: {width_px}\r\nX-Ocelli-Height: {height}\r\nX-Ocelli-Frames: {frames}\r\nX-Ocelli-Full-Range: {range}\r\nX-Ocelli-Frame: {frame}\r\nX-Ocelli-Window-Center: {center}\r\nX-Ocelli-Window-Width: {width}\r\nX-Ocelli-Window-Function: {function}\r\nX-Ocelli-Invert: {flag}\r\n",
            width_px = grey.width,
            height = grey.height,
            frames = grey.frames,
            frame = grey.shown_frame,
            function = grey.window_function,
        ));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(&reply.body)?;
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> Result<Request, ReadStop> {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 8192];
    let header_end = loop {
        let n = stream.read(&mut chunk).map_err(ReadStop::Io)?;
        if n == 0 {
            return Err(ReadStop::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "closed",
            )));
        }
        let Some(got) = chunk.get(..n) else {
            return Err(ReadStop::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "read",
            )));
        };
        buf.extend_from_slice(got);
        if let Some(end) = header_end_at(&buf) {
            break end;
        }
        if buf.len() > HEADER_CAP {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("header"),
                "the preview request header is too large",
            )));
        }
    };
    let Some(header_bytes) = buf.get(..header_end) else {
        return Err(ReadStop::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "header",
        )));
    };
    let header = std::str::from_utf8(header_bytes).map_err(|_| {
        ReadStop::Reply(text_reply(
            400,
            Some("header"),
            "the preview request header is not text",
        ))
    })?;
    let (method, path, length, frame) = parse_header(header).map_err(ReadStop::Reply)?;
    let declared = match length {
        Some(value) if value > BODY_CAP => {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("body"),
                "the preview request is too large",
            )));
        }
        Some(value) => value,
        None if method == "POST" => {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("length"),
                "the preview request has no length",
            )));
        }
        None => 0,
    };
    let mut body = match buf.get(header_end..) {
        Some(rest) => rest.to_vec(),
        None => Vec::new(),
    };
    if body.len() > declared {
        body.truncate(declared);
    }
    while body.len() < declared {
        let n = stream.read(&mut chunk).map_err(ReadStop::Io)?;
        if n == 0 {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("length"),
                "the preview request ended early",
            )));
        }
        let need = declared - body.len();
        let take = n.min(need);
        let Some(got) = chunk.get(..take) else {
            return Err(ReadStop::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "read",
            )));
        };
        body.extend_from_slice(got);
    }
    Ok(Request {
        method,
        path,
        body,
        frame,
    })
}

fn parse_header(header: &str) -> Result<(String, String, Option<usize>, FrameRequest), Reply> {
    let mut lines = header.split("\r\n");
    let Some(request) = lines.next() else {
        return Err(text_reply(
            400,
            Some("header"),
            "the preview request is empty",
        ));
    };
    let mut parts = request.split(' ');
    let Some(method) = parts.next() else {
        return Err(text_reply(
            400,
            Some("header"),
            "the preview request is empty",
        ));
    };
    let Some(raw_path) = parts.next() else {
        return Err(text_reply(
            400,
            Some("header"),
            "the preview request has no path",
        ));
    };
    if method != "GET" && method != "POST" {
        return Err(text_reply(
            400,
            Some("method"),
            "the preview request method is not supported",
        ));
    }
    let path = match raw_path.split('?').next() {
        Some(bare) => bare,
        None => raw_path,
    };
    let mut length = None;
    let mut seen_length = false;
    let mut seen_frame = false;
    let mut seen_invert = false;
    let mut seen_center = false;
    let mut seen_width = false;
    let mut seen_function = false;
    let mut frame = FrameRequest::default();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(text_reply(
                400,
                Some("header"),
                "the preview request header is not usable",
            ));
        };
        let text = value.trim_matches(' ');
        if name.eq_ignore_ascii_case("content-length") {
            if seen_length {
                return Err(text_reply(
                    400,
                    Some("length"),
                    "the preview request has two lengths",
                ));
            }
            seen_length = true;
            let parsed = text.parse::<usize>().map_err(|_| {
                text_reply(
                    400,
                    Some("length"),
                    "the preview request length is not a number",
                )
            })?;
            length = Some(parsed);
        } else if name.eq_ignore_ascii_case("x-ocelli-frame") {
            once(&mut seen_frame)?;
            frame.frame = text.parse::<u32>().map_err(|_| header_unusable())?;
        } else if name.eq_ignore_ascii_case("x-ocelli-invert") {
            once(&mut seen_invert)?;
            frame.invert = match text {
                "0" => false,
                "1" => true,
                _ => return Err(header_unusable()),
            };
        } else if name.eq_ignore_ascii_case("x-ocelli-window-center") {
            once(&mut seen_center)?;
            frame.window_center = Some(parse_finite(text)?);
        } else if name.eq_ignore_ascii_case("x-ocelli-window-width") {
            once(&mut seen_width)?;
            frame.window_width = Some(parse_finite(text)?);
        } else if name.eq_ignore_ascii_case("x-ocelli-window-function") {
            once(&mut seen_function)?;
            frame.window_function = Some(parse_function(text)?);
        }
    }
    Ok((method.to_owned(), path.to_owned(), length, frame))
}

fn once(seen: &mut bool) -> Result<(), Reply> {
    if *seen {
        return Err(header_unusable());
    }
    *seen = true;
    Ok(())
}

fn header_unusable() -> Reply {
    text_reply(
        400,
        Some("header"),
        "the preview request header is not usable",
    )
}

fn parse_finite(text: &str) -> Result<f32, Reply> {
    let value = text.parse::<f32>().map_err(|_| window_unusable())?;
    if !value.is_finite() {
        return Err(window_unusable());
    }
    Ok(value)
}

fn parse_function(text: &str) -> Result<VoiFunction, Reply> {
    window_function_from_token(text).map_err(|_| window_unusable())
}

fn window_unusable() -> Reply {
    text_reply(
        400,
        Some("window"),
        "the window or presentation could not be applied",
    )
}

fn header_end_at(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| start + 4)
}
