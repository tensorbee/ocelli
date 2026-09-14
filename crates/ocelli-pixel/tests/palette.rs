//! DICOM PS3.3 C.7.9 and C.7.6.3.1.5 palette colour fixtures.
//!
//! Every expected value was computed from the standard's descriptor rules and
//! not read out of this repository's output. HLD 27.2 R2.
//!
//! **The palette indexes the STORED value.** PS3.3 C.7.6.3.1.5 defines the
//! descriptor's second value as "the first stored pixel value mapped", and
//! that one word is deviation D-23: HLD section 18's stage table gives stage 4
//! as `Display -> RGB`, and no arm of it takes a `Display` input.

use ocelli_core::Stored;
use ocelli_pixel::{
    ColorTransform, DecodedLayout, DecodedPhotometric, LutDescriptor, PaletteColorLut,
    PhotometricInterpretation, PixelDataEncoding, PixelError, PlanarConfiguration, Rgb,
    SampleLayout,
};

const TOLERANCE: f32 = 0.001;

/// The first stored pixel value mapped. Deliberately not zero, because zero is
/// the value that makes the offset invisible.
const FIRST_MAPPED: i32 = 10;
const ENTRIES: u16 = 256;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok(), "{result:?}");
        let Ok(value) = result else { return };
        value
    }};
}

#[track_caller]
fn assert_rgb_close(actual: Rgb, expected: (f32, f32, f32)) {
    let (r, g, b) = expected;
    assert!(
        (actual.r - r).abs() < TOLERANCE
            && (actual.g - g).abs() < TOLERANCE
            && (actual.b - b).abs() < TOLERANCE,
        "was ({}, {}, {}), wanted ({r}, {g}, {b})",
        actual.r,
        actual.g,
        actual.b
    );
}

/// Entry `i` is `(i, 255 - i, 3i mod 256)`.
///
/// Three different functions of the index on purpose. Equal channels would
/// satisfy a red-green-blue transposition, and a monotone triple would satisfy
/// an off-by-one in every channel at once.
fn entry(index: usize) -> (f32, f32, f32) {
    let i = u32::try_from(index).unwrap_or(0);
    (
        f32::from(u16::try_from(i).unwrap_or(0)),
        f32::from(u16::try_from(255 - i.min(255)).unwrap_or(0)),
        f32::from(u16::try_from((i * 3) % 256).unwrap_or(0)),
    )
}

fn channel(select: usize) -> Vec<f32> {
    (0..usize::from(ENTRIES))
        .map(|index| {
            let (r, g, b) = entry(index);
            match select {
                0 => r,
                1 => g,
                _ => b,
            }
        })
        .collect()
}

fn palette(first_mapped: i32) -> Result<PaletteColorLut, PixelError> {
    PaletteColorLut::new(
        LutDescriptor::new(ENTRIES, first_mapped, 8, channel(0))?,
        LutDescriptor::new(ENTRIES, first_mapped, 8, channel(1))?,
        LutDescriptor::new(ENTRIES, first_mapped, 8, channel(2))?,
    )
}

fn transform(lut: PaletteColorLut) -> Result<ColorTransform, PixelError> {
    ColorTransform::resolve(
        SampleLayout::new(1, PhotometricInterpretation::PaletteColor, None)?,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        Some(lut),
    )
}

/// PS3.3 C.7.6.3.1.5, including both clamps.
///
/// > Stored pixel values less than [the first mapped value] are mapped to the
/// > first entry. Stored pixel values greater than [first mapped plus number
/// > of entries minus one] are mapped to the last entry.
///
/// With first mapped 10 and 256 entries, the mapped range is 10 through 265.
#[test]
fn the_first_mapped_stored_value_offsets_the_whole_table() {
    let transform = require_ok!(transform(require_ok!(palette(FIRST_MAPPED))));

    for (stored, expected_index, why) in [
        (0.0_f32, 0_usize, "far below the first mapped value, clamps"),
        (9.0, 0, "one below the first mapped value, clamps"),
        (10.0, 0, "the first mapped value is entry zero"),
        (11.0, 1, "one above, entry one"),
        (138.0, 128, "mid table"),
        (264.0, 254, "one below the last mapped value"),
        (265.0, 255, "the last mapped value is the last entry"),
        (300.0, 255, "above the table, clamps"),
    ] {
        let mut destination = [Rgb::BLACK];
        assert_eq!(
            transform.map_into(&[Stored(stored)], &mut destination),
            Ok(()),
            "{why}"
        );
        assert_rgb_close(destination[0], entry(expected_index));
    }
}

/// The offset made visible.
///
/// Setting the first mapped input to zero is the single-constant mutation this
/// fixture exists to catch, so it is asserted directly rather than left to a
/// reviewer to perform by hand. Stored 10 reads entry 0 with the offset and
/// entry 10 without it, and the two entries share no channel value.
#[test]
fn ignoring_the_offset_shifts_the_whole_colour_map() {
    let offset = require_ok!(transform(require_ok!(palette(FIRST_MAPPED))));
    let no_offset = require_ok!(transform(require_ok!(palette(0))));

    let mut with = [Rgb::BLACK];
    let mut without = [Rgb::BLACK];
    assert_eq!(offset.map_into(&[Stored(10.0)], &mut with), Ok(()));
    assert_eq!(no_offset.map_into(&[Stored(10.0)], &mut without), Ok(()));

    assert_rgb_close(with[0], (0.0, 255.0, 0.0));
    assert_rgb_close(without[0], (10.0, 245.0, 30.0));
}

/// PS3.3 C.7.6.3.1.5: "When the number of table entries is equal to 2^16 then
/// this value shall be 0."
///
/// Zero means 65,536 and not zero, and not an empty table. The descriptor is
/// built with 65,536 entries and a declared count of 0, and a value near the
/// top of the table is read back, which an implementation reading 0 as an
/// empty or one-entry table cannot do.
#[test]
fn a_declared_entry_count_of_zero_means_sixty_five_thousand_five_hundred_and_thirty_six() {
    let values: Vec<f32> = (0..65_536_u32)
        .map(|index| f32::from(u16::try_from(index).unwrap_or(0)))
        .collect();
    let red = require_ok!(LutDescriptor::new(0, 0, 16, values.clone()));
    let green = require_ok!(LutDescriptor::new(0, 0, 16, values.clone()));
    let blue = require_ok!(LutDescriptor::new(0, 0, 16, values));
    let transform = require_ok!(transform(require_ok!(PaletteColorLut::new(
        red, green, blue
    ))));

    let mut destination = [Rgb::BLACK; 3];
    assert_eq!(
        transform.map_into(
            &[Stored(0.0), Stored(40_000.0), Stored(65_535.0)],
            &mut destination
        ),
        Ok(())
    );
    assert_rgb_close(destination[0], (0.0, 0.0, 0.0));
    assert_rgb_close(destination[1], (40_000.0, 40_000.0, 40_000.0));
    assert_rgb_close(destination[2], (65_535.0, 65_535.0, 65_535.0));

    // The same 65,536 values declared as a count of 1 is a length
    // disagreement and is refused. That refusal is what stops a declared 0
    // from being quietly readable as some other small number.
    let again: Vec<f32> = (0..65_536_u32)
        .map(|index| f32::from(u16::try_from(index).unwrap_or(0)))
        .collect();
    assert_eq!(
        LutDescriptor::new(1, 0, 16, again),
        Err(PixelError::InvalidLutLength)
    );
}

/// PS3.3 C.7.6.3.1.5's third value is 8 or 16, and it is what decides.
///
/// A 16-bit descriptor holding values above 255 is legal and a reader that
/// inferred 8 bits from the data's appearance would refuse or truncate it.
#[test]
fn bits_per_entry_comes_from_the_descriptor_and_not_from_the_data() {
    let wide: Vec<f32> = (0..256_u32)
        .map(|index| f32::from(u16::try_from(index * 257).unwrap_or(0)))
        .collect();

    // Declared 16-bit, so 65,535 is in range.
    let sixteen = require_ok!(LutDescriptor::new(ENTRIES, 0, 16, wide.clone()));
    assert_eq!(sixteen.bits_per_entry(), 16);

    // The identical data declared 8-bit is out of range and refused, rather
    // than silently truncated to the low byte.
    assert_eq!(
        LutDescriptor::new(ENTRIES, 0, 8, wide.clone()),
        Err(PixelError::LutValueOutOfRange)
    );

    let transform = require_ok!(transform(require_ok!(PaletteColorLut::new(
        sixteen,
        require_ok!(LutDescriptor::new(ENTRIES, 0, 16, wide.clone())),
        require_ok!(LutDescriptor::new(ENTRIES, 0, 16, wide)),
    ))));
    let mut destination = [Rgb::BLACK];
    assert_eq!(
        transform.map_into(&[Stored(255.0)], &mut destination),
        Ok(())
    );
    assert_rgb_close(destination[0], (65_535.0, 65_535.0, 65_535.0));
}

/// Three descriptors that map different input ranges would shift hue against
/// luminance, which no shape check reveals.
#[test]
fn palette_channels_that_disagree_on_their_descriptor_are_refused() {
    let matched = require_ok!(LutDescriptor::new(ENTRIES, FIRST_MAPPED, 8, channel(0)));
    let shifted = require_ok!(LutDescriptor::new(ENTRIES, FIRST_MAPPED + 1, 8, channel(1)));
    let half: Vec<f32> = channel(2).into_iter().take(128).collect();
    let shorter = require_ok!(LutDescriptor::new(128, FIRST_MAPPED, 8, half));

    assert_eq!(
        PaletteColorLut::new(matched.clone(), shifted, matched.clone()),
        Err(PixelError::MismatchedPaletteDescriptors)
    );
    assert_eq!(
        PaletteColorLut::new(matched.clone(), matched.clone(), shorter),
        Err(PixelError::MismatchedPaletteDescriptors)
    );
}

/// A palette space with no palette, and a palette with no palette space.
#[test]
fn the_palette_and_the_photometric_interpretation_must_agree() {
    assert_eq!(
        ColorTransform::resolve(
            require_ok!(SampleLayout::new(
                1,
                PhotometricInterpretation::PaletteColor,
                None
            )),
            PixelDataEncoding::Native,
            DecodedPhotometric::Preserved,
            DecodedLayout::Preserved,
            None,
        ),
        Err(PixelError::MissingPaletteLut)
    );
    assert_eq!(
        ColorTransform::resolve(
            require_ok!(SampleLayout::new(
                3,
                PhotometricInterpretation::Rgb,
                Some(PlanarConfiguration::Interleaved)
            )),
            PixelDataEncoding::Native,
            DecodedPhotometric::Preserved,
            DecodedLayout::Preserved,
            Some(require_ok!(palette(FIRST_MAPPED))),
        ),
        Err(PixelError::UnexpectedPaletteLut)
    );
}

/// A palette frame is one stored sample per pixel, so the source and the
/// destination are the same length.
#[test]
fn mapping_refuses_a_length_mismatch_before_writing() {
    let transform = require_ok!(transform(require_ok!(palette(FIRST_MAPPED))));
    let mut destination = [Rgb {
        r: 77.0,
        g: 77.0,
        b: 77.0,
    }];
    assert_eq!(
        transform.map_into(&[Stored(10.0), Stored(11.0)], &mut destination),
        Err(PixelError::SourceLength)
    );
    assert_eq!(destination[0].r.to_bits(), 77.0_f32.to_bits());
}
