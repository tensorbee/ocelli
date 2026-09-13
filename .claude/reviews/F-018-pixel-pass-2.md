# F-018 review, pass 2

**Reviewed**: full staged working tree against
`586e503496a1762f49651bc1411e18350ceda4a8`, the approved F-018 plan, pass-one
review, and every staged remediation
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, the orientation tolerance rejects the decimal rounding it claims to admit

**Where**: `crates/ocelli-pixel/src/image_plane.rs:10` and
`docs/lld/pixel-pipeline.md:52`

**What**: `ImageOrientationPatient::new` rejects an otherwise usable pair of
conceptual orthonormal direction cosines after each component is rounded to six
decimal places. The LLD and test comment explicitly claim that the `1e-6`
dimensionless threshold admits this encoding noise.

**Why it is wrong**: Image Orientation Patient has DS value representation.
DICOM PS3.5 section 6.2 permits decimal strings up to 16 bytes, while PS3.3
C.7.6.2.1.1 defines the values as normal, mutually orthogonal vectors. A
practical validation boundary that promises to admit sixth-place decimal
rounding must account for error accumulated across all three dot-product
terms. Applying the component-scale threshold directly to the final dot
product rejects conforming conceptual geometry solely because of its legal
decimal representation. Any tolerance change remains a reviewed plan decision.

**Evidence**: A temporary fixture used row
`[0.800831, 0.390592, 0.453990]` and column
`[0.114807, -0.844119, 0.523720]` and required construction to succeed.
`bin/ocelli.sh test ocelli-pixel
review_probe_accepts_six_decimal_orthonormal_orientation -- --exact` exited
101. Independent arithmetic gave row length error `3.394376e-7`, column length
error `8.590500e-8`, and dot error `1.481031e-6`. The probe was removed.

### D2, legal single-row or single-column zero spacing is unrepresentable

**Where**: `crates/ocelli-pixel/src/image_plane.rs:68` and
`.claude/plans/F-018-design.md:126`

**What**: `PixelSpacing::new` rejects zero for either component under every
image dimension. The approved plan and LLD likewise state an unconditional
positive-spacing rule. The separately constructed spacing type cannot examine
the image dimensions when applying the exception.

**Why it is wrong**: DICOM PS3.3 section 10.7.1.3 requires positive nonzero
pixel spacing except when there is only one row or column or pixel. The spacing
corresponding to that singleton dimension may be zero. Rejecting it makes legal
single-row, single-column, and single-voxel evidence impossible to construct.

**Evidence**: The existing `malformed_plane_attributes_are_refused` unit test
passes because `PixelSpacing::new(0.0, 1.0)` is rejected without dimensions.
Inspection of `ImagePlane::new` found no later conditional validation against
`ImageDimensions`. PS3.3 section 10.7.1.3 states the singleton exception
explicitly.

### D3, the cross-target test row names commands that do not prove its claim

**Where**: `.claude/plans/F-018-design.md:168`

**What**: The plan says `bin/ocelli.sh check ocelli-pixel` and
`bin/ocelli.sh wasm` prove the pixel modules compile on native and wasm. The
first command is a host check. The second builds `ocelli-wasm`, whose current
dependency graph does not include `ocelli-pixel`.

**Why it is wrong**: The approved test matrix must identify executable evidence
for its claim. The repository's `gate native` is the actual shared-crate wasm
compile proof, and the focused target command used during this review proves
the pixel crate directly.

**Evidence**: `bin/ocelli.sh cargo check -p ocelli-pixel --all-targets
--target wasm32-unknown-unknown` exited 0. `bin/ocelli.sh gate native` exited 0
and compiled `ocelli-pixel` during its shared workspace wasm step. Inspection
of the `wasm` arm found only `wasm-pack build crates/ocelli-wasm`.

## Smells

None.

## Nitpicks

None.

## Pass-one remediation proofs

- Non-unit and skewed direction vectors are now refused. Accepted vectors are
  normalised and Gram-Schmidt orthogonalised before transform construction, so
  accepted input error does not become scale or shear. D1 records the overly
  narrow acceptance boundary and its false prose claim.
- LUT Data now rejects non-finite, negative, fractional, and overflowing entry
  values. Valid zero and maximum endpoints for both 8-bit and 16-bit LUTs pass.
- The approved plan and LLD now state the two pass-one invariants. The LUT
  statements match the implementation and DICOM PS3.3 C.11.

## Verified clean

- Read the pass-one review, approved plan, progress handoff, relevant HLD and
  LLD sections, and every staged source and test file for image plane, stored
  pixels, modality LUT, and VOI LUT behavior.
- A fresh LUT boundary mutation changed exact-key selection from `<=` to `<`.
  `modality_lut_sequence_takes_precedence_over_rescale` exited 101 because
  input zero selected the preceding entry. Reverting the mutation made the
  same exact test exit 0.
- `bin/ocelli.sh test ocelli-pixel` exited 0 with 24 tests passing.
- `bin/ocelli.sh check ocelli-pixel`, `bin/ocelli.sh clippy ocelli-pixel`, and
  the direct `wasm32-unknown-unknown` pixel check each exited 0.
- `bin/ocelli.sh gate native` exited 0 with native linkage, shared-crate wasm
  compilation, native all-target checks, and cross-target feature equality.
- `fmt`, `unsafe`, `provenance`, `prose`, `content`, `pins`, `deviations`,
  `bindgen`, and `nostd` gates each exited 0 before this report was added.
- Stored extraction masks Bits Stored and sign-extends from `BitsStored - 1`.
  Both byte orders and every supported container width are exercised. Source
  and destination length errors occur before output mutation.
- Modality LUT presence wins over rescale. VOI LUT presence wins over window
  evidence. Exact LUT inputs, below-range inputs, and above-range inputs map to
  the specified entries.
- `LINEAR` uses `c - 0.5` and `w - 1`. `LINEAR_EXACT` uses neither adjustment.
  Both functions implement the lower `<=` and upper `>` boundaries. SIGMOID
  uses exponent `-4 * (x - c) / w`.
- No new `unsafe`, `wasm-bindgen`, render-loop allocation, GPU submission,
  trait, generic abstraction, dynamic dispatch, or tier-specific arithmetic
  copy appears in the staged diff.
- The temporary probes and mutation were removed. The pre-report unstaged diff
  was empty and `git diff --cached --check` exited 0.
