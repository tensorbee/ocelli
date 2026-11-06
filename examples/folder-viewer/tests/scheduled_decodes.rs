//! The census under a body-form preview storm with no cache.
//!
//! `OCELLI_CACHE_MIB=0` makes `/file` answer `X-Ocelli-Held: 0`, so the page
//! sends every preview with the file's bytes. Every one of those decodes must
//! run on the two decode workers: the census never shows more than two decode
//! threads and never more than two decodes at once.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use ocelli_folder_viewer::{FrameRequest, preview_frame};

const ROWS: u16 = 64;
const COLUMNS: u16 = 64;
const CLIENTS: usize = 8;
const ROUNDS: usize = 25;

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

fn small_part10() -> Result<Vec<u8>, String> {
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
    let meta_length =
        u32::try_from(meta.len()).map_err(|_| "the meta group was too long".to_owned())?;
    bytes.extend(short(0x0002, 0x0000, *b"UL", &meta_length.to_le_bytes())?);
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

/// Start the real server binary and return it with its loopback port.
fn start_server(census: &std::path::Path) -> Result<(Child, u16), String> {
    let exe = env!("CARGO_BIN_EXE_ocelli-folder-viewer");
    let mut child = Command::new(exe)
        .env("OCELLI_PORT", "0")
        .env("OCELLI_CACHE_MIB", "0")
        .env("OCELLI_CENSUS_LOG", census)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut stdout = child.stdout.take().ok_or("the server printed nothing")?;
    let started = Instant::now();
    let mut line = Vec::new();
    loop {
        let mut byte = [0_u8; 1];
        let n = stdout
            .read(&mut byte)
            .map_err(|error| format!("the server stdout failed: {error}"))?;
        if n == 0 {
            return Err("the server closed its stdout".to_owned());
        }
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
        if started.elapsed() > Duration::from_secs(30) {
            return Err("the server did not print its URL in time".to_owned());
        }
    }
    let url = String::from_utf8_lossy(&line).into_owned();
    let port_text = url
        .trim()
        .strip_prefix("http://127.0.0.1:")
        .and_then(|rest| rest.strip_suffix('/'))
        .ok_or_else(|| format!("the server URL was not loopback: {url}"))?;
    let port = port_text
        .parse::<u16>()
        .map_err(|error| error.to_string())?;
    Ok((child, port))
}

fn request(port: u16, headers: &str, body: &[u8], stage: &str) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
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

fn header_bytes(response: &[u8]) -> &[u8] {
    let end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or(response.len());
    response.get(..end).unwrap_or(response)
}

fn header_value(response: &[u8], name: &str) -> Option<String> {
    let text = String::from_utf8_lossy(header_bytes(response));
    text.lines()
        .find_map(|line| line.strip_prefix(name))
        .map(|value| value.trim().to_owned())
}

fn body_of(response: &[u8]) -> &[u8] {
    let end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
        .unwrap_or(header_bytes(response).len());
    response.get(end..).unwrap_or(&[])
}

#[test]
fn the_census_stays_on_two_decode_threads() -> Result<(), String> {
    let out = std::env::temp_dir().join(format!(
        "f216-census-{}-{}.log",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let (mut child, port) = start_server(&out)?;
    let result = storm(port);
    let stopped = child.kill().is_ok();
    let _ = child.wait();
    let census = std::fs::read_to_string(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    result?;

    let mut lines = 0_usize;
    let mut saw_decode = false;
    for line in census.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            return Err(format!("the census line was short: {line}"));
        }
        lines += 1;
        let threads: usize = fields
            .get(1)
            .and_then(|text| text.parse().ok())
            .ok_or("decode_threads was not a count")?;
        let peak: usize = fields
            .get(2)
            .and_then(|text| text.parse().ok())
            .ok_or("peak_decodes was not a count")?;
        if peak > 0 {
            saw_decode = true;
        }
        if threads > 2 {
            return Err(format!("the census showed {threads} decode threads"));
        }
        if peak > 2 {
            return Err(format!("the census showed {peak} decodes at once"));
        }
    }
    if !stopped {
        return Err("the server could not be stopped by pid".to_owned());
    }
    if lines == 0 {
        return Err("the census log had no lines".to_owned());
    }
    if !saw_decode {
        return Err("the census never saw a decode".to_owned());
    }
    Ok(())
}

/// Post one file, then storm body-form previews and check every reply.
fn storm(port: u16) -> Result<(), String> {
    let file = small_part10()?;
    let fresh = preview_frame(&file, FrameRequest::default())
        .map_err(|error| format!("the small file did not decode: {error}"))?;
    let expected = fresh.grey.clone();
    let posted = request(
        port,
        &format!(
            "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            file.len()
        ),
        &file,
        "the file post",
    )?;
    if !status_is(&posted, 200) {
        return Err(format!(
            "the file was not accepted: {}",
            String::from_utf8_lossy(&posted)
        ));
    }
    if header_value(&posted, "X-Ocelli-Held:") != Some("0".to_owned()) {
        return Err("a zero budget still held the file".to_owned());
    }
    let headers = format!(
        "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        file.len()
    );
    let mut clients = Vec::new();
    for index in 0..CLIENTS {
        let headers = headers.clone();
        let expected = expected.clone();
        let body_file = file.clone();
        clients.push(std::thread::spawn(move || {
            for _ in 0..ROUNDS {
                let reply = request(port, &headers, &body_file, &format!("client {index}"))?;
                if !status_is(&reply, 200) {
                    return Err(format!(
                        "client {index} saw {}",
                        String::from_utf8_lossy(&reply)
                    ));
                }
                if header_value(&reply, "X-Ocelli-Source:") != Some("body".to_owned()) {
                    return Err(format!("client {index} lost its source header"));
                }
                if body_of(&reply) != expected.as_slice() {
                    return Err(format!("client {index} saw a grey that differed"));
                }
            }
            Ok(())
        }));
    }
    for (index, client) in clients.into_iter().enumerate() {
        client
            .join()
            .map_err(|_| format!("client {index} panicked"))??;
    }
    Ok(())
}
