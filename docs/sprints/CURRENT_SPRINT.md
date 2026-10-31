# Current sprint, S20

**Milestone**: M5, volume, MPR and 3D rendering.
**Branch**: `sprint/s20`
**Opened**: 2026-10-31
**Goal**: Oblique / arbitrary-plane reslicing, Thick slab with composite, MIP, MinIP and average blend modes, Slice-position sync and frame-of-reference handling, Multi-volume fusion with per-volume opacity and colormap (PET-CT), Activate the full-corpus candidate-render gate against Ocelli output.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-053 | E8.3 | Oblique / arbitrary-plane reslicing | Rust | 4w | done |
| F-054 | E8.4 | Thick slab with composite, MIP, MinIP and average blend modes | Rust | 4w | done |
| F-055 | E8.5 | Slice-position sync and frame-of-reference handling | Rust | 3w | done |
| F-056 | E8.6 | Multi-volume fusion with per-volume opacity and colormap (PET-CT) | Rust | 4w | done |
| F-X021 | Y1.16 | Activate the full-corpus candidate-render gate against Ocelli output | Test | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S20 / {print $2, $9}'
```

## What this sprint is

S19 closed at tag `s19`. The merge is `6808930`. M5 continues. This sprint
reslices a constructed volume on an arbitrary plane, draws a thick slab as
composite, MIP, MinIP or average, keeps a slice position inside one frame of
reference, fuses volumes that share that frame, and compares Ocelli output
with the oracle. The S20 table in `docs/sprints/SPRINT_PLAN.md` and the S20
rows in `docs/sprints/allocation.json` both name F-053, F-054, F-055, F-056
and F-X021. Transfer-function editing, lighting, cropping and the CPU volume
paths stay on S21.

F-052 samples the axial, sagittal and coronal planes of the constructed
volume. F-059 ray-casts ready bricks and skips a step whose residency flag
is 0. This sprint consumes those views. It does not reopen volume
construction and it does not reopen the LUT chain.

F-012 owns the candidate-directory contract. F-X021 connects Ocelli output
to that contract. A candidate directory filled with the reference is not
Ocelli output. Decision D7 holds. This open publishes nothing.

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
- **The wasm size budget records 2,151,196 bytes** with tolerance 0.05.
  F-050 measured the release module on 2026-10-27 and re-baselined
  `ci/wasm-size-budget.json` from 1,999,061 bytes. `bearing_on_gate_A4` says
  this number does not close A4, and cold start was not remeasured.
  `--accept-size` is a design-plan declaration, not a way to make a gate
  green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The shared instructions make the run-sprint host
  responsible for counting whole-sprint passes and invoking `set-phase
  blocked` at the configured limit. The per-feature microscope loop remains
  uncapped. No automatic script transition is claimed.
- **M5 is not complete, and this open does not publish.** S19 closed F-052,
  F-057, F-058 and F-059. `docs/RELEASE.md` maps one minor to each milestone
  from M2 to M12, so a finished M5 would name 0.5.0. S20 and S21 remain.
  M1, M2, M3 and M4 were not published. The workspace version is still
  0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. That blocks publication. It does not block these five
  stories.
- **No new measured duration.** The S19 rows in `SPRINT_TRACKER.md` read
  `not measured`, as does every earlier M5 and M4 row. There is no measured
  total and no ratio. The capacity points remain the S03 and S06 closes.
  This open does not invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Multi-component JPEG-LS decoding is still owed.

## The defect class this sprint is exposed to

**A sample that is not on the requested plane, or a blend that is a
different mode.** HLD section 7 and section 19. The volume has one origin,
one spacing and one direction. An oblique sample that steps along the slice
index instead of the requested plane is the wrong plane. Composite, MIP,
MinIP and average are distinct shader variants. Returning the first sample
and calling it a MIP is the wrong mode. Two volumes with different
`FrameOfReferenceUID` values are two candidates. Overlaying them as one view
is the wrong fusion. F-X021 judges Ocelli pixels. Copying the reference into
the candidate directory makes the comparison agree with itself.

HLD section 7 makes a 3D-texture ray-cast legal on tier B. Deviation D-07
says a feature that cannot run on the resolved tier reports unavailable.
F-X004 and F-X005 own the CPU MPR and CPU volume decision and are S21. This
sprint must not draw a different image on the CPU and call it the same view.

## What done means

- **F-053** samples an arbitrary plane through the constructed volume. The
  plane is that volume's origin, spacing and direction. It is not a rotation
  of the stored slice index and not a second patient frame.
- **F-054** draws a thick slab as composite, MIP, MinIP or average. The four
  modes are distinct. A mode that cannot run on the resolved tier reports
  unavailable. It does not return composite and name it MIP.
- **F-055** keeps a slice position inside one frame of reference. A second
  `FrameOfReferenceUID` is not the same position.
- **F-056** fuses volumes that share a frame of reference, with per-volume
  opacity and colormap. A PET-CT pair with two frames is not overlaid as if
  the frames matched.
- **F-X021** points the candidate-render gate at Ocelli output for the oracle
  views. The reference copied into the candidate directory is not that
  output.
- **Each story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- Transfer-function editing, lighting, cropping, CPU MPR and the CPU volume
  decision stay out. Those are F-060, F-061, F-062, F-X004 and F-X005.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device. The LUT arithmetic stays in `ocelli-pixel`.

## Dependency order

F-053, F-054, F-055 and F-056 depend on F-052. That backlog row is `done`.
F-X021 depends on F-012 and F-052. Both backlog rows are `done`. No backlog
cell in this sprint names another story in this sprint.

F-060, F-061 and F-062 depend on F-059 and are S21. F-X004 depends on F-052
and F-X003 and is S21. F-X005 depends on F-059 and F-X003 and is S21.

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

## Audit intake, 2026-10-31

S19 closed at tag `s19` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 1 clean at zero defects and zero smells. The gate
tree is `2a9588eb4eaf`. The merge is `6808930`. `scripts/backlog_check.py`,
`scripts/gen_sprint_plan.py --check` and `scripts/deviation_check.py` are
green at this open. The S20 allocation rows and the S20 plan table name the
same F-IDs. No new audit was run at this open.

F-053, F-054, F-055 and F-056 are done. F-X021 is pending.
