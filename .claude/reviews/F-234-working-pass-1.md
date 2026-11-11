# F-234 review, pass 1

**Reviewed**: `git diff 3ee216d9 2fb0073b` on `work/f-234-xai`, HEAD `2fb0073ba1975457af5fec5585152d7fdd31df4d`, tree `849f59946dff406f3f87db961ed7893582f3273b`. No unstaged paths before or after the pass, apart from this report.
**Author / reviewer**: implementation by GLM-5.3 in opencode (commit 2fb0073b, per the caller). Reviewer is a Claude subagent spawned by the lead, independent of the implementation. The reviewer did not write the plan or the code.
**Result**: 3 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **AC9 and decision 2, the value of the constant.** Derived independently: the deepest nesting of the repository's own DICOM writer output, and the nesting PS3.16 lets a TID 1500 document reach. Rejected: "8 is four times the deepest input".
- **AC2 and AC6, depth rather than count.** Derived independently: sibling sequences and many items each nested to the limit must parse. Mutation M1 checks whether any floor test catches a counter that never decrements.
- **AC3, refused before recursion.** Checked on Explicit and Implicit VR LE, undefined lengths, 100,000 deep, on a 256 KiB thread. The shipped test covers only the defined-length explicit form.
- **AC10, the folder viewer process stays alive.** A 20,000 deep file was posted through `handle_connection` on a `std::thread::spawn` thread (the stack `serve.rs` uses), on the fixed parser and on the parser with the check removed.
- **AC5, deflate.** The fixture was re-run with other constants to see whether it depends on the value of the constant.
- **AC8, no file data.** The wire body of the refusal was checked.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree at 2fb0073b | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-dicom -p ocelli-folder-viewer` | same | now | 0, part10 47 passed, folder viewer 72 + 60 + 1 + 1 + 1 + 24 |
| `cargo test -p ocelli-dicom --test corpus -- --ignored` | scratch copy `target/f234-review/src` from `git archive HEAD`, main checkout's `corpus/data` symlinked, its own target dir | now | 4 passed |
| `python3 scripts/guard_census.py`, `--check-runbook` | worktree | now | 0 and 0, "1134 refusal(s) in 79 file(s), all claimed" |
| `python3 scripts/prose_check.py` | worktree | now | 0, "voice rules clean over 890 file(s)" |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, "clean over 1757 files" |
| pydicom depth scan of every file under `corpus/data` | 104 files, main checkout | now | deepest 2, `synthetic/ct_multiframe_perframe.dcm` (SharedFunctionalGroupsSequence > PlaneOrientationSequence). GLM's corpus figure is confirmed |
| `writeTid1500` from `packages/core/dist/tid1500.js`, one NUM root, scanned with pydicom | main checkout dist, same `tid1500.ts` source as the worktree (diffed) | now | depth **5**: ContentSequence > ContentSequence > ContentSequence > MeasuredValueSequence > MeasurementUnitsCodeSequence |
| Scratch test `f234_review.rs`, `parse_part10` then `MetadataSet::from_object` on 2 MiB | the TID 1500 above, a PS3.16-shaped extension (depth 6), both wrapped in Original Attributes Sequence > Modified Attributes Sequence (7 and 8), 20 sibling top-level SQ each 8 deep (defined and undefined), 50 items each 8 deep in implicit VR | now | all `Ok(true)` |
| Same test, 100,000 deep undefined-length, explicit and implicit, 256 KiB | raw bytes, pydicom file meta | now | `Err(SequenceTooDeep)` both |
| Mutation M1, `sequence_depth -= 1` deleted | scratch copy | now | `cargo test -p ocelli-dicom` all green (part10 47 passed). Every review case above becomes `SequenceTooDeep`, including the repo's own TID 1500. Only the ignored corpus tests (3 of 4 red) and a corpus-gated folder viewer unit test catch it |
| Mutation M2, constant set to 32 and to 64 | scratch copy | now | 64: part10 47 passed on 2 MiB debug. 32: 46 passed, `refuses_deep_nesting_in_a_deflated_data_set` red with `InvalidDataSet` (see S1) |
| Mutation M4, the depth check deleted | scratch copy | now | `refuses_deep_nesting_without_recursing` aborts, "thread has overflowed its stack". The AC10 test with its direct `parse_part10` assertion removed still goes red, on "the deep file was not refused by the preview" |
| 20,000 deep file posted to `/preview` through `handle_connection` | scratch folder viewer test | now | fixed parser: `400`, `X-Ocelli-Error: not-part10`, body "the file is not a DICOM Part 10 object this preview can read", then the next exchange answers. Check deleted: SIGABRT, test process ends |
| Constant swept 9, 16, 31, 33, 40 against the deflate fixture | scratch copy | now | 9, 16, 40 pass. 31, 33 fail `InvalidDataSet` |

Every scratch copy, generated file and scratch target dir was deleted. Tree after the pass: HEAD and `git write-tree` unchanged, `git status` empty apart from this report.

## Defects

### D1, `MAX_SEQUENCE_DEPTH = 8` breaks the plan's rule and leaves no margin for valid SR

**Where**: `crates/ocelli-dicom/src/parse.rs:44`, and the rule sentences at `parse.rs:39` to `42` and in `docs/lld/dicom-ingest.md`, "Sequence depth bound", second paragraph.
**What**: Decision 2 says the value is "a power of two at least four times the deepest nesting found in any corpus row or tracked test fixture". GLM measured 2. It took that from the corpus and from reading the Rust `*b"SQ"` builders. It missed the deepest DICOM fixture the repository tracks, which is the TID 1500 document `packages/core/src/tid1500.test.ts` writes and reads back through `writeTid1500`. That document nests **5** deep. Four times 5 is 20, so the rule gives 32, not 8. The corpus has no SR, SEG, RTSTRUCT, RT Plan or KOS row, so its 2 says nothing about structured documents.
**Why it is wrong**: The plan's own AC11 column rejects "a limit below a real file", and PS3.5 section 7.5 sets no nesting bound. A document shaped by PS3.16 reaches the limit with no room left. A NUM in TID 300 has the INFERRED FROM SCOORD row, and that SCOORD has SELECTED FROM IMAGE. Under TID 1501 that gives depth 6. If that document is de-identified with its original values kept, PS3.3 C.12.1.1.9's Original Attributes Sequence > Modified Attributes Sequence holds the prior Content Sequence, and the depth becomes **8**, exactly the limit. One more legitimate level is refused, for example a Comprehensive SR section or a modifier under a finding site. This is from my reading of PS3.16 and PS3.3 and has not been checked against the current edition. The refusal reaches real consumers in this sprint. F-215 makes the folder viewer recognise SR and KOS documents through `inspect_json`, which calls `parse_part10` (`preview.rs` 2229 and 2361). A refused document is then listed as a file that fails rather than as marks.
**Evidence**: The pydicom scan rows above show the repo's TID 1500 at 5, the PS3.16-shaped one at 6 and the de-identified one at 8, all `Ok` at 8 today. M2 shows a larger constant fits the 2 MiB debug condition: 64 passes all 47 part10 tests, and GLM measured the debug overflow between 216 and 224.
**Scope and regression**: Present on HEAD. The inputs to the rule need a lead amendment. Count the deepest output of the repository's own writers (`tid1500.ts`, `tid2010.ts`, `secondary-capture.ts`) and the template depth PS3.16 permits for the SR documents the folder viewer reads, alongside the corpus. Then the implementer measures and re-derives the value. The smallest regression is a tracked fixture at the repo's TID 1500 nesting, plus the de-identification wrapper, that must parse. The LLD and doc-comment sentences that state the rule are false until then.

### D2, nothing in the floor suite distinguishes depth from a sequence count

**Where**: `crates/ocelli-dicom/tests/part10.rs`, `nested_sequence_data_set` and every depth test. Each one builds a single chain with one sequence per level.
**What**: Deleting `sequence_depth -= 1` at `parse.rs:434` turns the limit into a cap of eight sequences per file. That refuses the repository's own TID 1500, every Enhanced multi-frame file with more than eight functional group sequences, and most SR. All 47 part10 tests, the whole `ocelli-dicom` suite and the folder viewer tests stay green without the corpus.
**Why it is wrong**: Decision 1 defines depth as the count of *open* Sequence containers. Microscope section 2 says a test that passes both ways is a defect. AC2's "wrong implementation rejected" column names miscounting and is only half covered.
**Evidence**: Mutation M1 in the evidence table. Only `bin/ocelli.sh gate corpus`, which CI's floor does not run, goes red.
**Scope and regression**: Present on HEAD. Add a fixture with `MAX_SEQUENCE_DEPTH + 1` sibling top-level sequences, each nested to the limit, in both length forms, and one with many items in one sequence each nested to the limit, all expecting `Ok`. Or let the proptest generate trees with siblings rather than chains. Either goes red under M1.

### D3, the AC10 test cannot observe the failure it says it rejects

**Where**: `examples/folder-viewer/tests/preview.rs`, `a_sequence_one_past_the_limit_is_refused_and_the_process_answers_next` and its comment.
**What**: The comment says "A refusal that recursed would end the process under panic = abort and this test would die with it." At `MAX_SEQUENCE_DEPTH + 1` = 9 levels, dicom-rs's recursion does not come close to overflowing. GLM measured the overflow above 216 on 2 MiB, and GLM's own RED record says the old parser *parsed* this file. So the sentence is false, and the aliveness half of the test passes with or without the fix. AC10's column names "A refusal that ends the process".
**Why it is wrong**: This is a false claim in prose, and it is also a test that cannot fail for the reason it is named after (microscope sections 2 and 3). The plan's problem statement is that a deep file ends the folder viewer's process.
**Evidence**: A 20,000 deep file through `handle_connection` on a spawned thread gives SIGABRT with the check removed, and on the fixed tree gives `400 not-part10` followed by a served next exchange. Under M4 the shipped test goes red only on the 400 check, never on aliveness.
**Scope and regression**: Test-only, present on HEAD. Post a file deep enough to overflow an unchecked parser, as AC3 does, and then the next request. Delete the false sentence or make it true.

## Smells

### S1, the deflate fixture omits the PS3.5 A.5 pad and is valid only by coincidence

**Where**: `crates/ocelli-dicom/tests/part10.rs`, `refuses_deep_nesting_in_a_deflated_data_set`.
**What**: The encoded stream goes into `part10` unpadded. PS3.5 A.5 requires a trailing NULL when the deflated length is odd, and the parser refuses an unpadded odd stream (`deflated_data_set_requires_null_padding_when_the_stream_length_is_odd`). At 8 the stream happens to be even. With the constant at 31, 32 or 33 the test returns `InvalidDataSet` and goes red for a reason unrelated to depth.
**Failure mechanism**: D1's remediation changes the constant, and this test will then fail spuriously, or pass for the wrong reason if the expectation is loosened. Pad the stream as the file's other deflate fixtures do.

## Nitpicks

- N1. `use.html` line 531: the new sentence sits between the variant list and "A caller can log it without logging patient data", so "it" now reads as the data set rather than `ParseError`. Naming `ParseError` again would fix it.
- N2. The folder viewer test asserts the variant's `Display` through a direct `parse_part10` call. That duplicates AC8 and is not the process path. The wire assertion stands on its own (M4).
- N3. `dicom-ingest.md`, "A corpus row or fixture that nests deeper returns the story to the lead" does not say deeper than what. It presumably means deeper than a quarter of the constant.

## The plan-versus-tree contradiction GLM reported

GLM is right. `part10_error` at `examples/folder-viewer/src/preview.rs:1106` maps every `ParseError` to `PreviewError::NotPart10`, deliberately and with a comment saying so. That file is outside F-234's write set. The plan's consumer-map row "the file refused with the variant's message" and AC10's "refused with the variant's message" do not describe the tree. The wire answer is the fixed body "the file is not a DICOM Part 10 object this preview can read" with `X-Ocelli-Error: not-part10`, and it carries no file data, so AC8's safety property holds on the wire. This is a plan defect for the lead to amend, with AC10's observable becoming that header and body followed by a served next request. It is not an implementation defect. Whether a valid but deep file should be called "not a DICOM Part 10 object" is a wording question for the lead, not for this story.

## GLM's five open decisions

1. The value 8: D1.
2. Independent per-level Sequence and Item length forms in the proptest: correct. PS3.5 7.5.1 and 7.5.2 allow all four combinations.
3. AC1 and AC2 looped over four forms: acceptable.
4. A US element in the innermost item: acceptable.
5. AC10's message asserted at the parse boundary: see the contradiction above and N2.

## Verified clean

- The check sits at the `SequenceStart` push in the non-recursive preflight. That is before `collect_to_object` / `collect_sequence` in dicom-object 0.10.0 (collector.rs 1075, 1099, 1277). `LazyDataSetReaderOptions` exposes only `odd_length` and `charset_override`, as the LLD says.
- The refusal is exact: `>=` before the increment gives `Ok` at 8 and `SequenceTooDeep` at 9, in all four length combinations of both LE syntaxes and in the proptest. The decrement happens only on the Sequence end path and cannot underflow. Items and pixel items are not counted. Siblings and many items at the limit parse (the review fixtures).
- AC3 holds at 100,000 deep on 256 KiB for undefined lengths in both syntaxes, beyond the shipped defined-length explicit case. M4 shows the shipped AC3 test aborts without the check.
- Deflate: the check runs on the inflated data set, because `validate_data_set_structure` receives the adapted bytes.
- AC8: the variant is unit-like and `Copy`, its message is fixed, and the folder viewer's wire body is fixed text.
- A 20,000 deep file through the folder viewer's handler is refused and the next exchange is served.
- `example-server/src/reply.rs` catches the new variant with a wildcard arm. `DicomwebSource` forwards it unchanged.
- Doc sentences executed: the LLD parse-order step 5 sentence, the "Sequence depth bound" first paragraph, the error table row and the verification paragraph are true. The second paragraph's rule sentence is false under D1, and "recorded in the F-234 handoff" depends on `.claude/handoffs/F-234-ready.md`, which does not exist yet. The `use.html` sentence is true. `README.md` and `more.html` list no `ParseError` variants and are not stale.
- An explicit-VR UN element of undefined length, nested deep, is refused as `TruncatedDataSet` before any recursion, so it cannot route around the depth check. That refusal predates this story, and I did not confirm my UN fixture against PS3.5 6.2.2.

## Limitations and handoff

- Not exercised: Explicit VR Big Endian nesting, the example server's wire path, release builds, and `bin/ocelli.sh gate native` and `--sprint`. No GPU or oracle job was run, as instructed.
- The PS3.16 and PS3.3 C.12.1.1.9 shapes in D1 come from my reading and were not checked against the current edition. The repo-writer depth of 5 is measured.
- Next owner: the lead amends decision 2's inputs and AC10's observable. Then the implementer fixes D1 to D3 and S1, and a fresh independent pass reviews the changed material.
