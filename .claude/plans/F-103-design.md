# F-103, Event model, the cornerstone events re-shaped and documented

**Status**: approved
**Epic ref**: E16.4
**Sprint**: S17
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/B-parity-surface.md`, Appendix B, gives the row this story owns:

```text
| Core events | 50 | Plus 53 tool events; re-shaped rather than copied |
```

`docs/hld/11-decision-log.md`, decision D4, quoted in `CLAUDE.md`:

```text
| D4 | Events polled from a ring per frame, never a JS callback per event. |
```

`docs/hld/04-boundary-and-data-path.md`, section 5.3, gives the ring:

```text
A fixed-size ring in linear memory, drained once per frame rather than invoking a JavaScript callback per event. This removes a boundary crossing from the hot path and gives coalescing for free: a burst of camera changes during a drag collapses to one delivered event. Viewport state reads back as a small C-layout struct copy, not a JavaScript object graph.
```

`docs/hld/14-the-boundary-in-code.md`, section 17.3, gives the envelope:

```rust
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

```text
*Fixed stride so the JavaScript side is arithmetic, not deserialisation. Drain once per animation frame; surface `dropped` in telemetry rather than swallowing it.*
```

`docs/hld/07-concurrency-and-typescript.md`, section 10, keeps the DOM and the
tool state machines in the shell:

```text
- DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver

- The SVG annotation drawing layer — DOM-native, and faster left there

- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

`docs/hld/07-concurrency-and-typescript.md`, section 10, gives the size of the
surface this catalogue replaces:

```text
Cornerstone has 531 addEventListener sites, 137 uses of document, and 72 createElement calls, and pointer handling is the one workload where crossing into WebAssembly makes things measurably worse.
```

## What the specification does not cover

The HLD gives the ring envelope and the Appendix B counts. It does not give
Ocelli's event names, the delivery classes, the mapping table, the payload
decoders or the completeness rule.

Every item below is this plan's decision.

- **The catalogue.** Ocelli's own names, grouped by domain, with a delivery
  class of `ring`, `shell` or `unavailable`.
- **The mapping.** One row per cornerstone3D 5.8.2 event, 53 core and 53 tool,
  each with an Ocelli name or `null`, a delivery class and a reason when the
  event is unavailable.
- **The measured count.** The installed 5.8.2 enums carry 53 core events and 53
  tool events, so 106, not the Appendix B figure of 50 plus 53. Appendix B's
  count is left as the author measured it, and this story records the 5.8.2
  count beside it. D-11 already says the Appendix B counts are re-checked
  against 5.8.2 by `/parity`. This is a parity finding, not a deviation.
- **The payload decoders.** Six ring kinds, each with a typed payload. The
  layouts are already fixed by `ocelli-cache`'s encoders and `ocelli-viewport`'s
  `push_presented`.
- **The refusal.** `decodeNamedEvent` returns `null` for a kind the catalogue
  does not name, rather than guessing a payload shape.
- **The check.** A vitest test proves the internal agreement. A Python check
  proves the external agreement against the pinned 5.8.2 enums when the oracle
  stack is installed.

## Problem and reproducer

The ring delivers numeric kinds. `packages/core/src/ring.ts` returns
`OcelliEvent` with a `kind: number` and a 32-byte `payload: Uint8Array`. A
consumer has no name, no typed payload and no way to know which kinds the ring
can produce. Cornerstone's catalogue is 106 names spread over two packages, and
nothing in this repository says which of them Ocelli delivers, which belong to
the shell, and which do not exist yet.

The reproducer is a consumer that switches on `event.kind === 6` and reads
`payload[0..4]` as a generation. That is a magic number and a hand-decoded
payload. A second consumer that switches on the cornerstone name
`CORNERSTONE_IMAGE_RENDERED` is a second catalogue, and it disagrees with the
ring, which names the event `frame.presented`. That is the sprint's named
defect.

Source identity: `packages/core/src/ring.ts`,
`crates/ocelli-viewport/src/ring.rs`,
`crates/ocelli-cache/src/image.rs` and `crates/ocelli-cache/src/telemetry.rs`.

## Scope, API and compatibility

In scope:

- `packages/core/src/events.ts`, the catalogue, the delivery classes, the
  named-event union and the payload decoders.
- The 106-row cornerstone mapping, committed as a const table.
- `scripts/event_catalogue_check.py`, the external comparison against the
  pinned 5.8.2 enums.
- `docs/lld/events.md`, the catalogue and the mapping.
- Re-exports from `packages/core/src/index.ts`.

Non-goals:

- No second ring, no second producer and no `addEventListener` per event.
- No per-event callback. F-102 drains the ring once per frame.
- No tool implementations. The shell events are declared with their delivery
  class and a named producer or a deferred reason.
- No change to the ring's 48-byte stride, its 256-event capacity or its kinds.
- No change to the cornerstone pin. The mapping is derived from 5.8.2.

Public API:

```ts
// packages/core/src/events.ts
export const EVENT_DELIVERY = ["ring", "shell", "unavailable"] as const;
export type EventDelivery = (typeof EVENT_DELIVERY)[number];

export const OCELLI_EVENTS = [
  "cache.evicted",
  "cache.displaced",
  "cache.refused",
  "cache.pressure",
  "cache.budgetChanged",
  "frame.presented",
  "viewport.mounted",
  "viewport.unmounted",
  "viewport.resized",
  "pointer.down",
  "pointer.up",
  "pointer.move",
  "pointer.drag",
  "pointer.wheel",
  "pointer.click",
  "pointer.doubleClick",
  "key.down",
  "key.up",
  "touch.start",
  "touch.drag",
  "touch.end",
  "touch.tap",
  "touch.swipe",
  "tool.activated",
  "tool.modeChanged",
  "annotation.added",
  "annotation.completed",
  "annotation.modified",
  "annotation.removed",
  "annotation.selectionChanged",
  "annotation.locked",
  "annotation.visibilityChanged",
  "annotation.rendered",
] as const;
export type OcelliEventName = (typeof OCELLI_EVENTS)[number];

export interface CacheKeyPayload {
  readonly series: bigint;
  readonly instance: number;
  readonly frame: number;
  readonly bytes: bigint;
}

export interface CachePressurePayload {
  readonly tier: 0 | 1 | 2;
  readonly used: bigint;
  readonly budget: bigint;
  readonly entries: number;
  readonly evicted: number;
  readonly refused: number;
}

export interface CacheBudgetChangedPayload {
  readonly tier: 0 | 1 | 2;
  readonly budget: bigint;
  readonly evictedBytes: bigint;
  readonly evicted: number;
  readonly dropped: number;
}

export interface FramePresentedPayload {
  readonly generation: number;
  readonly drawnWidth: number;
  readonly drawnHeight: number;
  readonly flags: number;
}

export type OcelliNamedEvent =
  | { readonly name: "cache.evicted"; readonly viewport: number;
      readonly seq: bigint; readonly payload: CacheKeyPayload }
  | { readonly name: "cache.displaced"; readonly viewport: number;
      readonly seq: bigint; readonly payload: CacheKeyPayload }
  | { readonly name: "cache.refused"; readonly viewport: number;
      readonly seq: bigint; readonly payload: CacheKeyPayload }
  | { readonly name: "cache.pressure"; readonly viewport: number;
      readonly seq: bigint; readonly payload: CachePressurePayload }
  | { readonly name: "cache.budgetChanged"; readonly viewport: number;
      readonly seq: bigint; readonly payload: CacheBudgetChangedPayload }
  | { readonly name: "frame.presented"; readonly viewport: number;
      readonly seq: bigint; readonly payload: FramePresentedPayload };

export function decodeNamedEvent(event: OcelliEvent): OcelliNamedEvent | null;

export interface CornerstoneEventRow {
  readonly cornerstone: string;
  readonly source: "core" | "tool";
  readonly ocelli: OcelliEventName | null;
  readonly delivery: EventDelivery;
  readonly producer: string | null;
  readonly reason: string | null;
}

export const CORNERSTONE_EVENT_MAP: readonly CornerstoneEventRow[];
export const CORNERSTONE_EVENT_COUNT = 106;
```

The shell events are declared with a producer or a deferred reason. The ring
events are the six the producer emits today. The `unavailable` rows name the
cornerstone event and the reason, which is either "no Ocelli surface yet" or the
story that will own it.

Compatibility: `@ocelli/core` keeps every existing export. The new names are
additive. `OcelliEvent` from `ring.ts` stays the wire type, and
`OcelliNamedEvent` is the decoded view of it.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `decodeNamedEvent` | The six ring kinds and their payload layouts | Shell, after `drainRing` | A typed event or `null` |
| `CORNERSTONE_EVENT_MAP` | The committed 5.8.2 mapping | Documentation and the external check | One row per cornerstone event |
| F-102's pump | `decodeNamedEvent` | One decode per drained event | `OcelliNamedEvent[]` |
| `scripts/event_catalogue_check.py` | `tools/oracle/node_modules` and the pinned versions | The `oracle` gate, local `/verify` | A pass or a named mismatch |
| `docs/lld/events.md` | The table | A reader | The mapping and the delivery classes |

Unchanged consumers affected by changed providers: `ring.ts` keeps its
`OcelliEvent` type and its 48-byte stride. F-102 keeps its drain-per-frame
policy.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| A drained ring | `decodeNamedEvent` with kind 1, 2 or 3 | A cache key and byte payload | `null` for an unknown kind, no guessed payload |
| A drained ring | Kind 4 | A pressure payload | `null` if the payload is short |
| A drained ring | Kind 5 | A budget-changed payload | `null` if the payload is short |
| A drained ring | Kind 6 | A presented-frame payload | `null` if the payload is short |
| A drained ring | An unknown kind | `null` | The event is not named and not delivered as a typed event |
| The catalogue | A cornerstone event with no row | The external check fails | The catalogue is incomplete |
| The catalogue | A ring kind with no row | The internal test fails | The catalogue disagrees with the producer |
| The catalogue | A shell event with no producer and no reason | The internal test fails | A declared event with no owner |

Invariants that span consumers:

- **One catalogue.** No module outside `events.ts` declares an Ocelli event
  name.
- **The ring is the only core delivery.** A `ring` row names a kind the
  producer emits. A `shell` row never crosses the boundary.
- **No per-event callback.** The catalogue describes what a drained batch
  contains. It does not add a listener.
- **The count is measured.** 53 core plus 53 tool from the installed 5.8.2
  enums. Appendix B's 50 plus 53 is recorded beside it.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| The catalogue | `events.ts` | The tree hash | A passing internal test | A missing row fails the test |
| The mapping | `CORNERSTONE_EVENT_MAP` | The pinned 5.8.2 version | A passing external check | A missing or extra row fails the check |
| A decoded event | `decodeNamedEvent` | The copied `OcelliEvent` | The returned value | An unknown kind returns `null`, never a guessed shape |

No side effect precedes the commit point. The catalogue is data. An identical
repeated decode is the same value.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The mapping has exactly 106 rows, 53 `core` and 53 `tool`, one per 5.8.2 event, each with a delivery class and an Ocelli name or a reason | `packages/core/src/events.test.ts`, `mapping_is_complete` | A catalogue that covers only the ring kinds and calls itself complete | implementation |
| AC2 | Every Ocelli name is unique and appears in `OCELLI_EVENTS`, and every `OCELLI_EVENTS` entry has a mapping row, a declared shell producer or a deferred reason | Same file, `names_are_unique` | Two names for one event, or a name nothing delivers and nobody owns | implementation |
| AC3 | Every ring kind 1 to 6 has exactly one `ring` row whose name matches the generated `RING_EVENT_KIND`, and no other row claims `ring` | Same file, `ring_kinds_match_generated` | A catalogue that names a kind the producer does not emit, or misses one | implementation |
| AC4 | `decodeNamedEvent` decodes each of the six kinds to the documented fields and returns `null` for an unknown kind | Same file, `decodes_every_kind` and `refuses_unknown_kind` | A decoder that reads a magic offset or guesses a payload | implementation |
| AC5 | A hand-computed payload for series 7, instance 3, frame 5 and 4096 bytes decodes to `{ series: 7n, instance: 3, frame: 5, bytes: 4096n }`, and a pressure payload decodes tier, used, budget, entries, evicted and refused at their documented offsets | Same file, `decodes_fixture_payloads`, citing `ocelli-cache`'s encoder comments and HLD section 17.3 | A decoder that reads the key at the wrong offset, which names the wrong frame | implementation |
| AC6 | `scripts/event_catalogue_check.py --require-cornerstone` re-derives the 106 names from the pinned 5.8.2 enums and asserts set equality with the mapping. A missing or extra name fails with the name printed | The `oracle` gate's arm, after `tools/oracle/node_modules` is installed | A mapping that was hand-typed and drifted from 5.8.2 | integrated, needs the oracle stack |
| AC7 | A cornerstone event with no mapping row fails AC6, and a mapping row with no cornerstone event fails AC6 | The same check | A completeness claim that only counts rows | integrated, needs the oracle stack |
| AC8 | No module outside `events.ts` declares an Ocelli event name, and no module adds an `addEventListener` for a ring kind | A review of `packages/core/src` and `packages/react/src`, plus the internal test's import graph | A second catalogue, or a per-event callback that bypasses the ring | implementation |
| AC9 | The mapping records the measured 53 plus 53 beside Appendix B's 50 plus 53, and the parity note names the difference | `docs/lld/events.md` and the mapping header | Silently rewriting the Appendix B count, or claiming 103 without a measurement | implementation |
| AC10 | The catalogue does not change the ring's stride, capacity or kinds | `crates/ocelli-viewport/tests/ring.rs` stays green | A catalogue that renumbers a kind to fit a name | implementation |
| AC11 | Tier rows: the catalogue is tier-independent and performs no rendering | This section | A catalogue that assumes a GPU tier | implementation |

## Approach

The smallest implementation in terms of the transcribed source.

1. **`packages/core/src/events.ts`.** The constants, the union, the six
   decoders and the mapping table. The decoders use `DataView` over the copied
   payload, exactly as `readViewportState` does. `series` and `bytes` are
   `bigint` because the wire is `u64` and JavaScript numbers lose precision
   above 2^53. The tier byte is `0` encoded, `1` decoded, `2` GPU, which is
   `ocelli-cache`'s `tier_code`.
2. **The mapping.** Every 5.8.2 event, from
   `tools/oracle/node_modules/@cornerstonejs/core/dist/esm/enums/Events.js` and
   the tools package's same file. The table is committed, one row per event,
   with `source`, `ocelli`, `delivery`, `producer` and `reason`. The header
   records the package versions and the two file digests.
3. **`scripts/event_catalogue_check.py`.** It reads the mapping with the same
   narrow-regex style `scripts/error_code_check.py` uses for `errors.ts`. With
   `--require-cornerstone` it reads the two installed `Events.js` files,
   extracts the enum keys, and asserts set equality. Without the flag it
   asserts the internal counts and the pinned version. A missing file under
   `--require-cornerstone` is a refusal, never a skip.
4. **`bin/ocelli.sh`.** The `oracle` gate's arm becomes
   `"$0" oracle && python3 scripts/event_catalogue_check.py --require-cornerstone && "$0" compare`.
   The oracle arm already requires `tools/oracle/node_modules`, so the check
   cannot run without the pinned files.
5. **`packages/core/src/events.test.ts`.** The internal agreement, the
   decoders and the fixture payloads. The ring kinds come from F-104's
   generated module.
6. **`packages/core/src/index.ts`.** Re-export the catalogue and the decoders.
7. **`docs/lld/events.md`.** The catalogue, the delivery classes, the mapping
   and the measured-count note.

Rejected alternatives:

- **Copying the cornerstone names as Ocelli's names.** Appendix B says
  "re-shaped rather than copied", and a copied name carries cornerstone's
  vocabulary into a new API.
- **A per-event callback registered by the catalogue.** Decision D4 forbids
  it.
- **A second ring for shell events.** The shell events do not cross the
  boundary, so they need no ring.
- **A registry of event kinds.** F-032 and F-034 already decided against an
  event-kind registry this phase. The catalogue is a table, not a registry.
- **Generating the mapping from node_modules at build time.** CI has no oracle
  install, and a generated file would hide the provenance. The committed table
  records the pin.

## Dependencies, write ownership and resources

Declared dependencies: F-101, done. Source-level dependencies: F-104 for the
generated ring kinds, F-102 for the drain policy. This story lands after
F-104 and F-050, and before or beside F-102. It does not depend on F-050.

Files that cannot have concurrent writers:

- `packages/core/src/{events.ts,events.test.ts,index.ts}`
- `scripts/event_catalogue_check.py`
- `bin/ocelli.sh` (the `oracle` arm)
- `docs/lld/{events.md,public-api.md}`

The oracle stack is an exclusive resource. The external check runs with the
`oracle` gate and is serial.

## Verification and evidence reuse

Focused checks:

```bash
npx vitest run packages/core/src/events.test.ts
python3 scripts/event_catalogue_check.py
(cd tools/oracle && npm ci)
bin/ocelli.sh gate oracle
npm run lint
npm run typecheck
```

Final profile: `bin/ocelli.sh gate --sprint` over the staged union, with the
corpus result recorded. The `oracle` gate runs the external comparison.

Evidence identities: the internal test names the generated ring kinds and the
mapping. The external check names the two 5.8.2 enum files and their digests.
The fixture payloads cite `ocelli-cache`'s encoder comments and HLD section
17.3.

What invalidates evidence: any edit to the catalogue, the mapping, the
generated module or the oracle pin after the verification. A cornerstone
version move invalidates the external comparison and the `oracle` gate refuses
it.

## Boundary and tier

- wasm-bindgen: not touched. The catalogue is TypeScript over copied bytes.
- Pixels across the boundary: no. Payloads are state and telemetry.
- Render-loop allocation: `decodeNamedEvent` allocates one object per event.
  The pump already copies the event payload out of the ring, so the decode adds
  one small object per delivered event, not per frame per viewport. The event
  count is bounded by the ring's 256 slots.
- unsafe: none.
- Tier A (WebGPU): n/a, the catalogue is tier-independent.
- Tier B (WebGL2): n/a.
- Tier C (CPU): n/a.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | The mapping is complete, names are unique, ring kinds match the generated module, and each decoder refuses an unknown kind | `packages/core/src/events.test.ts` |
| `fixture` | Hand-computed payload bytes decode to the documented values, citing the encoder comments and HLD section 17.3 | Same file |
| `conformance` | The mapping equals the pinned 5.8.2 enums, name for name | `scripts/event_catalogue_check.py --require-cornerstone` |
| `browser` | Not applicable. The catalogue is data and the decoders are pure | This table |

`fixture` is mandatory because the decoders read byte offsets that decide which
frame an eviction names. A wrong offset names the wrong frame, which is a
quietly wrong observation rather than a crash.

## Parity surface covered

Appendix B row `Core events`, `50`, plus 53 tool events, re-shaped rather than
copied. This story establishes the mapping and the delivery classification.
The measured 5.8.2 count is 53 core and 53 tool, and the difference from
Appendix B's 50 is recorded for `/parity`. The story does not claim that every
cornerstone event has an Ocelli producer, and the `unavailable` rows say so.

## Deviations

None. Appendix B's count is left as written and the measured count is recorded
beside it, which is D-11's own instruction.

## LLD impact

- `docs/lld/events.md`, a new file: the catalogue, the mapping, the delivery
  classes and the measured count.
- `docs/lld/public-api.md`: the new exports and a pointer to `events.md`.
- `docs/lld/typescript-packaging.md`: the new module in the tarball.

## Open questions

**Q1, answered 2026-10-27. The catalogue's scope.** The operator chose the
delivered-events-plus-unavailable-rows scope over naming all 106 events or
ring-events only. The plan applies that answer. A name in `OCELLI_EVENTS` has a
producer or a declared shell reason, and every remaining cornerstone event is
an `unavailable` row with a reason and a future owner.

**Q2, answered 2026-10-27. The external comparison's home.** The operator chose
the `oracle` gate's arm over a new floor gate or a static snapshot only. The
plan applies that answer. The external comparison runs locally in `/verify`
where the pinned package files exist, and the floor's vitest test proves the
ring agreement without them.

**Q3, answered 2026-10-27. The naming scheme.** The operator chose
`domain.action` lower camel over cornerstone-derived names or SCREAMING_SNAKE
names. The plan applies that answer. Ocelli's catalogue is its own vocabulary,
grouped by domain, and cannot be mistaken for a cornerstone name.
