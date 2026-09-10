# F-023 review, pass 2

**Reviewed**: staged tree based on `4e59cf22c6e1e307d69fb44bb01875b9b9e99803`, including every pass-1 remediation and the full F-023 diff
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Rows and Columns were widened beyond their DICOM value representation

**Where**: `crates/ocelli-codec/src/registry.rs:66`, `crates/ocelli-codec/src/registry.rs:68`, and `crates/ocelli-codec/tests/registry.rs:368`

**What**: The pass-1 remediation changed `FrameDescInput::rows` and `columns`
from `u16` to `u32`. The overflow test then uses `u32::MAX` for both fields.
This makes the validated DICOM frame description accept dimension values above
65,535 solely to reach `OutputLengthOverflow` on a 64-bit test host.

**Why it is wrong**: DICOM PS3.6 defines Rows (0028,0010) and Columns
(0028,0011) with VR `US`, so both are unsigned 16-bit attributes. The
`FrameDesc` and LLD claim to validate DICOM Image Pixel fields. Widening the
public type certifies values that cannot be represented by those DICOM
attributes. A target-width overflow is reachable with conforming `u16`
dimensions on `wasm32`. Its test evidence must not broaden the input contract.

**Evidence**: The current 2026c PS3.6 data dictionary reports `US`, VM 1 for
both tags. In an isolated clone, a test constructed `FrameDesc` with
`rows=65_536`, `columns=1`, and otherwise valid 8-bit monochrome fields.
`bin/ocelli.sh test ocelli-codec accepts_dimension_outside_dicom_us -- --exact`
passed, proving that the validator accepts a row count outside the DICOM VR.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-1 D1 is otherwise corrected. `FrameDesc::new` now requires High Bit to
  equal Bits Stored minus one, the positive 12-bit fixture uses High Bit 11,
  and both lower and higher mismatches are refused. Removing the equality
  check in an isolated clone made
  `high_bit_must_equal_bits_stored_minus_one` fail with the non-conforming
  descriptor returned as `Ok`.
- Pass-1 D2 is corrected. `known_catalogue_refuses_empty_and_repeated_entries`
  independently supplies both invalid catalogue shapes. Replacing both
  `with_known` refusals with unconditional insertion made that targeted test
  fail on the empty entry before it reached the repeated entry.
- Pass-1 D3 is corrected apart from the dimension-width defect above. The
  negative matrix now covers zero rows, zero columns, zero samples per pixel,
  unsupported Bits Allocated, zero and excessive Bits Stored, High Bit below
  and above the required value, High Bit outside the container, and checked
  output multiplication. Removing the zero-columns refusal made the targeted
  validation test fail with an accepted zero-length frame.
- `bin/ocelli.sh test ocelli-codec` passed 13 tests, one crate unit test and 12
  registry integration tests. `bin/ocelli.sh check ocelli-codec`,
  `bin/ocelli.sh clippy ocelli-codec`, and `bin/ocelli.sh fmt --check` passed.
- `bin/ocelli.sh cargo check -p ocelli-codec --target
  wasm32-unknown-unknown --all-targets` passed, directly proving the registry
  and its tests compile for the wasm target.
- The earlier collision and exact-UID fallback mutation evidence remains
  applicable because those implementations and tests did not change in the
  remediation. The full diff was re-read and the exact map lookup, capability
  distinction, preflight-before-insert order, and caller-buffer dispatch remain
  consistent with the approved design and deviation D-19.
- `python3 scripts/no_std_check.py` reported exactly 7 no-std crates.
  `python3 scripts/guard_census.py` reported 790 refusals in 65 files, 434
  probes, and 0 refusals watched by nothing. The recorded no-std constant
  digest remains consistent with the seven-crate set.
- `bin/ocelli.sh gate bench` passed with 11 subjects, one subject currently
  naming no blocking story, and one recorded baseline. The `decode.frame`
  documentation still accurately describes its transition to `no_runner`
  only after F-023 is marked done.
- The 16 exact Transfer Syntax UIDs, the absence of concrete codecs, the
  setup-only allocations, the native and wasm sharing contract, and the
  worker-tier boundary remain unchanged from pass 1 and consistent with the
  corpus, PS3.6 Annex A, and the cited HLD sections.
- The changed Rust contains no `as` cast, `unsafe`, `wgpu`, `wasm-bindgen`,
  inventory registration, pixel-value arithmetic, or additional extension
  point. The `Decoder` trait and `Arc<dyn Decoder>` remain the explicit HLD
  section 21 structural exception.
- `git diff --cached --check`, `python3 scripts/prose_check.py`, and
  `python3 scripts/source_provenance_check.py` passed before this report was
  added.
