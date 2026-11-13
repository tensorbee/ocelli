//! Part 10 and transfer-syntax dispatch fixtures for F-016.
//!
//! The byte layouts here are independent inputs, not output from Ocelli.
//! PS3.10 sections 7 and 7.1 require the 128-byte preamble, `DICM` prefix,
//! Explicit VR Little Endian File Meta Information, and Transfer Syntax UID.
//! PS3.5 annex A defines each data-set encoding exercised below.

use std::io::Write;

use flate2::{Compression, write::DeflateEncoder};
use proptest::prelude::*;

use dicom_object::{StandardDataDictionary, mem::InMemDicomObject};
use ocelli_dicom::{
    DispatchPath, MAX_SEQUENCE_DEPTH, MetadataSet, ParseError, ParseLimits, Tag, parse_part10,
    parse_part10_with_limits,
};

const IMPLICIT_VR_LE: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const DEFLATED_EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1.99";
const EXPLICIT_VR_BE: &str = "1.2.840.10008.1.2.2";
const JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";

const SOP_CLASS_UID: &[u8] = b"1.2.840.10008.5.1.4.1.1.2\0";
const SOP_INSTANCE_UID: &[u8] = b"2.25.1";
const IMPLEMENTATION_UID: &[u8] = b"2.25.2";

const DEFLATED_DATA_SET: &[u8] = &[
    // Python 3 compressobj(wbits=-15) over explicit_little_data_set(), fixed
    // at the default level. PS3.5 A.5 specifies an RFC 1951 Deflate stream.
    227, 96, 16, 99, 8, 245, 148, 98, 48, 212, 51, 210, 179, 48, 49, 208, 51, 52, 48, 48, 176, 208,
    51, 213, 51, 212, 51, 1, 98, 160, 40, 131, 6, 3, 7, 131, 103, 48, 19, 131, 145, 130, 6, 131, 0,
    67, 40, 144, 197, 4, 20, 51, 96, 112, 9, 230, 96, 48, 208, 51, 141, 49, 208, 51, 50, 5, 0, 0,
];

// Python 3 compressobj(wbits=-15) over the Explicit VR Little Endian sequence
// fixtures below, fixed at the default level. An odd stream has its required
// PS3.5 A.5 NULL padding byte appended.
const DEFLATED_DEFINED_SEQUENCE_WITH_ITEM: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 224, 96, 96, 96, 248, 247, 159, 225, 1, 144, 98, 0, 0, 0,
];
const DEFLATED_DEFINED_SEQUENCE_WITH_ITEM_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 224, 96, 96, 96, 248, 247, 159, 247, 1, 144, 98, 0, 0, 0,
];
const DEFLATED_DEFINED_SEQUENCE_WITH_SEQUENCE_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 224, 96, 96, 96, 248, 247, 255, 238, 3, 32, 197, 0, 0, 0,
];
const DEFLATED_UNDEFINED_ITEM_WITH_NONZERO_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 248, 15, 4, 255, 254, 51, 60, 128, 208, 188, 15, 88, 24, 24,
    24, 254, 253, 191, 251, 0, 72, 49, 0, 0,
];
const DEFLATED_UNDEFINED_SEQUENCE_WITH_NONZERO_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 248, 15, 4, 255, 254, 223, 125, 192, 194, 192, 192, 0, 0, 0,
];

/// The Part 10 bytes of the TID 1500 document packages/core writeTid1500
/// writes, captured on 2026-11-06 from the built packages/core dist built
/// from the identical tid1500.ts source. One NUM root, Explicit VR Little
/// Endian, synthetic UIDs only. The document nests five Sequences deep.
const REPOSITORY_TID_1500: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    68, 73, 67, 77, 2, 0, 0, 0, 85, 76, 4, 0, 118, 0, 0, 0, 2, 0, 2, 0, 85, 73, 30, 0, 49, 46, 50,
    46, 56, 52, 48, 46, 49, 48, 48, 48, 56, 46, 53, 46, 49, 46, 52, 46, 49, 46, 49, 46, 56, 56, 46,
    51, 52, 0, 2, 0, 3, 0, 85, 73, 44, 0, 50, 46, 50, 53, 46, 50, 52, 56, 51, 49, 53, 57, 57, 54,
    57, 51, 55, 56, 56, 53, 52, 50, 52, 54, 55, 49, 54, 50, 57, 52, 54, 50, 54, 53, 50, 48, 57, 49,
    52, 49, 55, 48, 49, 48, 2, 0, 16, 0, 85, 73, 20, 0, 49, 46, 50, 46, 56, 52, 48, 46, 49, 48, 48,
    48, 56, 46, 49, 46, 50, 46, 49, 0, 8, 0, 22, 0, 85, 73, 30, 0, 49, 46, 50, 46, 56, 52, 48, 46,
    49, 48, 48, 48, 56, 46, 53, 46, 49, 46, 52, 46, 49, 46, 49, 46, 56, 56, 46, 51, 52, 0, 8, 0,
    24, 0, 85, 73, 44, 0, 50, 46, 50, 53, 46, 50, 52, 56, 51, 49, 53, 57, 57, 54, 57, 51, 55, 56,
    56, 53, 52, 50, 52, 54, 55, 49, 54, 50, 57, 52, 54, 50, 54, 53, 50, 48, 57, 49, 52, 49, 55, 48,
    49, 48, 8, 0, 96, 0, 67, 83, 2, 0, 83, 82, 8, 0, 16, 1, 83, 81, 0, 0, 255, 255, 255, 255, 254,
    255, 0, 224, 255, 255, 255, 255, 8, 0, 2, 1, 83, 72, 8, 0, 57, 57, 100, 99, 109, 106, 115, 32,
    8, 0, 3, 1, 83, 72, 2, 0, 48, 32, 8, 0, 21, 1, 83, 84, 20, 0, 67, 111, 100, 101, 115, 32, 117,
    115, 101, 100, 32, 102, 111, 114, 32, 100, 99, 109, 106, 115, 8, 0, 22, 1, 83, 84, 34, 0, 104,
    116, 116, 112, 115, 58, 47, 47, 103, 105, 116, 104, 117, 98, 46, 99, 111, 109, 47, 100, 99,
    109, 106, 115, 45, 111, 114, 103, 47, 100, 99, 109, 106, 115, 254, 255, 13, 224, 0, 0, 0, 0,
    254, 255, 221, 224, 0, 0, 0, 0, 8, 0, 17, 17, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 221,
    224, 0, 0, 0, 0, 32, 0, 13, 0, 85, 73, 6, 0, 50, 46, 50, 53, 46, 49, 32, 0, 14, 0, 85, 73, 44,
    0, 50, 46, 50, 53, 46, 50, 57, 50, 49, 51, 49, 57, 52, 52, 48, 57, 49, 57, 52, 54, 50, 53, 51,
    52, 52, 53, 55, 49, 50, 54, 49, 54, 55, 49, 50, 53, 49, 54, 52, 49, 56, 48, 49, 56, 32, 0, 19,
    0, 73, 83, 2, 0, 49, 32, 64, 0, 67, 160, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224,
    255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49, 50, 54, 48, 48, 48, 8, 0, 2, 1, 83, 72, 4, 0,
    68, 67, 77, 32, 8, 0, 4, 1, 76, 79, 26, 0, 73, 109, 97, 103, 105, 110, 103, 32, 77, 101, 97,
    115, 117, 114, 101, 109, 101, 110, 116, 32, 82, 101, 112, 111, 114, 116, 254, 255, 13, 224, 0,
    0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 80, 160, 67, 83, 8, 0, 83, 69, 80, 65, 82, 65,
    84, 69, 64, 0, 114, 163, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 221, 224, 0, 0, 0, 0, 64,
    0, 117, 163, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 21,
    17, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 153, 17, 83,
    81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 80, 17, 85, 73, 26,
    0, 49, 46, 50, 46, 56, 52, 48, 46, 49, 48, 48, 48, 56, 46, 53, 46, 49, 46, 52, 46, 49, 46, 49,
    46, 50, 0, 8, 0, 85, 17, 85, 73, 6, 0, 50, 46, 50, 53, 46, 51, 254, 255, 13, 224, 0, 0, 0, 0,
    254, 255, 221, 224, 0, 0, 0, 0, 32, 0, 14, 0, 85, 73, 6, 0, 50, 46, 50, 53, 46, 50, 254, 255,
    13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 32, 0, 13, 0, 85, 73, 6, 0, 50, 46, 50,
    53, 46, 49, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 145, 164, 67,
    83, 8, 0, 67, 79, 77, 80, 76, 69, 84, 69, 64, 0, 147, 164, 67, 83, 10, 0, 85, 78, 86, 69, 82,
    73, 70, 73, 69, 68, 64, 0, 4, 165, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255,
    255, 255, 255, 8, 0, 5, 1, 67, 83, 4, 0, 68, 67, 77, 82, 64, 0, 0, 219, 67, 83, 4, 0, 49, 53,
    48, 48, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 48, 167, 83, 81,
    0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 64, 0, 16, 160, 67, 83, 16, 0,
    72, 65, 83, 32, 67, 79, 78, 67, 69, 80, 84, 32, 77, 79, 68, 32, 64, 0, 64, 160, 67, 83, 4, 0,
    67, 79, 68, 69, 64, 0, 67, 160, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255,
    255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49, 50, 49, 48, 52, 57, 8, 0, 2, 1, 83, 72, 4, 0, 68, 67,
    77, 32, 8, 0, 4, 1, 76, 79, 40, 0, 76, 97, 110, 103, 117, 97, 103, 101, 32, 111, 102, 32, 67,
    111, 110, 116, 101, 110, 116, 32, 73, 116, 101, 109, 32, 97, 110, 100, 32, 68, 101, 115, 99,
    101, 110, 100, 97, 110, 116, 115, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0,
    0, 64, 0, 104, 161, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8,
    0, 0, 1, 83, 72, 2, 0, 101, 110, 8, 0, 2, 1, 83, 72, 8, 0, 82, 70, 67, 53, 54, 52, 54, 32, 8,
    0, 4, 1, 76, 79, 8, 0, 69, 110, 103, 108, 105, 115, 104, 32, 254, 255, 13, 224, 0, 0, 0, 0,
    254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 48, 167, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0,
    224, 255, 255, 255, 255, 64, 0, 16, 160, 67, 83, 16, 0, 72, 65, 83, 32, 67, 79, 78, 67, 69, 80,
    84, 32, 77, 79, 68, 32, 64, 0, 64, 160, 67, 83, 4, 0, 67, 79, 68, 69, 64, 0, 67, 160, 83, 81,
    0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49,
    50, 49, 48, 52, 54, 8, 0, 2, 1, 83, 72, 4, 0, 68, 67, 77, 32, 8, 0, 4, 1, 76, 79, 20, 0, 67,
    111, 117, 110, 116, 114, 121, 32, 111, 102, 32, 76, 97, 110, 103, 117, 97, 103, 101, 32, 254,
    255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 104, 161, 83, 81, 0, 0, 255,
    255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 2, 0, 85, 83, 8, 0, 2,
    1, 83, 72, 10, 0, 73, 83, 79, 51, 49, 54, 54, 95, 49, 32, 8, 0, 4, 1, 76, 79, 14, 0, 85, 110,
    105, 116, 101, 100, 32, 83, 116, 97, 116, 101, 115, 32, 254, 255, 13, 224, 0, 0, 0, 0, 254,
    255, 221, 224, 0, 0, 0, 0, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 254,
    255, 13, 224, 0, 0, 0, 0, 254, 255, 0, 224, 255, 255, 255, 255, 64, 0, 16, 160, 67, 83, 16, 0,
    72, 65, 83, 32, 79, 66, 83, 32, 67, 79, 78, 84, 69, 88, 84, 32, 64, 0, 64, 160, 67, 83, 6, 0,
    80, 78, 65, 77, 69, 32, 64, 0, 67, 160, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224,
    255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49, 50, 49, 48, 48, 56, 8, 0, 2, 1, 83, 72, 4, 0,
    68, 67, 77, 32, 8, 0, 4, 1, 76, 79, 20, 0, 80, 101, 114, 115, 111, 110, 32, 79, 98, 115, 101,
    114, 118, 101, 114, 32, 78, 97, 109, 101, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0,
    0, 0, 0, 64, 0, 35, 161, 80, 78, 16, 0, 117, 110, 107, 110, 111, 119, 110, 94, 117, 110, 107,
    110, 111, 119, 110, 32, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 0, 224, 255, 255, 255, 255,
    64, 0, 16, 160, 67, 83, 16, 0, 72, 65, 83, 32, 67, 79, 78, 67, 69, 80, 84, 32, 77, 79, 68, 32,
    64, 0, 64, 160, 67, 83, 4, 0, 67, 79, 68, 69, 64, 0, 67, 160, 83, 81, 0, 0, 255, 255, 255, 255,
    254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49, 50, 49, 48, 53, 56, 8, 0,
    2, 1, 83, 72, 4, 0, 68, 67, 77, 32, 8, 0, 4, 1, 76, 79, 18, 0, 80, 114, 111, 99, 101, 100, 117,
    114, 101, 32, 114, 101, 112, 111, 114, 116, 101, 100, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255,
    221, 224, 0, 0, 0, 0, 64, 0, 104, 161, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255,
    255, 255, 255, 8, 0, 0, 1, 83, 72, 10, 0, 51, 54, 51, 54, 55, 57, 48, 48, 53, 32, 8, 0, 2, 1,
    83, 72, 4, 0, 83, 67, 84, 32, 8, 0, 4, 1, 76, 79, 18, 0, 73, 109, 97, 103, 105, 110, 103, 32,
    112, 114, 111, 99, 101, 100, 117, 114, 101, 32, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221,
    224, 0, 0, 0, 0, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 0, 224, 255, 255, 255, 255, 64, 0,
    16, 160, 67, 83, 8, 0, 67, 79, 78, 84, 65, 73, 78, 83, 64, 0, 64, 160, 67, 83, 10, 0, 67, 79,
    78, 84, 65, 73, 78, 69, 82, 32, 64, 0, 67, 160, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0,
    224, 255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49, 49, 49, 48, 50, 56, 8, 0, 2, 1, 83, 72,
    4, 0, 68, 67, 77, 32, 8, 0, 4, 1, 76, 79, 14, 0, 73, 109, 97, 103, 101, 32, 76, 105, 98, 114,
    97, 114, 121, 32, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 80,
    160, 67, 83, 8, 0, 83, 69, 80, 65, 82, 65, 84, 69, 64, 0, 48, 167, 83, 81, 0, 0, 255, 255, 255,
    255, 254, 255, 0, 224, 255, 255, 255, 255, 64, 0, 16, 160, 67, 83, 8, 0, 67, 79, 78, 84, 65,
    73, 78, 83, 64, 0, 64, 160, 67, 83, 10, 0, 67, 79, 78, 84, 65, 73, 78, 69, 82, 32, 64, 0, 67,
    160, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 0, 1, 83,
    72, 6, 0, 49, 50, 54, 50, 48, 48, 8, 0, 2, 1, 83, 72, 4, 0, 68, 67, 77, 32, 8, 0, 4, 1, 76, 79,
    20, 0, 73, 109, 97, 103, 101, 32, 76, 105, 98, 114, 97, 114, 121, 32, 71, 114, 111, 117, 112,
    32, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 80, 160, 67, 83, 8,
    0, 83, 69, 80, 65, 82, 65, 84, 69, 64, 0, 48, 167, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255,
    0, 224, 255, 255, 255, 255, 8, 0, 153, 17, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224,
    255, 255, 255, 255, 8, 0, 80, 17, 85, 73, 26, 0, 49, 46, 50, 46, 56, 52, 48, 46, 49, 48, 48,
    48, 56, 46, 53, 46, 49, 46, 52, 46, 49, 46, 49, 46, 50, 0, 8, 0, 85, 17, 85, 73, 6, 0, 50, 46,
    50, 53, 46, 51, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 16, 160,
    67, 83, 8, 0, 67, 79, 78, 84, 65, 73, 78, 83, 64, 0, 64, 160, 67, 83, 6, 0, 73, 77, 65, 71, 69,
    32, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 254, 255, 13, 224, 0, 0, 0,
    0, 254, 255, 221, 224, 0, 0, 0, 0, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 0, 224, 255, 255,
    255, 255, 64, 0, 16, 160, 67, 83, 8, 0, 67, 79, 78, 84, 65, 73, 78, 83, 64, 0, 64, 160, 67, 83,
    10, 0, 67, 79, 78, 84, 65, 73, 78, 69, 82, 32, 64, 0, 67, 160, 83, 81, 0, 0, 255, 255, 255,
    255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49, 50, 54, 48, 49, 48, 8,
    0, 2, 1, 83, 72, 4, 0, 68, 67, 77, 32, 8, 0, 4, 1, 76, 79, 20, 0, 73, 109, 97, 103, 105, 110,
    103, 32, 77, 101, 97, 115, 117, 114, 101, 109, 101, 110, 116, 115, 254, 255, 13, 224, 0, 0, 0,
    0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0, 80, 160, 67, 83, 8, 0, 83, 69, 80, 65, 82, 65, 84,
    69, 64, 0, 48, 167, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 64,
    0, 16, 160, 67, 83, 8, 0, 67, 79, 78, 84, 65, 73, 78, 83, 64, 0, 64, 160, 67, 83, 10, 0, 67,
    79, 78, 84, 65, 73, 78, 69, 82, 32, 64, 0, 67, 160, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255,
    0, 224, 255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 6, 0, 49, 50, 53, 48, 48, 55, 8, 0, 2, 1, 83,
    72, 4, 0, 68, 67, 77, 32, 8, 0, 4, 1, 76, 79, 18, 0, 77, 101, 97, 115, 117, 114, 101, 109, 101,
    110, 116, 32, 71, 114, 111, 117, 112, 32, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0,
    0, 0, 0, 64, 0, 80, 160, 67, 83, 8, 0, 83, 69, 80, 65, 82, 65, 84, 69, 64, 0, 48, 167, 83, 81,
    0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 64, 0, 16, 160, 67, 83, 8, 0,
    67, 79, 78, 84, 65, 73, 78, 83, 64, 0, 64, 160, 67, 83, 4, 0, 78, 85, 77, 32, 64, 0, 67, 160,
    83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255, 255, 8, 0, 0, 1, 83, 72, 10,
    0, 52, 49, 48, 54, 54, 56, 48, 48, 51, 32, 8, 0, 2, 1, 83, 72, 4, 0, 83, 67, 84, 32, 8, 0, 4,
    1, 76, 79, 6, 0, 76, 101, 110, 103, 116, 104, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221,
    224, 0, 0, 0, 0, 64, 0, 0, 163, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255,
    255, 255, 64, 0, 234, 8, 83, 81, 0, 0, 255, 255, 255, 255, 254, 255, 0, 224, 255, 255, 255,
    255, 8, 0, 0, 1, 83, 72, 2, 0, 109, 109, 8, 0, 2, 1, 83, 72, 4, 0, 85, 67, 85, 77, 8, 0, 4, 1,
    76, 79, 2, 0, 109, 109, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 64, 0,
    10, 163, 68, 83, 4, 0, 49, 50, 46, 53, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0,
    0, 0, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 254, 255, 13, 224, 0, 0,
    0, 0, 254, 255, 221, 224, 0, 0, 0, 0, 254, 255, 13, 224, 0, 0, 0, 0, 254, 255, 221, 224, 0, 0,
    0, 0,
];

fn explicit_element_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    let length = fixture_u16(value.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn explicit_element_be(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_be_bytes());
    bytes.extend_from_slice(&tag.1.to_be_bytes());
    bytes.extend_from_slice(&vr);
    let length = fixture_u16(value.len());
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn implicit_element_le(tag: (u16, u16), value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    let length = fixture_u32(value.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn fixture_u16(length: usize) -> u16 {
    assert_eq!(
        length % 2,
        0,
        "DICOM element fixture values have even length"
    );
    assert!(u16::try_from(length).is_ok(), "fixture length fits in u16");
    u16::try_from(length).unwrap_or(u16::MAX)
}

fn fixture_u32(length: usize) -> u32 {
    assert_eq!(
        length % 2,
        0,
        "DICOM element fixture values have even length"
    );
    assert!(u32::try_from(length).is_ok(), "fixture length fits in u32");
    u32::try_from(length).unwrap_or(u32::MAX)
}

fn long_explicit_element_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    let length = fixture_u32(value.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn long_explicit_element_be(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_be_bytes());
    bytes.extend_from_slice(&tag.1.to_be_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    let length = fixture_u32(value.len());
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn file_meta(transfer_syntax: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(long_explicit_element_le((0x0002, 0x0001), *b"OB", &[0, 1]));
    body.extend(explicit_element_le((0x0002, 0x0002), *b"UI", SOP_CLASS_UID));
    body.extend(explicit_element_le(
        (0x0002, 0x0003),
        *b"UI",
        SOP_INSTANCE_UID,
    ));

    let mut syntax = transfer_syntax.as_bytes().to_vec();
    if !syntax.len().is_multiple_of(2) {
        syntax.push(0);
    }
    body.extend(explicit_element_le((0x0002, 0x0010), *b"UI", &syntax));
    body.extend(explicit_element_le(
        (0x0002, 0x0012),
        *b"UI",
        IMPLEMENTATION_UID,
    ));

    let mut meta = explicit_element_le(
        (0x0002, 0x0000),
        *b"UL",
        &fixture_u32(body.len()).to_le_bytes(),
    );
    meta.extend(body);
    meta
}

fn part10(transfer_syntax: &str, data_set: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta(transfer_syntax));
    bytes.extend_from_slice(data_set);
    bytes
}

fn explicit_little_data_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(explicit_element_le((0x0008, 0x0016), *b"UI", SOP_CLASS_UID));
    bytes.extend(explicit_element_le((0x0028, 0x0008), *b"IS", b"2 "));
    bytes.extend(explicit_element_le(
        (0x0028, 0x0010),
        *b"US",
        &2_u16.to_le_bytes(),
    ));
    bytes.extend(explicit_element_le((0x0028, 0x0030), *b"DS", b"0.5\\0.25"));
    bytes
}

fn implicit_little_data_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(implicit_element_le((0x0008, 0x0016), SOP_CLASS_UID));
    bytes.extend(implicit_element_le((0x0028, 0x0008), b"2 "));
    bytes.extend(implicit_element_le((0x0028, 0x0010), &2_u16.to_le_bytes()));
    bytes.extend(implicit_element_le((0x0028, 0x0030), b"0.5\\0.25"));
    bytes
}

fn explicit_big_data_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(explicit_element_be((0x0008, 0x0016), *b"UI", SOP_CLASS_UID));
    bytes.extend(explicit_element_be((0x0028, 0x0008), *b"IS", b"2 "));
    bytes.extend(explicit_element_be(
        (0x0028, 0x0010),
        *b"US",
        &2_u16.to_be_bytes(),
    ));
    bytes.extend(explicit_element_be((0x0028, 0x0030), *b"DS", b"0.5\\0.25"));
    bytes
}

fn encapsulated_data_set() -> Vec<u8> {
    encapsulated_data_set_with_delimiter_length(0)
}

fn encapsulated_data_set_with_delimiter_length(delimiter_length: u32) -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe000_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe000_u16.to_le_bytes());
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 3, 4]);
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe0dd_u16.to_le_bytes());
    bytes.extend_from_slice(&delimiter_length.to_le_bytes());
    bytes
}

fn encapsulated_pixel_data_header() -> Vec<u8> {
    encapsulated_pixel_data_header_with_vr(*b"OB")
}

fn encapsulated_pixel_data_header_with_vr(vr: [u8; 2]) -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x7fe0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes
}

fn encapsulated_data_set_with_vr(vr: [u8; 2]) -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header_with_vr(vr);
    bytes.extend(item_control_tag(0xe000));
    bytes.extend(item_control_tag_with_length(0xe000, 4));
    bytes.extend_from_slice(&[1, 2, 3, 4]);
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn encapsulated_data_set_without_fragment() -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag(0xe000));
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn encapsulated_data_set_with_fragment(fragment: &[u8]) -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag(0xe000));
    bytes.extend(item_control_tag_with_length(
        0xe000,
        fixture_u32(fragment.len()),
    ));
    bytes.extend_from_slice(fragment);
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn icon_image_sequence_with_native_pixel_data() -> Vec<u8> {
    icon_image_sequence_with_pixel_data_vr(*b"OB")
}

fn icon_image_sequence_with_pixel_data_vr(vr: [u8; 2]) -> Vec<u8> {
    let native_pixel_data = long_explicit_element_le((0x7fe0, 0x0010), vr, &[1, 2, 3, 4]);
    let mut item = item_control_tag_with_length(0xe000, fixture_u32(native_pixel_data.len()));
    item.extend(native_pixel_data);
    long_explicit_element_le((0x0088, 0x0200), *b"SQ", &item)
}

fn encapsulated_data_set_without_basic_offset_table() -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn encapsulated_data_set_with_two_byte_basic_offset_table() -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag_with_length(0xe000, 2));
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn encapsulated_data_set_with_item_delimiter_first() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x7fe0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe00d_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

fn odd_length_data_set() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x0028_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0100_u16.to_le_bytes());
    bytes.extend_from_slice(b"US");
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.push(8);
    bytes
}

fn forged_fixed_sentinel_before_truncated_value() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend(explicit_element_le(
        (0x0002, 0x0000),
        *b"UL",
        &0_u32.to_le_bytes(),
    ));
    bytes.extend_from_slice(&0x0028_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0101_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&12_u32.to_le_bytes());
    bytes
}

fn header_with_missing_twelve_byte_value() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x0028_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0101_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&12_u32.to_le_bytes());
    bytes
}

fn item_control_tag(element: u16) -> Vec<u8> {
    item_control_tag_with_length(element, 0)
}

fn item_control_tag_with_length(element: u16, length: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&element.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes
}

fn item_control_tag_be(element: u16) -> Vec<u8> {
    item_control_tag_be_with_length(element, 0)
}

fn item_control_tag_be_with_length(element: u16, length: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xfffe_u16.to_be_bytes());
    bytes.extend_from_slice(&element.to_be_bytes());
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes
}

fn undefined_explicit_sequence_le(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0008_u16.to_le_bytes());
    bytes.extend_from_slice(&0x1110_u16.to_le_bytes());
    bytes.extend_from_slice(b"SQ");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn undefined_implicit_sequence_le(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0008_u16.to_le_bytes());
    bytes.extend_from_slice(&0x1110_u16.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn undefined_explicit_sequence_be(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0008_u16.to_be_bytes());
    bytes.extend_from_slice(&0x1110_u16.to_be_bytes());
    bytes.extend_from_slice(b"SQ");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_be_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn assert_defined_sequence_route_results(
    element: u16,
    deflated_data_set: &[u8],
    expected: Result<(), ParseError>,
) {
    let item_le = item_control_tag(element);
    let item_be = item_control_tag_be(element);
    let explicit_le = long_explicit_element_le((0x0008, 0x1110), *b"SQ", &item_le);
    let cases = [
        (
            IMPLICIT_VR_LE,
            implicit_element_le((0x0008, 0x1110), &item_le),
        ),
        (EXPLICIT_VR_LE, explicit_le.clone()),
        (
            EXPLICIT_VR_BE,
            long_explicit_element_be((0x0008, 0x1110), *b"SQ", &item_be),
        ),
        (DEFLATED_EXPLICIT_VR_LE, deflated_data_set.to_vec()),
        (JPEG_BASELINE, explicit_le),
    ];

    for (transfer_syntax, data_set) in cases {
        let result = parse_part10(&part10(transfer_syntax, &data_set)).map(|_| ());
        assert_eq!(result, expected, "transfer syntax {transfer_syntax}");
    }
}

fn assert_undefined_sequence_refused_on_every_route(
    value_le: &[u8],
    value_be: &[u8],
    deflated_data_set: &[u8],
) {
    let explicit_le = undefined_explicit_sequence_le(value_le);
    let cases = [
        (IMPLICIT_VR_LE, undefined_implicit_sequence_le(value_le)),
        (EXPLICIT_VR_LE, explicit_le.clone()),
        (EXPLICIT_VR_BE, undefined_explicit_sequence_be(value_be)),
        (DEFLATED_EXPLICIT_VR_LE, deflated_data_set.to_vec()),
        (JPEG_BASELINE, explicit_le),
    ];

    for (transfer_syntax, data_set) in cases {
        assert_eq!(
            parse_part10(&part10(transfer_syntax, &data_set)).err(),
            Some(ParseError::InvalidDataSet),
            "transfer syntax {transfer_syntax}"
        );
    }
}

fn assert_route(transfer_syntax: &str, data_set: &[u8], route: DispatchPath) {
    let result = parse_part10(&part10(transfer_syntax, data_set));
    assert!(result.is_ok(), "the independently encoded fixture parses");
    if let Ok(parsed) = result {
        assert_eq!(parsed.transfer_syntax().uid(), transfer_syntax);
        assert_eq!(parsed.transfer_syntax().dispatch_path(), route);
        assert_eq!(
            parsed
                .object()
                .element(Tag(0x0028, 0x0010))
                .ok()
                .and_then(|element| element.to_int::<u16>().ok()),
            Some(2)
        );
    }
}

/// The 2 MiB stack the depth fixtures run on, AC1, AC2, AC5 and AC6.
const TWO_MIB: usize = 2 * 1024 * 1024;
/// The 256 KiB stack the no-recursion fixture runs on, AC3.
const QUARTER_MIB: usize = 256 * 1024;

/// The four PS3.5 forms the depth fixtures cover: both little endian
/// syntaxes, each with defined and with undefined lengths.
const DEPTH_FORMS: [(&str, bool, bool); 4] = [
    (EXPLICIT_VR_LE, false, false),
    (EXPLICIT_VR_LE, false, true),
    (IMPLICIT_VR_LE, true, false),
    (IMPLICIT_VR_LE, true, true),
];

/// Run `parse` on a thread with an explicit stack. A regression back to
/// recursion ends the test process instead of returning a wrong answer.
fn parse_on_stack<T: Send + 'static>(
    stack: usize,
    parse: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(parse)
        .map_err(|_| "the fixture thread did not start".to_owned())?
        .join()
        .map_err(|_| "the fixture thread did not survive the parse".to_owned())
}

/// A data set of `levels.len()` nested Sequences, one Item per Sequence,
/// with one US element inside the innermost Item. Each entry is that
/// level's PS3.5 length pair, `(sequence_undefined, item_undefined)`,
/// which sections 7.5.1 and 7.5.2 let the encoder choose per level.
/// `implicit` selects Implicit VR Little Endian for the Sequence headers
/// and the innermost element. The bytes are written once, so the 100,000
/// deep fixture does not copy its own bytes at every level.
fn nested_sequence_data_set(levels: &[(bool, bool)], implicit: bool) -> Vec<u8> {
    let header_len = if implicit { 8 } else { 12 };
    let content = if implicit {
        implicit_element_le((0x0028, 0x0010), &2_u16.to_le_bytes())
    } else {
        explicit_element_le((0x0028, 0x0010), *b"US", &2_u16.to_le_bytes())
    };
    // Each level's delimiters: an undefined Item adds its Item Delimitation
    // Item and an undefined Sequence adds its Sequence Delimitation Item,
    // eight bytes each.
    let suffix = |&(sequence_undefined, item_undefined): &(bool, bool)| {
        usize::from(item_undefined) * 8 + usize::from(sequence_undefined) * 8
    };
    // Walk the levels inner to outer. Each Item value is the next level's
    // complete Sequence element, or the US element at the innermost level.
    let mut item_values = Vec::with_capacity(levels.len());
    let mut inner = content.len();
    for level in levels.iter().rev() {
        item_values.push(inner);
        inner = header_len + 8 + inner + suffix(level);
    }
    item_values.reverse();

    let mut bytes = Vec::with_capacity(
        levels
            .iter()
            .map(|level| header_len + 8 + suffix(level))
            .sum::<usize>()
            + content.len(),
    );
    for (&level, &item_value) in levels.iter().zip(&item_values) {
        let (sequence_undefined, item_undefined) = level;
        // The Sequence tag is the same in both syntaxes. The Item tag is
        // always encoded as Implicit VR, PS3.5 section 7.5.
        bytes.extend_from_slice(&0x0008_u16.to_le_bytes());
        bytes.extend_from_slice(&0x1110_u16.to_le_bytes());
        if !implicit {
            bytes.extend_from_slice(b"SQ");
            bytes.extend_from_slice(&[0, 0]);
        }
        if sequence_undefined {
            bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            // The Sequence value is the Item header, the Item value and
            // this level's delimiters.
            let sequence_len = fixture_u32(8 + item_value + suffix(&level));
            bytes.extend_from_slice(&sequence_len.to_le_bytes());
        }
        let item_len = if item_undefined {
            u32::MAX
        } else {
            fixture_u32(item_value)
        };
        bytes.extend(item_control_tag_with_length(0xe000, item_len));
    }
    bytes.extend_from_slice(&content);
    for &(sequence_undefined, item_undefined) in levels.iter().rev() {
        if item_undefined {
            bytes.extend(item_control_tag(0xe00d));
        }
        if sequence_undefined {
            bytes.extend(item_control_tag(0xe0dd));
        }
    }
    bytes
}

/// One SQ element of `tag` in Explicit VR Little Endian with a defined
/// length, holding exactly one Item whose value is `item_value`.
fn sequence_of_one_item(tag: (u16, u16), item_value: &[u8]) -> Vec<u8> {
    let mut item = item_control_tag_with_length(0xe000, fixture_u32(item_value.len()));
    item.extend_from_slice(item_value);
    long_explicit_element_le(tag, *b"SQ", &item)
}

/// The basic coded entry of PS3.3 Table 8.8-1a, in PS3.5 section 7.1
/// ascending tag order: Code Value (0008,0100), Coding Scheme Designator
/// (0008,0102) and Code Meaning (0008,0104).
fn code_item(code_value: &[u8], scheme: &[u8], meaning: &[u8]) -> Vec<u8> {
    let mut item = explicit_element_le((0x0008, 0x0100), *b"SH", code_value);
    item.extend_from_slice(&explicit_element_le((0x0008, 0x0102), *b"SH", scheme));
    item.extend_from_slice(&explicit_element_le((0x0008, 0x0104), *b"LO", meaning));
    item
}

/// The maximum open Sequence count of `deepest_measured_shape_data_set`,
/// measured by parsing it in
/// `the_fixture_measures_the_recorded_deepest_shape_depth`. The plan's rule
/// keeps `MAX_SEQUENCE_DEPTH` at least four times this, enforced at compile
/// time rather than only recorded.
const DEEPEST_SHAPE_DEPTH: usize = 10;

const _: () = assert!(MAX_SEQUENCE_DEPTH >= 4 * DEEPEST_SHAPE_DEPTH);

/// The deepest known shape the rule measures, in Explicit VR Little Endian
/// with defined lengths. The measurement chain of PS3.16 2026d opens eight
/// Sequences: the document root Content Sequence holding the TID 1500 row 6
/// "Imaging Measurements" CONTAINER, the Imaging Measurements Content
/// Sequence holding the row 9 TID 1501 Measurement Group, the Measurement
/// Group Content Sequence holding the TID 1501 row 10 TID 300 measurement,
/// the TID 300 NUM's Content Sequence holding TID 315 row 1's CODE through
/// TID 300 row 1b's TID 301 and TID 301 row 11's INFERRED FROM of TID 315
/// "Equation or Table", the CODE's Content Sequence holding the TID 315 row
/// 2 HAS PROPERTIES NUM, that NUM's Measured Value Sequence (0040,A300) of
/// PS3.3 C.18, its Measurement Units Code Sequence (0040,08EA), and the
/// Type 3 Equivalent Code Sequence (0008,0121) of PS3.3 Table 8.8-1 inside
/// the units code item. The templates are Extensible, so this is the
/// deepest shape known, not a ceiling. PS3.3 C.12.1.1.9 keeps de-identified
/// prior values in the Original Attributes Sequence (0400,0561) through the
/// Modified Attributes Sequence (0400,0550), which wraps the chain once
/// more and lands at ten open Sequences. Every item is written in PS3.5
/// section 7.1 ascending tag order, including Measurement Units Code
/// Sequence (0040,08EA) before Numeric Value (0040,A30A) in the Measured
/// Value Item.
fn deepest_measured_shape_data_set() -> Vec<u8> {
    let equivalent = sequence_of_one_item(
        (0x0008, 0x0121),
        &code_item(b"2 ", b"99EQ", b"synthetic equivalent"),
    );
    let mut units_item = code_item(b"1 ", b"99SYN ", b"synthetic unit");
    units_item.extend_from_slice(&equivalent);
    let mut measured_value_item = sequence_of_one_item((0x0040, 0x08EA), &units_item);
    measured_value_item.extend_from_slice(&explicit_element_le((0x0040, 0xA30A), *b"DS", b"12.5"));
    let measured_value = sequence_of_one_item((0x0040, 0xA300), &measured_value_item);
    let code_content = sequence_of_one_item((0x0040, 0xA730), &measured_value);
    let num_content = sequence_of_one_item((0x0040, 0xA730), &code_content);
    let group_content = sequence_of_one_item((0x0040, 0xA730), &num_content);
    let measurements_content = sequence_of_one_item((0x0040, 0xA730), &group_content);
    let root_content = sequence_of_one_item((0x0040, 0xA730), &measurements_content);
    let modified = sequence_of_one_item((0x0400, 0x0550), &root_content);
    sequence_of_one_item((0x0400, 0x0561), &modified)
}

/// The maximum count of open Sequence containers reached while walking
/// `object`, counting a Sequence from its element through the end of its
/// Items. Items are not counted, matching the preflight, and encapsulated
/// pixel data cannot nest.
fn deepest_open_sequence_depth(
    object: &InMemDicomObject<StandardDataDictionary>,
    opened: usize,
) -> usize {
    let mut deepest = opened;
    for element in object {
        if let Some(items) = element.value().items() {
            deepest = deepest.max(opened + 1);
            for item in items {
                deepest = deepest.max(deepest_open_sequence_depth(item, opened + 1));
            }
        }
    }
    deepest
}

/// One top-level Sequence of tag (0008,1110) holding `items` Items, each
/// Item a chain of `depth_per_item` further Sequences ending in one US
/// element. The structure nests to `1 + depth_per_item` open Sequences and
/// holds `1 + items * depth_per_item` of them in total.
fn many_items_data_set(
    items: usize,
    depth_per_item: usize,
    implicit: bool,
    sequence_undefined: bool,
    item_undefined: bool,
) -> Vec<u8> {
    let levels = vec![(false, false); depth_per_item];
    let chain = nested_sequence_data_set(&levels, implicit);
    let mut content = Vec::new();
    for _ in 0..items {
        let item_length = if item_undefined {
            u32::MAX
        } else {
            fixture_u32(chain.len())
        };
        content.extend(item_control_tag_with_length(0xe000, item_length));
        content.extend_from_slice(&chain);
        if item_undefined {
            content.extend(item_control_tag(0xe00d));
        }
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0008_u16.to_le_bytes());
    bytes.extend_from_slice(&0x1110_u16.to_le_bytes());
    if !implicit {
        bytes.extend_from_slice(b"SQ");
        bytes.extend_from_slice(&[0, 0]);
    }
    if sequence_undefined {
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    } else {
        bytes.extend_from_slice(&fixture_u32(content.len()).to_le_bytes());
    }
    bytes.extend_from_slice(&content);
    if sequence_undefined {
        bytes.extend(item_control_tag(0xe0dd));
    }
    bytes
}

#[test]
fn file_meta_selects_implicit_vr_little_endian() {
    assert_route(
        IMPLICIT_VR_LE,
        &implicit_little_data_set(),
        DispatchPath::ImplicitVrLittleEndian,
    );
}

#[test]
fn file_meta_selects_explicit_vr_little_endian() {
    assert_route(
        EXPLICIT_VR_LE,
        &explicit_little_data_set(),
        DispatchPath::ExplicitVrLittleEndian,
    );
}

#[test]
fn file_meta_selects_retired_explicit_vr_big_endian() {
    assert_route(
        EXPLICIT_VR_BE,
        &explicit_big_data_set(),
        DispatchPath::ExplicitVrBigEndian,
    );
}

#[test]
fn file_meta_selects_deflated_explicit_vr_little_endian() {
    assert_route(
        DEFLATED_EXPLICIT_VR_LE,
        DEFLATED_DATA_SET,
        DispatchPath::DeflatedExplicitVrLittleEndian,
    );
}

#[test]
fn encapsulated_pixel_data_stays_encapsulated() {
    let result = parse_part10(&part10(JPEG_BASELINE, &encapsulated_data_set()));
    assert!(
        result.is_ok(),
        "encapsulated data set parses without pixel decode"
    );
    if let Ok(parsed) = result {
        assert_eq!(
            parsed.transfer_syntax().dispatch_path(),
            DispatchPath::Encapsulated
        );
        assert!(
            parsed
                .object()
                .element(Tag(0x7fe0, 0x0010))
                .is_ok_and(|element| element.value().fragments().is_some())
        );
    }
}

#[test]
fn string_padding_and_multiplicity_remain_available() {
    let result = parse_part10(&part10(EXPLICIT_VR_LE, &explicit_little_data_set()));
    assert!(result.is_ok());
    if let Ok(parsed) = result {
        assert_eq!(
            parsed
                .object()
                .element(Tag(0x0028, 0x0008))
                .ok()
                .and_then(|element| element.to_str().ok())
                .as_deref(),
            Some("2")
        );
        assert_eq!(
            parsed
                .object()
                .element(Tag(0x0028, 0x0030))
                .ok()
                .map(|element| element.value().multiplicity()),
            Some(2)
        );
        assert_eq!(parsed.object().meta().transfer_syntax(), EXPLICIT_VR_LE);
    }
}

#[test]
fn short_part10_input_is_refused() {
    assert_eq!(
        parse_part10(&[0; 131]).err(),
        Some(ParseError::TruncatedPart10)
    );
}

#[test]
fn wrong_part10_prefix_is_refused() {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"NOPE");
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::MissingDicmPrefix)
    );
}

#[test]
fn malformed_file_meta_is_refused() {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend_from_slice(&[2, 0, 0, 0, b'U', b'L', 4, 0, 1]);
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::InvalidFileMeta)
    );
}

#[test]
fn missing_transfer_syntax_is_refused() {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(explicit_element_le(
        (0x0002, 0x0000),
        *b"UL",
        &0_u32.to_le_bytes(),
    ));
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::MissingTransferSyntax)
    );
}

#[test]
fn unknown_transfer_syntax_is_refused_without_fallback() {
    let bytes = part10("1.2.826.0.1.3680043.10.999", &explicit_little_data_set());
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::UnknownTransferSyntax)
    );
}

#[test]
fn truncated_data_set_is_refused_without_fallback() {
    let mut data_set = explicit_little_data_set();
    data_set.truncate(data_set.len() - 1);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::TruncatedDataSet)
    );
}

#[test]
fn partial_top_level_headers_are_refused() {
    for tail in [&[0x08_u8][..], &[0x08, 0x00][..], &[0x08, 0x00, 0x20][..]] {
        let mut data_set = explicit_little_data_set();
        data_set.extend_from_slice(tail);
        assert_eq!(
            parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
            Some(ParseError::TruncatedDataSet)
        );
    }
}

#[test]
fn unterminated_encapsulated_pixel_data_is_refused() {
    let mut data_set = encapsulated_data_set();
    data_set.truncate(data_set.len() - 8);
    assert_eq!(
        parse_part10(&part10(JPEG_BASELINE, &data_set)).err(),
        Some(ParseError::TruncatedDataSet)
    );
}

#[test]
fn structurally_invalid_data_set_is_distinct_from_truncation() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_item_delimiter_first()
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn top_level_item_is_refused_as_invalid_structure() {
    let data_set = item_control_tag(0xe000);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn top_level_item_delimiter_is_refused_as_invalid_structure() {
    let data_set = item_control_tag(0xe00d);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn top_level_sequence_delimiter_is_refused_as_invalid_structure() {
    let data_set = item_control_tag(0xe0dd);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn defined_sequence_accepts_an_empty_item_on_every_route() {
    assert_defined_sequence_route_results(0xe000, DEFLATED_DEFINED_SEQUENCE_WITH_ITEM, Ok(()));
}

#[test]
fn defined_sequence_refuses_item_delimiter_without_an_item_on_every_route() {
    assert_defined_sequence_route_results(
        0xe00d,
        DEFLATED_DEFINED_SEQUENCE_WITH_ITEM_DELIMITER,
        Err(ParseError::InvalidDataSet),
    );
}

#[test]
fn defined_sequence_refuses_sequence_delimiter_on_every_route() {
    assert_defined_sequence_route_results(
        0xe0dd,
        DEFLATED_DEFINED_SEQUENCE_WITH_SEQUENCE_DELIMITER,
        Err(ParseError::InvalidDataSet),
    );
}

#[test]
fn undefined_item_refuses_a_nonzero_delimiter_length_on_every_route() {
    let mut value_le = item_control_tag_with_length(0xe000, u32::MAX);
    value_le.extend(item_control_tag_with_length(0xe00d, 4));
    value_le.extend(item_control_tag(0xe0dd));
    let mut value_be = item_control_tag_be_with_length(0xe000, u32::MAX);
    value_be.extend(item_control_tag_be_with_length(0xe00d, 4));
    value_be.extend(item_control_tag_be(0xe0dd));

    assert_undefined_sequence_refused_on_every_route(
        &value_le,
        &value_be,
        DEFLATED_UNDEFINED_ITEM_WITH_NONZERO_DELIMITER,
    );
}

#[test]
fn undefined_sequence_refuses_a_nonzero_delimiter_length_on_every_route() {
    assert_undefined_sequence_refused_on_every_route(
        &item_control_tag_with_length(0xe0dd, 4),
        &item_control_tag_be_with_length(0xe0dd, 4),
        DEFLATED_UNDEFINED_SEQUENCE_WITH_NONZERO_DELIMITER,
    );
}

#[test]
fn encapsulated_pixel_data_refuses_a_nonzero_delimiter_length() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_delimiter_length(4),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn native_syntax_refuses_encapsulated_pixel_data() {
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &encapsulated_data_set())).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_syntax_refuses_native_pixel_data() {
    let native_pixel_data = long_explicit_element_le((0x7fe0, 0x0010), *b"OB", &[1, 2, 3, 4]);
    assert_eq!(
        parse_part10(&part10(JPEG_BASELINE, &native_pixel_data)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_syntax_accepts_nested_native_pixel_data() {
    assert!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &icon_image_sequence_with_native_pixel_data(),
        ))
        .is_ok()
    );
}

#[test]
fn encapsulated_pixel_data_requires_ob_vr() {
    for vr in [*b"OW", *b"SQ"] {
        assert_eq!(
            parse_part10(&part10(JPEG_BASELINE, &encapsulated_data_set_with_vr(vr))).err(),
            Some(ParseError::InvalidDataSet),
            "VR {}{}",
            char::from(vr[0]),
            char::from(vr[1]),
        );
    }
}

#[test]
fn pixel_data_cannot_be_encoded_as_a_defined_length_sequence() {
    let pixel_data_sequence = long_explicit_element_le((0x7fe0, 0x0010), *b"SQ", &[]);
    for transfer_syntax in [EXPLICIT_VR_LE, JPEG_BASELINE] {
        assert_eq!(
            parse_part10(&part10(transfer_syntax, &pixel_data_sequence)).err(),
            Some(ParseError::InvalidDataSet),
            "transfer syntax {transfer_syntax}",
        );
    }
}

#[test]
fn native_pixel_data_requires_ob_or_ow_vr() {
    let top_level = explicit_element_le((0x7fe0, 0x0010), *b"UI", b"1.2\0");
    let nested = icon_image_sequence_with_pixel_data_vr(*b"UI");
    for (transfer_syntax, data_set) in [(EXPLICIT_VR_LE, top_level), (JPEG_BASELINE, nested)] {
        assert_eq!(
            parse_part10(&part10(transfer_syntax, &data_set)).err(),
            Some(ParseError::InvalidDataSet),
            "transfer syntax {transfer_syntax}",
        );
    }
}

#[test]
fn encapsulated_pixel_data_requires_a_basic_offset_table_item() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_without_basic_offset_table(),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn basic_offset_table_length_must_be_a_multiple_of_four() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_two_byte_basic_offset_table(),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_pixel_data_requires_a_fragment_after_the_offset_table() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_without_fragment(),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_pixel_data_refuses_an_empty_fragment() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_fragment(&[]),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_pixel_data_accepts_a_two_byte_fragment() {
    // F-244 AC6. PS3.5 A.4: the zero-length first Item is the Basic Offset
    // Table and the next Item, `01 02`, is the one Fragment. Asserting the
    // table and the Fragment, not only parse success, keeps this from passing
    // by dicom-parser re-emitting an unread two-byte Item.
    let parsed = parse_part10(&part10(
        JPEG_BASELINE,
        &encapsulated_data_set_with_fragment(&[1, 2]),
    ));
    assert_eq!(
        parsed.as_ref().ok().and_then(pixel_items),
        Some((Vec::new(), vec![vec![1, 2]])),
        "{:?}",
        parsed.err()
    );
}

/// The top-level Pixel Data Basic Offset Table values and Fragment values.
fn pixel_items(parsed: &ocelli_dicom::ParsedDicom) -> Option<(Vec<u32>, Vec<Vec<u8>>)> {
    let element = parsed.object().element(Tag(0x7fe0, 0x0010)).ok()?;
    items_of(element.value())
}

fn items_of(
    value: &dicom_core::value::Value<
        dicom_object::InMemDicomObject,
        dicom_core::value::InMemFragment,
    >,
) -> Option<(Vec<u32>, Vec<Vec<u8>>)> {
    let table = value.offset_table()?.to_vec();
    let fragments = value.fragments()?.to_vec();
    Some((table, fragments))
}

/// Encapsulated Pixel Data per PS3.5 A.4: the Basic Offset Table Item with
/// `table` as its value, one Item per Fragment, and the Sequence Delimitation
/// Item. Every length is written by hand from the value it precedes.
fn encapsulated_items(table: &[u8], fragments: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x7fe0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend(item_control_tag_with_length(
        0xe000,
        fixture_u32(table.len()),
    ));
    bytes.extend_from_slice(table);
    for fragment in fragments {
        bytes.extend(item_control_tag_with_length(
            0xe000,
            fixture_u32(fragment.len()),
        ));
        bytes.extend_from_slice(fragment);
    }
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn parse_encapsulated(
    table: &[u8],
    fragments: &[&[u8]],
) -> Result<ocelli_dicom::ParsedDicom, ParseError> {
    let mut data_set = explicit_little_data_set();
    data_set.extend(encapsulated_items(table, fragments));
    parse_part10(&part10(JPEG_BASELINE, &data_set))
}

#[test]
fn empty_table_then_a_six_byte_fragment() {
    // F-244 AC1. PS3.5 A.4 Table A.4-1's shape with one Fragment: the
    // Basic Offset Table Item `FE FF 00 E0 00 00 00 00` has no value, so the
    // six-byte Item after it is the first Fragment. Six is 2 mod 4, the
    // length dicom-object 0.10.0 refused as InvalidDataSet.
    let parsed = parse_encapsulated(&[], &[&[1, 2, 3, 4, 5, 6]]);
    assert_eq!(
        parsed.as_ref().ok().and_then(pixel_items),
        Some((Vec::new(), vec![vec![1, 2, 3, 4, 5, 6]])),
        "{:?}",
        parsed.err()
    );
}

#[test]
fn empty_table_then_an_eight_byte_fragment() {
    // F-244 AC2. PS3.5 A.4: "When the Item Value is not present, the Item
    // Length shall be zero". Eight bytes is 0 mod 4, which dicom-object
    // 0.10.0 read as a two-value table `[0x04030201, 0x08070605]` with no
    // Fragment, a quiet misread at parse level.
    let parsed = parse_encapsulated(&[], &[&[1, 2, 3, 4, 5, 6, 7, 8]]);
    assert_eq!(
        parsed.as_ref().ok().and_then(pixel_items),
        Some((Vec::new(), vec![vec![1, 2, 3, 4, 5, 6, 7, 8]])),
        "{:?}",
        parsed.err()
    );
}

#[test]
fn empty_table_then_three_fragments_in_order() {
    // F-244 AC3. PS3.5 A.4 Table A.4-1, "Encoded Single-frame Image Defined
    // as a Sequence of Three Fragments Without Basic Offset Table Item
    // Value". Lengths 4, 6 and 2 are each even per A.4's "even number of
    // bytes greater or equal to two".
    let parsed = parse_encapsulated(
        &[],
        &[
            &[0x0a, 0x0b, 0x0c, 0x0d],
            &[1, 2, 3, 4, 5, 6],
            &[0xee, 0xff],
        ],
    );
    assert_eq!(
        parsed.as_ref().ok().and_then(pixel_items),
        Some((
            Vec::new(),
            vec![
                vec![0x0a, 0x0b, 0x0c, 0x0d],
                vec![1, 2, 3, 4, 5, 6],
                vec![0xee, 0xff],
            ]
        )),
        "{:?}",
        parsed.err()
    );
}

#[test]
fn populated_table_is_unchanged() {
    // F-244 AC4. PS3.5 A.4 Table A.4-2's shape, two frames in three
    // Fragments. Offsets are measured from the first Fragment Item Tag:
    // frame 0 at 0, frame 1 at 8 bytes of Item header plus Fragment A's
    // 8 bytes of value, 0x10. Fragments B and C are frame 1's.
    let mut table = Vec::new();
    table.extend_from_slice(&0_u32.to_le_bytes());
    table.extend_from_slice(&0x10_u32.to_le_bytes());
    let parsed = parse_encapsulated(
        &table,
        &[&[1, 2, 3, 4, 5, 6, 7, 8], &[9, 10, 11, 12], &[13, 14]],
    );
    assert_eq!(
        parsed.as_ref().ok().and_then(pixel_items),
        Some((
            vec![0, 0x10],
            vec![
                vec![1, 2, 3, 4, 5, 6, 7, 8],
                vec![9, 10, 11, 12],
                vec![13, 14],
            ]
        )),
        "{:?}",
        parsed.err()
    );
}

#[test]
fn an_icon_with_an_empty_table_does_not_refuse_the_file() {
    // F-244 AC5. PS3.5 A.4 applies to any encapsulated Pixel Data, and an
    // Icon Image Sequence (0088,0200) item may carry it. The icon's table
    // is empty and its one Fragment is six bytes, 2 mod 4. The top level
    // carries a populated table `[0]` per A.4 for one frame.
    let icon_pixels = encapsulated_items(&[], &[&[1, 2, 3, 4, 5, 6]]);
    let mut icon_item = item_control_tag(0xe000);
    icon_item.splice(4..8, fixture_u32(icon_pixels.len()).to_le_bytes());
    icon_item.extend(icon_pixels);
    let mut data_set = explicit_little_data_set();
    data_set.extend(long_explicit_element_le(
        (0x0088, 0x0200),
        *b"SQ",
        &icon_item,
    ));
    data_set.extend(encapsulated_items(&0_u32.to_le_bytes(), &[&[1, 2]]));
    let parsed = parse_part10(&part10(JPEG_BASELINE, &data_set));
    let icon = parsed.as_ref().ok().and_then(|parsed| {
        let sequence = parsed.object().element(Tag(0x0088, 0x0200)).ok()?;
        let item = sequence.value().items()?.first()?;
        items_of(item.element(Tag(0x7fe0, 0x0010)).ok()?.value())
    });
    assert_eq!(
        icon,
        Some((Vec::new(), vec![vec![1, 2, 3, 4, 5, 6]])),
        "{:?}",
        parsed.as_ref().err()
    );
    assert_eq!(
        parsed.as_ref().ok().and_then(pixel_items),
        Some((vec![0], vec![vec![1, 2]]))
    );
}

#[test]
fn deflated_data_set_requires_null_padding_when_the_stream_length_is_odd() {
    let result = DEFLATED_DATA_SET
        .split_last()
        .and_then(|(_, unpadded)| parse_part10(&part10(DEFLATED_EXPLICIT_VR_LE, unpadded)).err());
    assert_eq!(result, Some(ParseError::InvalidDataSet));
}

#[test]
fn deflated_data_set_refuses_bytes_after_its_required_padding() {
    let mut data_set = DEFLATED_DATA_SET.to_vec();
    data_set.extend_from_slice(&[1, 2]);
    assert_eq!(
        parse_part10(&part10(DEFLATED_EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn odd_value_length_is_refused() {
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &odd_length_data_set())).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn input_tag_cannot_forge_completion_after_a_truncated_value() {
    assert_eq!(
        parse_part10(&part10(
            EXPLICIT_VR_LE,
            &forged_fixed_sentinel_before_truncated_value()
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn file_meta_group_is_refused_in_the_main_data_set() {
    let mut data_set = explicit_little_data_set();
    data_set.extend(long_explicit_element_le((0x0002, 0x0001), *b"OB", &[0, 1]));
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn completion_bytes_consumed_as_a_value_do_not_signal_success() {
    assert_eq!(
        parse_part10(&part10(
            EXPLICIT_VR_LE,
            &header_with_missing_twelve_byte_value()
        ))
        .err(),
        Some(ParseError::TruncatedDataSet)
    );
}

#[test]
fn parses_at_the_sequence_depth_limit() -> Result<(), String> {
    // PS3.5 section 7.5 places no bound on nesting, so the limit is this
    // build's MAX_SEQUENCE_DEPTH. A data set nested exactly that deep
    // parses, and its projection succeeds, on a 2 MiB debug test thread,
    // for every length form and both little endian syntaxes.
    for (syntax, implicit, undefined) in DEPTH_FORMS {
        let levels = vec![(undefined, undefined); MAX_SEQUENCE_DEPTH];
        let file = part10(syntax, &nested_sequence_data_set(&levels, implicit));
        let (parsed, projected) = parse_on_stack(TWO_MIB, move || {
            let parsed = parse_part10(&file);
            let projected = parsed
                .as_ref()
                .is_ok_and(|parsed| MetadataSet::from_object(parsed.object()).is_ok());
            (parsed, projected)
        })?;
        assert!(
            parsed.is_ok(),
            "a data set at the limit was refused under {syntax}: {parsed:?}"
        );
        assert!(
            projected,
            "the projection failed at the limit under {syntax}"
        );
    }
    Ok(())
}

#[test]
fn refuses_one_past_the_sequence_depth_limit() -> Result<(), String> {
    // One Sequence deeper than the limit is refused with the typed error,
    // for every length form and both little endian syntaxes. Items are not
    // counted, so the refusal lands one Sequence past the limit and not at
    // half of it.
    for (syntax, implicit, undefined) in DEPTH_FORMS {
        let levels = vec![(undefined, undefined); MAX_SEQUENCE_DEPTH + 1];
        let file = part10(syntax, &nested_sequence_data_set(&levels, implicit));
        let refused = parse_on_stack(TWO_MIB, move || parse_part10(&file).err())?;
        assert_eq!(
            refused,
            Some(ParseError::SequenceTooDeep),
            "transfer syntax {syntax}"
        );
    }
    Ok(())
}

#[test]
fn refuses_deep_nesting_without_recursing() -> Result<(), String> {
    // 100,000 nested Sequences. The preflight refuses at the push past the
    // limit, so the answer arrives on a 256 KiB thread. A check placed
    // after dicom-rs's recursive collection would overflow first.
    let levels = vec![(false, false); 100_000];
    let file = part10(EXPLICIT_VR_LE, &nested_sequence_data_set(&levels, false));
    let refused = parse_on_stack(QUARTER_MIB, move || parse_part10(&file).err())?;
    assert_eq!(refused, Some(ParseError::SequenceTooDeep));
    Ok(())
}

#[test]
fn refuses_deep_nesting_in_a_deflated_data_set() -> Result<(), String> {
    // PS3.5 A.5 inflates the whole data set before the preflight, so the
    // depth check runs on the inflated nesting and not on the encoded
    // bytes. The stream is RFC 1951 raw deflate at flate2's default level.
    // PS3.5 A.5 requires one trailing NULL when the stream length is odd,
    // and the stream length follows the nesting depth, so the fixture pads
    // it rather than passing only when the depth makes the stream even.
    let levels = vec![(false, false); MAX_SEQUENCE_DEPTH + 1];
    let data_set = nested_sequence_data_set(&levels, false);
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&data_set)
        .map_err(|error| error.to_string())?;
    let mut encoded = encoder.finish().map_err(|error| error.to_string())?;
    if encoded.len() % 2 == 1 {
        encoded.push(0);
    }
    let file = part10(DEFLATED_EXPLICIT_VR_LE, &encoded);
    let refused = parse_on_stack(TWO_MIB, move || parse_part10(&file).err())?;
    assert_eq!(refused, Some(ParseError::SequenceTooDeep));
    Ok(())
}

#[test]
fn a_deflated_odd_length_stream_requires_and_accepts_its_null_pad() -> Result<(), String> {
    // An RFC 1951 stream of odd length requires one trailing NULL per
    // PS3.5 A.5. The stream is made odd on purpose, independent of
    // MAX_SEQUENCE_DEPTH: a shallow data set carries one filler element
    // whose even value length grows until flate2's default level emits an
    // odd stream. The unpadded stream is refused as invalid, and the padded
    // stream parses and projects, on a 2 MiB thread.
    let mut encoded = None;
    for filler in 0_usize..=32 {
        let mut data_set = explicit_element_le((0x0028, 0x0010), *b"US", &2_u16.to_le_bytes());
        data_set.extend_from_slice(&explicit_element_le(
            (0x0008, 0x0080),
            *b"LO",
            &vec![b'o'; filler * 2],
        ));
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&data_set)
            .map_err(|error| error.to_string())?;
        let stream = encoder.finish().map_err(|error| error.to_string())?;
        if stream.len() % 2 == 1 {
            encoded = Some(stream);
            break;
        }
    }
    let mut encoded =
        encoded.ok_or_else(|| "no odd-length deflate stream in the filler sweep".to_string())?;
    assert_eq!(
        encoded.len() % 2,
        1,
        "the fixture stream is odd, measured {}",
        encoded.len()
    );
    let odd_file = part10(DEFLATED_EXPLICIT_VR_LE, &encoded);
    encoded.push(0);
    let padded_file = part10(DEFLATED_EXPLICIT_VR_LE, &encoded);
    let refused = parse_on_stack(TWO_MIB, move || parse_part10(&odd_file).err())?;
    assert_eq!(refused, Some(ParseError::InvalidDataSet));
    let (parsed, projected) = parse_on_stack(TWO_MIB, move || {
        let parsed = parse_part10(&padded_file);
        let projected = parsed
            .as_ref()
            .is_ok_and(|parsed| MetadataSet::from_object(parsed.object()).is_ok());
        (parsed, projected)
    })?;
    assert!(
        parsed.is_ok(),
        "the padded odd stream was refused: {parsed:?}"
    );
    assert!(projected, "the projection failed on the padded odd stream");
    Ok(())
}

#[test]
fn sequence_too_deep_message_is_fixed() {
    // The variant is Copy and carries no depth, value or tag, so its
    // Display is the one patient-safe sentence.
    let error = ParseError::SequenceTooDeep;
    let copied = error;
    assert_eq!(copied, error);
    assert_eq!(
        error.to_string(),
        "the DICOM data set nests sequences deeper than this build reads"
    );
}

#[test]
fn the_deepest_measured_shape_parses() -> Result<(), String> {
    // The deepest known shape the rule measures parses and projects on a
    // 2 MiB debug test thread: the PS3.16 2026d measurement chain of
    // deepest_measured_shape_data_set at ten open Sequences, well under
    // MAX_SEQUENCE_DEPTH. The limit itself is bounded directly by
    // parses_at_the_sequence_depth_limit and
    // refuses_one_past_the_sequence_depth_limit, not by this shape.
    let file = part10(EXPLICIT_VR_LE, &deepest_measured_shape_data_set());
    let (parsed, projected) = parse_on_stack(TWO_MIB, move || {
        let parsed = parse_part10(&file);
        let projected = parsed
            .as_ref()
            .is_ok_and(|parsed| MetadataSet::from_object(parsed.object()).is_ok());
        (parsed, projected)
    })?;
    assert!(
        parsed.is_ok(),
        "the deepest measured shape was refused: {parsed:?}"
    );
    assert!(
        projected,
        "the projection failed on the deepest measured shape"
    );
    Ok(())
}

#[test]
fn the_fixture_measures_the_recorded_deepest_shape_depth() -> Result<(), String> {
    // The margin rule is checked against the fixture, not a literal:
    // parsing deepest_measured_shape_data_set must measure exactly
    // DEEPEST_SHAPE_DEPTH, so removing a level from the fixture fails here
    // instead of silently weakening the constant's measured input.
    let file = part10(EXPLICIT_VR_LE, &deepest_measured_shape_data_set());
    let parsed = parse_part10(&file)
        .map_err(|error| format!("the deepest shape fixture was refused: {error}"))?;
    let measured = deepest_open_sequence_depth(parsed.object(), 0);
    assert_eq!(
        measured, DEEPEST_SHAPE_DEPTH,
        "the fixture no longer nests to the recorded depth"
    );
    Ok(())
}

#[test]
fn sibling_sequences_each_nested_to_the_limit_parse() -> Result<(), String> {
    // MAX_SEQUENCE_DEPTH + 1 sibling top-level Sequences, each a chain
    // nested to the limit, in every length form and both little endian
    // syntaxes. The file holds far more Sequences than the limit in total,
    // so a counter of Sequences instead of open depth refuses it, while the
    // depth of open containers accepts it.
    for (syntax, implicit, undefined) in DEPTH_FORMS {
        let levels = vec![(undefined, undefined); MAX_SEQUENCE_DEPTH];
        let chain = nested_sequence_data_set(&levels, implicit);
        let mut data_set = Vec::with_capacity(chain.len() * (MAX_SEQUENCE_DEPTH + 1));
        for _ in 0..MAX_SEQUENCE_DEPTH + 1 {
            data_set.extend_from_slice(&chain);
        }
        let file = part10(syntax, &data_set);
        let (parsed, projected) = parse_on_stack(TWO_MIB, move || {
            let parsed = parse_part10(&file);
            let projected = parsed
                .as_ref()
                .is_ok_and(|parsed| MetadataSet::from_object(parsed.object()).is_ok());
            (parsed, projected)
        })?;
        assert!(
            parsed.is_ok(),
            "sibling chains at the limit were refused under {syntax}: {parsed:?}"
        );
        assert!(
            projected,
            "the projection failed on sibling chains under {syntax}"
        );
    }
    Ok(())
}

#[test]
fn many_items_each_nested_to_the_limit_parse() -> Result<(), String> {
    // One top-level Sequence with MAX_SEQUENCE_DEPTH + 1 Items, each Item a
    // chain that nests the whole structure exactly to the limit, in every
    // length form and both little endian syntaxes. The file holds far more
    // Sequences than the limit in total, so a counter of Sequences instead
    // of open depth refuses it, while the depth of open containers accepts
    // it.
    for (syntax, implicit, undefined) in DEPTH_FORMS {
        let data_set = many_items_data_set(
            MAX_SEQUENCE_DEPTH + 1,
            MAX_SEQUENCE_DEPTH - 1,
            implicit,
            undefined,
            undefined,
        );
        let file = part10(syntax, &data_set);
        let (parsed, projected) = parse_on_stack(TWO_MIB, move || {
            let parsed = parse_part10(&file);
            let projected = parsed
                .as_ref()
                .is_ok_and(|parsed| MetadataSet::from_object(parsed.object()).is_ok());
            (parsed, projected)
        })?;
        assert!(
            parsed.is_ok(),
            "items at the limit were refused under {syntax}: {parsed:?}"
        );
        assert!(
            projected,
            "the projection failed on items at the limit under {syntax}"
        );
    }
    Ok(())
}

#[test]
fn the_repository_tid1500_output_parses() -> Result<(), String> {
    // The Part 10 bytes of the TID 1500 document packages/core writeTid1500
    // writes, captured on 2026-11-06 by running the built packages/core
    // dist built from the identical tid1500.ts source with one NUM root.
    // The document nests five Sequences deep: the root Content Sequence,
    // the Imaging Measurements Content Sequence, the Measurement Group
    // Content Sequence, the Measured Value Sequence and the Measurement
    // Units Code Sequence, so the repository's own writer output must stay
    // far inside the limit.
    let parsed = parse_part10(REPOSITORY_TID_1500)
        .map_err(|error| format!("the repository TID 1500 output was refused: {error}"))?;
    MetadataSet::from_object(parsed.object())
        .map_err(|_| "the repository TID 1500 output did not project".to_owned())?;
    Ok(())
}

proptest! {
    #[test]
    fn sequence_depth_is_refused_exactly_past_the_limit(
        levels in proptest::collection::vec(any::<(bool, bool)>(), 0..=MAX_SEQUENCE_DEPTH + 8),
        implicit in any::<bool>(),
    ) {
        // PS3.5 sections 7.5.1 and 7.5.2 let the encoder choose both length
        // forms per level, so every generated file is a valid data set and
        // the only reason for a refusal is the depth. The result is Ok
        // exactly at and under the limit, and SequenceTooDeep past it, on a
        // 2 MiB thread, without a panic.
        let depth = levels.len();
        let file = part10(
            if implicit { IMPLICIT_VR_LE } else { EXPLICIT_VR_LE },
            &nested_sequence_data_set(&levels, implicit),
        );
        let outcome = parse_on_stack(TWO_MIB, move || parse_part10(&file).map(|_| ()))
            .map_err(TestCaseError::fail)?;
        let expected = if depth <= MAX_SEQUENCE_DEPTH {
            Ok(())
        } else {
            Err(ParseError::SequenceTooDeep)
        };
        prop_assert_eq!(outcome, expected, "depth {}", depth);
    }
}

#[test]
fn f245_bounded_explicit_dataset_accepts_exact_length_and_refuses_length_minus_one() {
    let data_set = explicit_little_data_set();
    let decoded_len = data_set.len();
    let bytes = part10(EXPLICIT_VR_LE, &data_set);

    let exact = ParseLimits {
        decoded_dataset_max: decoded_len,
    };
    assert!(parse_part10_with_limits(&bytes, &exact).is_ok());

    let tighter = ParseLimits {
        decoded_dataset_max: decoded_len - 1,
    };
    let refused = parse_part10_with_limits(&bytes, &tighter).err();
    assert!(matches!(refused, Some(ParseError::SizeLimit)));

    assert!(parse_part10(&bytes).is_ok());
}

#[test]
fn f245_bounded_deflated_dataset_expands_to_explicit_length_and_enforces_cap() {
    let decoded_len = explicit_little_data_set().len();
    let bytes = part10(DEFLATED_EXPLICIT_VR_LE, DEFLATED_DATA_SET);
    assert_ne!(DEFLATED_DATA_SET.len(), decoded_len);

    let exact = ParseLimits {
        decoded_dataset_max: decoded_len,
    };
    assert!(parse_part10_with_limits(&bytes, &exact).is_ok());

    let tighter = ParseLimits {
        decoded_dataset_max: decoded_len - 1,
    };
    let refused = parse_part10_with_limits(&bytes, &tighter).err();
    assert!(matches!(refused, Some(ParseError::SizeLimit)));

    assert!(parse_part10(&bytes).is_ok());
}

#[test]
fn f245_bounded_deflated_dataset_with_trailing_suffix_is_invalid_regardless_of_cap() {
    let mut data_set = DEFLATED_DATA_SET.to_vec();
    data_set.extend_from_slice(&[1, 2]);
    let bytes = part10(DEFLATED_EXPLICIT_VR_LE, &data_set);

    let generous = ParseLimits {
        decoded_dataset_max: usize::MAX,
    };
    let refused = parse_part10_with_limits(&bytes, &generous).err();
    assert!(matches!(refused, Some(ParseError::InvalidDataSet)));

    assert_eq!(parse_part10(&bytes).err(), Some(ParseError::InvalidDataSet));
}
