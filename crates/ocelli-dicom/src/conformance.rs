//! DICOM conformance report for the rules F-225 lists.
//!
//! `check_conformance` reads a parsed data set and changes nothing. The report
//! carries the rules it actually checked. It is not a statement that the file
//! conforms to DICOM.

use std::collections::BTreeMap;

use dicom_core::VR;
use dicom_core::dictionary::{DataDictionary, DataDictionaryEntry, VirtualVr};
use dicom_core::value::{PrimitiveValue, Value};
use dicom_object::{InMemDicomObject, StandardDataDictionary, Tag};

use crate::parse::ParsedDicom;
use crate::parse_decimal_string;

#[path = "conformance/vm.rs"]
mod vm;

use vm::vm_entry;
pub use vm::{VmEntry, VmVr, vm_entries};

/// The sentence every report carries.
pub const CONFORMANCE_SCOPE: &str = "This report checks the rules it lists and nothing else. It is not a statement that the file conforms to DICOM.";

/// DICOM edition the quoted rules were read from.
pub const CONFORMANCE_EDITION: &str = "2026d";

const IS_MIN: i64 = -2_147_483_648;
const IS_MAX: i64 = 2_147_483_647;
const BYTE_MAX: u32 = u32::MAX - 1;
/// PS3.5 6.2. A UID is at most 64 bytes, not counting one trailing NULL.
const UID_MAX_BYTES: usize = 64;
/// PS3.6. The DICOM root is the components 1, 2, 840, 10008.
/// The trailing dot is the component boundary, not another component.
const DICOM_UID_ROOT: &str = "1.2.840.10008.";
/// Wire text when (0008,0016) is present and is not a UID. Never the value.
const NOT_A_VALID_UID: &str = "not a valid UID";
/// Wire text when (0008,0016) is a UID outside the DICOM root. Never the value.
const NOT_A_DICOM_UID: &str = "not a DICOM UID";
/// Closed set of `sopClassNote` strings. Null means the UID was absent or repeated.
pub const SOP_CLASS_NOTES: &[&str] = &[NOT_A_VALID_UID, NOT_A_DICOM_UID];
/// Tails after ESC from PS3.3 C.12.1.1.2 Tables C.12-3 and C.12-4.
/// A column/row byte is 16 times the column plus the row. Longest match first.
const ISO2022_TAILS: &[&str] = &[
    "$(D", "$)C", "$)A", "(B", "(J", ")I", "-A", "-B", "-C", "-D", "-F", "-G", "-H", "-L", "-M",
    "-T", "-b", "$B",
];

const CT_UID: &str = "1.2.840.10008.5.1.4.1.1.2";
const CR_UID: &str = "1.2.840.10008.5.1.4.1.1.1";
const MR_UID: &str = "1.2.840.10008.5.1.4.1.1.4";
const PET_UID: &str = "1.2.840.10008.5.1.4.1.1.128";

const PATIENT: &str = "Patient";
const GENERAL_STUDY: &str = "General Study";
const GENERAL_SERIES: &str = "General Series";
const GENERAL_IMAGE: &str = "General Image";
const IMAGE_PLANE: &str = "Image Plane";
const IMAGE_PIXEL: &str = "Image Pixel";
const SOP_COMMON: &str = "SOP Common";

const MODULES_WITH_PLANE: &[&str] = &[
    PATIENT,
    GENERAL_STUDY,
    GENERAL_SERIES,
    GENERAL_IMAGE,
    IMAGE_PLANE,
    IMAGE_PIXEL,
    SOP_COMMON,
];
const MODULES_CR: &[&str] = &[
    PATIENT,
    GENERAL_STUDY,
    GENERAL_SERIES,
    GENERAL_IMAGE,
    IMAGE_PIXEL,
    SOP_COMMON,
];

/// Decision 4, in the order that sentence names them and then the module table.
const NOT_CHECKED: &[&str] = &[
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

const TYPE1_COMMON: &[Tag] = &[
    Tag(0x0020, 0x000D),
    Tag(0x0008, 0x0060),
    Tag(0x0020, 0x000E),
    Tag(0x0028, 0x0002),
    Tag(0x0028, 0x0004),
    Tag(0x0028, 0x0010),
    Tag(0x0028, 0x0011),
    Tag(0x0028, 0x0100),
    Tag(0x0028, 0x0101),
    Tag(0x0028, 0x0102),
    Tag(0x0028, 0x0103),
    Tag(0x0008, 0x0016),
    Tag(0x0008, 0x0018),
];

const TYPE1_PLANE: &[Tag] = &[
    Tag(0x0028, 0x0030),
    Tag(0x0020, 0x0037),
    Tag(0x0020, 0x0032),
];

const TYPE2_COMMON: &[Tag] = &[
    Tag(0x0010, 0x0010),
    Tag(0x0010, 0x0020),
    Tag(0x0010, 0x0030),
    Tag(0x0010, 0x0040),
    Tag(0x0008, 0x0020),
    Tag(0x0008, 0x0030),
    Tag(0x0008, 0x0090),
    Tag(0x0020, 0x0010),
    Tag(0x0008, 0x0050),
    Tag(0x0020, 0x0011),
    Tag(0x0020, 0x0013),
];

const PALETTE: &[Tag] = &[
    Tag(0x0028, 0x1101),
    Tag(0x0028, 0x1102),
    Tag(0x0028, 0x1103),
    Tag(0x0028, 0x1201),
    Tag(0x0028, 0x1202),
    Tag(0x0028, 0x1203),
];

const TAG_CHARSET: Tag = Tag(0x0008, 0x0005);
const TAG_SOP_CLASS: Tag = Tag(0x0008, 0x0016);
const TAG_SPP: Tag = Tag(0x0028, 0x0002);
const TAG_PHOTOMETRIC: Tag = Tag(0x0028, 0x0004);
const TAG_PLANAR: Tag = Tag(0x0028, 0x0006);
const TAG_PIXEL_DATA: Tag = Tag(0x7FE0, 0x0010);
const TAG_PROVIDER_URL: Tag = Tag(0x0028, 0x7FE0);
const TAG_OFFSET_TABLE: Tag = Tag(0x7FE0, 0x0001);
const TAG_OFFSET_LENGTHS: Tag = Tag(0x7FE0, 0x0002);
const TAG_POSITION: Tag = Tag(0x0018, 0x5100);
const TAG_ORIENTATION_CODE: Tag = Tag(0x0054, 0x0410);
const TAG_PATIENT_ORIENTATION: Tag = Tag(0x0020, 0x0020);
const TAG_SLICE: Tag = Tag(0x0018, 0x0050);

/// One rule the report can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConformanceRule {
    /// A value is longer than its VR maximum.
    ValueLength,
    /// A value is not the VR's fixed length.
    FixedLength,
    /// A character is outside the VR repertoire.
    Repertoire,
    /// Kept in the catalogue. The checker emits Type 1C on (0008,0005) instead.
    CharsetUndeclared,
    /// The encoded VR is not the dictionary VR.
    VrMismatch,
    /// The value count is outside the transcribed PS3.6 VM.
    Multiplicity,
    /// A binary value length is not a multiple of the element size.
    BinaryLength,
    /// An AE value is only spaces.
    AeSpaces,
    /// AS is not nnnD, nnnW, nnnM or nnnY.
    AsFormat,
    /// DA is not a Gregorian YYYYMMDD.
    DaFormat,
    /// TM is not HHMMSS.FFFFFF with the stated ranges.
    TmFormat,
    /// DT is not the stated date-time form.
    DtFormat,
    /// DS is outside the existing decimal-string grammar.
    DsFormat,
    /// DS matches the grammar and does not fit in binary64.
    DsRange,
    /// IS is not an optional sign and digits.
    IsFormat,
    /// IS is outside the signed 32-bit range.
    IsRange,
    /// UI breaks PS3.5 9.1.
    UiFormat,
    /// PN has more than four carets in a group or more than two equals signs.
    PnFormat,
    /// UR breaks the URI character or leading-space rule.
    UrFormat,
    /// A Type 1 attribute is missing.
    Type1Absent,
    /// A Type 1 attribute has no value.
    Type1Empty,
    /// A decidable Type 1C attribute is missing.
    Type1cAbsent,
    /// A decidable Type 1C attribute has no value.
    Type1cEmpty,
    /// A Type 2 attribute is missing.
    Type2Absent,
    /// A decidable Type 2C attribute is missing.
    Type2cAbsent,
}

impl ConformanceRule {
    /// Every rule, in declaration order.
    pub const ALL: &'static [ConformanceRule] = &[
        Self::ValueLength,
        Self::FixedLength,
        Self::Repertoire,
        Self::CharsetUndeclared,
        Self::VrMismatch,
        Self::Multiplicity,
        Self::BinaryLength,
        Self::AeSpaces,
        Self::AsFormat,
        Self::DaFormat,
        Self::TmFormat,
        Self::DtFormat,
        Self::DsFormat,
        Self::DsRange,
        Self::IsFormat,
        Self::IsRange,
        Self::UiFormat,
        Self::PnFormat,
        Self::UrFormat,
        Self::Type1Absent,
        Self::Type1Empty,
        Self::Type1cAbsent,
        Self::Type1cEmpty,
        Self::Type2Absent,
        Self::Type2cAbsent,
    ];

    /// Stable identifier used on the wire.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ValueLength => "VALUE-LENGTH",
            Self::FixedLength => "FIXED-LENGTH",
            Self::Repertoire => "REPERTOIRE",
            Self::CharsetUndeclared => "CHARSET-UNDECLARED",
            Self::VrMismatch => "VR-MISMATCH",
            Self::Multiplicity => "MULTIPLICITY",
            Self::BinaryLength => "BINARY-LENGTH",
            Self::AeSpaces => "AE-SPACES",
            Self::AsFormat => "AS-FORMAT",
            Self::DaFormat => "DA-FORMAT",
            Self::TmFormat => "TM-FORMAT",
            Self::DtFormat => "DT-FORMAT",
            Self::DsFormat => "DS-FORMAT",
            Self::DsRange => "DS-RANGE",
            Self::IsFormat => "IS-FORMAT",
            Self::IsRange => "IS-RANGE",
            Self::UiFormat => "UI-FORMAT",
            Self::PnFormat => "PN-FORMAT",
            Self::UrFormat => "UR-FORMAT",
            Self::Type1Absent => "TYPE1-ABSENT",
            Self::Type1Empty => "TYPE1-EMPTY",
            Self::Type1cAbsent => "TYPE1C-ABSENT",
            Self::Type1cEmpty => "TYPE1C-EMPTY",
            Self::Type2Absent => "TYPE2-ABSENT",
            Self::Type2cAbsent => "TYPE2C-ABSENT",
        }
    }

    /// DICOM section this rule cites.
    #[must_use]
    pub const fn section(self) -> &'static str {
        match self {
            Self::ValueLength
            | Self::FixedLength
            | Self::Repertoire
            | Self::BinaryLength
            | Self::AeSpaces
            | Self::AsFormat
            | Self::DaFormat
            | Self::TmFormat
            | Self::DtFormat
            | Self::DsFormat
            | Self::DsRange
            | Self::IsFormat
            | Self::IsRange
            | Self::PnFormat
            | Self::UrFormat => "PS3.5 6.2 Table 6.2-1",
            Self::CharsetUndeclared => "PS3.5 6.1.2.2",
            Self::VrMismatch => "PS3.5 6.2.2",
            Self::Multiplicity => "PS3.5 6.4",
            Self::UiFormat => "PS3.5 9.1",
            Self::Type1Absent | Self::Type1Empty => "PS3.5 7.4.1",
            Self::Type1cAbsent | Self::Type1cEmpty => "PS3.5 7.4.2",
            Self::Type2Absent => "PS3.5 7.4.3",
            Self::Type2cAbsent => "PS3.5 7.4.4",
        }
    }
}

/// One sequence step. `item` is 1-based.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathStep {
    /// Sequence tag.
    pub sequence: Tag,
    /// 1-based item number.
    pub item: u32,
}

/// Why a character was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterClass {
    /// A control the VR does not allow.
    Control,
    /// A backslash inside one value.
    Backslash,
    /// Outside the VR's default repertoire.
    OutsideRepertoire,
    /// A character above U+007F in a VR that cannot take an extended repertoire.
    NonDefault,
}

impl CharacterClass {
    const fn wire(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Backslash => "backslash",
            Self::OutsideRepertoire => "outside-repertoire",
            Self::NonDefault => "non-default",
        }
    }
}

/// A length, a count, or a character position. Never a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Measure {
    /// Byte length against the VR maximum or fixed length.
    Bytes {
        /// Bytes measured for this value.
        measured: u32,
        /// VR limit in bytes.
        limit: u32,
    },
    /// Character length against the VR maximum.
    Characters {
        /// Characters measured for this value or PN group.
        measured: u32,
        /// VR limit in characters.
        limit: u32,
    },
    /// Value count against the transcribed VM.
    Values {
        /// Values present.
        measured: u32,
        /// VM minimum.
        minimum: u32,
        /// VM maximum. Absent when PS3.6 says `n`.
        maximum: Option<u32>,
    },
    /// One character, 0-based.
    Character {
        /// Index in the value.
        index: u32,
        /// Why it failed.
        class: CharacterClass,
    },
    /// The rule has no measure.
    None,
}

/// One finding. It never carries a value, a substring, or a character code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceFinding {
    /// Attribute tag.
    pub tag: Tag,
    /// Dictionary keyword, when the tag is standard.
    pub keyword: Option<&'static str>,
    /// Sequence path from the root. Empty at the root.
    pub path: Vec<PathStep>,
    /// Rule identifier.
    pub rule: ConformanceRule,
    /// 1-based value number when the rule is per value.
    pub value: Option<u32>,
    /// Measure, or [`Measure::None`].
    pub measure: Measure,
}

/// Whether the seven modules were applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleScope {
    /// SOP Class UID selected one of the four IODs.
    Evaluated {
        /// Canonical SOP Class UID.
        sop_class: &'static str,
        /// IOD name.
        iod: &'static str,
        /// PS3.3 IOD table.
        table: &'static str,
        /// Module names in IOD order.
        modules: &'static [&'static str],
    },
    /// Any other SOP Class UID, or none.
    NotEvaluated {
        /// Dataset SOP Class UID when it is a well-formed UID under the DICOM root.
        sop_class: Option<String>,
        /// Why the value was not repeated. One of [`SOP_CLASS_NOTES`], or none.
        sop_class_note: Option<&'static str>,
    },
}

/// The whole report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceReport {
    findings: Vec<ConformanceFinding>,
    modules: ModuleScope,
    elements: u32,
    not_checked_un: u32,
}

impl ConformanceReport {
    /// Findings sorted by path, tag, value, then rule.
    #[must_use]
    pub fn findings(&self) -> &[ConformanceFinding] {
        &self.findings
    }

    /// Module scope for this SOP Class UID.
    #[must_use]
    pub fn modules(&self) -> &ModuleScope {
        &self.modules
    }

    /// Undecidable attributes, by keyword. The same list for every report.
    #[must_use]
    pub fn not_checked(&self) -> &'static [&'static str] {
        NOT_CHECKED
    }

    /// Elements visited, including elements inside items.
    #[must_use]
    pub fn elements(&self) -> u32 {
        self.elements
    }

    /// Elements encoded as UN. Their values were not checked.
    #[must_use]
    pub fn not_checked_un(&self) -> u32 {
        self.not_checked_un
    }

    /// JSON object. Key order is scope, edition, rules, modules, counts, findings.
    ///
    /// The crate's `serde_json` map sorts keys, so the object is written out
    /// in the order above.
    #[must_use]
    pub fn to_json(&self) -> String {
        json_object(&[
            ("scope", json_string(CONFORMANCE_SCOPE)),
            ("edition", json_string(CONFORMANCE_EDITION)),
            ("rules", rules_json()),
            ("modules", modules_json(&self.modules)),
            (
                "counts",
                json_object(&[
                    ("elements", json_u32(self.elements)),
                    ("notCheckedUn", json_u32(self.not_checked_un)),
                ]),
            ),
            ("findings", findings_json(&self.findings)),
        ])
    }
}

/// Check one parsed Part 10 object. Infallible. Does not modify `parsed`.
#[must_use]
pub fn check_conformance(parsed: &ParsedDicom) -> ConformanceReport {
    let mut state = State::default();
    walk(parsed.object(), &mut state, &[]);
    state.finish()
}

struct Iod {
    uid: &'static str,
    iod: &'static str,
    table: &'static str,
    modules: &'static [&'static str],
    image_plane: bool,
    patient_position: bool,
    patient_orientation: bool,
}

const IODS: &[Iod] = &[
    Iod {
        uid: CR_UID,
        iod: "CR Image",
        table: "PS3.3 A.2.3",
        modules: MODULES_CR,
        image_plane: false,
        patient_position: false,
        patient_orientation: true,
    },
    Iod {
        uid: CT_UID,
        iod: "CT Image",
        table: "PS3.3 A.3.3",
        modules: MODULES_WITH_PLANE,
        image_plane: true,
        patient_position: true,
        patient_orientation: false,
    },
    Iod {
        uid: MR_UID,
        iod: "MR Image",
        table: "PS3.3 A.4.3",
        modules: MODULES_WITH_PLANE,
        image_plane: true,
        patient_position: true,
        patient_orientation: false,
    },
    Iod {
        uid: PET_UID,
        iod: "PET Image",
        table: "PS3.3 A.21.3",
        modules: MODULES_WITH_PLANE,
        image_plane: true,
        patient_position: false,
        patient_orientation: false,
    },
];

#[derive(Default)]
struct State {
    findings: Vec<ConformanceFinding>,
    elements: u32,
    not_checked_un: u32,
    expanded: bool,
    utf8: bool,
    /// PS3.3 C.12.1.1.2. Some Specific Character Set value uses code extension.
    iso2022: bool,
    /// Root tags. The bool is true when the element has no value.
    root: BTreeMap<Tag, bool>,
    spp: Option<u16>,
    photometric: Option<String>,
    sop_class: Option<String>,
}

impl State {
    fn finish(mut self) -> ConformanceReport {
        let modules = module_scope(self.sop_class.as_deref());
        if let ModuleScope::Evaluated { .. } = modules {
            self.apply_presence(iod_of(self.sop_class.as_deref()));
        }
        self.findings.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.tag.cmp(&right.tag))
                .then(left.value.cmp(&right.value))
                .then(left.rule.cmp(&right.rule))
        });
        ConformanceReport {
            findings: self.findings,
            modules,
            elements: self.elements,
            not_checked_un: self.not_checked_un,
        }
    }

    fn apply_presence(&mut self, iod: Option<&Iod>) {
        let Some(iod) = iod else {
            return;
        };
        for tag in TYPE1_COMMON {
            self.type1(*tag);
        }
        for tag in TYPE2_COMMON {
            self.type2(*tag);
        }
        if iod.image_plane {
            for tag in TYPE1_PLANE {
                self.type1(*tag);
            }
            self.type2(TAG_SLICE);
        }
        if iod.patient_position && self.absent(TAG_ORIENTATION_CODE) {
            self.type2c(TAG_POSITION);
        }
        if iod.patient_orientation {
            self.type2c(TAG_PATIENT_ORIENTATION);
        }
        if self.spp.is_some_and(|samples| samples > 1) {
            self.type1c(TAG_PLANAR);
        }
        if self.photometric.as_deref() == Some("PALETTE COLOR") {
            for tag in PALETTE {
                self.type1c(*tag);
            }
        }
        let url_carries_pixels = self.present(TAG_PROVIDER_URL) && !self.blank(TAG_PROVIDER_URL);
        if !url_carries_pixels {
            self.type1c(TAG_PIXEL_DATA);
        }
        if self.present(TAG_OFFSET_TABLE) {
            self.type1c(TAG_OFFSET_LENGTHS);
        }
        if self.expanded {
            if self.absent(TAG_CHARSET) {
                self.push(
                    TAG_CHARSET,
                    &[],
                    ConformanceRule::Type1cAbsent,
                    None,
                    Measure::None,
                );
            } else if self.blank(TAG_CHARSET) {
                self.push(
                    TAG_CHARSET,
                    &[],
                    ConformanceRule::Type1cEmpty,
                    None,
                    Measure::None,
                );
            }
        }
    }

    fn present(&self, tag: Tag) -> bool {
        self.root.contains_key(&tag)
    }

    fn absent(&self, tag: Tag) -> bool {
        !self.present(tag)
    }

    fn blank(&self, tag: Tag) -> bool {
        self.root.get(&tag).copied().unwrap_or(false)
    }

    fn type1(&mut self, tag: Tag) {
        if self.absent(tag) {
            self.push(tag, &[], ConformanceRule::Type1Absent, None, Measure::None);
        } else if self.blank(tag) {
            self.push(tag, &[], ConformanceRule::Type1Empty, None, Measure::None);
        }
    }

    fn type2(&mut self, tag: Tag) {
        if self.absent(tag) {
            self.push(tag, &[], ConformanceRule::Type2Absent, None, Measure::None);
        }
    }

    fn type1c(&mut self, tag: Tag) {
        if self.absent(tag) {
            self.push(tag, &[], ConformanceRule::Type1cAbsent, None, Measure::None);
        } else if self.blank(tag) {
            self.push(tag, &[], ConformanceRule::Type1cEmpty, None, Measure::None);
        }
    }

    fn type2c(&mut self, tag: Tag) {
        if self.absent(tag) {
            self.push(tag, &[], ConformanceRule::Type2cAbsent, None, Measure::None);
        }
    }

    fn push(
        &mut self,
        tag: Tag,
        path: &[PathStep],
        rule: ConformanceRule,
        value: Option<u32>,
        measure: Measure,
    ) {
        self.findings.push(ConformanceFinding {
            tag,
            keyword: keyword_of(tag),
            path: path.to_vec(),
            rule,
            value,
            measure,
        });
    }
}

fn module_scope(sop: Option<&str>) -> ModuleScope {
    match iod_of(sop) {
        Some(iod) => ModuleScope::Evaluated {
            sop_class: iod.uid,
            iod: iod.iod,
            table: iod.table,
            modules: iod.modules,
        },
        None => unevaluated_scope(sop),
    }
}

/// Repeat a SOP Class UID only under the DICOM root.
///
/// PS3.5 6.2 puts one trailing NULL on a Value Field of odd length. That pad
/// belongs to the field, so [`ui_field_values`] removes it from the last value
/// only. PS3.5 9.1 then decides whether the text is a UID. [`ui_grammar`] does
/// not remove a NULL. [`valid_ui`] does, and a second removal would repeat an
/// even-length UID that still holds one NULL. PS3.6 registers every standard
/// UID under the components 1, 2, 840, 10008, and one more component is
/// required. A private UID and a single component of digits are well formed
/// and are not repeated.
fn unevaluated_scope(sop: Option<&str>) -> ModuleScope {
    let Some(field) = sop.filter(|value| !value.is_empty()) else {
        return ModuleScope::NotEvaluated {
            sop_class: None,
            sop_class_note: None,
        };
    };
    let values = ui_field_values(field);
    let uid = values.first().copied().unwrap_or("");
    if !uid.is_empty() && uid.len() <= UID_MAX_BYTES && ui_grammar(uid) {
        if under_dicom_root(uid) {
            ModuleScope::NotEvaluated {
                sop_class: Some(uid.to_owned()),
                sop_class_note: None,
            }
        } else {
            ModuleScope::NotEvaluated {
                sop_class: None,
                sop_class_note: Some(NOT_A_DICOM_UID),
            }
        }
    } else if uid.is_empty() {
        ModuleScope::NotEvaluated {
            sop_class: None,
            sop_class_note: None,
        }
    } else {
        ModuleScope::NotEvaluated {
            sop_class: None,
            sop_class_note: Some(NOT_A_VALID_UID),
        }
    }
}

/// PS3.6. True when `uid` components begin with 1, 2, 840, 10008 and at least
/// one further component is present.
///
/// `DICOM_UID_ROOT` ends with the boundary dot. Without that dot the constant
/// is only the character prefix `1.2.840.10008`, which also matches
/// `1.2.840.100081.2` and the bare root. The fixtures refuse that prefix.
fn under_dicom_root(uid: &str) -> bool {
    let Some(root) = DICOM_UID_ROOT.strip_suffix('.') else {
        return uid.starts_with(DICOM_UID_ROOT);
    };
    if root.is_empty() {
        return false;
    }
    let mut components = uid.split('.');
    for expected in root.split('.') {
        if expected.is_empty() {
            return false;
        }
        match components.next() {
            Some(component) if component == expected => {}
            _ => return false,
        }
    }
    matches!(components.next(), Some(component) if !component.is_empty())
}

fn iod_of(sop: Option<&str>) -> Option<&'static Iod> {
    // PS3.5 6.2. One trailing NULL on the last value is the pad. Any other
    // NULL is not an IOD UID.
    let field = sop?;
    let values = ui_field_values(field);
    let uid = values.first().copied()?;
    IODS.iter().find(|iod| iod.uid == uid)
}

fn keyword_of(tag: Tag) -> Option<&'static str> {
    StandardDataDictionary
        .by_tag(tag)
        .map(DataDictionaryEntry::alias)
}

fn walk(object: &InMemDicomObject<StandardDataDictionary>, state: &mut State, path: &[PathStep]) {
    for element in object {
        let tag = element.header().tag;
        let vr = element.vr();
        state.elements = state.elements.saturating_add(1);
        let root = path.is_empty();
        if vr == VR::UN {
            state.not_checked_un = state.not_checked_un.saturating_add(1);
            if root {
                state.root.insert(tag, value_is_blank(vr, element.value()));
            }
            continue;
        }
        if !vr_matches(vr, tag) {
            state.push(tag, path, ConformanceRule::VrMismatch, None, Measure::None);
        }
        match element.value() {
            Value::Sequence(sequence) => {
                let blank = sequence.items().is_empty();
                if root {
                    state.root.insert(tag, blank);
                }
                if blank {
                    continue;
                }
                check_vm(state, tag, path, 1);
                for (index, item) in sequence.items().iter().enumerate() {
                    let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
                    let mut nested = path.to_vec();
                    nested.push(PathStep {
                        sequence: tag,
                        item: number,
                    });
                    walk(item, state, &nested);
                }
            }
            Value::PixelSequence(_) => {
                if root {
                    state.root.insert(tag, false);
                }
                check_vm(state, tag, path, 1);
            }
            Value::Primitive(primitive) => {
                let blank = primitive_blank_value(vr, primitive);
                if root {
                    state.root.insert(tag, blank);
                    note_root_facts(state, tag, primitive, blank);
                }
                if blank {
                    continue;
                }
                let count = value_count(vr, primitive);
                check_vm(state, tag, path, count);
                check_binary_length(state, tag, path, vr, element.header().len.get());
                check_text(state, tag, path, vr, primitive);
            }
        }
    }
}

fn note_root_facts(state: &mut State, tag: Tag, primitive: &PrimitiveValue, blank: bool) {
    if tag == TAG_SOP_CLASS
        && !blank
        && let Some(parts) = text_parts(primitive)
    {
        // PS3.5 6.2. Keep the Value Field. Readers remove one trailing NULL
        // from the last value and from nowhere else.
        let field = parts.join("\\");
        if ui_field_values(&field)
            .iter()
            .any(|value| !value.is_empty())
        {
            state.sop_class = Some(field);
        }
    }
    if tag == TAG_CHARSET
        && !blank
        && let Some(parts) = text_parts(primitive)
    {
        if let Some(text) = parts.first() {
            let code = text.trim_matches(' ');
            state.utf8 = code == "ISO_IR 192";
        }
        state.iso2022 = parts
            .iter()
            .any(|part| part.trim_matches(' ').starts_with("ISO 2022"));
    }
    if tag == TAG_SPP
        && let PrimitiveValue::U16(values) = primitive
        && values.len() == 1
    {
        state.spp = values.first().copied();
    }
    if tag == TAG_PHOTOMETRIC
        && !blank
        && let Some(text) = first_text(primitive)
    {
        state.photometric = Some(text.trim_matches(' ').to_owned());
    }
}

fn value_is_blank<I, P>(vr: VR, value: &Value<I, P>) -> bool {
    match value {
        Value::Primitive(primitive) => primitive_blank_value(vr, primitive),
        Value::Sequence(sequence) => sequence.items().is_empty(),
        Value::PixelSequence(_) => false,
    }
}

/// PS3.5 6.4 and 7.4.1. A Value Field is blank when it has no non-empty
/// value once the one trailing pad is removed from the last value.
///
/// Delimiters alone, with or without that pad, are treated like an empty
/// Data Element Value and do not satisfy Type 1.
fn primitive_blank_value(vr: VR, primitive: &PrimitiveValue) -> bool {
    match text_parts(primitive) {
        Some(parts) => field_values(vr, parts).iter().all(|value| value.is_empty()),
        None => primitive.multiplicity() == 0,
    }
}

fn value_count(vr: VR, primitive: &PrimitiveValue) -> u32 {
    // PS3.5 6.4. These VRs have one value. The primitive stores bytes or words.
    if matches!(
        vr,
        VR::OB | VR::OD | VR::OF | VR::OL | VR::OV | VR::OW | VR::UN
    ) {
        return 1;
    }
    match text_parts(primitive) {
        Some(parts) => u32::try_from(parts.len()).unwrap_or(u32::MAX),
        None => primitive.multiplicity(),
    }
}

fn first_text(primitive: &PrimitiveValue) -> Option<&str> {
    text_parts(primitive)
        .and_then(|parts| parts.first())
        .map(String::as_str)
}

fn text_parts(primitive: &PrimitiveValue) -> Option<&[String]> {
    match primitive {
        PrimitiveValue::Strs(parts) => Some(parts.as_slice()),
        PrimitiveValue::Str(part) => Some(std::slice::from_ref(part)),
        _ => None,
    }
}

fn vr_matches(vr: VR, tag: Tag) -> bool {
    let Some(entry) = StandardDataDictionary.by_tag(tag) else {
        return true;
    };
    match entry.vr() {
        VirtualVr::Exact(expected) => vr == expected,
        VirtualVr::Xs => vr == VR::US || vr == VR::SS,
        VirtualVr::Px | VirtualVr::Ox => vr == VR::OB || vr == VR::OW,
        VirtualVr::Lt => vr == VR::US || vr == VR::OW,
        _ => false,
    }
}

fn check_vm(state: &mut State, tag: Tag, path: &[PathStep], measured: u32) {
    let Some(entry) = vm_entry(tag) else {
        return;
    };
    if measured == 0 {
        return;
    }
    let above = entry.maximum.is_some_and(|limit| measured > limit);
    if measured < entry.minimum || above {
        state.push(
            tag,
            path,
            ConformanceRule::Multiplicity,
            None,
            Measure::Values {
                measured,
                minimum: entry.minimum,
                maximum: entry.maximum,
            },
        );
    }
}

fn check_binary_length(
    state: &mut State,
    tag: Tag,
    path: &[PathStep],
    vr: VR,
    header_len: Option<u32>,
) {
    let Some(width) = binary_width(vr) else {
        return;
    };
    let Some(len) = header_len else {
        return;
    };
    if width == 0 || len.is_multiple_of(width) {
        return;
    }
    state.push(
        tag,
        path,
        ConformanceRule::BinaryLength,
        None,
        Measure::Bytes {
            measured: len,
            limit: width,
        },
    );
}

fn binary_width(vr: VR) -> Option<u32> {
    match vr {
        VR::US | VR::SS | VR::OW => Some(2),
        VR::UL | VR::SL | VR::FL | VR::AT | VR::OL | VR::OF => Some(4),
        VR::UV | VR::SV | VR::FD | VR::OD | VR::OV => Some(8),
        _ => None,
    }
}

fn check_text(state: &mut State, tag: Tag, path: &[PathStep], vr: VR, primitive: &PrimitiveValue) {
    let Some(parts) = text_parts(primitive) else {
        return;
    };
    let total = u32::try_from(parts.len()).unwrap_or(u32::MAX);
    // PS3.5 6.2 and 6.4. The parser has already split on backslash. One
    // function removes the pad from the last value. One part is the whole
    // field, so `valid_ui` applies that function.
    let one_ui = vr == VR::UI && parts.len() == 1;
    let values = field_values(vr, parts);
    for (index, part) in parts.iter().enumerate() {
        let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        let last_of_many = total > 1 && number == total;
        // PS3.5 6.4. A value of a multi-valued string may be empty. It is
        // still counted by multiplicity, and no grammar describes it.
        let value = values.get(index).copied().unwrap_or(part.as_str());
        if value.is_empty() {
            continue;
        }
        if notes_expansion(vr, part) {
            state.expanded = true;
        }
        if recorded_length_or_repertoire(state, tag, path, vr, part, number, last_of_many) {
            continue;
        }
        if vr == VR::UI {
            let failed = if one_ui {
                !valid_ui(part)
            } else {
                !ui_grammar(value)
            };
            if failed {
                state.push(
                    tag,
                    path,
                    ConformanceRule::UiFormat,
                    Some(number),
                    Measure::None,
                );
            }
            continue;
        }
        let body = without_pad_exception(vr, part, last_of_many, state.utf8);
        if let Some(rule) = format_finding(vr, body) {
            state.push(tag, path, rule, Some(number), Measure::None);
        }
    }
}

fn recorded_length_or_repertoire(
    state: &mut State,
    tag: Tag,
    path: &[PathStep],
    vr: VR,
    raw: &str,
    number: u32,
    last_of_many: bool,
) -> bool {
    if let Some((rule, measure)) = length_finding(vr, raw, last_of_many, state.utf8) {
        state.push(tag, path, rule, Some(number), measure);
        return true;
    }
    let body = without_pad_exception(vr, raw, last_of_many, state.utf8);
    if let Some((index, class)) = repertoire_finding(vr, body, state.iso2022) {
        state.push(
            tag,
            path,
            ConformanceRule::Repertoire,
            Some(number),
            Measure::Character { index, class },
        );
        return true;
    }
    false
}

struct Bound {
    measured: u32,
    limit: u32,
    fixed: bool,
    pad: char,
    characters: bool,
}

fn text_bound(vr: VR, raw: &str, utf8: bool) -> Option<Bound> {
    match vr {
        VR::LO | VR::SH | VR::ST | VR::LT => {
            let limit = match vr {
                VR::SH => 16,
                VR::LO => 64,
                VR::ST => 1024,
                _ => 10240,
            };
            Some(Bound {
                measured: character_len(raw),
                limit,
                fixed: false,
                pad: ' ',
                characters: true,
            })
        }
        VR::AE | VR::CS | VR::DS | VR::DT | VR::IS | VR::TM | VR::UI | VR::UC | VR::UR | VR::UT => {
            let (limit, fixed) = byte_limit(vr)?;
            Some(Bound {
                measured: measured_bytes(raw, vr, utf8),
                limit,
                fixed,
                pad: pad_char(vr),
                characters: false,
            })
        }
        VR::AS | VR::DA => {
            let limit = if vr == VR::AS { 4 } else { 8 };
            Some(Bound {
                measured: measured_bytes(raw, vr, utf8),
                limit,
                fixed: true,
                pad: ' ',
                characters: false,
            })
        }
        _ => None,
    }
}

fn length_finding(
    vr: VR,
    raw: &str,
    last_of_many: bool,
    utf8: bool,
) -> Option<(ConformanceRule, Measure)> {
    if vr == VR::PN {
        return pn_length(raw, last_of_many).map(|measured| {
            (
                ConformanceRule::ValueLength,
                Measure::Characters {
                    measured,
                    limit: 64,
                },
            )
        });
    }
    let bound = text_bound(vr, raw, utf8)?;
    if !exceeds(
        bound.measured,
        bound.limit,
        bound.fixed,
        raw,
        last_of_many,
        bound.pad,
    ) {
        return None;
    }
    let rule = if bound.fixed {
        ConformanceRule::FixedLength
    } else {
        ConformanceRule::ValueLength
    };
    let measure = if bound.characters {
        Measure::Characters {
            measured: bound.measured,
            limit: bound.limit,
        }
    } else {
        Measure::Bytes {
            measured: bound.measured,
            limit: bound.limit,
        }
    };
    Some((rule, measure))
}

/// PS3.5 6.4. Drop the one pad byte before the format check.
///
/// The length check already allows that byte on the last value. PN keeps its
/// pad because each component group has its own length.
fn without_pad_exception(vr: VR, raw: &str, last_of_many: bool, utf8: bool) -> &str {
    let Some(bound) = text_bound(vr, raw, utf8) else {
        return raw;
    };
    if last_of_many && bound.measured == bound.limit.saturating_add(1) && raw.ends_with(bound.pad) {
        raw.strip_suffix(bound.pad).unwrap_or(raw)
    } else {
        raw
    }
}

/// Character length of an LO, SH, ST, LT or PN group.
///
/// PS3.5 6.2 Note. The count is characters after decode, not DICOM bytes.
fn character_len(raw: &str) -> u32 {
    u32::try_from(raw.chars().count()).unwrap_or(u32::MAX)
}

fn pn_length(raw: &str, last_of_many: bool) -> Option<u32> {
    let mut over = None;
    let groups = raw.split('=').collect::<Vec<_>>();
    let group_total = u32::try_from(groups.len()).unwrap_or(u32::MAX);
    for (index, group) in groups.iter().enumerate() {
        let number = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        let last_group = number == group_total;
        let measured = character_len(group);
        if exceeds(measured, 64, false, group, last_of_many && last_group, ' ') {
            over = Some(measured);
            break;
        }
    }
    over
}

fn byte_limit(vr: VR) -> Option<(u32, bool)> {
    match vr {
        VR::AE | VR::CS => Some((16, false)),
        VR::DS => Some((16, false)),
        VR::DT => Some((26, false)),
        VR::IS => Some((12, false)),
        VR::TM => Some((14, false)),
        VR::UI => Some((64, false)),
        VR::UC | VR::UR | VR::UT => Some((BYTE_MAX, false)),
        _ => None,
    }
}

fn measured_bytes(raw: &str, vr: VR, utf8: bool) -> u32 {
    let single_byte = matches!(vr, VR::AE | VR::CS | VR::AS | VR::UR);
    let count = if single_byte || !utf8 {
        raw.chars().count()
    } else {
        raw.len()
    };
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn pad_char(vr: VR) -> char {
    if vr == VR::UI { '\0' } else { ' ' }
}

/// PS3.5 6.2 and 6.4. The one-byte pad exception applies only to the last
/// value of a multi-valued field. A single value is held to the limit exactly.
fn exceeds(
    measured: u32,
    limit: u32,
    fixed: bool,
    raw: &str,
    last_of_many: bool,
    pad: char,
) -> bool {
    let inside = if fixed {
        measured == limit
    } else {
        measured <= limit
    };
    if inside {
        return false;
    }
    let padded = last_of_many && measured == limit.saturating_add(1) && raw.ends_with(pad);
    !padded
}

fn notes_expansion(vr: VR, raw: &str) -> bool {
    matches!(
        vr,
        VR::SH | VR::LO | VR::UC | VR::ST | VR::LT | VR::UT | VR::PN
    ) && raw.chars().any(|ch| ch > '\u{007F}' || ch == '\u{001B}')
}

fn repertoire_finding(vr: VR, raw: &str, iso2022: bool) -> Option<(u32, CharacterClass)> {
    let text = repertoire_body(vr, raw);
    let mut index: u32 = 0;
    let mut rest = text;
    while let Some(ch) = rest.chars().next() {
        let width = ch.len_utf8();
        if ch == '\u{001B}' && iso2022 && extended_string(vr) {
            let after = rest.get(width..).unwrap_or("");
            if let Some(tail) = iso2022_tail(after) {
                let tail_chars = u32::try_from(tail.chars().count()).unwrap_or(u32::MAX);
                index = index.saturating_add(1).saturating_add(tail_chars);
                let advance = width.saturating_add(tail.len());
                rest = rest.get(advance..).unwrap_or("");
                continue;
            }
        }
        if let Some(class) = char_problem(vr, ch) {
            return Some((index, class));
        }
        index = index.saturating_add(1);
        rest = rest.get(width..).unwrap_or("");
    }
    None
}

fn extended_string(vr: VR) -> bool {
    matches!(
        vr,
        VR::SH | VR::LO | VR::UC | VR::PN | VR::ST | VR::LT | VR::UT
    )
}

/// PS3.3 C.12.1.1.2. The bytes after ESC, when they are a registered sequence.
fn iso2022_tail(after: &str) -> Option<&'static str> {
    ISO2022_TAILS
        .iter()
        .copied()
        .find(|tail| after.starts_with(tail))
}

fn repertoire_body(vr: VR, raw: &str) -> &str {
    match vr {
        VR::AS | VR::DA => raw.strip_suffix(' ').unwrap_or(raw),
        _ => raw,
    }
}

/// PS3.5 6.2. Split a UI Value Field on backslash and remove at most one
/// trailing NULL from the last value.
///
/// The pad belongs to the Value Field. A NULL on any earlier value is part
/// of that value.
fn ui_field_values(field: &str) -> Vec<&str> {
    let mut values: Vec<&str> = field.split('\\').collect();
    strip_field_pad(&mut values, '\0');
    values
}

/// PS3.5 6.2 and 6.4. The values of a split character string Value Field,
/// with at most one trailing pad removed from the last value only. The pad
/// is NULL for UI and SPACE otherwise.
fn field_values(vr: VR, parts: &[String]) -> Vec<&str> {
    let mut values: Vec<&str> = parts.iter().map(String::as_str).collect();
    strip_field_pad(&mut values, pad_char(vr));
    values
}

/// PS3.5 6.4. "a single padding character shall be applied to the end of
/// the Value Field (to the last Value)".
fn strip_field_pad(values: &mut [&str], pad: char) {
    if let Some(last) = values.last_mut() {
        *last = last.strip_suffix(pad).unwrap_or(*last);
    }
}

fn char_problem(vr: VR, ch: char) -> Option<CharacterClass> {
    match vr {
        VR::SH | VR::LO | VR::UC | VR::PN | VR::ST | VR::LT | VR::UT => extended_problem(vr, ch),
        VR::CS => cs_problem(ch),
        VR::AE => ae_problem(ch),
        VR::AS => as_problem(ch),
        VR::DA => digit_only_problem(ch),
        VR::TM => tm_problem(ch),
        VR::DT => dt_problem(ch),
        VR::DS | VR::IS => high_only(ch),
        VR::UI => ui_problem(ch),
        VR::UR => ur_char_problem(ch),
        _ => None,
    }
}

fn high_only(ch: char) -> Option<CharacterClass> {
    (ch > '\u{007F}').then_some(CharacterClass::NonDefault)
}

fn extended_problem(vr: VR, ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return None;
    }
    if ch == '\\' {
        return if matches!(vr, VR::LT | VR::ST | VR::UT) {
            None
        } else {
            Some(CharacterClass::Backslash)
        };
    }
    // PS3.5 Table 6.2-1. ESC is a control unless a registered ISO 2022
    // sequence was skipped before this check.
    if ch == '\u{001B}' {
        return Some(CharacterClass::Control);
    }
    if ch.is_control() {
        let allowed =
            matches!(ch, '\t' | '\n' | '\r' | '\u{000C}') && matches!(vr, VR::LT | VR::ST | VR::UT);
        return if allowed {
            None
        } else {
            Some(CharacterClass::Control)
        };
    }
    if (' '..='~').contains(&ch) {
        None
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn cs_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if matches!(ch, 'A'..='Z' | '0'..='9' | ' ' | '_') {
        return None;
    }
    if ch.is_control() {
        Some(CharacterClass::Control)
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn ae_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if ch == '\\' {
        return Some(CharacterClass::Backslash);
    }
    if ch.is_control() {
        return Some(CharacterClass::Control);
    }
    if (' '..='~').contains(&ch) {
        None
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn as_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if matches!(ch, '0'..='9' | 'D' | 'W' | 'M' | 'Y') {
        return None;
    }
    if ch.is_control() {
        Some(CharacterClass::Control)
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn digit_only_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if ch.is_ascii_digit() {
        return None;
    }
    if ch.is_control() {
        Some(CharacterClass::Control)
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn tm_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if matches!(ch, '0'..='9' | '.' | ' ') {
        return None;
    }
    if ch.is_control() {
        Some(CharacterClass::Control)
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn dt_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if matches!(ch, '0'..='9' | '+' | '-' | '.' | ' ') {
        return None;
    }
    if ch.is_control() {
        Some(CharacterClass::Control)
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn ui_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if ch.is_control() && ch != '\0' {
        Some(CharacterClass::Control)
    } else {
        None
    }
}

fn ur_char_problem(ch: char) -> Option<CharacterClass> {
    if ch > '\u{007F}' {
        return Some(CharacterClass::NonDefault);
    }
    if ur_char(ch) || ch == ' ' {
        return None;
    }
    if ch.is_control() {
        Some(CharacterClass::Control)
    } else {
        Some(CharacterClass::OutsideRepertoire)
    }
}

fn ur_char(ch: char) -> bool {
    matches!(
        ch,
        'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | '-'
            | '.'
            | '_'
            | '~'
            | ':'
            | '/'
            | '?'
            | '#'
            | '['
            | ']'
            | '@'
            | '!'
            | '$'
            | '&'
            | '\''
            | '('
            | ')'
            | '*'
            | '+'
            | ','
            | ';'
            | '='
            | '%'
    )
}

fn format_finding(vr: VR, raw: &str) -> Option<ConformanceRule> {
    let broken = match vr {
        VR::AS => !valid_as(raw),
        VR::DA => !valid_da(raw),
        VR::TM => !valid_tm(raw),
        VR::DT => !valid_dt(raw),
        VR::DS => !valid_ds(raw),
        VR::IS => !valid_is(raw),
        VR::PN => !valid_pn(raw),
        VR::AE => raw.trim_matches(' ').is_empty(),
        VR::UR => !valid_ur(raw),
        _ => false,
    };
    if !broken {
        return None;
    }
    Some(match vr {
        VR::AS => ConformanceRule::AsFormat,
        VR::DA => ConformanceRule::DaFormat,
        VR::TM => ConformanceRule::TmFormat,
        VR::DT => ConformanceRule::DtFormat,
        VR::DS => {
            if ds_overflow(raw) {
                ConformanceRule::DsRange
            } else {
                ConformanceRule::DsFormat
            }
        }
        VR::IS => {
            if is_range(raw) {
                ConformanceRule::IsRange
            } else {
                ConformanceRule::IsFormat
            }
        }
        VR::PN => ConformanceRule::PnFormat,
        VR::AE => ConformanceRule::AeSpaces,
        VR::UR => ConformanceRule::UrFormat,
        _ => return None,
    })
}

fn valid_as(raw: &str) -> bool {
    let text = raw.strip_suffix(' ').unwrap_or(raw);
    let mut chars = text.chars();
    let first = chars.next();
    let second = chars.next();
    let third = chars.next();
    let unit = chars.next();
    if chars.next().is_some() {
        return false;
    }
    let Some(unit) = unit else {
        return false;
    };
    let digits = matches!(
        (first, second, third),
        (Some(a), Some(b), Some(c))
            if a.is_ascii_digit() && b.is_ascii_digit() && c.is_ascii_digit()
    );
    digits && matches!(unit, 'D' | 'W' | 'M' | 'Y')
}

fn valid_da(raw: &str) -> bool {
    if raw.len() != 8 || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Some((year, rest)) = take_digits(raw, 4) else {
        return false;
    };
    let Some((month, rest)) = take_digits(rest, 2) else {
        return false;
    };
    let Some((day, rest)) = take_digits(rest, 2) else {
        return false;
    };
    rest.is_empty() && gregorian_day(year, month, day)
}

/// Gregorian leap rule. A year divisible by 100 is a leap year only when it
/// is also divisible by 400. 1900 is not. 2000 is.
fn is_gregorian_leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn gregorian_day(year: u32, month: u32, day: u32) -> bool {
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_gregorian_leap(year) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    (1..=max).contains(&day)
}

fn valid_tm(raw: &str) -> bool {
    let text = raw.trim_end_matches(' ');
    if text.is_empty() || text.starts_with(' ') || text.contains(' ') {
        return false;
    }
    let (body, fraction) = match text.split_once('.') {
        Some((body, fraction)) => (body, Some(fraction)),
        None => (text, None),
    };
    if !body.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    if fraction.is_some() && body.len() != 6 {
        return false;
    }
    if !matches!(body.len(), 2 | 4 | 6) {
        return false;
    }
    if let Some(fraction) = fraction
        && (fraction.is_empty()
            || fraction.len() > 6
            || !fraction.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return false;
    }
    let Some((hour, rest)) = take_digits(body, 2) else {
        return false;
    };
    if hour > 23 {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    let Some((minute, rest)) = take_digits(rest, 2) else {
        return false;
    };
    if minute > 59 {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    let Some((second, rest)) = take_digits(rest, 2) else {
        return false;
    };
    rest.is_empty() && second <= 60
}

fn valid_dt(raw: &str) -> bool {
    let text = raw.trim_end_matches(' ');
    if text.is_empty() || text.starts_with(' ') || text.contains(' ') {
        return false;
    }
    let (body, offset) = split_offset(text);
    if let Some((sign, digits)) = offset {
        if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        let Some((hour, rest)) = take_digits(digits, 2) else {
            return false;
        };
        let Some((minute, rest)) = take_digits(rest, 2) else {
            return false;
        };
        if !rest.is_empty() || !valid_offset(sign, hour, minute) {
            return false;
        }
    }
    valid_dt_body(body)
}

fn split_offset(text: &str) -> (&str, Option<(char, &str)>) {
    let mut found = None;
    for (index, ch) in text.char_indices() {
        if ch == '+' || ch == '-' {
            found = Some((index, ch));
        }
    }
    let Some((index, sign)) = found else {
        return (text, None);
    };
    let body = text.get(..index).unwrap_or("");
    let digits = text
        .get(index.saturating_add(sign.len_utf8())..)
        .unwrap_or("");
    (body, Some((sign, digits)))
}

fn valid_offset(sign: char, hour: u32, minute: u32) -> bool {
    if minute > 59 {
        return false;
    }
    if sign == '-' && hour == 0 && minute == 0 {
        return false;
    }
    if sign == '-' {
        return hour < 12 || (hour == 12 && minute == 0);
    }
    if sign == '+' {
        return hour < 14 || (hour == 14 && minute == 0);
    }
    false
}

fn valid_dt_body(body: &str) -> bool {
    let (digits, fraction) = match body.split_once('.') {
        Some((digits, fraction)) => (digits, Some(fraction)),
        None => (body, None),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    if fraction.is_some() && digits.len() != 14 {
        return false;
    }
    if !matches!(digits.len(), 4 | 6 | 8 | 10 | 12 | 14) {
        return false;
    }
    if let Some(fraction) = fraction
        && (fraction.is_empty()
            || fraction.len() > 6
            || !fraction.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return false;
    }
    let Some((year, rest)) = take_digits(digits, 4) else {
        return false;
    };
    if rest.is_empty() {
        return true;
    }
    let Some((month, rest)) = take_digits(rest, 2) else {
        return false;
    };
    if !(1..=12).contains(&month) {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    let Some((day, rest)) = take_digits(rest, 2) else {
        return false;
    };
    if !gregorian_day(year, month, day) {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    let Some((hour, rest)) = take_digits(rest, 2) else {
        return false;
    };
    if hour > 23 {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    let Some((minute, rest)) = take_digits(rest, 2) else {
        return false;
    };
    if minute > 59 {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    let Some((second, rest)) = take_digits(rest, 2) else {
        return false;
    };
    rest.is_empty() && second <= 60
}

fn valid_ds(raw: &str) -> bool {
    parse_decimal_string(raw).is_some()
}

fn ds_overflow(raw: &str) -> bool {
    let text = raw.trim_matches(' ');
    if text.is_empty() {
        return false;
    }
    let chars_ok = text
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'e' | b'E'));
    if !chars_ok || !text.bytes().any(|byte| byte == b'e' || byte == b'E') {
        return false;
    }
    match text.parse::<f64>() {
        Ok(value) => !value.is_finite(),
        Err(_) => false,
    }
}

fn valid_is(raw: &str) -> bool {
    matches!(classify_is(raw), Some(IsClass::Ok))
}

fn is_range(raw: &str) -> bool {
    matches!(classify_is(raw), Some(IsClass::Range))
}

enum IsClass {
    Ok,
    Range,
}

fn classify_is(raw: &str) -> Option<IsClass> {
    let text = raw.trim_matches(' ');
    if text.is_empty() {
        return None;
    }
    let digits = text
        .strip_prefix('+')
        .or_else(|| text.strip_prefix('-'))
        .unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value = text.parse::<i64>().ok()?;
    if (IS_MIN..=IS_MAX).contains(&value) {
        Some(IsClass::Ok)
    } else {
        Some(IsClass::Range)
    }
}

/// PS3.5 6.2 and 9.1. One Value Field, with at most one trailing NULL removed
/// from its last value.
///
/// A second call removes one more NULL. Scope must use [`ui_grammar`] on the
/// value [`ui_field_values`] already returned.
fn valid_ui(raw: &str) -> bool {
    ui_field_values(raw).iter().copied().all(ui_grammar)
}

/// PS3.5 9.1. Digits and dots, no leading zero, and no NULL.
///
/// The caller has already applied the PS3.5 6.2 pad rule. This function does
/// not remove a pad byte.
fn ui_grammar(text: &str) -> bool {
    if text.is_empty() || text.contains(' ') || text.contains('\0') {
        return false;
    }
    if text.starts_with('.') || text.ends_with('.') {
        return false;
    }
    for part in text.split('.') {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
    }
    true
}

fn valid_pn(raw: &str) -> bool {
    let text = raw.strip_suffix(' ').unwrap_or(raw);
    let groups = text.split('=').collect::<Vec<_>>();
    if groups.len() > 3 {
        return false;
    }
    groups.iter().all(|group| group.matches('^').count() <= 4)
}

fn valid_ur(raw: &str) -> bool {
    if raw.starts_with(' ') {
        return false;
    }
    let text = raw.trim_end_matches(' ');
    !text.is_empty() && text.chars().all(ur_char)
}

fn take_digits(text: &str, count: usize) -> Option<(u32, &str)> {
    let mut value: u32 = 0;
    let mut rest = text;
    for _ in 0..count {
        let mut chars = rest.chars();
        let ch = chars.next()?;
        let digit = ch.to_digit(10)?;
        rest = chars.as_str();
        value = value.checked_mul(10)?.checked_add(digit)?;
    }
    Some((value, rest))
}

fn rules_json() -> String {
    let rules = ConformanceRule::ALL
        .iter()
        .map(|rule| {
            json_object(&[
                ("id", json_string(rule.id())),
                ("section", json_string(rule.section())),
            ])
        })
        .collect::<Vec<_>>();
    json_array(&rules)
}

fn modules_json(modules: &ModuleScope) -> String {
    match modules {
        ModuleScope::Evaluated {
            sop_class,
            iod,
            table,
            modules,
        } => {
            let names = modules
                .iter()
                .map(|name| json_string(name))
                .collect::<Vec<_>>();
            let skipped = NOT_CHECKED
                .iter()
                .map(|name| json_string(name))
                .collect::<Vec<_>>();
            json_object(&[
                ("evaluated", json_bool(true)),
                ("sopClass", json_string(sop_class)),
                ("sopClassNote", "null".to_owned()),
                ("iod", json_string(iod)),
                ("table", json_string(table)),
                ("modules", json_array(&names)),
                ("notChecked", json_array(&skipped)),
            ])
        }
        ModuleScope::NotEvaluated {
            sop_class,
            sop_class_note,
        } => json_object(&[
            ("evaluated", json_bool(false)),
            (
                "sopClass",
                sop_class
                    .as_ref()
                    .map(|uid| json_string(uid))
                    .unwrap_or_else(|| "null".to_owned()),
            ),
            (
                "sopClassNote",
                sop_class_note
                    .map(json_string)
                    .unwrap_or_else(|| "null".to_owned()),
            ),
        ]),
    }
}

fn findings_json(findings: &[ConformanceFinding]) -> String {
    let rows = findings.iter().map(finding_json).collect::<Vec<_>>();
    json_array(&rows)
}

fn finding_json(finding: &ConformanceFinding) -> String {
    let path = finding
        .path
        .iter()
        .map(|step| {
            json_object(&[
                ("sequence", json_string(&tag_text(step.sequence))),
                ("item", json_u32(step.item)),
            ])
        })
        .collect::<Vec<_>>();
    json_object(&[
        ("tag", json_string(&tag_text(finding.tag))),
        (
            "keyword",
            finding
                .keyword
                .map(json_string)
                .unwrap_or_else(|| "null".to_owned()),
        ),
        ("path", json_array(&path)),
        ("rule", json_string(finding.rule.id())),
        (
            "value",
            finding
                .value
                .map(json_u32)
                .unwrap_or_else(|| "null".to_owned()),
        ),
        ("measure", measure_json(finding.measure)),
    ])
}

fn measure_json(measure: Measure) -> String {
    match measure {
        Measure::None => "null".to_owned(),
        Measure::Bytes { measured, limit } => json_object(&[
            ("unit", json_string("bytes")),
            ("measured", json_u32(measured)),
            ("limit", json_u32(limit)),
        ]),
        Measure::Characters { measured, limit } => json_object(&[
            ("unit", json_string("characters")),
            ("measured", json_u32(measured)),
            ("limit", json_u32(limit)),
        ]),
        Measure::Values {
            measured,
            minimum,
            maximum,
        } => json_object(&[
            ("unit", json_string("values")),
            ("measured", json_u32(measured)),
            ("minimum", json_u32(minimum)),
            (
                "maximum",
                maximum.map(json_u32).unwrap_or_else(|| "null".to_owned()),
            ),
        ]),
        Measure::Character { index, class } => json_object(&[
            ("unit", json_string("character")),
            ("index", json_u32(index)),
            ("class", json_string(class.wire())),
        ]),
    }
}

fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
}

fn json_u32(value: u32) -> String {
    value.to_string()
}

fn json_bool(value: bool) -> String {
    if value {
        "true".to_owned()
    } else {
        "false".to_owned()
    }
}

fn json_array(items: &[String]) -> String {
    let mut out = String::from("[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(item);
    }
    out.push(']');
    out
}

fn json_object(pairs: &[(&str, String)]) -> String {
    let mut out = String::from("{");
    for (index, (key, value)) in pairs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&json_string(key));
        out.push(':');
        out.push_str(value);
    }
    out.push('}');
    out
}

fn tag_text(tag: Tag) -> String {
    format!("({:04X},{:04X})", tag.0, tag.1)
}

#[cfg(test)]
mod iso2022_table {
    use super::ISO2022_TAILS;

    /// Column/row byte from PS3.3 C.12.1.1.2. 16 times the column plus the row.
    fn iso2022_bytes(pairs: &[(u8, u8)]) -> String {
        pairs
            .iter()
            .map(|(column, row)| char::from(column.saturating_mul(16).saturating_add(*row)))
            .collect()
    }

    /// PS3.3 2026d C.12.1.1.2 Tables C.12-3 and C.12-4.
    ///
    /// Each row is the ESC Sequence column, written as column/row pairs. ESC
    /// itself is not part of the tail. Table C.12-3 lists two sequences for
    /// ISO 2022 IR 13. The pairs are quoted from those tables.
    #[test]
    fn iso2022_tails_match_ps33_tables() {
        let rows: &[(&str, &[(u8, u8)])] = &[
            ("ISO 2022 IR 6", &[(2, 8), (4, 2)]),
            ("ISO 2022 IR 100", &[(2, 13), (4, 1)]),
            ("ISO 2022 IR 101", &[(2, 13), (4, 2)]),
            ("ISO 2022 IR 109", &[(2, 13), (4, 3)]),
            ("ISO 2022 IR 110", &[(2, 13), (4, 4)]),
            ("ISO 2022 IR 144", &[(2, 13), (4, 12)]),
            ("ISO 2022 IR 127", &[(2, 13), (4, 7)]),
            ("ISO 2022 IR 126", &[(2, 13), (4, 6)]),
            ("ISO 2022 IR 138", &[(2, 13), (4, 8)]),
            ("ISO 2022 IR 148", &[(2, 13), (4, 13)]),
            ("ISO 2022 IR 13", &[(2, 9), (4, 9)]),
            ("ISO 2022 IR 13", &[(2, 8), (4, 10)]),
            ("ISO 2022 IR 166", &[(2, 13), (5, 4)]),
            ("ISO 2022 IR 203", &[(2, 13), (6, 2)]),
            ("ISO 2022 IR 87", &[(2, 4), (4, 2)]),
            ("ISO 2022 IR 159", &[(2, 4), (2, 8), (4, 4)]),
            ("ISO 2022 IR 149", &[(2, 4), (2, 9), (4, 3)]),
            ("ISO 2022 IR 58", &[(2, 4), (2, 9), (4, 1)]),
        ];
        assert_eq!(rows.len(), 18);
        let mut expected = Vec::new();
        for (term, pairs) in rows {
            let tail = iso2022_bytes(pairs);
            assert!(!tail.is_empty(), "{term}");
            assert!(term.starts_with("ISO 2022 IR "), "{term}");
            expected.push(tail);
        }
        let mut actual = ISO2022_TAILS
            .iter()
            .map(|tail| (*tail).to_owned())
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
    }
}

#[cfg(test)]
mod ui_field {
    use super::ui_field_values;

    /// PS3.5 6.2. One trailing NULL pads the Value Field. Only the last value
    /// of a multi-valued element may carry it.
    #[test]
    fn pad_byte_is_only_on_the_last_value() {
        assert_eq!(ui_field_values("1.2.3\0\\1.2.5"), vec!["1.2.3\0", "1.2.5"]);
        assert_eq!(ui_field_values("1.2.3\\1.2.5\0"), vec!["1.2.3", "1.2.5"]);
        assert_eq!(
            ui_field_values("1.2.840.10008.1.20\0\0"),
            vec!["1.2.840.10008.1.20\0"]
        );
        assert_eq!(ui_field_values("1.2.3\0"), vec!["1.2.3"]);
    }
}

#[cfg(test)]
mod dicom_root {
    use super::under_dicom_root;

    /// PS3.6. The root is the components 1, 2, 840, 10008. One more component
    /// is required, and that component is not empty. A shared character prefix
    /// is a different UID.
    #[test]
    fn dicom_root_is_a_component_boundary() {
        assert!(under_dicom_root("1.2.840.10008.1"));
        assert!(under_dicom_root("1.2.840.10008.5.1.4.1.1.7"));
        assert!(!under_dicom_root("1.2.840.10008"));
        assert!(!under_dicom_root("1.2.840.100081.2"));
        assert!(!under_dicom_root("1.2.840.10008."));
        assert!(!under_dicom_root("1.2.3.4"));
    }
}
