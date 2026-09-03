---
description: Run the differential harness against cornerstone3D over the corpus, and interpret the result correctly.
---

# /oracle [--case <path>] [--modality CT] [--update-hashes]

The differential harness. HLD section 11 and decision D7.

**Needs a GPU and a browser.** It does not run in CI (`DEVIATIONS.md` D-04),
which makes this command the only place divergence is caught before a release.

```bash
bin/ocelli.sh oracle [args]
```

## What it does

Pushes the same study through both stacks and compares frames within the
written per-modality tolerance, **with metadata diffed alongside pixels**,
because a wrong rescale slope still produces a plausible image.

## The tolerance policy is not yours to adjust

From `docs/hld/22-testing-and-tolerance.md` section 25.1:

- **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference of 1 LSB
  on at least 99.9% of pixels, and **zero** pixels differing by more than 2.
- **Colour and ultrasound:** perceptual difference below the stated threshold,
  because chroma subsampling and YBR conversion legitimately differ.
- **Geometry:** world coordinates within 1e-6 mm, canvas within a quarter pixel.

**A tolerance change is a pull request with a rationale, reviewed like code.**
Tuning tolerance per failure is how a suite stops meaning anything. If a case
fails and the tolerance looks wrong, that is a finding to report, not a number
to edit.

## Reading a failure

Work through these in order. The first three are far more common than a real
port defect, and each has a different fix.

1. **Is the corpus intact?** `python3 scripts/corpus_check.py`. A digest
   mismatch means the thing the tolerance was measured against moved, and every
   result in the run is meaningless, not just the failing one.
2. **Did cornerstone3D change?** Its version is pinned. An unpinned upgrade
   moves the reference, and the diff then measures the upgrade rather than our
   code.
3. **Is the divergence in metadata or in pixels?** A metadata diff points at
   parsing. A pixel diff with clean metadata points at the LUT chain or the
   shader.
4. **Which tier?** A case that passes on tier A and fails on tier B is a
   fallback defect, not an arithmetic one, and the fix is in the tier-B path.
5. **What is the shape of the difference?** A constant offset across the frame
   is a rescale or a window boundary. A gradient is a LUT function mismatch. A
   half-pixel shift is a sampling or an IPP-centre error. Scattered single
   pixels are rounding. **Look at the difference image before reading code.**

## Every field bug becomes a fixture

Story E2.6. When a divergence is found and fixed, the case that found it joins
the corpus permanently, with a manifest row. A bug found once and not captured
is a bug that returns.

## `--update-hashes`

Re-baselines the stable render hashes (story E2.7, F-015). Those hashes are
also the foundation of attestable rendering in HLD section 36, so a re-baseline
is a deliberate act with a recorded reason, not a way to make a run green.
