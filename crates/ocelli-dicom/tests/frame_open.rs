//! Frame-open fixtures for F-191.
//!
//! Expected display values are hand-computed from DICOM PS3.3 C.11.2.1.3.2.
//! Stored 50, slope 1, intercept 0, LINEAR_EXACT centre 100, width 200,
//! output 0 to 255:
//! `((50 - 100) / 200 + 0.5) * 255 = 63.75`.

use ocelli_codec::{
    DecodePhotometricInterpretation, Decoder, FrameDesc, FrameDescInput, JpegDecoder, PixelDataVr,
    PixelRepresentation,
};
use ocelli_core::Stored;
use ocelli_dicom::{
    BurnedInFlag, ColourSelection, FrameOpenError, LossyFlag, MetadataSet, OpenedFrame,
    WindowInput, fill_padding, open_frame, parse_part10, read_padding,
};
use ocelli_pixel::{
    ColorTransform, DecodedLayout, DecodedPhotometric, LutChain, PhotometricInterpretation,
    PixelDataEncoding, PlanarConfiguration, Rgb, SampleLayout, VoiFunction,
};

fn require<T: std::fmt::Debug>(result: Result<T, impl std::fmt::Debug>) -> T {
    assert!(result.is_ok(), "{result:?}");
    let Ok(value) = result else {
        unreachable!("asserted immediately above")
    };
    value
}

fn bits(value: f32) -> u32 {
    value.to_bits()
}

#[test]
fn stored_50_maps_through_the_opened_linear_exact_window() {
    // PS3.3 C.11.2.1.3.2. 63.75 is exact in f32.
    let parsed = require(parse_part10(&require(mono(true, &[], &[]))));
    let opened = require(open_frame(&parsed, 0));
    assert!(matches!(opened.window(), WindowInput::Transform(_)));
    let WindowInput::Transform(voi) = opened.window().clone() else {
        unreachable!("asserted immediately above")
    };
    let Some((center, width, function)) = voi.window() else {
        unreachable!("a window transform exposes the selected pair")
    };
    assert_eq!(function, VoiFunction::LinearExact);
    assert_eq!(center.to_bits(), 100.0_f32.to_bits());
    assert_eq!(width.to_bits(), 200.0_f32.to_bits());
    let chain = chain_of(&opened);
    let display = chain.apply(Stored(50.0));
    assert_eq!(bits(display.0), bits(63.75));
}

#[test]
fn a_modality_lut_sequence_wins_over_rescale() {
    // PS3.3 C.11.1. The sequence maps stored 50 to 50. Rescale 2 would map it to 100.
    let sequence = require(lut_sequence((0x0028, 0x3000), 1, 50, 8, &[50]));
    let parsed = require(parse_part10(&require(mono(
        true,
        &[(0x0028, 0x1053, "2")],
        &[sequence],
    ))));
    let opened = require(open_frame(&parsed, 0));
    let modality = opened.modality().apply(Stored(50.0));
    assert_eq!(bits(modality.0), bits(50.0));
}

#[test]
fn a_voi_lut_sequence_wins_over_the_window() {
    // PS3.3 C.11.2. The sequence is selected even though centre and width are present.
    let sequence = require(lut_sequence((0x0028, 0x3010), 1, 0, 8, &[10]));
    let parsed = require(parse_part10(&require(mono(true, &[], &[sequence]))));
    let opened = require(open_frame(&parsed, 0));
    assert!(matches!(opened.window(), WindowInput::Transform(_)));
    let WindowInput::Transform(voi) = opened.window() else {
        unreachable!("asserted immediately above")
    };
    assert!(voi.lut().is_some());
    assert!(voi.window().is_none());
}

#[test]
fn a_missing_window_stays_absent() {
    let parsed = require(parse_part10(&require(mono(false, &[], &[]))));
    let opened = require(open_frame(&parsed, 0));
    assert!(matches!(opened.window(), WindowInput::Absent));
}

#[test]
fn padding_is_masked_and_filled_with_the_black_end() {
    // PS3.3 C.7.5.1.1.2. Value 0 and limit 0. Stored 0 is inside the range.
    // LINEAR_EXACT centre 100 width 200 already maps stored 0 to ymin, so that
    // pair cannot show a missing fill. The second object declares padding
    // value 50, which the same window maps to 63.75, and the fill must replace
    // that sample with ymin while the neighbour stays on the chain.
    let opened = open_padding(0, 0);
    let mask = opened.padding_mask(&[0, 50]);
    assert_eq!(mask, vec![true, false]);
    let chain = chain_of(&opened);
    let mut samples = [chain.apply(Stored(0.0)), chain.apply(Stored(50.0))];
    assert!(fill_padding(&mut samples, &mask, opened.photometric(), &chain,).is_ok());
    let [first, second] = samples;
    assert_eq!(bits(first.0), bits(0.0));
    assert_eq!(bits(second.0), bits(63.75));

    let discriminating = open_padding(50, 50);
    let padding_mask = discriminating.padding_mask(&[50, 40]);
    assert_eq!(padding_mask, vec![true, false]);
    let chain = chain_of(&discriminating);
    let mut samples = [chain.apply(Stored(50.0)), chain.apply(Stored(40.0))];
    assert!(
        fill_padding(
            &mut samples,
            &padding_mask,
            discriminating.photometric(),
            &chain,
        )
        .is_ok()
    );
    let [padded, neighbour] = samples;
    assert_eq!(bits(padded.0), bits(0.0));
    assert_eq!(bits(neighbour.0), bits(chain.apply(Stored(40.0)).0));
}

#[test]
fn monochrome1_padding_fills_the_low_end_after_inversion() {
    // PS3.3 C.7.5.1.1.2 and C.11.6. The chain inverts MONOCHROME1 when no
    // presentation shape is declared, so black in the display buffer is ymin.
    // Stored 50 under LINEAR_EXACT centre 100 and width 200 is 191.25 after
    // that inverse. A fill that still writes ymax leaves 255.
    let opened = open_padding_photometric("MONOCHROME1", 50, 50);
    let mask = opened.padding_mask(&[50, 40]);
    assert_eq!(mask, vec![true, false]);
    let chain = chain_of(&opened);
    let before = chain.apply(Stored(50.0));
    let neighbour_before = chain.apply(Stored(40.0));
    assert_ne!(bits(before.0), bits(0.0));
    let mut samples = [before, neighbour_before];
    assert!(fill_padding(&mut samples, &mask, opened.photometric(), &chain,).is_ok());
    let [padded, neighbour] = samples;
    assert_eq!(bits(padded.0), bits(0.0));
    assert_eq!(bits(neighbour.0), bits(neighbour_before.0));

    let mut data = require(image_pixels("MONOCHROME1"));
    data.extend(require(rescale_one()));
    data.extend(require(short_le(
        (0x0028, 0x0120),
        *b"SS",
        &0_i16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0121),
        *b"SS",
        &10_i16.to_le_bytes(),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let result = open_frame(&parsed, 0);
    assert!(result.is_err());
    let Err(error) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error, FrameOpenError::PaddingOrder);
}

#[test]
fn a_reversed_monochrome2_padding_range_is_refused() {
    let mut data = require(image_module());
    data.extend(require(rescale_one()));
    data.extend(require(short_le(
        (0x0028, 0x0120),
        *b"US",
        &10_u16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0121),
        *b"US",
        &0_u16.to_le_bytes(),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let result = open_frame(&parsed, 0);
    assert!(result.is_err());
    let Err(error) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error, FrameOpenError::PaddingOrder);
}

#[test]
fn padding_is_read_alone_where_frame_open_refuses_a_flag() {
    // PS3.3 C.7.5.1.1.2. Pixel Padding Value 0 and Range Limit 10, US, in
    // MONOCHROME2 order (value at or below limit). Burned In Annotation
    // MAYBE is outside the enumerated values YES and NO of the General
    // Image Module, PS3.3 C.7.6.1, so `open_frame` refuses the object, and
    // the padding range alone is still the inclusive 0 to 10.
    let mut padded = require(image_module());
    padded.extend(require(rescale_one()));
    padded.extend(require(short_le(
        (0x0028, 0x0120),
        *b"US",
        &0_u16.to_le_bytes(),
    )));
    padded.extend(require(short_le(
        (0x0028, 0x0121),
        *b"US",
        &10_u16.to_le_bytes(),
    )));
    padded.extend(require(short_le(
        (0x0028, 0x0301),
        *b"CS",
        &text_pad("MAYBE"),
    )));
    let parsed = require(parse_part10(&require(part10(padded))));
    assert_eq!(
        open_frame(&parsed, 0).err(),
        Some(FrameOpenError::BurnedInCode)
    );
    let metadata = require(MetadataSet::from_object(parsed.object()));
    let range = require(read_padding(
        &metadata,
        PhotometricInterpretation::Monochrome2,
        false,
    ));
    let Some(range) = range else {
        unreachable!("the padding value is present")
    };
    assert_eq!((range.value(), range.limit()), (0, Some(10)));
    assert!(range.contains(10));
    assert!(!range.contains(11));
    // The same pair is out of order for MONOCHROME1, where the value is at
    // or above the limit.
    assert_eq!(
        read_padding(&metadata, PhotometricInterpretation::Monochrome1, false).err(),
        Some(FrameOpenError::PaddingOrder)
    );
}

#[test]
fn lossy_and_burned_in_codes_are_closed() {
    let lossy = require(open_codes(Some("01"), Some("YES")));
    assert_eq!(lossy.lossy(), LossyFlag::Lossy);
    assert_eq!(lossy.burned_in(), BurnedInFlag::Yes);

    let lossless = require(open_codes(Some("00"), None));
    assert_eq!(lossless.lossy(), LossyFlag::Lossless);
    assert_eq!(lossless.burned_in(), BurnedInFlag::Absent);

    let parsed = require(parse_part10(&require(open_code_bytes(Some("MAYBE"), None))));
    let result = open_frame(&parsed, 0);
    assert!(result.is_err());
    let Err(error) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error, FrameOpenError::LossyCode);

    let empty = require(parse_part10(&require(open_code_bytes(Some(""), Some("")))));
    let opened = require(open_frame(&empty, 0));
    assert_eq!(opened.lossy(), LossyFlag::Absent);
    assert_eq!(opened.burned_in(), BurnedInFlag::Absent);
}

#[test]
fn a_private_creator_is_the_identifier_not_the_value() {
    // PS3.5 7.8.1. (0009,0010) names the creator for (0009,10xx).
    let mut data = require(image_module());
    data.extend(require(rescale_one()));
    data.extend(require(short_le(
        (0x0009, 0x0010),
        *b"LO",
        &text_pad("SYNTH"),
    )));
    data.extend(require(short_le(
        (0x0009, 0x1001),
        *b"US",
        &1_u16.to_le_bytes(),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    let creators = opened.private_creators();
    assert_eq!(creators.len(), 1);
    let Some(creator) = creators.first() else {
        unreachable!("the length assertion requires one creator")
    };
    assert_eq!(creator.group(), 0x0009);
    assert_eq!(creator.slot(), 0x10);
    assert_eq!(creator.creator(), "SYNTH");
    assert_eq!(opened.creator_for(0x0009, 0x1001), Some("SYNTH"));
    assert_eq!(opened.creator_for(0x0009, 0x0010), None);
}

#[test]
fn an_absent_rescale_is_the_identity() {
    // PS3.3 C.11.1. MR and many secondary objects omit slope and intercept.
    let parsed = require(parse_part10(&require(part10(require(image_module())))));
    let opened = require(open_frame(&parsed, 0));
    assert_eq!(bits(opened.modality().apply(Stored(50.0)).0), bits(50.0));
    assert!(matches!(opened.window(), WindowInput::Absent));
}

#[test]
fn a_slope_without_an_intercept_is_refused() {
    let mut data = require(image_module());
    data.extend(require(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))));
    let parsed = require(parse_part10(&require(part10(data))));
    let result = open_frame(&parsed, 0);
    assert!(result.is_err());
    let Err(error) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(
        error,
        FrameOpenError::Attribute {
            group: 0x0028,
            element: 0x1052,
        }
    );
}

#[test]
fn a_ct_voi_lut_reads_a_negative_first_input() {
    // PS3.3 C.11.2.1.1. Unsigned pixels and intercept -1024. The second
    // descriptor value 0xFC00 is -1024, not 64512.
    let mut data = require(image_module());
    data.extend(require(short_le(
        (0x0028, 0x1052),
        *b"DS",
        &text_pad("-1024"),
    )));
    data.extend(require(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))));
    data.extend(require(lut_sequence((0x0028, 0x3010), 1, 0xFC00, 8, &[40])));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    let WindowInput::Transform(voi) = opened.window().clone() else {
        unreachable!("the VOI LUT sequence is present")
    };
    let Some(lut) = voi.lut() else {
        unreachable!("a sequence was selected")
    };
    assert_eq!(lut.first_mapped_input(), -1024);
}

#[test]
fn a_modality_lut_keeps_the_voi_descriptor_unsigned() {
    // PS3.3 C.11.2.1.1. A Modality LUT makes the VOI descriptor unsigned even
    // when the unused rescale intercept is negative.
    let mut data = require(image_module());
    data.extend(require(short_le(
        (0x0028, 0x1052),
        *b"DS",
        &text_pad("-1024"),
    )));
    data.extend(require(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))));
    data.extend(require(lut_sequence((0x0028, 0x3000), 1, 0, 8, &[50])));
    data.extend(require(lut_sequence((0x0028, 0x3010), 1, 0xFC00, 8, &[40])));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    let WindowInput::Transform(voi) = opened.window().clone() else {
        unreachable!("the VOI LUT sequence is present")
    };
    let Some(lut) = voi.lut() else {
        unreachable!("a sequence was selected")
    };
    assert_eq!(lut.first_mapped_input(), 64_512);
}

#[test]
fn an_ss_modality_descriptor_sign_extends_only_the_second_value() {
    // PS3.3 C.11.1.1.1. Explicit VR SS. The second value is signed. The first
    // and third stay unsigned.
    let mut data = require(image_pixels_signed("MONOCHROME2", true));
    let mut words = Vec::new();
    for value in [1_i16, -2_i16, 8_i16] {
        words.extend_from_slice(&value.to_le_bytes());
    }
    let item_body = concat_bytes(&[
        &require(short_le((0x0028, 0x3002), *b"SS", &words)),
        &require(short_le((0x0028, 0x3006), *b"US", &50_u16.to_le_bytes())),
    ]);
    data.extend(require(long_le(
        (0x0028, 0x3000),
        *b"SQ",
        &require(item(&item_body)),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    let Some(lut) = opened.modality().lut() else {
        unreachable!("the modality sequence is present")
    };
    assert_eq!(lut.first_mapped_input(), -2);
    assert_eq!(lut.bits_per_entry(), 8);
    assert_eq!(lut.declared_entry_count(), 1);
}

#[test]
fn monochrome1_identity_padding_stays_at_the_high_end() {
    // An explicit IDENTITY shape does not invert, so photometric black remains
    // ymax. Stored 50 is 63.75 before the fill.
    let mut data = require(image_pixels("MONOCHROME1"));
    data.extend(require(window_exact()));
    data.extend(require(rescale_one()));
    data.extend(require(short_le(
        (0x2050, 0x0020),
        *b"CS",
        &text_pad("IDENTITY"),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0120),
        *b"SS",
        &50_i16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0121),
        *b"SS",
        &50_i16.to_le_bytes(),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    let chain = chain_of(&opened);
    let before = chain.apply(Stored(50.0));
    assert_ne!(bits(before.0), bits(255.0));
    let mask = opened.padding_mask(&[50]);
    let mut samples = [before];
    assert!(fill_padding(&mut samples, &mask, opened.photometric(), &chain).is_ok());
    let [padded] = samples;
    assert_eq!(bits(padded.0), bits(255.0));
}

#[test]
fn monochrome1_inverse_padding_fills_the_low_end() {
    // Explicit INVERSE matches the chain's MONOCHROME1 reflection. Stored 50
    // is 191.25 before the fill, so ymin is observable.
    let opened = open_shaped_padding("MONOCHROME1", "INVERSE", 50, 50);
    let chain = chain_of(&opened);
    let before = chain.apply(Stored(50.0));
    assert_ne!(bits(before.0), bits(0.0));
    let mask = opened.padding_mask(&[50]);
    let mut samples = [before];
    assert!(fill_padding(&mut samples, &mask, opened.photometric(), &chain).is_ok());
    let [padded] = samples;
    assert_eq!(bits(padded.0), bits(0.0));
}

#[test]
fn monochrome2_inverse_padding_fills_the_high_end() {
    // Explicit INVERSE reflects MONOCHROME2, so black is ymax. Stored 50 is
    // 191.25 before the fill, so ymax is observable.
    let opened = open_shaped_padding("MONOCHROME2", "INVERSE", 50, 50);
    let chain = chain_of(&opened);
    let before = chain.apply(Stored(50.0));
    assert_ne!(bits(before.0), bits(255.0));
    let mask = opened.padding_mask(&[50]);
    let mut samples = [before];
    assert!(fill_padding(&mut samples, &mask, opened.photometric(), &chain).is_ok());
    let [padded] = samples;
    assert_eq!(bits(padded.0), bits(255.0));
}

#[test]
fn a_16_bit_voi_lut_padding_fill_uses_the_chain_range() {
    // PS3.3 C.11.2.1.1. A 16-bit VOI LUT declares 0 to 65535. One entry of
    // 100 maps stored 50 to 100, and explicit INVERSE reflects that to
    // 65435. Black is 65535. A fill of 255 would leave the sample dark.
    // LutDescriptor accepts 8 or 16 bits, so 12 is not a range this chain has.
    let mut data = require(image_pixels("MONOCHROME2"));
    data.extend(require(rescale_one()));
    data.extend(require(lut_sequence((0x0028, 0x3010), 1, 0, 16, &[100])));
    data.extend(require(short_le(
        (0x2050, 0x0020),
        *b"CS",
        &text_pad("INVERSE"),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0120),
        *b"SS",
        &50_i16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0121),
        *b"SS",
        &50_i16.to_le_bytes(),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    let chain = chain_of(&opened);
    let (ymin, ymax) = chain.voi().output_range();
    assert_eq!(bits(ymin), bits(0.0));
    assert_eq!(bits(ymax), bits(65_535.0));
    let before = chain.apply(Stored(50.0));
    assert_eq!(bits(before.0), bits(65_435.0));
    let mask = opened.padding_mask(&[50]);
    let mut samples = [before];
    assert!(fill_padding(&mut samples, &mask, opened.photometric(), &chain).is_ok());
    let [padded] = samples;
    assert_eq!(bits(padded.0), bits(65_535.0));
}

#[test]
fn burned_in_maybe_and_a_bare_padding_limit_are_refused() {
    let burned = require(parse_part10(&require(open_code_bytes(None, Some("MAYBE")))));
    let burned = open_frame(&burned, 0);
    assert!(burned.is_err());
    let Err(error) = burned else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error, FrameOpenError::BurnedInCode);

    let no = require(open_codes(None, Some("NO")));
    assert_eq!(no.burned_in(), BurnedInFlag::No);

    let mut data = require(image_module());
    data.extend(require(rescale_one()));
    data.extend(require(short_le(
        (0x0028, 0x0121),
        *b"US",
        &0_u16.to_le_bytes(),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let result = open_frame(&parsed, 0);
    assert!(result.is_err());
    let Err(error) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error, FrameOpenError::PaddingLimitWithoutValue);
}

#[test]
fn codec_rgb_is_once_and_only_on_colour() {
    let parsed = require(parse_part10(&require(part10(require(image_pixels("RGB"))))));
    let opened = require(open_frame(&parsed, 0));
    let once = require(opened.clone().with_codec_rgb(true));
    assert_eq!(
        once.colour(),
        ColourSelection::Colour {
            codec_already_rgb: true,
        }
    );
    let twice = once.with_codec_rgb(true);
    assert!(twice.is_err());

    let mono = require(parse_part10(&require(part10(require(image_module())))));
    let mono = require(open_frame(&mono, 0));
    let refused = mono.with_codec_rgb(true);
    assert!(refused.is_err());
    let Err(error) = refused else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error, FrameOpenError::SecondColourConversion);
}

#[test]
fn xyb_opens_as_the_named_photometric() {
    // PS3.3 C.7.6.3.1.2. `XYB` is a real photometric interpretation.
    // Samples per Pixel is 3, which is what that term requires.
    let parsed = require(parse_part10(&require(part10(require(colour_pixels(
        "XYB", 3, 8, 8, 8,
    ))))));
    let opened = require(open_frame(&parsed, 0));
    assert_eq!(opened.photometric(), PhotometricInterpretation::Xyb);
    assert_eq!(
        opened.colour(),
        ColourSelection::Colour {
            codec_already_rgb: false,
        }
    );
}

#[test]
fn ybr_full_422_reported_as_rgb_is_converted_once() {
    // PS3.3 C.7.6.3.1.2. A decoder that already produced RGB is not converted
    // again. `.111` is not decoded here. `jxl-jbr` 0.2.1 `src/reconstruct.rs:457`
    // makes that syntax unavailable, so the bytes are the `.50` baseline path.
    let desc = require(FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_FULL_422".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    }));
    let jpeg = include_bytes!("../../ocelli-codec/tests/fixtures/jpeg_baseline_rgb.jpg");
    let mut from_jpeg = vec![0_u8; desc.output_len()];
    require(JpegDecoder::baseline().decode(jpeg, &desc, &mut from_jpeg));

    let parsed = require(parse_part10(&require(part10(require(colour_pixels(
        "YBR_FULL_422",
        3,
        64,
        96,
        8,
    ))))));
    let opened = require(open_frame(&parsed, 0));
    assert_eq!(opened.photometric(), PhotometricInterpretation::YbrFull422);
    let reported = JpegDecoder::baseline().decode_photometric_interpretation(&desc);
    assert_eq!(reported, DecodePhotometricInterpretation::Rgb);
    let opened = require(opened.with_codec_rgb(true));
    assert_eq!(
        opened.colour(),
        ColourSelection::Colour {
            codec_already_rgb: true,
        }
    );
    let transform = require(ColorTransform::resolve(
        require(SampleLayout::new(
            3,
            PhotometricInterpretation::YbrFull422,
            Some(PlanarConfiguration::Interleaved),
        )),
        PixelDataEncoding::Encapsulated,
        DecodedPhotometric::Rgb,
        DecodedLayout::Interleaved,
        None,
    ));
    let stored: Vec<Stored> = from_jpeg
        .iter()
        .copied()
        .map(|byte| Stored(f32::from(byte)))
        .collect();
    let pixels = usize::from(desc.rows()) * usize::from(desc.columns());
    let mut rgb = vec![Rgb::BLACK; pixels];
    require(transform.map_into(&stored, &mut rgb, usize::from(desc.columns())));
    assert_eq!(from_jpeg.len(), rgb.len() * 3);
    for (pixel, sample) in rgb.iter().zip(from_jpeg.chunks_exact(3)) {
        let &[red, green, blue] = sample else {
            continue;
        };
        assert_eq!(pixel.r.to_bits(), f32::from(red).to_bits());
        assert_eq!(pixel.g.to_bits(), f32::from(green).to_bits());
        assert_eq!(pixel.b.to_bits(), f32::from(blue).to_bits());
    }
}

#[test]
fn an_unreadable_creator_and_a_reserved_group_do_not_refuse_the_frame() {
    let mut data = require(image_module());
    data.extend(require(long_le(
        (0x0009, 0x0010),
        *b"UN",
        &text_pad("SYNTH"),
    )));
    data.extend(require(short_le(
        (0x0001, 0x0010),
        *b"LO",
        &text_pad("SYNTH"),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    assert!(opened.private_creators().is_empty());
}

#[test]
fn palette_colour_is_not_a_monochrome_chain() {
    let mut data = Vec::new();
    data.extend(require(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0002),
        *b"US",
        &1_u16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0004),
        *b"CS",
        &text_pad("PALETTE COLOR"),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0010),
        *b"US",
        &1_u16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0011),
        *b"US",
        &1_u16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0100),
        *b"US",
        &8_u16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0101),
        *b"US",
        &8_u16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0102),
        *b"US",
        &7_u16.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0103),
        *b"US",
        &0_u16.to_le_bytes(),
    )));
    data.extend(require(rescale_one()));
    let parsed = require(parse_part10(&require(part10(data))));
    let opened = require(open_frame(&parsed, 0));
    assert_eq!(opened.colour(), ColourSelection::Palette);
    assert!(matches!(opened.window(), WindowInput::Absent));
    let refused = opened.clone().with_codec_rgb(true);
    assert!(refused.is_err());
    let Err(error) = refused else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error, FrameOpenError::SecondColourConversion);
}

#[test]
fn per_frame_windows_win_over_a_missing_top_level_window() {
    // PS3.3 C.7.6.16. Shared rescale is 1 and 0. Frame 0 is LINEAR_EXACT
    // 100/200, so stored 50 is 63.75. Frame 1 is LINEAR_EXACT 50/1, so the
    // same stored value is 127.5. A top-level-only read has no window.
    let parsed = require(parse_part10(&require(enhanced_windows())));
    let frame0 = require(open_frame(&parsed, 0));
    let display0 = chain_of(&frame0).apply(Stored(50.0));
    assert_eq!(bits(display0.0), bits(63.75));
    let frame1 = require(open_frame(&parsed, 1));
    let display1 = chain_of(&frame1).apply(Stored(50.0));
    assert_eq!(bits(display1.0), bits(127.5));
}

fn enhanced_windows() -> Result<Vec<u8>, String> {
    let rescale = item(&concat_bytes(&[
        &short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?,
        &short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?,
    ]))?;
    let shared = item(&long_le((0x0028, 0x9145), *b"SQ", &rescale)?)?;
    let frame_window = |center: &str, width: &str| -> Result<Vec<u8>, String> {
        let body = concat_bytes(&[
            &short_le((0x0028, 0x1050), *b"DS", &text_pad(center))?,
            &short_le((0x0028, 0x1051), *b"DS", &text_pad(width))?,
            &short_le((0x0028, 0x1056), *b"CS", &text_pad("LINEAR_EXACT"))?,
        ]);
        item(&long_le((0x0028, 0x9132), *b"SQ", &item(&body)?)?)
    };
    let per_frame = concat_bytes(&[&frame_window("100", "200")?, &frame_window("50", "1")?]);
    let mut data = image_module()?;
    data.extend(short_le((0x0028, 0x0008), *b"IS", &text_pad("2"))?);
    data.extend(long_le((0x5200, 0x9229), *b"SQ", &shared)?);
    data.extend(long_le((0x5200, 0x9230), *b"SQ", &per_frame)?);
    part10(data)
}

fn chain_of(opened: &OpenedFrame) -> LutChain {
    assert!(matches!(opened.window(), WindowInput::Transform(_)));
    let WindowInput::Transform(voi) = opened.window().clone() else {
        unreachable!("asserted immediately above")
    };
    require(LutChain::new(
        opened.modality().clone(),
        voi,
        opened.photometric(),
        opened.presentation(),
    ))
}

fn open_padding(value: i16, limit: i16) -> OpenedFrame {
    open_padding_photometric("MONOCHROME2", value, limit)
}

fn open_padding_photometric(photometric: &str, value: i16, limit: i16) -> OpenedFrame {
    open_shaped_padding(photometric, "", value, limit)
}

fn open_shaped_padding(photometric: &str, shape: &str, value: i16, limit: i16) -> OpenedFrame {
    let mut data = require(image_pixels(photometric));
    data.extend(require(window_exact()));
    data.extend(require(rescale_one()));
    if !shape.is_empty() {
        data.extend(require(short_le(
            (0x2050, 0x0020),
            *b"CS",
            &text_pad(shape),
        )));
    }
    data.extend(require(short_le(
        (0x0028, 0x0120),
        *b"SS",
        &value.to_le_bytes(),
    )));
    data.extend(require(short_le(
        (0x0028, 0x0121),
        *b"SS",
        &limit.to_le_bytes(),
    )));
    let parsed = require(parse_part10(&require(part10(data))));
    require(open_frame(&parsed, 0))
}

fn open_codes(
    lossy: Option<&str>,
    burned: Option<&str>,
) -> Result<ocelli_dicom::OpenedFrame, String> {
    let parsed =
        parse_part10(&open_code_bytes(lossy, burned)?).map_err(|error| format!("{error:?}"))?;
    open_frame(&parsed, 0).map_err(|error| format!("{error:?}"))
}

fn open_code_bytes(lossy: Option<&str>, burned: Option<&str>) -> Result<Vec<u8>, String> {
    let mut data = image_module()?;
    data.extend(rescale_one()?);
    if let Some(code) = lossy {
        data.extend(short_le((0x0028, 0x2110), *b"CS", &text_pad(code))?);
    }
    if let Some(code) = burned {
        data.extend(short_le((0x0028, 0x0301), *b"CS", &text_pad(code))?);
    }
    part10(data)
}

fn mono(
    window: bool,
    extra_text: &[(u16, u16, &str)],
    blobs: &[Vec<u8>],
) -> Result<Vec<u8>, String> {
    let mut data = image_module()?;
    if window {
        data.extend(window_exact()?);
    }
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    let slope = extra_text
        .iter()
        .find(|item| item.0 == 0x0028 && item.1 == 0x1053)
        .map(|item| item.2)
        .unwrap_or("1");
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad(slope))?);
    for blob in blobs {
        data.extend(blob);
    }
    part10(data)
}

fn lut_sequence(
    tag: (u16, u16),
    count: u16,
    first: u16,
    bits: u16,
    values: &[u16],
) -> Result<Vec<u8>, String> {
    let mut words = Vec::new();
    words.extend_from_slice(&count.to_le_bytes());
    words.extend_from_slice(&first.to_le_bytes());
    words.extend_from_slice(&bits.to_le_bytes());
    let mut data = Vec::new();
    for value in values {
        data.extend_from_slice(&value.to_le_bytes());
    }
    let item_body = concat_bytes(&[
        &short_le((0x0028, 0x3002), *b"US", &words)?,
        &short_le((0x0028, 0x3006), *b"US", &data)?,
    ]);
    long_le(tag, *b"SQ", &item(&item_body)?)
}

fn image_module() -> Result<Vec<u8>, String> {
    image_pixels("MONOCHROME2")
}

fn image_pixels(photometric: &str) -> Result<Vec<u8>, String> {
    image_pixels_signed(photometric, false)
}

fn colour_pixels(
    photometric: &str,
    samples: u16,
    rows: u16,
    columns: u16,
    bits: u16,
) -> Result<Vec<u8>, String> {
    let high = bits.checked_sub(1).ok_or("bits")?;
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &samples.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0004), *b"CS", &text_pad(photometric))?);
    data.extend(short_le((0x0028, 0x0006), *b"US", &0_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &rows.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &columns.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &bits.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &bits.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &high.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes())?);
    Ok(data)
}

fn image_pixels_signed(photometric: &str, signed: bool) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le(
        (0x0008, 0x0016),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0004), *b"CS", &text_pad(photometric))?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes())?);
    let representation: u16 = if signed { 1 } else { 0 };
    data.extend(short_le(
        (0x0028, 0x0103),
        *b"US",
        &representation.to_le_bytes(),
    )?);
    Ok(data)
}

fn window_exact() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le((0x0028, 0x1050), *b"DS", &text_pad("100"))?);
    data.extend(short_le((0x0028, 0x1051), *b"DS", &text_pad("200"))?);
    data.extend(short_le(
        (0x0028, 0x1056),
        *b"CS",
        &text_pad("LINEAR_EXACT"),
    )?);
    Ok(data)
}

fn rescale_one() -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend(short_le((0x0028, 0x1052), *b"DS", &text_pad("0"))?);
    data.extend(short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?);
    Ok(data)
}

fn part10(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta()?);
    bytes.extend(data);
    Ok(bytes)
}

fn file_meta() -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(short_le(
        (0x0002, 0x0002),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    body.extend(short_le((0x0002, 0x0003), *b"UI", &ui(b"2.25.1"))?);
    body.extend(short_le(
        (0x0002, 0x0010),
        *b"UI",
        &ui(b"1.2.840.10008.1.2.1"),
    )?);
    body.extend(short_le((0x0002, 0x0012), *b"UI", &ui(b"2.25.2"))?);
    let length = u32::try_from(body.len()).map_err(|_| "meta length".to_owned())?;
    let mut meta = short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes())?;
    meta.extend(body);
    Ok(meta)
}

fn ui(text: &[u8]) -> Vec<u8> {
    let mut value = text.to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    value
}

fn text_pad(text: &str) -> Vec<u8> {
    let mut value = text.as_bytes().to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(b' ');
    }
    value
}

fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u16::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u32::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn item(content: &[u8]) -> Result<Vec<u8>, String> {
    let length = u32::try_from(content.len()).map_err(|_| "item length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(content);
    Ok(bytes)
}

fn concat_bytes(parts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for part in parts {
        bytes.extend_from_slice(part);
    }
    bytes
}
