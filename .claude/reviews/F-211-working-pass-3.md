# F-211 review, pass 3

**Reviewed**: `git diff 20bca9f fb9bffe` on `work/f-211-xai` in `ocelli-wt-f-211` (13 files, +260 -48), plus the merge `ac50251` (HEAD `ac5025199c997533993cd576edc5bfaca0019d98`). The tree was clean at the start and at the end, apart from the untracked pass 1 and pass 2 reports, which are not under review.
**Author / reviewer**: A Claude implementer agent wrote fb9bffe under the lead's S1 ruling. This pass is by an independent Claude reviewer subagent that wrote none of the F-211 code, plan or rulings and did neither pass 1 nor pass 2.
**Result**: 3 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **The merge.** `git merge-tree --write-tree fb9bffe f50da1e` gives tree `73740813`, which is `ac50251^{tree}`. So ac50251 is the automatic merge with no hand resolution. `git diff fb9bffe ac50251` is 19 files, +1115 -49, the same stat as `git diff bb0419f f50da1e`, sprint/s46's side since the merge base. It brings only sprint/s46's changes.
- **One definition.** `git grep` over `crates`, `examples`, `packages`, `docs/lld` and `README.md` finds one `SPACING_EQUALITY_MM` and one comparison body, in `crates/ocelli-core/src/spacing.rs`. Its callers:
  - `ocelli-dicom` `spacing_agrees`, which `spacing_evidence`, the multiframe uniform-spacing check (`frame_geometry.rs:445`) and `series_open.rs:635` call.
  - `ocelli-geom` `same_pair`.
  - The `1e-9` in `ocelli-volume/src/lib.rs:950` is a different quantity, the duplicate projected position bound.
- **The core move against the crate's rules.**
  - `ocelli-core` is `#![cfg_attr(not(test), no_std)]`. `f64::abs` builds for no_std: `gate nostd` is green, and so is `cargo check --target wasm32-unknown-unknown` of core, geom and dicom.
  - There is no `unsafe`, no allocation and no new dependency. `ocelli-core/Cargo.toml` is unchanged, and geom and dicom already depended on core.
  - It fits the HLD section 4 row, "geometry primitives".
  - The LLD for this crate was not updated (D2).
- **Tolerance semantics against ocelli-dicom at 20bca9f.** The value is `1e-9`. The comparison is absolute, `abs() <= tol`, per half, with both halves required. NaN or infinity never agrees.
  - `spacing_evidence` now calls `spacing_agrees(imager, image_mm)`, and the old body compared `imager - image`. IEEE subtraction is exactly antisymmetric, so `abs` gives the same bits.
  - `spacing_agrees` is the same expression it was. So `ocelli-dicom` behaves byte for byte as before.
- **`select_calibration` against 20bca9f.** I ran the same probe on two scratch copies, one at HEAD and one at 20bca9f:

  | Pixel Spacing, IPS, factor | 20bca9f | HEAD |
  |---|---|---|
  | `0\0`, `5e-10\5e-10`, none | `Err(Unavailable)` | **`Ok(detector, 5e-10)`** |
  | `-5e-10`, `5e-10`, none | `Err(Unavailable)` | **`Ok(detector, 5e-10)`** |
  | `0\0`, `5e-10`, 2 | `Err(Unavailable)` | **`Ok(imager-spacing, 2.5e-10)`** |
  | `5e-10`, `0\0`, none | `Ok(pixel-spacing)` | `Err(Unavailable)` |
  | `-0.0`, `0.0`, NaN | `Err(Unavailable)` | `Err(Frame)` (N1) |
  | `-0.0`, `0.0`, 2 | `Err(Unavailable)` | `Err(Unavailable)` |
  | NaN, NaN | `Err(Frame)` | `Err(Frame)` |
  | inf, inf | `Err(Frame)` | `Err(Frame)` |
  | `0.1430000001`, `0.143` | `pixel-spacing` | `detector`, the intended S1 change |
  | `0.18\0.2`, `0.2\0.2`, 2 | `pixel-spacing` | `pixel-spacing` |

  The first three rows are D1.
- **Through the real caller.** `measure_frame` on a synthetic file with Pixel Spacing `0\0` and IPS `5E-10\5E-10` returns `"calibration":{"source":"detector","row_mm":0.0000000005,...}` and the text `0.0 mm at the detector`. At 20bca9f it refused with `calibration`. Pixel Spacing `-5E-10\-5E-10` behaves the same way. `0\0` against `0\0` still refuses.
- **PS3.3 2026d, read now at dicom.nema.org.**
  - 10.7.1.1: an uncalibrated Pixel Spacing "shall be the same as in Imager Pixel Spacing (0018,1164)". Values that differ mean the image "has been corrected for known or assumed geometric magnification or calibrated". 10.7.1 and its subsections say nothing about the detector plane or the magnification factor (D3).
  - C.7.6.1.1.5 is Lossy Image Compression.
  - C.8.11.4: IPS is "Physical distance measured at the front plane of the detector housing" and "shall never be adjusted to account for correction for the effect of geometric magnification".
- **C.7.6.1.1.5 uses left in the tree.**
  - `scripts/corpus_synth.py:807`, `scripts/tests/test_corpus_synth.py:682` and `tools/oracle/page/app.mjs:173` are about lossy compression and are correct.
  - The `.1` uses in corpus_synth are C.7.6.1.1.5.1, the Defined Terms for the method, and are correct.
  - `.claude/plans/F-020-design.md:115,127,136,384,385` and `docs/sprints/AS_BUILT.md:2615` still cite it for spacing calibration. Those are historical ledgers outside this diff (N4).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at ac50251, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-core -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-geom -p ocelli-dicom -p ocelli-seg -p ocelli-example-server` | same | now | 0. 762 passed, 0 failed, 3 ignored |
| `cargo check --target wasm32-unknown-unknown -p ocelli-core -p ocelli-geom -p ocelli-dicom` | same | now | 0 |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `gate bench`, `gate unsafe`, `gate nostd` | same, `OCELLI_PORT=0` | now | 0, 0, 0, 0 |
| `guard_census.py`, `--check-runbook` | same | now | 0, "0 watched by nothing". 0 |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (883 files), 0 (1744 files) |
| F-216 `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file` | scratch `git archive HEAD` copy, main checkout `corpus/data` symlinked in, own target dir | now | 2 passed in 2.76 s |
| Probes `review_probe_p3_calibration` (geom) and `review_probe_p3_view` (viewer) | the HEAD scratch copy and a second scratch copy at 20bca9f | now | outputs above |
| Mutation catalogue | the HEAD scratch copy. After each one the file was restored and touched | now | below |
| Cleanup | symlinks unlinked, both copies deleted with Python `shutil` | now | `git status` shows only the two earlier reports. HEAD is ac50251. Main `corpus/data` is intact |

| Mutation | Result |
|---|---|
| Core drops the row half | red: core unit, `a_later_frame_with_different_pixel_spacing_is_not_one_box`, `measure_calibration_provenance`, both geom equal-pair tests |
| Core drops the column half | red: core unit, `equal_pixel_and_imager_spacing_is_the_detector_plane` |
| `SPACING_EQUALITY_MM` 0 | red: core unit, `imager_spacing_within_the_one_tolerance_agrees`, `measure_calibration_provenance`, `an_equal_pair_is_equal_within_the_one_spacing_tolerance` |
| `SPACING_EQUALITY_MM` 1e-5 | red: core unit, the dicom and geom tolerance tests |
| `<=` to `<` | green (N2) |
| Geom `same_pair` back to exact `total_cmp`, a second definition | red: geom tolerance test, `measure_calibration_provenance` |
| Dicom `ImagerAgrees` by exact `==` | red: `imager_spacing_within_the_one_tolerance_agrees` |
| Pass 2 D1, `same_pair` column only | **red** (closed) |
| Pass 2 D2, zero-length skip removed | **red**, `a_figure_of_eight_crosses_itself` (closed) |
| Pass 2 D2, next-neighbour fold returns false | **red**, same (closed) |
| Pass 2 N2, page `OUTLINE_LONG` branch deleted | **red**, `the_page_offers_a_folder_picker` (closed) |
| The equal path also validates Pixel Spacing with `accept`, a candidate fix for D1 | green. No test pins either answer, which confirms the gap D1 describes |

## Defects

### D1, a present Pixel Spacing that is zero or negative is now measured when an Imager Pixel Spacing lies within 1e-9 mm of it
**Where**: `crates/ocelli-geom/src/calibration.rs:124-129` (`select_calibration`'s equal branch) and `:166-171` (`same_pair`).
**What**: At 20bca9f, "equal" was bit identity, so validating the imager pair in `detector_plane` also validated Pixel Spacing. With the tolerance, the two can differ in sign and in validity, and only the imager pair is checked. Three results follow:
- Pixel Spacing `0\0` beside IPS `5E-10\5E-10` returns `Ok(detector, 5e-10)`, and with factor 2 returns `Ok(imager-spacing, 2.5e-10)`.
- `-5E-10` beside `5E-10` also returns `Ok(detector)`.
- Through `measure_frame`, the first file is measured as `0.0 mm at the detector`, where 20bca9f refused it with `calibration`.

The reverse direction also changed: a valid Pixel Spacing `5e-10` beside IPS `0` used to be `pixel-spacing` and is now `Unavailable`.
**Why it is wrong**: The lead ruling for S1 asked for one definition of equal. It did not ask for an invalid present source to stop refusing. Each of the following is now false for this input:
- `docs/lld/calibration.md:6`: "An earlier source that is present but invalid is an error. It does not fall through."
- `README.md` step 8: "A Pixel Spacing that is present but not usable is not measured."
- The rustdoc `# Errors` of `select_calibration`: "`Unavailable` when ... a present magnitude is not strictly positive".
- `same_pair`'s "A validity check follows in [`accept`]". The check covers the imager pair only.
- `calibration.md:8`: "the two APIs give one answer for one file". `ocelli-dicom` refuses a negative Pixel Spacing in `PixelSpacing::new`, and `select_calibration` measures it.
**Evidence**: The probes at HEAD and at 20bca9f in the table above, through `select_calibration` and through `measure_frame`. The candidate-fix mutation is green, so no test pins either behaviour.
**Scope and regression**: The trigger needs an IPS within a nanometre of a zero or negative Pixel Spacing, so it is not a realistic file. It is still a reachable input on a public API, and it falsifies four standing sentences. The smallest fix is to apply Pixel Spacing's own validity check on the equal branch before the imager path decides, or to compare only after both pairs are accepted. The smallest regression is:
- Pixel Spacing `0\0` beside IPS `5e-10\5e-10` gives `Unavailable`, also with factor 2.
- `-5e-10` beside `5e-10` gives `Unavailable`.
Assert both in `crates/ocelli-geom/tests/calibration.rs`. One viewer row in `measure_calibration_provenance` covers the route.

### D2, `docs/lld/core-types.md` still says `ocelli-core` holds four modules and does not list `spacing`
**Where**: `docs/lld/core-types.md:5,12-21`, and the `core-types.md` row of `docs/lld/README.md:22`.
**What**:
- The LLD says "`ocelli-core` holds four modules" and tables `space`, `value`, `error` and `exact_f32`. fb9bffe adds a fifth public module and two root re-exports, `SPACING_EQUALITY_MM` and `spacing_pair_agrees`.
- The contributors line on that page and on the README row is F-001, F-005, F-018, without F-211.
- The tolerance is documented only in `dicom-ingest.md` and `calibration.md`. Neither is the LLD for the crate that now owns it.
**Why it is wrong**: It is a false current-state sentence in the living LLD for the changed crate. The brief asks for the module to be "documented in the right LLD". The F-211 plan's LLD impact names only `geometry.md`.
**Evidence**: `sed -n 12,21p docs/lld/core-types.md` against `crates/ocelli-core/src/lib.rs:17-23`.
**Scope and regression**: Readers of the core LLD. The fix is:
- "five modules".
- A `spacing` row naming `SPACING_EQUALITY_MM` and `spacing_pair_agrees`, with one sentence on what the rule is and who calls it.
- F-211 added to both contributor lists.

### D3, two of the C.7.6.1.1.5 replacements now cite 10.7.1.1 for a claim 10.7.1.1 does not make, and one keeps pass 2 D5's "differs" sentence
**Where**:
- `crates/ocelli-dicom/src/frame_geometry.rs:266`: "DICOM PS3.3 10.7.1.1. Imager Pixel Spacing is spacing at the detector plane. Substituting it ... scales every measurement by the source-to-image magnification factor".
- `crates/ocelli-dicom/tests/frame_geometry.rs:571-573`: "DICOM PS3.3 10.7.1.1. Imager Pixel Spacing is spacing at the DETECTOR plane. On a projection radiograph it differs from spacing in the image by the source-to-image magnification factor, typically 10 to 20 per cent."
- `docs/lld/dicom-ingest.md:184-186`, under the heading this diff renamed to "PS3.3 10.7.1.1", carries the same "it differs from spacing in the image by the source-to-image magnification factor" sentence.
**What**: The replacement was mechanical. 10.7.1 and its subsections, PS3.3 2026d, say nothing about the detector plane or the magnification factor (WebFetch of sect_10.7). The detector-plane definition and the never-adjusted rule are C.8.11.4. The magnification factor is C.8.7.5. Also, 10.7.1.1 says the two are *the same* when the image is uncorrected, so "it differs from spacing in the image" is the sentence pass 2 D5 found false in `use.html`. fb9bffe corrected it there and left it in the LLD and the test comment.
**Why it is wrong**: Microscope section 1: a formula or claim is checked against the cited section, and the cited section does not contain it. The project rule is that a fixture comment cites the section that states its claim.
**Evidence**: WebFetch of PS3.3 2026d sect_10.7 and sect_C.8.11.4, quoted under Acceptance challenged.
**Scope and regression**: Readers of the `ocelli-dicom` source, its tests and the ingest LLD. No behaviour changes. The fix is:
- Cite C.8.11.4 (and C.8.7.5 for the factor) at `frame_geometry.rs:266` and at the test.
- Say "differs from the patient-plane spacing" or "from a corrected Pixel Spacing" rather than "spacing in the image", in the test comment and in `dicom-ingest.md:184-186`.
- Keep 10.7.1.1 where the sentence is about the Pixel Spacing and IPS relationship, as on `:132`, `:162`, `:169`, the test at `:485` and the LLD heading. Those are correct.

## Smells

### S1, the lead's S1 ruling is cited in the tree and recorded nowhere in it
**Where**: `crates/ocelli-core/src/spacing.rs:3` ("F-211, the lead ruling after its review pass 2"), `.claude/plans/F-211-design.md:323-329`.
**What**: The plan records the operator answer and two lead rulings from after pass 1. It has nothing from after pass 2. fb9bffe has no plan change. The ruling made a 1e-9 mm tolerance govern `select_calibration`, where the comparison had been exact. The ruling also put new code in `ocelli-core` and `ocelli-dicom`, which the plan's "Writes" list (`:241`) and its LLD impact (`:281-283`) do not name.
**Mechanism**: `.claude/WORKFLOW.md:312-316,370` makes a tolerance change "a design-plan decision". `spacing.rs:17-18` itself says "Changing it is a design-plan decision". The next reviewer, or `/complete-feature`, checks the diff against a plan that does not authorise it. The doc comment then points at a decision a reader cannot find.
**Regression**: One "Lead ruling after review pass 2" paragraph in the plan. It should name the shared function, the 1e-9 mm value inherited from `ocelli-dicom`, the crates it touches, and the LLDs (D2). It should also say whether the equal branch must still refuse an invalid Pixel Spacing, which is D1's question.

## Nitpicks

- **N1.** Pixel Spacing `-0.0\-0.0` beside IPS `0\0` with an unparseable or NaN factor is now `Frame` (402) where it was `Unavailable` (403). The pair now agrees, so the factor is read. It matches what `0\0` against `0\0` already gave, and the viewer maps both codes to `calibration`. It is a change in a refusal code, recorded here rather than counted.
- **N2.** `<=` to `<` in `spacing_pair_agrees` is green. It differs only for a difference of exactly 1e-9 in `f64`, which `ocelli-dicom` never pinned either. Harmless.
- **N3.** `use.html:671` reads "stay distinct, PS3.3 10.7.1.1 ... so one is never substituted for the other". With an equal pair and a factor, `select_calibration` reports IPS divided by the factor in place of Pixel Spacing. That is a derivation from IPS rather than IPS itself, so the sentence is not false, but the "so" no longer follows from the sentence before it.
- **N4.** `.claude/plans/F-020-design.md:127,136,384,385` and `docs/sprints/AS_BUILT.md:2615` still cite C.7.6.1.1.5 for spacing calibration. They are historical ledgers outside this diff and are handed to the lead.

## Verified clean

- **Pass 2's findings.**
  - D1 is closed: a row-only pair is tested in geom and in the viewer, and the mutation is red.
  - D2 is closed: both branches are pinned and red when mutated.
  - D3 is closed: README step 8 now matches every probe except D1's input.
  - D4 is closed: the `Detector` variant doc, the `Calibration` doc, the field doc and the module doc are each checked against the code ("Chosen only when ... no magnification factor is present", true for `select_calibration`).
  - D5 is closed in `use.html`: the citation is 10.7.1.1 and the sentence no longer says the two always differ. Its LLD and test copies are D3.
  - S1 is closed as ruled: one definition, the dicom value, and both crates' tests in the 1e-10 band.
  - N2 is closed: the branch pin is red when mutated.
- **`spacing.rs` doc claims, executed.**
  - DS is at most 16 bytes, per PS3.5 Table 6.2-1.
  - `0.1430000001` differs from `0.143` by about 1e-10 and agrees.
  - Trailing zeros and exponent form parse to the same `f64`, as pass 2 also probed.
  - A non-finite half never agrees: `inf - inf` is NaN, and `inf - x` is greater than the bound.
  - "ocelli-dicom also uses it for in-plane spacing agreement between frames" is true, and series grouping uses it too.
- **`dicom-ingest.md:243-249`.** The three callers it names are the three that exist.
- **`calibration.md:8`.** It is accurate except for the "one answer for one file" clause (D1). Pass 2 N1's ordering is fixed.
- **`use.html:684,699`.** Both match the code and the probes.
- **The merge, through the tests.** F-210's `value.rs` tests, the `preview.rs` page contract and F-216's corpus pair all pass on the merged tree.
- **Boundary and tier.** There is no wasm-bindgen, no unsafe, no pixel crossing, no `as` cast, no `#[allow]` and no render-loop change in fb9bffe. Tiers A, B and C are not applicable.

## Limitations and handoff

- **Not run.** No browser, no live viewer and no Playwright, so AC18 is still unobserved. No oracle or GPU job.
- **Not checked.** The functional-group IPS limitation carries over from pass 1. cornerstone3D's handling of equal spacings was not checked, and nothing here claims it.
- **Scratchpad.** `g-guards.txt`, `g-bench.txt`, `g-unsafe.txt`, `g-nostd.txt` and `t.txt` in the shared scratchpad hold this pass's gate and test logs, and were left in place. Whether earlier files with those names were overwritten is not known. The scratch copies were deleted.
- **Next owner.** The implementer fixes D1 to D3, and the lead records S1 in the plan. A fresh pass 4 then reviews the changed material.
- This is a bounded review of fb9bffe and of the merge. It is not acceptance and not sprint readiness.
