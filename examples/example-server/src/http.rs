//! Loopback listener. It prints only its base URL.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use crate::interpolate::interpolate_reply;
use crate::overlay::overlay_reply;
use crate::reply::reply_body;
use crate::series::series_reply;

const HEADER_CAP: usize = 16 * 1024;
/// One posted file. The cap matches the folder page.
const BODY_CAP: usize = 1024 * 1024 * 1024;

struct Reply {
    status: u16,
    content_type: &'static str,
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
    println!("http://{addr}");
    std::io::stdout().flush()?;
    serve_listener(listener)
}

fn serve_listener(listener: TcpListener) -> std::io::Result<()> {
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
    let reply = match read_body(&mut stream) {
        Ok(loaded) => route(&loaded),
        Err(ReadStop::Io(error)) => return Err(error),
        Err(ReadStop::Reply(reply)) => reply,
    };
    write_reply(&mut stream, &reply)
}

struct Loaded {
    path: String,
    body: Vec<u8>,
    frame: u32,
    content_type: Option<String>,
}

fn route(loaded: &Loaded) -> Reply {
    let (status, json) = if loaded.path == "/series-open" {
        series_reply(&loaded.body, loaded.content_type.as_deref())
    } else if loaded.path == "/overlay" {
        overlay_reply(&loaded.body, loaded.frame)
    } else if loaded.path == "/interpolate" {
        interpolate_reply(&loaded.body)
    } else {
        reply_body(&loaded.body, loaded.frame)
    };
    Reply {
        status,
        content_type: "application/json; charset=utf-8",
        body: json.into_bytes(),
    }
}

fn write_reply(stream: &mut TcpStream, reply: &Reply) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {len}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: content-type\r\nConnection: close\r\n\r\n",
        status = reply.status,
        reason = reason(reply.status),
        ctype = reply.content_type,
        len = reply.body.len(),
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&reply.body)?;
    Ok(())
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    }
}

fn text_reply(status: u16, message: &str) -> Reply {
    Reply {
        status,
        content_type: "text/plain; charset=utf-8",
        body: message.as_bytes().to_vec(),
    }
}

fn read_body(stream: &mut TcpStream) -> Result<Loaded, ReadStop> {
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
                "the frame-open header is too large",
            )));
        }
    };
    let Some(header_bytes) = buf.get(..header_end) else {
        return Err(ReadStop::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "header",
        )));
    };
    let header = std::str::from_utf8(header_bytes)
        .map_err(|_| ReadStop::Reply(text_reply(400, "the frame-open header is not text")))?;
    let header = parse_header(header)?;
    if header.method == "OPTIONS" && open_path(&header.path) {
        return Err(ReadStop::Reply(Reply {
            status: 204,
            content_type: "text/plain; charset=utf-8",
            body: Vec::new(),
        }));
    }
    if header.method != "POST" || !open_path(&header.path) {
        return Err(ReadStop::Reply(text_reply(404, "not found")));
    }
    let (too_large, no_length) = body_messages(&header.path);
    let declared = match header.length {
        Some(value) if value > BODY_CAP => {
            return Err(ReadStop::Reply(text_reply(400, too_large)));
        }
        Some(value) => value,
        None => {
            return Err(ReadStop::Reply(text_reply(400, no_length)));
        }
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
                "the frame-open request ended early",
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
    Ok(Loaded {
        path: header.path,
        body,
        frame: header.frame,
        content_type: header.content_type,
    })
}

fn header_end_at(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index.saturating_add(4))
}

struct Header {
    method: String,
    path: String,
    frame: u32,
    length: Option<usize>,
    content_type: Option<String>,
}

fn parse_header(header: &str) -> Result<Header, ReadStop> {
    let mut lines = header.split("\r\n");
    let Some(request) = lines.next() else {
        return Err(ReadStop::Reply(text_reply(
            400,
            "the frame-open request is empty",
        )));
    };
    let mut parts = request.split(' ');
    let Some(method) = parts.next() else {
        return Err(ReadStop::Reply(text_reply(
            400,
            "the frame-open request is empty",
        )));
    };
    let Some(raw_path) = parts.next() else {
        return Err(ReadStop::Reply(text_reply(
            400,
            "the frame-open request has no path",
        )));
    };
    let (path, frame) = split_path(raw_path)?;
    let mut length = None;
    let mut seen_length = false;
    let mut content_type = None;
    let mut seen_type = false;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ReadStop::Reply(text_reply(
                400,
                "the frame-open header is not usable",
            )));
        };
        if name.eq_ignore_ascii_case("content-length") {
            if seen_length {
                return Err(ReadStop::Reply(text_reply(
                    400,
                    "the frame-open request has two lengths",
                )));
            }
            seen_length = true;
            let text = value.trim_matches(' ');
            let parsed = text.parse::<usize>().map_err(|_| {
                ReadStop::Reply(text_reply(400, "the frame-open length is not a number"))
            })?;
            length = Some(parsed);
        }
        if name.eq_ignore_ascii_case("content-type") {
            if seen_type {
                return Err(ReadStop::Reply(text_reply(
                    400,
                    "the request has two content types",
                )));
            }
            seen_type = true;
            content_type = Some(value.trim_matches(' ').to_owned());
        }
    }
    Ok(Header {
        method: method.to_owned(),
        path,
        frame,
        length,
        content_type,
    })
}

fn open_path(path: &str) -> bool {
    path == "/frame-open" || path == "/series-open" || path == "/overlay" || path == "/interpolate"
}

fn body_messages(path: &str) -> (&'static str, &'static str) {
    if path == "/series-open" {
        (
            "the series-open request is too large",
            "the series-open request has no length",
        )
    } else if path == "/overlay" {
        (
            "the overlay request is too large",
            "the overlay request has no length",
        )
    } else if path == "/interpolate" {
        (
            "the interpolate request is too large",
            "the interpolate request has no length",
        )
    } else {
        (
            "the frame-open request is too large",
            "the frame-open request has no length",
        )
    }
}

fn split_path(raw_path: &str) -> Result<(String, u32), ReadStop> {
    let (path, query) = match raw_path.split_once('?') {
        Some((path, query)) => (path.to_owned(), Some(query)),
        None => (raw_path.to_owned(), None),
    };
    let Some(query) = query else {
        return Ok((path, 0));
    };
    let mut frame = 0_u32;
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let Some((name, value)) = pair.split_once('=') else {
            return Err(ReadStop::Reply(text_reply(
                400,
                "the frame-open query is not usable",
            )));
        };
        if name == "frame" {
            frame = value.parse::<u32>().map_err(|_| {
                ReadStop::Reply(text_reply(400, "the frame-open frame is not a number"))
            })?;
        }
    }
    Ok((path, frame))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use super::{BODY_CAP, handle_connection};

    fn exchange(request: &[u8]) -> String {
        let bound = TcpListener::bind("127.0.0.1:0");
        assert!(bound.is_ok());
        let Ok(listener) = bound else {
            unreachable!("asserted immediately above")
        };
        let address = listener.local_addr();
        assert!(address.is_ok());
        let Ok(address) = address else {
            unreachable!("asserted immediately above")
        };
        let worker = thread::spawn(move || {
            let accepted = listener.accept();
            assert!(accepted.is_ok());
            let Ok((stream, _)) = accepted else {
                unreachable!("asserted immediately above")
            };
            assert!(handle_connection(stream).is_ok());
        });
        let mut stream = TcpStream::connect(address).unwrap_or_else(|_| panic_connect());
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        assert!(stream.write_all(request).is_ok());
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
        assert!(worker.join().is_ok());
        String::from_utf8_lossy(&response).into_owned()
    }

    fn panic_connect() -> TcpStream {
        unreachable!("the helper accepted on the bound port")
    }

    #[test]
    fn synthetic_post_returns_the_hand_value() {
        let request =
            b"POST /frame-open HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 9\r\n\r\nsynthetic";
        let response = exchange(request);
        assert!(response.contains("200"));
        assert!(response.contains("\"display\":63.75"));
        assert!(response.contains("\"padding_count\":1"));
    }

    #[test]
    fn an_over_cap_length_is_refused_before_a_body() {
        let request = format!(
            "POST /frame-open HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
            BODY_CAP.saturating_add(1)
        );
        let response = exchange(request.as_bytes());
        assert!(response.contains("400"));
        assert!(response.contains("too large"));
        assert!(!response.contains("\"display\""));
    }

    #[test]
    fn series_open_options_is_empty() {
        let request = b"OPTIONS /series-open HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let response = exchange(request);
        assert!(response.contains("204"));
    }

    #[test]
    fn synthetic_series_post_returns_the_axial_order() {
        let request =
            b"POST /series-open HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 9\r\n\r\nsynthetic";
        let response = exchange(request);
        assert!(response.contains("200"));
        assert!(
            response
                .contains("\"slots\":[[{\"input\":1,\"frame\":0}],[{\"input\":0,\"frame\":0}]]")
        );
        assert!(response.contains("\"milliseconds\":100"));
        assert!(response.contains("\"playback\":\"sweep\""));
    }

    #[test]
    fn overlay_options_is_empty() {
        let request = b"OPTIONS /overlay HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let response = exchange(request);
        assert!(response.contains("204"));
    }

    #[test]
    fn synthetic_overlay_post_reports_the_hit_and_the_hide() {
        let request =
            b"POST /overlay HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 9\r\n\r\nsynthetic";
        let response = exchange(request);
        assert!(response.contains("200"));
        assert!(response.contains("\"hit\":true"));
        assert!(response.contains("\"hidden\":true"));
        assert!(response.contains("\"presentation\":null"));
        assert!(!response.contains("\"presentation\":0"));
        assert!(response.contains("\"mask\":[1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]"));
    }

    #[test]
    fn overlay_frame_query_past_the_end_is_refused() {
        let request = b"POST /overlay?frame=1 HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 9\r\n\r\nsynthetic";
        let response = exchange(request);
        assert!(response.contains("400"));
        assert!(response.contains("\"error\":\"frame_out_of_range\""));
        assert!(response.contains("\"frame\":1"));
    }

    #[test]
    fn an_over_cap_overlay_length_names_the_overlay_route() {
        let request = format!(
            "POST /overlay HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
            BODY_CAP.saturating_add(1)
        );
        let response = exchange(request.as_bytes());
        assert!(response.contains("400"));
        assert!(response.contains("the overlay request is too large"));
    }

    #[test]
    fn interpolate_options_is_empty() {
        let request = b"OPTIONS /interpolate HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let response = exchange(request);
        assert!(response.contains("204"));
    }

    #[test]
    fn synthetic_interpolate_post_reports_both_modes_and_the_tiers() {
        let request =
            b"POST /interpolate HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 9\r\n\r\nsynthetic";
        let response = exchange(request);
        assert!(response.contains("200"));
        assert!(response.contains("\"nearest\":0"));
        assert!(response.contains("\"linear\":138.75"));
        assert!(response.contains("\"tier\":\"A\""));
        assert!(response.contains("\"tier\":\"B\""));
        assert!(response.contains("\"tier\":\"C\""));
        assert!(response.contains("\"nearest\":\"available\""));
        assert!(response.contains("\"linear\":\"available\""));
    }
}
