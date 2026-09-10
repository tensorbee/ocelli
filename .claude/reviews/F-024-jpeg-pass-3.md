# F-024 JPEG review, pass 3

**Reviewed**: full staged working tree on `work/f-024-codex` at base `586e503`,
the two pass-two remediations, and the F-019 encapsulated-frame boundary
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, legal encapsulated Item padding after JPEG EOI is rejected as trailing data

**Where**: `crates/ocelli-codec/src/jpeg.rs:269`,
`crates/ocelli-codec/tests/jpeg.rs:269`, and `docs/lld/codecs.md:111`

**What**: `inspect_jpeg` requires the EOI marker to consume the final byte of
the supplied source. Both lossless fixtures are odd-length 93-byte JPEG
streams. Appending the one zero byte needed to carry either stream in an
even-length encapsulated DICOM Item makes the decoder return `TrailingData`.
The existing trailing-data test encodes that legal case as a refusal, and the
LLD claims that no bytes may follow EOI.

**Why it is wrong**: DICOM PS3.5 Annex A.4 requires every Fragment Item Value
to have even length and explicitly permits the final Fragment to be padded.
Its note says JPEG padding may be appended after EOI. A decoder that receives a
physical Fragment Value must distinguish the single required Item pad from
unrelated trailing bytes.

F-019 can remove the pad when an Extended Offset Table Length gives the odd
encoded length. Basic Offset Tables provide offsets but no encoded lengths, so
`EncapsulatedFrameIndex::from_basic` necessarily returns the physical even
Fragment Values. The current F-024 contract therefore rejects a legal frame on
the Basic or empty Basic Offset Table path.

**Evidence**: `wc -c` reported 93 bytes for both lossless fixtures. A temporary
probe appended one zero byte to `jpeg_lossless_process14_12bit.jpg`, decoded it
with the existing 12-bit description, and required the hand-computed output.
`bin/ocelli.sh test ocelli-codec
review_probe_accepts_required_even_item_padding_after_odd_jpeg_stream --
--exact` exited 101 with `left: Err(TrailingData)` and `right: Ok(())`. The
probe was removed. Inspection of F-019 found that `from_extended` slices a
Fragment to its declared encoded length, while `from_basic` preserves the
physical Fragment slices.

## Smells

None.

## Nitpicks

None.

## Pass-two remediation proofs

- The public `Decoder::decode` documentation now states atomic caller output,
  allocation-free registry dispatch, and D-21's bounded JPEG and JPEG 2000
  allocation exception. It matches the implementation and deviation register.
- The approved plan now identifies `gate native` and the focused codec wasm
  target check as compilation evidence only. The LLD explicitly states that
  adapter fixtures execute natively, `bin/ocelli.sh wasm` does not compile the
  codec, and F-024 has no wasm browser execution proof.

## Verified clean

- Read the approved plan, both earlier review reports, progress handoff,
  relevant HLD and LLD sections, all staged JPEG source and tests, and F-019's
  Basic and Extended Offset Table frame-index behavior.
- A fresh mutation removed eight-bit precision from `.51` validation.
  `extended_process_2_eight_bit_decodes_against_independent_dcmtk_truth`
  exited 101 with `FrameMismatch`. Reverting the mutation made the same exact
  test exit 0.
- `bin/ocelli.sh test ocelli-codec` exited 0 with 2 unit tests, 8 JPEG tests,
  and 14 registry tests passing. D1 explains why the existing padding-shaped
  trailing-data case is a false-positive test rather than clean evidence.
- `bin/ocelli.sh check ocelli-codec`, `bin/ocelli.sh clippy ocelli-codec`, and
  the direct `wasm32-unknown-unknown` codec check each exited 0.
- `bin/ocelli.sh gate native` exited 0 with native linkage, shared workspace
  wasm compilation, native all-target checks, and feature equality.
- `bin/ocelli.sh gate bench` exited 0 with 25 Python tests and 80 passing Node
  tests. Its one browser-only cold-start test was skipped by the declared gate
  design.
- `fmt`, `unsafe`, `provenance`, `prose`, `content`, `deviations`, and
  `bindgen` gates each exited 0 before this report was added.
- Exact UID process selection, output-description reporting, conformance truth
  comparisons, atomic registration, dependency output checks, and caller-buffer
  atomicity were re-inspected and remain correct outside D1.
- No repository `unsafe`, `wasm-bindgen`, patient data, render-loop work, or
  tier-specific arithmetic was added. Temporary probes and mutations were
  removed, and `git diff --cached --check` exited 0 before this report.
