# F-073, Length, Height, Angle, CobbAngle, Bidirectional

**Status**: approved
**Epic ref**: E12.1
**Sprint**: S28
**Estimate**: 5w

## Normative source, transcribed

HLD `docs/hld/B-parity-surface.md` Appendix B:

```text
Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility
```

HLD `docs/hld/03-architecture-and-crates.md` names `ocelli-geom` and does not give a length, a height, an angle, a Cobb angle or a bidirectional pair. The backlog names those five tools. It does not give a formula either.

`crates/ocelli-geom` already measures a distance, a polyline length and an unsigned angle. Those functions are the approved local rules in `.claude/plans/F-070-design.md`. This story calls them. It does not replace them.

`distance` is the Euclidean length of two finite points. Identical points are 0. A non-finite point or a non-finite result is `GeomError::Frame`.

`polyline_length` sums segment lengths in order. One finite point is 0. A non-finite coordinate is `Frame`. This story does not change that function.

`angle_at(arm_start, vertex, arm_end)` is the unsigned angle at `vertex`, in radians, in `0..=PI`. A zero-length arm is `GeomError::Degenerate`. The value is `atan2` of the cross-product length and the dot product.

## What the specification does not cover

The HLD does not define these five tools. The decisions below are local. They are not a formula the HLD forgot to write, and they are not a transcription of another viewer's source.

- `length` of two or more points is `polyline_length`. Fewer than two points is `Degenerate`, checked before that call. One finite point is 0 from `polyline_length`. One non-finite point is `Frame` from `polyline_length` and `Degenerate` from `length`.
- `height` is the absolute difference of the caller-space `z` coordinates. The function does not choose an image axis and does not convert spaces. A non-finite point is `Frame`.
- The angle tool is the existing `angle_at` on exactly three points, vertex in the middle, in radians. A second function named `angle` would only call `angle_at`, and a wrapper that only forwards is refused. A zero-length arm is `Degenerate`.
- `cobb` is the acute angle between two undirected segments, four points, in radians, in `0..=PI/2`. A zero-length segment is `Degenerate`. The value uses `atan2` of the absolute cross length and the absolute dot, so reversing either segment does not change it.
- `bidirectional` is two segments. `length` is the larger Euclidean distance and `width` is the smaller. Equal distances use that shared value for both. A zero distance stays 0, because that is the true length of identical points.
- Points stay in the space the caller already chose. There is no canvas conversion and no spacing scale inside these functions.
- No TypeScript copy of the arithmetic. `packages/core/src/tools.ts` already says a length is not computed there. This story does not edit that file.
- The SVG layer and the SR content tree are not this story. F-071 draws. F-094 owns the type.
- Cornerstone 5.8.2 is not the specification. Its height helper returns no value for a general pair, and its line angle reverses one vector before `acos`. Those behaviours are not adopted.

## Problem and reproducer

`distance((0,0,0), (3,4,0))` is 5 and `angle_at((1,0,0), (0,0,0), (0,1,0))` is `PI/2`. A caller cannot ask for a named length of two or more points, a height, a Cobb angle or a bidirectional pair without inventing a second formula.

The observation is a Rust call. `length` of `(0,0,0)` and `(3,4,0)` is 5. `height` of `(0,0,1)` and `(5,5,4)` is 3. `angle_at` of `(1,0,0)`, `(0,0,0)`, `(0,1,0)` is `PI/2`. `cobb` of `(0,0,0)`, `(1,0,0)`, `(0,0,0)`, `(1,1,0)` is `PI/4`. `bidirectional` of `(0,0,0)`, `(3,0,0)`, `(0,0,0)`, `(0,4,0)` reports length 4 and width 3.

## Scope, API and compatibility

In scope:

- New functions in `crates/ocelli-geom/src/measure.rs`, re-exported from the crate root
- Unit tests with the hand-computed values above
- `docs/lld/geometry.md` and `docs/lld/public-api.md` gain `length`, `height`, `cobb` and `bidirectional` when the story is completed. `polyline_length` of one finite point stays 0. The angle tool remains `angle_at`.

Out of scope:

- Pointer handling, SVG drawing, tool registration, calibration and pixel spacing
- The ROI statistics engine, F-075
- An SR content item, F-094
- Editing `polyline_length`, `distance` or `angle_at`

```text
length(points) -> Result<f64, GeomError>
height(start, end) -> Result<f64, GeomError>
cobb(first_start, first_end, second_start, second_end) -> Result<f64, GeomError>
bidirectional(first_start, first_end, second_start, second_end) -> Result<Bidirectional, GeomError>
```

The angle tool is `angle_at`, already exported by F-070. This story does not add a second name.

`Bidirectional` holds `length` and `width`, both `f64`. Units are the units of the point space. Angles are radians.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| A later annotation tool | `length` | two or more finite points | one distance |
| A later annotation tool | `height` | two finite points | absolute `z` difference |
| A later annotation tool | `angle_at` | three finite points, vertex in the middle | radians in `0..=PI` |
| A later annotation tool | `cobb` | two finite segments | radians in `0..=PI/2` |
| A later annotation tool | `bidirectional` | two finite segments | longer and shorter lengths |

## State transitions and invariants

The functions are stateless. A refusal returns `Err` and writes nothing. An identical call returns the same value.

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| none | two points `(0,0,0)` and `(3,4,0)` | length 5 | no stored state |
| none | one point | `Degenerate` | no stored state |
| none | `(0,0,1)` and `(5,5,4)` | height 3 | no stored state |
| none | right angle at the origin | `PI/2` | no stored state |
| none | a zero-length angle arm | `Degenerate` | no stored state |
| none | segments along `x` and `(1,1,0)` | Cobb `PI/4` | no stored state |
| none | lengths 3 and 4 | length 4, width 3 | no stored state |

## Ownership, commit and recovery

Not applicable. The functions allocate nothing and store nothing.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | `(0,0,0)` to `(3,4,0)` is 5 | `length_of_a_three_four_five_segment_is_five` | using only `x` | implementation |
| AC2 | One point is `Degenerate` | `a_single_point_is_not_a_length` | returning 0 | implementation |
| AC3 | Height is `abs(z)` | `height_is_the_absolute_z_difference` | the Euclidean length | implementation |
| AC4 | A right angle is `PI/2` | `a_right_angle_is_half_pi` | a degree result | implementation |
| AC5 | A zero arm is `Degenerate` | `a_zero_arm_is_degenerate` | returning 0 | implementation |
| AC6 | Cobb of `x` and `(1,1,0)` is `PI/4`, and reversing a segment keeps it | `cobb_is_acute_and_ignores_direction` | an order-dependent supplement | implementation |
| AC7 | Lengths 3 and 4 report 4 and 3 | `bidirectional_keeps_the_longer_length` | swapping them | implementation |

## Approach

Add the four new functions beside the F-070 functions. `length` checks the count and calls `polyline_length`. The angle tool stays `angle_at`. `height` subtracts `z`. `cobb` and `bidirectional` use the same finite-point check and the same vector helpers. `f64::sqrt` and `f64::atan2` stay behind glam, as F-070 already does.

Rejected: a TypeScript copy, a cornerstone port, and a height that is undefined for a diagonal.

## Dependencies, write ownership and resources

Depends on F-070, which is done. This story writes `crates/ocelli-geom/src/measure.rs`, `crates/ocelli-geom/src/lib.rs` and the geom tests. No shader and no GPU.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-geom` for the focused loop. `/complete-feature` runs the feature profile. A later edit to `measure.rs` invalidates that receipt.

The fixtures are hand-computed from the local definitions above. They do not cite a DICOM section because these tools are not a PS3.3 formula.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The functions are pure and run on every target that builds `ocelli-geom`.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `crates/ocelli-geom` tests |
| fixture | hand-computed lengths and angles from this plan | the same tests |

## Parity surface covered

None of the tool-class rows are claimed. Appendix B counts the classes and does not define these measurements. A later story can compare a drawing. This story does not.

## Deviations

none

## LLD impact

`docs/lld/geometry.md` and `docs/lld/public-api.md` gain `length`, `height`, `cobb` and `bidirectional`. `geometry.md` keeps `polyline_length` of one finite point at 0, which is not `length`. The angle tool remains `angle_at`. No new LLD file.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The five definitions above are the local decisions.
