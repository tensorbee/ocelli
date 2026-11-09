# F-225 review, pass 3

**Reviewed**: index tree `99f6d533687dd162d92e364f6a332116977d19cf` on `work/f-225-xai` plus the unstaged remediation over it (`git diff` sha256 `d0111675…`, ten paths: the plan amendment, `conformance.rs`, `lib.rs`, `tests/conformance.rs`, `tests/conformance_typegen.rs`, `conformance.generated.ts`, `conformance.test.ts`, `conformance.ts`, `index.test.ts`, `index.ts`). `conformance.rs` sha256 `dd0c2c7e…`, `conformance.ts` sha256 `1fd89fb5…`. Library half only. Untracked `tools/oracle/out` and the pass-1 and pass-2 reports ignored.
**Author / reviewer**: remediation by the xAI Grok Build CLI, an implementer and not the out-of-bounds AGPL Grok JPEG 2000 codec (report `grok-F-225-fix2.log` in the session scratchpad), amendments by the sprint lead. Review by a Claude subagent that wrote none of the design, the implementation, either remediation or either earlier pass.
**Result**: 1 defect, 1 smell, 8 nitpicks

The viewer half (AC32 to AC34, the folder viewer, README, `use.html`, `more.html`) is out of this run by instruction and is not a finding.

## Acceptance challenged

Every case is an explicit VR little endian Part 10 file from the pass-1 byte builder `mk.py`, extended as `p3r/mk3.py` (47 files), run through `parse_part10` and `check_conformance` by the pass-1 scratch binary `f225chk` (target `target/f225chk`, depends on the worktree crate by path). Expected answers were written from the amendment text, PS3.5 2026d 6.2 and 9.1 and PS3.3 C.12.1.1.2 before running.

**N4 under the amendment.** The rule: repeat only a well-formed UID that begins `1.2.840.10008.`.

| Case, (0008,0016) as stored | Expected | Observed |
|-----------------------------|----------|----------|
| `1.2.3.4` | null, `not a DICOM UID` | same, value absent from JSON |
| `19620203` | null, `not a DICOM UID` | same, absent |
| `2.25.1234567890` + NULL | null, `not a DICOM UID` | same, absent |
| `1.2.840.10008` + NULL (the root, no trailing dot) | null, `not a DICOM UID` | same, absent |
| `1.2.840.100081.2` (shares the prefix string, not under the root) | null, `not a DICOM UID` | same, absent |
| `1.2.840.10008.5.1.4.1.1.7` + one NULL (SC) | repeated, null note | same |
| `1.2.840.10008.5.1.4.1.1.2` + one NULL (CT) | evaluated as CT | same |
| `1.2.840.10008.` | null, `not a valid UID`, UI-FORMAT | same |
| `1.2.840.10008.05.1` | null, `not a valid UID`, UI-FORMAT | same |
| SC UID + space | null, `not a valid UID`, UI-FORMAT | same |
| leading space + root UID | null, `not a valid UID`, UI-FORMAT | same |
| root + 51 digits (65 bytes) + NULL | null, `not a valid UID`, VALUE-LENGTH | same |
| `1.2.840.10008.0`, `1.2.840.10008.1` | repeated (under the root, well formed) | same |
| `SC\19620203` | SC repeated, MULTIPLICITY, `19620203` absent | same |
| `19620203\SC` | null, `not a DICOM UID`, MULTIPLICITY | same |
| empty | null, null | same |
| **SC UID + three NULLs** | **null, `not a valid UID`, UI-FORMAT** | **SC repeated with a null note, and UI-FORMAT. See D1** |

The prefix test is on UID components today, because the constant carries the trailing dot and the value has already passed 9.1. Nothing pins that, see S1.

**S1 replay (pass-2 smell).** The three new tests exist and each of the three pass-2 mutations is now red on a named test (table below). Independent cases under `\ISO 2022 IR 100`: all 18 tails of Tables C.12-3 and C.12-4 in an LO give no finding. ESC `%G`, ESC `(C`, ESC `$(C` (a prefix of a 3-byte tail) and ESC `-E` (not in the table) give REPERTOIRE control at index 2. ESC `-A` in UR (index 14) and in CS (index 1) give control. ESC `-A` in LT and LO gives none. ESC `-A` under `ISO_IR 100` alone gives control. I re-extracted the 18 sequences from the fetched PS3.3 2026d C.12 page, including the second IR 13 sequence 02/08 04/10, and they match the pairs written in `iso2022_tails_match_ps33_tables`.

**N7.** Both branches of `modules_json` emit `sopClassNote` after `sopClass`. `SOP_CLASS_NOTES` is generated into `conformance.generated.ts` and the typegen test goes red when a note string changes. Against the built `dist/conformance.js`, the reader accepts `(null, "not a DICOM UID")`, `(null, "not a valid UID")` and `(SC, null)`, and refuses `Not a DICOM UID`, a note paired with a UID, and a sopClass of `1.2.840.10008`, `1.2.840.10008.`, `1.2.840.100081.2` or `2.25.1`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree above | now | 0 |
| `cargo clippy -p ocelli-dicom --all-targets -- -D warnings` | tree above | now | 0 |
| `cargo test -p ocelli-dicom` | tree above | now | 0. 258 passed, 4 ignored, 15 binaries. Conformance 37, lib 12 including `iso2022_tails_match_ps33_tables` |
| `cargo test -p ocelli-dicom --test corpus -- --ignored --nocapture` | `corpus/manifest.tsv`, 95 rows | now | 0. 4 passed, `TYPE2C-ABSENT 42`, `corpus rows 95`, no other rule |
| scratch `f225chk` over 95 corpus files | `corpus/data` | now | Rule counts `{TYPE2C-ABSENT: 42}` only. Every module scope has a null note, 12 unevaluated rows all repeat a root UID |
| `npm run build`, `typecheck`, `lint`, `test` | tree above | now | 0, 0, 0, 0. 61 files, 492 tests |
| `bin/ocelli.sh gate packages` | tree above | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | tree above | now | 0. 418 refusal probes, 37 guards red for their declared reason, 0 known defects |
| `python3 scripts/guard_census.py`, `--check-runbook` | tree above | now | 0, 0. `0 watched by nothing` |
| `python3 scripts/prose_check.py`, `source_provenance_check.py` | tree above | now | 0 over 855 files, 0 over 1692 files |
| the 64 pass-1 derived cases (72 files) and 50 pass-2 cases rerun | scratchpad `cases/`, `p2r/cases/` | now | parsed and checked, outputs in `p3r/p1cases.out`, `p3r/p2cases.out` |
| privacy scan, `p2r/scan.py`, values of 3 or more characters | 259 reports: pass-1, pass-2 and pass-3 cases, planted `19620203`, `1.2.3.4`, `2.25.1234567890`, `1.2.840.100081.2`, name, MRN, ESC and BEL values | now | 0 leaks over 543, 484 and 448 values. Root-prefixed values are substrings of the registered UIDs and so are outside that scan, and were checked directly per case (table above) |
| same scan over the corpus, restricted to findings, counts and the four scope strings | 95 corpus reports, 8087 values | now | 2 hits, both the literal JSON key `value` matching a corpus word. 0 leaks. The unrestricted scan's other hits are fixed module and not-checked vocabulary (`Patient`, `Pixel`, `Structured`) |
| 32 pass-2 mutation entries (one anchor no longer matches) plus 14 new, `p3r/mut5.py`, file restored and sha256 checked (`dd0c2c7e…`) | `conformance.rs` | now | Table below |
| 11 TS mutations, `p3r/tsmut3.py`, restored and sha256 checked (`1fd89fb5…`) | `conformance.ts` | now | 6 red, 5 green, see S1, N3 and N6 |
| TS reader against real Rust output, `p3r/reader.mjs` | built `dist/conformance.js`, 259 reports (95 corpus, 67 pass-1, 50 pass-2, 47 pass-3) | now | 259 accepted, 0 refused |
| `git write-tree`, `git diff` hash and `git status --short` at the end | worktree | now | unchanged: `99f6d53`, `d0111675…`, same status lines |

Rust mutations, each on its named test:

| Mutation | Test | Result |
|----------|------|--------|
| Add `%G` to `ISO2022_TAILS` | `fixture_unregistered_escape_is_a_control` and `iso2022_tails_match_ps33_tables` | red, red |
| Add `(C` | `fixture_unregistered_escape_is_a_control` | red |
| Delete `$B` | `iso2022_tails_match_ps33_tables` | red |
| Delete `-A` | table test and `fixture_escape_requires_an_iso_2022_sequence` | red, red |
| Drop `extended_string(vr)` from the skip | `fixture_registered_escape_in_ur_is_a_control` | red (UrFormat instead of Repertoire) |
| Root check `if true` | `..._only_under_the_dicom_root` and `no_finding_carries_a_value` | red, red |
| Root constant `"1."` | full conformance binary | red |
| **Root constant without its trailing dot** | **full conformance binary and lib tests** | **green, see S1** |
| Private UID given the `not a valid UID` note | `..._only_under_the_dicom_root` | red |
| `not a DICOM UID` text edited | typegen test | red, "stale: line 36" |
| Evaluated `sopClassNote` dropped | `fixture_valid_ct_has_no_findings` | red |
| The other 32 pass-2 entries | as pass 2 | 26 red, 1 skipped (the pass-1 UTF-8 anchor, whose rewrite is red). Green: sort tiebreak (N1), scope sentence (N2), D2 strip against the corpus (N8), "tails drop -A", which is mis-named in the pass-2 script and adds `(C` (red on the new test above), and the `extended_string` drop against the old ESC test (red on the new UR test above) |

## Defects

### D1, a malformed SOP Class UID is repeated after its extra NULLs are trimmed

**Where**: `crates/ocelli-dicom/src/conformance.rs:819` (`note_root_facts` stores `text.trim_end_matches('\0')`) and `:716` (`unevaluated_scope` trims again before `valid_ui`).
**What**: (0008,0016) stored as `1.2.840.10008.5.1.4.1.1.7` followed by three NULLs (28 bytes, case `n4_sc_two_null`) gives `"sopClass":"1.2.840.10008.5.1.4.1.1.7","sopClassNote":null` and, in the same report, a UI-FORMAT finding on (0008,0016). Every trailing NULL is removed before the grammar check, so `valid_ui` never sees the extra padding. The report repeats a value that it also calls malformed, and the text it repeats is not the stored text.
**Why it is wrong**: the lead amendment, which is authoritative: "A value is repeated in `sopClass` only when it is a well-formed UID beginning `1.2.840.10008.`". PS3.5 6.2, UI: a value ending on an odd boundary is padded with one trailing NULL. Three NULLs is not that, and the checker agrees, since it reports UI-FORMAT.
**Evidence**: `f225chk` on `p3r/cases/n4_sc_two_null.dcm`, as above. The one-NULL case `n4_sc_null` is right. The same trim feeds `iod_of`, so CT with three NULLs (`n4_ct_three_null`) is evaluated as CT beside its UI-FORMAT finding. Evaluating an IOD despite malformed padding may be intended. Repeating the value under the "well-formed" rule is not.
**Scope and regression**: present on the current tree. No privacy exposure, because the repeated text is under the DICOM root. Affected consumers: the report JSON, the TS reader (which accepts it) and the viewer half when it shows `sopClass`. Smallest regression: a fixture with the SC UID and three NULLs, expecting `sopClass` null, `not a valid UID` and one UI-FORMAT. Fix by stripping at most one NULL (`strip_suffix('\0')`) at the capture or in `unevaluated_scope`. The lead decides whether `iod_of` keeps the lenient trim.

## Smells

### S1, the root's component boundary is unpinned on both sides

**Where**: `conformance.rs:35` (`DICOM_UID_ROOT = "1.2.840.10008."`) and `:720`, `packages/core/src/conformance.ts:21` and `isDicomRootUid`. The TS evaluated branch's `!isDicomRootUid(value.sopClass)` check (`conformance.ts:162`).
**Mechanism**: the rule is on UID components only because the constant ends in a dot. The PS3.6 root is always written `1.2.840.10008`, without one, so the dot is the likeliest thing for a later edit to drop. With the dot removed, the full Rust suite and the TS suite stay green, and both sides then repeat `1.2.840.100081.2` and the bare `1.2.840.10008`. Neither is under the root. Removing the root check from the TS evaluated branch is also green. That is the same shape as pass-2 S1, a rule the lead asked to have checked that can widen with nothing going red.
**Regression**: add `1.2.840.10008` and `1.2.840.100081.2` to the Rust root fixture, expecting null and `not a DICOM UID`. In `conformance.test.ts`, refuse both as an unevaluated `sopClass`, and refuse an evaluated report whose `sopClass` is `1.2.3.4`.

## Nitpicks

- N1. Carried from pass 2 N1. The sort tiebreak reversal is still green.
- N2. Carried from pass 2 N2. The scope sentence edit is green on `scope_sentence_is_exact` and red only in the typegen test.
- N3. Carried from pass 2 N5. The TS `isUid` bound can move to 65 with the suite green.
- N4. `conformance.ts:207`, the doc comment on `isDicomRootUid`, "and a prefix of the PS3.6 DICOM root", has the relation backwards. The text begins with the root. It is not a prefix of it.
- N5. `tests/conformance.rs:1270`, `fixture_registered_escape_in_ur_is_a_control`, cites PS3.5 6.1.2.3 for UR, but that sentence names AE and CS. The direct text is PS3.5 6.2, UR: it "shall not use any code extension or replacement techniques". The 6.1.2.2 citation in the same comment is right, and so is the conclusion.
- N6. Both `"sopClassNote" in value` checks in the TS reader are equivalent mutants. The next check refuses `undefined` either way. They do no harm.
- N7. The plan's JSON example (`F-225-design.md:283`) still has no `sopClassNote`. The amendment says the shape gains it and is authoritative, so the example is only stale.
- N8. Carried from pass 2 N6. The D2 strip has no corpus row that exercises it. The unit test covers it.

## Verified clean

- Pass-2 S1 is closed. The three tests named by the lead exist. Each of the three named mutations (unregistered tail, `$B` deleted, `extended_string` dropped) is red on a named test. The table test's pairs come from the standard and not from the constant, and I re-derived them from the PS3.3 page.
- Pass-2 N4 is closed for every case in the N4 table except D1. The prefix test is component-wise on today's constant. `19620203`, `1.2.3.4`, `2.25.…` and `1.2.840.100081.2` never appear in any report.
- Pass-2 N7 is closed. The key is on both branches, in the generated TS and in the reader. The reader refuses any other note, a note paired with a UID, and a sopClass outside the root. The new export `SOP_CLASS_NOTES` and the type `ConformanceSopClassNote` are in `index.ts`, and `SOP_CLASS_NOTES` is in `PUBLISHED`.
- Corpus: `TYPE2C-ABSENT 42` and nothing else, matching pass 2. No corpus row changes note or scope.
- No `unsafe`, no `wasm-bindgen`, no new dependency. Output is byte-identical on a second run for all 259 inputs (the scratch binary asserts it).

## Limitations and handoff

- Not exercised: the viewer half, ISO 2022 multi-byte decoding beyond the ASCII-trailing cases above, and GB18030. `docs/lld/public-api.md` does not list `SOP_CLASS_NOTES` yet. That belongs to the completion step, not to this library-half review.
- I wrote into an existing scratchpad directory `p3r/` that also holds an earlier, unrelated review's `*.txt` files. I added `*.log`, `mk3.py`, `mut5.py`, `tsmut3.py`, `scan3.py`, `reader.mjs` and `cases/`, and overwrote nothing with those names.
- Next owner: the implementer addresses D1 and S1. A fresh independent pass 4 then reviews that change. This is a bounded library-half review, not completed acceptance for F-225.
