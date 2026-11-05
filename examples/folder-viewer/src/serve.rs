//! Loopback page. The browser chooses the folder and posts each file here.
//!
//! The listener binds `127.0.0.1` and prints only its URL. A request body is
//! not written to a log, and a response does not repeat a path. One connection
//! may carry several requests. Bytes past one body stay available for the next.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ocelli_pixel::VoiFunction;

use crate::held::{FileId, PreviewSource, Viewer, ViewerConfig};
use crate::preview::{
    OwnedDecode, decode_held, decode_owned, decode_ticks, note_lookup, note_write, phases,
    profiling, render_held, reset_phases, validate_frame_request,
};
use crate::{
    FrameRequest, Preview, PreviewError, SLICES, headers_json, inspect_json, preview_frame, slice,
    window_function_from_token, window_preset_from_token,
};

const HEADER_CAP: usize = 16 * 1024;
/// One posted file. A multi-frame object arrives whole, so the cap is one
/// gibibyte rather than a single decoded frame.
const BODY_CAP: usize = 1024 * 1024 * 1024;
const PAGE: &str = include_str!("page.html");
const MORE: &str = include_str!("more.html");
const USE_PAGE: &str = include_str!("use.html");
const READ_TIMEOUT: Duration = Duration::from_secs(15);
const WRITE_TIMEOUT: Duration = Duration::from_secs(60);

struct Request {
    method: String,
    path: String,
    body: Vec<u8>,
    frame: FrameRequest,
    file_id: Option<FileId>,
    close: bool,
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
    row_spacing: Option<String>,
    column_spacing: Option<String>,
    spacing_source: &'static str,
    edge_left: String,
    edge_right: String,
    edge_top: String,
    edge_bottom: String,
}

struct Reply {
    status: u16,
    content_type: &'static str,
    error: Option<&'static str>,
    grey: Option<Box<Grey>>,
    body: Vec<u8>,
    source: Option<&'static str>,
    file_id: Option<String>,
    held_flag: Option<u8>,
}

enum ReadStop {
    Io(std::io::Error),
    Reply(Reply),
    CleanClose,
}

/// Bind `127.0.0.1` and serve until the process stops.
///
/// The port is ephemeral unless `OCELLI_PORT` is set, so a fixed address can
/// be bookmarked and a second copy fails to bind instead of starting beside
/// the first. A set value that is not a port number, empty included, is
/// refused rather than read as unset. The cache budget is `OCELLI_CACHE_MIB`,
/// or 256 MiB when that variable is unset.
///
/// # Errors
///
/// Returns the listener error when the socket cannot be opened, and an
/// `InvalidInput` error when `OCELLI_PORT` or `OCELLI_CACHE_MIB` is not usable.
pub fn serve() -> std::io::Result<()> {
    let config = ViewerConfig::from_env()?;
    let listener = TcpListener::bind(("127.0.0.1", chosen_port()?))?;
    let addr = listener.local_addr()?;
    println!("http://{addr}/");
    std::io::stdout().flush()?;
    serve_on(listener, Arc::new(Viewer::new(config)))
}

/// Accept connections on an existing listener. Does not print.
///
/// # Errors
///
/// Returns the listener error when accepting fails hard enough to end the loop.
/// A single failed accept is skipped.
pub fn serve_on(listener: TcpListener, viewer: Arc<Viewer>) -> std::io::Result<()> {
    for incoming in listener.incoming() {
        let Ok(stream) = incoming else {
            continue;
        };
        let viewer = Arc::clone(&viewer);
        std::thread::spawn(move || {
            let _ = handle_connection_with(stream, &viewer);
        });
    }
    Ok(())
}

fn chosen_port() -> std::io::Result<u16> {
    match std::env::var("OCELLI_PORT") {
        Ok(text) => text.trim().parse::<u16>().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "OCELLI_PORT is not a port number",
            )
        }),
        Err(std::env::VarError::NotPresent) => Ok(0),
        Err(std::env::VarError::NotUnicode(_)) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "OCELLI_PORT is not a port number",
        )),
    }
}

/// Read one request and write one response, then close.
///
/// The cache budget is zero, so nothing is held. Callers that already shut
/// the write half down still observe one response.
///
/// # Errors
///
/// Returns an I/O error when the socket fails before a response is written.
pub fn handle_connection(stream: TcpStream) -> std::io::Result<()> {
    let viewer = Viewer::new(ViewerConfig { cache_bytes: 0 });
    connection_session(stream, &viewer, Duration::from_secs(60), true)
}

/// Serve requests on one connection until it closes or sits idle.
///
/// A connection past the cap gets 503 and is closed without taking a slot.
///
/// # Errors
///
/// Returns an I/O error when the socket fails before a response is written.
/// A clean end of stream after a response is success.
pub fn handle_connection_with(mut stream: TcpStream, viewer: &Viewer) -> std::io::Result<()> {
    if !viewer.acquire_connection() {
        let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
        let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
        let mut pending = Vec::new();
        // Headers only. Reading a declared body here would reserve it on a
        // connection that is already over the cap.
        let got_headers = match read_header_bytes(&mut stream, &mut pending) {
            Ok(_) | Err(ReadStop::Reply(_)) => true,
            Err(ReadStop::CleanClose) | Err(ReadStop::Io(_)) => false,
        };
        if got_headers {
            let reply = busy_reply();
            let _ = write_reply(&mut stream, &reply, false);
        }
        return Ok(());
    }
    let _guard = ConnectionGuard { viewer };
    connection_session(stream, viewer, READ_TIMEOUT, false)
}

struct ConnectionGuard<'a> {
    viewer: &'a Viewer,
}

impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) {
        self.viewer.release_connection();
    }
}

fn connection_session(
    mut stream: TcpStream,
    viewer: &Viewer,
    read_timeout: Duration,
    one_request: bool,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(read_timeout))?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    let mut pending = Vec::new();
    let mut handled = 0_usize;
    loop {
        let request = match read_request(&mut stream, &mut pending, handled, viewer) {
            Ok(request) => request,
            Err(ReadStop::CleanClose) => return Ok(()),
            Err(ReadStop::Io(error)) => return Err(error),
            Err(ReadStop::Reply(reply)) => {
                write_reply(&mut stream, &reply, false)?;
                return Ok(());
            }
        };
        let close = one_request || request.close;
        if profiling() {
            reset_phases();
        }
        let reply = route(viewer, request, Some(&stream));
        let write_started = profiling().then(Instant::now);
        let recycle = reply.grey.is_some();
        let written = write_reply(&mut stream, &reply, !close);
        if recycle {
            viewer.recycle_grey(reply.body);
        }
        written?;
        if let Some(started) = write_started {
            note_write(started.elapsed().as_micros());
            log_phases();
        }
        handled = handled.saturating_add(1);
        if close {
            return Ok(());
        }
    }
}

fn route(viewer: &Viewer, request: Request, stream: Option<&TcpStream>) -> Reply {
    if request.method == "POST" && request.body.len() > BODY_CAP {
        return text_reply(400, Some("body"), "the preview request is too large");
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", _) => get_reply(viewer, &request),
        ("POST", "/preview") => preview_route(viewer, &request, stream),
        ("POST", "/file") => file_route(viewer, request),
        ("POST", "/inspect") => json_or_error(inspect_json(&request.body, request.frame.frame)),
        ("POST", "/headers") => headers_route(viewer, &request),
        _ => text_reply(404, None, "not found"),
    }
}

fn get_reply(viewer: &Viewer, request: &Request) -> Reply {
    if request.path == "/held" {
        return held_reply(viewer);
    }
    if request.path == "/" || request.path == "/index.html" {
        return page_reply();
    }
    if request.path == "/more" || request.path == "/more.html" {
        return html_page(MORE);
    }
    if request.path == "/use" || request.path == "/use.html" {
        return html_page(USE_PAGE);
    }
    if request.path == "/synthetic" {
        let body = format!("{{\"slices\":{SLICES}}}");
        return bare(200, "application/json", body.into_bytes());
    }
    if let Some(rest) = request.path.strip_prefix("/synthetic/")
        && let Ok(index) = rest.parse::<u32>()
        && let Some(bytes) = slice(index)
    {
        return bare(200, "application/octet-stream", bytes);
    }
    text_reply(404, None, "not found")
}

fn held_reply(viewer: &Viewer) -> Reply {
    let (pressure, files, frames) = viewer.census();
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let body = format!(
        "{{\"build\":\"{build}\",\"budget\":{budget},\"used\":{used},\"files\":{files},\"frames\":{frames}}}",
        budget = pressure.budget,
        used = pressure.used,
    );
    bare(200, "application/json", body.into_bytes())
}

fn page_reply() -> Reply {
    html_page(PAGE)
}

fn html_page(page: &str) -> Reply {
    bare(200, "text/html; charset=utf-8", page.as_bytes().to_vec())
}

fn preview_route(viewer: &Viewer, request: &Request, stream: Option<&TcpStream>) -> Reply {
    if request.file_id.is_some() && !request.body.is_empty() {
        return text_reply(
            400,
            Some("header"),
            "a file id and a body cannot be sent together",
        );
    }
    if let Some(id) = request.file_id {
        return preview_held(viewer, id, request.frame, stream);
    }
    let mut reply = from_preview(preview_frame(&request.body, request.frame));
    if reply.status == 200 {
        reply.source = Some("body");
    }
    reply
}

fn busy_reply() -> Reply {
    text_reply(
        503,
        Some("busy"),
        "this process is already serving its connection limit",
    )
}

fn peer_closed(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
    )
}

/// Whether a thread that does not yet hold a permit should keep waiting.
///
/// A reset, an abort, or a read of no bytes means the peer has gone. The
/// caller asks only while the permit pool is empty. A request that already
/// holds a permit is still answered, including after the client finished
/// its write.
fn peer_open(stream: &TcpStream) -> bool {
    if matches!(stream.take_error(), Ok(Some(error)) if peer_closed(&error)) {
        return false;
    }
    let timeout = stream.read_timeout().ok().flatten();
    if stream.set_nonblocking(true).is_err() {
        return true;
    }
    let mut probe = [0_u8; 1];
    let open = match stream.peek(&mut probe) {
        Ok(0) => false,
        Err(error) if peer_closed(&error) => false,
        Ok(_) | Err(_) => true,
    };
    let _ = stream.set_nonblocking(false);
    if let Some(timeout) = timeout {
        let _ = stream.set_read_timeout(Some(timeout));
    }
    open
}

fn preview_held(
    viewer: &Viewer,
    id: FileId,
    request: FrameRequest,
    stream: Option<&TcpStream>,
) -> Reply {
    if let Err(error) = validate_frame_request(request) {
        return error_reply(error);
    }
    let before = decode_ticks();
    let lookup_started = profiling().then(Instant::now);
    let found = viewer.preview_source(id, request.frame);
    if let Some(started) = lookup_started {
        note_lookup(started.elapsed().as_micros());
    }
    // A free permit serves the request already read, including a client that
    // has finished sending. A thread waits for a permit only while that client
    // is still there, so a closed peer does not keep the connection slot.
    let mut live = || stream.is_none_or(peer_open);
    let Some(mut loan) = viewer.loan_workspace(&mut live) else {
        if let PreviewSource::Owned(bytes) = found {
            viewer.restore_file(id, bytes);
        }
        return busy_reply();
    };
    let workspace = &mut loan.workspace;
    let preview = match found {
        PreviewSource::Gone => return gone(),
        PreviewSource::Hit(held) => {
            render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
        }
        PreviewSource::Shared(bytes) => {
            match decode_held(&bytes, request.frame, &mut workspace.native) {
                Ok(held) => {
                    let held = viewer.store_frame(id, request.frame, held);
                    render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
                }
                Err(error) => Err(error),
            }
        }
        PreviewSource::Owned(bytes) => match decode_owned(bytes, request.frame) {
            OwnedDecode::Frame(held) => {
                let held = viewer.store_frame(id, request.frame, *held);
                render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
            }
            OwnedDecode::Keep(bytes) => {
                match decode_held(&bytes, request.frame, &mut workspace.native) {
                    Ok(held) => {
                        if held.frames > 1 {
                            viewer.restore_file(id, bytes);
                        } else {
                            viewer.recycle_buffer(bytes);
                        }
                        let held = viewer.store_frame(id, request.frame, held);
                        render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
                    }
                    Err(error) => {
                        viewer.restore_file(id, bytes);
                        Err(error)
                    }
                }
            }
        },
    };
    let mut reply = from_preview(preview);
    if reply.status == 200 {
        reply.source = Some(if decode_ticks() == before {
            "held"
        } else {
            "decoded"
        });
    }
    reply
}

fn file_route(viewer: &Viewer, request: Request) -> Reply {
    match inspect_json(&request.body, request.frame.frame) {
        Ok(json) => {
            let mut reply = bare(200, "application/json; charset=utf-8", json.into_bytes());
            match viewer.insert_file(request.body) {
                Some(id) => {
                    reply.file_id = Some(id.header_value());
                    reply.held_flag = Some(1);
                }
                None => reply.held_flag = Some(0),
            }
            reply
        }
        Err(error) => error_reply(error),
    }
}

fn headers_route(viewer: &Viewer, request: &Request) -> Reply {
    if request.file_id.is_some() && !request.body.is_empty() {
        return text_reply(
            400,
            Some("header"),
            "a file id and a body cannot be sent together",
        );
    }
    if let Some(id) = request.file_id {
        return match viewer.file_bytes(id) {
            Some(bytes) => json_or_error(headers_json(bytes.as_ref())),
            None => gone(),
        };
    }
    json_or_error(headers_json(&request.body))
}

fn from_preview(result: Result<Preview, PreviewError>) -> Reply {
    match result {
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
                row_spacing: preview.facts.row_spacing,
                column_spacing: preview.facts.column_spacing,
                spacing_source: preview.facts.spacing_source,
                edge_left: preview.facts.edge_left,
                edge_right: preview.facts.edge_right,
                edge_top: preview.facts.edge_top,
                edge_bottom: preview.facts.edge_bottom,
            })),
            body: preview.grey,
            source: None,
            file_id: None,
            held_flag: None,
        },
        Err(error) => error_reply(error),
    }
}

fn error_reply(error: PreviewError) -> Reply {
    let message = error.to_string();
    text_reply(400, Some(error.code()), &message)
}

fn gone() -> Reply {
    text_reply(
        410,
        Some("file-gone"),
        "the file is no longer held by this process",
    )
}

fn json_or_error(result: Result<String, PreviewError>) -> Reply {
    match result {
        Ok(json) => bare(200, "application/json; charset=utf-8", json.into_bytes()),
        Err(error) => error_reply(error),
    }
}

fn bare(status: u16, content_type: &'static str, body: Vec<u8>) -> Reply {
    Reply {
        status,
        content_type,
        error: None,
        grey: None,
        body,
        source: None,
        file_id: None,
        held_flag: None,
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
        410 => "Gone",
        503 => "Service Unavailable",
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

fn log_phases() {
    let phase = phases();
    if phase.pixels == 0 {
        return;
    }
    eprintln!(
        "profile lookup_us={} unpack_us={} chain_us={} voi_us={} grey_us={} fused_us={} write_us={} pixels={}",
        phase.lookup_us,
        phase.unpack_us,
        phase.chain_us,
        phase.voi_us,
        phase.grey_us,
        phase.fused_us,
        phase.write_us,
        phase.pixels,
    );
}

fn write_reply(stream: &mut impl Write, reply: &Reply, keep_alive: bool) -> std::io::Result<()> {
    let connection = if keep_alive { "keep-alive" } else { "close" };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {len}\r\nCache-Control: no-store\r\nConnection: {connection}\r\n",
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
    if let Some(source) = reply.source {
        head.push_str("X-Ocelli-Source: ");
        head.push_str(source);
        head.push_str("\r\n");
    }
    if let Some(id) = &reply.file_id {
        head.push_str("X-Ocelli-File: ");
        head.push_str(id);
        head.push_str("\r\n");
    }
    if let Some(flag) = reply.held_flag {
        head.push_str("X-Ocelli-Held: ");
        head.push_str(&flag.to_string());
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
        if let Some(row) = &grey.row_spacing {
            head.push_str("X-Ocelli-Row-Spacing: ");
            head.push_str(row);
            head.push_str("\r\n");
        }
        if let Some(column) = &grey.column_spacing {
            head.push_str("X-Ocelli-Column-Spacing: ");
            head.push_str(column);
            head.push_str("\r\n");
        }
        head.push_str("X-Ocelli-Spacing-Source: ");
        head.push_str(grey.spacing_source);
        head.push_str("\r\n");
        head.push_str("X-Ocelli-Edge-Left: ");
        head.push_str(&grey.edge_left);
        head.push_str("\r\nX-Ocelli-Edge-Right: ");
        head.push_str(&grey.edge_right);
        head.push_str("\r\nX-Ocelli-Edge-Top: ");
        head.push_str(&grey.edge_top);
        head.push_str("\r\nX-Ocelli-Edge-Bottom: ");
        head.push_str(&grey.edge_bottom);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(&reply.body)?;
    Ok(())
}

fn read_header_bytes(stream: &mut TcpStream, pending: &mut Vec<u8>) -> Result<usize, ReadStop> {
    read_header_bytes_after(stream, pending, 0)
}

fn read_header_bytes_after(
    stream: &mut TcpStream,
    pending: &mut Vec<u8>,
    handled: usize,
) -> Result<usize, ReadStop> {
    let mut chunk = [0_u8; 8192];
    loop {
        if let Some(end) = header_end_at(pending) {
            return Ok(end);
        }
        if pending.len() > HEADER_CAP {
            pending.clear();
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("header"),
                "the preview request header is too large",
            )));
        }
        let n = stream.read(&mut chunk).map_err(ReadStop::Io)?;
        if n == 0 {
            if pending.is_empty() && handled > 0 {
                return Err(ReadStop::CleanClose);
            }
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
        pending.extend_from_slice(got);
    }
}

fn read_request(
    stream: &mut TcpStream,
    pending: &mut Vec<u8>,
    handled: usize,
    viewer: &Viewer,
) -> Result<Request, ReadStop> {
    let header_end = read_header_bytes_after(stream, pending, handled)?;
    let header = {
        let Some(header_bytes) = pending.get(..header_end) else {
            return Err(ReadStop::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "header",
            )));
        };
        std::str::from_utf8(header_bytes)
            .map_err(|_| {
                ReadStop::Reply(text_reply(
                    400,
                    Some("header"),
                    "the preview request header is not text",
                ))
            })?
            .to_owned()
    };
    let parsed = parse_header(&header).map_err(ReadStop::Reply)?;
    let declared = match parsed.length {
        Some(value) if value > BODY_CAP => {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("body"),
                "the preview request is too large",
            )));
        }
        Some(value) => value,
        None if parsed.method == "POST" => {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("length"),
                "the preview request has no length",
            )));
        }
        None => 0,
    };
    let body = fill_body(stream, pending, header_end, declared, |need| {
        viewer.take_buffer(need)
    })?;
    Ok(Request {
        method: parsed.method,
        path: parsed.path,
        body,
        frame: parsed.frame,
        file_id: parsed.file_id,
        close: parsed.close,
    })
}

fn fill_body<R: Read>(
    stream: &mut R,
    pending: &mut Vec<u8>,
    header_end: usize,
    declared: usize,
    allocate: impl FnOnce(usize) -> Vec<u8>,
) -> Result<Vec<u8>, ReadStop> {
    let tail = if pending.len() > header_end {
        pending.split_off(header_end)
    } else {
        Vec::new()
    };
    release_read_buffer(pending);
    if tail.len() >= declared {
        let mut incoming = tail;
        let surplus = incoming.split_off(declared);
        let mut body = allocate(declared);
        body.extend_from_slice(&incoming);
        if !surplus.is_empty() {
            *pending = surplus;
        }
        return Ok(body);
    }
    let mut body = allocate(declared);
    body.extend_from_slice(&tail);
    let mut chunk = [0_u8; 8192];
    while body.len() < declared {
        let n = stream.read(&mut chunk).map_err(ReadStop::Io)?;
        if n == 0 {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("length"),
                "the preview request ended early",
            )));
        }
        let Some(got) = chunk.get(..n) else {
            return Err(ReadStop::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "read",
            )));
        };
        let need = declared - body.len();
        if got.len() <= need {
            body.extend_from_slice(got);
        } else {
            let Some(take) = got.get(..need) else {
                return Err(ReadStop::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "read",
                )));
            };
            let Some(extra) = got.get(need..) else {
                return Err(ReadStop::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "read",
                )));
            };
            body.extend_from_slice(take);
            *pending = extra.to_vec();
            return Ok(body);
        }
    }
    Ok(body)
}

/// One allocation of `declared` bytes. A doubled growth trail stays resident
/// after the body is freed, because this host does not return large pages.
#[cfg(test)]
fn exact_body(declared: usize) -> Vec<u8> {
    let mut body = Vec::new();
    body.reserve_exact(declared);
    body
}

/// Drop a connection buffer that has kept a previous body.
fn release_read_buffer(pending: &mut Vec<u8>) {
    if pending.capacity() > HEADER_CAP {
        *pending = Vec::new();
    } else {
        pending.clear();
    }
}

struct HeaderParts {
    method: String,
    path: String,
    length: Option<usize>,
    frame: FrameRequest,
    file_id: Option<FileId>,
    close: bool,
}

fn parse_header(header: &str) -> Result<HeaderParts, Reply> {
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
    let version = parts.next().unwrap_or("");
    let mut close = version != "HTTP/1.1";
    let mut length = None;
    let mut seen_length = false;
    let mut seen_frame = false;
    let mut seen_invert = false;
    let mut seen_center = false;
    let mut seen_width = false;
    let mut seen_function = false;
    let mut seen_preset = false;
    let mut seen_file = false;
    let mut frame = FrameRequest::default();
    let mut file_id = None;
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
        } else if name.eq_ignore_ascii_case("connection") {
            if connection_closes(text) {
                close = true;
            }
        } else if name.eq_ignore_ascii_case("x-ocelli-file") {
            once(&mut seen_file)?;
            file_id = Some(FileId::parse(text).ok_or_else(header_unusable)?);
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
        } else if name.eq_ignore_ascii_case("x-ocelli-window-preset") {
            once(&mut seen_preset)?;
            frame.window_preset =
                Some(window_preset_from_token(text).map_err(|_| window_unusable())?);
        }
    }
    Ok(HeaderParts {
        method: method.to_owned(),
        path: path.to_owned(),
        length,
        frame,
        file_id,
        close,
    })
}

fn connection_closes(value: &str) -> bool {
    value
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case("close"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_body_is_held_at_its_declared_length() -> Result<(), &'static str> {
        // 100003 is not a power of two. Doubling the read buffer would keep
        // 131072 bytes, and that spare capacity would stay in a held file.
        let declared = 100_003_usize;
        let mut stream = std::io::Cursor::new(vec![9_u8; declared]);
        let mut pending = Vec::new();
        let body = fill_body(&mut stream, &mut pending, 0, declared, exact_body)
            .map_err(|_| "the declared body could not be read")?;
        if body.len() != declared {
            return Err("the request body length was not the declared length");
        }
        if body.capacity() != declared {
            return Err("the request body kept spare capacity");
        }
        if pending.capacity() > HEADER_CAP {
            return Err("the connection buffer kept the body");
        }
        Ok(())
    }

    #[test]
    fn a_second_preview_does_not_keep_a_second_byte_buffer() -> Result<(), String> {
        let bytes = slice(0).ok_or("synthetic slice 0 was absent")?;
        let viewer = Viewer::new(ViewerConfig {
            cache_bytes: 8 * 1024 * 1024,
        });
        let id = viewer
            .insert_file(bytes)
            .ok_or("the synthetic slice was not held")?;
        let first = preview_held(&viewer, id, FrameRequest::default(), None);
        if first.status != 200 {
            return Err(format!("the first preview status was {}", first.status));
        }
        let pixels = first.body.len();
        let pixel_bytes = pixels
            .checked_mul(2)
            .ok_or("pixel byte length overflowed")?;
        // Little-endian words are borrowed. A copy would keep a second frame.
        let expected_native = if cfg!(target_endian = "little") {
            0
        } else {
            pixel_bytes
        };
        viewer.recycle_grey(first.body);
        let (native, grey) = viewer.byte_buffer_capacities();
        if native != expected_native {
            return Err(format!(
                "the native buffer capacity was {native} for {pixel_bytes} pixel bytes"
            ));
        }
        if grey != pixels {
            return Err(format!(
                "the grey buffer capacity was {grey} for {pixels} pixels"
            ));
        }
        let second = preview_held(&viewer, id, FrameRequest::default(), None);
        if second.status != 200 {
            return Err(format!("the second preview status was {}", second.status));
        }
        if second.body.len() != pixels {
            return Err(format!(
                "the second preview length was {} for {pixels} pixels",
                second.body.len()
            ));
        }
        viewer.recycle_grey(second.body);
        let (native_again, grey_again) = viewer.byte_buffer_capacities();
        if native_again != expected_native {
            return Err(format!(
                "the second preview left native capacity {native_again}"
            ));
        }
        if grey_again != pixels {
            return Err(format!(
                "the second preview left grey capacity {grey_again}"
            ));
        }
        Ok(())
    }
}
