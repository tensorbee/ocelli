//! F-239 `colour_bytes_into` fixtures, three display bytes per pixel.
//!
//! Every expected byte is hand-computed: the channel values from cornerstone3D
//! 5.8.2's colour window (`getVOILut.js:3`, PS3.3 C.11.2.1.2's LINEAR over
//! `0 ..= 255`), the colour stage from PS3.3 C.7.6.3.1.2 and C.7.6.3.1.5, and
//! the byte from tier C's channel rule, which rounds to the nearest code and a
//! tie to the even one, as `Uint8ClampedArray` does. That tie rule is the
//! operator's decision of 2026-11-07 recorded in F-239's plan. None of the
//! bytes was read back from this function.

use ocelli_core::Stored;
use ocelli_pixel::{
    ColorTransform, ColourWindow, DecodedLayout, DecodedPhotometric, DisplayScale, LutDescriptor,
    PaletteColorLut, PhotometricInterpretation, PixelDataEncoding, PlanarConfiguration, Rgb,
    SampleLayout,
};
use ocelli_render::{
    CpuRaster, RasterColor, RasterError, RasterGrid, RasterQuality, colour_bytes_into,
};
use ocelli_viewport::view::View;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok(), "{result:?}");
        let Ok(value) = result else { return };
        value
    }};
}

fn stored(values: &[f32]) -> Vec<Stored> {
    values.iter().copied().map(Stored).collect()
}

fn native(
    photometric: PhotometricInterpretation,
    planar: Option<PlanarConfiguration>,
    palette: Option<PaletteColorLut>,
) -> Result<ColorTransform, ocelli_pixel::PixelError> {
    let samples = if photometric == PhotometricInterpretation::PaletteColor {
        1
    } else {
        3
    };
    ColorTransform::resolve(
        SampleLayout::new(samples, photometric, planar)?,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        palette,
    )
}

/// `RasterColor` carries no `Debug`, so this cannot go through `require_ok!`.
fn supported(transform: &ColorTransform) -> RasterColor<'_> {
    match RasterColor::supported(transform) {
        Ok(color) => color,
        Err(_) => unreachable!("an unscaled RGB or YBR transform has no palette to refuse"),
    }
}

fn rgb() -> Result<ColorTransform, ocelli_pixel::PixelError> {
    native(
        PhotometricInterpretation::Rgb,
        Some(PlanarConfiguration::Interleaved),
        None,
    )
}

/// Draw one band and return its bytes, or the refusal.
fn bytes_of(
    transform: &ColorTransform,
    window: &ColourWindow,
    samples: &[f32],
    columns: u32,
    pixels: usize,
) -> Result<Vec<u8>, RasterError> {
    let mut scratch = Vec::new();
    let mut out = vec![0_u8; pixels * 3];
    colour_bytes_into(
        RasterColor::supported(transform)?,
        window,
        &stored(samples),
        columns,
        &mut scratch,
        &mut out,
    )?;
    Ok(out)
}

/// Centre, width, invert, the stored RGB and the bytes expected.
type WindowRow = (f32, f32, bool, [f32; 3], [u8; 3]);

/// AC19, one RGB pixel per row, with the hand values of
/// `crates/ocelli-pixel/tests/color.rs` rounded by the byte rule:
///
/// ```text
/// w 100, c 128, 100 30 200 -> 56.666664 0 255           -> 57 0 255
///   inverted                -> 198.33334 255 0           -> 198 255 0
/// w 100, c 128, 128 x3      -> 128.78787                 -> 129 x3
///   inverted                -> 126.21213                 -> 126 x3
/// w 10, c 128, 125 128 130  -> 56.666664 141.66667 198.33334 -> 57 142 198
///   inverted                -> 198.33334 113.33333 56.666656 -> 198 113 57
/// w 2, c 128, 127 128 0     -> 0 255 0
/// w 256, c 128, 10 200 50   -> skipped                   -> 10 200 50
/// w 255, c 127, 100 x3      -> skipped                   -> 100 x3
/// ```
///
/// `w` in place of `w - 1` gives `64 140 191` at width 10.
#[test]
fn colour_window_bytes_follow_the_window_then_the_byte_rule() {
    let transform = require_ok!(rgb());
    let rows: [WindowRow; 11] = [
        (128.0, 100.0, false, [100.0, 30.0, 200.0], [57, 0, 255]),
        (128.0, 100.0, true, [100.0, 30.0, 200.0], [198, 255, 0]),
        (128.0, 100.0, false, [128.0; 3], [129; 3]),
        (128.0, 100.0, true, [128.0; 3], [126; 3]),
        (128.0, 10.0, false, [125.0, 128.0, 130.0], [57, 142, 198]),
        (128.0, 10.0, true, [125.0, 128.0, 130.0], [198, 113, 57]),
        (128.0, 2.0, false, [127.0, 128.0, 0.0], [0, 255, 0]),
        (128.0, 2.0, true, [127.0, 128.0, 0.0], [255, 0, 255]),
        (128.0, 256.0, false, [10.0, 200.0, 50.0], [10, 200, 50]),
        (127.0, 255.0, false, [100.0; 3], [100; 3]),
        (128.0, 256.0, true, [10.0, 200.0, 50.0], [245, 55, 205]),
    ];
    for (center, width, invert, samples, expected) in rows {
        let window = require_ok!(ColourWindow::new(center, width, invert));
        let drawn = require_ok!(bytes_of(&transform, &window, &samples, 1, 1));
        assert_eq!(drawn, expected, "{center} {width} {invert} {samples:?}");
    }
}

/// AC21, exact ties, now rounded to even by the operator's decision.
///
/// ```text
/// w 511, c 128.5, 130 x3 -> ((130 - 128) / 510 + 0.5) * 255 = 128.5 -> 128
///   inverted             -> 255 - 128.5 = 126.5                  -> 126
/// w 100, c 130.5, 130 x3 -> ((130 - 130) / 99 + 0.5) * 255 = 127.5 -> 128
///   inverted             -> 255 - 127.5 = 127.5                  -> 128
/// ```
///
/// All four are exact in `f32`. The first two ties round DOWN to an even code
/// and the last two round UP to one, so a rule that always rounds a tie up or
/// always down fails one pair. cornerstone3D 5.8.2 stores the same bytes,
/// because `Uint8ClampedArray` rounds a tie to even.
#[test]
fn a_colour_window_tie_rounds_half_to_even() {
    let transform = require_ok!(rgb());
    for (center, width, invert, expected) in [
        (128.5_f32, 511.0_f32, false, 128_u8),
        (128.5, 511.0, true, 126),
        (130.5, 100.0, false, 128),
        (130.5, 100.0, true, 128),
    ] {
        let window = require_ok!(ColourWindow::new(center, width, invert));
        assert_eq!(
            require_ok!(bytes_of(&transform, &window, &[130.0; 3], 1, 1)),
            [expected; 3],
            "{center} {width} {invert}"
        );
    }
}

/// AC3's two YBR_FULL rows, the library inverse of PS3.3 C.7.6.3.1.2 and the
/// byte rule. `100 150 90` is `46.72366 119.56544 138.98863`, and
/// `30 200 60` is `-65.3378 53.7829 157.59158`, whose negative red stores 0
/// rather than wrapping to 191.
#[test]
fn ybr_full_bytes_use_the_library_inverse() {
    let transform = require_ok!(native(
        PhotometricInterpretation::YbrFull,
        Some(PlanarConfiguration::Interleaved),
        None
    ));
    assert_eq!(
        require_ok!(bytes_of(
            &transform,
            &ColourWindow::initial(),
            &[100.0, 150.0, 90.0, 30.0, 200.0, 60.0],
            2,
            2
        )),
        [47, 120, 139, 0, 54, 158]
    );
}

/// AC11's bytes. RGB at Bits Stored 12, `4095 2048 15`, shifts by 4 to
/// `255 128 0`. Without the scale, 4095 is not a display byte and the band is
/// refused, which is the tier C rule this scale exists to satisfy.
#[test]
fn a_twelve_bit_colour_sample_is_shifted_before_its_byte() {
    let transform = require_ok!(rgb());
    let scaled = require_ok!(
        transform
            .clone()
            .with_sample_scale(require_ok!(DisplayScale::from_bits(12)))
    );
    assert_eq!(
        require_ok!(bytes_of(
            &scaled,
            &ColourWindow::initial(),
            &[4095.0, 2048.0, 15.0],
            1,
            1
        )),
        [255, 128, 0]
    );
    assert_eq!(
        bytes_of(
            &transform,
            &ColourWindow::initial(),
            &[4095.0, 2048.0, 15.0],
            1,
            1
        ),
        Err(RasterError::Unavailable)
    );
}

/// AC22's bytes. Descriptors `5\0\16` and entries above 255, so ruling R1
/// does not read the palette as 8-bit and every channel shifts by 8:
///
/// ```text
/// red   0 255 256 32768 65535 -> 0 0 1 128 255
/// green 65535 32768 256 255 0 -> 255 128 1 0 0, blue the same
/// ```
///
/// Before F-239 the 255-plus entries were not display bytes and the palette
/// was refused here.
#[test]
fn sixteen_bit_palette_entries_are_shifted_before_their_byte() {
    let red = require_ok!(LutDescriptor::new(
        5,
        0,
        16,
        vec![0.0, 255.0, 256.0, 32_768.0, 65_535.0]
    ));
    let falling = vec![65_535.0, 32_768.0, 256.0, 255.0, 0.0];
    let green = require_ok!(LutDescriptor::new(5, 0, 16, falling.clone()));
    let blue = require_ok!(LutDescriptor::new(5, 0, 16, falling));
    let palette = require_ok!(PaletteColorLut::new(red, green, blue));
    let transform = require_ok!(native(
        PhotometricInterpretation::PaletteColor,
        None,
        Some(palette)
    ));
    assert_eq!(
        require_ok!(bytes_of(
            &transform,
            &ColourWindow::initial(),
            &[0.0, 1.0, 2.0, 3.0, 4.0],
            5,
            5
        )),
        [0, 255, 255, 0, 128, 128, 1, 1, 1, 128, 0, 0, 255, 0, 0]
    );
}

/// The shared steps. At an identity view whose target is the source grid,
/// `raster_color_into` and `colour_bytes_into` draw the same RGB, so the
/// accept, map and byte steps are one routine and not two that agree today.
#[test]
fn colour_bytes_agree_with_the_raster_at_an_identity_grid() {
    let samples = [
        10.0, 20.0, 30.0, 200.0, 100.0, 50.0, 100.0, 150.0, 90.0, 30.0, 200.0, 60.0,
    ];
    for photometric in [
        PhotometricInterpretation::Rgb,
        PhotometricInterpretation::YbrFull,
    ] {
        let transform = require_ok!(native(
            photometric,
            Some(PlanarConfiguration::Interleaved),
            None
        ));
        let mut raster = require_ok!(CpuRaster::open(2.0, 2.0, 1.0));
        assert_eq!(raster.device(), [2, 2]);
        assert!(raster.reserve_source(2, 2).is_ok());
        let mut rgba = vec![0_u8; 16];
        let drawn = raster.raster_color_into(
            &View::new(),
            supported(&transform),
            &stored(&samples),
            RasterGrid {
                source: [2, 2],
                target: [2, 2],
            },
            RasterQuality::Settled { generation: 0 },
            &mut rgba,
        );
        assert!(matches!(drawn, Ok(result) if result.wrote));
        let from_raster: Vec<u8> = rgba
            .chunks_exact(4)
            .flat_map(|pixel| pixel.iter().take(3).copied())
            .collect();
        let from_bytes = require_ok!(bytes_of(
            &transform,
            &ColourWindow::initial(),
            &samples,
            2,
            4
        ));
        assert_eq!(from_bytes, from_raster, "{photometric:?}");
    }
}

/// A band is any whole number of rows, and the scratch is grown once and
/// reused for every band no larger than the first.
#[test]
fn bands_reuse_the_scratch_and_cover_whole_rows() {
    let transform = require_ok!(rgb());
    let accepted = supported(&transform);
    let mut scratch: Vec<Rgb> = Vec::new();
    let mut out = [0_u8; 12];
    assert_eq!(
        colour_bytes_into(
            accepted,
            &ColourWindow::initial(),
            &stored(&[
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0
            ]),
            2,
            &mut scratch,
            &mut out,
        ),
        Ok(())
    );
    assert_eq!(out, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    let grown = scratch.capacity();
    assert!(scratch.len() >= 4);

    let mut row = [0_u8; 6];
    assert_eq!(
        colour_bytes_into(
            supported(&transform),
            &ColourWindow::initial(),
            &stored(&[9.0, 8.0, 7.0, 6.0, 5.0, 4.0]),
            2,
            &mut scratch,
            &mut row,
        ),
        Ok(())
    );
    assert_eq!(row, [9, 8, 7, 6, 5, 4]);
    assert_eq!(scratch.capacity(), grown);
}

/// Every refusal writes nothing.
#[test]
fn a_refused_band_writes_nothing() {
    let transform = require_ok!(rgb());
    let window = ColourWindow::initial();
    let mut scratch = Vec::new();
    let refuse = |color: RasterColor<'_>,
                  samples: &[f32],
                  columns: u32,
                  out: &mut [u8],
                  scratch: &mut Vec<Rgb>| {
        colour_bytes_into(color, &window, &stored(samples), columns, scratch, out)
    };

    let mut out = [7_u8; 3];
    assert_eq!(
        refuse(
            RasterColor::Unsupported,
            &[1.0, 2.0, 3.0],
            1,
            &mut out,
            &mut scratch
        ),
        Err(RasterError::Unavailable)
    );
    assert_eq!(out, [7; 3]);

    let mut ragged = [7_u8; 4];
    assert_eq!(
        refuse(
            supported(&transform),
            &[1.0, 2.0, 3.0],
            1,
            &mut ragged,
            &mut scratch
        ),
        Err(RasterError::OutputLength)
    );
    assert_eq!(ragged, [7; 4]);

    for columns in [0_u32, 2] {
        assert_eq!(
            refuse(
                supported(&transform),
                &[1.0, 2.0, 3.0],
                columns,
                &mut out,
                &mut scratch
            ),
            Err(RasterError::Extent),
            "{columns}"
        );
        assert_eq!(out, [7; 3]);
    }

    assert_eq!(
        refuse(
            supported(&transform),
            &[1.0, 2.0],
            1,
            &mut out,
            &mut scratch
        ),
        Err(RasterError::Length)
    );
    assert_eq!(
        refuse(
            supported(&transform),
            &[256.0, 0.0, 0.0],
            1,
            &mut out,
            &mut scratch
        ),
        Err(RasterError::Unavailable)
    );
    assert_eq!(out, [7; 3]);

    let mut empty: [u8; 0] = [];
    assert_eq!(
        refuse(supported(&transform), &[], 1, &mut empty, &mut scratch),
        Err(RasterError::Extent)
    );
}
