# F-111, cold start and first frame

**Status**: approved
**Epic ref**: E18.2
**Sprint**: S41
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/23-performance-rules.md` section 26 says this.

```text
Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer.
```

`docs/hld/A-spike-gates.md` gate A4 asks whether binary size and cold start land within budget. The same file estimates 3 to 8 MB uncompressed before tuning, with Naga dominating, and says that figure is unmeasured.

`docs/hld/12-workspace-and-build.md` section 15.2 is the release profile. `docs/lld/benchmarks.md` records that a dev-profile cold start is a different number. The cold-start subject starts when the page begins loading the wasm-pack glue and ends when the first `ocelli_version()` call returns. The five phases are the glue module script, the fetch of `ocelli_wasm_bg.wasm`, `WebAssembly.compile`, instantiate, and the first call.

`render.first_frame` starts at the typed command that asks for the first render and ends at the first presented-frame event. It is not a pixel read-back.

The allocation note on this story says to measure honestly. It names cornerstone's lazy-loaded codec modules as the comparison, not as a formula.

## What the specification does not cover

Section 26 does not say which file a decode-only caller fetches. A4 does not say that the F-006 figure of 2.3 ms is the current module. The subject registry does not name a second artefact directory.

Those gaps are local decisions. They are not HLD formulas.

`startupArtifact` returns one relative path. `render` is `pkg/ocelli_wasm_bg.wasm`. `decode` is `pkg-decode/ocelli_wasm_bg.wasm`. A render path does not name `pkg-decode`. A decode path does not name `pkg/ocelli_wasm_bg.wasm`.

`summariseColdStart` adds the five phases only when the profile is `release` and every phase is finite and at least 0. Any other input returns null. The function does not read `ci/bench-baseline.json` and does not treat 2.3 as a special value.

`summariseFirstFrame` returns the caller interval only when the profile is `release` and the interval is finite and at least 0. It does not add setup.

This story does not run the browser harness and does not replace the 2.3 ms baseline. Gate A4 stays open. The size budget file is not edited.

## Problem and reproducer

A decode-only caller has no recorded choice of artefact. The render module measured by the S40 sprint gate is 2,171,133 bytes. The decode module is 16,455 bytes. Fetching the render module in order to decode pays for wgpu and Naga.

The current cold-start baseline is the F-006 figure. `docs/lld/benchmarks.md` says it is not a measurement of the current module.

## Scope, API and compatibility

In scope:

- `startupArtifact(role)` in `packages/core/src/startup.ts`
- `summariseColdStart(profile, phases)`
- `summariseFirstFrame(profile, commandToPresentMs)`
- Tests and the published names
- A short LLD note in `docs/lld/benchmarks.md`
- The public-api row

Out of scope:

- Rebuilding wasm
- Editing `ci/wasm-size-budget.json` or `ci/bench-baseline.json`
- Closing gate A4
- A browser probe
- Changing `tools/bench/src/runners/wasm_cold_start.mjs`

Roles are `render` and `decode`. Any other role returns null from `startupArtifact`.

Phases are `glue`, `fetch`, `compile`, `instantiate` and `firstCall`. The total is their sum in that order. The function does not round.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Shell choosing a first fetch | `startupArtifact` | Role is render or decode | One relative path, or null |
| A release timing record | `summariseColdStart` | Five phases already measured | The sum, or null |
| A release first-frame interval | `summariseFirstFrame` | Interval already excludes setup | That interval, or null |
| Existing bench runner | unchanged | Still reads `pkg/` | Unchanged |

## State transitions and invariants

Stateless. A second call with the same arguments returns the same result. A null result writes nothing.

## Ownership, commit and recovery

Not applicable. The functions allocate no wasm, open no worker and write no baseline file.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Role `render` returns `pkg/ocelli_wasm_bg.wasm` | unit | Returning the decode path for render | implementation |
| AC2 | Role `decode` returns `pkg-decode/ocelli_wasm_bg.wasm` | unit | Returning the render path for decode | implementation |
| AC3 | Any other role returns null | unit | Defaulting an unknown role to render | implementation |
| AC4 | Five finite release phases sum | unit | Dropping a phase or treating 2.3 as an input | implementation |
| AC5 | A negative, NaN or non-release profile returns null | unit | Coercing bad numbers to 0 | implementation |
| AC6 | A release first-frame interval is returned unchanged | unit | Adding a startup constant | implementation |
| AC7 | A non-release or non-finite first frame returns null | unit | Accepting a dev-profile time | implementation |
| AC8 | The published names are exported | `index.test.ts` | Leaving the functions private | implementation |

## Approach

One new module. No wasm-bindgen. No pixels. No `unsafe`. No listener.

## Tiers

| Tier | Behaviour |
|------|-----------|
| A | Not applicable. The path choice does not render. |
| B | Not applicable. The same paths are returned. |
| C | Not applicable. The same paths are returned. |

A tier argument is not accepted. The functions do not report a different number on a different tier.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | paths, sums and refusals | `startup.test.ts` |
| export | published names | `index.test.ts` |

No pixel fixture. No baseline edit.

## Parity surface covered

None. The story does not render a frame.

## Deviations

None.

## LLD impact

`docs/lld/benchmarks.md`, `docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`.

## Open questions

None. The path names, the release-only rule and the decision not to replace the 2.3 ms baseline are approved local decisions.
