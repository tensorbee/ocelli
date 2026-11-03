# F-107, per-viewport strangler flags

**Status**: approved
**Epic ref**: E17.3
**Sprint**: S39
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/09-migration.md` section 12 says both libraries coexist in the
application, with a feature flag per viewport. Stack viewports move first,
then MPR, then 3D. Shadow mode renders both and only then flips the flag.

The author's strangler bullet contains an em-dash. This plan does not copy it.

Decision D1 says the API is new and is not a drop-in shim. F-105 owns
`enableElement`. F-106 owns the neighbour element. F-109 owns the shadow
compare. This story does not import the other library and does not compare
frames.

## What the specification does not cover

Section 12 does not name the flag function, the three kind strings, the
default, or what a refused flag does to an element that is already enabled.

This plan decides those gaps. They are not HLD formulas.

The kinds are `stack`, `mpr` and `volume`. All three flags start off.
A later kind cannot turn on while the earlier kind is off. An earlier kind
cannot turn off while a later kind is on. `enableFlagged` calls
`enableElement` only when that kind's flag is on. Turning a kind off
disables the elements this module enabled for that kind.

## Problem and reproducer

A viewport that starts using Ocelli while an earlier kind is still off, or
a flag that turns off without disabling the elements it enabled, does not
follow the strangler order.

The expected observation is that `setKindEnabled("mpr", true)` returns
false while stack is off, and that turning stack off disables only the
stack elements this module enabled.

## Scope, API and compatibility

In scope:

- `setKindEnabled`, `kindEnabled` and `enableFlagged` in
  `packages/core/src/flag.ts`.
- The published names on the package index.

Out of scope:

- Importing another viewer.
- Comparing two frames. That is F-109.
- Editing `seam.ts`, `alongside.ts`, `events.ts` or `migrate.ts`.
- A per-element flag that ignores the kind order.

```ts
export type ViewportKind = "stack" | "mpr" | "volume";

export function kindEnabled(kind: ViewportKind): boolean;

export function setKindEnabled(kind: ViewportKind, enabled: boolean): boolean;

export function enableFlagged(
  element: EventTarget,
  kind: ViewportKind,
): ElementSeam | null;
```

An unknown kind returns false from the setters and null from
`enableFlagged`. Setting a flag to the value it already has returns true
and does not disable. A refused change returns false and leaves every flag
and every element as they were.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `setKindEnabled` | The three flags | Kind order | True, or false with no change |
| `enableFlagged` | `enableElement` | Flag for that kind is on | A seam, or null |
| Turn a kind off | `ElementSeam.disable` | Elements this module enabled for that kind | Those elements receive `viewport.unmounted` |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| All flags off | Enable stack | Stack on | MPR and volume stay off |
| Stack off | Enable MPR or volume | False | No flag changes |
| Stack on, MPR off | Enable volume | False | Volume stays off |
| Stack on, MPR on | Disable stack | False | Stack stays on and its elements stay |
| A kind on, with elements | Disable that kind, and no later kind is on | Flag off, those elements disabled | Other kinds stay |
| Flag off | `enableFlagged` | Null | `enableElement` is not called |
| Flag on | `enableFlagged` twice on one element | The second call is null | The first seam stays |

## Ownership, commit and recovery

The module stores the seam `enableElement` returned. Turning a kind off
deletes each stored element for that kind and then calls `disable`. A throw
from `viewport.unmounted` does not put the element back, because `disable`
already dropped the seam registration. A refused `setKindEnabled` does not
call `disable`.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Enable MPR while stack is off | False. `kindEnabled("mpr")` stays false | Turning MPR on anyway | `flag.test.ts` |
| AC2 | Enable stack, then MPR, then volume. Then disable stack | Volume enable returns false while MPR is off. After MPR is on, volume can turn on. Disabling stack returns false and stack stays on | Allowing volume before MPR, or turning stack off while MPR is on | `flag.test.ts` |
| AC3 | Stack is on. `enableFlagged` once, then again | The first call returns a seam and the element receives `viewport.mounted`. The second returns null | Enabling twice | `flag.test.ts` |
| AC4 | Flag off, then turn stack off after one enabled element | The off flag does not enable. Turning stack off sends `viewport.unmounted` and a later dispatch returns false | Leaving the element enabled, or enabling while the flag is off | `flag.test.ts` |
| AC5 | Set stack to true twice, and pass a kind that is not one of the three | The second true returns true and does not disable. An unknown kind returns false and changes nothing | Disabling on a repeated set, or accepting an unknown kind | `flag.test.ts` |

## Approach

Keep one boolean per kind in a fixed order, `stack`, `mpr`, `volume`.
Check the previous kind before turning one on, and the next kind before
turning one off. Store enabled elements in a map from kind to a set, and
the seam in a `WeakMap`. Do not edit `seam.ts`.

Rejected alternatives: one boolean for the whole application, a flag that
does not disable on the way off, and importing the other library.

## Dependencies, write ownership and resources

Depends on F-106, which is done. Lands before F-109 because the two stories
share the delivery ledgers. No GPU. No corpus row. Do not edit `seam.ts`.

## Verification and evidence reuse

`npx tsc -p packages/core --noEmit --pretty false` without
`--incremental false`. `npm test` for `flag.test.ts` and `index.test.ts`.
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
| Unit | AC1 to AC5 | `packages/core/src/flag.test.ts` |
| Published names | `enableFlagged`, `kindEnabled` and `setKindEnabled` are on the package object | `packages/core/src/index.test.ts` |

No pixel fixture. There is no pixel arithmetic.

## Parity surface covered

none. This story does not render a frame.

## Deviations

none

## LLD impact

`docs/lld/migration.md`, `docs/lld/public-api.md`, and the rows for both
in `docs/lld/README.md`.

## Open questions

none. The operator declined a design-question round. The three kind names,
the off default, and the order checks are the local decisions in this plan.

dwv, Horos, and the medical viewer Grok were not opened.
