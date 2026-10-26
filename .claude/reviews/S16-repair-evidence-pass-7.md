# S16 repair evidence, pass 7

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree this repair answers**: `c21279c6aa1e5028b10297db4311978cb3b6aed1`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8`, `1f525c04`, `7de677af36b9122e60d2da4700c96b730f422e0a`, `8d0a94a34d4b35cd38e26c5056311eab05937231`, `f6057868244a71679ebfd95a17e440084699aa76`, `9ae40681bae54a69a21705d98a40a6337adce74d`, `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502`, `289d1e0f7a84ccbe15845274187fc58b27b8a262` and `c21279c6aa1e5028b10297db4311978cb3b6aed1` do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

Pass 7 named D10. This pass repairs that defect inside the producer review scope. It does not reopen D8 or D9. F-048 and F-101 were not reworked. AC5 stays unmet.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-X003 D10 applied camera | repaired in this tree | `tools/oracle/src/produce.rs` and the camera tests below | implementing agent | Review the staged tree |
| F-X003 AC5 pixel gate | unmet | comparison-failure, 0 pass, 67 fail, 17 unmeasured | implementing agent | Leave the rows in the run |
| Interaction instruments | implementation missing | `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` record no sample | implementing agent | Leave the instruments unimplemented |
| CPU-time and wall-time collector | configuration missing | summary field `not-configured` | implementing agent | Leave the collector unconfigured |
| Representative GPU-less host | external observation unavailable | summary field `unavailable` | a future representative host | Keep the earlier no-host answer |
| Residual pixel groups | reduced to synthetic checks, arithmetic unchanged | the five producer tests below | implementing agent | Do not retune from the red corpus |
| Release wasm size | not measured in this pass | The budget file is unchanged | implementing agent | Do not re-baseline from this repair |
| New instruction additions | reviewed separately, 0 defects | `.claude/reviews/S16-instruction-refinement-pass-5.md`, left unstaged | implementing agent | Leave the canonical files and that review unstaged |

## D10

The approved plan stays `approved`. The copied-sidecar sentence in `Repair, 2026-10-19, applicable stack candidate` now states the applied camera contract. The historical sentence that AC5 was the missing producer is left in place above that section.

`executed_camera` fits the source, then applies `parallelScale` as zoom when it differs from the fitted scale by more than the existing world bound of `1e-6` mm. A requested scale inside that bound stays at zoom 1. The declared canvas scale must match `2 * parallelScale / canvasHeight` applied to the row and column spacing. A disagreement is `camera-scale-disagrees` and is not published. A missing or non-positive scale is `camera-scale-refused`.

The supported pose is the centred focal point, view-up equal to the negated column direction, and a gaze along the plane normal. An off-centre focal point is `camera-placement-refused`. A rotated view-up or an oblique gaze is `camera-orientation-refused`. Flips are applied. The published parallel scale, canvas scale and flips are the executed request. Position, focal point and view-up are kept only after that pose check. No shell command was added. Tolerances, `IMAGE_FROM_INDEX` and the fit policy were not changed.

## Original red run

The pass-6 comparison is preserved at gitignored `tools/oracle/compare-out/pass-6-red/`. Its disposition file is timestamped 2026-10-19 12:37:55. It was not deleted and it was not staged.

That run is comparison-failure. The scalars are resources `ran`, selected 85, produced 84, compared 84, unsupported 19, failed 68, unrun 0, pass 0, outcomeFail 67, unmeasured 17, produceFailed 1, comparatorRefused 0. The case list has 104 records. Tier A is `unrun`. Instruments are `not-implemented`. The collector is `not-configured`. The host measurement is `unavailable`.

Reason counts are `class-not-monochrome` 1, `colour-not-scored` 4, `palette-not-scored` 2, `pixel-fail` 67, `pixel-layout-refused` 1, `reference-row-not-rendered` 2, `unmeasured` 17, `volume-reformat` 9 and `ybr-not-scored` 1.

## Regenerated run

`produce.rs` is timestamped 2026-10-19 12:58:22. The corpus filter ran after that edit.

```bash
bin/ocelli.sh test ocelli-oracle --lib applicable_monochrome_rows_reach_the_comparator
```

Exit 0. 1 passed, 99 filtered, 0 failed. Finished in 67.70s. The disposition file it wrote is timestamped 2026-10-19 13:00:13.

The disposition scalars and the reason histogram equal the preserved red run. The identities hold. Selected equals produced plus produce-failed plus unrun. Failed equals produce-failed plus outcome-fail plus comparator-refused. Compared equals pass plus outcome-fail plus unmeasured. Produced equals compared plus comparator-refused. Unsupported plus selected equals 104.

The gate command is the existing candidate form. `bin/ocelli.sh gate oracle` was not run.

```bash
bin/ocelli.sh compare gate \
  --reference tools/oracle/compare-out/reference-selected \
  --candidate tools/oracle/compare-out/candidate \
  --out tools/oracle/compare-out/gate
```

The release build finished in 6.21s. The command exited 1. `compare.json` is timestamped 2026-10-19 13:00:40. The summary lines are:

```text
84 views: 0 pass, 67 fail, 17 unmeasured, 0 absent
  claimed verdict views: 67, unsupported source rows: 0, declared volume refusals: 0
  bias: 82
  decimated: 1
  divergent-while-unmeasured: 17
  letterbox-only: 1
  weak: 16
gate verdict: comparison-failure
compare: candidate gate is RED (comparison-failure)
```

`compare.json` agrees. `views` 84, `pass` 0, `fail` 67, `unmeasured` 17, `absent` 0, `claimedVerdictViews` 67, `gateVerdict` `comparison-failure`, `green` false. The `problems` array is empty. `absorbedDivergences` has 17 entries. `coverageProblems` has 31 entries. The lines themselves are not copied here. Render hashes are not copied here.

All 84 sidecars were compared with the reference camera and canvas scale. The worst absolute component difference is `2.842e-14`. No component exceeds `1e-6`. Three position lists and seven focal-point lists differ only by JSON reserialization below that bound. View-up lists do not differ. All 84 gate records have zero parameter divergences and zero geometry divergences.

Applying zoom did not turn the corpus green. The recorded parallel scales already match the fitted scale inside the world bound, so the corpus rows stay at zoom 1.

## Residual groups

These checks use hand-derived synthetic inputs. They do not lock a corpus mismatch as a desired pixel. Sampling, the colour-table truncation, the sigmoid formula and the fit were not changed.

| Group | Hand-derived check | Result |
| --- | --- | --- |
| Layout refusal | High bit 15 with 12 bits stored is `pixel-layout-refused`. High bit 11 is accepted. PS3.3 C.7.6.3.3 | Pass. The left-aligned corpus row stays refused |
| Fractional LINEAR | Stored 40, centre 40, width 400 is 127.819547... Truncation toward zero addresses 127. PS3.3 C.11.2.1.2 | Pass. The byte is 127 |
| Letterbox | On an 8 by 4 canvas a 1 mm source at scale 4 starts at x = 2. Pixel 0 is black. Pixel 2 is white | Pass |
| Nonsquare centres | Row spacing 2 and column spacing 1. Texel centres land on the hand-derived canvas pixels. PS3.3 C.7.6.2.1.1 | Pass |
| SIGMOID width 0.5 | Modality 37.75, 40 and 42.5 truncate to 0, 127 and 255. PS3.3 C.11.2.1.3.1 | Pass |

The corpus one-code rows still differ from the reference by one display code inside the image, with a darker candidate. That is the same direction as truncation against a rounded byte. It is not attributed to a wrong sample, and the truncation was not changed. The shader uses the same toward-zero table index.

`synthetic__ct_unsigned_16` still matches inside the image rectangle. Its remaining differences are the two columns immediately outside that rectangle. The candidate pixel is 0. The reference pixel is not. The letterbox test checks the centre rule and passes. The fit was not changed.

`synthetic__mr_nonsquare_spacing` still has the one-code interior plus two edge rows. The last image row differs by 64 across the full width. The small nonsquare test matches the centre samples and does not reproduce that edge row. The edge row stays unattributed.

`synthetic__ct_sigmoid_width_half` still differs from the reference by more than one code on the shoulders. The centred synthetic matches the standard truncation. The formula was not changed and the divergence register was not edited.

Syntax rows stay in the run. Most follow the one-code pattern. The lossy rows with a larger in-image difference were not given a second codec fixture and were not dropped. The decimated unmeasured row is not named here. No failing row was removed.

## Checks

```bash
bin/ocelli.sh cargo fmt -p ocelli-oracle
bin/ocelli.sh clippy ocelli-oracle
bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator
```

`fmt` applied. `clippy` exited 0 on the dev profile. The library command exited 0 with 99 passed, 0 failed and 1 filtered. The filtered test is the corpus producer, which was run separately above. The five camera tests from the D10 repair and the five residual checks are in the 99.

Tier A was not opened. The interaction instruments were not implemented. No CPU-time or wall-time sample was recorded. No vCPU fraction and no changed-frame rate were recorded. Wasm was not rebuilt. `gate bindgen` and `gate pins` were not rerun. The colour-path size measurement from an earlier pass was not repeated.

## Instruction review

Pass 5 is `.claude/reviews/S16-instruction-refinement-pass-5.md`. It reads only the pass-7 paragraphs. It does not extend pass 2, pass 3 or pass 4. Result: 0 defects, 0 smells, 0 nitpicks. The canonical instruction files, pass 3, pass 4 and pass 5 stay unstaged.

## What remains separate

AC5 is unmet because the pixel gate is comparison-failure. Fixing D10 made the published camera describe the executed view. It did not make the corpus pixels pass.

The interaction instruments remain unimplemented. The collector remains unconfigured. The representative GPU-less host remains unavailable. No sample was invented for any of the three.

F-048, F-101 and F-X003 stay in progress. No story was marked done. Nothing was committed or pushed. S16 was not closed.
