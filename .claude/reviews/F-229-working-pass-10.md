# F-229 review, pass 10

**Reviewed**: `git diff 3f8fba7 HEAD`, HEAD `a087f6102c1be3956f3adec1b4a4c00ce44516ed` on `work/f-229-xai`, AC24 only. Paths: `tools/oracle/unsupported.json`, `docs/lld/oracle.md`. No unstaged tracked changes.
**Author / reviewer**: a Claude implementer wrote `a087f61`. This pass is an independent Claude reviewer that wrote none of the reviewed material.
**Result**: 1 defect, 0 smells, 3 nitpicks

## Acceptance challenged

AC24. I checked the five JPEG XL rows against what the harness actually reported, not against the plan's prediction. Each `errorContains` and `boundary` was compared with the real `--report-unsupported` output. I mutated the entries and fed them through the harness's own matcher to show that a wrong entry fails, and I ran the full gate to show that no entry claims a row that succeeds. A wrong behaviour these checks rule out: an entry written from expectation, for example `boundary: "presented"`, or a UID typo in `errorContains`, or a row filed under the wrong syntax. Each of those is refused, as shown below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `node tools/oracle/run.mjs --report-unsupported` | HEAD a087f61, corpus 100 rows | now | exit 2 (report mode). Seven candidates. The five JPEG XL rows are all `decoded` with `No decoder for transfer syntax <uid>`, the UIDs .110 (2 rows), .111 (1) and .112 (2). Character-identical to the three entries |
| `grep -rlE '10008\.1\.2\.4\.11[012]'` over all of `node_modules/@cornerstonejs/` | dicom-image-loader 5.8.2, core 5.8.2 | now | no file. The only JPEG XL trace is `wadors/loadImage.js:37`, `'image/jxl': '1.2.840.10008.1.2.4.140'`, a MIME map. It is none of the three UIDs and has no decoder |
| `dicom-image-loader/dist/esm/imageLoader/decodeImageFrame.js` | 5.8.2 | now | the switch has no case for .110, .111 or .112. Line 61 is `Promise.reject(new Error(\`No decoder for transfer syntax ${transferSyntax}\`))`, as the three `why`s quote |
| `ls node_modules/@cornerstonejs` | installed | now | the codec packages are exactly the four in fact 12 |
| scratch `mutate.mjs`, which imports `tools/oracle/src/unsupported.mjs` (`entryFor`, `validateRecord`, `claimedRows`) and replays the report candidates against mutated copies of the committed record | scratchpad, record unchanged on disk | now | baseline: 0 problems. `.111` errorContains changed to `.119`: the row is unaccounted. `.110` boundary changed to `presented`: both rows unaccounted. The xyb row moved into the `.110` entry: unaccounted (transferSyntax conjunct). The `.111` entry removed: unaccounted |
| `bin/ocelli.sh gate oracle`, run alone | HEAD a087f61 | now | exit 0, `gate verdict: pass`. `applicable 100, reached 100, decoded 94, presented 94, read back 93, unsupported 7`. Self test faults all red as required. Compare shows `unsupported source rows: 7` |
| `python3 scripts/prose_check.py` | worktree | now | exit 0, 864 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | exit 0, 1710 files |
| `python3 scripts/guard_census.py` | worktree | now | exit 0, 0 watched by nothing |
| `scripts/tests/test_corpus_synth.py::test_lossless_syntaxes_round_trip_exactly`, read only, plus `corpus/decode-digests.tsv` | worktree | read, not run | the .110 rows are compared with `SYNTAX_REFERENCE` by `np.array_equal`, and .111 is skipped as unavailable. `jpegxl_lossless.dcm`'s decode digest equals `explicit_vr_le.dcm`'s (`9cb18079...`) |

The harness has no option for an alternate `unsupported.json`, because `UNSUPPORTED_PATH` is fixed by `oraclePath`. So the mutations went through its own exported matcher, not through a mutated browser run. The step from a matcher miss to a failed gate is my reasoning from `run.mjs`, not something I ran. At `run.mjs:1052-1071` an unmatched failure pushes a problem, and at 1821-1849 any problem returns 1.

## Defects

### D1, the applicability sentence two lines above the edit still encodes two unsupported rows of ninety-two

**Where**: `docs/lld/oracle.md:576-577`, directly above the edited line 581
**What**: "That is the difference between an oracle that covers 90 rows and knows it, and one that covers 90 and reports 92." The commit replaced line 581's "Two rows of ninety-two" because that count is now wrong, and added a five-row bullet, so the section lists seven unsupported rows. The sentence just above still says 92 rows applicable, 90 covered, 2 unsupported. The measured run is 100 applicable, 93 read back, 7 unsupported.
**Why it is wrong**: under microscope class 3 a false prose claim is a defect. The brief puts a contradiction with the edited lines in scope. Here the sentence and the edited list give different unsupported counts for the same section (2 against 7).
**Evidence**: the gate run above (`read back 93, unsupported 7`), plus `git show 3f8fba7:docs/lld/oracle.md`, where 576-577 and the old 581 agreed on two of ninety-two.
**Scope and regression**: present on HEAD. Readers of the LLD are affected. The smallest fix follows CLAUDE.md's own lesson: remove the numbers instead of updating them, e.g. "an oracle that covers what it rendered and knows it, and one that reports every row as covered". A count beside the command that prints it goes stale again.

## Smells

None.

## Nitpicks

- **N1, the older stale counts, for the lead.** None of these contradicts the edited lines. `oracle.md:49` "92 digests" (illustrative). Lines 281, 385 and 761 say "the ninety rendered rows". Lines 1105 and 1143 say "the ninety stack frames". Line 1152 says "on this corpus ... `readBack: 90`", which is a present-tense corpus claim and the measured value is 93. Lines 611-616 and `unsupported.json`'s `$notes` say "both JPEG-LS rows" and "those five rows", and the manifest now has three JPEG-LS rows (`jpegls_lossless`, `jpegls_lossless_rgb8`, `jpegls_near_lossless`). These go back before F-229.
- **N2, `.110` why, relative clause.** "the uncompressed synthetic source ..., which libjxl encoded" grammatically says libjxl encoded the source. libjxl (imagecodecs) encoded the JPEG XL rows from it. Decision 14's own "encoded it" is just as ambiguous. Suggested: "...source, independent of `jxl-oxide` because libjxl encoded the rows".
- **N3, `.112` why, causal "so".** "This is a capability beyond the ... parity surface, so the rows have no reference". The reason there is no reference is fact 12, which the previous sentence already gives. Both statements are true, but the "so" credits the parity surface with the cause.

## Verified clean

- All three `errorContains` are byte-identical to the report's `error` strings, and all three `boundary` values are `decoded`, as the report says.
- The grouping is one entry per transfer syntax with every row listed. The six required keys are present and `validateRecord` accepts the file. The transferSyntax conjunct refuses a row filed under the wrong syntax (mutation above).
- With the real matcher, a wrong errorContains, a wrong boundary, a misfiled row and a missing entry each leave the row unaccounted.
- The gate passes with 7 unsupported, so no entry claims a row that renders. Stale-claim branch: `run.mjs:1074-1084`.
- Fact 12: the four codec packages are as listed. No file in any `@cornerstonejs` package names .110, .111 or .112. The rejection in `decodeImageFrame.js` is quoted correctly. The new `oracle.md` bullet's claims (no JPEG XL codec, no UID in `dist`, rejects at decode with the message and UID, three entries copied from the report) are all true.
- `.111` why: the operator's ruling in the design plan ("`.111` is reported unavailable") exists, and `crates/ocelli-codec/tests/jpegxl.rs::jpegxl_recompression_is_unavailable` and `registry.rs` carry it.
- `.110` why: the round trip against the uncompressed synthetic source exists (`test_lossless_syntaxes_round_trip_exactly`, decode digests equal to the references).
- The header's F-ID list and date were updated. Prose and provenance checks are clean, and the census bucket is empty.
- `README.md`, `use.html` and `more.html` mention neither JPEG XL nor `unsupported.json`, so the oracle edit leaves nothing stale there.

## Limitations and handoff

- The mutation was not run through a browser, because the harness has no alternate-record option. It was driven through the harness's exported matcher, plus reasoning from `run.mjs`.
- The corpus round-trip test was read, not executed, in this pass.
- The worktree's `tools/oracle/out` is a real ignored directory, not a link to the main checkout as the brief said. The gate rewrote the worktree's own `out/` (mtime 20:25). The main checkout's `tools/oracle/out` was not touched (mtime 5 Nov 01:46).
- Tree unchanged: HEAD `a087f61`, no tracked modification. The only new file is this report.
- Next: the implementer fixes D1. N1 goes to the lead as a separate cleanup. AC24 then needs a fresh pass. Viewer docs (`use.html` etc.) for JPEG XL belong to other ACs and were out of scope.
