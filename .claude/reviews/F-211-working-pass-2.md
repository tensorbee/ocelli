# F-211 review, pass 2

**Reviewed**: `git diff 333d1fe HEAD` on `work/f-211-xai` in `ocelli-wt-f-211`, HEAD `20bca9fbb5498a05e728d98d3364a3f0b28832d5` (b6c6735, 5a1b04e, 815692e, 94a4065, 20bca9f, 14 files, +537 -91). The tree was clean at the start and at the end, apart from the untracked `.claude/reviews/F-211-working-pass-1.md`, which is not under review.
**Author / reviewer**: A Claude implementer agent wrote b6c6735, 815692e and 20bca9f. The lead wrote the rulings in 5a1b04e and 94a4065. This pass is by an independent Claude reviewer subagent that wrote none of the F-211 code, plan or rulings and did not do pass 1.
**Result**: 5 defects, 1 smell, 6 nitpicks

## Acceptance challenged

- **PS3.3 2026d, read now at dicom.nema.org (sect_10.7).**
  - 10.7.1.1: an uncalibrated Pixel Spacing "shall be the same as in Imager Pixel Spacing (0018,1164) or Nominal Scanned Pixel Spacing (0018,2010)". Different values mean the image "has been corrected for known or assumed geometric magnification or calibrated".
  - 10.7.1.2 is Pixel Spacing Calibration Type, GEOMETRY or FIDUCIAL.
  - 10.7.1.3: row spacing first, then column spacing.
  - C.8.11.4 and C.8.7.5 match pass 1's transcription.
  - C.7.6.1.1.5 is **Lossy Image Compression**. This matters for D5.
- **The pass 1 closures, replayed through the real caller.**
  - D1: `measure_frame` refuses the five-point ring through a vertex with `self-intersecting`.
  - D2: Pixel Spacing `0.5\0.5` beside ERMF `two`, and beside a one-value IPS, both give `pixel-spacing` 2.0 mm.
  - D3: the equal pair gives `detector` 0.8 mm at the detector with no factor, `imager-spacing` 0.4 mm with factor 2, and `calibration` with `two`, `0`, `-2` or `2\2`.
  - S1: the LLD and the docstring now describe the touch rule.
- **Calibration routes against the rulings.** I ran every route through `select_calibration` and through `measure_frame` from synthetic DS text:
  - Unequal Pixel Spacing decides alone, also with a NaN or 0 factor or an unusable IPS.
  - Equal pair: a usable factor corrects, an absent factor gives detector, an unusable factor refuses.
  - IPS alone: an absent factor gives detector, 0 or `two` refuses, 2 gives 0.4 mm.
  - ERMF alone refuses.
  - Equality is on the parsed f64. `0.200\0.20` and `2e-1\2E-1` against `0.2\0.2` both give detector.
  - `-0.0` against `0.0`, NaN against NaN, and an equal negative pair all refuse, as they did before.
  - Only one copy of the calibration choice decides a measurement. A second copy of the 10.7.1.1 *classification* lives in `ocelli-dicom` (S1).
- **The only other caller.** `ocelli-example-server` `/calibration` sends IPS + ERMF + ultrasound + manual with no Pixel Spacing. It still gives `imager-spacing` 0.1, the same as before. F-195's tests are unchanged and pass.
- **Crossing rule, every case the brief lists, through `polygon_crosses_itself`, `polygon_area` and the pixel-centre walk.** Each case is listed as refused or not, then area and sample count:

  | Case | Result | Area | Samples |
  |---|---|---|---|
  | T touch | refused | 8 | 17 |
  | Edge through vertex | refused | 0 | 17 |
  | Collinear overlap | refused | 0 | 5 |
  | Neighbour fold | refused | 3 | 9 |
  | Spike out and back | refused | 16 | 25 |
  | Straight run | measured | 9 | 16 |
  | Zero-length edges | measured | 16 | 25 |
  | Duplicate closing point, also doubled at the start | measured | 16 | 25 |
  | Tiny triangle | measured | 0.005 | 1, the vertex centre |
  | All-collinear rings (3, 5 and 3 points) | all refused | | |
  | `A B A` | refused | | |
  | All three points identical | measured | 0 | 1 (N6) |

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at 20bca9f, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-geom -p ocelli-dicom -p ocelli-seg -p ocelli-example-server` | same | now | 0. 710 passed, 0 failed, 3 ignored |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `gate bench` | same, `OCELLI_PORT=0` | now | 0, 0 |
| `guard_census.py`, `--check-runbook` | same | now | 0, "0 watched by nothing". 0 |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (878 files), 0 (1738 files) |
| F-216 `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file`, F-210 AC12 `value_and_preview_agree_on_stage_one` | scratch copy `target/rv211p2`, main checkout `corpus/data` symlinked in, own target dir | now | 3 passed. The corpus pair took 0.35 s and 2.78 s, so they read the corpus. AC12 took 0.02 s and is synthetic, so it does not need the corpus |
| Probe tests `review_probe_*` in geom and the viewer | same scratch copy | now | outputs quoted in this report |
| Mutation catalogue, 23 mutations | same scratch copy. After each one the file was restored and touched | now | listed below |
| Cleanup | symlink unlinked, then the copy deleted with Python `shutil` | now | `git status` shows only the pass 1 file. HEAD is 20bca9f. Main `corpus/data` is intact |

| Mutation | Result |
|---|---|
| Wraps pair treated as non-neighbours | red, `a_figure_of_eight_crosses_itself` |
| `within` strict | red, same |
| All four collinear touch terms dropped | red, `measure_refuses_a_crossing_outline`, and the geom probe flips the through-vertex and overlap rings |
| Equal pair ignores the factor | red, `measure_calibration_provenance` |
| Equal pair, unusable factor goes to detector | red, same |
| Viewer drops an unparseable factor beside Pixel Spacing | red, same |
| Viewer refuses an unusable IPS beside Pixel Spacing | red, same |
| Viewer labels Detector as patient `mm` | red, same |
| Page `OUTLINE_CAP` 4001 | red, `the_page_offers_a_folder_picker` |
| Page sentence 4001 | red, same |
| Server `FREEFORM_POINT_CAP` 4001 | red, `measure_points_body_is_bounded` |
| Ellipse widening removed | red, `ellipse_bounds_keep_a_rounded_boundary_sample`. Pass 1 N1 is closed |
| **`same_pair` compares the column only** | **green (D1)** |
| **Zero-length edges not skipped** | **green (D2)** |
| **Next-neighbour fold returns false** | **green (D2)** |
| Wrap fold returns false | green, equivalent (N3) |
| Fold `dot < 0` to `<= 0` | green, equivalent: a collinear pair of edges with length has a non-zero dot |
| `kept < 2` returns true | green, only the all-identical ring reaches it (N6) |
| Page `OUTLINE_LONG` branch deleted | green (N2) |

My first attempt at the collinear-terms mutation dropped only one of the four terms, because of operator precedence. It went green. The corrected mutation above went red. The first attempt is recorded here and is not counted.

## Defects

### D1, `same_pair`'s row half has no test: a column-only comparison labels a corrected image as detector plane
**Where**: `crates/ocelli-geom/src/calibration.rs:155-157`. The tests are `crates/ocelli-geom/tests/calibration.rs:143-155` and `examples/folder-viewer/tests/measure.rs` "unequal".
**What**: Deleting `left.row.total_cmp(&right.row).is_eq() &&` leaves every test green. Every "differs" fixture differs in the column (`0.2\0.18`) or in both halves (`0.5\0.5`). With the mutant, Pixel Spacing `0.18\0.2` beside IPS `0.2\0.2` returns `("detector", 0.2, 0.2)` instead of `("pixel-spacing", 0.18, 0.2)`. The source is wrong and the row spacing is wrong by 11 percent.
**Why it is wrong**: PS3.3 10.7.1.1 says values that differ mean the image has been corrected. The lead ruling says "A Pixel Spacing that differs in either half decides on its own". The test comment reads "A pair that differs in one half", so it is counted as covering either half, yet it passes both ways (microscope section 2).
**Evidence**: The mutation together with probe `CAL ps 0.18\0.2 vs ips 0.2\0.2 row differs`. The unmutated tree gives `pixel-spacing`.
**Scope and regression**: Every DX, CR, MG and XA measurement with a corrected Pixel Spacing whose column happens to equal the IPS column. The code on HEAD is correct, and the gap is in the tests only. The smallest regression is a row-only-differs case, `0.18\0.2` against `0.2\0.2`, expected `pixel-spacing` 0.18 and 0.2, in the library test or in the viewer's AC11.

### D2, two branches of the new crossing rule pass both ways: the zero-length skip and the next-neighbour fold
**Where**: `crates/ocelli-geom/src/region.rs:262-269` (the skip) and `:283-285` (`folds_back(a, b, d)`). The tests are `crates/ocelli-geom/tests/region.rs` `a_figure_of_eight_crosses_itself`.
**What**:
- **The skip.** Removing it leaves every test green. The mutant then **refuses** a plain square with a duplicate closing point, `(0,0),(4,0),(4,4),(0,4),(0,0)`, and a square with repeated consecutive vertices. HEAD measures both at area 16 with 25 samples. No fixture has a repeated consecutive vertex, although the docstring and `docs/lld/geometry.md` both state the skip as behaviour.
- **The fold.** Replacing `folds_back(a, b, d)?` with `false` leaves every test green. The only fold fixtures (`(0,0),(4,0),(2,0)` and the four-point fold) are also caught by the wrap fold or by a non-neighbour touch. The mutant then **measures** the flat ring `(0,0),(1,0),(-2,0)` with area 0 and 2 samples. HEAD refuses it. "A flat ring that folds back" says "only the fold rule can refuse it", and it is counted as covering the fold rule, but it does not pin the branch that is the fold rule's sole detector for a three-edge ring.
**Why it is wrong**: The lead ruling makes a fold a refusal and a straight or repeated vertex not one, and each branch carries half of that. Microscope section 2 counts a test that passes both ways as a defect.
**Evidence**: The two mutations together with probes `zero-length edges`, `duplicate closing point` and `flat +1 -3 +2`.
**Scope and regression**: The route and handle drags. A handle can be dropped on its neighbour's pixel, and a route client can send a closing point. HEAD is correct and the gap is in the tests only. The smallest regressions are:
- Expect `Ok(false)` for the duplicate-closing-point square.
- Expect `Ok(true)` for `(0,0),(1,0),(-2,0)`. Both of its folds are next pairs, at `(1,0)` and `(-2,0)`, and the wrap pair continues straight.

### D3, README step 8 says an unusable factor is not measured "with or without Pixel Spacing", and beside an unequal Pixel Spacing it is measured
**Where**: `README.md:241-243`.
**What**: "A factor that is present but cannot be used is not measured, with or without Pixel Spacing." A file with Pixel Spacing `0.5\0.5`, IPS `0.2\0.2` and ERMF `two` (or `0`) is measured as `pixel-spacing` 2.0 mm. The tree's own test asserts this: `measure_calibration_provenance`, "an unusable factor after Pixel Spacing", the pass 1 D2 fix. Only an *equal* Pixel Spacing, or none at all, leads to the refusal.
**Why it is wrong**: The sentence is false as written. It also contradicts the lead ruling: "An unequal Pixel Spacing decides on its own and no later source is read". README is an always-in-scope document.
**Evidence**: Probe `VIEW ps 0.5 + ips 0.2 + ermf two`, `"2.0 mm"`.
**Scope and regression**: A reader of the README who has a DX file with a stray ERMF. The fix narrows the sentence to "beside an equal Pixel Spacing or with no Pixel Spacing", or adds that a Pixel Spacing that differs decides on its own. `use.html` already says the latter.

### D4, the public API docs still say every `Calibration` is a patient spacing and that Detector is chosen when no *usable* factor is present
**Where**: `crates/ocelli-geom/src/calibration.rs`:
- `:29-31` (`CalibrationSource::Detector`): "Chosen when Pixel Spacing equals Imager Pixel Spacing ... and no usable magnification factor is present."
- `:80` (`Calibration`): "The selected source and the patient spacing in millimetres."
- `:7` (module): "Pixel spacing is the patient spacing of PS3.3 C.7.6.2."

**What**:
- After 20bca9f, a present unusable factor refuses (`Frame` for NaN, `Unavailable` for 0 or -2). So Detector is chosen only when the factor is **absent**, and "no usable" is false.
- A `Calibration` whose source is `Detector` holds detector-plane spacing, which `docs/lld/calibration.md` and `use.html` both say "is not a patient spacing". `Calibration` is the struct every caller reads.
- An equal Pixel Spacing is, by 10.7.1.1, not a patient spacing either.

**Why it is wrong**: These are prose claims that are false. The rustdoc is the contract a library caller of `select_calibration` reads, and a caller that trusts `:80` will label a `detector` result as patient millimetres. That is exactly the mislabel the operator answer exists to prevent.
**Evidence**: Probe `CAL ps + equal ips, factor NaN: Err(Frame)`. Reading `:80` next to `detector_plane`.
**Scope and regression**: Library consumers. The folder viewer is unaffected because it matches on `Detector`. The fix is three doc edits:
- "and no magnification factor is present".
- "the spacing in millimetres, a patient spacing unless the source is `Detector`".
- A module sentence naming the 10.7.1.1 exception.

### D5, `use.html` chapter 10 now says, a few lines before F-211's new paragraph, that the two spacings "differ by the magnification factor", and it cites Lossy Image Compression
**Where**: `examples/folder-viewer/src/use.html:671`.
**What**: "Pixel Spacing and Imager Pixel Spacing stay distinct, PS3.3 C.7.6.1.1.5. On a projection radiograph they differ by the magnification factor, so one is never substituted for the other." F-211 added `:684`, which says "A Pixel Spacing equal to the Imager Pixel Spacing has not been calibrated for geometric magnification, PS3.3 10.7.1.1, so the imager path decides". So the chapter now states both that the two always differ and that they can be equal. PS3.3 2026d C.7.6.1.1.5 is Lossy Image Compression. The relationship is 10.7.1.1.
**Why it is wrong**: The sentence is false by 10.7.1.1, which says an uncalibrated image carries equal values. It is also stale beside the diff's own paragraph, and microscope section 3 says "check that a change in the diff did not leave one of them stale". The sentence comes from 965f6fb (S46 polish), and the citation goes back to F-020's 3696dc5 in `ocelli-dicom`. The text predates F-211, but F-211's edit to this chapter is what made it contradict.
**Evidence**: WebFetch of PS3.3 2026d sect_C.7.6 (its C.7.6.1.1.5 is Lossy Image Compression) and sect_10.7.
**Scope and regression**: The Use book reader. The smallest fix is to cite 10.7.1.1 and to say the two differ when Pixel Spacing has been corrected. The same wrong citation sits in `crates/ocelli-dicom/src/frame_geometry.rs:137,173` and `docs/lld/dicom-ingest.md:174`. Those are outside this diff and are handed off, not counted.

## Smells

### S1, a second classifier of the PS3.3 10.7.1.1 relationship disagrees with `select_calibration` about what "equal" is
**Where**: `crates/ocelli-dicom/src/frame_geometry.rs:622-636` (`spacing_evidence`, `SPACING_EQUALITY_MM = 1e-9`, giving `SpacingRelationship::ImagerAgrees` "so no calibration is implied") against `crates/ocelli-geom/src/calibration.rs:155` (`same_pair`, exact `total_cmp`).
**What**: Pixel Spacing `0.1430000001\0.1430000001` and IPS `0.143\0.143` differ by 1e-10. That is legal DS text, 12 characters. `ocelli-dicom` calls the pair `ImagerAgrees`, uncalibrated. `select_calibration` and the viewer call it `pixel-spacing` and report patient mm (probe: `0.6 mm`). The same file thus gets opposite 10.7.1.1 answers from two public APIs. `use.html` chapter 10 presents both APIs side by side.
**Mechanism**: F-211 makes the equality decision load-bearing for the label, while `ocelli-dicom` is the crate that `FrameGeometry`, and any later viewer route, reads spacing from. A caller that switches to `SpacingEvidence::relationship()` to choose the label changes the answer in that band without any test noticing. `docs/lld/dicom-ingest.md:243` says the tolerance "only absorbs a differing number of trailing zeros". Trailing zeros already parse to identical f64 values, so that is not what it absorbs.
**Regression**: One rule for "equal", shared or cross-referenced, with a fixture in the 1e-10 band that pins both crates to the same answer. Alternatively, a recorded decision that the two are deliberately different.

## Nitpicks

- **N1.** `docs/lld/calibration.md:8`: "It is the only source that is not a patient spacing." This follows the sentence about factor refusal, so "It" reads as the refusal. Moving it after the `Detector` sentence fixes the reading.
- **N2.** The page's `OUTLINE_LONG` branch in `askMeasure` (`page.html:3164`) can be deleted with every test green. The pin ties the constant and the sentence to `FREEFORM_POINT_CAP`, and it does not tie their use. The consequence is only the message text (pass 1 N4).
- **N3.** In `polygon_crosses_itself`, the wrap fold (`folds_back(c, a, b)`) is never the only detector:
  - With three edges there are two sign changes, and at least one is a next pair.
  - With four or more edges, a fold at vertex 0 also gives a non-neighbour touch.
  - The mutant is equivalent. It is harmless and kept for symmetry, but it costs a reader time.
- **N4.** README step 8 and `use.html:684` attribute the converse to 10.7.1.1: "equal ... has not been corrected, PS3.3 10.7.1.1". The standard states the forward direction, and it states that different values mean the image was corrected. `docs/lld/calibration.md:8` states it accurately. Pixel Spacing Calibration Type (0028,0A02) is not read. The operator answer covers the behaviour, so only the attribution is in question.
- **N5.** `use.html:699` says "`polygon_crosses_itself` says whether a ring crosses itself", and the page's `tool-contour` sentence says "An outline that crosses itself is not measured". Both are narrower than the rule, which now also refuses touches and folds. Neither is false.
- **N6.** A freeform of three identical points is measured, area 0 with 1 sample, `kept < 2`. This matches a zero-size rectangle, which is also measured at area 0 with 1 sample, so it is consistent with the inclusive pixel-centre rule. It is pinned by no test either way.

## Verified clean

- **`select_calibration` routing.**
  - The equal-pair test happens before `accept` and is exact on the f64.
  - An equal pair that is invalid refuses exactly as the Pixel Spacing path did: NaN gives `Frame`, zero, negative or `-0.0` gives `Unavailable`.
  - An equal pair blocks ultrasound and manual (probe).
  - F-195's own cases are untouched, because none of them has an equal pair.
  - There is no `as` cast and no `#[allow]` in the diff.
- **`choose_spacing`.**
  - The Pixel Spacing branch reads IPS and the factor only to compare, mapping `Unusable` IPS to `None` and an unparseable factor to `NaN`.
  - The no-Pixel-Spacing branch keeps F-207's detector row and the refusal of an unusable factor.
  - The `detector()` helper is the single place that sets the `mm at the detector` unit.
  - Ultrasound and Manual still refuse.
  - Five mutations here all went red.
- **`polygon_crosses_itself`.**
  - The neighbour definition is over edges that have a length, and the wrap pair is excluded from the touch test (mutation red).
  - `segments_touch` covers a proper crossing and the four collinear endpoint cases (mutation red), and `within` is inclusive (mutation red).
  - The quadratic loop runs after both size caps.
  - The `Vec` is bounded by `FREEFORM_POINT_CAP`, so a request allocates at most 4000 edge pairs.
- **The cap pin.** `FREEFORM_POINT_CAP` is the route's parser cap (`serve.rs:1384`) and the shape's `accepts`. The page constant and the page sentence are both tied to it (mutations red).
- **The ellipse widening test.** It is red without the widening. The f64 claim `2.2 - 1.2 = 1.0000000000000002` is what the bound depends on.
- **Docs, executed.**
  - README step 8: the Probe sentence (`postValue`) is now true, and the sentences about the 4000 points, crosses or touches, detector and division by the factor match probes, apart from D3.
  - `use.html` chapter 10's new paragraph matches the probes, apart from N4.
  - `calibration.md` matches the code, apart from N1.
  - The `geometry.md` crossing paragraph matches the code and the mutations. "Last updated" is bumped (pass 1 N2 is closed).
  - `more.html` has no calibration sentence. A grep for calibrat, imager, magnif and detector found none.
- **Boundary and tier.** There is no wasm-bindgen, no unsafe, no pixel crossing and no render-loop change. Tier A, B and C are not applicable, as pass 1 found.

## Limitations and handoff

- **Not run.** No browser, no live viewer and no Playwright, so AC18 and the page's message branch were not observed. No oracle or GPU job.
- **Not checked.** The functional-group IPS and ERMF limitation carries over from pass 1. cornerstone3D's handling of equal spacings was not checked, and nothing here claims it.
- **Carried outside this diff.** The C.7.6.1.1.5 citation in `ocelli-dicom` `frame_geometry.rs:137,173` and `docs/lld/dicom-ingest.md:174`, and the `dicom-ingest.md:243` tolerance sentence. These go to the lead as follow-up, not as F-211 blockers. The `use.html` instance is D5 because it sits beside F-211's paragraph.
- **Next owner.** The implementer fixes D1 to D5 and S1 (or the lead records a decision on S1). A fresh pass 3 then reviews the changed material.
- This is a bounded review of the diff. It is not acceptance of AC18 and not sprint readiness.
