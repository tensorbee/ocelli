//! F-X003 CPU stack fixtures.
//!
//! Display literals are copied from `ocelli-pixel`'s PS3.3 C.11.2.1 rows.
//! They are not read back from this raster. Grey and Stops index the existing
//! preset tables, which round the address to the nearest code, ties up. The
//! display value itself is asserted before that quantisation.

use ocelli_core::{Canvas, Pt, Stored};
use ocelli_pixel::{
    ColorTransform, DecodedLayout, DecodedPhotometric, LutChain, LutDescriptor, ModalityTransform,
    PaletteColorLut, PhotometricInterpretation, PixelDataEncoding, PlanarConfiguration,
    PresentationLutEvidence, SampleLayout, VoiFunction, VoiTransform,
};
use ocelli_render::{CpuRaster, RasterColor, RasterError, RasterGrid, RasterQuality};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::view::{Radians, View, Zoom};

const TOLERANCE: f32 = 0.001;

fn require_raster(css_width: f64, css_height: f64, dpr: f64) -> CpuRaster {
    let built = CpuRaster::open(css_width, css_height, dpr);
    assert!(built.is_ok());
    match built {
        Ok(raster) => raster,
        Err(_) => unreachable!("canvas inputs are finite and positive"),
    }
}

fn modality(slope: f32, intercept: f32) -> ModalityTransform {
    let built = ModalityTransform::new(None, Some(slope), Some(intercept));
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("slope and intercept are finite"),
    }
}

fn window(center: f32, width: f32, function: VoiFunction) -> VoiTransform {
    let built = VoiTransform::new(None, &[center], &[width], 0, function, 0.0, 255.0);
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("window is inside the VOI domain"),
    }
}

fn chain(
    slope: f32,
    intercept: f32,
    center: f32,
    width: f32,
    function: VoiFunction,
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
) -> LutChain {
    let built = LutChain::new(
        modality(slope, intercept),
        window(center, width, function),
        photometric,
        evidence,
    );
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("monochrome chain is in domain"),
    }
}

struct VoiSpec {
    center: f32,
    width: f32,
    function: VoiFunction,
}

fn presentation_with(
    slope: f32,
    intercept: f32,
    voi: VoiSpec,
    photometric: PhotometricInterpretation,
    evidence: PresentationLutEvidence,
    colormap: Colormap,
) -> Presentation {
    Presentation::new(
        chain(
            slope,
            intercept,
            voi.center,
            voi.width,
            voi.function,
            photometric,
            evidence,
        ),
        photometric,
        evidence,
        colormap,
    )
}

fn passthrough() -> Presentation {
    presentation_with(
        1.0,
        0.0,
        VoiSpec {
            center: 127.5,
            width: 255.0,
            function: VoiFunction::LinearExact,
        },
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    )
}

fn stored(values: &[f32]) -> Vec<Stored> {
    values.iter().copied().map(Stored).collect()
}

fn rgba_at(frame: &[u8], index: usize) -> [u8; 4] {
    let start = index.saturating_mul(4);
    let Some(bytes) = frame.get(start..start.saturating_add(4)) else {
        unreachable!("the frame was sized to the target");
    };
    [
        bytes.first().copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
        bytes.get(2).copied().unwrap_or(0),
        bytes.get(3).copied().unwrap_or(0),
    ]
}

fn draw(
    raster: &mut CpuRaster,
    view: &View,
    presentation: &Presentation,
    samples: &[Stored],
    source: [u32; 2],
) -> Vec<u8> {
    let target = raster.device();
    let bytes = usize::try_from(target[0])
        .unwrap_or(0)
        .saturating_mul(usize::try_from(target[1]).unwrap_or(0))
        .saturating_mul(4);
    let mut out = vec![7_u8; bytes];
    let before = raster.allocations();
    let drawn = raster.raster_into(
        view,
        presentation,
        samples,
        RasterGrid { source, target },
        RasterQuality::Settled {
            generation: raster.generation(),
        },
        &mut out,
    );
    assert!(drawn.is_ok());
    match drawn {
        Ok(result) => {
            assert!(result.wrote);
            assert_eq!(result.extent, target);
            assert!(!result.refining);
        }
        Err(_) => unreachable!("asserted immediately above"),
    }
    assert_eq!(raster.allocations(), before);
    out
}

/// Grey-table byte for a passthrough stored value.
///
/// LINEAR_EXACT identity is not an integer in `f32`, and the preset table
/// rounds to the nearest code, ties up, so the channel is `stored` unless the
/// stored value itself is fractional. Neighbouring texels in these fixtures
/// are at least ten apart, and an outside sample is opaque black.
fn assert_grey_near(frame: &[u8], index: usize, stored: u8) {
    let pixel = rgba_at(frame, index);
    assert!(
        pixel[0].abs_diff(stored) <= 1,
        "channel {} against stored {stored}",
        pixel[0]
    );
    assert_eq!(pixel[1], pixel[0]);
    assert_eq!(pixel[2], pixel[0]);
    assert_eq!(pixel[3], 255);
}

fn assert_close(actual: f32, expected: f32) {
    let difference = (actual - expected).abs();
    assert!(
        difference < TOLERANCE,
        "was {actual}, wanted {expected}, difference {difference}"
    );
}

fn assert_display(actual: Result<Option<f32>, RasterError>, expected: f32) {
    assert!(actual.is_ok());
    match actual {
        Ok(Some(value)) => assert_close(value, expected),
        Ok(None) => unreachable!("the sample is inside the image"),
        Err(_) => unreachable!("display_at was asserted ok"),
    }
}

#[test]
fn section_18_3_display_quantizes_to_the_nearest_grey_byte() {
    // PS3.3 C.11.2.1.2 and C.11.2.1.3.2 at centre 40, width 400, output 0..255.
    // Both centre rows quantize to the same nearest byte, so the display
    // literals are what discriminate them. The nearest code is 128 for both:
    // 127.81955 rounds up from .819 and 127.5 ties up. The pinned reference
    // quantises its table with `Math.floor(value * 255 + 0.5)`, deviation D-11
    // makes that reference the definition of correct, and HLD 25.1's
    // signed-mean bound rejects the one-code truncation that used to sit here.
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let view = View::new();
    let linear = presentation_with(
        1.0,
        0.0,
        VoiSpec {
            center: 40.0,
            width: 400.0,
            function: VoiFunction::Linear,
        },
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    let exact = presentation_with(
        1.0,
        0.0,
        VoiSpec {
            center: 40.0,
            width: 400.0,
            function: VoiFunction::LinearExact,
        },
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    let samples = stored(&[40.0]);
    assert_display(
        raster.display_at(&view, &linear, &samples, [1, 1], [1, 1], [0, 0]),
        127.819_55,
    );
    assert_display(
        raster.display_at(&view, &exact, &samples, [1, 1], [1, 1], [0, 0]),
        127.5,
    );
    let linear_bytes = draw(&mut raster, &view, &linear, &samples, [1, 1]);
    raster.invalidate();
    let exact_bytes = draw(&mut raster, &view, &exact, &samples, [1, 1]);
    assert_eq!(rgba_at(&linear_bytes, 0), [128, 128, 128, 255]);
    assert_eq!(rgba_at(&exact_bytes, 0), [128, 128, 128, 255]);
}

#[test]
fn slope_and_monochrome1_and_sigmoid_use_the_chain_once() {
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let view = View::new();
    // Stored 0, slope 2, intercept 40 lands on modality 40. Ignoring the
    // slope leaves modality 0, whose LINEAR display rounds to 102.
    let scaled = presentation_with(
        2.0,
        40.0,
        VoiSpec {
            center: 40.0,
            width: 400.0,
            function: VoiFunction::Linear,
        },
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    let zero = stored(&[0.0]);
    assert_display(
        raster.display_at(&view, &scaled, &zero, [1, 1], [1, 1], [0, 0]),
        127.819_55,
    );
    assert_eq!(
        rgba_at(&draw(&mut raster, &view, &scaled, &zero, [1, 1]), 0),
        [128, 128, 128, 255]
    );

    // Stored -60 LINEAR is 63.909775. MONOCHROME1 reflects it inside 0..255.
    // 255 - 63.909775 = 191.090225, and the grey table rounds to 191.
    raster.invalidate();
    let mono1 = presentation_with(
        1.0,
        0.0,
        VoiSpec {
            center: 40.0,
            width: 400.0,
            function: VoiFunction::Linear,
        },
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    let minus_sixty = stored(&[-60.0]);
    assert_display(
        raster.display_at(&view, &mono1, &minus_sixty, [1, 1], [1, 1], [0, 0]),
        191.090_23,
    );
    assert_eq!(
        rgba_at(&draw(&mut raster, &view, &mono1, &minus_sixty, [1, 1]), 0),
        [191, 191, 191, 255]
    );

    // PS3.3 C.11.2.1.3.1. At x=-60, c=40, w=400 the exponent is +1.
    // 255 / (1 + e) is about 68.57. The nearest code is 69. LINEAR would be
    // 63.91, which rounds to 64.
    raster.invalidate();
    let sigmoid = presentation_with(
        1.0,
        0.0,
        VoiSpec {
            center: 40.0,
            width: 400.0,
            function: VoiFunction::Sigmoid,
        },
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    assert_display(
        raster.display_at(&view, &sigmoid, &minus_sixty, [1, 1], [1, 1], [0, 0]),
        255.0 / (1.0 + core::f32::consts::E),
    );
    assert_eq!(
        rgba_at(&draw(&mut raster, &view, &sigmoid, &minus_sixty, [1, 1]), 0),
        [69, 69, 69, 255]
    );

    raster.invalidate();
    let mut inverse = presentation_with(
        1.0,
        0.0,
        VoiSpec {
            center: 40.0,
            width: 400.0,
            function: VoiFunction::Linear,
        },
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    );
    assert!(inverse.set_invert(true).is_ok());
    assert_eq!(
        rgba_at(&draw(&mut raster, &view, &inverse, &minus_sixty, [1, 1]), 0),
        [191, 191, 191, 255]
    );
}

#[test]
fn stops_indexes_128_as_red_and_grey_does_not() {
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let view = View::new();
    let mut stops = passthrough();
    stops.set_colormap(Colormap::Stops);
    let frame = draw(&mut raster, &view, &stops, &stored(&[128.0]), [1, 1]);
    assert_eq!(rgba_at(&frame, 0), [255, 0, 0, 255]);
    raster.invalidate();
    let grey = draw(
        &mut raster,
        &view,
        &passthrough(),
        &stored(&[128.0]),
        [1, 1],
    );
    assert_eq!(rgba_at(&grey, 0), [128, 128, 128, 255]);
}

#[test]
fn dpr_two_keeps_the_left_half_of_texel_zero() {
    // CSS 1 by 1 at dpr 2 is a 2 by 2 device grid. Pixel 0's centre is device
    // (0.5, 0.5), CSS (0.25, 0.25), image 0.25. That is still texel 0.
    // Dropping the half-texel offset makes the image coordinate -0.25 and
    // the pixel is rejected as outside.
    let mut raster = require_raster(1.0, 1.0, 2.0);
    assert_eq!(raster.device(), [2, 2]);
    let frame = draw(
        &mut raster,
        &View::new(),
        &passthrough(),
        &stored(&[100.0]),
        [1, 1],
    );
    assert_eq!(rgba_at(&frame, 0), [100, 100, 100, 255]);
    assert_eq!(rgba_at(&frame, 3), [100, 100, 100, 255]);
}

#[test]
fn dpr_one_and_a_quarter_uses_one_division() {
    // css 4 by 1 at dpr 1.25. round(4 * 1.25) = 5 and round(1.25) = 1.
    // Pixel 4's centre is device 4.5, CSS 4.5 / 1.25 = 3.6, texel 3.
    // Treating the device coordinate as CSS puts 4.5 outside a 4-column image.
    let mut raster = require_raster(4.0, 1.0, 1.25);
    assert_eq!(raster.device(), [5, 1]);
    let frame = draw(
        &mut raster,
        &View::new(),
        &passthrough(),
        &stored(&[10.0, 20.0, 30.0, 40.0]),
        [4, 1],
    );
    assert_grey_near(&frame, 4, 40);
    assert_grey_near(&frame, 0, 10);
}

#[test]
fn identity_negative_pan_and_a_two_pixel_border() {
    let mut raster = require_raster(2.0, 2.0, 1.0);
    let samples = stored(&[10.0, 20.0, 30.0, 40.0]);
    let frame = draw(&mut raster, &View::new(), &passthrough(), &samples, [2, 2]);
    assert_grey_near(&frame, 0, 10);
    assert_grey_near(&frame, 1, 20);
    assert_grey_near(&frame, 2, 30);
    assert_grey_near(&frame, 3, 40);

    raster.invalidate();
    let wide = require_raster(2.0, 1.0, 1.0);
    let mut wide = wide;
    let border = draw(
        &mut wide,
        &View::new(),
        &passthrough(),
        &stored(&[200.0]),
        [1, 1],
    );
    assert_grey_near(&border, 0, 200);
    assert_eq!(rgba_at(&border, 1), [0, 0, 0, 255]);

    let mut panned = View::new();
    assert!(panned.set_pan(Pt::<Canvas>::new(1.0, 0.0, 0.0)).is_ok());
    let mut one = require_raster(1.0, 1.0, 1.0);
    let outside = draw(&mut one, &panned, &passthrough(), &stored(&[100.0]), [1, 1]);
    assert_eq!(rgba_at(&outside, 0), [0, 0, 0, 255]);
    assert!(matches!(
        one.display_at(
            &panned,
            &passthrough(),
            &stored(&[100.0]),
            [1, 1],
            [1, 1],
            [0, 0]
        ),
        Ok(None)
    ));
}

#[test]
fn fit_anisotropic_spacing_shift_flip_quarter_turn_and_zoom() {
    let mut fitted = View::new();
    fitted.fit(4.0, 2.0, 2, 2);
    let mut raster = require_raster(4.0, 2.0, 1.0);
    assert_eq!(raster.device(), [4, 2]);
    let samples = stored(&[10.0, 20.0, 30.0, 40.0]);
    let frame = draw(&mut raster, &fitted, &passthrough(), &samples, [2, 2]);
    assert_eq!(rgba_at(&frame, 0), [0, 0, 0, 255]);
    assert_grey_near(&frame, 1, 10);
    assert_grey_near(&frame, 2, 20);
    assert_eq!(rgba_at(&frame, 3), [0, 0, 0, 255]);

    let mut spacing = View::new();
    spacing.set_spacing(2.0, 1.0);
    spacing.fit(8.0, 1.0, 4, 1);
    let mut wide = require_raster(8.0, 1.0, 1.0);
    assert_eq!(wide.device(), [8, 1]);
    let row = draw(
        &mut wide,
        &spacing,
        &passthrough(),
        &stored(&[11.0, 22.0, 33.0, 44.0]),
        [4, 1],
    );
    assert_grey_near(&row, 3, 22);

    let mut shifted = View::new();
    shifted.set_index_shift(1.0, 0.0);
    let mut pair = require_raster(2.0, 1.0, 1.0);
    let moved = draw(
        &mut pair,
        &shifted,
        &passthrough(),
        &stored(&[10.0, 20.0]),
        [2, 1],
    );
    assert_eq!(rgba_at(&moved, 0), [0, 0, 0, 255]);
    assert_grey_near(&moved, 1, 10);

    let mut flipped = View::new();
    flipped.set_flip_horizontal(true);
    pair.invalidate();
    let mirror = draw(
        &mut pair,
        &flipped,
        &passthrough(),
        &stored(&[10.0, 20.0]),
        [2, 1],
    );
    assert_grey_near(&mirror, 0, 20);
    assert_grey_near(&mirror, 1, 10);

    let mut turned = View::new();
    turned.set_rotation(Radians::new(std::f64::consts::FRAC_PI_2));
    let mut square = require_raster(2.0, 2.0, 1.0);
    let rotated = draw(&mut square, &turned, &passthrough(), &samples, [2, 2]);
    assert_grey_near(&rotated, 0, 30);

    let mut zoomed = View::new();
    zoomed.set_zoom(Zoom::new(2.0));
    let mut zoom_raster = require_raster(4.0, 1.0, 1.0);
    let zoomed_frame = draw(
        &mut zoom_raster,
        &zoomed,
        &passthrough(),
        &stored(&[10.0, 20.0, 30.0, 40.0]),
        [4, 1],
    );
    assert_grey_near(&zoomed_frame, 0, 20);
}

#[test]
fn a_singular_map_writes_nothing_and_leaves_the_view() {
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let mut view = View::new();
    view.set_zoom(Zoom::new(0.0));
    let mut out = [3_u8; 4];
    let drawn = raster.raster_into(
        &view,
        &passthrough(),
        &stored(&[1.0]),
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut out,
    );
    assert!(matches!(drawn, Err(RasterError::Singular)));
    assert_eq!(out, [3, 3, 3, 3]);
    assert_eq!(view.scale().to_bits(), 0.0_f64.to_bits());

    let mut flat = View::new();
    flat.set_spacing(0.0, 0.0);
    let again = raster.raster_into(
        &flat,
        &passthrough(),
        &stored(&[1.0]),
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut out,
    );
    assert!(matches!(again, Err(RasterError::Singular)));
    assert_eq!(out, [3, 3, 3, 3]);
}

#[test]
fn interaction_settle_and_idle_follow_the_generation() {
    let mut raster = require_raster(2.0, 1.0, 1.0);
    let view = View::new();
    let presentation = passthrough();
    let samples = stored(&[10.0, 20.0]);
    let generation = raster.begin_interaction();
    assert_eq!(generation, 1);
    assert!(raster.refining());
    let mut reduced = [9_u8; 4];
    let interacting = raster.raster_into(
        &view,
        &presentation,
        &samples,
        RasterGrid {
            source: [2, 1],
            target: [1, 1],
        },
        RasterQuality::Interacting,
        &mut reduced,
    );
    assert!(interacting.is_ok());
    assert!(raster.refining());
    let mut stale_out = [9_u8; 8];
    let stale = raster.raster_into(
        &view,
        &presentation,
        &samples,
        RasterGrid {
            source: [2, 1],
            target: raster.device(),
        },
        RasterQuality::Settled { generation: 0 },
        &mut stale_out,
    );
    assert!(matches!(stale, Err(RasterError::Stale)));
    assert_eq!(stale_out, [9_u8; 8]);
    assert!(raster.refining());
    let settled = draw(&mut raster, &view, &presentation, &samples, [2, 1]);
    assert!(!raster.refining());
    assert_grey_near(&settled, 0, 10);
    let mut idle = [4_u8; 8];
    let second = raster.raster_into(
        &view,
        &presentation,
        &samples,
        RasterGrid {
            source: [2, 1],
            target: raster.device(),
        },
        RasterQuality::Settled { generation: 1 },
        &mut idle,
    );
    assert!(matches!(
        second,
        Ok(result) if !result.wrote && result.extent == [0, 0]
    ));
    assert_eq!(idle, [4_u8; 8]);
    raster.invalidate();
    let again = draw(&mut raster, &view, &presentation, &samples, [2, 1]);
    assert_grey_near(&again, 1, 20);
}

#[test]
fn a_dpr_change_with_the_same_device_extent_drops_the_settled_frame() {
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let view = View::new();
    let presentation = passthrough();
    let samples = stored(&[100.0]);
    let first = draw(&mut raster, &view, &presentation, &samples, [1, 1]);
    assert_grey_near(&first, 0, 100);
    assert_eq!(raster.generation(), 0);
    let resized = raster.set_device_pixel_ratio(1.1);
    assert!(resized.is_ok());
    assert_eq!(raster.device(), [1, 1]);
    assert_ne!(raster.generation(), 0);
    let mut stale = [9_u8; 4];
    let old = raster.raster_into(
        &view,
        &presentation,
        &samples,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut stale,
    );
    assert!(matches!(old, Err(RasterError::Stale)));
    assert_eq!(stale, [9, 9, 9, 9]);
    let redrawn = draw(&mut raster, &view, &presentation, &samples, [1, 1]);
    assert_grey_near(&redrawn, 0, 100);
    let mut idle = [4_u8; 4];
    let second = raster.raster_into(
        &view,
        &presentation,
        &samples,
        RasterGrid {
            source: [1, 1],
            target: raster.device(),
        },
        RasterQuality::Settled {
            generation: raster.generation(),
        },
        &mut idle,
    );
    assert!(matches!(
        second,
        Ok(result) if !result.wrote && result.extent == [0, 0]
    ));
    assert_eq!(idle, [4, 4, 4, 4]);
    let refused = raster.set_device_pixel_ratio(f64::NAN);
    assert!(matches!(refused, Err(RasterError::Canvas)));
    assert_eq!(raster.generation(), 1);
    let still = raster.raster_into(
        &view,
        &presentation,
        &samples,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 1 },
        &mut idle,
    );
    assert!(matches!(
        still,
        Ok(result) if !result.wrote && result.extent == [0, 0]
    ));
}

#[test]
fn a_colour_source_larger_than_the_target_is_drawn() {
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let view = View::new();
    let rgb = require_color(PhotometricInterpretation::Rgb, None);
    let before = raster.allocations();
    assert!(raster.reserve_source(2, 1).is_ok());
    let reserved = raster.allocations();
    assert!(reserved > before);
    let mut out = [3_u8; 4];
    let drawn = raster.raster_color_into(
        &view,
        RasterColor::Supported(&rgb),
        &stored(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0]),
        RasterGrid {
            source: [2, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut out,
    );
    assert!(drawn.is_ok());
    assert_eq!(out, [10, 20, 30, 255]);
    assert_eq!(raster.allocations(), reserved);

    let mut wide = require_raster(2.0, 1.0, 1.0);
    assert!(wide.reserve_source(1, 1).is_ok());
    let mut smaller = [3_u8; 8];
    let fitted = wide.raster_color_into(
        &view,
        RasterColor::Supported(&rgb),
        &stored(&[10.0, 20.0, 30.0]),
        RasterGrid {
            source: [1, 1],
            target: [2, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut smaller,
    );
    assert!(fitted.is_ok());
    assert_eq!(rgba_at(&smaller, 0), [10, 20, 30, 255]);
    assert_eq!(rgba_at(&smaller, 1), [0, 0, 0, 255]);

    let red = descriptor(0, &[10.0, 20.0]);
    let green = descriptor(0, &[30.0, 40.0]);
    let blue = descriptor(0, &[50.0, 60.0]);
    let palette = PaletteColorLut::new(red, green, blue);
    assert!(palette.is_ok());
    let Ok(palette) = palette else { return };
    let keyed = require_color(PhotometricInterpretation::PaletteColor, Some(palette));
    assert!(raster.reserve_source(2, 1).is_ok());
    raster.invalidate();
    let keyed_draw = raster.raster_color_into(
        &view,
        RasterColor::Supported(&keyed),
        &stored(&[0.0, 1.0]),
        RasterGrid {
            source: [2, 1],
            target: [1, 1],
        },
        RasterQuality::Settled {
            generation: raster.generation(),
        },
        &mut out,
    );
    assert!(keyed_draw.is_ok());
    assert_eq!(out, [10, 30, 50, 255]);
}

#[test]
fn supported_colour_is_drawn_and_icc_is_not_scored() {
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let view = View::new();
    let rgb = require_color(PhotometricInterpretation::Rgb, None);
    assert!(raster.reserve_source(1, 1).is_ok());
    let mut out = [0_u8; 4];
    let drawn = raster.raster_color_into(
        &view,
        RasterColor::Supported(&rgb),
        &stored(&[10.0, 20.0, 30.0]),
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut out,
    );
    assert!(drawn.is_ok());
    assert_eq!(out, [10, 20, 30, 255]);

    let red = descriptor(0, &[10.0, 20.0]);
    let green = descriptor(0, &[30.0, 40.0]);
    let blue = descriptor(0, &[50.0, 60.0]);
    let palette = PaletteColorLut::new(red, green, blue);
    assert!(palette.is_ok());
    let Ok(palette) = palette else { return };
    let transform = require_color(PhotometricInterpretation::PaletteColor, Some(palette));
    raster.invalidate();
    let colored = raster.raster_color_into(
        &view,
        RasterColor::Supported(&transform),
        &stored(&[1.0]),
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut out,
    );
    assert!(colored.is_ok());
    assert_eq!(out, [20, 40, 60, 255]);

    let mut refused = [9_u8; 4];
    let icc = raster.raster_color_into(
        &view,
        RasterColor::Unsupported,
        &stored(&[1.0, 2.0, 3.0]),
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut refused,
    );
    assert!(matches!(icc, Err(RasterError::Unavailable)));
    assert_eq!(refused, [9, 9, 9, 9]);
}

#[test]
fn settled_idle_rejects_a_source_that_does_not_match_the_layout() {
    let view = View::new();
    let presentation = passthrough();
    let mut raster = require_raster(1.0, 1.0, 1.0);
    let mono = stored(&[40.0]);
    let mut frame = [1_u8; 4];
    let drawn = raster.raster_into(
        &view,
        &presentation,
        &mono,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut frame,
    );
    assert!(matches!(drawn, Ok(result) if result.wrote));
    let mut refused = [2_u8; 4];
    let colour = raster.raster_rgb_into(
        &view,
        &mono,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut refused,
    );
    assert!(matches!(colour, Err(RasterError::Length)));
    assert_eq!(refused, [2, 2, 2, 2]);
    let mut idle_out = [3_u8; 4];
    let idle = raster.raster_into(
        &view,
        &presentation,
        &mono,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut idle_out,
    );
    assert!(matches!(
        idle,
        Ok(result) if !result.wrote && result.extent == [0, 0]
    ));
    assert_eq!(idle_out, [3, 3, 3, 3]);

    let mut colour_raster = require_raster(1.0, 1.0, 1.0);
    assert!(colour_raster.reserve_source(1, 1).is_ok());
    let rgb = stored(&[10.0, 20.0, 30.0]);
    let mut coloured = [4_u8; 4];
    let settled = colour_raster.raster_rgb_into(
        &view,
        &rgb,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut coloured,
    );
    assert!(matches!(settled, Ok(result) if result.wrote));
    assert_eq!(coloured, [10, 20, 30, 255]);
    let mut mono_refused = [5_u8; 4];
    let back = colour_raster.raster_into(
        &view,
        &presentation,
        &rgb,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut mono_refused,
    );
    assert!(matches!(back, Err(RasterError::Length)));
    assert_eq!(mono_refused, [5, 5, 5, 5]);
    assert_eq!(coloured, [10, 20, 30, 255]);
    let mut colour_idle = [6_u8; 4];
    let again = colour_raster.raster_rgb_into(
        &view,
        &rgb,
        RasterGrid {
            source: [1, 1],
            target: [1, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut colour_idle,
    );
    assert!(matches!(
        again,
        Ok(result) if !result.wrote && result.extent == [0, 0]
    ));
    assert_eq!(colour_idle, [6, 6, 6, 6]);
    assert_eq!(colour_raster.allocations(), 1);
}

#[test]
fn ybr_partial_422_keeps_studio_black_and_the_matrix_row() {
    let mut raster = require_raster(2.0, 1.0, 1.0);
    let view = View::new();
    let transform = require_color(PhotometricInterpretation::YbrPartial422, None);
    assert!(raster.reserve_source(2, 1).is_ok());
    let mut black = [5_u8; 8];
    let drawn = raster.raster_color_into(
        &view,
        RasterColor::Supported(&transform),
        &stored(&[16.0, 16.0, 128.0, 128.0]),
        RasterGrid {
            source: [2, 1],
            target: [2, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut black,
    );
    assert!(drawn.is_ok());
    assert_eq!(rgba_at(&black, 0), [0, 0, 0, 255]);
    assert_eq!(rgba_at(&black, 1), [0, 0, 0, 255]);

    // [81, 90, 240] is the pixel fixture row (254.44283, -0.4849587, -0.9851914).
    // The byte store maps the negative channels to 0 and the red channel to 254.
    raster.invalidate();
    let mut row = [5_u8; 8];
    let colored = raster.raster_color_into(
        &view,
        RasterColor::Supported(&transform),
        &stored(&[81.0, 81.0, 90.0, 240.0]),
        RasterGrid {
            source: [2, 1],
            target: [2, 1],
        },
        RasterQuality::Settled { generation: 0 },
        &mut row,
    );
    assert!(colored.is_ok());
    assert_eq!(rgba_at(&row, 0), [254, 0, 0, 255]);
    assert_eq!(rgba_at(&row, 1), [254, 0, 0, 255]);
}

fn descriptor(first: i32, values: &[f32]) -> LutDescriptor {
    let count = u16::try_from(values.len()).unwrap_or(0);
    let built = LutDescriptor::new(count, first, 8, values.to_vec());
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("palette descriptor is in domain"),
    }
}

fn require_color(
    photometric: PhotometricInterpretation,
    palette: Option<PaletteColorLut>,
) -> ColorTransform {
    let samples = match photometric {
        PhotometricInterpretation::PaletteColor => 1,
        _ => 3,
    };
    let planar = if samples == 1 {
        None
    } else {
        Some(PlanarConfiguration::Interleaved)
    };
    let layout = SampleLayout::new(samples, photometric, planar);
    assert!(layout.is_ok());
    let Ok(layout) = layout else {
        unreachable!("sample layout matches the photometric interpretation");
    };
    let built = ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        palette,
    );
    assert!(built.is_ok());
    match built {
        Ok(value) => value,
        Err(_) => unreachable!("colour transform resolved"),
    }
}
