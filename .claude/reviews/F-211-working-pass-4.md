# F-211 review, pass 4

**Reviewed**: `git diff ac50251 HEAD` on `work/f-211-xai` in `ocelli-wt-f-211`, HEAD `d8034cd57d5f93bcd3b19b148821089f382688c4` (a6f306c, the lead's ruling, and d8034cd, the pass 3 fixes, 11 files, +84 -22). I also swept the whole F-211 diff once as `git diff f50da1e HEAD -- . ':!.claude/reviews'`, 30 files, +3825 -130. HEAD merges f50da1e, so this is `bb0419f..HEAD` without what sprint/s46 brought in through ac50251. The tree was clean at the start and at the end, apart from the untracked pass 1 to 3 reports, which are not under review.
**Author / reviewer**: The lead wrote a6f306c. A Claude implementer agent wrote d8034cd. This pass is by an independent Claude reviewer subagent that wrote none of the F-211 code, plan or rulings and did none of passes 1 to 3.
**Result**: 2 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **Pass 3 D1, replayed through `select_calibration`.** The probe ran in a scratch `git archive HEAD` copy:

  | Pixel Spacing, IPS, factor | HEAD |
  |---|---|
  | `0\0`, `5e-10`, none / 2 / NaN / 0 | `Unavailable` in all four cases |
  | `-5e-10`, `5e-10`, none | `Unavailable` |
  | `5e-10`, `0\0`, none / 2 | `Unavailable` (the reverse order: imager path decides, imager refused) |
  | `5e-10`, `-5e-10`, none | `Unavailable` |
  | `0\0`, `0\0` and `-5e-10`, `-5e-10` | `Unavailable` |
  | `NaN\0.2`, `NaN\0.2` | `Frame` (NaN never agrees, so Pixel Spacing decides) |
  | NaN, `0.2` | `Frame` |
  | `0.2`, NaN, and `0.2`, `NaN\0.2` with factor 2 | `Ok(pixel-spacing)`. Not equal, so the later source is not read, as F-195 already did |
  | `-0.0`, `0.0`, none / NaN / 2, and `0.0`, `-0.0` | `Unavailable`. Pass 3 N1's `Frame` is gone, because Pixel Spacing is now checked first |
  | inf, inf | `Frame` |
  | `0.2`, `0.2`, none / 2 | `detector 0.2` / `imager-spacing 0.1` |
  | `0.1430000001`, `0.143` | `detector 0.143` |

  Every row agrees with the rewritten rustdoc and with `calibration.md:8`.
- **Through the real caller.** I called `measure_frame` on synthetic files. Pixel Spacing `0\0` beside IPS `5E-10\5E-10` refuses `calibration`, with no factor, with factor 2 and with factor `two`. So do `-5E-10` beside `5E-10`, `5E-10` beside `0\0`, `5E-10` beside `-5E-10`, `0\0` beside `0\0` and `-0.0` beside `0.0`. Pixel Spacing `0.2` beside an unparseable IPS `x\y` is measured as `pixel-spacing`, 0.8 mm. The viewer maps an unusable imager to absent beside Pixel Spacing, and its doc comment says so ("Only an equal imager pair can change the choice"). Pass 3 D1 is closed.
- **"The two APIs give one answer for one file."** `ocelli-dicom` builds both pairs with `PixelSpacing::new`, which refuses a value that is not strictly positive, so it refuses each invalid row above. `select_calibration` refuses each one too.
- **PS3.3 2026d, fetched now from dicom.nema.org.** The `2026d/output/chtml` path returns 404. `current/output/chtml` returns 200, and its page header says "DICOM PS3.3 2026d".
  - 10.7.1.1: an uncalibrated Pixel Spacing "shall be the same as in Imager Pixel Spacing (0018,1164) or Nominal Scanned Pixel Spacing (0018,2010)". Values that differ mean the image "has been corrected for known or assumed geometric magnification or calibrated".
  - C.8.11.4: Imager Pixel Spacing is "Physical distance measured at the front plane of the detector housing" and "shall never be adjusted to account for correction for the effect of geometric magnification or calibration against an object of known size".
  - C.8.7.5: the factor is the "Ratio of Source Image Receptor Distance (SID) over Source Object Distance (SOD)".
  - The new C.8.11.4 citations at `frame_geometry.rs:266`, the test at `:571` and `dicom-ingest.md:184` quote C.8.11.4 correctly. Pass 3 D3 is closed. See N1 and N2 for two looser phrasings.
- **AC mapping.** Each implementation AC names a test. Each test exists in the named file and passed in this run's 763:
  - AC1: `rectangle_uses_pixel_centres` and `measure_rectangle_half_pixel`.
  - AC2: `ellipse_excludes_the_four_corners`.
  - AC3: `polygon_includes_its_edges`.
  - AC4: `region_matches_the_f085_rule` (ocelli-seg).
  - AC5: `region_refusals`.
  - AC6: `measure_route_synthetic_rectangle`.
  - AC7: `measure_angle_uses_physical_spacing`.
  - AC8: `measure_length_is_row_then_column`.
  - AC9: `measure_cobb_acute`.
  - AC10: `measure_bidirectional_lengths`.
  - AC11: `measure_calibration_provenance`.
  - AC12: `measure_excludes_padding`.
  - AC13: `measure_refuses_a_crossing_outline`.
  - AC14: `measure_bounds_the_region`.
  - AC15: `measure_points_body_is_bounded`.
  - AC16: `measure_ignores_window_headers`.
  - AC17: `the_page_offers_a_folder_picker` (tests/preview.rs).
  - AC20: `measure_route_takes_only_a_held_id`.
  - **AC18 is external**: a browser observation at sprint close, not run here.
  - AC19 is a reviewer check. I redid it for C.8.11.4, C.8.7.5 and 10.7.1.1 above. I did not re-read C.7.6.2, C.7.6.2.1.1 or C.7.5.1.1.2 in this pass.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at d8034cd, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-core -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-geom -p ocelli-dicom -p ocelli-seg -p ocelli-example-server` | same | now | 0. 763 passed, 0 failed, 3 ignored |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `gate bench`, `gate unsafe`, `gate nostd` | same, `OCELLI_PORT=0` | now | 0, 0, 0, 0 (nostd: 7 crates) |
| `guard_census.py`, `--check-runbook` | same | now | 0, "0 watched by nothing". 0 |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (883 files), 0 (1744 files) |
| F-216 `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file` | scratch `git archive HEAD` copy, main checkout `corpus/data` symlinked in, target `<wt>/target/p4-scratch` | now | 2 passed in 2.83 s |
| Probes `review_probe_p4` (geom) and `review_probe_p4_view` (viewer) | the scratch copy | now | tables above |
| Mutation catalogue | the scratch copy. After each mutation the file was restored, `cmp` showed it identical to HEAD, and it was touched | now | below |
| Cleanup | symlink unlinked, scratch copy and `target/p4-scratch` deleted with Python `shutil` | now | `git status` shows only the three earlier reports. HEAD is d8034cd. Main `corpus/data` is intact |

| Mutation | Result |
|---|---|
| M1, drop the new Pixel Spacing `accept` on the equal branch (pass 3 D1 reintroduced) | **red**: `an_invalid_pixel_spacing_is_refused_beside_an_agreeing_imager_pair`, `measure_calibration_provenance` |
| M6, equal branch checks only `< 0` rather than `<= 0` | red: `measure_calibration_provenance` |
| M2, equal branch checks the imager path first, then Pixel Spacing | **green** (S1) |
| M3, equal branch falls back to Pixel Spacing when the imager pair is not positive | **green** (S1) |
| M4 / M5, equal branch checks only the row / only the column half of Pixel Spacing | **green** / **green** (S1) |
| M7, viewer IPS-alone detector row: positivity check removed | **green** (D1) |
| M8, viewer IPS-alone detector row: finiteness check removed | **green** (D1) |
| M9, viewer IPS-alone: an unparseable imager pair treated as absent | **green** (D1) |

## Defects

### D1, the viewer's Imager-Pixel-Spacing-alone refusals are pinned by no test
**Where**: `examples/folder-viewer/src/measure.rs`, `choose_spacing`. These are the `pair(inputs.imager_spacing)?` in the no-Pixel-Spacing branch and the `!raw.row.is_finite() || ... || raw.column <= 0.0` check before `detector(raw)`.
**What**: IPS alone, with no Pixel Spacing and no factor, is the example's own `detector` row, by the Q1 answer. At HEAD, IPS `0\0`, `-0.2\-0.2`, `x\y` or a single value `0.2` each refuse `calibration`, which is correct. But removing the positivity check (M7), the finiteness check (M8), or the refusal of an unparseable pair (M9) leaves the whole `ocelli-folder-viewer` suite green. Under M7, IPS `0\0` is measured as `0.0 mm at the detector`. Under M9, an unreadable IPS is measured as `uncalibrated` in px.
**Why it is wrong**: Microscope section 4, a guard that nothing goes red without. The function's rustdoc claims the behaviour: "A present source that is chosen and cannot be used refuses." AC11's refusal half covers Pixel Spacing `0\0.5` and an unusable or zero factor. It does not cover an unusable imager pair on the detector row, which F-211 introduced.
**Evidence**: M7, M8 and M9 are green with `cargo test --no-fail-fast -p ocelli-folder-viewer`. The HEAD behaviour comes from `review_probe_p4_view`.
**Scope and regression**: Viewer consumers of `/measure`. HEAD behaves correctly, but the refusal is unprotected. The smallest regression is to add `spaced(&[(IMAGER_SPACING, "0\\0")])` and `spaced(&[(IMAGER_SPACING, "0.2")])`, or `"x\\y"`, to the `expect_refusal` loop in `measure_calibration_provenance`. M8 may be unreachable from a file, since the Decimal String parser is unlikely to yield a non-finite number. I did not establish that, so a direct unit row is optional.

### D2, the `ocelli-geom` crate doc still says `polygon_crosses_itself` finds a proper crossing
**Where**: `crates/ocelli-geom/src/lib.rs:26`, added in the F-211 diff: "[`polygon_crosses_itself`] finds a proper crossing of two edges."
**What**: The lead ruling after pass 1 (plan `:327`) replaced "proper crossing" with refusing any shared point between non-neighbours, plus a fold-back of neighbours. The function's own rustdoc (`region.rs:233-245`), `geometry.md:150-154`, `use.html` chapter 11 and README step 8 ("crosses or touches itself") all say so. The crate-level summary still describes the superseded rule, so a reader of `cargo doc`'s front page would expect a touching ring to be accepted.
**Why it is wrong**: Microscope section 3, a false current-state sentence in prose this diff added.
**Evidence**: `grep -n proper crates/ocelli-geom/src/lib.rs crates/ocelli-geom/src/region.rs`. Pass 2's figure-of-eight and touch tests pin the actual behaviour.
**Scope and regression**: Documentation only. Fix it with "finds where a ring crosses or touches itself", matching `region.rs:233`. No test is needed.

## Smells

### S1, the equal branch's "both checked, in that order" is pinned only for a symmetric invalid Pixel Spacing beside a usable or absent factor
**Where**: `crates/ocelli-geom/src/calibration.rs:124-125` ("On the equal branch both Pixel Spacing and the imager pair are checked, in that order") and the comment at `:131-132`. The new test is `an_invalid_pixel_spacing_is_refused_beside_an_agreeing_imager_pair`.
**Mechanism**: The order is observable. Pixel Spacing `0\0` beside IPS `5e-10` with a NaN factor gives `Unavailable` (403) because Pixel Spacing is checked first. Checking the imager path first gives `Frame` (402). M2 does exactly that and is green, so a refactor that folds the Pixel Spacing check into `detector_plane` after the factor check flips the code silently and falsifies the sentence. M3 is also green: a valid Pixel Spacing beside an agreeing, non-positive imager pair could fall back to `pixel-spacing`, against the rustdoc's "both ... are checked", and nothing would notice. M4 and M5 are green because every invalid Pixel Spacing in the new test has equal halves. A one-half check, the same shape as pass 2 D1, passes it.
**Regression**: Add these rows to the new test's loop:
- `(pair(0.0, 0.0), Some(f64::NAN))` gives `Unavailable`, which pins the order.
- `(pair(0.0, 5.0e-10), None)` and `(pair(5.0e-10, 0.0), None)` give `Unavailable`, which pins both halves.
- Pixel Spacing `pair(5.0e-10, 5.0e-10)` beside IPS `pair(0.0, 0.0)` gives `Unavailable`, which pins the imager check.

All are hand-derivable from the rustdoc and C.7.6.2 / C.8.11.4. No code change is needed.

## Nitpicks

- **N1.** `dicom-ingest.md:189`: "PS3.3 10.7.1.1 says only that an uncorrected Pixel Spacing has the same values as Imager Pixel Spacing and a corrected one differs." The section also names Nominal Scanned Pixel Spacing (0018,2010) and calibration against an object, so "only" is broader than the text. The sentence is otherwise accurate.
- **N2.** "PS3.3 C.8.7.5's magnification factor relates it to the patient plane" (`frame_geometry.rs:268`, the test at `:573`, `dicom-ingest.md:186`). C.8.7.5 defines the factor as SID over SOD and does not mention Imager Pixel Spacing or a patient plane. The relation is the standard geometric inference, and `calibration.md` already states it the same way, so I did not count it.
- **N3.** Pass 3 N4 carries over: `.claude/plans/F-020-design.md` and `docs/sprints/AS_BUILT.md:2615` still cite C.7.6.1.1.5 for spacing calibration. These are historical ledgers outside this diff, for the lead.

## Verified clean

- **Pass 3's findings.**
  - D1 is closed: the probes above, and M1 is red in both geom and the viewer.
  - D2 is closed:
    - `core-types.md` says five modules, which matches `lib.rs:19-23`.
    - The `spacing` row names both re-exports from `lib.rs:28`.
    - The paragraph's callers match `git grep`: `frame_geometry.rs:447,633`, `series_open.rs:635`, `calibration.rs:177`.
    - F-211 is on both contributor lines.
  - D3 is closed: C.8.11.4 is quoted correctly, and the "differs from spacing in the image" sentence is gone from the LLD and the test.
  - S1 is closed: plan `:331-333` records the ruling, the write set, the LLD impact and "An equal pair is chosen only when Pixel Spacing is itself valid", which the code does.
  - N1 is resolved by the reorder. N3 is addressed: `use.html:671` now ends "so a measurement never reports one as the other", which follows.
- **Rewritten prose against the code.** I checked each of these against the probes:
  - the `select_calibration` rustdoc,
  - the `same_pair` doc,
  - `calibration.md:6,8,10`,
  - `use.html:671,684`,
  - README step 8, every equal-pair, IPS-alone and factor sentence,
  - `core-types.md:12-28`,
  - `dicom-ingest.md:175-191,243-249`.
- **Whole-diff sweep.**
  - Every new refusal site in `measure.rs`, `region.rs`, `serve.rs`, `held.rs` and `preview.rs` is listed by `git diff f50da1e HEAD | grep Err(`.
  - The calibration-input sites were mutated: D1, S1.
  - The region, too-large, self-intersecting, points and header refusals map to AC5, AC13, AC14, AC15 and AC20, and passes 1 and 2 mutated them.
  - `more.html` has no measurement sentence made stale by F-211. Its histogram card still matches.
  - `use.html` chapter 11's region paragraph matches `region.rs`.
- **Boundary and tier.** The pass 3 fixes add no `as` cast, no `#[allow]`, no `unsafe`, no wasm-bindgen and no render-loop code. Tiers A, B and C are not applicable.

## Limitations and handoff

- **Not run.** No browser, no live viewer and no Playwright, so AC18 stays external and unobserved. No oracle or GPU job.
- **Not re-read.** PS3.3 C.7.6.2, C.7.6.2.1.1 and C.7.5.1.1.2 were not re-fetched in this pass, and AC1 to AC10 and AC12 arithmetic was not re-derived. Earlier passes cover those.
- **Not checked.** `page.html` (+453) was not re-swept line by line beyond AC17's contract test. The functional-group IPS limitation carries over from pass 1.
- **Scratchpad.** `scratchpad/p4/` holds this pass's logs (`test.txt`, `g-*.txt`, `census.txt`, `prose.txt`, `prov.txt`) and `mut.py`, `mut2.py`. The scratch copy and `target/p4-scratch` were deleted.
- **Next owner.** The implementer fixes D1 and D2 and adds S1's rows. A fresh pass 5 reviews that material.
- This is a bounded review of a6f306c, d8034cd and one sweep of the F-211 diff. It is not acceptance and not sprint readiness.
