# F-225 review, pass 4

**Reviewed**: index tree `99f6d533687dd162d92e364f6a332116977d19cf` on `work/f-225-xai`, plus the unstaged diff over it (`git diff` sha256 `08605ff9…`, the same ten paths as pass 3). `conformance.rs` sha256 `8e0ebdbe…`, `conformance.ts` sha256 `7e1a5d75…`. Library half only. Untracked `tools/oracle/out` and the pass-1 to pass-3 reports were ignored.
**Author / reviewer**: the remediation is by the xAI Grok Build CLI, an implementer and not the out-of-bounds AGPL Grok JPEG 2000 codec (report `grok-F-225-fix3.log` in the session scratchpad). The review is by a Claude subagent that wrote none of the design, the implementation, any remediation or any earlier pass.
**Result**: 1 defect, 1 smell, 7 nitpicks

The viewer half (AC32 to AC34, the folder viewer, README, `use.html`, `more.html`) is outside this run by instruction, so nothing there is a finding.

## Acceptance challenged

Every case is an explicit VR little endian Part 10 file. The pass-1 byte builder `mk.py` was extended as `f225p4/mk4.py` (28 files) and run through `parse_part10` and `check_conformance` by the pass-1 scratch binary `f225chk`, built into `target/f225p4chk`, which has since been deleted. I wrote the expected answers from PS3.5 2026d 6.2, 6.4 and 9.1 and from the lead amendment before running anything.

**Pass-3 D1 replay. One NULL is the pad, more than one is not.**

| Case, (0008,0016) as stored | Expected | Observed |
|-----------------------------|----------|----------|
| SC UID (25 bytes) + 1 NULL | repeated, note null, no finding | same |
| SC + 3 NULLs, SC + 5 NULLs | null, `not a valid UID`, one UI-FORMAT | same |
| CT UID + 1 NULL | evaluated as CT | same |
| CT UID + 3 NULLs | not evaluated, null, `not a valid UID`, UI-FORMAT | same. `iod_of` no longer trims |
| `1.2.840.10008.5.1.4.1.1.128` (27) + 3 NULLs | null, `not a valid UID`, UI-FORMAT | same |
| `1.2.840.10008.1.20` (18, even) + 2 NULLs | null, `not a valid UID`, UI-FORMAT | same. With mutation R9 this case is repeated and the suite stays green, see S1 |
| SC + NULL SPACE NULL, SC + SPACE NULL NULL | null, `not a valid UID`, UI-FORMAT | same |
| SC + 3 NULLs + `\1.2.3` | null, `not a valid UID`, MULTIPLICITY and UI-FORMAT | same |
| **SC + 1 NULL + `\19620203` + NULL** | **null, `not a valid UID`, UI-FORMAT on value 1, MULTIPLICITY** | **SC repeated, note null, MULTIPLICITY only. See D1** |
| (0008,1150) in (0008,1140) item, CT + 1 NULL | no finding | same |
| (0008,1150) in an item, CT + 3 NULLs | UI-FORMAT at path `(0008,1140)` item 1 | same |
| (0008,0016) inside a (0040,A730) item, SC + 3 NULLs | UI-FORMAT at that path, root scope unchanged | same. The nested value does not set `sopClass` |
| (0020,000D) `9.8.7` + 3 NULLs | UI-FORMAT | same |
| (0008,001A) `1.2.3\0\0\0\1.2` | UI-FORMAT on value 1 | same |
| **(0008,001A) `1.2.3\0\1.2.5`** | **UI-FORMAT on value 1** | **no finding. See D1** |
| (0008,001A) `1.2.3\1.2.5` + NULL | no finding | same |
| Transfer Syntax UID (0002,0010) + 3 NULLs | parse succeeds, no finding | same. The checker does not walk group 0002 at all, and `parse_part10` reads the UID through dicom-rs's `meta.transfer_syntax()`. Neither path is in this story's diff |

I swept every UI trim site in `ocelli-dicom/src`. Inside the checker, `strip_one_ui_pad` is the only NULL strip, and it is used at the capture (`conformance.rs:851`), in `valid_ui` (`:1817`) and in `strip_ui_pad` for the repertoire body (`:1285`). `unevaluated_scope` and `iod_of` no longer strip. Outside the checker, `metadata.rs:260` still does `trim_end_matches('\0')` on UI. That file is not in the diff, and it feeds the metadata model rather than the report.

**Pass-3 S1 replay. The root test is on components.** On the Rust side `1.2.840.100081.2`, `1.2.840.10008` + NULL, `1.2.840.1000.81`, `1.2.840.5`, `11.2.840.10008.1` and `1.2.840.10008.` are all not repeated. The last of these gives `not a valid UID` and UI-FORMAT, and the rest give `not a DICOM UID`. `1.2.840.10008.1` is repeated. A `1.2.840.100081.2` nested in an item with no root (0008,0016) gives `sopClass` null and note null. On the TS side the tests refuse an unevaluated `1.2.840.100081.2` and `1.2.840.10008`, and refuse an evaluated `1.2.3.4`. Against the built `dist/conformance.js`, 240 real Rust reports were all accepted (95 corpus, 67 pass-1, 50 pass-2 and 28 pass-4).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree above | now | 0 |
| `cargo clippy -p ocelli-dicom --all-targets -- -D warnings` | tree above | now | 0 |
| `cargo test -p ocelli-dicom` | tree above | now | 0. 260 passed, 4 ignored, 15 binaries |
| `cargo test -p ocelli-dicom --test corpus -- --ignored --nocapture` | `corpus/manifest.tsv`, 95 rows | now | 0. 4 passed, `TYPE2C-ABSENT 42`, `corpus rows 95`, no other rule |
| scratch `f225chk` over every file under `corpus/data` | 99 files, 95 DICOM and 4 `LICENSE` files that do not parse | now | `{TYPE2C-ABSENT: 42}` only. Each report matches the pass-2 scratch output `p2r/corpus.out` once the new `sopClassNote` key is removed. 11 are unevaluated, all under the root with a null note, and the pass-2 output also has 11 |
| `npm run build`, `typecheck`, `lint`, `test` | tree above | now | 0, 0, 0, 0. 61 files, 493 tests |
| `bin/ocelli.sh gate packages` | tree above | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | tree above | now | 0. 418 refusal probes, 37 guards red for their declared reason, 0 known defects |
| `python3 scripts/guard_census.py`, `--check-runbook` | tree above | now | 0, 0. `0 watched by nothing` |
| `python3 scripts/prose_check.py`, `source_provenance_check.py` | tree above | now | 0 over 855 files, 0 over 1692 files |
| pass-1 and pass-2 cases, one process per file | scratchpad `cases/` (72), `p2r/cases/` (50) | now | 67 and 50 reports. The five 1,000-level and deeper nesting probes abort in `parse_part10` on the debug stack, as pass 2 recorded |
| privacy scan, `p2r/scan.py` under `gdcmenv` pydicom 3.0.2 | pass-1, pass-2 and pass-4 reports | now | 0 hits over 543, 484 and 217 values. Values that are substrings of registered UIDs, such as the bare root, are outside that scan and were checked case by case in the table above |
| Rust and TS mutations, `f225p4/mut.py`, each file restored and its sha256 asserted | `conformance.rs`, `conformance.ts` | now | Table below |
| TS reader against real Rust output, `f225p4/reader.mjs` | built `dist/conformance.js`, 240 reports | now | 240 accepted, 0 refused |
| diff against pass 3's tree | `/private/tmp/conformance.rs.good` (sha256 `dd0c2c7e…`, pass 3's recorded hash) and mtimes against the pass-3 report (18:02) | now | `conformance.rs` changed only in the D1 and S1 hunks (root doc, `unevaluated_scope`, `under_dicom_root`, `iod_of`, capture, `strip_one_ui_pad`, `valid_ui`/`ui_grammar`, the `dicom_root` unit test). Of the other files, only `tests/conformance.rs`, `conformance.ts` and `conformance.test.ts` were modified after pass 3. The test counts rose by exactly the new tests (lib 12 to 13, conformance 37 to 38, TS 492 to 493). No pass-3 copy of the three other files exists, so their content was checked by reading the diff and not by `diff` |
| `git write-tree`, `git diff` hash and `git status --short` at the end | worktree | now | unchanged: `99f6d53`, `08605ff9…`, same status lines |

Mutations, run with `cargo test --no-fail-fast -p ocelli-dicom --lib --test conformance` or `npx vitest run packages/core/src/conformance.test.ts`:

| Mutation | Result |
|----------|--------|
| R1. `DICOM_UID_ROOT` without its trailing dot | red: `dicom_root_is_a_component_boundary`, `fixture_sop_class_is_repeated_only_under_the_dicom_root`, `no_finding_carries_a_value` |
| R2. `under_dicom_root` replaced by `uid.starts_with("1.2.840.10008")` | red: the same three tests |
| R3. The further component is not required (`true`) | red: the same three tests |
| R4. The further component may be empty | red: the unit test |
| R5. Capture trims every NULL | red: `fixture_ui_value_has_at_most_one_null_pad` |
| R6. `unevaluated_scope` trims every NULL | red: the same |
| R7. `iod_of` trims every NULL | red: the same, at the CT line |
| R8. `strip_one_ui_pad` strips every NULL | red: the same |
| **R9. `unevaluated_scope` calls `valid_ui` instead of `ui_grammar`, a second one-NULL strip** | **green. See S1** |
| T1. TS root constant without its trailing dot | red: `refuses a sop class outside the DICOM root components` |
| T2. TS component test replaced by `startsWith("1.2.840.10008")` | red: the same |
| T3. TS evaluated-branch `!isDicomRootUid` removed | red: the same, at the evaluated `1.2.3.4` |
| T4. TS bare root accepted | red: the same |
| T5. TS unevaluated branch back to `isUid` | red: that test and `accepts only the closed sop class notes` |

## Defects

### D1, a NULL that ends a value which is not the last is accepted as the pad

**Where**: `crates/ocelli-dicom/src/conformance.rs:1817` (`valid_ui` strips one NULL from every value it is given) and `:851` (`note_root_facts` strips one NULL from the first value, through `first_text` at `:911`). `check_text` (`:998`) passes each value separately.
**What**: (0008,001A) stored as `1.2.3` NULL `\` `1.2.5` (12 bytes) gives no finding. (0008,0016) stored as the SC UID, NULL, `\`, `19620203`, NULL (36 bytes) gives `"sopClass":"1.2.840.10008.5.1.4.1.1.7","sopClassNote":null` with MULTIPLICITY and no UI-FORMAT. In both, value 1 is a UID followed by a NULL byte, and the NULL is not the pad.
**Why it is wrong**: PS3.5 6.2, UI: "If a Value Field containing one or more UIDs is an odd number of bytes in length, the Value Field shall be padded with a single trailing NULL". The pad belongs to the Value Field, so only the last value can end in it. PS3.5 9.1 limits a UID to digits and dots, so value 1 is not a UID. The checker's own length rule already reads 6.2 and 6.4 this way, because `exceeds` (`:1205`) and `without_pad_exception` (`:1142`) give the pad byte only to the last value of many, as design item 8 says. Only the format check and the capture give it to any value. The lead amendment repeats only "a well-formed UID", and this value is not one. As an independent check, pydicom 3.0.2 warns `Invalid value for VR UI: '1.2.840.10008.5.1.4.1.1.7\x00'` on the same file.
**Evidence**: `f225chk` on `f225p4/cases/multi_mid_null.dcm` and `sc_1null_then_multi.dcm`. The three-NULL version of the first case (`multi_mid_3null`) does give UI-FORMAT on value 1, so the only gap is the single NULL.
**Scope and regression**: present on the current tree, and present since pass 1. Pass 3's `conformance.rs` (`dd0c2c7e`) already had `raw.strip_suffix('\0')` in `valid_ui`, so the remediation did not introduce it, and it is the same D1 class that the brief asked me to sweep. There is no privacy exposure, because the only value repeated is under the root. Consumers affected: the report JSON, the TS reader (which accepts it) and the viewer half's `sopClass`. Smallest fix: strip the NULL only when the value is the last in the field (`number == total` in `check_text`), and at the capture only when (0008,0016) has a single value. Smallest regression: the two cases above, expecting UI-FORMAT on value 1, and for (0008,0016) also `sopClass` null and `not a valid UID` beside the MULTIPLICITY finding.

## Smells

### S1, a second one-NULL strip on the unevaluated path survives the suite

**Where**: `conformance.rs:721` (`unevaluated_scope` calls `ui_grammar`) and the fixture `fixture_ui_value_has_at_most_one_null_pad` (`tests/conformance.rs:1229`).
**Mechanism**: two helpers differ only in whether they strip a NULL first. `valid_ui` strips one and `ui_grammar` strips none. The capture has already stripped one, so calling `valid_ui` at `:721` (mutation R9) strips a second. The fixture only uses odd-length UIDs with three NULLs, and after two strips those still end in a NULL, so they fail anyway. With R9 in place, an even-length root UID with two NULLs (`1.2.840.10008.1.20` + 2 NULLs, 20 bytes, case `root_2null_even`) gives `"sopClass":"1.2.840.10008.1.20","sopClassNote":null` together with a UI-FORMAT finding, and the whole Rust suite is green. That is pass-3 D1's symptom exactly, reachable by a one-word edit that nothing pins. The two helper names are the hazard.
**Regression**: add a fixture with an even-length UID under the root plus two NULLs, expecting `sopClass` null, `not a valid UID` and one UI-FORMAT. Then confirm R9 goes red.

## Nitpicks

- N1. New. `under_dicom_root` (`conformance.rs:751`) and `componentsBeginWithDicomRoot` (`conformance.ts:223`) keep a raw-prefix fallback for a constant without the dot, plus `root.is_empty()` and empty-component guards. None of these can run on today's constant. They exist so that the dot-drop mutation reproduces the defect and goes red. Holding the root as its four components would make that spelling hazard disappear rather than keep it as dead code. The tests pin it either way, so this does not block.
- N2. New. Pass 3's evidence table says "12 unevaluated rows". Today's corpus output and the pass-2 output both have 11. That is a historical report, and it is noted only for the next reader.
- N3. Carried from pass 3 N1. The sort tiebreak reversal is still green. Not re-run.
- N4. Carried from pass 3 N2. The scope-sentence edit is red only in the typegen test. Not re-run.
- N5. Carried from pass 3 N3. The TS `isUid` bound can move to 65 with the suite green. Not re-run.
- N6. Carried from pass 3 N5. `tests/conformance.rs:1380` still cites PS3.5 6.1.2.3 for UR, where the direct text is PS3.5 6.2.
- N7. Carried from pass 3 N7. The plan's JSON example (`F-225-design.md:283`) still has no `sopClassNote`.

Closed from pass 3: N4, the backwards "prefix of the PS3.6 DICOM root" doc comment (rewritten). N6, the equivalent `"sopClassNote" in value` checks, is unchanged and still harmless, so it is not carried. N8, the D2 strip with no corpus row, is unchanged and covered by the unit test.

## Verified clean

- Pass-3 D1 is closed for single-valued fields. One NULL is stripped and more than one is not, at the capture, in the unevaluated scope, in `iod_of`, in the format check, for root and nested UI elements, for any UID in a sequence, and for (0008,0016) nested in an item, which does not set `sopClass`. All four strip-everything mutations (R5 to R8) are red on the new fixture. The one remaining gap is D1 above, a NULL before a delimiter.
- Pass-3 S1 is closed. The three mutations the lead named (dropping the dot, a raw prefix, removing the TS evaluated-branch check) are each red on both sides, and so are R3, R4, T4 and T5.
- The remediation changed nothing outside D1 and S1. In `conformance.rs` it touched only the hunks listed in the evidence table, and only four of the ten paths changed after pass 3. Corpus output is identical to pass 2 apart from the agreed `sopClassNote` key.
- Privacy: 0 leaks across 1,244 scanned values. No report repeats `19620203`, `1.2.840.100081.2`, a private UID or a malformed root UID.
- No `unsafe`, no `wasm-bindgen`, no new dependency, no `as` cast in the remediation hunks. A second run gives byte-identical output for all 240 reports, which the scratch binary asserts.

## Limitations and handoff

- Not exercised: the viewer half, group 0002 (which the checker does not walk by design), `metadata.rs`'s UI trim (outside the diff) and the deep-nesting debug-stack abort in `parse_part10` (outside the story, known since pass 1).
- Pass 3's copies of `tests/conformance.rs`, `conformance.ts` and `conformance.test.ts` are not on disk, so for those three files "changed nothing else" rests on reading the diff, the mtimes and the test counts, not on a `diff`.
- Scratch files are in the session scratchpad under `f225p4/` (`mk4.py`, `cases/`, `mut.py`, `reader.mjs`, `gates.sh`, `logs/`, `*.out`). The scratch build directory `target/f225p4chk` was deleted.
- Next owner: the implementer addresses D1 and S1, then a fresh independent pass 5 reviews that change. This is a bounded library-half review and not completed acceptance for F-225.
