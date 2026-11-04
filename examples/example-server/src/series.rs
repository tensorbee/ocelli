//! JSON for `POST /series-open`.
//!
//! The synthetic body is three built-in objects. Any other body is multipart
//! Part 10. Part headers are discarded, so a filename is never copied into the
//! response or a log.

use ocelli_dicom::{
    CineTiming, DisplaySeries, ParsedDicom, ParsedFrame, Playback, SeriesOpenError,
    SeriesOpenReason, SpatialOrder, assemble_display, declared_frame_count, parse_part10,
};

use crate::bytes::synthetic_series_parts;

/// Answer one series-open body.
///
/// `synthetic` runs the built-in trio. Every other body is multipart.
#[must_use]
pub fn series_reply(body: &[u8], content_type: Option<&str>) -> (u16, String) {
    if body == b"synthetic" {
        synthetic_reply()
    } else {
        posted_reply(body, content_type)
    }
}

fn synthetic_reply() -> (u16, String) {
    let parts = match synthetic_series_parts() {
        Ok(parts) => parts,
        Err(_) => return error_json(500, "fixture", "part", 0),
    };
    let mut parsed = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let part_index = match u32::try_from(index) {
            Ok(part_index) => part_index,
            Err(_) => return error_json(500, "fixture", "part", 0),
        };
        match parse_part10(part) {
            Ok(object) => parsed.push(object),
            Err(_) => return error_json(400, "parse", "part", part_index),
        }
    }
    assemble_parsed(&parsed)
}

fn posted_reply(body: &[u8], content_type: Option<&str>) -> (u16, String) {
    let Some(content_type) = content_type else {
        return error_json(400, "multipart", "part", 0);
    };
    let Some(boundary) = multipart_boundary(content_type) else {
        return error_json(400, "multipart", "part", 0);
    };
    let parts = match split_multipart(body, boundary) {
        Ok(parts) => parts,
        Err(()) => return error_json(400, "multipart", "part", 0),
    };
    if parts.is_empty() {
        return error_json(400, "multipart", "part", 0);
    }
    let mut parsed = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let part_index = match u32::try_from(index) {
            Ok(part_index) => part_index,
            Err(_) => return error_json(400, "multipart", "part", 0),
        };
        match parse_part10(part) {
            Ok(object) => parsed.push(object),
            Err(_) => return error_json(400, "parse", "part", part_index),
        }
    }
    assemble_parsed(&parsed)
}

fn assemble_parsed(objects: &[ParsedDicom]) -> (u16, String) {
    let frames = match frames_of(objects) {
        Ok(frames) => frames,
        Err(error) => {
            return error_json(400, reason_code(error.reason()), "part", error.index());
        }
    };
    match assemble_display(&frames) {
        Ok(groups) => (200, success_json(&groups)),
        Err(error) => error_json(400, reason_code(error.reason()), "frame", error.index()),
    }
}

fn frames_of(objects: &[ParsedDicom]) -> Result<Vec<ParsedFrame<'_>>, SeriesOpenError> {
    let mut frames = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        let part_index = u32::try_from(index)
            .map_err(|_| SeriesOpenError::new(SeriesOpenReason::Metadata, 0))?;
        let count = declared_frame_count(object).map_err(|error| error.with_index(part_index))?;
        for frame in 0..count {
            frames.push(ParsedFrame::new(object, frame));
        }
    }
    Ok(frames)
}

fn reason_code(reason: SeriesOpenReason) -> &'static str {
    match reason {
        SeriesOpenReason::Geometry => "geometry",
        SeriesOpenReason::FrameOutOfRange => "frame_out_of_range",
        SeriesOpenReason::Cine => "cine",
        SeriesOpenReason::Metadata => "metadata",
        SeriesOpenReason::Playback => "playback",
        _ => "open",
    }
}

fn success_json(groups: &[DisplaySeries]) -> String {
    let mut out = String::from("{\"groups\":[");
    for (index, group) in groups.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"order\":");
        push_token(&mut out, order_name(group.order()));
        out.push_str(",\"frames\":");
        out.push_str(&group.frame_count().to_string());
        out.push_str(",\"slots\":");
        push_slots(&mut out, group);
        out.push_str(",\"timing\":");
        push_token(&mut out, timing_name(group.timing()));
        push_timing_numbers(&mut out, group.timing());
        out.push_str(",\"playback\":");
        push_token(&mut out, playback_name(group.playback()));
        out.push_str(",\"sheared\":");
        out.push_str(if group.sheared() { "true" } else { "false" });
        out.push_str(",\"non_uniform_step\":");
        out.push_str(if group.non_uniform_step() {
            "true"
        } else {
            "false"
        });
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn push_slots(out: &mut String, group: &DisplaySeries) {
    out.push('[');
    for (slot_index, slot) in group.slots().iter().enumerate() {
        if slot_index > 0 {
            out.push(',');
        }
        out.push('[');
        for (place_index, place) in slot.iter().enumerate() {
            if place_index > 0 {
                out.push(',');
            }
            out.push_str("{\"input\":");
            out.push_str(&place.input_index().to_string());
            out.push_str(",\"frame\":");
            out.push_str(&place.frame_index().to_string());
            out.push('}');
        }
        out.push(']');
    }
    out.push(']');
}

fn push_timing_numbers(out: &mut String, timing: CineTiming) {
    match timing {
        CineTiming::FrameTime { milliseconds } => {
            out.push_str(",\"milliseconds\":");
            out.push_str(&milliseconds.to_string());
        }
        CineTiming::RecommendedDisplayFrameRate { frames_per_second }
        | CineTiming::CineRate { frames_per_second } => {
            out.push_str(",\"frames_per_second\":");
            out.push_str(&frames_per_second.to_string());
        }
        CineTiming::Unavailable | CineTiming::FrameTimeVector => {}
        _ => {}
    }
}

fn order_name(order: SpatialOrder) -> &'static str {
    match order {
        SpatialOrder::Projected => "projected",
        SpatialOrder::InstanceNumber => "instance_number",
        _ => "projected",
    }
}

fn timing_name(timing: CineTiming) -> &'static str {
    match timing {
        CineTiming::Unavailable => "unavailable",
        CineTiming::FrameTimeVector => "frame_time_vector",
        CineTiming::FrameTime { .. } => "frame_time",
        CineTiming::RecommendedDisplayFrameRate { .. } => "recommended_display_frame_rate",
        CineTiming::CineRate { .. } => "cine_rate",
        _ => "unavailable",
    }
}

fn playback_name(playback: Playback) -> &'static str {
    match playback {
        Playback::Loop => "loop",
        Playback::Sweep => "sweep",
        _ => "loop",
    }
}

fn push_token(out: &mut String, token: &str) {
    out.push('"');
    out.push_str(token);
    out.push('"');
}

fn error_json(status: u16, code: &str, field: &str, index: u32) -> (u16, String) {
    let mut out = String::from("{\"error\":");
    push_token(&mut out, code);
    out.push(',');
    push_token(&mut out, field);
    out.push(':');
    out.push_str(&index.to_string());
    out.push('}');
    (status, out)
}

fn multipart_boundary(content_type: &str) -> Option<&str> {
    let mut pieces = content_type.split(';');
    let kind = pieces.next()?.trim();
    if !kind.eq_ignore_ascii_case("multipart/form-data")
        && !kind.eq_ignore_ascii_case("multipart/related")
    {
        return None;
    }
    for piece in pieces {
        let Some((name, value)) = piece.trim().split_once('=') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("boundary") {
            let boundary = value.trim().trim_matches('"');
            if boundary.is_empty() {
                return None;
            }
            return Some(boundary);
        }
    }
    None
}

fn split_multipart<'a>(body: &'a [u8], boundary: &str) -> Result<Vec<&'a [u8]>, ()> {
    let mut marker = Vec::from(b"--");
    marker.extend_from_slice(boundary.as_bytes());
    let positions = marker_positions(body, &marker);
    if positions.len() < 2 {
        return Err(());
    }
    let mut parts = Vec::new();
    for pair in positions.windows(2) {
        let Some(left) = pair.first().copied() else {
            continue;
        };
        let Some(right) = pair.get(1).copied() else {
            continue;
        };
        let after = left.saturating_add(marker.len());
        if body.get(after..after.saturating_add(2)) == Some(b"--") {
            break;
        }
        let mut start = after;
        if body.get(start..start.saturating_add(2)) == Some(b"\r\n") {
            start = start.saturating_add(2);
        }
        let mut end = right;
        if end >= 2 && body.get(end.saturating_sub(2)..end) == Some(b"\r\n") {
            end = end.saturating_sub(2);
        }
        let Some(segment) = body.get(start..end) else {
            return Err(());
        };
        let Some(header_end) = header_end_at(segment) else {
            return Err(());
        };
        let Some(payload) = segment.get(header_end..) else {
            return Err(());
        };
        if payload.is_empty() {
            return Err(());
        }
        parts.push(payload);
    }
    if parts.is_empty() { Err(()) } else { Ok(parts) }
}

fn marker_positions(body: &[u8], marker: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    if marker.is_empty() {
        return positions;
    }
    let mut index = 0;
    while index < body.len() {
        let Some(rest) = body.get(index..) else {
            break;
        };
        if rest.starts_with(marker) {
            positions.push(index);
            index = index.saturating_add(marker.len());
        } else {
            index = index.saturating_add(1);
        }
    }
    positions
}

fn header_end_at(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index.saturating_add(4))
}

#[cfg(test)]
mod tests {
    use super::series_reply;

    #[test]
    fn synthetic_series_reports_two_groups_and_the_axial_order() {
        let (status, body) = series_reply(b"synthetic", None);
        assert_eq!(status, 200);
        assert!(body.contains("\"order\":\"projected\""));
        let axial = "\"slots\":[[{\"input\":1,\"frame\":0}],[{\"input\":0,\"frame\":0}]]";
        let sagittal = "\"slots\":[[{\"input\":2,\"frame\":0}]]";
        let Some(axial_at) = body.find(axial) else {
            unreachable!("axial slots are present")
        };
        let Some(sagittal_at) = body.find(sagittal) else {
            unreachable!("sagittal slot is present")
        };
        assert!(axial_at < sagittal_at);
        assert!(body.contains("\"timing\":\"frame_time\""));
        assert!(body.contains("\"milliseconds\":100"));
        assert!(body.contains("\"playback\":\"loop\""));
        assert!(body.contains("\"playback\":\"sweep\""));
        assert!(!body.contains("frames_per_second"));
        assert_eq!(body.matches("\"order\"").count(), 2);
    }

    #[test]
    fn a_bad_multipart_part_fails_the_request() {
        let (status, synthetic) = series_reply(b"synthetic", None);
        assert_eq!(status, 200);
        let boundary = "ocelli";
        let mut body = Vec::from(
            format!("--{boundary}\r\nContent-Type: application/dicom\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(b"not-dicom");
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let (status, response) = series_reply(&body, Some("multipart/form-data; boundary=ocelli"));
        assert_eq!(status, 400);
        assert!(response.contains("\"error\":\"parse\""));
        assert!(response.contains("\"part\":0"));
        assert!(!response.contains("\"groups\""));
        assert!(synthetic.contains("\"groups\""));
    }

    #[test]
    fn an_assembly_error_names_the_frame_list_index() {
        // z = 0 sorts before the good part at z = 3, so the bad frame is the cine source.
        let built = crate::bytes::synthetic_series_parts();
        assert!(built.is_ok(), "synthetic parts");
        let Ok(parts) = built else {
            unreachable!("asserted immediately above")
        };
        let Some(good) = parts.first() else {
            unreachable!("the synthetic body has a first object")
        };
        let bad = crate::bytes::frame_time_zero_part();
        assert!(bad.is_ok(), "frame time zero");
        let Ok(bad) = bad else {
            unreachable!("asserted immediately above")
        };
        let boundary = "ocelli";
        let mut body = Vec::new();
        for part in [good.as_slice(), bad.as_slice()] {
            body.extend_from_slice(
                format!("--{boundary}\r\nContent-Type: application/dicom\r\n\r\n").as_bytes(),
            );
            body.extend_from_slice(part);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        let (status, response) = series_reply(&body, Some("multipart/form-data; boundary=ocelli"));
        assert_eq!(status, 400);
        assert!(response.contains("\"error\":\"cine\""));
        assert!(response.contains("\"frame\":1"));
        assert!(!response.contains("\"part\""));
    }
}
