# F-106, side-by-side runtime

**Status**: approved
**Epic ref**: E17.2
**Sprint**: S38
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/09-migration.md` section 12 says a new API plus an existing product
means the migration is engineering work, not documentation. The mitigation is
to keep the integration seam identical even though the API is not. The new
library enables on a plain DOM element and dispatches events on it.

The same section says both libraries coexist in the application, with a
feature flag per viewport. Stack viewports move first, then MPR, then 3D.
Shadow mode renders both, diffs, alerts, and only then flips the flag.
Codemods cover the mechanical call-site changes. A concept that is different
is documented rather than auto-migrated.

The author's strangler bullet contains an em-dash. This plan does not copy it.

Decision D1 says the API is new and is not a drop-in shim. Decision D4 says
events are polled from a ring per frame.

F-105 owns `enableElement`, `disableElement` and `dispatchOnElement`. F-107
owns the per-viewport flag. F-109 owns shadow mode. F-108 owns the listener
rename. This story does not define a second copy of any of those.

## What the specification does not cover

Section 12 does not name the function that holds the two elements, the type
of the neighbour, or which library package this repository imports.

This plan decides those gaps. They are not HLD formulas.

The function is `attachSideBySide` in `packages/core`. It enables only the
Ocelli element, through `enableElement`. The neighbour is the element the
application uses for the other library. This package does not import that
library and does not call it. `release` disables only the Ocelli element.
The feature flag and the second render stay later stories.

## Problem and reproducer

An application that enables Ocelli on the same element it already gave the
other library, or that tears the neighbour down when Ocelli is released,
cannot keep both libraries alive.

The expected observation is two distinct elements. The Ocelli element
receives `viewport.mounted`. The neighbour does not. After release, a
listener the caller put on the neighbour still runs, and a later Ocelli
dispatch does not.

## Scope, API and compatibility

In scope:

- `attachSideBySide` in `packages/core/src/alongside.ts`.
- The published name on the package index.
- One paragraph in the migration guide.

Out of scope:

- Importing `@cornerstonejs/core` or any other viewer.
- A feature flag, a second render, a diff, or an alert.
- Editing `seam.ts`, `events.ts`, or `migrate.ts`.
- Rewriting `cornerstone.enable`.

```ts
export interface SideBySide {
  readonly ocelli: ElementSeam;
  readonly neighbour: EventTarget;
  release(): boolean;
}

export function attachSideBySide(
  ocelliElement: EventTarget,
  neighbour: EventTarget,
): SideBySide | null;
```

The same object for both arguments returns null and calls nothing. A value
that `enableElement` refuses returns null. The neighbour is not passed to
`enableElement` or `disableElement`. A second attach of the same Ocelli
element returns null and does not replace the first pair. `release` removes
that pair and then disables the Ocelli seam. A second release returns false
and does not disable again.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `attachSideBySide` | `enableElement` | Two distinct targets | A pair, or null |
| `SideBySide.release` | `ElementSeam.disable` | The pair this call returned | True once, then false |
| Application neighbour | The other library, outside this package | After a non-null return | Unchanged by Ocelli |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No pair | Two distinct targets | Ocelli enabled, neighbour stored | Neighbour listeners stay |
| No pair | The same target twice | Null | That target is not enabled by this call |
| A live pair | Attach of that Ocelli element again | Null | The first pair stays |
| A live pair | `release` | Ocelli disabled, pair removed | Neighbour listeners stay |
| Released pair | `release` again | False | No second `viewport.unmounted` |

One Ocelli element has at most one pair. Dispatch from the returned seam
delivers only to that element.

## Ownership, commit and recovery

The pair holds the `ElementSeam` from `enableElement` and the neighbour
target. `release` drops the pair before it disables the seam, so a throw
from `viewport.unmounted` does not restore the pair. A failed
`enableElement` does not store a pair and does not touch the neighbour.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Two distinct elements | Ocelli receives `viewport.mounted`. The neighbour does not. A neighbour listener still runs for a neighbour event. An Ocelli dispatch does not | Enabling the neighbour, or delivering the Ocelli event to both | `alongside.test.ts` |
| AC2 | The same element for both arguments | Null, and that element receives no `viewport.mounted` | Enabling it once and treating it as both | `alongside.test.ts` |
| AC3 | `release`, then a neighbour event, then a second `release` | The first release returns true. The neighbour listener still runs. The second release returns false and does not dispatch again | Disabling the neighbour, or dispatching unmounted twice | `alongside.test.ts` |
| AC4 | A non-object neighbour, and an Ocelli element whose mounted dispatch throws | Both calls return null. The neighbour is unchanged. No pair remains | Throwing, or leaving the Ocelli element enabled | `alongside.test.ts` |
| AC5 | A second attach of the same Ocelli element with a different neighbour | Null. The first seam still dispatches. The first neighbour stays | Replacing the pair | `alongside.test.ts` |

## Approach

Call `enableElement` only after the two arguments fail `Object.is`. Store the
returned seam and the neighbour in a `WeakMap` keyed by the Ocelli element.
`release` deletes that entry and then calls `disable` on the seam. Do not
edit `seam.ts`.

Rejected alternatives: a dependency on `@cornerstonejs/core`, enabling both
elements with `enableElement`, and a flag that chooses which element is live.

## Dependencies, write ownership and resources

Depends on F-105, which is done. Shared ledgers are `CHANGELOG.md`,
`docs/lld/public-api.md`, `docs/lld/migration.md`, `docs/lld/README.md`,
`docs/sprints/AS_BUILT.md`, `docs/sprints/BACKLOG.md`,
`docs/sprints/CURRENT_SPRINT.md` and `docs/sprints/SPRINT_TRACKER.md`.
No GPU. No corpus row. Do not edit `seam.ts`.

## Verification and evidence reuse

`npx tsc -p packages/core --noEmit --pretty false` without
`--incremental false`. `npm test` for `alongside.test.ts` and
`index.test.ts`. Feature profile is `bin/ocelli.sh gate --floor` plus
the corpus gate. No comparison report on the feature commit.

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
| Unit | AC1 to AC5 | `packages/core/src/alongside.test.ts` |
| Published names | `attachSideBySide` is on the package object | `packages/core/src/index.test.ts` |

No pixel fixture. There is no pixel arithmetic.

## Parity surface covered

none. This story does not render a frame.

## Deviations

none

## LLD impact

`docs/lld/migration.md`, `docs/lld/public-api.md`, and the rows for both
in `docs/lld/README.md`.

## Open questions

none. The operator declined a design-question round. The function name, the
refusal to import the other library, and `release` touching only the Ocelli
element are the local decisions in this plan.

dwv, Horos, and the medical viewer Grok were not opened.
