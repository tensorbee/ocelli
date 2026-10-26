# F-X003 applicable-corpus producer review, pass 7

**Reviewed**: staged tree `c21279c6aa1e5028b10297db4311978cb3b6aed1`
**Previous**: `289d1e0f7a84ccbe15845274187fc58b27b8a262`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 1 defect, 0 smells, 0 nitpicks in the producer review scope
**Date**: 2026-10-19

## Verdict

The producer is useful implementation progress and the implementing report
honestly preserves a failed comparison. It is not acceptable as completed AC5
evidence. One reproduced producer defect makes its geometry metadata unreliable.
The reported 67 pixel failures are unresolved candidate/reference mismatches,
not 67 independently attributed renderer defects. No claim is made that fixing
this defect will resolve every mismatch.

## Defects

### D10, copied reference geometry describes a camera the candidate ignores

**Where**: `tools/oracle/src/produce.rs:746`, `:760`, `:1103`, `:1122`.

**Contract**: F-X003 AC5 compares settled output for matched input, presentation
and geometry. HLD section 25.1 distinguishes geometry from display-code error.
The existing comparator's `Sidecar::camera`, geometry comparisons and canvas
scale contract consume the candidate's declared camera and scale as evidence.
Candidate metadata must describe the executed candidate state.

**Mechanism**: `raster_settled` creates a new View, applies spacing and flips,
then fits the source independently to the output extent. With pixel spacing
present, it does not apply camera.parallelScale or the recorded canvas scale.
It also does not apply camera placement/orientation as a general camera.
`candidate_sidecar` nevertheless clones the reference metadata and changes
only the frame digest and statistics. The comparator can therefore observe
identical camera and scale declarations for differently executed views.

**Synthetic trigger**: A 2 by 2 MONOCHROME2 source has samples `[0,40,0,40]`,
spacing `[1,1]`, slope 1, intercept 0, LINEAR centre 40 and width 1. The frame
is 8 by 8. Start with parallelScale 1 and declared canvas scale 4 per source
pixel. Then request parallelScale 2 and scale 2 per source pixel, keeping the
other fields fixed. Doubling the field of view must change this nonconstant
frame, or the producer must explicitly refuse unsupported camera state.
The relation follows the existing oracle contract:
millimetres per canvas pixel equals `2 * parallelScale / canvasHeight`.

**Actual**: Both requests render successfully and produce byte-identical
frames. The candidate sidecar for the second request declares parallelScale 2
and canvas scale 2. The first frame contains both opaque black and white, and
a MONOCHROME1 control changes its bytes, so the case is not a constant or
inactive-raster control.

**Expected**: Apply the requested camera using the repository's coordinate
contracts and describe the actual resulting geometry. If a state cannot be
represented, refuse it explicitly before publishing a misleading comparison.
Do not copy executed-state claims solely to satisfy metadata equality.

**Evidence**: Reviewed source was archived without changing the index into
`/private/tmp/ocelli-pass7-mjbpw_n9/source`. A synthetic test module named
`independent_pass7` was appended only to that isolated produce.rs. Command:

```bash
cd /private/tmp/ocelli-pass7-mjbpw_n9/source
CARGO_TARGET_DIR=/Users/atulsharma/Documents/projects/tensorbee/ocelli/target cargo test -p ocelli-oracle --lib independent_pass7 -- --nocapture
```

It exited 101 at the assertion requiring the zoomed frame to differ. Log:
`/private/tmp/ocelli-pass7-probe.log`. The test source is at the isolated
produce.rs after the original module. All input and output are synthetic.
For future replay use a separate target directory as required below.

**Scope**: Confirmed in the supplied index. The producer is absent from the
reported HEAD. This finding concerns candidate geometry and comparison
attribution. It does not reopen D9 or establish whether a particular real
corpus pixel mismatch originates in rendering, reference behavior or setup.

**Regression and repair boundary**: Add permanent nonconstant synthetic camera
controls through the actual producer, with applied-state metadata checks.
Cover zoom and a placement/orientation case that the implementation claims
to support. Verify pixels respond correctly or an unsupported state is
explicitly refused. Preserve selected-row dispositions. Do not drop failing
rows, change fit/sampling policy or widen tolerance merely to obtain green.
Retain semantic old-source RED and repaired-source GREEN.

## Checks and evidence identity

Rebuilt the original source and ran:

```bash
bin/ocelli.sh test ocelli-oracle --lib -- --skip applicable_monochrome_rows_reach_the_comparator
```

Exit 0, 89 passed and one filtered. Log:
`/private/tmp/ocelli-pass7-library-rebuilt.log`. This confirms the existing
library suite passes while the independent camera control fails.

An initial library run is discarded: sharing a Cargo target directory with
the isolated source selected its probe executable. That run contained the
extra independent test and is not evidence against the original library
suite. Cleaned only ocelli-oracle build artifacts and rebuilt the original
source for the 89-pass result above. Future isolated copies use distinct
build outputs. No tracked product source was modified by the probe.

Did not rerun the corpus producer or candidate gate. Counts of 85 selected,
84 produced/compared, 67 failed comparisons, 17 unmeasured and zero passes
are the implementing agent's reported run in
`S16-repair-evidence-pass-6.md`, not checks executed by this reviewer.
Their red disposition is retained. Resource execution, successful production,
comparator execution and numerical acceptance remain separate claims.

The staged unsafe-check failure is reported separately from pixel failures.
Inspection confirms that script's staged mode selects changed paths, while
its vendor ratchet expects both recorded vendor packages. No allow-list or
guard was changed here. This does not justify describing every required
check as green.

## Instruction refinements and next handoff

Added applied-state metadata requirements to canonical `design.md`,
`microscope.md` and `dicom-tooling`. Added red-comparison diagnosis guidance
to dicom-tooling: preserve the run, group layout, geometry/parameter,
in-image arithmetic/quantization and background signatures, and reduce
representatives independently before attributing defects. Updated `verify.md`
to isolate build outputs for alternate source copies, based on the discarded
review experiment above. These are small additions to existing contracts.

Regenerated adapters. The new guidance is unstaged and requires independent
review. Instruction pass 4 covers its recorded version, not these additions.
Record the observed triggers in the next actual AS_BUILT entry. No second
tracker, new gate or changed acceptance policy was added.

The implementing agent should repair D10 first, update the design's copied
sidecar account to the actual applied-state contract, and add permanent
producer-level regressions. Independently review the new guidance additions.
Then regenerate candidate evidence with matched applied parameters and retain
the original red run. Classify residual failure signatures and prepare
hand-derived reproducers before changing product arithmetic. The existing
left-aligned refusal, one-code differences, nonsquare geometry, sigmoid and
letterbox-only cases are investigation leads, not pre-attributed defects.

Return the staged repair tree, exact checks and new run dispositions for a
fresh independent pass. AC5 remains unmet. Interaction instruments, collector
work and the absent representative host remain separate. Stories remain in
progress. No whole-sprint approval, verification receipt, commit, push, close
or new sprint is supplied.
