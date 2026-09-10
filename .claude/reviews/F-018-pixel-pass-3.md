# F-018 review, pass 3

**Reviewed**: full staged working tree against
`586e503496a1762f49651bc1411e18350ceda4a8`, both earlier reviews, the approved
plan, and every staged pass-two remediation
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the LUT zero-count sentinel has no permanent regression guard

**Where**: `crates/ocelli-pixel/src/lut.rs:29` and
`crates/ocelli-pixel/tests/modality.rs:52`

**What**: `LutDescriptor::new` correctly interprets a descriptor entry count
of zero as 65,536 entries. No permanent test constructs that sentinel case.
The descriptor-length tests and all modality fixtures use nonzero entry
counts, so an off-by-one implementation remains green.

**Why it is wrong**: DICOM PS3.3 C.11.1.1.1 states that the first LUT
Descriptor value shall be zero when the table contains `2^16` entries. The
LLD explicitly claims that the constructor validates the zero-means-65,536
entry count. Under the microscope rule, a normative arithmetic branch that a
meaningful wrong mutation can change without making its claimed evidence red
is a defect.

Normative text:
<https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.html#sect_C.11.1.1.1>

**Evidence**: a fresh mutation changed the sentinel from 65,536 to 65,535.
`bin/ocelli.sh test ocelli-pixel` still exited 0 with all 25 tests passing.
After reverting it, a temporary exact probe supplied 65,536 entries, placed a
distinct value at index 65,535, and mapped stored input 65,535 to that final
entry. `review_probe_zero_descriptor_count_means_exactly_65536_entries`
exited 0. The probe was removed. The current implementation is correct and the
defect is the absent permanent regression evidence.

## Smells

None.

## Nitpicks

None.

## Pass-two remediation proofs

- Independent arithmetic for the deterministic rounded oblique pair produced
  row length error `3.39437557617117e-7`, column length error
  `8.59049962276259e-8`, and absolute dot error
  `1.48103099997798e-6`. The derived dot bound is
  `1.73205155756888e-6`, so all are below the reviewed `2e-6` input bound.
- Accepted vectors are normalised, then the column is Gram-Schmidt
  orthogonalised against the normalised row before storage. A fresh arithmetic
  mutation retained normalization but removed orthogonalisation. The rounded
  oblique fixture exited 101 on its `dot.abs() < 1e-12` assertion. Reverting
  the mutation restored the exact test and full suite to exit 0.
- `PixelSpacing` admits finite nonnegative values provisionally.
  `ImagePlane::new` permits a zero row value only with one row and a zero
  column value only with one column. Private spacing and dimension fields keep
  later mutation from bypassing this contextual check.
- `index_to_world` substitutes a unit row or column direction only for a legal
  zero singleton-axis spacing. A temporary one-by-one probe with both spacing
  values zero observed unit x, y, and normal extensions and an exact origin
  round trip through the inverse. It exited 0 and was removed.
- The direct `wasm32-unknown-unknown` pixel-crate check and `gate native` both
  exited 0. Dependency-tree inspection confirmed that `ocelli-wasm` depends
  on `ocelli-core`, not `ocelli-pixel`, so the plan and LLD correctly avoid
  treating `gate wasm` as pixel-crate execution evidence.

## Verified clean

- Read both earlier reviews, the approved plan, progress handoff, relevant HLD
  and LLD sections, all staged source and tests, the DICOM expert skill, and
  the current normative PS3.3 image-plane, spacing, and LUT sections.
- DICOM PS3.3 C.7.6.2.1.1 mapping is correct. Column index uses column spacing
  with the row direction cosine, row index uses row spacing with the column
  direction cosine, and Image Position Patient is the first voxel centre.
- Stored extraction supports 8, 16, and 32-bit containers in both byte orders.
  It masks Bits Stored, sign-extends from `BitsStored - 1`, and checks source
  and destination lengths before writing caller output.
- LUT Data refuses non-finite, negative, fractional, and values above the
  declared unsigned 8-bit or 16-bit range. First mapped input validation,
  clamped lookup, Modality LUT precedence, and VOI LUT precedence agree with
  PS3.3 C.11. D1 is the only missing permanent LUT constraint proof found.
- `LINEAR` uses `c - 0.5` and `w - 1`. `LINEAR_EXACT` uses neither adjustment.
  Both use `<=` at the lower bound and `>` at the upper bound. `SIGMOID` uses
  exponent `-4 * (x - c) / w`. Their width constraints are correct.
- After all mutations and probes were removed, `bin/ocelli.sh test
  ocelli-pixel` exited 0 with 10 unit, 4 image-plane, 4 modality, 2 stored-pixel,
  and 5 VOI tests passing.
- `bin/ocelli.sh check ocelli-pixel`, `bin/ocelli.sh clippy ocelli-pixel`, and
  the direct pixel-crate wasm target check each exited 0.
- `bin/ocelli.sh gate native` exited 0 with native linkage, shared-crate wasm
  compilation, native all-target checks, and feature equality.
- `fmt`, `unsafe`, `provenance`, `prose`, `content`, `deviations`, `bindgen`,
  `nostd`, `pins`, and `errors` gates exited 0 before this report was added.
- No new `unsafe`, `wasm-bindgen`, render-loop allocation, GPU submission,
  trait, generic abstraction, dynamic dispatch, or tier-specific arithmetic
  copy appears in the staged diff. Temporary probes and mutations were
  removed, the unstaged implementation diff was empty, and
  `git diff --cached --check` exited 0 before this report.
