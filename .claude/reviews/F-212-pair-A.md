# F-212 review, pass 1, implementation A

**Reviewed**: commit 0e896db2 against base 5451cb7d, worktree clean apart from the untracked `.venv` and `tools/oracle/out` links
**Author / reviewer**: author withheld for a blind comparison. Reviewer is an independent Claude reviewer who did not write either side and did not read authorship data
**Result**: 6 defects, 4 smells, 3 nitpicks

## Acceptance challenged

| Row | Verdict | Note |
|-----|---------|------|
| AC1 | Met for the listed rows | `^=山田^太郎` shows nothing, see N2 |
| AC2 | Met | |
| AC3 | Met | The `143012.` row stays stored, which PS3.5 2026d TM supports: "If FFFFFF is unspecified the preceding "." shall not be included" |
| AC4 | Met | |
| AC5 | Listed rows pass, the rule is wrong | D1. Any days answer that crosses 29 February of a leap year is one too many |
| AC6 | Met for the listed rows | D4 on the 12 byte rule |
| AC7 | Met | |
| AC8 | Met | |
| AC9 | Met | |
| AC10 | Met for the three named mutations | Survivors in S1 to S4 |
| AC11 | Met | |
| AC12 | Not claimed, not measured | Browser click-through at sprint close. The formula matches the plan's worked cases |
| AC13 | Checked by this reviewer against PS3.5 and PS3.3 2026d | DA, TM, AS, DS, IS and PN agree. The plan's quadruped list writes `CA` where the standard writes `CD`. That is a plan error and does not touch the code |
| Amendment 1 | Met | `X-Ocelli-Spacing-Choice`, `X-Ocelli-Row-Mm`, `X-Ocelli-Column-Mm` from `choose_spacing` |
| Amendment 2 | Partly met | D2. Only the Patient Orientation path honours QUADRUPED |
| Amendment 3 | Met for a sequence, not for a binary value | D3 |
| Amendment 4 | Met | `paintScale(vp)` runs from `layoutViewport` on `vp.dom.cell` |
| Amendment 5 | Met apart from D4 | |
| Amendment 6 | Met | Fields precede `planes` |
| Amendment 7 | Met | Step 8 edited |

Values I derived by hand and ran through the public API (identical probe on both sides):

- Birth 20240220, study 20240305. 2024 is a leap year, so 9 days to 29 February and 5 more, 14 days. A gives `15 days` through `inspect_json`.
- Birth 20000228, study 20000301. 2000 is divisible by 400, so 28 February to 29 February to 1 March is 2 days. A gives `3 days`.
- Birth 20240131, study 20240229, 29 days. A gives 29, because both dates are before March.
- QUADRUPED with Image Orientation (Patient) `1\0\0\0\1\0`. Expected four empty letters. A gives `R|L|A|P|ImageOrientationPatient`.
- IS `0000000000200`, 13 bytes. PS3.5 6.2 caps IS at 12 bytes. Expected not an IS. A gives `200` from `Exposure`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree at 0e896db2 | now | exit 0, preview binary 88 passed |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings` | same | now | exit 0 |
| `bin/ocelli.sh gate viewer-browser` | same, synthetic folders, OCELLI_PORT=0 | now | ALL GREEN, exit 0 |
| `cargo fmt --all --check`, `prose_check.py`, `source_provenance_check.py` | same | now | all clean |
| reviewer probe appended to a scratch copy of `tests/preview.rs` | scratch copy of 0e896db2 | now | the findings above, and the binary plant output quoted in D3 |
| 19 mutations, each in a scratch copy with its own target dir | scratch copy | now | table below |

| Mutation | Result |
|----------|--------|
| M1 leap divisor 400 to 401 | red |
| M2 birthday `<` to `<=` | red |
| M3 opposite of F | red |
| M4 accept a 7 digit fraction | **survived** |
| M5 seconds limit 60 to 59 | red |
| M6 month rule `sd < bd` to `sd <= bd` | **survived** |
| M7 µAs over 100 | red |
| M8 IS upper bound dropped | red |
| M9 right and left edges swapped | red |
| M10 QUADRUPED token | red |
| M11 unusable IOP rescued by Patient Orientation | red |
| M12 malformed AS falls through | red |
| M13 calibration source constant | red |
| M14 column header carries the row value | **survived** |
| M18 month 13 accepted | survived, equivalent mutant, `days_in_month` returns 0 for month 13 |
| M19 PN phonetic group first | red |
| M20 remove the extra leap day in `gregorian_day` (the fix for D1) | **survived** |
| M21 page drops `aspectX` | red |
| M22 `X-Ocelli-Edge-Source` header dropped | red |

## Defects

### D1, the Gregorian day count adds 29 February twice
**Where**: `examples/folder-viewer/src/corners.rs`, `gregorian_day`
**What**: `earlier` already sums `days_in_month(year, past)`, which is 29 for February in a leap year, and then `if month > 2 && is_leap(year) { day_of_year += 1 }` adds the leap day again. Every date after February in a leap year is one day late, so a computed age in days that spans 29 February is one too many.
**Why it is wrong**: PS3.5 6.2 DA is the Gregorian calendar, and the plan's rule is "the count of Gregorian days between the two dates". The doc comment "any monotone count would do" is also false, because a difference needs a uniform count, not a monotone one.
**Evidence**: birth 20240220 and study 20240305 give `"age":"15 days"` through `inspect_json`, expected 14. `age_text(None, Some("20000228"), Some("20000301"))` gives `3 days`, expected 2. M20 removes the extra line and all of A's tests stay green, so the suite passes on both the wrong code and the right code.
**Scope and regression**: any patient younger than one month whose birth and study straddle 29 February of a leap year. This is the corner text, so it is patient-visible. Regression: the AC5 row `20240220` `20240305` gives `14 days`.

### D2, QUADRUPED does not clear letters derived from Image Orientation (Patient)
**Where**: `examples/folder-viewer/src/preview.rs`, `orientation`
**What**: the QUADRUPED check runs only after the Image Orientation (Patient) branch has returned, so a quadruped CT or MR shows human letters `R L A P H F`.
**Why it is wrong**: amendment 2 says the edge letters follow Anatomical Orientation Type and are empty for QUADRUPED, "because the quadruped letter set overlaps the human one and would be drawn as human directions". PS3.3 2026d C.7.6.2.1.1 defines +y as dorsal and +z as cranial or rostral for a quadruped, so `A` and `P` from the F-206 rule mislabel the edge.
**Evidence**: QUADRUPED plus `1\0\0\0\1\0` previews as `R|L|A|P|ImageOrientationPatient`.
**Scope and regression**: veterinary cross-sectional images. Regression: QUADRUPED plus axial cosines gives four empty letters.

### D3, a binary value in a corner attribute is shown as text
**Where**: `examples/folder-viewer/src/preview.rs`, `inspect_json` corner block, through `optional_text`, which calls `element.to_str()`
**What**: `to_str` formats a binary primitive as its number. A UL 19700102 at Patient's Birth Date shows `1970-01-02` and computes `54 years`. A US 7 at Patient's Name shows `7`, and at Slice Location shows `Location 7 mm`.
**Why it is wrong**: amendment 3 asks for every new read to be non-failing, and for a non-text value to leave that corner field empty. The added test plants only an SQ, which `to_str` refuses, so it cannot see this path.
**Evidence**: probe output `"patientNameText":"7"`, `"birthDate":"1970-01-02"`, `"age":"54 years"`, `"ageSource":"computed"`, `"sliceLocation":7`.
**Scope and regression**: files with a wrong explicit VR on these tags. Regression: plant US or UL in each new attribute and expect the field empty or null.

### D4, `integer_string` documents a 12 byte limit it does not enforce
**Where**: `examples/folder-viewer/src/corners.rs`, `integer_string`
**What**: the doc says "at most 12 bytes", and the code checks only the digits and the 32-bit range. `0000000000200` is 13 bytes and is accepted as 200 mAs.
**Why it is wrong**: PS3.5 2026d Table 6.2-1 gives IS "12 bytes maximum". The doc sentence is false about the code.
**Evidence**: `exposure_mas(Some("0000000000200"), None)` gives `(Some("200"), "Exposure")`.
**Scope and regression**: malformed IS values only. Regression: a 13 byte zero-padded row expecting the next source.

### D5, `orientation` documents an empty source that the code does not return
**Where**: `examples/folder-viewer/src/preview.rs`, the doc of `orientation`
**What**: the doc says the source "is empty when neither is present". With QUADRUPED and neither orientation attribute, the code returns `PatientOrientation`.
**Why it is wrong**: a false claim in a doc comment, in the class microscope section 3 names.
**Evidence**: probe `QUADRUPED` alone gives `||||PatientOrientation`. The added `quadruped_orientation_type_leaves_the_letters_empty` test pins `PatientOrientation` for a file that does carry Patient Orientation, so it does not cover this case.
**Scope and regression**: the `edgeSource` field and the `X-Ocelli-Edge-Source` header. Smallest fix: return `""` when Patient Orientation is absent, or delete the sentence.

### D6, README says the bar adds `at the detector` for Imager Pixel Spacing
**Where**: `README.md`, step 8
**What**: "its label says `at the detector` for Imager Pixel Spacing". The page adds the words only when `choose_spacing` returns `detector`. Imager Pixel Spacing with a usable magnification factor returns `imager-spacing`, and the label is plain `cm` or `mm`.
**Why it is wrong**: a false sentence in README is a defect under microscope section 3. A's own test `the_bar_and_the_length_tool_follow_one_choice` shows the `imager-spacing` source for that file.
**Scope and regression**: documentation only. Say "when the spacing is at the detector".

## Smells

### S1, no TM row has a seven digit fraction
M4 lets `(8..=14)` accept `143012.1234567` and every test stays green. PS3.5 TM allows 1 to 6 digits.

### S2, no month-branch row has the study day equal to the birth day
M6 turns `sd < bd` into `sd <= bd` and every test stays green. The row `20240105` `20240305` is `2 months` and would catch it.

### S3, AC5 has no days row that crosses 29 February of a leap year
This is the test half of D1. Both the shipped arithmetic and the corrected one pass. It is kept as a separate finding because it is the reason D1 survived the author's own mutation loop.

### S4, the calibration headers are only tested with square spacing
M14 writes the row value into `X-Ocelli-Column-Mm` and every test stays green. The bar reads `columnMm`, so on AC12's third case (row 0.25, column 0.5) a swap would draw the wrong bar with nothing going red.

## Nitpicks

- N1, `layoutViewport` calls `paintScale` directly and again through `paintOverlays`, so it runs twice per layout.
- N2, `^=山田^太郎` gives an empty name, because the alphabetic group `^` is not empty as text and has no component. The plan's "when it is empty" is ambiguous here.
- N3, `paintScale` measures `getBoundingClientRect().width` where the plan names `clientWidth`. The two differ by the cell border.

## Verified clean

- No `unsafe`, no wasm-bindgen, no dependency change. The diff adds no `as` cast, and its conversions are `u32::from`, `i64::from` and `u32::try_from`.
- TM, DA, AS, PN and the Patient Orientation rule were checked against PS3.5 and PS3.3 2026d.
- The calibration headers come from `choose_spacing`, the function `/measure` uses, and M13 proves the test catches a fixed source.
- The page no longer defines `personName` or `dicomDate`, and M21 proves the contract test catches the missing `aspectX`.
- Fields are written before `planes`. `patientName` and `studyDate` still hold the stored text.
- No en-dash or em-dash was added. Prose and provenance checks are clean.

## Limitations and handoff

- AC12 was not measured in a browser. The formula was compared with the plan's three worked cases.
- Unconfirmed concern: `showFault` does not hide `.scale`, so a bar painted for the previous image may stay over a fault message until the next layout. Not reproduced.
- The rendered page with real DX or CR files was not exercised. Only synthetic folders were used.
- Next owner: the implementer fixes D1 to D6 and S1 to S4, then a fresh pass is needed.
