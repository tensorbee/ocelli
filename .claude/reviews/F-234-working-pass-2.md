# F-234 review, pass 2

**Reviewed**: `git diff 44c2fcb2 HEAD` on `work/f-234-xai`, HEAD `1a72f35ceb2961ee0cdfb7784c0c98527103aa9e`, tree `c4cc932d15116b7e733eae33f076411f8eb62a0a`. Before and after the pass the only untracked path was `.claude/reviews/F-234-working-pass-1.md`, plus this report.
**Author / reviewer**: GLM-5.3 in opencode did the remediation, per the caller and the run log `glm-F-234-fix1.log`. The commit is authored as Atul Sharma, so the commit was made after the run, not by GLM, whose report says "no commit or push". The reviewer is a Claude subagent spawned by the lead. It is independent of the remediation and did not write the plan or the pass 1 review.
**Result**: 2 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **AC9 and the lead amendment, the measured inputs.** I derived each input again myself: a pydicom scan of the corpus, the bytes each TypeScript writer produces today when built from the worktree source, the Rust fixture builders, and the PS3.16 2026d template rows read from dicom.nema.org together with the PS3.3 macros those content items carry. GLM's claim was that the template chain reaches 5, so 7 wrapped. That claim is rejected.
- **D2 of pass 1, depth against a count.** I replayed M1.
- **D3 of pass 1, AC10 can fail.** I replayed M4 against the shipped AC10 test and against AC3.
- **S1 of pass 1, deflate parity.** I swept the constant to see which tests depend on the value rather than on the behaviour.
- **The TID 1500 fixture is real writer output.** I wrote the document again from `packages/core/src/tid1500.ts` through esbuild, not from the main checkout's `dist`, which is older than its source (02:15 against 02:46), and compared the two structurally.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree at 1a72f35c | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-dicom -p ocelli-folder-viewer` | same | now | 0. part10 52 passed. Folder viewer 72 + 60 + 1 + 1 + 1 + 24 |
| `npx vitest run packages/core` | same | now | 0. 49 files, 444 tests |
| `cargo test -p ocelli-dicom --test corpus -- --ignored` | scratch copy from `git archive HEAD`, the main checkout's `corpus/data` symlinked, its own target dir | now | 0, 4 passed |
| `guard_census.py`, `--check-runbook`, `prose_check.py`, `source_provenance_check.py` | worktree | now | 0, 0, 0 ("890 file(s)") and 0 ("1757 files") |
| pydicom 3.0.2 depth scan of `corpus/data` | main checkout, 100 `.dcm` plus 4 LICENSE files | now | deepest 2, `synthetic/ct_multiframe_perframe.dcm`, SharedFunctionalGroupsSequence > PlaneOrientationSequence |
| `REPOSITORY_TID_1500` pulled out of `part10.rs`, scanned | 2974 bytes | now | depth 5: ContentSequence x3 > MeasuredValueSequence > MeasurementUnitsCodeSequence |
| `writeTid1500` from the worktree's `tid1500.ts` (esbuild bundle), one root NUM, the test's concept | worktree source | now | 2974 bytes, depth 5. The pydicom structure dump (108 lines, UIDs excluded) is **identical** to the embedded constant |
| `writeKeyObject`, `writeSecondaryCapture` from worktree source, the test requests | worktree source | now | KOS 2 (ContentSequence > ConceptNameCodeSequence), SC 0 |
| Rust fixture builders | `git grep` for writers. dcmjs is used only in `tid1500.ts`, `tid2010.ts` and `secondary-capture.ts`, and `corpus_synth.py` is the only Python writer, whose output is the corpus | now | the deepest Rust builder outside part10 is `value.rs`, PerFrameFunctionalGroups > PixelValueTransformation, which is 2 |
| PS3.16 2026d TID 1500, 1501, 300, 301, 320, 1600, 1601, 1602, 2010 rows, and PS3.3 2026d C.18.3, C.18.4, C.12.1.1.9 | `chapter_A.html`, `sect_TID_2010.html`, `sect_C.18.3.html`, `sect_C.18.4.html`, `sect_C.12.html`, pages headed "PS3.16 2026d" and "PS3.3 2026d" | now | see D1 |
| pydicom model of that chain using the real tags | TID 1500 r6 > r9 TID 1501 r10 TID 300 > TID 301 r13 > TID 320 r3 SCOORD > r4 IMAGE with C.18.3 Referenced SOP Sequence | now | **6** unwrapped and **8** wrapped in (0400,0561) > (0400,0550). With the Type 3 presentation-state Referenced SOP Sequence of C.18.4 it is **7** and **9** |
| M1, `sequence_depth -= 1` deleted | scratch copy | now | part10: 50 passed, 2 failed (`many_items_each_nested_to_the_limit_parse`, `sibling_sequences_each_nested_to_the_limit_parse`). Pass 1's D2 is closed |
| M4, the three-line depth check deleted | scratch copy | now | the AC10 test `a_sequence_deep_enough_to_abort_an_unchecked_parse_is_refused_and_the_process_answers_next`: "has overflowed its stack", SIGABRT, cargo 101. AC3 `refuses_deep_nesting_without_recursing`: the same. Pass 1's D3 is closed |
| Constant swept to 6, 7, 8, 16, 33, 48, 64, 128 | scratch copy, part10 | now | 16, 33, 48, 64 and 128 all give 52 passed, so AC1 at 128 fits 2 MiB debug. 8 and 7 give one failure only, `a_deflated_odd_length_stream_requires_and_accepts_its_null_pad`, on its "the fixture stream is odd" precondition. At 6, `the_deepest_measured_shape_parses_at_the_limit` fails. At 8, all of `-p ocelli-dicom -p ocelli-folder-viewer` fail only that deflate test |

Every scratch copy, venv, generated file and NEMA download has been deleted, along with the scratch target dir. After the pass, HEAD and `git write-tree` are unchanged and `git status` shows only the two review files.

## Defects

### D1, the template-chain measurement is 6 rather than 5, and three sentences state 5 and 7

**Where**: `crates/ocelli-dicom/src/parse.rs:44` to `46` ("the TID 1500 measurement chain of PS3.16 2026d at five Sequences, seven wrapped"). `docs/lld/dicom-ingest.md:76` to `78` ("the TID 1500 template chain of PS3.16 2026d reaches 5 ... so the deepest input is 7 wrapped"). The doc comment on `deepest_measured_shape_data_set` at `part10.rs` about 702 to 713 ("reaches five open Sequences ... lands at seven").
**What**: In the PS3.16 2026d rows GLM cites, the TID 300 NUM is the item in the Measurement Group's Content Sequence, which is the third. The NUM's own Content Sequence (TID 301 row 13, which includes TID 320) is the fourth. The TID 320 row 3 SCOORD's Content Sequence, which holds the row 4 SELECTED FROM IMAGE, is the fifth. An IMAGE content item carries the Image Reference Macro, PS3.3 Table C.18.4-1, and that macro includes the Composite Object Reference Macro, Table C.18.3-1, whose Referenced SOP Sequence (0008,1199) is **Type 1**. That is the sixth. Two other listed chains also reach 6. One is TID 301 row 9, INFERRED FROM NUM, whose own Measured Value Sequence and Measurement Units Code Sequence are the fifth and sixth. The other is TID 1500 row 5 to TID 1600 row 4 to TID 1601 row 2 to TID 1602 rows 11 and 12, which are NUM descriptors under the library IMAGE. GLM's own report gives 4 for the Image Library chain, which counts the IMAGE's Referenced SOP Sequence but not the TID 1602 NUM under it. Pass 1's estimate of 6 was right.
**Why it is wrong**: The lead amendment makes the template shapes an input to the rule, and microscope section 3 treats a false factual sentence as a defect. The rule still gives 32, because the largest wrapped input is 8 and 4 x 8 = 32, so the constant is not wrong under a Type 1 reading. But the margin is exactly the rule's factor rather than the 4.6x the prose implies. One open question decides the value. PS3.3 Table C.18.4-1 also allows a Type 3 nested Referenced SOP Sequence (presentation state), a Real World Value Mapping sequence and an Icon Image Sequence inside that same item. If those count, the input is 7, 9 wrapped, and the rule gives **64**. I measured that 64 meets the 2 MiB debug condition (all 52 part10 tests green, including AC3 on 256 KiB). Every template in this chain is marked "Type: Extensible" in 2026d, so the listed rows are a floor, not a ceiling.
**Evidence**: The template rows and macro tables were read now from dicom.nema.org (see the Evidence table). A pydicom model with the real tags gives 6 and 8, or 7 and 9 with the Type 3 sequence.
**Scope and regression**: This is prose on HEAD, and the value is unaffected under the Type 1 reading. Correct the three sentences to 6 and 8, or delete the per-input figures from the doc comment and leave them in the LLD and the handoff. **For the lead**: say whether Type 3 sequences in an IOD macro carried by a template's content item count toward "template shapes". If they do, the value is 64.

### D2, the "deepest measured shape" fixture is not the shape it names, and its test claims a limit it does not test

**Where**: `crates/ocelli-dicom/tests/part10.rs:714` to `727`, `deepest_measured_shape_data_set`. Also `:1364` to `1370`, `the_deepest_measured_shape_parses_at_the_limit` and its comment.
**What**: Three of the seven sequence tags are not the attributes the doc comment names. `(0040,A168)` is Concept Code Sequence, but Measured Value Sequence is `(0040,A300)`. `(0040,A170)` is Purpose of Reference Code Sequence, but Measurement Units Code Sequence is `(0040,08EA)`. `(0400,0562)` is Attribute Modification DateTime, a **DT** element, but Modified Attributes Sequence is `(0400,0550)`, by PS3.3 C.12.1.1.9's own table and by pydicom's dictionary. The doc comment cites "Modified Attributes Sequence (0400,0562) of C.12.1.1.9.1", which is a false citation. The SELECTED FROM IMAGE item holds a bare `(0008,1155)` without the Type 1 Referenced SOP Sequence that should hold it, so the fixture is one level short of the shape it names (7 against 8, see D1). The test name says "parses at the limit" and the shape is 7 against a limit of 32. Its comment says "One Sequence deeper is refused by refuses_one_past_the_sequence_depth_limit", which is false, because an 8-deep shape is accepted. That other test refuses 33.
**Why it is wrong**: HLD 27.2 R2 and microscope section 2 require a fixture to be what its citation says, and microscope section 3 applies to the two false sentences. The tags happen not to change the depth in Explicit VR, because the wire VR says SQ. But under Implicit VR, the dictionary VR of `(0400,0562)` is DT, so the same shape cannot be carried there, and the fixture cannot be reused for the AC4 form matrix it would naturally join.
**Evidence**: Dictionary lookup now: `0x4000562 AttributeModificationDateTime DT`, `0x4000550 ModifiedAttributesSequence SQ`, `0x40a168 ConceptCodeSequence`, `0x40a170 PurposeOfReferenceCodeSequence`, `0x4008ea MeasurementUnitsCodeSequence`, `0x40a300 MeasuredValueSequence`. The constant sweep shows that this test goes red only at 6 or below.
**Scope and regression**: Test-only, on HEAD. Use the real tags, wrap the image reference in `(0008,1199)`, and name the test for what it proves, for example "the deepest PS3.16 shape parses". The smallest regression is the same test with the corrected bytes, at depth 8.

## Smells

### S1, the odd-length deflate test ties its precondition to the value of the constant

**Where**: `crates/ocelli-dicom/tests/part10.rs:1311` to `1328`, `a_deflated_odd_length_stream_requires_and_accepts_its_null_pad`.
**What**: The test builds its stream from a chain `MAX_SEQUENCE_DEPTH` deep and asserts that the flate2 output length is odd. That parity depends on both the constant and the compressor's output. At a constant of 7 or 8 the stream is even, and the test fails on "the fixture stream is odd" with the parser behaving correctly. A flate2 or miniz_oxide bump can flip it the same way.
**Failure mechanism**: This is pass 1's S1 again, moved from a silent pass to a spurious failure. D1's open question may move the constant to 64, which happens to stay odd today, but any other change to the constant or to the compressor turns this into a red test unrelated to padding. It is also the only floor test that fails when the constant is set back to pass 1's rejected 8, so it gives a false impression that the rule's margin is guarded. The padding behaviour has nothing to do with depth, so give it a fixed depth, or add a pad element and adjust it until the stream is odd, independent of `MAX_SEQUENCE_DEPTH`.

## Nitpicks

- N1. `dicom-ingest.md:77` to `78`, "the key-object and secondary-capture writers and the synthetic slice writer reach 2 and 0". That is three writers and two numbers. They measure 2, 0 and 0.
- N2. No floor test enforces the rule's margin. The constant at 16 passes every test, and at 8 only S1's parity coincidence fails. The plan records AC9 in the handoff rather than in a test, so this does not block, but a test of the form `assert!(MAX_SEQUENCE_DEPTH >= 4 * DEEPEST_SHAPE_DEPTH)`, beside the corrected D2 fixture, would make the rule executable.
- N3. The GLM report's own Image Library figure of 4 and its "TID 320 ... deepest chain 5" are not in the tree, but they are the source of D1's sentences.

## Verified clean

- **Pass 1 D2 is closed.** M1 goes red on the sibling and many-items tests in every length form. The many-items helper's doc arithmetic holds: depth `1 + depth_per_item` is 32 when called with `MAX - 1`.
- **Pass 1 D3 is closed.** The AC10 test posts 20,000 levels through `handle_connection` on a `std::thread::spawn` thread (`one_exchange`, `preview.rs:331`), and the unchecked parser aborts the test process. With the check, it asserts `400` plus `X-Ocelli-Error: not-part10` and then a 200 on the next exchange. The new comment is true. N2 of pass 1, the direct `parse_part10` call, has been removed.
- **Pass 1 D1's measured inputs, except the template chain.** The corpus is 2, `writeTid1500` is 5, KOS is 2, SC is 0 and the Rust builders are 2 or less, all measured independently. `REPOSITORY_TID_1500` is real writer output: it is byte-length and structure identical to what today's worktree source writes, contains synthetic UIDs only and has no PatientName.
- **The 32 value and the stack conditions.** AC1 parses and projects at 32 on 2 MiB debug, and 64 and 128 do too. AC3 refuses 100,000 on 256 KiB, unaffected by the constant. The check placement is unchanged from pass 1's verified state.
- **Pass 1 S1's pad fix.** `refuses_deep_nesting_in_a_deflated_data_set` now pads an odd stream per PS3.5 A.5, and passes at every swept value from 8 upward.
- **Doc sentences executed.** In `use.html:531`, N1 of pass 1 is fixed and the sentence is true. In the LLD, the corpus 2, writer 5 and "value is 32" are true, the template 5 and "7 wrapped" are false (D1), "deeper than a quarter of the constant" (pass 1 N3) is fixed, and the verification paragraph at 388 to 391 is true. The `parse.rs` doc comment's rule sentence and the 2 MiB clause are true, and its measurement clause is false (D1). `README.md` and `more.html` name no `ParseError` variant and are not stale.
- **GLM's tree is complete.** The five files of the write set match the report's description: five new part10 tests, so 47 + 5 = 52, the preview test, the LLD, `use.html` and `parse.rs`. Its gate tails reproduce. The opencode exit 1 came after the report and left nothing half-written.

## Limitations and handoff

- Not exercised: Explicit VR Big Endian, the example server's wire path, release builds, `bin/ocelli.sh gate native` and `--sprint`. No GPU or oracle job was run.
- The "recorded in the F-234 handoff" sentence depends on `.claude/handoffs/F-234-ready.md`, which `/complete-feature --prepare` writes later. It does not exist yet.
- The Type 3 question in D1 is the lead's decision, not the implementer's. The rule as amended does not settle it.
- Next owner: the lead decides on D1's Type 3 question. Then the implementer fixes D1's sentences, rebuilds D2's fixture with the real tags and its correct test name, and makes S1's fixture independent of the constant. A fresh independent pass then reviews the changed material.
