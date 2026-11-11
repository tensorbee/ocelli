//! PS3.6 2026d Table 6-1, the VR and VM of the attributes the presence rules name.
//!
//! Forty-seven tags. The plan's range rows for the palette descriptors and
//! palette data are one tag each. VM is not in `dicom-dictionary-std`. This
//! table is the only VM source the checker uses.

use dicom_core::VR;
use dicom_object::Tag;

/// VR cell of one PS3.6 row, including the dictionary's alternative forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmVr {
    /// The dictionary names one VR.
    Exact(VR),
    /// US or SS. Palette lookup table descriptors.
    UsOrSs,
    /// OB or OW. Pixel Data.
    ObOrOw,
}

/// One transcribed PS3.6 row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmEntry {
    /// Attribute tag.
    pub tag: Tag,
    /// PS3.6 keyword.
    pub keyword: &'static str,
    /// VR cell.
    pub vr: VmVr,
    /// Minimum value count.
    pub minimum: u32,
    /// Maximum value count. Absent means the PS3.6 `n` side.
    pub maximum: Option<u32>,
}

const fn row(
    tag: Tag,
    keyword: &'static str,
    vr: VmVr,
    minimum: u32,
    maximum: Option<u32>,
) -> VmEntry {
    VmEntry {
        tag,
        keyword,
        vr,
        minimum,
        maximum,
    }
}

const fn exact(vr: VR) -> VmVr {
    VmVr::Exact(vr)
}

/// The 47 tags, in the order PS3.6 was transcribed for F-225.
pub const VM_ENTRIES: &[VmEntry] = &[
    row(
        Tag(0x0010, 0x0010),
        "PatientName",
        exact(VR::PN),
        1,
        Some(1),
    ),
    row(Tag(0x0010, 0x0020), "PatientID", exact(VR::LO), 1, Some(1)),
    row(
        Tag(0x0010, 0x0030),
        "PatientBirthDate",
        exact(VR::DA),
        1,
        Some(1),
    ),
    row(Tag(0x0010, 0x0040), "PatientSex", exact(VR::CS), 1, Some(1)),
    row(Tag(0x0010, 0x1010), "PatientAge", exact(VR::AS), 1, Some(1)),
    row(
        Tag(0x0020, 0x000D),
        "StudyInstanceUID",
        exact(VR::UI),
        1,
        Some(1),
    ),
    row(Tag(0x0008, 0x0020), "StudyDate", exact(VR::DA), 1, Some(1)),
    row(Tag(0x0008, 0x0030), "StudyTime", exact(VR::TM), 1, Some(1)),
    row(
        Tag(0x0008, 0x0090),
        "ReferringPhysicianName",
        exact(VR::PN),
        1,
        Some(1),
    ),
    row(Tag(0x0020, 0x0010), "StudyID", exact(VR::SH), 1, Some(1)),
    row(
        Tag(0x0008, 0x0050),
        "AccessionNumber",
        exact(VR::SH),
        1,
        Some(1),
    ),
    row(Tag(0x0008, 0x0060), "Modality", exact(VR::CS), 1, Some(1)),
    row(
        Tag(0x0020, 0x000E),
        "SeriesInstanceUID",
        exact(VR::UI),
        1,
        Some(1),
    ),
    row(
        Tag(0x0020, 0x0011),
        "SeriesNumber",
        exact(VR::IS),
        1,
        Some(1),
    ),
    row(Tag(0x0020, 0x0060), "Laterality", exact(VR::CS), 1, Some(1)),
    row(
        Tag(0x0018, 0x5100),
        "PatientPosition",
        exact(VR::CS),
        1,
        Some(1),
    ),
    row(
        Tag(0x0054, 0x0410),
        "PatientOrientationCodeSequence",
        exact(VR::SQ),
        1,
        Some(1),
    ),
    row(
        Tag(0x0020, 0x0013),
        "InstanceNumber",
        exact(VR::IS),
        1,
        Some(1),
    ),
    row(
        Tag(0x0020, 0x0020),
        "PatientOrientation",
        exact(VR::CS),
        2,
        Some(2),
    ),
    row(
        Tag(0x0008, 0x0023),
        "ContentDate",
        exact(VR::DA),
        1,
        Some(1),
    ),
    row(
        Tag(0x0008, 0x0033),
        "ContentTime",
        exact(VR::TM),
        1,
        Some(1),
    ),
    row(
        Tag(0x0028, 0x0030),
        "PixelSpacing",
        exact(VR::DS),
        2,
        Some(2),
    ),
    row(
        Tag(0x0020, 0x0037),
        "ImageOrientationPatient",
        exact(VR::DS),
        6,
        Some(6),
    ),
    row(
        Tag(0x0020, 0x0032),
        "ImagePositionPatient",
        exact(VR::DS),
        3,
        Some(3),
    ),
    row(
        Tag(0x0018, 0x0050),
        "SliceThickness",
        exact(VR::DS),
        1,
        Some(1),
    ),
    row(
        Tag(0x0028, 0x0002),
        "SamplesPerPixel",
        exact(VR::US),
        1,
        Some(1),
    ),
    row(
        Tag(0x0028, 0x0004),
        "PhotometricInterpretation",
        exact(VR::CS),
        1,
        Some(1),
    ),
    row(Tag(0x0028, 0x0010), "Rows", exact(VR::US), 1, Some(1)),
    row(Tag(0x0028, 0x0011), "Columns", exact(VR::US), 1, Some(1)),
    row(
        Tag(0x0028, 0x0100),
        "BitsAllocated",
        exact(VR::US),
        1,
        Some(1),
    ),
    row(Tag(0x0028, 0x0101), "BitsStored", exact(VR::US), 1, Some(1)),
    row(Tag(0x0028, 0x0102), "HighBit", exact(VR::US), 1, Some(1)),
    row(
        Tag(0x0028, 0x0103),
        "PixelRepresentation",
        exact(VR::US),
        1,
        Some(1),
    ),
    row(
        Tag(0x0028, 0x0006),
        "PlanarConfiguration",
        exact(VR::US),
        1,
        Some(1),
    ),
    row(
        Tag(0x0028, 0x1101),
        "RedPaletteColorLookupTableDescriptor",
        VmVr::UsOrSs,
        3,
        Some(3),
    ),
    row(
        Tag(0x0028, 0x1102),
        "GreenPaletteColorLookupTableDescriptor",
        VmVr::UsOrSs,
        3,
        Some(3),
    ),
    row(
        Tag(0x0028, 0x1103),
        "BluePaletteColorLookupTableDescriptor",
        VmVr::UsOrSs,
        3,
        Some(3),
    ),
    row(
        Tag(0x0028, 0x1201),
        "RedPaletteColorLookupTableData",
        exact(VR::OW),
        1,
        Some(1),
    ),
    row(
        Tag(0x0028, 0x1202),
        "GreenPaletteColorLookupTableData",
        exact(VR::OW),
        1,
        Some(1),
    ),
    row(
        Tag(0x0028, 0x1203),
        "BluePaletteColorLookupTableData",
        exact(VR::OW),
        1,
        Some(1),
    ),
    row(Tag(0x7FE0, 0x0010), "PixelData", VmVr::ObOrOw, 1, Some(1)),
    row(
        Tag(0x0028, 0x7FE0),
        "PixelDataProviderURL",
        exact(VR::UR),
        1,
        Some(1),
    ),
    row(
        Tag(0x7FE0, 0x0001),
        "ExtendedOffsetTable",
        exact(VR::OV),
        1,
        Some(1),
    ),
    row(
        Tag(0x7FE0, 0x0002),
        "ExtendedOffsetTableLengths",
        exact(VR::OV),
        1,
        Some(1),
    ),
    row(
        Tag(0x0008, 0x0016),
        "SOPClassUID",
        exact(VR::UI),
        1,
        Some(1),
    ),
    row(
        Tag(0x0008, 0x0018),
        "SOPInstanceUID",
        exact(VR::UI),
        1,
        Some(1),
    ),
    row(
        Tag(0x0008, 0x0005),
        "SpecificCharacterSet",
        exact(VR::CS),
        1,
        None,
    ),
];

/// The transcribed rows.
#[must_use]
pub fn vm_entries() -> &'static [VmEntry] {
    VM_ENTRIES
}

pub(super) fn vm_entry(tag: Tag) -> Option<&'static VmEntry> {
    VM_ENTRIES.iter().find(|entry| entry.tag == tag)
}
