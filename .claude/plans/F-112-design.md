# F-112, worker pool and the threading decision

**Status**: approved
**Epic ref**: E18.3
**Sprint**: S40
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/07-concurrency-and-typescript.md` section 9 says this.

```text
Start single-threaded per worker. N decode workers, each holding its own
WebAssembly instance. One render worker owning the GPUDevice and every
OffscreenCanvas. The main thread doing DOM, events and tool UI. No
SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain.
```

The same section says to escalate only on a measurement that demands it.
The allocation note says to stay on stable Rust unless measurements force
nightly. Spike A7 says a CPU renderer is not that measurement.

Section 10 leaves DOM and worker construction in TypeScript.

## What the specification does not cover

Section 9 does not give N, and it does not name a function.

Those gaps are local decisions. They are not HLD formulas.

`openDecodePool(owner, count, start)` is the pool. `count` must be an
integer from 1 to 8. The ceiling is local, so a caller cannot open an
unbounded set. `DEFAULT_DECODE_WORKERS` is 1, because this sprint has no
measurement that demands more than one decode instance.

`threadingEscalation()` returns `"stable"`. The pool does not construct
a shared-memory buffer and does not request a nightly toolchain. The
render worker is not started here. One render worker remains outside
this pool, which is what section 9 already assigns to the GPU owner.

`start` is called once per index, with that index only. Tests pass a
fake starter so the suite does not launch a browser.

## Problem and reproducer

The shell has no decode pool, so nothing records the decision to stay
on stable Rust. A pool that set a shared-memory buffer, or that treated
a non-integer count as one worker, would be the defect.

## Scope, API and compatibility

In scope: `packages/core/src/pool.ts`, its tests, and the published
names.

Out of scope: a real `Worker` URL, a render-worker spawn, nightly
features, and any change to `ocelli-wasm`.

```ts
export const DEFAULT_DECODE_WORKERS = 1;
export const MAX_DECODE_WORKERS = 8;
export function threadingEscalation(): "stable";
export function openDecodePool(
  owner: object,
  count: number,
  start: (index: number) => { terminate(): void },
): DecodePool | null;
```

`DecodePool.close()` returns true the first time and false after that.
`DecodePool.sharedMemory` is false. `DecodePool.threading` is
`"stable"`.

A non-object owner, a count outside 1 to 8, or a second open of the
same owner returns null and does not call `start`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Application shell | `openDecodePool` | one owner, one count | N starters, or null |
| `index.ts` | re-export | published names | the same functions |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No pool for this owner | count 2 | two workers, slot stored |  |
| Pool already open | any count | null, start not called | the first pool stays |
| Pool open | close | workers terminated, slot deleted |  |
| Closed | close again | false | terminate is not called again |
| Any | count 0, 1.5, 9 or -1 | null | no worker is stored |

If `terminate` throws, the slot stays deleted and the other workers
still get `terminate`. The thrown worker is not restored.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Worker handles | `start` during open | owner object | the function returns the pool | a refused open stores nothing |

A second close does not terminate again.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 0, 1.5, 9 and -1 return null and do not start | `pool.test.ts` | Treating 1.5 as 1 | implementation |
| AC2 | count 1 calls start with 0 | the same | A pool that does not construct | implementation |
| AC3 | count 2 calls start with 0 and 1 | the same | Sharing one instance | implementation |
| AC4 | A second open of the same owner returns null | the same | Replacing the first pool | implementation |
| AC5 | close terminates each once, and the second close returns false | the same | Terminating twice | implementation |
| AC6 | `threadingEscalation` is `stable` and `sharedMemory` is false | the same | A threaded default | implementation |
| AC7 | The published names match the module exports | `index.test.ts` | A private helper presented as the API | implementation |

## Approach

One `Map` from owner to the handles. The slot is stored before `start`
returns the pool. A later open sees the slot. `close` deletes the slot
before `terminate`.

The file does not name `SharedArrayBuffer`. The test reads `pool.ts`
and fails if that identifier appears.

## Dependencies, write ownership and resources

Depends on F-035, which is done. Writes `packages/core/src/pool.ts`,
`pool.test.ts`, `index.ts` and `index.test.ts`. No GPU. No wasm build.

## Verification and evidence reuse

`npx vitest run packages/core/src/pool.test.ts packages/core/src/index.test.ts`
then the feature profile. Rebuild `packages/core` dist with
`npx tsc -b packages/core packages/react` before the floor test gate,
without `--incremental false`.

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
| unit | count, slot, close, stable decision | `pool.test.ts` |
| export | published names | `index.test.ts` |

No pixel fixture. This story does not compute a sample.

## Parity surface covered

None.

## Deviations

None.

## LLD impact

- `docs/lld/public-api.md`

## Open questions

None.
