//! JSON for `POST /frame-open`.
//!
//! The synthetic body is the only response that carries a display number.
//! A posted object returns flags, colour, creator slots and a padding count.

use ocelli_core::{Display, Stored};
use ocelli_dicom::{
    BurnedInFlag, ColourSelection, FrameOpenError, LossyFlag, OpenedFrame, PrivateCreator,
    WindowInput, fill_padding, open_frame, parse_part10,
};
use ocelli_pixel::{LutChain, VoiFunction};

use crate::bytes::synthetic_part10;

/// Answer one frame-open body.
///
/// `synthetic` runs the built-in fixture. Every other body is Part 10 bytes.
#[must_use]
pub fn reply_body(body: &[u8], frame: u32) -> (u16, String) {
    if body == b"synthetic" {
        synthetic_reply(frame)
    } else {
        posted_reply(body, frame)
    }
}

fn synthetic_reply(frame: u32) -> (u16, String) {
    let bytes = match synthetic_part10() {
        Ok(bytes) => bytes,
        Err(_) => return error_reply(500, "fixture", None),
    };
    let parsed = match parse_part10(&bytes) {
        Ok(parsed) => parsed,
        Err(error) => return error_reply(400, parse_code(error), None),
    };
    let opened = match open_frame(&parsed, frame) {
        Ok(opened) => opened,
        Err(error) => return frame_error(error),
    };
    let WindowInput::Transform(voi) = opened.window().clone() else {
        return error_reply(500, "fixture", None);
    };
    let chain = match LutChain::new(
        opened.modality().clone(),
        voi,
        opened.photometric(),
        opened.presentation(),
    ) {
        Ok(chain) => chain,
        Err(_) => return error_reply(500, "chain", None),
    };
    let source = [Stored(0.0), Stored(50.0)];
    let mut destination = [Display(0.0), Display(0.0)];
    if chain.map_into(&source, &mut destination).is_err() {
        return error_reply(500, "chain", None);
    }
    let mask = opened.padding_mask(&[0, 50]);
    if fill_padding(&mut destination, &mask, opened.photometric(), &chain).is_err() {
        return error_reply(500, "mask_length", None);
    }
    let mut shown = None;
    let mut padding_count = 0_usize;
    for (sample, padded) in destination.iter().zip(mask.iter().copied()) {
        if padded {
            padding_count += 1;
        } else {
            shown = Some(sample.0);
        }
    }
    let Some(display) = shown else {
        return error_reply(500, "fixture", None);
    };
    if !display.is_finite() {
        return error_reply(500, "chain", None);
    }
    (200, success_json(&opened, Some(display), padding_count))
}

fn posted_reply(body: &[u8], frame: u32) -> (u16, String) {
    let parsed = match parse_part10(body) {
        Ok(parsed) => parsed,
        Err(error) => return error_reply(400, parse_code(error), None),
    };
    match open_frame(&parsed, frame) {
        Ok(opened) => {
            let padding_count = if opened.padding().is_some() { 1 } else { 0 };
            (200, success_json(&opened, None, padding_count))
        }
        Err(error) => frame_error(error),
    }
}

fn success_json(opened: &OpenedFrame, display: Option<f32>, padding_count: usize) -> String {
    let mut out = String::from("{");
    if let Some(display) = display {
        out.push_str("\"display\":");
        out.push_str(&display.to_string());
        out.push(',');
    }
    let (colour, codec_already_rgb) = colour_of(opened.colour());
    out.push_str("\"padding_count\":");
    out.push_str(&padding_count.to_string());
    out.push_str(",\"colour\":");
    push_json_string(&mut out, colour);
    out.push_str(",\"lossy\":");
    push_json_string(&mut out, lossy_of(opened.lossy()));
    out.push_str(",\"burned_in\":");
    push_json_string(&mut out, burned_in_of(opened.burned_in()));
    out.push_str(",\"window\":");
    push_json_string(&mut out, window_of(opened.window()));
    out.push_str(",\"codec_already_rgb\":");
    out.push_str(if codec_already_rgb { "true" } else { "false" });
    out.push_str(",\"creators\":");
    out.push_str(&creators_json(opened.private_creators()));
    out.push('}');
    out
}

fn creators_json(creators: &[PrivateCreator]) -> String {
    let mut out = String::from("[");
    for (index, creator) in creators.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"group\":");
        out.push_str(&creator.group().to_string());
        out.push_str(",\"slot\":");
        out.push_str(&creator.slot().to_string());
        out.push_str(",\"creator\":");
        push_json_string(&mut out, creator.creator());
        out.push('}');
    }
    out.push(']');
    out
}

fn push_json_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn colour_of(colour: ColourSelection) -> (&'static str, bool) {
    match colour {
        ColourSelection::Monochrome => ("monochrome", false),
        ColourSelection::Palette => ("palette", false),
        ColourSelection::Colour { codec_already_rgb } => ("colour", codec_already_rgb),
    }
}

fn lossy_of(flag: LossyFlag) -> &'static str {
    match flag {
        LossyFlag::Absent => "absent",
        LossyFlag::Lossless => "lossless",
        LossyFlag::Lossy => "lossy",
    }
}

fn burned_in_of(flag: BurnedInFlag) -> &'static str {
    match flag {
        BurnedInFlag::Absent => "absent",
        BurnedInFlag::No => "no",
        BurnedInFlag::Yes => "yes",
    }
}

fn window_of(window: &WindowInput) -> &'static str {
    match window {
        WindowInput::Absent => "absent",
        WindowInput::Transform(voi) => match voi.window() {
            None => "lut",
            Some((_, _, VoiFunction::Linear)) => "linear",
            Some((_, _, VoiFunction::LinearExact)) => "linear_exact",
            Some((_, _, VoiFunction::Sigmoid)) => "sigmoid",
        },
    }
}

fn frame_error(error: FrameOpenError) -> (u16, String) {
    match error {
        FrameOpenError::FrameOutOfRange => error_reply(400, "frame_out_of_range", None),
        FrameOpenError::Attribute { group, element } => {
            error_reply(400, "attribute", Some((group, element)))
        }
        FrameOpenError::LossyCode => error_reply(400, "lossy_code", None),
        FrameOpenError::BurnedInCode => error_reply(400, "burned_in_code", None),
        FrameOpenError::PaddingOrder => error_reply(400, "padding_order", None),
        FrameOpenError::PaddingLimitWithoutValue => {
            error_reply(400, "padding_limit_without_value", None)
        }
        FrameOpenError::SecondColourConversion => {
            error_reply(400, "second_colour_conversion", None)
        }
        FrameOpenError::Chain => error_reply(400, "chain", None),
        FrameOpenError::Photometric => error_reply(400, "photometric", None),
        FrameOpenError::Metadata => error_reply(400, "metadata", None),
        FrameOpenError::MaskLength => error_reply(400, "mask_length", None),
        _ => error_reply(400, "open", None),
    }
}

fn parse_code(error: ocelli_dicom::ParseError) -> &'static str {
    match error {
        ocelli_dicom::ParseError::TruncatedPart10 => "truncated_part10",
        ocelli_dicom::ParseError::MissingDicmPrefix => "missing_dicm",
        ocelli_dicom::ParseError::InvalidFileMeta => "invalid_file_meta",
        ocelli_dicom::ParseError::MissingTransferSyntax => "missing_transfer_syntax",
        ocelli_dicom::ParseError::UnknownTransferSyntax => "unknown_transfer_syntax",
        ocelli_dicom::ParseError::UnsupportedDataSetTransferSyntax => "unsupported_transfer_syntax",
        ocelli_dicom::ParseError::TruncatedDataSet => "truncated_data_set",
        ocelli_dicom::ParseError::InvalidDataSet => "invalid_data_set",
        _ => "parse",
    }
}

fn error_reply(status: u16, code: &str, tag: Option<(u16, u16)>) -> (u16, String) {
    let mut out = String::from("{\"error\":");
    push_json_string(&mut out, code);
    if let Some((group, element)) = tag {
        out.push_str(",\"group\":");
        out.push_str(&group.to_string());
        out.push_str(",\"element\":");
        out.push_str(&element.to_string());
    }
    out.push('}');
    (status, out)
}

#[cfg(test)]
mod tests {
    use crate::bytes::plain_part10;

    use super::{push_json_string, reply_body};

    #[test]
    fn synthetic_reports_the_hand_value_and_one_padding_sample() {
        let (status, body) = reply_body(b"synthetic", 0);
        assert_eq!(status, 200);
        assert!(body.contains("\"display\":63.75"));
        assert!(body.contains("\"padding_count\":1"));
        assert!(body.contains("\"window\":\"linear_exact\""));
    }

    #[test]
    fn a_posted_object_has_no_display_sample() {
        let bytes = match plain_part10() {
            Ok(bytes) => bytes,
            Err(_) => unreachable!("the plain fixture is static"),
        };
        let (status, body) = reply_body(&bytes, 0);
        assert_eq!(status, 200);
        assert!(!body.contains("\"display\""));
        assert!(body.contains("\"padding_count\":0"));
        assert!(body.contains("\"colour\":\"monochrome\""));
    }

    #[test]
    fn json_string_escapes_a_quote() {
        let mut out = String::new();
        push_json_string(&mut out, "A\"B");
        assert_eq!(out, "\"A\\\"B\"");
    }
}
