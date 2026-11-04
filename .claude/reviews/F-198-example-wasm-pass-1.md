# F-198 review, pass 1

**Reviewed**: uncommitted F-198 on `sprint/s43`. HEAD was `90723c4`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 1 defect, 3 smells, 2 nitpicks

## Acceptance challenged

AC1 stays false on a clean import. AC2 matches the flag transitions in `load.ts`. AC3 adds no pixel getter. AC4 posts the WADO-RS body through `DicomwebClient` and reads no DICOM tag. AC5's page test mocks the worker, so it does not prove the dev page loads the core.

## Defects

### D1, the dev page never loads the core under StrictMode

**Where**: `examples/viewer-react/src/core-panel.tsx`, `examples/viewer-react/src/core-worker-client.ts`, `examples/viewer-react/src/main.tsx`

**What**: `main.tsx` renders inside `StrictMode`. Development runs the effect, then cleanup, then the effect again on the same canvas. The second `transferControlToOffscreen` throws `InvalidStateError`. `openCoreWorker` returns null. The header stays on `core not built`.

**Why it is wrong**: the approved plan's AC5 says the example page shows the tier and the core-available flag.

**Evidence**: source trace. The page test mocks `openCoreWorker` and does not render `StrictMode`.

**Scope and regression**: a panel test that keeps the real client, renders `StrictMode`, and throws on a second transfer of the same element.

## Smells

### S1, holdSessionBytes parks the only bulk slot

`Session::alloc` returns 0 while a slot is outstanding. After `holdSessionBytes`, `writeFrame`, `writeCommands` and `writeStack` refuse. The export did not say so.

### S2, the held bytes have lost their boundary

The sink receives the multipart envelope and `kind.boundary`. The post kept the bytes and dropped the boundary.

### S3, nothing executes the adapter-wins rule

`stored_tier` and `resolvedTier` had no test. Returning a constant `C` would leave the suite green.

## Nitpicks

### N1, "finishes" where the contract says the load succeeds

### N2, a failed bind or publish hides the tier

## Verified clean

wasm-bindgen stays in `ocelli-wasm`. The worker posts no pixels. Views over wasm memory stay in `bulk.ts`. `loadCore` does not call the initialiser a second time after success. An empty URL does not fetch.

## Limitations and handoff

The reviewer ran nothing. No browser was opened. The next pass reviews the remediation.
