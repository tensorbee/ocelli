//! The image rectangle, and HLD 25.1's geometry bound at each boundary.
//!
//! The rectangle matters twice. It is what separates a difference in the
//! picture from a difference in the letterbox, and it is the denominator of
//! `informativeFraction`. It BOUNDS the region the bias bullet is evaluated
//! over and it is not that region: the bullet says "signed mean difference
//! over the informative region", which is the subset of this rectangle that is
//! not clipped to the same extreme on both sides.
//!
//! **A wrong column is caught here and not elsewhere.** It does NOT move the
//! bias: a letterbox column is the same clear colour on both sides, and that
//! colour is an 8-bit extreme, so it is clipped to the same extreme, is not
//! informative, and never enters the bias denominator. **Both halves of that
//! are needed**, because `clipped_to_the_same_extreme` requires
//! `reference == candidate && (reference == 0 || reference == u8::MAX)`. The
//! colour is `base.background` in `tools/oracle/render-params.json`, today
//! `[0, 0, 0]`, and `the_declared_background_is_an_eight_bit_extreme` below is
//! what makes that dependency a checked one. What a wrong column moves is
//! `informativeFraction`, whose denominator is this rectangle, and the
//! `letterbox-only` qualifier, which can only fire when the image region
//! carries no difference at all. Neither shows up as a corpus failure, so
//! `an_edge_exactly_on_a_pixel_centre_belongs_to_the_image` below is the only
//! thing standing between an off-by-one edge rule and silence. Today's corpus
//! carries no extent landing on a pixel centre, which is exactly why that case
//! is constructed rather than waited for.
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
use std::path::Path;

use ocelli_oracle::frame::Rect;
use ocelli_oracle::geometry::{Camera, CanvasExtent, canvas_divergences, world_divergences};
use ocelli_oracle::tolerance;
use serde_json::Value;

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
/// 0.25 exactly.
///
/// **The first half of this test is arithmetic and asserts nothing about the
/// instrument.** It says the rectangle this crate derives from the published
/// scale has a letterbox of a quarter of the frame, and it computes both sides
/// of that itself. Until the eighth review pass this comment claimed the
/// fixture was "checked against a number the instrument produced", and it read
/// no instrument output at all.
///
/// **The second half reads the number, where there is one to read.**
/// `tools/oracle/out/` is gitignored, so it exists on a machine that has run
/// the reference half and not in CI. `run.mjs` writes `run.json` after every
/// row sidecar and calls `discardOutput` on any later problem, so the
/// directory holds the output of one complete run that passed every boundary
/// or it holds nothing. **That guarantee is the discard and not the write
/// order**, which the S03 review's ninth pass had to say because a `console.log`
/// follows the write and an earlier version of this comment leaned on "last".
/// So `run.json` present is the condition, and
/// under it the worked row's sidecar must exist and must carry
/// `black: 65536` and `blackFraction: 0.25`. A missing sidecar under a
/// complete run means the worked case named in `docs/lld/oracle.md` no longer
/// renders, which is a finding rather than a reason to skip.
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
        "the letterbox is a quarter of the frame, and the two sides of this \
         are both computed here"
    );

    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("out");
    if !out.join("run.json").exists() {
        // No complete reference render on this machine, so there is no
        // instrument number to read. The arithmetic above ran regardless.
        return Ok(());
    }
    let sidecar = out.join("syntax__reference_mono12.json");
    let text = std::fs::read_to_string(&sidecar).map_err(|error| {
        format!(
            "{} holds a complete run and not the worked case docs/lld/oracle.md \
             names: {error}",
            out.display()
        )
    })?;
    let measured: Value = serde_json::from_str(&text)?;
    let black = measured
        .pointer("/frame/statistics/black")
        .and_then(Value::as_u64)
        .ok_or("the worked case's sidecar records no /frame/statistics/black")?;
    assert_eq!(
        black, background,
        "the reference counted {black} black pixels on that frame and this \
         rectangle leaves {background} outside the image"
    );
    let fraction = measured
        .pointer("/frame/statistics/blackFraction")
        .and_then(Value::as_f64)
        .ok_or("the worked case's sidecar records no blackFraction")?;
    assert_eq!(
        fraction.to_bits(),
        0.25_f64.to_bits(),
        "the reference recorded a blackFraction of {fraction}"
    );
    let pixels = measured
        .pointer("/frame/statistics/pixels")
        .and_then(Value::as_u64)
        .ok_or("the worked case's sidecar records no pixel count")?;
    assert_eq!(
        pixels, frame_pixels,
        "and it is the declared 512 by 512 canvas"
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
/// background counted as picture, added to the denominator of
/// `informativeFraction` alone, and a column in which a fit error can no
/// longer be reported as `letterbox-only`.
///
/// **What that costs the fraction is `f * n / (P + n)`, not `n / P`**, and
/// this comment compared the second against the margin until the sprint
/// review's sixth pass. The informative COUNT does not change, so the fraction
/// goes from `I / P` to `I / (P + n)`. On the worst view the identity run does
/// not call weak, `synthetic/ct_series_nonuniform`, `I` is 21973 over
/// `P = 218112`, which is 0.10074 against a floor of 0.10, so the margin is
/// 0.00074. One column of `n = 512` moves it to 21973 / 218624 = 0.100506, a
/// move of 0.000236, which does not cross. Four do, at
/// 21973 / 220160 = 0.099805. Reproduce from `informativeFraction`,
/// `imagePixels` and `informativePixels` in the `compare.json` that
/// `./target/release/ocelli-compare identity` writes to
/// `tools/oracle/compare-out/`.
///
/// The mechanism is unchanged and only the supporting arithmetic was wrong:
/// one admitted column is still a guard nothing else watches.
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

/// **The case where the boundary convention is decidable, constructed because
/// the corpus does not contain one.**
///
/// Every extent in `tools/oracle/out/` today lands strictly between two pixel
/// centres, so `>=` and `>` in `first_centre_at_or_after` agree on all of them
/// and the whole suite stayed green when the comparison was changed. That is a
/// guard nobody has watched fail, and it is the shape this repository refuses.
///
/// The rule, from the extent's own definition: the extent is the half-open
/// interval `[x0, x0 + width)`, and a pixel belongs to the image when its
/// centre lies in it, so pixel `i` is inside exactly when
/// `x0 <= i + 0.5 < x0 + width`. **The two ends need opposite conventions and
/// one comparison serves both**, because the search returns the first index at
/// or after the edge and that index is the first pixel INCLUDED at the near
/// edge and the first pixel EXCLUDED at the far one. So `>=` is right twice
/// and `>` is wrong twice.
///
/// Three cases, each with at least one edge exactly on a centre:
///
/// - `x0 = 0.5`, width 42, far edge 42.5. Column 0's centre is 0.5, which is
///   in `[0.5, 42.5)`, so it is the first column. Column 42's centre is 42.5,
///   which is not, so 41 is the last. 42 columns from 0.
/// - `x0 = 42.5`, width 384, far edge 426.5. Columns 42 through 425, so 384
///   columns from 42.
/// - `x0 = 42.5`, width 384.3, far edge 426.8. The near edge is on a centre
///   and the far edge is not, so column 426 joins: 385 columns from 42. This
///   is the case where the wrong comparison changes the pixel COUNT and not
///   only the offset, which is what moves the denominator of every bias number
///   the comparator reports.
#[test]
fn an_edge_exactly_on_a_pixel_centre_belongs_to_the_image() -> Outcome {
    let flush = CanvasExtent {
        x0: 0.5,
        y0: 0.5,
        width: 42.0,
        height: 42.0,
    };
    assert_eq!(flush.rect(CANVAS, CANVAS), Rect::new(0, 0, 42, 42)?);

    let offset = CanvasExtent {
        x0: 42.5,
        y0: 42.5,
        width: 384.0,
        height: 384.0,
    };
    assert_eq!(offset.rect(CANVAS, CANVAS), Rect::new(42, 42, 384, 384)?);

    let one_edge_only = CanvasExtent {
        x0: 42.5,
        y0: 42.5,
        width: 384.3,
        height: 384.3,
    };
    assert_eq!(
        one_edge_only.rect(CANVAS, CANVAS),
        Rect::new(42, 42, 385, 385)?
    );
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

/// HLD 25.1's geometry bullet, verbatim, semicolon included:
///
/// > **Geometry:** world coordinates within 1e-6 mm; canvas coordinates within
/// > a quarter pixel.
///
/// The semicolon is 25.1's own. It was a comma here until the eighth review
/// pass, which is the substitution `SECTION_25_1_MONOCHROME` in
/// `tools/oracle/src/tolerance.rs` spends a paragraph establishing that no
/// lint requires: `scripts/prose_check.py` covers no Rust source at all.
/// Reproduce by asking the checker itself. **Paste this and run it**, which
/// two earlier wrappings of this comment could not survive: the first split
/// the path argument across lines, and the second indented a continuation
/// inside the `-c` string, which Python reads as a second statement and
/// refuses with an IndentationError. The line continuation is the shell's,
/// outside the quotes, so the Python source stays one statement:
///
/// ```text
/// PYTHONPATH=scripts python3 -c \
///   "import prose_check; print(prose_check.in_scope('tools/oracle/tests/geometry_fixture.rs'))"
/// ```
///
/// It prints `False`. A quotation labelled
/// verbatim that is not verbatim costs the label its meaning.
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

/// **The letterbox argument's one unchecked dependency, made checked.**
///
/// The argument at the head of this file, in `CanvasExtent::rect`'s comment
/// and in `docs/lld/comparator.md` is that a wrongly admitted letterbox column
/// is uninformative. `clipped_to_the_same_extreme` in
/// `tools/oracle/src/frame.rs` is
/// `reference == candidate && (reference == 0 || reference == u8::MAX)`, so
/// the step that carries it is not "the same colour on both sides", it is
/// "and that colour is 0 or 255". At `base.background` of `[16, 16, 16]` every
/// letterbox pixel becomes informative, the admitted column enters the bias
/// denominator, and the argument inverts.
///
/// Nothing in the comparator cited that value until the sprint review's sixth
/// pass. The file's digest is compared BETWEEN the two sides by the
/// `the-render-params-digest-disagrees` mutation and never against an expected
/// value, so a change both halves agreed on would be silent. `attribution.rs`
/// already refuses rather than rely on an unchecked coupling with the same
/// file, in `CompareError::NoBiasDenominator`, and this is that shape.
///
/// It asserts the RULE and not today's value: any 8-bit extreme keeps the
/// argument, so `[255, 255, 255]` would pass and `[16, 16, 16]` would not.
#[test]
fn the_declared_background_is_an_eight_bit_extreme() -> Outcome {
    let params: Value = serde_json::from_str(include_str!("../render-params.json"))?;
    let background = params
        .pointer("/base/background")
        .and_then(Value::as_array)
        .ok_or("tools/oracle/render-params.json declares no base.background")?;
    assert_eq!(
        background.len(),
        3,
        "the clear colour is an RGB triple: {background:?}"
    );
    for channel in background {
        let value = channel
            .as_u64()
            .ok_or("a base.background channel is not a whole number")?;
        assert!(
            value == 0 || value == u64::from(u8::MAX),
            "tools/oracle/render-params.json declares base.background \
             {background:?}. A letterbox painted anything but an 8-bit extreme \
             is INFORMATIVE under clipped_to_the_same_extreme, so a wrongly \
             admitted column would enter the bias denominator and the \
             argument in this file's header would be false rather than merely \
             imprecise."
        );
    }
    Ok(())
}
