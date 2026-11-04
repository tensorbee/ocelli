# F-198, example viewer through the wasm core

**Status**: approved
**Epic ref**: E38.8
**Sprint**: S43
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/03-architecture-and-crates.md` places DICOMweb fetch on the TypeScript shell and decode plus rendering in the Rust worker. Pixels do not cross that boundary.

`docs/hld/07-concurrency-and-typescript.md` names one wasm instance per worker. `docs/hld/10-extension-points.md` names `SeriesSource` as the place bytes come from, with DICOMweb as the Phase 1 source.

`coreAvailable()` in `packages/core/src/index.ts` returns false until a built module is loaded. `crates/ocelli-wasm/src/canvas.rs` already binds an `OffscreenCanvas`, draws the committed frame inside the worker, and publishes with `putImageData`. It exposes no pixel getter.

`docs/lld/dicomweb.md` gives QIDO-RS, WADO-RS and WADO-URI to `@ocelli/core`, and Part 10 parsing to `ocelli-dicom`. The live `commit_dicomweb_response` wasm entry is still an open gap. This story does not invent that entry. The shell fetches one instance and passes the Part 10 bytes through the session bulk slot that already exists.

## What the specification does not cover

The HLD does not name the example page's control layout. This plan adds one render section: a worker, one offscreen canvas, the resolved tier string, and a DICOMweb base URL plus a study, series and instance UID. An empty URL uses the example server's existing synthetic Part 10 route when that route is present. No patient study is fetched.

## Problem and reproducer

The example page shows S42 results from the example server. `coreAvailable()` is false, so the page never draws through `ocelli-wasm`.

## Scope, API and compatibility

- `loadCore()` initialises the built wasm package when the package directory exists.
- `coreAvailable()` returns true only after that initialisation succeeds.
- A worker module calls `bindCpuCanvas` and `publishCpuCanvas` for one committed monochrome frame.
- The page shows the tier the worker reports and whether the canvas published.
- A DICOMweb form calls the existing client for one WADO-RS instance and posts those bytes to the worker. The worker commits them through the session. Pixels are not posted back.

Out of scope: a new `commit_dicomweb_response` export, a second wasm-bindgen crate, SharedArrayBuffer, and turning `coreAvailable()` true when the package failed to load.

A failed load leaves `coreAvailable()` false and the page on the existing "core not built" state.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Example render section | `loadCore`, worker, `bindCpuCanvas`, `publishCpuCanvas` | One synthetic frame | `coreAvailable()` true after load, a published canvas, a tier name |
| Example DICOMweb form | existing `DicomwebClient` | Caller-supplied base URL and UIDs | Bytes handed to the worker, or the client's existing error with no body retained |
| Tests that expect `coreAvailable()` false before load | `coreAvailable` | No `loadCore` call | false, unchanged |

## State transitions and invariants

`coreAvailable()` starts false. `loadCore()` either sets it true for the page or leaves it false. A second successful load does not create a second wasm instance in that worker. A failed publish does not clear the last successful canvas.

## Ownership, commit and recovery

The worker owns the wasm session and the offscreen canvas. The shell owns the fetch. Commit is `publishCpuCanvas` returning Ok. A refused commit leaves the previous image.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Before `loadCore`, `coreAvailable()` is false | existing package test | Returning true from a clean import | implementation |
| AC2 | After a successful load in the worker test double, `coreAvailable()` is true | package test with a fake initialiser | Flipping the flag when init throws | implementation |
| AC3 | `publishCpuCanvas` is the only blit, and the shell has no pixel getter | source test or existing canvas contract | Returning a pixel array to the page | implementation |
| AC4 | A WADO-RS instance fetch uses `DicomwebClient` and posts the body to the worker | unit test with a fake fetch | Parsing DICOM tags in TypeScript | implementation |
| AC5 | The example page shows the tier and the core-available flag | page section | A render path with no control | integrated |

## Approach

Keep wasm-bindgen inside `ocelli-wasm`. Add the loader and the worker beside the example page. Reuse `DicomwebClient` and the session bulk commit. Do not add a pixel export.

## Dependencies, write ownership and resources

Depends on F-191, F-192, F-194 and F-112, all done. Writes `packages/core`, `examples/viewer-react`, and, only if a tier string is missing, one wasm-bindgen function in `ocelli-wasm`. Also the changelog and the sprint ledgers. The GPU stays on this worktree. No second worker runs a browser test at the same time.

## Verification and evidence reuse

Package tests for the flag and the fetch handoff. `bin/ocelli.sh check ocelli-wasm` if that crate changes. The wasm size gate runs at sprint verification, not as a reason to widen the budget. Feature profile at story close.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only
- Pixels across the boundary: no
- Render-loop allocation: the canvas image is sized at bind and resize, not per frame
- unsafe: none
- Tier A (WebGPU): the worker reports the tier the adapter resolved. It does not substitute another tier
- Tier B (WebGL2): same report
- Tier C (CPU): `publishCpuCanvas` draws the CPU raster. A machine with no GPU still publishes through that path when the session binds it

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | `coreAvailable` stays false until load succeeds | `packages/core` |
| contract | fetch bytes are forwarded and tags are not parsed in the shell | `packages/core` |

## Parity surface covered

none. This story wires the example. It does not mark a parity row measured.

## Deviations

none

## LLD impact

One paragraph in `docs/lld/dicomweb.md` stating that the example page fetches one instance and the worker commits the bytes. The open `commit_dicomweb_response` gap stays recorded.

## Open questions

none
