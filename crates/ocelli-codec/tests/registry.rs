use std::{
    collections::HashSet,
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, Decoder, FrameDesc, FrameDescError,
    FrameDescInput, KNOWN_TRANSFER_SYNTAXES, PixelRepresentation, Registry, RegistryError,
};

type TestResult = Result<(), Box<dyn Error>>;

const TS_A: &str = "1.2.840.10008.1.2.4.50";
const TS_B: &str = "1.2.840.10008.1.2.4.51";
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

    assert_eq!(KNOWN_TRANSFER_SYNTAXES.len(), 16);
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
