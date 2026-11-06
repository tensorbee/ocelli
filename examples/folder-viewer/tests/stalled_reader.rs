//! A client that stops reading holds no decode permit.
//!
//! One client requests a large grey and never reads it. Six previews of the
//! same frame must still answer. On the tree before F-216 pass 4 remediation
//! the permit came back only after the reply was written, so the six waited
//! for the stalled write.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ocelli_folder_viewer::{FrameRequest, Viewer, ViewerConfig, preview_frame, serve_on};

const ROWS: u16 = 3000;
const COLUMNS: u16 = 2900;
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

fn large_part10() -> Result<Vec<u8>, String> {
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
    data.extend(short(0x0028, 0x0010, *b"US", &ROWS.to_le_bytes())?);
    data.extend(short(0x0028, 0x0011, *b"US", &COLUMNS.to_le_bytes())?);
    data.extend(short(0x0028, 0x0100, *b"US", &16_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x0101, *b"US", &16_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x0102, *b"US", &15_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x0103, *b"US", &0_u16.to_le_bytes())?);
    data.extend(short(0x0028, 0x1050, *b"DS", &text_pad(b"40"))?);
    data.extend(short(0x0028, 0x1051, *b"DS", &text_pad(b"400"))?);
    data.extend(short(0x0028, 0x1052, *b"DS", &text_pad(b"0"))?);
    data.extend(short(0x0028, 0x1053, *b"DS", &text_pad(b"1"))?);
    data.extend(short(0x0028, 0x1056, *b"CS", &text_pad(b"LINEAR"))?);
    let pixels = vec![50_u8; usize::from(ROWS) * usize::from(COLUMNS) * 2];
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

#[test]
fn a_reader_that_stops_does_not_hold_the_decode_permit() -> Result<(), String> {
    let file = large_part10()?;
    let fresh = preview_frame(&file, FrameRequest::default())
        .map_err(|error| format!("the large file did not decode: {error}"))?;
    let grey_len = fresh.grey.len();
    // macOS autotunes one socket's buffers to about 4 MiB, so the reply must
    // be larger than two socket buffers can hold for the write to block.
    if grey_len <= 8 * 1024 * 1024 {
        return Err(format!("the grey was only {grey_len} bytes"));
    }

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(Viewer::server(ViewerConfig {
        cache_bytes: 64 * 1024 * 1024,
    }));
    let serving = Arc::clone(&viewer);
    std::thread::spawn(move || {
        let _ = serve_on(listener, serving);
    });

    let file_post_headers = format!(
        "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        file.len()
    );
    let posted = post(addr, &file_post_headers, &file, "the file post")?;
    if !status_is(&posted, 200) {
        return Err("the file was not accepted".to_owned());
    }
    let header_text = String::from_utf8_lossy(&posted).into_owned();
    let id = header_text
        .split("\r\n")
        .find_map(|line| line.strip_prefix("X-Ocelli-File: "))
        .ok_or("the file id was missing")?
        .trim()
        .to_owned();

    // One client asks for the grey and never reads the reply. The decode
    // finishes and the write blocks.
    let stall_headers = format!(
        "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let stall = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    let mut stall = stall;
    stall
        .write_all(stall_headers.as_bytes())
        .map_err(|error| error.to_string())?;
    // The stalled socket is never read from.
    std::thread::sleep(Duration::from_millis(2000));

    // Six previews of the same frame must all answer within the deadline.
    let mut readers = Vec::new();
    for index in 0..6 {
        let headers = stall_headers.clone();
        readers.push(std::thread::spawn(move || {
            let started = Instant::now();
            let reply = post(addr, &headers, &[], &format!("preview {index}"))?;
            let took = started.elapsed();
            eprintln!("preview {index} answered in {:.3}s", took.as_secs_f32());
            if !status_is(&reply, 200) {
                return Err(format!("preview {index} was not 200"));
            }
            if started.elapsed() > DEADLINE {
                return Err(format!(
                    "preview {index} answered after {:.3} s",
                    started.elapsed().as_secs_f32()
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
    // The stalled client is closed only after the others have been judged.
    drop(stall);
    if let Some(first) = failures.first() {
        return Err(first.clone());
    }
    Ok(())
}
