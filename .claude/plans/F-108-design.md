# F-108, migration guide and codemods

**Status**: approved
**Epic ref**: E17.4
**Sprint**: S37
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/09-migration.md` section 12 says codemods cover the mechanical
call-site changes, with genuinely different concepts documented rather
than auto-migrated.

The same section says the library enables on a plain DOM element and
dispatches events on it. Decision D1 says the API is new and is not a
drop-in shim. Decision D4 says events are polled from a ring per frame.

F-103 records `CORNERSTONE_EVENT_MAP`. A row with a non-null `ocelli`
name is a mechanical rename. A row whose `ocelli` is null is a concept
this catalogue does not deliver. `CORNERSTONE_EVENT_COUNT` is 103.

F-105 owns `enableElement`. This story does not define a second seam.

## What the specification does not cover

Section 12 does not name the codemod engine, the file it reads, or which
call shapes are mechanical. It does not give a guide outline.

This plan decides those gaps. They are not HLD formulas.

The codemod is a TypeScript function in this package. It does not add
jscodeshift and it does not shell out. It rewrites a string literal that
is the first argument of `addEventListener` or `removeEventListener` when
that literal is a cornerstone name whose map row has an Ocelli name. Every
other token stays. `cornerstone.enable` stays. A null `ocelli` row stays.
The guide is `docs/lld/migration.md`.

## Problem and reproducer

A call site that still says `addEventListener("ELEMENT_ENABLED", listener)`
does not hear `viewport.mounted`. Rewriting `CAMERA_MODIFIED` to an
invented name would claim a delivery the map marks unavailable.

The expected observation is the first literal becoming
`viewport.mounted`, and the second literal staying `CAMERA_MODIFIED`.

## Scope, API and compatibility

In scope:

- `rewriteListenerNames` in `packages/core/src/migrate.ts`.
- The guide in `docs/lld/migration.md`, with one mechanical example and
  one documented difference.
- The published name on the package index.

Out of scope:

- Rewriting `cornerstone.enable`, tool constructors, or viewport setup.
- Implementing the feature flag, the side-by-side runtime, or shadow mode.
- Editing `CORNERSTONE_EVENT_MAP` or the seam module except to import
  nothing from the seam. The guide names `enableElement` in prose. The
  codemod does not call it.
- A second copy of the 103-row table.

```ts
export function rewriteListenerNames(source: string): string | null;
```

A non-string returns null. A string with no matching literal returns the
same characters. The function does not write a file.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `rewriteListenerNames` | `CORNERSTONE_EVENT_MAP` | One source string | Rewritten text or null |
| Migration guide | The same map, by reference | Reader of the LLD | Names the seam and the refusal |

## State transitions and invariants

The function is stateless. The same source always returns the same text.
A refused non-string does not throw. There is no file to preserve.

## Ownership, commit and recovery

Not applicable. The function allocates the returned string and does not
own a DOM node, a file, or a registration.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `addEventListener("ELEMENT_ENABLED", listener)` | The result contains `viewport.mounted` and does not contain `ELEMENT_ENABLED` | Leaving the cornerstone name, or inventing a different Ocelli name | `migrate.test.ts` |
| AC2 | `addEventListener("CAMERA_MODIFIED", listener)` | The literal stays `CAMERA_MODIFIED` | Rewriting an unavailable row | `migrate.test.ts` |
| AC3 | `cornerstone.enable(element)` plus a `removeEventListener("ELEMENT_DISABLED", listener)` | `enable` stays and the listener name becomes `viewport.unmounted` | Rewriting `enable` into `enableElement` | `migrate.test.ts` |
| AC4 | A number, and a template literal that merely mentions the name | The number returns null and the template literal is unchanged | Throwing, or rewriting inside a template | `migrate.test.ts` |

## Approach

Build one map from cornerstone name to Ocelli name for rows whose
`ocelli` is a string. Scan the source for `addEventListener` or
`removeEventListener`, optional whitespace, then `(` and a single-quoted
or double-quoted literal with no escape. Replace that literal only when
the map has the name. Quotes of the other calls are not a match. The
guide states the seam function, the once-per-frame drain, the stack then
MPR then 3D order, and that a null row is not rewritten.

Rejected alternatives: a jscodeshift binary, a full parser, and a codemod
that emits `cornerstone.enable` as `enableElement`.

## Dependencies, write ownership and resources

Depends on F-103, which is done. Lands after F-105 because the guide
names `enableElement`. Shared ledgers are `CHANGELOG.md`,
`docs/lld/public-api.md`, `docs/lld/README.md`, `docs/sprints/AS_BUILT.md`,
`docs/sprints/BACKLOG.md`, `docs/sprints/CURRENT_SPRINT.md` and
`docs/sprints/SPRINT_TRACKER.md`. No GPU. No corpus row.

## Verification and evidence reuse

`npx tsc -p packages/core --noEmit --pretty false` without
`--incremental false`. `npm test` for `migrate.test.ts` and
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
| Unit | AC1 to AC4 | `packages/core/src/migrate.test.ts` |
| Published names | `rewriteListenerNames` is on the package object | `packages/core/src/index.test.ts` |

No pixel fixture. There is no pixel arithmetic.

## Parity surface covered

none. The guide does not claim rendered pixels match cornerstone.

## Deviations

none

## LLD impact

`docs/lld/migration.md`, `docs/lld/public-api.md`, and the rows for both
in `docs/lld/README.md`.

## Open questions

none. The operator declined a design-question round. The function name,
the two listener calls, and the refusal to rewrite `enable` are the
local decisions in this plan.

dwv, Horos, and the medical viewer Grok were not opened.
