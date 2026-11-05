# F-212, Folder viewer four corners at parity, scale bar and edge letters from Patient Orientation

**Status**: approved
**Epic ref**: E38.22
**Sprint**: S46
**Estimate**: 4w

## Normative source, transcribed

The HLD does not specify this page or a corner overlay. `docs/hld/B-parity-surface.md` counts about 63 tool classes and names no overlay row. cornerstone3D 5.8.2 has no four-corner text either. That text is a viewer convention, recorded in the research note `pro-viewer-research.md` section 4 from the RadiAnt overlay manual: patient identity top left, study and series identity top right, position, window and MR timing bottom left, acquisition date and time bottom right. The note says the corner assignment is configurable per viewer and is not standardised by DICOM. Parity in this story means that convention, not a measured cornerstone row.

The binding HLD rules this plan keeps:

- `docs/hld/15-lut-chain.md` section 18. The LUT chain exists once. This story does not touch it.
- `docs/hld/11-decision-log.md` D3. Pixels do not cross the wasm boundary. The folder viewer is not that path, and this story adds text fields and one header to the existing local HTTP replies.

No copy of PS3.5 or PS3.3 is in the repository, and `.claude/skills/dicom-expert/SKILL.md` section 2 carries only the `DS`, `IS`, `CS`, `UI` rows. The rows below are transcribed from the published standard by the author without a copy at hand, which is a transcription from memory, and the flag stays. AC13 makes the reviewer check them against the current edition at dicom.nema.org before the formatters and the orientation rule are implemented.

PS3.5 section 6.2, Table 6.2-1:

```text
DA  A string of characters of the format YYYYMMDD, where YYYY shall contain
    year, MM shall contain the month, and DD shall contain the day,
    interpreted as a date of the Gregorian calendar system.
    8 bytes fixed.

TM  A string of characters of the format HHMMSS.FFFFFF, where HH contains
    hours (range "00" - "23"), MM contains minutes (range "00" - "59"),
    SS contains seconds (range "00" - "60"), and FFFFFF contains a
    fractional part of a second as small as 1 millionth of a second
    (range "000000" - "999999"). A 24-hour clock is used. Midnight shall be
    represented by only "0000" since "2400" would violate the hour range.
    The string may be padded with trailing spaces. Leading and embedded
    spaces are not allowed. One or more of the components MM, SS, or FFFFFF
    may be unspecified as long as every component to the right of an
    unspecified component is also unspecified. The FFFFFF component may
    contain from 1 to 6 digits.
    14 bytes maximum.

AS  A string of characters with one of the following formats: nnnD, nnnW,
    nnnM, nnnY, where nnn shall contain the number of days for D, weeks for
    W, months for M, or years for Y.
    Example: "018M" would represent an age of 18 months.
    4 bytes fixed.

DS  A string of characters representing either a fixed point number or a
    floating point number. Decimal Strings may be padded with leading or
    trailing spaces. Embedded spaces are not allowed.
    16 bytes maximum.

IS  A string of characters representing an Integer in base-10 (decimal),
    with an optional leading "+" or "-". It may be padded with leading and
    trailing spaces. Embedded spaces are not allowed.
    12 bytes maximum.

PN  A character string encoded using a 5 component convention. The five
    components in their order of occurrence are: family name complex, given
    name complex, middle name, name prefix, name suffix. Any of the five
    components may be an empty string. The component delimiter shall be the
    caret "^" character (5EH). There shall be no more than five components.
    Component groups are separated by the "=" character (3DH): a single
    byte character representation, an ideographic representation, and a
    phonetic representation. 64 chars maximum per component group.
```

PS3.5 section 6.2 also records that the ACR-NEMA 300 forms `YYYY.MM.DD` for DA and `HH:MM:SS.frac` for TM are retired. This plan does not parse them.

PS3.3, the attributes and their modules:

| Attribute | Tag | VR | Module | Meaning used here |
|---|---|---|---|---|
| Patient's Name | (0010,0010) | PN | C.7.1.1 Patient | Already read |
| Patient's Birth Date | (0010,0030) | DA | C.7.1.1 Patient | Birth date |
| Patient's Sex | (0010,0040) | CS | C.7.1.1 Patient | Enumerated M male, F female, O other |
| Patient's Age | (0010,1010) | AS | C.7.2.2 Patient Study | Age of the patient |
| Study Date | (0008,0020) | DA | C.7.2.1 General Study | Already read |
| Study Time | (0008,0030) | TM | C.7.2.1 General Study | Time the study started |
| Institution Name | (0008,0080) | LO | C.7.5.1 General Equipment | Institution where the equipment is located |
| Slice Location | (0020,1041) | DS | C.7.6.2 Image Plane | C.7.6.2.1.2: relative position of the image plane in mm, relative to an unspecified implementation-specific reference point |
| Slice Thickness | (0018,0050) | DS | C.7.6.2 Image Plane | Nominal slice thickness in mm |
| KVP | (0018,0060) | DS | C.8.2.1 CT Image, C.8.7.2 X-Ray Acquisition | Peak kilo voltage output of the X-Ray generator used |
| Exposure | (0018,1152) | IS | C.8.2.1 CT Image, C.8.7.8 X-Ray Acquisition Dose | The exposure expressed in mAs |
| Exposure in µAs | (0018,1153) | IS | Same modules | The exposure expressed in µAs |
| Repetition Time | (0018,0080) | DS | C.8.3.1 MR Image | Period in ms between the beginning of a pulse sequence and the beginning of the succeeding pulse sequence |
| Echo Time | (0018,0081) | DS | C.8.3.1 MR Image | Time in ms between the middle of the excitation pulse and the peak of the echo produced |
| Patient Orientation | (0020,0020) | CS, VM 2 | C.7.6.1 General Image | See below |

PS3.3 C.7.6.1.1.1, Patient Orientation:

```text
The Patient Orientation (0020,0020) relative to the image plane shall be
specified by two values that designate the anatomical direction of the
positive row axis (left to right) and the positive column axis (top to
bottom). The first entry is the direction of the rows, given by the direction
of the last pixel in the first row from the first pixel in that row. The
second entry is the direction of the columns, given by the direction of the
last pixel in the first column from the first pixel in that column.

Anatomical direction shall be designated by the capital letters: A
(anterior), P (posterior), R (right), L (left), H (head), F (foot).

Each value of the orientation attribute shall contain at least one of these
characters. If refinements in the orientation descriptions are to be
specified, then they shall be designated by one or two additional characters
in each value. Within each value, the letters shall be ordered with the
principal orientation designated in the first character.
```

PS3.3 C.7.6.1.1.1 also defines quadruped terms such as `LE`, `RT`, `CR`, `CA`, `D`, `V`, and in that set `R` means rostral. Patient Orientation is Type 2C in the General Image Module, required when the image does not require Image Orientation (Patient). F-206 already reads Image Orientation (Patient) under PS3.3 C.7.6.2.1.1 and keeps that rule.

Spacing provenance is F-206's and is not reopened. `FrameFacts` reports `PixelSpacing`, else `ImagerPixelSpacing`, else empty. A present but unusable Pixel Spacing does not fall through. PS3.3 C.7.6.2.1.1 gives row spacing then column spacing. F-207's `lengthText` labels an `ImagerPixelSpacing` length `mm at the detector`.

The scale bar choice follows cornerstone3D 5.8.2 `ScaleOverlayTool.computeScaleSize` and `_getTextLines`, read from `tools/oracle/node_modules/@cornerstonejs/tools/dist/esm/tools/ScaleOverlayTool.js` at the oracle pin:

```js
const scaleSizes = [16000, 8000, 4000, 2000, 1000, 500, 250, 100, 50, 25, 10, 5, 2]
currentScaleSize = scaleSizes.filter((scaleSize) => scaleSize < worldWidthViewport * 0.6 &&
    scaleSize > worldWidthViewport * 0.2)
return currentScaleSize[0]
// label: scaleSize >= 50 -> scaleSize / 10 + ' cm', else scaleSize + ' mm'
```

## What the specification does not cover

Every rule below is this plan's. None is an HLD formula.

**Where derived text is made.** Every derived string is made in Rust, in a new `examples/folder-viewer/src/corners.rs`, and sent in the inspect JSON. The page places strings and does not reformat them. The working tree's `personName()` and `dicomDate()` in `page.html` are removed. A rule in page JavaScript has no fixture runner in this crate, and the request is a fixture for every derived-text function. The scale bar is the one exception, because its length follows the zoom, and it is the only page-side arithmetic this story adds.

**Patient name, PS3.5 6.2 PN.** Take the first value when the element is multi-valued. Take the alphabetic group, the text before the first `=`. When it is empty, take the ideographic group, then the phonetic group. Split on `^` into family, given, middle, prefix, suffix. Each component is trimmed. The display is `family, prefix given middle, suffix`, with empty parts and their separators dropped. `Doe^John^Quincy^Dr^Jr` is `Doe, Dr John Quincy, Jr`. This is the working tree rule moved to Rust, with the ideographic fallback added.

**Dates, PS3.5 6.2 DA.** Exactly eight ASCII digits that name a real Gregorian day render as `YYYY-MM-DD`. A leap year is divisible by 4 and not by 100, or divisible by 400. Anything else, including the retired `YYYY.MM.DD`, renders as the stored text unchanged and is never used for arithmetic. Empty stays empty.

**Times, PS3.5 6.2 TM.** `HH`, `HHMM`, `HHMMSS` or `HHMMSS.F` to `HHMMSS.FFFFFF` with HH 00 to 23, MM 00 to 59 and SS 00 to 60 render as `HH`, `HH:MM` or `HH:MM:SS`. The fraction is dropped and never rounded, so `235959.999999` is `23:59:59` and not midnight. No component is invented, so `1430` is `14:30` and not `14:30:00`. Anything else renders as the stored text unchanged.

**Sex, PS3.3 C.7.1.1.** `M`, `F` or `O` is shown as stored. Any other value is empty. It is not guessed.

**Age.** The rule, in order:

1. Patient's Age (0010,1010) present with a value. When it matches `nnn` digits and one of `D`, `W`, `M`, `Y`, render the number without leading zeros and the unit as a word: `045Y` is `45 years`, `001Y` is `1 year`, `018M` is `18 months`, `003W` is `3 weeks`, `012D` is `12 days`. Source `PatientAge`.
2. Patient's Age present with a value that does not match. Age is empty and source is empty. It does not fall through to a computed age, which is F-206's rule for a present but unusable Pixel Spacing.
3. Patient's Age absent or zero length. When Patient's Birth Date and Study Date are both valid DA and the study is not before the birth, compute the age at the study. Source `computed`.
4. Otherwise empty. The clock is never read. The corner shows the age at the study, and a page opened next year shows the same age.

The computed age in whole years is `sy - by`, less one when `(sm, sd) < (bm, bd)` compared as a pair. A birthday on 29 February is therefore reached on 1 March in a year that is not a leap year. When the years are 0, the months are `(sy - by) * 12 + (sm - bm)`, less one when `sd < bd`. When the months are also 0, the age is the count of Gregorian days between the two dates. The display is the largest unit that is not 0, in the same words as rule 1.

**Numbers.** Slice Location, Slice Thickness, KVP, Repetition Time and Echo Time are VM 1 DS and use F-206's `clean_decimal`, so a token that is already a JSON number is kept as stored and nothing is rounded. A multi-valued or unparseable value is null. Units are fixed by the field: mm, mm, kVp, ms, ms.

**mAs.** Exposure (0018,1152) when it is a usable IS, source `Exposure`. Otherwise Exposure in µAs (0018,1153) divided by 1000 in integer arithmetic and written as an exact decimal, so `200000` is `200` and `500` is `0.5`, source `ExposureInuAs`. Otherwise null. It is never computed from tube current and exposure time.

**Top level only.** These acquisition fields are read from the top-level data set. An enhanced CT or MR multiframe file keeps them in functional groups, under FD VRs this preview does not read, and shows them empty rather than wrong. That is a stated non-goal.

**Edge letters from Patient Orientation.** The order of precedence is Image Orientation (Patient), then Patient Orientation. When Image Orientation (Patient) is present, the F-206 rule decides, and a present but unusable value gives four empty letters and does not fall through. When it is absent and Patient Orientation has exactly two values, each of one to three characters from `A P R L H F`, and no value holds a letter together with its opposite, then:

- right edge is the first value
- left edge is the first value with each letter replaced by its opposite
- bottom edge is the second value
- top edge is the second value with each letter replaced by its opposite

Any other Patient Orientation gives four empty letters. That covers the quadruped set, where `R` is rostral. A source field reports which attribute was used. The F-207 rule that hides the letters while the picture is turned or flipped stays.

**Corner layout.**

| Corner | Lines, each omitted when empty |
|---|---|
| Top left | Patient name. Patient ID. Birth date, sex and age on one line, as `1970-01-02  F  54 years`. Institution. |
| Top right | Modality and series description. Study description. Study date and study time on one line. `Acc` and the accession. |
| Bottom left | Image n of N, instance and frame, as today. `Location` and slice location in mm, `Thickness` and slice thickness in mm. kVp and mAs on one line. `TR` and `TE` in ms on one line. |
| Bottom right | The F-207 window line and zoom, unchanged. The scale bar above them. |

**Scale bar.** Shown only in the single view, while the corners are shown, and when the preview gave a spacing source. Screen pixels per millimetre are `scale * aspectX / columnSpacing`. That equals `scale * aspectY / rowSpacing` because `greyAspect` stretches each axis by its own spacing, so the value does not depend on rotation or flip. The viewport width in mm is `wrap.clientWidth` divided by that. The length is chosen by the cornerstone rule above. The bar is that length times the pixels per mm, horizontal, right aligned in the bottom right corner. The label is the cornerstone label, followed by ` at the detector` when the source is `ImagerPixelSpacing`, the same provenance words as `lengthText`. When no candidate fits, or there is no spacing, there is no bar.

## Problem and reproducer

Open a CT from the folder picker. The corners show name, ID and study date top left, series and study top right, image and instance bottom left, window and zoom bottom right. Birth date, sex, age, study time, institution, slice location, slice thickness, kVp, mAs, TR and TE are absent, and there is no scale bar.

Open a DX or CR chest that has Patient Orientation `L\F` and no Image Orientation (Patient). `inspect_json` reports four empty edge letters and the page shows none. A radiologist expects R on the left and L on the right.

## Scope, API and compatibility

In scope:

- `examples/folder-viewer/src/corners.rs`, public functions, each taking the trimmed stored text:

```rust
pub fn person_name_text(stored: &str) -> String
pub fn dicom_date_text(stored: &str) -> String
pub fn dicom_time_text(stored: &str) -> String
pub fn age_text(patient_age: Option<&str>, birth: Option<&str>, study: Option<&str>) -> (String, &'static str)
pub fn exposure_mas(exposure: Option<&str>, micro: Option<&str>) -> (Option<String>, &'static str)
pub fn patient_orientation_letters(values: &[&str]) -> Option<[String; 4]>
```

`patient_orientation_letters` returns left, right, top, bottom, or `None` for a refused value. `age_text` returns the text and `PatientAge`, `computed` or empty.

- `frame_facts` in `preview.rs` gains the Patient Orientation branch and `FrameFacts::edge_source`, one of `ImageOrientationPatient`, `PatientOrientation` or empty.
- `inspect_json` gains the fields in the protocol table below.
- The preview reply gains `X-Ocelli-Edge-Source`.
- `page.html` places the new fields, drops `personName` and `dicomDate`, draws the scale bar, and changes the `toggle-letters` sentence to name both attributes.
- `lib.rs` re-exports the `corners` functions so the integration tests reach them.

Protocol strings that change:

| Where | Name | Value |
|---|---|---|
| inspect JSON | `patientNameText` | `person_name_text` of Patient's Name |
| inspect JSON | `birthDate` | `dicom_date_text` of (0010,0030) |
| inspect JSON | `sex` | `M`, `F`, `O` or empty |
| inspect JSON | `age`, `ageSource` | `age_text` |
| inspect JSON | `studyDateText`, `studyTime` | `dicom_date_text`, `dicom_time_text` |
| inspect JSON | `institution` | (0008,0080), trimmed |
| inspect JSON | `sliceLocation`, `sliceThickness`, `kvp`, `repetitionTime`, `echoTime` | JSON number token or `null` |
| inspect JSON | `exposureMas`, `exposureSource` | `exposure_mas` |
| inspect JSON | `edgeSource` | `FrameFacts::edge_source` |
| preview reply | `X-Ocelli-Edge-Source` | Same token |
| page sentence | `toggle-letters` | Names Image Orientation (Patient) and Patient Orientation |

Every existing field and header keeps its name and value. `patientName` and `studyDate` stay the stored text, because the series list reads them.

Non-goals: a configurable corner layout, a hide-patient-data key, functional-group acquisition fields, oblique two-letter edges from Image Orientation (Patient), Estimated Radiographic Magnification Factor, and a scale bar in the 2 by 2 view or in the F-208 grid cells. F-208 decides whether each cell has corners.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Folder scan, `POST /inspect` | `inspect_json` calling `corners.rs` | Frame 0, top-level attributes | New JSON fields, strings already formatted |
| Corner text in `paintOverlays` | `image.info` from the scan | Places strings, never reformats | Four corners |
| `POST /preview` | `frame_facts` | Requested frame | Edge headers and `X-Ocelli-Edge-Source` |
| `paintEdges` | `shownFacts` from preview headers | Hidden while turned, flipped or in a grid | Four letters |
| Scale bar | `shownFacts` row, column, source, and the page `scale`, `aspectX` | Single view, corners shown | Bar and label |
| `lengthText`, unchanged | Same `shownFacts` | Same provenance words | Length label |
| `/more` and `/use` | unchanged | None | None |

Units, rounding and selection are owned by `corners.rs` and by F-206's `clean_decimal`. The page owns only the scale bar pixel length.

## State transitions and invariants

Stateless on the server. Each inspect and preview reply is a function of the bytes and the frame.

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Scan of a CT | Birth `19700102`, study `20240305`, no Patient's Age | `age` `54 years`, `ageSource` `computed` | A study before the birth gives an empty age, and the file still lists |
| Scan of a file with Patient's Age `45Y` | Three characters | Empty age and source | No computed age replaces it |
| Preview of a DX | Patient Orientation `L\F`, no Image Orientation (Patient) | Left R, right L, top H, bottom F, source `PatientOrientation` | `LR\F` gives four empty letters and the image still draws |
| Image shown, zoom changes | Ctrl wheel | Bar length and label follow the zoom | No spacing source: no bar, and the length tool still reads px |
| Image turned 90 degrees | Rotate | Bar length unchanged | Letters hidden, as F-207 |
| Corners hidden | `A` | Bar hidden with the corners | Nothing else changes |

Invariants across consumers: the scale bar and `lengthText` read the same `shownFacts`, so they cannot disagree about mm against mm at the detector. A malformed attribute never makes `inspect_json` refuse a file that it lists today.

## Ownership, commit and recovery

Not applicable to a device or a file. The corner text lives in the page until the next scan. A failed preview keeps F-207's behaviour. The scale bar is redrawn from scratch in `layout`.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | PN rows: `Doe^John^Quincy^Dr^Jr` is `Doe, Dr John Quincy, Jr`. `Doe^John` is `Doe, John`. `^John` is `John`. `Yamada^Tarou=山田^太郎=やまだ^たろう` is `Yamada, Tarou`. `=山田^太郎` is `山田, 太郎`. `A\B` is `A`. Empty is empty. | `person_name_text_follows_ps3_5_pn` | Given name first, the ideographic group chosen while an alphabetic one exists, or the whole multi-valued string | implementation |
| AC2 | DA rows: `19700102` is `1970-01-02`. `20000229` is `2000-02-29`. `19000229`, `19701302`, `19700132`, `1970.01.02` and `197001` stay as stored. | `dicom_date_text_follows_ps3_5_da` | A regex for eight digits that accepts 29 February 1900 | implementation |
| AC3 | TM rows: `143012` is `14:30:12`. `143012.123456` is `14:30:12`. `1430` is `14:30`. `14` is `14`. `235959.999999` is `23:59:59`. `235960` is `23:59:60`. `2400`, `143`, `14:30:12` and `146012` stay as stored. | `dicom_time_text_follows_ps3_5_tm` | Rounding the fraction to midnight, or inventing `:00` seconds | implementation |
| AC4 | AS rows: `045Y` is `45 years`. `001Y` is `1 year`. `018M` is `18 months`. `003W` is `3 weeks`. `012D` is `12 days`. `000D` is `0 days`. `45Y` and `045y` are empty with an empty source and no computed age. | `patient_age_follows_ps3_5_as` | Falling through to the computed age on a malformed AS | implementation |
| AC5 | Computed rows, birth then study: `19700102` `20240305` is `54 years`. `19700306` `20240305` is `53 years`. `19700305` `20240305` is `54 years`. `20000229` `20010228` is `11 months`. `20000229` `20010301` is `1 year`. `20240115` `20240305` is `1 month`. `20240301` `20240315` is `14 days`. `20240305` `20240301` is empty. Birth with no study date is empty. | `computed_age_is_the_age_at_the_study` | Reading the clock, or subtracting years only | implementation |
| AC6 | Exposure `200` is `200`, source `Exposure`. No Exposure and µAs `200000` is `200`. µAs `500` is `0.5`. µAs `1234567` is `1234.567`. Both absent is null. | `exposure_mas_prefers_exposure_then_micro` | A float division that prints `0.49999` | implementation |
| AC7 | Patient Orientation `L\F` gives left R, right L, top H, bottom F. `A\F` gives left P, right A, top H, bottom F. `A\FR` gives bottom FR and top HL. `LR\F`, `X\F`, `A`, `ABCD\F` and the quadruped `RT\V` give `None`. | `patient_orientation_follows_ps3_3_c_7_6_1_1_1` | The first value on the top edge, or the first letter used alone | implementation |
| AC8 | A projection file with Patient Orientation `L\F` and no Image Orientation (Patient) previews with `X-Ocelli-Edge-Left: R` and `X-Ocelli-Edge-Source: PatientOrientation`. The same file with the axial `1\0\0\0\1\0` added reports source `ImageOrientationPatient` and F-206's letters. With a non-unit orientation added it reports four empty letters. | `patient_orientation_is_used_only_without_image_orientation` | Patient Orientation overriding Image Orientation (Patient), or rescuing an unusable one | implementation |
| AC9 | A built CT with every new attribute reports each new inspect field, number tokens unrounded. Slice Location `-12.50` stays `-12.50`. The synthetic study still has no Patient's Name, and its new fields are empty or null. | `inspect_json_reports_the_corner_fields` | Rounding a DS, or a hard-coded value in the phantom | implementation |
| AC10 | Mutating one constant flips a test red: the leap-year divisor, the `<` in the birthday comparison, or the opposite of `F`. | Recorded in the review | A test that passes on the wrong code | implementation, reviewer |
| AC11 | The page no longer defines `personName` or `dicomDate`, reads `patientNameText` and `age`, contains the scale list `16000, 8000, 4000, 2000, 1000, 500, 250, 100, 50, 25, 10, 5, 2`, the bounds `0.2` and `0.6`, and ` at the detector` in the bar label, and the letters sentence names Patient Orientation. | `the_page_offers_a_folder_picker` extended | The JavaScript formatter left in place beside the Rust one | implementation |
| AC12 | In a browser, a 0.5 mm CT at scale 2 in an 800 px viewport shows a bar of 400 CSS px labelled `10 cm`. A 0.143 mm Imager Pixel Spacing at scale 0.25 in a 1000 px viewport shows 437 px labelled `25 cm at the detector`. Row 0.25 and column 0.5 at scale 1 in 600 px show 200 px labelled `5 cm`. | Click-through at sprint close, measured with the inspector | Using `scale / columnSpacing` without `aspectX`, which gives `10 cm` in the third case | integrated, not claimed by Rust tests |
| AC13 | Before `corners.rs` and the Patient Orientation branch are implemented, the reviewer checks every transcription above against the DICOM standard text at dicom.nema.org, current edition: PS3.5 section 6.2 Table 6.2-1 (the DA, TM, AS, DS, IS and PN rows, and the retired ACR-NEMA forms), PS3.3 C.7.1.1 (Patient), C.7.2.1 (General Study), C.7.2.2 (Patient Study), C.7.5.1 (General Equipment), C.7.6.1 and C.7.6.1.1.1 (General Image, Patient Orientation), C.7.6.2 and C.7.6.2.1.2 (Image Plane, Slice Location), C.8.2.1 (CT Image), C.8.3.1 (MR Image), C.8.7.2 (X-Ray Acquisition) and C.8.7.8 (X-Ray Acquisition Dose). Any correction lands in this plan before the code | Review note naming the edition read and each section | Fixtures citing rows nobody checked | reviewer, before implementation |

The scale fixtures in AC12, worked. First, `2 * 1 / 0.5 = 4` px per mm, `800 / 4 = 200` mm, the bounds are 40 and 120, and the first candidate below 120 and above 40 is 100, which is `10 cm` and `100 * 4 = 400` px. Second, `0.25 / 0.143 = 1.7483` px per mm, `1000 / 1.7483 = 572.0` mm, the bounds are 114.4 and 343.2, the candidate is 250, which is `25 cm` and `250 * 1.7483 = 437.1` px. Third, `greyAspect` gives `least = 0.25` and `aspectX = 0.5 / 0.25 = 2`, so `1 * 2 / 0.5 = 4` px per mm, and the vertical check `1 * 1 / 0.25 = 4` agrees. The view is `600 / 4 = 150` mm, the bounds are 30 and 90, the candidate is 50, which is `5 cm` and 200 px. The wrong rule `1 / 0.5 = 2` px per mm gives a 300 mm view, bounds 60 and 180, and `10 cm` at 200 px.

## Approach

1. Add `corners.rs` with the six functions and a private Gregorian day count. No dependency is added.
2. In `preview.rs`, add the tag constants. Read each new attribute with `optional_text`, which already trims. Call `corners.rs`. Push each field with the existing `push_field` and `push_json_token`.
3. In `orientation_letters`, distinguish absent from unusable Image Orientation (Patient). Only absent reads Patient Orientation (0020,0020) from the top level through `token_list`. Return the source with the letters.
4. In `serve.rs`, carry `edge_source` in `Grey` and write `X-Ocelli-Edge-Source`.
5. In `page.html`, rebuild `paintOverlays` from the table, add `paintScale` called from `layout` and `paintOverlays`, remove the two formatters, and edit the letters sentence.

Rejected: formatting in the page, which has no fixture runner. A Rust scale bar, which would need a request per zoom step. A second spacing read for the bar.

## Dependencies, write ownership and resources

Declared dependency F-206, done. Source-level dependency on F-207's `paintOverlays`, `paintEdges`, `lengthText` and `greyAspect`, done.

Writes `examples/folder-viewer/src/corners.rs` (new), `src/preview.rs`, `src/serve.rs`, `src/lib.rs`, `src/page.html` and `tests/preview.rs`.

`page.html`, `preview.rs`, `serve.rs` and `tests/preview.rs` are also written by F-208, F-209, F-210, F-211, F-213, F-214, F-215 and F-216 in this sprint. They cannot have concurrent writers.

**Sprint build order, decided in the S46 design round on 2026-11-05.** F-217, F-216, F-210, F-211, F-208, F-209, then this story, then F-213, F-214 and F-215. F-119 is independent. This story therefore writes the new `inspect_json` fields once, and F-216's `POST /file` returns them because its body is byte-identical to `/inspect`. The scale bar reads F-208's per-viewport `scale` and `aspectX`, and is drawn in the single view of the active layout cell as F-208 shapes it. The working tree already holds uncommitted edits to `page.html` and `serve.rs` from the S46 opening polish. The implementer starts from those edits, not from `s45`. No GPU.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-folder-viewer` while iterating, and `bin/ocelli.sh check ocelli-folder-viewer`. The sprint gate runs once for the sprint. A later edit to any file in the write set invalidates the record. AC12 is a separate browser observation at sprint close and is not a passed row until someone measures it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. Text fields and one header are added to the local HTTP reply.
- Render-loop allocation: n/a. This crate does not render on a device.
- unsafe: none
- Tier A (WebGPU): n/a. This page does not open a device.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a for the corner text. The grey image stays F-206's CPU `LutChain` and is unchanged.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | PN display, PS3.5 6.2 | `examples/folder-viewer/tests/preview.rs`, `person_name_text_follows_ps3_5_pn` |
| fixture | DA display and Gregorian validity, PS3.5 6.2 | same, `dicom_date_text_follows_ps3_5_da` |
| fixture | TM display, PS3.5 6.2 | same, `dicom_time_text_follows_ps3_5_tm` |
| fixture | AS display, PS3.5 6.2, and the no-fallthrough rule | same, `patient_age_follows_ps3_5_as` |
| fixture | Computed age from two DA values | same, `computed_age_is_the_age_at_the_study` |
| fixture | mAs from IS, PS3.3 C.8.2.1 | same, `exposure_mas_prefers_exposure_then_micro` |
| fixture | Edge letters, PS3.3 C.7.6.1.1.1 | same, `patient_orientation_follows_ps3_3_c_7_6_1_1_1` |
| integration | Precedence through `preview_frame` and the HTTP reply | same, `patient_orientation_is_used_only_without_image_orientation` |
| integration | Every new inspect field | same, `inspect_json_reports_the_corner_fields` |
| contract | The page reads the fields and draws the bar | same, `the_page_offers_a_folder_picker` |

No LUT arithmetic changes. The fixtures are hand-computed from the cited rows and are not read back from the implementation.

## Parity surface covered

None. Appendix B has no corner-text row, and cornerstone3D 5.8.2 has no four-corner overlay. The scale bar copies the `ScaleOverlayTool` choice rule and is not measured against a cornerstone frame.

## Deviations

none

## LLD impact

none. The local example is not the published API.

## User-facing documentation impact

- `README.md`, "Start the local series viewer", step 7: the corners also show birth date, sex, age, study time, institution, slice location and thickness, kVp, mAs, TR and TE when the file has them. Edge letters come from Image Orientation (Patient) or, on a projection image without it, from Patient Orientation. A scale bar shows in the single view when the file gives a spacing, and says "at the detector" for Imager Pixel Spacing.
- `examples/folder-viewer/src/use.html`: no change. `corners.rs` is in the example and adds no library export.
- `examples/folder-viewer/src/more.html`: no change. No card covers corner text or a scale bar, and `#metadata` stays true.

## Open questions

1. **Transcription check.** The PS3.5 and PS3.3 rows above are not in the repository. Affects AC1 to AC7. Owner: reviewer. Option: confirm against the current edition, or add the rows to the dicom-expert skill first. Deferral leaves the fixtures citing text nobody has checked.
2. **Malformed Patient's Age.** This plan shows nothing rather than a computed age, for consistency with F-206. The alternative is to treat a malformed AS as absent and compute. Affects AC4 and AC5. Owner: operator. Recommendation: keep the plan's rule. Deferral is harmless, because it is one branch.
3. **Age words.** `45 years` rather than the stored `045Y`, because `18M` beside the sex `M` reads as male. Owner: operator. Recommendation: words. Deferral keeps the plan's choice.
4. **Enhanced multiframe acquisition fields.** Left empty. Reading Exposure in mAs (0018,9332) and the MR timing sequences needs FD support in this preview. Owner: operator. Recommendation: a follow-up story. Deferral leaves those corner lines empty on enhanced files, never wrong.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

1. Answered: the flag stays, and AC13 makes the reviewer check the PS3.5 and PS3.3 rows against dicom.nema.org, current edition, before implementation of that code.
2. Answered: recommended, keep the plan's rule. A malformed Patient's Age shows nothing and no computed age replaces it.
3. Answered: recommended, age in words, `45 years` rather than `045Y`.
4. Answered: recommended, enhanced multiframe acquisition fields stay empty in this story, and a follow-up story is filed by the sprint lead at `/complete-feature`.

No question is left open.
