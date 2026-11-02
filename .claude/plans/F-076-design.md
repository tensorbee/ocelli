# F-076, PlanarFreehandROI, SplineROI and spline mathematics

**Status**: approved
**Epic ref**: E12.4
**Sprint**: S28
**Estimate**: 5w

## Normative source, transcribed

HLD `docs/hld/B-parity-surface.md` Appendix B counts about 63 tool classes, of which 26 are annotation. It does not define a freehand polygon or a spline.

`polyline_length` already sums segment lengths. This story does not replace it. A freehand length is that function, reached through F-073 `length` once that function exists. This story does not add a second length.

## What the specification does not cover

The HLD does not define these tools. The decisions below are local. They are not a formula the HLD forgot to write.

- `polygon_area` is the shoelace sum on `x` and `y`. The ring is closed by the function. The result is the absolute value. Fewer than three points is `Degenerate`. A non-finite point is `Frame`. `z` is not part of the area.
- A spline is a uniform Catmull-Rom curve. It passes through every input point. End ghosts repeat the first and last points. `spline_point(points, segment, t)` evaluates one segment. `segment` selects the span from point `segment` to point `segment + 1`. `t` is in `0..=1`. `t` outside that range, or a segment index that has no next point, is `Degenerate`.
- The segment formula, with `P0` the segment start, `P1` the segment end, `Pm` the previous point and `Pn` the next point, is

```text
q(t) = 0.5 * (
  2*P0
  + (-Pm + P1) * t
  + (2*Pm - 5*P0 + 4*P1 - Pn) * t^2
  + (-Pm + 3*P0 - 3*P1 + Pn) * t^3
)
```

- A straight input stays on that line. The numeric check is points `(0,0,0)`, `(1,0,0)`, `(2,1,0)` on the first segment at `t = 0.5`, which is `(0.4375, -0.0625, 0)`.
- No TypeScript copy. No SVG drawing. No SR content item. The statistics of the polygon are F-075.

## Problem and reproducer

A caller can sum a polyline and cannot ask for the enclosed area or a point on a spline. A shell implementation would be a second formula.

`polygon_area` of `(0,0,0)`, `(1,0,0)`, `(1,1,0)`, `(0,1,0)` is 1. `spline_point` of the three points above, segment 0, `t = 0.5`, is `(0.4375, -0.0625, 0)`. `t = 0` is the first point and `t = 1` is the second point.

## Scope, API and compatibility

In scope:

- `polygon_area` and `spline_point` in `crates/ocelli-geom/src/measure.rs`
- Unit tests with the values above

Out of scope:

- A second polyline length
- ROI mean, standard deviation and SUV
- Drawing the curve

```text
polygon_area(points) -> Result<f64, GeomError>
spline_point(points, segment, t) -> Result<Pt<S>, GeomError>
```

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| A later freehand tool | `polygon_area` | three or more finite points | absolute shoelace area |
| A later spline tool | `spline_point` | `t` in `0..=1` and a real segment | one point on the curve |
| A later freehand tool | F-073 `length` | not reimplemented here | polyline length |

## State transitions and invariants

The functions are stateless. A refusal returns `Err` and writes nothing.

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| none | the unit square | area 1 | no stored state |
| none | two points | `Degenerate` | no stored state |
| none | the three-point spline at `t = 0.5` | `(0.4375, -0.0625, 0)` | no stored state |
| none | `t = 0` on that segment | the first point | no stored state |
| none | `t = 1.5` | `Degenerate` | no stored state |

## Ownership, commit and recovery

Not applicable. The functions allocate nothing and store nothing.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | Unit square area is 1 | `the_unit_square_has_area_one` | forgetting the half or the close | implementation |
| AC2 | Two points are `Degenerate` | `two_points_have_no_area` | returning 0 | implementation |
| AC3 | Midpoint is `(0.4375, -0.0625, 0)` | `the_first_spline_midpoint_is_hand_computed` | a polyline midpoint | implementation |
| AC4 | `t = 0` and `t = 1` are the segment ends | `the_spline_passes_through_the_knots` | an open curve that misses them | implementation |
| AC5 | `t` outside `0..=1` is `Degenerate` | `a_parameter_outside_the_segment_is_degenerate` | extrapolating | implementation |

## Approach

Implement shoelace with an `f64` accumulator and refuse a non-finite total as `Frame`. Implement the Catmull-Rom basis with the same glam vector path F-070 uses, so `no_std` does not call `f64::mul_add` from `std`. The previous point of segment 0 is the first point. The next point of the last segment is the last point.

Rejected: a second length function, and a spline that does not pass through the knots.

## Dependencies, write ownership and resources

Depends on F-070, which is done. The freehand length call depends on F-073, which is the same sprint and is implemented first. This story writes `ocelli-geom` only.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-geom` for the focused loop. `/complete-feature` runs the feature profile.

The area and the midpoint are hand-computed from this plan.

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
| fixture | the unit square and the spline midpoint | the same tests |

## Parity surface covered

None of the tool-class rows are claimed. Appendix B counts the classes and does not define the polygon or the spline.

## Deviations

none

## LLD impact

`docs/lld/geometry.md` and `docs/lld/public-api.md` gain `polygon_area` and `spline_point` when the story is completed. Freehand length remains `length`.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The shoelace rule and the Catmull-Rom basis are the local decisions.
