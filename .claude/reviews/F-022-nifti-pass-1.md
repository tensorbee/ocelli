# F-022 microscope review, pass 1

**Reviewed**: `81577024886c1815c100a0bf4cb136d294b6addb` through exact staged tree `be2ef60e54f5a51c5e540d2812478430b4b04d0d`

**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1. Exact `vox_offset` conversion can discard significant high bits

**Where**: `crates/ocelli-dicom/src/nifti.rs`, `exact_u64_from_f32`

The exponent-at-least-23 branch uses `u64::checked_shl`, which checks only whether the shift count is smaller than 64. It does not report significant bits shifted out of the result. An exact NIfTI-1 `vox_offset` of `2^64`, encoded by `f32::from_bits(0x5f80_0000)`, therefore computes `0x0080_0000_u64.checked_shl(41) == Some(0)`. The parser returns `VoxelOffsetBeforeData` instead of the required `ArithmeticOverflow`. The adjacent representability case `f32::from_bits(0x5f80_0001)` likewise produces a truncated low value and reaches payload validation instead of reporting overflow. The staged `f32::MAX` test detects only the separate shift-count-overflow path.

A disposable exact-index test also showed that `-0.0` is rejected as `InvalidVoxelOffset` because the sign check precedes the zero and minimum-offset check. This is inconsistent with treating the exact numeric zero as a below-data offset, although the discarded-high-bits case alone is blocking.

**Why it matters**: The approved plan requires an exact, representable integer conversion before payload addressing. Silent wraparound can select a false payload offset and misclassifies arithmetic overflow as an unrelated structural error.

**Evidence**: In a disposable checkout of the staged index, direct tests for `f32::from_bits(0x5f80_0000)` and `f32::from_bits(0x5f80_0001)` failed with `VoxelOffsetBeforeData` and `TruncatedPayload`, respectively, rather than `ArithmeticOverflow`. The exact boundary `16_777_216.0` continued to convert and reached `TruncatedPayload`, confirming the probe exercised conversion rather than a blanket rejection.

### D2. The qform tests do not exercise the mixed quaternion terms

**Where**: `crates/ocelli-dicom/tests/nifti.rs`, qform fixtures and assertions

The staged non-identity qform fixture sets `b = 0` and `c = 0`, with only `d` nonzero. It therefore exercises the `r12`, `r21`, `r11`, and `r22` plane but leaves `r13`, `r23`, `r31`, and `r32` insensitive to sign or term errors. There is no fixture with mixed nonzero `b` and `c` signs.

**Why it matters**: The qform expansion is a sign-sensitive implementation of the normative quaternion equations. A regression in an unexercised term can rotate or reflect voxel geometry while all staged tests pass.

**Evidence**: Changing `r12` from `2bc - 2ad` to `2bc + 2ad` failed the named qform test, showing that the probe is mutation-sensitive where the fixture has coverage. After restoring that expression, changing `r13` from `2bd + 2ac` to `2bd - 2ac` left the entire NIfTI integration suite green. All 16 tests in the disposable checkout passed, including the additional offset probe test.

### D3. No test proves byte-swapped NIfTI-2 discriminator recognition

**Where**: `crates/ocelli-dicom/tests/nifti.rs`, envelope refusal coverage

The implementation deliberately recognizes both little-endian and byte-swapped NIfTI-2 header sizes before refusing version 2, but the staged tests cover only the little-endian `540` discriminator. Removing the byte-swapped `540` branch leaves the full NIfTI suite green.

**Why it matters**: Detection order is part of the F-022 contract. A byte-swapped NIfTI-2 header must be identified as version 2 rather than falling through to an invalid-size error. Without a direct assertion, the documented two-byte-order discriminator can regress silently.

**Evidence**: In the disposable exact-index checkout, changing the version check from `little_size == 540 || big_size == 540` to `little_size == 540` left all 16 NIfTI tests passing.

## Verified clean

- Compared the parser's NIfTI-1 field offsets, `n+1` magic, datatype codes, units, quaternion equations, qfac behavior, and RAS coordinate convention against the official `nifti1.h`. Compared the 540-byte, 8-byte-magic NIfTI-2 discriminator against the official `nifti2.h`.
- Endian detection precedes version and magic handling. A mutation forcing the swapped rank probe false made `envelope_refusals_are_distinct` fail with `InvalidHeaderSize` instead of `BigEndianUnsupported`.
- Rank, positive spatial dimensions, singleton declared higher dimensions, datatype and bitpix pairing, finite positive spatial pixdims, millimetre spatial units, raw scaling retention, qform and sform code handling, and selected-only affine validation match the approved plan.
- Sform rows are placed correctly into `glam` columns. Sform precedence, left-multiplied RAS-to-LPS conversion, finiteness, and determinant rejection are implemented directly. Mutating the first RAS-to-LPS diagonal sign made the non-symmetric sform test fail.
- Qfac accepts exact positive or negative one, treats either signed zero as effective positive one, and refuses other values. Raw qform and sform values remain retained while only the selected transform is constructed and validated.
- Payload size uses checked multiplication across every declared dimension and the datatype byte width. Changing the product to ignore the third dimension made the property test fail with minimal dimensions `1 x 1 x 2`.
- Exact payload extent is required while a legal header-to-payload gap and trailing bytes are accepted. Extension payloads are refused. Error variants carry no input strings or input bytes.
- The checked output-size helper is instantiated as both `usize` in production and `u32` in a host test. This satisfies the repository rule that a generic must have two current instantiations.
- Positive tests assert success before their variant extraction, so their `let-else` branches cannot silently pass a wrong result variant.
- The staged implementation adds no I/O, compression, `Volume`, `unsafe`, `wasm-bindgen`, or target-specific behavior. Direct native and wasm dependency sets remain identical.
- The plan, LLD index, build-target documentation, crate exports, and dependency declarations otherwise describe the implemented scope accurately.

## Executed evidence

- `bin/ocelli.sh test ocelli-dicom`: pass, 92 tests passed and 1 declared corpus test ignored. The NIfTI integration target passed 15 tests.
- `bin/ocelli.sh check ocelli-dicom`: pass.
- `bin/ocelli.sh clippy ocelli-dicom`: pass.
- `bin/ocelli.sh fmt --check`: pass.
- `bin/ocelli.sh native`: pass, all four steps. Fourteen direct dependencies were identical for native and `wasm32-unknown-unknown`.
- `bin/ocelli.sh wasm`: pass, including the release `wasm-pack` build and size check.
- `python3 scripts/prose_check.py`: pass across 235 files.
- `bin/ocelli.sh gate content`: pass.
- `git diff --cached --check`: pass.
- Root independently completed the formerly unavailable Playwright oracle gate: 223 unit tests, 219 passed and 4 declared skipped, across 99 views. Candidate and reference SHA-256 values were identical, the gate passed, and all 29 controlled mutations were detected.
- Review mutations were made only in a disposable checkout populated from the exact staged index. No implementation file in the canonical worktree was modified.
