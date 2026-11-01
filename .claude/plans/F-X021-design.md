# F-X021, Activate the full-corpus candidate-render gate against Ocelli output

**Status**: approved
**Epic ref**: Y1.16
**Sprint**: S20
**Estimate**: 3w
**Layer**: Test
**Depends on**: F-012, F-052, both `done`

This is a Test-layer story. It is not a new renderer.

Fenced blocks below are quotations. The source punctuation is unchanged.

## Normative source, transcribed

### `docs/sprints/allocation.json`, F-X021 notes

```text
F-012 deliberately lands the candidate-directory, coverage and ledger contract before an Ocelli renderer exists. This follow-up connects every current oracle view to real Ocelli output, makes --require-comparison binding in local verification and pre-push evidence, and preserves D-04's rule that CI validates the submitted tree's local corpus record rather than pretending CI owns the ignored corpus or a GPU.
```

### `docs/lld/comparator.md`, candidate directory contract

```text
Decision D7 is that the validation oracle exists before the port code, and
F-011 did not break it. The comparator has no second side of the corpus to
compare against. A stub renderer was refused in the design
round: it would be port code written before it was designed, and its frames
would prove only that the stub and the comparator agree.
```

```text
F-012 adds a fourth operation without claiming a fourth producer. `gate`
requires both `--reference` and `--candidate`, resolves both paths, and refuses
when they name the same directory. It writes the same report as `identity` but
is the only command whose invocation is suitable for candidate evidence. A
copied reference directory is useful as controlled contract evidence. It is
not Ocelli evidence and is not made binding by this story.
```

```text
The candidate side is a directory contract and not a call into a renderer.
When the port lands, the Ocelli half writes `<id>.raw` and `<id>.json` in the
shape `docs/lld/oracle.md` already specifies, points the comparator at it with
`--candidate`, and nothing here changes.
```

The candidate side is Ocelli output. Copying the reference into the candidate directory is not that output.

### `docs/lld/comparator.md`, "What the comparator still does not build"

```text
Named, because each is somebody's story.

- Any Ocelli renderer, decoder, LUT chain or port code. Decision D7.
- A perceptual colour metric, and any CIEDE2000 implementation. Metric and
  threshold are one decision and belong together.
- Re-rendering the saturated rows at a wider window, or a magnified render for
  the two decimated rows. Both change `render-params.json` and therefore every
  reference frame.
- The Ocelli producer for all current views. F-X003 writes settled CPU frames
  for the monochrome stack rows its raster can draw and hands those directories
  to `ocelli-compare gate`. F-X021 still activates the full candidate record
  after F-052. Colour stays unscored. Under D-04, CI validates the local
  comparator record and does not render the ignored corpus.
```

### `docs/lld/comparator.md`, "Verification-ledger evidence"

```text
`scripts/verify_ledger.py record --comparison-report OUT/compare.json` accepts
only a green `gate` report with a positive judged-view count, zero absent views
and a complete record set whose paths, attribution states, sparse signed
histograms, derived statistics, predicates and aggregate hashes are internally
consistent. It records the report's exact SHA-256 digest, judged count and
verdict. The commit hook carries those values in `Ocelli-Verify` when present.
```

```text
`tools/oracle/report-contract.json` is the cross-language contract for the
closed object schemas, vocabularies, hash domains and green control report.
The Rust report test rebuilds that control through `RunReport::to_json()` and
checks its schemas and constants against production types. The Python ledger
and guard harness read the same file. A serializer change therefore cannot be
copied into a hand-built Python control while leaving validation semantics
behind.
```

```text
`assert --require-comparison` and `check-commit --require-comparison` are the
local and CI readers. F-012 deliberately leaves them dormant in the ordinary
profiles. A bounded monochrome candidate directory does not activate that
requirement. F-X021 still does, after F-052.
```

### `docs/hld/DEVIATIONS.md`, D-04 only

```text
| D-04 | §11, "Every pull request renders the corpus in CI" | CI runs no GPU build and no GPU test. The corpus renders locally, in `/verify`, and is required green before a push | Operator constraint, GPU CI minutes are expensive. See the risk below, this one is not free. | Bootstrap |
```

The claim about CI, the corpus and the GPU is the "We do" cell. CI runs no GPU build and no GPU test. The corpus renders locally, in `/verify`, and is required green before a push.

### HLD sections that do not name the producer

`docs/hld/08-validation-architecture.md` section 11 does not name a candidate directory and does not name the producer.

`docs/hld/22-testing-and-tolerance.md` section 25 and section 25.1 do not name a candidate directory and do not name the producer. Section 25.1 remains the tolerance policy. This story does not change it.

### Decision D7

From `docs/hld/11-decision-log.md`:

```text
| D7 | Validation oracle before port code | Validation as a tail phase | Makes generated Rust safe to merge at volume |
```

From `CLAUDE.md`, the same decision is "The validation oracle exists before port code."

The oracle already exists. The candidate side this story adds is Ocelli output. A copy of the reference pixels is not that output.

### Current views, from the parameter files this story does not edit

`tools/oracle/render-params.json` declares the stack pass. `base.canvas` is width 512 and height 512. `base.interpolation` is `NEAREST`. `base.camera.mode` is `reset`. `base.voi.source` is `file`. The colour rule sets `voi.source` to `none`. The file says a change here changes every reference frame.

`tools/oracle/volume-params.json` declares the volume pass. `orientations` are `AXIAL`, `SAGITTAL` and `CORONAL`. `blendMode` is `COMPOSITE`. `slabThicknessMm` is `null`. `cameraMode` is `reset`. The canvas, the background, the interpolation and the VOI policy are read from `render-params.json`, not declared a second time. The slab note says the reformat is a single-plane cut. The blend note says a maximum-intensity or average blend over a slab would fold several slices into one pixel.

Those stack frames and those three orientations are the current views. `volume__real__ct_cmb_mml` is the declared volume refusal in `tools/oracle/volume-truth.json`. It is not a rendered view.

## What the specification does not cover

The HLD does not name the producer, the candidate directory, or `--require-comparison`. The decisions below are local. They are not a new deviation.

1. The producer is a new test or tool entry. It writes a candidate directory in the existing F-012 layout. It calls Ocelli decode, the existing LUT, and the volume cut in decision 5. It does not call cornerstone. It does not copy reference pixels into the candidate frame bytes.
2. `--require-comparison` becomes required on the local sprint profile and on `check-commit` for commits that claim comparison evidence. The producer writes a real Ocelli image for every current view before that flag is bound. A missing image is absent and fails. CI still does not render. It reads the trailer. D-04 stays.
3. Section 25.1 tolerances stay as written. `tools/oracle/render-params.json` and `tools/oracle/volume-params.json` stay as written.
4. Colour views that the current Ocelli stack path cannot score stay a named gap. They are not marked `pass`. Palette and YBR stay in that same gap. This story does not choose a perceptual metric and does not move D-16.
5. The candidate applies the pinned renderer's reconstruction around the product calls. The product LUT, `StoredBits::new`, `Volume::construct` and `decode_extended` stay on their existing rules.
   - A SIGMOID file width in `(0, 1)` is presented at the recovered window and then evaluated by the unchanged C.11.2.1.3.1 function. Width 0.5 at centre 40 is width 1.5 at centre 40. The product LUT keeps the file width.
   - An irregular series `Volume::construct` refuses as non-uniform spacing is placed on the endpoint grid and then constructed. A duplicate position or a sheared stack stays refused.
   - The volume cut is the reference canvas sampled with NEAREST, because `render-params.json` sets interpolation to NEAREST. `Volume::reslice_nearest` rounds a positive half away from zero and then calls `sample_at`. The displayed byte is the thin composite of the 1024 colour table, then the quad blend.
   - Unsigned 12-bit samples of transfer syntax `1.2.840.10008.1.2.4.51` gain the jpeg.js residual of 15 after product decode. The codec return value does not.
   - A native frame whose high bit is not `BitsStored - 1` is unpacked with the conforming high bit. `StoredBits::new` still refuses the file high bit.
   - Grey `LINEAR` and `LINEAR_EXACT` bytes use one 1024-wide ramp on the CPU and the GPU. `SIGMOID` and a stops preset keep the nearest display byte.

The empty reading of this section would be false. The five choices above are the ones the HLD does not make.

## Problem and reproducer

F-012 can judge two directories and can record a green `gate` report. F-X003 can write a bounded monochrome stack directory. Neither connects every current view to Ocelli output, and neither turns `--require-comparison` on. A directory filled by copying the reference compares the reference with itself. Decision D7 says that is not the candidate side.

Trigger. Produce a candidate directory from Ocelli, then change one Ocelli constant the producer actually reads, then produce again. The candidate `sha256-rgba8-v1` digest changes. The reference digest does not. Replace the producer with a byte copy of the reference frame and repeat the same constant change. The candidate digest stays put. That second result is a failed test, not a pass.

Current behavior. No entry writes that full candidate directory. `assert --require-comparison` and `check-commit --require-comparison` stay dormant. Colour stays unscored. CI runs `python3 scripts/verify_ledger.py check-commit HEAD` and does not render.

Source identity. Decode, `LutChain::map_into`, and the volume cut in decision 5. Not a call into cornerstone. Not the reference `.raw` bytes.

## Scope, API and compatibility

In scope.

- One new entry that writes `<id>.raw`, `<id>.json` and `run.json` in the layout `docs/lld/oracle.md` already specifies.
- Stack views use the reference sidecar's canvas, which is the 512 square from `render-params.json`, with `NEAREST`, camera `reset`, and VOI taken from the file. Colour rules keep VOI source `none` and are not scored.
- Volume views use `Volume::construct` and the NEAREST canvas cut in decision 5 for `AXIAL`, `SAGITTAL` and `CORONAL`. Slab thickness stays null, so the cut is one plane. Blend stays `COMPOSITE` only as that single-plane cut. The entry does not implement MIP, MinIP or average.
- The existing `ocelli-compare gate` judges the directory. The entry does not grow a second comparator.
- The mutation test and the missing-view test below.
- Local sprint recording of a comparison report, after every current view has a real Ocelli image.

Out of scope.

- A new renderer, a new viewport, and a new raster.
- Any edit to section 25.1, `render-params.json`, `volume-params.json`, or `docs/hld/DEVIATIONS.md`.
- Calling cornerstone, or copying reference pixels into the candidate `.raw`.
- Marking a colour, palette or YBR view `pass`.
- Oblique reslice, thick slab, fusion, transfer functions, lighting and cropping. Those are F-053, F-054, F-056, F-060, F-061 and F-062.
- CPU MPR and the CPU volume decision. Those are F-X004 and F-X005.
- A GPU job in CI. A wasm-bindgen export. `unsafe`.

Public entry. A subcommand of the existing comparator binary, reached as `bin/ocelli.sh compare produce`, with `--reference`, `--candidate` and `--corpus`. Corpus-scale work stays a subcommand. An ignored test that needs `tools/oracle/out/` would read as a pass on a day it did not run, which this repository already refuses. The subcommand writes the candidate directory and does not itself record a ledger pass. Judgement stays `bin/ocelli.sh compare gate --reference REF --candidate CANDIDATE --out OUT`.

The F-012 refusal stands. The two resolved directories must differ. A missing candidate path is not filled from the reference.

Compatibility. `identity` and `mutations` stay. `produce_applicable` in `tools/oracle/src/produce.rs` stays the bounded monochrome stack producer. This story does not widen that function into a full-corpus pass. `python3 scripts/verify_ledger.py check-commit HEAD` stays the CI command. It gains no GPU step.

Refusal. The producer refuses a candidate path that resolves to the reference run. It refuses to publish a candidate `.raw` it did not build. A view it cannot score is a named gap. It is not written as `pass`. If a view would require a GPU ortho upload, tier C reports that view unavailable and writes no substitute image.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `compare produce` | Ocelli decode registry, `LutChain::map_into`, `Volume::construct`, `Volume::reslice_nearest`, existing `CpuRaster` for the stack canvas | Native tool. No cornerstone. Parameters come from the reference sidecar and the unchanged param files. Decision 5 names the reconstructions around those calls. | Candidate `<id>.raw`, `<id>.json`, `run.json` |
| `ocelli-compare gate` | F-012 directory contract in `tools/oracle/src/bin/ocelli-compare.rs` | Two distinct directories. Tolerances in `tools/oracle/src/tolerance.rs` unchanged. | `compare.json` with judged, unmeasured, absent, unsupported source rows and declared volume refusals as separate counts |
| Local sprint `/verify` | `scripts/verify_ledger.py record --comparison-report` then `assert --require-comparison` | Sprint profile, after every current view has a candidate image. | Ledger `comparison` object, or a refusal |
| `.githooks/pre-push` | `scripts/verify_ledger.py check-commit` | Local head. Reads the trailer. | Push allowed only when the trailer matches the tree |
| CI evidence job | `python3 scripts/verify_ledger.py check-commit HEAD` in `.github/workflows/ci.yml` | No corpus checkout as a renderer input. No GPU. | Pass or fail from the trailer alone |
| Colour, palette and YBR stack views | No scoring provider | Named gap. Current stack path cannot score them. | Not a `pass` record |
| `produce_applicable` | Existing bounded monochrome path | Unchanged. A subset is not the full record. | Unchanged subset directories |
| `identity` and `mutations` | Existing comparator self-tests | Unchanged. Neither is Ocelli evidence. | Unchanged |

The product LUT stays the only VOI formula. Decision 5 names the pinned-renderer reconstructions around it. Grey LINEAR presentation is one ramp on both tiers. Selection of a current view is the reference run's declared frame list. Conversion to RGBA8 for the candidate `.raw` reuses the existing stack presentation path. State transition of the ledger is the comparison object written only after `comparison_evidence` accepts the report.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No candidate directory | `compare gate` | Nonzero refusal. No green report. | Reference directory unchanged |
| Candidate run omits one reference view id | `compare gate` | That id is `absent`. The run is not `pass`. | No absent view is counted as judged |
| Candidate `run.json` names an id whose `.raw` is missing | `Run::load` | `DeclaredFileMissing`. Structural refusal. Not `pass`. | No partial report is recorded |
| Producer copies reference bytes | Change one Ocelli constant and produce again | Candidate `sha256-rgba8-v1` digest does not move | The mutation test is red |
| Producer reads that constant | Same constant change | Candidate digest moves. Reference digest does not. | Reference `.raw` bytes stay the reference |
| Colour, palette or YBR view | Producer classification | Named gap. Disposition is not `pass`. | The view is not given a passing predicate |
| View needs a GPU ortho upload on tier C | Produce on tier C | View reported unavailable. No image written for it. | No CPU image is substituted |
| Sprint tree with no acceptable comparison report | `assert --require-comparison` if bound | Nonzero. Trailer gains no comparison claim. | Earlier receipt for that tree is not reused after `invalidate` |
| Pushed head | CI `check-commit HEAD` | Trailer checked. No render. | D-04 unchanged |

Invariants that span the producer, the gate and the ledger.

- Candidate frame bytes are a function of Ocelli decode, the LUT and decision 5, not of the reference frame bytes.
- The candidate `run.json` carries the same `*Sha256` input digests as the reference. A mismatch is a refusal before pixels, which is the existing contract.
- A ledger-accepted comparison report has a positive judged-view count, zero absent views, and a complete record set.
- A missing view is not a pass. A colour view is not a pass.
- Section 25.1 and the two parameter files are not an input this story may rewrite.
- CI does not own the ignored corpus and does not own a GPU.

This section is applicable because the entry writes a directory and the ledger publishes a claim about that directory. It is not a pure function of one call.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Candidate directory | `compare produce` | Resolved path, required to differ from the reference path | `run.json` exists only after every declared `.raw` and sidecar for that run is written | A failed write removes the partial files. A second run replaces the directory. Equal arguments do not prove the bytes were not rewritten. |
| Candidate `.raw` | Ocelli decode, LUT and decision 5 | View id plus `sha256-rgba8-v1` digest | Bytes on disk whose digest the sidecar `frame.sha256` names | A copy of the reference raw is not a commit of Ocelli output. The mutation test rejects it. |
| `compare.json` | `ocelli-compare gate` | Output directory of that invocation | Report whose `operation` is `gate` and whose verdict is green | A red, empty, or absent-bearing report is not recorded. Identity output stays unacceptable. |
| Ledger `comparison` object | `verify_ledger.py record --comparison-report` | `git write-tree` of the index | Object with `reportSha256`, `claimedVerdictViews` greater than zero, and `verdict` `pass` | `invalidate` drops that tree's receipt before a rerun. A stale tree does not satisfy a new index. |
| `Ocelli-Verify` trailer | `.githooks/commit-msg` from the ledger | Commit message on that tree | Trailer fields emitted by `trailer`, never typed by hand | CI re-reads the trailer. A trailer for another tree fails. |
| Reference `tools/oracle/out/` | Existing oracle gate, not this producer | Reference run | Unchanged by the producer | The producer must not delete or overwrite it. |

Side effects before the comparison commit are the candidate files and nothing in the reference tree. No browser setter and no device setter is required on the planned path. A refused produce must not leave a candidate `run.json` that the gate could read as complete. A valid produce, a refused second request, and a valid retry must leave the last complete candidate directory in place until the retry's own files are complete.

Validity of a recorded comparison is the report digest plus the tree hash. A change to source, to the corpus bytes, or to the reference run invalidates it. Format is RGBA8. Layout is the F-012 sidecar. Rendering mode is the unchanged stack and volume parameters. The ledger checks that record before it accepts an idle receipt. A refused comparison must not replace a prior receipt for a different tree, and must not keep a receipt for this tree after `invalidate`.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Produce from Ocelli. Change one constant the producer reads. Produce again. The candidate `sha256-rgba8-v1` digest changes. The reference digest does not. A writer that copies the reference `.raw` fails this test because its digest does not move. | Mutation test beside the producer. The constant is an Ocelli input, not a section 25.1 tolerance. | Copying the reference into the candidate directory. Hashing the reference and calling it the candidate. | Implementation. Oracle test. |
| AC2 | A current view id present on the reference and missing from the candidate is `absent`. The gate is not `pass`. A declared id whose `.raw` is missing is `DeclaredFileMissing` and is also not `pass`. | Existing `ocelli-compare` coverage test, pointed at a directory this entry wrote and then deleted one view from. | Omitting the file and still reporting `pass`, `unmeasured`, or zero absent. | Implementation. Comparator. |
| AC3 | The CI evidence command does not invoke a GPU, the oracle renderer, or the corpus. It reads the trailer. | `.github/workflows/ci.yml` evidence step remains `python3 scripts/verify_ledger.py check-commit HEAD`. | A GPU job, a corpus render, or a step that calls `bin/ocelli.sh oracle` or `compare produce` in CI. | External. CI workflow text. No equipment beyond the workflow file. |
| AC4 | No `wasm-bindgen` outside `ocelli-wasm`. This story adds none. | Existing wasm-bindgen gate over the diff. | A bindgen import in the producer or the oracle crate. | Implementation. Pins gate. |
| AC5 | No `unsafe` outside the two permitted files. This story adds none. | Existing `unsafe` gate over the diff. | An `unsafe` block in the producer. | Implementation. `unsafe` gate. |
| AC6 | The producer runs on `CpuRaster` and `Volume::reslice_nearest`. The gate process adds no raster. Grey LINEAR uses one 1024 ramp on the CPU and the GPU. Tier C for the gate process is n/a. If a view needs a GPU ortho upload, tier C reports unavailable and writes no other image. | Producer execution record names the entry. A forced GPU-upload path on tier C has no `.raw`. | A silent stack image whose CPU byte differs from the GPU byte. A new raster inside the ledger or `check-commit`. | Implementation for the producer. The gate process row is the absence of a raster in `verify_ledger.py` and `ci.yml`. |
| AC7 | A colour, palette or YBR view the current stack path cannot score stays a named gap and is not `pass`. | Producer disposition list contains the view with a not-scored reason. No record for it has outcome `pass`. | Marking the gap `pass`, or copying the reference colour frame and scoring it. | Implementation. Producer dispositions. |
| AC8 | `--require-comparison` on the local sprint profile and on `check-commit` for a commit that claims comparison evidence. The candidate directory contains every current view. | `verify_ledger.py assert --require-comparison` on that report, then the commit trailer. | Binding the flag by pointing it at a reference copy, or at a subset that drops views and still says the full record passed. | External local verification. Owner is the sprint profile. |

AC1 is the production-stage control for pixel identity. A synthetic comparator call does not replace it. The full current-view run is `bin/ocelli.sh oracle` for the reference, then `bin/ocelli.sh compare produce` for the candidate, then `bin/ocelli.sh compare gate`. The reference pin stays cornerstone3D 5.8.2 under the existing oracle. The candidate entry point is the new subcommand. The host is the local machine that already runs `/verify`, because D-04 keeps the corpus and the GPU off CI. A missing view cannot be recorded as a pass to make the run green. Colour images are emitted and stay unscored. They are not `pass` and they are not absent.

AC3's procedure is to read the evidence job. The command starts at checkout and ends at `check-commit`. No adapter is opened. No duration threshold applies.

## Approach

The smallest change that matches the transcribed contract.

1. Add `compare produce` next to the existing gate command in `tools/oracle/src/bin/ocelli-compare.rs` and `bin/ocelli.sh`. The function lives beside `produce_applicable` and does not replace it.
2. For each rendered stack row the current path can score, reuse the existing decode, `LutChain::map_into`, and `CpuRaster` path in `tools/oracle/src/produce.rs`. The canvas, interpolation, camera and VOI come from the reference sidecar, which was produced under `render-params.json`. Decision 5 names the window and unpack adjustments on that path.
3. For each rendered volume reformat, build the volume with `Volume::construct` where it accepts the series, or the endpoint grid in decision 5 where construct reports non-uniform spacing. Cut the reference canvas with `Volume::reslice_nearest`. Present that plane with the thin composite in decision 5 so the candidate width and height match the reference. A size mismatch is already a frame-size refusal. Do not add a slab blend. Null slab is one plane.
4. Write candidate bytes only from that path. Reuse the F-012 `run.json` shape, including equal `*Sha256` keys. Do not copy the reference `.raw` into the candidate `.raw`.
5. Leave colour, palette and YBR as named not-scored dispositions. Do not emit `pass` for them.
6. Keep `volume__real__ct_cmb_mml` as the declared refusal the reference already records. Do not invent frames for it. The refusal count must match or the gate reports a coverage problem.
7. Prove AC1 and AC2 in tests. Point `compare gate` at the produced directory for the views the entry wrote.
8. Bind `--require-comparison` on the local sprint profile and on `check-commit` only after the producer has written a real image for every current view. Do not add a GPU step to `.github/workflows/ci.yml`.

Rejected.

- A stub or a reference copy called Ocelli output.
- Calling cornerstone from the producer.
- Editing tolerances or the two parameter files so the images agree.
- Marking an unscored colour view `pass`.
- Treating `produce_applicable`'s monochrome subset as the full candidate record.
- A new trait, a new generic, or a new renderer crate. One producer entry calls the concrete functions that already exist.
- Binding `--require-comparison` by dropping views until the absent count is zero and calling that the full corpus.

## Dependencies, write ownership and resources

Declared dependencies are F-012 and F-052. Both are `done`. Source dependencies are the decode registry, `ocelli-pixel` LUT, `ocelli-volume` construct and reslice, and the F-X003 stack presentation path. No story in S20 is a predecessor in the backlog cell.

Safe order. Tests for the copy mutation and the absent view can land with the entry. A sprint recording that passes `--require-comparison` waits until every current view has a candidate image. CI stays on the trailer the whole time.

Files this story would write, and that another S20 story must not write at the same time.

- `tools/oracle/src/bin/ocelli-compare.rs`
- `tools/oracle/src/produce.rs`
- `bin/ocelli.sh` compare arm
- tests under `tools/oracle` for AC1 and AC2
- `.claude/commands/verify.md` and `.githooks/pre-push`, because the flag is bound
- `docs/lld/comparator.md` and `docs/lld/oracle.md` at completion, not in the design commit

Not written. `docs/hld/DEVIATIONS.md`, `tools/oracle/render-params.json`, `tools/oracle/volume-params.json`, `tools/oracle/src/tolerance.rs`, `.github/workflows/ci.yml` except to keep the evidence command free of a GPU step. No shader. No GPU exclusivity on the planned path. The ignored corpus and `tools/oracle/out/` are inputs. The producer does not take ownership of the reference directory.

## Verification and evidence reuse

Focused checks, before any sprint claim.

- The AC1 mutation test and the AC2 absent-view test.
- `bin/ocelli.sh check` on the oracle crate while iterating.
- Existing `unsafe` and wasm-bindgen gates on the diff.
- `python3 scripts/prose_check.py` and `python3 scripts/deviation_check.py` once this plan is tracked.

The CI observation is the workflow text. `python3 scripts/verify_ledger.py check-commit HEAD` reads `Ocelli-Verify`. It does not run `bin/ocelli.sh oracle`, `compare produce`, or a GPU test.

Final local profile is sprint verification. The comparison input is the reference run from `bin/ocelli.sh oracle` and the candidate run from `compare produce`. The observer is `compare gate` and then `verify_ledger.py record --comparison-report`. A report with any absent view is rejected by the ledger and cannot be the recorded evidence. Colour gaps are not converted into passes to make that report green.

Evidence is invalid when the index tree changes, when the reference run changes, when a candidate `.raw` matches the reference bytes under the AC1 mutation, or when the report digest no longer matches the trailer. Completion metadata does not make a missing view into a pass. No tolerance baseline moves. No performance threshold is claimed.

What this story can show is that the candidate bytes depend on Ocelli and that a missing file is not a pass. What it cannot show, until a colour metric exists, is a `pass` on colour or ultrasound. D-16 already says those views are `unmeasured` when they are scored. This story does not score them.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The producer is not the frame loop. It reuses buffers the existing `CpuRaster` and `LutChain::map_into` already size when they run.
- unsafe: none
- Tier A (WebGPU): n/a for the gate process. The planned producer does not open a device. It runs the existing LUT and CPU ortho path. A GPU ortho upload is not part of this plan. If one becomes necessary, that view is unavailable where the upload cannot run, and no other image is written.
- Tier B (WebGL2): n/a for the gate process. No new WebGL path. The same rule as tier A if a GPU upload is required.
- Tier C (CPU): n/a for the gate process itself, because no new raster is introduced. The producer runs on `Volume::reslice_nearest` and `LutChain::map_into`, plus the existing `CpuRaster` already used for monochrome stack frames. Grey LINEAR matches the GPU ramp. If the producer needs the GPU ortho upload, tier C reports that view unavailable rather than writing a different image.

D-07 is the existing third tier. This story does not extend it and does not draw a second image under a tier check.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| mutation | A reference-byte copy does not move its digest when one Ocelli constant changes. A real producer digest does move. The reference digest does not. | New test next to the producer. Uses the `sha256-rgba8-v1` contract already in `tools/oracle/src/render_hash.rs`. |
| unit | A view id on one side only is `absent` and the run is not `pass`. A declared missing `.raw` is a structural refusal and is not `pass`. | `tools/oracle` coverage test over a directory the entry wrote. |
| unit | Colour, palette and YBR dispositions are not outcome `pass`. | Producer disposition test. |
| fixture | The product LUT fixtures stay on PS3.3 C.11. Decision 5 names the candidate reconstructions. `reslice_nearest` is covered beside the ortho tests. | `crates/ocelli-pixel` and `crates/ocelli-volume/tests/oblique.rs`. |
| conformance | The CI command text does not render. | Review of `.github/workflows/ci.yml` against AC3. |
| property | Binding the flag on a report with an absent view cannot record a green comparison. | Existing ledger probe `comparison report has absent views`, reused. Not a new tolerance. |

HLD 27.2 R3 is met by refusing a new arithmetic function. The mutation test rejects a copy. It does not replace the existing hand-computed LUT fixtures.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column. Its rows are viewport types, tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation representations, core events and adapters. None of those rows is keyed to Y1.16.

This story does not add a viewport, a tool, a blend mode, a LUT function, a transfer syntax, a segmentation representation, an event or an adapter. `COMPOSITE` with null slab is the current volume view's declared single-plane cut, consumed through `Volume::reslice_nearest`, not a new blend implementation.

Observations this story can establish are only AC1 through AC7. A parity pass on colour, ultrasound, oblique, thick-slab MIP, MinIP, average, or fusion stays unavailable. Those gaps are F-X021's named colour gap and the other S20 and S21 stories. A missing observation is not a passed row.

## Deviations

None new. This draft does not edit `docs/hld/DEVIATIONS.md`.

Existing citations. D-04 is the rule that CI validates the local record and does not render. D-07 is the CPU tier. Decision D7, with no hyphen, is the oracle before port code. D-16 stays the reason a scored colour view is not `pass`, and this story does not score colour.

## LLD impact

`/complete-feature` step 9 updates these files after implementation, not in this draft.

- `docs/lld/comparator.md`, the two sections quoted above, to say whether the producer exists and whether `--require-comparison` is still dormant.
- `docs/lld/oracle.md`, the paragraph that still assigns full viewer activation to F-X021.
- `docs/lld/codecs.md`, the JPEG-LS section, because the adapter decodes the lossless RGB corpus row.

No HLD edit. No deviation edit.

## Open questions

None. The consolidated design round was offered on 2026-10-31 and declined. The producer emits a real Ocelli image for every current stack view and every current volume orientation before `--require-comparison` is bound. An already declared unsupported row stays unsupported. A missing image is absent and fails. Copying the reference is not a candidate. CI still does not render. D-04 stays.

Files that decide unsupported versus absent, recorded so the producer does not confuse them:

Reference stack rows the reference cannot render are unsupported source rows, not absent views.

- `tools/oracle/unsupported.json` names them.
- `tools/oracle/src/unsupported.mjs` matches a failure to an entry and rejects both an unnamed failure and a stale excuse.
- `tools/oracle/run.mjs` writes `rows[]` with `ok` false only for a matched entry, and checks `readBack + unsupported == applicable`.
- `tools/oracle/src/sidecar.rs`, `Run::load`, skips `ok` false when it builds the view set. `Run::unsupported_source_rows` counts those rows.
- `tools/oracle/src/bin/ocelli-compare.rs` compares the two runs' unsupported counts and stores `unsupportedSourceRows` apart from `absent`.

Declared volume refusals are a second count, not absent views.

- `tools/oracle/volume-truth.json` field `expectedRefusal` on `volume__real__ct_cmb_mml`.
- `tools/oracle/src/volume.mjs` validates that field and calls it the volume pass's unsupported list.
- `tools/oracle/run.mjs` writes `outcome` `refused-as-declared` and an empty frame list when the refusal matches.
- `tools/oracle/src/sidecar.rs`, `Run::declared_volume_refusals`, counts that outcome. An empty frame list adds no view id.
- `tools/oracle/src/bin/ocelli-compare.rs` compares the two refusal counts as `declaredVolumeRefusals`.

Absent is an identifier in one loaded view set and not the other.

- `tools/oracle/src/sidecar.rs`, `Run::load`, builds the view set from `rows[]` with `ok` true and from `volumes[].frames[]`. A declared id whose `.raw` or sidecar is missing is `LoadError::DeclaredFileMissing`, a structural refusal, not an absent outcome.
- `tools/oracle/src/bin/ocelli-compare.rs` sets `absent_views` from the symmetric difference of those ids and records a coverage problem. The text says an identifier on one side only is `absent` and a run failure, never a skip.
- `tools/oracle/src/report.rs` folds that count into `absent_count`. A green run requires zero absent views.
- `scripts/verify_ledger.py` exits `comparison report has absent views` when the coverage absent count is not zero. That is the refusal that makes a bound `--require-comparison` fail while any current view is missing from the candidate set.

A third vocabulary is not either of those decisions.

- `tools/oracle/src/produce.rs`, `stack_selection_reason` and `classify_volumes`, record a local disposition `unsupported` for colour, palette, YBR and volume reformats. That disposition is not `tools/oracle/unsupported.json` and it does not by itself change the comparator's absent count. Omitting those ids from a candidate that is compared with the full reference run makes them absent. Omitting them from both a subset reference and a subset candidate is the bounded directory F-012 refused to treat as the full record.

`tools/oracle/compare-expectations.json` is the `unmeasured` census. It does not decide unsupported versus absent.
