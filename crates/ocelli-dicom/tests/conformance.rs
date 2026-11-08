//! F-225 conformance fixtures.
//!
//! Expected findings are hand-computed from DICOM PS3.5 2026d sections 6.2,
//! 6.4, 7.4 and 9.1, and from PS3.3 2026d tables C.7 and C.12. The bytes are
//! synthetic. A single odd value length cannot be parsed, because
//! `parse_part10` uses `OddLengthStrategy::Fail`. Where the acceptance text
//! shows an odd spelling, the stored bytes add the PS3.5 pad and the comment
//! says so.

use dicom_core::dictionary::{DataDictionary, DataDictionaryEntry, VirtualVr};
use dicom_object::StandardDataDictionary;
use ocelli_dicom::{
    CONFORMANCE_SCOPE, CharacterClass, ConformanceReport, ConformanceRule, Measure, ModuleScope,
    Tag, VmVr, check_conformance, open_frame, parse_part10, vm_entries,
};

const CT_UID: &str = "1.2.840.10008.5.1.4.1.1.2";
const CR_UID: &str = "1.2.840.10008.5.1.4.1.1.1";
const MR_UID: &str = "1.2.840.10008.5.1.4.1.1.4";
const PET_UID: &str = "1.2.840.10008.5.1.4.1.1.128";
const SC_UID: &str = "1.2.840.10008.5.1.4.1.1.7";
/// A well-formed UID that does not begin with the PS3.6 DICOM root.
const PRIVATE_SOP: &str = "1.2.3.4";
/// PS3.5 9.1 accepts one component of digits. These digits have the shape of a date.
const DATE_SHAPED_SOP: &str = "19620203";
/// Shares the characters of the PS3.6 root. The next component is 100081.
const ROOT_PREFIX_LOOKALIKE: &str = "1.2.840.100081.2";
/// The PS3.6 root alone. A value under the root has one more component.
const DICOM_ROOT_ONLY: &str = "1.2.840.10008";
/// PS3.5 6.2. 18 bytes, so the field is even before padding. Two trailing
/// NULLs are not the one pad byte. The components sit under the DICOM root.
const EVEN_ROOT_UID: &str = "1.2.840.10008.1.20";
/// ESC 02/05 04/07 under ISO 2022. Not in PS3.3 Tables C.12-3 or C.12-4.
const ESC_PERCENT_G: &str = "AB\u{001B}%G ";
/// ESC 02/08 04/03 under ISO 2022. Not in PS3.3 Tables C.12-3 or C.12-4.
const ESC_PAREN_C: &str = "AB\u{001B}(C ";
/// ESC 02/13 04/01 in a UR. Table C.12-3. UR is outside PS3.5 6.1.2.2.
const ESC_UR: &str = "http://a.test/\u{001B}-A ";

const SOP_CLASSES: &[&str] = &[CT_UID, CR_UID, MR_UID, PET_UID, SC_UID];

const META_SECRETS: &[&str] = &["2.25.1", "1.2.840.10008.1.2.1", "2.25.2"];

const NOT_DECIDABLE: &[&str] = &[
    "Laterality",
    "ContentDate",
    "ContentTime",
    "AnatomicalOrientationType",
    "PixelAspectRatio",
    "PixelPaddingRangeLimit",
    "PixelDataProviderURL",
    "PatientAlternativeCalendar",
    "PatientSpeciesDescription",
    "PatientSpeciesCodeSequence",
    "PatientBreedDescription",
    "PatientBreedCodeSequence",
    "BreedRegistrationSequence",
    "ResponsiblePerson",
    "ResponsiblePersonRole",
    "ResponsibleOrganization",
    "DeidentificationMethod",
    "DeidentificationMethodCodeSequence",
    "EncryptedAttributesSequence",
    "HL7StructuredDocumentReferenceSequence",
    "QueryRetrieveView",
    "ConversionSourceAttributesSequence",
];

const PLANE_MODULES: &[&str] = &[
    "Patient",
    "General Study",
    "General Series",
    "General Image",
    "Image Plane",
    "Image Pixel",
    "SOP Common",
];

const CR_MODULES: &[&str] = &[
    "Patient",
    "General Study",
    "General Series",
    "General Image",
    "Image Pixel",
    "SOP Common",
];

#[derive(Clone)]
struct El {
    tag: (u16, u16),
    vr: [u8; 2],
    value: Vec<u8>,
    secrets: Vec<String>,
}

struct Fixture {
    name: &'static str,
    sop: &'static str,
    elements: Vec<El>,
}

struct Want {
    tag: (u16, u16),
    keyword: Option<&'static str>,
    rule: ConformanceRule,
    value: Option<u32>,
    measure: Measure,
    path: &'static [(u16, u16, u32)],
}

fn require<T: std::fmt::Debug>(result: Result<T, impl std::fmt::Debug>) -> T {
    assert!(result.is_ok(), "{result:?}");
    let Ok(value) = result else {
        unreachable!("asserted immediately above")
    };
    value
}

fn is_long(vr: [u8; 2]) -> bool {
    matches!(
        &vr,
        b"OB"
            | b"OD"
            | b"OF"
            | b"OL"
            | b"OV"
            | b"OW"
            | b"SQ"
            | b"SV"
            | b"UC"
            | b"UN"
            | b"UR"
            | b"UT"
            | b"UV"
    )
}

fn secrets_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in text.split('\\') {
        let trimmed = part.trim_matches(|ch: char| ch == ' ' || ch == '\0');
        if trimmed.chars().count() >= 3 {
            out.push(trimmed.to_owned());
        }
    }
    out
}

fn el(tag: (u16, u16), vr: [u8; 2], value: Vec<u8>, secrets: Vec<String>) -> El {
    assert!(
        value.len().is_multiple_of(2),
        "odd value length on ({:04X},{:04X})",
        tag.0,
        tag.1
    );
    El {
        tag,
        vr,
        value,
        secrets,
    }
}

fn text(tag: (u16, u16), vr: [u8; 2], text: &str) -> El {
    el(tag, vr, text.as_bytes().to_vec(), secrets_in(text))
}

fn ui(tag: (u16, u16), text: &str) -> El {
    let mut bytes = text.as_bytes().to_vec();
    if !bytes.len().is_multiple_of(2) {
        bytes.push(0);
    }
    let mut owned = text.to_owned();
    if !owned.len().is_multiple_of(2) {
        owned.push('\0');
    }
    el(tag, *b"UI", bytes, secrets_in(&owned))
}

/// PS3.5 6.2. `value` is the Value Field, NULL bytes included. Length is even.
fn ui_field(tag: (u16, u16), value: &str) -> El {
    assert!(
        value.len().is_multiple_of(2),
        "UI value field length {} is odd",
        value.len()
    );
    el(tag, *b"UI", value.as_bytes().to_vec(), secrets_in(value))
}

/// PS3.5 6.2. SC UID, one NULL, a backslash, then 19620203 and one NULL.
/// The first NULL is not the Value Field pad.
fn sop_null_before_delimiter() -> El {
    let mut value = String::new();
    value.push_str(SC_UID);
    value.push('\0');
    value.push('\\');
    value.push_str(DATE_SHAPED_SOP);
    value.push('\0');
    assert_eq!(SC_UID.len(), 25);
    assert_eq!(value.len(), 36);
    ui_field((0x0008, 0x0016), &value)
}

/// PS3.5 6.2. CT UID, one NULL, a backslash, then 1.2.3.4 with no pad.
/// The NULL is on the first value, so it is not the Value Field pad and the
/// first value is not the CT Image Storage UID.
fn ct_null_before_delimiter() -> El {
    let mut value = String::new();
    value.push_str(CT_UID);
    value.push('\0');
    value.push('\\');
    value.push_str(PRIVATE_SOP);
    assert_eq!(CT_UID.len(), 25);
    assert_eq!(value.len(), 34);
    ui_field((0x0008, 0x0016), &value)
}

/// PS3.5 6.4. CT UID, two backslashes and the single NULL pad. 28 bytes and
/// three values, the second and third empty. The NULL pads the last value,
/// not the first, so value 1 is the CT Image Storage UID.
fn ct_then_two_empty_values() -> El {
    let mut value = String::new();
    value.push_str(CT_UID);
    value.push('\\');
    value.push('\\');
    value.push('\0');
    assert_eq!(CT_UID.len(), 25);
    assert_eq!(value.len(), 28);
    ui_field((0x0008, 0x0016), &value)
}

/// PS3.5 6.4. One backslash and then the CT UID. 26 bytes and two values,
/// the first empty. Value 1 is empty, so the IOD is not evaluated and the
/// scope repeats no UID.
fn empty_then_ct() -> El {
    let mut value = String::new();
    value.push('\\');
    value.push_str(CT_UID);
    assert_eq!(value.len(), 26);
    ui_field((0x0008, 0x0016), &value)
}

/// PS3.5 6.2. `nulls` trailing NULL bytes, with no extra pad added.
fn ui_nulls(tag: (u16, u16), text: &str, nulls: usize) -> El {
    let mut bytes = text.as_bytes().to_vec();
    bytes.extend(std::iter::repeat_n(0, nulls));
    assert!(
        bytes.len().is_multiple_of(2),
        "UI length {} plus {nulls} NULL bytes is odd",
        text.len()
    );
    el(tag, *b"UI", bytes, secrets_in(text))
}

fn latin1(tag: (u16, u16), vr: [u8; 2], bytes: &[u8]) -> El {
    let text: String = bytes.iter().copied().map(char::from).collect();
    el(tag, vr, bytes.to_vec(), secrets_in(&text))
}

fn us(tag: (u16, u16), value: u16) -> El {
    el(tag, *b"US", value.to_le_bytes().to_vec(), Vec::new())
}

fn blank(tag: (u16, u16), vr: [u8; 2]) -> El {
    el(tag, vr, Vec::new(), Vec::new())
}

fn sequence(tag: (u16, u16), items: &[Vec<El>]) -> El {
    let mut secrets = Vec::new();
    let mut value = Vec::new();
    for item_elements in items {
        let mut ordered = item_elements.clone();
        ordered.sort_by_key(|item| item.tag);
        let mut body = Vec::new();
        for item in &ordered {
            body.extend(encode_element(item));
            secrets.extend(item.secrets.clone());
        }
        value.extend(item_bytes(&body));
    }
    el(tag, *b"SQ", value, secrets)
}

fn ct_elements() -> Vec<El> {
    vec![
        ui((0x0008, 0x0016), CT_UID),
        ui((0x0008, 0x0018), "2.25.1"),
        text((0x0008, 0x0020), *b"DA", "19991231"),
        text((0x0008, 0x0030), *b"TM", "153045"),
        text((0x0008, 0x0050), *b"SH", "A1"),
        text((0x0008, 0x0060), *b"CS", "CT"),
        text((0x0008, 0x0090), *b"PN", "N "),
        text((0x0010, 0x0010), *b"PN", "N "),
        text((0x0010, 0x0020), *b"LO", "P1"),
        text((0x0010, 0x0030), *b"DA", "19991231"),
        text((0x0010, 0x0040), *b"CS", "O "),
        text((0x0018, 0x0050), *b"DS", "5 "),
        text((0x0018, 0x5100), *b"CS", "HFS "),
        ui((0x0020, 0x000D), "9.8.7"),
        ui((0x0020, 0x000E), "9.8.8"),
        text((0x0020, 0x0010), *b"SH", "S1"),
        text((0x0020, 0x0011), *b"IS", "1 "),
        text((0x0020, 0x0013), *b"IS", "1 "),
        text((0x0020, 0x0032), *b"DS", "0\\0\\0 "),
        text((0x0020, 0x0037), *b"DS", "1\\0\\0\\0\\1\\0 "),
        us((0x0028, 0x0002), 1),
        text((0x0028, 0x0004), *b"CS", "MONOCHROME2 "),
        us((0x0028, 0x0010), 1),
        us((0x0028, 0x0011), 1),
        text((0x0028, 0x0030), *b"DS", "1\\1 "),
        us((0x0028, 0x0100), 16),
        us((0x0028, 0x0101), 16),
        us((0x0028, 0x0102), 15),
        us((0x0028, 0x0103), 0),
        el((0x7FE0, 0x0010), *b"OW", vec![0, 0], Vec::new()),
    ]
}

fn with_sop(mut elements: Vec<El>, sop: &str) -> Vec<El> {
    elements.retain(|item| item.tag != (0x0008, 0x0016));
    elements.push(ui((0x0008, 0x0016), sop));
    elements
}

fn without(mut elements: Vec<El>, tag: (u16, u16)) -> Vec<El> {
    elements.retain(|item| item.tag != tag);
    elements
}

fn cr_elements() -> Vec<El> {
    let mut elements = ct_elements();
    for tag in [
        (0x0018, 0x0050),
        (0x0018, 0x5100),
        (0x0020, 0x0032),
        (0x0020, 0x0037),
        (0x0028, 0x0030),
    ] {
        elements = without(elements, tag);
    }
    elements = with_sop(elements, CR_UID);
    elements.push(text((0x0020, 0x0020), *b"CS", "L\\PR"));
    elements
}

fn pet_elements() -> Vec<El> {
    with_sop(without(ct_elements(), (0x0018, 0x5100)), PET_UID)
}

fn mr_elements() -> Vec<El> {
    with_sop(ct_elements(), MR_UID)
}

fn changed(mut elements: Vec<El>, extra: El) -> Vec<El> {
    elements.retain(|item| item.tag != extra.tag);
    elements.push(extra);
    elements
}

fn fix(name: &'static str, sop: &'static str, elements: Vec<El>) -> Fixture {
    Fixture {
        name,
        sop,
        elements,
    }
}

fn ct(name: &'static str, extra: El) -> Fixture {
    fix(name, CT_UID, changed(ct_elements(), extra))
}

fn ct_two(name: &'static str, first: El, second: El) -> Fixture {
    fix(name, CT_UID, changed(changed(ct_elements(), first), second))
}

fn age(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0010, 0x1010), *b"AS", value))
}

fn institution(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0008, 0x0080), *b"LO", value))
}

fn study_date(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0008, 0x0020), *b"DA", value))
}

fn study_time(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0008, 0x0030), *b"TM", value))
}

fn series_number(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0020, 0x0011), *b"IS", value))
}

fn slice(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0018, 0x0050), *b"DS", value))
}

fn when(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0008, 0x002A), *b"DT", value))
}

fn frame_uid(name: &'static str, value: &str) -> Fixture {
    ct(name, text((0x0020, 0x0052), *b"UI", value))
}

fn overlay(base: Vec<El>, extras: Vec<El>) -> Vec<El> {
    let mut elements = base;
    for extra in extras {
        elements = changed(elements, extra);
    }
    elements
}

fn words(tag: (u16, u16), vr: [u8; 2], values: &[u16]) -> El {
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    el(tag, vr, bytes, Vec::new())
}

fn quads(tag: (u16, u16), vr: [u8; 2], values: &[u64]) -> El {
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    el(tag, vr, bytes, Vec::new())
}

/// PS3.5 6.4. Six 16-byte DS values need one pad byte on the last value.
fn padded_orientation() -> String {
    let values = [
        "1.00000000000000",
        "0.00000000000000",
        "0.00000000000000",
        "0.00000000000000",
        "1.00000000000000",
        "0.00000000000000",
    ];
    let mut field = String::new();
    for (index, value) in values.iter().enumerate() {
        assert_eq!(value.len(), 16);
        if index > 0 {
            field.push('\\');
        }
        field.push_str(value);
    }
    assert_eq!(field.len(), 101);
    field.push(' ');
    assert_eq!(field.len(), 102);
    field
}

/// PS3.5 6.4. A last CS value two bytes over 16, ending in the pad.
fn cs_two_past_the_pad() -> String {
    let last = "ABCDEFGHIJKLMNOP  ";
    assert_eq!(last.len(), 18);
    assert!(last.ends_with(' '));
    let field = format!("ORIGINAL\\PRIMARYX\\{last}");
    assert_eq!(field.len(), 36);
    field
}

/// PS3.5 6.2 and 9.1, duplicated here so the privacy scan does not trust the checker.
/// One trailing NULL is the pad. A second NULL is not.
fn ps35_uid(text: &str) -> bool {
    let text = text.strip_suffix('\0').unwrap_or(text);
    if text.is_empty() || text.contains(' ') || text.contains('\0') {
        return false;
    }
    if text.starts_with('.') || text.ends_with('.') {
        return false;
    }
    text.split('.').all(|part| {
        !part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && !(part.len() > 1 && part.starts_with('0'))
    })
}

fn fixtures() -> Vec<Fixture> {
    let mut all = vec![
        fix("valid-ct", CT_UID, ct_elements()),
        age("age-040", "040 "),
        age("age-018M", "018M"),
        age("age-000D", "000D"),
        age("age-120Y", "120Y"),
        age("age-40Y", "40Y "),
        age("age-0040", "0040"),
        institution("lo-170", &"A".repeat(170)),
        institution("lo-64", &"A".repeat(64)),
        institution("lo-63-pad", &format!("{} ", "A".repeat(63))),
        institution("lo-66", &format!("{} ", "A".repeat(65))),
        ct_two(
            "lo-65-utf8",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 192"),
            text(
                (0x0008, 0x0080),
                *b"LO",
                &format!("{}{}\u{00E9} ", "A".repeat(63), ""),
            ),
        ),
        ct_two(
            "lo-64-utf8",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 192"),
            text((0x0008, 0x0080), *b"LO", &"\u{00E9}".repeat(64)),
        ),
        ct(
            "cs-pad-last",
            text(
                (0x0008, 0x0008),
                *b"CS",
                "ORIGINAL\\PRIMARY\\ABCDEFGHIJKLMNOP ",
            ),
        ),
        ct(
            "cs-pad-first",
            text((0x0008, 0x0008), *b"CS", "ABCDEFGHIJKLMNOP \\AB"),
        ),
        ct(
            "cs-pad-two",
            text((0x0008, 0x0008), *b"CS", &cs_two_past_the_pad()),
        ),
        ct(
            "da-pad-last",
            text((0x0018, 0x1200), *b"DA", "20240101\\20240102 "),
        ),
        ct(
            "ds-iop-pad",
            text((0x0020, 0x0037), *b"DS", &padded_orientation()),
        ),
        fix(
            "sop-secret",
            CT_UID,
            changed(ct_elements(), ui((0x0008, 0x0016), "SECRET^NAME")),
        ),
        fix(
            "sop-private",
            PRIVATE_SOP,
            changed(ct_elements(), ui((0x0008, 0x0016), PRIVATE_SOP)),
        ),
        fix(
            "sop-date",
            DATE_SHAPED_SOP,
            changed(ct_elements(), ui((0x0008, 0x0016), DATE_SHAPED_SOP)),
        ),
        fix(
            "sop-root-prefix",
            ROOT_PREFIX_LOOKALIKE,
            vec![ui((0x0008, 0x0016), ROOT_PREFIX_LOOKALIKE)],
        ),
        fix(
            "sop-root-bare",
            DICOM_ROOT_ONLY,
            vec![ui((0x0008, 0x0016), DICOM_ROOT_ONLY)],
        ),
        fix("sop-one-null", SC_UID, vec![ui((0x0008, 0x0016), SC_UID)]),
        fix(
            "sop-three-null",
            SC_UID,
            vec![ui_nulls((0x0008, 0x0016), SC_UID, 3)],
        ),
        fix(
            "sop-ct-three-null",
            CT_UID,
            vec![ui_nulls((0x0008, 0x0016), CT_UID, 3)],
        ),
        fix(
            "sop-null-then-date",
            SC_UID,
            vec![sop_null_before_delimiter()],
        ),
        fix(
            "sop-even-two-null",
            EVEN_ROOT_UID,
            vec![ui_nulls((0x0008, 0x0016), EVEN_ROOT_UID, 2)],
        ),
        ct("ui-null-mid", ui_field((0x0008, 0x001A), "1.2.3\0\\1.2.5")),
        ct("ui-pad-last", ui((0x0008, 0x001A), "1.2.3\\1.2.5")),
        fix(
            "sop-ct-null-then-uid",
            CT_UID,
            vec![ct_null_before_delimiter()],
        ),
        fix(
            "sop-ct-then-two-empty",
            CT_UID,
            changed(ct_elements(), ct_then_two_empty_values()),
        ),
        fix("sop-empty-then-ct", CT_UID, vec![empty_then_ct()]),
        ct(
            "ui-empty-last-pad",
            ui_field((0x0008, 0x001A), "1.2.34\\\0"),
        ),
        ct(
            "da-empty-mid",
            text((0x0018, 0x1200), *b"DA", "20240101\\\\20240102"),
        ),
        ct("ds-empty-mid", text((0x0028, 0x1050), *b"DS", "40\\\\50")),
        ct("vm-empty-counted", text((0x0008, 0x0060), *b"CS", "CT\\ ")),
        ct_two(
            "type1-delim-pad",
            ui_field((0x0008, 0x0018), "\\\0"),
            text((0x0008, 0x0060), *b"CS", "\\ "),
        ),
        ct_two(
            "lo-esc-100",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 100"),
            text((0x0008, 0x0080), *b"LO", "ABC\u{001B}"),
        ),
        ct_two(
            "lo-esc-192",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 192"),
            text((0x0008, 0x0080), *b"LO", "ABC\u{001B}"),
        ),
        ct(
            "lo-esc-absent",
            text((0x0008, 0x0080), *b"LO", "ABC\u{001B}"),
        ),
        ct_two(
            "lo-esc-2022",
            text((0x0008, 0x0005), *b"CS", "\\ISO 2022 IR 100"),
            text((0x0008, 0x0080), *b"LO", "A\u{001B}-A"),
        ),
        ct_two(
            "lo-esc-2022-bare",
            text((0x0008, 0x0005), *b"CS", "\\ISO 2022 IR 100"),
            text((0x0008, 0x0080), *b"LO", "ABC\u{001B}"),
        ),
        ct_two(
            "lo-esc-percent-g",
            text((0x0008, 0x0005), *b"CS", "\\ISO 2022 IR 100"),
            text((0x0008, 0x0080), *b"LO", ESC_PERCENT_G),
        ),
        ct_two(
            "lo-esc-paren-c",
            text((0x0008, 0x0005), *b"CS", "\\ISO 2022 IR 100"),
            text((0x0008, 0x0080), *b"LO", ESC_PAREN_C),
        ),
        ct_two(
            "ur-esc-2022",
            text((0x0008, 0x0005), *b"CS", "\\ISO 2022 IR 100"),
            text((0x0028, 0x7FE0), *b"UR", ESC_UR),
        ),
        ct_two(
            "ds-utf8",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 192"),
            text(
                (0x0018, 0x0050),
                *b"DS",
                &format!("{}{}", "1".repeat(14), "\u{00E9}\u{00E9}"),
            ),
        ),
        ct(
            "ds-latin1",
            latin1((0x0018, 0x0050), *b"DS", &{
                let mut bytes = vec![b'1'; 14];
                bytes.extend_from_slice(&[0xE9, 0xE9]);
                bytes
            }),
        ),
        fix(
            "native-vm",
            CT_UID,
            overlay(
                ct_elements(),
                vec![
                    us((0x0028, 0x0010), 4),
                    us((0x0028, 0x0011), 4),
                    el((0x7FE0, 0x0010), *b"OW", vec![0; 32], Vec::new()),
                    words((0x0028, 0x1201), *b"OW", &[0, 1, 2, 3]),
                    quads((0x7FE0, 0x0001), *b"OV", &[0, 32]),
                    quads((0x7FE0, 0x0002), *b"OV", &[32, 32]),
                ],
            ),
        ),
        ct_two(
            "type1-empty",
            blank((0x0008, 0x0018), *b"UI"),
            text((0x0008, 0x0060), *b"CS", "\\\\"),
        ),
        fix(
            "rows-absent",
            CT_UID,
            without(ct_elements(), (0x0028, 0x0010)),
        ),
        fix(
            "name-absent",
            CT_UID,
            without(ct_elements(), (0x0010, 0x0010)),
        ),
        ct("name-empty", blank((0x0010, 0x0010), *b"PN")),
        slice("ds-space", " 12.5 "),
        slice("ds-exponent", "+1.5E-3 "),
        slice("ds-comma", "1,5 "),
        slice("ds-gap", "1 5 "),
        ct(
            "ds-17",
            text((0x0018, 0x1164), *b"DS", "12345678901234567\\12"),
        ),
        slice("ds-range", "1E999 "),
        series_number("is-min", "-2147483648 "),
        series_number("is-max", "2147483647"),
        series_number("is-over", "2147483648"),
        series_number("is-bad", "12a "),
        ct("is-13", text((0x0019, 0x1002), *b"IS", "1234567890123\\12")),
        series_number("is-padded", " +7 "),
        study_date("da-2024", "20240229"),
        study_date("da-2023", "20230229"),
        study_date("da-june", "20260230"),
        study_date("da-1900", "19000229"),
        study_date("da-2000", "20000229"),
        study_date("da-dotted", "2026.11.05"),
        study_time("tm-frac", "070907.0705 "),
        study_time("tm-1010", "1010"),
        study_time("tm-021", "021 "),
        study_time("tm-2400", "2400"),
        study_time("tm-leap", "235960"),
        study_time("tm-1260", "1260"),
        study_time("tm-dot", "12.5"),
        study_time("tm-empty-frac", "120000. "),
        study_time("tm-long-frac", "120000.1234567"),
        when("dt-195308", "195308"),
        when("dt-full", "19530827111300.0"),
        when("dt-offset", "2007-0500 "),
        when("dt-negzero", "20070101-0000 "),
        when("dt-1500", "20070101+1500 "),
        when("dt-1300", "20070101-1300 "),
        when("dt-1400", "20070101+1400 "),
        when("dt-short", "20070 "),
        frame_uid("ui-lead", "1.02.3"),
        frame_uid("ui-empty", "1..2"),
        frame_uid("ui-letter", "1.2.a "),
        ct("ui-zero", ui((0x0020, 0x0052), "0.1")),
        ct(
            "ui-long",
            text((0x0019, 0x1001), *b"UI", &format!("{}\\12", "1".repeat(65))),
        ),
        ct("ui-null", ui((0x0020, 0x0052), "1.2.3")),
        frame_uid("ui-space", "1.2.3 "),
        ct("cs-lower", text((0x0008, 0x0060), *b"CS", "ct")),
        ct("cs-token", text((0x0008, 0x0060), *b"CS", "CT_A")),
        ct(
            "cs-17",
            text((0x0008, 0x0008), *b"CS", "ABCDEFGHIJKLMNOPQ\\AB"),
        ),
        ct(
            "ae-spaces",
            text((0x0008, 0x0054), *b"AE", "                "),
        ),
        ct("pn-ok", text((0x0010, 0x0010), *b"PN", "A^B^C^D^E ")),
        ct("pn-carets", text((0x0010, 0x0010), *b"PN", "A^B^C^D^E^F ")),
        ct("pn-groups", text((0x0010, 0x0010), *b"PN", "A=B=C ")),
        ct("pn-groups-bad", text((0x0010, 0x0010), *b"PN", "A=B=C=D ")),
        ct(
            "pn-65",
            text((0x0010, 0x0010), *b"PN", &format!("{}=BB", "A".repeat(65))),
        ),
        ct(
            "pn-split",
            text(
                (0x0010, 0x0010),
                *b"PN",
                &format!("{}={}", "A".repeat(64), "B".repeat(63)),
            ),
        ),
        ct("lo-tab", text((0x0008, 0x0080), *b"LO", "ABC\t")),
        ct("lt-ok", text((0x0010, 0x21B0), *b"LT", "A\t\r\n")),
        ct("st-bel", text((0x0008, 0x2111), *b"ST", "A\u{0007}")),
        ct("lo-e9", latin1((0x0008, 0x0080), *b"LO", &[0xE9, b'Z'])),
        ct_two(
            "lo-e9-100",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 100"),
            latin1((0x0008, 0x0080), *b"LO", &[0xE9, b'Z']),
        ),
        ct("cs-e9", latin1((0x0008, 0x0060), *b"CS", &[0xE9, b' '])),
        ct_two(
            "cs-e9-100",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 100"),
            latin1((0x0008, 0x0060), *b"CS", &[0xE9, b' ']),
        ),
        ct("vm-spacing", text((0x0028, 0x0030), *b"DS", "1.5 ")),
        ct(
            "vm-orient",
            text((0x0020, 0x0037), *b"DS", "0\\0\\0\\0\\1 "),
        ),
        ct("vm-id", text((0x0010, 0x0020), *b"LO", "ABC\\DE")),
        ct(
            "vm-charset",
            text((0x0008, 0x0005), *b"CS", "ISO_IR 100\\ISO_IR 100\\ISO_IR 6"),
        ),
        ct("vr-lo", text((0x0010, 0x1010), *b"LO", "018M")),
        ct(
            "vr-un",
            el(
                (0x0010, 0x1010),
                *b"UN",
                b"018M".to_vec(),
                secrets_in("018M"),
            ),
        ),
        ct("spp-3", us((0x0028, 0x0002), 3)),
        ct("palette", text((0x0028, 0x0004), *b"CS", "PALETTE COLOR ")),
        fix(
            "pixels-absent",
            CT_UID,
            without(ct_elements(), (0x7FE0, 0x0010)),
        ),
        fix(
            "pixels-url",
            CT_UID,
            changed(
                without(ct_elements(), (0x7FE0, 0x0010)),
                text((0x0028, 0x7FE0), *b"UR", "https://example.test/a"),
            ),
        ),
        fix(
            "pixels-blank-url",
            CT_UID,
            changed(
                without(ct_elements(), (0x7FE0, 0x0010)),
                blank((0x0028, 0x7FE0), *b"UR"),
            ),
        ),
        ct("offset-only", blank((0x7FE0, 0x0001), *b"OV")),
        fix(
            "position-absent",
            CT_UID,
            without(ct_elements(), (0x0018, 0x5100)),
        ),
        fix(
            "position-code",
            CT_UID,
            changed(
                without(ct_elements(), (0x0018, 0x5100)),
                blank((0x0054, 0x0410), *b"SQ"),
            ),
        ),
        fix("cr-base", CR_UID, cr_elements()),
        fix(
            "cr-orient",
            CR_UID,
            without(cr_elements(), (0x0020, 0x0020)),
        ),
        fix("mr-base", MR_UID, mr_elements()),
        fix(
            "mr-position",
            MR_UID,
            without(mr_elements(), (0x0018, 0x5100)),
        ),
        fix("pet-base", PET_UID, pet_elements()),
        fix(
            "sc-rows",
            SC_UID,
            vec![
                ui((0x0008, 0x0016), SC_UID),
                text((0x0010, 0x1010), *b"AS", "040 "),
            ],
        ),
        ct(
            "nested",
            sequence(
                (0x0054, 0x0410),
                &[Vec::new(), vec![text((0x0008, 0x0100), *b"SH", "ABC\t")]],
            ),
        ),
        fix(
            "reproducer",
            CT_UID,
            changed(
                changed(
                    changed(
                        changed(
                            changed(
                                ct_elements(),
                                text((0x0010, 0x0010), *b"PN", "SECRET^NAME "),
                            ),
                            text((0x0010, 0x1010), *b"AS", "040 "),
                        ),
                        text((0x0008, 0x0080), *b"LO", &"A".repeat(170)),
                    ),
                    blank((0x0008, 0x0018), *b"UI"),
                ),
                blank((0x0008, 0x0060), *b"CS"),
            ),
        ),
    ];
    let order = changed(
        changed(ct_elements(), text((0x0010, 0x1010), *b"DA", "040 ")),
        sequence(
            (0x0054, 0x0410),
            &[Vec::new(), vec![text((0x0008, 0x0100), *b"SH", "ABC\t")]],
        ),
    );
    all.push(fix("order", CT_UID, order));
    all
}

fn encode_element(el: &El) -> Vec<u8> {
    if is_long(el.vr) {
        require(long_le(el.tag, el.vr, &el.value))
    } else {
        require(short_le(el.tag, el.vr, &el.value))
    }
}

fn encode(fixture: &Fixture) -> Vec<u8> {
    let mut elements = fixture.elements.clone();
    elements.sort_by_key(|item| item.tag);
    let mut data = Vec::new();
    for item in &elements {
        data.extend(encode_element(item));
    }
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(require(file_meta(fixture.sop)));
    bytes.extend(data);
    bytes
}

fn file_meta(sop: &str) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(short_le((0x0002, 0x0002), *b"UI", &ui_bytes(sop))?);
    body.extend(short_le((0x0002, 0x0003), *b"UI", &ui_bytes("2.25.1"))?);
    body.extend(short_le(
        (0x0002, 0x0010),
        *b"UI",
        &ui_bytes("1.2.840.10008.1.2.1"),
    )?);
    body.extend(short_le((0x0002, 0x0012), *b"UI", &ui_bytes("2.25.2"))?);
    let length = u32::try_from(body.len()).map_err(|_| "meta length".to_owned())?;
    let mut meta = short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes())?;
    meta.extend(body);
    Ok(meta)
}

fn ui_bytes(text: &str) -> Vec<u8> {
    let mut value = text.as_bytes().to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
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

fn item_bytes(content: &[u8]) -> Vec<u8> {
    let length = u32::try_from(content.len()).unwrap_or(u32::MAX);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(content);
    bytes
}

fn load(name: &str) -> ConformanceReport {
    let all = fixtures();
    let fixture = all.iter().find(|item| item.name == name);
    assert!(fixture.is_some(), "missing fixture {name}");
    let Some(fixture) = fixture else {
        unreachable!("asserted immediately above")
    };
    let parsed = require(parse_part10(&encode(fixture)));
    check_conformance(&parsed)
}

fn describe(report: &ConformanceReport) -> String {
    let mut lines = Vec::new();
    for finding in report.findings() {
        lines.push(format!(
            "({},{:04X}) {} {:?} {:?}",
            finding.tag.0,
            finding.tag.1,
            finding.rule.id(),
            finding.value,
            finding.measure
        ));
    }
    lines.join(" | ")
}

fn assert_only(report: &ConformanceReport, expected: &[Want]) {
    let expected_ids = expected
        .iter()
        .map(|want| want.rule.id())
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(
        report.findings().len(),
        expected.len(),
        "missing {expected_ids}, findings were {}",
        describe(report)
    );
    for (found, want) in report.findings().iter().zip(expected.iter()) {
        assert_eq!(
            found.tag,
            Tag(want.tag.0, want.tag.1),
            "{}",
            describe(report)
        );
        assert_eq!(found.keyword, want.keyword);
        assert_eq!(found.rule, want.rule);
        assert_eq!(found.value, want.value);
        assert_eq!(found.measure, want.measure);
        assert_eq!(found.path.len(), want.path.len());
        for (step, expected_step) in found.path.iter().zip(want.path.iter()) {
            assert_eq!(step.sequence, Tag(expected_step.0, expected_step.1));
            assert_eq!(step.item, expected_step.2);
        }
    }
}

fn one(
    tag: (u16, u16),
    keyword: &'static str,
    rule: ConformanceRule,
    value: Option<u32>,
    measure: Measure,
) -> Want {
    Want {
        tag,
        keyword: Some(keyword),
        rule,
        value,
        measure,
        path: &[],
    }
}

fn none_kw(tag: (u16, u16), rule: ConformanceRule, value: Option<u32>, measure: Measure) -> Want {
    Want {
        tag,
        keyword: None,
        rule,
        value,
        measure,
        path: &[],
    }
}

fn chars(measured: u32, limit: u32) -> Measure {
    Measure::Characters { measured, limit }
}

fn bytes(measured: u32, limit: u32) -> Measure {
    Measure::Bytes { measured, limit }
}

fn values(measured: u32, minimum: u32, maximum: Option<u32>) -> Measure {
    Measure::Values {
        measured,
        minimum,
        maximum,
    }
}

fn character(index: u32, class: CharacterClass) -> Measure {
    Measure::Character { index, class }
}

fn assert_clear(name: &str) {
    let report = load(name);
    assert!(
        report.findings().is_empty(),
        "{} {}",
        name,
        describe(&report)
    );
}

fn evaluated(report: &ConformanceReport) -> (&str, &str, &str, &[&str]) {
    assert!(
        matches!(report.modules(), ModuleScope::Evaluated { .. }),
        "expected an evaluated scope"
    );
    let ModuleScope::Evaluated {
        sop_class,
        iod,
        table,
        modules,
    } = report.modules()
    else {
        unreachable!("asserted immediately above")
    };
    (*sop_class, *iod, *table, *modules)
}

#[test]
fn fixture_valid_ct_has_no_findings() {
    // PS3.3 A.3.3. The seven modules are mandatory for CT Image Storage.
    // PS3.5 7.4.3. A Type 2 element that is present, including padding, is
    // not a finding.
    let report = load("valid-ct");
    assert!(report.findings().is_empty(), "{}", describe(&report));
    let needle = format!("\"sopClass\":\"{CT_UID}\",\"sopClassNote\":null");
    assert!(report.to_json().contains(&needle), "{}", report.to_json());
    let (sop, iod, table, modules) = evaluated(&report);
    assert_eq!(sop, CT_UID);
    assert_eq!(iod, "CT Image");
    assert_eq!(table, "PS3.3 A.3.3");
    assert_eq!(modules, PLANE_MODULES);
    assert_eq!(ct_elements().len(), 30);
    assert_eq!(report.elements(), 30);
    assert_eq!(report.not_checked_un(), 0);
    assert_eq!(report.not_checked(), NOT_DECIDABLE);
}

#[test]
fn fixture_age_string() {
    // PS3.5 Table 6.2-1. AS is exactly four bytes: three digits and D, W, M
    // or Y. A trailing space is not a unit. One value yields one finding.
    assert_only(
        &load("age-040"),
        &[one(
            (0x0010, 0x1010),
            "PatientAge",
            ConformanceRule::AsFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_clear("age-018M");
    assert_clear("age-000D");
    assert_clear("age-120Y");
    assert_only(
        &load("age-40Y"),
        &[one(
            (0x0010, 0x1010),
            "PatientAge",
            ConformanceRule::AsFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("age-0040"),
        &[one(
            (0x0010, 0x1010),
            "PatientAge",
            ConformanceRule::AsFormat,
            Some(1),
            Measure::None,
        )],
    );
}

#[test]
fn fixture_long_string_counts_characters() {
    // PS3.5 6.2 Note. LO is 64 characters, not 64 bytes. PS3.5 6.4 allows one
    // padding byte only on the last value of a multi-valued element. A single
    // value is held to 64 exactly. 65 ASCII A plus a space is 66 and even.
    // 64 ASCII A plus a space is 65 and odd, so the single-value pad case is
    // 63 A, one U+00E9, and a space under ISO_IR 192 (65 characters, 66 bytes).
    assert_only(
        &load("lo-170"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::ValueLength,
            Some(1),
            chars(170, 64),
        )],
    );
    assert_clear("lo-64");
    assert_clear("lo-63-pad");
    assert_only(
        &load("lo-66"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::ValueLength,
            Some(1),
            chars(66, 64),
        )],
    );
    assert_only(
        &load("lo-65-utf8"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::ValueLength,
            Some(1),
            chars(65, 64),
        )],
    );
    assert_clear("lo-64-utf8");
}

#[test]
fn fixture_last_value_may_carry_the_pad() {
    // PS3.5 6.4. ORIGINAL\\PRIMARY\\ plus 16 characters is 33 bytes. The even
    // pad makes the third value 17 bytes ending in SPACE, and that last value
    // is allowed. The same 17 bytes in the first value are not.
    assert_clear("cs-pad-last");
    assert_only(
        &load("cs-pad-first"),
        &[one(
            (0x0008, 0x0008),
            "ImageType",
            ConformanceRule::ValueLength,
            Some(1),
            bytes(17, 16),
        )],
    );
}

#[test]
fn fixture_binary_values_have_multiplicity_one() {
    // PS3.5 6.4. OB, OD, OF, OL, OV, OW and UN have VM 1. A 4 by 4
    // 16-bit Pixel Data value is one OW value, not 16 words. Palette LUT
    // data and an Extended Offset Table of two offsets are each one value.
    // PS3.3 C.7-11a requires the lengths when the table is present.
    let report = load("native-vm");
    assert!(
        report
            .findings()
            .iter()
            .all(|finding| finding.rule != ConformanceRule::Multiplicity),
        "{}",
        describe(&report)
    );
    assert!(report.findings().is_empty(), "{}", describe(&report));
}

#[test]
fn fixture_padded_da_and_ds_are_clean() {
    // PS3.5 6.4. The one pad byte on the last value is not part of the
    // format. DA is YYYYMMDD. Both dates are legal Gregorian days.
    // Image Orientation (Patient) is six DS values of 16 bytes plus the pad.
    assert_clear("da-pad-last");
    assert_clear("ds-iop-pad");
}

#[test]
fn fixture_sop_class_is_repeated_only_under_the_dicom_root() {
    // PS3.5 9.1 and PS3.6. A SOP Class UID is repeated only when it is a
    // well-formed UID whose components begin with 1, 2, 840, 10008 and one
    // more component follows. SECRET^NAME is not a UID. 1.2.3.4 is a private
    // UID. 19620203 is one component of digits with the shape of a date.
    // 1.2.840.100081.2 shares the root's characters and its next component is
    // 100081. 1.2.840.10008 is the root alone. None of these is repeated, and
    // the note names the case. The stored text is still a UI-FORMAT finding
    // when it is not a UID.
    assert_eq!(
        ocelli_dicom::SOP_CLASS_NOTES,
        &["not a valid UID", "not a DICOM UID"][..]
    );
    let secret = load("sop-secret");
    let secret_json = secret.to_json();
    assert!(!secret_json.contains("SECRET^NAME"), "{secret_json}");
    assert!(
        secret_json.contains("\"sopClass\":null,\"sopClassNote\":\"not a valid UID\""),
        "{secret_json}"
    );
    assert_only(
        &secret,
        &[one(
            (0x0008, 0x0016),
            "SOPClassUID",
            ConformanceRule::UiFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_eq!(ROOT_PREFIX_LOOKALIKE.len(), 16);
    assert!(!ROOT_PREFIX_LOOKALIKE.starts_with("1.2.840.10008."));
    assert!(ROOT_PREFIX_LOOKALIKE.starts_with("1.2.840.10008"));
    assert_eq!(DICOM_ROOT_ONLY.len(), 13);
    assert!(ps35_uid(ROOT_PREFIX_LOOKALIKE));
    assert!(ps35_uid(DICOM_ROOT_ONLY));
    for (name, stored) in [
        ("sop-private", PRIVATE_SOP),
        ("sop-date", DATE_SHAPED_SOP),
        ("sop-root-prefix", ROOT_PREFIX_LOOKALIKE),
        ("sop-root-bare", DICOM_ROOT_ONLY),
    ] {
        let report = load(name);
        let json = report.to_json();
        assert!(!json.contains(stored), "{name} leaked {stored} in {json}");
        assert!(
            json.contains("\"sopClass\":null,\"sopClassNote\":\"not a DICOM UID\""),
            "{name} {json}"
        );
        assert!(report.findings().is_empty(), "{name} {}", describe(&report));
        assert!(
            matches!(report.modules(), ModuleScope::NotEvaluated { .. }),
            "{name} was evaluated"
        );
        let ModuleScope::NotEvaluated {
            sop_class,
            sop_class_note,
        } = report.modules()
        else {
            unreachable!("{name} was evaluated")
        };
        assert_eq!(sop_class.as_deref(), None, "{name}");
        assert_eq!(*sop_class_note, Some("not a DICOM UID"), "{name}");
    }
}

#[test]
fn fixture_ui_value_has_at_most_one_null_pad() {
    // PS3.5 6.2. A UI value of odd length is padded with one trailing NULL.
    // PS3.5 9.1 then reads the components. The Secondary Capture UID is 25
    // bytes, so one NULL is that pad and the UID is repeated. Three NULLs
    // are not that pad. The value is not a UID, it is not repeated, and the
    // finding is UI-FORMAT. The same three NULLs on the CT UID do not select
    // the CT IOD.
    assert_eq!(SC_UID.len(), 25);
    assert_eq!(CT_UID.len(), 25);
    let padded = load("sop-one-null");
    assert!(padded.findings().is_empty(), "{}", describe(&padded));
    let ModuleScope::NotEvaluated {
        sop_class,
        sop_class_note,
    } = padded.modules()
    else {
        unreachable!("one NULL was evaluated")
    };
    assert_eq!(sop_class.as_deref(), Some(SC_UID));
    assert_eq!(*sop_class_note, None);
    assert!(
        padded
            .to_json()
            .contains("\"sopClass\":\"1.2.840.10008.5.1.4.1.1.7\",\"sopClassNote\":null"),
        "{}",
        padded.to_json()
    );
    for name in ["sop-three-null", "sop-ct-three-null"] {
        let report = load(name);
        let json = report.to_json();
        assert!(!json.contains(SC_UID), "{name} leaked {SC_UID} in {json}");
        assert!(!json.contains(CT_UID), "{name} leaked {CT_UID} in {json}");
        assert!(
            json.contains("\"sopClass\":null,\"sopClassNote\":\"not a valid UID\""),
            "{name} {json}"
        );
        assert_only(
            &report,
            &[one(
                (0x0008, 0x0016),
                "SOPClassUID",
                ConformanceRule::UiFormat,
                Some(1),
                Measure::None,
            )],
        );
        let ModuleScope::NotEvaluated {
            sop_class,
            sop_class_note,
        } = report.modules()
        else {
            unreachable!("{name} was evaluated")
        };
        assert_eq!(sop_class.as_deref(), None, "{name}");
        assert_eq!(*sop_class_note, Some("not a valid UID"), "{name}");
    }
    // PS3.5 6.2. An even-length UID needs no pad. Two trailing NULLs leave
    // one NULL in the value, so it is not a UID and is not repeated.
    // 1.2.840.10008.1.20 is 18 bytes and its components begin with the
    // DICOM root, which is why a second NULL strip would repeat it.
    assert_eq!(EVEN_ROOT_UID.len(), 18);
    assert!(EVEN_ROOT_UID.len().is_multiple_of(2));
    assert!(EVEN_ROOT_UID.starts_with("1.2.840.10008."));
    assert!(ps35_uid(EVEN_ROOT_UID));
    let even = load("sop-even-two-null");
    let even_json = even.to_json();
    assert!(!even_json.contains(EVEN_ROOT_UID), "{even_json}");
    assert!(
        even_json.contains("\"sopClass\":null,\"sopClassNote\":\"not a valid UID\""),
        "{even_json}"
    );
    assert_only(
        &even,
        &[one(
            (0x0008, 0x0016),
            "SOPClassUID",
            ConformanceRule::UiFormat,
            Some(1),
            Measure::None,
        )],
    );
    let ModuleScope::NotEvaluated {
        sop_class,
        sop_class_note,
    } = even.modules()
    else {
        unreachable!("two NULLs were evaluated")
    };
    assert_eq!(sop_class.as_deref(), None);
    assert_eq!(*sop_class_note, Some("not a valid UID"));
}

#[test]
fn fixture_ui_pad_belongs_to_the_value_field() {
    // PS3.5 6.2. The single trailing NULL pad belongs to the Value Field,
    // so only the last value may carry it. A NULL anywhere else is part of
    // the value. PS3.5 9.1 then allows only digits and dots.
    // 1.2.3, NULL, backslash, 1.2.5 is 12 bytes. Value 1 is not a UID.
    assert_eq!("1.2.3\0\\1.2.5".len(), 12);
    assert_only(
        &load("ui-null-mid"),
        &[one(
            (0x0008, 0x001A),
            "RelatedGeneralSOPClassUID",
            ConformanceRule::UiFormat,
            Some(1),
            Measure::None,
        )],
    );
    // 1.2.3, backslash, 1.2.5 is 11 bytes. One NULL on that odd field is
    // the pad, and it sits on the last value. Both values are UIDs.
    assert_eq!("1.2.3\\1.2.5".len(), 11);
    assert_clear("ui-pad-last");
}

#[test]
fn fixture_sop_class_null_before_delimiter_is_not_repeated() {
    // PS3.5 6.2. (0008,0016) is the SC UID, NULL, backslash, 19620203, NULL.
    // The first NULL is not the Value Field pad. PS3.5 9.1 rejects it, so
    // the SOP Class is not repeated. PS3.5 6.4 reports the two values.
    let mixed = load("sop-null-then-date");
    let mixed_json = mixed.to_json();
    assert!(!mixed_json.contains(SC_UID), "{mixed_json}");
    assert!(!mixed_json.contains(DATE_SHAPED_SOP), "{mixed_json}");
    assert!(
        mixed_json.contains("\"sopClass\":null,\"sopClassNote\":\"not a valid UID\""),
        "{mixed_json}"
    );
    assert_only(
        &mixed,
        &[
            one(
                (0x0008, 0x0016),
                "SOPClassUID",
                ConformanceRule::Multiplicity,
                None,
                values(2, 1, Some(1)),
            ),
            one(
                (0x0008, 0x0016),
                "SOPClassUID",
                ConformanceRule::UiFormat,
                Some(1),
                Measure::None,
            ),
        ],
    );
    let ModuleScope::NotEvaluated {
        sop_class,
        sop_class_note,
    } = mixed.modules()
    else {
        unreachable!("a NULL before the delimiter was repeated")
    };
    assert_eq!(sop_class.as_deref(), None);
    assert_eq!(*sop_class_note, Some("not a valid UID"));

    // PS3.5 6.2 and 6.4. (0008,0016) is the CT Image Storage UID, NULL,
    // backslash, 1.2.3.4. The NULL ends the first value and is not the pad,
    // so the first value is not the CT UID. The IOD lookup must not evaluate
    // the CT IOD, and the scope must not repeat the UID.
    let ct_mixed = load("sop-ct-null-then-uid");
    let ct_json = ct_mixed.to_json();
    assert!(!ct_json.contains(CT_UID), "{ct_json}");
    assert!(
        ct_json.contains("\"sopClass\":null,\"sopClassNote\":\"not a valid UID\""),
        "{ct_json}"
    );
    assert_only(
        &ct_mixed,
        &[
            one(
                (0x0008, 0x0016),
                "SOPClassUID",
                ConformanceRule::Multiplicity,
                None,
                values(2, 1, Some(1)),
            ),
            one(
                (0x0008, 0x0016),
                "SOPClassUID",
                ConformanceRule::UiFormat,
                Some(1),
                Measure::None,
            ),
        ],
    );
    let ModuleScope::NotEvaluated {
        sop_class,
        sop_class_note,
    } = ct_mixed.modules()
    else {
        unreachable!("a CT UID with a NULL before the delimiter was evaluated")
    };
    assert_eq!(sop_class.as_deref(), None);
    assert_eq!(*sop_class_note, Some("not a valid UID"));
}

#[test]
fn fixture_multi_valued_sop_class_is_scoped_by_its_first_value() {
    // PS3.5 6.4. "Each string Value in a multi-valued character string may be
    // empty, unless otherwise specified." (0008,0016) holds the CT Image
    // Storage UID, two backslashes and the NULL pad, 28 bytes and three
    // values, the second and third empty. The first value is the CT UID, so
    // the IOD is evaluated with its seven modules, the over-multiplicity is
    // reported as MULTIPLICITY 3 against 1, and neither empty value has a
    // UI-FORMAT finding.
    let ct = load("sop-ct-then-two-empty");
    let (sop, iod, table, modules) = evaluated(&ct);
    assert_eq!(sop, CT_UID);
    assert_eq!(iod, "CT Image");
    assert_eq!(table, "PS3.3 A.3.3");
    assert_eq!(modules, PLANE_MODULES);
    let ct_json = ct.to_json();
    let needle = format!("\"sopClass\":\"{CT_UID}\",\"sopClassNote\":null");
    assert!(ct_json.contains(&needle), "{ct_json}");
    assert_only(
        &ct,
        &[one(
            (0x0008, 0x0016),
            "SOPClassUID",
            ConformanceRule::Multiplicity,
            None,
            values(3, 1, Some(1)),
        )],
    );

    // PS3.5 6.4. The same field with the empty value first, one backslash and
    // then the CT UID, is not evaluated. The first value is empty, so the
    // scope repeats no UID and names no reason, and the MULTIPLICITY finding
    // of two values against 1 stands with no UI-FORMAT finding.
    let empty_first = load("sop-empty-then-ct");
    let empty_json = empty_first.to_json();
    assert!(
        empty_json.contains("\"sopClass\":null,\"sopClassNote\":null"),
        "{empty_json}"
    );
    assert_only(
        &empty_first,
        &[one(
            (0x0008, 0x0016),
            "SOPClassUID",
            ConformanceRule::Multiplicity,
            None,
            values(2, 1, Some(1)),
        )],
    );
    let ModuleScope::NotEvaluated {
        sop_class,
        sop_class_note,
    } = empty_first.modules()
    else {
        unreachable!("an empty first value was evaluated")
    };
    assert_eq!(sop_class.as_deref(), None);
    assert_eq!(*sop_class_note, None);
}

#[test]
fn fixture_empty_value_in_a_multi_valued_string_is_not_checked() {
    // PS3.5 6.4. "Each string Value in a multi-valued character string may
    // be empty, unless otherwise specified." An empty value is not described
    // by the VR grammar, so it has no format, length or repertoire finding.
    // PS3.5 6.2 and 6.4. The single trailing pad belongs to the last value
    // and is removed before the value is tested for empty.
    // 1.2.34, backslash, NULL is 8 bytes. Value 2 is empty after the pad.
    assert_eq!("1.2.34\\\0".len(), 8);
    assert_clear("ui-empty-last-pad");
    // 20240101, two backslashes, 20240102 is 18 bytes. Value 2 is empty.
    assert_eq!("20240101\\\\20240102".len(), 18);
    assert_clear("da-empty-mid");
    // 40, two backslashes, 50 is 6 bytes. Value 2 is empty.
    assert_eq!("40\\\\50".len(), 6);
    assert_clear("ds-empty-mid");
}

#[test]
fn fixture_escape_requires_an_iso_2022_sequence() {
    // PS3.5 Table 6.2-1 and 6.1.2.5. ESC is a control unless it introduces
    // an ISO/IEC 2022 escape sequence. PS3.3 C.12.1.1.2. ISO_IR 100 and
    // ISO_IR 192 do not use code extension. Absent (0008,0005) does not
    // either, and decision 6 still reports the missing set. ISO 2022 IR 100
    // uses ESC 02/13 04/01, which is ESC, hyphen, A.
    assert_only(
        &load("lo-esc-100"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::Repertoire,
            Some(1),
            character(3, CharacterClass::Control),
        )],
    );
    assert_only(
        &load("lo-esc-192"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::Repertoire,
            Some(1),
            character(3, CharacterClass::Control),
        )],
    );
    assert_only(
        &load("lo-esc-absent"),
        &[
            one(
                (0x0008, 0x0005),
                "SpecificCharacterSet",
                ConformanceRule::Type1cAbsent,
                None,
                Measure::None,
            ),
            one(
                (0x0008, 0x0080),
                "InstitutionName",
                ConformanceRule::Repertoire,
                Some(1),
                character(3, CharacterClass::Control),
            ),
        ],
    );
    assert_clear("lo-esc-2022");
    assert_only(
        &load("lo-esc-2022-bare"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::Repertoire,
            Some(1),
            character(3, CharacterClass::Control),
        )],
    );
}

#[test]
fn fixture_unregistered_escape_is_a_control() {
    // PS3.5 Table 6.2-1. ESC is a control unless the bytes after it are a
    // sequence from PS3.3 C.12.1.1.2 Tables C.12-3 and C.12-4. ESC 02/05 04/07
    // and ESC 02/08 04/03 are not in those tables. ISO 2022 IR 100 on the
    // second value of (0008,0005) declares code extension. The control is the
    // third character of each value.
    assert_eq!(ESC_PERCENT_G.chars().nth(2), Some('\u{001B}'));
    assert_eq!(ESC_PAREN_C.chars().nth(2), Some('\u{001B}'));
    assert_only(
        &load("lo-esc-percent-g"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::Repertoire,
            Some(1),
            character(2, CharacterClass::Control),
        )],
    );
    assert_only(
        &load("lo-esc-paren-c"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::Repertoire,
            Some(1),
            character(2, CharacterClass::Control),
        )],
    );
}

#[test]
fn fixture_registered_escape_in_ur_is_a_control() {
    // PS3.5 6.1.2.2 names SH, LO, UC, ST, LT, UT and PN. UR is not among them.
    // PS3.5 6.1.2.3 says that replacement repertoire does not apply to other
    // textual VRs. ESC 02/13 04/01 is Table C.12-3 for ISO 2022 IR 100, and
    // it remains a control in a UR.
    assert_eq!(ESC_UR.len(), 18);
    assert_eq!(ESC_UR.chars().position(|ch| ch == '\u{001B}'), Some(14));
    assert_only(
        &load("ur-esc-2022"),
        &[one(
            (0x0028, 0x7FE0),
            "PixelDataProviderURL",
            ConformanceRule::Repertoire,
            Some(1),
            character(14, CharacterClass::Control),
        )],
    );
}

#[test]
fn fixture_pad_boundary_is_one_byte() {
    // PS3.5 6.4. The last value may exceed the VR maximum by exactly one
    // padding byte. Two bytes over the maximum is a length finding even
    // when the final byte is the pad.
    assert_clear("cs-pad-last");
    assert_only(
        &load("cs-pad-two"),
        &[one(
            (0x0008, 0x0008),
            "ImageType",
            ConformanceRule::ValueLength,
            Some(3),
            bytes(18, 16),
        )],
    );
}

#[test]
fn fixture_iso_ir_192_measures_bytes() {
    // PS3.5 6.2 Note and Table 6.2-1. DS is 16 bytes. Under ISO_IR 192 two
    // U+00E9 characters make 16 characters and 18 bytes. The same characters
    // without that set are 16 bytes and a repertoire finding, not a length.
    let utf8 = format!("{}{}", "1".repeat(14), "\u{00E9}\u{00E9}");
    assert_eq!(utf8.chars().count(), 16);
    assert_eq!(utf8.len(), 18);
    assert_only(
        &load("ds-utf8"),
        &[one(
            (0x0018, 0x0050),
            "SliceThickness",
            ConformanceRule::ValueLength,
            Some(1),
            bytes(18, 16),
        )],
    );
    assert_only(
        &load("ds-latin1"),
        &[one(
            (0x0018, 0x0050),
            "SliceThickness",
            ConformanceRule::Repertoire,
            Some(1),
            character(14, CharacterClass::NonDefault),
        )],
    );
}

#[test]
fn fixture_type1_empty_and_delimiter_only() {
    // PS3.5 7.4.1. Zero length is empty. Two backslashes delimit empty values
    // and are empty too. Neither value is also a VM or format finding.
    assert_only(
        &load("type1-empty"),
        &[
            one(
                (0x0008, 0x0018),
                "SOPInstanceUID",
                ConformanceRule::Type1Empty,
                None,
                Measure::None,
            ),
            one(
                (0x0008, 0x0060),
                "Modality",
                ConformanceRule::Type1Empty,
                None,
                Measure::None,
            ),
        ],
    );
    // PS3.5 6.4. "a single padding character shall be applied to the end
    // of the Value Field". One backslash is one byte, so the field carries
    // the pad, NULL for UI and SPACE for CS. After the pad is removed the
    // field is delimiters alone, which 6.4 treats like an empty Value and
    // 7.4.1 says does not satisfy Type 1.
    assert_eq!("\\\0".len(), 2);
    assert_eq!("\\ ".len(), 2);
    assert_only(
        &load("type1-delim-pad"),
        &[
            one(
                (0x0008, 0x0018),
                "SOPInstanceUID",
                ConformanceRule::Type1Empty,
                None,
                Measure::None,
            ),
            one(
                (0x0008, 0x0060),
                "Modality",
                ConformanceRule::Type1Empty,
                None,
                Measure::None,
            ),
        ],
    );
}

#[test]
fn fixture_absent_type1_and_type2() {
    // PS3.5 7.4.1 and 7.4.3. Type 1 absence and Type 2 absence are findings.
    // Type 2 with zero length is not.
    assert_only(
        &load("rows-absent"),
        &[one(
            (0x0028, 0x0010),
            "Rows",
            ConformanceRule::Type1Absent,
            None,
            Measure::None,
        )],
    );
    assert_only(
        &load("name-absent"),
        &[one(
            (0x0010, 0x0010),
            "PatientName",
            ConformanceRule::Type2Absent,
            None,
            Measure::None,
        )],
    );
    assert_clear("name-empty");
}

#[test]
fn fixture_decimal_string() {
    // PS3.5 Table 6.2-1. The grammar is `parse_decimal_string`. A 17-byte
    // value is over the 16 byte maximum. `1E999` matches the character
    // grammar and does not fit in binary64, so it is DS-RANGE. Odd spellings
    // in the acceptance text are stored with one trailing space.
    assert_clear("ds-space");
    assert_clear("ds-exponent");
    assert_only(
        &load("ds-comma"),
        &[one(
            (0x0018, 0x0050),
            "SliceThickness",
            ConformanceRule::DsFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("ds-gap"),
        &[one(
            (0x0018, 0x0050),
            "SliceThickness",
            ConformanceRule::DsFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("ds-17"),
        &[one(
            (0x0018, 0x1164),
            "ImagerPixelSpacing",
            ConformanceRule::ValueLength,
            Some(1),
            bytes(17, 16),
        )],
    );
    assert_only(
        &load("ds-range"),
        &[one(
            (0x0018, 0x0050),
            "SliceThickness",
            ConformanceRule::DsRange,
            Some(1),
            Measure::None,
        )],
    );
}

#[test]
fn fixture_integer_string() {
    // PS3.5 Table 6.2-1. IS is at most 12 bytes and a signed 32-bit integer.
    // 2147483647 is the maximum. 2147483648 is one past it. The 13-byte value
    // is the first of two so the element length is even, on a private tag so
    // a second value is not also a VM finding.
    assert_clear("is-min");
    assert_clear("is-max");
    assert_only(
        &load("is-over"),
        &[one(
            (0x0020, 0x0011),
            "SeriesNumber",
            ConformanceRule::IsRange,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("is-bad"),
        &[one(
            (0x0020, 0x0011),
            "SeriesNumber",
            ConformanceRule::IsFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("is-13"),
        &[none_kw(
            (0x0019, 0x1002),
            ConformanceRule::ValueLength,
            Some(1),
            bytes(13, 12),
        )],
    );
    assert_clear("is-padded");
}

#[test]
fn fixture_date() {
    // PS3.5 Table 6.2-1. DA is Gregorian YYYYMMDD. 2024 and 2000 are leap
    // years. 1900 is not, because a century year must also be divisible by
    // 400. `2026.11.05` is 10 bytes against the fixed 8.
    assert_clear("da-2024");
    assert_only(
        &load("da-2023"),
        &[one(
            (0x0008, 0x0020),
            "StudyDate",
            ConformanceRule::DaFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("da-june"),
        &[one(
            (0x0008, 0x0020),
            "StudyDate",
            ConformanceRule::DaFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("da-1900"),
        &[one(
            (0x0008, 0x0020),
            "StudyDate",
            ConformanceRule::DaFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_clear("da-2000");
    assert_only(
        &load("da-dotted"),
        &[one(
            (0x0008, 0x0020),
            "StudyDate",
            ConformanceRule::FixedLength,
            Some(1),
            bytes(10, 8),
        )],
    );
}

#[test]
fn fixture_time() {
    // PS3.5 Table 6.2-1, the standard's own examples. Hour is 00 to 23.
    // Minute is 00 to 59. Second is 00 to 60. A fraction needs seconds and
    // at most six digits. Odd examples are stored with one trailing space,
    // which TM then ignores.
    assert_clear("tm-frac");
    assert_clear("tm-1010");
    assert_only(
        &load("tm-021"),
        &[one(
            (0x0008, 0x0030),
            "StudyTime",
            ConformanceRule::TmFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("tm-2400"),
        &[one(
            (0x0008, 0x0030),
            "StudyTime",
            ConformanceRule::TmFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_clear("tm-leap");
    assert_only(
        &load("tm-1260"),
        &[one(
            (0x0008, 0x0030),
            "StudyTime",
            ConformanceRule::TmFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("tm-dot"),
        &[one(
            (0x0008, 0x0030),
            "StudyTime",
            ConformanceRule::TmFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("tm-empty-frac"),
        &[one(
            (0x0008, 0x0030),
            "StudyTime",
            ConformanceRule::TmFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("tm-long-frac"),
        &[one(
            (0x0008, 0x0030),
            "StudyTime",
            ConformanceRule::TmFormat,
            Some(1),
            Measure::None,
        )],
    );
}

#[test]
fn fixture_date_time() {
    // PS3.5 Table 6.2-1 and its note. The offset is -1200 to +1400, and
    // -0000 is forbidden. Odd examples are stored with one trailing space.
    assert_clear("dt-195308");
    assert_clear("dt-full");
    assert_clear("dt-offset");
    assert_only(
        &load("dt-negzero"),
        &[one(
            (0x0008, 0x002A),
            "AcquisitionDateTime",
            ConformanceRule::DtFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("dt-1500"),
        &[one(
            (0x0008, 0x002A),
            "AcquisitionDateTime",
            ConformanceRule::DtFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("dt-1300"),
        &[one(
            (0x0008, 0x002A),
            "AcquisitionDateTime",
            ConformanceRule::DtFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_clear("dt-1400");
    assert_only(
        &load("dt-short"),
        &[one(
            (0x0008, 0x002A),
            "AcquisitionDateTime",
            ConformanceRule::DtFormat,
            Some(1),
            Measure::None,
        )],
    );
}

#[test]
fn fixture_uid() {
    // PS3.5 9.1. Components are digits with no leading zero, except a single
    // zero. NULL is the pad. SPACE is not. 65 ones cannot stand alone because
    // 65 is odd, so they are the first of two values on a private UI tag.
    let valid = load("valid-ct");
    assert!(
        valid
            .findings()
            .iter()
            .all(|finding| finding.rule != ConformanceRule::UiFormat)
    );
    assert_only(
        &load("ui-lead"),
        &[one(
            (0x0020, 0x0052),
            "FrameOfReferenceUID",
            ConformanceRule::UiFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("ui-empty"),
        &[one(
            (0x0020, 0x0052),
            "FrameOfReferenceUID",
            ConformanceRule::UiFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("ui-letter"),
        &[one(
            (0x0020, 0x0052),
            "FrameOfReferenceUID",
            ConformanceRule::UiFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_clear("ui-zero");
    assert_only(
        &load("ui-long"),
        &[none_kw(
            (0x0019, 0x1001),
            ConformanceRule::ValueLength,
            Some(1),
            bytes(65, 64),
        )],
    );
    assert_clear("ui-null");
    assert_only(
        &load("ui-space"),
        &[one(
            (0x0020, 0x0052),
            "FrameOfReferenceUID",
            ConformanceRule::UiFormat,
            Some(1),
            Measure::None,
        )],
    );
}

#[test]
fn fixture_code_string_and_ae() {
    // PS3.5 Table 6.2-1. CS is uppercase, digits, space and underscore.
    // `ct` fails at index 0. AE of only spaces is AE-SPACES, not empty.
    assert_only(
        &load("cs-lower"),
        &[one(
            (0x0008, 0x0060),
            "Modality",
            ConformanceRule::Repertoire,
            Some(1),
            character(0, CharacterClass::OutsideRepertoire),
        )],
    );
    assert_clear("cs-token");
    assert_only(
        &load("cs-17"),
        &[one(
            (0x0008, 0x0008),
            "ImageType",
            ConformanceRule::ValueLength,
            Some(1),
            bytes(17, 16),
        )],
    );
    assert_only(
        &load("ae-spaces"),
        &[one(
            (0x0008, 0x0054),
            "RetrieveAETitle",
            ConformanceRule::AeSpaces,
            Some(1),
            Measure::None,
        )],
    );
}

#[test]
fn fixture_person_name() {
    // PS3.5 Table 6.2-1. Each component group is at most 64 characters.
    // Five components are four carets. Three groups are two equals signs.
    // Odd spellings carry one trailing space. 64 A and 63 B is 128 characters
    // and still legal, which a whole-value limit would reject.
    assert_clear("pn-ok");
    assert_only(
        &load("pn-carets"),
        &[one(
            (0x0010, 0x0010),
            "PatientName",
            ConformanceRule::PnFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_clear("pn-groups");
    assert_only(
        &load("pn-groups-bad"),
        &[one(
            (0x0010, 0x0010),
            "PatientName",
            ConformanceRule::PnFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert_only(
        &load("pn-65"),
        &[one(
            (0x0010, 0x0010),
            "PatientName",
            ConformanceRule::ValueLength,
            Some(1),
            chars(65, 64),
        )],
    );
    assert_clear("pn-split");
}

#[test]
fn fixture_control_characters() {
    // PS3.5 Table 6.2-1. LO rejects TAB. LT allows TAB, CR and LF. ST rejects
    // BEL.
    assert_only(
        &load("lo-tab"),
        &[one(
            (0x0008, 0x0080),
            "InstitutionName",
            ConformanceRule::Repertoire,
            Some(1),
            character(3, CharacterClass::Control),
        )],
    );
    assert_clear("lt-ok");
    assert_only(
        &load("st-bel"),
        &[one(
            (0x0008, 0x2111),
            "DerivationDescription",
            ConformanceRule::Repertoire,
            Some(1),
            character(1, CharacterClass::Control),
        )],
    );
}

#[test]
fn fixture_character_set() {
    // PS3.5 6.1.2.2 and PS3.3 C.12-1. Byte E9 in an LO is an expanded
    // repertoire, so a missing Specific Character Set is one Type 1C finding
    // and the LO itself is quiet. The same byte in a CS is NonDefault whether
    // or not the set is declared.
    assert_only(
        &load("lo-e9"),
        &[one(
            (0x0008, 0x0005),
            "SpecificCharacterSet",
            ConformanceRule::Type1cAbsent,
            None,
            Measure::None,
        )],
    );
    assert_clear("lo-e9-100");
    assert_only(
        &load("cs-e9"),
        &[one(
            (0x0008, 0x0060),
            "Modality",
            ConformanceRule::Repertoire,
            Some(1),
            character(0, CharacterClass::NonDefault),
        )],
    );
    assert_only(
        &load("cs-e9-100"),
        &[one(
            (0x0008, 0x0060),
            "Modality",
            ConformanceRule::Repertoire,
            Some(1),
            character(0, CharacterClass::NonDefault),
        )],
    );
}

#[test]
fn fixture_multiplicity() {
    // PS3.5 6.4 and PS3.6 Table 6-1. Pixel Spacing is 2. Image Orientation is
    // 6. Patient ID is 1. The acceptance text `AB\\CD` is five bytes and odd,
    // so the stored pair is `ABC\\DE`. Specific Character Set is 1-n.
    assert_only(
        &load("vm-spacing"),
        &[one(
            (0x0028, 0x0030),
            "PixelSpacing",
            ConformanceRule::Multiplicity,
            None,
            values(1, 2, Some(2)),
        )],
    );
    assert_only(
        &load("vm-orient"),
        &[one(
            (0x0020, 0x0037),
            "ImageOrientationPatient",
            ConformanceRule::Multiplicity,
            None,
            values(5, 6, Some(6)),
        )],
    );
    assert_only(
        &load("vm-id"),
        &[one(
            (0x0010, 0x0020),
            "PatientID",
            ConformanceRule::Multiplicity,
            None,
            values(2, 1, Some(1)),
        )],
    );
    assert_clear("vm-charset");
    // PS3.5 6.4. Values are delimited by backslash, so an empty value is
    // still a value for multiplicity. Modality is VM 1. CT, backslash and
    // the SPACE pad is two values, the second empty, and the empty value
    // has no format finding.
    assert_eq!("CT\\ ".len(), 4);
    assert_only(
        &load("vm-empty-counted"),
        &[one(
            (0x0008, 0x0060),
            "Modality",
            ConformanceRule::Multiplicity,
            None,
            values(2, 1, Some(1)),
        )],
    );
}

#[test]
fn vm_table_agrees_with_the_dictionary() {
    // PS3.6 Table 6-1, cross-checked against dicom-dictionary-std 0.10.0.
    // US or SS is Xs. OB or OW is Px for Pixel Data.
    assert_eq!(vm_entries().len(), 47);
    for entry in vm_entries() {
        let dict = StandardDataDictionary.by_tag(entry.tag);
        assert!(dict.is_some(), "{}", entry.keyword);
        let Some(dict) = dict else {
            unreachable!("asserted immediately above")
        };
        assert_eq!(dict.alias(), entry.keyword);
        let same = match (entry.vr, dict.vr()) {
            (VmVr::Exact(left), VirtualVr::Exact(right)) => left == right,
            (VmVr::UsOrSs, VirtualVr::Xs) => true,
            (VmVr::ObOrOw, VirtualVr::Px | VirtualVr::Ox) => true,
            _ => false,
        };
        assert!(same, "{}", entry.keyword);
    }
}

#[test]
fn fixture_vr_mismatch_and_un() {
    // PS3.5 6.2.2. PatientAge encoded LO is a mismatch. The LO value itself
    // is legal. UN is permitted and counted, not flagged.
    assert_only(
        &load("vr-lo"),
        &[one(
            (0x0010, 0x1010),
            "PatientAge",
            ConformanceRule::VrMismatch,
            None,
            Measure::None,
        )],
    );
    let un = load("vr-un");
    assert!(un.findings().is_empty(), "{}", describe(&un));
    assert_eq!(un.not_checked_un(), 1);
}

#[test]
fn fixture_decidable_type1c() {
    // PS3.3 C.7-11c and C.7-11a. Planar Configuration is required when Samples
    // per Pixel is greater than 1. PALETTE COLOR requires all six palette
    // elements. Pixel Data is required when the provider URL is absent or
    // empty. Extended Offset Table Lengths is required when the table is
    // present.
    assert_only(
        &load("spp-3"),
        &[one(
            (0x0028, 0x0006),
            "PlanarConfiguration",
            ConformanceRule::Type1cAbsent,
            None,
            Measure::None,
        )],
    );
    assert_clear("valid-ct");
    let palette = load("palette");
    let tags = [
        (0x0028, 0x1101, "RedPaletteColorLookupTableDescriptor"),
        (0x0028, 0x1102, "GreenPaletteColorLookupTableDescriptor"),
        (0x0028, 0x1103, "BluePaletteColorLookupTableDescriptor"),
        (0x0028, 0x1201, "RedPaletteColorLookupTableData"),
        (0x0028, 0x1202, "GreenPaletteColorLookupTableData"),
        (0x0028, 0x1203, "BluePaletteColorLookupTableData"),
    ];
    assert_eq!(
        palette.findings().len(),
        tags.len(),
        "{}",
        describe(&palette)
    );
    for (index, (group, element, keyword)) in tags.iter().enumerate() {
        let finding = palette.findings().get(index);
        assert!(finding.is_some());
        let Some(finding) = finding else {
            unreachable!("asserted immediately above")
        };
        assert_eq!(finding.tag, Tag(*group, *element));
        assert_eq!(finding.keyword, Some(*keyword));
        assert_eq!(finding.rule, ConformanceRule::Type1cAbsent);
    }
    assert_only(
        &load("pixels-absent"),
        &[one(
            (0x7FE0, 0x0010),
            "PixelData",
            ConformanceRule::Type1cAbsent,
            None,
            Measure::None,
        )],
    );
    assert_clear("pixels-url");
    assert_only(
        &load("pixels-blank-url"),
        &[one(
            (0x7FE0, 0x0010),
            "PixelData",
            ConformanceRule::Type1cAbsent,
            None,
            Measure::None,
        )],
    );
    assert_only(
        &load("offset-only"),
        &[one(
            (0x7FE0, 0x0002),
            "ExtendedOffsetTableLengths",
            ConformanceRule::Type1cAbsent,
            None,
            Measure::None,
        )],
    );
}

#[test]
fn fixture_decidable_type2c() {
    // PS3.3 C.7-5a and C.7-9. Patient Position is required for CT and MR when
    // (0054,0410) is absent. It is not required for CR or PET. Patient
    // Orientation is required for CR only.
    assert_only(
        &load("position-absent"),
        &[one(
            (0x0018, 0x5100),
            "PatientPosition",
            ConformanceRule::Type2cAbsent,
            None,
            Measure::None,
        )],
    );
    assert_clear("position-code");
    assert_clear("cr-base");
    assert_only(
        &load("cr-orient"),
        &[one(
            (0x0020, 0x0020),
            "PatientOrientation",
            ConformanceRule::Type2cAbsent,
            None,
            Measure::None,
        )],
    );
    assert_clear("valid-ct");
    assert_only(
        &load("mr-position"),
        &[one(
            (0x0018, 0x5100),
            "PatientPosition",
            ConformanceRule::Type2cAbsent,
            None,
            Measure::None,
        )],
    );
    assert_clear("pet-base");
}

#[test]
fn fixture_module_scope_by_sop_class() {
    // PS3.3 A.2.3. CR has six modules and no Image Plane attributes.
    // A Secondary Capture SOP class is not evaluated. Rows may be absent
    // without a presence finding, and a value finding is still reported.
    let cr = load("cr-base");
    assert!(cr.findings().is_empty(), "{}", describe(&cr));
    let (sop, iod, table, modules) = evaluated(&cr);
    assert_eq!(sop, CR_UID);
    assert_eq!(iod, "CR Image");
    assert_eq!(table, "PS3.3 A.2.3");
    assert_eq!(modules, CR_MODULES);
    assert_eq!(modules.len(), 6);
    let mr = load("mr-base");
    let (sop, iod, table, modules) = evaluated(&mr);
    assert!(mr.findings().is_empty(), "{}", describe(&mr));
    assert_eq!(sop, MR_UID);
    assert_eq!(iod, "MR Image");
    assert_eq!(table, "PS3.3 A.4.3");
    assert_eq!(modules, PLANE_MODULES);
    let pet = load("pet-base");
    let (sop, iod, table, modules) = evaluated(&pet);
    assert!(pet.findings().is_empty(), "{}", describe(&pet));
    assert_eq!(sop, PET_UID);
    assert_eq!(iod, "PET Image");
    assert_eq!(table, "PS3.3 A.21.3");
    assert_eq!(modules, PLANE_MODULES);
    let sc = load("sc-rows");
    assert!(
        matches!(sc.modules(), ModuleScope::NotEvaluated { .. }),
        "secondary capture is outside the four IODs"
    );
    let ModuleScope::NotEvaluated {
        sop_class,
        sop_class_note,
    } = sc.modules()
    else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(sop_class.as_deref(), Some(SC_UID));
    assert_eq!(*sop_class_note, None);
    let needle = format!("\"sopClass\":\"{SC_UID}\",\"sopClassNote\":null");
    assert!(sc.to_json().contains(&needle), "{}", sc.to_json());
    assert_only(
        &sc,
        &[one(
            (0x0010, 0x1010),
            "PatientAge",
            ConformanceRule::AsFormat,
            Some(1),
            Measure::None,
        )],
    );
    assert!(!sc.to_json().contains("\"notChecked\""));
}

#[test]
fn undecidable_conditions_are_named() {
    // Decision 4. The not-decidable rows are named and their absence is not
    // a finding. The valid CT base omits every one of them.
    let report = load("valid-ct");
    assert_eq!(report.not_checked(), NOT_DECIDABLE);
    assert!(report.findings().is_empty(), "{}", describe(&report));
    let sc = load("sc-rows");
    assert_eq!(sc.not_checked(), NOT_DECIDABLE);
}

#[test]
fn fixture_nested_item_path() {
    // PS3.5 7.5. The second item is item 2. A sequence's VM is 1, not the
    // item count, so two items are not a multiplicity finding.
    let report = load("nested");
    assert_eq!(report.findings().len(), 1, "{}", describe(&report));
    let finding = report.findings().first();
    assert!(finding.is_some());
    let Some(finding) = finding else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(finding.tag, Tag(0x0008, 0x0100));
    assert_eq!(finding.keyword, Some("CodeValue"));
    assert_eq!(finding.rule, ConformanceRule::Repertoire);
    assert_eq!(finding.value, Some(1));
    assert_eq!(finding.measure, character(3, CharacterClass::Control));
    assert_eq!(finding.path.len(), 1);
    let step = finding.path.first();
    assert!(step.is_some());
    let Some(step) = step else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(step.sequence, Tag(0x0054, 0x0410));
    assert_eq!(step.item, 2);
    assert_eq!(report.elements(), 32);
}

#[test]
fn no_finding_carries_a_value() {
    // Decision 5, as amended after review pass 2. A finding carries a measure,
    // never a value. The scan is the trimmed text of every written value of
    // three or more characters. The report may repeat a SOP Class UID only
    // when PS3.5 9.1 accepts it and the text begins with the PS3.6 root
    // 1.2.840.10008. A shorter written value can sit inside that UID, so the
    // scan removes such a UID. A private UID and a date-shaped component stay
    // in the scan.
    let all = fixtures();
    let mut names = Vec::new();
    let mut saw_age = false;
    let mut saw_name = false;
    let mut saw_long = false;
    let mut saw_private = false;
    let mut saw_date = false;
    let long = "A".repeat(170);
    for fixture in &all {
        assert!(!names.contains(&fixture.name), "duplicate {}", fixture.name);
        names.push(fixture.name);
        let parsed = require(parse_part10(&encode(fixture)));
        let mut json = check_conformance(&parsed).to_json();
        if ps35_uid(fixture.sop) && fixture.sop.starts_with("1.2.840.10008.") {
            json = json.replace(fixture.sop, "");
        }
        let mut secrets = Vec::new();
        for item in &fixture.elements {
            for secret in &item.secrets {
                if !SOP_CLASSES.contains(&secret.as_str()) {
                    secrets.push(secret.clone());
                }
            }
        }
        for secret in META_SECRETS {
            secrets.push((*secret).to_owned());
        }
        if secrets.iter().any(|secret| secret == "040") {
            saw_age = true;
        }
        if secrets.iter().any(|secret| secret == "SECRET^NAME") {
            saw_name = true;
        }
        if secrets.iter().any(|secret| secret == &long) {
            saw_long = true;
        }
        if secrets.iter().any(|secret| secret == PRIVATE_SOP) {
            saw_private = true;
        }
        if secrets.iter().any(|secret| secret == DATE_SHAPED_SOP) {
            saw_date = true;
        }
        for secret in &secrets {
            assert!(!json.contains(secret), "{} leaked {secret}", fixture.name);
        }
    }
    assert!(saw_age);
    assert!(saw_name);
    assert!(saw_long);
    assert!(saw_private);
    assert!(saw_date);
}

#[test]
fn the_report_never_gates_open() {
    // The reproducer is the CT base plus PatientAge `040 `, a 170 character
    // LO, an empty SOP Instance UID and an empty Modality. `open_frame` is
    // the same before and after the check.
    let all = fixtures();
    let fixture = all.iter().find(|item| item.name == "reproducer");
    assert!(fixture.is_some());
    let Some(fixture) = fixture else {
        unreachable!("asserted immediately above")
    };
    let parsed = require(parse_part10(&encode(fixture)));
    let before = require(open_frame(&parsed, 0));
    let report = check_conformance(&parsed);
    let after = require(open_frame(&parsed, 0));
    assert_eq!(before, after);
    assert_only(
        &report,
        &[
            one(
                (0x0008, 0x0018),
                "SOPInstanceUID",
                ConformanceRule::Type1Empty,
                None,
                Measure::None,
            ),
            one(
                (0x0008, 0x0060),
                "Modality",
                ConformanceRule::Type1Empty,
                None,
                Measure::None,
            ),
            one(
                (0x0008, 0x0080),
                "InstitutionName",
                ConformanceRule::ValueLength,
                Some(1),
                chars(170, 64),
            ),
            one(
                (0x0010, 0x1010),
                "PatientAge",
                ConformanceRule::AsFormat,
                Some(1),
                Measure::None,
            ),
        ],
    );
}

#[test]
fn report_is_deterministic() {
    // Findings sort by path, then tag, then value, then rule. Two runs match
    // byte for byte. Value None sorts before Some on the same tag.
    let first = load("order");
    let second = load("order");
    assert_eq!(first.to_json(), second.to_json());
    assert!(first.to_json().starts_with("{\"scope\":"));
    assert_only(
        &first,
        &[
            one(
                (0x0010, 0x1010),
                "PatientAge",
                ConformanceRule::VrMismatch,
                None,
                Measure::None,
            ),
            one(
                (0x0010, 0x1010),
                "PatientAge",
                ConformanceRule::FixedLength,
                Some(1),
                bytes(4, 8),
            ),
            Want {
                tag: (0x0008, 0x0100),
                keyword: Some("CodeValue"),
                rule: ConformanceRule::Repertoire,
                value: Some(1),
                measure: character(3, CharacterClass::Control),
                path: &[(0x0054, 0x0410, 2)],
            },
        ],
    );
}

#[test]
fn scope_sentence_is_exact() {
    let report = load("valid-ct");
    let json = report.to_json();
    let quoted = format!("\"{CONFORMANCE_SCOPE}\"");
    assert!(json.contains(&quoted));
    assert!(json.starts_with(&format!("{{\"scope\":{quoted}")));
    assert!(json.contains("\"edition\":\"2026d\""));
    for rule in ocelli_dicom::ConformanceRule::ALL {
        assert!(json.contains(&format!("\"id\":\"{}\"", rule.id())));
        assert!(json.contains(&format!("\"section\":\"{}\"", rule.section())));
    }
}

#[test]
fn binary_length_follows_the_parser() {
    // PS3.5 6.4. A UL value whose length is not a multiple of 4 is a
    // BINARY-LENGTH finding when the parser admits the element. A two byte
    // UL is even, so the odd-length rule does not reject it first.
    let mut elements = ct_elements();
    elements.push(el((0x7FE1, 0x0010), *b"UL", vec![0, 0], Vec::new()));
    let fixture = fix("binary", CT_UID, elements);
    match parse_part10(&encode(&fixture)) {
        Err(error) => {
            // Measured on dicom-parser 0.10.0. A UL of two bytes is
            // TruncatedDataSet, so the element never reaches the checker.
            // BINARY-LENGTH stays in the catalogue for a parser that admits it.
            let text = format!("{error:?}");
            assert!(text.contains("TruncatedDataSet"), "{text}");
            assert!(ConformanceRule::ALL.contains(&ConformanceRule::BinaryLength));
        }
        Ok(parsed) => {
            let report = check_conformance(&parsed);
            assert!(
                report.findings().iter().any(|finding| {
                    finding.rule == ConformanceRule::BinaryLength && finding.measure == bytes(2, 4)
                }),
                "{}",
                describe(&report)
            );
        }
    }
}
