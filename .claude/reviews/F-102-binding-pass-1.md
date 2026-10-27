# F-102 review, pass 1

**Reviewed**: uncommitted F-102 work on `sprint/s17` at parent `245bb469a05ba737f540a1702d6d1a9e62e03114`. The pass covered `packages/react/src/pump.ts`, `useOcelliViewport.ts`, `OcelliViewport.tsx`, `index.ts`, both test files, `examples/viewer-react/src/App.tsx`, the packaging docs, the changelog and the sprint ledger rows.

**Author / reviewer**: The same session wrote the implementation and this pass. The operator directed that session to review and repair without waiting on DeepSeek. This is not an independent reviewer.

**Result**: 0 defects, 0 smells, 1 nitpick.

## Acceptance challenged

AC1 and the idle frame. Ten pans produce one `apply_commands` whose bytes equal `packCommands`. `surfaces_dropped` flushes with an empty queue, and that frame allocates nothing and applies nothing. A frame that always submitted would fail that assertion.

AC5. `stop_is_idempotent` cancels the scheduled frame, and a later `fire` does not drain. `restart_inside_the_frame_keeps_one_loop` calls `stop` then `start` from `onEvents`. The request count stays at 2, so the outer frame does not schedule a second callback.

AC6. Changing `pending.length > 0` to `pending.length > 1` made `refusal_is_reported_once` receive `[]` against `[1]`. `batches_one_frame` stayed green. Restoring the comparison made the file pass. `refusal_callback_does_not_reenter` calls `flushNow` from `onRefused`, and the refusal count stays 1.

AC9. The checkpoint fixture is the F-050 `OCS1` record, and `parseSnapshot` reports ok. A nonzero `restore_snapshot` returns false, and a following `flushNow` still does not apply.

AC10. The jsdom component starts one frame, stops on unmount, and renders `data-ocelli-core="absent"` without a session. A new `onEvents` function does not request a second frame, and the flush after that rerender observes marker 2.

AC12. `drawing` is `TIER_C_DRAWING`, which is `unavailable`, with and without a session.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run packages/react/src/pump.test.ts packages/react/src/OcelliViewport.test.tsx` | pump and component tests after the idle-frame and refused-restore assertions | executed now | 17 passed, exit 0 |
| `npx eslint packages/react/src` | pump, hook, component and tests | executed now | exit 0 |
| `npm run lint` and `npm run typecheck` | workspace eslint and `tsc --build` | prior receipt this session, before the last test assertions | both exit 0 |
| batch threshold mutation | `pending.length > 0` changed to `> 1`, then restored | executed now | `refusal_is_reported_once` red, `batches_one_frame` green, suite exit 1, source restored |
| `npx vitest run packages/core/src/events.test.ts` | listener scan over `packages/core/src` and `packages/react/src` | prior receipt this session | 8 passed, exit 0 |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, unknown pump bind is silent

**Where**: `packages/react/src/pump.ts`, `bindViewportPump`
**What**: a pump that `createViewportPump` did not register is ignored.
**Why it does not block**: the hook is the only caller, and it binds the object just returned. A wrong object would keep the construction-time callbacks.

## Verified clean

The pump submits through `writeCommands` and `sessionCommandSink`. No wasm memory view is built in `packages/react`. The queue is cleared before the write, so a zero return is not retried. `sameReadback` walks `VIEWPORT_STATE_FIELDS` and compares the camera by value with `Object.is`. `cpu_frame` and `presented_frame` on the fake session stay at 0. The six ring names dispatched by `imports_the_catalogue` are members of `OCELLI_EVENTS`. The published package manifest does not gain `jsdom` or testing-library. The example factory returns null, and the host stays absent.

The plan's frame body names `submitCommands(write, ...)`. A wasm memory view is legal only in `bulk.ts` and `panic.ts`, so the pump calls `writeCommands` and the session exposes `memory`. The plan's hook stores the pump in a ref. The recommended hooks rules refuse that read during render and refuse setState from the effect, so the pump is created during render and callbacks are bound after commit. Neither change is an HLD departure.

## Limitations and handoff

AC11, `npm run dev`, was not run. There is no browser observation of the example viewer. This pass is not an independent review. Feature verification is the next action.
