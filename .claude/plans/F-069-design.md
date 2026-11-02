# F-069, pointer, touch and wheel event normalisation and dispatch

**Status**: approved
**Epic ref**: E11.2
**Sprint**: S26
**Estimate**: 3w

## Normative source, transcribed

HLD `docs/hld/03-architecture-and-crates.md` section 3:

```text
The shell is TypeScript on the main thread: DOM and pointer events, the SVG annotation layer, tool interaction state, framework bindings, and DICOMweb fetch and authentication. The core is Rust running in workers. Between them sits a deliberately narrow boundary carrying commands down, bulk bytes down, and events up.
```

HLD `docs/hld/04-boundary-and-data-path.md` section 5.1:

```text
Small, infrequent, one call per user intent: set camera, set VOI, activate tool, load series. Never one call per pointer move. Pointer streams are normalised in TypeScript and batched into a single packed command buffer per animation frame.
```

HLD `docs/hld/23-performance-rules.md`:

```text
Batch pointer events into one command buffer per animation frame. Never cross the boundary per event.
```

HLD `docs/hld/07-concurrency-and-typescript.md` section 10 puts DOM, pointer, touch and wheel events in TypeScript. Tool interaction state machines stay in TypeScript. Geometry and hit-testing stay in `ocelli-geom`.

F-103 names the shell events and names their producer. Pointer names are `pointer.down`, `pointer.up`, `pointer.move`, `pointer.drag`, `pointer.wheel`, `pointer.click` and `pointer.doubleClick`, producer `shell pointer`. Touch names are `touch.start`, `touch.drag`, `touch.end`, `touch.tap` and `touch.swipe`, producer `shell touch`. Delivery is `shell`. This story does not put those events on the ring.

`View::set_pan`, `View::set_zoom` and `View::set_rotation` replace the stored value. `StackSequence::scroll_by` adds. `CommandSession` applies a packed buffer in order and refuses a 65th command (`MAX_COMMANDS` is 64). `packCommands` is the existing packer. `packPointerFrame` packs every sample as its own `Pan` and is not the dispatcher.

The allocation note says this work is TypeScript. The epic layer cell says Rust+TS. The note wins for where the code lives. No Rust file is added.

## What the specification does not cover

The HLD does not give a gesture-to-payload formula. The decisions below are local. They are not a formula the HLD forgot to write.

- One `takeFrame` returns at most one packed buffer. Moves are not one command each. The module does not call `submitCommands`, `writeCommands`, `alloc` or wasm.
- The active tool and its command are captured at `pointerdown`. A later mode change does not retarget that gesture.
- `Pan` is `basis.panX + (x - originX)` and `basis.panY + (y - originY)`. It is not the client position. The last sample in the frame is the one sample. Without a finite basis the frame packs nothing for this command.
- `Zoom` is `basis.zoom * exp(-(y - originY) / 256)`. `Rotate` is `basis.radians + (x - originX) / 256`. The divisor 256 is a local scale. A non-finite result is not packed.
- `Scroll` from a wheel is one command. Pixel mode (`deltaMode` 0 or omitted) contributes the sign of the summed `deltaY`, and a zero sum contributes 0. Line mode (`deltaMode` 1) contributes `trunc(deltaY)` per event, summed. Page mode (`deltaMode` 2) contributes the sign of each non-zero `deltaY`, summed. A drag whose captured command is `Scroll` contributes the sign of that frame's `y` change. The frame sums those contributions and clamps to the i32 range. Zero packs no `Scroll`.
- A wheel packs only when exactly one active tool in the group has command `Scroll`. None is `no-owner`. Two or more is `ambiguous-scroll`.
- `Fit`, `FlipHorizontal`, `FlipVertical`, `SetWindow`, `ApplyPreset`, `SetInvert` and `SetColormap` pack nothing. The reason is `payload-not-in-gesture`. This story does not invent a window formula.
- A second pointer id during an open gesture does not pack. Its events are still reported.
- `pointerType` `touch` uses the touch names. `mouse` and `pen` use the pointer names. The adapter listens to pointer and wheel events only, not to mouse or touch events, because a second listener would dispatch one contact twice.
- A click is a gesture whose net movement is strictly under 3 CSS pixels. A double click is a second click whose down is at most 400 ms after the first up and strictly under 3 pixels from that up. A touch swipe ends a touch gesture whose distance is at least 24 and whose duration is at most 300 ms, and that ending is not also a tap.
- Coordinates are whatever `clientX` and `clientY` the caller supplies. This story does not read a bounding box.
- The viewport must already be attached to the group. An unknown group or a missing viewport packs nothing.

## Problem and reproducer

F-068 stores the binding and does not listen. `packPointerFrame` turns every sample into a `Pan` of that sample's coordinates. Applied in order, the last `set_pan` would store the client position as the camera translation.

The observation is a dispatcher call, not a rendered frame. Register `Pan` active on primary. Ingest two moves in one frame from origin `(100, 200)` to `(110, 250)` with basis pan `(10, 20)`. `takeFrame` returns one `Pan` of `(20, 70)` and one buffer. It does not return two commands and it does not call wasm.

## Scope, API and compatibility

In scope:

- `packages/core/src/pointer.ts`
- `packages/core/src/pointer.test.ts`
- Re-exports from `packages/core/src/index.ts` and the published-name list in `index.test.ts`
- `docs/lld/public-api.md` gains the dispatch section when the story is completed

Out of scope:

- Cursor style and `tool.activated`. That is F-072.
- SVG. That is F-071.
- Editing `events.ts`, `commands.ts`, `tools.ts`, command ids, or the Rust camera.
- A window, preset, invert, colormap, fit or flip payload.
- Hit-testing. That stays in `ocelli-geom`.

`createPointerDispatch(options): PointerDispatch`

`options` is the registry, the group id, the viewport id, an optional target, an optional `basis()` and an optional `now()`. `now` defaults to `Date.now`. `basis` defaults to a function that returns null.

`PointerDispatch`:

| Operation | Result |
|-----------|--------|
| `ingest(event)` | queue one DOM-shaped event |
| `takeFrame()` | the normalised events, the commands, the packed bytes, the captured tool and the reason |
| `attach()` | subscribe when a target was given |
| `detach()` | remove those listeners |

`ingest` does not throw. A non-finite coordinate is dropped. Button 0, 1 and 2 are `primary`, `auxiliary` and `secondary`. Any other button does not start a gesture.

`takeFrame` reasons are `packed`, `empty`, `no-owner`, `payload-not-in-gesture`, `missing-basis`, `non-finite`, `ambiguous-scroll`, `ignored-contact`, `unknown-group` and `missing-viewport`. `empty` means the queue had nothing to report. `ignored-contact` is reported on the frame when a second pointer id arrived and the primary gesture also produced events. The primary gesture's own reason stays on the frame. A second contact alone, with no primary gesture samples in that frame, uses `ignored-contact`.

Packed bytes for a non-empty command list are `packCommands(viewport, commands)`. An empty list is a zero-length buffer and does not call the packer.

The DOM target is a small surface with `addEventListener` and `removeEventListener`. `setPointerCapture` and `releasePointerCapture` are used when present. Capture failure does not refuse the gesture. The listener types are `pointerdown`, `pointermove`, `pointerup`, `pointercancel` and `wheel`. `preventDefault` is not called.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| A later frame pump | `takeFrame` | one buffer, then the pump submits | `Uint8Array` |
| F-103 shell names | `ingest` | touch versus pointer by `pointerType` | named events |
| F-068 registry | `bindingOwner` at down, group read at frame | captured tool, attached viewport | commands or a reason |
| `packCommands` | the existing packer | not `packPointerFrame` | bytes |

## State transitions and invariants

| Start | Input | Result | Preserved |
|-------|-------|--------|-----------|
| idle | primary down on an owner | gesture open, tool captured | registry |
| gesture open | moves | one command on `takeFrame` from the last point | captured tool |
| gesture open | second pointer id | events only for that id | first gesture's command |
| gesture open | up under 3 px | click or tap, no camera command | registry |
| gesture open | up at 3 px or more | drag end, no click | registry |
| after a click | second click within 400 ms and 3 px | `pointer.doubleClick` or `touch` has no double name, so a second touch tap stays `touch.tap` | first tap already emitted |
| gesture open | `pointercancel` | gesture closed, no click, no final command | registry |
| idle | wheel | one `Scroll` or a no-pack reason | registry |

A touch gesture does not emit `pointer.doubleClick`. Two quick taps emit two `touch.tap` events when each qualifies as a tap.

## Ownership, commit and recovery

The dispatcher owns the queue and the open gesture. The registry owns tools. `takeFrame` is the commit point for the returned buffer. It does not submit that buffer. `detach` drops listeners and does not clear a queue already taken. An identical second `takeFrame` with an empty queue returns `empty` and does not repeat the previous commands.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | Two moves, one `Pan` of basis plus delta, one buffer | `one frame packs the last pan` | one `Pan` per move, or client position as pan | implementation |
| AC2 | No basis, events kept, no `Pan` | `a missing basis does not pack a pan` | packing zero | implementation |
| AC3 | Zoom and rotate use the 256 scale and the last sample | `zoom and rotate use the gesture origin` | per-event multiply stored by replacement | implementation |
| AC4 | Non-finite zoom result packs nothing | `a non finite zoom is not packed` | packing `Infinity` | implementation |
| AC5 | Pixel wheel is the sign of the sum, one `Scroll` | `a wheel frame scrolls once` | raw `deltaY` or one command per event | implementation |
| AC6 | Two active `Scroll` tools pack nothing | `two scroll tools are ambiguous` | packing both | implementation |
| AC7 | Line mode sums truncations. Page mode sums signs | `wheel mode changes the step` | treating every mode as pixels | implementation |
| AC8 | Scroll drag uses the sign of the frame dy and sums with a wheel | `a scroll drag and a wheel share one step` | two `Scroll` commands | implementation |
| AC9 | `SetWindow` and `Fit` pack nothing | `a window tool does not invent a payload` | a local window formula | implementation |
| AC10 | Second pointer id does not pack | `a second contact does not pack` | two owners | implementation |
| AC11 | Touch names for touch, pointer names for mouse | `pointer type selects the event family` | both families for one event | implementation |
| AC12 | Distance under 3 is a click. Distance 3 is not | `three pixels is not a click` | `<=` | implementation |
| AC13 | Second click at 400 ms is a double click. 401 is not | `double click uses 400 milliseconds` | a fixed double on every second up | implementation |
| AC14 | Touch distance 24 within 300 ms is a swipe and not a tap | `a swipe is not a tap` | both events | implementation |
| AC15 | Cancel packs nothing and is not a click | `cancel closes the gesture` | a click on cancel | implementation |
| AC16 | Modifier order matches the stored binding | `modifier order still finds the owner` | raw key order | implementation |
| AC17 | Passive tool and unknown group and missing viewport pack nothing | `a refused group packs nothing` | throwing, or packing anyway | implementation |
| AC18 | Button 3 does not start a gesture | `an extra button is not a binding` | treating it as primary | implementation |
| AC19 | Non-finite coordinates are dropped | `a non finite point is dropped` | packing `NaN` | implementation |
| AC20 | Source does not name `submitCommands` or `writeCommands` | `the dispatcher does not cross the boundary` | a wasm call per frame | implementation |
| AC21 | `detach` stops delivery | `detach removes the listener` | a listener left in place | implementation |
| AC22 | Empty `takeFrame` does not repeat commands | `an empty frame stays empty` | replaying the previous buffer | implementation |

## Approach

`packages/core/src/pointer.ts` owns the queue, the gesture and the frame reducer. It imports `packCommands` and the F-068 registry types. It does not import the cursor module. The DOM adapter is optional and thin.

Rejected: calling `packPointerFrame`, submitting inside `ingest`, and reading the 16-number camera to recover pan.

## Dependencies, write ownership and resources

Depends on F-068, which is done. Serial in the canonical worktree. This story writes `pointer.ts`, `pointer.test.ts`, `index.ts` and `index.test.ts`. It does not write `tools.ts`, `cursor.ts`, shaders, or the GPU. F-072 later edits `index.ts` and the ledgers, so it waits until this story is committed.

## Verification and evidence reuse

`npm test` and `npm run typecheck` and `npm run lint` in `packages/core`. No wasm rebuild is required because this story does not change Rust or the wasm exports. The feature profile is still the later `/complete-feature` gate. A change to `pointer.ts` after that gate invalidates it.

No pixel fixture. No geometry fixture. The numbers above are local shell scales, not DICOM samples.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none in this module. The returned buffer is one array per `takeFrame`, outside the wasm render loop
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `packages/core/src/pointer.test.ts` |
| fixture | none. No pixel arithmetic | |

## Parity surface covered

None of the tool-class rows. The shell pointer and shell touch producers for the F-103 names are this module. The catalogue file is not edited. The sixty-three tool classes stay unimplemented.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` gains a dispatch section. `docs/lld/README.md` does not gain a new file.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The gaps above are decided in this plan.
