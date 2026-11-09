# F-225 review, pass 5

**Reviewed**: index tree `99f6d533687dd162d92e364f6a332116977d19cf` on `work/f-225-xai` (unchanged since pass 4), plus the unstaged diff over it (`git diff` sha256 `567ed8b0…`, the same ten paths). `conformance.rs` sha256 `e7c2de94…`, `conformance.ts` sha256 `7e1a5d75…`, which is the same as pass 4. Library half only. Untracked `tools/oracle/out` and the pass-1 to pass-4 reports were ignored.
**Author / reviewer**: the remediation is by the xAI Grok Build CLI, an implementer and not the out-of-bounds AGPL Grok JPEG 2000 codec (report `grok-F-225-fix4.log` in the session scratchpad). The review is by a Claude subagent that wrote none of the design, the implementation, any remediation or any earlier pass.
**Result**: 2 defects, 1 smell, 6 nitpicks

The viewer half (AC32 to AC34, the folder viewer, README, `use.html`, `more.html`) is outside this run by instruction.

## Acceptance challenged

Every case is an explicit VR little endian Part 10 file built by `f225p5/mk5.py` (33 files, from pass 1's `mk.py` helpers) and run through `parse_part10` and `check_conformance` by the pass-1 scratch binary `f225chk`, built into `target/f225p5chk`. That directory has since been deleted. I wrote the expected answers from PS3.5 2026d 6.2, 6.4, 7.4.1 and 9.1 before running anything. PS3.5 6.4 says it directly: "Only the last UID Value in a multi-valued Data Element with a VR of UI shall be padded with a single trailing NULL", and "a single padding character shall be applied to the end of the Value Field (to the last Value)".

**Pass-4 D1 and S1 replay, and adjacent NULL cases**

| Case, as stored | Expected | Observed |
|-----------------|----------|----------|
| (0008,001A) `1.2.3` NULL `\1.2.5` | UI-FORMAT value 1 | same |
| (0008,001A) `1.2.3\1.2.5` NULL | none | same |
| (0008,0016) SC NULL `\19620203` NULL | null, `not a valid UID`, MULTIPLICITY, UI-FORMAT 1 | same |
| (0008,0016) `1.2.840.10008.1.20` + 2 NULLs | null, `not a valid UID`, UI-FORMAT 1 | same |
| (0008,0016) CT NULL `\1.2.3.4` | not evaluated, null, `not a valid UID`, MULTIPLICITY, UI-FORMAT 1 | same |
| (0008,0016) CT `\1.2.3.4` NULL | evaluated as CT, MULTIPLICITY | same. First value with no pad |
| (0008,001A) `1.2.3` NULL `\1.2.5` in a (0040,A730) item | UI-FORMAT 1 at that path | same |
| (0008,001A) `1.2.3\1.2.5` NULL in an item | none | same |
| (0008,1150) CT NULL `\1.2.3.4` in a (0008,1140) item | UI-FORMAT 1 at that path | same |
| (0008,0016) SC NULL `\1.2.3.4` in an item | MULTIPLICITY and UI-FORMAT 1 at that path, root scope still CT | same |
| (0008,0016), (0008,0018), (0008,001A) each two NULLs | UI-FORMAT 1, and for (0008,0016) null and `not a valid UID` | same |
| A Value Field of one NULL byte (VL 1) | not reachable | `parse_part10` refuses the odd length with `InvalidDataSet`, so no report exists |
| (0008,001A) `1.2.34\` NULL, an empty last value with the pad | none. PS3.5 6.4, "Each string Value in a multi-valued character string may be empty" | **UI-FORMAT value 2. See D2** |
| (0008,001A) `\1.2.34` NULL and `1.2.3\\1.2.56` NULL | none | **UI-FORMAT on the empty value. See D2** |
| (0008,0018) `\` NULL, VL 2 | TYPE1-EMPTY only. 6.4 and 7.4.1 say delimiters alone are an empty value, and AC5 says no VM or format finding | **MULTIPLICITY, UI-FORMAT 1, UI-FORMAT 2, no TYPE1-EMPTY. See D1** |
| (0008,0018) `\\`, VL 2 | TYPE1-EMPTY only | same (AC5) |

**The space pad on other VRs (PS3.5 6.2 and 6.4)**

| Case | Expected | Observed |
|------|----------|----------|
| (0018,1200) DA `20240101 \20240102` | FIXED-LENGTH value 1. The space is not on the last value | same |
| (0018,1200) DA `20240101\20240102` SPACE | none | same |
| (0018,1201) TM `1200 \1300` | none. Table 6.2-1 lets TM carry trailing spaces on any value | same |
| (0028,0030) DS `1 \1`, (0008,0008) CS `ORIGINAL \PRIMARY ` | none. DS and CS spaces are not significant | same |
| (0008,001A) `1.2.3\1.2.5` SPACE, and `1.2.3 \1.2.5` | UI-FORMAT on the value with the space. UI pads with NULL | same |
| (0008,0060) CS `\` SPACE, VL 2 | TYPE1-EMPTY only | **MULTIPLICITY only. See D1** |
| (0018,1200) DA `20240101\\20240102` and `20240101\` SPACE | none. 6.4 permits the empty value | **FIXED-LENGTH on the empty value. See D2** |
| (0028,1050) DS `40\\50` | none | **DS-FORMAT value 2. See D2** |
| (0008,0005) CS `\ISO 2022 IR 87` SPACE | none. PS3.5 7.4.1 names this exact empty first value as permitted | same, because CS has no format check |

So the space pad follows the last-value-only rule wherever a length or grammar depends on it. AS and DA are fixed length and `exceeds` gives the pad only to the last value, and the VRs whose definition allows trailing spaces on every value (TM, DT, DS, IS, CS, AE, PN, UR) trim them on every value as Table 6.2-1 says. What does not follow 6.4 is the treatment of an empty value and of a field that is delimiters plus the pad, and that applies to every VR, not only UI.

**There is now one UI pad path.** In `ocelli-dicom/src/conformance.rs`, `ui_field_values` (`:1334`) is the only place a NULL is stripped. The capture (`:858`) keeps the joined Value Field. `unevaluated_scope` (`:720`) and `iod_of` (`:780`) call `ui_field_values` and take the first value. `check_text` (`:1012`) calls `valid_ui` (which is `ui_field_values` plus `ui_grammar`) for a single value, and for many values joins the parts and indexes `ui_field_values`. The remaining `'\0'` sites are `pad_char` for the length exception (`:1250`, last value only through `exceeds`), `ui_problem` allowing NULL in the repertoire check (`:1485`), `ui_grammar` refusing any NULL (`:1881`) and the unit test. `strip_one_ui_pad` and `strip_ui_pad` are gone, and UI was removed from `format_finding` and the repertoire body. NULL passes the repertoire check and a space is not a control, so removing `strip_ui_pad` from the repertoire body changes no result. `vm.rs` has no pad handling. Outside the diff, `metadata.rs:260` still trims every NULL for the metadata model, as pass 4 noted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree above | now | 0 |
| `cargo clippy -p ocelli-dicom --all-targets -- -D warnings` | tree above | now | 0 |
| `cargo test -p ocelli-dicom` | tree above | now | 0. 263 passed, 4 ignored, 15 binaries. Library 14 and conformance 40, up from 13 and 38 by the three new tests |
| `cargo test -p ocelli-dicom --test corpus -- --ignored --nocapture` | `corpus/manifest.tsv`, 95 rows | now | 0. 4 passed, `TYPE2C-ABSENT 42`, `corpus rows 95`, no other rule |
| scratch `f225chk` over every file under `corpus/data` | 104 files. The shared `corpus/data` gained five `syntax/jpegxl_*.dcm` files at 18:38, which are not in this branch's manifest | now | The 99 files pass 4 ran give byte-identical reports. The five new files give two TYPE2C-ABSENT and three unevaluated SC reports with no finding |
| pass-1, pass-2 and pass-4 cases | scratchpad `cases/` (72), `p2r/cases/` (50), `f225p4/cases/` (28) | now | 67, 50 and 28 reports. Against pass 4's outputs, only `multi_mid_null` and `sc_1null_then_multi` changed, which are the two D1 triggers. The five deep-nesting probes still abort in `parse_part10` on the debug stack, as pass 2 recorded |
| `npm run build`, `typecheck`, `lint`, `test` | tree above | now | 0, 0, 0, 0. 61 files, 493 tests |
| `bin/ocelli.sh gate packages` | tree above | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | tree above | now | 0. 418 refusal probes, 37 guards red for their declared reason, 0 known defects |
| `python3 scripts/guard_census.py`, `--check-runbook` | tree above | now | 0, 0. `0 watched by nothing` |
| `python3 scripts/prose_check.py`, `source_provenance_check.py` | tree above | now | 0 over 855 files, 0 over 1692 files |
| privacy scan, `p2r/scan.py` under `gdcmenv` pydicom 3.0.2 | pass-1, pass-2, pass-4 and pass-5 reports | now | 0 hits over 543, 484, 218 and 270 values |
| TS reader, `f225p4/reader.mjs` | built `dist/conformance.js`, 278 reports | now | 278 accepted, 0 refused |
| mutations, `f225p5/mut5.py` | a copy of the workspace sources under `target/f225p5mut`, so the worktree was never edited. The copy was asserted restored after each run and then deleted | now | Table below |
| diff against pass 4's tree | `f225p4/conformance.rs.saved` (sha256 `8e0ebdbe…`, pass 4's recorded hash) and mtimes against the pass-4 report (18:37) | now | `conformance.rs` changed only in the pad hunks listed above. Of the other nine paths only `tests/conformance.rs` has a later mtime (18:59). `conformance.ts` has pass 4's hash. No pass-4 copy of `tests/conformance.rs` exists, so its change was checked by reading the three new and one extended fixture, not by `diff` |
| `git write-tree`, `git diff` hash and `git status --short` at the end | worktree | now | unchanged: `99f6d53`, `567ed8b0…`, same status lines |

Mutations, each run with `cargo test --no-fail-fast -p ocelli-dicom --lib --test conformance` in the copy:

| Mutation | Result |
|----------|--------|
| M1. `ui_field_values` strips one NULL from every value | red: the unit test, `fixture_ui_pad_belongs_to_the_value_field`, `fixture_sop_class_null_before_delimiter_is_not_repeated` |
| M2. Every trailing NULL stripped from the last value | red: the unit test, `fixture_ui_value_has_at_most_one_null_pad` |
| M3. Many values checked with `valid_ui(part)`, which is pass 4's behaviour | red: the two new fixtures |
| M4. A single value checked without stripping its pad | red: 36 tests |
| **M5. `iod_of` strips one NULL from the first value** | **green. See S1** |
| M6. `unevaluated_scope` strips one NULL from the first value | red: `fixture_sop_class_null_before_delimiter_is_not_repeated` |
| M7. `unevaluated_scope` uses `valid_ui` (pass-4 R9) | red: that fixture and `fixture_ui_value_has_at_most_one_null_pad` |
| M8. M5 and M6 together | red: the same fixture as M6, so only the scope half is pinned |

## Defects

### D1, a Value Field of delimiters plus the pad is not empty

**Where**: `crates/ocelli-dicom/src/conformance.rs:904` (`primitive_blank_value` counts a part as empty only when it is the empty string) and its use in `walk` (`:836` to `:844`).
**What**: SOP Instance UID (0008,0018) stored as `\` NULL (VL 2) gives MULTIPLICITY (2 values against 1), UI-FORMAT on value 1 and on value 2, and no TYPE1-EMPTY. Modality (0008,0060) stored as `\` SPACE (VL 2) gives MULTIPLICITY and no TYPE1-EMPTY, so the Type 1 requirement counts as met. A single backslash is one byte, so the pad is required, and these are the only well-formed encodings of "one delimiter alone".
**Why it is wrong**: PS3.5 6.4, "a single padding character shall be applied to the end of the Value Field", so the trailing NULL or SPACE is not a value. Then 6.4, "The presence of one or more delimiter (BACKSLASH) characters alone, without any Values, is treated like an empty Data Element Value, even if the Value Length is greater than zero", and 7.4.1, "is not sufficient to satisfy the Type 1 requirement". The design quotes both (lines 68 and 74), and AC5 names the wrong behaviour: "Neither also gives a VM or format finding", "Reporting `UI-FORMAT` for an empty value". AC5's fixture uses `\\` with no pad, so the suite does not see the padded form. This is the same principle as pass 4's D1, the pad belongs to the Value Field, applied to the blank test, which never strips it.
**Evidence**: `f225chk` on `f225p5/cases/inst_delims_pad.dcm` and `mod_delim_space.dcm`. The pad-free controls `inst_delims.dcm` and `mod_delim_only.dcm` give TYPE1-EMPTY only.
**Scope and regression**: present on the current tree and on pass 4's tree, because `primitive_blank_value` is not in the remediation diff. The remediation did not introduce it. It affects the root presence map (Type 1, Type 1C and the Pixel Data Provider URL condition read `blank`), the VM check and the value checks, and nested elements get the VM and format findings. Smallest fix: before testing for blank, remove the one pad character (NULL for UI, SPACE otherwise) from the last part, and treat a space-only CS or other insignificant-space part as empty where the VR allows it. Smallest regression: the two cases above in `fixture_type1_empty_and_delimiter_only`, each expecting TYPE1-EMPTY only.

### D2, an empty value in a multi-valued string gets a format or length finding

**Where**: `check_text` (`conformance.rs:1012`) runs `recorded_length_or_repertoire`, `ui_grammar` / `valid_ui` and `format_finding` on every part, including an empty one.
**What**: (0008,001A) `1.2.34\` NULL gives UI-FORMAT value 2. `1.2.3\\1.2.56` NULL gives UI-FORMAT value 2, and `\1.2.34` NULL gives UI-FORMAT value 1. (0018,1200) DA `20240101\\20240102` gives FIXED-LENGTH value 2 measured 0 against 8, and `20240101\` SPACE gives FIXED-LENGTH value 2 measured 1. (0028,1050) DS `40\\50` gives DS-FORMAT value 2. (0008,0016) CT `\\` NULL is evaluated as CT with UI-FORMAT on values 2 and 3.
**Why it is wrong**: PS3.5 6.4, "Each string Value in a multi-valued character string may be empty, unless otherwise specified." An empty value is not a violation of the DA, DS or UI grammar or of a fixed length, because it is not a value the grammar is asked to describe. PS3.5 7.4.1 says the same for Type 1 ("other Values may be empty, unless otherwise specified by the IOD") and gives (0008,0005) as the example. The design quotes that sentence at line 68, and AC5 lists "Reporting `UI-FORMAT` for an empty value" as the wrong behaviour. No design decision says empty values are checked. Where an attribute does specify that a value may not be empty, that is an IOD rule outside this story's list, not a grammar finding.
**Evidence**: `f225chk` on `f225p5/cases/rsc_empty_last_pad.dcm`, `rsc_empty_mid.dcm`, `rsc_empty_first.dcm`, `da_empty_mid.dcm`, `da_empty_last_pad.dcm`, `wc_empty_mid.dcm` and `sop_empty_last_pad.dcm`. The control `charset_empty_first.dcm` is clean only because CS has no format rule.
**Scope and regression**: present since pass 1 and not introduced by the remediation. It gives false findings in the report JSON, which the TS reader accepts and the viewer half would list. The last-value cases depend on the pad being removed first, so fixing D2 means skipping a part that is empty after `ui_field_values` for UI, and after removing the 6.4 pad from the last value for the other VRs. Smallest regression: an empty middle value and an empty padded last value for UI, DA and DS, each expecting no value finding, with VM still counted as PS3.6 says.

## Smells

### S1, the NULL-before-delimiter rule in `iod_of` is pinned by nothing

**Where**: `conformance.rs:780` (`iod_of`) and the fixture `fixture_sop_class_null_before_delimiter_is_not_repeated` (`tests/conformance.rs:1384`), whose value is an SC UID. SC is not in `IODS`, so `iod_of` returns `None` whether or not it strips.
**Mechanism**: mutation M5 makes `iod_of` strip one NULL from the first value (`field.split('\\').next()` then `strip_suffix('\0')`), and the whole suite stays green. With M5 in place, the scratch binary on `ct_1null_then_multi.dcm` (CT UID, NULL, `\1.2.3.4`) gives `"evaluated":true,"sopClass":"1.2.840.10008.5.1.4.1.1.2"` with the CT IOD and its seven modules, beside a UI-FORMAT on value 1 that says the same text is not a UID. That is pass-4 D1's symptom on the evaluated path, reachable by a one-line edit. M8 is red only because it also changes the scope. This is the same shape as pass 4's S1: two readers of the same field, and only one of them pinned.
**Regression**: add the CT-UID-NULL-backslash case to the fixture, expecting not evaluated, `sopClass` null, `not a valid UID`, MULTIPLICITY and UI-FORMAT 1. Then confirm M5 goes red.

## Nitpicks

- N1. New. The `unevaluated_scope` doc comment explains why `valid_ui` must not be called there, and the `valid_ui` doc says "A second call removes one more NULL". The S1-class hazard is now described in two comments and pinned by M7. Taking the first value inside one helper that both `iod_of` and the scope call would remove the hazard instead of documenting it.
- N2. Carried from pass 4 N1. The raw-prefix fallback and empty-component guards in `under_dicom_root` and `componentsBeginWithDicomRoot` are unreachable with today's constant.
- N3. Carried from pass 3 N1. The sort tiebreak reversal is still green. Not re-run.
- N4. Carried from pass 3 N2. The scope-sentence edit is red only in the typegen test. Not re-run.
- N5. Carried from pass 3 N3. The TS `isUid` bound can move to 65 with the suite green. Not re-run.
- N6. Carried from pass 3 N5 and N7. `tests/conformance.rs` still cites PS3.5 6.1.2.3 for UR, and the plan's JSON example (`F-225-design.md:283`) still has no `sopClassNote`.

Closed from pass 4: N2 (a historical count in pass 3's report, noted for the reader only).

## Verified clean

- Pass-4 D1 is closed. A NULL that ends a value other than the last is part of that value at the capture, in the scope, in `iod_of` and in the format check, at the root and inside sequences, for (0008,0016), (0008,001A) and (0008,1150). The NULL on the last value is still the pad. M1, M3 and M6 are red.
- Pass-4 S1 is closed. The even-length root UID with two NULLs is in `fixture_ui_value_has_at_most_one_null_pad`, and the second strip (M7, which was pass 4's R9) is red on it.
- Exactly one function removes the UI pad, `ui_field_values`, and every UI reader goes through it. M2 shows it removes no more than one NULL.
- The space pad on the other VRs follows the last-value-only rule wherever a length or grammar depends on it.
- The remediation changed nothing else. Every pass-1, pass-2, pass-4 and corpus report is byte-identical to pass 4's apart from the two D1 triggers. `conformance.ts` is unchanged.
- Privacy: 0 leaks across 1,515 scanned values, including the new cases. No report repeats `19620203`, a private UID or a UID with a NULL.
- No `unsafe`, no `wasm-bindgen`, no new dependency and no new `as` cast in the remediation hunks. The new fixtures cite PS3.5 6.2 and 9.1 and assert their byte lengths.

## Limitations and handoff

- Not exercised: the viewer half, group 0002, `metadata.rs`'s UI trim (outside the diff) and the deep-nesting debug-stack abort (outside the story).
- D1 and D2 are not regressions. Both were present in every earlier pass and were found here by the adjacent cases the brief asked for. The fix for D2 touches every VR's value loop, so a fresh pass will need to replay the length, repertoire and format fixtures as well as these cases.
- Pass 4's copy of `tests/conformance.rs` is not on disk, so for that file "changed nothing else" rests on reading the new fixtures and the test counts.
- Scratch files are in the session scratchpad under `f225p5/` (`mk5.py`, `cases/`, `mut5.py`, `gates.sh`, `logs/`, `*.out`). The scratch directories `target/f225p5chk` and `target/f225p5mut` were deleted.
- Next owner: the implementer addresses D1, D2 and S1, then a fresh independent pass 6 reviews that change. This is a bounded library-half review and not completed acceptance for F-225.
