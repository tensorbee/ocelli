# Current sprint, S41

**Milestone**: M13, performance, hardening and release.
**Branch**: `sprint/s41`
**Opened**: 2026-11-03
**Goal**: Cold-start and first-frame optimisation, Accessibility and keyboard interaction parity, Documentation, examples and the migration playbook, Release engineering: semver, changelog, deprecation policy, Provenance recording for agent-generated code.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-111 | E18.2 | Cold-start and first-frame optimisation | Build | 3w | done |
| F-115 | E19.2 | Accessibility and keyboard interaction parity | All | 3w | done |
| F-116 | E19.3 | Documentation, examples and the migration playbook | All | 4w | done |
| F-117 | E19.4 | Release engineering: semver, changelog, deprecation policy | All | 2w | pending |
| F-118 | E19.5 | Provenance recording for agent-generated code | All | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S41 / {print $2, $9}'
```

## What this sprint is

S40 closed at tag `s40`. The merge is `ce27223`. Closing S40 did not
complete M13 and did not publish. The workspace version stays 0.1.0.

This sprint finishes the planned M13 range. HLD section 23 says measure
before optimising, and the cold-start subject is release-profile only.
HLD section 27.2 R6 wants a provenance trailer on every commit, written
from a ledger rather than by hand. `docs/RELEASE.md` says 1.0.0 is gated
on M13 and that F-117 sets the semver, changelog and deprecation policy.
This sprint does not publish.

Closing this sprint completes M13 and the planned Phase 1 range. It does
not publish. Phase 1.5 starts at S42 and this close does not open it.

## What is carried in

- **S40 is closed at tag `s40`, and that close did not complete M13 and
  did not publish.** The merge is `ce27223`. The tag object is `730fdcac`
  and peels to that merge. The sprint tip is `1a747da`, tree
  `8234db09422f`. F-110 is `a305453`. F-112 is `b7a9992`. F-113 is
  `39fd74d`. F-114 is `05c7691`. The open is `5164eba`. The design is
  `e17e73e`. No row in that close has a measured duration. No ratio is
  invented. The comparison report is unchanged. The S40 sprint gate
  measured the render wasm module at 2,171,133 bytes and the decode wasm
  module at 16,455 bytes. The render budget file was not edited.
- **S39 is closed at tag `s39`, and that close completed M12 and did not
  publish.** The merge is `f115491`.
- **M13 is S40 to S41.** This open does not complete it.
- **M8 through M12 were not published.** The workspace version is still
  0.1.0. D-22 blocks publication because `openjph-core` 0.1.0 carries no
  BSD notice material. That does not block this sprint, and it still
  blocks publication.
- **F-X003 stays `in-progress`.** It is not a story in this sprint. AC6
  needs a representative GPU-less incumbent host, and none is available.
- **The open LINEAR finding against `ocelli-pixel` stays unscheduled.**
  It has no F-ID. Clamping a readback is a tolerance change.
- **The SIGMOID file width stays on the product LUT.** File width 0.5 at
  centre 40 is about 4.586 under PS3.3 C.11.2.1.3.1. The product formula
  is not changed to settle the pinned renderer's recovered width.
- **The wasm size budget records 2,151,196 bytes** with tolerance 0.05.
  The ceiling is 2,258,755. The S40 sprint gate measured 2,171,133 bytes
  and passed. The budget file was not edited by that close.
- **`wasm.cold_start` stays the F-006 figure of 2.3 ms** until a later
  story measures the current release module and records its own
  calibration. That figure is not an answer to gate A4.
- **`max_review_passes` is stored and is not a stop.** The review loop
  continues until a pass reports zero defects and zero smells.
- **No new measured duration.** There is no ratio. This open does not
  recalculate capacity.
- **F-X011 remains pending.** ICC is still unimplemented. Colour stays
  unmeasured under D-16.

## The defect class this sprint is exposed to

**A cold-start figure copied from the F-006 baseline and reported as the
current module, a first-frame time that includes setup and calls it
decode, a keyboard path that claims a key the plan did not name, a
playbook sentence that states a formula the HLD does not, a policy change
that publishes or creates a `v` tag, or a provenance trailer typed into
the commit message by hand.** wasm-opt stays on. The render size budget
is not edited to hide a move. Stack, MPR and volume flag order from S39
stays. A shadow count still does not edit the corpus comparator. dwv,
Horos, and the medical viewer Grok must not be opened.

## What done means

- **F-111** measures cold start and first frame on the release artefact
  this sprint builds. A dev-profile time is not that measurement. The
  2.3 ms baseline is replaced only by a new measurement with its own
  calibration. Gate A4 stays open unless both of its halves are
  measurements of this module. The story does not edit the size budget
  to absorb a move.
- **F-115** adds keyboard actions on the existing tool commands. The HLD
  does not name the key map, so the approved plan does. A key outside
  that map does nothing. The story does not certify a browser row.
- **F-116** writes the migration playbook and the examples on top of
  F-108. It does not claim that 1.0.0 is published. It does not state a
  formula the HLD does not contain.
- **F-117** writes the semver, changelog and deprecation policy. It does
  not publish, does not create a `v` tag, and does not run `/release`.
  D-22 still blocks publication. F-X003 stays in progress.
- **F-118** records the generator the way HLD section 27.2 R6 specifies.
  The commit-msg hook writes the trailer from the verify ledger. A
  hand-written trailer is not a record. There is no agent co-author
  trailer.
- **Each story declares all three tier rows.**

## Dependency order

F-111 depends on F-110. F-115 depends on F-072. F-116 depends on F-108.
F-117 depends on F-100. F-118 depends on F-001. Those five rows are
`done`. The five stories share the delivery ledgers, so they land one at
a time. F-111 lands first because it measures the wasm graph F-110
split. F-115, F-116, F-117 and F-118 follow. F-X003 stays `in-progress`
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

S40 closed at tag `s40`. The merge is `ce27223`. Closing S40 did not
complete M13 and did not publish. The S41 allocation rows and the S41
plan table name F-111, F-115, F-116, F-117 and F-118. F-X003 stays in
progress. F-X011 remains pending.
