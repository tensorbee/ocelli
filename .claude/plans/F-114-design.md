# F-114, browser matrix and tier certification

**Status**: approved
**Epic ref**: E19.1
**Sprint**: S40
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/05-rendering.md` section 7 says this.

```text
Two capability tiers, one codebase. Tier A is WebGPU. Tier B is WebGL2
through wgpu's downlevel profile. Every feature declares the tier it
needs. The tier resolves once at startup.
```

Deviation D-07 adds tier C, CPU. A feature that cannot run on the
resolved tier reports unavailable.

`docs/spikes/A7-tier-c.md` widens this story. The matrix must certify
tier C on a software-rasteriser session, not only tier B in a browser.
A software rasteriser that reports WebGL2 is still tier C. Presence of
a WebGL2 API is not certification of tier B.

## What the specification does not cover

Section 7 does not list the browsers, and it does not say how a missing
browser is recorded.

Those gaps are local decisions. They are not HLD formulas.

`certifyRow` recomputes the status. A caller-supplied status is
ignored.

A row is `certified` only when `exitCode` is 0, the resolved tier
equals the expected tier, and the renderer string is non-empty.
Tier B is certified only when the renderer string does not contain
`SwiftShader`. Tier C is certified only when the resolved tier is C
and the renderer string does contain `SwiftShader`. A SwiftShader
string with expected tier B returns null. A missing observation is
`not-run`. Any other finished run is `observed`.

The named rows are `chromium-webgl2`, `chromium-swiftshader`,
`firefox`, `safari` and `enterprise-locked`. This machine can run the
first two only when the existing probe tools actually finish. The
other three stay `not-run`. A row is not marked `certified` because
the tool exists.

`ci/browser-matrix.json` stores the recomputed rows. It starts with
every row `not-run` until an observation is recorded.

## Problem and reproducer

Nothing refuses a matrix that marks Firefox certified when Firefox was
not launched. That is the defect. A SwiftShader session recorded as
tier B is the same defect, because A7 says that session is tier C.

## Scope, API and compatibility

In scope: `packages/core/src/matrix.ts`, its tests, the published
name, and `ci/browser-matrix.json`.

Out of scope: a new browser farm, a pixel tolerance change, and a
claim that an unrun browser passed.

```ts
export function certifyRow(row: MatrixInput): "certified" | "observed" | "not-run" | null;
```

`MatrixInput` carries `id`, `expectedTier` of `"A"`, `"B"` or `"C"`,
and an optional observation `{ renderer, resolvedTier, exitCode }`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Matrix file and tests | `certifyRow` | one row | certified, observed, not-run, or null |
| `index.ts` | re-export | published name | the same function |

## State transitions and invariants

The function is stateless. It does not write the JSON file. A refused
row returns null and does not become `certified`.

## Ownership, commit and recovery

Not applicable. No browser is launched by the function.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | SwiftShader, tier C, exit 0, expected C is certified | `matrix.test.ts` | Certifying tier C without that string | implementation |
| AC2 | A renderer without SwiftShader, tier B, exit 0, expected B is certified | the same | Certifying B from a software rasteriser | implementation |
| AC3 | SwiftShader with expected B returns null | the same | Recording that pair as tier B | implementation |
| AC4 | No observation is not-run, even if a status field says certified | the same | Trusting a caller status | implementation |
| AC5 | Exit code 1 is observed | the same | Treating a failed probe as certified | implementation |
| AC6 | The committed matrix has no certified row unless its observation satisfies AC1 or AC2 | the JSON file read by the test | A hand-written certified row | implementation |
| AC7 | The published name matches the module export | `index.test.ts` | A private helper presented as the API | implementation |

## Approach

One function. The JSON file is data the test loads. Firefox, Safari
and the locked estate are present as `not-run` with no observation.
The Chromium rows are `not-run` in this commit unless a probe on this
machine writes an observation that `certifyRow` accepts. A probe that
does not finish does not upgrade the row. The SwiftShader row expects
tier C. The other four named rows expect tier B. That assignment is a
local decision. Section 7 does not name the rows. No row in this
commit carries an observation, so none is certified.

The existing `tools/tier-b/run.mjs` and `tools/swiftshader/run.mjs`
stay the probes. This story does not change their pixel comparison.

## Dependencies, write ownership and resources

Depends on F-042, which is done. Writes `packages/core` and
`ci/browser-matrix.json`. A probe, if run, needs the GPU and is serial
with cargo. The function tests do not need it.

## Verification and evidence reuse

`npx vitest run packages/core/src/matrix.test.ts packages/core/src/index.test.ts`
then the feature profile. Rebuild dist before the floor test gate,
without `--incremental false`.

A missing browser is `not-run`. That is not a passed certification.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a for the classifier. A tier A observation can be `observed` and is not certified by AC1 or AC2
- Tier B (WebGL2): certified only by AC2
- Tier C (CPU): certified only by AC1

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the six row outcomes | `matrix.test.ts` |
| file | the committed matrix agrees with the function | the same |
| export | published name | `index.test.ts` |

No pixel fixture. The tier B pixel probe remains F-042's tool.

## Parity surface covered

None.

## Deviations

None. D-07 is already recorded. This plan cites it and does not extend it.

## LLD impact

`docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`.

## Open questions

None.
