# F-211, folder viewer measurements

**Status**: approved
**Epic ref**: E38.21
**Sprint**: S46
**Estimate**: 6w

## Normative source, transcribed

`docs/hld/07-concurrency-and-typescript.md` section 10:

```text
- The SVG annotation drawing layer — DOM-native, and faster left there
- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

`docs/hld/15-lut-chain.md` section 18.1 and the stage table are transcribed in `.claude/plans/F-210-design.md` and are not repeated here. Region statistics are in modality units, so they are taken after stage 1 and before stage 2. The window never enters them.

`docs/hld/26-differentiating-capabilities.md` lists "per-frame histogram and ROI statistics" among the initial compute kernels. That is the GPU path. This story is the CPU path in `ocelli-geom` and does not dispatch a kernel.

The library rules this story calls, quoted from their doc comments, which record the approved local plans named beside them:

- `ocelli_geom::roi_statistics(samples: &[f64], area: f64) -> Result<RoiStats, GeomError>`, `crates/ocelli-geom/src/stats.rs`, F-075. "An empty sample list is degenerate. The standard deviation divides by the count. The area is the caller's number." `RoiStats` has `count`, `mean`, `sd`, `min`, `max`, `area`.
- `ocelli_geom::angle_at(arm_start, vertex, arm_end) -> Result<f64, GeomError>`, `measure.rs`, F-070. "Unsigned angle at `vertex`, in radians, in `0..=PI`. The value is `atan2` of the cross-product length and the dot product of the two arms. A zero-length arm is degenerate."
- `ocelli_geom::cobb(first_start, first_end, second_start, second_end)`, F-073. "Acute angle between two undirected segments, in radians, in `0..=PI/2`."
- `ocelli_geom::bidirectional(..) -> Result<Bidirectional, GeomError>`, F-073. "Longer and shorter Euclidean lengths of two segments." Fields `length` and `width`.
- `ocelli_geom::length(points)`, F-073, "Polyline length. Fewer than two points is degenerate."
- `ocelli_geom::rectangle(corner, opposite) -> Result<Rectangle, GeomError>`, F-074. "`width * height`. `z` is not a factor."
- `ocelli_geom::ellipse(rx, ry) -> Result<f64, GeomError>`, F-074. "`PI * abs(rx) * abs(ry)` ... A zero radius is degenerate rather than area 0."
- `ocelli_geom::polygon_area(points)`, F-076. "A polygon area is half the absolute shoelace sum, and the function closes the ring. Fewer than three points is degenerate."
- `ocelli_geom::select_calibration(request) -> Result<Calibration, SuvError>`, `calibration.rs`, F-195. "The order is pixel spacing, imager spacing with the magnification factor, ultrasound deltas, then manual." Imager spacing is divided by the Estimated Radiographic Magnification Factor. A present but invalid earlier source is an error and does not fall through. `CalibrationSource::name` is `pixel-spacing`, `imager-spacing`, `ultrasound`, `manual`.

The only sample-inclusion rules in the workspace are in `ocelli-seg`, for labelmap painting, all `pub(crate)`:

- `ThresholdShape::Rectangle` in `crates/ocelli-seg/src/threshold.rs`, F-083: "Inclusive index bounds on this grid", tested by `index_in_span(x, min_x, max_x)`.
- `disk_contains` in `crates/ocelli-seg/src/brush.rs`, F-080: "Squared index distance on `x` and `y` is within `radius_sq`", compared with `is_le`.
- `inside_polygon` in `crates/ocelli-seg/src/contour_edit.rs`, F-085: "Even-odd fill plus the boundary is the approved plan." A sample on an edge is inside.

In all three a sample is the integer index point `(x, y)`, which is the sample centre. There is no ellipse rule anywhere. `ocelli-geom` has no inclusion rule at all.

From the DICOM standard. These lines were transcribed without a local copy of the standard, which is a transcription from memory, and the flag stays. AC19 makes the reviewer confirm them against the current edition at dicom.nema.org before the spacing, calibration and padding code is implemented (open question Q6):

- PS3.3 C.7.6.2, Pixel Spacing (0028,0030), "Physical distance in the patient between the center of each pixel, specified by a numeric pair - adjacent row spacing (delimiter) adjacent column spacing in mm."
- PS3.3 C.7.6.2.1.1, the image plane equation. With `i` the column index and `j` the row index, `P = S + i * Δi * X + j * Δj * Y`, where `Δi` is the column spacing `PixelSpacing[1]` and `Δj` the row spacing `PixelSpacing[0]`. For orthonormal `X` and `Y` this map is an isometry from the plane scaled by `(Δi, Δj)` into LPS, so a distance or an angle between in-plane points scaled that way equals the LPS one.
- PS3.3 C.8.11.4 Imager Pixel Spacing (0018,1164), at the detector, not corrected for magnification. PS3.3 C.8.7.5 Estimated Radiographic Magnification Factor (0018,1114). Both as transcribed in `.claude/plans/F-195-design.md`.
- PS3.3 C.7.5.1.1.2 Pixel Padding Value, as in the F-210 plan.

## What the specification does not cover

Neither the HLD nor the library defines which pixels a region contains. This plan defines it once, in `ocelli-geom`.

The pixel-centre rule. A sample at column index `c` and row index `r` has its centre at the index point `(c, r)`. It belongs to a region when that centre is inside the shape or on its boundary.

- Rectangle with index corners `(x0, y0)` and `(x1, y1)`: `min(x0, x1) <= c <= max(x0, x1)` and the same for `r`.
- Axis-aligned ellipse with index centre `(cx, cy)` and radii `rx > 0`, `ry > 0`: `((c - cx) / rx)^2 + ((r - cy) / ry)^2 <= 1`.
- Polygon of three or more index vertices, closed from the last to the first: inside by the even-odd rule, and a centre on an edge is inside. This is the F-085 rule, so a freeform measured here and the same ring painted by `edit_labelmap_with_contour` cover the same samples.

The page-to-index conversion. Page marks are in image-pixel coordinates where cell `c` covers `[c, c + 1)` and has its centre at `c + 0.5`. Index space puts that centre at `c`. So `index = page - 0.5` on both axes. This conversion happens once, in the server's `measure.rs`, and the page sends its stored points unchanged. It cancels in every length and angle and matters in every region. A box the page draws on cell edges has index bounds on `.5`, so no sample centre ties with it.

Area. The reported area is the area of the drawn shape, from `rectangle`, `ellipse` or `polygon_area` on millimetre points. It is not the sample count times the pixel area. Both numbers are reported: `area` and `count`.

Millimetre points. Each index point `(c, r)` becomes `Pt::<World>::new(c * column_mm, r * row_mm, 0.0)`. This is plane-local, a rigid motion away from LPS by C.7.6.2.1.1, and it is labelled that way in the code. Uncalibrated points stay in `Pt::<Index>` and lengths are in `px`.

Calibration provenance. `select_calibration` is called with Pixel Spacing from the frame's Pixel Measures Sequence or the top level, Imager Pixel Spacing, and Estimated Radiographic Magnification Factor (0018,1114). Ultrasound regions are not read by this example and `ultrasound_delta` is `None`. `manual` is `None`. The reply's `source` and its unit label:

| Inputs present | `select_calibration` result | `source` | Length unit | Area unit |
|---|---|---|---|---|
| Pixel Spacing | `PixelSpacing` | `pixel-spacing` | `mm` | `mm²` |
| Imager Pixel Spacing and ERMF | `ImagerSpacing`, imager spacing divided by ERMF | `imager-spacing` | `mm` | `mm²` |
| Imager Pixel Spacing only | `Unavailable` | `detector` | `mm at the detector` | `mm² at the detector` |
| none | `Unavailable` | `uncalibrated` | `px` | `px²` |
| a present source that is invalid | `Frame` or `Unavailable` from that source | refused, code `calibration` | none | none |

The `detector` row is not a `select_calibration` source. It keeps the F-207 behaviour the page shows today and uses the raw Imager Pixel Spacing. Open question Q1 asks whether that row stays.

Angles. `angle_at` and `cobb` run on millimetre points when a calibration exists, so a non-square pixel gives the physical angle. Uncalibrated angles run on index points and say `square pixels assumed`.

Statistics. Samples are modality values from F-210's provider, `ModalityTransform::apply` on each included stored sample, widened to `f64` without loss. Padding samples are excluded and counted as `padding_excluded`. A region whose every sample is padding returns the degenerate refusal. The unit is F-210's unit rule. The standard deviation is F-075's population form.

Self-intersecting freeform. The shoelace area and the even-odd sample set disagree for a ring that crosses itself. The server tests every pair of non-adjacent edges for a proper crossing and refuses a crossing ring with code `self-intersecting` and the sentence `This outline crosses itself, so its area is not measured.`

Size limit. A region is refused when its index bounding box holds more than `4096 * 4096` samples, with code `too-large`. That bounds one request at 128 MiB of `f64`. A freeform is also refused with `too-large` when its bounding-box sample count times its vertex count exceeds `2^28`, because `region_contains` is evaluated per sample and is linear in the vertices. The self-intersection test is quadratic in the vertices, which the 4000-point cap bounds at 8 million pairs.

Text and rounding, all in Rust. Lengths `{:.1}` plus the unit. Angles in degrees `{:.1}` plus `°`. Area `{:.2}` plus the unit. Mean and SD `{:.1}` plus the modality unit. Min and max with the shortest `f32` `Display`, because they are sample values. `{:.1}` rounds the exact binary value to the nearest decimal.

Tool keys. `Q` angle, `J` Cobb, `B` bidirectional, `U` arrow, `X` probe. None of these is bound today. The draft gave angle `G`, and the operator's answer to F-208 Q1 binds `G` to the layout picker and `Shift G` to tile mode, so angle moves to `Q`, which no plan in this sprint binds. F-208, F-213 and F-214 also change bindings this sprint.

Bidirectional. The page places the short axis at the long axis midpoint, perpendicular in image-pixel space, at half its length. After an edit the two segments are measured as drawn. The reply reports both lengths and makes no perpendicularity claim.

Arrow. A line, a head and an optional note. It has no number and no server call.

## Problem and reproducer

Open Synthetic study, choose Rectangle, and drag over the body. The page shows an outline and the sentence `A drawing around a region. It is not an area and it is not saved.` There is no area, mean or SD. There is no angle, Cobb, bidirectional, arrow or probe tool. Length is computed in the page by `lengthText`, `Math.hypot(dCol * facts.column, dRow * facts.row)`, which is a second length formula beside `ocelli_geom::length`. A mark cannot be reshaped after it is placed.

## Scope, API and compatibility

In scope, in `ocelli-geom`:

```rust
pub enum RegionShape<'a> {
    Rectangle { corner: Pt<Index>, opposite: Pt<Index> },
    Ellipse { center: Pt<Index>, rx: f64, ry: f64 },
    Polygon { points: &'a [Pt<Index>] },
}

pub fn region_contains(shape: &RegionShape<'_>, column: u32, row: u32) -> Result<bool, GeomError>

pub fn region_bounds(shape: &RegionShape<'_>, width: u32, height: u32) -> Result<Option<IndexBounds>, GeomError>
```

`IndexBounds` holds inclusive `first_column`, `last_column`, `first_row`, `last_row` clipped to the grid, or `None` when the shape misses the grid. Non-finite inputs are `Frame`. A zero ellipse radius and a polygon under three points are `Degenerate`. `z` is ignored. The functions allocate nothing, so the crate stays `no_std`.

In scope, in the example:

- `fn measure_json(held: &HeldFrame, shape: MeasureShape, points: &[ImagePoint]) -> Result<String, PreviewError>` in a new `examples/folder-viewer/src/measure.rs`, used by the route. `pub fn measure_frame(bytes: &[u8], frame: u32, shape: MeasureShape, points: &[ImagePoint]) -> Result<String, PreviewError>` is its composition with F-216's `decode_held`, re-exported from `lib.rs`, so every fixture runs through the function the route uses. `MeasureShape` is `Length`, `Angle`, `Cobb`, `Bidirectional`, `Rectangle`, `Ellipse`, `Freeform`.
- F-216's `HeldFrame` gains one field, `calibration: CalibrationInputs`, filled by `decode_held` for the frame: Pixel Spacing from the Pixel Measures Sequence or the top level, Imager Pixel Spacing and Estimated Radiographic Magnification Factor, each as the resolved stored text. `select_calibration` runs per request on those inputs, so the held frame carries no derived spacing. Modality values and padding come from F-210's `HeldFrame::value`.
- `POST /measure` in `serve.rs`.
- Page tools Angle, Cobb, Bidirectional, Arrow, Probe. Rectangle, Ellipse and Freeform gain statistics. Length moves to the server. Handles on every point of the selected mark. A measurement list in the inspector.

Route shape. `/measure` uses F-216's file id protocol. The file is posted once to `POST /file`, and a measure request names it by id. Points can number in the thousands for a freeform, which does not fit the 16 KiB header cap, so the body carries the points and nothing else.

```text
POST /measure
X-Ocelli-File: <id from POST /file>
X-Ocelli-Frame: 0
X-Ocelli-Shape: rectangle
Content-Length: 11

69,48 71,49
```

The body is ASCII space-separated `x,y` pairs, at most 256 KiB. Point counts per shape: length 2, angle 3 (arm, vertex, arm), Cobb 4, bidirectional 4, rectangle 2 corners, ellipse 2 corners of its box, freeform 3 to 4000. A wrong count, a malformed pair, a non-finite number, a non-ASCII byte, or a body over 256 KiB is 400 code `points`. A request without `X-Ocelli-File` is 400 `header`. An unknown, evicted or other-process id is F-216's 410 `file-gone`, and the page re-posts once through `ensureFile` and retries once. F-216's rule that an id with a non-empty body is refused applies to the routes whose body is a Part 10 file. `/measure` is the one id route whose body is a point list, and that is stated in F-216's protocol table too. The reply carries `X-Ocelli-Source: held` or `decoded`.

Reply for a region, 200, `application/json`:

```json
{"shape":"rectangle","frame":0,
 "calibration":{"source":"pixel-spacing","row_mm":0.7,"column_mm":0.5},
 "area":{"value":0.7,"unit":"mm²","text":"0.70 mm²"},
 "stats":{"count":2,"padding_excluded":0,"unit":"HU",
          "mean":370,"sd":330,"min":40,"max":700,
          "text":"mean 370.0 HU, SD 330.0 HU, min 40 HU, max 700 HU"}}
```

A line shape returns `length` or `angle` objects of the same form. Bidirectional returns `length` and `width`. Refusals reuse `PreviewError` codes and add `Calibration`, `SelfIntersecting` and `Degenerate`, codes `calibration`, `self-intersecting`, `degenerate`. A refused reply has no partial numbers.

Probe uses F-210's `POST /value` with the probe points of the shown frame. It is not a second sampler.

Page model additions to F-207's note map: `angle` 3 points, `cobb` 4, `bidirectional` 4, `arrow` 2 points and a text, `probe` 1 point. Each measured note gains `result` (the reply) and `revision` (a counter). The marks list shows the type, the reply's text, or the refusal sentence.

Sentences that change:

| Key | Current | After F-211 |
|---|---|---|
| `rect` | `A drawing around a region. It is not an area and it is not saved.` | `A region. The local process reports its area and the mean, SD, minimum and maximum of the modality values whose pixel centres are inside it. It is not saved.` |
| `ellipse` | same as `rect` | same as the new `rect` |
| `contour` | `A drawing that follows the pointer. It is kept when it has at least three points. It is not saved.` | `A freehand region with at least three points. The local process reports its area and statistics. An outline that crosses itself is not measured. It is not saved.` |
| `length` | `The distance between the two points. It is in millimetres when the file gives Pixel Spacing, and in pixels when it gives no spacing. It is not saved.` | `The distance between the two points, from the local process. Millimetres from Pixel Spacing, or from Imager Pixel Spacing with a magnification factor, millimetres at the detector from Imager Pixel Spacing alone, and pixels when the file gives no spacing. It is not saved.` |

Non-goals: saving, TID 1500 (F-215), a manual calibration control, ultrasound regions, a histogram, SUV, a circle tool, a rotated ellipse, measurement across frames, statistics in a 2 by 2 tile, and any GPU path.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Length, angle, Cobb, bidirectional | `POST /measure` then `length`, `angle_at`, `cobb`, `bidirectional` | mm points when calibrated | `length` or `angle` text |
| Rectangle, ellipse, freeform | `POST /measure`, `region_bounds`, `region_contains`, `roi_statistics`, and `rectangle`, `ellipse` or `polygon_area` | page minus 0.5, padding excluded, F-210 unit | area and stats text |
| Calibration for every shape | `HeldFrame::calibration`, then `select_calibration` plus the `detector` row | Pixel Measures Sequence first | `calibration.source` |
| Modality samples | F-216's `Viewer::frame` and F-210's `HeldFrame::value` | Modality LUT Sequence refused | `f64` samples |
| Probe | `POST /value` (F-210) | probe points of the shown frame | value text at the probe |
| Arrow | none | page drawing only | line, head, note |
| `ocelli-seg` painting | unchanged `inside_polygon` | not edited | unchanged coverage |

`measure.rs` owns units, rounding, conversion and refusal. The page owns tool state, handles, drawing and the list.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Drawing tool, release | Shape kept by F-207's minimum size rules | Note added with `revision` 1, one `/measure` request, list row reads `measuring` | A refusal sets the row to its sentence and keeps the drawing |
| Mark selected | Drag a handle | Point moves. On release `revision` increases and one request is sent | A reply whose `revision` is older than the note's is dropped |
| Two edits in quick succession | Release, release | Two requests. Only the reply for the latest revision is shown | |
| Mark selected | Remove, Delete or Backspace | Note and its list row removed. A late reply finds no note and is dropped | |
| Any measured mark | Preset, W/L, invert | No request. Statistics are modality values. | Text unchanged |
| Any measured mark | Rotate, flip, zoom, pan, aspect | No request. Points are stored in image pixels. | Text unchanged |
| Slice change and back | Return to the frame | Notes and results for that file and frame are as left | No re-measure unless an edit occurs |
| 2 by 2 | Draw | Refused by F-207's `Drawing works in the single view.` | |

Invariant: a reported statistic depends only on the file, the frame, the shape and its points.

## Ownership, commit and recovery

The server holds no state of its own for this route. It reads F-216's held frame by id, and a frame miss decodes once and inserts through the path `/preview` uses. The page note map owns marks until reload. A result is committed to a note only when its revision matches. A failed request leaves the drawing and shows the refusal sentence in the list. A repeated request with the same points returns the same reply.

## Numbered acceptance inventory

Grid fixtures use a 5 by 5 value grid `v(c, r) = c + 10 * r`. Synthetic fixtures use slice 0, stored HU + 1024, intercept -1024, Pixel Spacing `0.7\0.5`, so row 0.7 mm and column 0.5 mm.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Rectangle index corners (0.5, 0.5) and (2.5, 2.5), from page (1, 1) and (3, 3), contains columns and rows 1 and 2, four samples 11, 12, 21, 22, mean 16.5, SD `sqrt(25.25)` = 5.024937810560445, min 11, max 22, area 4 px² | `rectangle_uses_pixel_centres` in `crates/ocelli-geom/tests/region.rs` and `measure_rectangle_half_pixel` in the example | Skipping the 0.5 shift, which gives nine samples and mean 22 | implementation |
| AC2 | Ellipse box page (0, 0) to (5, 5), index centre (2, 2), radii 2.5, contains 21 samples (the four corners have `dc² + dr² = 8 > 6.25`), mean 22, SD `sqrt(3434 / 21)` = 12.787642844707914, min 1, max 43, area `PI * 6.25` = 19.634954084936208 px² | `ellipse_excludes_the_four_corners` | A bounding-box rule, which keeps 25 samples | implementation |
| AC3 | Triangle index (0, 0), (4, 0), (0, 4) contains the 15 samples with `c + r <= 4`, sum 220, mean 14.666666666666666, SD 11.897712198383164, min 0, max 40, area 8 px² | `polygon_includes_its_edges` | A strict interior rule, which drops the 12 boundary samples and keeps 3 | implementation |
| AC4 | The same triangle through `region_contains` and through `ocelli-seg`'s `edit_labelmap_with_contour` on a 5 by 5 grid covers the same 15 samples | `region_matches_the_f085_rule` in `crates/ocelli-seg/tests/contour_edit.rs` with `ocelli-geom` as a dev-dependency | Two polygon rules that drift | implementation |
| AC5 | A shape wholly outside the grid is `None` from `region_bounds` and `degenerate` from the route. An ellipse radius 0 is `Degenerate`. A NaN vertex is `Frame`. | `region_refusals` | Mean 0 for an empty region | implementation |
| AC6 | Synthetic rectangle page (69, 48) to (71, 49) covers columns 69 and 70 of row 48, values 40 and 700 HU, mean 370.0 HU, SD 330.0 HU, min 40, max 700, area `1.0 mm * 0.7 mm` = 0.70 mm², source `pixel-spacing` | `measure_route_synthetic_rectangle` | Statistics in grey bytes, or an area without spacing | integrated |
| AC7 | Synthetic angle page arm (14, 10), vertex (10, 10), arm (14, 14): mm arms (2, 0) and (2, 2.8), angle `atan(1.4)` = 54.46232220802562°, text `54.5°` | `measure_angle_uses_physical_spacing` | Pixel-space angle 45° | implementation |
| AC8 | Synthetic length page (10, 10) to (14, 14) is `hypot(2, 2.8)` = 3.4409301068170506 mm, text `3.4 mm`. Page (10, 10) to (14, 10) is 2.0 mm and (10, 10) to (10, 14) is 2.8 mm. PS3.3 C.7.6.2 row then column. | `measure_length_is_row_then_column` | Swapped spacing, which gives 2.8 mm for the horizontal segment | implementation |
| AC9 | Cobb lines (10, 10) to (14, 10) and (20, 20) to (24, 16): mm vectors (2, 0) and (2, -2.8), angle 54.5° | `measure_cobb_acute` | Returning the obtuse 125.5° | implementation |
| AC10 | Bidirectional long (10, 10) to (18, 10) and short (14, 8) to (14, 12): length 4.0 mm, width 2.8 mm | `measure_bidirectional_lengths` | Swapping the two | implementation |
| AC11 | Imager Pixel Spacing `0.2\0.2` with ERMF 2 gives source `imager-spacing` and 0.1 mm spacing. Without ERMF it gives `detector`, `mm at the detector`, 0.2 mm. With neither it gives `uncalibrated` and `px`. A Pixel Spacing of `0\0.5` is refused code `calibration` and does not fall through. | `measure_calibration_provenance` | Multiplying by ERMF, or falling through an invalid Pixel Spacing | implementation |
| AC12 | A padded CT region with two of four samples equal to Pixel Padding Value reports `count` 2 and `padding_excluded` 2. All four padded is `degenerate`. PS3.3 C.7.5.1.1.2. | `measure_excludes_padding` | Including -3024 HU padding in the mean | implementation |
| AC13 | A figure-of-eight freeform is refused code `self-intersecting`. A convex freeform is measured. | `measure_refuses_a_crossing_outline` | A shoelace area beside an even-odd mean | implementation |
| AC14 | A box over `4096 * 4096` samples, or a freeform whose box samples times vertices exceed `2^28`, is refused code `too-large` before allocation | `measure_bounds_the_region` | A 512 MiB allocation | implementation |
| AC15 | A body over 256 KiB, a non-ASCII byte, a wrong point count for the shape, or a malformed pair is 400 code `points` | `measure_points_body_is_bounded` | Reading an unbounded body, or accepting a file as points | implementation |
| AC16 | Window headers on `/measure` do not change the statistics | `measure_ignores_window_headers` | Statistics after VOI | integrated |
| AC17 | Page contract: the tools Angle, Cobb, Bidirectional, Arrow and Probe exist, `fetch("/measure"` exists, the four new sentences exist, `Math.hypot(dCol * facts.column` is gone, and the reply is applied only on a matching `revision` | `the_page_offers_a_folder_picker` extended | A page that keeps its own length formula | implementation |
| AC18 | Draw each tool on Synthetic study in Chrome, drag a handle, and read the list update with units | Manual observation at sprint close | none claimed by Rust tests | external, sprint close |
| AC19 | Before the spacing, calibration and padding code is implemented, the reviewer checks the transcriptions above against the DICOM standard text at dicom.nema.org, current edition: PS3.3 C.7.6.2 (Pixel Spacing), C.7.6.2.1.1 (the image plane equation), C.8.11.4 (Imager Pixel Spacing), C.8.7.5 (Estimated Radiographic Magnification Factor) and C.7.5.1.1.2 (Pixel Padding Value). Any correction lands in this plan before the code | Review note naming the edition read and each section | Fixtures citing text nobody checked | reviewer, before implementation |
| AC20 | `/measure` without `X-Ocelli-File` is 400 `header`. An evicted id is 410 `file-gone`. Two measures on one id and frame report `X-Ocelli-Source` `decoded` then `held` | `measure_route_takes_only_a_held_id` | A route that posts the whole file per release, or decodes per request | implementation |

Every row from AC1 to AC12 is a hand-computed fixture and its test comment cites PS3.3 C.7.6.2 and C.7.6.2.1.1 for spacing, C.11.1 for the modality values, and the F-075, F-073, F-074, F-076 or F-195 plan for the local formula.

## Approach

1. `ocelli-geom`: add `region.rs` with `RegionShape`, `IndexBounds`, `region_contains` and `region_bounds`. The polygon test reproduces the F-085 even-odd and on-edge rule, including its `same_number` treatment of the two zeros. Re-export from `lib.rs` and extend the crate doc comment.
2. `ocelli-seg` gains `ocelli-geom` as a dev-dependency only, for AC4. Its source is not edited.
3. Example `measure.rs`: parse the points, read the held frame's samples, F-210's `HeldFrame::value` and this story's `HeldFrame::calibration` (filled in `decode_held` with the existing `spacing_pair` and `top_level_pair` and the ERMF read), call `select_calibration`, apply the table above, shift by 0.5, and call the `ocelli-geom` functions. Statistics walk only the clipped bounding box, collect included non-padding samples into one `Vec<f64>`, then call `roi_statistics` with the shape area.
4. `serve.rs`: the route, the id through F-216's `parse_header` and `Viewer::frame`, the bounded point body, and `X-Ocelli-Source`.
5. Page: the tools, handles, the revision rule, the list, the sentences. Remove `lengthText`'s arithmetic and print the reply.

Rejected: computing statistics in the page from a modality buffer, which breaks the story's notes. Rejected: making `inside_polygon` public in `ocelli-seg` and depending on it from the example, which puts a measurement rule in the segmentation crate. Rejected: a sample-count area, which drifts from `ellipse` and `polygon_area` on small regions.

## Dependencies, write ownership and resources

Declared: F-210 and F-195. F-195 is done. F-216 and F-210 must land first, because this story reads F-216's held frame and file id protocol, and F-210's `HeldFrame::value`, the unit rule and `/value`.

**Sprint build order, decided in the S46 design round on 2026-11-05.** F-217, then F-216, then F-210, then this story, then F-208, F-209, F-212, F-213, F-214 and F-215. F-119 is independent and may run in parallel. F-215 therefore lands after this story and names the five new kinds as not saved, under F-215's own out-of-scope rule.

Writes `crates/ocelli-geom/src/region.rs` (new), `crates/ocelli-geom/src/lib.rs`, `crates/ocelli-geom/tests/region.rs` (new), `crates/ocelli-seg/Cargo.toml` (dev-dependency), `crates/ocelli-seg/tests/contour_edit.rs`, `examples/folder-viewer/src/measure.rs` (new), `src/held.rs` (the `calibration` field), `src/lib.rs`, `src/serve.rs`, `src/page.html`, `tests/measure.rs` (new), `tests/preview.rs` (page contract), the root `README.md` step 8, and `docs/lld/geometry.md`.

`page.html` and `serve.rs` are shared with F-208, F-210, F-213, F-214 and F-215. Run after F-210 and serially with the other page stories. `ocelli-geom` has no other writer this sprint. No GPU.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-geom`, `cargo test -p ocelli-geom`, `cargo test -p ocelli-seg`, `cargo test -p ocelli-folder-viewer`. Mutation checks: change `<=` to `<` in the ellipse rule and confirm AC2 goes red. Remove the 0.5 shift and confirm AC1 and AC6 go red. Swap `row_mm` and `column_mm` and confirm AC7 and AC8 go red. The sprint profile is the later whole-sprint run. A change to `roi_statistics`, `select_calibration`, F-216's `decode_held`, F-210's value facts, or the page-to-index shift invalidates this evidence.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The page receives numbers and text, never a sample buffer.
- Render-loop allocation: none. One `Vec<f64>` of included samples per request on the server, bounded by AC14. The frame itself is F-216's held frame.
- unsafe: none
- Tier A (WebGPU): n/a. This is host arithmetic in `ocelli-geom`. The compute kernel of HLD section 26 is not started.
- Tier B (WebGL2): n/a, for the same reason.
- Tier C (CPU): n/a. Nothing is drawn by the core. The marks are page SVG.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, AC2, AC3 pixel-centre inclusion and the F-075 statistics on a hand grid | `crates/ocelli-geom/tests/region.rs` |
| fixture | AC4 the polygon rule equals F-085's | `crates/ocelli-seg/tests/contour_edit.rs` |
| refusal | AC5 | `crates/ocelli-geom/tests/region.rs` |
| fixture | AC6 to AC10 synthetic HU and spacing, PS3.3 C.7.6.2, C.7.6.2.1.1 and C.11.1 | `examples/folder-viewer/tests/measure.rs` |
| fixture | AC11 calibration provenance, PS3.3 C.8.11.4 and C.8.7.5 | same |
| fixture | AC12 padding, PS3.3 C.7.5.1.1.2 | same |
| refusal | AC13, AC14, AC15 | same |
| integrated | AC16 | same |
| contract | AC17 | `examples/folder-viewer/tests/preview.rs` |

## Parity surface covered

none measured. Appendix B counts 26 annotation tool classes and has no per-tool row. cornerstone3D's `Angle`, `CobbAngle`, `Bidirectional`, `ArrowAnnotate`, `Probe`, `RectangleROI`, `EllipticalROI` and `PlanarFreehandROI` are the nearest names. No cornerstone observation is claimed, and cornerstone's own inclusion rule is not asserted to match.

## Deviations

none

## LLD impact

`docs/lld/geometry.md` gains `region_contains`, `region_bounds`, the pixel-centre rule, and the note that it is the F-085 polygon rule. The example is not the published API.

## User-facing documentation impact

- `README.md`, "Start the local series viewer". Step 8 (already in the write set): Length comes from the local process, Rectangle, Ellipse and Freeform report area, mean, SD, minimum and maximum, a crossing freeform is not measured, and handles reshape a selected mark. Step 5's drawing tools gain Angle (`Q`), Cobb (`J`), Bidirectional (`B`), Arrow (`U`) and Probe (`X`).
- `examples/folder-viewer/src/use.html`, chapter 11 "Measurement": the full set gains `region_contains` and `region_bounds`, with one sentence on the pixel-centre rule and that it is the F-085 polygon rule.
- `examples/folder-viewer/src/more.html`: no change. `#histogram` names `roi_statistics` and says no view draws a histogram, which stays true. No card covers region statistics on the image page.

`use.html` joins the write set for this edit.

## Open questions

Q1. The `detector` row. Affects AC11 and the length sentence. `select_calibration` returns `Unavailable` for Imager Pixel Spacing without ERMF, while the page today shows `mm at the detector`. Options: (a) keep the example's `detector` row as planned, (b) report `uncalibrated` and `px`, which follows F-195 exactly and removes a feature the page has, (c) add a `Detector` source to `CalibrationSource` in `ocelli-geom`, which changes F-195's refusal. Owner: operator. Recommendation (a). Deferral cost: the length sentence and AC11 stay unsettled.

Q2. Polygon rule placement. Affects AC4. Options: (a) a copy in `ocelli-geom` held equal by AC4, as planned, (b) move F-085's `inside_polygon` into `ocelli-geom` and have `ocelli-seg` depend on it, which edits a done story's crate. Owner: operator. Recommendation (a) now and (b) as a backlog row, because HLD section 18's "exactly once" spirit argues for one copy.

Q3. Padding in statistics. Affects AC12. Options: (a) exclude and count, as planned, (b) include, which is what a viewer that ignores padding reports. Owner: operator. Recommendation (a).

Q4. Self-intersection. Affects AC13. Options: (a) refuse, as planned, (b) measure with the even-odd area, which needs a new area function. Owner: operator. Recommendation (a).

Q5. Text precision. Lengths and angles to 0.1, area to 0.01, mean and SD to 0.1. Options: keep, or follow the cornerstone3D 5.8.2 display precision once someone reads it. Owner: operator. Deferral cost: none for correctness. The JSON numbers carry full precision.

Q6. Standard transcription. The Pixel Spacing definition in C.7.6.2 and the C.7.6.2.1.1 equation were transcribed without a local copy of the standard. The reviewer confirms them before approval.

Q7. Transport cost. F-210 Q1 applies here with the points prefix. A measure request is one per release, not per pointer move, so the cost matters less than for the cross.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

- Q1. Answered: recommended (a), keep the example's `detector` row.
- Q2. Answered: recommended (a), a copy of the polygon rule in `ocelli-geom` held equal by AC4, and a backlog row for (b), moving `inside_polygon` into `ocelli-geom`, filed by the sprint lead at `/complete-feature`.
- Q3. Answered: recommended (a), padding samples are excluded and counted.
- Q4. Answered: recommended (a), a self-intersecting freeform is refused.
- Q5. Answered: recommended, keep the plan's precision: lengths and angles to 0.1, area to 0.01, mean and SD to 0.1. The JSON numbers keep full precision.
- Q6. Answered: the flag stays, and AC19 makes the reviewer check the transcriptions against dicom.nema.org, current edition, before implementation of that code.
- Q7. Answered by the operator, by the cross-story decision: `/measure` uses F-216's file id protocol and posts only its point list. The route spec, the consumer map, AC15 and AC20 say so. The arithmetic and the fixtures AC1 to AC14 are unchanged.

Consequence of the F-208 key answer: angle is bound to `Q`, not `G`. No question is left open.

### Operator answer and lead rulings after review pass 1

The operator answered on 2026-11-06, after `.claude/reviews/F-211-working-pass-1.md` found that PS3.3 10.7.1.1 says a projection image whose Pixel Spacing equals its Imager Pixel Spacing has not been corrected for magnification: treat it as detector plane. When the two are present and equal, the Imager Pixel Spacing path decides, so the length is corrected by the Estimated Radiographic Magnification Factor when the file has a usable one, and otherwise is reported as detector plane, not corrected for magnification. This changes F-195's choice order for that case only.

Lead rulings under the operator's delegation of the quality call: an outline is refused when two edges that are not neighbours share any point, or two neighbours fold back along one line, which replaces "proper crossing" above. A present Pixel Spacing that differs from Imager Pixel Spacing decides on its own, and a later source cannot veto it. The page's outline point cap is the server's, tied by a test.

Lead ruling on the implementation of that answer: on the imager path, whether reached from Imager Pixel Spacing alone or from an equal pair, an absent magnification factor gives the detector plane and a present factor that cannot be used refuses with `calibration`, the same on both routes. An unequal Pixel Spacing decides on its own and no later source is read.

### Lead ruling after review pass 2

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-211-working-pass-2.md`. There is one definition of an equal spacing pair, the absolute per-half tolerance `ocelli-dicom` already used for `ImagerAgrees`. It moves to `ocelli-core` as `spacing_pair_agrees` so that `ocelli-dicom` and `ocelli-geom` share it, and its value does not change. The write set therefore adds `crates/ocelli-core/src/spacing.rs`, its re-exports and `docs/lld/core-types.md`, and the `ocelli-dicom` spacing comments, and the LLD impact adds `docs/lld/core-types.md` and `docs/lld/dicom-ingest.md`. An equal pair is chosen only when Pixel Spacing is itself valid.
