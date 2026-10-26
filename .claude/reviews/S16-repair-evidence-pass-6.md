# S16 repair evidence, pass 6

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree this repair answers**: `289d1e0f7a84ccbe15845274187fc58b27b8a262`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8`, `1f525c04`, `7de677af36b9122e60d2da4700c96b730f422e0a`, `8d0a94a34d4b35cd38e26c5056311eab05937231`, `f6057868244a71679ebfd95a17e440084699aa76`, `9ae40681bae54a69a21705d98a40a6337adce74d`, `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502` and `289d1e0f7a84ccbe15845274187fc58b27b8a262` do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

Pass 6 closed D9 inside the scope it reviewed. This pass does not reopen D8 or D9. F-048 and F-101 were not reworked. The mode-preservation repair stays as pass 6 accepted it.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-X003 D9 mode preservation | closed in pass 6 | `.claude/reviews/F-X003-raster-pass-6.md` | Codex, within that scope | Do not reopen it |
| F-X003 AC5 producer | implemented and connected | `tools/oracle/src/produce.rs` and the existing `compare gate` command below | implementing agent | Review the staged tree. The pixel gate is comparison-failure |
| Interaction instruments | implementation missing | `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` record no sample | implementing agent | Leave the instruments unimplemented |
| CPU-time and wall-time collector | configuration missing | summary field `not-configured` | implementing agent | Leave the collector unconfigured |
| Representative GPU-less host | external observation unavailable | summary field `unavailable` | a future representative host | Keep the earlier no-host answer |
| Synthetic comparator controls | retained | `candidate.rs`, stored 40 passes and stored 0 fails | implementing agent | Leave both controls. They are not corpus rows |
| Release wasm size | not measured in this pass | The budget file is unchanged | implementing agent | Do not re-baseline from this repair |
| New instruction additions | reviewed separately, 0 defects | `.claude/reviews/S16-instruction-refinement-pass-4.md`, left unstaged | implementing agent | Leave the canonical files and that review unstaged |

## Producer

The approved plan stays `approved`. The new section is `Repair, 2026-10-19, applicable stack candidate`. The historical sentence that AC5 was the missing producer is left in place above that section.

`tools/oracle/src/produce.rs` reads the reference run and the permitted corpus files. It writes one settled `CpuRaster` frame for each monochrome stack row the raster can draw. Colour, palette, YBR, volume reformats and rows the reference did not render are recorded as unsupported and are not scored. No shell command was added. `ocelli-compare gate` was not changed. Tier A was not opened.

The command that ran is the existing candidate form:

```bash
bin/ocelli.sh compare gate \
  --reference tools/oracle/compare-out/reference-selected \
  --candidate tools/oracle/compare-out/candidate \
  --out tools/oracle/compare-out/gate
```

`bin/ocelli.sh gate oracle` was not run. `bin/ocelli.sh oracle` was not run. The three output directories are gitignored and are not staged. `tools/oracle/out` was not modified by this command.

The corpus filter `applicable_monochrome_rows_reach_the_comparator` was run after the last edit of `produce.rs` and exited 0. This closeout did not repeat it. The ignored disposition file it wrote is timestamped 2026-10-19 11:59:40. `produce.rs` is timestamped 11:58:11. The gate log is `/tmp/ocelli-ac5-gate.log`, timestamped 12:00:00. The release build in that log finished in 17.92s. The command exited 1.

Disposition scalars, and 104 case records:

| Field | Value |
| --- | --- |
| resources | ran |
| selected | 85 |
| produced | 84 |
| compared | 84 |
| unsupported | 19 |
| failed | 68 |
| unrun | 0 |
| pass | 0 |
| outcomeFail | 67 |
| unmeasured | 17 |
| produceFailed | 1 |
| comparatorRefused | 0 |
| recorded | 104 |
| tierA | unrun |
| instruments | not-implemented |
| collector | not-configured |
| hostMeasurement | unavailable |

`recorded` is the case-list length. The disposition file stores the cases and does not repeat that length as its own field. The identities hold. Selected equals produced plus produce-failed plus unrun. Failed equals produce-failed plus outcome-fail plus comparator-refused. Compared equals pass plus outcome-fail plus unmeasured. Produced equals compared plus comparator-refused. Unsupported plus selected equals recorded.

Case dispositions are failed 1, unsupported 19 and compared 84. Reason counts are `class-not-monochrome` 1, `colour-not-scored` 4, `palette-not-scored` 2, `pixel-fail` 67, `pixel-layout-refused` 1, `reference-row-not-rendered` 2, `unmeasured` 17, `volume-reformat` 9 and `ybr-not-scored` 1.

Synthetic corpus rows, by disposition. None of these is a pass:

| View | Disposition | Reason |
| --- | --- | --- |
| `synthetic__ct_signed_12in16_left` | failed | `pixel-layout-refused` |
| `synthetic__ct_signed_12in16_right` | compared | `pixel-fail` |
| `synthetic__ct_unsigned_16` | compared | `unmeasured` |
| `synthetic__ct_multiframe_perframe` | compared | `pixel-fail` |
| `synthetic__sc_palette_color` | unsupported | `palette-not-scored` |
| `synthetic__sc_rgb_interleaved` | unsupported | `colour-not-scored` |
| `synthetic__us_ybr_full_422` | unsupported | `reference-row-not-rendered` |

The left-aligned 12-in-16 row was not written, so it is absent from the gate directories. Palette, RGB and the unrendered YBR row were not written either.

Each subset directory holds 84 raw frames plus `run.json` and 84 sidecars. The gate directory holds 84 difference frames and `compare.json`.

## Gate

The log's summary lines are:

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

`compare.json` agrees. `views` 84, `pass` 0, `fail` 67, `unmeasured` 17, `absent` 0, `claimedVerdictViews` 67, `gateVerdict` `comparison-failure`, `green` false, `operation` `gate`. The `problems` array is empty. `absorbedDivergences` has 17 entries and those are the PROBLEM lines. `coverageProblems` has 31 entries and those are the COVERAGE lines. The lines themselves are not copied here. Render hashes are not copied here.

No parameter divergence and no geometry divergence is recorded on the six synthetic views that reached the gate. Sampling and the section 25.1 tolerances were not changed.

The grey image lane and the full frame, from those six gate records:

| View | Outcome | Image pixels | Image differ | Image max | Image over 2 | Full differ | Full max |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `synthetic__ct_unsigned_16` | unmeasured | 218112 | 0 | 0 | 0 | 1024 | 64 |
| `synthetic__ct_signed_12in16_right` | fail | 218112 | 11008 | 1 | 0 | 11350 | 64 |
| `synthetic__cr_monochrome1` | fail | 218112 | 113487 | 1 | 0 | 114468 | 64 |
| `synthetic__mr_nonsquare_spacing` | fail | 157696 | 38356 | 64 | 960 | 38356 | 64 |
| `synthetic__ct_sigmoid_width_half` | fail | 218112 | 185344 | 57 | 131072 | 185856 | 64 |
| `synthetic__ct_multiframe_perframe` | fail | 218112 | 10084 | 1 | 0 | 10639 | 64 |

Image differ is image pixels minus the grey lane's zero-difference count. The unsigned frame matches inside the image rectangle. Its 1024 full-frame differences are outside that rectangle, at maximum 64. The record's qualifiers are `weak`, `letterbox-only` and `divergent-while-unmeasured`. The other five image regions do not match. Nonsquare and sigmoid exceed 2 inside the image. That disagreement is the recorded pixel result. It is not a change to `IMAGE_FROM_INDEX` or to a tolerance.

A separate count of the ignored synthetic raw bytes, any of the three colour channels, agrees with the full-frame column. The unsigned pair differs on 1024 of 262144 pixels. The mean of the per-pixel maximum channel difference, on those differing pixels only and rounded to two decimals, is 62.40. The same mean is 2.52, 1.28, 1.96, 19.80 and 3.79 for the right-aligned, monochrome1, nonsquare, sigmoid and multiframe pairs. Those means are a diagnostic. The gate record above is the verdict.

## Controls

`tools/oracle/src/candidate.rs` is not a corpus row. `the_settled_linear_frame_is_compared_not_assumed` rasters stored 40 through LINEAR centre 40 and width 400. The first byte is 127. `compare_view` returns `Outcome::Pass`. `a_stored_zero_frame_is_the_wrong_candidate` rasters stored 0 against the same hand grey. The first byte is not 127. `compare_view` returns `Outcome::Fail`. Both tests passed in the library run below.

## Checks

```bash
bin/ocelli.sh cargo fmt -p ocelli-oracle -- --check
bin/ocelli.sh clippy ocelli-oracle
bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator
```

Each exited 0 on this closeout. Format check was clean. Clippy finished the dev profile in 0.35s with no diagnostic. The library run is 89 passed, 0 failed, 1 filtered, in 0.01s. The filtered test is the corpus producer. The two synthetic controls are in the 89.

```bash
bin/ocelli.sh gate bindgen
bin/ocelli.sh gate pins
python3 scripts/unsafe_allowlist_check.py
```

Each exited 0. Bindgen reports `OK: wasm-bindgen is confined to ocelli-wasm`. Pins report `OK: wasm-bindgen, wgpu pinned exactly`. The pin check was the pin-only form. No release wasm size was measured. The working-tree unsafe check reports 152 files checked, 2 permitted, vendored audit surface unchanged (`openjph-core-0.1.0` 104, `ritk-codecs-0.6.0` 0).

`tools/oracle/tests` was not run. The library tests and the candidate gate are the lifecycle this pass connected.

`python3 scripts/prose_check.py --staged` exited 0 after this file was staged: `OK: voice rules clean over 62 file(s)`.

`python3 scripts/unsafe_allowlist_check.py --staged` exited 1. It reports `openjph-core-0.1.0` recorded at 104 and `ritk-codecs-0.6.0` recorded at 0, and says neither package is in the tree. That form reads `git diff --cached --name-only`, so a diff that does not include the vendored crates answers as if they were absent. The working-tree check above is the one that reads them. The allow-list was not edited.

## Still open

The pixel gate is comparison-failure. The producer wrote 84 frames and the comparator judged them. Pass count is 0. That is the recorded corpus result. It is not a green oracle and it is not a claim of parity with cornerstone3D.

The interaction instruments stay a separate gap. `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` were not modified. No sample was recorded under those subject ids. This is missing implementation.

The collector and the host stay a third gap. No CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded. This is an external observation, not a refused sample. A missing GPU adapter does not account for the producer, which ran on the CPU raster.

Tier A stayed `unrun`. Rendered DPR 1.25, tier B and a browser GPU canvas were not run. No new release-size number is claimed. F-X021 was not implemented.

## Instruction paragraphs

The new additions in canonical `WORKFLOW.md`, `design.md`, `implement-feature.md` and `dicom-tooling` were reviewed by the implementing agent. The review is `.claude/reviews/S16-instruction-refinement-pass-4.md`. It reports 0 defects, 0 smells and 0 nitpicks on that diff only. It stays unstaged with the instruction sources and with pass 3. Pass 2 and pass 3 are not extended. Codex authorship of the paragraphs is not independent approval. The adapters were not regenerated in this turn.

The next actual `AS_BUILT` entry is not written now. It still needs the D9 cache-validity trigger, the pass 6 instruction triggers, this design append, the two LLD corrections, the shell comment, and the producer result above. Ordinary comparison-ledger profiles stay dormant. This bounded directory does not activate them.

## Handback

Please give the staged index a fresh independent Codex review before completion. Do not mark F-048, F-101 or F-X003 done. Do not commit, push, close S16, start S17, or record a verification receipt from this evidence alone.
