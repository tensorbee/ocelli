# Current sprint, S30

**Milestone**: M9, segmentation tools.
**Branch**: `sprint/s30`
**Opened**: 2026-11-02
**Goal**: Brush with sphere and circle brush geometry, SegmentSelect, SegmentLabel, SegmentBidirectional.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-080 | E13.1 | Brush with sphere and circle brush geometry | Rust+TS | 4w | done |
| F-084 | E13.5 | SegmentSelect, SegmentLabel, SegmentBidirectional | Rust+TS | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S30 / {print $2, $9}'
```

## What this sprint is

S29 closed at tag `s29`. The merge is `26f003c`. F-075, F-077, F-078 and
F-079 are done. M8 is complete. This open does not publish. The workspace
version stays 0.1.0.

This sprint owns a circle brush and a sphere brush on a caller label grid,
and the select, label and bidirectional readings of one segment. The stamp
writes a segment number. It does not draw. The existing labelmap presenter
still draws. The tools do not own the DICOM SR content tree. That type is
F-094.

F-080 depends on F-063. F-084 depends on F-066. Those rows are `done`. The
canonical worktree runs one story at a time. There is no worker branch.

Closing this sprint does not complete M9. M9 is S30 to S32, 7 stories and
24 engineer-weeks. A release is not taken from this open.

## What is carried in

- **M8 is complete at tag `s29`, and this open does not publish.** The
  merge is `26f003c`. No row in that close has a measured duration. No
  ratio is invented. A finished M8 would name 0.8.0. M1 through M8 were
  not published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and
  that command refuses while `openjph-core` 0.1.0 carries no BSD notice
  material, which is D-22. That blocks publication. It does not block
  this sprint.
- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. This sprint does not
  depend on it.
- **The open LINEAR finding against `ocelli-pixel`'s own arithmetic** stays
  unscheduled. It has no F-ID. Clamping it so a readback matches is a
  tolerance change.
- **The SIGMOID file width stays on the product LUT.** File width 0.5 at
  centre 40 is about 4.586 under PS3.3 C.11.2.1.3.1. The pinned renderer
  presents recovered width 1.5, and the candidate passes that window into
  the same function. The product formula is not changed to settle it.
- **Tier B greyscale has been drawn.** F-042 ran the section 18.3 rows on
  this host through headless Chromium WebGL2 and matched `LutChain::map_into`
  within 0.001, format `R32Float`. A tier B colour LUT readback was not run.
- **The wasm size budget records 2,151,196 bytes** with tolerance 0.05.
  The ceiling is 2,258,755. The S29 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** Every `SPRINT_TRACKER.md` row the S29 close
  inspected reads `not measured`. There is no measured total and no ratio.
  The capacity points remain the S03 and S06 closes. This open does not
  invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A stamp or a run that is quietly one sample off.** A sample on the radius
belongs inside the brush. A sample past the radius stays as it was. A
refusal writes nothing. Segment 0 is background and is not a named segment.
The longest run is the one the tie rule names. A shorter run that looks
central is still the wrong answer.

## What done means

- **F-080** stamps a circle or a sphere of segment numbers into the caller's
  label grids. The comparison is squared distance. A negative radius or a
  non-finite centre writes nothing.
- **F-084** reads one segment number, stores one label for a segment above
  0, and reports the longest horizontal and vertical runs. Background is
  not a label and not a bidirectional subject.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-080 depends on F-063. F-084 depends on F-066. Those two rows are `done`.
F-081, F-082, F-083 and F-085 depend on F-080 and stay in S31. F-094 owns
the SR type and is not a dependency of this sprint. F-X003 stays
`in-progress` and is not a dependency.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation. **`wgpu` is
pinned exactly and agents confidently emit APIs that have not existed for
two years**, so every wgpu call is checked against the pinned version's own
documentation rather than from memory. Treat GPU code that compiles first
try with suspicion.

No patient data enters a prompt, tracked file, fixture, log, error or
commit. The ignored corpus remains behind `corpus/manifest.tsv` and its
generators.

A feature that cannot run on the resolved tier **reports unavailable**. It
never quietly produces a different result, which is HLD section 31's rule
generalised by D-07.

## Audit intake, 2026-11-02

S29 closed at tag `s29`. The merge is `26f003c`. M8 is complete and this
open does not publish. The S30 allocation rows and the S30 plan table name
F-080 and F-084. F-063 and F-066 are done. F-X003 stays in progress.
F-X011 remains pending.
