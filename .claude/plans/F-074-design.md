# F-074, Probe, DragProbe, RectangleROI, EllipticalROI, CircleROI

**Status**: approved
**Epic ref**: E12.2
**Sprint**: S28
**Estimate**: 5w

## Normative source, transcribed

HLD `docs/hld/B-parity-surface.md` Appendix B counts about 63 tool classes, of which 26 are annotation. It does not define a probe, a rectangle, an ellipse or a circle.

`Volume::sample_world` already returns the level-0 sample at a patient point. An index that is not finite, or a corner that has not landed, is `PlaneSample::Absent`. It is not stored zero. `spacing[0]` divides the column cosine and `spacing[1]` divides the row cosine.

The ROI statistics engine is F-075. Mean, standard deviation, area statistics, SUV and the histogram are not this story.

## What the specification does not cover

The HLD does not define these tools. The decisions below are local. They are not a formula the HLD forgot to write.

- A probe and a drag probe are the same read. The caller passes the current patient point to `Volume::sample_world`. This story does not add a second sampler and does not edit `ocelli-volume`. Drag is the caller repeating the point.
- A rectangle is two opposite corners. `width` is the absolute `x` difference and `height` is the absolute `y` difference. `area` is their product. `z` is not part of the area. The function does not convert spaces.
- An ellipse takes two radii in the caller plane. `area` is `PI * abs(rx) * abs(ry)`. A zero radius is `Degenerate`.
- A circle takes a centre and a rim point. The radius is the Euclidean distance. `area` is `PI * radius * radius`. Identical points are `Degenerate`.
- A non-finite coordinate is `Frame`.
- No TypeScript copy of the arithmetic. No SVG drawing. No SR content item.

## Problem and reproducer

`ocelli-geom` has no rectangle, ellipse or circle. A caller who multiplies the sides in the shell would be a second formula.

`rectangle` of `(0,0,0)` and `(2,3,9)` has width 2, height 3 and area 6. `ellipse` with radii 2 and 3 has area `6 * PI`. `circle` from `(0,0,0)` to `(3,4,0)` has radius 5 and area `25 * PI`.

## Scope, API and compatibility

In scope:

- `rectangle`, `ellipse` and `circle` in `crates/ocelli-geom/src/measure.rs`
- Unit tests with the values above

Out of scope:

- Pixel statistics, SUV and histograms. Those are F-075.
- A new volume sampler
- Tool registration and SVG drawing

```text
rectangle(corner, opposite) -> Result<Rectangle, GeomError>
ellipse(rx, ry) -> Result<f64, GeomError>
circle(centre, rim) -> Result<Circle, GeomError>
```

`Rectangle` holds `width`, `height` and `area`. `Circle` holds `radius` and `area`.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| A later ROI tool | `rectangle` | two finite corners | width, height, area |
| A later ROI tool | `ellipse` | two non-zero radii | area |
| A later ROI tool | `circle` | centre and a different rim | radius and area |
| A probe caller | `Volume::sample_world` | existing function, not edited | one plane sample |

## State transitions and invariants

The new functions are stateless. A refusal returns `Err` and writes nothing.

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| none | corners `(0,0,0)` and `(2,3,9)` | width 2, height 3, area 6 | no stored state |
| none | radii 2 and 3 | area `6 * PI` | no stored state |
| none | radius 0 | `Degenerate` | no stored state |
| none | `(0,0,0)` to `(3,4,0)` | radius 5, area `25 * PI` | no stored state |

## Ownership, commit and recovery

Not applicable. The functions allocate nothing and store nothing.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | Corners `(0,0,0)` and `(2,3,9)` | `rectangle_ignores_z` | including `z` in the area | implementation |
| AC2 | Radii 2 and 3 | `ellipse_area_is_pi_times_the_radii` | using diameters | implementation |
| AC3 | A zero radius | `a_zero_radius_is_degenerate` | returning area 0 | implementation |
| AC4 | Rim `(3,4,0)` | `circle_radius_is_the_euclidean_length` | using only `x` | implementation |

## Approach

Add the three functions beside the F-073 measurements. `circle` calls `distance`. `PI` is `core::f64::consts::PI`, which is in `core` on this `no_std` crate.

Rejected: a probe wrapper that only calls `sample_world`, and a statistics engine in this story.

## Dependencies, write ownership and resources

Depends on F-070, which is done. This story writes `ocelli-geom` only. It does not write `ocelli-volume`.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-geom` for the focused loop. `/complete-feature` runs the feature profile.

The fixtures are hand-computed from the local definitions above. They do not cite a DICOM section.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `crates/ocelli-geom` tests |
| fixture | hand-computed areas from this plan | the same tests |

## Parity surface covered

None of the tool-class rows are claimed. Appendix B counts the classes and does not define these shapes.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` gains the three shape names when the story is completed. The probe stays the existing `sample_world` sentence.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The shape definitions above are the local decisions.
