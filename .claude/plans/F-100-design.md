# F-100, API design: viewport, scene and tool surface, documented

**Status**: approved
**Epic ref**: E16.1
**Sprint**: S15
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/02-standing-decisions.md`, section 2.1

```text
The core is designed on its own terms rather than shaped to fit cornerstone's. The cost lands squarely on the existing product: a new API means a migration, and it is a first-class epic (E17, twenty engineer-weeks), not a cleanup task. §12 is the strategy.
```

Decision D1 is a new API. A drop-in shim is the rejected alternative. Names in this story are not cornerstone names.

### `docs/hld/07-concurrency-and-typescript.md`, section 10

```text
Not a compromise — the correct home for this work.
```

```text
DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver
The SVG annotation drawing layer — DOM-native, and faster left there
Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
Framework bindings, DICOMweb fetch and authentication
```

Tool state machines stay in TypeScript. Hit-testing stays in `ocelli-geom`, which this story does not fill. Framework bindings are F-102. The React `OcelliViewport` scaffold stays a scaffold.

### `docs/hld/04-boundary-and-data-path.md`, section 5, the three channels

```text
The shell is TypeScript on the main thread. The core is Rust running in workers. Between them sits a deliberately narrow boundary carrying commands down, bulk bytes down, and events up.
```

```text
A fixed-size ring in linear memory, drained once per frame rather than invoking a JavaScript callback per event.
```

### `docs/hld/14-the-boundary-in-code.md`, section 17, the wire this story documents and does not build

```text
// wire format, little-endian
// [u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
```

```rust
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

```rust
pub struct ViewportState {
    pub camera: [f32; 16],
    pub voi_center: f32,
    pub voi_width: f32,
    pub slice_index: u32,
    pub num_slices: u32,
    pub flags: u32, // bit 0 invert, bit 1 loading, bit 2 error
    pub _pad: [u32; 3],
}
```

F-101 implements `apply_commands`, the ring and the readback copy. This story writes the field list down so F-045's sixteen floats, F-046's centre and width and invert flag, and F-047's index and length are the fields that copy will carry. It does not pack a buffer.

### `docs/hld/01-purpose-and-scope.md`

```text
In scope for Phase 1: feature parity with cornerstone3D v5.8.9 — DICOM ingest and metadata, the pixel pipeline, stack and volume viewports, MPR and 3D rendering, segmentation, and the approximately sixty-three tools — delivered as a Rust core with a TypeScript shell.
```

The sixty-three tools are not enumerated here. F-103 is the event model. This story names the command set the stack viewport accepts.

### Allocation note

```text
allocation.json notes: New API by decision — see HLD §2
```

That note points at section 2.1 above.

## What the specification does not cover

The HLD does not list the method names. The names below are the new API. They match the Rust types in the other four plans. A second set of names is a defect.

1. **`Scene` is an ordered list of viewport ids.** It has no pixels and no GPU handle. Adding a viewport appends an id. It does not create a device.

2. **`StackViewport` is the documented state**, not a class that draws. Its fields are the `View` inputs, the presentation inputs, and the slice index and length. Camera serialisation beyond the sixteen floats already in section 17.4 is F-050.

3. **The command set is the tool surface.** A tool is a name mapped to one command. The commands are `Fit`, `Pan`, `Zoom`, `Rotate`, `FlipHorizontal`, `FlipVertical`, `SetWindow`, `ApplyPreset`, `SetInvert`, `SetColormap`, `Scroll`. No other command is declared. Annotation tools are not in the set. The mapping from a pointer gesture to a command is data, a `ToolBinding`, and this story does not run a gesture.

4. **Types only, in `packages/core/src/viewport.ts`.** They compile. They do not call wasm, do not import `ocelli-wasm`, and do not construct a `Uint8Array` over wasm memory. `coreAvailable` stays as it is. The package exports the types from `index.ts`.

5. **Unavailable is a value, not a throw that becomes a blank success.** `TierAvailability` is `'full' | 'unavailable'` for drawing. Tier C drawing is `'unavailable'`. The documented note says the CPU raster is F-X003.

## Approach

`docs/lld/public-api.md` is the prose. It transcribes section 17's header, the 48-byte stride and `ViewportState`, and then the command table. It states that F-101 is the implementation of the wire and that this sprint does not build it.

`packages/core/src/viewport.ts` exports the types a caller can name without a runtime. A vitest file asserts the command names and that `ViewportState` field order is the section 17.4 order: camera of sixteen numbers, then centre, width, slice index, slice count, flags. The test does not render.

`docs/lld/README.md` gains one index row. That file is also edited by F-045 for `viewport.md`, so the two stories are not concurrent.

No Rust crate is edited. `ocelli-geom` stays a scaffold. `packages/react` stays a scaffold.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: n/a. This story does not run a frame.
- unsafe: none
- Tier A (WebGPU): n/a. Documentation does not draw.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a for code. The documented drawing row says unavailable until F-X003.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | The exported command names are exactly the eleven in this plan, in that order. | `packages/core/src/viewport.test.ts` |
| unit | `ViewportState` documents sixteen camera numbers, then centre, width, slice index, slice count and flags. | same |
| unit | Importing the module does not touch wasm memory. The file does not contain `wasm.memory`. | same, as a source assertion |

## Parity surface covered

Appendix B counts `Tool classes` at about 63 and `Core events` at 50 plus 53 tool events, "re-shaped rather than copied". This story does not implement those classes. It records that they are re-shaped, which is section 2.1, and stops.

## Deviations

none

## LLD impact

- `docs/lld/public-api.md`, created
- `docs/lld/README.md`, one index row
- `docs/lld/typescript-packaging.md`, one sentence that `viewport.ts` is types and not the wasm boundary

## Open questions

None.
