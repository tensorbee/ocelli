# F-023 review, pass 3

**Reviewed**: staged tree based on `4e59cf22c6e1e307d69fb44bb01875b9b9e99803`, including both prior review remediations and the complete F-023 diff
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-1 High Bit behavior remains corrected. The public descriptor uses the
  PS3.3 C.7.6.3.3 equality, the positive 12-bit case uses High Bit 11, and
  lower and higher mismatches are covered. The isolated mutation that removed
  the equality refusal made
  `high_bit_must_equal_bits_stored_minus_one` fail with the invalid descriptor
  returned as `Ok`.
- Pass-1 catalogue and validation coverage remains corrected. Empty and
  repeated known catalogues have independent inputs. The frame refusal matrix
  covers zero dimensions, zero samples per pixel, unsupported Bits Allocated,
  zero and excessive Bits Stored, and High Bit on both sides of the required
  value. The prior isolated mutations of catalogue checks and zero-column
  validation each made their targeted tests fail.
- Pass-2 dimension behavior is corrected. Rows and Columns are `u16` in both
  `FrameDescInput` and `FrameDesc`, matching their DICOM PS3.6 `US` value
  representations. In an isolated compile probe, assigning `65_536` to Rows
  failed with `literal out of range for u16`, so the public API can no longer
  certify the non-DICOM value accepted in pass 2.
- `checked_output_len` is private and has exactly two concrete integer-width
  instantiations in the staged source. Production uses `usize`. The host unit
  test uses `u32`, which has the same maximum as wasm32 `usize`, and proves
  that `65_535 * 65_535 * 1 * 4` is refused while the one-byte form is the
  representable value `4_294_836_225`.
- The checked-length evidence is mutation-sensitive. In an isolated clone,
  changing the helper to convert `length / 4` made
  `registry::tests::conforming_dicom_dimensions_can_overflow_a_32_bit_output_length`
  fail. The observed value was `Some(4_294_836_225)` where the test required
  `None`.
- `bin/ocelli.sh test ocelli-codec` passed 14 tests, two crate unit tests and
  12 registry integration tests. `bin/ocelli.sh check ocelli-codec`,
  `bin/ocelli.sh clippy ocelli-codec`, and `bin/ocelli.sh fmt --check` passed.
- `bin/ocelli.sh cargo check -p ocelli-codec --target
  wasm32-unknown-unknown --all-targets` passed. This directly compiles the
  production `usize` helper instantiation for wasm32 and the shared registry
  API for that target.
- The exact Transfer Syntax catalogue still matches all 16 distinct UIDs in
  `corpus/manifest.tsv`, with no UID present on only one side. Capability
  remains an exact string lookup with distinct available, known unavailable,
  and unknown states.
- Atomic registration still preflights an entire decoder declaration before
  insertion. The collision and common-prefix fallback mutations recorded in
  pass 1 remain applicable because those implementation paths and targeted
  tests are unchanged. Registration behavior remains consistent with
  deviation D-19.
- `bin/ocelli.sh gate nostd` reported exactly 7 no-std crates.
  `python3 scripts/guard_census.py` reported 790 refusals in 65 files, 434
  probes, and 0 refusals watched by nothing. The changed
  `EXPECTED_NO_STD_CRATES` constant matches its recorded digest in
  `ci/guard-probe-budget.json`.
- `bin/ocelli.sh gate bench` passed with 11 subjects, one current subject
  naming no blocking story, and one recorded baseline. The plan and LLD keep
  `decode.frame` unavailable without fabricating a dispatch-only measurement.
- `bin/ocelli.sh cargo tree -p ocelli-codec -e normal` showed only the crate
  itself. The changed Rust contains no `as` cast, unsafe code, concrete codec,
  pixel-value arithmetic, `wgpu`, `wasm-bindgen`, or inventory registration.
  The HLD section 21 `Decoder` trait and `Arc<dyn Decoder>` remain the declared
  structural exceptions.
- The full staged diff, both earlier review artifacts, approved plan, LLD,
  no-std guard, and recorded constant update were re-read together.
  `git diff --cached --check`, `python3 scripts/prose_check.py`, and
  `python3 scripts/source_provenance_check.py` passed before this report was
  added.
