//! The camera, the image rectangle, and HLD 25.1's geometry bound.
//!
//! **The canvas scale is read, never re-derived.** `canvasScale` in
//! `tools/oracle/src/params.mjs` computes canvas pixels per source pixel from
//! the camera's own `parallelScale` and the row and column spacing, and F-X007
//! publishes it on every stack sidecar as `canvasPixelsPerSourcePixel`. That
//! was decision 13 of F-011's design round, taken so this file would not hold
//! a second copy of that derivation. HLD section 18's "do not let a second
//! copy of this logic appear anywhere" is about the LUT chain and the
//! reasoning generalises.
//!
//! What this file derives is the RECTANGLE that scale implies, which is the
//! line between the picture and the letterbox and the denominator of
//! `informativeFraction`. It is not the region the bias bullet is averaged
//! over: 25.1 names the informative region, which is the subset of this
//! rectangle that is not clipped to the same display extreme on both sides.
//!
//! **There is no float to integer cast here.** `as` on a float truncates
//! toward zero and saturates silently out of range, and all three cast lints
//! are denied. The rectangle's edges are found by comparing pixel centres
//! against the extent, which is a scan over at most a canvas width of
//! candidates and is exact.

use crate::frame::{FrameError, Rect};
use crate::tolerance::{CANVAS_TOLERANCE_PIXELS, WORLD_TOLERANCE_MM};

/// One field that is outside a bound, with both readings and the bound it
/// broke, so the report can name it rather than saying "geometry differs".
#[derive(Clone, Debug)]
pub struct Divergence {
    pub field: String,
    pub reference: f64,
    pub candidate: f64,
    pub difference: f64,
    pub bound: f64,
}

/// The camera block of a sidecar. `view_plane_normal` is present on a volume
/// reformat and absent on a stack frame, and a reformat is meaningless without
/// it.
#[derive(Clone, Debug)]
pub struct Camera {
    pub position: [f64; 3],
    pub focal_point: [f64; 3],
    pub view_up: [f64; 3],
    pub parallel_scale: f64,
    pub view_plane_normal: Option<[f64; 3]>,
}

fn compare(field: &str, a: f64, b: f64, bound: f64, into: &mut Vec<Divergence>) {
    let difference = (b - a).abs();
    // A NaN comparison is false in both directions, so a NaN would slip past a
    // `>` test silently. It is reported as a divergence with an infinite
    // difference instead, because a camera that carries one is not a camera.
    let outside = difference.is_nan() || difference > bound;
    if outside {
        into.push(Divergence {
            field: field.to_owned(),
            reference: a,
            candidate: b,
            difference,
            bound,
        });
    }
}

fn compare_triple(field: &str, a: [f64; 3], b: [f64; 3], bound: f64, into: &mut Vec<Divergence>) {
    for (index, (left, right)) in a.iter().zip(b.iter()).enumerate() {
        compare(&format!("{field}[{index}]"), *left, *right, bound, into);
    }
}

/// 25.1's "world coordinates within 1e-6 mm", applied to the camera.
///
/// `view_up` and `view_plane_normal` are direction cosines rather than
/// millimetres, and they are held to the same number deliberately: a unit
/// vector wrong by more than a millionth is a different orientation, and 25.1
/// states no second bound for a direction.
#[must_use]
pub fn world_divergences(reference: &Camera, candidate: &Camera) -> Vec<Divergence> {
    let mut found = Vec::new();
    compare_triple(
        "camera.position",
        reference.position,
        candidate.position,
        WORLD_TOLERANCE_MM,
        &mut found,
    );
    compare_triple(
        "camera.focalPoint",
        reference.focal_point,
        candidate.focal_point,
        WORLD_TOLERANCE_MM,
        &mut found,
    );
    compare_triple(
        "camera.viewUp",
        reference.view_up,
        candidate.view_up,
        WORLD_TOLERANCE_MM,
        &mut found,
    );
    compare(
        "camera.parallelScale",
        reference.parallel_scale,
        candidate.parallel_scale,
        WORLD_TOLERANCE_MM,
        &mut found,
    );
    match (reference.view_plane_normal, candidate.view_plane_normal) {
        (Some(a), Some(b)) => compare_triple(
            "camera.viewPlaneNormal",
            a,
            b,
            WORLD_TOLERANCE_MM,
            &mut found,
        ),
        (None, None) => {}
        (a, b) => found.push(Divergence {
            field: "camera.viewPlaneNormal".to_owned(),
            reference: if a.is_some() { 1.0 } else { 0.0 },
            candidate: if b.is_some() { 1.0 } else { 0.0 },
            difference: f64::INFINITY,
            bound: WORLD_TOLERANCE_MM,
        }),
    }
    found
}

/// The image's extent on the canvas, in canvas pixels, before it is turned
/// into whole pixels.
#[derive(Clone, Debug)]
pub struct CanvasExtent {
    pub x0: f64,
    pub y0: f64,
    pub width: f64,
    pub height: f64,
}

impl CanvasExtent {
    /// The extent of an image of the given canvas size, centred in the canvas.
    ///
    /// `resetCamera` fits the image by its extent in millimetres and centres
    /// it, which is what makes the offset half the leftover in each direction.
    #[must_use]
    pub fn centred(width: f64, height: f64, canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            x0: (f64::from(canvas_width) - width) / 2.0,
            y0: (f64::from(canvas_height) - height) / 2.0,
            width,
            height,
        }
    }

    /// The whole canvas pixels this extent covers.
    ///
    /// A pixel belongs to the image when its CENTRE lies inside the extent, so
    /// pixel `i` is inside when `i + 0.5 >= x0` and `i + 0.5 < x0 + width`.
    /// The alternatives were tried and both are wrong in a way that is
    /// invisible: flooring the offset admits a letterbox column, and ceiling
    /// the far edge admits one on the other side. At 512 rows either is 512
    /// background pixels counted as picture.
    ///
    /// **The harm is not to the bias bullet.** Both sides paint the declared
    /// clear colour in the letterbox, so a wrongly admitted column is black on
    /// both sides, which is clipped to the same extreme and therefore
    /// uninformative, and it never reaches the bias denominator at all. The
    /// two things it does break:
    ///
    /// 1. `informativeFraction` is informative pixels over image-rectangle
    ///    pixels, so an admitted column inflates the denominator alone and
    ///    pushes the fraction down towards `INFORMATIVE_FRACTION_FLOOR`. The
    ///    lowest fraction among the views the identity run does not call weak
    ///    is 0.10074 against a floor of 0.10, so the margin is 0.0007 and 512
    ///    of 262144 pixels is 0.00195 of the rectangle. Reproduce the fraction
    ///    from `informativeFraction` in the `compare.json` that
    ///    `./target/release/ocelli-compare identity` writes.
    /// 2. The `letterbox-only` qualifier fires only when the image region
    ///    carries NO difference and the background carries one. A fit error in
    ///    a column that should have been letterbox then lands inside the image
    ///    region, the qualifier cannot fire, and a difference in the fit is
    ///    attributed to the picture and to us.
    ///
    /// The result is clamped to the canvas, because the published scale
    /// carries floating point noise and a frame fitted DOWN into the canvas
    /// can put its far edge a millionth of a pixel outside.
    #[must_use]
    pub fn rect(&self, canvas_width: u32, canvas_height: u32) -> Rect {
        let x_start = first_centre_at_or_after(self.x0, canvas_width);
        let x_end = first_centre_at_or_after(self.x0 + self.width, canvas_width);
        let y_start = first_centre_at_or_after(self.y0, canvas_height);
        let y_end = first_centre_at_or_after(self.y0 + self.height, canvas_height);
        Rect {
            x0: x_start,
            y0: y_start,
            width: x_end.saturating_sub(x_start),
            height: y_end.saturating_sub(y_start),
        }
    }
}

/// The first pixel index in `0..limit` whose centre is at or after `edge`, or
/// `limit` when there is none.
///
/// A scan rather than a cast. `limit` is a canvas dimension, 512 today, so the
/// cost is nothing and the arithmetic is exact for every input including a
/// negative or a non-finite edge.
fn first_centre_at_or_after(edge: f64, limit: u32) -> u32 {
    if edge.is_nan() {
        return limit;
    }
    (0..limit)
        .find(|index| f64::from(*index) + 0.5 >= edge)
        .unwrap_or(limit)
}

/// The stack case: the image's canvas extent from the published scale.
///
/// # Errors
/// Never today. The signature carries a `Result` because the caller's other
/// extent sources can fail and a uniform shape keeps the call sites readable.
pub fn stack_extent(
    columns: u32,
    rows: u32,
    horizontal: f64,
    vertical: f64,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<CanvasExtent, FrameError> {
    Ok(CanvasExtent::centred(
        f64::from(columns) * horizontal,
        f64::from(rows) * vertical,
        canvas_width,
        canvas_height,
    ))
}

/// 25.1's "canvas coordinates within a quarter pixel", applied to the two
/// sides' image extents.
///
/// The offsets are compared as well as the sizes, because two extents of the
/// same size in different places are two different pictures and a size-only
/// comparison would pass a translated frame.
#[must_use]
pub fn canvas_divergences(reference: &CanvasExtent, candidate: &CanvasExtent) -> Vec<Divergence> {
    let mut found = Vec::new();
    compare(
        "imageExtent.x0",
        reference.x0,
        candidate.x0,
        CANVAS_TOLERANCE_PIXELS,
        &mut found,
    );
    compare(
        "imageExtent.y0",
        reference.y0,
        candidate.y0,
        CANVAS_TOLERANCE_PIXELS,
        &mut found,
    );
    compare(
        "imageExtent.width",
        reference.width,
        candidate.width,
        CANVAS_TOLERANCE_PIXELS,
        &mut found,
    );
    compare(
        "imageExtent.height",
        reference.height,
        candidate.height,
        CANVAS_TOLERANCE_PIXELS,
        &mut found,
    );
    found
}

/// The reformat case: 25.1's canvas bullet applied to a frame with no source
/// pixel grid.
///
/// A reformat plane is a cut through a volume, so there is no magnification
/// factor and no image rectangle inside the canvas. What the two sides must
/// agree on is the SCALE, `reformat.millimetresPerCanvasPixel`, and the
/// quarter-pixel bound is applied where it bites hardest: at the canvas edge,
/// half a canvas away from the focal point, where a scale disagreement has
/// accumulated the most.
#[must_use]
pub fn reformat_scale_divergences(
    reference_mm_per_pixel: f64,
    candidate_mm_per_pixel: f64,
    canvas_height: u32,
) -> Vec<Divergence> {
    let half = f64::from(canvas_height) / 2.0;
    let drift_mm = (candidate_mm_per_pixel - reference_mm_per_pixel).abs() * half;
    let drift_pixels = if reference_mm_per_pixel > 0.0 {
        drift_mm / reference_mm_per_pixel
    } else {
        f64::INFINITY
    };
    let mut found = Vec::new();
    if drift_pixels.is_nan() || drift_pixels > CANVAS_TOLERANCE_PIXELS {
        found.push(Divergence {
            field: "reformat.millimetresPerCanvasPixel".to_owned(),
            reference: reference_mm_per_pixel,
            candidate: candidate_mm_per_pixel,
            difference: drift_pixels,
            bound: CANVAS_TOLERANCE_PIXELS,
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{Camera, CanvasExtent, reformat_scale_divergences, world_divergences};

    /// A camera carrying a `NaN` is not a camera, and a `>` test against a
    /// bound is false in both directions for one. It is reported rather than
    /// passed.
    #[test]
    fn a_nan_in_the_camera_is_a_divergence_and_not_a_pass() {
        let base = Camera {
            position: [0.0, 0.0, 0.0],
            focal_point: [0.0, 0.0, 0.0],
            view_up: [0.0, -1.0, 0.0],
            parallel_scale: 16.0,
            view_plane_normal: None,
        };
        let mut broken = base.clone();
        broken.parallel_scale = f64::NAN;
        assert_eq!(world_divergences(&base, &broken).len(), 1);
    }

    /// A stack sidecar carries no `viewPlaneNormal` and a reformat one does.
    /// Comparing a frame that has one against a frame that does not is a
    /// divergence rather than a field quietly skipped.
    #[test]
    fn a_missing_view_plane_normal_on_one_side_only_is_a_divergence() {
        let base = Camera {
            position: [0.0, 0.0, 0.0],
            focal_point: [0.0, 0.0, 0.0],
            view_up: [0.0, -1.0, 0.0],
            parallel_scale: 16.0,
            view_plane_normal: None,
        };
        let mut reformat = base.clone();
        reformat.view_plane_normal = Some([0.0, 0.0, -1.0]);
        assert_eq!(world_divergences(&base, &reformat).len(), 1);
        assert!(world_divergences(&reformat, &reformat).is_empty());
    }

    /// The reformat scale bound, at the AXIAL MR frame's own numbers.
    /// 0.703125 mm per canvas pixel over a 512 pixel canvas: a quarter pixel
    /// at the canvas edge is a scale change of
    /// 0.25 * 0.703125 / 256 = 0.000687 mm per pixel.
    #[test]
    fn the_reformat_scale_bound_is_a_quarter_pixel_at_the_canvas_edge() {
        let base = 0.703_125_f64;
        assert!(reformat_scale_divergences(base, base + 0.000_6, 512).is_empty());
        assert_eq!(
            reformat_scale_divergences(base, base + 0.000_8, 512).len(),
            1
        );
    }

    /// An extent that lies entirely outside the canvas produces an empty
    /// rectangle rather than a rectangle that wraps.
    #[test]
    fn an_extent_off_the_canvas_produces_an_empty_rectangle() {
        let extent = CanvasExtent {
            x0: 600.0,
            y0: 0.0,
            width: 10.0,
            height: 512.0,
        };
        let rect = extent.rect(512, 512);
        assert!(rect.is_empty());
    }
}
