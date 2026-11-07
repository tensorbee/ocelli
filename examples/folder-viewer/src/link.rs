//! `POST /link`: where linked viewports move and where reference lines go.
//!
//! The geometry is `ocelli_viewport::link`: `same_frame_of_reference`,
//! `closest_coplanar` and `reference_segment`. This module parses the body,
//! rebuilds each plane through the `ocelli-pixel` constructors, calls the
//! library and writes the JSON. It computes no normal, distance or
//! intersection of its own. The page-to-index shift is
//! `ImagePoint::from_index`. The local rules are
//! `.claude/plans/F-209-design.md`.
//!
//! The body is ASCII lines joined by `\n`, with at most one trailing `\n`,
//! and fields separated by one space:
//!
//! ```text
//! source <uid> <plane>
//! target <cell> <shown> <scroll> <uid> <plane> <uid> <plane> ...
//! ```
//!
//! `<uid>` is a UID or `-`, `<plane>` a token from `plane_token` or `-`,
//! `<cell>` the zero-based viewport, each at most once, `<shown>` the index
//! the target shows into its own list, and `<scroll>` `1` when linked scroll
//! is on and `0` otherwise. The page sends only text this process wrote, so
//! anything else is refused whole.

use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, PixelSpacing,
};
use ocelli_viewport::link::{
    FrameOfReferenceUid, LinkError, closest_coplanar, reference_segment, same_frame_of_reference,
};

use crate::preview::{FRAME_LIST_CAP, UID_MAX_CHARS};
use crate::value::ImagePoint;

/// The longest `f64` field `plane_token` writes. `{:e}` writes at most 17
/// significant digits, so a sign, a digit, a point, 16 digits and `e-308`
/// make 24, which a test measures over the extreme values.
pub(crate) const FLOAT_FIELD_CHARS: usize = 24;
/// The longest `u32` field, `4294967295`.
pub(crate) const COUNT_FIELD_CHARS: usize = 10;
/// `f64` fields in a token: position, two cosines, two spacings.
pub(crate) const TOKEN_FLOATS: usize = 11;
/// `u32` fields in a token: rows and columns.
pub(crate) const TOKEN_COUNTS: usize = 2;
/// The longest token: its fields and the commas between them.
pub(crate) const TOKEN_MAX_CHARS: usize = TOKEN_FLOATS * FLOAT_FIELD_CHARS
    + TOKEN_COUNTS * COUNT_FIELD_CHARS
    + (TOKEN_FLOATS + TOKEN_COUNTS - 1);
/// The page's `MAX_SIDE`. Cells run 0 to `MAX_SIDE * MAX_SIDE - 1`.
pub(crate) const MAX_SIDE: usize = 3;
/// Viewports a body may name.
pub(crate) const CELLS: usize = MAX_SIDE * MAX_SIDE;
/// The fixed words. The page writes `source` once and `target` per line.
pub(crate) const SOURCE_WORD: &str = "source";
pub(crate) const TARGET_WORD: &str = "target";

/// The body is not one this process would accept. Nothing is linked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinkRefused;

/// A plane token read back, or `None` when it is not one.
///
/// The fields are the thirteen of `plane_token`, each within its length,
/// and the plane is rebuilt through `ImagePositionPatient::new`,
/// `ImageOrientationPatient::new`, `PixelSpacing::new` with the row spacing
/// first, PS3.3 10.7.1.3, `ImageDimensions::new` and `ImagePlane::new`, so a
/// token those constructors refuse is not a plane.
#[must_use]
pub fn plane_from_token(text: &str) -> Option<ImagePlane> {
    let mut fields = text.split(',');
    let mut floats = [0.0_f64; TOKEN_FLOATS];
    for slot in &mut floats {
        let field = fields.next()?;
        if field.len() > FLOAT_FIELD_CHARS {
            return None;
        }
        *slot = field.parse().ok()?;
    }
    let mut counts = [0_u32; TOKEN_COUNTS];
    for slot in &mut counts {
        let field = fields.next()?;
        if field.len() > COUNT_FIELD_CHARS {
            return None;
        }
        *slot = field.parse().ok()?;
    }
    if fields.next().is_some() {
        return None;
    }
    let [sx, sy, sz, xx, xy, xz, yx, yy, yz, row_mm, column_mm] = floats;
    let [rows, columns] = counts;
    ImagePlane::new(
        ImagePositionPatient::new([sx, sy, sz]).ok()?,
        ImageOrientationPatient::new([xx, xy, xz], [yx, yy, yz]).ok()?,
        PixelSpacing::new(row_mm, column_mm).ok()?,
        ImageDimensions::new(rows, columns).ok()?,
    )
    .ok()
}

/// One listed image: its frame of reference and its plane, either absent.
struct Listed {
    uid: Option<FrameOfReferenceUid>,
    plane: Option<ImagePlane>,
}

struct Target {
    cell: usize,
    shown: usize,
    scroll: bool,
    images: Vec<Listed>,
}

/// What one target is told.
struct Answer {
    cell: usize,
    index: Option<usize>,
    line: Option<[ImagePoint; 2]>,
    reason: &'static str,
}

/// The reply to one `/link` body, as JSON.
///
/// # Errors
///
/// [`LinkRefused`] for a body that does not follow the grammar above, a
/// UID that is not digits and `.` within 64 characters, a token the
/// constructors refuse, a `<cell>` past 8 or repeated, a target listing no
/// image or more than `FRAME_LIST_CAP`, or a `<shown>` outside its list.
pub(crate) fn link_json(body: &[u8]) -> Result<String, LinkRefused> {
    let (source, targets) = parse(body).ok_or(LinkRefused)?;
    let mut out = String::from("{\"targets\":[");
    for (at, target) in targets.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        push_answer(&mut out, &answer(&source, target));
    }
    out.push_str("]}");
    Ok(out)
}

/// The library's answer for one target.
///
/// The target links only when the source UID and every listed image's UID
/// are one frame. Every listed image and the source then need a plane.
/// With scroll on, the target moves to `closest_coplanar` when that differs
/// from what it shows. The line is `reference_segment` against the plane
/// the target will show. A scroll refusal names the reason before a line
/// refusal, and neither undoes the other half.
fn answer(source: &Listed, target: &Target) -> Answer {
    let refused = |reason| Answer {
        cell: target.cell,
        index: None,
        line: None,
        reason,
    };
    let one_frame = target
        .images
        .iter()
        .all(|image| same_frame_of_reference(source.uid.as_ref(), image.uid.as_ref()));
    if !one_frame {
        return refused("frame-of-reference");
    }
    let Some(source_plane) = source.plane else {
        return refused("no-plane");
    };
    let Some(stack) = target
        .images
        .iter()
        .map(|image| image.plane)
        .collect::<Option<Vec<ImagePlane>>>()
    else {
        return refused("no-plane");
    };
    let Some(shown) = stack.get(target.shown) else {
        return refused("degenerate");
    };
    let mut index = None;
    let mut reason = "";
    if target.scroll {
        match closest_coplanar(&source_plane, shown, &stack) {
            Ok(nearest) if nearest != target.shown => index = Some(nearest),
            Ok(_) => {}
            Err(error) => reason = reason_of(error),
        }
    }
    let drawn = index.and_then(|at| stack.get(at)).unwrap_or(shown);
    let line = match reference_segment(&source_plane, drawn) {
        Ok((start, end)) => Some([ImagePoint::from_index(start), ImagePoint::from_index(end)]),
        Err(error) => {
            if reason.is_empty() {
                reason = reason_of(error);
            }
            None
        }
    };
    Answer {
        cell: target.cell,
        index,
        line,
        reason,
    }
}

fn reason_of(error: LinkError) -> &'static str {
    match error {
        LinkError::NotCoplanar => "not-coplanar",
        LinkError::Parallel => "parallel",
        LinkError::Degenerate => "degenerate",
    }
}

fn push_answer(out: &mut String, answer: &Answer) {
    out.push_str("{\"cell\":");
    out.push_str(&answer.cell.to_string());
    out.push_str(",\"index\":");
    match answer.index {
        Some(index) => out.push_str(&index.to_string()),
        None => out.push_str("null"),
    }
    out.push_str(",\"line\":");
    match answer.line {
        Some([start, end]) => {
            out.push_str(&format!("[{},{},{},{}]", start.x, start.y, end.x, end.y));
        }
        None => out.push_str("null"),
    }
    out.push_str(",\"reason\":\"");
    out.push_str(answer.reason);
    out.push_str("\"}");
}

fn parse(body: &[u8]) -> Option<(Listed, Vec<Target>)> {
    let text = std::str::from_utf8(body)
        .ok()
        .filter(|text| text.is_ascii())?;
    let text = text.strip_suffix('\n').unwrap_or(text);
    let mut lines = text.split('\n');
    let mut fields = lines.next()?.split(' ');
    if fields.next()? != SOURCE_WORD {
        return None;
    }
    let source = listed(fields.next()?, fields.next()?)?;
    if fields.next().is_some() {
        return None;
    }
    let mut targets: Vec<Target> = Vec::new();
    for line in lines {
        let target = target_line(line)?;
        if targets.iter().any(|seen| seen.cell == target.cell) {
            return None;
        }
        targets.push(target);
    }
    Some((source, targets))
}

fn target_line(line: &str) -> Option<Target> {
    let mut fields = line.split(' ');
    if fields.next()? != TARGET_WORD {
        return None;
    }
    let cell = small_number(fields.next()?)?;
    let shown = small_number(fields.next()?)?;
    let scroll = match fields.next()? {
        "1" => true,
        "0" => false,
        _ => return None,
    };
    if cell >= CELLS {
        return None;
    }
    let mut images = Vec::new();
    while let Some(uid) = fields.next() {
        if images.len() >= FRAME_LIST_CAP {
            return None;
        }
        images.push(listed(uid, fields.next()?)?);
    }
    if shown >= images.len() {
        return None;
    }
    Some(Target {
        cell,
        shown,
        scroll,
        images,
    })
}

/// A decimal index of at most [`COUNT_FIELD_CHARS`] digits.
fn small_number(text: &str) -> Option<usize> {
    if text.is_empty()
        || text.len() > COUNT_FIELD_CHARS
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    text.parse().ok()
}

fn listed(uid: &str, plane: &str) -> Option<Listed> {
    let uid = match uid {
        "-" => None,
        text if !text.is_empty()
            && text.len() <= UID_MAX_CHARS
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.') =>
        {
            Some(FrameOfReferenceUid(text.to_owned()))
        }
        _ => return None,
    };
    let plane = match plane {
        "-" => None,
        token => Some(plane_from_token(token)?),
    };
    Some(Listed { uid, plane })
}
