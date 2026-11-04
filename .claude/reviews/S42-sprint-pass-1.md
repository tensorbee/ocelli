# S42 review, pass 1

**Reviewed**: `sprint/s42` at `fb1381d` against merge base `bf3c9f3616bbd18eceecb9b916e24a0eb3db0284`, read from `/tmp/ocelli-s42-sprint.diff`. Current source was opened for `crates/ocelli-dicom/src/{lib.rs,series_open.rs}`, `crates/ocelli-render/src/{cpu.rs,graph.rs,ortho.rs}`, `crates/ocelli-pixel/src/color.rs`, `docs/lld/feature-availability.md`, `docs/sprints/{SPRINT_PLAN.md,BACKLOG.md,allocation.json}`, `README.md`, `docs/DEVELOPER_SETUP.md` and `bin/example-dev.sh`. No unstaged paths were in scope.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 2 defects, 3 smells, 4 nitpicks

## Acceptance challenged

- **The shared metadata projection across F-191, F-192 and F-193.** `project_metadata` was written for one F-191 frame. F-192 calls it once per expanded frame and keeps each copy. F-193 calls it alongside `declared_frame_count`, which projects a second time. I traced the `/series-open` route from `frames_of` (`examples/example-server/src/series.rs:92`) into `prepare` (`series_open.rs:391`). That trace is S1.
- **The F-194 raster mode seen by consumers F-194 did not edit.** `present_cpu_plane` (F-X004) and the wasm session colour draw both run through `CpuRaster`. I checked what `Interpolation::Linear` and the new `reserve_source` plane do to each of them. That is S2 and S3.
- **The shader linear branch against the CPU blend on edge texels.** I worked through mapped `x` in `[0, 0.5)` and in `(dims.x - 0.5, dims.x)`. Both paths refuse and draw opaque black: the shader through `sx < 0.0` and the footprint test, and the CPU through `floor_fraction` and then `OPAQUE_BLACK`. They agree.
- **Camera lane persistence.** Every `write_buffer(&slot.camera, ...)` site goes through `device_camera(..., state.interpolation)`. Those sites are `graph.rs:1926`, `:2424` and `:2475`. Rebind and DPR changes keep the selected mode. Only `device_camera_words` hard-codes nearest, and pass 1 already left that as a nitpick.
- **Ledger claims on M19.** I checked the milestone text against the story rows, the showcase decision in `CURRENT_SPRINT.md`, and the `phase` field in `allocation.json`. That is D1 and D2.
- **Test counts in AS_BUILT** against the `#[test]` counts in the current files.
- **README library snippet** against the constructors that `frame_open.rs` and `reply.rs` call.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read the sprint diff: designs, last review passes, all Rust, server, `App.tsx`, ledgers | `/tmp/ocelli-s42-sprint.diff` | executed now, read only | findings below |
| Grep `#[test]` in `crates/ocelli-dicom/tests` and `examples/example-server/src` | HEAD working tree | executed now | frame_open 23, series_open 34, overlay 25, server 16. These match AS_BUILT |
| Grep `device_camera(` and the camera `write_buffer` sites | `crates/ocelli-render/src/graph.rs` | executed now | three sites, all carry the stored mode |
| Grep `present_cpu_plane` and `raster_into` | `crates/ocelli-render/src/ortho.rs:100-116` | executed now | takes the caller's `CpuRaster` as it stands, mode included |
| Grep `"phase"` for F-191 to F-194 | `docs/sprints/allocation.json:2397-2464` | executed now | `"phase": "P1"` on all four |
| Grep `folder viewer calling` | `docs/` | executed now | `BACKLOG.md:325`, `SPRINT_PLAN.md:612`, `allocation.json:119` |
| `gate --sprint` ALL GREEN, 31 gates | host | prior receipt | not re-run |
| `compare produce`: 102 written, 0 omitted, 2 unsupported | host | prior receipt | not re-run |
| `compare gate`: 102 views, 71 pass, 0 fail, 31 unmeasured, verdict pass | host | prior receipt | not re-run |

## Defects

### D1, the M19 milestone text says the folder viewer calls the core

**Where**: `docs/sprints/allocation.json:119`, which appears to be the source of the two copies. Also `docs/sprints/BACKLOG.md:325` and `docs/sprints/SPRINT_PLAN.md:612`.
**What**: The M19 goal reads "The series-open gaps that Phase 1 left in the core, then the folder viewer calling that core."
**Why it is wrong**: Every viewer story in M19 names the example viewer, not the folder viewer. F-198 is "Example viewer renders through @ocelli/core and ocelli-wasm", and F-199 to F-201 are the viewer stories that depend on it. `CURRENT_SPRINT.md` and the F-191 design both record the operator decision that `examples/viewer-react` is the showcase and that `examples/folder-viewer` "is not the showcase and is not changed". A later sprint that plans from this milestone sentence would aim at the wrong page.
**Evidence**: the grep above, set against the BACKLOG rows F-198 to F-201 at `BACKLOG.md:161-164` of the diff.
**Scope and regression**: present on HEAD in all three files. The fix is one sentence in `allocation.json`, regenerated into the two markdown files. It names the example viewer.

### D2, "Phase 1 is S01 to S41" contradicts the P1 rows now scheduled in S42 to S44

**Where**: `docs/sprints/SPRINT_PLAN.md:7`, with the new clause at about `:10`.
**What**: Line 7 still says Phase 1 is S01 to S41, and line 8 now says Phase 1.5 is S45 to S75. The edited paragraph then says F-191 to F-201 "land in Phase 1", and `allocation.json` gives them `"phase": "P1"` in S42 to S44. S42 to S44 are therefore P1 sprints that fall outside the stated Phase 1 range. The text now names no phase for them and places their stories in a phase whose stated range excludes them.
**Why it is wrong**: under the microscope severity table, a false prose claim is a defect. Any count or tool that maps sprint numbers to phase by the stated ranges will drop eleven P1 stories.
**Evidence**: the allocation grep above, and `SPRINT_PLAN.md:7-8`.
**Scope and regression**: present on HEAD. The fix is either to state that Phase 1 now runs S01 to S44 and keep the 118 and 397 imported-row totals as they are, or to name M19 as its own range.

## Smells

### S1, series assembly projects and keeps a whole-object metadata copy for every frame

**Where**: `crates/ocelli-dicom/src/series_open.rs:391` (`prepare` calls `project` for each `ParsedFrame`), `:347` (`Prepared.metadata` keeps that copy), and `:679` (only the first frame's copy is ever read). The caller is `examples/example-server/src/series.rs:92`, which expands every declared frame of every part.
**Mechanism**: an N-frame enhanced object posted to `/series-open` is projected N + 1 times. One projection is in `declared_frame_count` and the other N are in `prepare`. All N copies stay alive in `prepared` until grouping finishes. Each copy holds the whole Per-frame Functional Groups Sequence and every non-pixel bulk element, because `project_metadata` skips only `(7FE0,0008/0009/0010)`. That includes Overlay Data `(60xx,3000)` and private OB blobs. Time and memory both grow as N squared. A 1,000-frame enhanced CT or MR, well inside the 1 GiB body cap, holds about a thousand copies of its own functional-group tree. F-191 and F-193 each project one object once, so no single-story pass would have seen this. It only appears when F-192 reuses F-191's projection per frame.
**Smallest regression**: project once per distinct `ParsedDicom` and share it across that object's frames. Keep only what the first displayed frame needs for cine, or project only that object at `finish`. A test can count projections for a two-frame object, or a bench can assert linear growth.
**Uncertainty**: not executed. The size estimate is reasoning from the code, not a measurement.

### S2, every colour reserve now allocates a linear display plane that nearest never reads

**Where**: `crates/ocelli-render/src/cpu.rs:309`, in `reserve_source`.
**Mechanism**: `reserve_source` now always allocates `display_scratch` at 3 × pixels `f32`, beside `rgb_scratch` at one `Rgb` of three `f32` per pixel (`ocelli-pixel/src/color.rs:32`). Tier C colour scratch is therefore 24 bytes per pixel where it used to be 12, whatever the mode. A 2048 by 2048 colour frame adds about 48 MB. The wasm session reserves on every colour draw and never calls `set_interpolation`, which pass 5 confirmed. Wasm linear memory does not shrink, so tier C colour sessions pay the doubled footprint for a mode nothing can select today. Pass 5 checked the count of 2 against the reserve contract, but not the size or whether the plane is needed at all.
**Smallest regression**: grow `display_scratch` only when the mode is linear. That growth can happen in `set_interpolation` or in a reserve made after linear is selected, so the draw still does not allocate. A test can then assert that a nearest colour reserve leaves the display plane empty.

### S3, the raster and stack-pass mode reaches MPR presentation, which the design kept out of scope

**Where**: `crates/ocelli-render/src/ortho.rs:100-116` (`present_cpu_plane` takes the caller's `CpuRaster` unchanged) and `docs/lld/feature-availability.md:35`. The design is `.claude/plans/F-194-design.md`, which places "Volume raycast sampling. Existing MPR and raycast stay on their current filters" out of scope.
**Mechanism**: no F-194 code stops linear mode reaching MPR. On tier C, a raster set to linear makes `present_cpu_plane` blend the resliced samples a second time, after the reslicer's own interpolant. That falsifies the LLD row "There is no second interpolant" for CPU MPR. The same raster also returns `RasterError::Extent` from `present_cpu_plane` until someone calls `reserve_source`, and no MPR caller does. On tiers A and B, `ortho.rs:3` says the plane goes through the stack pass, so `set_stack_interpolation` on that viewport makes MPR linear there too. Nothing calls `set_interpolation` or `set_stack_interpolation` in production yet. That is why this is a smell and not a defect.
**Smallest regression**: decide whether MPR inherits the stack mode. If it does not, `present_cpu_plane` refuses or forces nearest on a raster whose mode is linear, and a test pins it. If it does, update the design scope sentence and `feature-availability.md:35`, and give MPR its own reserve.

## Nitpicks

1. `docs/DEVELOPER_SETUP.md:80` calls the helper the "frame-open helper" and says only the Frame open section uses it. `README.md:105` lists frame-open, series-open and overlay but not interpolation. `bin/example-dev.sh:2` says "Start the frame-open helper". After F-192 to F-194 the helper serves four panels.
2. `crates/ocelli-dicom/src/lib.rs:5-10`. The crate doc lists F-192 and F-193 but has no F-191 line, although `frame_open` is the module the other two build on.
3. `docs/lld/feature-availability.md` has no row for F-194's `available(mode, tier)` table, even though that file is "the contract every tier-gated feature declares". The approved design limited LLD impact to `render-graph.md`, so this is a preference.
4. `examples/example-server/src/overlay.rs`. `mask_list` returns overlay bits for a posted file whenever the image has at most 256 pixels. The design described the mask as the synthetic preview. Overlay bits are not Pixel Data, so no boundary rule is broken.

## Verified clean

- **One LUT chain.** `frame_open.rs` builds inputs through `ModalityTransform::new` and `VoiTransform::new` and never maps. Padding black comes only from `chain.inverts()` and `chain.voi().output_range()`. The CPU linear path calls `chain.apply` once per stored sample before blending. The shader linear branch calls `lut_chain` on each of the four texels before blending. Palette linear blends table colours on both sides. Nothing in `series_open.rs` or `overlay.rs` calls `LutChain`.
- **CPU and shader agreement.** The half-texel shift, the negative-coordinate refusal, the zero-fraction skip (including `-0.0`), the footprint refusal and the four weights match term for term at `graph.rs` and `interpolation.rs`. The nearest path still uses `textureLoad(stored_tex, texel, 0).r` and `texel_of(index + INDEX_TO_IMAGE_CENTRE)`.
- **Approved F-194 values.** Nearest 0 and linear 138.75 at `(0.5, 0.5)` are untouched in the fixture, the helper and the page, as the brief instructs.
- **Server routing across four stories.** `open_path`, `body_messages` and the Vite proxy agree on `/frame-open`, `/series-open`, `/overlay` and `/interpolate`. The 1 GiB cap is checked before any body read on every route. OPTIONS returns 204 on all four. Error JSON carries a code, an index or a tag, and never a UID, path or filename. Multipart part headers are discarded.
- **Page against JSON, field by field, for all four panels.** Each panel keeps its last success and adds the error sentence. Sweep is labelled Bounce. The neutral value prints as absent rather than 0. The interpolation sentence switches with the control and never shows the other mode's number.
- **AS_BUILT test counts** for F-191, F-192, F-193 and F-194 match the files.
- **README library snippet.** The constructor shapes match `reply.rs` and `frame_open.rs`.
- **Boundaries.** There is no `wasm-bindgen` outside `ocelli-wasm` and no new `unsafe`. The example server is native-only behind `cfg(not(target_arch = "wasm32"))`, and the measured wasm module gains neither `ocelli-dicom` nor `ocelli-codec`.
- **Prose rules.** I found no em dash and no prose semicolon in the S42 prose I read.

## Limitations and handoff

- I ran no test, no gate, no mutation and no shader compiler. The 31-gate sprint receipt and the compare receipts belong to the host. S1 and S2 are reasoned from the code and not measured. S3 has no production trigger today.
- The F-193 multi-frame rule, where an overlay without the Multi-frame Overlay module applies to frame 0 only, was accepted by the story review. I did not reopen it.
- I did not read the regenerated `guard-verification.md` renumbering or the full `catalogue.py` addition beyond the example-dev probe header.
- Next: the host remediates D1, D2 and S1 to S3, then a fresh pass 2 reviews the changed material. This pass is a bounded review and not sprint closure.

S42, pass 1: 2 defects, 3 smells, 4 nitpicks.
