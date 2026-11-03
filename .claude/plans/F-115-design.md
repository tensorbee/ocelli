# F-115, keyboard actions

**Status**: approved
**Epic ref**: E19.2
**Sprint**: S41
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/04-boundary-and-data-path.md` says pointer events are batched into one command buffer per animation frame and never cross the boundary per event. `docs/hld/23-performance-rules.md` section 26 repeats that rule.

`packages/core/src/events.ts` already names `key.down` and `key.up` as shell events. The HLD does not list the keys.

F-072 owns the pointer dispatch. `set_pan`, `set_zoom` and `set_rotation` replace. This story does not call them.

## What the specification does not cover

The HLD does not name a key map. It does not say which key pans, which key zooms, or which key changes slice.

That gap is a local decision. It is not an HLD formula and it is not a cornerstone formula.

`keyAction(key, shiftKey)` returns one of eight names, or null.

- `ArrowLeft` pans left. Shift does not change it.
- `ArrowRight` pans right. Shift does not change it.
- `ArrowUp` pans up. With shift it scrolls to the previous slice.
- `ArrowDown` pans down. With shift it scrolls to the next slice.
- `+` and `=` zoom in.
- `-` zooms out.
- Any other key returns null, including a lowercase `arrowleft`.

The match is case-sensitive. The function does not listen, does not read the DOM and does not move a camera.

## Problem and reproducer

The shell names `key.down` and has no function that turns a key into a tool action. A caller cannot tell a pan key from an unbound key.

## Scope, API and compatibility

In scope:

- `keyAction` in `packages/core/src/keys.ts`
- Tests and the published name
- The public-api row

Out of scope:

- `addEventListener`
- Calling `createPointerDispatch`
- Changing pan, zoom or stack arithmetic
- Certifying a browser row
- A focus or ARIA attribute writer

The returned names are `pan-left`, `pan-right`, `pan-up`, `pan-down`, `zoom-in`, `zoom-out`, `scroll-previous` and `scroll-next`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A shell key handler the caller writes | `keyAction` | DOM `key` and `shiftKey` | One action name, or null |
| Pointer dispatch | unchanged | Still owns the camera write | Unchanged |
| Event catalogue | unchanged | `key.down` stays a shell event | Unchanged |

## State transitions and invariants

Stateless. The same key and shift flag return the same action. Null writes nothing.

## Ownership, commit and recovery

Not applicable. No listener is registered and no camera is stored.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Arrow keys without shift return the four pan names | unit | Swapping left and right | implementation |
| AC2 | Shift with ArrowUp or ArrowDown returns the scroll names | unit | Still panning when shift is set | implementation |
| AC3 | Shift with ArrowLeft or ArrowRight stays a pan | unit | Treating every shifted arrow as a scroll | implementation |
| AC4 | `+` and `=` zoom in, and `-` zooms out | unit | Requiring shift for `+` | implementation |
| AC5 | An unknown or lowercase arrow name returns null | unit | Case-folding the key | implementation |
| AC6 | The module source does not call `addEventListener` | unit | Registering a listener inside the helper | implementation |
| AC7 | The published name is exported | `index.test.ts` | Leaving the function private | implementation |

## Approach

One new module. No wasm-bindgen. No pixels. No `unsafe`. No DOM.

## Tiers

| Tier | Behaviour |
|------|-----------|
| A | Not applicable. The function does not render. |
| B | Not applicable. The same name is returned. |
| C | Not applicable. The same name is returned. |

The action does not depend on the resolved tier. Applying it on a viewport that cannot pan remains the caller's existing tool refusal.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the key map and the nulls | `keys.test.ts` |
| source | no `addEventListener` call | the same |
| export | published name | `index.test.ts` |

No pixel fixture. No browser probe.

## Parity surface covered

None. The key names are a local map. They do not establish a rendered observation.

## Deviations

None.

## LLD impact

`docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`.

## Open questions

None. The key map above is the approved local decision.
