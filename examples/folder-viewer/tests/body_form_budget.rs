//! A body-form preview never evicts or steals at a full budget.
//!
//! `OCELLI_CACHE_MIB=16` holds six ids. One body-form preview of an unheld
//! file whose decoded samples exceed the whole budget must leave all six
//! ids answering 200. The body form takes a parked free buffer or allocates,
//! and never makes room, because its frame is never stored.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ROWS: u16 = 3000;
const COLUMNS: u16 = 2900;
/// The decoded samples are 3000 * 2900 * 2 bytes, over the whole budget.
const BUDGET_MIB: &str = "16";

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

fn part10(rows: u16, columns: u16) -> Result<Vec<u8>, String> {
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

/// Start the real server binary and return it with its loopback port.
fn start_server() -> Result<(Child, u16), String> {
    let exe = env!("CARGO_BIN_EXE_ocelli-folder-viewer");
    let mut child = Command::new(exe)
        .env("OCELLI_PORT", "0")
        .env("OCELLI_CACHE_MIB", BUDGET_MIB)
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
        .set_read_timeout(Some(Duration::from_secs(20)))
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

/// Hold one file and return its id, with its frame decoded and cached.
fn hold(port: u16, file: &[u8], index: usize) -> Result<String, String> {
    let posted = request(
        port,
        &format!(
            "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            file.len()
        ),
        file,
        "the held post",
    )?;
    if !status_is(&posted, 200) {
        return Err(format!(
            "held {index} was not accepted: {}",
            String::from_utf8_lossy(&posted)
        ));
    }
    let id = header_value(&posted, "X-Ocelli-File:")
        .ok_or_else(|| format!("held {index} had no file id"))?;
    let preview = request(
        port,
        &format!(
            "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
        &[],
        "the held preview",
    )?;
    if !status_is(&preview, 200) {
        return Err(format!("held {index} did not preview: {id}"));
    }
    Ok(id)
}

#[test]
fn a_body_form_preview_of_an_over_budget_file_evicts_nothing() -> Result<(), String> {
    let small = part10(64, 64)?;
    let large = part10(ROWS, COLUMNS)?;
    let (mut child, port) = start_server()?;
    let outcome = hold_six_and_preview(port, &small, &large);
    let stopped = child.kill().is_ok();
    let _ = child.wait();
    if !stopped {
        return Err("the server could not be stopped by pid".to_owned());
    }
    outcome
}

fn hold_six_and_preview(port: u16, small: &[u8], large: &[u8]) -> Result<(), String> {
    let mut ids = Vec::new();
    for index in 0..6 {
        ids.push(hold(port, small, index)?);
    }
    // One body-form preview of an unheld file whose samples exceed the whole
    // budget. It must not empty the cache.
    let body = request(
        port,
        &format!(
            "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            large.len()
        ),
        large,
        "the body-form preview",
    )?;
    if !status_is(&body, 200) {
        return Err(format!(
            "the body-form preview did not answer: {}",
            String::from_utf8_lossy(&body)
        ));
    }
    for (index, id) in ids.iter().enumerate() {
        let again = request(
            port,
            &format!(
                "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            &[],
            "the held id preview",
        )?;
        if !status_is(&again, 200) {
            return Err(format!(
                "held id {index} answered {} after the body-form preview: the page would re-post",
                String::from_utf8_lossy(&again).lines().next().unwrap_or("")
            ));
        }
    }
    Ok(())
}
