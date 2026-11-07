//! Two clients that stop reading do not stop other previews.
//!
//! Each of two clients asks for a 4 MiB grey and never reads it. They hold
//! two 4 MiB greys under the grey cap, which is `P * N_max`, 17,400,000
//! bytes. Six previews of a 512 by 512 frame fit beside them and must all
//! answer 200 well inside the stalled writes' own bound. On the tree before
//! F-216 pass 4 remediation the decode permit came back only after the
//! reply was written, so both workers stayed held and the six waited. On
//! the pass 6 tree the cap was twice the largest grey seen, 8 MiB here, so
//! the two stalled greys filled it and the six waited for the write timeout.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ocelli_folder_viewer::{FrameRequest, Viewer, ViewerConfig, preview_frame, serve_on};

/// One stalled grey is 4 MiB.
const STALL_ROWS: u16 = 2048;
const STALL_COLUMNS: u16 = 2048;
/// Each stalled client pipelines this many previews. Socket buffers may
/// absorb the first replies whole, and the first one they cannot absorb
/// blocks its write, so each client then holds exactly one grey.
const PIPELINED: usize = 4;
/// The six answers carry a 262,144 byte grey.
const SIX_ROWS: u16 = 512;
const SIX_COLUMNS: u16 = 512;
/// The grey cap, `P * N_max`, with `N_max` the formula's 3000 by 2900 frame.
const GREY_CAP: usize = 2 * 3000 * 2900;
/// Answers must arrive well inside the stalled write's own bound.
const DEADLINE: Duration = Duration::from_secs(5);

fn ui(value: &[u8]) -> Vec<u8> {
    let mut text = value.to_vec();
    if !text.len().is_multiple_of(2) {
        text.push(0);
    }
    text
}

/// Text VRs pad with spaces. A trailing NUL in a DS value is not a decimal.
fn text_pad(value: &[u8]) -> Vec<u8> {
    let mut text = value.to_vec();
    if !text.len().is_multiple_of(2) {
        text.push(b' ');
    }
    text
}

fn short(group: u16, element: u16, vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    let length =
        u16::try_from(value.len()).map_err(|_| "a short element was too long".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&group.to_le_bytes());
    bytes.extend_from_slice(&element.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn long(group: u16, element: u16, vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    let length =
        u32::try_from(value.len()).map_err(|_| "a long element was too long".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&group.to_le_bytes());
    bytes.extend_from_slice(&element.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn large_part10(rows: u16, columns: u16) -> Result<Vec<u8>, String> {
    let mut meta = Vec::new();
    meta.extend(long(0x0002, 0x0001, *b"OB", &[0, 1])?);
    meta.extend(short(
        0x0002,
        0x0002,
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    meta.extend(short(0x0002, 0x0003, *b"UI", &ui(b"2.25.216"))?);
    meta.extend(short(0x0002, 0x0010, *b"UI", &ui(b"1.2.840.10008.1.2.1"))?);
    meta.extend(short(0x0002, 0x0012, *b"UI", &ui(b"2.25.217"))?);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(short(
        0x0002,
        0x0000,
        *b"UL",
        &u32::try_from(meta.len())
            .map_err(|_| "the meta group was too long".to_owned())?
            .to_le_bytes(),
    )?);
    bytes.extend_from_slice(&meta);
    let mut data = Vec::new();
    data.extend(short(
        0x0008,
        0x0016,
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short(0x0008, 0x0018, *b"UI", &ui(b"2.25.216"))?);
    data.extend(short(0x0008, 0x0060, *b"CS", &text_pad(b"CT"))?);
    data.extend(short(0x0028, 0x0002, *b"US", &1_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x0004, *b"CS", &text_pad(b"MONOCHROME2"))?);
    data.extend(short(0x0028, 0x0010, *b"US", &rows.to_le_bytes())?);
    data.extend(short(0x0028, 0x0011, *b"US", &columns.to_le_bytes())?);
    data.extend(short(0x0028, 0x0100, *b"US", &16_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x0101, *b"US", &16_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x0102, *b"US", &15_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x0103, *b"US", &0_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x1050, *b"DS", &text_pad(b"40"))?);
    data.extend(short(0x0028, 0x1051, *b"DS", &text_pad(b"400"))?);
    data.extend(short(0x0028, 0x1052, *b"DS", &text_pad(b"0"))?);
    data.extend(short(0x0028, 0x1053, *b"DS", &text_pad(b"1"))?);
    data.extend(short(0x0028, 0x1056, *b"CS", &text_pad(b"LINEAR"))?);
    let pixels = vec![50_u8; usize::from(rows) * usize::from(columns) * 2];
    data.extend(long(0x7FE0, 0x0010, *b"OW", &pixels)?);
    bytes.extend_from_slice(&data);
    Ok(bytes)
}

fn post(
    addr: std::net::SocketAddr,
    headers: &str,
    body: &[u8],
    stage: &str,
) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(DEADLINE))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(headers.as_bytes())
        .map_err(|error| error.to_string())?;
    stream.write_all(body).map_err(|error| error.to_string())?;
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let Some(got) = chunk.get(..n) else {
                    return Err(format!("{stage} read was short"));
                };
                buf.extend_from_slice(got);
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                return Err(format!("{stage} did not arrive in time"));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(buf)
}

fn status_is(response: &[u8], status: u16) -> bool {
    response.starts_with(format!("HTTP/1.1 {status} ").as_bytes())
}

fn header_value(response: &[u8], name: &str) -> Option<String> {
    let text = String::from_utf8_lossy(response);
    text.lines()
        .find_map(|line| line.strip_prefix(name))
        .map(|value| value.trim().to_owned())
}

/// One number from the `GET /held` census.
fn held_number(addr: std::net::SocketAddr, key: &str) -> Result<usize, String> {
    let reply = post(
        addr,
        "GET /held HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        &[],
        "the census",
    )?;
    let text = String::from_utf8_lossy(&reply).into_owned();
    let needle = format!("\"{key}\":");
    let rest = text
        .split_once(&needle)
        .ok_or_else(|| format!("the census omitted {key}"))?
        .1;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits
        .parse()
        .map_err(|_| format!("the census {key} was not a number"))
}

#[test]
fn two_stalled_readers_do_not_stop_six_small_previews() -> Result<(), String> {
    let stall_file = large_part10(STALL_ROWS, STALL_COLUMNS)?;
    let six_file = large_part10(SIX_ROWS, SIX_COLUMNS)?;
    let stall_grey = preview_frame(&stall_file, FrameRequest::default())
        .map_err(|error| format!("the stall file did not decode: {error}"))?
        .grey
        .len();
    let six_grey = preview_frame(&six_file, FrameRequest::default())
        .map_err(|error| format!("the six file did not decode: {error}"))?
        .grey
        .len();
    if stall_grey != 4 * 1024 * 1024 || six_grey != 512 * 512 {
        return Err(format!("the greys were {stall_grey} and {six_grey} bytes"));
    }
    // Beside two stalled greys, all six fit under the cap at once.
    if 2 * stall_grey + 6 * six_grey > GREY_CAP {
        return Err("the six do not fit beside the stalled greys".to_owned());
    }

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(Viewer::server(ViewerConfig {
        cache_bytes: 128 * 1024 * 1024,
    }));
    let serving = Arc::clone(&viewer);
    std::thread::spawn(move || {
        let _ = serve_on(listener, serving);
    });

    let post_headers = |file: &[u8]| {
        format!(
            "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            file.len()
        )
    };
    let preview_headers = |id: &str, close: &str| {
        format!(
            "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n{close}\r\n"
        )
    };
    let close = "Connection: close\r\n";
    let posted = post(
        addr,
        &post_headers(&stall_file),
        &stall_file,
        "the stall post",
    )?;
    if !status_is(&posted, 200) {
        return Err("the stall file was not accepted".to_owned());
    }
    let stall_id =
        header_value(&posted, "X-Ocelli-File: ").ok_or("the stall file id was missing")?;
    let posted_six = post(addr, &post_headers(&six_file), &six_file, "the six post")?;
    if !status_is(&posted_six, 200) {
        return Err("the six file was not accepted".to_owned());
    }
    let six_id =
        header_value(&posted_six, "X-Ocelli-File: ").ok_or("the six file id was missing")?;

    // Decode both frames once, so the stalls and the six are renders.
    for (id, name) in [
        (&stall_id, "the stall warm-up"),
        (&six_id, "the six warm-up"),
    ] {
        let warm = post(addr, &preview_headers(id, close), &[], name)?;
        if !status_is(&warm, 200) {
            return Err(format!("{name} did not answer"));
        }
    }

    // Two clients pipeline previews of the 4 MiB grey and never read.
    let mut pipelined = String::new();
    for _ in 0..PIPELINED {
        pipelined.push_str(&preview_headers(&stall_id, ""));
    }
    let mut stalls = Vec::new();
    for _ in 0..2 {
        let mut stall = TcpStream::connect(addr).map_err(|error| error.to_string())?;
        stall
            .write_all(pipelined.as_bytes())
            .map_err(|error| error.to_string())?;
        stalls.push(stall);
    }
    // Each stalled client holds exactly one grey in a blocked write.
    let started = Instant::now();
    loop {
        let held = held_number(addr, "grey_replies")?;
        if held == 2 * stall_grey {
            break;
        }
        if started.elapsed() > Duration::from_secs(20) {
            return Err(format!("the stalled replies held {held} grey bytes"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Six previews of the small frame must all answer within the deadline.
    let six_headers = preview_headers(&six_id, close);
    let mut readers = Vec::new();
    for index in 0..6 {
        let headers = six_headers.clone();
        readers.push(std::thread::spawn(move || {
            let started = Instant::now();
            let reply = post(addr, &headers, &[], &format!("preview {index}"))?;
            let took = started.elapsed();
            eprintln!("preview {index} answered in {:.3}s", took.as_secs_f32());
            if !status_is(&reply, 200) {
                return Err(format!("preview {index} was not 200"));
            }
            if took > DEADLINE {
                return Err(format!(
                    "preview {index} answered after {:.3} s",
                    took.as_secs_f32()
                ));
            }
            Ok(())
        }));
    }
    let mut failures = Vec::new();
    for (index, reader) in readers.into_iter().enumerate() {
        match reader.join() {
            Ok(outcome) => {
                if let Err(error) = outcome {
                    failures.push(format!("preview {index}: {error}"));
                }
            }
            Err(_) => failures.push(format!("preview {index} thread panicked")),
        }
    }
    // The stalls still hold their greys, and every grey is under the cap.
    let held = held_number(addr, "grey_replies")?;
    let grey = held_number(addr, "grey")?;
    // The stalled clients are closed only after the others have been judged.
    drop(stalls);
    if let Some(first) = failures.first() {
        return Err(first.clone());
    }
    if held != 2 * stall_grey || grey > GREY_CAP {
        return Err(format!(
            "after the six the replies held {held} and all greys {grey}"
        ));
    }
    Ok(())
}
