//! DICOM Part 10 parsing and transfer-syntax dispatch.
//!
//! PS3.10 sections 7 and 7.1 fix File Meta Information to Explicit VR Little
//! Endian. Transfer Syntax UID `(0002,0010)` then selects the encoding of the
//! data set that follows. Keeping those two reads separate makes it impossible
//! to apply the selected data-set syntax to the meta group by accident.

use std::{
    cell::Cell,
    io::{BufReader, Cursor, Read, Seek, SeekFrom},
    rc::Rc,
};

use dicom_encoding::transfer_syntax::{Codec, TransferSyntax};
use dicom_object::{
    DefaultDicomObject, DicomCollectorOptions, FileMetaTable, InMemDicomObject, Tag,
    collector::Error as CollectorError,
    file::{OddLengthStrategy, ReadPreamble},
};
use dicom_parser::dataset::{
    LazyDataToken,
    lazy_read::{LazyDataSetReader, LazyDataSetReaderOptions},
};
use dicom_transfer_syntax_registry::{TransferSyntaxIndex, TransferSyntaxRegistry};

const PART10_PREFIX_END: usize = 132;
const PREAMBLE_LENGTH: u64 = 128;
const DICM_PREFIX: &[u8; 4] = b"DICM";

const IMPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.1";
const DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.1.99";
const EXPLICIT_VR_BIG_ENDIAN: &str = "1.2.840.10008.1.2.2";
const COMPLETION_SENTINEL: Tag = Tag(0x0002, 0x0000);
const PIXEL_DATA: Tag = Tag(0x7fe0, 0x0010);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataSetContainer {
    Sequence {
        end: Option<u64>,
    },
    Item {
        end: Option<u64>,
    },
    PixelSequence {
        has_basic_offset_table: bool,
        has_fragment: bool,
    },
    PixelItem {
        end: u64,
        kind: PixelItemKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PixelItemKind {
    BasicOffsetTable,
    Fragment,
}

struct PositionedCursor<'a> {
    inner: Cursor<&'a [u8]>,
    position: Rc<Cell<u64>>,
}

impl Read for PositionedCursor<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.position.set(self.inner.position());
        Ok(read)
    }
}

impl Seek for PositionedCursor<'_> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let position = self.inner.seek(position)?;
        self.position.set(position);
        Ok(position)
    }
}

/// The data-set parser path selected by File Meta Information.
///
/// Encapsulated syntaxes use Explicit VR Little Endian for the data set and
/// retain pixel fragments. F-016 does not decode those fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DispatchPath {
    /// PS3.5 A.1, no VR field and little-endian values.
    ImplicitVrLittleEndian,
    /// PS3.5 A.2, explicit VR and little-endian values.
    ExplicitVrLittleEndian,
    /// PS3.5 A.3, retired explicit VR and big-endian values.
    ExplicitVrBigEndian,
    /// PS3.5 A.5, a deflated Explicit VR Little Endian data set.
    DeflatedExplicitVrLittleEndian,
    /// PS3.5 A.4, Explicit VR Little Endian with encapsulated pixel data.
    Encapsulated,
}

/// The immutable transfer-syntax evidence attached to a parsed object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransferSyntaxInfo {
    uid: &'static str,
    name: &'static str,
    dispatch_path: DispatchPath,
}

impl TransferSyntaxInfo {
    /// The canonical transfer-syntax UID resolved by dicom-rs.
    #[must_use]
    pub const fn uid(self) -> &'static str {
        self.uid
    }

    /// The dicom-rs registry name for the transfer syntax.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// The data-set parser path selected from the UID.
    #[must_use]
    pub const fn dispatch_path(self) -> DispatchPath {
        self.dispatch_path
    }
}

/// A parsed Part 10 object together with evidence of its selected parser path.
#[derive(Debug)]
pub struct ParsedDicom {
    object: DefaultDicomObject,
    transfer_syntax: TransferSyntaxInfo,
}

impl ParsedDicom {
    /// Borrow the complete dicom-rs object, including File Meta Information.
    #[must_use]
    pub const fn object(&self) -> &DefaultDicomObject {
        &self.object
    }

    /// Consume this result and return the complete dicom-rs object.
    #[must_use]
    pub fn into_object(self) -> DefaultDicomObject {
        self.object
    }

    /// The declared transfer syntax and the path it selected.
    #[must_use]
    pub const fn transfer_syntax(&self) -> TransferSyntaxInfo {
        self.transfer_syntax
    }
}

/// A patient-safe parser failure.
///
/// No variant retains input bytes, data-element values, instance identifiers,
/// paths, or upstream tokens. A caller can report the failure class without
/// putting DICOM content into a log or boundary event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Fewer than 132 bytes cannot contain a Part 10 preamble and prefix.
    #[error("DICOM Part 10 input ends before the DICM prefix")]
    TruncatedPart10,
    /// Bytes 128 through 131 are not the Part 10 prefix.
    #[error("DICOM Part 10 input has no DICM prefix")]
    MissingDicmPrefix,
    /// File Meta Information is malformed under Explicit VR Little Endian.
    #[error("DICOM File Meta Information is invalid")]
    InvalidFileMeta,
    /// File Meta Information omitted required Transfer Syntax UID `(0002,0010)`.
    #[error("DICOM File Meta Information has no Transfer Syntax UID")]
    MissingTransferSyntax,
    /// The built-in dicom-rs registry does not know the declared UID.
    #[error("DICOM File Meta Information declares an unknown transfer syntax")]
    UnknownTransferSyntax,
    /// The registry knows the syntax but this build cannot read its data set.
    #[error("this build cannot read the declared DICOM data-set transfer syntax")]
    UnsupportedDataSetTransferSyntax,
    /// The data set ended inside a token or value.
    #[error("the DICOM data set is truncated")]
    TruncatedDataSet,
    /// The data set is not valid under the declared transfer syntax.
    #[error("the DICOM data set is invalid under its declared transfer syntax")]
    InvalidDataSet,
}

/// Parse one DICOM Part 10 file from memory.
///
/// File Meta Information is read first under its fixed Explicit VR Little
/// Endian encoding. Its Transfer Syntax UID is resolved exactly once. The
/// remaining data set is adapted once, structurally preflighted, and collected
/// once under that resolved syntax. There is no fallback parse.
///
/// # Errors
///
/// Returns a [`ParseError`] for a malformed Part 10 envelope, an unknown or
/// unavailable transfer syntax, or a data set that is invalid under the
/// declared syntax.
pub fn parse_part10(bytes: &[u8]) -> Result<ParsedDicom, ParseError> {
    if bytes.len() < PART10_PREFIX_END {
        return Err(ParseError::TruncatedPart10);
    }
    if bytes.get(128..PART10_PREFIX_END) != Some(DICM_PREFIX.as_slice()) {
        return Err(ParseError::MissingDicmPrefix);
    }

    let mut reader = Cursor::new(bytes);
    reader.set_position(PREAMBLE_LENGTH);
    let meta = FileMetaTable::from_reader(&mut reader).map_err(map_meta_error)?;
    let uid = meta.transfer_syntax();
    let transfer_syntax = TransferSyntaxRegistry
        .get(uid)
        .ok_or(ParseError::UnknownTransferSyntax)?;
    let info = classify_transfer_syntax(
        transfer_syntax.uid(),
        transfer_syntax.name(),
        transfer_syntax.can_decode_dataset(),
        transfer_syntax.is_encapsulated_pixel_data(),
    )?;

    let data_set_start =
        usize::try_from(reader.position()).map_err(|_| ParseError::InvalidDataSet)?;
    let encoded_data_set = bytes
        .get(data_set_start..)
        .ok_or(ParseError::InvalidDataSet)?;
    let mut data_set = adapt_data_set(encoded_data_set, transfer_syntax)?;
    validate_data_set_structure(&data_set, transfer_syntax)?;
    append_completion_sentinel(&mut data_set, info.dispatch_path());

    let mut collector = DicomCollectorOptions::new()
        .read_preamble(ReadPreamble::Never)
        .odd_length_strategy(OddLengthStrategy::Fail)
        .expected_ts(transfer_syntax.uid())
        .from_reader(BufReader::new(Cursor::new(data_set)));
    let mut object = InMemDicomObject::new_empty();
    collector
        .read_dataset_to_end(&mut object)
        .map_err(map_collector_error)?;
    if object.take(COMPLETION_SENTINEL).is_none() {
        return Err(ParseError::TruncatedDataSet);
    }
    if object.tags().any(|tag| tag.0 == 0x0002) {
        return Err(ParseError::InvalidDataSet);
    }
    let object = object.with_exact_meta(meta);

    Ok(ParsedDicom {
        object,
        transfer_syntax: info,
    })
}

fn classify_transfer_syntax(
    uid: &'static str,
    name: &'static str,
    can_decode_data_set: bool,
    encapsulated: bool,
) -> Result<TransferSyntaxInfo, ParseError> {
    if !can_decode_data_set {
        return Err(ParseError::UnsupportedDataSetTransferSyntax);
    }

    let dispatch_path = match uid {
        IMPLICIT_VR_LITTLE_ENDIAN => DispatchPath::ImplicitVrLittleEndian,
        EXPLICIT_VR_LITTLE_ENDIAN => DispatchPath::ExplicitVrLittleEndian,
        EXPLICIT_VR_BIG_ENDIAN => DispatchPath::ExplicitVrBigEndian,
        DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN => DispatchPath::DeflatedExplicitVrLittleEndian,
        _ if encapsulated => DispatchPath::Encapsulated,
        _ => return Err(ParseError::UnsupportedDataSetTransferSyntax),
    };

    Ok(TransferSyntaxInfo {
        uid,
        name,
        dispatch_path,
    })
}

fn map_meta_error(error: dicom_object::meta::Error) -> ParseError {
    match error {
        dicom_object::meta::Error::MissingElement {
            alias: "TransferSyntax",
            ..
        } => ParseError::MissingTransferSyntax,
        _ => ParseError::InvalidFileMeta,
    }
}

fn adapt_data_set(
    encoded: &[u8],
    transfer_syntax: &'static TransferSyntax,
) -> Result<Vec<u8>, ParseError> {
    match transfer_syntax.codec() {
        Codec::Dataset(Some(_)) if transfer_syntax.uid() == DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN => {
            let mut decoded = Vec::new();
            let mut decoder = flate2::read::DeflateDecoder::new(encoded);
            decoder.read_to_end(&mut decoded).map_err(|error| {
                if error.kind() == std::io::ErrorKind::UnexpectedEof {
                    ParseError::TruncatedDataSet
                } else {
                    ParseError::InvalidDataSet
                }
            })?;
            validate_deflated_stream_end(encoded, decoder.total_in())?;
            Ok(decoded)
        }
        Codec::Dataset(Some(adapter)) => {
            let mut decoded = Vec::new();
            adapter
                .adapt_reader(Box::new(Cursor::new(encoded)))
                .read_to_end(&mut decoded)
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::UnexpectedEof {
                        ParseError::TruncatedDataSet
                    } else {
                        ParseError::InvalidDataSet
                    }
                })?;
            Ok(decoded)
        }
        Codec::Dataset(None) => Err(ParseError::UnsupportedDataSetTransferSyntax),
        Codec::None | Codec::EncapsulatedPixelData(..) => Ok(encoded.to_vec()),
    }
}

fn validate_deflated_stream_end(encoded: &[u8], consumed: u64) -> Result<(), ParseError> {
    let consumed = usize::try_from(consumed).map_err(|_| ParseError::InvalidDataSet)?;
    let suffix = encoded.get(consumed..).ok_or(ParseError::InvalidDataSet)?;
    let valid = if consumed.is_multiple_of(2) {
        suffix.is_empty()
    } else {
        suffix == [0]
    };
    if valid {
        Ok(())
    } else {
        Err(ParseError::InvalidDataSet)
    }
}

/// Strictly walk the adapted data set before adding the completion marker.
///
/// File Meta Information elements are not valid in the main data set. Refusing
/// a top-level group 0002 element here reserves one valid Explicit VR element
/// as an out-of-band completion marker without relying on private-element
/// allocation or a finite search through untrusted value bytes.
fn validate_data_set_structure(
    data_set: &[u8],
    transfer_syntax: &'static TransferSyntax,
) -> Result<(), ParseError> {
    let mut options = LazyDataSetReaderOptions::default();
    options.odd_length = OddLengthStrategy::Fail;
    let position = Rc::new(Cell::new(0));
    let source = PositionedCursor {
        inner: Cursor::new(data_set),
        position: Rc::clone(&position),
    };
    let mut reader = LazyDataSetReader::new_with_ts_options(source, transfer_syntax, options)
        .map_err(|error| map_data_set_error(&error))?;
    let mut containers = Vec::new();
    let encapsulated = transfer_syntax.is_encapsulated_pixel_data();

    loop {
        let position_before = position.get();
        let Some(token) = reader.advance() else {
            break;
        };
        let position_after = position.get();
        let token = token.map_err(|error| map_data_set_error(&error))?;
        match token {
            LazyDataToken::ElementHeader(header) => {
                let native_pixel_data_vr = header.vr.to_bytes();
                if matches!(containers.last(), Some(DataSetContainer::Sequence { .. }))
                    || header.tag.0 == 0xfffe
                    || (containers.is_empty() && header.tag.0 == 0x0002)
                    || (encapsulated && containers.is_empty() && header.tag == PIXEL_DATA)
                    || (header.tag == PIXEL_DATA
                        && native_pixel_data_vr != *b"OB"
                        && native_pixel_data_vr != *b"OW")
                {
                    return Err(ParseError::InvalidDataSet);
                }
            }
            LazyDataToken::SequenceStart { tag, len } => {
                if matches!(containers.last(), Some(DataSetContainer::Sequence { .. }))
                    || tag.0 == 0xfffe
                    || tag == PIXEL_DATA
                    || (containers.is_empty() && tag.0 == 0x0002)
                {
                    return Err(ParseError::InvalidDataSet);
                }
                containers.push(DataSetContainer::Sequence {
                    end: container_end(position_after, len.0)?,
                });
            }
            LazyDataToken::PixelSequenceStart => {
                if !encapsulated
                    || matches!(containers.last(), Some(DataSetContainer::Sequence { .. }))
                {
                    return Err(ParseError::InvalidDataSet);
                }
                validate_encapsulated_pixel_data_header(data_set, position_before, position_after)?;
                containers.push(DataSetContainer::PixelSequence {
                    has_basic_offset_table: false,
                    has_fragment: false,
                });
            }
            LazyDataToken::SequenceEnd => {
                match containers.last().copied() {
                    Some(DataSetContainer::Sequence { end }) => {
                        validate_container_end(data_set, end, position_before, position_after)?;
                    }
                    Some(DataSetContainer::PixelSequence {
                        has_basic_offset_table: true,
                        has_fragment: true,
                    }) => {
                        validate_container_end(data_set, None, position_before, position_after)?;
                    }
                    _ => return Err(ParseError::InvalidDataSet),
                };
                containers.pop();
            }
            token @ LazyDataToken::LazyValue { .. } => {
                if matches!(containers.last(), Some(DataSetContainer::Sequence { .. })) {
                    return Err(ParseError::InvalidDataSet);
                }
                token.skip().map_err(|error| map_data_set_error(&error))?;
            }
            token @ LazyDataToken::LazyItemValue { .. } => {
                if !matches!(
                    containers.last(),
                    Some(DataSetContainer::Item { .. } | DataSetContainer::PixelItem { .. })
                ) {
                    return Err(ParseError::InvalidDataSet);
                }
                token.skip().map_err(|error| map_data_set_error(&error))?;
            }
            LazyDataToken::ItemStart { len } => {
                let container = match containers.last().copied() {
                    Some(DataSetContainer::Sequence { .. }) => DataSetContainer::Item {
                        end: container_end(position_after, len.0)?,
                    },
                    Some(DataSetContainer::PixelSequence {
                        has_basic_offset_table,
                        ..
                    }) if !has_basic_offset_table
                        && len.0 != u32::MAX
                        && len.0.is_multiple_of(4) =>
                    {
                        DataSetContainer::PixelItem {
                            end: position_after
                                .checked_add(u64::from(len.0))
                                .ok_or(ParseError::InvalidDataSet)?,
                            kind: PixelItemKind::BasicOffsetTable,
                        }
                    }
                    Some(DataSetContainer::PixelSequence {
                        has_basic_offset_table: true,
                        ..
                    }) if len.0 != u32::MAX && len.0 >= 2 && len.0.is_multiple_of(2) => {
                        DataSetContainer::PixelItem {
                            end: position_after
                                .checked_add(u64::from(len.0))
                                .ok_or(ParseError::InvalidDataSet)?,
                            kind: PixelItemKind::Fragment,
                        }
                    }
                    _ => return Err(ParseError::InvalidDataSet),
                };
                containers.push(container);
            }
            LazyDataToken::ItemEnd => {
                let pixel_item_kind = match containers.last().copied() {
                    Some(DataSetContainer::Item { end }) => {
                        validate_container_end(data_set, end, position_before, position_after)?;
                        None
                    }
                    Some(DataSetContainer::PixelItem { end, kind }) => {
                        validate_container_end(
                            data_set,
                            Some(end),
                            position_before,
                            position_after,
                        )?;
                        Some(kind)
                    }
                    _ => return Err(ParseError::InvalidDataSet),
                };
                containers.pop();
                if let Some(kind) = pixel_item_kind {
                    let Some(DataSetContainer::PixelSequence {
                        has_basic_offset_table,
                        has_fragment,
                    }) = containers.last_mut()
                    else {
                        return Err(ParseError::InvalidDataSet);
                    };
                    match kind {
                        PixelItemKind::BasicOffsetTable => *has_basic_offset_table = true,
                        PixelItemKind::Fragment => *has_fragment = true,
                    }
                }
            }
            _ => return Err(ParseError::InvalidDataSet),
        }
    }

    if containers.is_empty() {
        Ok(())
    } else {
        Err(ParseError::TruncatedDataSet)
    }
}

fn validate_encapsulated_pixel_data_header(
    data_set: &[u8],
    position_before: u64,
    position_after: u64,
) -> Result<(), ParseError> {
    let start = usize::try_from(position_before).map_err(|_| ParseError::InvalidDataSet)?;
    let end = usize::try_from(position_after).map_err(|_| ParseError::InvalidDataSet)?;
    let header = data_set.get(start..end).ok_or(ParseError::InvalidDataSet)?;
    if header.get(4..12) == Some(&[b'O', b'B', 0, 0, 0xff, 0xff, 0xff, 0xff]) {
        Ok(())
    } else {
        Err(ParseError::InvalidDataSet)
    }
}

fn container_end(value_start: u64, length: u32) -> Result<Option<u64>, ParseError> {
    if length == u32::MAX {
        Ok(None)
    } else {
        value_start
            .checked_add(u64::from(length))
            .map(Some)
            .ok_or(ParseError::InvalidDataSet)
    }
}

fn validate_container_end(
    data_set: &[u8],
    expected_end: Option<u64>,
    position_before: u64,
    position_after: u64,
) -> Result<(), ParseError> {
    let valid = match expected_end {
        Some(expected_end) => position_before == expected_end && position_after == expected_end,
        None => {
            let length_start = position_before
                .checked_add(4)
                .and_then(|position| usize::try_from(position).ok());
            let delimiter_end = usize::try_from(position_after).ok();
            position_after.checked_sub(position_before) == Some(8)
                && length_start
                    .zip(delimiter_end)
                    .and_then(|(start, end)| data_set.get(start..end))
                    == Some([0_u8; 4].as_slice())
        }
    };
    if valid {
        Ok(())
    } else {
        Err(ParseError::InvalidDataSet)
    }
}

/// Append one complete File Meta Information element after strict preflight.
/// dicom-rs treats EOF one to three bytes into a top-level tag as a clean end.
/// Shifting this element behind such a tail either produces a parse error or
/// makes the exact marker disappear. A missing encapsulated sequence
/// delimiter also becomes observable because these bytes are not an item.
/// The marker is removed before the object crosses the public boundary.
fn append_completion_sentinel(data_set: &mut Vec<u8>, dispatch_path: DispatchPath) {
    let tag = (COMPLETION_SENTINEL.0, COMPLETION_SENTINEL.1);
    match dispatch_path {
        DispatchPath::ImplicitVrLittleEndian => {
            data_set.extend_from_slice(&tag.0.to_le_bytes());
            data_set.extend_from_slice(&tag.1.to_le_bytes());
            data_set.extend_from_slice(&4_u32.to_le_bytes());
            data_set.extend_from_slice(&0_u32.to_le_bytes());
        }
        DispatchPath::ExplicitVrBigEndian => {
            data_set.extend_from_slice(&tag.0.to_be_bytes());
            data_set.extend_from_slice(&tag.1.to_be_bytes());
            data_set.extend_from_slice(b"UL");
            data_set.extend_from_slice(&4_u16.to_be_bytes());
            data_set.extend_from_slice(&0_u32.to_be_bytes());
        }
        DispatchPath::ExplicitVrLittleEndian
        | DispatchPath::DeflatedExplicitVrLittleEndian
        | DispatchPath::Encapsulated => {
            data_set.extend_from_slice(&tag.0.to_le_bytes());
            data_set.extend_from_slice(&tag.1.to_le_bytes());
            data_set.extend_from_slice(b"UL");
            data_set.extend_from_slice(&4_u16.to_le_bytes());
            data_set.extend_from_slice(&0_u32.to_le_bytes());
        }
    }
}

fn map_collector_error(error: CollectorError) -> ParseError {
    map_data_set_error(&error)
}

fn map_data_set_error(error: &(dyn std::error::Error + 'static)) -> ParseError {
    if error_chain_has_unexpected_eof(error) {
        ParseError::TruncatedDataSet
    } else {
        ParseError::InvalidDataSet
    }
}

fn error_chain_has_unexpected_eof(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(source) = current {
        if source
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::UnexpectedEof)
        {
            return true;
        }
        current = source.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{ParseError, classify_transfer_syntax};

    #[test]
    fn a_known_syntax_without_a_data_set_reader_is_refused() {
        assert_eq!(
            classify_transfer_syntax("1.2.3", "fixture", false, false),
            Err(ParseError::UnsupportedDataSetTransferSyntax)
        );
    }

    #[test]
    fn a_non_encapsulated_syntax_without_a_supported_route_is_refused() {
        assert_eq!(
            classify_transfer_syntax("1.2.3", "fixture", true, false)
                .map(|info| info.dispatch_path()),
            Err(ParseError::UnsupportedDataSetTransferSyntax)
        );
    }
}
