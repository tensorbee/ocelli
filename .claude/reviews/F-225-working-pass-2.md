# F-225 review, pass 2

**Reviewed**: staged tree `99f6d533687dd162d92e364f6a332116977d19cf` on `work/f-225-xai`. Library half only, the same eleven staged paths as pass 1. `conformance.rs` sha256 `56798c8f…`, `conformance.ts` sha256 `5a51307b…`. Untracked `tools/oracle/out` and `.claude/reviews/F-225-working-pass-1.md` ignored. No unstaged paths.
**Author / reviewer**: remediation by the grok CLI (report `grok-F-225-fix1.log` in the session scratchpad). Review by a Claude subagent that wrote neither the design, the implementation, the remediation nor pass 1.
**Result**: 0 defects, 1 smell, 8 nitpicks

The viewer half (AC32, AC33, AC34, the folder viewer, README, `use.html`, `more.html`) is out of this run by instruction and is not a finding.

## Pass-1 findings replayed

Every case is an explicit VR little endian Part 10 file from the pass-1 byte builder (`mk.py`), extended in the scratchpad as `p2r/mk2.py`, run through `parse_part10` and `check_conformance` by the pass-1 scratch binary `f225chk` (own target directory `target/f225chk`, depends on the worktree crate by path). Each expected answer was worked out from PS3.5 2026d before the run.

| Finding | Pass-1 case, now | Adjacent cases, now | Verdict |
|---------|------------------|---------------------|---------|
| D1 | `pixel_many` and `eot_multi` give none | OW Pixel Data of 32 bytes, OB Pixel Data of 32 bytes at 8 bits, 4-frame native Pixel Data, encapsulated Pixel Data with 2 fragments and an empty offset table under 1.2.840.10008.1.2.4.50, palette LUT data of 4 words on all three colours with real descriptors, an Extended Offset Table and its Lengths of three entries: all give none | Fixed |
| D2 | `da_multi_pad`, `ds_last16_pad`, `ds_iop16` give none | Last value one byte over and ending in the pad, so legal, for TM (Time of Last Calibration `120000\120000.123456  `), DT (`2024\20240229235960.123456+1400 `), IS (Pixel Aspect Ratio `12\-00000000001 `), UI (Related General SOP Class UID, a 64-byte last value plus NULL), CS (Image Type, 16 bytes plus space), DS and DA: all give none. Discriminating negatives: CS 18 bytes ending in two spaces gives VALUE-LENGTH 18 against 16. TM 15 bytes not ending in a space gives VALUE-LENGTH 15 against 14. UI 64 bytes plus a space instead of NULL gives VALUE-LENGTH 65 against 64. DS `0.4882812500000X ` gives DS-FORMAT, so the strip does not hide a bad body. DA `20240101 \2024010 ` gives FIXED-LENGTH on value 1 and DA-FORMAT on value 2, so only the last value gets the exception | Fixed |
| D3 | `sop_name` and `sop_bad_uid` give `sopClass: null`, `sopClassNote: "not a valid UID"` | `1.2.3.SMITH`, `1.2.3 ` (space pad), ` 1.2.3`, a 65-byte grammatical UID, and SOP Class UID written as LO `SMITH^JOHNX `: each gives null and the note, never the text. `1.2.840.10008.5.1.4.1.1.7` is echoed with a null note. A two-valued `1.2.3\JONES` echoes `1.2.3` and gives MULTIPLICITY and UI-FORMAT on value 2, and `JONES` appears nowhere | Fixed. See N4 for what 9.1 grammar still admits |
| D4 | `esc_latin1` and `esc_default_cs` give REPERTOIRE control at index 2 | ESC `-A` under ISO_IR 100 and under ISO_IR 192 give control. Absent (0008,0005) gives TYPE1C-ABSENT plus control. `\ISO 2022 IR 100` (ISO 2022 on the second value only) with ESC `-A` in LO, ST and UC gives none. `ISO 2022 IR 100` as the only value gives none. `ISO_IR 100\ISO 2022 IR 87` gives none, which is defensible because a code extension is declared. Unregistered sequences under `\ISO 2022 IR 100`, ESC `(C` and ESC `%G`, give control. A lone trailing ESC and a truncated ESC `$(` give control. ESC in CS and TM under ISO 2022 gives control. A JIS PN `Yamada=ESC$B;3EDESC(B` under `\ISO 2022 IR 87` gives none | Fixed. See S1 for what the suite pins |
| S1 | Mutation `measured == limit + 1` to `measured <= limit + 2` is red on `fixture_pad_boundary_is_one_byte` (missing VALUE-LENGTH) | | Fixed |
| S2 | Mutation `state.utf8 = code == "ISO_IR 192"` to `!=` is red on `fixture_iso_ir_192_measures_bytes` (`conformance.rs:856`) | | Fixed |

I also transcribed PS3.3 2026d Tables C.12-3 and C.12-4 from the fetched section page. The 18 tails in `ISO2022_TAILS` (`conformance.rs:38`) match the 18 escape sequences there, including both IR 13 sequences, IR 203 as `-b`, and the four multi-byte sequences.

## TYPE2C-ABSENT over the corpus

The ignored corpus test reports `TYPE2C-ABSENT 42` over 95 rows and nothing else. My scratch run over the same 95 files gives the same 42, and 53 rows with no finding at all.

| Element, module | Rows | SOP classes |
|-----------------|------|-------------|
| Patient Position (0018,5100), General Series C.7.3.1 | 41 | 40 CT Image Storage (5 `synthetic/ct_*`, 20 `synthetic/ct_series_*`, 15 `syntax/*`) and 1 MR Image Storage (`synthetic/mr_nonsquare_spacing.dcm`) |
| Patient Orientation (0020,0020), General Image C.7.6.1 | 1 | CR Image Storage (`synthetic/cr_monochrome1.dcm`) |

The conditions, quoted from the fetched PS3.3 2026d pages:

- Patient Position, Table C.7-5a: "Required for images where Patient Orientation Code Sequence (0054,0410) is not present and whose SOP Class UID (0008,0016) is one of the following: "1.2.840.10008.5.1.4.1.1.2" (CT Image Storage), "1.2.840.10008.5.1.4.1.1.4" (MR Image Storage), …"
- Patient Orientation, Table C.7-9: "Required if image does not require Image Orientation (Patient) (0020,0037) and Image Position (Patient) (0020,0032) or if image does not require Image Orientation (Slide) (0048,0102)." The CR IOD, Table A.2-1, has no Image Plane module, so it requires neither attribute.

I read each file's own attributes with pydicom rather than trusting the checker. Sample, eleven rows across modalities and transfer syntaxes: `synthetic/ct_unsigned_16.dcm`, `synthetic/ct_signed_12in16_left.dcm`, `synthetic/ct_series_uniform/slice_000.dcm`, `synthetic/ct_series_nonuniform/slice_009.dcm`, `synthetic/mr_nonsquare_spacing.dcm`, `syntax/implicit_vr_le.dcm`, `syntax/explicit_vr_be.dcm`, `syntax/deflated_explicit_vr_le.dcm`, `syntax/j2k_lossless.dcm`, `syntax/rle_lossless.dcm` and `synthetic/cr_monochrome1.dcm`. Every CT and MR sample has the CT or MR Image Storage SOP class, no (0018,5100) and no (0054,0410), so the condition is met and the finding is true. The CR sample has the CR SOP class and no (0020,0020), so that finding is true. I then extended the same check to all 95 rows. All 41 Patient Position rows meet the condition. The 27 real CT and 15 real MR rows carry (0018,5100) and get no finding. The Enhanced CT, DX, US and Secondary Capture rows are outside the four SOP classes and are `evaluated: false`, so they get no presence finding. **No TYPE2C-ABSENT finding is false.** These are synthetic fixtures that leave out a Type 2C attribute. They are not a checker defect.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 99f6d53 | now | 0 |
| `cargo clippy -p ocelli-dicom --all-targets -- -D warnings` | tree 99f6d53 | now | 0 |
| `cargo test -p ocelli-dicom` | tree 99f6d53 | now | 0. 255 passed over 15 binaries, conformance 35, typegen 1, corpus 4 ignored |
| `cargo test -p ocelli-dicom --test corpus -- --ignored --nocapture` | `corpus/manifest.tsv`, 95 rows | now | 0. 4 passed, `TYPE2C-ABSENT 42`, `corpus rows 95` |
| `npm run build`, `typecheck`, `lint`, `test` | tree 99f6d53 | now | 0, 0, 0, 0. 61 files, 491 tests |
| `bin/ocelli.sh gate packages` | tree 99f6d53 | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | tree 99f6d53 | now | 0. 418 refusal probes, 37 guards red for their declared reason, 0 known defects |
| `python3 scripts/guard_census.py`, `--check-runbook` | tree 99f6d53 | now | 0, 0. 0 watched by nothing |
| `python3 scripts/prose_check.py`, `source_provenance_check.py` | tree 99f6d53 | now | 0 over 855 files, 0 over 1692 files |
| scratch `f225chk` over 95 corpus files | `corpus/data` | now | 42 TYPE2C-ABSENT, breakdown above |
| pass-1 cases rerun, one process per file | scratchpad `cases/`, 72 files | now | Every change from pass 1 is a D1 to D4 fix. Everything else identical. The five debug-stack probes (`d1000` to `d2500`, `deep3000`) still abort inside `parse_part10`, as pass 1 recorded |
| pass-2 adjacent cases | scratchpad `p2r/cases/`, 46 files | now | Table above |
| privacy scan, planted values | 115 reports (pass-1 and pass-2 cases), 1009 written values of 3 or more characters, planted name, MRN, impossible date, ESC in LO, BEL in SH, non-UID SOP classes, nested sequence secrets | now | 0 leaks. Values that are substrings of the four registered SOP class UIDs are excluded, the same carve-out as AC25 |
| 20 pass-1 Rust mutations plus 11 new, `p2r/mut2.py` to `mut4.py`, file restored and sha256 checked (`56798c8f…`) | `conformance.rs` | now | 25 red on the named test. 6 green: sort tiebreak (N1), scope sentence (N2), and four ESC mutations (S1). The pass-1 UTF-8 anchor had moved, and the rewritten mutation is red |
| D1 and D2 mutations against the corpus test | `conformance.rs` | now | D1 red, listing manifest rows. D2 green, see N6 |
| 5 TS mutations, file restored and sha256 checked (`5a51307b…`) | `conformance.ts` | now | 3 red. `isUid` at 65 is green (N5). Dropping the `"sopClassNote" in value` check is an equivalent mutant, because the next guard refuses `undefined` |
| TS reader against real Rust output | built `packages/core/dist/conformance.js`, 210 reports (95 corpus, 115 cases) | now | 210 accepted, 0 refused |
| `git write-tree` and `git status --short` at the end | worktree | now | unchanged, `99f6d53`, same status lines |

## Smells

### S1, the ESC skip is pinned by one escape sequence on one VR

**Where**: `conformance.rs:38` to `41` (`ISO2022_TAILS`) and `:1187` (`ch == '\u{001B}' && iso2022 && extended_string(vr)`).
**Mechanism**: `fixture_escape_requires_an_iso_2022_sequence` has one accepted sequence, `ESC -A` in an LO, and one lone ESC. Four mutations leave it green. (a) Adding `"%G"` to the table, the ISO/IEC 2022 DOCS escape to UTF-8, which neither Table C.12-3 nor C.12-4 lists. Adding `"(C"` is green the same way. (b) Deleting `"$B"`, the IR 87 sequence. (c) Deleting `extended_string(vr)` from the skip. After (c), an ESC `-A` in a UR under a declared ISO 2022 set passes, because `-` and `A` are both in the RFC 3986 set, and UR is not a 6.1.2.2 extensible VR. The checker is right today. I confirmed the 18 entries against the tables and saw ESC `(C` and ESC `%G` reported as control. But the rule the remediation says it enforces, "a sequence from PS3.3 C.12.1.1.2 Tables C.12-3 and C.12-4", is enforced by a table whose other 17 rows and whose closure nothing tests. That is the same shape as pass 1's S1, a rule that can widen with nothing going red.
**Regression**: one fixture with an unregistered sequence such as ESC `%G` under `\ISO 2022 IR 100`, expecting REPERTOIRE control. One test that compares `ISO2022_TAILS` with the 18 column/row pairs quoted from the two tables, written from the standard and not copied from the constant. One UR or CS case with ESC `-A` under ISO 2022, expecting a finding.

## Nitpicks

- N1. Carried from pass 1 N2. No fixture has two findings on the same path, tag and value, so reversing the rule tiebreak (`conformance.rs:564`) stays green.
- N2. Carried from pass 1 N1. `scope_sentence_is_exact` compares with the constant, so editing the constant leaves it green. The typegen test and the TS literal still catch it.
- N3. Pass-1 N3 to N9 were left as written by the remediation, per its report. I did not re-examine them, apart from N3 (`isPositiveU32`), which is unchanged.
- N4. Unconfirmed concern, privacy. The D3 fix echoes any value that passes the 9.1 grammar, and the grammar admits a single component of digits. A SOP Class UID of `19620203`, which is the shape of a birth date, is echoed as `"sopClass":"19620203"` with a null note (case `priv_sop_digits`). Plan decision 5 justifies the one exception as "a registry identifier". A grammatical UID that is not registered is not one, but a private SOP class is legitimate and useful to show. The lead should decide whether 9.1 grammar is the intended bar and say so in the plan.
- N5. The TS reader's 64-byte bound in `isUid` (`conformance.ts:186`) can be moved to 65 with the suite green. The Rust side never emits a longer echo, so nothing is lost today.
- N6. The DA and DS half of `every_manifest_row_avoids_rules_a_well_formed_file_cannot_trip` has no corpus row that exercises it. Reverting the D2 strip leaves the corpus test green. The unit test `fixture_padded_da_and_ds_are_clean` does go red, so the rule is covered. The remediation report's line "no DA-FORMAT, DS-FORMAT … on a legal one-byte pad" is true but vacuous for this corpus. It should not be copied as evidence.
- N7. `sopClassNote` is a new wire key, added on both sides and accepted by the reader on every real report. The plan's JSON example and decision 3 do not list it. Record it in the plan or the LLD before the viewer half reads `modules`.
- N8. The Patient Orientation condition in Table C.7-9 read literally, "or if image does not require Image Orientation (Slide)", would require Patient Orientation on every CT. The plan's AC21 reads it as CR only, which is the evident intent and gives fewer findings. Not a defect. Noted so that a later reader does not "fix" it.

## Verified clean

- D1. `value_count` returns 1 for OB, OD, OF, OL, OV, OW and UN. SQ and PixelSequence already measured 1. Every pass-1 corpus MULTIPLICITY finding is gone, and the corpus test goes red, listing manifest rows, if OB, OV and OW are dropped from the list.
- D2. `without_pad_exception` strips exactly one pad character, only from the last of several values, and only when that value is exactly limit plus one. Repertoire and format then see the body. The length decision still belongs to `length_finding`.
- D3. `unevaluated_scope` trims NULL, refuses empty, over 64 bytes and anything outside 9.1, and never stores the refused text. The TS reader refuses a note paired with a UID, a non-UID `sopClass`, and any other note text, and each of those three refusals is seen red under mutation.
- D4. `extended_problem` makes ESC a control, and `repertoire_finding` skips it only for an extensible VR, with some (0008,0005) value starting `ISO 2022`, and when a listed tail follows. The character index still advances over the skipped tail.
- The AC25 scan now strips `fixture.sop` only when the test's own `ps35_uid` accepts it. `ps35_uid` is written in the test and does not call the crate. Making the checker echo any SOP class turns the scan red.
- `check_conformance` still takes `&ParsedDicom`, panics on nothing I gave it, and gives byte-identical JSON on a second run for all 210 inputs. There is still no `unsafe`, no `wasm-bindgen` and no new dependency.

## Limitations and handoff

- Not exercised: ISO 2022 multi-byte decoding beyond one JIS PN, GB18030 byte counting, and the viewer half. dicom-rs 0.10 maps `ISO 2022 IR 100` to a plain Latin-1 decoder, which is why ESC bytes reach the checker at all. I did not probe every ISO 2022 term.
- The debug-build stack overflow at about 1,000 nesting levels is in `parse_part10`, unchanged, and outside this story.
- Next owner: the implementer addresses S1. A fresh independent pass 3 then reviews that change. This is a bounded library-half review, not completed acceptance for F-225.
