# F-079, UltrasoundDirectional and VideoRedaction

**Status**: approved
**Epic ref**: E12.7
**Sprint**: S29
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts about 63 tool classes, of which 25 are manipulation and utility. It does not define an ultrasound direction or a redaction rectangle.

`docs/hld/03-architecture-and-crates.md` names `ocelli-geom` and does not give these formulas.

F-070 already returns a Euclidean length through `distance`. This story does not replace that function.

## What the specification does not cover

The HLD does not define these tools. The decisions below are local. They are not a formula the HLD forgot to write.

- `ultrasound_direction` is the image-plane direction of two caller points. It does not read an ultrasound DICOM tag and it does not convert spaces.
- `radians` is `atan2(dy, dx)` through the same glam vector path F-070 uses. Zero is positive `x`. The range is the range of that `atan2`.
- `length` is the Euclidean length of the same span. A zero-length span, including both signs of zero, is `Degenerate`. A non-finite point or a non-finite result is `Frame`.
- A direction along positive `y`, from `(0, 0, 0)` to `(0, 2, 9)`, is `PI / 2` and length 2. `z` is included in the length and is not included in the angle. The angle is the image plane.
- `redact_rect` writes a caller-supplied fill into the intersection of a rectangle and a row-major sample buffer. It does not read a video file.
- The buffer length must equal `width * height`. A mismatch or a zero frame is `Degenerate`, and the buffer is unchanged.
- A non-finite fill, or a non-finite sample inside the intersection, is `Frame`. The function scans the intersection before it writes, so a refusal leaves every sample unchanged.
- A rectangle with a zero side, or a rectangle that misses the frame, returns `applied: false`, `samples_written: 0` and `samples_already_fill: 0`. The buffer is unchanged. That is the report that nothing was redacted.
- An overlapping rectangle writes only the intersection. A sample that already equals the fill counts as `samples_already_fill` and is not a write. A sample that changes counts as `samples_written`. `applied` is true when the intersection is non-empty, including when every sample was already the fill.
- Equality with the fill is `==` on `f64`. There is no epsilon. A negative zero fill matches a positive zero sample, because those values compare equal.
- No TypeScript copy. No SVG. No second length function.

## Problem and reproducer

A caller can measure a length and cannot ask which way an ultrasound mark points, or redact a rectangle while being told whether any sample changed.

`ultrasound_direction` of `(0, 0, 0)` to `(0, 2, 9)` is `PI / 2` and length 2. Identical points are `Degenerate`.

A 2 by 2 buffer `1, 2, 3, 4`, rectangle `x = 0`, `y = 0`, width 1, height 2, fill 0, becomes `0, 2, 0, 4` with `samples_written` 2 and `applied` true. The same call again reports `samples_written` 0, `samples_already_fill` 2 and `applied` true. A rectangle at `(5, 5)` reports `applied` false and leaves the buffer unchanged.

## Scope, API and compatibility

In scope:

- `ultrasound_direction` and `redact_rect` in `crates/ocelli-geom/src/redact.rs`
- `Direction` and `Redaction` structs
- Unit tests with the values above

Out of scope:

- A DICOM reader, a video decoder, and a change to `distance`
- A TypeScript copy and a `wasm-bindgen` export
- The ETDRS grid. That is F-078.

```text
ultrasound_direction(start, end) -> Result<Direction, GeomError>
redact_rect(samples, width, height, x, y, rect_w, rect_h, fill) -> Result<Redaction, GeomError>
```

`Direction` holds `radians` and `length`. `Redaction` holds `samples_written`, `samples_already_fill` and `applied`.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| A later ultrasound tool | `ultrasound_direction` | two finite points and a non-zero span | one plane angle and the Euclidean length |
| A later redaction tool | `redact_rect` | a rectangular intersection, or an explicit miss | the buffer and the written count |
| F-070 `distance` | unchanged | this story does not replace it | Euclidean length for other tools |

## State transitions and invariants

`ultrasound_direction` is stateless. `redact_rect` writes the buffer only after the intersection validates and only inside that intersection. A refusal preserves every sample.

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| none | `(0,0,0)` to `(0,2,9)` | `PI / 2` and length 2 | no stored state |
| none | identical points | `Degenerate` | no stored state |
| `1, 2, 3, 4` | the left column, fill 0 | `0, 2, 0, 4`, written 2 | the old buffer on `Frame` |
| `0, 2, 0, 4` | the same rectangle | written 0, already 2, applied true | samples that were already 0 |
| `0, 2, 0, 4` | rectangle `(5, 5)` | applied false | the whole buffer |
| `1, NaN, 3, 4` | the left column | `Frame` | `NaN` still in place |

## Ownership, commit and recovery

The sample buffer is borrowed. The commit point is the return of `Ok` after the writes. A refusal returns before the first write. A repeated call on an already filled rectangle writes nothing and says so through `samples_already_fill`.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | Positive `y` is `PI / 2` and length 2 | `positive_y_is_half_pi` | using `z` in the angle, or dropping it from the length | implementation |
| AC2 | Identical points are `Degenerate` | `a_zero_direction_is_degenerate` | returning angle 0 | implementation |
| AC3 | The left column becomes 0 and the right column stays | `the_left_column_is_redacted` | filling the whole frame | implementation |
| AC4 | A second pass reports already filled | `a_second_pass_says_the_pixels_were_unchanged` | reporting a write that did not happen | implementation |
| AC5 | A missed rectangle reports `applied` false | `a_missed_rectangle_is_not_applied` | claiming a redaction of an unchanged buffer | implementation |
| AC6 | A non-finite sample in the intersection writes nothing | `a_non_finite_sample_blocks_the_write` | redacting the finite neighbour and then refusing | implementation |

## Approach

Add `redact.rs` and re-export it. The angle uses `DVec2::new(dx, dy).to_angle()`. The length uses the existing `span_length` shape, a `DVec3` length, so `z` counts in the length only. The redaction walks `y` then `x` in `u32` and checks the product with `u32` checked arithmetic before it indexes.

Rejected: a TypeScript redaction, a default black fill that reports success on a miss, and a change to `distance`.

## Dependencies, write ownership and resources

Depends on F-070, which is done. This story writes `crates/ocelli-geom` and its tests. F-075 and F-077 write that crate first. This story does not write `packages/core`.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-geom` for the focused loop. `/complete-feature` runs the feature profile.

The angle, the length and the redacted buffer are hand-computed from this plan.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The functions do not open a device. A missed rectangle is reported. It is not a different pixel on another tier.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `crates/ocelli-geom/tests/redact.rs` |
| fixture | `PI / 2`, length 2, and the buffer `0, 2, 0, 4` | the same tests |

## Parity surface covered

None. Appendix B counts the utility classes and does not define these two tools.

## Deviations

none

## LLD impact

`docs/lld/geometry.md` and `docs/lld/public-api.md` gain `ultrasound_direction` and `redact_rect` when the story is completed. They say a missed rectangle sets `applied` false.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The plane angle, the `z` length, and the explicit miss report are the local decisions.
