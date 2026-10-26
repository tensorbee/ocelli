# F-X003 D11 repair review and residual diagnosis handoff, pass 10

**Reviewed**: staged tree `0b26935f41842818a1afd61ee7df3b14c1f4940f`
**Previous**: `37fc701125277675fe9cd243ca1df5a4dff923e9`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 0 defects, 0 smells, 0 nitpicks within the D11 repair scope
**Date**: 2026-10-19

## Disposition

D11 is repaired in the exercised producer path. The raster uses the camera's
signed horizontal basis once. Observed reference flip flags retain their
reference meaning in candidate metadata and are no longer replayed as fresh
commands. Internal View coefficients remain distinct. Horizontal flip and
flip-back fixtures now describe the pinned reference readback. Vertical and
combined poses outside the supported view-up are explicitly refused.

The supplied library suite passes. More importantly, the original independent
D11 reproducer now passes without changing its expected asymmetric pixel
order. The previous source produced black at pixel 0 for the observed flipped
camera. The repaired source produces white there and black at pixel 6, as
the independent camera basis requires.

No new general instruction was added. The design and dicom-tooling changes
from pass 9 directly address this failure and have a separate independent
instruction review in `S16-instruction-refinement-pass-6.md`. That review
retains its stated scope. The remaining task is to execute the existing
disagreement-preserving diagnosis requirements.

## Checks executed now

`bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator`
exited 0, 100 passed and one filtered. Log:
`/private/tmp/ocelli-pass10-library.log`.

Archived the pinned source into
`/private/tmp/ocelli-pass10-nnj4pxr2/source`. Copied the independent D11 test
from the prior isolated source into that archive's produce.rs test module.
Original product source and staged index were preserved. Command:

```bash
cd /private/tmp/ocelli-pass10-nnj4pxr2/source
CARGO_TARGET_DIR=/private/tmp/ocelli-pass10-probe-target cargo test -p ocelli-oracle --lib independent_pass9 -- --nocapture
```

Exit 0, one passed and 101 filtered. Build finished in 13.46 seconds. Log:
`/private/tmp/ocelli-pass10-probe.log`. The old-source RED remains in
`/private/tmp/ocelli-pass9-probe.log` and the pass-9 review. This pass did not
rebuild the old source or claim a new mutation result.

Adapter sync check and `git diff --check` passed. All inputs in the independent
probe are synthetic. No corpus producer, candidate gate, browser/GPU rendering,
new release-size or representative-host measurement was executed here.

## Concrete next diagnosis, SIGMOID width below one

Start with the recorded `synthetic__ct_sigmoid_width_half` disagreement.
Preserve its existing comparison and construct a small paired candidate and
reference case that retains the shoulder discrepancy. A local standard-value
fixture alone is not that reproducer.

One concrete investigation lead was executed in this review against the
installed permitted cornerstone3D 5.8.2 utilities:

```javascript
const range = toLowHighRange(0.5, 40, "SIGMOID");
const roundTrip = toWindowLevel(range.lower, range.upper);
```

It returns lower 39.75 and upper 39.25, then width 1.5 and centre 40.
Widths 1 and 2 return widths 1 and 2 as positive controls. Node exited 0.
Log: `/private/tmp/ocelli-pass10-sigmoid-lead.log`.

Primary local sources are
`tools/oracle/node_modules/@cornerstonejs/core/dist/esm/utilities/windowLevel.js`
and `createSigmoidRGBTransferFunction.js`. The latter calls `toWindowLevel`
when building its sampled curve. `tools/oracle/page/app.mjs:582` uses the
range helper before setting viewport properties. This is a demonstrated
helper-level transformation and a plausible lead for the shoulder difference.
It is not yet a new attributed browser-rendering defect or proof that it
explains every differing corpus pixel.

The next implementing-agent assignment is diagnosis first:

1. Trace the actual pinned reference stack path from requested centre/width
   to applied range, transfer function and rendered samples. Verify that the
   helper behavior above is reached in the paired case.
2. Use a small synthetic stored-value ramp with nonidentity rescale where
   relevant. Record independently derived DICOM SIGMOID values and actual
   candidate/reference samples at the centre and both shoulders. Keep geometry
   and background controlled. Preserve the original mismatch and exact inputs.
3. Determine whether the cause is candidate arithmetic, reference behavior,
   adapter parameters, quantization or another layer. Make the narrowed case
   fail for the observed reason before proposing a repair or divergence policy.
4. Return the executable reproducer, source/artifact identities, own command
   exits and attribution evidence for review. Do not alter product arithmetic,
   reference pins, tolerances, baselines or the divergence register merely to
   make this comparison pass. Any necessary policy decision uses the normal
   design and review process.

Other one-code, letterbox, nonsquare and codec signatures remain unattributed.
This bounded assignment does not require solving all of them in the same
patch or treat them as the same cause.

## Limits and status

AC5 remains unmet. The standing reported corpus verdict is comparison-failure:
zero pass, 67 fail and 17 unmeasured. That is historical implementing evidence,
not a new run in this pass. A scoped clean D11 repair does not change it.
The staged unsafe-check failure also remains separate from the passing
working-tree guard and was not repaired or waived here.

Interaction instruments, CPU-time and wall-time collector work, and the
unavailable representative GPU-less host remain separate unfinished work.
F-048, F-101 and F-X003 stay in progress. No verification receipt, whole-sprint
approval, completion, commit, push, close or new sprint is supplied. This
review remains unstaged and the reviewed index is unchanged.
