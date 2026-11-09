use std::{
    collections::HashSet,
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescError, FrameDescInput, KNOWN_TRANSFER_SYNTAXES, PixelDataVr,
    PixelRepresentation, Registry, RegistryError, register_htj2k_decoders, register_jpeg_decoders,
    register_jpeg2000_decoders, register_jpegls_decoders, register_jpegxl_decoders,
    register_native_and_rle_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

const TS_A: &str = "1.2.840.10008.1.2.4.50";
const TS_B: &str = "1.2.840.10008.1.2.4.51";
const IMPLICIT_VR_LE: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";
const TS_UNKNOWN: &str = "1.2.840.10008.1.2.4.500";
const ONE_TS_A: &[&str] = &[TS_A];
const ONE_TS_B: &[&str] = &[TS_B];
const TWO_SYNTAXES: &[&str] = &[TS_A, TS_B];
const REPEATED_SYNTAX: &[&str] = &[TS_A, TS_A];
const EMPTY_SYNTAX: &[&str] = &[""];
const KNOWN_WITH_EMPTY: &[&str] = &[TS_A, ""];
const UNKNOWN_SYNTAX: &[&str] = &[TS_UNKNOWN];
const NO_SYNTAXES: &[&str] = &[];

struct FillDecoder {
    syntaxes: &'static [&'static str],
    fill: u8,
    calls: Arc<AtomicUsize>,
    failure: Option<CodecError>,
}

impl Decoder for FillDecoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        self.syntaxes
    }

    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert_eq!(src, [7, 8, 9]);
        assert_eq!(desc.rows(), 1);
        assert_eq!(desc.columns(), 3);
        if let Some(error) = self.failure {
            return Err(error);
        }
        out.fill(self.fill);
        Ok(())
    }
}

fn frame_input() -> FrameDescInput {
    FrameDescInput {
        rows: 1,
        columns: 3,
        samples_per_pixel: 1,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    }
}

fn frame_desc() -> Result<FrameDesc, FrameDescError> {
    FrameDesc::new(frame_input())
}

fn decoder(
    syntaxes: &'static [&'static str],
    fill: u8,
    calls: Arc<AtomicUsize>,
) -> Arc<dyn Decoder> {
    Arc::new(FillDecoder {
        syntaxes,
        fill,
        calls,
        failure: None,
    })
}

#[test]
fn built_in_catalogue_starts_known_but_unavailable() {
    let registry = Registry::new();
    let distinct: HashSet<_> = KNOWN_TRANSFER_SYNTAXES.iter().copied().collect();

    assert_eq!(KNOWN_TRANSFER_SYNTAXES.len(), KNOWN_SYNTAX_COUNT);
    assert_eq!(distinct.len(), KNOWN_TRANSFER_SYNTAXES.len());
    assert!(KNOWN_TRANSFER_SYNTAXES.iter().all(|uid| !uid.is_empty()));
    assert!(
        KNOWN_TRANSFER_SYNTAXES
            .iter()
            .all(|uid| registry.capability(uid) == Capability::KnownUnavailable)
    );
}

#[test]
fn decoder_output_description_preserves_the_frame_by_default() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::new();
    registry.register(decoder(ONE_TS_A, 0, calls))?;
    assert_eq!(
        registry.decode_photometric_interpretation(TS_A, &frame_desc()?)?,
        DecodePhotometricInterpretation::Preserved
    );
    assert_eq!(
        registry.decode_sample_layout(TS_A, &frame_desc()?)?,
        DecodeSampleLayout::Preserved
    );
    Ok(())
}

#[test]
fn valid_frame_description_preserves_fields_and_checked_output_length() -> TestResult {
    let desc = FrameDesc::new(FrameDescInput {
        rows: 2,
        bits_allocated: 16,
        bits_stored: 12,
        high_bit: 11,
        pixel_representation: PixelRepresentation::Signed,
        photometric_interpretation: "MONOCHROME1".to_owned(),
        ..frame_input()
    })?;

    assert_eq!(desc.rows(), 2);
    assert_eq!(desc.columns(), 3);
    assert_eq!(desc.samples_per_pixel(), 1);
    assert_eq!(desc.bits_allocated(), 16);
    assert_eq!(desc.bits_stored(), 12);
    assert_eq!(desc.high_bit(), 11);
    assert_eq!(desc.pixel_representation(), PixelRepresentation::Signed);
    assert_eq!(desc.photometric_interpretation(), "MONOCHROME1");
    assert_eq!(desc.pixel_data_vr(), PixelDataVr::Ob);
    assert_eq!(desc.output_len(), 12);
    Ok(())
}

#[test]
fn legacy_nonconforming_high_bit_15_descriptor_is_refused_exactly() {
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_allocated: 16,
            bits_stored: 12,
            high_bit: 15,
            pixel_representation: PixelRepresentation::Signed,
            ..frame_input()
        }),
        Err(FrameDescError::HighBitMismatch {
            high_bit: 15,
            expected: 11,
        })
    );
}

#[test]
fn known_catalogue_refuses_empty_and_repeated_entries() {
    assert_eq!(
        Registry::with_known(KNOWN_WITH_EMPTY).err(),
        Some(RegistryError::EmptyTransferSyntax)
    );
    assert_eq!(
        Registry::with_known(REPEATED_SYNTAX).err(),
        Some(RegistryError::RepeatedTransferSyntax { uid: TS_A })
    );
}

#[test]
fn capability_distinguishes_available_unavailable_and_unknown_exactly() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::with_known(&[TS_A, TS_B])?;
    registry.register(decoder(ONE_TS_A, 1, calls))?;

    assert_eq!(registry.capability(TS_A), Capability::Available);
    assert_eq!(registry.capability(TS_B), Capability::KnownUnavailable);
    assert_eq!(registry.capability(TS_UNKNOWN), Capability::Unknown);
    assert_eq!(
        registry.decoder(TS_B).map(|_| ()),
        Err(CodecError::KnownUnavailable)
    );
    assert_eq!(
        registry.decoder(TS_UNKNOWN).map(|_| ()),
        Err(CodecError::UnknownTransferSyntax)
    );
    Ok(())
}

#[test]
fn common_prefix_is_not_a_transfer_syntax_fallback() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::with_known(&[TS_A])?;
    registry.register(decoder(ONE_TS_A, 1, calls))?;

    assert_eq!(registry.capability(TS_UNKNOWN), Capability::Unknown);
    assert_eq!(
        registry.decoder(TS_UNKNOWN).map(|_| ()),
        Err(CodecError::UnknownTransferSyntax)
    );
    Ok(())
}

#[test]
fn registration_refuses_invalid_declarations() -> TestResult {
    let cases = [
        (NO_SYNTAXES, RegistryError::NoTransferSyntaxes),
        (EMPTY_SYNTAX, RegistryError::EmptyTransferSyntax),
        (
            REPEATED_SYNTAX,
            RegistryError::RepeatedTransferSyntax { uid: TS_A },
        ),
        (
            UNKNOWN_SYNTAX,
            RegistryError::UnknownTransferSyntax { uid: TS_UNKNOWN },
        ),
    ];

    for (syntaxes, expected) in cases {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = Registry::with_known(&[TS_A, TS_B])?;
        assert_eq!(
            registry.register(decoder(syntaxes, 1, calls)),
            Err(expected)
        );
        assert_eq!(registry.capability(TS_A), Capability::KnownUnavailable);
        assert_eq!(registry.capability(TS_B), Capability::KnownUnavailable);
    }
    Ok(())
}

#[test]
fn multi_syntax_registration_is_atomic_on_collision() -> TestResult {
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::with_known(&[TS_A, TS_B])?;
    registry.register(decoder(ONE_TS_B, 11, Arc::clone(&first_calls)))?;

    assert_eq!(
        registry.register(decoder(TWO_SYNTAXES, 22, second_calls)),
        Err(RegistryError::AlreadyRegistered { uid: TS_B })
    );
    assert_eq!(registry.capability(TS_A), Capability::KnownUnavailable);

    let mut out = [0; 3];
    registry.decode(TS_B, &[7, 8, 9], &frame_desc()?, &mut out)?;
    assert_eq!(out, [11; 3]);
    assert_eq!(first_calls.load(Ordering::Relaxed), 1);
    Ok(())
}

#[test]
fn registration_order_cannot_replace_a_successful_mapping() -> TestResult {
    for (first_fill, second_fill) in [(17, 29), (29, 17)] {
        let mut registry = Registry::with_known(&[TS_A])?;
        registry.register(decoder(ONE_TS_A, first_fill, Arc::new(AtomicUsize::new(0))))?;
        assert_eq!(
            registry.register(decoder(
                ONE_TS_A,
                second_fill,
                Arc::new(AtomicUsize::new(0)),
            )),
            Err(RegistryError::AlreadyRegistered { uid: TS_A })
        );

        let mut out = [0; 3];
        registry.decode(TS_A, &[7, 8, 9], &frame_desc()?, &mut out)?;
        assert_eq!(out, [first_fill; 3]);
    }
    Ok(())
}

#[test]
fn dispatch_passes_borrowed_inputs_and_propagates_decoder_errors() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let expected = CodecError::DecoderFailure;
    let mut registry = Registry::with_known(&[TS_A])?;
    registry.register(Arc::new(FillDecoder {
        syntaxes: ONE_TS_A,
        fill: 0,
        calls: Arc::clone(&calls),
        failure: Some(expected),
    }))?;

    let mut out = [0; 3];
    assert_eq!(
        registry.decode(TS_A, &[7, 8, 9], &frame_desc()?, &mut out),
        Err(expected)
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    Ok(())
}

#[test]
fn dispatch_rejects_the_wrong_output_length_before_calling_a_decoder() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::with_known(&[TS_A])?;
    registry.register(decoder(ONE_TS_A, 1, Arc::clone(&calls)))?;

    let mut out = [0; 2];
    assert_eq!(
        registry.decode(TS_A, &[7, 8, 9], &frame_desc()?, &mut out),
        Err(CodecError::OutputLength {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    Ok(())
}

#[test]
fn frame_description_validates_dicom_pixel_container_bounds() {
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            rows: 0,
            ..frame_input()
        }),
        Err(FrameDescError::ZeroRows)
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            columns: 0,
            ..frame_input()
        }),
        Err(FrameDescError::ZeroColumns)
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            samples_per_pixel: 0,
            ..frame_input()
        }),
        Err(FrameDescError::ZeroSamplesPerPixel)
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_allocated: 12,
            bits_stored: 12,
            high_bit: 11,
            pixel_representation: PixelRepresentation::Signed,
            ..frame_input()
        }),
        Err(FrameDescError::UnsupportedBitsAllocated { bits: 12 })
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_stored: 0,
            high_bit: 0,
            ..frame_input()
        }),
        Err(FrameDescError::BitsStoredOutOfRange {
            bits_stored: 0,
            bits_allocated: 8,
        })
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_stored: 9,
            high_bit: 8,
            ..frame_input()
        }),
        Err(FrameDescError::BitsStoredOutOfRange {
            bits_stored: 9,
            bits_allocated: 8,
        })
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_allocated: 16,
            bits_stored: 12,
            high_bit: 10,
            pixel_representation: PixelRepresentation::Signed,
            ..frame_input()
        }),
        Err(FrameDescError::HighBitMismatch {
            high_bit: 10,
            expected: 11,
        })
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_allocated: 16,
            bits_stored: 12,
            high_bit: 12,
            ..frame_input()
        }),
        Err(FrameDescError::HighBitMismatch {
            high_bit: 12,
            expected: 11,
        })
    );
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_allocated: 16,
            bits_stored: 16,
            high_bit: 16,
            ..frame_input()
        }),
        Err(FrameDescError::HighBitMismatch {
            high_bit: 16,
            expected: 15,
        })
    );
}

#[test]
fn high_bit_must_equal_bits_stored_minus_one() {
    assert_eq!(
        FrameDesc::new(FrameDescInput {
            bits_allocated: 16,
            bits_stored: 12,
            high_bit: 13,
            ..frame_input()
        }),
        Err(FrameDescError::HighBitMismatch {
            high_bit: 13,
            expected: 11,
        })
    );
}

/// Deflated Explicit VR Little Endian. Known, and no decoder in this crate
/// claims it. PS3.5 A.5 deflates the whole data set.
const DEFLATED: &str = "1.2.840.10008.1.2.1.99";

/// PS3.5 Annex A membership this catalogue carries.
const KNOWN_SYNTAX_COUNT: usize = 19;
/// Syntaxes a fully registered build can decode.
const AVAILABLE_SYNTAX_COUNT: usize = 17;
/// Syntaxes that stay known and unregistered.
const UNAVAILABLE_SYNTAX_COUNT: usize = 2;

/// The decoder's own answer to how long a decoded frame is, per the operator's
/// answer to F-239 question 1. Native `YBR_FULL_422` stores two of its three
/// samples per pixel (PS3.3 C.7.6.3.1.2 stores each pixel pair as `Y1 Y2 Cb
/// Cr`), so its frame is four bytes and not six, while every decoder of
/// three-component output keeps six.
#[test]
fn decoded_output_len_is_the_decoders() -> TestResult {
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry)?;
    register_jpeg_decoders(&mut registry)?;

    // One row, two columns, three samples per pixel at eight bits. The
    // container's own length is 2 * 3 * 1 = 6.
    let colour = FrameDesc::new(colour_input())?;
    // PS3.5 8.2: implicit VR Pixel Data is OW, explicit VR may carry OB.
    let colour_ow = FrameDesc::new(FrameDescInput {
        pixel_data_vr: PixelDataVr::Ow,
        ..colour_input()
    })?;
    let rgb = FrameDesc::new(FrameDescInput {
        photometric_interpretation: "RGB".to_owned(),
        ..colour_input()
    })?;

    // PS3.3 C.7.6.3.1.2: two stored samples per pixel for the native 4:2:2
    // frame, so 1 * 2 * 2 * 8 / 8 = 4.
    assert_eq!(registry.decoded_output_len(IMPLICIT_VR_LE, &colour_ow)?, 4);
    assert_eq!(
        registry.decoded_output_len(EXPLICIT_VR_LE, &colour)?,
        4,
        "both native byte orders store two samples per pixel"
    );
    // Full three-sample output keeps the container length, 6.
    assert_eq!(registry.decoded_output_len(EXPLICIT_VR_LE, &rgb)?, 6);
    // JPEG decodes to three samples per pixel even for a 4:2:2 photometric.
    assert_eq!(registry.decoded_output_len(JPEG_BASELINE, &colour)?, 6);

    // And the dispatch length is the decoder's answer, not the container's.
    // A caller sizing from Samples per Pixel hands the native decoder six
    // bytes for a four-byte frame and is refused before any decoder runs.
    let source = [50_u8, 200, 100, 160];
    let mut too_long = [0_u8; 6];
    assert_eq!(
        registry.decode(IMPLICIT_VR_LE, &source, &colour_ow, &mut too_long),
        Err(CodecError::OutputLength {
            expected: 4,
            actual: 6,
        })
    );
    assert_eq!(too_long, [0_u8; 6], "a refused decode writes nothing");

    let mut out = [0_u8; 4];
    registry.decode(IMPLICIT_VR_LE, &source, &colour_ow, &mut out)?;
    assert_eq!(out, source, "a native 4:2:2 frame copies its stored bytes");

    let mut explicit_out = [0_u8; 4];
    registry.decode(EXPLICIT_VR_LE, &source, &colour, &mut explicit_out)?;
    assert_eq!(explicit_out, source);

    // A pixel pair with no partner. PS3.3 C.7.6.3.1.2 stores `Y1 Y2 Cb Cr`,
    // so an odd Columns cannot name which pixels share the last chroma pair
    // and the frame is refused rather than re-paired across rows.
    let odd = FrameDesc::new(FrameDescInput {
        columns: 3,
        pixel_data_vr: PixelDataVr::Ow,
        ..colour_input()
    })?;
    let mut out = [0_u8; 6];
    assert_eq!(
        registry.decode(IMPLICIT_VR_LE, &[0_u8; 6], &odd, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert_eq!(out, [0_u8; 6], "a refused decode writes nothing");

    // PS3.3 C.7.6.3.1.2 defines both 4:2:2 codes over three samples per
    // pixel. A header whose Samples per Pixel contradicts the photometric is
    // refused rather than re-counted. The buffer here is the length the
    // header itself declares, so the registry's buffer check passes and the
    // decoder's own refusal is the answer.
    for photometric in ["YBR_FULL_422", "YBR_PARTIAL_422"] {
        for samples_per_pixel in [1_u16, 2] {
            let contradicted = FrameDesc::new(FrameDescInput {
                samples_per_pixel,
                photometric_interpretation: photometric.to_owned(),
                ..colour_input()
            })?;
            let source = vec![0_u8; contradicted.output_len()];
            let mut out = vec![0xa5; contradicted.output_len()];
            assert_eq!(
                registry.decode(EXPLICIT_VR_LE, &source, &contradicted, &mut out),
                Err(CodecError::FrameMismatch),
                "Samples per Pixel {samples_per_pixel} under {photometric} must be refused"
            );
            assert!(
                out.iter().all(|byte| *byte == 0xa5),
                "a refused decode writes nothing"
            );
        }
    }
    Ok(())
}

fn colour_input() -> FrameDescInput {
    FrameDescInput {
        rows: 1,
        columns: 2,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_FULL_422".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    }
}

#[test]
fn every_decoder_family_registers_into_one_registry_without_collision() -> TestResult {
    // Each family's own test asserts that its registration is atomic and that
    // its own UIDs become available. **None of them registers more than one
    // family**, so the property that matters at the crate level is asserted
    // here: the six coexist, they partition the known catalogue, and no two
    // claim the same UID.
    //
    // Registration refuses a collision rather than overwriting, deviation
    // D-19, so a seventh family claiming a UID an existing one owns would fail
    // at run time in whatever order a caller happened to register. This test
    // is what makes that fail at build time instead.
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry)?;
    register_jpeg_decoders(&mut registry)?;
    register_jpeg2000_decoders(&mut registry)?;
    register_jpegls_decoders(&mut registry)?;
    register_htj2k_decoders(&mut registry)?;
    register_jpegxl_decoders(&mut registry)?;

    let mut available = Vec::new();
    let mut unavailable = Vec::new();
    for uid in KNOWN_TRANSFER_SYNTAXES {
        match registry.capability(uid) {
            Capability::Available => available.push(*uid),
            Capability::KnownUnavailable => unavailable.push(*uid),
            Capability::Unknown => return Err(format!("{uid} is not known").into()),
        }
    }

    // Seventeen available. Two stay known and unregistered. The names are
    // asserted, because a count alone would hide which one moved.
    assert_eq!(KNOWN_TRANSFER_SYNTAXES.len(), KNOWN_SYNTAX_COUNT);
    assert_eq!(available.len(), AVAILABLE_SYNTAX_COUNT);
    assert_eq!(unavailable.len(), UNAVAILABLE_SYNTAX_COUNT);
    assert_eq!(unavailable, vec![DEFLATED, JPEGXL_RECOMPRESSION]);
    assert_eq!(available.len() + unavailable.len(), KNOWN_SYNTAX_COUNT);

    // Deflated Explicit VR Little Endian is unavailable by design. PS3.5 A.5
    // deflates the whole data set rather than a frame, so it is a parser
    // route owned by `ocelli-dicom` under deviation D-18.
    assert_eq!(registry.capability(DEFLATED), Capability::KnownUnavailable);
    assert_eq!(
        registry.decoder(DEFLATED).err(),
        Some(CodecError::KnownUnavailable)
    );
    // JPEG XL JPEG Recompression is unavailable by design. `jxl-jbr` 0.2.1
    // `src/reconstruct.rs:457` panics with `expect("is_last not found")` in
    // the DHT branch of JPEG reconstruction. No adapter claims the UID, so
    // decode refuses before any byte reaches `jxl-oxide`.
    assert_eq!(
        registry.capability(JPEGXL_RECOMPRESSION),
        Capability::KnownUnavailable
    );
    let refusal = match registry.decoder(JPEGXL_RECOMPRESSION) {
        Err(error) => error,
        Ok(_) => return Err(".111 registered a decoder".into()),
    };
    assert_eq!(refusal, CodecError::KnownUnavailable);
    assert_eq!(
        refusal.to_string(),
        "known DICOM transfer syntax has no registered decoder"
    );
    Ok(())
}

#[test]
fn registering_a_family_twice_refuses_and_leaves_the_others_intact() -> TestResult {
    // The order-independence half of the same property. A repeated
    // registration must refuse rather than overwrite, and must not disturb the
    // families already present.
    let mut registry = Registry::new();
    register_jpegls_decoders(&mut registry)?;
    register_htj2k_decoders(&mut registry)?;
    assert!(register_jpegls_decoders(&mut registry).is_err());

    for uid in [
        "1.2.840.10008.1.2.4.80",
        "1.2.840.10008.1.2.4.81",
        "1.2.840.10008.1.2.4.201",
        "1.2.840.10008.1.2.4.202",
        "1.2.840.10008.1.2.4.203",
    ] {
        assert_eq!(registry.capability(uid), Capability::Available);
    }
    // And the families not registered are still distinguishable from unknown.
    assert_eq!(
        registry.capability("1.2.840.10008.1.2.4.90"),
        Capability::KnownUnavailable
    );
    Ok(())
}

const JPEGXL_LOSSLESS: &str = "1.2.840.10008.1.2.4.110";
const JPEGXL_RECOMPRESSION: &str = "1.2.840.10008.1.2.4.111";
const JPEGXL: &str = "1.2.840.10008.1.2.4.112";

#[test]
fn registering_jpegxl_twice_refuses() -> TestResult {
    // A second registration reports the first claimed UID. `.110` and `.112`
    // stay available. `.111` stays known and unregistered.
    let mut registry = Registry::new();
    register_jpegxl_decoders(&mut registry)?;
    assert_eq!(
        register_jpegxl_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered {
            uid: JPEGXL_LOSSLESS
        })
    );
    assert_eq!(registry.capability(JPEGXL_LOSSLESS), Capability::Available);
    assert_eq!(registry.capability(JPEGXL), Capability::Available);
    assert_eq!(
        registry.capability(JPEGXL_RECOMPRESSION),
        Capability::KnownUnavailable
    );
    Ok(())
}

#[test]
fn preflight_is_atomic() -> TestResult {
    // The catalogue knows `.110` and `.112` and does not know `.111`.
    // Preflight refuses `.111` before `.110` or `.112` is registered.
    let mut registry = Registry::with_known(&[JPEGXL_LOSSLESS, JPEGXL])?;
    assert_eq!(
        register_jpegxl_decoders(&mut registry),
        Err(RegistryError::UnknownTransferSyntax {
            uid: JPEGXL_RECOMPRESSION
        })
    );
    assert_eq!(
        registry.capability(JPEGXL_LOSSLESS),
        Capability::KnownUnavailable
    );
    assert_eq!(registry.capability(JPEGXL), Capability::KnownUnavailable);
    Ok(())
}
