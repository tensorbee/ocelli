# F-116, migration order

**Status**: approved
**Epic ref**: E19.3
**Sprint**: S41
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/09-migration.md` section 12 says this. The fence is the author's text.

```text
A new API plus an existing product means the migration is engineering work, not documentation. The mitigation is to keep the integration seam identical even though the API is not: the new library enables on a plain DOM element and dispatches events on it, exactly as cornerstone does. That single constraint is what makes incremental replacement possible.

- Both libraries coexist in the application, with a feature flag per viewport.
- Strangler by viewport type — stack viewports first, being the highest-volume and lowest-risk surface, then MPR, then 3D.
- Shadow mode before cutover on each viewport type: render both, diff, alert, and only then flip the flag.
- Codemods for the mechanical call-site changes, with genuinely different concepts documented rather than auto-migrated.
```

F-108 already rewrites a listener string when the event map has an Ocelli name. This story does not replace that function.

## What the specification does not cover

Section 12 names the order and does not give a function. It does not say what a skipped step returns.

That gap is a local decision. It is not an HLD formula.

`nextMigrationKind(done)` returns the next kind only when `done` is a prefix of `stack`, then `mpr`, then `volume`.

- An empty list returns `stack`.
- `stack` returns `mpr`.
- `stack` then `mpr` returns `volume`.
- All three returns null.
- A skip, a duplicate, an unknown name or a reordered list returns null.

The example in the LLD is this call sequence. This story does not add an npm package.

## Problem and reproducer

The playbook is prose. A caller can list MPR before stack and nothing in the shell rejects that order.

## Scope, API and compatibility

In scope:

- `nextMigrationKind` in `packages/core/src/playbook.ts`
- Tests and the published name
- `docs/lld/playbook.md` stating the section 12 order and one worked call sequence
- The public-api row

Out of scope:

- Editing `rewriteListenerNames`
- A new example package
- Claiming that 1.0.0 is published
- Flipping a feature flag
- Rendering a shadow diff

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A migration driver | `nextMigrationKind` | Kinds already cut over, in order | The next kind, or null |
| `rewriteListenerNames` | unchanged | F-108 | Unchanged |
| Feature flags | unchanged | F-107 | Unchanged |

## State transitions and invariants

Stateless. The input list is not rewritten. A null result does not mean the migration is finished unless the list is exactly the three kinds in order. A skipped kind is also null, so the caller must look at the list to tell those two nulls apart. The LLD says that.

## Ownership, commit and recovery

Not applicable. No flag is stored and no file is written by the function.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Empty input returns `stack` | unit | Starting at MPR | implementation |
| AC2 | Stack then MPR then volume, one step at a time | unit | Allowing volume before MPR | implementation |
| AC3 | The finished list returns null | unit | Returning `stack` again | implementation |
| AC4 | A skip, a duplicate or an unknown name returns null | unit | Repairing the list | implementation |
| AC5 | The LLD names the same order | file read in the test | A doc that starts at MPR | implementation |
| AC6 | The published name is exported | `index.test.ts` | Leaving the function private | implementation |

## Approach

One new module and one LLD page. No wasm-bindgen. No pixels. No `unsafe`. No listener.

## Tiers

| Tier | Behaviour |
|------|-----------|
| A | Not applicable. The order does not render. |
| B | Not applicable. The same kind is returned. |
| C | Not applicable. The same kind is returned. |

Shadow rendering stays the existing story. This function does not draw.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the prefix order and the refusals | `playbook.test.ts` |
| doc | the LLD names stack, then MPR, then volume | the same |
| export | published name | `index.test.ts` |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

`docs/lld/playbook.md`, `docs/lld/public-api.md`, the public-api row and a playbook row in `docs/lld/README.md`.

## Open questions

None. The prefix rule is the approved local decision.
