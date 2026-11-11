# F-225 review, pass 6

**Reviewed**: index tree `99f6d533687dd162d92e364f6a332116977d19cf` on `work/f-225-xai` (the same index as passes 4 and 5), plus the unstaged diff over it (`git diff` sha256 `c3238187…`, the same ten paths). `conformance.rs` sha256 `6ee2d61d…`, `tests/conformance.rs` sha256 `e1e7fe41…`, `conformance.ts` sha256 `7e1a5d75…`, which is pass 5's hash. Library half only. Untracked `tools/oracle/out` and the pass-1 to pass-5 reports were ignored.
**Author / reviewer**: the remediation is by a Claude implementer agent working from the brief `fix-F-225-5.md` in the session scratchpad. The review is by a separate Claude subagent that wrote none of the design, the implementation, any remediation or any earlier pass.
**Result**: 0 defects, 1 smell, 7 nitpicks

The viewer half (AC32 to AC34, the folder viewer, README, `use.html`, `more.html`) is outside this run by instruction.

## Acceptance challenged

Every case is an explicit VR little endian Part 10 file built by `f225p6/mk6.py` (33 files, from pass 1's `mk.py` helpers, each Value Field asserted even) and run through `parse_part10` and `check_conformance` by the pass-1 scratch binary `f225chk`, built into `target/f225p6chk`. That directory has since been deleted. I wrote the expected answers from PS3.5 2026d 6.2, 6.4 and 7.4.1 before running anything. The two sentences that decide this round are 6.4, "a single padding character shall be applied to the end of the Value Field (to the last Value)", and 6.4, "Each string Value in a multi-valued character string may be empty, unless otherwise specified. The presence of one or more delimiter (BACKSLASH) characters alone, without any Values, is treated like an empty Data Element Value".

**Pass-5 D1, D2 and S1 replayed through the real checker**

| Case, as stored | Expected | Observed |
|-----------------|----------|----------|
| (0008,0018) `\` NULL, VL 2 (pass-5 D1) | TYPE1-EMPTY only | same |
| (0008,0018) `\\\` NULL, VL 4 | TYPE1-EMPTY only | same |
| (0008,0060) `\` SPACE, VL 2 (pass-5 D1) | TYPE1-EMPTY only | same |
| (0008,0060) `\\`, VL 2 (AC5) | TYPE1-EMPTY only | same |
| (0008,001A) `1.2.34\` NULL, empty last value plus pad (pass-5 D2) | none | same |
| (0008,001A) `\1.2.34` NULL, empty first value | none | same |
| (0008,001A) `1.2.3\\1.2.56` NULL, empty middle value | none | same |
| (0008,001A) `1.2.3\\1.02` NULL | UI-FORMAT value 3. The skip must not move the numbering | same |
| (0018,1200) DA `20240101\` SPACE, empty last value plus pad | none | same |
| (0018,1200) DA `20240101\\20240102` | none | same |
| (0028,1050) DS `40\\50` | none | same |
| (0008,0016) RAW `1.2.840.10008.5.1.4.1.1.66\` NULL, a SOP Class with an empty last value plus NULL | not evaluated, `sopClass` repeated (under the root), MULTIPLICITY 2 against 1, no UI-FORMAT | same |
| (0008,0016) CT NULL `\1.2.3.4` (pass-5 S1) | not evaluated, null, `not a valid UID`, MULTIPLICITY, UI-FORMAT 1 | same, and now pinned (mutation P5-M5 below is red) |

**An empty first and an empty middle value under a space-padded VR with a repertoire rule**

| Case | Expected | Observed |
|------|----------|----------|
| (0008,0008) CS `\PRIMARY` | none | same |
| (0008,0008) CS `ORIGINAL\\PRIMARY` SPACE | none | same |
| (0008,0008) CS `\primary` | REPERTOIRE value 2, index 0, outside-repertoire | same |
| (0008,0008) CS `ORIGINAL\\prim` | REPERTOIRE value 3, index 0 | same |
| (0018,1020) LO `\AB` BEL `C` SPACE | REPERTOIRE value 2, index 2, control | same |
| (0018,1020) LO `A\\B` BEL SPACE | REPERTOIRE value 3, index 1, control | same |
| (0008,0008) CS `\\` then 17 `A` and SPACE | VALUE-LENGTH value 3, 18 bytes against 16 | same |

**Can the empty-value skip hide a real finding?** The skip in `check_text` (`conformance.rs:1032`) fires only on a value that is zero length after `field_values` removes at most one pad, from the last value only. A value of spaces is not empty under that rule, so it goes through every check:

| Case | Expected | Observed |
|------|----------|----------|
| (0020,4000) LT of four spaces | a value, not blank. LT may carry trailing spaces and has no format rule | not blank, no finding |
| (0008,0081) ST of two spaces | the same | not blank, no finding |
| (0020,4000) LT `A\B` BEL | one value. LT is not multi-valued and 5CH is allowed | REPERTOIRE value 1 index 3, so the parser does not split LT |
| (0020,4000) LT `\\` | one value of two backslashes, not delimiters | not blank, no finding |
| (0028,7FE0) UR `\` SPACE, no Pixel Data | a value. UR is not multi-valued | REPERTOIRE value 1, and the URL counts as present |
| (0008,0054) AE `AE1\` + two SPACEs | AE-SPACES value 2. "A Value consisting solely of spaces shall not be used" | same |
| (0008,0054) AE `AE12\` SPACE | none. The last value is the pad alone | same |
| (0018,1200) DA `20240101\ \20240102` SPACE | FIXED-LENGTH value 2, 1 byte against 8 | same |
| (0028,1050) DS `40\ \50` SPACE | a finding on value 2 | DS-FORMAT value 2 |
| (0008,0060) CS of two SPACEs | see N7 | not blank, no finding, as in pass 1 |

So the skip cannot hide a value that has any byte beyond the single 6.4 pad. In a VR where spaces are significant (LT, ST, UT leading spaces) a space-only value is a value, which is what Table 6.2-1 says.

**The ruling case: (0008,0016) holding the CT UID, two backslashes and a NULL** (28 bytes, three values, the second and third empty). Observed: evaluated as CT Image with its seven modules, MULTIPLICITY 3 against 1, and no UI-FORMAT. The neighbours: CT then one backslash with no pad (26 bytes) is evaluated as CT with MULTIPLICITY 2. `\` then the CT UID (26 bytes) is not evaluated, `sopClass` null, `sopClassNote` null, with MULTIPLICITY 2. The same field in a (0008,1140) item gives no finding, because (0008,1150) is not in the VM table.

Ruling. Treating it as its first value is right, and the report is not quietly wrong. PS3.5 6.4 makes the two trailing values empty values, so the field's first value is exactly the CT Image Storage UID, and PS3.5 has no rule naming which value of an over-multiplicity element governs. The VM violation is reported as MULTIPLICITY, so the reader is told the element is malformed. Decision 3 of the plan scopes presence by "the SOP Class UID", and `iod_of` already takes value 1 for `CT\1.2.3.4`, which passes 4 and 5 replayed and accepted. Refusing to evaluate would be a defensible alternative, but nothing in PS3.5 or the plan asks for it, and it would drop presence checks on a file whose SOP Class is not in doubt. **It is not covered by a test.** See S1, where three mutations that change this choice stay green.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree above | now | 0 |
| `cargo clippy -p ocelli-dicom --all-targets -- -D warnings` | tree above | now | 0 |
| `cargo test -p ocelli-dicom` | tree above | now | 0. 264 passed, 4 ignored, 15 binaries. `tests/conformance.rs` 41, up from 40 by `fixture_empty_value_in_a_multi_valued_string_is_not_checked` |
| `cargo test -p ocelli-dicom --test corpus -- --ignored --nocapture` | `corpus/manifest.tsv`, 95 rows | now | 0. 4 passed, `TYPE2C-ABSENT 42`, `corpus rows 95`, no other rule |
| scratch `f225chk` over the 104 files pass 5 ran under `corpus/data` | the shared corpus | now | byte-identical to pass 5's `corpus.out` |
| pass-1, pass-2, pass-4 and pass-5 cases | scratchpad `cases/` (72), `p2r/cases/` (50), `f225p4/cases/` (28), `f225p5/cases/` (33) | now | Against pass 4's outputs, pass-1 and pass-2 reports are unchanged and pass-4 cases differ only in pass 5's two D1 triggers. Against pass 5's outputs, only the D1 and D2 triggers changed, each to the expected answer in the first table. The five deep-nesting probes still abort in `parse_part10` on the debug stack, as pass 2 recorded |
| pass-6 cases, `f225p6/mk6.py` | `f225p6/cases/` (33) | now | the tables above |
| `npm run build`, `typecheck`, `lint`, `test` | tree above | now | 0, 0, 0, 0. 61 files, 493 tests |
| `bin/ocelli.sh gate packages` | tree above | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | tree above | now | 0. 418 refusal probes, 37 guards red for their declared reason, 0 known defects |
| `python3 scripts/guard_census.py`, `--check-runbook` | tree above | now | 0, 0. `0 watched by nothing` |
| `python3 scripts/prose_check.py`, `source_provenance_check.py` | tree above | now | 0 over 855 files, 0 over 1692 files |
| privacy scan, `p2r/scan.py` under `gdcmenv` pydicom 3.0.2 | pass-1, pass-2, pass-4, pass-5 and pass-6 reports | now | 0 hits over 543, 484, 218, 270 and 221 values |
| TS reader, `f225p4/reader.mjs` | built `dist/conformance.js`, 207 reports from the pass-6 runs | now | 207 accepted, 0 refused |
| mutations, `f225p6/mut6.py` | a copy of the workspace under `target/f225p6mut/ws`, so the worktree was never edited. The copy's `conformance.rs` sha256 was asserted restored after every run, and the copy was then deleted | now | Table below |
| `git write-tree`, `git diff` hash, file hashes and `git status --short` at the end | worktree | now | unchanged: `99f6d53`, `c3238187…`, the three hashes above, status identical to the start apart from this report |

Mutations. The suite is `cargo test --no-fail-fast -p ocelli-dicom --lib --test conformance --test conformance_typegen`. The corpus entries run the ignored corpus test.

The 31 pass-1 and pass-2 mutations that still have an anchor (`p2r/mut4.py` has 34 entries, and "charset UN keyed" and both "D3 echo any" entries name code that pass 2 and pass 4 rewrote):

| Result | Mutations |
|--------|-----------|
| red on the suite (28) | IS_MAX +1, TM hour 24, DT +1500, DT -1300, UI leading zero, PN groups 4, PN carets 5, AE spaces off, CS lowercase, LO tab allowed, PET position, CR image plane, NOT_CHECKED drop, pad +2, SPP >0, item 0-based, rule id typegen, scope sentence (typegen only, N4), D1 binary VM, D2 no strip, D4 ESC allowed, D4 iso2022 any value off, D4 tails add `(C`, S2 utf8 inverted, D4 drop `-A`, D4 drop `$B`, D4 add `%G`, D4 skip check off VR |
| red on the corpus test (1) | D1 binary VM corpus |
| green (2) | sort reversed rule (N4), D2 no strip corpus (pass-2 N6, red on the suite) |

Pass-5 and pass-6 mutations:

| Mutation | Result |
|----------|--------|
| P5-M1. `strip_field_pad` strips every value | red: `pad_byte_is_only_on_the_last_value`, `fixture_ui_pad_belongs_to_the_value_field`, `fixture_sop_class_null_before_delimiter_is_not_repeated` |
| P5-M2. Every trailing pad stripped from the last value | red: the unit test, `fixture_code_string_and_ae`, `fixture_ui_value_has_at_most_one_null_pad` |
| P5-M3. A multi-valued UI checked with `valid_ui(part)` | red: two fixtures |
| P5-M4. A single UI value checked without stripping | red: 37 tests |
| **P5-M5. `iod_of` strips one NULL from the first value (pass-5 S1)** | **red: `fixture_sop_class_null_before_delimiter_is_not_repeated`. Pass-5 S1 is closed** |
| P5-M6. The scope strips one NULL from the first value | red: the same fixture |
| P5-M7. The scope uses `valid_ui` | red: two fixtures |
| P6-A. `primitive_blank_value` ignores the pad (pass-5 D1) | red: `fixture_type1_empty_and_delimiter_only` |
| P6-B. No empty-value skip (pass-5 D2) | red: `fixture_empty_value_in_a_multi_valued_string_is_not_checked` |
| P6-C. Empty test on the raw part, pad kept | red: the same |
| P6-D. Length and repertoire run before the skip | red: the same |
| P6-E. Multiplicity counts only non-empty values | red: `fixture_multiplicity` |
| P6-I. The blank test strips every trailing pad | red: `fixture_code_string_and_ae` |
| P6-J. The pad is SPACE for every VR, UI included | red: four fixtures |
| P6-K. The scope repeats any well-formed UID | red: `fixture_sop_class_is_repeated_only_under_the_dicom_root`, `no_finding_carries_a_value` |
| **P6-F. `iod_of` returns None for a field of more than one value** | **green. See S1** |
| **P6-G. `iod_of` takes the first non-empty value** | **green. See S1** |
| **P6-H. `iod_of` returns None when any value is empty** | **green. See S1** |

## Defects

None.

## Smells

### S1, which value of a multi-valued SOP Class UID decides the IOD is pinned by nothing

**Where**: `iod_of` (`crates/ocelli-dicom/src/conformance.rs:780`), its caller `module_scope`, and `unevaluated_scope` (`:720`) for the `\`-first case. No fixture in `tests/conformance.rs` puts the CT, CR, MR or PET UID in a (0008,0016) of more than one value. The only CT multi-valued fixture, `sop-ct-null-then-uid`, has a NULL on value 1 and so is not evaluated whatever the rule is.
**Mechanism**: the ruling above says a VM-1 SOP Class with extra values is scoped by value 1. Three one-line edits to `iod_of` change that and the whole suite stays green. P6-F stops evaluating CT for the case the implementer left uncovered (CT, `\\`, NULL) and for `CT\1.2.3.4`, which silently drops every Type 1, Type 2 and conditional presence check on those files. P6-G evaluates `\` + CT as CT, and P6-H stops evaluating a CT UID followed by empty values. This is the same shape as pass-5 S1 and pass-4 S1: two readers of one field (`iod_of` and `unevaluated_scope`), and the choice that couples them is pinned on only one path. The `\` + CT state also shows it. The report says not evaluated with `sopClass` null and `sopClassNote` null, beside a MULTIPLICITY finding. `SOP_CLASS_NOTES` (`conformance.rs:41`) and the TS type (`conformance.ts:73`) document a null note as "the UID was absent or repeated". Value 1 is empty, so "absent" is a defensible reading, but nothing pins it, and before this round that state also carried a UI-FORMAT on value 1, which the D2 fix rightly removed.
**Regression**: one fixture that asserts the ruling on both sides. CT, `\\`, NULL (28 bytes) expects evaluated as CT Image with MULTIPLICITY 3 against 1 and no UI-FORMAT. `\` + CT (26 bytes) expects not evaluated, `sopClass` null, `sopClassNote` null and MULTIPLICITY 2. Then confirm P6-F, P6-G and P6-H each go red. If the lead prefers "not evaluated" for any over-multiplicity SOP Class, that is a plan amendment, and the same fixture pins it.

## Nitpicks

- N1. New. `fixture_multiplicity`'s `vm-empty-counted` comment says "the empty value has no format finding", but CS has no format rule, so that clause cannot fail. The fixture pins multiplicity (P6-E is red on it), and the empty-value skip is pinned by the other fixture.
- N2. Carried from pass 5 N1. The `valid_ui` and `unevaluated_scope` doc comments still describe the double-strip hazard instead of removing it. Pinned by P5-M7.
- N3. Carried from pass 4 N1. The raw-prefix fallback and empty-component guards in `under_dicom_root` and `componentsBeginWithDicomRoot` are unreachable with today's constant.
- N4. Carried from pass 3 N1 and N2. The sort tiebreak reversal is still green, and the scope-sentence edit is red only in the typegen test. Both re-run now.
- N5. Carried from pass 3 N3. The TS `isUid` bound can move to 65 with the suite green. Not re-run, `conformance.ts` is unchanged.
- N6. Carried from pass 3 N5 and N7. `tests/conformance.rs:1611` still cites PS3.5 6.1.2.3 for UR, and the plan's JSON example (`F-225-design.md:283`) still has no `sopClassNote`.
- N7. A plausible concern, not confirmed. A Type 1 CS of two SPACEs (Modality) satisfies Type 1 and gets no finding, as it has since pass 1. Table 6.2-1 says CS spaces "are not significant", but PS3.5 7.4.1 only names zero length and delimiters alone as not satisfying Type 1, and only AE says "A Value consisting solely of spaces shall not be used". The standard does not decide this, so it does not block. If the lead wants space-only values of insignificant-space VRs treated as empty, that belongs in the plan first.

Closed from pass 5: D1, D2 and S1. Pass-5 N2 to N6 are carried above as N3 to N6.

## Verified clean

- Pass-5 D1 is closed. A Value Field that is delimiters alone plus the single pad, NULL for UI and SPACE otherwise, is blank. It gives TYPE1-EMPTY and no VM or format finding, at VL 2 and VL 4. P6-A, P6-I and P6-J are red.
- Pass-5 D2 is closed. An empty first, middle or last value gets no length, repertoire or format finding for UI, DA, DS, CS and LO. Value numbers after an empty value stay positional (REPERTOIRE on value 3 after an empty value 2). Empty values still count toward multiplicity, as PS3.5 6.4 counts delimited values. P6-B to P6-E are red.
- Pass-5 S1 is closed. The CT, NULL, `\1.2.3.4` case is in `fixture_sop_class_null_before_delimiter_is_not_repeated`, and P5-M5 is red on it.
- The empty-value skip cannot hide a value of spaces. One pad is removed from the last value only, so a space-only value in LT, ST, AE, DA or DS is checked, and LT and UR are not split on backslash.
- One pad path. `strip_field_pad` is the only function that removes a pad. `ui_field_values` and `field_values` both call it, and the blank test, the capture, the scope, `iod_of` and `check_text` all go through one of the two.
- The remediation changed nothing else. Pass-1, pass-2 and corpus reports are byte-identical to the earlier runs, and the earlier case reports changed only where the D1 and D2 triggers are. `conformance.ts` is unchanged.
- Privacy: 0 leaks across 1,736 scanned values. No new report repeats a value other than a SOP Class UID under the DICOM root.
- No `unsafe`, no `wasm-bindgen`, no `as` cast and no new dependency in `conformance.rs` or `vm.rs`. The new fixtures cite PS3.5 6.2, 6.4 and 7.4.1 and assert their byte lengths.

## Limitations and handoff

- Not exercised: the viewer half, group 0002, `metadata.rs`'s UI trim (outside the diff) and the deep-nesting debug-stack abort (outside the story).
- There is no saved copy of pass 5's `conformance.rs` or `tests/conformance.rs`, so "changed nothing else" rests on the replayed reports, the mutation table and reading the pad hunks, not on a file diff.
- Scratch files are in the session scratchpad under `f225p6/` (`mk6.py`, `cases/`, `show.py`, `mut6.py`, `mut6.out`, `mut6b.out`, `gates.sh`, `logs/`, `*.out`). The scratch directories `target/f225p6chk` and `target/f225p6mut` were deleted.
- Next owner: the implementer adds the S1 fixture, or the lead amends the plan if the ruling should go the other way, and a fresh independent pass 7 reviews that change. This is a bounded library-half review and not completed acceptance for F-225.
