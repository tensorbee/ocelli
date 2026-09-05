//! The image rectangle, and HLD 25.1's geometry bound at each boundary.
//!
//! The rectangle matters twice. It is the region the bias bullet is evaluated
//! over, "signed mean difference over the image rectangle", and it is what
//! separates a difference in the picture from a difference in the letterbox.
//! Getting it wrong by a column would move the bias by a fraction of a code
//! and nothing would say so.
//!
//! **The canvas scale is READ, not re-derived.** `canvasScale` in
//! `tools/oracle/src/params.mjs` computes canvas pixels per source pixel from
//! the camera's own `parallelScale` and the row and column spacing, and F-X007
//! publishes the result on every stack sidecar as
//! `canvasPixelsPerSourcePixel`. HLD section 18's rule that a piece of
//! arithmetic exists exactly once is about the LUT chain and its reasoning
//! generalises, so this crate reads that number and derives only the rectangle
//! from it. What is asserted below is therefore the rectangle, against a
//! number the reference itself measured.
//!
//! PS3.3 C.7.6.2.1.1 gives Pixel Spacing as `[between rows, between columns]`,
//! so advancing one row moves along the column direction cosine. That is why
//! the published pair is named `vertical` and `horizontal` rather than being
//! indexed, and why a fixture that indexed it could transpose silently.

use std::error::Error;

use ocelli_oracle::frame::Rect;
use ocelli_oracle::geometry::{Camera, CanvasExtent, canvas_divergences, world_divergences};
use ocelli_oracle::tolerance;

type Outcome = Result<(), Box<dyn Error>>;

/// The declared canvas, from `tools/oracle/render-params.json`.
const CANVAS: u32 = 512;

/// `docs/lld/oracle.md`'s worked case, verbatim:
///
/// > `syntax/reference_mono12.dcm` is the worked case: 64 by 96 at spacing
/// > [0.5, 0.25] with `parallelScale` 16 gives 8 canvas pixels per source
/// > pixel vertically and 4 horizontally, so the image fills the height and
/// > 384 of the 512 columns, and the frame's recorded `blackFraction` is 0.25
/// > exactly.
///
/// 64 rows at 8 canvas pixels each is 512, so the image fills the height. 96
/// columns at 4 each is 384, so 128 columns of the 512 are letterbox, 64 on
/// each side. 512 * 128 = 65536 background pixels out of 262144, which is
/// 0.25 exactly. The reference measured 65536 black pixels on that frame and
/// recorded `blackFraction: 0.25`, so this fixture is checked against a number
/// the instrument produced rather than against itself.
#[test]
fn the_worked_case_rectangle_reproduces_the_reference_black_fraction() -> Outcome {
    let extent = CanvasExtent::centred(
        f64::from(96_u32) * 4.0,
        f64::from(64_u32) * 8.0,
        CANVAS,
        CANVAS,
    );
    assert_eq!(extent.width.to_bits(), 384.0_f64.to_bits());
    assert_eq!(extent.height.to_bits(), 512.0_f64.to_bits());
    assert_eq!(extent.x0.to_bits(), 64.0_f64.to_bits());
    assert_eq!(extent.y0.to_bits(), 0.0_f64.to_bits());

    let rect = extent.rect(CANVAS, CANVAS);
    assert_eq!(rect, Rect::new(64, 0, 384, 512)?);

    let frame_pixels = u64::from(CANVAS) * u64::from(CANVAS);
    let background = frame_pixels - rect.pixels();
    assert_eq!(background, 65_536, "512 rows by 128 letterbox columns");
    assert_eq!(frame_pixels, 262_144);
    assert_eq!(
        background * 4,
        frame_pixels,
        "the letterbox is a quarter of the frame, which is the reference's own \
         recorded blackFraction of 0.25"
    );
    Ok(())
}

/// A pixel belongs to the image rectangle when its CENTRE lies inside the
/// extent, so pixel `i` is inside when `i + 0.5 >= x0` and `i + 0.5 < x0 + w`.
/// The corpus produces non-integer extents: `synthetic/ct_unsigned_16.dcm`
/// publishes 426.66666 canvas pixels across a 512 pixel canvas, at an offset
/// of 42.66667. The centre rule gives columns 43 through 468 inclusive, which
/// is 426 columns.
///
/// A rule that floored the offset would have started at column 42 and a rule
/// that ceiled the far edge would have ended at 469, and both would have
/// counted a letterbox column as image. At 512 rows that is 512 pixels of
/// background inside the region the bias bullet averages over.
#[test]
fn a_fractional_extent_takes_the_pixels_whose_centres_are_inside() -> Outcome {
    let extent = CanvasExtent {
        x0: 42.666_67,
        y0: -0.000_002,
        width: 426.666_66,
        height: 512.000_004,
    };
    let rect = extent.rect(CANVAS, CANVAS);
    assert_eq!(rect, Rect::new(43, 0, 426, 512)?);
    Ok(())
}

/// The rectangle is clamped to the canvas. A frame fitted DOWN into the canvas
/// covers all of it, and floating point noise in the published scale can put
/// the far edge a millionth of a pixel outside. A rectangle that ran past the
/// frame would index outside the buffer, and the clamp is what makes that
/// impossible rather than merely unlikely.
#[test]
fn the_rectangle_is_clamped_to_the_canvas() -> Outcome {
    let extent = CanvasExtent {
        x0: -3.5,
        y0: -0.000_001,
        width: 519.0,
        height: 512.000_002,
    };
    let rect = extent.rect(CANVAS, CANVAS);
    assert_eq!(rect, Rect::new(0, 0, 512, 512)?);
    Ok(())
}

/// HLD 25.1's geometry bullet, verbatim:
///
/// > **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
/// > a quarter pixel.
///
/// The world half, at its boundary. 5e-7 mm is inside, 1e-6 mm is AT the bound
/// and "within" includes it, 2e-6 mm is outside. 2e-6 is the same perturbation
/// `volume-geometry-drift` in `tools/oracle/src/faults.mjs` uses against the
/// reference half's own reading of the same bullet, so the two halves of the
/// harness are held to one number.
#[test]
fn the_world_bound_is_1e_6_mm_and_it_includes_the_boundary() {
    let base = Camera {
        position: [10.0, 20.0, 30.0],
        focal_point: [10.0, 20.0, 0.0],
        view_up: [0.0, -1.0, 0.0],
        parallel_scale: 16.0,
        view_plane_normal: None,
    };

    let mut inside = base.clone();
    inside.position = [10.000_000_5, 20.0, 30.0];
    assert!(
        world_divergences(&base, &inside).is_empty(),
        "5e-7 mm is inside 1e-6 mm"
    );

    let mut at_bound = base.clone();
    at_bound.position = [10.000_001, 20.0, 30.0];
    assert!(
        world_divergences(&base, &at_bound).is_empty(),
        "\"within 1e-6 mm\" includes 1e-6 mm"
    );

    let mut outside = base.clone();
    outside.position = [10.000_002, 20.0, 30.0];
    let found = world_divergences(&base, &outside);
    assert_eq!(found.len(), 1, "one field is outside, so one divergence");
    let Some(first) = found.first() else {
        return;
    };
    assert_eq!(first.field, "camera.position[0]");

    let mut scaled = base.clone();
    scaled.parallel_scale = 16.000_002;
    assert_eq!(world_divergences(&base, &scaled).len(), 1);

    let mut focal = base.clone();
    focal.focal_point = [10.0, 20.000_002, 0.0];
    assert_eq!(world_divergences(&base, &focal).len(), 1);

    let mut up = base.clone();
    up.view_up = [0.0, -1.000_002, 0.0];
    assert_eq!(world_divergences(&base, &up).len(), 1);
}

/// The canvas half of the same bullet, at its boundary. The two sides' image
/// extents are compared in canvas pixels, and a quarter of a pixel is the
/// bound. 0.2 is inside, 0.25 is AT it, 0.3 is outside.
#[test]
fn the_canvas_bound_is_a_quarter_pixel_and_it_includes_the_boundary() {
    let base = CanvasExtent {
        x0: 64.0,
        y0: 0.0,
        width: 384.0,
        height: 512.0,
    };

    let mut inside = base.clone();
    inside.width = 384.2;
    assert!(canvas_divergences(&base, &inside).is_empty(), "0.2 < 0.25");

    let mut at_bound = base.clone();
    at_bound.width = 384.25;
    assert!(
        canvas_divergences(&base, &at_bound).is_empty(),
        "\"within a quarter pixel\" includes a quarter pixel"
    );

    let mut outside = base.clone();
    outside.width = 384.3;
    let found = canvas_divergences(&base, &outside);
    assert_eq!(found.len(), 1);
    let Some(first) = found.first() else {
        return;
    };
    assert_eq!(first.field, "imageExtent.width");

    let mut shifted = base.clone();
    shifted.x0 = 64.3;
    assert_eq!(
        canvas_divergences(&base, &shifted).len(),
        1,
        "an offset outside the bound is a divergence even when the size agrees"
    );
}

/// The two decimated rows are the reason 25.1's canvas bullet is not enough on
/// its own, and the arithmetic is recorded here rather than only in prose.
///
/// `real/dx_varepop/00000001.dcm` renders at 0.438356 canvas pixels per source
/// pixel. A quarter of a canvas pixel is therefore 0.25 / 0.438356 = 0.5703
/// SOURCE pixels, which is more than half of one. Under `NEAREST` a
/// sub-source-pixel difference in the fit selects a different source pixel, so
/// two cameras agreeing inside 25.1's written canvas tolerance can still
/// sample different stored values, and the resulting difference at a canvas
/// pixel is unbounded. That is why those two rows are `unmeasured` with the
/// qualifier `decimated` and why their pixel statistics do not gate.
#[test]
fn a_quarter_canvas_pixel_is_more_than_half_a_source_pixel_when_decimated() {
    let scale = 0.438_356_f64;
    let source_pixels = tolerance::CANVAS_TOLERANCE_PIXELS / scale;
    assert!(
        source_pixels > 0.5,
        "0.25 / 0.438356 = {source_pixels}, which is over half a source pixel"
    );
    assert!(source_pixels < 0.6);

    let ultrasound = tolerance::CANVAS_TOLERANCE_PIXELS / 0.625_153_f64;
    assert!(
        ultrasound < 0.5,
        "the ultrasound row at 0.625153 is under half a source pixel, so the \
         two decimated rows are not decimated to the same degree and only one \
         of them defeats the written bound arithmetically"
    );
}
