# Current sprint, S39

**Milestone**: M12, migration and rollout.
**Branch**: `sprint/s39`
**Opened**: 2026-11-03
**Goal**: Per-viewport strangler migration behind feature flags, Shadow mode: render both in production, alert on divergence.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-107 | E17.3 | Per-viewport strangler migration behind feature flags | TS | 4w | done |
| F-109 | E17.5 | Shadow mode: render both in production, alert on divergence | TS | 5w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S39 / {print $2, $9}'
```

## What this sprint is

S38 closed at tag `s38`. The merge is `3553c4d`. F-106 is done. Closing
S38 did not complete M12 and did not publish. The workspace version stays
0.1.0.

This sprint finishes M12. HLD section 12 says a feature flag is per
viewport and that stack viewports move first, then MPR, then 3D. Shadow
mode compares both presentations and alerts before the flag flips. These
stories do not import the other library. F-106 already keeps the neighbour
element.

Closing this sprint completes M12 and does not publish. M12 is S37 to S39.

## What is carried in

- **S38 is closed at tag `s38`, and that close did not complete M12 and
  did not publish.** The merge is `3553c4d`. The tag object is `6b2121ef`
  and peels to that merge. The sprint tip is `00010b6`, tree
  `aa951846e3a6`. F-106 is `06eeef4`. The open is `5814ca3`. The design is
  `9eec472`. No row in that close has a measured duration. No ratio is
  invented. The comparison report is unchanged. The S38 sprint gate
  measured wasm at 2,171,131 bytes.
- **S37 is closed at tag `s37`, and that close did not complete M12 and
  did not publish.** The merge is `2dcdaff`. The tag object is `994841ec`
  and peels to that merge.
- **M11 is complete at tag `s36`, and that close did not publish.** The
  merge is `ed60524`. The tag object is `ec222761` and peels to that
  merge.
- **M8 through M11 were not published.** The workspace version is still
  0.1.0. D-22 blocks publication because `openjph-core` 0.1.0 carries no
  BSD notice material. That does not block this sprint.
- **F-X003 stays `in-progress`.** It is not a story in this sprint. AC6
  needs a representative GPU-less incumbent host, and none is available.
- **The open LINEAR finding against `ocelli-pixel` stays unscheduled.**
  It has no F-ID. Clamping a readback is a tolerance change.
- **The SIGMOID file width stays on the product LUT.** File width 0.5 at
  centre 40 is about 4.586 under PS3.3 C.11.2.1.3.1. The product formula
  is not changed to settle the pinned renderer's recovered width.
- **The wasm size budget records 2,151,196 bytes** with tolerance 0.05.
  The ceiling is 2,258,755. The S38 sprint gate measured 2,171,131 bytes
  and passed. The budget file was not edited.
- **`max_review_passes` is stored and is not a stop.** The review loop
  continues until a pass reports zero defects and zero smells.
- **No new measured duration.** There is no ratio. This open does not
  recalculate capacity.
- **F-X011 remains pending.** ICC is still unimplemented. Colour stays
  unmeasured under D-16.

## The defect class this sprint is exposed to

**A flag that enables a later viewport kind while an earlier kind is off,
or a shadow compare that writes a tolerance or renders a different image
quietly.** Stack stays before MPR, and MPR stays before volume. A refused
flag changes nothing. A shadow compare returns a count or nothing. It does
not edit the corpus comparator. dwv, Horos, and the medical viewer Grok
must not be opened.

## What done means

- **F-107** keeps three kind flags. They start off. MPR cannot turn on
  while stack is off. Volume cannot turn on while MPR is off. Turning
  stack off while MPR is on does nothing. An element is enabled only when
  its kind flag is on. Turning that flag off disables the elements this
  story enabled for that kind.
- **F-109** counts bytes whose absolute difference is above a caller
  limit. Different lengths return nothing. The buffers are not written.
  The function does not render and does not import the other library.
- **Each story declares all three tier rows.**

## Dependency order

F-107 and F-109 both depend on F-106. That row is `done`. They share the
delivery ledgers, so F-107 lands first and F-109 lands second. F-X003
stays `in-progress` and is not a dependency.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation. **`wgpu` is
pinned exactly and agents confidently emit APIs that have not existed for
two years**, so every wgpu call is checked against the pinned version's own
documentation rather than from memory.

No patient data enters a prompt, tracked file, fixture, log, error or
commit. The ignored corpus remains behind `corpus/manifest.tsv`.

A feature that cannot run on the resolved tier **reports unavailable**. It
never quietly produces a different result.

## Audit intake, 2026-11-03

S38 closed at tag `s38`. The merge is `3553c4d`. Closing S38 did not
complete M12 and did not publish. The S39 allocation rows and the S39 plan
table name F-107 and F-109. F-X003 stays in progress. F-X011 remains
pending.
