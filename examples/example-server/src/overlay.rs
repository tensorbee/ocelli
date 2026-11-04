//! JSON for `POST /overlay`.
//!
//! The synthetic body is one 4 by 4 object. Any other body is one Part 10
//! object. The mask lists graphic overlay bits. It is not a pixel buffer.

use ocelli_dicom::{OverlayReason, OverlaySet, parse_part10, read_overlays};

use crate::bytes::synthetic_overlay_object;

/// Answer one overlay body.
///
/// `synthetic` runs the built-in 4 by 4 fixture. Every other body is Part 10.
#[must_use]
pub fn overlay_reply(body: &[u8], frame: u32) -> (u16, String) {
    if body == b"synthetic" {
        let bytes = match synthetic_overlay_object() {
            Ok(bytes) => bytes,
            Err(_) => return error_json(500, "fixture", frame),
        };
        answer(&bytes, frame)
    } else {
        answer(body, frame)
    }
}

fn answer(bytes: &[u8], frame: u32) -> (u16, String) {
    let parsed = match parse_part10(bytes) {
        Ok(parsed) => parsed,
        Err(_) => return error_json(400, "parse", frame),
    };
    match read_overlays(&parsed, frame) {
        Ok(set) => (200, success_json(&set)),
        Err(error) => error_json(400, reason_code(error.reason()), error.index()),
    }
}

fn reason_code(reason: OverlayReason) -> &'static str {
    match reason {
        OverlayReason::Metadata => "metadata",
        OverlayReason::FrameOutOfRange => "frame_out_of_range",
        OverlayReason::FrameOverlayUnsupported => "frame_overlay_unsupported",
        OverlayReason::BitsAllocated => "bits_allocated",
        OverlayReason::Polygon => "polygon",
        OverlayReason::Shutter => "shutter",
        _ => "overlay",
    }
}

fn success_json(set: &OverlaySet) -> String {
    let hit = set.planes().iter().any(|plane| plane.bit(1, 1));
    let hidden = match set.shutter() {
        Some(shutter) => shutter.hides(1, 1),
        None => false,
    };
    let mut outside = 0_u32;
    for plane in set.planes() {
        outside = outside.saturating_add(plane.outside_count());
    }
    let presentation = match set.presentation_value() {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };
    let mut out = String::from("{\"rows\":");
    out.push_str(&set.image_rows().to_string());
    out.push_str(",\"columns\":");
    out.push_str(&set.image_columns().to_string());
    out.push_str(",\"hit\":");
    out.push_str(if hit { "true" } else { "false" });
    out.push_str(",\"hidden\":");
    out.push_str(if hidden { "true" } else { "false" });
    out.push_str(",\"outside\":");
    out.push_str(&outside.to_string());
    out.push_str(",\"presentation\":");
    out.push_str(&presentation);
    out.push_str(",\"mask\":");
    out.push_str(&mask_list(set));
    out.push('}');
    out
}

fn mask_list(set: &OverlaySet) -> String {
    let rows = u32::from(set.image_rows());
    let columns = u32::from(set.image_columns());
    let Some(count) = rows.checked_mul(columns) else {
        return "null".to_string();
    };
    if count > 256 {
        return "null".to_string();
    }
    let mut out = String::from("[");
    let mut first = true;
    let mut row = 1_i32;
    let row_end = i32::from(set.image_rows());
    let column_end = i32::from(set.image_columns());
    while row <= row_end {
        let mut column = 1_i32;
        while column <= column_end {
            if !first {
                out.push(',');
            }
            first = false;
            let set_bit = set.planes().iter().any(|plane| plane.bit(row, column));
            out.push(if set_bit { '1' } else { '0' });
            column = column.saturating_add(1);
        }
        row = row.saturating_add(1);
    }
    out.push(']');
    out
}

fn error_json(status: u16, code: &str, frame: u32) -> (u16, String) {
    let mut out = String::from("{\"error\":\"");
    out.push_str(code);
    out.push_str("\",\"frame\":");
    out.push_str(&frame.to_string());
    out.push('}');
    (status, out)
}
