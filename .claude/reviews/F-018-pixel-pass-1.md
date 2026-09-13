# F-018 review, pass 1

**Reviewed**: fully staged working tree against
`586e503496a1762f49651bc1411e18350ceda4a8`, plus the approved F-018 design
plan
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, invalid direction cosines produce incorrect patient-space distances

**Where**: `crates/ocelli-pixel/src/image_plane.rs:31`

**What**: `ImageOrientationPatient::new` rejects zero and exactly collinear
vectors, but accepts vectors that are not unit length or not perpendicular.
`ImagePlane::index_to_world` then multiplies the unnormalised row and column
vectors by Pixel Spacing. An accepted row value of `[2, 0, 0]` therefore
doubles the declared physical distance along the row direction. Accepted
non-perpendicular vectors also shear the in-plane coordinate system.

**Why it is wrong**: DICOM PS3.3 C.7.6.2.1.1 requires the row and column
direction cosine vectors to be orthogonal and normal. It then defines patient
position from those direction cosines and Pixel Spacing. The public type and
LLD call these values validated direction cosines, so silently accepting a
non-unit vector and using its magnitude as another scale factor violates the
physical-coordinate contract. Normative text:
<https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.7.6.2.html#sect_C.7.6.2.1.1>

The approved plan's approach and malformed-input test list stop at finite,
nonzero, and non-collinear vectors. That checklist contradicts the DICOM
section the same plan declares normative. The implementation must resolve the
normative constraint rather than preserve the incomplete checklist.

**Evidence**: A temporary integration probe constructed
`ImageOrientationPatient` with non-unit `[2, 0, 0]` and perpendicular
`[0, 1, 0]`, and separately with skewed `[1, 0, 0]` and `[0.5, 1, 0]`.
`bin/ocelli.sh test ocelli-pixel orientation_domain_probe -- --exact` exited
101 with `left: [true, true]` and `right: [false, false]`. A second probe used
position x `10`, column spacing `0.5`, and column index `4`.
`bin/ocelli.sh test ocelli-pixel nonunit_orientation_distance_probe --
--exact` exited 101 with `left: 14.0` and `right: 12.0`. Both probes were
removed after observation.

### D2, LUT Data accepts values outside its encoded unsigned integer domain

**Where**: `crates/ocelli-pixel/src/lut.rs:23`

**What**: `LutDescriptor::new` checks the declared entry count, descriptor bit
width, first mapped input, and finiteness. It does not check that every LUT
Data entry is an integer in the unsigned range named by `bits_per_entry`.
For an 8-bit LUT, negative `-1`, out-of-range `256`, and fractional `1.5` all
construct successfully and can be returned as modality or display values.

**Why it is wrong**: DICOM PS3.3 C.11.1.1.1 and C.11.2.1.1 define LUT Data as
entry values encoded with the descriptor's bit width. Both sections say the
output range is `0` through `2^n - 1` and is always unsigned. The third
descriptor value therefore constrains the values, not only a stored
`bits_per_entry` accessor. A type documented as validated LUT Descriptor and
LUT Data cannot admit negative, fractional, or overflowing entries.
Normative text:
<https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.html#sect_C.11.1.1.1>
and
<https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.2.html#sect_C.11.2.1.1>.

**Evidence**: A temporary integration probe called the 8-bit constructor with
`-1.0`, `256.0`, and `1.5` and compared the three `is_ok()` results with three
expected refusals. `bin/ocelli.sh test ocelli-pixel lut_data_domain_probe --
--exact` exited 101 with `left: [true, true, true]` and
`right: [false, false, false]`. The probe was removed after observation.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Read the complete microscope workflow, approved F-018 plan, staged diff,
  image-plane and LUT HLD sections, current pixel-pipeline LLD, and all added
  source and test files. The reviewed staged diff contains 1,538 insertions
  and 3 deletions across 14 files.
- Checked stored-value extraction against DICOM PS3.3 C.7.6.3. Container
  widths are explicit, High Bit is constrained to `Bits Stored - 1`, the
  stored mask handles 32 bits without shifting by 32, and signed extraction
  extends from `Bits Stored - 1`. Both source and destination lengths are
  checked before the first write.
- Checked image-plane arithmetic against DICOM PS3.3 C.7.6.2.1.1 apart from
  D1. Column index `i` uses column spacing and the row direction cosine. Row
  index `j` uses row spacing and the column direction cosine. Image Position
  Patient is the centre of index zero. The third column uses row cross column
  and is normalised.
- Checked modality and VOI arithmetic against DICOM PS3.3 C.11. Modality is
  `stored * slope + intercept`. Modality LUT presence wins over rescale under
  the approved HLD rule. `LINEAR` uses `c - 0.5` and `w - 1`.
  `LINEAR_EXACT` uses neither adjustment. Both use `<=` at the lower bound and
  `>` at the upper bound. `SIGMOID` uses exponent `-4 * (x - c) / w`.
- Per the microscope requirement, a temporary mutation changed the LINEAR
  adjusted centre from `center - 0.5` to `center + 0.5`.
  `bin/ocelli.sh test ocelli-pixel
  hld_section_18_3_linear_and_linear_exact_rows_apply_d_13 -- --exact`
  exited 101 with `was 127.18045, wanted 127.81955`. The implementation was
  restored, `git diff --exit-code` then exited 0 for unstaged content, and the
  final `bin/ocelli.sh test ocelli-pixel` exited 0 with 22 tests passing.
- `bin/ocelli.sh check ocelli-pixel` and `bin/ocelli.sh clippy ocelli-pixel`
  exited 0. `bin/ocelli.sh cargo check -p ocelli-pixel --all-targets --target
  wasm32-unknown-unknown` exited 0.
- `bin/ocelli.sh gate bindgen pins`, `bin/ocelli.sh gate unsafe`,
  `bin/ocelli.sh gate nostd`, `bin/ocelli.sh gate prose`, `bin/ocelli.sh gate
  content`, `bin/ocelli.sh gate provenance`, and `bin/ocelli.sh gate wasm`
  each exited 0. The wasm gate measured 16,455 release bytes against a 17,207
  byte ceiling. `git diff --cached --check` exited 0.
- The oracle initially exited 1 because the macOS sandbox denied Chromium's
  Mach port. The same command outside that sandbox exited 0 with 99 views,
  71 pass, 28 unmeasured, 0 fail, 0 absent, and all 29 comparator mutations
  detected.
- No new cast, unsafe block, `wasm-bindgen` use, render-loop path, GPU
  submission, trait, generic abstraction, dynamic dispatch, forwarding-only
  wrapper, or tier-specific arithmetic copy appears in the staged diff.
  Mapping borrows caller-provided output. Only LUT construction owns and
  allocates setup-time vectors.
- The dependency declarations and lockfile agree. `glam` and `ocelli-core`
  are the only new normal dependencies of `ocelli-pixel`, and all public
  exports name substantive evidence or arithmetic types from the approved
  plan.
