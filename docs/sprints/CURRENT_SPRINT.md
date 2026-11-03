# Current sprint, S38

**Milestone**: M12, migration and rollout.
**Branch**: `sprint/s38`
**Opened**: 2026-11-03
**Goal**: Side-by-side runtime: both libraries alive in one application.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-106 | E17.2 | Side-by-side runtime: both libraries alive in one application | TS | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S38 / {print $2, $9}'
```

## What this sprint is

S37 closed at tag `s37`. The merge is `2dcdaff`. F-105 and F-108 are done.
Closing S37 did not complete M12 and did not publish. The workspace version
stays 0.1.0.

This sprint continues M12. HLD section 12 says both libraries coexist in
the application. The feature flag per viewport is F-107. Shadow mode is
F-109. This story keeps an Ocelli element and a neighbour element alive
together. Ocelli enables only its own element. It does not import the
other library.

Closing this sprint does not complete M12 and does not publish. M12 is
S37 to S39.

## What is carried in

- **S37 is closed at tag `s37`, and that close did not complete M12 and
  did not publish.** The merge is `2dcdaff`. The tag object is `994841ec`
  and peels to that merge. The sprint tip is `430967e`, tree
  `1409c3c32602`. F-105 is `b6c7bdb`. F-108 is `9068255`. The open is
  `e4562ff`. The design is `ab92b6d`. No row in that close has a measured
  duration. No ratio is invented. The comparison report is unchanged.
  The S37 sprint gate measured wasm at 2,171,131 bytes.
- **M11 is complete at tag `s36`, and that close did not publish.** The
  merge is `ed60524`. The tag object is `ec222761` and peels to that
  merge.
- **M10 is complete at tag `s34`, and that close did not publish.** The
  merge is `c3f85e5`. The tag object is `bff12197` and peels to that
  merge.
- **M9 is complete at tag `s32`, and that close did not publish.** The
  merge is `7ae7559`. The tag object is `1a255447` and peels to that
  merge.
- **M8 is complete at tag `s29` and was not published.** A finished M8
  would name 0.8.0. M1 through M11 were not published. The workspace
  version is still 0.1.0. `/release` runs
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
  The ceiling is 2,258,755. The S37 sprint gate measured 2,171,131 bytes
  and passed that budget. The budget file was not edited. `--accept-size`
  is a design-plan declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** The S37 tracker rows read `not measured`.
  There is no measured total and no ratio. The capacity points remain the
  S03 and S06 closes. This open does not invent a ratio and does not
  recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A release of one element that also disables the other, or a package
import that turns the new API into a shim.** HLD section 12 keeps both
libraries in the application. Decision D1 refuses a drop-in shim. Ocelli
enables its own element and leaves the neighbour's listeners in place.
dwv, Horos, and the medical viewer Grok must not be opened.

## What done means

- **F-106** attaches one Ocelli element and one neighbour. The same object
  cannot be both. The neighbour keeps listeners Ocelli did not register.
  Releasing disables only the Ocelli element. A failed Ocelli enable leaves
  the neighbour untouched. The feature flag and shadow mode stay later
  stories.
- **The story declares all three tier rows.** An omitted row is not an
  answer. A feature that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-106 depends on F-105. That row is `done`. F-107 and F-109 depend on
F-106 and are not in this sprint. F-X003 stays `in-progress` and is not
a dependency.

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

## Audit intake, 2026-11-03

S37 closed at tag `s37`. The merge is `2dcdaff`. Closing S37 did not
complete M12 and did not publish. The S38 allocation row and the S38 plan
table name F-106. F-X003 stays in progress. F-X011 remains pending.
