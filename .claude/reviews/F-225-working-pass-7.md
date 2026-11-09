# F-225 review, pass 7

**Reviewed**: index tree `99f6d533687dd162d92e364f6a332116977d19cf` on `work/f-225-xai` at HEAD `259bcd5c` (the same index as passes 4 to 6), plus the unstaged diff over it (`git diff` sha256 `cd805fd0…`, the same ten paths as pass 6). `src/conformance.rs` sha256 `6ee2d61d…`, which is pass 6's hash. `tests/conformance.rs` sha256 `4a5f56fd…`, was `e1e7fe41…` in pass 6. `conformance.ts` sha256 `7e1a5d75…`, pass 6's hash. Library half only. Untracked `tools/oracle/out` and the pass-1 to pass-6 reports were ignored.
**Author / reviewer**: the remediation is by GLM-5.3 in opencode (log `glm-F-225-fix6.log` in the session scratchpad). The review is by a separate Claude subagent that wrote none of the design, the implementation, any remediation or any earlier pass.
**Result**: 0 defects, 0 smells, 8 nitpicks

The viewer half (AC32 to AC34, the folder viewer, README, `use.html`, `more.html`) is outside this run by instruction.

## Acceptance challenged

Pass-6 S1 asked for one fixture that pins the ruling on both sides. The new test `fixture_multi_valued_sop_class_is_scoped_by_its_first_value` (`tests/conformance.rs:1532`) with its two builders `ct_then_two_empty_values` (`:221`) and `empty_then_ct` (`:235`) was checked against what S1 asked for, written down before reading it.

| Asked for | In the fixture |
|-----------|----------------|
| CT UID, `\\`, NULL, 28 bytes. Evaluated as CT Image | `evaluated()` asserts `ModuleScope::Evaluated`, `sop == CT_UID`, `iod == "CT Image"`, `table == "PS3.3 A.3.3"`, `modules == PLANE_MODULES` (seven entries) |
| MULTIPLICITY 3 against 1, no UI-FORMAT | `assert_only` with exactly one finding, MULTIPLICITY on (0008,0016), `values(3, 1, Some(1))`. `assert_only` refuses any other finding, so a UI-FORMAT or a presence finding turns it red |
| `\` then CT, 26 bytes. Not evaluated | the `let ModuleScope::NotEvaluated { .. } else { unreachable!(..) }` binding |
| `sopClass` null and `sopClassNote` null | both on the typed scope (`assert_eq!` on `None` and `None`) and on the JSON (`"sopClass":null,"sopClassNote":null`) |
| MULTIPLICITY 2, no UI-FORMAT | `assert_only` with one finding, `values(2, 1, Some(1))` |

Byte construction, recomputed by hand. `CT_UID` is `1.2.840.10008.5.1.4.1.1.2`, 25 characters. CT plus two backslashes is 27, odd, so PS3.5 6.2 requires one trailing NULL, giving 28. A backslash plus CT is 26, even, so no pad. Both builders assert their length and go through `ui_field`, which asserts an even Value Field and adds nothing, and `el`, which asserts an even length again. The CT fixture is `changed(ct_elements(), ..)`, so the valid CT body is present and the single MULTIPLICITY finding means no presence finding appeared. `changed` removes the default (0008,0016) first, so the file holds one SOP Class element. The empty-first fixture carries only (0008,0016), which is enough because a not-evaluated scope runs no presence check, and P7-G below shows that this half does discriminate.

Comments. The fixture quotes PS3.5 6.4, "Each string Value in a multi-valued character string may be empty, unless otherwise specified", which matches the text pass 6 quoted from PS3.5 2026d. "28 bytes and three values, the second and third empty", "the NULL pads the last value, not the first", "26 bytes and two values, the first empty", "the scope repeats no UID and names no reason" were each checked against the bytes and against the observed report and are true. The single-pad sentence is PS3.5 6.4's "to the last Value" together with 6.2's UI NULL pad. The comments cite 6.4 only, which is enough for the sentence they make.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree above | now | 0 |
| `cargo clippy -p ocelli-dicom --all-targets -- -D warnings` | tree above | now | 0 |
| `cargo test -p ocelli-dicom` | tree above | now | 0. 265 passed, 4 ignored, 14 test binaries plus doctests. `tests/conformance.rs` 42, up from 41 |
| `cargo test -p ocelli-dicom --test corpus -- --ignored --nocapture` | `corpus/manifest.tsv`, 95 rows | now | 0. 4 passed, `TYPE2C-ABSENT 42`, `corpus rows 95`, as in pass 6 |
| `python3 scripts/prose_check.py` | tree above | now | 0, clean over 855 files |
| `python3 scripts/source_provenance_check.py` | tree above | now | 0, clean over 1692 files |
| source unchanged | sha256 of `src/conformance.rs` | now | `6ee2d61dea53…`, the prefix pass 6 recorded. `conformance.ts` `7e1a5d75…`, pass 6's. Every other changed path has an mtime before pass 6's report (19:39), apart from `tests/conformance.rs` (20:12) and `src/conformance.rs` (20:13, see N8) |
| mutations, `f225p7/mut7.py` and `mut7b.py` in the session scratchpad | a copy of the workspace under `target/f225p7mut/ws` with its own target directory, so the worktree was never edited. The copy's `conformance.rs` sha256 was asserted restored after each run. The directory was deleted at the end | now | Table below |
| `git write-tree`, `git diff` hash, file hashes and `git status --short` at the end | worktree | now | unchanged from the start: `99f6d53`, `cd805fd0…`, the hashes above, status identical apart from this report |

The suite for each mutation is `cargo test --no-fail-fast -p ocelli-dicom --lib --test conformance --test conformance_typegen`. A baseline run on the copy was green (14, 42, 1).

| Mutation to `src/conformance.rs` | Result |
|----------------------------------|--------|
| P7-F (pass 6 P6-F). `iod_of` returns None when the field has more than one value | red, `fixture_multi_valued_sop_class_is_scoped_by_its_first_value` only, 41 passed and 1 failed |
| P7-G (P6-G). `iod_of` takes the first non-empty value | red, the same test only |
| P7-H (P6-H). `iod_of` returns None when any value is empty | red, the same test only |
| P7-J, adjacent. `unevaluated_scope` notes an empty value 1 as `not a valid UID` | red, the same test only |
| P7-K, adjacent. `unevaluated_scope` takes the first non-empty value | red, the same test only |

The implementer's log reports P6-F and P6-H failing at `evaluated()` and P6-G failing on the `"sopClass":null` assertion of the second half, so each half of the fixture is the one that catches the mutation aimed at it. My replay agrees on the test name and counts.

## Defects

None.

## Smells

None. Pass-6 S1 is closed: the ruling that a VM-1 SOP Class with extra empty values is scoped by its first value is pinned on both readers of the field, `iod_of` (P7-F, P7-G, P7-H) and `unevaluated_scope` (P7-J, P7-K).

## Nitpicks

- N1 to N7. Carried from pass 6 unchanged, because the files they name were not touched this round. N1 the `vm-empty-counted` comment clause that cannot fail. N2 the `valid_ui` and `unevaluated_scope` doc comments describing the double-strip hazard. N3 unreachable root guards. N4 sort tiebreak reversal green and the scope-sentence edit red only in typegen (not re-run this pass). N5 the TS `isUid` bound (not re-run, `conformance.ts` unchanged). N6 the PS3.5 6.1.2.3 citation for UR and the plan's JSON example without `sopClassNote`. N7 a space-only Type 1 CS, a plausible concern the standard does not decide.
- N8. New, process. The implementer replayed its mutations by editing `src/conformance.rs` in the worktree and restoring it, not on a copy. The restore is exact (the hash matches pass 6), but the mtime moved to 20:13 and a crash mid-run would have left a mutated source in the reviewed tree. It also left `target/f225-s1/` and `target/f225-s1-baseline.log` behind. Both are under ignored `target/` and change nothing reviewed.

## Verified clean

- The new fixture asserts both sides S1 asked for, through the typed `ModuleScope` and the JSON, and `assert_only` closes it against any extra finding.
- The byte lengths (28 and 26), the even padding and the single NULL pad are right by hand count and by the builders' own assertions.
- The comments cite PS3.5 6.4 with the correct quotation and every factual sentence in them matches the observed report.
- Three requested mutations and two adjacent ones go red on this fixture and nothing else, so it is the only pin, and it holds.
- `src/conformance.rs` and `conformance.ts` are byte-identical to pass 6. The implementer's claim that it changed only `tests/conformance.rs` agrees with the hashes and mtimes.
- The gates listed above are green now. The implementer's gate tails (42 in `tests/conformance.rs`, 265 total, 855 and 1692 files) match what I ran.

## Limitations and handoff

- Not exercised this pass: the npm build and tests, `gate packages`, `gate guards`, the guard census, the privacy scan and the TS reader. No TS or library source changed, so pass 6's receipts for them stand for those paths, but they were not re-run against this tree.
- There is no saved copy of pass 6's `tests/conformance.rs`, so "only the new test and its two builders and fixture rows were added" rests on reading the new material and on pass 6's fixture names, not on a file diff.
- Scratch files are under `f225p7/` in the session scratchpad (`mut7.py`, `mut7b.py`, `*.out`, gate logs). `target/f225p7mut` was deleted.
- Next owner: the lead. This is a clean bounded library-half pass, not completed acceptance for F-225. The viewer half still needs its own review.
