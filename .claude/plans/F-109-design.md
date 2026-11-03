# F-109, shadow divergence alert

**Status**: approved
**Epic ref**: E17.5
**Sprint**: S39
**Estimate**: 5w

## Normative source, transcribed

`docs/hld/09-migration.md` section 12 says shadow mode comes before cutover
on each viewport type. Render both, diff, alert, and only then flip the
flag.

Decision D1 says the API is new and is not a drop-in shim. F-106 does not
import the other library. F-107 owns the kind flags. The corpus comparator
owns section 25.1. This story does not change that comparator and does not
claim those tolerances.

## What the specification does not cover

Section 12 does not name the function, the numeric limit, or who renders
the other library.

This plan decides those gaps. They are not HLD formulas.

The application renders both and passes two byte buffers.
`shadowDivergence` counts samples whose absolute difference is greater
than the caller limit. It does not render, does not write either buffer,
and does not import the other library. A count of zero is agreement. A
count above zero is the alert. The function returns that count, or null
when the inputs are refused.

## Problem and reproducer

A cutover that flips a flag without a count of differing bytes cannot
alert. A compare that writes one of the buffers, or that treats unequal
lengths as zero, hides the divergence.

The expected observation is a count of one for buffers `[0]` and `[2]`
with limit 1, and null for buffers whose lengths differ.

## Scope, API and compatibility

In scope:

- `shadowDivergence` in `packages/core/src/shadow.ts`.
- The published name on the package index.
- One paragraph in the migration guide.

Out of scope:

- Rendering either library.
- Importing `@cornerstonejs/core` or any other viewer.
- Editing the corpus comparator, `ci/tolerances.json`, or a kind flag.
- Editing `seam.ts`, `alongside.ts`, `flag.ts`, `events.ts` or `migrate.ts`.

```ts
export function shadowDivergence(
  ocelli: Uint8Array,
  other: Uint8Array,
  limit: number,
): number | null;
```

The lengths must be equal. `limit` must be an integer from 0 to 255
inclusive. Any other limit returns null. A sample is a byte already, so
the absolute difference is from 0 to 255. The comparison is `>` the limit.
Equal differences at the limit are not counted.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `shadowDivergence` | The two caller buffers | Equal length and an integer limit | A count, or null |
| Application renderer | Outside this package | Renders both before the call | Bytes this function does not create |

## State transitions and invariants

The function is stateless. The same inputs return the same count. A
refusal returns null and does not write either buffer. There is no flag
to flip here. F-107 remains the only flag writer.

## Ownership, commit and recovery

Not applicable. The function allocates nothing it keeps, and it does not
own a DOM node, a device, or a file. The returned number is the whole
result.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `[0]` and `[2]` with limit 1 | The count is 1 | Returning 0, or returning null for a real difference | `shadow.test.ts` |
| AC2 | `[0, 5]` and `[0, 5]` with limit 0 | The count is 0 | Counting an equal byte | `shadow.test.ts` |
| AC3 | Lengths 1 and 2, and a limit of 1.5, and a limit of 256, and a limit of -1 | Each call returns null. Both buffers keep their bytes | Treating unequal lengths as 0, or accepting a non-integer limit | `shadow.test.ts` |
| AC4 | `[0, 10]` and `[0, 10]` with limit 0, then one changed copy `[0, 11]` | The first count is 0. The second count is 1. The first buffer is unchanged | Writing the difference back into a buffer | `shadow.test.ts` |

## Approach

Walk the indexes once. Add one when `Math.abs(ocelli[i] - other[i]) > limit`.
Return null before that walk when the lengths differ or the limit is not an
integer in 0..255. Do not copy the buffers.

Rejected alternatives: calling the corpus comparator, rendering a second
library inside this package, and a boolean that hides the count.

## Dependencies, write ownership and resources

Depends on F-106, which is done. Lands after F-107 because the delivery
ledgers are shared. No GPU. No corpus row. Do not edit `flag.ts` or
`seam.ts`.

## Verification and evidence reuse

`npx tsc -p packages/core --noEmit --pretty false` without
`--incremental false`. `npm test` for `shadow.test.ts` and `index.test.ts`.
Feature profile is `bin/ocelli.sh gate --floor` plus the corpus gate. No
comparison report on the feature commit.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| Unit | AC1 to AC4 | `packages/core/src/shadow.test.ts` |
| Published names | `shadowDivergence` is on the package object | `packages/core/src/index.test.ts` |

No DICOM pixel fixture. The counts are hand-computed from the plan. This
function is not the section 25.1 comparator.

## Parity surface covered

none. The count is not a rendered-frame verdict.

## Deviations

none

## LLD impact

`docs/lld/migration.md`, `docs/lld/public-api.md`, and the rows for both
in `docs/lld/README.md`.

## Open questions

none. The operator declined a design-question round. The caller-supplied
buffers, the integer limit, and the refusal to render the other library
are the local decisions in this plan.

dwv, Horos, and the medical viewer Grok were not opened.
