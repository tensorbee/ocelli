# F-195 review, pass 1

**Reviewed**: uncommitted working tree on sprint/s43 for F-195
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 3 smells, 6 nitpicks

## Acceptance challenged

Pixel spacing wins when later sources are also present. An invalid earlier source returns. Imager spacing 0.2 mm divided by ERMF 2 is 0.1 mm, and a second component of 0.4 mm becomes 0.2 mm. Ultrasound is used only when the earlier sources are absent. Manual is last. SUVlbm is 350 and SUVbsa is 90. The existing body-weight fixtures stay 140 and 280. Length ratio 4/2 is 2, and both signs of zero are degenerate. Contour volume is 150. The example route reports those fixture numbers. No unsafe, no wasm-bindgen and no second LUT.

## Evidence

The reviewer had no shell. The host had run the calibration and stats tests, the example route tests, clippy, tsc and eslint. An ERMF multiply and a BSA scale of 1000 each went red and were restored.

## Defects

none

## Smells

### S1, ultrasound deltas were described as millimetres

**Where**: `crates/ocelli-geom/src/calibration.rs` and `docs/lld/calibration.md` at that pass, and the plan citation C.8.5.5.1.4
**What**: The text said Physical Delta X and Y are in millimetres.
**Why it is wrong**: PS3.3 C.8.5.5.1.15 names the units, and 0003H is centimetres. PS3.3 C.8.5.5.1.17 defines the deltas. A later tag reader that trusted the millimetre sentence would scale ultrasound lengths by ten.
**Evidence**: Read against the cited sections. The code itself takes `SpacingMm` and does not read tags.
**Scope and regression**: Documentation and the plan citation. The arithmetic was not wrong for a caller who already converted.

### S2, a non-finite imager half returned Unavailable

**Where**: `crates/ocelli-geom/src/calibration.rs` incompleteness branch
**What**: NaN on one half of the imager pair returned Unavailable before a finiteness check.
**Why it is wrong**: The function doc said a non-finite present value is Frame.
**Evidence**: Reading the branch order.
**Scope and regression**: Callers that branch on 402 versus 403.

### S3, a half imager pair blocks later sources without a pinned test

**Where**: `crates/ocelli-geom/src/calibration.rs`
**What**: A lone ERMF or lone imager spacing returns Unavailable even when manual is present. The plan's no-fall-through rule supports that, and no test paired a half with manual.
**Why it is wrong**: The behaviour was incidental rather than stated.
**Evidence**: Reading the branch and the tests.
**Scope and regression**: Calibration selection only.

## Nitpicks

The route test used short substrings. The zero check is copied. The SUV scale is validated even though it is a constant. The stats module header named only F-075. `length_ratio` keeps a negative sign without saying so. Calibration returns `SuvError`.

## Verified clean

Selection order, the division, the shared decay, the volume sum, the route registration and the changelog bullet agreed with the plan. `suv_bw` still matches the F-075 fixtures in the current source.

## Limitations and handoff

The reviewer could not diff `stats.rs` and did not open the DICOM standard during the pass. The host checked C.8.11.4, C.8.7.5 and C.8.5.5.1.15 after the pass. Fix the citations, check finiteness before incompleteness, and pin a half pair against manual.
