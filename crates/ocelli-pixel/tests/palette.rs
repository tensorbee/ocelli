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
            transform.map_into(&[Stored(stored)], &mut destination, 1),
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
    assert_eq!(offset.map_into(&[Stored(10.0)], &mut with, 1), Ok(()));
    assert_eq!(no_offset.map_into(&[Stored(10.0)], &mut without, 1), Ok(()));

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
            &mut destination,
            3,
        ),
        Ok(())
    );
    // The entries are 16-bit and some exceed 255, so ruling R1 does not read
    // the palette as 8-bit and the display scale shifts each one right by 8
    // on the way out of the table: 40000 >> 8 = 156, 65535 >> 8 = 255.
    // Entry 40000 is read at all only if the table really has 65,536 rows.
    assert_rgb_close(destination[0], (0.0, 0.0, 0.0));
    assert_rgb_close(destination[1], (156.0, 156.0, 156.0));
    assert_rgb_close(destination[2], (255.0, 255.0, 255.0));

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
        transform.map_into(&[Stored(255.0)], &mut destination, 1),
        Ok(())
    );
    // Stored 255 reads entry 255, which is `255 * 257 = 65535`. Every
    // channel holds entries above 255, so ruling R1 does not read the
    // palette as 8-bit and the 16-bit descriptor shifts it right by 8 to 255.
    assert_rgb_close(destination[0], (255.0, 255.0, 255.0));
    let mut low = [Rgb::BLACK];
    assert_eq!(transform.map_into(&[Stored(1.0)], &mut low, 1), Ok(()));
    // Entry 1 is 257, and 257 >> 8 = 1.
    assert_rgb_close(low[0], (1.0, 1.0, 1.0));
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
        transform.map_into(&[Stored(10.0), Stored(11.0)], &mut destination, 1,),
        Err(PixelError::SourceLength)
    );
    assert_eq!(destination[0].r.to_bits(), 77.0_f32.to_bits());
}

// ---------------------------------------------------------------------------
// Display scaling of palette entries, F-239 item 11 and AC22
// ---------------------------------------------------------------------------

/// PS3.3 C.7.6.3.1.5 makes the descriptor's third value the bit depth of
/// every entry. Every channel here is declared 16-bit and holds an entry above
/// 255, so the palette is not read as 8-bit and each channel is shifted right
/// by its own declared depth minus 8.
///
/// ```text
/// descriptors 5\0\16, so every channel shifts by 8
/// red   0 255 256 32768 65535  ->  0 0 1 128 255
/// green 65535 32768 256 255 0  ->  255 128 1 0 0
/// blue  65535 32768 256 255 0  ->  255 128 1 0 0
/// stored 0 1 2 3 4 -> (0 255 255) (0 128 128) (1 1 1) (128 0 0) (255 0 0)
/// ```
///
/// Rounding instead of truncating gives 128 for 32768 too, but 1 for 255.
/// cornerstone3D 5.8.2 shifts all three by 8 only when a RED entry exceeds
/// 255 (`convertPALETTECOLOR.js:24`), which agrees on this table and is
/// rejected by the next fixture.
#[test]
fn palette_entries_are_shifted_by_their_descriptor() {
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
    // Every channel holds an entry above 255, so none falls back.
    assert!(!palette.entries_read_as_8_bit());
    let transform = require_ok!(transform(palette));
    let mut destination = [Rgb::BLACK; 5];
    assert_eq!(
        transform.map_into(
            &[
                Stored(0.0),
                Stored(1.0),
                Stored(2.0),
                Stored(3.0),
                Stored(4.0)
            ],
            &mut destination,
            5,
        ),
        Ok(())
    );
    let expected = [
        (0.0, 255.0, 255.0),
        (0.0, 128.0, 128.0),
        (1.0, 1.0, 1.0),
        (128.0, 0.0, 0.0),
        (255.0, 0.0, 0.0),
    ];
    for (actual, wanted) in destination.into_iter().zip(expected) {
        assert_rgb_close(actual, wanted);
    }
}

/// Ruling R1, decided once for the whole palette. A palette is read as 8-bit
/// only when at least one channel is declared above 8 bits and every entry of
/// every such channel fits in 8 bits, as cornerstone3D 5.8.2 reads it, and the
/// palette says so. Otherwise each channel is shifted by its own declared
/// depth minus 8.
///
/// ```text
/// three 16-bit channels, each 0 100 255 -> read as 8-bit -> 0 100 255, flag set
/// strictly by C.7.6.3.1.5 it would be 0 >> 8, 100 >> 8, 255 >> 8 = 0 0 0
/// ```
///
/// One entry of 256 in any channel declared above 8 bits means the palette is
/// not read as 8-bit, so every channel is shifted and the flag is clear. A
/// conforming palette is therefore unaffected.
#[test]
fn a_palette_is_read_as_8_bit_only_when_every_wide_channel_fits_in_8_bits() {
    let small = vec![0.0, 100.0, 255.0];
    let palette = require_ok!(PaletteColorLut::new(
        require_ok!(LutDescriptor::new(3, 0, 16, small.clone())),
        require_ok!(LutDescriptor::new(3, 0, 16, small.clone())),
        require_ok!(LutDescriptor::new(3, 0, 16, small)),
    ));
    assert!(palette.entries_read_as_8_bit());
    let entries: Vec<f32> = palette.channel_entries().collect();
    assert_eq!(
        entries,
        vec![0.0, 100.0, 255.0, 0.0, 100.0, 255.0, 0.0, 100.0, 255.0]
    );

    // The lead's ruling on R1: the fallback is decided once for the whole
    // palette. A conforming 16-bit palette may hold a channel whose entries
    // are all small, a red that is near zero throughout. Read per channel,
    // that channel would be taken as 8-bit, about 256 times too bright, while
    // the others are shifted. All three are declared 16-bit here, and blue
    // holds 256, so the palette is 16-bit and EVERY channel is shifted:
    //
    //   red   0 100 255 -> >> 8 -> 0 0 0
    //   green 0 100 255 -> >> 8 -> 0 0 0
    //   blue  0 100 256 -> >> 8 -> 0 0 1
    let mixed = require_ok!(PaletteColorLut::new(
        require_ok!(LutDescriptor::new(3, 0, 16, vec![0.0, 100.0, 255.0])),
        require_ok!(LutDescriptor::new(3, 0, 16, vec![0.0, 100.0, 255.0])),
        require_ok!(LutDescriptor::new(3, 0, 16, vec![0.0, 100.0, 256.0])),
    ));
    assert!(!mixed.entries_read_as_8_bit());
    let entries: Vec<f32> = mixed.channel_entries().collect();
    assert_eq!(entries, vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);

    // A channel declared 8-bit takes no part in the decision. Red and green
    // are 16-bit and all small, blue is 8-bit, so the palette falls back.
    let with_narrow = require_ok!(PaletteColorLut::new(
        require_ok!(LutDescriptor::new(3, 0, 16, vec![0.0, 100.0, 255.0])),
        require_ok!(LutDescriptor::new(3, 0, 16, vec![0.0, 100.0, 255.0])),
        require_ok!(LutDescriptor::new(3, 0, 8, vec![0.0, 100.0, 255.0])),
    ));
    assert!(with_narrow.entries_read_as_8_bit());
    let entries: Vec<f32> = with_narrow.channel_entries().collect();
    assert_eq!(
        entries,
        vec![0.0, 100.0, 255.0, 0.0, 100.0, 255.0, 0.0, 100.0, 255.0]
    );

    let conforming = require_ok!(PaletteColorLut::new(
        require_ok!(LutDescriptor::new(3, 0, 16, vec![0.0, 100.0, 256.0])),
        require_ok!(LutDescriptor::new(3, 0, 16, vec![0.0, 100.0, 256.0])),
        require_ok!(LutDescriptor::new(3, 0, 8, vec![0.0, 100.0, 255.0])),
    ));
    assert!(!conforming.entries_read_as_8_bit());
}

/// Green and blue are declared above 8 bits and each holds an entry above 255,
/// so the palette is not read as 8-bit and each channel is shifted by its own
/// declared depth minus 8. Red is declared 8-bit, which is a shift of 0.
///
/// ```text
/// red 8-bit 200 -> 200, green 16-bit 51200 -> 200, blue 12-bit 3200 -> 200
/// ```
///
/// Twelve is outside C.7.6.3.1.5's "8 or 16" but inside the operator's
/// `9 ..= 16`, so it is accepted with entries carried in a 16-bit container
/// and, in this palette, shifted by `12 - 8 = 4`. A 12-bit entry above 4095 is not a 12-bit value and is
/// refused, as is a depth above 16 or above the entry's own container.
#[test]
fn each_palette_channel_is_scaled_by_its_own_depth() {
    let palette = require_ok!(PaletteColorLut::with_entry_bits(
        require_ok!(LutDescriptor::new(1, 0, 8, vec![200.0])),
        require_ok!(LutDescriptor::new(1, 0, 16, vec![51_200.0])),
        require_ok!(LutDescriptor::new(1, 0, 16, vec![3_200.0])),
        [8, 16, 12],
    ));
    let transform = require_ok!(transform(palette));
    let mut destination = [Rgb::BLACK];
    assert_eq!(
        transform.map_into(&[Stored(0.0)], &mut destination, 1),
        Ok(())
    );
    assert_rgb_close(destination[0], (200.0, 200.0, 200.0));

    let entry = |bits: u8, value: f32| LutDescriptor::new(1, 0, bits, vec![value]);
    assert_eq!(
        PaletteColorLut::with_entry_bits(
            require_ok!(entry(16, 4_096.0)),
            require_ok!(entry(16, 0.0)),
            require_ok!(entry(16, 0.0)),
            [12, 12, 12],
        ),
        Err(PixelError::LutValueOutOfRange)
    );
    assert_eq!(
        PaletteColorLut::with_entry_bits(
            require_ok!(entry(16, 0.0)),
            require_ok!(entry(16, 0.0)),
            require_ok!(entry(16, 0.0)),
            [17, 16, 16],
        ),
        Err(PixelError::UnsupportedColourDepth)
    );
    assert_eq!(
        PaletteColorLut::with_entry_bits(
            require_ok!(entry(8, 0.0)),
            require_ok!(entry(8, 0.0)),
            require_ok!(entry(8, 0.0)),
            [12, 8, 8],
        ),
        Err(PixelError::InvalidLutBits)
    );
}

/// Review pass 2, S1. An all-8-bit palette took no fallback, so its flag is
/// clear. The page states the flag, and a set flag here would put the
/// departure notice on every conforming 8-bit palette.
#[test]
fn an_all_8_bit_palette_does_not_report_the_fallback() {
    let palette = require_ok!(palette(FIRST_MAPPED));
    assert!(!palette.entries_read_as_8_bit());
}

/// Review pass 3, S1. The rule's "declared above 8 bits" is every depth from 9
/// to 16, not only 16. Three 12-bit channels whose entries all fit in 8 bits
/// are read as 8-bit and flagged.
///
/// ```text
/// 12-bit, entries 0 100 255 -> read as 8-bit -> 0 100 255, flag set
/// shifted by 12 - 8 = 4 instead, 255 >> 4 = 15, nearly black
/// ```
#[test]
fn a_twelve_bit_palette_of_8_bit_entries_is_read_as_8_bit() {
    let small = vec![0.0, 100.0, 255.0];
    let palette = require_ok!(PaletteColorLut::with_entry_bits(
        require_ok!(LutDescriptor::new(3, 0, 16, small.clone())),
        require_ok!(LutDescriptor::new(3, 0, 16, small.clone())),
        require_ok!(LutDescriptor::new(3, 0, 16, small)),
        [12, 12, 12],
    ));
    assert!(palette.entries_read_as_8_bit());
    let entries: Vec<f32> = palette.channel_entries().collect();
    assert_eq!(
        entries,
        vec![0.0, 100.0, 255.0, 0.0, 100.0, 255.0, 0.0, 100.0, 255.0]
    );
}
