# Geometry

**F-IDs that contributed:** F-070, F-073
**Last updated:** 2026-11-02

Hit-testing, projection and measurement in `ocelli-geom`. The HLD names the
crate and does not give the formulas. The rules below are the local
decisions in `.claude/plans/F-070-design.md` and
`.claude/plans/F-073-design.md`.

The functions take `Pt<S>`. They do not convert spaces. A canvas point is
measured in canvas units and a world point in millimetres. The caller
applies an F-048 transform before the call when a conversion is required.
Camera mapping stays in `ocelli-viewport`.

`f64::sqrt` and `f64::atan2` are not in `core` for this `no_std` crate.
Length and the unsigned angle go through workspace `glam` and its libm
backend. The segment parameter is still the written division.

## Refusals

`GeomError::Frame` is 400. `GeomError::Degenerate` is 401. `number` is a
match. These are not `ErrorCode` values. Nothing in this crate writes the
event ring.

For a segment hit, a non-finite input or a negative tolerance is `Frame`
before a zero-length segment is `Degenerate`. A miss is `Ok(false)`.

A non-finite calculation, including a difference that overflows, is
`Frame`. No measurement returns `NaN` or infinity.

## Distance and length

`distance` is the Euclidean length of the difference. A point and itself
have distance 0.

`polyline_length` sums the segments in order. An empty list or one finite
point has length 0. One non-finite coordinate refuses the whole sum.

## Angle

`angle_at` is the unsigned angle at the vertex, in radians, in `0..=PI`.
It is `atan2` of the cross-product length and the dot product. A
zero-length arm is `Degenerate`. The angle tool remains this function.
There is no second name.

## Length, height, Cobb and bidirectional

`length` is `polyline_length` after a count check. Fewer than two points
is `Degenerate`. That includes one non-finite point, because the count is
checked first. One finite point still has `polyline_length` 0. One
non-finite point is `Frame` from `polyline_length`.

`height` is the absolute difference of the caller-space `z` coordinates.
It ignores `x` and `y`. A non-finite point or a non-finite difference is
`Frame`.

`cobb` is the acute angle between two undirected segments, in radians, in
`0..=PI/2`. It is `atan2` of the cross-product length and the absolute
dot. Reversing a segment does not change it. A zero-length segment is
`Degenerate`.

`bidirectional` reports the larger Euclidean distance as `length` and the
smaller as `width`. Equal distances share that value. Identical endpoints
stay 0.

## Segment

`project_onto_segment` uses

```text
t = dot(point - start, end - start) / dot(end - start, end - start)
```

and clamps `t` into `0..=1`. The foot is `start + t * (end - start)`.
An exact zero denominator, `+0` or `-0`, is `Degenerate`. A shorter
segment still projects. There is no epsilon.

`hit_segment` is that distance `<=` the caller tolerance, in the units of
`S`. Tolerance 0 hits a point on the segment.

## Plane

`project_onto_plane` divides `normal` by its length and subtracts that
component from the point. The normal does not have to be unit length. An
exact zero normal is `Degenerate`.

## Tiers

The functions do not open a device and do not change with the tier. Tier
A, tier B and tier C are n/a. The same call returns the same number.
