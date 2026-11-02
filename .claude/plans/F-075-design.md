# F-075, ROI statistics engine: mean, SD, min, max, area, SUV, histogram

**Status**: approved
**Epic ref**: E12.3
**Sprint**: S29
**Estimate**: 4w

## Normative source, transcribed

HLD `docs/hld/15-lut-chain.md` section 18.1 gives the modality rescale and then says SUV is a separate path. The italic sentence contains an em-dash, so it is quoted in a fence.

```text
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

```text
If a Modality LUT Sequence is present it wins over slope and intercept. PET SUV is a separate path and needs the radiopharmaceutical sequence — do not fold it in here.
```

HLD `docs/hld/05-rendering.md` section 7 says a histogram compute pass is tier A and must degrade.

```text
Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail.
```

HLD `docs/hld/26-differentiating-capabilities.md` names the future kernel and the fallback rule. It does not give a mean, an SD, a bin edge or an SUV equation.

```text
Every tier-A kernel declares a fallback — CPU, or a worker — so a feature degrades rather than fails on WebGL2. A kernel with no fallback marks its feature unavailable; it never silently produces a different answer.
```

```text
Initial kernel set: region growing and interactive segmentation, denoise and gradient filtering, per-frame histogram and ROI statistics, full-resolution MPR and oblique resampling, and registration refinement.
```

`docs/hld/B-parity-surface.md` counts about 63 tool classes. It does not define an ROI statistic.

## What the specification does not cover

The HLD does not define these numbers. The decisions below are local. They are not a formula the HLD forgot to write.

The SUV arithmetic is the project skill `.claude/skills/dicom-expert/SKILL.md` section 9. The skill is not the HLD. This plan does not present it as a PS3.3 equation. The tags it names are the inputs a later parser would read. This story does not parse a file.

```text
activity = stored * RescaleSlope + RescaleIntercept
SUVbw    = activity * PatientWeight_kg * 1000 / decayed_dose_Bq
decayed_dose = RadionuclideTotalDose * 2 ^ ( -elapsed_seconds / RadionuclideHalfLife )
```

- `roi_statistics` reads a caller-supplied sample list. It does not read pixels, a DICOM data set, or `Volume::sample_world`.
- An empty sample list is `GeomError::Degenerate`. It is not a mean of 0.
- A non-finite sample, a non-finite area, or a non-finite total is `GeomError::Frame`.
- `mean` is the sum divided by the count.
- `sd` is the population standard deviation. The divisor is the count, not the count minus one. One sample has `sd` 0.
- `min` and `max` are the ordinary extrema of the same finite samples.
- `area` is the caller-supplied number. This function does not call `rectangle`, `ellipse`, `circle` or `polygon_area`.
- `histogram` writes a caller-supplied `u32` buffer. It allocates nothing. An empty sample list or an empty bin buffer is `Degenerate`, and the buffer is left unchanged. A non-finite sample is `Frame`, and the buffer is left unchanged.
- Bin edges are equal. `width` is `(max - min) / bin_count` when `max` differs from `min`. A value on an interior edge belongs to the higher bin. The maximum belongs to the last bin, including when adding the width steps past it. When every sample is equal, `width` is 0 and every sample is counted in bin 0.
- `suv_bw` does not call `modality` and does not accept a `Modality` value. The activity line is the skill's first line, computed here so the SUV path cannot be inserted into the LUT chain.
- Every SUV input is an argument. There is no default weight, dose, half-life or elapsed time.
- A non-finite SUV input is `SuvError::Frame`, checked before the other refusals.
- `units_bqml` false is `SuvError::Unavailable`.
- Weight, dose or half-life that is not strictly positive is `Unavailable`. A negative elapsed time is `Unavailable`. Elapsed 0 is valid.
- This story does not dispatch a compute shader. The HLD histogram kernel remains a later story. That later kernel must match this function or report unavailable.

## Problem and reproducer

A caller can measure an area and cannot ask for the mean of the samples inside it. A shell that returned 0 for an empty region would look measured.

`roi_statistics` of samples `1` and `3` with area `4` is count 2, mean 2, sd 1, min 1, max 3, area 4. `histogram` of `0`, `0.5` and `1` into two bins is counts `1`, `2`, minimum 0, maximum 1, width 0.5. `suv_bw` with stored 2, slope 1, intercept 0, weight 70, dose 1000, half-life 100, elapsed 0 and `units_bqml` true is 140. The same inputs with elapsed 100 are 280. `units_bqml` false is `Unavailable` and is not 140.

## Scope, API and compatibility

In scope:

- `roi_statistics` and `histogram` in `crates/ocelli-geom/src/stats.rs`
- `suv_bw` in the same file
- `SuvError` as its own enum. `GeomError` gains no variant.
- Unit tests with the values above

Out of scope:

- A DICOM parser, a patient-weight lookup, and a change to `Volume`
- A GPU histogram
- A second area formula
- A TypeScript copy of these numbers
- A `wasm-bindgen` export

```text
roi_statistics(samples, area) -> Result<RoiStats, GeomError>
histogram(samples, counts) -> Result<Histogram, GeomError>
suv_bw(inputs) -> Result<f64, SuvError>
```

`RoiStats` holds `count`, `mean`, `sd`, `min`, `max` and `area`. `Histogram` holds `min`, `max` and `width`. `SuvInputs` holds `stored`, `slope`, `intercept`, `patient_weight_kg`, `total_dose_bq`, `half_life_seconds`, `elapsed_seconds` and `units_bqml`.

`SuvError::Frame` is 402. `SuvError::Unavailable` is 403.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| A later ROI tool | `roi_statistics` | non-empty finite samples and a finite area | mean, population sd, min, max, and the supplied area |
| A later ROI tool | `histogram` | a non-empty bin buffer | counts written by this function |
| A later PET tool | `suv_bw` | finite inputs, BQML, positive weight, dose and half-life | one SUVbw |
| Rectangle, ellipse, circle, polygon | unchanged | this story does not call them | the area the caller already has |

## State transitions and invariants

`roi_statistics` and `suv_bw` are stateless. `histogram` writes only after the inputs validate. A refusal leaves the count buffer unchanged.

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| none | samples 1 and 3, area 4 | mean 2, sd 1, area 4 | no stored state |
| none | empty samples | `Degenerate` | no stored state |
| bins all zero | samples 0, 0.5 and 1 | counts 1 and 2 | the old counts on `Frame` or `Degenerate` |
| none | SUV elapsed 0 | 140 | no stored state |
| none | `units_bqml` false | `Unavailable` | no stored state |

## Ownership, commit and recovery

`histogram` borrows the count buffer. The commit point is the return of `Ok`. A refusal writes nothing. The other two functions allocate nothing and store nothing.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | Samples 1 and 3 with area 4 are mean 2, sd 1, min 1, max 3, area 4 | `two_samples_have_population_sd_one` | dividing by `n - 1`, or recomputing the area | implementation |
| AC2 | An empty sample list is `Degenerate` | `an_empty_region_is_degenerate` | returning 0 | implementation |
| AC3 | One sample has sd 0 | `one_sample_has_sd_zero` | treating one sample as degenerate | implementation |
| AC4 | Samples 0, 0.5 and 1 in two bins are 1 and 2 | `an_interior_edge_belongs_to_the_higher_bin` | using `>` so 0.5 stays in bin 0 | implementation |
| AC5 | Equal samples stay in bin 0 with width 0 | `equal_samples_fill_the_first_bin` | dividing by a zero width | implementation |
| AC6 | A non-finite sample leaves the bins unchanged | `a_non_finite_sample_does_not_write_bins` | writing before the check | implementation |
| AC7 | Elapsed 0 is SUVbw 140 | `elapsed_zero_is_one_hundred_forty` | folding the value through the modality LUT type | implementation |
| AC8 | Elapsed equal to the half-life is 280 | `one_half_life_is_two_hundred_eighty` | skipping the decay | implementation |
| AC9 | Units other than BQML are `Unavailable` | `counts_are_unavailable` | still returning 140 | implementation |
| AC10 | A zero weight is `Unavailable` | `a_zero_weight_is_unavailable` | substituting a default weight | implementation |

## Approach

Add `stats.rs` and re-export it from `ocelli-geom`. Square roots and the power of two go through glam's libm backend, which this crate already uses for length. `2^(-1)` is one half in that backend for the AC8 fixture. The population variance is a second pass over the samples.

Rejected: a sample standard deviation, a GPU dispatch, a DICOM reader, and a call into `ocelli-pixel`.

## Dependencies, write ownership and resources

Depends on F-074, which is done. This story writes `crates/ocelli-geom` and its tests. It does not write `ocelli-pixel`, `ocelli-volume`, shaders, or the TypeScript package. F-077 and F-079 also write `ocelli-geom` and run later in this sprint.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-geom` for the focused loop. `/complete-feature` runs the feature profile.

The mean, the population sd, the bin counts and both SUV values are hand-computed from this plan. AC8 assumes `2^(-1)` is exactly one half.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The functions do not open a device and do not read the resolved tier. The HLD compute kernel is not this story.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `crates/ocelli-geom/tests/stats.rs` |
| fixture | mean 2, sd 1, bins 1 and 2, SUV 140 and 280 | the same tests |

The SUV fixture cites this plan and the skill. It does not cite a PS3.3 equation the standard does not state. The tags the skill names stay unread here.

## Parity surface covered

None. Appendix B counts the tool classes and does not define the statistic.

## Deviations

none

## LLD impact

`docs/lld/geometry.md` and `docs/lld/public-api.md` gain `roi_statistics`, `histogram` and `suv_bw` when the story is completed. They keep the sentence that rectangle, ellipse and circle do not compute these numbers.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The population divisor, the bin edge, the caller-supplied area and the skill SUV formula are the local decisions.
