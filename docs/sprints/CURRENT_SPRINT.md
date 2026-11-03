# Current sprint, S40

**Milestone**: M13, performance, hardening and release.
**Branch**: `sprint/s40`
**Opened**: 2026-11-03
**Goal**: Binary size budget: Naga trimming, wasm-opt, split builds, Worker pool tuning and the threading escalation decision, Memory profiling under 4D and large-volume load, Browser matrix validation and WebGL2 tier certification.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-110 | E18.1 | Binary size budget: Naga trimming, wasm-opt, split builds | Build | 4w | done |
| F-112 | E18.3 | Worker pool tuning and the threading escalation decision | Build | 4w | done |
| F-113 | E18.4 | Memory profiling under 4D and large-volume load | Build | 3w | done |
| F-114 | E19.1 | Browser matrix validation and WebGL2 tier certification | All | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S40 / {print $2, $9}'
```

## What this sprint is

S39 closed at tag `s39`. The merge is `f115491`. Closing S39 completed M12
and did not publish. The workspace version stays 0.1.0.

This sprint starts M13. HLD section 15.2 sets the release profile that the
size budget measures. HLD section 9 starts one wasm instance per decode
worker and keeps shared-memory threading off unless a measurement demands
it. HLD section 8 gives one budget and three tiers. HLD section 7 names
tier B as WebGL2, and spike A7 widens the browser matrix so a software
rasteriser is certified as tier C rather than as tier B.

Closing this sprint does not complete M13 and does not publish. M13 is S40
to S41.

## What is carried in

- **S39 is closed at tag `s39`, and that close completed M12 and did not
  publish.** The merge is `f115491`. The tag object is `ebd7a4de` and peels
  to that merge. The sprint tip is `dbc4c9c`, tree `5e41feb08a96`. F-107 is
  `bf8981a`. F-109 is `238329c`. The open is `0d73b9a`. The design is
  `c446a84`. No row in that close has a measured duration. No ratio is
  invented. The comparison report is unchanged. The S39 sprint gate
  measured wasm at 2,171,131 bytes.
- **S38 is closed at tag `s38`, and that close did not complete M12 and
  did not publish.** The merge is `3553c4d`. The tag object is `6b2121ef`
  and peels to that merge.
- **M12 is complete at tag `s39`, and that close did not publish.** M12 is
  S37 to S39.
- **M8 through M12 were not published.** The workspace version is still
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
  The ceiling is 2,258,755. The S39 sprint gate measured 2,171,131 bytes
  and passed. The budget file was not edited by that close.
- **`max_review_passes` is stored and is not a stop.** The review loop
  continues until a pass reports zero defects and zero smells.
- **No new measured duration.** There is no ratio. This open does not
  recalculate capacity.
- **F-X011 remains pending.** ICC is still unimplemented. Colour stays
  unmeasured under D-16.

## The defect class this sprint is exposed to

**A decode module that still links Naga while the record says the build is
split, a size move that is not declared, a worker pool that enables
shared-memory threading without a measurement, a memory profile that
reports bytes it did not compute, or a browser row marked certified when
that browser was not run.** wasm-opt stays on. Stack, MPR and volume flag
order from S39 stays. A shadow count still does not edit the corpus
comparator. dwv, Horos, and the medical viewer Grok must not be opened.

## What done means

- **F-110** keeps wasm-opt on for the render module. The decode build of
  the same crate does not link `wgpu` or Naga. Native backends stay on the
  native build. A size move outside the recorded tolerance is declared
  with a cause. The budget file is not edited to hide a move.
- **F-112** opens N decode workers only through a caller count. Each call
  constructs one worker. SharedArrayBuffer stays unused. The escalation
  decision stays on stable Rust because this sprint has no measurement
  that demands nightly threads.
- **F-113** reports encoded, decoded and GPU bytes for a synthetic load
  from dimensions, frame count and bytes per sample. Encoded bytes are
  zero after decode. The report does not allocate the volume and does not
  read patient bytes.
- **F-114** certifies a tier only for a browser session that ran. Tier B
  and tier C are separate rows. A browser that did not run stays not run.
- **Each story declares all three tier rows.**

## Dependency order

F-110 and F-114 depend on F-042. F-112 depends on F-035. F-113 depends on
F-034. Those three rows are `done`. The four stories share the delivery
ledgers, so they land one at a time. F-110 lands first because it changes
the wasm graph. F-112, F-113 and F-114 follow. F-X003 stays `in-progress`
and is not a dependency.

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

S39 closed at tag `s39`. The merge is `f115491`. Closing S39 completed M12
and did not publish. The S40 allocation rows and the S40 plan table name
F-110, F-112, F-113 and F-114. F-X003 stays in progress. F-X011 remains
pending.
