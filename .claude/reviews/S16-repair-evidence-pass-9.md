# S16 repair evidence, pass 9

**Date**: 2026-10-19
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04` (`S16, open the sprint`)
**Reviewed tree this repair answers**: `37fc701125277675fe9cd243ca1df5a4dff923e9`
**Not a receipt**: the hashes listed in the pass 8 evidence, and `37fc701125277675fe9cd243ca1df5a4dff923e9`, do not cover the repaired tree. No sprint-profile receipt was recorded for this repair.
**Stories**: F-048, F-101 and F-X003 stay `in-progress`. F-X022 and F-X023 stay pending. S16 is not closed. S17 is not opened.

This file is the implementing agent's evidence for a fresh independent Codex review. It is not that review, not a completion record, and not an `AS_BUILT` entry.

The index hash is recorded with the handback and in the gitignored progress notes. It is not written here, because writing it into this file would change the hash.

Pass 9 continues D10. The zoom repair and the gaze-sign pixel change stay. This pass stops treating the observed flip flag as a second reflection. It does not reopen D8 or D9. F-048 and F-101 were not reworked. AC5 stays unmet.

## Acceptance table

| Item | Disposition | Evidence | Owner | Next step |
| --- | --- | --- | --- | --- |
| F-X003 D11 observed flip readback | repaired in this tree | `tools/oracle/src/produce.rs` and the readback tests below | implementing agent | Review the staged tree |
| F-X003 AC5 pixel gate | unmet | pass 7 comparison-failure remains the corpus record, 0 pass, 67 fail, 17 unmeasured | implementing agent | Leave the rows in the run |
| Interaction instruments | implementation missing | `render.first_frame`, `interaction.window_level_drag` and `interaction.scroll` record no sample | implementing agent | Leave the instruments unimplemented |
| CPU-time and wall-time collector | configuration missing | summary field `not-configured` | implementing agent | Leave the collector unconfigured |
| Representative GPU-less host | external observation unavailable | summary field `unavailable` | a future representative host | Keep the earlier no-host answer |
| Residual pixel groups | unattributed | local fixtures below are not a causal reduction | implementing agent | A later reproducer must keep the candidate and reference disagreement |
| Release wasm size | not measured in this pass | The budget file is unchanged | implementing agent | Do not re-baseline from this repair |
| Instruction pass 6 | 0 defects, unstaged | `.claude/reviews/S16-instruction-refinement-pass-6.md` | implementing agent | Leave the canonical files, adapters and that review unstaged |

## Reference contract checked before the patch

The page enables `ViewportType.STACK` and calls `coreInit` with no arguments. Pinned `@cornerstonejs/core` 5.8.2 keeps `useGenericViewport` false unless that flag is set, so the stack is `StackViewport` and not `PlanarViewport`. `applyPlanarViewFlip` is a different class and is not this readback.

`init` leaves CPU rendering off when WebGL is present. The oracle runs on SwiftShader through WebGL, so `resetCamera` is `resetCameraGPU`. That function clears both flip flags and calls `Viewport.resetCamera`. `resetCameraCPU` and `setFlipCPU` are the fallback that writes `hflip`. They are not the path this producer follows.

`getImageDataMetadata` builds direction as row, column, then their cross product. `_getCameraOrientation` sets the initial view-plane normal to the negated slice direction and view-up to the negated column. vtk.js stores `viewPlaneNormal` as the negation of `directionOfProjection`. `setDirectionOfProjection` is called with the negated cornerstone normal, so the stored normal is the cornerstone normal. Reset then places position at the focal point plus distance times that normal. The reset camera sits on the negative-normal side, with view-up equal to the negated column and both flip flags false.

vtk `getViewMatrix` builds `mat4.lookAt` and transposes it. For view-up `[0, -1, 0]`, a camera at z = -10 has screen-right `[1, 0, 0]`, and a camera at z = 10 has screen-right `[-1, 0, 0]`. The unflipped raster maps increasing column index to the right, so the positive-normal side is the one internal horizontal reversal.

`Viewport.flip` for a horizontal request negates the view-plane normal, rebuilds position from that normal, and then toggles `flipHorizontal`. The flag is not passed into the VTK camera. `getCamera` reports the VTK pose plus the flag. The page writes that object into the sidecar. The pose and the flag are one observed operation.

A vertical request in the same function also negates view-up. That view-up is no longer the negated column, so the producer refuses the pose and publishes nothing. One `flip` call with both flags does the same view-up change and is refused. There is no supported combined route.

## Producer

`executed_camera` reads the observed flags and publishes them unchanged. A missing or null flag stays false. A non-bool flag is `camera-flip-refused`. The gaze sign is applied once with `set_flip_horizontal`. The vertical raster coefficient is false on every supported pose. `ExecutedCamera.flip_horizontal` stores the observed flag, not the raster coefficient. The coefficient lives only on the view.

The approved plan stays `approved`. The applied-pose paragraph now says the flags are the observed readback and the raster coefficient stays internal.

A sign count over the reference stack sidecars, with no coordinates written down, found 93 negative-normal cameras and 0 positive-normal cameras. Those rows also have flip false. Removing the extra flag reflection does not move them. The other 9 cameras are volume reformats and are not selected. The corpus producer and the candidate gate were not re-run. `tools/oracle/compare-out/pass-6-red/` is unchanged. The standing verdict remains comparison-failure, 0 pass, 67 fail and 17 unmeasured.

## Permanent tests

`a_reference_horizontal_flip_readback_is_applied_once` uses the 2 by 2 source `[0, 40, 0, 40]` on an 8 by 8 canvas. LINEAR centre 40 and width 1 paint stored 0 black and stored 40 white. Column 0 is black and column 1 is white.

The reset sidecar is position z = -10, view-up `[0, -1, 0]` and `flipHorizontal` false. Pixel 0 is black and pixel 6 is white. The horizontal-flip readback is position z = 10, the same view-up and `flipHorizontal` true. Pixel 0 is white and pixel 6 is black. Flip back restores the reset frame and publishes the flag false. The same positive pose with the flag false produces the same pixels and publishes the flag false. The two positive frames match. Their published flags differ. That is the split between the internal coefficient and the reference field.

The vertical-only readback and the one-call combined readback both carry view-up `[0, 1, 0]`. Both return `camera-orientation-refused` and publish nothing.

`each_gaze_sign_applies_its_own_horizontal_basis` keeps `flipHorizontal` false on both sides. Negative z keeps pixel 0 black and pixel 6 white. Positive z swaps them. Both published flags stay false.

Zoom, off-centre focal, rotated view-up and disagreeing scale stay. The five residual fixtures stay on the negative-normal side. Sampling, fit, `IMAGE_FROM_INDEX`, the sigmoid formula, tolerances, the census and the divergence register were not changed.

## RED

The unrepaired staged tree was probed from the isolated copy at `/private/tmp/ocelli-pass9-5p21cne4/source`, with `CARGO_TARGET_DIR=/private/tmp/ocelli-pass9-probe-target`. The probe is not in the product. Log `/private/tmp/ocelli-pass9-probe.log`.

The build finished in 13.64s. The command exited 101. One test failed and 101 were filtered. Pixel 0 was `Some(0)` where the readback requires `Some(255)`.

The unrepaired library command exited 0 with 100 passed and 1 filtered. Log `/private/tmp/ocelli-pass9-library.log`. That run includes the composition assertion this repair replaced. It is not this repair's GREEN.

## GREEN

```bash
bin/ocelli.sh cargo fmt -p ocelli-oracle
bin/ocelli.sh clippy ocelli-oracle
bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator
```

`fmt` exited 0 before the library run. `clippy` exited 0 on the dev profile and finished in 3.13s. The library command exited 0. 100 passed, 0 failed, 1 filtered, finished in 0.01s. The test profile finished in 0.13s. The filtered test is the corpus producer. `a_reference_horizontal_flip_readback_is_applied_once` and `each_gaze_sign_applies_its_own_horizontal_basis` passed.

## Residuals stay unattributed

The one-code interior, the letterbox boundary, the nonsquare edge and the sigmoid shoulder are still unexplained. The passing local fixtures do not reproduce those corpus signatures. A later diagnosis needs a minimal case that keeps the observed disagreement on both the candidate and the reference, then attributes a layer. A nearby value that passes is not that reduction. Syntax rows and lossy rows stay unattributed. No failing row was dropped.

## Separate gaps

AC5 stays unmet. The producer is connected, and the standing corpus verdict is still comparison-failure with 0 pass, 67 fail and 17 unmeasured. This readback repair does not turn that run green.

The interaction instruments remain unimplemented and record no sample.

No CPU-time or wall-time collector is configured, and no representative GPU-less incumbent host is available. No vCPU fraction and no changed-frame rate were recorded.

No verification receipt, independent whole-sprint approval, commit, push, sprint close or new sprint is part of this repair. F-X003 is not marked done.

## Instruction review

`.claude/reviews/S16-instruction-refinement-pass-6.md` reads only the pass-9 sentences in `design.md` and `dicom-tooling`. Result: 0 defects, 0 smells, 0 nitpicks. It stays unstaged. The canonical files and the generated adapters stay unstaged. Pass 5 does not cover those sentences.

## Staged checks

`python3 scripts/prose_check.py --staged` exited 0. Voice rules were clean over 68 files. `python3 scripts/unsafe_allowlist_check.py` exited 0 over 153 files, with 2 permitted. `python3 scripts/unsafe_allowlist_check.py --staged` exited 1 because `openjph-core-0.1.0` and `ritk-codecs-0.6.0` are recorded and absent from the staged name set. The allow-list was not edited.
