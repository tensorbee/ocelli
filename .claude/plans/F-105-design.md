# F-105, integration seam

**Status**: approved
**Epic ref**: E17.1
**Sprint**: S37
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/09-migration.md` section 12 says a new API plus an existing
product means the migration is engineering work. The mitigation is to keep
the integration seam identical even though the API is not. The new library
enables on a plain DOM element and dispatches events on it, exactly as
cornerstone does. That constraint is what makes incremental replacement
possible.

The same section says both libraries coexist, with a feature flag per
viewport. Strangler by viewport type puts stack viewports first, then MPR,
then 3D. Shadow mode runs before cutover. Codemods cover mechanical
call-site changes. Genuinely different concepts are documented rather than
auto-migrated. The flag, the strangler and the shadow mode are later
stories. This story is the element and the events.

Decision D1 is feature parity with a new API. The rejected alternative is
drop-in compatibility and an adapter shim.

Decision D4 is events polled from a ring, per frame. The rejected
alternative is a JS callback per event.

`docs/hld/14-the-boundary-in-code.md` section 17.3 fixes the event stride
at 48 bytes. The JavaScript side drains once per animation frame and
surfaces `dropped` rather than swallowing it.

F-103 already names the catalogue in `OCELLI_EVENTS` and the rows in
`CORNERSTONE_EVENT_MAP`. `ELEMENT_ENABLED` maps to `viewport.mounted`.
`ELEMENT_DISABLED` maps to `viewport.unmounted`. Both rows say delivery
`shell`. This story does not add a name and does not change a row.

## What the specification does not cover

Section 12 does not name a function, a return value, a second-enable
rule, or the DOM event constructor. It does not say whether delivery
bubbles.

This plan decides those gaps. They are not HLD formulas.

`enableElement` accepts one object that has a `dispatchEvent` function.
A second call with the same object returns null and does not replace the
first registration. `disableElement` removes that registration and then
dispatches `viewport.unmounted`. `dispatchOnElement` delivers one named
event on an enabled element. The event type is the Ocelli name. The
detail is the event object. `bubbles` is false and `cancelable` is false.

## Problem and reproducer

`packages/react/src/OcelliViewport.tsx` renders a div and says the element
is the seam. `packages/core/src/events.ts` says the module does not
register a listener. Nothing calls `dispatchEvent` on that div. A listener
on the element hears nothing.

The expected observation is a listener on the enabled element receiving
`viewport.mounted`, then each later named event, then `viewport.unmounted`
after disable.

## Scope, API and compatibility

In scope:

- `enableElement`, `disableElement` and `dispatchOnElement` in
  `packages/core/src/seam.ts`.
- The React host div enables on commit and disables on cleanup. Drained
  named events are also dispatched on that div. The existing `onEvents`
  callback stays.

Out of scope:

- A cornerstone-named function or a cornerstone event type string.
- A feature flag, a second library in one page, and shadow rendering.
  Those are F-106, F-107 and F-109.
- Editing `events.ts`, the ring, or the pump's drain rule.
- Measurement arithmetic and DICOM parsing.

```ts
export interface ElementSeam {
  readonly dispatch: (event: SeamEvent) => boolean;
  readonly disable: () => boolean;
}

export type SeamEvent =
  | OcelliNamedEvent
  | { readonly name: OcelliEventName };

export function enableElement(element: EventTarget): ElementSeam | null;
export function disableElement(element: EventTarget): boolean;
export function dispatchOnElement(element: EventTarget, event: SeamEvent): boolean;
```

A value that is null, or that has no function `dispatchEvent`, returns
null from `enableElement`. The registration is a `WeakMap` keyed by the
object. The caller array of listeners is not copied.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `enableElement` | `WeakMap` plus `dispatchEvent` | One object, one registration | Seam or null |
| `dispatchOnElement` | The registration from enable | Name must be in `OCELLI_EVENTS` | True only when the element receives the event |
| `OcelliViewport` | `enableElement` on the host div | Effect after commit | `viewport.mounted` on the div |
| Pump `onEvents` | Unchanged callback | Still one drain per frame | Callback and element both see the drained events |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Absent | `enableElement` on a fresh object | Registered, `viewport.mounted` delivered, seam returned | A throw from that dispatch removes the registration and returns null |
| Registered | `enableElement` again | Null, no second mounted event | The first seam still dispatches |
| Registered | `dispatchOnElement` with a catalogue name | One `CustomEvent` on the object, true | An unknown name returns false and calls nothing |
| Registered | `disableElement` | Registration removed, then `viewport.unmounted` | A missing registration returns false and dispatches nothing |
| Removed | `dispatchOnElement` | False | No event |

Disable removes the registration before the unmounted event. A throw from
that notification does not put the registration back. The next enable can
succeed.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Element registration | `enableElement` | The object identity in the `WeakMap` | Return of the seam after `viewport.mounted` | A second enable does not replace it |
| DOM notification | `dispatchEvent` | The event type string | The listener runs | A throw on a later dispatch leaves the registration in place and returns false |

Repeated `disableElement` on an already removed object returns false. It
does not dispatch a second unmounted event.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Enable a fake element and dispatch `frame.presented` with generation 4 | The listener type is `frame.presented` and the detail generation is 4 | Dispatching `IMAGE_RENDERED`, or dropping the payload | `seam.test.ts` |
| AC2 | Enable the same object twice | The second call is null and the listener saw mounted once | Replacing the first registration | `seam.test.ts` |
| AC3 | Disable, then dispatch | Dispatch returns false and the listener count stays at the unmounted event | Leaving the registration active | `seam.test.ts` |
| AC4 | Dispatch the name `not.an.event` | Returns false and `dispatchEvent` is not called | Accepting a private name | `seam.test.ts` |
| AC5 | Render `OcelliViewport` and unmount it | The div hears `viewport.mounted` and then `viewport.unmounted` | A React callback with no DOM event | `OcelliViewport.test.tsx` |

## Approach

One module. `enableElement` checks `dispatchEvent`, refuses a second
registration, stores the object, and dispatches `viewport.mounted`.
`dispatchOnElement` refuses an unknown name before it constructs the
event. The event type is `event.name`. The detail is the same object the
caller passed. The React component holds a ref on the host div, enables
in an effect, and disables in cleanup. The pump callback remains. The
effect also wraps `onEvents` so each drained named event is dispatched
on the div.

Rejected alternatives: a cornerstone `enable` alias, a module-level
singleton element, and a listener registered inside `events.ts`.

## Dependencies, write ownership and resources

Depends on F-100, which is done. F-103's catalogue is read and not edited.
Serial with F-108 because both touch `packages/core/src/index.ts` and the
delivery ledgers. This story lands first. No GPU. No corpus row.

## Verification and evidence reuse

`npx tsc -p packages/core --noEmit --pretty false` without
`--incremental false`. `npm test` for `seam.test.ts`, `index.test.ts`
and `OcelliViewport.test.tsx`. Feature profile is `bin/ocelli.sh gate
--floor` plus the corpus gate. No comparison report on the feature
commit. A later sprint profile records comparison.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The registration is created at enable, not per event object beyond the one `CustomEvent` the DOM requires for that delivery
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The functions do not open a device and do not read a pixel.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| Unit | AC1 to AC4 | `packages/core/src/seam.test.ts` |
| Component | AC5 | `packages/react/src/OcelliViewport.test.tsx` |
| Published names | The three functions are on the package object | `packages/core/src/index.test.ts` |

No pixel fixture. There is no pixel arithmetic.

## Parity surface covered

none. The seam is the migration constraint in section 12. It does not
claim a rendered viewport matches cornerstone.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`.

## Open questions

none. The operator declined a design-question round. The function names,
the second-enable refusal, and `bubbles: false` are the local decisions
in this plan.

dwv, Horos, and the medical viewer Grok were not opened.
