# Current sprint, S47

**Milestone**: M21, one showcase viewer and usable feature completion, and M20, local folder viewer.
**Branch**: `sprint/s47`
**Opened**: 2026-11-11
**Goal**: One showcase viewer, the equipped comparison's reference scalar route, and the M20 folder viewer follow-ups, with DICOM video and the VOI shader's tier fixture beside them. The full goal line is the S47 entry in `SPRINT_PLAN.md`.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-202 | E38.12 | Worker decode and owned frame render in the single showcase viewer | All | 6w | pending |
| F-204 | E38.14 | One stored pixel per device pixel in the single showcase viewer | All | 2w | pending |
| F-205 | E38.15 | Validated shared preferences in the single showcase viewer | All | 3w | pending |
| F-235 | E6.9 | VOI shader stage 1 matches LutChain on every tier | Rust | 2w | pending |
| F-241 | E4.11 | DICOM video in the folder viewer and @ocelli/core | All | 12w | pending |
| F-245 | E38.40 | One showcase viewer for local, DICOMweb and future user-facing workflows | All | 4w | in-progress |
| F-266 | E38.61 | Built example viewer loads its core from its own output | TS | 1w | prepared |
| F-267 | E38.62 | Folder viewer corners on enhanced multiframe CT and MR | Rust | 2w | pending |
| F-268 | E19.8 | Rustdoc links resolve | Build | 1w | done |
| F-269 | E4.14 | Vendored ritk-codecs JPEG-LS comments audited against ITU-T T.87 | Rust | 1w | prepared |
| F-272 | E2.8 | Oracle tests write nothing into the checkout | Test | 1w | prepared |
| F-273 | E38.64 | Reference scalar normalization for the equipped comparison | All | 3w | in-progress |
| F-278 | E38.68 | Comparison page states what it is | TS | 1w | pending |

**The Status column above is hand-typed from run state at the S47 sync and
nothing derives it, so it goes stale.** `docs/sprints/BACKLOG.md` is the
authority for status and `.claude/scratch/S47-run.json` for worker state.

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S47 / {print $2, $9}'
python3 scripts/sprint_workflow.py status --sprint S47
```

## What this sprint is

S46 is closed at tag `s46` and published nothing. This sprint starts M21,
which makes one viewer carry every developed user-facing feature, and keeps
going on M20's folder viewer follow-ups. F-245 is the one showcase viewer that
F-202, F-204 and F-205 build on. F-273 gives the equipped cornerstone3D
comparison a reviewed scalar and upload route, and F-278 makes the published
comparison page say what kind of observation it is. F-235 proves the VOI
shader's rescale against `ocelli_pixel::modality` on a fused multiply and add
backend. F-241 adds DICOM video. Closing this sprint does not publish.

## What is carried in

- **F-223 carried to F-273.** F-223's pixel precheck refused, so its AC18 and
  AC19 equipped acceptance could not be claimed. F-273 continues that work.
  The real-study hold on F-273 stays until the operator lifts it.
- **Work started before this sync.** Isolated S47 work was authorised during
  S46 after design approval. F-245 and F-273 are in progress in GLM worker
  checkouts, and F-266, F-268, F-269 and F-272 are prepared with their review
  passes in run state. Their branches are preserved. Their records were kept
  exactly when F-278 was added at this sync.
- **F-245 waits on F-268's shared slice** for viewer-browser registration.
- **F-278 follows F-273**, because the page it corrects is F-273's output.
- **D-22 still blocks publication.** The version stays 0.1.0.
- **RUSTSEC-2021-0153** on the `encoding` crate through dicom-rs stays an
  accepted risk, as the operator decided on 2026-11-05. No gate reports Rust
  advisories.
- **The S46 review follow-ups** F-274, F-275, F-277, F-279 and F-280 are in
  S48 and F-276 in S50. F-280 gives every folder viewer window change one
  owner, after the S46 review found a late window reply overwriting a newer
  choice in three different setters. Until F-280 lands, any S47 story that
  adds a window setter guards it the same way, with a browser control that
  holds the late request.

## The defect class this sprint is exposed to

**A scalar that changes on its way to the GPU.** F-273's route uploads stored
values for the reference side, and the quiet failure is a full-range signed
value clipped or wrapped by an unsigned or half-float upload, or a rescale
applied twice, once in the route and once in the shader. The pixel precheck
passes on the synthetic rows and draws a wrong value on the real study.
F-235 is the same class seen from the other side: one fused rounding in the
shader's `slope * stored + intercept` gives a different bit from
`ocelli_pixel::modality`, invisible until a window edge lands on it. F-241's
version is `YBR_PARTIAL_420` converted twice, or at full range where PS3.3
C.7.6.3.1.2 gives partial range.

## What done means

- **F-273** is done when the declared scalar route preserves full-range
  signed values through pixel precheck and the complete timed protocol, the
  native pixel arithmetic and the acceptance thresholds unchanged. The
  equipped and real-study acceptance needs the operator.
- **F-245** is done when the one viewer serves local, DICOMweb and later
  workflows, its browser suite is registered in `viewer-browser`, and the
  More page comparison matrix states each row truthfully.
- **F-241** is 12 weeks against a sprint. It may carry, and a carry needs its
  recorded reason under `## Carried forward from S47`.
- **A prepared story** is done only after integration onto `sprint/s47` and
  its review loop and feature profile there. A worker's own pass is not
  integration.
