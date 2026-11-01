# F-066, segmentation state model and per-viewport representation registry

**Status**: approved
**Epic ref**: E10.4
**Sprint**: S23
**Estimate**: 3w

The operator told this run to finish the sprint without a question pause.
The choices in `## What the specification does not cover` are local.
No material question remains.

## Normative source, transcribed

From `docs/hld/03-architecture-and-crates.md` section 4, the `ocelli-seg` row:

Segmentation state and the three representations. wasm yes. native yes.

From `docs/hld/B-parity-surface.md`:

Segmentation representations, 3, Labelmap, contour, surface.

From `docs/hld/19-render-graph.md` section 22:

```rust
pub enum Pass {
    Stack(StackPass),
    VolumeRaycast(VolumePass),
    SegOverlay(SegPass),
}
```

From the same section:

- Pipelines compile at init, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame.
- Dirty tracking. A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.
- One queue.submit() per frame across all viewports.
- Device loss is a real state. Rebuild the device and all resources, and restore viewport state from the shell's copy.

From `docs/hld/26-differentiating-capabilities.md` section 31:

Results stay resident on the GPU wherever the consumer is the renderer. A segmentation mask should never round-trip through JavaScript to be drawn.

`docs/hld/` does not define the registry, the enum, or what a representation change does to an already uploaded overlay. F-063, F-064 and F-065 own the three presenters. This story does not choose a colour and does not build pixels.

## What the specification does not cover

The HLD says `ocelli-seg` holds segmentation state and the three representations. It does not say how a viewport remembers which one it shows. These choices are local.

- The value is `Representation::{Labelmap, Contour, Surface}`. It is not a colour, not a grid, and not a mesh.
- A new composite `SegOverlay` starts as `Labelmap`, which is what F-063 already draws. Existing labelmap tests do not call the setter.
- `set_representation` on a stack, a slab, or a volume returns `NotOverlay` and changes nothing.
- Setting the same value again does not drop the overlay and does not mark dirt.
- Setting a different value drops the overlay binding and marks segmentation dirt. It does not upload. The tick stays unavailable until the matching `show_*` uploads. The previous representation's pixels are not drawn again.
- `show_labelmap` requires `Labelmap`. `show_contour` requires `Contour`. `show_surface` requires `Surface`. A mismatch returns before the presenter runs, uploads nothing, and keeps the binding that the setter already left.
- The registry is CPU state. Device recovery keeps the enum and still drops the texture. The next matching show uploads again.
- Labelmap edge projection stays uncompiled. This story does not implement F-067.

## Problem and reproducer

After F-064 and F-065, one overlay slot can hold labelmap bytes, contour bytes, or surface bytes. Nothing records which one the viewport was given. A `show_labelmap` after the caller selected a contour would replace the contour with a grid. The registry must refuse that call before `present_labelmap` runs.

## Scope, API and compatibility

In scope: the enum, the per-viewport slot on the render graph, the mismatch refusal on the three show paths, and the drop of the overlay binding when the choice changes.

Out of scope: a new presenter, a new shader, DICOM SEG parsing, colour selection, and labelmap edge projection.

```rust
pub enum Representation { Labelmap, Contour, Surface }
```

```rust
pub fn representation(&self, id: ViewportId) -> Result<Representation, GraphError>
pub fn set_representation(&mut self, id: ViewportId, representation: Representation) -> Result<(), GraphError>
```

`ocelli-seg` owns the enum. `ocelli-render` stores one value per viewport. The show methods grow a `RepresentationMismatch` error.

F-064's contour GPU test and F-065's surface GPU test call `set_representation` before the show once this story lands. Their CPU presenters stay unchanged.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `set_representation` | Slot on the viewport | `SegOverlay` only | Stored enum, or `NotOverlay` |
| `show_labelmap` | Registry then F-063 presenter | Choice is `Labelmap` | Upload, or mismatch with no present |
| `show_contour` | Registry then F-064 presenter | Choice is `Contour` | Upload, or mismatch with no present |
| `show_surface` | Registry then F-065 presenter | Choice is `Surface` | Upload, or mismatch with no present |
| `install_recovered` | Existing drop of GPU objects | Any choice | Enum unchanged, texture gone |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| New `SegOverlay` | `representation` | `Labelmap` | Not a refusal |
| Shown labelmap, dirt clear | `set_representation(Labelmap)` | Dirt stays clear and the tick is a no-op | Binding stays |
| Shown labelmap | `set_representation(Contour)` | Binding dropped, segmentation dirt set | No upload. Tick is unavailable |
| Choice is `Contour` | `show_labelmap` | Mismatch | Presenter does not run. Destination binding unchanged |
| Choice is `Contour`, binding empty | `show_contour` | Contour bytes uploaded | A present error still uploads nothing |
| After recovery | `representation` | The enum from before the loss | Texture stays empty until show |

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Representation enum | `set_representation` | Viewport id | Setter returns `Ok` | Same value does not drop the texture |
| Overlay texture | Matching `show_*` | Byte stamp | Binding replaced after the texture exists | Mismatch and present errors leave it |
| Registry across recovery | CPU slot | Viewport id | `install_recovered` returns | Enum kept. GPU objects dropped |

The setter allocates no image. Equal bytes after a matching show still do not allocate. A representation change is not an equal-bytes case, because the binding is dropped first.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | A new composite overlay reports `Labelmap`. | `a_new_overlay_starts_as_labelmap` | Defaulting to contour and breaking F-063 shows | implementation |
| AC2 | `set_representation` on a stack returns `NotOverlay`. The stack tick still submits. | `a_stack_has_no_representation` | Storing the enum on the stack | gpu gate |
| AC3 | Setting `Contour` after a shown labelmap makes the next tick unavailable and does not submit. `show_labelmap` then returns mismatch. | `switching_drops_the_overlay_without_upload` | Leaving the labelmap texture bound while the enum says contour | gpu gate |
| AC4 | After that switch, `show_contour` uploads and the readback matches `present_contour`. | `the_matching_show_uploads` | The setter painting a colour of its own | gpu gate |
| AC5 | `show_surface` while the choice is `Contour` returns mismatch and does not submit. | `a_surface_show_respects_the_registry` | Falling through to the surface presenter | gpu gate |
| AC6 | Setting the current value again leaves a clean viewport as a no-op tick. | `the_same_choice_keeps_the_binding` | Dropping the texture on every setter call | gpu gate |
| AC7 | `install_recovered` keeps `Contour`. `show_contour` before rebind is `NotRebound`. After rebind the matching show matches the presenter. | `recovery_keeps_the_choice_and_drops_the_texture` | Restoring the enum to `Labelmap`, or keeping the dead texture | gpu gate |

## Approach

Add `Representation` in `ocelli-seg` and re-export it. Store it on the viewport slot beside the pass. `set_representation` is the only writer. Each `show_*` reads it before the presenter. A change drops `slot.labelmap` and sets segmentation dirt.

Rejected: a second overlay texture per representation. Rejected: uploading inside the setter. Rejected: a trait with one implementer per representation.

## Dependencies, write ownership and resources

F-063, F-064 and F-065 are the three presenters. This story edits `crates/ocelli-seg/src/lib.rs`, `crates/ocelli-render/src/graph.rs`, and the render tests for the three show paths. It does not edit shaders.

Serial, in the canonical worktree, after F-064 and F-065. The GPU gate stays serial.

## Verification and evidence reuse

Focused checks: `bin/ocelli.sh test ocelli-render --test registry` and the existing labelmap, contour and surface tests. Clippy on both crates.

Completion uses the feature floor plus corpus. This story does not edit CPU candidate bytes or the oracle producer.

A mutation that ignores the enum inside `show_labelmap` must fail AC3. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): full. The registry gates the existing composite upload.
- Tier B (WebGL2): full. The same gate. No compute.
- Tier C (CPU): the enum is available. `show_*` still returns unavailable before a presenter runs, which is the F-063 rule. The CPU presenters remain the tier C image.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | The new overlay starts as `Labelmap` and a stack refuses the setter | `crates/ocelli-render/tests/registry.rs` |
| gpu | A switch drops the binding, a mismatch does not present, the matching show matches the presenter, and recovery keeps the enum | same file, ignored, gpu gate |

## Parity surface covered

Segmentation representations, the state that selects among the three rows. No new representation is added. Labelmap edge projection stays uncovered.

## Deviations

none

## LLD impact

`docs/lld/render-graph.md` and `docs/lld/feature-availability.md` name the registry. `docs/lld/README.md` if the index row needs the F-ID.

## Open questions

none
