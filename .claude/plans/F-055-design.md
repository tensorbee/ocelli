# F-055, Slice-position sync and frame-of-reference handling

**Status**: approved
**Epic ref**: E8.5
**Sprint**: S20
**Estimate**: 3w

Depends on F-052. That backlog row is `done`. `docs/sprints/allocation.json` notes for F-055 are empty. There is no architecture-hook flag and no source-policy warning.

## Normative source, transcribed

### `.claude/skills/dicom-expert/SKILL.md`, section 1

> **Frame of Reference** `FrameOfReferenceUID` (0020,0052) is what makes coordinates comparable. Two series share a spatial frame if and only if they share this UID. **An annotation is only transferable between series that share it.** This is the mechanism HLD section 35 relies on for putting a slide layer and a volume layer in one scene.

### `.claude/skills/dicom-expert/SKILL.md`, section 6, and `docs/lld/volumes.md`

> With `X = IOP[0..3]` the row direction and `Y = IOP[3..6]` the column direction, `i` the **column** index and `j` the **row** index:

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

> **TRAP: IPP is the centre of the first voxel, not its corner.** A half-voxel offset propagates into every measurement and every fused overlay, and it is exactly the size that looks like acceptable registration error.

`docs/lld/volumes.md` extends that point with this volume's third axis:

> `Volume::voxel_world` is the patient point of a continuous index `(i, j, k)`. The origin is the centre of voxel `(0, 0, 0)`. The point is the origin plus `i * spacing[0] * direction.col(0)` plus `j * spacing[1] * direction.col(1)` plus `k * spacing[2] * direction.col(2)`. `spacing[0]` is column millimetres along the row cosine. That is DICOM PS3.3 C.7.6.2.1.1 with the volume's third axis.

### `docs/lld/volumes.md`, grouping

| Type | What it is |
|------|-----------|
| `FrameOfReferenceUid` | The `FrameOfReferenceUID` string, compared and never parsed |

> `group_volumes` splits on `FrameOfReferenceUID` and `ImageOrientationPatient` together, in first-seen order. A localiser mixed with axials is two candidates. A second frame of reference is a second candidate. Assembly never re-checks the frame of reference, because slice evidence does not carry one.

### `docs/hld/26-differentiating-capabilities.md`, section 35

> A slide layer and a volume layer **share the annotation coordinate model** through the frame of reference. That is what makes correlation possible rather than merely adjacent.

### `docs/hld/19-render-graph.md`, section 22

> **Dirty tracking.** A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.

### `docs/hld/03-architecture-and-crates.md`, section 4

| Crate | Responsibility | wasm | native |
|-------|----------------|------|--------|
| ocelli-volume | Volume assembly, geometry, reslicing | yes | yes |
| ocelli-viewport | Viewport and scene model, camera, presentation state | yes | yes |

### `docs/hld/13-core-types.md`, section 16

World points in this story are `Pt<World>`. The shared point is not a voxel index and not a canvas point.

```rust
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
#[derive(Debug, PartialEq)]
pub struct Pt<S> { pub x: f64, pub y: f64, pub z: f64, _s: PhantomData<S> }
// NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
// types do not satisfy. Implement by hand.
impl<S> Clone for Pt<S> { fn clone(&self) -> Self { *self } }
impl<S> Copy for Pt<S> {}
impl<S> Pt<S> {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, _s: PhantomData }
    }
}
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
impl<A, B> Transform<A, B> {
    pub fn apply(&self, p: Pt<A>) -> Pt<B> { /* ... */ }
    pub fn inverse(&self) -> Transform<B, A> { /* ... */ }
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> { /* ... */ }
}
```

## What the specification does not cover

The HLD says two series share a spatial frame only when they share `FrameOfReferenceUID`, and that a viewport re-renders when the slice changes. It does not say how one world point moves a second plane. These choices are local. They are not a new deviation.

1. A sync group is a `FrameOfReferenceUid` plus a world point `Pt<World>`. Setting the point on one viewport moves every viewport registered in that same UID, including the source, so that its plane contains the point. The plane normal is unchanged. The stored anchor is the plane origin at index 0. The current plane origin is `anchor + index * spacing_mm * normal`. The offset along the normal is `(point - plane_origin) dot normal`. The new index is `index + offset / spacing_mm`. For a unit normal this is `((point - anchor) dot normal) / spacing_mm`.
2. The caller passes the stored direction column as `normal` and does not ask this function to normalise it. Identity row, column and slice columns are unit. Assembly already normalises the slice column. A second normalise would be a second copy of that geometry.
3. A viewport whose volume UID differs does not move. Comparing UIDs is string equality on the existing `FrameOfReferenceUid` type. No parsing. No trimming. A trailing space is a different UID.
4. A missing UID does not join a group. If the source viewport has no UID, the call reports unavailable and leaves every viewport where it was. A peer with no UID stays put while same-UID peers move. That peer case is not an error.
5. The anchor stays put when the index changes. Storing the current plane origin and the index together would apply the offset again on the next call.
6. This does not fuse pixels. Fusion is F-056. This does not change spacing. This does not read `InstanceNumber`, `SliceLocation` or `SpacingBetweenSlices`. This does not add the UID back onto `Volume`. Slice evidence still does not carry one, so assembly still does not re-check it.
7. The index is continuous `f64`. It is not rounded to an integer and it is not clamped to the volume dimensions. The function does not take dimensions. An index of `-10` is a stored result. `Volume::reslice` still returns `PlaneOutOfRange` later if a caller samples that index. This story does not call `reslice`.
8. Registration copies one plane into a slot and does not move any peer. Only setting the point moves planes. A viewport registered after a set keeps the index it was given until the next set.
9. A zero normal, a spacing that is not finite and greater than zero, or a non-finite anchor, normal, index or point is unavailable. No index is written. There is no tolerance on the normal length.
10. The geometry test is CPU, in `ocelli-volume`, with a hand-computed point citing PS3.3 C.7.6.2.1.1. Two identity volumes 10 mm apart in z, same UID: a point on slice 0 of the first lands on the matching world z of the second. A different UID leaves the second plane index unchanged. The UID cases live on the scene. The arithmetic they call lives on `HeldPlane`.
11. The sync group is one `SyncScene` in `ocelli-viewport`. Section 4 gives that crate the camera. The shell does not own the UID list.

## Problem and reproducer

F-052 samples axial, sagittal and coronal planes of one constructed volume. Nothing moves a second volume's plane because a point lies on the first. `Volume` does not keep the UID `group_volumes` already used. A caller that overlaid two UIDs, or that stepped to the same slice index in both, would show a plausible slice of the wrong patient position. Equal indexes hide it when the origins differ.

Expected observation, identity direction, spacing `1` mm on each axis. Anchor of the first plane `(0, 0, 0)`. Anchor of the second `(0, 0, 10)`. Both start at axial index `0`. Normal `(0, 0, 1)`.

Slice 0 of the first is the plane `z = 0`. Two points on that plane, from PS3.3 C.7.6.2.1.1 with `k = 0`:

```text
P(0, 0, 0) = (0, 0, 0)
P(4, 7, 0) = (0, 0, 0) + 4 * 1 * (1, 0, 0) + 7 * 1 * (0, 1, 0)
           = (4, 7, 0)
```

For the second plane, `plane_origin` is `(0, 0, 10)`. The offset of either point is `(0 - 10) = -10` mm. The in-plane part of `(4, 7, 0)` dots to zero against `(0, 0, 1)`. The new index is `0 + (-10 / 1) = -10`. World z of that index is `10 + (-10) * 1 = 0`.

A sign error yields index `10` and world z `20`. An implementation that adds `x` yields index `-10` for `(0, 0, 0)` and `-6` for `(4, 7, 0)`.

Current behaviour: no such function and no such scene exist.

Source identity: `FrameOfReferenceUid` and `Volume::assemble` in `crates/ocelli-volume/src/lib.rs`. `Volume::voxel_world` in `crates/ocelli-volume/src/ortho.rs`.

## Scope, API and compatibility

In scope:

- `HeldPlane::index_through_point` in `ocelli-volume`.
- One `SyncScene` in `ocelli-viewport`, which stores registrations and calls that function. The formula does not move.
- String equality on the existing `FrameOfReferenceUid`.
- A change list of viewport ids whose index actually changed, so a later frame can mark the existing slice dirt and no other dirt.

Out of scope:

- Pixel fusion, opacity and colormap. F-056.
- Oblique sampling. F-053. This function does not read voxels.
- Thick slab and blend modes. F-054.
- Crosshairs, reference lines and reference cursors. F-088.
- Stack scroll order. F-047. Instance number does not select a slice here.
- Writing `Dirty` inside `ocelli-render`, submitting a queue, or creating a device.
- A wasm-bindgen export. The shell is not given a new boundary type in this story.
- Putting `FrameOfReferenceUid` onto `Volume` or re-checking it in `assemble`.
- Parsing, trimming, or validating the UID against PS3.5.
- Clamping, rounding, or changing `spacing`.
- Annotations, the slide pyramid, and the section 37 CRDT.

```rust
pub struct HeldPlane {
    pub anchor: Pt<World>,
    pub normal: glam::DVec3,
    pub spacing_mm: f64,
    pub index: f64,
}

pub enum PlaneSyncError {
    Unavailable,
}

impl HeldPlane {
    pub fn index_through_point(&self, point: Pt<World>) -> Result<f64, PlaneSyncError>
}
```

`anchor` is the world point of index 0. `index_through_point` returns the new index and does not mutate `self`.

Recommended scene, not approved:

```rust
pub struct SyncScene {
    // one slot per viewport id, in registration order
}

pub struct SliceChange {
    pub id: u16,
    pub index: f64,
}

pub enum SyncError {
    Unavailable,
}

impl SyncScene {
    pub fn register(&mut self, id: u16, uid: Option<FrameOfReferenceUid>, plane: HeldPlane)
    pub fn set_point(&mut self, source: u16, point: Pt<World>) -> Result<Vec<SliceChange>, SyncError>
    pub fn plane(&self, id: u16) -> Option<HeldPlane>
}
```

`register` of an id that is already present replaces that slot's UID and plane and writes no other slot. `set_point` on an id that was never registered is `SyncError::Unavailable` and writes nothing.

On success, every slot whose UID equals the source UID receives the index `index_through_point` returned. The change list contains only ids whose new index is not `==` to the stored index, in registration order. The source is included when its own index changes. A different UID is omitted. A missing peer UID is omitted. `Ok` of an empty list is a successful no-op. `Err` is not an empty list.

`HeldPlane` is `Copy` if its fields are `Copy`. `FrameOfReferenceUid` stays an owned string. The scene stores that string. It does not borrow the caller's temporary.

No existing public function changes its signature. `group_volumes`, `Volume::assemble`, `Volume::reslice` and `Volume::voxel_world` stay as they are. `StackViewport` scroll stays as it is.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `HeldPlane::index_through_point` | anchor, unit normal, spacing, index | CPU. Millimetres. No LUT. No tags | new `f64` index, or `Unavailable` with no write |
| `SyncScene::set_point` (recommended) | `index_through_point` and `FrameOfReferenceUid` equality | Source UID required. Peers of that UID only | indexes committed together, or no write |
| `SyncScene::register` (recommended) | caller-supplied UID and plane | Does not sync | one slot replaced |
| `Volume::voxel_world` | unchanged | Caller may build an anchor from it. This story does not call it inside the formula | no second patient equation |
| `group_volumes` | unchanged | UID still compared only at grouping | `Volume` still has no UID field |
| `Volume::reslice` | unchanged | Not called. Out-of-range index stays stored here | later `PlaneOutOfRange` is not this story |
| `RenderGraph::mark_dirty` | unchanged | Not called. A later owner may set `Dirty.slice` from the change list only | no camera, VOI, segmentation or data dirt from this call |
| `StackViewport` scroll | unchanged | Not a world-point path | stored frame order untouched |
| F-056 fusion | not this type | Later | no shared pixels |
| F-088 crosshairs | not this type | Later | no hit test |

Units: world is LPS millimetres on `Pt<World>`. Spacing is millimetres along `normal`. Index is a continuous count of spacing steps, not a rounded slice number.

Rounding: none. The quotient is the result. The fixture values are exact in `f64`.

Selection: `FrameOfReferenceUid`'s existing string equality. The scene does not parse.

Owner of the transition: `set_point` commits indexes. `index_through_point` owns the quotient and owns no slot.

## State transitions and invariants

The index function is stateless. The scene is stateful. The invariant is that after a successful `set_point`, every registered plane with the source UID contains the point, and every other plane still has its previous index, anchor, normal and spacing. Containment means the offset from that plane origin to the point, along the stored normal, is zero.

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Two axial planes, indexes `0`, anchors z `0` and z `10`, same UID | `set_point` of `(0, 0, 0)` or `(4, 7, 0)` | Second index `-10`. First index stays `0`. Change list is the second id | Anchors, normals and spacing unchanged |
| Same pair plus a third UID | same point | Third index stays `0` and is not in the change list | Not an error |
| Source UID absent | any finite point | `Unavailable` | Every index stays, including peers that have a UID |
| Peer UID absent, source UID present | finite point | That peer stays. Same-UID peers move | Not an error |
| Any plane | non-finite input, spacing not greater than zero, or zero normal | `Unavailable` | No index written. A scene call writes none of its slots |
| After a successful set | the same point again | `Ok` empty change list. Indexes stay | Not a second move |
| After index `-10` | source UID absent | `Unavailable` | Index stays `-10` |
| After that refusal | a new finite point on a present source UID | indexes move to the new quotient | The refused call did not clear the slot |
| Unknown source id | any point | `Unavailable` | No slot created |
| `register` | a new id or a replacement of one id | that slot only | No peer index changes |

Competing route: `register` replacing a slot does not run `set_point`. A stale index in that slot remains until the next set. Scroll commands do not write `HeldPlane`. This story adds no second writer on `StackViewport`.

There is no rendered cache. The settled state is the stored index. A refused request does not replace the last stored plane.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| `HeldPlane` index in a scene slot | `register` copies the caller plane. `set_point` stages one quotient per same-UID slot | viewport `u16`, not the address of the plane | all staged quotients written, then the change list returned | any refusal drops the staged quotients and leaves every slot unchanged |
| `FrameOfReferenceUid` string in the slot | `register` | same `u16` | replacement of that slot only | a failed `set_point` does not clear the UID |
| Change list | `set_point` after the commit | the ids in registration order | the `Ok` value | `Err` returns no list. An empty `Ok` list means no index changed |
| GPU texture, device, queue | not acquired | none | none | this story has no device setter and no browser setter |

`set_point` computes every new index before it writes any slot. A failure on the last peer does not leave the first peer already moved.

An identical second `set_point` commits equal indexes. Equal arguments are a geometric no-op. The change list is empty because each comparison is `==`. That is not a claim about a GPU upload. This story does not upload.

There is no idle image cache to invalidate. Inputs that determine a stored index are the slot anchor, normal, spacing, previous index, the source UID, and the point. `register` of a new anchor on one id invalidates that id only, by replacement, and does not recompute peers.

Recovery sequence: register, successful set, missing-UID refusal, successful set of a different point. The refusal keeps the first result visible to `plane`. The retry replaces it.

`set_point` may allocate the change list and the staged quotients on the command. It is not called from the render loop. The scene `Vec` grows on `register`, not per frame.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Identity axial plane, anchor `(0, 0, 10)`, normal `(0, 0, 1)`, spacing `1`, index `0`. Points `(0, 0, 0)` and `(4, 7, 0)`, both on slice 0 of an anchor at `(0, 0, 0)`. Each returns index `-10`. Hand world z is `10 + (-10) * 1 = 0`. Anchor, normal and spacing are not inputs that the function writes | `crates/ocelli-volume/tests/sync.rs` `matching_world_z` | Sign error returns `10` and world z `20`. Adding `x` returns `-6` for `(4, 7, 0)`. Clamping needs dimensions the signature does not take and would return `0` | implementation. CPU fixture. `ocelli-volume` |
| AC2 | Recommended scene. Same two planes, UID `1.2.3` on both. `set_point` of `(4, 7, 0)` from the first id. Second index `-10` and is the only change. First index stays `0`. A third id with UID `1.2.4` stays at index `0` and is absent from the list. UID `1.2.3 ` with a trailing space also stays | `crates/ocelli-viewport/tests/sync.rs` `same_uid_moves_other_uid_stays`, if the scene lands in this crate | Treating two UIDs as one position. Trimming or parsing the UID. Omitting the peer. Moving the other UID. Marking the unchanged source as a slice change | implementation. CPU. Scene owner is `ocelli-viewport` |
| AC3 | Source slot UID is `None`. `set_point` returns `Unavailable`. The source index and every peer index stay, including a peer whose UID is `1.2.3` | scene test `missing_source_uid_moves_nobody` | Joining a default group. Moving the peers that do have a UID. Returning an empty success list | implementation. Scene owner is `ocelli-viewport` |
| AC4 | Source UID `1.2.3`. One peer UID `None`. That peer's index stays while the same-UID peer moves. The call is `Ok` | scene test `missing_peer_uid_does_not_join` | Refusing the whole call because one peer has no UID. Moving the peer that has no UID | implementation. Scene owner is `ocelli-viewport` |
| AC5 | Non-finite point, non-finite anchor, non-finite normal, non-finite index, spacing `0`, negative spacing, or normal `(0, 0, 0)` returns `Unavailable`. A scene whose second same-UID peer has spacing `0` writes no slot, including the peer whose spacing was valid | volume test `refuses_before_a_quotient` and scene test `stages_then_refuses` | Writing the valid peer before noticing the bad one. Substituting index `0`. Skipping the zero normal and reporting the old index as containment | implementation. Volume half and scene half both run. The scene is `ocelli-viewport` |
| AC6 | After AC2, the same point again returns an empty change list and index `-10` stays. A later point `(0, 0, 10)` sets the second plane's index to `0` (`(point - anchor)` along z is `0`) and sets the first plane's index to `10` (`(0, 0, 10)` against anchor z `0`) | scene test `repeat_is_empty_then_a_new_point_moves` | Applying the offset a second time because the anchor was replaced by the plane origin. Treating `Err` as the empty list. Leaving the source unmoved when the new point is not on its plane | implementation. Scene owner is `ocelli-viewport` |
| AC7 | Sagittal held plane. Anchor `(0, 0, 0)`, normal `(1, 0, 0)`, spacing `1`, index `3`, point `(0, 0, 5)`. New index `0`. Normal stays `(1, 0, 0)`. Hand check: plane x is `0`, so the plane contains the point, and the `5` mm z component is not the offset | volume test `sagittal_uses_its_own_normal` | Using `(0, 0, 1)` yields index `5`. Keeping index `3` leaves the point off the plane | implementation. CPU fixture. `ocelli-volume` |
| AC8 | After AC2, AC3's refusal leaves index `-10` in the peer slot. `register` of a further id does not change the peer. A later successful point then replaces the index | scene test `refusal_and_register_preserve_then_retry` | Clearing the slot on refusal. Copying the new registration's index onto existing peers | implementation. Scene owner is `ocelli-viewport` |
| AC9 | `bin/ocelli.sh test ocelli-volume -- sync` computes AC1 and AC7 with no adapter and no `#[ignore]`. The same expected index is the tier A, B and C result because the function has no tier argument. The scene command is `bin/ocelli.sh test ocelli-viewport -- sync` only if that crate owns the scene | those two test filters | A tier branch that returns a different index on CPU. A GPU-ignored test. A second formula behind a tier check | implementation. Native tests. No external host |

AC2, AC3, AC4, AC6 and AC8 name `SyncScene`. If the operator puts the group on the shell, those observations stay and the test file moves with the group. AC1, AC5's volume half, AC7 and the volume command in AC9 do not move.

## Approach

The smallest implementation is the offset formula on a held plane, then a UID filter in one scene.

`index_through_point` in a new `crates/ocelli-volume/src/sync.rs`:

```text
plane_origin = anchor + index * spacing_mm * normal
offset = (point - plane_origin) dot normal
new_index = index + offset / spacing_mm
```

Refuse before the division when spacing is not finite and greater than zero, when the normal is `(0, 0, 0)`, or when any component of the anchor, normal, index or point is not finite. Return the new index. Do not mutate the plane. Do not read voxels, tags, or a UID.

The recommended `SyncScene` in a new `crates/ocelli-viewport/src/sync.rs` holds a `Vec` of slots. `set_point` looks up the source. A missing source or a missing source UID returns `Unavailable`. It then runs `index_through_point` for the source and for every later slot with the same UID, into a side buffer. Any `PlaneSyncError` drops that buffer. Only then does it write indexes and build the change list.

`ocelli-viewport` does not depend on `ocelli-volume` today. The recommendation adds that dependency so the scene can name `FrameOfReferenceUid` and `HeldPlane` rather than a second UID type. `ocelli-volume` does not depend on `ocelli-viewport`, so the edge does not cycle. `ocelli-wasm` already reaches `ocelli-volume` through `ocelli-render`. This story adds no wasm-bindgen export and does not re-baseline the size budget.

Rejected alternatives:

- Re-check the UID inside `Volume::assemble`. Slice evidence does not carry one. The LLD already says assembly does not.
- Store the current plane origin in `anchor` and also store `index`. The second set walks away from the point.
- Clamp the index into `[0, dim - 1]`. That reports a plane that does not contain the point.
- Parse or trim the UID. That is a second UID type.
- A trait of sync targets. One concrete plane is enough. No second implementer exists today.
- A render-graph pass or a shader. The result is an index.
- Fuse the two volumes. F-056.
- Sort by `InstanceNumber`, `SliceLocation` or `SpacingBetweenSlices`. Those tags are the wrong slice position.

`ortho.rs` stays untouched so F-053 can own oblique sampling there. The new module does not call `reslice`.

## Dependencies, write ownership and resources

Declared dependency: F-052, done. No other S20 story is a backlog dependency. F-053, F-054 and F-056 can run beside this story. They must not share a function body with it.

Source dependencies: `Pt<World>` in `ocelli-core`. `FrameOfReferenceUid` in `ocelli-volume`. `glam::DVec3` already used by the volume crate. `Dirty` in `ocelli-render` is read by a later owner and is not written here.

Write set for the settled arithmetic:

- `crates/ocelli-volume/src/sync.rs`, new.
- `crates/ocelli-volume/src/lib.rs`, one module declaration and the re-export. Do not reorder existing modules. F-053 may also touch this file.
- `crates/ocelli-volume/tests/sync.rs`, new.

Write set for `SyncScene` in `ocelli-viewport`:

- `crates/ocelli-viewport/src/sync.rs`, new.
- `crates/ocelli-viewport/src/lib.rs`, one module declaration.
- `crates/ocelli-viewport/Cargo.toml`, add `ocelli-volume`.
- `crates/ocelli-viewport/tests/sync.rs`, new.

Do not write `crates/ocelli-volume/src/ortho.rs`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-wasm`, shaders, `docs/hld/DEVIATIONS.md`, or `ci/wasm-size-budget.json`.

No GPU and no browser exclusivity. No corpus row. No patient data. Fixture numbers are synthetic. UID strings in tests are `1.2.3` and `1.2.4`, the same shape the volume tests already use.

Safe order: land the volume function and AC1, AC5 and AC7 first. Land the scene after the operator answers where it lives.

## Verification and evidence reuse

Focused checks, from the repo root:

```text
bin/ocelli.sh test ocelli-volume -- sync
bin/ocelli.sh check ocelli-volume
```

If the scene is in `ocelli-viewport`:

```text
bin/ocelli.sh test ocelli-viewport -- sync
bin/ocelli.sh check ocelli-viewport
bin/ocelli.sh check ocelli-wasm
```

The wasm check is only the compile of the new dependency edge. It is not a size measurement and it is not a new export. Do not pass `--accept-size`. Do not edit the size budget to make a later gate green.

No oracle, no corpus, and no adapter. A missing GPU observation does not apply. AC9 is the native test filter above. `#[ignore]` would hide it from the floor.

Final story profile, when the story is implemented, is the sprint gate over the staged tree. This design does not run that gate. A green check of the volume crate does not close the scene rows.

Evidence is invalid when the formula, the UID equality rule, the commit rule, or the test file moves. Completion metadata is not part of this design.

There is no baseline image to update. There is no performance threshold. The command is one index per registered viewport and it is not a frame loop.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. `register` and `set_point` allocate on the command, outside the frame loop. No per-frame buffer
- unsafe: none
- Tier A (WebGPU): full for the plane index. Same quotient on every tier. No device is created and this story does not draw
- Tier B (WebGL2): full for the plane index. Same quotient as tier A. No device is created and this story does not draw
- Tier C (CPU): full for the plane index. Same quotient as tier A. No CPU raster is added. F-X004 remains the CPU display. D-07 does not make this index unavailable, because the work is geometry and it runs on the host

A display refresh, if a later caller needs one, reuses the existing viewport path and sets `Dirty.slice` only for ids in the change list. This story does not create a device and does not call that path. Camera, VOI, segmentation and data stay clear unless some other owner already set them.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1. Index `-10` and world z `0` for two points on slice 0, citing PS3.3 C.7.6.2.1.1. In-plane millimetres do not change the axial index | `crates/ocelli-volume/tests/sync.rs` |
| fixture | AC7. Sagittal index follows the row cosine. The z component of the point is not the offset | `crates/ocelli-volume/tests/sync.rs` |
| refusal | AC5. Non-finite input, bad spacing and a zero normal return `Unavailable` before a quotient | `crates/ocelli-volume/tests/sync.rs` |
| group | AC2 and AC4. Same UID moves. A different string, including a trailing space, stays. A missing peer UID stays | scene test, recommended `crates/ocelli-viewport/tests/sync.rs` |
| refusal | AC3 and the scene half of AC5. Missing source UID, unknown id, and a bad peer abort before any write | scene test |
| duplicate | AC6. The same point returns an empty change list. The anchor is not replaced by the plane origin | scene test |
| recovery | AC8. A refusal keeps index `-10`. A later register does not copy indexes. A later valid point moves the slot | scene test |

The fixture literals are `-10`, `0` and `10`. They are not computed by calling `index_through_point` in the assertion. World z is `anchor.z + index * spacing_mm` written out by hand for the identity axial normal.

Mutating the sign of the offset, or adding the point's `x` into the axial offset, must turn AC1 red. Mutating the sagittal normal to the slice normal must turn AC7 red.

## Parity surface covered

`docs/hld/B-parity-surface.md` has no row named slice sync or frame of reference. The viewport-types row lists twelve types, including `ORTHOGRAPHIC`. This story does not add a viewport type and does not render one.

What this story can establish: two planes that share a `FrameOfReferenceUID` move so each contains one `Pt<World>`, and a second UID does not move. The observation is the CPU index fixture and the scene tests above. It is not a cornerstone pixel diff.

What stays unavailable: tool-class rows, including crosshairs and reference lines (F-088). Blend-mode rows (F-054). Fusion pixels (F-056). A slide-layer annotation in the shared coordinate model (section 35 names the model, and this story does not build the slide). An oracle verdict on a rendered frame (F-X021).

## Deviations

none

## LLD impact

`/complete-feature` step 9 updates:

- `docs/lld/volumes.md`, for `HeldPlane::index_through_point` and for the statement that `Volume` still does not store the UID.
- `docs/lld/viewport.md`, for `SyncScene`.

`docs/lld/render-graph.md` stays unchanged. This story does not write `Dirty` and does not submit a frame. `docs/hld/DEVIATIONS.md` stays unchanged.

## Open questions

None. The consolidated design round was offered on 2026-10-31 and declined. The sync group is one `SyncScene` in `ocelli-viewport`. The crate depends on `ocelli-volume` and calls `index_through_point`. The shell does not own the UID list.
