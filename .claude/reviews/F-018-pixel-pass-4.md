# F-018 review, pass 4

**Reviewed**: full staged working tree against
`586e503496a1762f49651bc1411e18350ceda4a8`, all three earlier reviews, the
approved plan, and the staged pass-three remediation
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Pass-three remediation proof

- The permanent sentinel fixture supplies exactly 65,536 LUT Data entries for
  a zero first LUT Descriptor value. Only entry 65,535 is distinct, and stored
  input 65,535 maps to it. The same fixture separately refuses 65,535 entries.
- A fresh mutation changed the production sentinel from 65,536 to 65,535.
  `zero_descriptor_count_maps_exactly_65_536_entries_through_the_final_index`
  exited 101 at the fixture's construction assertion. Reverting the mutation
  made the same exact test exit 0.
- The fixture expectation is derived from DICOM PS3.3 C.11.1.1.1, which says a
  first descriptor value of zero represents `2^16` entries. It is not copied
  from implementation output.

## Earlier remediation proofs

- Independent arithmetic for the deterministic rounded oblique pair produced
  row length error `3.3943755761711714e-7`, column length error
  `8.590499622762593e-8`, absolute dot error
  `1.4810309999779836e-6`, and derived accumulated dot bound
  `1.7320515575688771e-6`. These remain below the reviewed `2e-6` input bound.
- `ImageOrientationPatient` rejects input outside that bound, then normalises
  the row and Gram-Schmidt orthogonalises the column before storage. The
  rounded-oblique fixture proves unit lengths and a dot product below `1e-12`.
- `ImagePlane::new` permits zero row spacing only with one row and zero column
  spacing only with one column. `index_to_world` substitutes unit extensions
  only on those unused axes, preserving valid voxel positions while keeping
  the transform invertible.
- LUT Data rejects non-finite, negative, fractional, and overflowing values.
  Eight-bit and sixteen-bit endpoints, first-input clamping, and Modality and
  VOI sequence precedence remain covered.
- The approved plan and LLD correctly name `gate native` and the direct pixel
  crate wasm target check as compile evidence. Dependency-tree inspection
  confirmed that `ocelli-wasm` depends on `ocelli-core`, not `ocelli-pixel`, so
  neither document incorrectly treats `gate wasm` as pixel-crate evidence.

## Verified clean

- Read all three earlier reviews, the approved plan, progress handoff,
  relevant HLD and LLD sections, all staged source and tests, the DICOM expert
  skill, and the normative PS3.3 image-plane, spacing, and LUT sections.
- The DICOM PS3.3 C.7.6.2.1.1 transform uses column spacing with the row
  direction cosine for column index, row spacing with the column direction
  cosine for row index, and Image Position Patient as the first voxel centre.
- Stored extraction masks Bits Stored and sign-extends from
  `BitsStored - 1`. It supports all declared container widths and byte orders,
  and checks source and destination lengths before caller output mutation.
- `LINEAR` uses `c - 0.5` and `w - 1`. `LINEAR_EXACT` uses neither adjustment.
  Both use `<=` at the lower bound and `>` at the upper bound. `SIGMOID` uses
  exponent `-4 * (x - c) / w`, and all width constraints remain correct.
- After the sentinel mutation was reverted, `bin/ocelli.sh test ocelli-pixel`
  exited 0 with 10 unit, 4 image-plane, 5 modality, 2 stored-pixel, and 5 VOI
  tests passing.
- `bin/ocelli.sh check ocelli-pixel`, `bin/ocelli.sh clippy ocelli-pixel`, and
  the direct `wasm32-unknown-unknown` pixel-crate check each exited 0.
- `bin/ocelli.sh gate native` exited 0 with native linkage, shared-crate wasm
  compilation, native all-target checks, and feature equality.
- `fmt`, `unsafe`, `provenance`, `prose`, `content`, `deviations`, `bindgen`,
  `nostd`, `pins`, and `errors` gates each exited 0 before this report.
- No `unsafe`, `wasm-bindgen`, render-loop allocation, GPU submission, trait,
  generic abstraction, dynamic dispatch, or tier-specific arithmetic copy was
  added. The mutation was removed, the unstaged implementation diff was empty,
  and `git diff --cached --check` exited 0 before this report.
