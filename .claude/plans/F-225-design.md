# F-225, DICOM conformance report for the listed rules

**Status**: approved
**Epic ref**: E3.8
**Sprint**: S46
**Estimate**: 4w

Proposed story row for `docs/sprints/BACKLOG.md`, section M20: "DICOM conformance report: value checks for every PS3.5 Table 6.2-1 VR, value multiplicity for the declared module attributes, Type 1, 1C, 2 and 2C presence for seven PS3.3 modules on four SOP classes, exposed through @ocelli/core and shown in the folder viewer's headers sheet, never stopping display". Layer All, 4w, depends on F-216 and F-213, milestone M20, phase P1.

Proposed `allocation.json` entry: `eid` E3.8, `epic` "E3 DICOM ingest & metadata", `epic_key` E3, `depends_eids` E38.26 and E38.23, `depends_fids` F-216 and F-213, `notes` "Asked by the operator on 2026-11-05 after a team found that files cornerstone displayed were invalid DICOM that another viewer refused: an AS value without its unit letter, a 170-character LO value and empty Type 1 elements. Placed in S46 by the operator the same day. Its page.html, serve.rs, use.html and more.html edits serialise after F-215, the last viewer story in the sprint order." E3 ends at E3.7 in both `docs/sprints/BACKLOG.md` and `allocation.json`, so E3.8 is the next free number. E3 is the fitting epic because the checks are DICOM ingest, the same epic as `parse_part10` (E3.1) and the metadata model (E3.2).

The operator's instruction, relayed by the lead on 2026-11-05: make Ocelli a better option than legacy cornerstone for the issues the team hit, with every resulting story in S46.

## Normative source, transcribed

The HLD contains no conformance checker, no VR validation and no Type 1 presence rule. `grep -rn -i 'conformance\|value representation' docs/hld/*.md` finds only section 25's test category "Conformance, each transfer syntax decodes correctly", which is a codec test row and not this. The normative source is therefore the DICOM standard, edition 2026d, fetched on 2026-11-05 from dicom.nema.org (PS3.5 `part05.html`, PS3.3 `chtml/part03` section pages, PS3.6 `part06.xml`) into the session scratchpad and read there. Every rule this plan implements is quoted below. Nothing in this section is paraphrase.

### PS3.5 2026d section 6.2, Value Representation

"Values with VRs constructed of character strings, except in the case of the VR UI, shall be padded with SPACE characters (20H, in the Default Character Repertoire) when necessary to achieve even length. Values with a VR of UI shall be padded with a single trailing NULL (00H) character when necessary to achieve even length."

"An individual Value, including padding, shall not exceed the Length of Value, except in the case of the last Value of a multi-valued field as specified in Section 6.4."

Note: "The lengths of Value Representations for which the Character Repertoire can be extended or replaced are expressly specified in characters rather than bytes in Table 6.2-1. This is because the mapping from a character to the number of bytes used for that character's encoding may be dependent on the character set used."

### PS3.5 2026d Table 6.2-1, the rows this story checks

Quoted from the table's Definition, Character Repertoire and Length of Value columns. Clauses that apply only "in the context of a Query" are quoted where they appear and are handled by decision 9.

| VR | Definition, quoted | Character Repertoire, quoted | Length of Value, quoted |
|----|--------------------|------------------------------|-------------------------|
| AE | "A string of characters that identifies an Application Entity with leading and trailing spaces (20H) being non-significant. A Value consisting solely of spaces shall not be used." | "Default Character Repertoire excluding character code 5CH (the BACKSLASH "\" in ISO-IR 6), and all control characters." | "16 bytes maximum" |
| AS | "A string of characters with one of the following formats -- nnnD, nnnW, nnnM, nnnY; where nnn shall contain the number of days for D, weeks for W, months for M, or years for Y. Example: "018M" would represent an age of 18 months." | ""0"-"9", "D", "W", "M", "Y" of Default Character Repertoire" | "4 bytes fixed" |
| AT | "Ordered pair of 16-bit unsigned integers that is the Value of a Data Element Tag." | "not applicable" | "4 bytes fixed" |
| CS | "A string of characters identifying a controlled concept. Leading or trailing spaces (20H) are not significant." | "Uppercase characters, "0"-"9", the SPACE character, and underscore "_", of the Default Character Repertoire" | "16 bytes maximum" |
| DA | "A string of characters of the format YYYYMMDD; where YYYY shall contain year, MM shall contain the month, and DD shall contain the day, interpreted as a date of the Gregorian calendar system." | ""0"-"9" of Default Character Repertoire" | "8 bytes fixed" |
| DS | "A string of characters representing either a fixed point number or a floating point number. A fixed point number shall contain only the characters 0-9 with an optional leading "+" or "-" and an optional "." to mark the decimal point. A floating point number shall be conveyed as defined in ANSI X3.9, with an "E" or "e" to indicate the start of the exponent. Decimal Strings may be padded with leading or trailing spaces. Embedded spaces are not allowed." | ""0"-"9", "+", "-", "E", "e", "." and the SPACE character of Default Character Repertoire" | "16 bytes maximum" |
| DT | "A concatenated date-time character string in the format: YYYYMMDDHHMMSS.FFFFFF&ZZXX ... HH = Hour (range "00" - "23"), MM = Minute (range "00" - "59"), SS = Second (range "00" - "60"). FFFFFF = Fractional Second ... (range "000000" - "999999"). &ZZXX is an optional suffix for offset from Coordinated Universal Time (UTC), where & = "+" or "-", and ZZ = Hours and XX = Minutes of offset. ... The Fractional Second component, if present, shall contain 1 to 6 digits. If Fractional Second is unspecified the preceding "." shall not be included. The offset suffix, if present, shall contain 4 digits. The string may be padded with trailing SPACE characters. Leading and embedded spaces are not allowed. ... The YYYY component shall not be null. Non-trailing null components are prohibited." Note: "The range of the offset is -1200 to +1400. ... The RFC 2822 use of -0000 as an offset to indicate local time is not allowed. A Date Time Value of 195308 means August 1953 ... The offset may be included regardless of null components, e.g., 2007-0500 is a legal Value." | ""0"-"9", "+", "-", "." and the SPACE character of Default Character Repertoire" | "26 bytes maximum" |
| FL, FD | "All [IEEE 754] values are permitted, including NaN (Not a Number) and infinity values." | "not applicable" | "4 bytes fixed", "8 bytes fixed" |
| IS | "A string of characters representing an Integer in base-10 (decimal), shall contain only the characters 0 - 9, with an optional leading "+" or "-". It may be padded with leading and/or trailing spaces. Embedded spaces are not allowed. The integer, n, represented shall be in the range: -2^31 <= n <= (2^31 -1)." | ""0"-"9", "+", "-" and the SPACE character of Default Character Repertoire" | "12 bytes maximum" |
| LO | "A character string that may be padded with leading and/or trailing spaces. The character code 5CH (the BACKSLASH "\" in ISO-IR 6) shall not be present, as it is used as the delimiter between Values in multi-valued Data Elements. The string shall not have Control Characters except for ESC." | "Default Character Repertoire and/or as defined by (0008,0005) excluding character code 5CH ... and all Control Characters except ESC when used for [ISO/IEC 2022] escape sequences." | "64 chars maximum (see Note in Section 6.2)" |
| LT | "It may contain the Graphic Character set and the Control Characters, TAB (see Note 3), CR, LF, FF, and ESC. It may be padded with trailing spaces, which may be ignored, but leading spaces are considered to be significant. Data Elements with this VR shall not be multi-valued" | "Default Character Repertoire and/or as defined by (0008,0005) excluding Control Characters except TAB, LF, FF, CR (and ESC when used for [ISO/IEC 2022] escape sequences)." | "10240 chars maximum" |
| OB, OD, OF, OL, OV, OW | stream definitions | "not applicable" | "2^32-2 bytes maximum" (OB, OW), "2^32-8" (OD, OV), "2^32-4" (OF, OL) |
| PN | "The character code 5CH ... shall not be present ... The component delimiter shall be the caret "^" character (5EH). There shall be no more than four component delimiters ... The delimiter for component groups shall be the equals character "=" (3DH). There shall be no more than two component group delimiters" | "Default Character Repertoire and/or as defined by (0008,0005) excluding character code 5CH ... and all Control Characters except ESC when used for [ISO/IEC 2022] escape sequences." | "64 chars maximum per component group" |
| SH | "The character code 05CH ... shall not be present ... The string shall not have Control Characters except ESC." | as LO | "16 chars maximum" |
| SL, SS, SV, UL, US, UV | binary integers of 32, 16, 64, 32, 16 and 64 bits | "not applicable" | "4 bytes fixed", "2 bytes fixed", "8 bytes fixed", "4 bytes fixed", "2 bytes fixed", "8 bytes fixed" |
| SQ | "Value is a Sequence of zero or more Items" | "not applicable" | "2^32-2 bytes maximum" |
| ST | as LT | as LT | "1024 chars maximum" |
| TM | "A string of characters of the format HHMMSS.FFFFFF, where HH contains hours (range "00" - "23"), MM contains minutes (range "00" - "59"), SS contains seconds (range "00" - "60"), and FFFFFF contains a fractional part of a second ... (range "000000" - "999999"). ... Midnight shall be represented by only "0000" since "2400" would violate the hour range. The string may be padded with trailing spaces. Leading and embedded spaces are not allowed. One or more of the components MM, SS, or FFFFFF may be unspecified as long as every component to the right of an unspecified component is also unspecified ... The FFFFFF component, if present, shall contain 1 to 6 digits. If FFFFFF is unspecified the preceding "." shall not be included. Examples: "070907.0705 " represents a time of 7 hours, 9 minutes and 7.0705 seconds. "1010" represents a time of 10 hours, and 10 minutes. "021 " is an invalid Value." | ""0"-"9", "." and the SPACE character of Default Character Repertoire" | "14 bytes maximum" |
| UC | "may be padded with trailing spaces. The character code 5CH ... shall not be present ... The string shall not have Control Characters except for ESC." | as LO | "2^32-2 bytes maximum" |
| UI | "The UID is a series of numeric components separated by the period "." character. If a Value Field containing one or more UIDs is an odd number of bytes in length, the Value Field shall be padded with a single trailing NULL (00H) character" | ""0"-"9", "." of Default Character Repertoire" | "64 bytes maximum" |
| UN | "An octet-stream where the encoding of the contents is unknown" | "not applicable" | "Any length valid for any of the other DICOM Value Representations" |
| UR | "Leading spaces are not allowed. Trailing spaces shall be ignored. Data Elements with this VR shall not be multi-valued." | "The subset of the Default Character Repertoire required for the URI as defined in IETF RFC3986 Section 2, plus the space (20H) character permitted only as trailing padding." | "2^32-2 bytes maximum" |
| UT | as LT | as LT | "2^32-2 bytes maximum" |

### PS3.5 2026d section 6.1.2.2, Extension or Replacement of the Default Character Repertoire

"For Data Elements with Value Representations of SH (Short String), LO (Long String), UC (Unlimited Characters), ST (Short Text), LT (Long Text), UT (Unlimited Text) or PN (Person Name) the Default Character Repertoire may be extended or replaced ... If such an extension or replacement is used, the relevant "Specific Character Set" shall be defined as an Attribute of the SOP Common Module (0008,0005)". Section 6.1.2.3 note: "This replacement character repertoire does not apply to other textual Value Representations (AE and CS)."

### PS3.5 2026d section 6.4, Value Multiplicity and Delimitation

"The VM of each Data Element is specified explicitly in PS3.6. If the number of Values that may be encoded in a Data Element is variable, it shall be represented by two numbers separated by a dash, e.g., "1-10" means that there may be 1 to 10 Values in the Data Element." (The source sentence uses a semicolon before "e.g.", replaced here by a comma under the voice rule.)

"For character strings, the character 5CH (BACKSLASH "\" in the case of the repertoire ISO IR-6) shall be used as a delimiter between Values."

"Each string Value in a multi-valued character string may be of even or odd length, but the length of the entire Value Field (including "\" delimiters) shall be of even length. If padding is required to make the Value Field of even length, a single padding character shall be applied to the end of the Value Field (to the last Value), in which case the length of the last Value may exceed the length of Value by 1."

"Each string Value in a multi-valued character string may be empty, unless otherwise specified. The presence of one or more delimiter (BACKSLASH) characters alone, without any Values, is treated like an empty Data Element Value, even if the Value Length is greater than zero."

"Data Elements with a VR of LT, OB, OD, OF, OL, OV, OW, SQ, ST, UN, UR or UT shall always have a Value Multiplicity of one."

### PS3.5 2026d section 7.4, Data Element Type

7.4.1: "IODs and SOP Classes define Type 1 Data Elements that shall be included and are mandatory Data Elements. The Value Field shall contain valid data as defined by the Data Element's VR and VM as specified in PS3.6. The Length of the Value Field shall not be zero. Absence of a valid Value in a Type 1 Data Element is a protocol violation." And: "For Data Elements with a string rather than binary, text or sequence Value Representation, and for which multiple Values are allowed, the presence of a single Value is sufficient to satisfy the Type 1 requirement, unless specified otherwise in the Attribute description, and other Values may be empty, unless otherwise specified by the IOD." And: "The presence of one or more delimiter (BACKSLASH) characters alone, without any Values, is not sufficient to satisfy the Type 1 requirement, since even though the Value Length is greater than zero, there is no valid Data Element Value present."

7.4.2: "Type 1C Data Elements have the same requirements as Type 1 Data Elements under these conditions. It is a protocol violation if the specified conditions are met and the Data Element is not included."

7.4.3: "IODs and SOP Classes define Type 2 Data Elements that shall be included and are mandatory Data Elements. However, it is permissible that if a Value for a Type 2 Data Element is unknown it can be encoded with zero Value Length and no Value. ... These Data Elements shall be included in the Data Set and their absence is a protocol violation."

7.4.4: "IODs and SOP Classes define Type 2C Data Elements that have the same requirements as Type 2 Data Elements under certain specified conditions. It is a protocol violation if the specified conditions are met and the Data Element is not included."

### PS3.5 2026d section 9.1, UID Encoding Rules

"Each component of a UID is a number and shall consist of one or more digits. The first digit of each component shall not be zero unless the component is a single digit." "Components shall be separated by the character "." (2EH)." "UIDs, shall not exceed 64 total characters, including the digits of each component, separators between components, and the NULL (00H) padding character if needed."

### PS3.3 2026d, the IOD module tables that select the modules

Read from the chtml pages on 2026-11-05. Usage `M` is mandatory.

| IOD, section | SOP Class UID | Patient C.7.1.1 | General Study C.7.2.1 | General Series C.7.3.1 | General Image C.7.6.1 | Image Plane C.7.6.2 | Image Pixel C.7.6.3 | SOP Common C.12.1 |
|--------------|---------------|-----------------|-----------------------|------------------------|-----------------------|---------------------|---------------------|-------------------|
| CR Image, A.2.3 | 1.2.840.10008.5.1.4.1.1.1 | M | M | M | M | not in the IOD | M | M |
| CT Image, A.3.3 | 1.2.840.10008.5.1.4.1.1.2 | M | M | M | M | M | M | M |
| MR Image, A.4.3 | 1.2.840.10008.5.1.4.1.1.4 | M | M | M | M | M | M | M |
| PET Image, A.21.3 | 1.2.840.10008.5.1.4.1.1.128 | M | M | M | M | M | M | M |

### PS3.3 2026d, the attributes of those modules with Type 1, 1C, 2 or 2C

Type 3 rows and rows inside a sequence (`>`) are omitted, because this story checks neither. The condition text of every 1C and 2C row is quoted, because decision 4 decides which conditions are decidable.

| Module, table | Attribute | Tag | Type | Condition, quoted where conditional |
|---------------|-----------|-----|------|-------------------------------------|
| Patient, C.7-1 | Patient's Name | (0010,0010) | 2 | |
| Patient, C.7-1 | Patient ID | (0010,0020) | 2 | |
| Patient, C.7-1 | Patient's Birth Date | (0010,0030) | 2 | |
| Patient, C.7-1 | Patient's Sex | (0010,0040) | 2 | |
| Patient, C.7-1 | Patient's Alternative Calendar, Patient Species Description and Code Sequence, Patient Breed Description and Code Sequence, Breed Registration Sequence, Responsible Person and Role, Responsible Organization, De-identification Method and Code Sequence | (0010,0035), (0010,2201), (0010,2202), (0010,2292), (0010,2293), (0010,2294), (0010,2297), (0010,2298), (0010,2299), (0012,0063), (0012,0064) | 1C or 2C | non-human organism, alternative calendar or de-identification conditions, not decidable from the data set (decision 4) |
| General Study, C.7-3 | Study Instance UID | (0020,000D) | 1 | |
| General Study, C.7-3 | Study Date, Study Time, Referring Physician's Name, Study ID, Accession Number | (0008,0020), (0008,0030), (0008,0090), (0020,0010), (0008,0050) | 2 | |
| General Series, C.7-5a | Modality, Series Instance UID | (0008,0060), (0020,000E) | 1 | |
| General Series, C.7-5a | Series Number | (0020,0011) | 2 | |
| General Series, C.7-5a | Laterality | (0020,0060) | 2C | "Required if the body part examined is a paired structure and Image Laterality (0020,0062) or Frame Laterality (0020,9072) or Measurement Laterality (0024,0113) are not present." |
| General Series, C.7-5a | Patient Position | (0018,5100) | 2C | "Required for images where Patient Orientation Code Sequence (0054,0410) is not present and whose SOP Class UID (0008,0016) is one of the following: "1.2.840.10008.5.1.4.1.1.2" (CT Image Storage), "1.2.840.10008.5.1.4.1.1.4" (MR Image Storage), "1.2.840.10008.5.1.4.1.1.2.1" (Enhanced CT Image Storage), "1.2.840.10008.5.1.4.1.1.4.1" (Enhanced MR Image Storage), "1.2.840.10008.5.1.4.1.1.4.3" (Enhanced Color MR Image Storage), "1.2.840.10008.5.1.4.1.1.4.2" (MR Spectroscopy Storage SOP Class)." |
| General Series, C.7-5a | Anatomical Orientation Type | (0010,2210) | 1C | "Required if the Patient is a non-human organism and the anatomical Frame of Reference is not bipedal." |
| General Image, C.7-9 | Instance Number | (0020,0013) | 2 | |
| General Image, C.7-9 | Patient Orientation | (0020,0020) | 2C | "Required if image does not require Image Orientation (Patient) (0020,0037) and Image Position (Patient) (0020,0032) or if image does not require Image Orientation (Slide) (0048,0102). May be present otherwise." |
| General Image, C.7-9 | Content Date, Content Time | (0008,0023), (0008,0033) | 2C | "Required if image is part of a Series in which the images are temporally related." |
| Image Plane, C.7-10 | Pixel Spacing, Image Orientation (Patient), Image Position (Patient) | (0028,0030), (0020,0037), (0020,0032) | 1 | |
| Image Plane, C.7-10 | Slice Thickness | (0018,0050) | 2 | |
| Image Pixel, C.7-11a | Include Table C.7-11c | | | "Required if the IOD is not being transferred in a STOW-RS Request and is not encoded as metadata and compressed bulk pixel data." |
| Image Pixel, C.7-11a | Pixel Data | (7FE0,0010) | 1C | "Required if Pixel Data Provider URL (0028,7FE0) is not present." |
| Image Pixel, C.7-11a | Pixel Data Provider URL | (0028,7FE0) | 1C | "Required if the image is to be transferred in one of the following presentation contexts ... 1.2.840.10008.1.2.4.94 ... 1.2.840.10008.1.2.4.95" |
| Image Pixel, C.7-11a | Pixel Padding Range Limit | (0028,0121) | 1C | "Required if pixel padding is to be defined as a range rather than a single Value." |
| Image Pixel, C.7-11a | Extended Offset Table Lengths | (7FE0,0002) | 1C | "Required if Extended Offset Table (7FE0,0001) is present." |
| Image Pixel Description, C.7-11c | Samples per Pixel, Photometric Interpretation, Rows, Columns, Bits Allocated, Bits Stored, High Bit, Pixel Representation | (0028,0002), (0028,0004), (0028,0010), (0028,0011), (0028,0100), (0028,0101), (0028,0102), (0028,0103) | 1 | |
| Image Pixel Description, C.7-11c | Planar Configuration | (0028,0006) | 1C | "Required if Samples per Pixel (0028,0002) has a Value greater than 1." |
| Image Pixel Description, C.7-11c | Pixel Aspect Ratio | (0028,0034) | 1C | "Required if the aspect ratio values do not have a ratio of 1:1 and the physical pixel spacing is not specified by Pixel Spacing (0028,0030), or Imager Pixel Spacing (0018,1164) or Nominal Scanned Pixel Spacing (0018,2010)" |
| Image Pixel Description, C.7-11c | Red, Green and Blue Palette Color Lookup Table Descriptor and Data | (0028,1101) to (0028,1103), (0028,1201) to (0028,1203) | 1C | "Required if Photometric Interpretation (0028,0004) has a Value of PALETTE COLOR or Pixel Presentation (0008,9205) at the image level equals COLOR or MIXED." |
| SOP Common, C.12-1 | SOP Class UID, SOP Instance UID | (0008,0016), (0008,0018) | 1 | |
| SOP Common, C.12-1 | Specific Character Set | (0008,0005) | 1C | "Required if an expanded or replacement character set is used." |
| SOP Common, C.12-1 | Encrypted Attributes Sequence, HL7 Structured Document Reference Sequence, Query/Retrieve View, Conversion Source Attributes Sequence | (0400,0500), (0040,A390), (0008,0053), (0020,9172) | 1C | conditions on encryption, HL7 references, a C-MOVE view and conversion, not decidable from the data set (decision 4) |

### PS3.6 2026d Table 6-1, the VR and VM of every attribute the presence rules name

Read from `part06.xml` on 2026-11-05. This is the table the bounded VM check transcribes (decision 2).

| Tag | Keyword | VR | VM |
|-----|---------|----|----|
| (0010,0010) | PatientName | PN | 1 |
| (0010,0020) | PatientID | LO | 1 |
| (0010,0030) | PatientBirthDate | DA | 1 |
| (0010,0040) | PatientSex | CS | 1 |
| (0010,1010) | PatientAge | AS | 1 |
| (0020,000D) | StudyInstanceUID | UI | 1 |
| (0008,0020) | StudyDate | DA | 1 |
| (0008,0030) | StudyTime | TM | 1 |
| (0008,0090) | ReferringPhysicianName | PN | 1 |
| (0020,0010) | StudyID | SH | 1 |
| (0008,0050) | AccessionNumber | SH | 1 |
| (0008,0060) | Modality | CS | 1 |
| (0020,000E) | SeriesInstanceUID | UI | 1 |
| (0020,0011) | SeriesNumber | IS | 1 |
| (0020,0060) | Laterality | CS | 1 |
| (0018,5100) | PatientPosition | CS | 1 |
| (0054,0410) | PatientOrientationCodeSequence | SQ | 1 |
| (0020,0013) | InstanceNumber | IS | 1 |
| (0020,0020) | PatientOrientation | CS | 2 |
| (0008,0023) | ContentDate | DA | 1 |
| (0008,0033) | ContentTime | TM | 1 |
| (0028,0030) | PixelSpacing | DS | 2 |
| (0020,0037) | ImageOrientationPatient | DS | 6 |
| (0020,0032) | ImagePositionPatient | DS | 3 |
| (0018,0050) | SliceThickness | DS | 1 |
| (0028,0002) | SamplesPerPixel | US | 1 |
| (0028,0004) | PhotometricInterpretation | CS | 1 |
| (0028,0010) | Rows | US | 1 |
| (0028,0011) | Columns | US | 1 |
| (0028,0100) | BitsAllocated | US | 1 |
| (0028,0101) | BitsStored | US | 1 |
| (0028,0102) | HighBit | US | 1 |
| (0028,0103) | PixelRepresentation | US | 1 |
| (0028,0006) | PlanarConfiguration | US | 1 |
| (0028,1101) to (0028,1103) | Red, Green, BluePaletteColorLookupTableDescriptor | US or SS | 3 |
| (0028,1201) to (0028,1203) | Red, Green, BluePaletteColorLookupTableData | OW | 1 |
| (7FE0,0010) | PixelData | OB or OW | 1 |
| (0028,7FE0) | PixelDataProviderURL | UR | 1 |
| (7FE0,0001) | ExtendedOffsetTable | OV | 1 |
| (7FE0,0002) | ExtendedOffsetTableLengths | OV | 1 |
| (0008,0016) | SOPClassUID | UI | 1 |
| (0008,0018) | SOPInstanceUID | UI | 1 |
| (0008,0005) | SpecificCharacterSet | CS | 1-n |

### The code this story builds on, read from the tree

- `crates/ocelli-dicom/src/parse.rs`, `parse_part10(bytes) -> Result<ParsedDicom, ParseError>`. It reads through `DicomCollectorOptions` with `OddLengthStrategy::Fail`, so an odd-length value never reaches a `ParsedDicom`.
- `dicom-parser` 0.10.0 `ValueReadStrategy` defaults to `Preserved`: "Values will be stored without decoding dates or textual numbers ... Date-time values and numbers are kept in their original string representation as string objects." `read_value_strs` splits on `\` and keeps padding. `read_value_cs` is the same path. Text VRs are decoded through the declared character set, and AE, CS, AS and UR through `DefaultCharacterSetCodec`, which decodes as ISO 8859-1, so a byte above 7FH arrives as a character above U+007F rather than as a parse failure.
- `crates/ocelli-dicom/src/frame_geometry.rs:718`, `pub fn parse_decimal_string(raw: &str) -> Option<f64>`, the DS grammar of PS3.5 6.2 with "at most sixteen bytes including padding". It refuses `inf`, `nan` and Rust's `1_0` digit separator, and it refuses an empty value.
- `crates/ocelli-dicom/src/multiframe.rs:323`, the 12-byte IS limit for Number of Frames, inline and private.
- `dicom-dictionary-std` 0.10.0, already in `Cargo.lock` through `dicom-object`, `MIT OR Apache-2.0`. Its `DataDictionaryEntryRef` carries `tag`, `alias` and `vr` and **no VM**. The VM appears only in generated doc comments such as `/// PatientAge (0010,1010) AS 1 DICOM`.
- `examples/folder-viewer/src/preview.rs:1282`, `headers_json(bytes)`, which the `POST /headers` route in `serve.rs:131` calls. The headers sheet is `#modal` in `page.html`, opened by the `#headers` button (`data-key="D"`).
- `packages/core/src/generated.ts`, written by `crates/ocelli-viewport/tests/typegen.rs` under `UPDATE_TYPES=1` and asserted equal otherwise. This is the existing pattern for a Rust source of truth mirrored into `@ocelli/core`.
- `packages/core/src/index.test.ts`, whose hand-typed `PUBLISHED` list fails when a value export is added without a line.
- `crates/ocelli-wasm/Cargo.toml` depends on `ocelli-cache`, `ocelli-core`, and with `render-worker` on `ocelli-render` and `ocelli-viewport`. It does not depend on `ocelli-dicom`. No wasm module links the parser today.

## What the specification does not cover

The HLD covers none of this story, and DICOM does not say how a checker reports. Every item below is a lead decision under the operator's delegation of 2026-11-05.

1. **Where the report is produced.** In Rust, in `ocelli-dicom`, as `check_conformance(&ParsedDicom) -> ConformanceReport` with a JSON serialisation the crate owns. `@ocelli/core` gets the report's typed contract and a validating reader, `readConformanceReport`, plus the rule catalogue generated from Rust by a typegen test, the way `generated.ts` is produced today. The rules exist once. A TypeScript copy of the checks over dcmjs would be a second implementation of the same rules, which is the defect class `ocelli-pixel` exists to prevent. In-browser production needs a wasm module that links `ocelli-dicom`, and none does (the last bullet above). Adding the parser to a wasm module is a change to the boundary and the size budget and belongs with the decode worker (F-202, unscheduled). This story says so in `use.html`.
2. **The dictionary.** VR comes from the encoded element for explicit-VR files and from `dicom-dictionary-std` for implicit-VR files, which is what `parse_part10` already does, so no new dependency is added. VM is checked only for the 47 rows of the PS3.6 table above, transcribed into a tracked `const` table in `crates/ocelli-dicom/src/conformance/vm.rs` with the edition and table cited in its doc comment. A unit test asserts every row's VR equals `dicom-dictionary-std`'s VR for the same tag, so the two sources cross-check. A full PS3.6 VM table is out of scope: it would need a generated extract of PS3.6 tracked in the repository, and redistributing the standard's text is a licence question F-217 did not settle. The report lists the VM rule with the words "for the listed attributes only".
3. **The module set.** The seven modules the operator named, on four SOP classes: CR Image Storage (six modules, the IOD has no Image Plane module), CT, MR and PET Image Storage (all seven). For any other SOP Class UID the presence rules report `not evaluated` with the SOP class, and the value rules still run. Assuming the seven modules are mandatory in an IOD this plan did not read would produce false findings, which is the same quietly wrong answer in a different place.
4. **Which 1C and 2C conditions are decidable.** Decidable from the data set alone, and checked: Planar Configuration (SPP greater than 1), the six palette elements (Photometric Interpretation `PALETTE COLOR`, the Pixel Presentation half applies to enhanced IODs outside the set), Pixel Data (Pixel Data Provider URL absent), Extended Offset Table Lengths (Extended Offset Table present), Specific Character Set (decision 6), Patient Position (CT or MR SOP class and (0054,0410) absent), and Patient Orientation (CR, whose IOD does not require Image Orientation and Image Position, and not CT, MR or PET, which do). Not decidable and listed by name in the report as not checked: Laterality, Content Date, Content Time, Anatomical Orientation Type, Pixel Aspect Ratio, Pixel Padding Range Limit, Pixel Data Provider URL, and the non-human, alternative calendar, de-identification, encryption, HL7, Query/Retrieve View and conversion rows.
5. **What a finding carries.** The tag, the keyword from the dictionary or none for a private or unknown tag, the sequence path as (tag, 1-based item number) steps, the rule identifier, the section it cites, the 1-based value number where the rule is per value, and a measure. A measure is a measured and permitted length in bytes or characters, a measured and permitted value count, or a 0-based character index with a class (`control`, `backslash`, `outside-repertoire`, `non-default`). **It never carries a value, a substring of one, or a character code**, because a single character of a name is part of the name. Lengths and counts are kept, because the operator's example findings are a length and an absence and a report without them is not actionable.
6. **Specific Character Set 1C.** "An expanded or replacement character set is used" is decided as: an SH, LO, UC, ST, LT, UT or PN value decodes to a character above U+007F or contains ESC, and (0008,0005) is absent. One finding on (0008,0005), not one per element. A character above U+007F in AE, CS, AS, DA, DS, DT, IS, TM, UI or UR is a repertoire finding on that element whatever (0008,0005) says, by section 6.1.2.3's note.
7. **One finding per value per family, in a fixed order.** For each value: length, then repertoire, then format. The first failure is reported and the later checks of the same value are skipped, so "040 " gives one finding and not three. Presence, VM and VR-mismatch are per element and independent of the value checks. A Type 1 element that is empty is reported as empty and its VM and value checks are skipped.
8. **The padding exception of section 6.4.** The last value of an element may exceed its VR's length limit by exactly one when its final character is the padding character, SPACE or NULL for UI. Every other value is held to the limit exactly.
9. **Query-context clauses do not apply.** The QUOTATION MARK empty value, DA range `-` and the longer range lengths are permitted only "in the context of a Query", and a stored instance is not one, so they are findings.
10. **DS reuses `parse_decimal_string`.** The checker measures length itself and then calls the existing grammar. The one value the grammar refuses that PS3.5 permits lexically, an exponent outside binary64 such as `1E999`, is reported as `DS-RANGE` with the text "outside the binary64 range, the grammar is not violated", so the report does not claim a violation the standard does not make. IS gets its own check because the multiframe one is private and inline. It is not extracted in this story, which would touch `multiframe.rs`.
11. **VR mismatch.** An explicit VR that differs from the dictionary's exact VR for a standard tag is a finding, except UN (section 6.2.2 permits it) and the dictionary's `US or SS` and `OB or OW` alternatives. A value encoded as UN is counted as not checked.
12. **Binary VRs.** A value length that is not a multiple of the element size is a finding where the parser admits it. Whether `dicom-parser` admits it is not known and is measured at implementation, and the AC says which.
13. **The report states its own scope.** `CONFORMANCE_SCOPE` is the sentence "This report checks the rules it lists and nothing else. It is not a statement that the file conforms to DICOM." The JSON carries it, the TypeScript reader refuses a report without the exact sentence, and the viewer prints it above the list.
14. **The viewer computes it lazily.** `POST /conformance` is a new route. The page requests it for the current image 300 ms after that image's first paint, if the image is still current, with one request in flight and an abandoned request ignored by token, the same pattern as `openHeaders`. Results are kept per F-216 file id in a page `Map` capped at 512 entries and cleared on a new open. Nothing is requested during `POST /file`, thumbnails or scrolling.

## Problem and reproducer

A synthetic Part 10 file with PatientAge `040 ` (AS, four bytes, the trailing space is padding), InstitutionName holding 170 characters (LO), and SOP Instance UID and Modality present with zero length, in a CT Image Storage object. Built in a test from bytes, as `crates/ocelli-dicom/tests/part10.rs` builds its data sets, so no DICOM file is written to the tree.

Today `parse_part10` accepts it and `open_frame` opens the frame, which is correct, and nothing anywhere says the file breaks four rules. The folder viewer's headers sheet shows the values and no finding. Expected after this story: four findings, `AS-FORMAT` on (0010,1010) value 1, `VALUE-LENGTH` on (0008,0080) value 1 measured 170 characters against 64, `TYPE1-EMPTY` on (0008,0018) and on (0008,0060), the badge reads 4, the frame still displays, and no finding contains `040` or any character of the institution name.

```bash
bin/ocelli.sh test ocelli-dicom --test conformance
```

## Scope, API and compatibility

In scope: the checker and its JSON in `ocelli-dicom`, the typegen mirror and reader in `@ocelli/core`, the `/conformance` route, the badge and list in the folder viewer, and the user documents.

Non-goals: Type 3 attributes, attributes inside sequences for presence, Enumerated Values and Defined Terms (PS3.5 6.3, for example Patient's Sex `M`, `F`, `O`), IOD-specific constraints beyond the module tables, enhanced and multi-frame IODs, functional groups, private attribute semantics, a full PS3.6 VM table, repair of any value, any change to `parse_part10`, `open_frame` or any refusal they make, and in-browser production (decision 1).

Rust, `crates/ocelli-dicom/src/conformance.rs` and `crates/ocelli-dicom/src/conformance/vm.rs`, re-exported from `lib.rs`:

```rust
pub const CONFORMANCE_SCOPE: &str =
    "This report checks the rules it lists and nothing else. It is not a statement that the file conforms to DICOM.";
pub const CONFORMANCE_EDITION: &str = "2026d";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConformanceRule {
    ValueLength, FixedLength, Repertoire, CharsetUndeclared, VrMismatch, Multiplicity,
    BinaryLength, AeSpaces, AsFormat, DaFormat, TmFormat, DtFormat, DsFormat, DsRange,
    IsFormat, IsRange, UiFormat, PnFormat, UrFormat,
    Type1Absent, Type1Empty, Type1cAbsent, Type1cEmpty, Type2Absent, Type2cAbsent,
}
impl ConformanceRule {
    pub const ALL: &'static [ConformanceRule];
    pub const fn id(self) -> &'static str;      // "AS-FORMAT", "TYPE1-EMPTY", ...
    pub const fn section(self) -> &'static str; // "PS3.5 6.2 Table 6.2-1", "PS3.5 7.4.1", ...
}

pub struct PathStep { pub sequence: Tag, pub item: u32 }  // item is 1-based
pub enum Measure {
    Bytes { measured: u32, limit: u32 },
    Characters { measured: u32, limit: u32 },
    Values { measured: u32, minimum: u32, maximum: Option<u32> },
    Character { index: u32, class: CharacterClass },
    None,
}
pub enum CharacterClass { Control, Backslash, OutsideRepertoire, NonDefault }

pub struct ConformanceFinding { /* tag, keyword: Option<&'static str>, path, rule, value: Option<u32>, measure */ }
pub enum ModuleScope {
    Evaluated { sop_class: &'static str, iod: &'static str, table: &'static str, modules: &'static [&'static str] },
    NotEvaluated { sop_class: Option<String> },
}
pub struct ConformanceReport { /* findings sorted by path, tag, value, rule; scope; counts */ }
impl ConformanceReport {
    pub fn findings(&self) -> &[ConformanceFinding];
    pub fn modules(&self) -> &ModuleScope;
    pub fn not_checked(&self) -> &'static [&'static str]; // decision 4's undecidable rows, by keyword
    pub fn to_json(&self) -> String;
}
pub fn check_conformance(parsed: &ParsedDicom) -> ConformanceReport;
```

`check_conformance` is infallible. It reads `ParsedDicom::object()` by shared reference and changes nothing. A SOP Class UID is the only value the report repeats, because it is a registry identifier and decides the module scope.

JSON, written with the crate's existing `serde_json`:

```json
{"scope":"This report checks the rules it lists and nothing else. It is not a statement that the file conforms to DICOM.",
 "edition":"2026d",
 "rules":[{"id":"AS-FORMAT","section":"PS3.5 6.2 Table 6.2-1"}],
 "modules":{"evaluated":true,"sopClass":"1.2.840.10008.5.1.4.1.1.2","iod":"CT Image","table":"PS3.3 A.3.3",
            "modules":["Patient","General Study","General Series","General Image","Image Plane","Image Pixel","SOP Common"],
            "notChecked":["Laterality","ContentDate"]},
 "counts":{"elements":57,"notCheckedUn":0},
 "findings":[{"tag":"(0010,1010)","keyword":"PatientAge","path":[],"rule":"AS-FORMAT","value":1,"measure":null}]}
```

TypeScript, `packages/core/src/conformance.ts` and the generated `packages/core/src/conformance.generated.ts`:

```ts
export const CONFORMANCE_RULES: readonly { id: string; section: string }[]; // generated
export const CONFORMANCE_SCOPE: string;                                      // generated
export interface ConformanceFinding { tag: string; keyword: string | null; path: { sequence: string; item: number }[];
  rule: string; section: string; value: number | null; measure: ConformanceMeasure | null }
export interface ConformanceReport { scope: string; edition: string; findings: readonly ConformanceFinding[];
  modules: ConformanceModules; counts: { elements: number; notCheckedUn: number } }
export function readConformanceReport(text: string):
  { ok: true; report: ConformanceReport } | { ok: false; reason: "json" | "scope" | "rule" | "shape" };
```

The finding's `section` is filled from the generated catalogue, never taken from the wire, so a report naming an unknown rule is refused as `rule`.

Folder viewer: `POST /conformance` with `X-Ocelli-File` (F-216) or a whole body, answering the same error forms as `/headers`. `LIBRARY_FILES` (F-213) gains `conformance` and `conformance.generated`. The `#headers` button gains a badge, hidden until a report arrives and while the count is zero. The sheet gains a "Checks" section above the table with the scope sentence, the module line and one row per finding.

Compatibility: no existing route, header, export or Rust signature changes. `/headers` is byte-identical. The report is additive.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `check_conformance` | `ParsedDicom::object()`, `dicom-dictionary-std`, `vm.rs`, `parse_decimal_string` | Pure, on demand, never on the open path | `ConformanceReport` |
| `ConformanceReport::to_json` | `serde_json` | Owns the wire shape | JSON text |
| `POST /conformance` | F-216 held bytes or the body, `parse_part10`, `check_conformance` | One parse per request, no pixel decode | 200 JSON or the `/headers` error form |
| `readConformanceReport` | `conformance.generated.ts` | Refuses malformed, unknown-rule and wrong-scope reports | Typed report or a reason |
| `page.html` badge and list | `/conformance`, `readConformanceReport` via the import map | 300 ms after first paint, one in flight, cached per file id | Badge count, list, scope sentence |
| `open_frame`, `/preview`, `/inspect`, `/headers` | unchanged | The report never gates them | Unchanged bytes |
| F-216 benchmark subjects | `viewer.open`, `viewer.first_image`, `viewer.scroll_rate` | Release build, F-216's host | Within F-216's answered thresholds |

Owner of the rule set, sections and scope sentence: `ocelli-dicom`. Owner of the TypeScript mirror: the typegen test. Owner of request scheduling: `page.html`.

## State transitions and invariants

The checker is stateless. The page state is the per-file-id report cache and the request token.

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Image shown, no report cached | 300 ms elapse, image still current | One `/conformance` request, then badge and cache entry | Request error: badge hidden, sheet says "the checks could not be read", headers unaffected |
| Request in flight | Image changes | Token advances, reply ignored when it arrives | Badge shows the new image's cached count or stays hidden |
| Report cached for this file id | Image shown again | Badge from cache, no request | none |
| Any | New folder or synthetic open | Cache cleared, token advanced | none |
| F-216 file id expired (410) | `/conformance` by id | Page re-posts per F-216's rule, then requests once | Second 410: hidden badge, no loop |

Invariants: the report for a file is a pure function of its bytes. No request is issued while the page is in `POST /file` or thumbnail work. A displayed frame never waits on a report.

## Ownership, commit and recovery

Not applicable for the checker, which allocates its own report and holds nothing. The page cache owns small JSON objects keyed by file id, bounded at 512 entries, evicted oldest first, cleared on open. A duplicate request for the same file id while one is in flight is not sent.

## Numbered acceptance inventory

Every fixture is a synthetic data set built in the test from bytes (explicit VR little endian unless stated), CT Image Storage unless stated, with every Type 1 and Type 2 attribute of the seven modules present and valid except the one the row changes. That base data set is AC1's control.

| ID | Trigger and expected behaviour | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The valid CT base data set gives zero findings, module scope evaluated with seven modules, `table` "PS3.3 A.3.3" | `fixture_valid_ct_has_no_findings` | A checker that flags padding, or Type 2 present-empty, as a violation | implementation |
| AC2 | AS, PS3.5 Table 6.2-1. PatientAge `040 ` gives one `AS-FORMAT` on (0010,1010) value 1. `018M`, `000D`, `120Y` give none. `40Y ` gives `AS-FORMAT`. `0040` gives `AS-FORMAT` | `fixture_age_string` | Accepting a trailing space as padding in a fixed 4-byte VR. Reporting repertoire and format both for one value | implementation |
| AC3 | LO length in characters, PS3.5 6.2 Note and 6.4. InstitutionName of 170 `A` gives `VALUE-LENGTH`, measure Characters 170 against 64. 64 gives none. 63 plus one SPACE pad gives none. 65 plus pad (66) gives 66 against 64. A two-byte-per-character ISO_IR 192 value of 64 characters (128 bytes) gives none | `fixture_long_string_counts_characters` | Counting bytes for a character-limited VR. Allowing the pad exception on a single value whose content is already at the limit | implementation |
| AC4 | The 6.4 last-value exception. ImageType CS `ORIGINAL\PRIMARY\` plus a 16-character third value, total 33 bytes padded to 34, gives none. The same 17 bytes as the FIRST value gives `VALUE-LENGTH` value 1, 17 against 16 | `fixture_last_value_may_carry_the_pad` | Applying the exception to every value, or to none | implementation |
| AC5 | Type 1 empty, PS3.5 7.4.1. SOP Instance UID zero length gives `TYPE1-EMPTY`. Modality `\\` (two backslashes, VL 2) gives `TYPE1-EMPTY`. Neither also gives a VM or format finding | `fixture_type1_empty_and_delimiter_only` | Treating VL greater than zero as present. Reporting `UI-FORMAT` for an empty value | implementation |
| AC6 | Type 1 and 2 absence, PS3.5 7.4.1 and 7.4.3. Rows absent gives `TYPE1-ABSENT` on (0028,0010). Patient's Name absent gives `TYPE2-ABSENT`. Patient's Name present with zero length gives none | `fixture_absent_type1_and_type2` | Requiring a value for Type 2 | implementation |
| AC7 | DS, Table 6.2-1. ` 12.5 ` none. `+1.5E-3` none. `1,5` `DS-FORMAT`. `1 5` `DS-FORMAT`. A 17-byte value `VALUE-LENGTH` 17 against 16. `1E999` `DS-RANGE` | `fixture_decimal_string` | A second DS grammar beside `parse_decimal_string`. Calling `1E999` a grammar violation | implementation |
| AC8 | IS, Table 6.2-1. `-2147483648` none. `2147483647` none. `2147483648` `IS-RANGE`. `12a` `IS-FORMAT`. 13 bytes `VALUE-LENGTH` 13 against 12. ` +7 ` none | `fixture_integer_string` | An `i64` range, or a parse that accepts `12a` as 12 | implementation |
| AC9 | DA, Table 6.2-1 and the Gregorian calendar. `20240229` none, 2024 is a leap year. `20230229` `DA-FORMAT`. `20260230` `DA-FORMAT`. `19000229` `DA-FORMAT`, 1900 is not a leap year. `20000229` none, 2000 is. `2026.11.05` `FIXED-LENGTH` 10 against 8 | `fixture_date` | A divisible-by-four leap rule, which accepts 1900-02-29 | implementation |
| AC10 | TM, Table 6.2-1, using the standard's own examples. `070907.0705 ` none. `1010` none. `021 ` `TM-FORMAT`. `2400` `TM-FORMAT`. `235960` none (leap second). `1260` `TM-FORMAT`. `12.5` `TM-FORMAT`. `120000.` `TM-FORMAT`. `120000.1234567` `TM-FORMAT` (7 fraction digits) | `fixture_time` | Accepting `2400`. Accepting a lone `.` | implementation |
| AC11 | DT, Table 6.2-1 and its note. `195308` none. `19530827111300.0` none. `2007-0500` none. `20070101-0000` `DT-FORMAT`. `20070101+1500` `DT-FORMAT`. `20070101-1300` `DT-FORMAT`. `20070101+1400` none. `20070` `DT-FORMAT` | `fixture_date_time` | Accepting `-0000`. An offset range of plus or minus 1200 | implementation |
| AC12 | UI, PS3.5 9.1. `1.2.840.10008.5.1.4.1.1.2` none. `1.02.3` `UI-FORMAT`. `1..2` `UI-FORMAT`. `1.2.a` `UI-FORMAT`. `0.1` none (single-digit zero). 65 bytes `VALUE-LENGTH` 65 against 64. `1.2.3` with NULL pad (6 bytes) none. `1.2.3` with SPACE pad `UI-FORMAT` | `fixture_uid` | Accepting leading zeros. Treating a SPACE pad as valid for UI | implementation |
| AC13 | CS and AE, Table 6.2-1. Modality `ct` `REPERTOIRE`, Character index 0 class OutsideRepertoire. Modality `CT_A` none. A 17-byte CS `VALUE-LENGTH`. An AE of 16 spaces `AE-SPACES` | `fixture_code_string_and_ae` | A case-insensitive CS check | implementation |
| AC14 | PN, Table 6.2-1. `A^B^C^D^E` none. `A^B^C^D^E^F` `PN-FORMAT`. `A=B=C` none. `A=B=C=D` `PN-FORMAT`. A 65-character first group `VALUE-LENGTH` 65 against 64 | `fixture_person_name` | A whole-value 64 limit instead of per group | implementation |
| AC15 | Control characters, Table 6.2-1. LO with TAB at index 3 `REPERTOIRE` index 3 class Control. LT with TAB, CR and LF none. ST with BEL `REPERTOIRE` | `fixture_control_characters` | One control-character rule for every text VR | implementation |
| AC16 | Specific Character Set 1C, PS3.5 6.1.2.2 and PS3.3 C.12-1. LO containing byte E9H with (0008,0005) absent gives one `TYPE1C-ABSENT` on (0008,0005), and none on the LO. With `ISO_IR 100` none. CS containing E9H gives `REPERTOIRE` class NonDefault on the CS with or without (0008,0005) | `fixture_character_set` | A finding per text element, or none for CS under a declared set | implementation |
| AC17 | VM, PS3.5 6.4 and PS3.6 Table 6-1. Pixel Spacing with one value `MULTIPLICITY` measured 1 minimum 2 maximum 2. Image Orientation with five values `MULTIPLICITY`. Patient ID `AB\CD` `MULTIPLICITY` measured 2 against 1. Specific Character Set with three values none | `fixture_multiplicity` | A VM check outside the 47-row table. Ignoring a backslash split in a VM 1 element | implementation |
| AC18 | VM table cross-check. Every `vm.rs` row's VR equals `dicom-dictionary-std`'s VR for the tag, `US or SS` and `OB or OW` included | `vm_table_agrees_with_the_dictionary` | A transcription error in the VR column | implementation |
| AC19 | VR mismatch, PS3.5 6.2. PatientAge encoded explicit `LO` gives `VR-MISMATCH`. Encoded `UN` gives none, and `counts.notCheckedUn` is 1 | `fixture_vr_mismatch_and_un` | Flagging UN, which 6.2.2 permits | implementation |
| AC20 | 1C, PS3.3 C.7-11c and C.7-11a. SPP 3 without Planar Configuration `TYPE1C-ABSENT`. SPP 1 without it none. `PALETTE COLOR` without the Red descriptor `TYPE1C-ABSENT` on (0028,1101). Pixel Data absent with no Provider URL `TYPE1C-ABSENT`. Extended Offset Table present without Lengths `TYPE1C-ABSENT` | `fixture_decidable_type1c` | A 1C rule applied unconditionally | implementation |
| AC21 | 2C, PS3.3 C.7-5a and C.7-9. CT without Patient Position and without (0054,0410) `TYPE2C-ABSENT`. CT with (0054,0410) present none. CR without Patient Position none. CR without Patient Orientation `TYPE2C-ABSENT`. CT without Patient Orientation none | `fixture_decidable_type2c` | Requiring Patient Position on CR. Requiring Patient Orientation on CT | implementation |
| AC22 | Module scope, PS3.3 A.2.3. CR base data set with no Image Plane attributes gives none, `modules` lists six. A Secondary Capture SOP class gives `evaluated: false` with the UID, no presence finding even with Rows absent, and its value findings still reported | `fixture_module_scope_by_sop_class` | Requiring Image Plane on CR. Assuming the seven modules for an unread IOD | implementation |
| AC23 | Undecidable conditions are listed, not checked. The `notChecked` list equals decision 4's not-decidable list, by keyword, and absence of any of them gives no finding | `undecidable_conditions_are_named` | Silently skipping them so the report overstates its coverage | implementation |
| AC24 | Sequences. A `REPERTOIRE` violation in a CodeValue inside the second item of (0054,0410) reports path `[{(0054,0410), 2}]` | `fixture_nested_item_path` | A 0-based item number. Losing the path | implementation |
| AC25 | Privacy. For every fixture in this table, the JSON contains none of the values the fixture wrote that are not a SOP Class UID, checked by substring over every written value of 3 or more characters, including `040`, `SECRET^NAME` and the 170-character run | `no_finding_carries_a_value` | Echoing the offending value or a character code in a message | implementation |
| AC26 | Never stops display. The problem-and-reproducer data set opens through `open_frame` with the same `OpenedFrame` before and after `check_conformance` runs, and `parse_part10`, `open_frame` and every `/preview` byte are unchanged by this story. `git diff --stat` shows no change to `parse.rs`, `frame_open.rs` or `preview_reply` | `the_report_never_gates_open` plus review | A checker wired into `open_frame` or the preview route | implementation |
| AC27 | Order and determinism. Findings sort by path, tag, value, rule. Two runs give byte-identical JSON | `report_is_deterministic` | Ordering from a `HashMap` | implementation |
| AC28 | Scope sentence. The JSON `scope` equals `CONFORMANCE_SCOPE`. `readConformanceReport` refuses the same report with the sentence changed by one character, reason `scope` | `scope_sentence_is_exact` (Rust), `conformance.test.ts` | A report that can be shown without its disclaimer | implementation |
| AC29 | Typegen. `conformance.generated.ts` equals the output of `crates/ocelli-dicom/tests/conformance_typegen.rs`. Adding a variant to `ConformanceRule` without regenerating fails the test. Mutation: delete one rule from the generated file, the test goes red | `conformance_typegen` | A hand-maintained TypeScript rule list that drifts | implementation |
| AC30 | Reader. `readConformanceReport` accepts the JSON from AC1 and the reproducer, refuses invalid JSON (`json`), an unknown rule id (`rule`), a missing `findings` (`shape`), and fills `section` from the catalogue | `conformance.test.ts` | Trusting `section` from the wire | implementation |
| AC31 | Published surface. `index.test.ts` `PUBLISHED` gains `readConformanceReport`, `CONFORMANCE_RULES` and `CONFORMANCE_SCOPE` | `index.test.ts` | An export nobody tests | implementation |
| AC32 | Route. `POST /conformance` by `X-Ocelli-File` and by body give byte-identical JSON. A non-Part-10 body gives the `/headers` error form. `/headers` is byte-identical to before | `conformance_by_id_matches_the_body_form` | A second error vocabulary | implementation |
| AC33 | Viewer. The page holds the badge and the "Checks" section with the scope sentence, imports `readConformanceReport` from `@ocelli/core`, schedules 300 ms after first paint with one in flight, and never requests during `/file` or thumbnails. With the library build absent the section reads "The checks need the library build. Run npm run build in the repository, then start the viewer again." | page source test in the crate's `tests/`, plus a headless browser observation through F-208's harness: open the reproducer, badge reads 4, the list shows four rows, the frame is drawn | A request per scroll step. A badge before any report | implementation, then integrated |
| AC34 | Off the critical path. F-216's `viewer.open`, `viewer.first_image` and `viewer.scroll_rate` on the release build and F-216's host stay within F-216's answered thresholds (open at most 1,100 ms, first image at most 20 ms CT and 250 ms DX, scroll at least 55 presented frames per second) with the badge enabled. The `/conformance` reply latency for the held 512 by 512 CT file and the 17 MB DX is recorded, median of 9, and is not a threshold | `bin/ocelli.sh bench viewer.open viewer.first_image viewer.scroll_rate` | A report computed inside `/file` or `/preview` | integrated, on F-216's named host |
| AC35 | Semantic RED, recorded in the progress note: AC2, AC3, AC5, AC9 and AC12 run against a stub `check_conformance` returning no findings and fail with the named finding missing. Mutations after GREEN, each seen red: byte counting for LO (AC3), the four-year leap rule (AC9), dropping the 6.4 exception (AC4), echoing the value in a finding (AC25) | progress note | Tests that pass against an empty checker | implementation |

## Approach

1. `vm.rs`: the 47-row table as `(Tag, &'static str keyword, VrSpec, Vm { min, max })`, with AC18's test.
2. `conformance.rs`: walk `object()` recursively, tracking the path. Per element: VR mismatch, then per value length, repertoire and format by VR, then VM for table rows. Then module scope from SOP Class UID, then presence over the declared attributes and the decidable conditions. Sort, return.
3. `to_json` with `serde_json`, field names as in the scope section.
4. `tests/conformance.rs` with AC1 to AC27, its byte builders copied from the shape of `tests/part10.rs` into a `tests/common` helper only if a second test file needs them.
5. `tests/conformance_typegen.rs` writing `packages/core/src/conformance.generated.ts` under `UPDATE_TYPES=1`.
6. `packages/core/src/conformance.ts`, its test, `index.ts` and `index.test.ts`.
7. `examples/folder-viewer`: `conformance_json` in `preview.rs` beside `headers_json`, the route in `serve.rs`, `LIBRARY_FILES`, then `page.html`.
8. User documents and LLD.

Rejected: a TypeScript checker over dcmjs (decision 1). dcmjs's own validation, which is not a rule catalogue with sections. A full PS3.6 extract (decision 2). Reading VM from `dicom-dictionary-std`'s doc comments, which are not an API. Computing the report inside `/file` (decision 14). Returning the offending value with a redaction flag, which moves the privacy decision to every caller.

## Dependencies, write ownership and resources

Declared dependencies: F-216 (the held-file id, `/file` and its benchmark subjects) and F-213 (the `/lib/` route, the import map and `LIBRARY_FILES`). Both are pending or in progress in S46.

Write set:

```text
crates/ocelli-dicom/src/conformance.rs              new
crates/ocelli-dicom/src/conformance/vm.rs           new
crates/ocelli-dicom/src/lib.rs                      mod and re-exports
crates/ocelli-dicom/tests/conformance.rs            new
crates/ocelli-dicom/tests/conformance_typegen.rs    new
packages/core/src/conformance.ts                    new
packages/core/src/conformance.generated.ts          new, generated
packages/core/src/conformance.test.ts               new
packages/core/src/index.ts, packages/core/src/index.test.ts
examples/folder-viewer/src/preview.rs, serve.rs, page.html
examples/folder-viewer/tests/ (route and page source tests)
README.md, examples/folder-viewer/src/use.html, examples/folder-viewer/src/more.html
docs/lld/dicom-ingest.md, docs/lld/public-api.md
```

Order. The Rust checker, the typegen and the `@ocelli/core` module share no file with any other S46 story except `packages/core/src/index.ts` and `index.test.ts`, and may be built in a worktree at any time. Every viewer file (`preview.rs`, `serve.rs`, `page.html`, `use.html`, `more.html`, `README.md`) is written after F-215 integrates, F-215 being the last viewer story in the build order recorded in F-217's plan (F-216, F-210, F-211, F-208, F-209, F-212, F-213, F-214, F-215), and after F-223 if F-223 writes any of them. `index.ts` and `index.test.ts` are serialised with F-213 and F-215. No GPU. AC34 needs F-216's host and its release build, alone on the machine as F-223 requires.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-dicom
bin/ocelli.sh test ocelli-dicom --test conformance
bin/ocelli.sh test ocelli-dicom --test conformance_typegen
bin/ocelli.sh clippy ocelli-dicom
bin/ocelli.sh test ocelli-folder-viewer
npm run test -w packages/core
npm run lint && npm run typecheck
bin/ocelli.sh native
bin/ocelli.sh bench viewer.open viewer.first_image viewer.scroll_rate
```

Final profile: `bin/ocelli.sh gate --sprint`. Identities: DICOM 2026d PS3.3, PS3.5 and PS3.6 as read on 2026-11-05, dicom-rs 0.10.0 as locked. Evidence is invalidated by any change to `conformance.rs`, `vm.rs`, `parse_decimal_string`, the dicom-rs lock entries, or the generated file. AC34 is invalidated by any change to `page.html` scheduling or F-216's routes. A missing host leaves AC34 open, not passed.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. No pixel is read. The report travels over the local process's HTTP route, which is not the wasm boundary
- Render-loop allocation: none. The check is on demand and off the render path
- unsafe: none
- Tier A (WebGPU): n/a, header checking has no rendering or compute, and the report is identical on every tier
- Tier B (WebGL2): n/a, as tier A
- Tier C (CPU): n/a, as tier A. `ocelli-dicom` builds for wasm32 under `bin/ocelli.sh native`, and no wasm module runs it in this story

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC2 to AC17, AC19 to AC24, each value hand-derived from the quoted PS3.5 Table 6.2-1, 6.1.2.2, 6.4, 7.4 and 9.1 text and the PS3.3 tables, cited in each test's doc comment | `crates/ocelli-dicom/tests/conformance.rs` |
| unit | AC18 VM table against the dictionary, AC27 determinism, AC28 scope | `crates/ocelli-dicom/tests/conformance.rs` |
| unit | AC25 privacy scan over every fixture's JSON | same |
| unit | AC29 typegen drift | `crates/ocelli-dicom/tests/conformance_typegen.rs` |
| unit | AC28, AC30, AC31 reader and surface | `packages/core/src/conformance.test.ts`, `index.test.ts` |
| unit | AC32, AC33 route and page source | `examples/folder-viewer/tests/` |
| browser | AC33 badge and list on the reproducer | F-208's headless harness |
| benchmark | AC34 | `bin/ocelli.sh bench` |

No pixel arithmetic is added, so no pixel fixture is owed. The date, time and integer checks are arithmetic on values, and their fixtures cite PS3.5 Table 6.2-1.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no conformance or validation row, and cornerstone3D 5.8.2 reads only the tags it uses and reports no conformance. This is a capability beyond parity.

## Deviations

None. The HLD does not specify a checker, so nothing here departs from it.

## LLD impact

- `docs/lld/dicom-ingest.md`: a "Conformance report" section with the rule catalogue, the module and SOP class scope, the decidable conditions, the privacy rule, the VM table's source and the scope sentence.
- `docs/lld/public-api.md`: `readConformanceReport`, `CONFORMANCE_RULES`, `CONFORMANCE_SCOPE` and the generated file.

## User-facing documentation impact

- `README.md`, "Start the local series viewer": the Headers button shows a count of findings, and the Headers sheet lists them with the rule and its section, under the sentence that the report checks only the listed rules. One sentence that a finding never stops an image from showing.
- `examples/folder-viewer/src/use.html`: a "Conformance report" section with `ocelli_dicom::check_conformance`, the JSON shape, `readConformanceReport` from `@ocelli/core`, the four SOP classes and seven modules, the not-checked list, and the statement that in-browser production waits for a wasm build that links the parser.
- `examples/folder-viewer/src/more.html`: why a viewer that reads only the tags it draws can show invalid files, what this report checks and what it does not, and that it is not proof of conformance.

## Open questions

None open. Each is a lead decision under the operator's delegation of 2026-11-05.

- Q1, where the report is produced and how `@ocelli/core` exposes it. Answered: lead decision under the operator's delegation of 2026-11-05. Rust in `ocelli-dicom`, a typed reader and generated catalogue in `@ocelli/core`, decision 1. In-browser production is named as waiting on a wasm build that links the parser, not built here.
- Q2, the dictionary. Answered: lead decision under the operator's delegation of 2026-11-05. VR from the stream and `dicom-dictionary-std`, VM from a 47-row table transcribed from PS3.6 2026d Table 6-1, decision 2. No new dependency and no copyleft source.
- Q3, the module set. Answered: lead decision under the operator's delegation of 2026-11-05. Seven modules on CR, CT, MR and PET Image Storage, others not evaluated, decision 3.
- Q4, decidable conditions. Answered: lead decision under the operator's delegation of 2026-11-05. Decision 4's list, with the rest named as not checked.
- Q5, what a finding may carry. Answered: lead decision under the operator's delegation of 2026-11-05. No value and no character code, lengths, counts and indexes only, decision 5.
- Q6, when the viewer computes. Answered: lead decision under the operator's delegation of 2026-11-05. 300 ms after first paint, decision 14, held to F-216's thresholds by AC34.
- Q7, the E-ID. Answered: lead decision under the operator's delegation of 2026-11-05. E3.8, the next free number in E3. If a sibling story designed in the same round also claims E3.8, the lead renumbers one of them when writing the allocation.
