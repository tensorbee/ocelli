# F-024 JPEG review, pass 4

**Reviewed**: full staged working tree on `work/f-024-codex` at base `586e503`,
all three earlier review reports, the pass-three remediation, and the F-019
encapsulated-frame boundary
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, permanent padding tests do not hold the necessary-NULL boundary

**Where**: `crates/ocelli-codec/tests/jpeg.rs:93` and
`crates/ocelli-codec/tests/jpeg.rs:282`

**What**: the production check correctly accepts one trailing NULL only when
the EOI offset is odd. The permanent acceptance test exercises only the
odd-length 93-byte lossless stream. The two trailing-data cases append a
nonzero byte or two zero bytes to another odd-length 93-byte stream. No
permanent test refuses one unnecessary NULL after an already even-length JPEG
codestream.

**Why it is wrong**: the approved plan and LLD promise the single trailing
NULL *needed* to make an odd stream even, while refusing other trailing data.
DICOM PS3.5 Annex A.4 requires even Fragment Item Values and permits padding
the final Fragment. One NULL after an already even codestream is not required
Item padding and is genuine trailing data. The untested branch is especially
important at the F-019 Basic and empty Basic Offset Table boundary, where the
codec receives the physical even-length Fragment Value and must distinguish
the one required pad from unrelated bytes.

**Evidence**: a fresh mutation changed the EOI rule from
`offset % 2 == 1 && trailing == [0]` to `trailing == [0]`. This deliberately
wrong implementation accepts one unnecessary NULL after any even-length
codestream. `bin/ocelli.sh test ocelli-codec` still exited 0 with all 25 tests
passing. After reverting the mutation, a temporary probe appended one NULL to
the even-length 936-byte baseline fixture and required `TrailingData` with an
unchanged caller buffer. The exact probe exited 0, proving that the current
implementation is correct and the defect is in permanent regression coverage.
The probe was removed.

## Smells

None.

## Nitpicks

None.

## Pass-three remediation proofs

- `inspect_jpeg` accepts no trailing bytes, or exactly one zero byte only when
  EOI ends at an odd offset. It rejects a nonzero trailing byte, excess zero
  bytes, and an unnecessary zero after an even-length codestream.
- The accepted external NULL is excluded from the `codestream` slice passed to
  the JPEG dependency. FF marker fill before EOI remains part of the JPEG
  codestream and is accepted.
- The permanent 93-byte lossless fixture proves exact one-pad acceptance and
  the same hand-computed decoded samples. Permanent cases prove non-padding and
  excess-padding refusal with caller-buffer atomicity. D1 records the missing
  permanent proof for an unnecessary single NULL.
- F-019's Basic and empty Basic Offset Table path preserves even physical
  Fragment Values, including any required Item pad. Its Extended Offset Table
  path uses the declared encoded length to remove exactly one physical pad.
  The focused F-019 frame-index suite exited 0 with all 9 tests passing.

## Earlier remediation and prose proofs

- The public `Decoder::decode` documentation accurately states atomic caller
  output, allocation-free registry dispatch, and D-21's bounded JPEG and JPEG
  2000 allocation exception.
- The approved plan and LLD identify `gate native` and the focused codec wasm
  target check as compilation evidence only. They do not claim browser runtime
  proof for F-024.
- The plan and LLD now describe both PS3.5 A.4 padding forms, removal of an
  external NULL before dependency decode, and refusal of excess or non-padding
  trailing data. These statements match the implementation outside D1's
  missing regression guard.
- Process selection, output-description reporting, conformance truth
  comparisons, atomic registration, dependency output checks, and
  caller-buffer atomicity remain correct on reinspection.

## Verified clean

- Read the approved plan, all earlier review reports, progress handoff,
  relevant HLD and LLD sections, all staged JPEG source and tests, and F-019's
  Basic and Extended Offset Table frame-index implementation and tests.
- The fresh necessity mutation described in D1 left the full 25-test codec
  suite green. The mutation was reverted. The temporary even-stream probe then
  exited 0 and was removed.
- After the revert, `bin/ocelli.sh test ocelli-codec` exited 0 with 2 unit
  tests, 9 JPEG tests, and 14 registry tests passing.
- `bin/ocelli.sh check ocelli-codec`, `bin/ocelli.sh clippy ocelli-codec`, and
  the direct `wasm32-unknown-unknown` codec check each exited 0.
- `bin/ocelli.sh gate native` exited 0 with native linkage, shared workspace
  wasm compilation, native all-target checks, and feature equality.
- `bin/ocelli.sh gate bench` exited 0 with 25 Python tests and 80 passing Node
  tests. Its one browser-only cold-start test was skipped by the declared gate
  design.
- `fmt`, `unsafe`, `provenance`, `prose`, `content`, `deviations`, and
  `bindgen` gates each exited 0 before this report was added.
- No repository `unsafe`, `wasm-bindgen`, patient data, render-loop work, or
  tier-specific arithmetic was added. Temporary probes and mutations were
  removed, and `git diff --cached --check` exited 0 before this report.
