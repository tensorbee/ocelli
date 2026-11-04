# F-192, series assembly and cine timing

**Status**: approved
**Epic ref**: E38.2
**Sprint**: S42
**Estimate**: 5w

## Normative source, transcribed

HLD `docs/hld/16-volume-representation.md` section 19 defines `Volume` as one contiguous allocation with spacing, origin, direction, voxel kind, levels and per-slice presence. It does not define display order, mixed-series splits, a temporal axis, or cine timing.

The implementation that already exists, `Volume::assemble`, sorts by projected position and refuses a mixed orientation, a duplicate position, non-uniform spacing and a sheared stack. Those refusals stay. This story does not call `assemble` to order a series a person can scroll.

`StackGeometry::derive` in `ocelli-dicom` measures shear and refuses non-parallel frames, non-uniform spacing and coincident frames. `GEOMETRY_TOLERANCE_MM` is `1e-6` and is private. This story does not change that constant and does not change `derive`'s verdicts.

DICOM PS3.3 C.7.6.2.1.1, already implemented by `ImagePlane`, is the projection this story reuses:

```text
P = IPP + column * column_spacing * row_direction + row * row_spacing * column_direction
normal = row_direction × column_direction
projected = dot(IPP, normal)
```

`docs/hld/B-parity-surface.md` has no cine or series-split row. This story covers none.

## What the specification does not cover

The HLD does not say how to scroll a series that is not one volume. The story does. Display assembly is a new type beside `StackGeometry`. It orders frames and reports splits. It does not build a `Volume` and it does not weaken a volume refusal.

DICOM cine attributes used here, because the HLD is silent:

| Tag | Name | Use |
|----|----|----|
| (0018,1065) | Frame Time Vector | Per-frame milliseconds, only when the length equals the frame count |
| (0018,1063) | Frame Time | One interval in milliseconds |
| (0008,2144) | Recommended Display Frame Rate | Frames per second |
| (0018,0040) | Cine Rate | Frames per second |
| (0018,1244) | Preferred Playback Sequencing | LOOP or SWEEP |

The first timing source that is present and valid wins, in the table order above. An absent or zero-length source is skipped. A non-finite, zero, or negative interval is an error for that source and does not fall through to a quieter default. When every source is absent, timing is `Unavailable`. The library does not invent 10 frames per second.

LOOP is the sequencing when `(0018,1244)` is absent. SWEEP is the DICOM name for the story's bounce. The core enum is `Loop` and `Sweep`. The page label for `Sweep` is Bounce. The stored value is not renamed inside the crate.

Temporal position uses Frame Content Sequence items already reachable through `MultiframeMetadata`: In-Stack Position Number `(0020,9057)`, Temporal Position Index `(0020,9128)`, and Stack ID `(0020,9056)`. A classic single-frame series uses Image Position Patient and Instance Number only.

Showcase helper and page ownership follow F-191. This story adds a route and a panel. It does not add a second server.

## Problem and reproducer

Two synthetic instances share a Series Instance UID. One is axial and one is a localiser with a different orientation. A third and fourth share the axial orientation at two projected positions, and the fourth repeats the second position with a later temporal index. Expected: two display groups. The axial group orders by projected position and carries a temporal axis of length 2 at the repeated position. Current behaviour: `StackGeometry::derive` returns `NonParallelFrames` for a mixed object, and nothing orders a bag of instances.

Cine fixture: Frame Time 100. Expected delay 100 ms and mode Loop. Preferred Playback Sequencing SWEEP selects Sweep. No timing attribute selects Unavailable.

## Scope, API and compatibility

In scope:

- `crates/ocelli-dicom/src/series_open.rs`.
- Input is a list of already parsed objects, each with a frame index. This story does not scan a directory.
- Group key: Frame of Reference UID, orientation agreement through the existing cosine check, and in-plane spacing agreement through the existing spacing check. A missing Frame of Reference UID does not join a group that has one. Two missing UIDs do not join each other.
- Inside a group, order by projected `dot(IPP, normal)`. Frames with no Image Position Patient sort after positioned frames by instance number, then by SOP Instance UID bytes, then by frame index. The group reports `SpatialOrder::Projected` or `SpatialOrder::InstanceNumber` so a caller can see which key was used.
- Equal projected positions within `GEOMETRY_TOLERANCE_MM` stay in one spatial slot. They order by Temporal Position Index when present, otherwise by instance number. The group reports the duplicate. It does not drop a frame.
- A group whose frames are not one volume still returns. Shear, non-uniform spacing and a single frame are reports, not refusals, on this type. `StackGeometry::derive` and `Volume::assemble` are unchanged.
- Cine timing as the table above.
- Page panel: group count, order labels, timing source, interval, and Loop or Bounce.
- Route `POST /series-open` accepts a multipart body of Part 10 objects capped by the same 1 GiB total. The synthetic button sends two tiny objects built in the server.

Out of scope:

- Gantry-tilt volume construction. That is F-197.
- Changing `GEOMETRY_TOLERANCE_MM`, the 0.5 percent volume spacing rule, or the 1e-9 mm volume duplicate bound.
- Folder viewer sort. It stays on instance number.
- Playback inside the wasm session. The page shows the numbers and a Loop or Bounce control whose initial value is the file value. Playing pixels is the existing folder page, not this panel.

```rust
pub struct DisplaySeries {
    // private fields, accessors
}

pub enum SpatialOrder {
    Projected,
    InstanceNumber,
}

pub enum Playback {
    Loop,
    Sweep,
}

pub enum CineTiming {
    Unavailable,
    FrameTimeVector,
    FrameTime { milliseconds: f64 },
    RecommendedDisplayFrameRate { frames_per_second: f64 },
    CineRate { frames_per_second: f64 },
}

pub fn assemble_display(frames: &[ParsedFrame]) -> Result<Vec<DisplaySeries>, SeriesOpenError>;
```

`ParsedFrame` holds a borrowed parsed object, a frame index, and no path.

`SeriesOpenError` carries a closed reason and a frame index. No UID string and no path, because a UID can be identifying. The page shows the reason and the index.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `assemble_display` | `FrameGeometry::derive` per frame | A bag of frames the caller already parsed | Groups, order, cine |
| Volume construction | `Volume::assemble` | Unchanged | Same refusals as today |
| Shear measurement | `StackGeometry::derive` | Unchanged | Same refusals as today |
| Example panel | `POST /series-open` | Synthetic pair, or posted files | Group count and timing JSON |

Rounding. Projected positions stay `f64` from the existing plane. This story does not round them to a display pixel.

## State transitions and invariants

Stateless. Order is a pure function of the frames. Repeating the call returns the same group lengths and the same permutation.

Invariant. A mixed orientation never shares a group. A missing Series Instance UID is not invented, and this function does not merge across Series Instance UIDs. The caller passes one series. If two SOP series are passed together, that is the caller's bag and the function still splits on frame of reference and orientation. The page sends one series at a time.

## Ownership, commit and recovery

Same request lifetime as F-191. No retained series. A failed parse of one part fails the request and discards the body. The previous panel stays.

Not applicable as a cache. Nothing is settled across requests.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Two axials at z = 0 and z = 3, normal along z. Order is z then the later slice. | Fixture citing PS3.3 C.7.6.2.1.1 | Sort by instance number when positions exist | implementation |
| AC2 | An axial and a sagittal orientation. Two groups. | Fixture | One group in instance order | implementation |
| AC3 | Same projected position, temporal indexes 1 and 2. One spatial slot, temporal length 2, both frames kept. | Fixture | Dropping the duplicate the way `Volume::assemble` does | implementation |
| AC4 | No Image Position Patient. `SpatialOrder::InstanceNumber` and both frames present. | Unit | A zero vector treated as anatomy | implementation |
| AC5 | Frame Time 100. `CineTiming::FrameTime { milliseconds: 100.0 }` and `Playback::Loop`. | Fixture citing the cine module | A silent 10 fps | implementation |
| AC6 | Preferred Playback Sequencing SWEEP. `Playback::Sweep`. | Unit | Bounce implemented as a second timing formula | implementation |
| AC7 | Frame Time 0. Error. Recommended Display Frame Rate is not consulted. | Unit | Falling through to a usable rate | implementation |
| AC8 | `StackGeometry::derive` on a non-parallel enhanced object still returns `NonParallelFrames`. | Existing test, rerun | This story catching that error and sorting anyway inside `derive` | implementation |
| AC9 | The page synthetic button shows two groups and the axial order. | curl JSON | Order only in a unit test | integrated |

## Approach

New file `series_open.rs`. For each frame, call `FrameGeometry::derive`. On `FrameGeometryError` for a missing position, record that frame as unpositioned. Other geometry errors that mean the frame has no plane also become unpositioned only when the error is the missing-position variant. A malformed cosine is still an error for that frame.

Grouping compares orientation with the existing `cosines_agree` logic by calling the public plane accessors and `DIRECTION_COSINE_TOLERANCE`. Do not copy the tolerance number into a new constant. Spacing uses the existing `PixelSpacing` equality already used by `spacing_agrees`. If that function is private, compare through the public `spacing()` values with the same public tolerance the pixel crate already exports for direction cosines only where a public tolerance exists. Spacing equality stays the private `SPACING_EQUALITY_MM` behaviour by using a crate-visible helper moved next to the existing one, still one definition.

Projected sort uses `position_vector().dot(slice_normal())`. The normal comes from the group's first positioned frame.

Cine reads attributes from the first frame's metadata through the existing element readers. Frame Time Vector is accepted only when its count equals the group length in stack order.

Rejected. Teaching `Volume::assemble` to return mixed series. Sorting the folder page by position in this story. A default frame rate.

## Dependencies, write ownership and resources

Depends on F-051 and F-047, both done. Source dependency on F-191 for the example server and `App.tsx`. Serial wave, after F-191, same canonical worktree.

Writes:

- `crates/ocelli-dicom/src/series_open.rs` and `lib.rs`
- `crates/ocelli-dicom/tests/series_open.rs`
- `examples/example-server` route
- `examples/viewer-react/src/App.tsx` panel
- `docs/lld/series-open.md` at completion

Does not write `frame_geometry.rs` except to expose a crate-visible spacing helper if the comparison cannot otherwise share one definition. That edit must not change a numeric tolerance.

No GPU.

## Verification and evidence reuse

```bash
bin/ocelli.sh test ocelli-dicom series_open
bin/ocelli.sh test ocelli-dicom frame_geometry
bin/ocelli.sh clippy ocelli-dicom
```

`frame_geometry` is the regression that AC8 names. The sprint gate is the final profile.

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
| fixture | AC1, AC2, AC3, AC5. Positions are hand-picked millimetre triples. | `series_open.rs` tests |
| unit | AC4, AC6, AC7 | same |
| regression | AC8 existing geometry tests | `frame_geometry` tests |
| integrated | AC9 curl | example server |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

New `docs/lld/series-open.md`. `docs/lld/volumes.md` is not rewritten. It remains the volume refusal path.

## Open questions

None.
