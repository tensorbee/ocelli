# F-211 review, pass 1

**Reviewed**: `git diff bb0419f 333d1fe` on `work/f-211-xai` in `ocelli-wt-f-211` (19 files, +3055 -80). The working tree was clean at the start and at the end, so there were no unstaged paths. The base `bb0419f` is F-210's head, which is still under review itself.
**Author / reviewer**: The implementation is by a Claude implementer agent. This review is by an independent Claude reviewer subagent that wrote none of the F-211 code or plan.
**Result**: 3 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **AC19.** I read PS3.3 **2026d** at dicom.nema.org: C.7.6.2, C.7.6.2.1.1, 10.7.1.1, 10.7.1.3, C.8.11.4, C.8.7.5 (Table C.8-30) and C.7.5.1.1.2. The plan's transcriptions are accurate, with one omission:
  - Pixel Spacing is "adjacent row spacing (delimiter) adjacent column spacing in mm".
  - In `P = S + i·Δi·X + j·Δj·Y`, `Δi` is the column spacing and `Δj` the row spacing.
  - 10.7.1.3: the first value is the row spacing and the second is the column spacing.
  - Imager Pixel Spacing is "measured at the front plane of the detector housing" and "shall never be adjusted" for magnification.
  - ERMF is "Ratio of Source Image Receptor Distance (SID) over Source Object Distance (SOD)", so imager spacing / ERMF is the patient spacing.
  - Padding is compared in stored values "before the Modality LUT Transformation", and the range is inclusive.
  - The omission is 10.7.1.1, which governs how Pixel Spacing relates to Imager Pixel Spacing. See D3.
  - The HLD (`docs/hld/`) has no measurement calibration rule. A grep for pixel spacing, calibration and magnification finds only Part 14 display calibration.
- **AC1 to AC3, AC6 to AC12.** Every fixture number was re-derived by hand or in a separate Python calculation, not from the code:
  - AC2: sum 462, squared deviations 5050 - 1616 = 3434.
  - AC3: 15 samples, sum 220, SD 11.897712198383164.
  - `atan(1.4)` = 54.46232220802562°, `hypot(2, 2.8)` = 3.4409301068170506, `PI * 6.25` = 19.634954084936208.
  - AC6: values 40 and 700 HU, area 1.0 x 0.7. AC12: -1024 and -924 HU.
  - The SD is the population form, which is what the plan and F-075 require.
- **Anisotropic spacing.** I ran a discriminating case the tests do not have, Pixel Spacing `1\2`:
  - A horizontal 4-column segment gives 8.0 mm and a vertical 4-row segment gives 4.0 mm, so `PixelSpacing[1]` scales the column (x) direction, as C.7.6.2.1.1 requires.
  - An ellipse with page box (0, 0) to (4, 2) gives area `PI * 4 * 1` = 12.566 mm² and 8 samples. By hand, index centre (1.5, 0.5) with radii 2 and 1 keeps columns 0 to 3 of rows 0 and 1.
- **Self-intersection.** I tried a crossing that passes through a vertex. The test misses it (D1).
- **Calibration precedence.** I tried an earlier usable source next to a later unusable one. It is refused (D2).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at 333d1fe | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-geom -p ocelli-dicom -p ocelli-seg` | same | now | 0. 688 passed, 0 failed, 3 ignored |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `gate bench` | same | now | 0, ALL GREEN |
| `guard_census.py`, `--check-runbook` | same | now | 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (878 files), 0 (1738 files) |
| F-216 `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file`, F-210 `value_and_preview_agree_on_stage_one` | scratch copy `target/rv211`, main checkout `corpus/data` symlinked in, own target dir | now | 3 passed in 2.78 s. The corpus tests return early without a corpus, so the run time is the evidence that they read it |
| Mutation catalogue, 17 mutations (below) | scratch copy, each mutation restored and its file touched | now | 16 red, 1 green |
| Probe test `review_probe` (D1, D2, D3 and the anisotropic cases) | scratch copy | now | outputs quoted below |
| Scratch copy deleted with Python `shutil`, corpus symlink unlinked first | | now | `git status` empty, HEAD 333d1fe, main `corpus/data` intact |

Mutations and the tests that went red:

| Mutation | Red |
|---|---|
| ellipse `<=` to `<` | `ellipse_includes_its_boundary` |
| no half-pixel shift | 7 tests, AC1 and AC6 among them |
| row and column mm swapped | AC6 to AC10, AC16 |
| crossing never reported | AC13 |
| crossing test non-strict | `a_figure_of_eight_crosses_itself` (the vertex touch) |
| padding not excluded | AC12 |
| region cap raised to 4097 * 4096 | AC14 |
| freeform cap raised by 4144 | AC14 |
| `/measure` body cap removed | AC15 |
| ERMF multiplies | AC11 |
| detector row dropped | AC11 |
| "square pixels assumed" dropped | AC11 |
| polygon on-edge test dropped | AC4 |
| rectangle strict | `rectangle_includes_a_centre_on_its_edge` |
| stored value in place of the modality value | AC6, AC12, AC16 |
| held Pixel Spacing dropped | 9 tests |
| ellipse bounds not widened by one | **nothing red** (N1) |

## Defects

### D1, a ring that crosses itself through a vertex is measured, with area 0 next to 17 samples
**Where**: `crates/ocelli-geom/src/region.rs:242-280`, `polygon_crosses_itself` and `opposite`.
**What**: The test counts only *proper* crossings, where every orientation is strictly non-zero. Take a ring whose crossing point is one of its own vertices: index points (0, 0), (2, 2), (4, 4), (4, 0), (0, 4). Edge (4, 0) to (0, 4) passes through vertex (2, 2), so one orientation is 0, `opposite` is false, and the ring is reported as not crossing. It is still a bow-tie, and its two lobes wind in opposite directions. `/measure` returns:
`"area":{"value":0,...,"text":"0.00 px²"},"stats":{"count":17,...,"mean":22,...}`.
That is the exact disagreement the plan's AC13 and Q4 answer refuse: a shoelace area of 0 beside an even-odd sample set.
**Why it is wrong**: The plan says "The shoelace area and the even-odd sample set disagree for a ring that crosses itself ... refuses a crossing ring with code `self-intersecting`". The operator answered Q4 (a), refuse. The plan's "proper crossing" wording is the mechanism and the refusal is the contract, and the mechanism misses a crossing route. The plan's own wording therefore needs correcting too.
**Evidence**: Probe `measure_frame(value_grid, 0, Freeform, [(0.5,0.5),(2.5,2.5),(4.5,4.5),(4.5,0.5),(0.5,4.5)])` returned `Ok` with the JSON above.
**Scope and regression**: The reply then shows an area of 0.00 beside a mean, and the README, `tool-contour` and the LLD say a crossing outline is not measured. The case is reachable through the route and from the page. A handle drag can put a vertex exactly on another edge, and pointer points at zoom 1 fall on a regular grid. The smallest regression is the five-point ring above, expected `self-intersecting`.
Possible fix: refuse any contact between non-adjacent edges, collinear touches included. The cost is that the harmless pinched "bow" in `a_figure_of_eight_crosses_itself` would also be refused. Alternatively, keep vertex touches but detect a crossing through a vertex by the turn sides on either side. Either way the plan's "proper crossing" sentence and the LLD paragraph change with it.

### D2, an unusable later spacing source refuses a measurement that has a valid Pixel Spacing
**Where**: `examples/folder-viewer/src/measure.rs:159-171`, `choose_spacing`.
**What**: `pair(inputs.imager_spacing)?` and `FactorInput::Unusable => return Err(...)` run before `select_calibration`. A file with a usable Pixel Spacing `0.5\0.5` is therefore refused `calibration` in two cases:
- Its Estimated Radiographic Magnification Factor is unparseable (`two`).
- Its Imager Pixel Spacing has one value (`0.2`).
**Why it is wrong**: The plan's approach is "call `select_calibration`, apply the table above". `select_calibration` (F-195) picks "the first present source". Its documented rule is that an invalid *earlier* source does not fall through. It never lets a *later* source veto an earlier one. With valid numbers in the same slots, `ps+ips+ermf` returns `pixel-spacing` 2.0 mm, so the result depends on how well-formed an attribute the rule says is never read.
**Evidence**: Probe outputs `CAL ps+bad ermf: Err(Calibration)`, `CAL ps+bad ips: Err(Calibration)`, `CAL ps+ips+ermf: Ok(... "pixel-spacing" ... "2.0 mm")`.
**Scope and regression**: This affects DX, CR, MG and XA files that carry both Pixel Spacing and a malformed IPS or ERMF. Every shape is refused, lengths included. Progress note decision 5 states the refusal but not that it overrides a valid Pixel Spacing. The smallest regression is Pixel Spacing `0.5\0.5` with ERMF `two`, expected `pixel-spacing` 2.0 mm.
Possible fix: pass an unusable input to `select_calibration` as a non-finite `SpacingMm` or factor, and let F-195's order decide.

### D3, AC19 correction: Pixel Spacing equal to Imager Pixel Spacing is labelled patient `mm`, and README step 8 claims that every number comes through `/measure`
**Where**: The plan's calibration table and `measure.rs:194-213`. Also `README.md` step 8, first sentence.
**What, part 1, normative (the AC19 finding)**: PS3.3 2026d 10.7.1.1 says: "If Pixel Spacing (0028,0030) is present and the image has not been calibrated to correct for the effect of geometric magnification, the Values of this Attribute shall be the same as in Imager Pixel Spacing (0018,1164)". Values that differ mean the image "has been corrected". So Pixel Spacing equal to Imager Pixel Spacing is detector-plane spacing, not patient spacing. The tree returns `"source":"pixel-spacing"` and `0.8 mm` for Pixel Spacing `0.2\0.2` with Imager Pixel Spacing `0.2\0.2` (probe `CAL ps==ips`). The same pair with only Imager Pixel Spacing would read `0.8 mm at the detector`. The label claims a patient length that the standard says is magnified by SID/SOD, which is about 10 percent on a chest film.
The plan transcribed C.7.6.2, C.8.11.4 and C.8.7.5 but not 10.7.1.1, and AC19 says "Any correction lands in this plan before the code". The fix changes F-195's provider order or adds a row to this story's table, so it is an operator decision. The options are:
- (a) Pixel Spacing equal to Imager Pixel Spacing takes the `detector` row, or imager / ERMF when the factor is present.
- (b) Keep `pixel-spacing` with a qualified unit.
- (c) Record a deviation.
**What, part 2, prose**: README step 8 opens "Every number on a mark comes from the local process through `POST /measure`". The Probe is a mark with a number, and it comes from `POST /value` (`askMeasure` sends `postValue` for `probe`). The sentence is false for one of the five new tools.
**Evidence**: The probe output above. For part 2: `page.html` `askMeasure`, where `shape === "probe" ? postValue(...)`.
**Scope and regression**: Projection radiography lengths, angles and areas carry a patient-mm label. The README is one of the three always-in-scope documents. Smallest regressions:
- An AC11 row for Pixel Spacing equal to Imager Pixel Spacing, once the operator decides.
- The README sentence names `/value` for Probe, or drops "Every".

## Smells

### S1, `polygon_crosses_itself` and its LLD paragraph describe the miss in D1 as a design property
**Where**: `docs/lld/geometry.md`, the new "Region inclusion" section, last paragraph. Also `region.rs:231-236`.
**What**: Both say "A touch at a vertex and a collinear overlap are not crossings" and then "The shoelace area and the even-odd samples disagree on a ring that crosses, which is why the folder viewer refuses one". A reader concludes that every ring the viewer measures has agreeing area and samples, and D1 shows one that does not. This is a maintenance hazard independent of how D1 is fixed. If D1 is fixed by tightening the test, the sentence becomes false. If D1 is fixed elsewhere, the function's documented contract stays the narrower one while consumers read the broader one.
**Regression**: The paragraph names the route a pass-through vertex takes, or the function's contract changes with D1.

## Nitpicks

- **N1.** The ellipse bound widening by one sample (`region.rs:119-122`) has no test. Removing it left every test green, and 3,000,000 random page boxes found no sample it rescues. The margin is harmless and the LLD sentence "so a rounded extent never drops a sample" is a design intent rather than something measured. Keep it, or name it as an unexercised margin.
- **N2.** `docs/lld/geometry.md` still says `**Last updated:** 2026-11-04` after this story's section was added.
- **N3.** README step 8 has one physical line much longer than its neighbours: "Padding pixels are left out. A freeform that crosses itself is not measured. Freeform keeps an outline of three or more points. Probe is a". The text is unchanged and only the wrap is off.
- **N4.** A freehand outline with more than 4000 points, about 5700 px of perimeter at the page's 1.41 px step, is refused `points`. The page then shows "the point list is not usable", which does not say the outline is too long. The page maps `too-large` to its own sentence and could map this one too.

## Verified clean

- **`region_contains`.**
  - The rectangle is inclusive on both bounds (mutation red).
  - The ellipse uses the division-free form with absolute radii, and the boundary tie is kept (mutation red). Zero radius in either sign is `Degenerate`.
  - The polygon is a faithful copy of F-085 `inside_polygon`, `on_segment`, `straddles`, `product_underflowed` and `same_number`, compared line by line with `crates/ocelli-seg/src/contour_edit.rs` and `brush.rs`. AC4 holds the two equal on two rings, one with an off-grid vertex.
- **`region_bounds`.** `span` and `lower_bound` compare `f64::from(u32)` exactly, clip to the grid, and return `None` for an empty grid, an off-grid shape and a box between centres.
- **Points and spacing in `measure.rs`.**
  - The half-pixel shift happens once (`measure.rs:273`) and cancels in lengths and angles.
  - The mm points are `(c * column_mm, r * row_mm)`. An ellipse area on mm points equals `PI * rx_mm * ry_mm`.
  - Angle, Cobb and bidirectional run on mm points when calibrated. Uncalibrated angles say "square pixels assumed".
- **Statistics.**
  - The modality value comes from F-210's `ModalityTransform::apply`. Padding is compared in stored space through `stored_integer`, excluded and counted. All-padding is `degenerate`.
  - `min` and `max` narrow back to `f32` exactly, since they are widened `f32` samples.
  - The Rust diff has no `as` cast and no `#[allow]`.
- **Size caps.** 4096 * 4096 box samples and 2^28 box-times-vertices are both checked before `Vec::with_capacity`. The self-intersection test runs after the caps. Each cap's boundary is tested by mutation.
- **`/measure` route.**
  - A missing id is `header`. An unknown or missing shape is `header`. A malformed or wrong-count body is `points`.
  - A declared body over 256 KiB is refused before the body is read (mutation red). This check comes after header parsing and before the general `BODY_CAP` check.
  - Window, preset and invert headers do not change the reply (AC16).
  - The first and second request on one id report `decoded` then `held`. Another viewer's id and an evicted id are 410 `file-gone`.
  - `/value` and `/headers` still refuse an id with a body. Only `/measure` takes one.
- **Budget.** `CalibrationInputs` owns no heap. `charged_bytes` and the pre-decode estimate at `held.rs:1482` both use `size_of::<HeldFrame>()`, so F-216's and F-210's accounting stays exact, and the F-216 budget tests pass.
- **Page.**
  - The JS does no spacing or statistics arithmetic. The remaining `facts.row` and `facts.column` uses are the display aspect, and `lengthText` is gone.
  - Points go out unchanged through `pointsAttr`, whose format the server's parser accepts.
  - A reply is applied only when `live.revision === revision`. A removed mark drops its late reply, and a move or handle release re-measures.
  - Probe uses F-210's `postValue`. The 410 path re-posts once.
  - Handles are drawn on every point of the selected non-text mark.
- **Docs.**
  - The `use.html` paragraph and the LLD inclusion rules match the code, apart from S1.
  - The README step 5 keys match `onKey` and the `data-key` attributes.
  - `more.html` needs no change, because `#histogram` still says no view draws one.

## Limitations and handoff

- **Not run.** No browser or Playwright session and no live viewer, so AC18 (the manual Chrome check at sprint close) is not observed. No oracle or GPU job.
- **Calibration attributes in functional groups.** Enhanced XA and DX Imager Pixel Spacing and ERMF inside functional groups are not read, because the tree reads only the top level. Such files report `uncalibrated px`, which is honestly labelled and outside the plan's scope.
- **cornerstone3D.** Its handling of Pixel Spacing equal to Imager Pixel Spacing was not checked, and nothing here claims to know it.
- **Next owner.**
  - The implementer remediates D1, D2, the README half of D3, and S1.
  - The sprint lead takes D3's normative half to the operator, because it changes F-195's order or this plan's table.
  - A fresh pass 2 then reviews the changed material.
- This is a bounded review of the diff, not acceptance of AC18 or sprint readiness.
