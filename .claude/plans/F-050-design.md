# F-050, Camera, display-area and presentation-state serialisation

**Status**: approved
**Epic ref**: E7.6
**Sprint**: S17
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/03-architecture-and-crates.md`, section 4, gives this crate its row:

```text
| ocelli-viewport | Viewport and scene model, camera, presentation state | yes | yes |
```

`docs/hld/04-boundary-and-data-path.md`, section 5.3, gives the readback
contract:

```text
A fixed-size ring in linear memory, drained once per frame rather than invoking a JavaScript callback per event. This removes a boundary crossing from the hot path and gives coalescing for free: a burst of camera changes during a drag collapses to one delivered event. Viewport state reads back as a small C-layout struct copy, not a JavaScript object graph.
```

`docs/hld/14-the-boundary-in-code.md`, section 17.4, gives the struct and the
rule beside it:

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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

```text
Copied into a JS-owned buffer on request. Never returned as a JS object graph — that is one allocation and one conversion per field.
```

`docs/hld/26-differentiating-capabilities.md`, section 37, names the
presentation state as already serialisable:

```text
- A **CRDT over the presentation-state struct and the annotation set**. Both are already serialisable structs on the Rust side (§17.4), so this is a sync layer over data that exists rather than a new subsystem.
```

`docs/hld/15-lut-chain.md`, section 18, gives the stage table a round trip must
preserve:

```text
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2\. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3\. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
```

DICOM PS3.3 C.11.2.1.2.2 permits a VOI LUT Sequence and a window pair as
alternative views, with one applied at a time. `docs/lld/viewport.md` records
the project policy: the constructor selects the sequence when present, a manual
window on a VOI LUT sequence is refused, and a preset on a modality LUT
sequence is refused. F-050 carries that selected chain through the round trip
rather than collapsing it to a window.

DICOM PS3.3 C.11.6 gives Presentation LUT Shape `(2050,0020)` as `IDENTITY` or
`INVERSE`. `LutChain::new` resolves it once, and `MONOCHROME1` supplies the
default only when no shape is declared. F-050 carries the declared evidence,
not the resolved boolean alone, because `Absent` on `MONOCHROME1` and
`Shape(INVERSE)` are different states that produce the same flag bit.

`docs/hld/19-render-graph.md`, section 22, gives the dirty rule a restore must
respect:

```text
- **Dirty tracking.** A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.
```

## What the specification does not cover

The HLD gives the 96-byte readback and calls the presentation state
serialisable. It does not give a serialised shape, a version, a restore
procedure, a refusal set, or a storage format.

Every item below is this plan's decision.

- **The format.** A versioned UTF-8 JSON object owned by `ocelli-viewport`,
  encoded with `serde` and `serde_json` at their `alloc` feature set. The
  operator chose JSON on 2026-10-27 over a packed binary record, for
  readability and for a record that a human can inspect and diff.
- **The dependencies.** `serde` and `serde_json` are already workspace
  dependencies, added for the oracle and used by `ocelli-dicom`. They are not
  in HLD section 15.2's dependency list, so using them in a shipped core crate
  is a departure and needs a deviation row. The workspace entries change to
  `default-features = false` with `alloc` so `ocelli-viewport` keeps its
  `no_std` posture. Measured: `python3 scripts/no_std_check.py` stays green,
  `cargo check -p ocelli-dicom --all-targets` stays green, and
  `cargo check -p ocelli-viewport --target wasm32-unknown-unknown` stays green.
- **The size.** `serde_json` enters the shipped wasm module. A faithful nested
  probe measured 2,135,113 bytes against the current 2,099,014-byte ceiling,
  36,099 over. A flat probe measured 2,098,431, 583 under, which leaves no
  headroom for the real implementation. The operator chose a deliberate
  re-baseline of `ci/wasm-size-budget.json` on 2026-10-27. The release
  measurement on 2026-10-27 is 2,151,196 bytes, from 1,999,061, a delta of
  152,135. Gate A4 is unaffected because the recorded number never claimed
  to close it.
- **The version.** `SNAPSHOT_VERSION = 1`, a top-level `version` field beside a
  `magic` string. A reader refuses every other version rather than guessing.
  `serde`'s `deny_unknown_fields` refuses a field the version does not name.
- **What the readback cannot carry.** The 96-byte `ViewportState` is the
  inverse camera matrix plus five scalars. It cannot carry pan, zoom, rotation,
  flip, spacing, the index shift, the display grid, the VOI function, the
  selected VOI sequence, the colormap, or whether the presentation shape was
  declared. A restore from the readback would reconstruct a different `View`.
  So the snapshot reads the `View` and `Presentation` state, not the readback.
- **What is excluded.** Scroll keys, the selected slice, the loading, error and
  refining flags, the generation and the device pixel ratio are runtime state,
  not camera, display area or presentation state. The stack is configured
  separately and restoring a snapshot must not upload pixels or select a
  neighbour.
- **The TS surface.** The record is UTF-8 JSON. The shell parses it, refuses an
  unknown version before calling the core, and the core refuses it again. The
  TS type is hand-written because the schema is self-describing, and a boundary
  test proves the TS parser reads a record the Rust writer produced. The magic,
  version and error codes come from F-104's generated module, so the two
  readers cannot disagree about the version.

## Problem and reproducer

A live stack viewport holds its camera and presentation only in the Rust
session. There is no way to save them and bring them back. A reload, a
remount, a second viewport or a future CRDT layer cannot reconstruct the state,
and the 96-byte readback is not enough to do so.

The reproducer is a `StackViewport` whose presentation selected a VOI LUT
Sequence, whose camera has a non-zero pan and a quarter-turn rotation, and
whose display area was fitted to a 512 by 512 grid in a 800 by 600 CSS box.
There is no API that returns that state. `write_state` returns the inverse
matrix and two zero floats for the sequence's window. Restoring from those
bytes would produce a linear window and a `View` with pan zero, zoom one and no
rotation, which is a different camera and a different presentation. That is the
defect the sprint names.

Source identity: `crates/ocelli-viewport/src/view.rs` `View` and
`crates/ocelli-viewport/src/presentation.rs` `Presentation`, both private-field
types with no encode or decode path, and `crates/ocelli-wasm/src/session.rs`
`copy_state` returning only the 96-byte readback.

## Scope, API and compatibility

In scope:

- A versioned JSON `ViewportSnapshot` encoder and decoder in `ocelli-viewport`,
  covering the camera, the display area and the presentation state.
- `StackViewport::snapshot() -> Result<Vec<u8>, SnapshotError>` and
  `StackViewport::restore(&[u8]) -> Result<(), SnapshotError>`.
- Two wasm exports in `ocelli-wasm`, `snapshot() -> Vec<u8>` and
  `restore_snapshot(&[u8]) -> u32`, plus `snapshot_error() -> u32` so a refused
  encode is named rather than collapsed to an empty array.
- A `@ocelli/core` JSON parser and a session adapter in
  `packages/core/src/snapshot.ts`.
- Accessors on `LutDescriptor`, `ModalityTransform`, `VoiTransform` and
  `Presentation` needed to read the state without rebuilding it differently.
- A deliberate re-baseline of `ci/wasm-size-budget.json`.

Non-goals:

- No DICOM GSPS read or write. This is Ocelli's own state record, not a
  presentation state SOP instance.
- No annotation, segmentation or measurement state.
- No pixel, texture or brick upload, and no stack selection.
- No change to the 96-byte readback layout.
- No second camera matrix. The record stores the camera inputs and
  `View::camera_matrix` remains the only derivation.
- No change to the LUT arithmetic. The chain is rebuilt through
  `LutChain::new` from the same selections.

Public Rust signatures:

```rust
pub const SNAPSHOT_MAGIC: &str = "OCS1";
pub const SNAPSHOT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    NonFinite,
    Encode,
    Json,
    BadMagic,
    UnknownVersion(u16),
    BadEnum,
    BadLut,
    ChainUnavailable,
}

impl StackViewport {
    pub fn snapshot(&self) -> Result<Vec<u8>, SnapshotError>;
    pub fn restore(&mut self, bytes: &[u8]) -> Result<(), SnapshotError>;
}
```

`SnapshotError::number()` returns a stable nonzero `u32` for the wasm boundary.
Zero means success. The numbering is generated into TS by F-104 so the shell
maps a code to a name without a hand-written second table.

The JSON record, version 1:

```json
{
  "magic": "OCS1",
  "version": 1,
  "viewportId": 1,
  "displayArea": {
    "cssWidth": 800.0,
    "cssHeight": 600.0,
    "columns": 512,
    "rows": 512,
    "columnMillimetres": 1.0,
    "rowMillimetres": 1.0,
    "shiftColumn": 0.0,
    "shiftRow": 0.0,
    "fitted": true,
    "anchorX": 400.0,
    "anchorY": 300.0,
    "fitScale": 1.0
  },
  "camera": {
    "panX": 0.0,
    "panY": 0.0,
    "zoom": 1.0,
    "radians": 0.0,
    "flipHorizontal": false,
    "flipVertical": false
  },
  "presentation": {
    "modality": { "kind": "rescale", "slope": 1.0, "intercept": 0.0 },
    "voi": {
      "kind": "window",
      "center": 40.0,
      "width": 400.0,
      "function": "linear",
      "ymin": 0.0,
      "ymax": 255.0
    },
    "photometric": "monochrome2",
    "evidence": { "kind": "shape", "shape": "inverse" },
    "colormap": "grey"
  }
}
```

A LUT alternative replaces the object with:

```json
{
  "kind": "lut",
  "entryCount": 0,
  "firstMappedInput": -1024,
  "bitsPerEntry": 16,
  "values": [0.0, 1.0, 2.0]
}
```

`entryCount` is the declared descriptor value, so `0` keeps its DICOM meaning of
65,536 entries. The `values` array carries the resolved entries, which is
65,536 numbers for a declared zero. The presentation `evidence` object is one
of `{ "kind": "absent" }`, `{ "kind": "shape", "shape": "identity" }`,
`{ "kind": "shape", "shape": "inverse" }` or `{ "kind": "sequence" }`. The
`photometric`, `function`, `shape` and `colormap` values are the names the
existing public API already publishes, so the record is readable without a
codebook.

`SnapshotError` refusals, all leaving the viewport unchanged:

| Error | Trigger |
|-------|---------|
| `NonFinite` | A camera, display-area or LUT value is NaN or infinite. JSON has no NaN, so this is refused before encoding rather than serialised as `null` |
| `Encode` | `serde_json` refuses to serialise the record |
| `Json` | The input is not one JSON value, has a trailing byte, or carries a field this version does not name |
| `BadMagic` | `magic` is not `OCS1` |
| `UnknownVersion` | `version` is not `1` |
| `BadEnum` | A `kind`, `function`, `photometric`, `shape` or `colormap` name is outside its domain |
| `BadLut` | `LutDescriptor::new` refuses the descriptor |
| `ChainUnavailable` | `LutChain::new` refuses the rebuilt chain |

Compatibility: the exported names of `@ocelli/core` are unchanged. The new
exports are additive. `SNAPSHOT_VERSION` and the error codes are generated by
F-104, so a later version bump changes one Rust constant and the shell refuses
the old version without a second edit.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `StackViewport::snapshot` | `View`, `Presentation` and `serde_json::to_vec` | Rust caller or the wasm session | UTF-8 JSON that begins with `{"magic":"OCS1","version":1` |
| `StackViewport::restore` | `serde_json::from_slice`, `View::decode`, `Presentation::decode`, `LutChain::new` | Rust caller or the wasm session | The viewport's camera, display area and presentation replaced, scroll and flags untouched |
| `Session::snapshot` in `ocelli-wasm` | `StackViewport::snapshot` | The only crate that may import `wasm-bindgen` | A copied `Vec<u8>` to JavaScript, empty on a refusal |
| `Session::snapshot_error` | The last `snapshot` refusal | The only crate that may import `wasm-bindgen` | `0` after a success, `SnapshotError::number()` after a refusal |
| `Session::restore_snapshot` | `StackViewport::restore` | The only crate that may import `wasm-bindgen` | `0` on success, `SnapshotError::number()` otherwise |
| `sessionSnapshot(session)` in `@ocelli/core` | The two wasm exports | Shell adapter, no pixel access | `Uint8Array` out and a typed refusal in |
| `parseSnapshot(bytes)` | Generated constants and the schema | Shell parser | A typed `ViewportSnapshot` or a refusal |
| `OcelliViewport` in `@ocelli/react` | F-102 | Not this story | A saved and restored state, later |

Unchanged consumers affected by changed providers: `write_state` keeps its
96-byte layout and its zero window for a VOI sequence. `apply_commands` keeps
its refusals and its atomicity. `RenderGraph::set_view` keeps taking the CSS
matrix and the ratio.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| A viewport with a linear window | `restore` of a valid record | `view`, `presentation`, `columns`, `rows`, `css_width` and `css_height` replaced | None |
| A viewport with a linear window | `restore` of a record carrying a VOI LUT Sequence | The chain's stage 2 becomes the sequence, `voi().window()` is `None` | `BadLut` or `ChainUnavailable` if the descriptor or chain is refused, and the old chain stays |
| A viewport with a VOI LUT Sequence | `restore` of a linear window record | The chain's stage 2 becomes the window | The same refusal set |
| A fitted viewport | `restore` of an unfitted record | The anchor is the image centre and `fit_scale` is one | None |
| A viewport with an explicit inverse shape | `restore` of `Absent` on `MONOCHROME1` | `explicit_inverse()` is false and the chain inverts through the photometric default | None |
| Any viewport | `restore` of an unknown version | No change | `UnknownVersion(2)` |
| Any viewport | `restore` of malformed JSON or an unknown field | No change | `Json` |
| Any viewport | `snapshot` of a viewport holding a non-finite value | No bytes | `NonFinite` |
| Any viewport | `restore` of an out-of-domain enum name | No change | `BadEnum` |
| Any viewport | `restore` of a record whose chain cannot be rebuilt | No change | `ChainUnavailable` |
| Any viewport | `restore` of the same valid record twice | The same state both times, no accumulation | None |

Invariants that span consumers:

- **Atomicity.** Decode every field into locals, validate the descriptor and
  the chain, then assign. A refusal leaves `view`, `presentation`, `columns`,
  `rows`, `css_width` and `css_height` unchanged.
- **No pixel work.** `restore` does not call `upload_stored`, `set_voi`, a
  texture write or a brick upload. `ocelli-viewport` does not depend on
  `ocelli-render`, so the claim is by construction and by the unchanged
  `RenderGraph` counters in the integrated test.
- **No second matrix.** The record carries camera inputs. The only matrix is
  `View::camera_matrix`, derived on demand.
- **The readback is unchanged.** `write_state` still writes 96 bytes and a VOI
  sequence still reports zero centre and width.
- **Canvas, world and index stay distinct.** The pan and the anchor are
  `Pt<Canvas>` quantities, the shift is an index quantity in canvas pixels per
  the `View` mapping, and the spacing is millimetres. The record has named
  fields for each, not one number list.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| The encoded record | `StackViewport::snapshot` | Caller-owned `Vec<u8>`, no ticket and no wasm address | The returned bytes | A non-finite value returns `NonFinite`. A `serde_json` refusal returns `Encode` |
| The restored view | `StackViewport::restore` | The viewport instance | The final assignment block | A refusal changes nothing. A repeated valid restore is idempotent |
| The wasm copy | `Session::snapshot` | `wasm-bindgen` copies the `Vec<u8>` to a JS-owned array | The JS `Uint8Array` | No linear-memory view is returned, so a later growth cannot detach it |
| The restored session | `Session::restore_snapshot` | The session's `StackViewport` | The same commit point as the Rust call | A refused restore returns a nonzero code and keeps the previous state |
| The size budget | `pin_and_size_check.py --accept-size` | `ci/wasm-size-budget.json` | The recorded measurement | A measurement that is not taken leaves the old budget, and the `wasm` gate refuses an over-size module |

No side effect precedes the commit point. The snapshot allocates the JSON text
and the descriptor arrays, both owned by the caller or by the viewport after
the commit. An identical repeated restore does not append, select a neighbour
or change the generation.

For cached or settled output: the snapshot is not cached. It is generated from
the live state on every call. The restored state determines every input the
renderer reads, so a later `set_view` and `set_voi` see the restored values.
A refused restore must not replace the last usable camera or presentation,
which the atomicity rule guarantees.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | A viewport with a linear window, non-zero pan, quarter-turn rotation, one flip and a fitted display area round-trips. The restored `View`'s `camera_matrix` and `canvas_to_index` equal the original's exactly, and `Presentation::chain` equals the original's | `crates/ocelli-viewport/tests/snapshot.rs`, `round_trip_window` | Storing only the inverse matrix and reconstructing a view with pan zero, zoom one and no rotation | implementation |
| AC2 | A viewport whose stage 2 selected a VOI LUT Sequence round-trips to a chain whose `voi().window()` is `None` and whose descriptor values equal the original's | Same file, `round_trip_voi_sequence` | Storing centre and width and restoring a linear window, which is the sprint's named defect | implementation |
| AC3 | A viewport whose stage 1 selected a Modality LUT Sequence round-trips with `modality().rescale() == None` and equal descriptor values | Same file, `round_trip_modality_sequence` | Restoring slope and intercept that the sequence overrode | implementation |
| AC4 | A viewport with `PresentationLutEvidence::Shape(Inverse)` round-trips with `explicit_inverse() == true`. A `MONOCHROME1` viewport with `Absent` round-trips with `explicit_inverse() == false` and still inverts through the default | Same file, `round_trip_explicit_inverse` and `round_trip_absent_monochrome1` | Collapsing the evidence to the resolved boolean, so the two states become one | implementation |
| AC5 | A record whose `version` is `2` is refused with `UnknownVersion(2)` and the viewport is unchanged. A record with an unknown field is refused with `Json` | Same file, `refuses_unknown_version` and `refuses_unknown_field` | Accepting a future version and guessing the schema | implementation |
| AC6 | A bad magic, malformed JSON, a trailing byte, a non-finite field, an out-of-domain enum name, a refused descriptor and a refused chain are each refused with the named error and no state change. A non-finite live value makes `snapshot` return `NonFinite` | Same file, `refuses_*`, one case per error | A reader that validates some fields and commits before the rest, or a writer that emits `null` for NaN | implementation |
| AC7 | A restore followed by `canvas_to_world` on a known `ImagePlane` yields the same `Pt<World>` as before the snapshot. A canvas pan is not a patient translation, and the index shift is not a spacing | Same file, `canvas_world_and_index_are_distinct` | Writing one number list and reading it back as another space, which is HLD section 16's defect | implementation |
| AC8 | `restore` leaves the scroll index, the scroll length, the loading, error and refining flags and the generation unchanged, and the integrated render counters do not move | `crates/ocelli-viewport/tests/snapshot.rs` for the viewport fields, and an integrated test that reads the `RenderGraph` counters before and after a restore | A restore that re-uploads the stored texture or selects a neighbour | integrated |
| AC9 | The record contains no camera matrix. A mutation that replaces the camera inputs with the matrix fails AC1 and AC7 | The two tests above plus a review of the schema | Storing the matrix and calling it the camera, which is a second source of truth | implementation |
| AC10 | The built wasm module round-trips a snapshot through `snapshot()` and `restore_snapshot()`, and a refused restore returns the code that names the error. A non-finite live value makes `snapshot()` return an empty vector and `snapshot_error()` return `NonFinite`'s code | `packages/core/src/boundary.wasm.test.ts`, after `wasm-pack build crates/ocelli-wasm --dev --target web` | An export that copies a view over linear memory, a code that collapses every refusal to `1`, or an empty array with no way to learn why | integrated, needs the built module |
| AC11 | `parseSnapshot` refuses an unknown version, a bad magic, malformed JSON and an unknown field, and parses a record the Rust writer produced into the documented fields | `packages/core/src/snapshot.test.ts` and the boundary test | A hand-written parser that disagrees with the Rust schema | implementation and integrated |
| AC12 | Tier rows: the snapshot is tier-independent and performs no rendering | This section and a review of the call graph | A snapshot path that touches a device or a texture | implementation |
| AC13 | The magic, version and error codes come from F-104's generated module, and no hand-written copy of them remains in `@ocelli/core` | `packages/core/src/snapshot.test.ts` and the generated-file check | A second constant that drifts from the Rust record | implementation |
| AC14 | `bin/ocelli.sh wasm` measures the release module, `ci/wasm-size-budget.json` records that measurement as the new base with a `rebaselined` entry naming F-050, the from value, the to value, the delta and the cause, and the `wasm` gate passes | The measured build and the budget file | Re-baselining without a cause, or leaving the old base and reporting a green size check against it | integrated |

## Approach

The smallest implementation in terms of the transcribed source.

1. **`Cargo.toml`.** Change the workspace entries to
   `serde = { version = "1", default-features = false, features = ["derive", "alloc"] }`
   and
   `serde_json = { version = "1", default-features = false, features = ["alloc"] }`.
   Measured: `nostd` stays green, `ocelli-dicom` still compiles, and
   `ocelli-viewport` compiles for wasm32.
2. **`crates/ocelli-viewport/Cargo.toml`.** Add `serde.workspace = true` and
   `serde_json.workspace = true`.
3. **`crates/ocelli-viewport/src/snapshot.rs`**, a new module. It owns
   `SNAPSHOT_MAGIC`, `SNAPSHOT_VERSION`, `SnapshotError` and the serde types.
   The serde types are private to the module and are the schema. The public
   `StackViewport` methods convert between them and `View` and `Presentation`.
   `#[serde(deny_unknown_fields)]` is on every struct.
4. **`View::encode` and `View::decode`** in `view.rs`. The private fields stay
   private. `View` does not gain a matrix field.
5. **`Presentation::encode` and `Presentation::decode`** in `presentation.rs`.
   They read the chain's `modality()` and `voi()` and the stored `photometric`,
   `evidence` and `colormap`. The decode path calls `ModalityTransform::new`,
   `VoiTransform::new`, `LutChain::new` and `Presentation::new`, so the
   arithmetic and the validation stay in `ocelli-pixel`.
6. **`StackViewport::snapshot` and `StackViewport::restore`** in `commands.rs`.
   The snapshot builds the serde value, validates finiteness, and calls
   `serde_json::to_vec`. The restore calls `serde_json::from_slice`, checks the
   magic and version, builds the view and presentation, validates the chain,
   then assigns.
7. **`ocelli-pixel` accessors.** `LutDescriptor` retains its declared entry
   count so the `0`-means-65,536 rule stays in `new`, and gains
   `declared_entry_count()` and `values()`. `ModalityTransform` gains
   `lut()` returning `Option<&LutDescriptor>` and `VoiTransform` gains the
   same. `Presentation` gains `photometric()` and `evidence()`. These are
   accessors over state already held, adding no arithmetic. The
   `LutDescriptor` clone counter is not incremented by a read.
8. **`crates/ocelli-wasm/src/session.rs`.** Three exports:
   `snapshot() -> Vec<u8>` (empty on a refusal),
   `snapshot_error() -> u32` (the last encode refusal, `0` after a success) and
   `restore_snapshot(bytes: &[u8]) -> u32`. The session is the existing
   thread-local `StackViewport::try_identity(1)`.
9. **`packages/core/src/snapshot.ts`.** `SNAPSHOT_MAGIC`, `SNAPSHOT_VERSION` and
   `SNAPSHOT_ERROR` imported from the generated module, `parseSnapshot`,
   `SessionSnapshotExports` and `sessionSnapshot(session)`.
10. **`packages/core/src/index.ts`.** Re-export the new names. This is the file
    that makes every wave serial, see the next section.
11. **`ci/wasm-size-budget.json`.** After the implementation is measured, set
    `bytes` to the measured size and add a `rebaselined` entry naming F-050,
    the from value, the to value, the delta and the cause. Keep the
    `bearing_on_gate_A4` note that the number does not close A4.

Rejected alternatives:

- **A versioned little-endian binary record.** It is the smallest option and
  adds no dependency, but the operator chose JSON on 2026-10-27 for
  readability.
- **A fixed-size record.** A VOI LUT Sequence can carry 65,536 entries.
  Truncating it or refusing it would make the round trip fail on a legal
  DICOM object.
- **Serialising from the 96-byte readback.** It cannot carry pan, zoom,
  rotation, flip, spacing, the VOI function, the selected sequence, the
  colormap or the evidence. The round trip would produce a different camera.
- **Storing the camera matrix and reconstructing the view.** A second source
  of truth, and it cannot recover the rotation, zoom and pan decomposition.
  HLD section 16's separation of canvas, world and index is the reason this
  project does not pass bare number lists around.
- **A new `Snapshot` trait.** One implementer today, so it is refused by
  `AGENTS.md`'s structural test.

## Dependencies, write ownership and resources

Declared dependencies: F-048, done. Source-level dependencies: F-104, because
the magic, version and error codes must come from the generated module. F-104
lands first in this sprint.

Files that cannot have concurrent writers:

- `Cargo.toml` and `Cargo.lock`
- `crates/ocelli-viewport/src/{snapshot.rs,view.rs,presentation.rs,commands.rs}`
- `crates/ocelli-viewport/Cargo.toml`
- `crates/ocelli-pixel/src/lut.rs`
- `crates/ocelli-wasm/src/session.rs`
- `packages/core/src/{snapshot.ts,index.ts}`
- `ci/wasm-size-budget.json`
- `docs/lld/viewport.md` and `docs/lld/public-api.md`

Ledgers: `BACKLOG.md`, `SPRINT_TRACKER.md`, `AS_BUILT.md`, `CURRENT_SPRINT.md`.
No GPU and no browser. The boundary test needs a dev wasm build, and the size
measurement needs a release build. Both are serial.

## Verification and evidence reuse

Focused checks:

```bash
bin/ocelli.sh check ocelli-viewport
bin/ocelli.sh test ocelli-viewport
bin/ocelli.sh check ocelli-pixel
bin/ocelli.sh test ocelli-pixel
bin/ocelli.sh check ocelli-wasm
bin/ocelli.sh clippy
bin/ocelli.sh fmt
python3 scripts/no_std_check.py
cargo check -p ocelli-dicom --all-targets
cargo check -p ocelli-viewport --target wasm32-unknown-unknown
npx vitest run packages/core/src/snapshot.test.ts
wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm
npx vitest run --config vitest.boundary.config.ts
bin/ocelli.sh wasm
python3 scripts/pin_and_size_check.py --with-size
```

Final profile: `bin/ocelli.sh gate --sprint` over the staged union, with the
corpus result recorded. The boundary suite is an explicit required suite for
the changed boundary path, per `.claude/WORKFLOW.md`.

Evidence identities: the Rust tests name the record version and the schema. The
boundary test names the built module path and the export names. The size
measurement names the release build and the budget file. The corpus is
unchanged, so a corpus result that moves is a signal to check the digests
before the code.

What invalidates evidence: any edit to a Rust boundary type, the schema, the
generated module, `packages/core/src/snapshot.ts`, the budget file or the
ledgers after the verification. The record is a new boundary object, so its
evidence must be recorded against the tree that contains it.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only, for the two new exports.
- Pixels across the boundary: no. The record is state text.
- Render-loop allocation: none. `snapshot` allocates once per call and is not
  called per frame. `restore` allocates the parsed record and the descriptor
  arrays.
- unsafe: none. The parser is safe and the existing helpers are checked.
- Tier A (WebGPU): n/a, the record carries no device state.
- Tier B (WebGL2): n/a.
- Tier C (CPU): n/a. The snapshot is tier-independent and a tier C session
  uses the same record.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | The reader refuses every error in the table, and the writer emits the documented schema | `crates/ocelli-viewport/src/snapshot.rs` under `#[cfg(test)]` |
| `fixture` | A hand-written JSON document with known values round-trips to the documented `View` and `Presentation`, including a VOI LUT descriptor with a declared entry count of `0` | `crates/ocelli-viewport/tests/snapshot.rs`, citing PS3.3 C.11.2.1.1 and C.11.6 |
| `property` | A generated family of views and presentations round-trips exactly, including fitted and unfitted, flipped and rotated, and zero spacing on one axis | `crates/ocelli-viewport/tests/snapshot.rs` |
| `unit` | The shell parser refuses an unknown version, a bad magic, malformed JSON and an unknown field, and reads the generated constants | `packages/core/src/snapshot.test.ts` |
| `browser` | The built wasm module round-trips the record through the public helpers, and the shell parser reads the Rust-produced record | `packages/core/src/boundary.wasm.test.ts` |

`fixture` is mandatory here because the record carries the presentation state
that determines pixel arithmetic. The hand-computed values cite PS3.3
C.11.2.1.1 for the descriptor and C.11.6 for the shape.

## Parity surface covered

Appendix B rows this story can establish: none on its own. The snapshot is
Ocelli's own state record, not a cornerstone3D surface. It prepares the
`CORNERSTONE_DISPLAY_AREA_MODIFIED`, `CORNERSTONE_CAMERA_MODIFIED` and
`CORNERSTONE_VIEWPORT_RENDERING_PRESET_MODIFIED` observations that F-103
catalogues, but it does not claim them.

## Deviations

**D-26.** HLD section 15.2's workspace dependency list does not include
`serde` or `serde_json`. F-050 uses both in `ocelli-viewport`, a shipped core
crate, and changes their workspace entries to `default-features = false` with
`alloc`.

Why: the operator chose a JSON record on 2026-10-27. JSON needs a serialiser
and a parser, and the two crates are already in the workspace for the oracle
and for `ocelli-dicom`, so this is a use of an existing dependency rather than
a new one. The `alloc` feature set is what keeps `ocelli-viewport`'s
`no_std` posture, which `scripts/no_std_check.py` enforces. Measured on this
host: `nostd` green, `ocelli-dicom` green, `ocelli-viewport` green for
wasm32. The cost is the wasm size re-baseline recorded above.

The row is in `docs/hld/DEVIATIONS.md` in the same change that approves this
plan, and `scripts/deviation_check.py` resolves it.

## LLD impact

- `docs/lld/viewport.md`: the JSON schema, the version rule, the refusal set
  and the round-trip invariants.
- `docs/lld/public-api.md`: the new exports and the shell parser.
- `docs/lld/typescript-packaging.md`: the new `snapshot.ts` module in the
  tarball.
- `docs/lld/build-targets.md` or `docs/lld/tier-resolution.md`: the size
  re-baseline, if either records the budget. `ci/wasm-size-budget.json` is the
  authority.

## Open questions

**Q1, answered 2026-10-27. The record format.** The operator chose a JSON text
record over a versioned binary record. This plan applies that answer. The
consequence is the serde dependency and the size re-baseline recorded below.

**Q2, answered 2026-10-27. The size re-baseline.** The operator chose to
re-baseline `ci/wasm-size-budget.json` deliberately rather than switch to a
binary record or shrink JSON under the ceiling. A faithful nested probe
measured 2,135,113 bytes against the 2,099,014-byte ceiling. The release
module measured on 2026-10-27 is 2,151,196 bytes. The budget file records
that base, the cause, and the note that the number does not close gate A4.

**Q3, written 2026-10-27. The deviation row.** The JSON choice needed a D-26
row because `serde` and `serde_json` are not in HLD section 15.2's dependency
list. The row is in `docs/hld/DEVIATIONS.md` in this change, so the register
change is visible beside the plan that depends on it.
