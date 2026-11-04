# F-195, calibration provenance and body-size SUV

**Status**: approved
**Epic ref**: E38.5
**Sprint**: S43
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/15-lut-chain.md` says PET SUV is a separate path and must not be folded into the modality LUT. It gives no SUV formula.

`docs/hld/` does not name calibration provenance, ERMF, ultrasound region spacing, length ratio, or volume from contours.

The local SUV body-weight rule is `suv_bw` in `crates/ocelli-geom/src/stats.rs`, approved in F-075. Activity is `stored * slope + intercept`. Decay is `2^(-elapsed / half_life)`. The result is `activity * weight_kg * 1000 / decayed_dose`. Units must be BQML.

DICOM PS3.3 C.7.6.2 gives Pixel Spacing as the patient spacing, row then column. PS3.3 C.8.11.4 gives Imager Pixel Spacing at the detector. PS3.3 C.8.7.3 gives Estimated Radiographic Magnification Factor as source-to-detector distance divided by source-to-object distance, so patient spacing derived from the imager is imager spacing divided by that factor. PS3.3 C.8.5.5.1.4 gives ultrasound Physical Delta X and Physical Delta Y in millimetres.

## What the specification does not cover

The HLD does not choose an anthropometric formula for lean body mass or body surface area. This plan does not compute either from height and weight. The caller supplies lean body mass in kilograms and body surface area in square metres, the same way `suv_bw` receives weight.

SUVbsa uses the historical scale `activity * bsa_m2 * 10000 / decayed_dose`. The 10000 converts square metres to square centimetres so the number sits beside SUVbw, which multiplies kilograms by 1000 to reach grams. SUVlbm uses `activity * lbm_kg * 1000 / decayed_dose`.

Length ratio is the caller's two lengths. Volume from contours is slice spacing times the sum of the caller areas. This plan does not integrate a trapezoid and does not read a contour from DICOM.

## Problem and reproducer

`suv_bw` exists. Nothing selects a spacing source, and nothing computes SUVlbm or SUVbsa. A caller that has only imager spacing has no named provenance. The example page has no control for these values.

## Scope, API and compatibility

In scope, in `ocelli-geom`:

- `select_calibration` chooses the first present source in this order: pixel spacing, imager spacing divided by ERMF, ultrasound physical deltas, then a manual pair.
- `suv_lbm` and `suv_bsa` share the decay of `suv_bw` and do not change `suv_bw`.
- `length_ratio` and `contour_volume`.

Out of scope: reading DICOM tags inside `ocelli-geom`, changing `suv_bw`, changing spacing tolerances in `ocelli-dicom`, and resampling pixels.

A missing source, a non-finite value, a non-positive spacing, a non-positive ERMF, a non-positive body-size magnitude, or a zero length denominator returns the existing error. Nothing is substituted from a later source when an earlier source is present but invalid. An invalid earlier source is an error, not a fall-through.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Example calibration route | `select_calibration` | Synthetic pairs, no DICOM file | Source name and the two millimetre values |
| Example SUV route | `suv_lbm`, `suv_bsa` | The same decay inputs as `suv_bw` | Two finite numbers, or the existing refusal |
| `suv_bw` callers | unchanged `suv_bw` | F-075 inputs | The F-075 value |

## State transitions and invariants

Stateless. A repeated call with the same inputs returns the same value. A refusal writes no stored spacing.

## Ownership, commit and recovery

Not applicable. These functions allocate no cache and publish no image.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Pixel spacing present wins, even when imager spacing is also present | unit test | Falling through to ERMF | implementation |
| AC2 | Imager spacing 0.2 mm and ERMF 2 yields 0.1 mm | hand fixture | Multiplying by ERMF | implementation |
| AC3 | Ultrasound deltas are used only when pixel spacing and the imager pair are absent | unit test | Preferring ultrasound over pixel spacing | implementation |
| AC4 | Manual is last. All absent returns unavailable | unit test | Inventing 1 mm | implementation |
| AC5 | SUVlbm for activity 5, LBM 70 kg, dose 1000 Bq, elapsed 0 is 350 | hand fixture | Using weight instead of LBM | implementation |
| AC6 | SUVbsa for activity 5, BSA 1.8 m2, dose 1000 Bq, elapsed 0 is 90 | hand fixture | Omitting the 10000 scale | implementation |
| AC7 | `suv_bw` fixture stays the value F-075 recorded | existing test | Editing the body-weight formula | implementation |
| AC8 | Length ratio 4 / 2 is 2. A zero denominator is degenerate | unit test | Returning infinity | implementation |
| AC9 | Areas 10 and 20 with spacing 5 mm give volume 150 | hand fixture | Averaging the areas | implementation |
| AC10 | The example page shows the source name, 350, 90, the ratio and the volume | route test | A core result with no control | integrated |

## Approach

Add the functions beside `suv_bw`. Extract the shared decay so the three SUV functions cannot drift. Keep tag reading out of `ocelli-geom`. The example server calls the functions with the fixture constants and the page prints the JSON.

## Dependencies, write ownership and resources

Depends on F-049 and F-075, both done. Writes `ocelli-geom`, the example server, the example page, the LLD note, the changelog and the sprint ledgers. No GPU.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-geom` for the new cases, then the example route test. Feature profile at story close. The sprint profile is later. A change to the 0.5 percent volume spacing rule or to `suv_bw` invalidates this story.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The functions are host arithmetic. They do not draw.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | ERMF division, SUVlbm 350, SUVbsa 90, volume 150, citing the sections above | `ocelli-geom` tests |
| regression | `suv_bw` unchanged | existing SUV test |

## Parity surface covered

none

## Deviations

none

## LLD impact

`docs/lld/` gains a short calibration note, linked from `docs/lld/README.md`.

## Open questions

none
