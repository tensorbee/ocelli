# F-023 review, pass 1

**Reviewed**: `4e59cf22c6e1e307d69fb44bb01875b9b9e99803` through the complete working tree, including modified and untracked files
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, FrameDesc accepts a High Bit forbidden by the Image Pixel Description Macro

**Where**: `crates/ocelli-codec/src/registry.rs:140` and `crates/ocelli-codec/tests/registry.rs:102`

**What**: `FrameDesc::new` accepts any stored-bit window that fits within Bits
Allocated. The positive test declares 12 stored bits with High Bit 13 and calls
that descriptor valid.

**Why it is wrong**: DICOM PS3.3 C.7.6.3.3, Image Pixel Description Macro,
requires High Bit (0028,0102) to be one less than Bits Stored (0028,0101).
For 12 stored bits the only conforming High Bit is 11. Accepting 13 makes the
validated type certify a non-conforming pixel description and teaches the
positive test the same wrong rule.

**Evidence**: The current 2026c normative PS3.3 C.7.6.3.3 table says, "High
Bit (0028,0102) shall be one less than Bits Stored (0028,0101)." Inspection of
the constructor condition shows that `bits_allocated=16`, `bits_stored=12`,
and `high_bit=13` passes. `bin/ocelli.sh test ocelli-codec` confirms the test
carrying that tuple passes as written.

### D2, Known-catalogue validation has no test that reaches either refusal

**Where**: `crates/ocelli-codec/src/registry.rs:368`

**What**: `Registry::with_known` documents and implements refusal of empty and
repeated catalogue entries, but no test supplies either invalid catalogue.
The similarly named registration test exercises decoder declarations only.

**Why it is wrong**: HLD section 27.2 R2 requires tests to be independent
evidence. The microscope rule treats a refusal branch that nothing executes as
a defect because a green suite cannot show that the public constructor keeps
its documented contract.

**Evidence**: In an isolated clone, both `with_known` checks were replaced by
an unconditional `known_set.insert(*uid)`. `bin/ocelli.sh test ocelli-codec`
still reported 11 passed and 0 failed.

### D3, Four FrameDesc refusal branches are not exercised

**Where**: `crates/ocelli-codec/src/registry.rs:123` and `crates/ocelli-codec/tests/registry.rs:261`

**What**: The validation test covers zero rows, unsupported Bits Allocated,
Bits Stored above Bits Allocated, and a High Bit below the stored window. It
does not cover zero columns, zero samples per pixel, zero Bits Stored, or High
Bit at or beyond Bits Allocated. The target-width output overflow branch also
has no 32-bit test.

**Why it is wrong**: These are public validation promises in the plan, LLD,
API documentation, and error enum. HLD section 27.2 R2 and the microscope
mutation rule require evidence that each refusal is live rather than source
that merely looks authoritative.

**Evidence**: In an isolated clone, the zero-columns and zero-samples checks
were removed together. `bin/ocelli.sh test ocelli-codec` still reported 11
passed and 0 failed. The remaining named cases are absent from
`crates/ocelli-codec/tests/registry.rs` by direct search.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `bin/ocelli.sh check ocelli-codec` passed.
- `bin/ocelli.sh test ocelli-codec` passed 11 tests, including the crate unit
  test and 10 registry integration tests.
- `bin/ocelli.sh clippy ocelli-codec` and `bin/ocelli.sh fmt --check` passed.
- `bin/ocelli.sh wasm` completed the release wasm build and optimization.
- `python3 scripts/no_std_check.py` and `bin/ocelli.sh gate nostd` both
  reported exactly 7 no-std crates.
- `python3 scripts/guard_census.py` reported 790 refusals in 65 files, 434
  probes, and 0 refusals watched by nothing. Changing only
  `EXPECTED_NO_STD_CRATES` in an isolated clone made the census fail with the
  expected recorded-versus-found digest mismatch.
- `bin/ocelli.sh gate bench` passed. Its registry reported 11 subjects and one
  recorded baseline. An isolated backlog transition of F-023 to done made
  `bin/ocelli.sh bench --list` report `decode.frame` as an existing subject
  while no runner file exists, matching the documented future `no_runner`
  boundary.
- The implementation catalogue and `corpus/manifest.tsv` contain the same 16
  distinct Transfer Syntax UIDs with no entry present on only one side. The
  sampled Deflated, JPEG-LS, and HTJ2K values agree with DICOM PS3.6 Annex A.
- The required collision mutation was observed red. Removing the existing-map
  preflight made
  `registration_order_cannot_replace_a_successful_mapping` fail because the
  second registration returned `Ok(())`.
- The required fallback mutation was observed red. Returning the first decoder
  after an exact-UID miss made
  `common_prefix_is_not_a_transfer_syntax_fallback` fail because lookup
  returned `Ok(())` instead of `UnknownTransferSyntax`.
- Exact capability lookup, atomic multi-UID registration, output-length
  rejection before decoder invocation, borrowed dispatch inputs, and decoder
  error propagation were inspected against the approved plan and observed in
  the passing focused tests.
- The changed Rust contains no `as` cast, `unsafe`, `wgpu`, `wasm-bindgen`,
  inventory registration, pixel-value arithmetic, or render-loop path. The
  only new trait and `Arc<dyn Decoder>` are the HLD section 21 extension-point
  exception named by `AGENTS.md`.
- `git diff --check`, `python3 scripts/prose_check.py`, and
  `python3 scripts/source_provenance_check.py` passed before this report was
  added.
