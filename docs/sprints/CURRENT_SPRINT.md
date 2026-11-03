# Current sprint, S36

**Milestone**: M11, annotation state and DICOM interop.
**Branch**: `sprint/s36`
**Opened**: 2026-11-03
**Goal**: Measurement persistence and serialisation format, DICOM SR TID 1500 round-trip.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-096 | E15.3 | Measurement persistence and serialisation format | Rust+TS | 3w | done |
| F-097 | E15.4 | DICOM SR TID 1500 round-trip | Rust+TS | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S36 / {print $2, $9}'
```

## What this sprint is

S35 closed at tag `s35`. The merge is `ea3f140`. F-094, F-095, F-098 and
F-099 are done. Closing S35 did not complete M11 and did not publish.
The workspace version stays 0.1.0.

This sprint finishes M11. Decision D13 says the annotation type is a
DICOM SR content tree. Decision D10 keeps dcmjs for SR generation.
HLD section 34 names two containers, the content tree for semantics and
a bulk binary once counts pass a few thousand. The section does not give
either byte layout. HLD section 10 keeps the TID 1500 bridge in
TypeScript. The plans decide the layouts locally and say so. F-070
already owns measurement values. These stories do not replace that
arithmetic.

Closing this sprint completes M11 and does not publish.

## What is carried in

- **S35 is closed at tag `s35`, and that close did not complete M11 and
  did not publish.** The merge is `ea3f140`. The tag object is `466ffac`
  and peels to that merge. The sprint tip is `1986cf2`, tree
  `b3315fb23248`. F-094 is `32d7534`. F-095 is `4c13d23`. F-098 is
  `bec0072`. F-099 is `2ac7844`. The open is `1c133de`. The design is
  `79eb0ae`. No row in that close has a measured duration. No ratio is
  invented. The comparison report is unchanged.
- **M10 is complete at tag `s34`, and that close did not publish.** The
  merge is `c3f85e5`. The tag object is `bff12197` and peels to that
  merge.
- **M9 is complete at tag `s32`, and that close did not publish.** The
  merge is `7ae7559`. The tag object is `1a255447` and peels to that
  merge.
- **M8 is complete at tag `s29` and was not published.** A finished M8
  would name 0.8.0. M1 through M10 were not published. The workspace
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
  The ceiling is 2,258,755. The S35 sprint gate measured 2,171,131 bytes
  and passed that budget. The budget file was not edited. `--accept-size`
  is a design-plan declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** The S35 tracker rows read `not measured`.
  There is no measured total and no ratio. The capacity points remain the
  S03 and S06 closes. This open does not invent a ratio and does not
  recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A second measurement model beside the SR content tree, or a TID 1500
document encoded without dcmjs.** HLD section 34 says the in-memory type
is the content tree. Decision D10 keeps dcmjs for SR generation. A private
measurement struct, or a Rust template writer, is the defect. dwv, Horos,
and the medical viewer Grok must not be opened. The SR idea is taken from
DICOM PS3.3 and PS3.16.

## What done means

- **F-096** persists the F-094 content tree in one JSON document and
  reads it back through `contentTree`. At the local bulk count it returns
  nothing. It does not invent the bulk binary layout.
- **F-097** writes and reads a TID 1500 measurement through dcmjs. The
  number, the unit and the concept survive. It does not reimplement the
  template in Rust.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-096 depends on F-094. F-097 depends on F-095. Those rows are `done`.
The two stories share the shell index, so they land one after the other.
F-096 is first. F-X003 stays `in-progress` and is not a dependency.

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

S35 closed at tag `s35`. The merge is `ea3f140`. Closing S35 did not
complete M11 and did not publish. The S36 allocation rows and the S36
plan table name F-096 and F-097. F-X003 stays in progress. F-X011 remains
pending.
