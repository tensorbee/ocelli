# Current sprint, S32

**Milestone**: M9, segmentation tools.
**Branch**: `sprint/s32`
**Opened**: 2026-11-02
**Goal**: Between-slice labelmap interpolation.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-086 | E13.7 | Between-slice labelmap interpolation | Rust+TS | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S32 / {print $2, $9}'
```

## What this sprint is

S31 closed at tag `s31`. The merge is `a7828da`. F-081, F-082, F-083 and
F-085 are done. M9 is not complete until this sprint closes. This open
does not publish. The workspace version stays 0.1.0.

This sprint owns one tool. It writes a segment number into the slices
that sit strictly between slices that already hold that segment. It does
not draw. The existing labelmap presenter still draws. The tool does not
own the DICOM SR content tree. That type is F-094.

F-086 depends on F-080. That row is `done`. The canonical worktree runs
this story alone. There is no worker branch.

Closing this sprint completes M9. M9 is S30 to S32, 7 stories and 24
engineer-weeks. A release is not taken from this open.

## What is carried in

- **M8 is complete at tag `s29`, and this open does not publish.** The
  merge is `26f003c`. No row in that close has a measured duration. No
  ratio is invented. A finished M8 would name 0.8.0. M1 through M8 were
  not published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and
  that command refuses while `openjph-core` 0.1.0 carries no BSD notice
  material, which is D-22. That blocks publication. It does not block
  this sprint.
- **S31 is closed at tag `s31` and did not complete M9.** The merge is
  `a7828da`. The tag object is `f894be05` and peels to that merge. The
  sprint tip is `9ab9509`, tree `fb86414d`. F-081, F-082, F-083 and F-085
  stay as committed. The comparison report is unchanged.
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
  The ceiling is 2,258,755. The S31 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** Every `SPRINT_TRACKER.md` row the S31 close
  inspected reads `not measured`. There is no measured total and no ratio.
  The capacity points remain the S03 and S06 closes. This open does not
  invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A between-slice write that is one slice off, or a halfway sample that
is dropped.** An endpoint slice stays as the caller left it. A slice
closer to one end takes that end's mask. The exact halfway slice takes
the union. A slice outside the pair stays. A refusal writes nothing.
Segment 0 remains a legal written value and is still background for the
presenter.

## What done means

- **F-086** fills slices strictly between consecutive slices that already
  hold the caller segment. A slice whose step fraction is below one half
  takes the lower mask. A slice whose step fraction is above one half
  takes the upper mask. The exact half takes a sample when either end
  holds the segment. Endpoint slices are not rewritten. There is no
  TypeScript copy.
- **The story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-086 depends on F-080. That row is `done`. F-094 owns the SR type and is
not a dependency of this sprint. F-X003 stays `in-progress` and is not a
dependency.

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

S31 closed at tag `s31`. The merge is `a7828da`. M9 is not complete and
this open does not publish. The S32 allocation row and the S32 plan table
name F-086. F-080 is done. F-X003 stays in progress. F-X011 remains pending.
