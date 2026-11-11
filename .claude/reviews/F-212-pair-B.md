# F-212 review, pass 1, implementation B

**Reviewed**: commit b236d6f2 against base 5451cb7d, worktree clean apart from the untracked `.venv` and `tools/oracle/out` links
**Author / reviewer**: author withheld for a blind comparison. Reviewer is an independent Claude reviewer who did not write either side and did not read authorship data
**Result**: 1 defect, 2 smells, 2 nitpicks

## Acceptance challenged

| Row | Verdict | Note |
|-----|---------|------|
| AC1 | Met | `^=山田^太郎` falls through to the ideographic group too |
| AC2 | Met | Extra rows for 1600, 2020 and 2100 |
| AC3 | Met | Includes `143012.` and a seven digit fraction. PS3.5 2026d TM: "If FFFFFF is unspecified the preceding "." shall not be included" |
| AC4 | Met | Malformed rows carry dates that a fall-through would use |
| AC5 | Met | Leap rows across February in 2024, 2020 and 2023 |
| AC6 | Met | 12 byte and 32-bit range both enforced |
| AC7 | Met | |
| AC8 | Met | Also covers a binary and a zero-length Image Orientation (Patient) |
| AC9 | Met | Also checks the HTTP `/inspect` body is byte-identical |
| AC10 | Met for the three named mutations | Survivors in S1 and S2 |
| AC11 | Met | |
| AC12 | Not claimed, not measured | Browser click-through at sprint close. The formula matches the plan's worked cases |
| AC13 | Checked by this reviewer against PS3.5 and PS3.3 2026d | DA, TM, AS, DS, IS and PN agree. The plan's quadruped list writes `CA` where the standard writes `CD`. That is a plan error and does not touch the code |
| Amendment 1 | Met | `X-Ocelli-Calibration-Source`, `-Row`, `-Column` from `choose_spacing` |
| Amendment 2 | Met | QUADRUPED clears letters from both attributes. PS3.3 C.7.6.2.1.1 gives quadruped axes as dorsal and cranial, so this is the correct reading |
| Amendment 3 | Met | Both a US and an SQ plant, and `CornerText::NotText` keeps a binary value out of the corner |
| Amendment 4 | Met | `paintScale(vp)` runs through `paintOverlays` from `layoutViewport` on `dom.cell.clientWidth` |
| Amendment 5 | Met | |
| Amendment 6 | Met | Fields precede `planes` |
| Amendment 7 | Met | Step 8 edited |

I ran the same hand-derived probe on both sides, and B returned the expected value on every row. The rows were birth 20240131 with study 20240229 (29 days), 20000228 to 20000301 (2 days), 19000228 to 19000301 (1 day), 19991231 to 20000101 (1 day), 20240229 to 20250228 (11 months), 20240229 to 20250301 (1 year), and the same day (0 days). The probe also covered a stored `045Y` with the study before the birth, which gives `45 years` from PatientAge. The TM rows were `0000`, `1430.5`, `14301`, `23`, `24`, `143060` and `143012.1`. The PN rows were `Doe^^^^Jr`, `^^^Dr`, `Doe^John^^^` and `^=山田^太郎`. The DA rows included 2024, 2023, 2100 and 1600 on 29 February. The IS rows were a 13 byte value and `-2147483648`. The rest were QUADRUPED with Image Orientation (Patient), BIPED with it, binary plants in six attributes, and a zero-length Patient's Age beside a lowercase sex and a multi-valued KVP.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree at b236d6f2 | now | exit 0, preview binary 87 passed |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings` | same | now | exit 0 |
| `bin/ocelli.sh gate viewer-browser` | same, synthetic folders, OCELLI_PORT=0 | now | ALL GREEN, exit 0 |
| `cargo fmt --all --check`, `prose_check.py`, `source_provenance_check.py` | same | now | all clean |
| reviewer probe appended to a scratch copy of `tests/preview.rs` | scratch copy of b236d6f2 | now | every row as expected |
| 21 mutations, each in a scratch copy with its own target dir | scratch copy | now | table below |

| Mutation | Result |
|----------|--------|
| M1 leap divisor 400 to 401 | red |
| M2 birthday `<` to `<=` | red |
| M3 opposite of F | red |
| M4 accept a 7 digit fraction | red |
| M5 seconds limit 60 to 59 | red |
| M6 month rule `sd < bd` to `sd <= bd` | red |
| M7 µAs over 100 | red |
| M8 IS range check dropped | red |
| M9 right and left edges swapped | red |
| M10 QUADRUPED token | red |
| M11 unusable IOP rescued by Patient Orientation | red |
| M12 malformed AS falls through | red |
| M13 calibration source constant | red |
| M14 column header carries the row value | **survived** |
| M15 binary value read as text | red |
| M16 binary Patient's Age falls through | red |
| M18 month 13 accepted | red |
| M19 PN phonetic group first | red |
| M20 year loop in `day_number` ignores leap years | **survived** |
| M21 page drops `aspectX` | red |
| M22 `X-Ocelli-Edge-Source` header dropped | red |

## Defects

### D1, README says `at the detector` is for Imager Pixel Spacing alone
**Where**: `README.md`, step 8
**What**: "says `at the detector` for Imager Pixel Spacing alone". `choose_spacing` also returns `detector` when Pixel Spacing equals Imager Pixel Spacing and there is no magnification factor. Its doc names that as the PS3.3 10.7.1.1 case. A file that has both attributes then gets the words, and "alone" says it would not.
**Why it is wrong**: a false sentence in README is a defect under microscope section 3.
**Evidence**: the `choose_spacing` doc in `measure.rs` says the equal-pair case "takes the imager path: `detector` with no factor". The page appends the words on `calibrationSource === "detector"`.
**Scope and regression**: documentation only. Say "when the spacing is at the detector".

## Smells

### S1, no days row crosses a year boundary after a leap year
M20 makes every year 365 days in `day_number` and every test stays green. Birth 20001215 and study 20010105 is 21 days, and the mutant gives 20. The shipped code is correct. The test just would not notice if it broke.

### S2, the calibration headers are only tested with square spacing
M14 writes `{row}` into `X-Ocelli-Calibration-Column` and every test stays green. The bar reads the column value, so on AC12's third case (row 0.25, column 0.5) a swap would draw the wrong bar with nothing going red.

## Nitpicks

- N1, the `toggle-corners` sentence now names the scale bar. The plan's protocol table does not list that change. The sentence is true.
- N2, `paintScale` runs from both branches of `paintOverlays` and again on every layout. That is harmless, but `paintOverlays` is now the only path to the bar, so a later caller that skips it skips the bar too.

## Verified clean

- No `unsafe`, no wasm-bindgen, no dependency change, and the diff adds no `as` cast.
- TM, DA, AS, IS, PN and Patient Orientation were checked against PS3.5 and PS3.3 2026d, including the QUADRUPED axes of C.7.6.2.1.1.
- `edge_letters` reads Image Orientation (Patient) first, keeps an unusable value from falling through (M11 red), and treats zero length as absent.
- `corner_text` separates absent, text and not-text, and both of its guards go red under mutation (M15, M16).
- The page comment "a stored -12.50 shows as -12.5" is true, because `JSON.parse` yields a number.
- `showFault` hides the bar.
- No en-dash or em-dash was added. Prose and provenance checks are clean.

## Limitations and handoff

- AC12 was not measured in a browser. The formula was compared with the plan's three worked cases.
- The rendered page with real DX or CR files was not exercised. Only synthetic folders were used.
- Next owner: the implementer fixes D1, S1 and S2, then a fresh pass is needed.
