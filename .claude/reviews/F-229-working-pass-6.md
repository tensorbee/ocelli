# F-229 review, pass 6

**Reviewed**: working tree of `work/f-229-xai` in `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-229` against the pass 5 checkpoint `9c53913`. `git diff 9c53913 -- . ':!tools/oracle/out' | shasum -a 256` = `e1f3645cbd327b93c09793b3b3f9c5aec9cc674dd3efab9666d3bc8f4a2d8325`. Index tree `ecc282f6ed93a701d0cb036e27887dac3918979c`, which is `9c53913^{tree}`, so nothing is staged. One modified path, `crates/ocelli-codec/tests/jpegxl.rs`, 51 insertions and 5 deletions. `git diff --stat 9c53913 -- crates/ocelli-codec/src` is empty, so no source file changed. Untracked and not reviewed: the pass 1 to 5 reports and `tools/oracle/out`. The hash and the index tree were the same at the start and the end of the pass. Library half only.
**Author / reviewer**: remediation by GLM-5.3 in opencode (`glm-F-229-fix6.log`). Reviewed by an independent Claude reviewer that wrote none of this material and none of passes 1 to 5.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 5 S1 replay.** In a scratch copy, `check_header(&codestream)?` at `jpegxl_icc.rs:150` became `if container_tail.is_none() { check_header(&codestream)?; }`, so the pre-check runs for bare codestreams only. `declared_icc_refuses_unsupported_before_any_parse` goes red with `left: Err(InvalidCodestream)`, `right: Err(UnsupportedPixelFormat)`, and it is the only test red (28 of 29 pass, the scan skipped). Since the test stops at its first failure, the exact `jxlc` block was then deleted in the scratch copy and the test went red again on the `jxlp` case alone, so both new container cases go red by kind, as the comment says. With the source restored the test is green.
- **Peak resident set**, `/usr/bin/time -l` on a release probe binary calling `JpegXlDecoder::general().decode`, built from a scratch copy whose `src/` was `diff -r` identical to the worktree. The crafted stream bare, in an exact `jxlc` box (46 bytes), in `jxlp` boxes of 7 bytes (86 bytes) and in a to-end `jxlc` (46 bytes): `UnsupportedPixelFormat` at 2,260,992, 2,277,376, 2,277,376 and 2,277,376 bytes. Under the bare-only mutation: bare unchanged, and the three containers `InvalidCodestream` at 270,843,904, 270,843,904 and 270,827,520 bytes. Both decoders gave the same kinds case by case, with the output buffer untouched.
- **The mechanism the new comment names.** Ocelli's `read_header` calls `uninit.try_init()` (`jpegxl.rs` around `:317`), which for `want_icc()` calls `jxl_color::icc::read_icc`, whose `encoded_icc.resize(enc_size as usize, 0)` (`jxl-color-0.11.0/src/icc/decode.rs:66`) admits the crafted `1 << 28`. The 270 MB figure above is that resize. Only the MIT or Apache-2.0 registry sources of `jxl-oxide` 0.12.6 and `jxl-color` 0.11.0 were read.
- **Pass 5 D1 replay, each rule deleted alone in a scratch copy**, then the `jpegxl` suite without the scan, plus a probe of five containers:

| Rule deleted | Tests red | Probe |
|--------------|-----------|-------|
| two `jxlc` | `containers_that_break_the_extraction_rules_refuse` and `two_jxlc_boxes_refuse`, each `left: Ok(())` | the `duplicate` container and the two-halves container both decode, and both match `jpegxl_reference_rgb8.raw` |
| no codestream | none, 29 passed | the `ftyp`-only container and a signature-only container still refuse `InvalidCodestream`. An instrumented copy shows `check_header` returning `Err(InvalidCodestream)` on `len=0` for both |
| index order | `jxlp_index_order_refuses_out_of_order_indexes` only, `left: Ok(())` | the `out_of_order` case still refuses, through the no-last-flag rule |
| no last flag | `containers_that_break_the_extraction_rules_refuse` only, `left: Ok(())` | `out_of_order` still refuses, through the index rule |
| `jxlc` after `jxlp` | `jxlc_after_a_final_jxlp_refuses` only | |
| `jxlp` after `jxlc` | `jxlp_after_a_jxlc_refuses` only | |
| `jxlp` after the final one | `jxlp_after_the_final_jxlp_refuses` only | |

  Every sentence in the rewritten comment holds against that table. The duplicate case detects the two-`jxlc` rule because the walk then concatenates the two copies and the decode succeeds. The `out_of_order` case cannot tell the index rule from the no-last-flag rule, because neither piece is flagged last, and the index rule's discriminating case is `jxlp_index_order_refuses_out_of_order_indexes`. The no-codestream rule's removal cannot be seen by kind, because `check_header` refuses the empty codestream with the same `InvalidCodestream`. The new test's comment holds too. Its two boxes reach no `jxlp` check, a codestream is present, and with the rule deleted the halves decode.
- **The sentences the round kept or changed in the 46 byte case.** That case still refuses `InvalidCodestream` under the bare-only mutation, so "the kind here is the walk's" holds. With the pre-check intact, the bare and both exact containers refuse `UnsupportedPixelFormat`, so the edited "the bare stream and the exact-size containers above are the pre-check's detectors" holds.
- **Prose.** None of the added lines has an em-dash, an en-dash or a semicolon. `prose_check.py` is clean.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo clean -p ocelli-codec`, then `cargo fmt --all --check` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0 and 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | worktree | now | 0, no warning |
| `cargo test -p ocelli-codec -p ocelli-pixel -p ocelli-dicom -p ocelli-viewport` | worktree, after the clean, run twice | now | 0, 506 passed, 0 failed, `jpegxl` 29 passed in 158.43 s |
| `bin/ocelli.sh gate licences`, `corpus`, `corpus-tests`, `guards` | worktree | now | each 0, ALL GREEN |
| `guard_census.py`, `guard_census.py --check-runbook` | worktree | now | 0 and 0 |
| `prose_check.py`, `source_provenance_check.py`, `deviation_check.py` | worktree | now | 0, 864 files, 1,710 files, 28 deviations |
| S1 mutation and the single-case follow-up | scratch copy, `--test jpegxl` | now | red as tabled, restored and `diff`ed clean |
| Seven rule deletions and the five-container probe | scratch copy | now | table above, restored and `diff`ed clean |
| `/usr/bin/time -l` over 4 inputs, tree and mutation | scratch release example | now | 2.26 to 2.28 MB on the tree, 270.8 MB under the mutation |
| Implementer's red runs and verification tails | `glm-F-229-fix6.log` | prior receipt | consistent with mine, 29 `jpegxl` tests |

Environment: macOS 27.0.1, no GPU or browser job, no oracle, no wasm32 execution. The scratch copy had its own target directory under `target/f229-p6-scratch`.

## Defects

None.

## Smells

None. The pass 5 S1 gap is closed: the container route now has two detectors of the pre-check's removal, and each goes red alone.

## Nitpicks

- **N1.** `jpegxl.rs:851` to `:852`: "the rule's case only it catches is `two_jxlc_boxes_refuse` below" reads as though that test were the only such case. The `duplicate` case in the same test is also refused by the two-`jxlc` rule alone, and every deletion in the table turns both red or neither. The new test adds a named, isolated detector in the pass 4 shape but no extra discrimination. The sentence is not false, because the test is such a case.
- **N2.** `jpegxl.rs:856` to `:859`: "so the case here pins the walk's own refusal" is loose. The case pins the refusal kind, and it stays green with the walk's rule deleted, as the clause before it says. "pins only the kind" would say the same thing without the ambiguity.
- **N3.** Pass 5 N1 to N3 and pass 4 N1, N2 and N4 are carried unchanged, since no source file changed this round.

## Verified clean

- **Pass 5 S1 closed.** The exact `jxlc` and `jxlp` cases detect a container-only skip of the pre-check by kind. On the tree, all three container forms refuse `UnsupportedPixelFormat` near 2.3 MB.
- **Pass 5 D1 closed.** The false sentence is gone, and each sentence that replaced it was executed by deleting the rule it names.
- **No source change**, no new `as` cast, no `unsafe`, no `#[allow]`. The new test uses the existing helpers and `assert_refused`, which also checks that the output buffer is untouched.
- **Source policy.** No dwv, Horos or Grok source was opened.

## Limitations and handoff

- **Carried, not this round's.** A bare codestream with trailing bytes decodes, and so does a `jxlc` codestream followed by a second complete copy once the two-`jxlc` rule is removed. Both are within the design's "when detectable".
- **Not exercised**: the oracle, any GPU or browser job, a wasm32 execution, `bin/ocelli.sh wasm` and `gate pins` (not asked this pass, and no manifest, lock or notice changed since `9c53913`).
- **Carried from pass 1, outside this half**: `tools/oracle/unsupported.json` (AC24), `docs/SOURCE-POLICY.md` rows and the AC25 audit, `docs/lld/codecs.md`, `README.md`, `use.html` and `more.html`, the viewer registration, AC20.
- **Scratch**: one tree copy, a probe test, a release example, crafted inputs and their target directory under `target/f229-p6-scratch`, deleted at the end with this pass's logs. This report is the only file written in the worktree.
- **Result**: this is the first clean pass of the library half. That is a bounded claim about this tree, not acceptance of F-229.
