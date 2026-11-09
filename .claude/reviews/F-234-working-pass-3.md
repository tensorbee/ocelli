# F-234 review, pass 3

**Reviewed**: `git diff 3363a6fb 7077a257` on `work/f-234-xai`, HEAD `7077a257f91d0d8597e88d26f25779be9685d49b`, tree `cb28d8bd905deec16d6a325dcfe59fd0c18a1d72`. The range touches `parse.rs`, `tests/part10.rs` and `docs/lld/dicom-ingest.md`, and it also commits the pass 1 and pass 2 reports. `git status` was empty before the pass. Afterwards the only new path is this report, and HEAD and `git write-tree` are unchanged.
**Author / reviewer**: GLM in opencode did the remediation, per the caller. The reviewer is a Claude subagent spawned by the lead. It is independent of the remediation and did not write the plan, the lead ruling, or the pass 1 and pass 2 reports.
**Result**: 1 defect, 1 smell, 2 nitpicks

## Acceptance challenged

- **AC9 and the lead ruling after pass 2, the value of the constant.** I derived the deepest nesting myself from the PS3.16 2026d template tables (TID 1500, 1501, 300, 301, 310, 311, 312, 315, 320, 321, 1000, 1001 to 1010, 1410, 1411, 1419, 1420, 1502, 1600 to 1602) and the PS3.3 2026d macros (C.18 NUM, C.18.2, C.18.3, C.18.4, C.18.8, 8.8, 10.8, C.12.1.1.9), and I did not start from the fixture. The ruling's rule is that every sequence a conformant file may carry counts, Type 3 included. Applied to every row, that rule gives 8 open Sequences and 10 wrapped. The tree says 7 and 9. The value 64 is still right, because 4 x 10 = 40 and the next power of two is still 64.
- **The fixture's tag inventory and VRs.** Every tag was checked against the PS3.6 2026d registry and the dumped bytes were read back with pydicom.
- **Limit tests against mutation.** I mutated the constant, the comparison, the decrement and the check itself, and `DEEPEST_SHAPE_DEPTH` and the fixture as well.
- **The odd-length deflate test.** I checked it against the constant and mutated the PS3.5 A.5 pad check in both directions.
- **Stack claims at 64.** AC3 runs at 100,000 deep on 256 KiB, and AC1 runs on 2 MiB in the debug test profile. I measured the headroom by raising the constant.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree at 7077a257 | now | 0 |
| `cargo clippy -p ocelli-dicom --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-dicom -p ocelli-folder-viewer` | same | now | 0. part10 52 passed. Folder viewer 72, 60, 1, 1, 1, 24 |
| `cargo test -p ocelli-dicom --test corpus -- --ignored` | scratch copy under `target/f234-p3` from `git archive HEAD`, with the main checkout's `corpus/data` symlinked and its own target dir | now | 0, 4 passed |
| `prose_check.py`, `source_provenance_check.py`, `guard_census.py` | worktree | now | 0 ("892 file(s)"), 0 ("1759 files"), 0 ("1134 refusal(s) ... all claimed", 0 watched by nothing) |
| pydicom 3.0.2 depth scan of `corpus/data` | main checkout, 100 DICOM files | now | deepest 2, `synthetic/ct_multiframe_perframe.dcm` |
| `REPOSITORY_TID_1500` pulled out of `part10.rs` and scanned | 2974 bytes | now | depth 5, ContentSequence x3 > MeasuredValueSequence > MeasurementUnitsCodeSequence |
| `deepest_measured_shape_data_set` written to a file by a scratch test and dumped with pydicom | scratch copy | now | 9 open Sequences. Tags and VRs as in Verified clean. `NumericValue (0040,A30A)` is written before `MeasurementUnitsCodeSequence (0040,08EA)` in the bytes (see S1) |
| PS3.6 2026d `chapter_6.html` registry lookup | dicom.nema.org, page headed "PS3.6 2026d" | now | every fixture tag matches its name and VR, see Verified clean |
| PS3.16 2026d `chapter_A.html` template tables, PS3.3 2026d C.18, C.18.2, C.18.3, C.18.4, C.18.8, 8.8, 10.8, C.12 | dicom.nema.org, pages headed 2026d | now | see D1 |
| pydicom model of TID 1500 r6 > r9 TID 1501 > r10 TID 300 NUM > TID 301 r11 TID 315 CODE > r2 NUM, with Measured Value Sequence > Measurement Units Code Sequence > Equivalent Code Sequence, wrapped in (0400,0561) > (0400,0550) | scratch | now | 8 unwrapped, 10 wrapped. `parse_part10` then `MetadataSet::from_object` gives `Ok` at 64 |
| M1, constant 64 to 32 | scratch copy | now | compile error, `assert!(MAX_SEQUENCE_DEPTH >= 4 * DEEPEST_SHAPE_DEPTH)` |
| M2, `DEEPEST_SHAPE_DEPTH` 9 to 17 | scratch copy | now | the same compile error |
| M3, `>=` to `>` | scratch copy | now | 3 red: `refuses_one_past_the_sequence_depth_limit`, `refuses_deep_nesting_in_a_deflated_data_set`, the proptest |
| M4, refuse at the limit (`sequence_depth + 1 >= MAX`) | scratch copy | now | 4 red: `parses_at_the_sequence_depth_limit`, the sibling and many-items tests, the proptest |
| M5, `sequence_depth -= 1` deleted | scratch copy | now | 2 red: the sibling and many-items tests (pass 1 D2 replay) |
| M6, the depth check deleted | scratch copy | now | `refuses_deep_nesting_without_recursing` and the folder viewer test `a_sequence_deep_enough_to_abort_an_unchecked_parse_is_refused_and_the_process_answers_next` both stop with "has overflowed its stack" (pass 1 D3 replay) |
| M7, pad check accepts an unpadded odd stream | scratch copy | now | 2 red: `deflated_data_set_requires_null_padding_when_the_stream_length_is_odd` and `a_deflated_odd_length_stream_requires_and_accepts_its_null_pad` |
| M8, pad check refuses the NULL pad | scratch copy | now | 4 red, including `a_deflated_odd_length_stream_requires_and_accepts_its_null_pad` |
| M9, constant 63, 128, 192, 256 | scratch copy | now | 63, 128 and 192 all 52 passed. 256 overflows the 2 MiB debug thread |
| M10, the nested presentation-state level removed from the fixture | scratch copy | now | 52 passed (see N1) |

The scratch copy, its target dir, the venv, the downloaded NEMA pages and the generated files were all deleted with Python `shutil` and `os.remove`. The worktree was never checked out, restored, reset, stashed or cleaned.

## Defects

### D1, the deepest template input is 8 open Sequences and 10 wrapped, not 7 and 9

**Where**: `crates/ocelli-dicom/src/parse.rs:44` to `52`, "The deepest measured input is the PS3.16 2026d measurement chain ... at seven open Sequences, nine wrapped". `docs/lld/dicom-ingest.md:78` to `80`, "the deepest input is the PS3.16 2026d measurement chain, at 7 open Sequences and 9 wrapped". `crates/ocelli-dicom/tests/part10.rs:702` to `706`, `DEEPEST_SHAPE_DEPTH = 9`, with its doc comment. Also the comment at `part10.rs:720` to `722`, which says the Measured Value Sequence and Measurement Units Code Sequence "stops one level short". In the fixture they reach 5 against 7, so they are two levels short.
**What**: The lead ruling after pass 2 says the template shapes count every sequence a conformant file may carry at each level, Type 3 included. PS3.16 2026d TID 301 row 11 is INFERRED FROM INCLUDE TID 315 (Equation or Table), under the TID 300 NUM. TID 315 row 1 is a CODE, and row 2 is a HAS PROPERTIES NUM under that CODE. Counting from the document root:
1. the root Content Sequence holds TID 1500 row 6
2. Imaging Measurements holds the row 9 TID 1501 Measurement Group
3. the Measurement Group holds the row 10 TID 300 NUM
4. the NUM holds the TID 315 CODE
5. the CODE holds the TID 315 row 2 NUM
6. that NUM's Measured Value Sequence (0040,A300), Type 2 in PS3.3 C.18
7. Measurement Units Code Sequence (0040,08EA), Type 1 inside that Item
8. the Equivalent Code Sequence (0008,0121), which PS3.3 Table 8.8-1 makes Type 3 in every Code Sequence Macro

The result is 8, and the de-identification wrapper makes it 10. TID 1410 > TID 1419 row 15 reaches the same 8. Even without counting the Type 3 Equivalent Code Sequence, this route reaches 7 on Type 1 and Type 2 attributes alone, so "the deepest" is not unique under any reading. I found no route deeper than 8. The IMAGE route ends at 7 because Table 10.8 SOP Instance Reference Macro has no sequence. The Icon Image Sequence's Image Pixel Macro has none either, and Table 8.8-1's Equivalent Code Sequence items do not recurse.
**Why it is wrong**: The plan's rule, as the lead ruled it, takes the largest input. A factual sentence that is false is a defect under microscope section 3. The ruling itself names the 7 and 9 chain "as the review measured it", so GLM followed the ruling. The derivation underneath the ruling is incomplete, and the plan's ruling paragraph carries the same false figure.
**Evidence**: The template and macro rows were read now from the 2026d pages. A pydicom model of the TID 315 chain with real tags measures 8 and 10, and it parses and projects at 64 on the current tree.
**Scope and regression**: Prose and one test constant on HEAD. **The value 64 is unaffected**, because 4 x 10 = 40 and 64 is still the next power of two, and a 10-deep file parses today. The lead amends the ruling's figure, or removes the per-chain claim. Then the three sentences and `DEEPEST_SHAPE_DEPTH` become 8 and 10, or they stop naming a single deepest chain. The smallest regression is the fixture extended with the TID 315 NUM and an Equivalent Code Sequence item inside its unit code, at 10 wrapped, with `DEEPEST_SHAPE_DEPTH = 10`. The compile-time assert still holds at 40 against 64. I may have missed a deeper row in a template I did not open (TID 1603 to 1609, 4019, 4108). PS3.16 marks these templates Extensible, so no finite figure is a ceiling.

## Smells

### S1, the fixture writes its Measured Value Item out of tag order

**Where**: `crates/ocelli-dicom/tests/part10.rs:735` to `736`.
**What**: `measured_value_item` writes Numeric Value (0040,A30A) and then Measurement Units Code Sequence (0040,08EA). PS3.5 section 7.1 requires the Data Elements of a Data Set, and so of every Item, in increasing tag order. So the bytes are not a conformant data set, although the doc comment calls them the "deepest real shape". The preflight checks no tag order, and dicom-rs stores the elements in a sorted map, so the test passes today.
**Failure mechanism**: The preflight already refuses many PS3.5 structural violations. If it ever enforces section 7.1, this test goes red for a reason unrelated to depth. That is the pattern pass 1 S1 and pass 2 S1 flagged for the deflate fixture. Swap the two lines so 08EA comes first. A pydicom round trip of the dumped bytes then matches byte for byte.

## Nitpicks

- N1. The compile-time assert ties the constant to the literal 9, not to the fixture. Mutation M10 removes a level from `deepest_measured_shape_data_set`, and all 52 tests stay green. So the doc claim "The open Sequence count of `deepest_measured_shape_data_set`" is checked by nothing. A test that walks the parsed object and asserts its maximum depth equals `DEEPEST_SHAPE_DEPTH` would close this. The power-of-two half of the rule is unchecked as well, since the constant at 63 passes every test.
- N2. In `dicom-ingest.md:75`, "Re-measured on 2026-11-07" covers the key-object, secondary-capture and synthetic slice writers. I re-measured the corpus (2) and `REPOSITORY_TID_1500` (5) now. I did not rebuild the TypeScript writers, and their figures (2, 0, 0) rest on pass 2's receipt from the same unchanged source.

## Verified clean

- **Pass 1 D1** is closed. The constant follows the rule from the measured inputs, and the compile-time assert enforces the factor of four.
- **Pass 1 D2** is closed: M5 goes red.
- **Pass 1 D3** is closed: under M6 the folder viewer test aborts the process.
- **Pass 1 S1** is closed. The deflate depth fixture pads an odd stream and is independent of the constant.
- **Pass 1 N1 to N3** are closed.
- **Pass 2 D1** is closed in the form the lead ruled. The template figures were rewritten to the ruling's 7 and 9, and the residue is this pass's D1.
- **Pass 2 D2** is closed. Every fixture tag matches PS3.6 2026d:
  - (0040,A300) Measured Value Sequence SQ
  - (0040,08EA) Measurement Units Code Sequence SQ
  - (0040,A30A) Numeric Value DS
  - (0008,0100) Code Value SH
  - (0008,0102) Coding Scheme Designator SH
  - (0008,0104) Code Meaning LO
  - (0008,1199) Referenced SOP Sequence SQ
  - (0008,1150) and (0008,1155) UI
  - (0040,A730) Content Sequence SQ
  - (0400,0550) Modified Attributes Sequence SQ
  - (0400,0561) Original Attributes Sequence SQ

  The real Referenced SOP Sequence wraps the image reference, and C.18.4's Type 3 nested Referenced SOP Sequence sits inside its Item. The 2026d table confirms (0400,0550) as a Type 1 child of (0400,0561) under C.12.1.1.9. Every value has an even length. The test was renamed `the_deepest_measured_shape_parses`, and its new comment is true.
- **Pass 2 S1** is closed. `a_deflated_odd_length_stream_requires_and_accepts_its_null_pad` does not reference the constant and builds a sequence-free data set. It stays green at 63, 128 and 192. M7 and M8 show it goes red when the pad check is wrong in either direction, so it really exercises the PS3.5 A.5 NULL pad.
- **Pass 2 N1** is closed: three writers now carry three numbers.
- **Pass 2 N2** is closed: the compile-time assert exists.
- **The chain's row citations.** TID 1500 rows 6 and 9, TID 1501 row 10, TID 300 row 1b (INCLUDE TID 301), TID 301 row 13, and TID 320 rows 3 and 4 are as cited. C.18.3's Referenced SOP Sequence is Type 1, and C.18.4's nested presentation-state reference is Type 3.
- **The limit itself.** M3 and M4 show the comparison is exact in both directions.
- **Recursion-free refusal at 64.** AC3 refuses 100,000 levels on 256 KiB, and M6 shows the guard is what keeps that thread alive.
- **The 2 MiB debug claim.** The workspace sets no dev or test opt-level, so the tests run unoptimised. AC1 at 64 passes, 192 also passes, and 256 overflows, so the headroom is a factor of three to four.
- **The arithmetic.** "Next power of two at or above 4 times 9, which is 64" is right. "A quarter of the constant" is 16, which is above every measured input.
- **Unchanged code.** The check placement and the decrement are unchanged from pass 2's verified state. `use.html:531` names no value and is still true. `README.md` and `more.html` name no `ParseError` variant.

## Limitations and handoff

- Not exercised: Explicit VR Big Endian nesting, the example server's wire path, release builds, `bin/ocelli.sh gate native` and `--sprint`. No GPU or oracle job was run.
- The template sweep covered the templates listed under Acceptance challenged. TID 1603 to 1609, 4019 and 4108 were not opened, and every template involved is Extensible.
- Next owner: the lead amends the ruling's deepest-input figure (D1). Then the implementer updates the three sentences and `DEEPEST_SHAPE_DEPTH`, extends or relabels the fixture, and reorders its Measured Value Item (S1). A fresh independent pass then reviews the changed material. This is a bounded review of the range, not a completed acceptance or sprint readiness.
