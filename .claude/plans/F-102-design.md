# F-102, Framework bindings (React first)

**Status**: approved
**Epic ref**: E16.3
**Sprint**: S17
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/07-concurrency-and-typescript.md`, section 9, gives the thread
model:

```text
**Start single-threaded per worker.** N decode workers, each holding its own WebAssembly instance; one render worker owning the GPUDevice and every OffscreenCanvas; the main thread doing DOM, events and tool UI. No SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain.
```

`docs/hld/07-concurrency-and-typescript.md`, section 10, gives what stays in
the shell:

```text
Not a compromise — the correct home for this work. Cornerstone has 531 addEventListener sites, 137 uses of document, and 72 createElement calls, and pointer handling is the one workload where crossing into WebAssembly makes things measurably worse.

- DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver

- The SVG annotation drawing layer — DOM-native, and faster left there

- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom

- Framework bindings, DICOMweb fetch and authentication
```

`docs/hld/04-boundary-and-data-path.md`, section 5.1, gives the command
granularity:

```text
Small, infrequent, one call per user intent: set camera, set VOI, activate tool, load series. **Never one call per pointer move.** Pointer streams are normalised in TypeScript and batched into a single packed command buffer per animation frame.
```

`docs/hld/04-boundary-and-data-path.md`, section 5.3, gives the event path:

```text
A fixed-size ring in linear memory, drained once per frame rather than invoking a JavaScript callback per event. This removes a boundary crossing from the hot path and gives coalescing for free: a burst of camera changes during a drag collapses to one delivered event. Viewport state reads back as a small C-layout struct copy, not a JavaScript object graph.
```

`docs/hld/14-the-boundary-in-code.md`, sections 17.1 and 17.3, give the wire
and the ring:

```text
// wire format, little-endian
// [u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
```

```text
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

```text
*Fixed stride so the JavaScript side is arithmetic, not deserialisation. Drain once per animation frame; surface `dropped` in telemetry rather than swallowing it.*
```

`docs/hld/09-migration.md`, section 12, gives the integration seam:

```text
A new API plus an existing product means the migration is engineering work, not documentation. The mitigation is to keep the **integration seam** identical even though the API is not: the new library enables on a plain DOM element and dispatches events on it, exactly as cornerstone does. That single constraint is what makes incremental replacement possible.
```

`docs/hld/20-errors-and-panics.md`, section 23, gives the recovery constraint:

```text
| **A PANIC POISONS THE INSTANCE** Once a Rust panic aborts inside WebAssembly, that module instance's memory may be inconsistent and it must not be reused. So: panic = "abort" in release; no exported function may let a panic escape as a normal error path; and on panic the worker instance is torn down and rebuilt, with the JavaScript shell reconstructing viewport state from its own copy. That last clause is a design constraint on the shell, not an afterthought — the shell must always hold enough state to rebuild a viewport from nothing. |
```

`docs/hld/12-workspace-and-build.md`, section 15.1, gives the package:

```text
> react/ \# @ocelli/react
```

## What the specification does not cover

The HLD says framework bindings stay TypeScript and the shell keeps the seam.
It does not give the React component's props, the hook's shape, the frame
pump's API, the checkpoint policy or the event dispatch type.

Every item below is this plan's decision.

- **The pump.** A plain `createViewportPump` function owns the animation-frame
  loop, the command queue and the ring drain. React binds to it. The behavior
  is testable without a renderer.
- **The command queue.** `enqueue` appends to a pending array and never calls
  the boundary. One frame packs the whole array once and submits it once.
- **The state read.** One `copy_state` per frame, decoded with
  `readViewportState`, dispatched only when a field changed.
- **The checkpoint.** The shell holds the latest F-050 snapshot and the
  selected slice. `checkpoint()` calls `session.snapshot()`, which is not a per
  frame call. `restore(checkpoint)` calls `restore_snapshot` and then one
  `Scroll` step, because `restore` deliberately leaves the scroll alone.
- **The event type.** The pump dispatches F-103's named events. F-102 does not
  declare a second name mapping.
- **The session.** The component takes a session as a prop. There is no module
  singleton and no context provider, because two ways to obtain a session is
  two places to look.

## Problem and reproducer

`@ocelli/react` is a scaffold. `packages/react/src/OcelliViewport.tsx` mounts a
`<div>`, writes two data attributes and renders. It never attaches a session,
never drains the ring and never sends a command. Its own comments name `F-097`,
which is not this story, so the file describes a story that does not exist.

The reproducer is a React application that mounts the component with a session
and drags the pointer across the viewport. Today nothing crosses the boundary,
so no camera changes. A naive implementation that calls `apply_commands` from
the pointer handler crosses the boundary once per pointer move, which HLD
section 5.1 forbids, and an implementation that calls `session.drain_events()`
inside a per-event callback is the D4 defect. There is no API that batches a
pointer stream into one frame or drains the ring once per frame.

Source identity: `packages/react/src/OcelliViewport.tsx`,
`packages/react/src/index.ts` and `examples/viewer-react/src/App.tsx`.

## Scope, API and compatibility

In scope:

- `createViewportPump` in `packages/react/src/pump.ts`, the frame loop, the
  command queue, the ring drain and the state read.
- `useOcelliViewport` in `packages/react/src/useOcelliViewport.ts`, the React
  binding.
- `OcelliViewport` in `packages/react/src/OcelliViewport.tsx`, rewritten to
  use the hook.
- A checkpoint type and the restore order.
- The example viewer wired to a session when one is supplied.
- Correcting the stale `F-097` references to `F-102`.

Non-goals:

- No wasm module loading. The session is supplied by the caller. The example
  viewer keeps its honest "core not built" state when no session is present.
- No second event catalogue. F-103 owns the names.
- No pixel buffer on the public surface. `cpu_frame()` is not exposed.
- No new framework. React is first, and a second framework is a later story.
- No new runtime dependency. `react` and `react-dom` stay peer dependencies.
  Two dev-only dependencies, `jsdom` and `@testing-library/react`, are added so
  the component lifecycle has a real test. Neither enters the published tarball.

Public API:

```ts
// packages/react/src/pump.ts
export interface OcelliSession {
  alloc(len: number): number;
  bulk_ticket(): number;
  release_bulk(ticket: number): number;
  apply_commands(ticket: number, len: number): number;
  drain_events(): Uint8Array;
  copy_state(): Uint8Array;
  snapshot(): Uint8Array;
  snapshot_error(): number;
  restore_snapshot(bytes: Uint8Array): number;
}

export interface ViewportCheckpoint {
  readonly snapshot: Uint8Array;
  readonly sliceIndex: number;
}

export interface ViewportPumpOptions {
  readonly viewport: ViewportId;
  readonly onEvents?: (events: readonly OcelliNamedEvent[], dropped: number) => void;
  readonly onState?: (state: ViewportReadback) => void;
  readonly onRefused?: (commands: readonly PackedCommand[]) => void;
  readonly requestFrame?: (callback: FrameRequestCallback) => number;
  readonly cancelFrame?: (handle: number) => void;
}

export interface ViewportPump {
  enqueue(command: PackedCommand): void;
  start(): void;
  stop(): void;
  flushNow(): void;
  checkpoint(): ViewportCheckpoint | null;
  restore(checkpoint: ViewportCheckpoint): boolean;
}

export function createViewportPump(
  session: OcelliSession,
  options: ViewportPumpOptions,
): ViewportPump;
```

```ts
// packages/react/src/useOcelliViewport.ts
export interface UseOcelliViewportOptions {
  readonly viewport: ViewportId;
  readonly session: OcelliSession | null;
  readonly onEvents?: (events: readonly OcelliNamedEvent[], dropped: number) => void;
  readonly onRefused?: (commands: readonly PackedCommand[]) => void;
}

export interface OcelliViewportHandle {
  readonly pump: ViewportPump | null;
  readonly state: ViewportReadback | null;
  readonly drawing: TierAvailability;
}

export function useOcelliViewport(
  options: UseOcelliViewportOptions,
): OcelliViewportHandle;
```

```tsx
// packages/react/src/OcelliViewport.tsx
export interface OcelliViewportProps {
  readonly id: string;
  readonly viewport: ViewportId;
  readonly className?: string;
  readonly session?: OcelliSession | null;
  readonly onEvents?: (events: readonly OcelliNamedEvent[], dropped: number) => void;
  readonly onRefused?: (commands: readonly PackedCommand[]) => void;
}

export function OcelliViewport(props: OcelliViewportProps): JSX.Element;
```

The component renders a `<div>` with `data-ocelli-viewport` and
`data-ocelli-core`. It does not render a canvas in this story. The canvas
lifecycle belongs to the render worker and a later canvas story. The element is
the seam.

Compatibility: `OcelliViewportProps` gains `viewport` and `session`. The
existing `id` and `className` stay. `@ocelli/react`'s `index.ts` keeps its two
existing exports and adds the new ones. The stale `F-097` comments are
corrected.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `enqueue` | The pump's pending array | A pointer gesture or a tool binding, one command per intent | No boundary call until the next frame |
| `flushNow` and the frame callback | `packCommands`, `submitCommands`, `sessionCommandSink` | One packed buffer per frame | One `apply_commands` call |
| The frame callback | `session.drain_events`, `drainRing` | One drain per frame | One `onEvents` array and the `dropped` count |
| The frame callback | `session.copy_state`, `readViewportState` | One read per frame | `onState` only when a field changed |
| `checkpoint` | `session.snapshot` | Caller-chosen moment, not per frame | A `ViewportCheckpoint` |
| `restore` | `session.restore_snapshot` and one `Scroll` command | The stack is configured first | The camera, presentation and slice restored |
| `OcelliViewport` | `useOcelliViewport` | A session prop or `null` | The host element and the honest absent state |
| F-103 | `OcelliNamedEvent` and the catalogue | Not this story | The names the pump dispatches |

Unchanged consumers affected by changed providers: `packages/core/src/
commands.ts` and `ring.ts` keep their signatures. The example viewer keeps its
`coreAvailable()` path.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| A stopped pump | `start` | One frame is scheduled | A second `start` does not schedule a second loop |
| A running pump | `enqueue` | The command joins the pending array | No boundary call |
| A running pump | A frame with a non-empty queue | One `apply_commands` call with one buffer | A zero return reports `onRefused` and clears the queue |
| A running pump | A frame with an empty queue | No `apply_commands` call, one drain and one state read | None |
| A running pump | A frame with events | `onEvents` receives them in order with `dropped` | A dropped count is surfaced, never swallowed |
| A running pump | A frame with unchanged state | No `onState` call | None |
| A running pump | `stop` | The frame is cancelled and no further boundary call happens | A pending queue stays pending |
| An unmounted component | The effect cleanup | `stop` is called | No pump survives unmount |
| A session change | The effect re-runs | The old pump stops and a new one starts | The old session is not called again |
| A checkpoint | `checkpoint` | A snapshot and the current slice index, or `null` when the core refuses the encode | The caller reads `snapshot_error()` for the code, see AC9 |
| A restore | `restore` | `restore_snapshot`, then one `Scroll` step | A nonzero code returns `false` and leaves the state alone |

Invariants that span consumers:

- **One boundary crossing per frame per channel.** One command buffer, one
  drain, one state copy.
- **No pixel buffer.** The pump never calls `cpu_frame` or `presented_frame`,
  and the public types carry no pixel array.
- **One catalogue.** The pump dispatches F-103's names and declares none.
- **The shell holds a rebuild copy.** The latest checkpoint plus the stack keys
  the caller already holds is enough to rebuild the camera, the presentation
  and the slice.
- **No per-event callback.** The ring is drained once per frame. A per-event
  callback is the D4 defect.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| The animation-frame handle | `requestFrame` | One handle per running pump | The scheduled callback | `stop` cancels it. A second `start` does not create a second handle |
| The pending command array | `enqueue` | Cleared at the frame's commit | A nonzero `apply_commands` return | A refused batch is reported and cleared, so it is not retried forever |
| The ring drain | `drainRing` | One copied buffer per frame | The `onEvents` call | A short buffer yields no events and reports `dropped` |
| The checkpoint | `session.snapshot` | Caller-owned bytes | The returned `ViewportCheckpoint` | A snapshot that fails is reported, never silently replaced with an empty one |
| The restored state | `restore` | The session's `StackViewport` | The `restore_snapshot` return of zero, then the `Scroll` | A nonzero code returns `false` and changes nothing |

No side effect precedes the commit point. A React StrictMode double mount runs
the effect twice, so `start` and `stop` must be idempotent and the cleanup must
cancel the handle. A duplicate `restore` of the same checkpoint is a no-op for
the camera and the presentation and clamps the slice to the same index.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Ten pointer samples enqueued before one frame produce one `apply_commands` call whose buffer holds ten `Pan` commands, and the return is ten | `packages/react/src/pump.test.ts`, `batches_one_frame` | One boundary call per pointer move, which HLD section 5.1 forbids | implementation |
| AC2 | A frame calls `drain_events` once and `copy_state` once, regardless of the number of events | Same file, `drains_once_per_frame` | A callback per event, the D4 defect | implementation |
| AC3 | `onEvents` receives the events in ring order with the `dropped` count, and a nonzero dropped count is passed through | Same file, `surfaces_dropped` | Swallowing the overflow count | implementation |
| AC4 | `onState` is not called when the readback is unchanged, and is called when one field changes | Same file, `state_only_on_change` | A React re-render every frame | implementation |
| AC5 | `stop` cancels the frame and no boundary call follows, and a second `start` does not create a second loop | Same file, `stop_is_idempotent` | A leaked animation-frame loop | implementation |
| AC6 | A refused batch calls `onRefused` once and clears the queue, and the next frame does not retry it | Same file, `refusal_is_reported_once` | An infinite retry of a batch the core will always refuse | implementation |
| AC7 | The pump's public types and calls expose no pixel buffer. A review of the module plus a test that a fake session's `cpu_frame` is never called | Same file, `never_reads_pixels` | A `pixels` prop or a `cpu_frame` read | implementation |
| AC8 | The pump dispatches F-103's named events and declares no name mapping of its own | The module imports the catalogue and a test asserts every dispatched name is in it | A second catalogue that disagrees with the ring | implementation |
| AC9 | `checkpoint` returns a snapshot that `restore` accepts, and the restored readback matches the saved camera, presentation and slice. A refused snapshot returns `null` and `snapshot_error()` names the code, never an empty checkpoint | Same file, `checkpoint_round_trip` and `refused_checkpoint_is_null`, with a fake session backed by the real F-050 record | A rebuild that loses the camera, restores a linear window, or hands back empty bytes as a checkpoint | integrated, needs F-050 |
| AC10 | The component mounts the host element, starts the pump when a session is present, stops it on unmount, and renders the honest absent state when the session is `null` | `packages/react/src/OcelliViewport.test.tsx` under a jsdom environment | A pump that survives unmount, or a component that throws without a core | implementation |
| AC11 | The example viewer renders the absent state without a session and a live state with one, and `npm run dev` starts | Manual smoke test named in `docs/lld/typescript-packaging.md` | A viewer that requires a built module to start | external, manual |
| AC12 | Tier A, tier B and tier C all run the binding. The drawing availability is the core's answer, and tier C reports the stack viewport's GPU drawing `unavailable` while the CPU surface remains separate | The readback's `drawing` value and the existing `TIER_C_DRAWING` | A binding that silently draws a different image on tier C | implementation |

## Approach

The smallest implementation in terms of the transcribed source.

1. **`packages/react/src/pump.ts`.** The pump holds `pending: PackedCommand[]`,
   a `lastState: ViewportReadback | null`, a frame handle and the session.
   `enqueue` appends. The frame callback:
   - If `pending.length > 0`, `packCommands(viewport, pending)` once,
     `submitCommands(write, sessionCommandSink(session), bytes)` once. A zero
     return calls `onRefused(pending)` and clears. A nonzero return clears.
   - `drainRing(session.drain_events())` once. `onEvents(events, dropped)`.
   - `readViewportState(session.copy_state())` once. Compare the six fields and
     call `onState` only on a change.
   - Schedule the next frame while running.
   `flushNow` runs the same body once without scheduling.
   `checkpoint` calls `session.snapshot()`, and returns `null` when the result
   is empty after reading `session.snapshot_error()`. Otherwise it reads
   `sliceIndex` from the last state, defaulting to zero.
   `restore` calls `session.restore_snapshot(checkpoint.snapshot)`, and on
   zero enqueues one `Scroll { step: checkpoint.sliceIndex }` and flushes.

2. **`packages/react/src/useOcelliViewport.ts`.** `useRef` for the pump,
   `useState` for the state, `useEffect` keyed on `session` and `viewport`.
   The effect creates the pump, starts it, and returns a cleanup that stops it.
   `onEvents` and `onRefused` are held in refs so a changing callback does not
   restart the loop.

3. **`packages/react/src/OcelliViewport.tsx`.** Uses the hook. Renders the host
   `<div>` with the data attributes. Sets `data-ocelli-core` from the session.
   No canvas in this story.

4. **The root `package.json`.** Add `jsdom` and `@testing-library/react` to
   `devDependencies`. The component test carries
   `// @vitest-environment jsdom` at the top, so the rest of the suite keeps the
   default environment.

5. **`packages/react/src/index.ts`.** Export the new names and correct the
   stale `F-097` comment.

6. **`examples/viewer-react/src/App.tsx`.** Accept a session from a factory
   that returns `null` when the module is absent. Render the honest state.

7. **`docs/lld/typescript-packaging.md`.** Record the new modules, the two
   dev-only dependencies and the jsdom test environment.

Rejected alternatives:

- **A context provider or a module singleton.** Two ways to obtain a session
  is two places to look. The prop is explicit and testable.
- **A React hook that calls `apply_commands` from `onPointerMove`.** It crosses
  the boundary per pointer move, which HLD section 5.1 forbids.
- **Holding events in React state.** A re-render per frame. The pump calls
  `onEvents` imperatively and the caller decides what to render.
- **A second event catalogue in `@ocelli/react`.** F-103 owns the catalogue.
- **Snapshotting every frame.** It allocates and crosses the boundary per
  frame. The checkpoint is caller-chosen.

## Dependencies, write ownership and resources

Declared dependencies: F-101, done. Source-level dependencies: F-050 for the
snapshot, F-103 for the named events, and F-104 for the generated constants.
This story lands last in the sprint.

Files that cannot have concurrent writers:

- `packages/react/src/{pump.ts,useOcelliViewport.ts,OcelliViewport.tsx,index.ts}`
- `examples/viewer-react/src/App.tsx`
- `packages/core/src/index.ts`, if the pump imports a new core export
- `docs/lld/typescript-packaging.md`

No GPU, no corpus. A browser observation is a manual smoke test, and the
jsdom component test is serial.

## Verification and evidence reuse

Focused checks:

```bash
npm run lint
npm run typecheck
npx vitest run packages/react/src/pump.test.ts
npx vitest run packages/react/src/OcelliViewport.test.tsx
bin/ocelli.sh gate packages
bin/ocelli.sh gate types
bin/ocelli.sh gate lint
```

Final profile: `bin/ocelli.sh gate --sprint` over the staged union, with the
corpus result recorded.

Evidence identities: the pump tests name the fake session and the frame
scheduler. The component test names the jsdom environment. The manual smoke
test names `npm run dev` and the absent-core path. The checkpoint test names
F-050's record version.

What invalidates evidence: any edit to the pump, the hook, the component, the
example viewer or the core exports after the verification.

## Boundary and tier

- wasm-bindgen: not touched. The session is a structural interface.
- Pixels across the boundary: no. The pump never reads a pixel buffer.
- Render-loop allocation: the pump allocates one packed buffer, one drained
  event array and one state copy per frame. The event array is produced by
  `drainRing` and is bounded by the ring's 256-event capacity. The command
  buffer is bounded by `MAX_COMMAND_BYTES`. No per-frame buffer grows without a
  bound.
- unsafe: none.
- Tier A (WebGPU): available. The binding is tier-independent and the drawing
  availability is the core's answer.
- Tier B (WebGL2): available, same answer.
- Tier C (CPU): available for the binding. The stack viewport's GPU drawing
  reports `unavailable` and the CPU canvas surface remains separate, which is
  D-07's honesty rule.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | The pump batches one frame, drains once, reads once, stops idempotently and reports a refusal once | `packages/react/src/pump.test.ts` |
| `unit` | The component mounts, starts and stops with a session, and renders the absent state without one | `packages/react/src/OcelliViewport.test.tsx` under jsdom |
| `property` | A generated family of command batches packs to one buffer whose command count and byte length equal the sum of the generated payload lengths | `packages/react/src/pump.test.ts` |
| `browser` | The manual smoke test in `npm run dev` with a built module | Manual, named in the LLD |
| `fixture` | Not applicable. The pump performs no pixel or geometry arithmetic | This table |

The checkpoint test uses F-050's real record through a fake session, so it is
an integrated observation rather than a second implementation.

## Parity surface covered

Appendix B row `Core events`, `50`, plus 53 tool events, re-shaped rather than
copied. F-102 establishes the delivery half: an event the ring carries is
delivered once per frame, not per event. It does not establish the names or the
mapping, which are F-103's.

## Deviations

None.

## LLD impact

- `docs/lld/typescript-packaging.md`: the new modules, the jsdom test and the
  example viewer's session path.
- `docs/lld/public-api.md`: the React binding's public surface.

## Open questions

**Q1, answered 2026-10-27. The React test environment.** The operator chose
`jsdom` plus `@testing-library/react` as dev-only dependencies over pump-only
tests or a Playwright browser test. The plan applies that answer. The published
tarball and its runtime dependency set are unchanged.

**Q2, answered 2026-10-27. The session acquisition.** The operator chose an
explicit `session` prop over a context provider or a module singleton. The plan
applies that answer. Two viewports get two sessions by construction, and there
is one way to obtain a session.

**Q3, answered 2026-10-27. The checkpoint policy.** The operator chose a
caller-chosen checkpoint over a per-frame snapshot or a command log. The plan
applies that answer. The shell holds the latest snapshot and the slice index,
and no per-frame allocation is added.
