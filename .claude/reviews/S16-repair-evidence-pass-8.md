# S16 repair evidence, pass 8

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree this repair answers**: `f9b56c9261373bcd4e560b5c320ab35338ad7095`
**Not a receipt**: `6b562be784f4365e7781869b5c511e9e0a1162a8`, `1f525c04`, `7de677af36b9122e60d2da4700c96b730f422e0a`, `8d0a94a34d4b35cd38e26c5056311eab05937231`, `f6057868244a71679ebfd95a17e440084699aa76`, `9ae40681bae54a69a21705d98a40a6337adce74d`, `d8ae7aa9c810e3aa1e619e67a96ca710a9df4502`, `289d1e0f7a84ccbe15845274187fc58b27b8a262`, `c21279c6aa1e5028b10297db4311978cb3b6aed1` and `f9b56c9261373bcd4e560b5c320ab35338ad7095` do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is `git write-tree` after this file is staged. It is reported with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

Pass 8 continues D10. The zoom repair from pass 7 stays. This pass applies the signed camera basis. It does not reopen D8 or D9. F-048 and F-101 were not reworked. AC5 stays unmet.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-X003 D10 signed basis | repaired in this tree | `tools/oracle/src/produce.rs` and the gaze-sign test below | implementing agent | Review the staged tree |
| F-X003 AC5 pixel gate | unmet | pass 7 comparison-failure remains the corpus record, 0 pass, 67 fail, 17 unmeasured | implementing agent | Leave the rows in the run |
| Interaction instruments | implementation missing | `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` record no sample | implementing agent | Leave the instruments unimplemented |
| CPU-time and wall-time collector | configuration missing | summary field `not-configured` | implementing agent | Leave the collector unconfigured |
| Representative GPU-less host | external observation unavailable | summary field `unavailable` | a future representative host | Keep the earlier no-host answer |
| Residual pixel groups | unattributed | local fixtures below are not a causal reduction | implementing agent | A later reproducer must keep the candidate and reference disagreement |
| Release wasm size | not measured in this pass | The budget file is unchanged | implementing agent | Do not re-baseline from this repair |
| Instruction pass 5 | unchanged, 0 defects, unstaged | `.claude/reviews/S16-instruction-refinement-pass-5.md` | implementing agent | Leave the canonical files and that review unstaged |

## Signed basis

The approved plan stays `approved`. The applied-pose paragraph in `Repair, 2026-10-19, applicable stack candidate` now states both gaze signs. The historical sentence that AC5 was the missing producer is left in place above that section.

The reference right vector is `viewUp × viewPlaneNormal`. The view-plane normal points from the focal point toward the camera. Supported view-up is the negated column direction, so that right vector equals the row direction on the negative-normal side and the negated row on the positive-normal side. The unflipped view maps increasing column index to the right, which is the negative-normal side. The positive-normal side is applied as one extra horizontal flip. An explicit horizontal flip composes with that basis. Vertical flip stays the requested flag. The published flip is the value the raster executed. Position, focal point and view-up are published only after the pose check.

The camera-matrix control in `/private/tmp/ocelli-pass8-camera.log` agrees. At z = 10 the horizontal coefficient is -1 and the view-plane normal is positive Z. At z = -10 the horizontal coefficient is +1 and the view-plane normal is negative Z. The vertical coefficient is -1 on both sides. That log is a matrix control, not a browser render and not a corpus observation.

`synthetic__ct_unsigned_16` sits on the negative-normal side with identity orientation and view-up `[0, -1, 0]`. A sign count over the reference stack sidecars, with no coordinates written down, found 93 negative-normal cameras and 0 positive-normal cameras. The other 9 cameras are volume reformats and are not selected. This repair therefore does not change a published corpus frame. The corpus producer and the candidate gate were not re-run. `tools/oracle/compare-out/pass-6-red/` is unchanged.

## Permanent tests

`each_gaze_sign_applies_its_own_horizontal_basis` uses the 2 by 2 source `[0, 40, 0, 40]` on an 8 by 8 canvas. LINEAR centre 40 and width 1 paint stored 0 black and stored 40 white. Column 0 is the black sample and column 1 is the white sample.

The negative-normal position `[0.5, 0.5, -10]` keeps pixel 0 black and pixel 6 white. The published horizontal flip is false, and the published position z is -10. The positive-normal position `[0.5, 0.5, 10]` swaps those samples. The request flip stays false and the published flip is true. The published position z is 10. Focal point and view-up stay the requested centred pose. An explicit horizontal flip on the positive side restores the negative-side frame, and the published flip is false.

Zoom, explicit flip, off-centre focal, rotated view-up and disagreeing scale stay. Zoom and the explicit flip now use the negative-normal side, which is the unflipped column order those assertions derive. The five residual fixtures do the same, so their hand-derived centres and display codes stay the local contracts they state. Sampling, fit, `IMAGE_FROM_INDEX`, the sigmoid formula, tolerances, the census and the divergence register were not changed.

## RED

The unrepaired staged tree was probed from the isolated copy at `/private/tmp/ocelli-pass8-il2t0vbs/source`, with `CARGO_TARGET_DIR=/private/tmp/ocelli-pass8-probe-target`. The probe is not in the product. Log `/private/tmp/ocelli-pass8-probe.log`.

The build finished in 13.71s. The command exited 101. One test failed and 100 were filtered. Both gaze signs rendered the same nonconstant frame, and the negative position was still published.

The unrepaired library command exited 0 with 99 passed and 1 filtered. Log `/private/tmp/ocelli-pass8-library.log`. That run is the old source. It is not this repair's GREEN.

## GREEN

```bash
bin/ocelli.sh cargo fmt -p ocelli-oracle
bin/ocelli.sh clippy ocelli-oracle
bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator
```

`fmt` exited 0. `clippy` exited 0 on the dev profile and finished in 1.68s. The library command exited 0. 100 passed, 0 failed, 1 filtered, finished in 0.00s. The filtered test is the corpus producer. The final run was on the formatted source and finished its test profile in 0.11s.

## Residuals stay unattributed

The one-code interior, the letterbox boundary, the nonsquare edge and the sigmoid shoulder are still unexplained. The passing local fixtures do not reproduce those corpus signatures. A later diagnosis needs a minimal case that keeps the observed disagreement on both the candidate and the reference, then attributes a layer. A nearby value that passes is not that reduction. Syntax rows and lossy rows stay unattributed. No failing row was dropped.

## Separate gaps

AC5 stays unmet. The producer is connected, and the standing corpus verdict is still comparison-failure with 0 pass, 67 fail and 17 unmeasured. Applying this basis does not turn that run green.

The interaction instruments remain unimplemented and record no sample.

No CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded.

No verification receipt, independent whole-sprint approval, commit, push, sprint close or new sprint is part of this repair. F-X003 is not marked done.

## Staged checks

`python3 scripts/prose_check.py --staged` exited 0. Voice rules were clean over 66 files.

`python3 scripts/unsafe_allowlist_check.py` exited 0. It checked 153 files, with 2 permitted. The vendored audit surface is unchanged. `openjph-core-0.1.0` has 104 constructs and `ritk-codecs-0.6.0` has 0.

`python3 scripts/unsafe_allowlist_check.py --staged` exited 1. Those two recorded packages are absent from the staged name set. The allow-list was not edited.
