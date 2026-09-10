# F-024 JPEG review, pass 5

**Reviewed**: full staged working tree on `work/f-024-codex` at base
`586e503`, all four earlier review reports, and the pass-four regression
remediation
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Pass-four remediation proof

- The permanent
  `unnecessary_null_after_even_length_codestream_is_trailing_data` regression
  uses the 936-byte baseline stream whose EOI ends at offset 936. Appending one
  NULL makes 937 bytes, so the byte is not padding needed to make the stream
  even. The test requires `TrailingData` and verifies that every caller-buffer
  byte retains its sentinel value.
- The exact regression first exited 0 against the staged implementation. A
  fresh mutation removed only `offset % 2 == 1` from the trailing-NULL guard.
  The exact regression then exited 101 with `left: Ok(())` and
  `right: Err(TrailingData)`. Restoring the predicate left no unstaged diff,
  and the same exact regression exited 0 again.
- The 93-byte lossless stream has EOI ending at offset 93. One trailing NULL
  makes the Fragment Value 94 bytes, and one FF fill byte before EOI also makes
  EOI end at offset 94. The permanent acceptance fixture decodes both forms to
  the same four hand-computed twelve-bit samples.
- Current DICOM PS3.5 2026c A.4 requires every Fragment Item Value to be even,
  permits padding the last Fragment only when necessary, and permits JPEG fill
  before EOI or padding after EOI. PS3.5 6.2 defines the necessary OB padding
  as one trailing NULL byte. The implementation accepts exactly that external
  form and excludes the NULL from the dependency codestream slice.

## Earlier remediation and behavior proofs

- JPEG Extended `.51` accepts SOF1 process 2 at eight bits and process 4 at
  twelve bits. Baseline, extended, process-14, and SV1 fixtures retain their
  independent or hand-computed comparisons.
- The four-UID registration helper preflights every exact UID before its first
  insertion. `.70` still requires predictor selection value 1 in every scan.
- The public allocation contract, D-21, the plan, and the codec LLD consistently
  describe bounded dependency output allocation and the owned encoded-input
  copy on the twelve-bit `.51` route. Registry dispatch and raw plus RLE paths
  remain allocation-free per call.
- Output Photometric Interpretation is reported separately as `Rgb` or
  `Preserved`, and the prose accurately leaves consumption as future work.
- Cross-target prose names `gate native` and the focused codec wasm target
  check as compilation evidence only. It does not claim browser execution or
  treat the unrelated `ocelli-wasm` package build as codec evidence.
- JPEG header, process, dimensions, components, precision, dependency layout,
  output length, truncation, trailing data, and dependency failure checks all
  precede the caller-buffer copy. Refusal paths inspected in this pass remain
  atomic.
- The release benchmark times 31 individual decode calls after one warm-up.
  Process startup, fixture setup, decoder construction, and caller output
  allocation remain outside the timer. Dependency work inside `decode` remains
  measured, matching the plan, baseline record, runner, and benchmark LLD.
- The exact dependency graph and source-policy entries remain consistent.
  Fresh whole-word inspection found eleven `unsafe` sites in
  `oxideav-core` 0.1.35, four unsafe implementations and seven unsafe blocks,
  all under arena support. `oxideav-mjpeg` 0.1.8 contained no match.
- No repository `unsafe`, `wasm-bindgen`, patient data, render-loop work,
  tier-specific decode arithmetic, new trait, or new generic was added.

## Verified clean

- Read the approved plan, all four earlier reviews, the progress handoff,
  relevant HLD and LLD sections, every staged source and prose diff, and the
  metadata, lengths, hashes, and marker boundaries of all eight fixture files.
- `bin/ocelli.sh fmt`, `bin/ocelli.sh check ocelli-codec`,
  `bin/ocelli.sh test ocelli-codec`, and
  `bin/ocelli.sh clippy ocelli-codec` executed in one guarded command and
  exited 0. The codec suite passed 2 unit tests, 10 JPEG tests, and 14 registry
  tests, 26 tests total.
- `bin/ocelli.sh gate native` exited 0. Its four proofs covered native entry
  points, shared-crate wasm compilation, native all-target checks, and equal
  feature resolution across targets.
- `bin/ocelli.sh cargo check -p ocelli-codec --all-targets --target
  wasm32-unknown-unknown` exited 0.
- `bin/ocelli.sh gate bench unsafe provenance prose content pins deviations
  bindgen errors` exited 0 with nine gates. Bench passed 25 Python tests and 80
  Node tests, with its one declared browser-only test skipped. The unsafe gate
  checked 73 files with two permitted, provenance checked 596 files, prose
  checked 250 files, and the error gate's 15 tests passed.
- `git diff --cached --check` and `git diff --check` exited 0 before this report
  was added. The mutation was reverted and no implementation or canonical
  sprint state was changed by this review.
