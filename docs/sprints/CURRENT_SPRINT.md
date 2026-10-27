# Current sprint, S17

**Milestone**: M4, the public API, the boundary, and the stack viewport.
**Branch**: `sprint/s17`
**Opened**: 2026-10-26
**Goal**: Serialise the camera, the display area and the presentation state,
bind the viewport to React, reshape and document the cornerstone event
catalogue, and generate the boundary's TypeScript types from the Rust
definitions.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-050 | E7.6 | Camera, display-area and presentation-state serialisation | Rust+TS | 3w | done |
| F-102 | E16.3 | Framework bindings (React first) | TS | 3w | done |
| F-103 | E16.4 | Event model: the ~103 cornerstone events, re-shaped and documented | TS | 4w | done |
| F-104 | E16.5 | TypeScript type generation from Rust definitions | TS | 4w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S17 / {print $2, $9}'
```

## What this sprint is

S16 closed the typed command channel, the polled event ring, the state
readback, the canvas and world and index adapters, and the CPU stack raster.
It also preserved a selected VOI sequence across a manual window change.
S17 continues M4. F-050 makes the camera, the display area and the
presentation state survive a round trip rather than living only in a live
viewport. F-102 binds that surface to React. F-103 reshapes cornerstone's
event catalogue into Ocelli's own and documents the mapping. F-104 generates
the boundary's TypeScript types from the Rust definitions, so the wire has
one source of truth.

M4 runs from S15 through S17. The plan calls this the Phase 1 credibility
gate, because the three-channel boundary is real and a stack viewport diffs
clean against cornerstone3D. S17 is the last sprint of M4, so closing it
completes the milestone. The oracle still has no Ocelli renderer to compare
a full viewer against. Decision D7 holds. This open publishes nothing.

## What is carried in

- **F-X003 is carried out of S16 and is not scheduled here.** AC6 needs a
  representative GPU-less incumbent host, the operator has confirmed none is
  available, and no local diagnostic is accepted as that measurement. AC5's
  coverage verdict is `coverage-loss` with 67 matched views, 0 pixel failures
  and 17 unmeasured. Its backlog row stays `in-progress`. Resume when a
  representative host, an incumbent workload and an accepted budget exist.
- **The open LINEAR finding against `ocelli-pixel`'s own arithmetic** stays
  unscheduled. It has no F-ID. Clamping it so a readback matches is a
  tolerance change.
- **The SIGMOID reference-width diagnosis** stays open in
  `docs/lld/oracle.md`. The rendered reference codes fit width 0.5 while the
  installed transfer-function path statically yields width 1.5. An oracle run
  is needed to settle it. Do not change DICOM arithmetic or the parity policy
  to settle it.
- **Tier B greyscale has been drawn.** F-042 ran the section 18.3 rows on
  this host through headless Chromium WebGL2 and matched `LutChain::map_into`
  within 0.001, format `R32Float`. A tier B colour LUT readback was not run.
- **The wasm size budget records 2,151,196 bytes.** F-050 measured the release
  module on 2026-10-27 and re-baselined `ci/wasm-size-budget.json`. The
  previous baseline was 1,999,061 bytes. S16 measured 2,052,951 bytes against
  that older baseline and did not move the file. `bearing_on_gate_A4` says
  this number does not close A4, and cold start was not remeasured.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The shared instructions make the run-sprint host
  responsible for counting whole-sprint passes and invoking `set-phase
  blocked` at the configured limit. The per-feature microscope loop remains
  uncapped. No automatic script transition is claimed.
- **M1, M2 and M3 were not published, and M4 is not complete until this
  sprint closes.** The cadence names 0.1.0 for M1, 0.2.0 for M2 and 0.3.0 for
  M3. The workspace version is still 0.1.0. `/release` step 5 runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. None of that blocks a story here. All of it blocks
  publication. No release is due at this open.
- **No new measured duration.** Every S16 row in `SPRINT_TRACKER.md` reads
  `not measured`. The capacity points remain the S03 and S06 closes. This
  open does not invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Multi-component JPEG-LS decoding is still owed.
- **The oracle has no Ocelli renderer to compare a full viewer against.**
  F-X021 writes the candidate record and depends on F-012 and F-052. It is
  not this sprint. Decision D7 holds.

## The defect class this sprint is exposed to

**A serialised state that round-trips only in the implementation's own
representation.** HLD section 7 and F-050. A camera written as sixteen bare
numbers, a display area that loses its origin, or a presentation state that
restores a selected VOI sequence as a linear window is the defect. Canvas
points, world points and indices stay distinct across the wire. A serialised
state is versioned, and a reader refuses a version it does not know rather
than guessing. Restoring a state must not upload pixels and must not add a
second camera matrix.

**A second event catalogue, or a boundary crossing per event.** Decisions
D2, D3 and D4. F-103 reshapes cornerstone's event catalogue into Ocelli's
own and documents the mapping. It does not run a second ring. F-102 binds
React to the drained ring, not to a callback per event. A React component
that invokes the wasm boundary once per pointer move is the defect, and so
is a catalogue that disagrees with the events the ring delivers.

**Two sources of truth for the boundary types.** F-104 generates the
TypeScript types from the Rust definitions. A hand-written interface that
drifts from the Rust type is the defect. The generated file is checked, and
a change to a Rust boundary type that does not regenerate it must fail a
gate rather than pass silently.

## What done means

- **F-050** serialises the camera, the display area and the presentation
  state through one versioned shape. A round trip preserves the selected VOI
  sequence and does not replace it with linear numbers. Canvas points, world
  points and indices stay non-interchangeable. A reader refuses an unknown
  version. The state does not upload pixels and does not add a second camera
  matrix.
- **F-102** binds the viewport to React first. The binding uses the typed
  command channel and the drained event ring. It does not cross the boundary
  per event and does not expose a pixel buffer. It does not reshape the event
  catalogue, which is F-103.
- **F-103** reshapes cornerstone's event catalogue into Ocelli's catalogue,
  maps every event to its boundary delivery, and documents the mapping. One
  catalogue, not two. A delivered event is the one the ring carries. No
  per-event callback.
- **F-104** generates the boundary's TypeScript types from the Rust
  definitions. One source of truth. The generated types are checked against
  the Rust types, and drift fails a gate. Hand-editing a generated type is
  not the answer.
- **Every story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device.

## Dependency order and the shared resources

F-050 depends on F-048. F-102, F-103 and F-104 depend on F-101. All of those
backlog dependencies are `done`. No backlog cell in this sprint names
another story in this sprint, so no story begins blocked by that column.

**One boundary, one catalogue, one generated type set.** F-102, F-103 and
F-104 share the command protocol, the event ring and the boundary types that
F-101 built. F-104 is the source of truth those three consume. The designs
land before implementation.

F-X003 is carried and not scheduled. F-X004 depends on F-X003 and is not this
sprint. F-X005 also depends on F-X003.

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

## Audit intake and preparation, 2026-10-26

S16 closed at tag `s16` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 3 clean at zero defects and zero smells.
`scripts/backlog_check.py`, `scripts/gen_sprint_plan.py --check` and
`scripts/deviation_check.py` are green at this open. Allocation and
`SPRINT_PLAN.md` agree on the four S17 stories and on their 14 estimated
engineer-weeks. No new audit was run at this open.

`/prepare-sprint` is the next command. It writes the draft designs and stops
before implementation, verification receipts, commits or publication. No
story is in progress, so no design plan exists yet.
