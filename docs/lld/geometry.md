# Geometry

**F-IDs that contributed:** F-070, F-073, F-074, F-075, F-076, F-077, F-079
**Last updated:** 2026-11-02

Hit-testing, projection and measurement in `ocelli-geom`. The HLD names the
crate and does not give the formulas. The rules below are the local
decisions in `.claude/plans/F-070-design.md`,
`.claude/plans/F-073-design.md`, `.claude/plans/F-074-design.md`,
`.claude/plans/F-075-design.md`,
`.claude/plans/F-076-design.md`,
`.claude/plans/F-077-design.md` and
`.claude/plans/F-079-design.md`.

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

## Rectangle, ellipse and circle

`rectangle` takes two opposite corners. `width` is the absolute `x`
difference and `height` is the absolute `y` difference. `area` is their
product. `z` is not a factor. A non-finite point or a non-finite area is
`Frame`. A zero side stays area 0.

`ellipse` returns `PI` times the product of the absolute radii. Radii of
opposite sign still give that positive area. A zero radius, including
either sign of zero, is `Degenerate`. A non-finite radius is `Frame`.

`circle` takes a centre and a rim point. The radius is the Euclidean
distance. `area` is `PI * radius * radius`. Identical points are
`Degenerate`. A non-finite point is `Frame`.

A probe is `Volume::sample_world` in `ocelli-volume`. This crate does not
add a second sampler. Mean, standard deviation, SUV and the histogram are
not these functions.

## Region statistics

`roi_statistics` summarises a caller-supplied sample list. An empty list is
`Degenerate`. The mean is the sum divided by the count. The standard
deviation divides by the count. One sample has standard deviation 0. The
area is the caller's number. This function does not call `rectangle`,
`ellipse`, `circle` or `polygon_area`.

`histogram` writes a caller-supplied bin buffer. An empty sample list or an
empty bin buffer is `Degenerate`, and the buffer stays unchanged. A value on
an interior edge belongs to the higher bin. The maximum belongs to the last
bin, including when adding the width steps past it. Equal samples land in
bin 0 and the width is 0.

`suv_bw` is the separate PET path. It does not call the modality LUT. Units
other than BQML, or a weight, dose or half-life that is not strictly
positive, or a negative elapsed time, is unavailable. A non-finite input is
`Frame`. There is no default weight. `SuvError::Frame` is 402 and
`SuvError::Unavailable` is 403.

## Polygon and spline

`polygon_area` is half the absolute shoelace sum on `x` and `y`. The
function closes the ring. Fewer than three points is `Degenerate`. A
non-finite point or a non-finite sum is `Frame`. `z` is not a factor.

`spline_point` evaluates one uniform Catmull-Rom segment. The curve passes
through the knots. End ghosts repeat the first and last points. `t` outside
`0..=1`, including a non-finite `t`, is `Degenerate`. A segment with no
following point is `Degenerate`. A non-finite used point or a non-finite
result is `Frame`.

Freehand length remains `length`. There is no second length function.

## Livewire

`livewire_path` is a 4-connected path on a caller-supplied cost grid. The
cost is the sum of the pixels entered. The start pixel adds nothing. `z`
is ignored and every returned point stores 0. A negative cost is
`Degenerate`. A non-finite cost or index is `Frame`. A non-integer index
is `Degenerate`. Start equal to goal is one point.

`livewire_mask` marks the pixels that path visits. It does not fill an
interior. An empty path is `Degenerate` and no mask is returned.

`polygon_area` and `spline_point` stay the F-076 functions. This path
does not call them.

## Ultrasound direction and redaction

`ultrasound_direction` is the image-plane angle of two caller points.
The angle is `atan2(dy, dx)`. Zero is positive `x`. `z` is not a factor.
The length is the Euclidean length of the same span, so `z` is a factor.
A zero-length span, including both signs of zero, is `Degenerate`. A
non-finite point or result is `Frame`.

`redact_rect` writes a caller fill into the intersection of a rectangle
and a row-major buffer. The buffer length must equal `width * height`.
A mismatch or a zero frame is `Degenerate` and the buffer is unchanged.
A non-finite fill, or a non-finite sample in the intersection, is
`Frame` and nothing is written. A zero side or a missed rectangle sets
`applied` false. A sample that already equals the fill counts as
`samples_already_fill`. Equality matches a negative zero with a positive
zero. `applied` is true when the intersection is non-empty.

`distance` stays the F-070 function.

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
