//! Loopback page. The browser chooses the folder and posts each file here.
//!
//! The listener binds `127.0.0.1` and prints only its URL. A request body is
//! not written to a log, and a response does not repeat a path. One connection
//! may carry several requests. Bytes past one body stay available for the next.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use ocelli_pixel::VoiFunction;

use crate::held::{
    BodyPermit, CacheHandle, CachedFrame, FileId, GreyGate, GreyHome, PreviewSource, Viewer,
    ViewerConfig, detach_grey, retire_bytes, with_decode_workspace,
};
use crate::preview::{
    decode_held, decode_ticks, merge_worker_phases, note_lookup, note_write, phases, profiling,
    render_held, reset_phases, validate_frame_request,
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
/// Bytes read and discarded after a 503 so a raw client can finish the response.
///
/// A body above this cap may see a reset after the 503 on a raw client.
/// The page's fetch reads the 503.
const BUSY_DRAIN_CAP: usize = 64 * 1024;
const BUSY_DRAIN_WAIT: Duration = Duration::from_millis(200);

struct Request {
    method: String,
    path: String,
    body: Vec<u8>,
    frame: FrameRequest,
    file_id: Option<FileId>,
    close: bool,
    /// Capacity charged to inflight for this body.
    body_charge: usize,
    /// `enter_body` ran for this request.
    body_counted: bool,
    /// Set when a worker allocated the body.
    body_slot: Option<BodyPermit>,
}

/// Inflight charge for one request body.
///
/// Drop releases whatever is still charged. `release_for_cache` releases it
/// before the bytes become visible in the LRU.
struct BodyCharge<'a> {
    viewer: &'a Viewer,
    bytes: usize,
    /// `enter_body` ran for this request.
    counted: bool,
    /// Worker slot for a pooled body. Drop releases it.
    slot: Option<BodyPermit>,
}

impl BodyCharge<'_> {
    fn release_for_cache(&mut self) {
        if self.bytes == 0 {
            return;
        }
        self.viewer.leave_inflight(self.bytes);
        self.bytes = 0;
    }
}

impl Drop for BodyCharge<'_> {
    fn drop(&mut self) {
        self.release_for_cache();
        if self.counted {
            self.viewer.leave_body();
            self.counted = false;
        }
    }
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
    let viewer = Arc::new(Viewer::server(config));
    start_census_log(&viewer);
    println!("http://{addr}/");
    std::io::stdout().flush()?;
    serve_on(listener, viewer)
}

/// Append a schedule line every 20 ms when `OCELLI_CENSUS_LOG` names a file.
///
/// Diagnostic only. Unset leaves the process unchanged.
fn start_census_log(viewer: &Arc<Viewer>) {
    let Ok(path) = std::env::var("OCELLI_CENSUS_LOG") else {
        return;
    };
    if path.is_empty() {
        return;
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    else {
        return;
    };
    let viewer = Arc::clone(viewer);
    std::thread::spawn(move || {
        let started = Instant::now();
        let _ = file.write_all(
            b"ms decode_threads peak_decodes decodes_now body_threads peak_bodies bodies_now used inflight grey workspace free files frames peak_used peak_inflight peak_workspace peak_files peak_frames\n",
        );
        loop {
            let memory = viewer.memory_census();
            let (decode_threads, peak_decodes, decodes_now) = crate::preview::decode_schedule();
            let (body_threads, peak_bodies, bodies_now) = viewer.body_schedule();
            let (peak_used, peak_inflight, peak_workspace, peak_files, peak_frames) =
                viewer.census_peaks();
            let line = format!(
                "{ms} {decode_threads} {peak_decodes} {decodes_now} {body_threads} {peak_bodies} {bodies_now} {used} {inflight} {grey} {workspace} {free} {files} {frames} {peak_used} {peak_inflight} {peak_workspace} {peak_files} {peak_frames}\n",
                ms = started.elapsed().as_millis(),
                used = memory.pressure.used,
                inflight = memory.inflight,
                grey = memory.grey,
                workspace = memory.workspace,
                free = memory.free,
                files = memory.files,
                frames = memory.frames,
            );
            if file.write_all(line.as_bytes()).is_err() {
                return;
            }
            let _ = file.flush();
            std::thread::sleep(Duration::from_millis(20));
        }
    });
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
        let _ = stream.set_nodelay(true);
        let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
        let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
        let mut pending = Vec::new();
        // Headers only. The declared body is discarded after the 503 is
        // written, and it is never copied into a buffer of its own.
        let mut header_end = None;
        let got_headers = match read_header_bytes(&mut stream, &mut pending) {
            Ok(end) => {
                header_end = Some(end);
                true
            }
            Err(ReadStop::Reply(_)) => true,
            Err(ReadStop::CleanClose) | Err(ReadStop::Io(_)) => false,
        };
        if got_headers {
            let reply = busy_reply();
            let _ = write_reply(&mut stream, &reply, false);
            let _ = stream.flush();
            if let Some(end) = header_end {
                discard_busy_body(&mut stream, &pending, end);
            }
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
        let mut request = request;
        let mut charge = BodyCharge {
            viewer,
            bytes: request.body_charge,
            counted: request.body_counted,
            slot: request.body_slot.take(),
        };
        let (mut reply, home) = route(viewer, request, Some(&stream), &mut charge);
        drop(charge);
        let write_started = profiling().then(Instant::now);
        let written = write_reply(&mut stream, &reply, !close);
        if let Some(home) = home {
            let grey = std::mem::take(&mut reply.body);
            // The reservation was `width * height`, so the release is too.
            let reserved = reply
                .grey
                .as_ref()
                .map_or(grey.len(), |shown| grey_bytes(shown.width, shown.height));
            viewer.release_grey(reserved);
            home.restore_grey(grey);
        } else if reply.grey.is_some() {
            viewer.recycle_grey(std::mem::take(&mut reply.body));
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

fn route(
    viewer: &Viewer,
    request: Request,
    stream: Option<&TcpStream>,
    charge: &mut BodyCharge<'_>,
) -> (Reply, Option<GreyHome>) {
    if request.method == "POST" && request.body.len() > BODY_CAP {
        retire_request_body(charge, request.body);
        return (
            text_reply(400, Some("body"), "the preview request is too large"),
            None,
        );
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", _) => {
            let reply = get_reply(viewer, &request);
            retire_request_body(charge, request.body);
            (reply, None)
        }
        ("POST", "/preview") => preview_route(viewer, request, stream, charge),
        ("POST", "/file") => (file_route(viewer, request, charge), None),
        ("POST", "/inspect") => {
            let reply = json_or_error(inspect_json(&request.body, request.frame.frame));
            retire_request_body(charge, request.body);
            (reply, None)
        }
        ("POST", "/headers") => {
            let reply = headers_route(viewer, &request);
            retire_request_body(charge, request.body);
            (reply, None)
        }
        _ => {
            retire_request_body(charge, request.body);
            (text_reply(404, None, "not found"), None)
        }
    }
}

/// Free a request body on the thread that allocated it.
///
/// A pooled body was allocated by a worker, so its bytes go back there
/// through the retire channel. A body the connection thread allocated is
/// dropped here, which is the same thread.
fn retire_request_body(charge: &BodyCharge<'_>, body: Vec<u8>) {
    match charge.slot.as_ref() {
        Some(slot) => retire_bytes(&slot.retire(), body),
        None => drop(body),
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
    let memory = viewer.memory_census();
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let body = format!(
        "{{\"build\":\"{build}\",\"budget\":{budget},\"used\":{used},\"files\":{files},\"frames\":{frames},\"inflight\":{inflight},\"workspace\":{workspace},\"free\":{free}}}",
        budget = memory.pressure.budget,
        used = memory.pressure.used,
        files = memory.files,
        frames = memory.frames,
        inflight = memory.inflight,
        workspace = memory.workspace,
        free = memory.free,
    );
    bare(200, "application/json", body.into_bytes())
}

fn page_reply() -> Reply {
    html_page(PAGE)
}

fn html_page(page: &str) -> Reply {
    bare(200, "text/html; charset=utf-8", page.as_bytes().to_vec())
}

fn preview_route(
    viewer: &Viewer,
    request: Request,
    stream: Option<&TcpStream>,
    charge: &mut BodyCharge<'_>,
) -> (Reply, Option<GreyHome>) {
    if request.file_id.is_some() && !request.body.is_empty() {
        retire_request_body(charge, request.body);
        return (
            text_reply(
                400,
                Some("header"),
                "a file id and a body cannot be sent together",
            ),
            None,
        );
    }
    if let Some(id) = request.file_id {
        return preview_held(viewer, id, request.frame, stream);
    }
    let body = request.body;
    if !viewer.schedules() {
        // The direct path decodes on the caller, which is the thread that
        // allocated the body here.
        let mut reply = from_preview(preview_frame(&body, request.frame));
        if reply.status == 200 {
            reply.source = Some("body");
        }
        retire_request_body(charge, body);
        return (reply, None);
    }
    // The scheduled server decodes every preview on a decode worker, the
    // body form included. No connection thread decodes.
    if let Err(error) = validate_frame_request(request.frame) {
        retire_request_body(charge, body);
        return (error_reply(error), None);
    }
    let home = charge.slot.as_ref().map(BodyPermit::worker);
    let mut live = || stream.is_none_or(peer_open);
    let Some(permit) = viewer.acquire_decode(home, &mut live) else {
        retire_request_body(charge, body);
        return (busy_reply(), None);
    };
    let body_retire = charge.slot.as_ref().map(BodyPermit::retire);
    let inflight = viewer.inflight_counter();
    let gate = viewer.grey_gate();
    let cache = viewer.cache_handle();
    let frame = request.frame;
    let body = BodyHome {
        body: Some(body),
        retire: body_retire,
    };
    let job_gate = Arc::clone(&gate);
    let job_cache = cache.clone();
    let job_inflight = Arc::clone(&inflight);
    let Some(outcome) =
        permit.call(move || pooled_body_preview(body, frame, &job_inflight, &job_gate, &job_cache))
    else {
        return (busy_reply(), None);
    };
    let work = match outcome {
        PooledOutcome::Ready(work) => work,
        PooledOutcome::Congested(congested) => {
            let grey_len = congested.input.grey_len();
            let sample_charge = congested.sample_charge;
            // The samples stay charged as in flight until they are recycled,
            // on every path out of this branch.
            let settle = |samples: Option<Vec<u8>>| {
                if let Some(samples) = samples {
                    cache.recycle_sample_buffer(samples);
                }
                if sample_charge > 0 {
                    Viewer::release_counter(&inflight, sample_charge);
                }
            };
            let RenderInput::Decoded(held) = congested.input else {
                // The body form only defers a frame it decoded.
                settle(None);
                return (grey_busy_reply(), None);
            };
            // The decode slot is free while the reply waits for the cap. The
            // body slot is kept, so at most four body-form previews wait with
            // their samples.
            drop(permit);
            if !wait_for_grey(&gate, grey_len, stream) {
                settle(Some(held.samples));
                return (grey_busy_reply(), None);
            }
            let Some(permit) = viewer.acquire_decode(None, &mut live) else {
                viewer.release_grey(grey_len);
                settle(Some(held.samples));
                return (busy_reply(), None);
            };
            let job_gate = Arc::clone(&gate);
            let Some((preview, held, phase)) =
                permit.call(move || render_decoded(held, request.frame, &job_gate))
            else {
                viewer.release_grey(grey_len);
                settle(None);
                return (busy_reply(), None);
            };
            merge_worker_phases(phase);
            settle(Some(held.samples));
            let mut reply = from_preview(preview);
            if reply.status == 200 {
                reply.source = Some("body");
                let home = permit.finish();
                return (reply, Some(home));
            }
            return (reply, None);
        }
    };
    merge_worker_phases(work.phase);
    let mut reply = from_preview(work.preview);
    if reply.status == 200 {
        reply.source = Some("body");
        let home = permit.finish();
        return (reply, Some(home));
    }
    (reply, None)
}

/// A pooled request body inside a decode job.
///
/// The decode runs on whichever decode worker is free, and the body was
/// allocated by another worker. Drop sends the bytes back to that worker, so
/// the body is freed where it was allocated. A dead worker drops them here
/// instead, as [`retire_bytes`] does.
struct BodyHome {
    body: Option<Vec<u8>>,
    retire: Option<Sender<Vec<u8>>>,
}

impl BodyHome {
    fn as_slice(&self) -> &[u8] {
        self.body.as_deref().unwrap_or(&[])
    }
}

impl Drop for BodyHome {
    fn drop(&mut self) {
        if let Some(bytes) = self.body.take() {
            match &self.retire {
                Some(tx) => retire_bytes(tx, bytes),
                None => drop(bytes),
            }
        }
    }
}

/// Decode a body-form preview on a decode worker.
///
/// The sample bytes are charged as in flight until the job ends. The decode
/// takes a parked buffer of its length when there is one, and allocates
/// otherwise. It never evicts a cached entry and never steals a cached
/// frame, because no frame is stored. After the render the buffer returns to
/// the free list when cached bytes plus free bytes fit the budget, and is
/// freed otherwise.
fn pooled_body_preview(
    body: BodyHome,
    request: FrameRequest,
    inflight: &std::sync::atomic::AtomicUsize,
    gate: &GreyGate,
    cache: &CacheHandle,
) -> PooledOutcome {
    let mut samples = Vec::new();
    let mut sample_charge = 0_usize;
    let decoded = with_decode_workspace(|workspace| {
        decode_held(
            body.as_slice(),
            request.frame,
            &mut workspace.native,
            &mut samples,
            &mut |capacity| {
                sample_charge = capacity;
                inflight.fetch_add(capacity, std::sync::atomic::Ordering::Relaxed);
            },
            &mut |len| cache.claim_parked_buffer(len),
        )
    });
    match decoded {
        Ok(held) => {
            let grey_len = grey_bytes(held.width, held.height);
            if !gate.reserve(grey_len) {
                drop(body);
                return PooledOutcome::Congested(Box::new(CongestedWork {
                    input: RenderInput::Decoded(Box::new(held)),
                    sample_charge,
                }));
            }
            let preview = with_decode_workspace(|workspace| {
                render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
            });
            if preview.is_err() {
                gate.release(grey_len);
            }
            samples = held.samples;
            if sample_charge > 0 {
                Viewer::release_counter(inflight, sample_charge);
            }
            cache.recycle_sample_buffer(samples);
            detach_grey(grey_capacity(&preview));
            let phase = phases();
            drop(body);
            PooledOutcome::Ready(Box::new(PooledWork {
                gone: false,
                preview,
                fresh: false,
                phase,
                decoded: None,
            }))
        }
        Err(error) => {
            if sample_charge > 0 {
                Viewer::release_counter(inflight, sample_charge);
            }
            drop(body);
            PooledOutcome::Ready(Box::new(PooledWork {
                gone: false,
                preview: Err(error),
                fresh: false,
                phase: phases(),
                decoded: None,
            }))
        }
    }
}

fn busy_reply() -> Reply {
    text_reply(
        503,
        Some("busy"),
        "this process is already serving its connection limit",
    )
}

/// Discard an over-cap body so close does not reset the socket.
///
/// The 503 is already written. This reads into an 8 KiB stack array and
/// drops every byte. It stops at the declared length, at [`BUSY_DRAIN_CAP`],
/// or after [`BUSY_DRAIN_WAIT`] with no further bytes.
fn discard_busy_body(stream: &mut TcpStream, pending: &[u8], header_end: usize) {
    let declared = declared_length(pending, header_end);
    let already = pending.len().saturating_sub(header_end);
    let limit = declared.min(BUSY_DRAIN_CAP);
    let mut left = limit.saturating_sub(already.min(limit));
    if declared == 0 {
        left = BUSY_DRAIN_CAP.saturating_sub(already.min(BUSY_DRAIN_CAP));
    }
    let _ = stream.set_read_timeout(Some(BUSY_DRAIN_WAIT));
    let mut chunk = [0_u8; 8192];
    while left > 0 {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => left = left.saturating_sub(n),
        }
    }
}

fn declared_length(pending: &[u8], header_end: usize) -> usize {
    let Some(header) = pending.get(..header_end) else {
        return 0;
    };
    let Ok(text) = std::str::from_utf8(header) else {
        return 0;
    };
    for line in text.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value.trim().parse::<usize>().unwrap_or(0);
        }
    }
    0
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

struct PooledWork {
    gone: bool,
    preview: Result<Preview, PreviewError>,
    fresh: bool,
    phase: crate::preview::PhaseRecord,
    decoded: Option<DecodedWork>,
}

struct DecodedWork {
    held: crate::preview::HeldFrame,
    sample_charge: usize,
}

/// A frame waiting to be rendered, held by the connection that owns the grey.
enum RenderInput {
    /// A cached frame. Its grey is `width * height` bytes.
    Hit(Arc<CachedFrame>),
    /// A decoded frame that no cache entry holds yet. Its sample bytes stay
    /// charged to inflight until the caller stores or recycles them.
    Decoded(Box<crate::preview::HeldFrame>),
}

impl RenderInput {
    fn grey_len(&self) -> usize {
        let (width, height) = match self {
            Self::Hit(held) => (held.width, held.height),
            Self::Decoded(held) => (held.width, held.height),
        };
        grey_bytes(width, height)
    }
}

/// What one decode worker job returned.
enum PooledOutcome {
    /// The grey cap had room and the render ran.
    Ready(Box<PooledWork>),
    /// The grey would exceed the cap, so nothing was rendered. The input is
    /// returned unharmed and nothing is counted except the input's own charge.
    Congested(Box<CongestedWork>),
}

/// A render the grey cap deferred.
struct CongestedWork {
    input: RenderInput,
    /// Inflight charge on decoded samples, released when they are consumed.
    sample_charge: usize,
}

/// Grey bytes for one frame: one display byte per pixel.
fn grey_bytes(width: u32, height: u32) -> usize {
    usize::try_from(width)
        .unwrap_or(0)
        .saturating_mul(usize::try_from(height).unwrap_or(0))
}

/// Wait for grey cap room, up to the write timeout, while the client waits.
///
/// `false` means the client is gone or the cap stayed full to the deadline.
fn wait_for_grey(gate: &GreyGate, len: usize, stream: Option<&TcpStream>) -> bool {
    let deadline = Instant::now() + WRITE_TIMEOUT;
    let live = || stream.is_none_or(peer_open);
    loop {
        if gate.reserve(len) {
            return true;
        }
        if !live() {
            return false;
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        gate.wait_slice(
            deadline
                .saturating_duration_since(now)
                .min(Duration::from_millis(50)),
        );
    }
}

/// Render a body-form frame the cap delayed, on whichever worker is free.
///
/// The caller already holds the grey reservation. A refused render releases
/// it here. The frame comes back either way, so its samples are recycled.
fn render_decoded(
    held: Box<crate::preview::HeldFrame>,
    request: FrameRequest,
    gate: &GreyGate,
) -> (
    Result<Preview, PreviewError>,
    Box<crate::preview::HeldFrame>,
    crate::preview::PhaseRecord,
) {
    let preview = with_decode_workspace(|workspace| {
        render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
    });
    if preview.is_err() {
        gate.release(grey_bytes(held.width, held.height));
    }
    detach_grey(grey_capacity(&preview));
    (preview, held, phases())
}

/// Use the grey reservation this request already waited for, or take one.
///
/// A reservation of another length is returned before the new one is tried.
fn take_grey(gate: &GreyGate, reserved: usize, len: usize) -> bool {
    if reserved == len {
        return true;
    }
    if reserved > 0 {
        gate.release(reserved);
    }
    gate.reserve(len)
}

fn grey_busy_reply() -> Reply {
    text_reply(503, Some("busy"), "the grey reply cap has no room")
}

fn preview_held_on_worker(
    viewer: &Viewer,
    id: FileId,
    request: FrameRequest,
    stream: Option<&TcpStream>,
) -> (Reply, Option<GreyHome>) {
    if let Err(error) = validate_frame_request(request) {
        return (error_reply(error), None);
    }
    let lookup_started = profiling().then(Instant::now);
    let found = viewer.preview_source(id, request.frame);
    if let Some(started) = lookup_started {
        note_lookup(started.elapsed().as_micros());
    }
    let gate = viewer.grey_gate();
    let inflight = viewer.inflight_counter();
    let cache = viewer.cache_handle();
    let mut live = || stream.is_none_or(peer_open);
    // Grey bytes this request already waited for, and whether any round
    // decoded the frame.
    let mut reserved = 0_usize;
    let mut decoded_here = false;
    let mut found = Some(found);
    for _ in 0..GREY_ROUNDS {
        let found = match found.take() {
            Some(found) => found,
            None => viewer.preview_source(id, request.frame),
        };
        let home = match &found {
            PreviewSource::Hit(frame) => frame.worker(),
            PreviewSource::Shared(bytes) => bytes.worker(),
            PreviewSource::Gone => {
                release_reserved(&gate, reserved);
                return (gone(), None);
            }
        };
        let Some(permit) = viewer.acquire_decode(home, &mut live) else {
            release_reserved(&gate, reserved);
            return (busy_reply(), None);
        };
        let worker = permit.worker();
        let retire = permit.retire();
        let job_gate = Arc::clone(&gate);
        let job_cache = cache.clone();
        let job_inflight = Arc::clone(&inflight);
        let Some(outcome) = permit.call(move || {
            pooled_preview(
                found,
                request,
                reserved,
                &job_inflight,
                &job_gate,
                &job_cache,
            )
        }) else {
            // The job did not run, so the reservation is still this request's.
            release_reserved(&gate, reserved);
            return (busy_reply(), None);
        };
        // The job took over the reservation: it is in the reply, or released.
        // Every path below returns or takes a new reservation.
        let work = match outcome {
            PooledOutcome::Ready(work) => work,
            PooledOutcome::Congested(congested) => {
                let grey_len = congested.input.grey_len();
                // The frame goes into the cache before the wait, so a waiting
                // request holds no frame and no permit. After the wait it
                // looks the frame up again.
                if let RenderInput::Decoded(held) = congested.input {
                    decoded_here = true;
                    let _kept = viewer.store_frame_with(
                        id,
                        request.frame,
                        *held,
                        Some(worker),
                        Some(retire),
                    );
                }
                if congested.sample_charge > 0 {
                    viewer.leave_inflight(congested.sample_charge);
                }
                drop(permit);
                if !wait_for_grey(&gate, grey_len, stream) {
                    return (grey_busy_reply(), None);
                }
                reserved = grey_len;
                continue;
            }
        };
        if work.gone {
            return (gone(), None);
        }
        merge_worker_phases(work.phase);
        if let Some(decoded) = work.decoded {
            let _kept = viewer.store_frame_with(
                id,
                request.frame,
                decoded.held,
                Some(worker),
                Some(retire),
            );
            if decoded.sample_charge > 0 {
                viewer.leave_inflight(decoded.sample_charge);
            }
            #[cfg(test)]
            viewer.trace_inflight("frame");
        }
        let mut reply = from_preview(work.preview);
        if reply.status == 200 {
            reply.source = Some(if work.fresh || decoded_here {
                "decoded"
            } else {
                "held"
            });
            // The grey is ready, so the permit ends with the decode. The slot
            // is free before the reply is written, and the grey goes back to
            // its worker by message after the write.
            let home = permit.finish();
            return (reply, Some(home));
        }
        return (reply, None);
    }
    release_reserved(&gate, reserved);
    (grey_busy_reply(), None)
}

/// Rounds a held preview may wait for grey room. The second round already
/// holds a reservation of the frame's length, so a third is not reached.
const GREY_ROUNDS: usize = 3;

fn release_reserved(gate: &GreyGate, reserved: usize) {
    if reserved > 0 {
        gate.release(reserved);
    }
}

/// One held preview on a decode worker.
///
/// `reserved` is grey room the caller already waited for. The job takes it
/// over: it is in the reply's grey, or released here, on every outcome.
fn pooled_preview(
    found: PreviewSource,
    request: FrameRequest,
    reserved: usize,
    inflight: &std::sync::atomic::AtomicUsize,
    gate: &GreyGate,
    cache: &CacheHandle,
) -> PooledOutcome {
    let before = decode_ticks();
    match found {
        PreviewSource::Gone => {
            release_reserved(gate, reserved);
            PooledOutcome::Ready(Box::new(PooledWork {
                gone: true,
                preview: Err(PreviewError::Frame),
                fresh: false,
                phase: phases(),
                decoded: None,
            }))
        }
        PreviewSource::Hit(held) => {
            let grey_len = grey_bytes(held.width, held.height);
            if !take_grey(gate, reserved, grey_len) {
                return PooledOutcome::Congested(Box::new(CongestedWork {
                    input: RenderInput::Hit(held),
                    sample_charge: 0,
                }));
            }
            let preview = with_decode_workspace(|workspace| {
                render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
            });
            if preview.is_err() {
                gate.release(grey_len);
            }
            detach_grey(grey_capacity(&preview));
            PooledOutcome::Ready(Box::new(PooledWork {
                gone: false,
                preview,
                fresh: false,
                phase: phases(),
                decoded: None,
            }))
        }
        PreviewSource::Shared(bytes) => {
            let mut samples = Vec::new();
            let mut sample_charge = 0_usize;
            let decoded = with_decode_workspace(|workspace| {
                decode_held(
                    bytes.as_slice(),
                    request.frame,
                    &mut workspace.native,
                    &mut samples,
                    &mut |capacity| {
                        sample_charge = capacity;
                        inflight.fetch_add(capacity, std::sync::atomic::Ordering::Relaxed);
                    },
                    &mut |len| cache.claim_sample_buffer(len),
                )
            });
            drop(bytes);
            match decoded {
                Ok(held) => {
                    let grey_len = grey_bytes(held.width, held.height);
                    if !take_grey(gate, reserved, grey_len) {
                        return PooledOutcome::Congested(Box::new(CongestedWork {
                            input: RenderInput::Decoded(Box::new(held)),
                            sample_charge,
                        }));
                    }
                    let preview = with_decode_workspace(|workspace| {
                        render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
                    });
                    if preview.is_err() {
                        gate.release(grey_len);
                    }
                    detach_grey(grey_capacity(&preview));
                    PooledOutcome::Ready(Box::new(PooledWork {
                        gone: false,
                        fresh: decode_ticks() != before,
                        phase: phases(),
                        decoded: Some(DecodedWork {
                            held,
                            sample_charge,
                        }),
                        preview,
                    }))
                }
                Err(error) => {
                    release_reserved(gate, reserved);
                    cache.recycle_sample_buffer(std::mem::take(&mut samples));
                    if sample_charge > 0 {
                        Viewer::release_counter(inflight, sample_charge);
                    }
                    PooledOutcome::Ready(Box::new(PooledWork {
                        gone: false,
                        preview: Err(error),
                        fresh: false,
                        phase: phases(),
                        decoded: None,
                    }))
                }
            }
        }
    }
}

fn grey_capacity(preview: &Result<Preview, PreviewError>) -> usize {
    match preview {
        Ok(preview) => preview.grey.capacity(),
        Err(_) => 0,
    }
}

fn preview_held(
    viewer: &Viewer,
    id: FileId,
    request: FrameRequest,
    stream: Option<&TcpStream>,
) -> (Reply, Option<GreyHome>) {
    if viewer.schedules() {
        return preview_held_on_worker(viewer, id, request, stream);
    }
    if let Err(error) = validate_frame_request(request) {
        return (error_reply(error), None);
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
        return (busy_reply(), None);
    };
    let workspace = &mut loan.workspace;
    let preview = match found {
        PreviewSource::Gone => return (gone(), None),
        PreviewSource::Hit(held) => {
            render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
        }
        PreviewSource::Shared(bytes) => {
            // The posted file stays in the cache for the whole decode.
            // The sample buffer is inflight only until it is stored.
            let mut samples = Vec::new();
            let mut sample_charge = 0_usize;
            let cache = viewer.cache_handle();
            let decoded = decode_held(
                &bytes,
                request.frame,
                &mut workspace.native,
                &mut samples,
                &mut |capacity| {
                    sample_charge = capacity;
                    viewer.enter_inflight(capacity);
                    #[cfg(test)]
                    viewer.trace_inflight("decoding");
                },
                &mut |len| cache.claim_sample_buffer(len),
            );
            drop(bytes);
            match decoded {
                Ok(held) => {
                    let held = viewer.store_frame(id, request.frame, held);
                    if sample_charge > 0 {
                        viewer.leave_inflight(sample_charge);
                    }
                    #[cfg(test)]
                    viewer.trace_inflight("frame");
                    render_held(&held, request, &mut workspace.scratch, &mut workspace.grey)
                }
                Err(error) => {
                    viewer.recycle_sample_buffer(std::mem::take(&mut samples));
                    if sample_charge > 0 {
                        viewer.leave_inflight(sample_charge);
                    }
                    Err(error)
                }
            }
        }
    };
    loan.note_growth();
    let mut reply = from_preview(preview);
    if reply.status == 200 {
        reply.source = Some(if decode_ticks() == before {
            "held"
        } else {
            "decoded"
        });
    }
    (reply, None)
}

fn file_route(viewer: &Viewer, request: Request, charge: &mut BodyCharge<'_>) -> Reply {
    let frame = request.frame.frame;
    let body = request.body;
    let inspected = if let Some(slot) = charge.slot.as_ref() {
        let worker = slot.worker();
        let retire = slot.retire();
        slot.call(move || {
            let result = inspect_json(&body, frame);
            (result, body, Some(worker), Some(retire))
        })
    } else {
        let result = inspect_json(&body, frame);
        Some((result, body, None, None))
    };
    let Some((result, body, worker, retire)) = inspected else {
        return busy_reply();
    };
    match result {
        Ok(json) => {
            // Leave inflight before the bytes are visible in the LRU.
            charge.release_for_cache();
            let mut reply = bare(200, "application/json; charset=utf-8", json.into_bytes());
            match viewer.insert_posted(body, worker, retire) {
                Some(id) => {
                    reply.file_id = Some(id.header_value());
                    reply.held_flag = Some(1);
                }
                None => reply.held_flag = Some(0),
            }
            #[cfg(test)]
            viewer.trace_inflight("cached");
            reply
        }
        Err(error) => {
            if let Some(slot) = charge.slot.as_ref() {
                retire_bytes(&slot.retire(), body);
            } else {
                drop(body);
            }
            error_reply(error)
        }
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
            Some(bytes) => json_or_error(headers_json(bytes.as_slice())),
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
    let mut entered = 0_usize;
    let mut counted = false;
    let mut body_slot = None;
    let filled = if viewer.schedules() && declared > 0 {
        read_pooled_body(stream, pending, header_end, declared, viewer)
    } else {
        fill_body(stream, pending, header_end, declared, |need| {
            let body = begin_body(viewer, need);
            entered = body.capacity();
            if need > 0 {
                viewer.enter_body();
                counted = true;
            }
            body
        })
        .map(|body| (body, None))
    };
    let (body, pooled) = match filled {
        Ok(pair) => pair,
        Err(stop) => {
            viewer.leave_inflight(entered);
            if counted {
                viewer.leave_body();
            }
            return Err(stop);
        }
    };
    if pooled.is_some() {
        entered = body.capacity();
        body_slot = pooled;
    }
    let final_charge = body.capacity();
    if final_charge > entered {
        viewer.enter_inflight(final_charge - entered);
    } else if final_charge < entered {
        viewer.leave_inflight(entered - final_charge);
    }
    Ok(Request {
        method: parsed.method,
        path: parsed.path,
        body,
        frame: parsed.frame,
        file_id: parsed.file_id,
        close: parsed.close,
        body_charge: final_charge,
        body_counted: counted,
        body_slot,
    })
}

fn read_pooled_body(
    stream: &mut TcpStream,
    pending: &mut Vec<u8>,
    header_end: usize,
    declared: usize,
    viewer: &Viewer,
) -> Result<(Vec<u8>, Option<BodyPermit>), ReadStop> {
    let mut live = || peer_open(stream);
    let Some(permit) = viewer.acquire_body(&mut live) else {
        return Err(ReadStop::Reply(busy_reply()));
    };
    let Some(mut body) = permit.call(move || exact_body(declared)) else {
        return Err(ReadStop::Reply(busy_reply()));
    };
    let capacity = body.capacity();
    viewer.enter_inflight(capacity);
    #[cfg(test)]
    viewer.trace_inflight("reading");
    if let Err(stop) = fill_prepared(stream, pending, header_end, declared, &mut body) {
        viewer.leave_inflight(capacity);
        let _ = permit.call(move || drop(body));
        return Err(stop);
    }
    Ok((body, Some(permit)))
}

fn fill_prepared<R: Read>(
    stream: &mut R,
    pending: &mut Vec<u8>,
    header_end: usize,
    declared: usize,
    body: &mut Vec<u8>,
) -> Result<(), ReadStop> {
    if body.capacity() < declared {
        return Err(ReadStop::Reply(text_reply(
            400,
            Some("body"),
            "the preview request could not be stored",
        )));
    }
    let tail = if pending.len() > header_end {
        pending.split_off(header_end)
    } else {
        Vec::new()
    };
    release_read_buffer(pending);
    if tail.len() >= declared {
        let mut incoming = tail;
        let surplus = incoming.split_off(declared);
        if !push_within(body, &incoming) {
            return Err(ReadStop::Reply(text_reply(
                400,
                Some("body"),
                "the preview request could not be stored",
            )));
        }
        if !surplus.is_empty() {
            *pending = surplus;
        }
        return Ok(());
    }
    if !push_within(body, &tail) {
        return Err(ReadStop::Reply(text_reply(
            400,
            Some("body"),
            "the preview request could not be stored",
        )));
    }
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
            if !push_within(body, got) {
                return Err(ReadStop::Reply(text_reply(
                    400,
                    Some("body"),
                    "the preview request could not be stored",
                )));
            }
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
            if !push_within(body, take) {
                return Err(ReadStop::Reply(text_reply(
                    400,
                    Some("body"),
                    "the preview request could not be stored",
                )));
            }
            *pending = extra.to_vec();
            return Ok(());
        }
    }
    Ok(())
}

fn push_within(body: &mut Vec<u8>, bytes: &[u8]) -> bool {
    let Some(new_len) = body.len().checked_add(bytes.len()) else {
        return false;
    };
    if new_len > body.capacity() {
        return false;
    }
    body.extend_from_slice(bytes);
    true
}

/// Allocate a body and count it while the following read fills it.
fn begin_body(viewer: &Viewer, declared: usize) -> Vec<u8> {
    let body = exact_body(declared);
    viewer.enter_inflight(body.capacity());
    #[cfg(test)]
    viewer.trace_inflight("reading");
    body
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

/// One buffer of exactly `declared` bytes.
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
    fn a_body_is_inflight_while_read_and_not_twice_once_cached() -> Result<(), String> {
        // Slice 0 is one 96 by 128 16-bit frame. 96 * 128 * 2 = 24576 sample bytes.
        let file = slice(0).ok_or_else(|| "synthetic slice 0 was absent".to_owned())?;
        let declared = file.len();
        let viewer = Viewer::new(ViewerConfig {
            cache_bytes: 8 * 1024 * 1024,
        });
        let during_inflight = std::sync::atomic::AtomicUsize::new(usize::MAX);
        let during_files = std::sync::atomic::AtomicUsize::new(usize::MAX);
        let during_used = std::sync::atomic::AtomicUsize::new(usize::MAX);
        let mut stream = WatchRead {
            inner: std::io::Cursor::new(file),
            on_first: Some(|| {
                let census = viewer.memory_census();
                during_inflight.store(census.inflight, std::sync::atomic::Ordering::Relaxed);
                during_files.store(census.files, std::sync::atomic::Ordering::Relaxed);
                during_used.store(census.pressure.used, std::sync::atomic::Ordering::Relaxed);
            }),
        };
        let mut pending = Vec::new();
        let body = fill_body(&mut stream, &mut pending, 0, declared, |need| {
            begin_body(&viewer, need)
        })
        .map_err(|_| "the declared body could not be read".to_owned())?;
        let capacity = body.capacity();
        if capacity != declared {
            return Err(format!(
                "the body capacity was {capacity} for {declared} declared bytes"
            ));
        }
        let seen_inflight = during_inflight.load(std::sync::atomic::Ordering::Relaxed);
        let seen_files = during_files.load(std::sync::atomic::Ordering::Relaxed);
        let seen_used = during_used.load(std::sync::atomic::Ordering::Relaxed);
        if seen_inflight != capacity || seen_files != 0 || seen_used != 0 {
            return Err(format!(
                "during the read inflight was {seen_inflight}, files {seen_files}, used {seen_used}"
            ));
        }
        let filled = viewer.memory_census();
        if filled.inflight != capacity || filled.files != 0 || filled.frames != 0 {
            return Err(format!(
                "after the read inflight was {}, files {}, frames {}",
                filled.inflight, filled.files, filled.frames
            ));
        }
        let mut charge = BodyCharge {
            viewer: &viewer,
            bytes: capacity,
            counted: false,
            slot: None,
        };
        let reply = file_route(
            &viewer,
            Request {
                method: "POST".to_owned(),
                path: "/file".to_owned(),
                body,
                frame: FrameRequest::default(),
                file_id: None,
                close: true,
                body_charge: capacity,
                body_counted: false,
                body_slot: None,
            },
            &mut charge,
        );
        if reply.status != 200 || reply.held_flag != Some(1) {
            return Err(format!("the file was not cached: status {}", reply.status));
        }
        if charge.bytes != 0 {
            return Err("the cached file stayed inflight".to_owned());
        }
        let id_text = reply
            .file_id
            .clone()
            .ok_or_else(|| "the file id was missing".to_owned())?;
        let id = FileId::parse(&id_text).ok_or_else(|| "the file id did not parse".to_owned())?;
        let cached = viewer.memory_census();
        if cached.inflight != 0 || cached.files != 1 || cached.pressure.used < capacity {
            return Err(format!(
                "after cache inflight was {}, files {}, used {}",
                cached.inflight, cached.files, cached.pressure.used
            ));
        }
        let (preview, _permit) = preview_held(&viewer, id, FrameRequest::default(), None);
        if preview.status != 200 {
            return Err(format!("the preview status was {}", preview.status));
        }
        let done = viewer.memory_census();
        if done.inflight != 0 || done.files != 0 || done.frames != 1 {
            return Err(format!(
                "after the frame inflight was {}, files {}, frames {}",
                done.inflight, done.files, done.frames
            ));
        }
        let steps = viewer.inflight_steps();
        let reading = step_named(&steps, "reading")?;
        let cached_step = step_named(&steps, "cached")?;
        let decoding = step_named(&steps, "decoding")?;
        let frame = step_named(&steps, "frame")?;
        if reading.inflight != capacity || reading.files != 0 || reading.used != 0 {
            return Err(format!(
                "the reading step was inflight {} files {} used {}",
                reading.inflight, reading.files, reading.used
            ));
        }
        if cached_step.inflight != 0 || cached_step.files != 1 || cached_step.used < capacity {
            return Err(format!(
                "the cached step was inflight {} files {} used {}",
                cached_step.inflight, cached_step.files, cached_step.used
            ));
        }
        let sample_floor = 96_usize * 128 * 2;
        if decoding.files != 1
            || decoding.frames != 0
            || decoding.inflight < sample_floor
            || decoding.inflight >= capacity
            || decoding.used < capacity
        {
            return Err(format!(
                "the decoding step was inflight {} files {} frames {} used {}",
                decoding.inflight, decoding.files, decoding.frames, decoding.used
            ));
        }
        if frame.inflight != 0
            || frame.files != 0
            || frame.frames != 1
            || frame.used >= cached_step.used
        {
            return Err(format!(
                "the frame step was inflight {} files {} frames {} used {}, cached used {}",
                frame.inflight, frame.files, frame.frames, frame.used, cached_step.used
            ));
        }
        Ok(())
    }

    fn step_named<'a>(
        steps: &'a [crate::held::InflightStep],
        label: &str,
    ) -> Result<&'a crate::held::InflightStep, String> {
        steps
            .iter()
            .find(|step| step.label == label)
            .ok_or_else(|| format!("the {label} step was missing"))
    }

    struct WatchRead<R, F> {
        inner: R,
        on_first: Option<F>,
    }

    impl<R: std::io::Read, F: FnOnce()> std::io::Read for WatchRead<R, F> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if let Some(hook) = self.on_first.take() {
                hook();
            }
            self.inner.read(buf)
        }
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
        let (first, _permit) = preview_held(&viewer, id, FrameRequest::default(), None);
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
        let (second, _permit) = preview_held(&viewer, id, FrameRequest::default(), None);
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

    #[test]
    fn concurrent_previews_and_headers_keep_the_posted_file() -> Result<(), String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        let addr = listener.local_addr().map_err(|error| error.to_string())?;
        let viewer = Arc::new(Viewer::with_connection_cap(
            ViewerConfig {
                cache_bytes: 8 * 1024 * 1024,
            },
            8,
        ));
        let serving = Arc::clone(&viewer);
        std::thread::spawn(move || {
            let _ = serve_on(listener, serving);
        });
        let bytes = slice(0).ok_or("synthetic slice 0 was absent")?;
        let posted = post_bytes(addr, &bytes)?;
        if !status_is(&posted, 200) {
            return Err(format!("the only post was not accepted: {posted}"));
        }
        let id = header_value(&posted, "X-Ocelli-File").ok_or("the file id was missing")?;
        let parsed = FileId::parse(&id).ok_or("the file id did not parse")?;
        let first_permit = viewer
            .loan_workspace(&mut || true)
            .ok_or("the first permit was missing")?;
        let second_permit = viewer
            .loan_workspace(&mut || true)
            .ok_or("the second permit was missing")?;
        let before = viewer.preview_calls();
        let first_addr = addr;
        let first_id = id.clone();
        let first = std::thread::spawn(move || request_preview(first_addr, &first_id));
        wait_for_preview(&viewer, before)?;
        if viewer.file_bytes(parsed).is_none() {
            return Err("the held file was removed while a decode waited".to_owned());
        }
        let headers = request_headers(addr, &id)?;
        if !status_is(&headers, 200) {
            return Err(format!("headers during a decode were not 200: {headers}"));
        }
        let mid = viewer.preview_calls();
        let second_addr = addr;
        let second_id = id.clone();
        let second = std::thread::spawn(move || request_preview(second_addr, &second_id));
        wait_for_preview(&viewer, mid)?;
        if viewer.file_bytes(parsed).is_none() {
            return Err("the held file was removed for a second preview".to_owned());
        }
        drop(first_permit);
        drop(second_permit);
        let first_reply = first
            .join()
            .map_err(|_| "the first preview thread panicked".to_owned())??;
        let second_reply = second
            .join()
            .map_err(|_| "the second preview thread panicked".to_owned())??;
        if !status_is(&first_reply, 200) || !status_is(&second_reply, 200) {
            return Err(format!(
                "concurrent previews were not both 200\n{first_reply}\n{second_reply}"
            ));
        }
        Ok(())
    }

    #[test]
    fn a_reply_reading_a_frame_keeps_it_cached_for_the_next_preview() -> Result<(), String> {
        // Slice 0 is 96 by 128 and slice 1 is the same 24576 sample bytes with
        // different rows of pixels. Both are single-frame objects, so the
        // encoded bytes leave when the frame is stored.
        let mut file_a = slice(0).ok_or("synthetic slice 0 was absent")?;
        let mut file_b = slice(1).ok_or("synthetic slice 1 was absent")?;
        file_a.shrink_to_fit();
        file_b.shrink_to_fit();
        let file_b_cap = file_b.capacity();
        let file_b_copy = file_b.clone();
        let probe = Viewer::new(ViewerConfig {
            cache_bytes: 64 * 1024 * 1024,
        });
        let probe_id = probe
            .insert_file(file_a.clone())
            .ok_or("the probe file was refused")?;
        let (probe_reply, _) = preview_held(&probe, probe_id, FrameRequest::default(), None);
        if probe_reply.status != 200 {
            return Err(format!(
                "the probe preview status was {}",
                probe_reply.status
            ));
        }
        let frame_used = probe.pressure().used;
        if frame_used == 0 {
            return Err("the probe frame cost nothing".to_owned());
        }
        // The budget holds file B's bytes beside frame A, and leaves frame A
        // one byte short of also holding frame B's claim. The claim for B then
        // reaches the steal, finds A's frame being read, and must leave it.
        let budget = frame_used
            .saturating_mul(2)
            .saturating_add(file_b_cap)
            .saturating_sub(1);
        let viewer = Viewer::new(ViewerConfig {
            cache_bytes: budget,
        });
        let id_a = viewer.insert_file(file_a).ok_or("file A was refused")?;
        let (first, _) = preview_held(&viewer, id_a, FrameRequest::default(), None);
        if first.status != 200 {
            return Err(format!("preview A status was {}", first.status));
        }
        // A reply is reading frame A: the Arc a render holds while it works.
        let reading = match viewer.preview_source(id_a, 0) {
            PreviewSource::Hit(frame) => frame,
            PreviewSource::Shared(_) | PreviewSource::Gone => {
                return Err("frame A was not cached after its preview".to_owned());
            }
        };
        let id_b = viewer.insert_file(file_b).ok_or("file B was refused")?;
        let fresh = preview_frame(&file_b_copy, FrameRequest::default())
            .map_err(|error| format!("a fresh decode of B failed: {error}"))?;
        let (second, _) = preview_held(&viewer, id_b, FrameRequest::default(), None);
        if second.status != 200 {
            return Err(format!("preview B status was {}", second.status));
        }
        if second.body != fresh.grey {
            return Err("preview B differed from a fresh decode".to_owned());
        }
        // The image on screen is still served from the cache. A 410 would
        // make the page re-post the file.
        let (again, _) = preview_held(&viewer, id_a, FrameRequest::default(), None);
        if again.status != 200 {
            return Err(format!(
                "the frame being read answered {}: the page would re-post",
                again.status
            ));
        }
        if again.body != first.body {
            return Err("the frame being read changed its grey".to_owned());
        }
        if again.source != Some("held") {
            return Err("the frame being read was not served from the cache".to_owned());
        }
        drop(reading);
        Ok(())
    }

    #[test]
    fn a_pooled_server_posts_and_previews_on_two_workers() -> Result<(), String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        let addr = listener.local_addr().map_err(|error| error.to_string())?;
        let viewer = Arc::new(Viewer::server(ViewerConfig {
            cache_bytes: 8 * 1024 * 1024,
        }));
        let serving = Arc::clone(&viewer);
        std::thread::spawn(move || {
            let _ = serve_on(listener, serving);
        });
        let bytes = slice(0).ok_or("synthetic slice 0 was absent")?;
        let posted = post_bytes(addr, &bytes)?;
        if !status_is(&posted, 200) {
            return Err(format!("the pooled post was not accepted: {posted}"));
        }
        let id = header_value(&posted, "X-Ocelli-File").ok_or("the file id was missing")?;
        let (threads, peak, now) = viewer.body_schedule();
        if threads != 2 || peak != 1 || now != 0 {
            return Err(format!(
                "bodies were threads {threads} peak {peak} now {now}"
            ));
        }
        let preview = request_preview(addr, &id)?;
        if !status_is(&preview, 200) {
            return Err(format!("the pooled preview was not 200: {preview}"));
        }
        let source = header_value(&preview, "X-Ocelli-Source").unwrap_or_default();
        if source != "decoded" {
            return Err(format!("the pooled preview source was {source}"));
        }
        let census = viewer.memory_census();
        if census.inflight != 0 || census.frames != 1 || census.workspace == 0 {
            return Err(format!(
                "after preview inflight {} frames {} workspace {}",
                census.inflight, census.frames, census.workspace
            ));
        }
        Ok(())
    }

    #[test]
    fn an_evicted_frame_buffer_is_reused_for_the_next_decode() -> Result<(), String> {
        let mut file_a = slice(0).ok_or("synthetic slice 0 was absent")?;
        let mut file_b = slice(1).ok_or("synthetic slice 1 was absent")?;
        file_a.shrink_to_fit();
        file_b.shrink_to_fit();
        let probe = Viewer::new(ViewerConfig {
            cache_bytes: 64 * 1024 * 1024,
        });
        let probe_id = probe
            .insert_file(file_a.clone())
            .ok_or("the probe file was refused")?;
        let (probe_reply, _) = preview_held(&probe, probe_id, FrameRequest::default(), None);
        if probe_reply.status != 200 {
            return Err(format!(
                "the probe preview status was {}",
                probe_reply.status
            ));
        }
        let (_, sample_len) = probe
            .cached_sample(probe_id, 0)
            .ok_or("the probe frame was not cached")?;
        let frame_used = probe.pressure().used;
        if sample_len == 0 || frame_used <= sample_len {
            return Err(format!(
                "frame used {frame_used} sample length {sample_len}"
            ));
        }
        // The budget holds the file or the frame, not both. The spare byte
        // under that sum is enough for the evicted sample buffer.
        let file_cap = file_a.capacity().max(file_b.capacity());
        let budget = file_cap + frame_used - 1;
        let viewer = Viewer::new(ViewerConfig {
            cache_bytes: budget,
        });
        let id_a = viewer.insert_file(file_a).ok_or("file A was refused")?;
        let (reply_a, _) = preview_held(&viewer, id_a, FrameRequest::default(), None);
        if reply_a.status != 200 {
            return Err(format!("preview A status was {}", reply_a.status));
        }
        let (reused_before, _) = viewer.buffer_claims();
        let id_b = viewer
            .insert_file(file_b.clone())
            .ok_or("file B was refused")?;
        let parked = viewer
            .free_sample_ptr(sample_len)
            .ok_or("the evicted buffer was not parked")?;
        let fresh = preview_frame(&file_b, FrameRequest::default())
            .map_err(|error| format!("a fresh decode of B failed: {error}"))?;
        let (reply_b, _) = preview_held(&viewer, id_b, FrameRequest::default(), None);
        if reply_b.status != 200 {
            return Err(format!("preview B status was {}", reply_b.status));
        }
        if reply_b.body != fresh.grey {
            return Err("recycled grey differed from a fresh decode".to_owned());
        }
        let (addr, len) = viewer
            .cached_sample(id_b, 0)
            .ok_or("frame B was not cached")?;
        if addr != parked || len != sample_len {
            return Err(format!(
                "recycled pointer {addr} length {len}, parked {parked}, sample {sample_len}"
            ));
        }
        let (reused, _) = viewer.buffer_claims();
        if reused <= reused_before {
            return Err("the decode allocated instead of taking the free buffer".to_owned());
        }
        let census = viewer.memory_census();
        if census.pressure.used.saturating_add(census.free) > budget {
            return Err(format!(
                "used {} free {} budget {budget}",
                census.pressure.used, census.free
            ));
        }
        Ok(())
    }

    #[test]
    fn recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file() -> Result<(), String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("corpus/data");
        if !data.is_dir() {
            return Ok(());
        }
        let manifest = std::fs::read_to_string(root.join("corpus/manifest.tsv"))
            .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
        let mut files = 0_usize;
        let mut accepted = 0_usize;
        let mut reused = 0_usize;
        for (row_index, line) in manifest.lines().skip(1).enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let row = row_index + 2;
            let relative = line
                .split('\t')
                .next()
                .ok_or_else(|| format!("manifest row {row} has no path"))?;
            let mut bytes = std::fs::read(data.join(relative))
                .map_err(|_| format!("corpus file for manifest row {row} is unreadable"))?;
            bytes.shrink_to_fit();
            let first = match preview_frame(&bytes, FrameRequest::default()) {
                Ok(preview) => preview,
                Err(_) => continue,
            };
            let total = first.frames;
            let budget = bytes
                .len()
                .saturating_mul(4)
                .saturating_add(8 * 1024 * 1024);
            let viewer = Viewer::new(ViewerConfig {
                cache_bytes: budget,
            });
            prove_recycled_frame(&viewer, &bytes, 0, &first.grey, row)?;
            files = files.saturating_add(1);
            accepted = accepted.saturating_add(1);
            reused = reused.saturating_add(viewer.buffer_claims().0);
            for index in 1..total {
                let request = FrameRequest {
                    frame: index,
                    ..FrameRequest::default()
                };
                let fresh = match preview_frame(&bytes, request) {
                    Ok(preview) => preview,
                    Err(_) => continue,
                };
                let before = viewer.buffer_claims().0;
                prove_recycled_frame(&viewer, &bytes, index, &fresh.grey, row)?;
                accepted = accepted.saturating_add(1);
                reused = reused.saturating_add(viewer.buffer_claims().0.saturating_sub(before));
            }
        }
        if accepted == 0 {
            return Err("the corpus had no file the viewer accepts".to_owned());
        }
        if reused == 0 {
            return Err("no accepted frame reused a sample buffer".to_owned());
        }
        let out_dir = root.join("target/f216-runs");
        std::fs::create_dir_all(&out_dir).map_err(|error| error.to_string())?;
        std::fs::write(
            out_dir.join("corpus-identity.txt"),
            format!(
                "files {files}\nframes {accepted}\ndifferences 0\nreused {reused}\npoison 0xa5\n"
            ),
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn prove_recycled_frame(
        viewer: &Viewer,
        bytes: &[u8],
        index: u32,
        fresh_grey: &[u8],
        row: usize,
    ) -> Result<(), String> {
        let request = FrameRequest {
            frame: index,
            ..FrameRequest::default()
        };
        let id = viewer
            .insert_file(bytes.to_vec())
            .ok_or_else(|| format!("manifest row {row} frame {index} was refused"))?;
        let (first, _) = preview_held(viewer, id, request, None);
        if first.status != 200 {
            return Err(format!(
                "manifest row {row} frame {index} preview status was {}",
                first.status
            ));
        }
        if let Some(diff) = first_grey_diff(&first.body, fresh_grey) {
            return Err(format!(
                "manifest row {row} frame {index} first grey differed at {diff}"
            ));
        }
        let sample_len = viewer
            .cached_sample(id, index)
            .ok_or_else(|| format!("manifest row {row} frame {index} was not cached"))?
            .1;
        if !viewer.evict_frame_to_free_list(id, index) {
            return Err(format!(
                "manifest row {row} frame {index} could not be evicted"
            ));
        }
        let parked = viewer
            .free_sample_ptr(sample_len)
            .ok_or_else(|| format!("manifest row {row} frame {index} buffer was not parked"))?;
        // The parked buffer still holds this frame's samples, so a decode that
        // skipped a byte would reproduce it by accident. 0xA5 is not that.
        if viewer.poison_free_samples(sample_len, 0xA5) == 0 {
            return Err(format!(
                "manifest row {row} frame {index} had no free buffer to poison"
            ));
        }
        let again_id = if viewer.file_bytes(id).is_some() {
            id
        } else {
            viewer.insert_file(bytes.to_vec()).ok_or_else(|| {
                format!("manifest row {row} frame {index} could not be posted again")
            })?
        };
        let reused_before = viewer.buffer_claims().0;
        let (second, _) = preview_held(viewer, again_id, request, None);
        if second.status != 200 {
            return Err(format!(
                "manifest row {row} frame {index} recycled preview status was {}",
                second.status
            ));
        }
        if let Some(diff) = first_grey_diff(&second.body, fresh_grey) {
            return Err(format!(
                "manifest row {row} frame {index} recycled grey differed at {diff}"
            ));
        }
        let (addr, len) = viewer.cached_sample(again_id, index).ok_or_else(|| {
            format!("manifest row {row} frame {index} was not cached after reuse")
        })?;
        if addr != parked || len != sample_len {
            return Err(format!(
                "manifest row {row} frame {index} recycled pointer {addr} length {len}"
            ));
        }
        if viewer.buffer_claims().0 <= reused_before {
            return Err(format!(
                "manifest row {row} frame {index} allocated a new sample buffer"
            ));
        }
        let _ = viewer.evict_frame_to_free_list(again_id, index);
        Ok(())
    }

    fn first_grey_diff(got: &[u8], fresh: &[u8]) -> Option<usize> {
        if got.len() != fresh.len() {
            return Some(got.len().min(fresh.len()));
        }
        got.iter()
            .zip(fresh.iter())
            .position(|(left, right)| left != right)
    }

    fn wait_for_preview(viewer: &Viewer, before: u64) -> Result<(), String> {
        let started = std::time::Instant::now();
        while viewer.preview_calls() == before {
            if started.elapsed() > Duration::from_secs(2) {
                return Err("a preview did not reach the held file".to_owned());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    fn post_bytes(addr: std::net::SocketAddr, bytes: &[u8]) -> Result<String, String> {
        let mut request = format!(
            "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        )
        .into_bytes();
        request.extend_from_slice(bytes);
        read_http(addr, &request)
    }

    fn request_preview(addr: std::net::SocketAddr, id: &str) -> Result<String, String> {
        let request = format!(
            "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        read_http(addr, request.as_bytes())
    }

    fn request_headers(addr: std::net::SocketAddr, id: &str) -> Result<String, String> {
        let request = format!(
            "POST /headers HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        read_http(addr, request.as_bytes())
    }

    fn read_http(addr: std::net::SocketAddr, request: &[u8]) -> Result<String, String> {
        let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| error.to_string())?;
        stream
            .write_all(request)
            .map_err(|error| error.to_string())?;
        let mut buf = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let Some(got) = chunk.get(..n) else {
                        return Err("the response read was short".to_owned());
                    };
                    buf.extend_from_slice(got);
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    fn status_is(response: &str, status: u16) -> bool {
        response.starts_with(&format!("HTTP/1.1 {status} "))
    }

    fn header_value(response: &str, name: &str) -> Option<String> {
        let header = response.split("\r\n\r\n").next().unwrap_or(response);
        for line in header.split("\r\n") {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            if key.eq_ignore_ascii_case(name) {
                return Some(value.trim().to_owned());
            }
        }
        None
    }

    /// One Part 10 monochrome frame of `rows` by `columns`, 16-bit.
    fn grey_file(rows: u16, columns: u16) -> Result<Vec<u8>, String> {
        let short = |group: u16, element: u16, vr: [u8; 2], value: &[u8]| {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&group.to_le_bytes());
            bytes.extend_from_slice(&element.to_le_bytes());
            bytes.extend_from_slice(&vr);
            bytes.extend_from_slice(&u16::try_from(value.len()).unwrap_or(0).to_le_bytes());
            bytes.extend_from_slice(value);
            bytes
        };
        let long = |group: u16, element: u16, vr: [u8; 2], value: &[u8]| {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&group.to_le_bytes());
            bytes.extend_from_slice(&element.to_le_bytes());
            bytes.extend_from_slice(&vr);
            bytes.extend_from_slice(&[0, 0]);
            bytes.extend_from_slice(&u32::try_from(value.len()).unwrap_or(0).to_le_bytes());
            bytes.extend_from_slice(value);
            bytes
        };
        let pad = |text: &[u8]| {
            let mut text = text.to_vec();
            if !text.len().is_multiple_of(2) {
                text.push(b' ');
            }
            text
        };
        let ui = |text: &[u8]| {
            let mut text = text.to_vec();
            if !text.len().is_multiple_of(2) {
                text.push(0);
            }
            text
        };
        let mut data = Vec::new();
        data.extend(short(
            0x0008,
            0x0016,
            *b"UI",
            &ui(b"1.2.840.10008.5.1.4.1.1.2"),
        ));
        data.extend(short(0x0008, 0x0018, *b"UI", &ui(b"2.25.216")));
        data.extend(short(0x0008, 0x0060, *b"CS", &pad(b"CT")));
        data.extend(short(0x0028, 0x0002, *b"US", &1_u16.to_le_bytes()));
        data.extend(short(0x0028, 0x0004, *b"CS", &pad(b"MONOCHROME2")));
        data.extend(short(0x0028, 0x0010, *b"US", &rows.to_le_bytes()));
        data.extend(short(0x0028, 0x0011, *b"US", &columns.to_le_bytes()));
        data.extend(short(0x0028, 0x0100, *b"US", &16_u16.to_le_bytes()));
        data.extend(short(0x0028, 0x0101, *b"US", &16_u16.to_le_bytes()));
        data.extend(short(0x0028, 0x0102, *b"US", &15_u16.to_le_bytes()));
        data.extend(short(0x0028, 0x0103, *b"US", &0_u16.to_le_bytes()));
        data.extend(short(0x0028, 0x1050, *b"DS", &pad(b"40")));
        data.extend(short(0x0028, 0x1051, *b"DS", &pad(b"400")));
        data.extend(short(0x0028, 0x1052, *b"DS", &pad(b"0")));
        data.extend(short(0x0028, 0x1053, *b"DS", &pad(b"1")));
        data.extend(short(0x0028, 0x1056, *b"CS", &pad(b"LINEAR")));
        let pixels = vec![50_u8; usize::from(rows) * usize::from(columns) * 2];
        data.extend(long(0x7FE0, 0x0010, *b"OW", &pixels));
        let mut meta = Vec::new();
        meta.extend(long(0x0002, 0x0001, *b"OB", &[0, 1]));
        meta.extend(short(
            0x0002,
            0x0002,
            *b"UI",
            &ui(b"1.2.840.10008.5.1.4.1.1.2"),
        ));
        meta.extend(short(0x0002, 0x0003, *b"UI", &ui(b"2.25.216")));
        meta.extend(short(0x0002, 0x0010, *b"UI", &ui(b"1.2.840.10008.1.2.1")));
        meta.extend(short(0x0002, 0x0012, *b"UI", &ui(b"2.25.217")));
        let mut bytes = vec![0_u8; 128];
        bytes.extend_from_slice(b"DICM");
        bytes.extend(short(
            0x0002,
            0x0000,
            *b"UL",
            &u32::try_from(meta.len()).unwrap_or(0).to_le_bytes(),
        ));
        bytes.extend_from_slice(&meta);
        bytes.extend_from_slice(&data);
        Ok(bytes)
    }

    fn send_late(addr: std::net::SocketAddr, headers: &str) -> Result<TcpStream, String> {
        let mut late = TcpStream::connect(addr).map_err(|error| error.to_string())?;
        late.set_read_timeout(Some(Duration::from_secs(15)))
            .map_err(|error| error.to_string())?;
        late.write_all(headers.as_bytes())
            .map_err(|error| error.to_string())?;
        Ok(late)
    }

    /// Read one reply whole, failing past 15 s from `started`.
    fn read_late(
        mut late: TcpStream,
        started: std::time::Instant,
        name: &str,
    ) -> Result<String, String> {
        let mut reply = Vec::new();
        let mut chunk = [0_u8; 64 * 1024];
        loop {
            match late.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let Some(got) = chunk.get(..n) else {
                        return Err(format!("{name} read was short"));
                    };
                    reply.extend_from_slice(got);
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut =>
                {
                    return Err(format!("{name} was never answered"));
                }
                Err(error) => return Err(error.to_string()),
            }
            if started.elapsed() > Duration::from_secs(15) {
                return Err(format!("{name} answered too late"));
            }
        }
        Ok(String::from_utf8_lossy(&reply).into_owned())
    }

    /// Grey-sized bytes the two workspaces may hold beside the cap: each
    /// keeps native bytes of two greys, four bytes of scratch per pixel and
    /// one returned grey, and the cap is two more.
    const WORKSPACE_GREYS: usize = 16;

    /// Twenty clients that stop reading hold at most the cap's grey bytes,
    /// and a later preview is answered, never left waiting without end.
    #[test]
    fn twenty_stalled_readers_never_exceed_the_grey_cap() -> Result<(), String> {
        let file = grey_file(3000, 2900)?;
        let grey_len = preview_frame(&file, FrameRequest::default())
            .map_err(|error| format!("the file did not decode: {error}"))?
            .grey
            .len();
        // The stalled replies must be past two socket buffers to block.
        if grey_len <= 8 * 1024 * 1024 {
            return Err(format!("the grey was only {grey_len} bytes"));
        }
        // The cap is the formula's grey term: twice the largest grey.
        let cap = 2 * grey_len;

        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        let addr = listener.local_addr().map_err(|error| error.to_string())?;
        let viewer = Arc::new(Viewer::server(ViewerConfig {
            cache_bytes: 64 * 1024 * 1024,
        }));
        let serving = Arc::clone(&viewer);
        std::thread::spawn(move || {
            let _ = serve_on(listener, serving);
        });
        let preview_headers = |id: &str| {
            format!(
                "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
        };
        let posted = post_bytes(addr, &file)?;
        if !status_is(&posted, 200) {
            return Err("the file was not accepted".to_owned());
        }
        let id = header_value(&posted, "X-Ocelli-File")
            .ok_or("the file id was missing")?
            .trim()
            .to_owned();
        let headers = preview_headers(&id);

        // Twenty clients ask and never read. Their greys would be twenty
        // times the cap without it.
        let mut stalls = Vec::new();
        for _ in 0..20 {
            let mut stall = TcpStream::connect(addr).map_err(|error| error.to_string())?;
            stall
                .write_all(headers.as_bytes())
                .map_err(|error| error.to_string())?;
            stalls.push(stall);
            std::thread::sleep(Duration::from_millis(100));
        }
        // The decodes and renders are done long before this point.
        std::thread::sleep(Duration::from_secs(2));
        for _ in 0..20 {
            let census = viewer.memory_census();
            if census.grey > cap {
                return Err(format!(
                    "the replies held {} grey bytes over {cap}",
                    census.grey
                ));
            }
            // `workspace` counts every grey a reply took from a worker, so
            // it sees greys the gate's own counter would miss. Twenty
            // unbounded replies would be twenty greys here.
            if census.workspace > WORKSPACE_GREYS * grey_len {
                return Err(format!(
                    "the workspaces held {} bytes, over {WORKSPACE_GREYS} greys",
                    census.workspace
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // A second file, posted now, has no decoded frame yet. Its preview
        // decodes and then meets the full cap. While it waits, its frame is
        // in the cache and nothing it decoded is in flight.
        let before = viewer.memory_census();
        let fresh_posted = post_bytes(addr, &file)?;
        let fresh_id = header_value(&fresh_posted, "X-Ocelli-File")
            .ok_or("the second file id was missing")?
            .trim()
            .to_owned();
        let started = std::time::Instant::now();
        let late = send_late(addr, &headers)?;
        let late_fresh = send_late(addr, &preview_headers(&fresh_id))?;
        let mut waited = None;
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(50));
            let census = viewer.memory_census();
            if census.frames > before.frames {
                waited = Some(census);
                break;
            }
        }
        let waiting = waited.ok_or("the second frame never reached the cache")?;
        std::thread::sleep(Duration::from_millis(300));
        let waiting_now = viewer.memory_census();
        if waiting_now.inflight != before.inflight || waiting.inflight != before.inflight {
            return Err(format!(
                "a waiting preview kept {} bytes in flight, against {} before",
                waiting_now.inflight, before.inflight
            ));
        }
        // Capacity frees only once the stalled writes fail.
        drop(stalls);
        for (name, stream) in [
            ("the later preview", late),
            ("the fresh preview", late_fresh),
        ] {
            let text = read_late(stream, started, name)?;
            if !text.starts_with("HTTP/1.1 200 ") && !text.starts_with("HTTP/1.1 503 ") {
                return Err(format!(
                    "{name} answered {}",
                    text.lines().next().unwrap_or("")
                ));
            }
        }
        // Every reply is written or abandoned now, so no reservation is
        // left. A reservation leaked on any path above stays counted.
        let settled = std::time::Instant::now();
        loop {
            let held = viewer.memory_census().grey;
            if held == 0 {
                break;
            }
            if held > cap || settled.elapsed() > Duration::from_secs(5) {
                return Err(format!("after every reply the gate still held {held}"));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    }
}
