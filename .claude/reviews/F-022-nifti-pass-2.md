# F-022 microscope review, pass 2

**Reviewed**: `81577024886c1815c100a0bf4cb136d294b6addb` through exact staged tree `fc9e315f305b6dafcfc5ff611e7493ebf39a261c`

**Result**: 0 defects, 0 smells, 0 nitpicks

## Pass 1 remediation

### D1. Exact `vox_offset` conversion can discard significant high bits

Verified clean. `exact_u64_from_f32` now accepts either signed zero as the exact integer zero before applying the negative-value refusal. For left shifts, it computes `u64::MAX >> shift` and refuses any significand above that bound before shifting. This distinguishes the greatest representable f32 below `2^64`, `0x5f7f_ffff`, from exact `2^64`, `0x5f80_0000`, and larger values. Production then performs the separate checked conversion from `u64` to target `usize`.

The direct unit test covers negative zero, the greatest f32 below `2^64`, exact `2^64`, and the next f32 above it. The public parser test confirms that negative zero reaches the product-scope below-352 refusal and that both high-boundary cases report `ArithmeticOverflow`.

Controlled mutations proved both sides of the boundary. Changing the significand comparison from `>` to `>=` made the greatest representable value fail. Bypassing the comparison made exact `2^64` wrap to zero and fail the overflow assertion. Refusing sign-negative zero made the signed-zero assertion fail.

### D2. The qform tests do not exercise the mixed quaternion terms

Verified clean. The new fixture uses `a = b = c = d = 1/2`, unequal voxel spacings, negative qfac, and a nonzero translation. Independent calculation gives the RAS rotation rows `[0,0,1]`, `[1,0,0]`, `[0,1,0]`, scaled rows `[0,0,-4]`, `[2,0,0]`, `[0,3,0]`, and LPS points `(-11,13,17)`, `(-11,11,17)`, `(-11,13,20)`, and `(-7,13,17)` for the origin and three basis points. These are the fixture's expected values.

Each sign in `r12`, `r13`, `r21`, `r23`, `r31`, and `r32` was flipped independently in the disposable exact-index checkout. Every mutation made `qform_mixed_terms_rotate_about_a_non_axis_direction` fail, either at affine validation or at an expected coordinate. This proves all mixed quaternion terms are live today.

### D3. No test proves byte-swapped NIfTI-2 discriminator recognition

Verified clean. The envelope fixture now writes big-endian `540` at bytes 0 through 3 with the normative NIfTI-2 single-file magic and requires `Nifti2Unsupported`. Removing the byte-swapped size branch made the named envelope test fail with `TruncatedHeader` instead. Detection remains ahead of the NIfTI-1 minimum-length and byte-order checks.

## Complete surface verified clean

- The complete staged parser, public exports, dependencies, integration tests, approved plan, LLD, build-target documentation, and status-row changes were reviewed. The pass-1 report remains preserved in the staged tree.
- Header size, endian and version discrimination, magic, declared dimensions, datatype and bitpix pairing, units, scaling retention, qform and sform selection, qfac, selected-only affine validation, RAS-to-LPS conversion, payload gaps, trailing bytes, and structural error safety remain consistent with the approved plan and official NIfTI header definitions.
- Checked payload multiplication still includes all three dimensions and the datatype width. The private length helper remains instantiated as `usize` in production and `u32` in an executed host overflow test.
- The affine is still constructed in `glam` column order and converted by left multiplication with `diag(-1,-1,1,1)`. No tolerance changed.
- Positive fixtures assert success before extracting the successful variant. The new qform fixture therefore cannot pass silently on an error result.
- There is still no NIfTI I/O, decompression, `Volume`, target-specific branch, `unsafe`, or `wasm-bindgen`. Native and wasm resolve the same direct dependency set.
- Public errors retain only structural enum variants and no input bytes, paths, header values, identifiers, or free text.
- The plan and LLD truthfully describe the exact offset conversion, parser boundary, NIfTI-2 discriminator, affine behavior, and target posture.

## Executed evidence

- Exact index before this review: `fc9e315f305b6dafcfc5ff611e7493ebf39a261c`.
- `bin/ocelli.sh test ocelli-dicom`: pass. Eight library tests, 11 DICOMweb tests, 18 metadata tests, 16 NIfTI tests, and 41 Part 10 tests passed. The one corpus test remained explicitly ignored for the dedicated corpus gate.
- `bin/ocelli.sh check ocelli-dicom`: pass.
- `bin/ocelli.sh clippy ocelli-dicom`: pass.
- `bin/ocelli.sh fmt --check`: pass.
- `bin/ocelli.sh native`: pass through all four steps. Fourteen direct dependencies resolved identically for native and `wasm32-unknown-unknown`.
- `bin/ocelli.sh wasm`: pass, including release compilation and `wasm-opt`.
- `python3 scripts/prose_check.py`: pass across 236 files.
- `bin/ocelli.sh gate content`: pass.
- `git diff --cached --check` and `git diff --check`: pass.
- Nine controlled mutations in the disposable exact-index checkout were detected: overflow-boundary rejection, overflow-check bypass, signed-zero rejection, six independent quaternion off-diagonal sign flips, and removal of byte-swapped NIfTI-2 recognition.
- No canonical implementation file was modified during review.
