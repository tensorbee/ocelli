# F-036, Chunked residency model and brick addressing in the cache

**Status**: approved
**Epic ref**: E5.6
**Sprint**: S12
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/17-cache-and-allocation.md`, section 20, in full

```rust
pub trait Budgeted { fn bytes(&self) -> usize; }
pub struct Lru<K, V: Budgeted> {
    budget: usize,
    used: usize,
    /* ... */
}
impl<K, V: Budgeted> Lru<K, V> {
    /// Returns the entries evicted to make room, so the caller can emit events.
    pub fn insert(&mut self, k: K, v: V) -> Vec<(K, V)> { /* ... */ }
}
```

### `docs/hld/06-memory-and-cache.md`, section 8, in full

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |
| Volume assembly is progressive by default: the viewport renders a partial volume and refines as slices land, which matters more for perceived speed than any decode optimisation. The absence of a garbage collector is the real memory story - a 300 MB volume load has no pause behaviour to tune around, only a budget to respect. |

### `docs/hld/27-phase1-hooks.md`, section 38, the row this story is

| **Hook** | **Story** | **Now** | **Later** |
|----|----|----|----|
| Chunked residency in the cache | E5.6 | 4 wk | Re-architecting the cache and every consumer of it |

### `docs/hld/16-volume-representation.md`, section 19, in full

```rust
pub struct Volume {
    pub dims: [u32; 3],
    pub spacing: [f64; 3], // mm
    pub origin: Pt<World>, // IPP of the first slice
    pub direction: glam::DMat3, // derived from ImageOrientationPatient
    pub voxel: VoxelKind, // U8 | I16 | U16 | F32
    pub data: VoxelStore, // one contiguous allocation, x fastest
    pub levels: Vec<Level>, // multiscale; level 0 is full resolution
    pub present: bitvec::BitVec, // per-slice, for progressive assembly
}
```

|  |
|----|
| **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n * dims[0] * dims[1] * bytes_per_voxel. Do not introduce a per-slice Vec; the whole point is a single upload region. |
| **Progressive assembly.** Clear the 3D texture at creation and upload slices as they land. present drives the loading indicator and tells the oracle which frames are comparable yet. |
| **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick; §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford. |

### `docs/hld/05-rendering.md`, section 7, the bricking bullet

|  |
|----|
| **Bricking above 256 MiB.** A 512x512x600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation. |

### `docs/hld/11-decision-log.md`, section 14, decision D11

| **#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D11 | Chunked residency is the default path | Brick only above a size threshold | Bounded GPU memory becomes a claim, not a fallback |

Decision D11 is not deviation D-11. Deviation D-11 in `docs/hld/DEVIATIONS.md` pins the cornerstone3D oracle version at 5.8.2 and says nothing about bricking. The hyphen is the whole distinction, and this plan cites D11 without one wherever it means the chunked default.

### `docs/hld/26-differentiating-capabilities.md`, section 30, the brick bullets

|  |
|----|
| **Residency** is the view frustum intersected with the current level of detail, recomputed per frame against a byte budget. GPU memory becomes a configured number rather than a function of series size. |
| **Level selection** compares projected voxel size against screen pixel size. |
| **Bricks** are 128³, or 64³ for sixteen-bit data, addressed through a 3D indirection texture. The cache is ocelli-cache with a brick key - no second cache. |
| **Prefetch** runs along the scroll axis, which is where the next request almost always is. |
| **Time is another chunk axis.** This is what makes 4D a first-class data model instead of a special case of the memory problem. |

Section 30 is the Part III consumer this hook exists for. This story builds the addressing and the residency bookkeeping it will select over. It does not build frustum selection, level selection, prefetch or the indirection texture, and Approach section 5 says where each of those lands instead.

### `docs/hld/23-performance-rules.md`, section 26, in full

|  |
|----|
| No allocation in the render loop. |
| One queue.submit() per frame. |
| Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload. |
| Batch pointer events into one command buffer per animation frame. Never cross the boundary per event. |
| Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer. |

### What the tree says today

`crates/ocelli-cache/src/lib.rs` exports `Budgeted`, `Lru`, `Admission`, `CacheTier` and `Pressure` from F-031 and holds no value type. `docs/lld/cache.md` records the gap in one line, that there is no brick addressing and that it is F-036. `crates/ocelli-volume/src/lib.rs` is still the F-001 scaffold, so the `Volume`, `VoxelStore`, `VoxelKind` and `Level` types section 19 names do not exist yet and arrive with F-033 in this same sprint. Brick addressing shares those types, which is why this plan is written against the F-033 draft rather than against the scaffold.

## What the specification does not cover

1. **What a brick key is.** Section 30 says the cache is `ocelli-cache` with a brick key and names no fields. Section 19 gives a volume `dims`, a `voxel` kind and a `levels` axis, and none of them is a key. This plan defines the key in Approach section 1.
2. **How 128 cubed and 64 cubed reconcile.** Section 19 says 128 cubed with no qualification. Section 30 says 128 cubed, or 64 cubed for sixteen-bit data. Neither says what eight-bit, thirty-two-bit float or a future labelmap kind takes, and neither says whether the edge is a property of the voxel kind or of the volume. See Open question A.
3. **Where the brick types live.** Section 4 puts volume assembly in `ocelli-volume` and the budgeted LRU in `ocelli-cache`, and section 30 puts a brick key in `ocelli-cache`. A brick is both a subvolume of a `Volume` and an entry in a cache, and the HLD never draws that dependency edge. See Open question B.
4. **What a resident brick holds.** No GPU texture type exists yet, because F-040 is S13. A residency entry in this story can hold host bytes ready to upload or it can hold only the fact of residency. See Open question C.
5. **Whether time is in the key now.** Section 30 says time is another chunk axis. F-124, E30.6 in S45, is the 4D story, and no volume type in Phase 1 carries a time axis. See Open question D.
6. **What selective residency means in Phase 1.** Section 19 says Phase 1 uploads every brick and section 30 makes residency selective. So the residency set in this story records what is resident and what is missing, and nothing in Phase 1 evicts a brick to honour a view frustum. The budget still evicts, through `Lru`, and that is the only selection this story implements.
7. **The indirection texture.** Section 30 addresses bricks through a 3D indirection texture and gives no layout, no format and no update rule for it. F-043, E6.7 in S14, is chunked brick residency as the default upload path, and the texture belongs with the upload path that writes it. See Open question E.
8. **Edge bricks.** A volume whose dimensions are not a multiple of the brick edge has partial bricks on three faces. No section says whether a partial brick is stored at full edge size with cleared padding or at its true extent. Approach section 2 takes the true extent, because `Budgeted::bytes` must report what the entry costs and padding nobody reads is cost reported wrong.

## Approach

### 1. `BrickKey`, the key section 30 asks for

```rust
/// One brick of one volume at one level of detail.
/// The ordering is the `BTreeMap` ordering `Lru` is built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BrickKey {
    pub volume: VolumeId,
    pub level: u32,
    pub brick: [u32; 3],
}
```

`VolumeId` is a `u64` newtype, on the same reasoning F-032 gives for its `SeriesId`, that a key compared per brick on the upload path carries no string. Which study or series a `VolumeId` names belongs to whoever owns a session, and this story provides the newtype and not the assignment.

`level` is in the key from the start even though Phase 1 has one level with content, because section 19 carries the axis in the type from the start and a key that gains a field later is a key whose stored form changes under every consumer. That is the retrofit section 38 prices.

There is deliberately no time field. See Open question D.

### 2. Brick addressing, the arithmetic this story owns

```rust
/// The brick decomposition of one level of one volume.
/// Pure arithmetic over `dims`. No allocation and no I/O.
pub struct BrickGrid {
    pub dims: [u32; 3],
    pub edge: u32,
    pub grid: [u32; 3],
}

impl BrickGrid {
    pub fn new(dims: [u32; 3], voxel: VoxelKind) -> Self;
    /// The brick containing a voxel, one component per axis.
    pub fn brick_of(&self, voxel: [u32; 3]) -> [u32; 3];
    /// The voxel offset of a brick origin within the level.
    pub fn origin_of(&self, brick: [u32; 3]) -> [u32; 3];
    /// The true extent of a brick, clipped at the volume faces.
    pub fn extent_of(&self, brick: [u32; 3]) -> [u32; 3];
    /// The linear brick index, x fastest then y then z.
    pub fn index_of(&self, brick: [u32; 3]) -> usize;
    /// The volume byte offset of a voxel, section 19 layout.
    pub fn byte_offset(&self, voxel: [u32; 3], bytes_per_voxel: usize) -> usize;
}
```

Every function is one rule stated once. `grid` is a ceiling division per axis. `brick_of` is a floor division per axis. `origin_of` is a multiplication per axis. `extent_of` is the smaller of the edge and what remains on that axis. `index_of` follows section 19 layout, x fastest then y then z, so a brick index and a voxel offset are the same convention read at two scales. `byte_offset` is section 19 layout sentence applied to one voxel, `((z * dims[1] + y) * dims[0] + x) * bytes_per_voxel`, and it exists here so the brick extraction in section 3 and any later upload path compute a source offset in one place.

Checked arithmetic throughout, with `u64` intermediates widened before the products and narrowed once at the end, because `dims` of 512 by 512 by 600 at two bytes is 314,572,800 and the intermediate products are exactly where a `u32` wrap would hide. An out-of-range voxel or brick is an error, never a wrapped index. The error type is Open question F, and no `as` cast narrows without a checked conversion, which HLD section 27.3 makes a human review item.

### 3. `Brick`, the value the cache holds

```rust
/// The voxels of one brick, x fastest, at true extent.
/// One contiguous allocation cut from the volume layout of section 19.
pub struct Brick {
    pub extent: [u32; 3],
    pub samples: Vec<u8>,
}

impl Budgeted for Brick {
    /// The allocated buffer, which is what this entry costs its tier.
    fn bytes(&self) -> usize { self.samples.len() }
}
```

Cutting a brick out of a `Volume` is a row by row copy, because a brick is contiguous in x and strided in y and z inside the volume allocation. That copy happens on the decode and assembly path when a slice lands, never in the render loop, and it is the only place the volume layout and the brick layout meet.

`bytes` is the buffer length and not a computed `edge * edge * edge * bytes_per_voxel`, for the reason F-032 states for its frame type, that the buffer is the allocation and a computed figure can disagree with it at an edge brick. A fixture asserts the two agree for an interior brick and that `bytes` follows the buffer at an edge brick, where they do not agree.

### 4. The residency model, which is `Lru` and not a second cache

```rust
pub struct BrickCache {
    bricks: Lru<BrickKey, Brick>,
}

impl BrickCache {
    pub fn new(budget: usize) -> Self;
    pub fn insert(&mut self, key: BrickKey, brick: Brick) -> Admission<BrickKey, Brick>;
    pub fn get(&mut self, key: &BrickKey) -> Option<&Brick>;
    pub fn remove(&mut self, key: &BrickKey) -> Option<Brick>;
    pub fn pressure(&self) -> Pressure;
    /// The bricks of one level that are not resident.
    pub fn missing(&self, volume: VolumeId, grid: &BrickGrid) -> Vec<BrickKey>;
    /// Whether every brick of one level is resident.
    pub fn is_resident(&self, volume: VolumeId, grid: &BrickGrid) -> bool;
}
```

Section 30's sentence that the cache is `ocelli-cache` with a brick key and no second cache is the whole design. `BrickCache` is one `Lru` on `CacheTier::Gpu`, because a resident brick is a thing whose cost is GPU residency even while its bytes are still host side, and giving it the decoded tier would let bricks and frames evict each other against one budget that describes neither resource. See Open question C for the tier half of that choice.

`missing` enumerates the grid in `index_of` order and returns the keys `get` would miss, without touching recency, because a residency query is telemetry in the same sense F-031 established for `pressure` and `contains_key`, and a query that bumped recency would change what the budget evicts next. `is_resident` is the completeness question progressive assembly asks, and it is what tells a viewport a level is comparable yet, in the sense section 19 gives `present` for slices.

**Chunked is the only path.** There is no size threshold, no whole volume fast path and no branch that asks how big the volume is before decomposing it. A volume of one brick decomposes to a grid of one and flows through the same code as a volume of five hundred, which is decision D11 stated as a control flow fact. The one brick case is a test row, not a special case.

### 5. What this story deliberately does not build

- **No frustum or LOD selection.** Section 30 recomputes residency from the view frustum per frame, and that needs a camera, a viewport and a render graph. F-120, E30.2 in S43, is frustum and LOD chunk selection. This story gives it `BrickGrid`, `BrickKey` and `missing` to select over.
- **No prefetch.** Section 30 prefetches along the scroll axis. F-122, E30.4 in S44, owns residency budget, prefetch and eviction policy.
- **No indirection texture and no GPU upload.** F-040 is the texture upload path and F-043 makes chunked residency the default upload path. This story ends at host bytes with an address.
- **No multiscale content.** F-058, E8.8 in S19, is the multiscale pyramid hook, and F-119, E30.1 in S42, generates the pyramid. `level` is carried in the key and in the grid, and level 0 is the only level with bricks.
- **No eviction events.** Eviction surfacing is F-032 for the decoded tier and F-034 for pressure telemetry. An `Admission` from `BrickCache::insert` is returned to the caller unchanged, and what that caller emits is that caller's story.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. A `Brick` holds voxels and never crosses, and a `BrickKey` is three coordinates and two identifiers. Decision D3
- Render-loop allocation: none. Every addressing function in Approach section 2 is arithmetic over a precomputed `BrickGrid`. `missing` allocates its result vector and is an assembly and load path call, never a per frame call. `insert` allocates as `Lru::insert` does, which `docs/lld/cache.md` already measures, and insertion happens when a slice lands
- unsafe: none. `bytemuck::cast_slice` is the only reinterpretation a later upload path needs, which is section 20 third bullet, and this story reinterprets nothing
- Tier A (WebGPU): full. Brick addressing and residency bookkeeping are host arithmetic and are identical on every tier. A 128 edge brick is inside tier A 3D texture limit of 2048 on every axis
- Tier B (WebGL2): full, with the limit named. Tier B conservative 3D texture floor is 256 per section 7, and a 128 edge brick is inside it, so no brick needs splitting for tier B. A volume dimension above 256 is a grid of bricks and not one texture, which is the point of the decomposition. If Open question A lands an edge above 128 for any voxel kind, that row must be rechecked against 256 before it is approved
- Tier C (CPU): full. A tier C session assembles volumes and reslices them on the CPU under deviation D-07, and it decomposes and addresses bricks with the same code. Its GPU tier cache simply holds host bricks that no upload consumes, and residency still answers which bricks a reformat needs

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | **The brick containing a voxel.** A 512 by 512 by 600 volume at edge 64 has a grid of 8 by 8 by 10, hand-computed as ceiling divisions, and voxel column 200, row 130, slice 599 sits in brick 3, 2, 9. The dimensions are `Columns` (0028,0011), `Rows` (0028,0010) and the slice count of the assembled stack. PS3.3 C.7.6.3.1 for the Image Pixel module and C.7.6.2.1.1 for the voxel to patient ordering the axes follow | `crates/ocelli-cache/tests/bricks.rs` |
| `fixture` | **A brick origin and its byte offset agree.** In the same volume at two bytes per voxel, brick 3, 2, 9 starts at voxel 192, 128, 576, and section 19 layout puts that voxel at byte offset ((576 * 512 + 128) * 512 + 192) * 2, hand-computed above the assertion. PS3.3 C.7.6.2.1.1 | same |
| `fixture` | **An edge brick reports its true extent and its true bytes.** A 500 by 300 by 130 volume at edge 128 has a grid of 4 by 3 by 2, and the last brick on each axis has extent 116, 44 and 2. At one byte per voxel that brick holds 116 * 44 * 2 samples, hand-computed, and `bytes` equals the buffer and not 128 cubed. PS3.3 C.7.6.3.1 | same |
| `fixture` | **A one brick volume is the same path.** A 64 by 64 by 40 eight-bit volume at edge 128 is a grid of 1 by 1 by 1, `missing` returns that one key before insert and nothing after, and no threshold branch exists to take. Decision D11, and the fixture is the claim | same |
| `fixture` | **The sixteen-bit edge.** A 512 by 512 by 600 sixteen-bit volume decomposes at edge 64 into 8 by 8 by 10 bricks of 524,288 bytes each, hand-computed as 64 cubed times two. The whole volume is 314,572,800 bytes against a 256 MiB budget of 268,435,456, which is section 7 bricking figure and the reason the grid exists. PS3.3 C.7.6.3.1, `Bits Allocated` (0028,0100) | same |
| `unit` | Ceiling division rounds up on a partial axis and does not add a brick on an exact one, one test per axis so a failure names the axis | same |
| `unit` | `index_of` is x fastest, so brick 1, 0, 0 follows brick 0, 0, 0 and brick 0, 1, 0 follows the whole first row of bricks, matching the voxel layout at brick scale | same |
| `unit` | An out-of-range voxel and an out-of-range brick are errors and never wrapped indices, at one past the end on each axis | same |
| `unit` | `missing` returns keys in `index_of` order and returns fewer after each insert, and asking does not change what a later budget eviction removes | same |
| `unit` | Eviction under budget pressure removes the least recently used brick and returns it in `Admission::evicted`, and `is_resident` goes false for that level in the same step | same |
| `unit` | An oversized single brick arrives as `Admission::refused` with nothing evicted, which is the case a brick edge too large for the budget produces | same |
| `property` | For randomised dimensions, edges and voxel kinds, every voxel maps to a brick whose origin is at or below it and whose origin plus extent is above it, and the extents of one axis sum to that axis dimension exactly | same |
| `property` | For a randomised insert and get sequence over bricks, the cache pressure used equals the sum of the live bricks admitted bytes and never exceeds budget, which is F-031 invariant restated over this value type | same |

**Mutation check, HLD 27.3.** Divide with rounding down instead of ceiling and the edge brick fixture loses its last brick. Report full edge bytes at an edge brick and its fixture goes red. Make `index_of` z fastest and its unit test goes red. Add a whole volume path above a size threshold and the one brick fixture still passes, so a dedicated test asserts no public constructor or method branches on total volume size, and the threshold mutation is caught by review against decision D11 rather than by a green suite. Widen an intermediate product in `u32` and the 512 by 512 by 600 fixture wraps and goes red.

## Parity surface covered

None of `docs/hld/B-parity-surface.md` rows. Its surfaces are viewport types, tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation representations, core events and adapters. Brick addressing underlies the volume viewports and is none of them, and the appendix carries no `Covered by` column in this repository to update. Section 30 bounded residency claim is a Part III differentiator rather than a parity row, and this story is its Phase 1 hook under section 38.

## Deviations

None expected, and no new row is added to `docs/hld/DEVIATIONS.md` by this plan.

Decision D11 is followed rather than departed from, and it is a decision in `docs/hld/11-decision-log.md` section 14 and not deviation D-11. Sixteen-bit bricks at edge 64 follow `docs/hld/26-differentiating-capabilities.md` section 30 exactly, so the reconciliation in Open question A takes no row if it lands as recommended. If Open question B is answered by moving the volume types, or Open question D by adding a time axis the HLD does not type, the answer that departs from a transcribed section takes a `D-NN` row in the same change and this section is updated before the plan is approved.

## LLD impact

- `docs/lld/cache.md`, which gains the brick key, the brick grid arithmetic, the brick value type and the residency queries, and loses the line that says brick addressing is not here
- `docs/lld/volumes.md`, which F-033 creates, gains the note that a volume enumerates its bricks through `BrickGrid` rather than carrying a second decomposition of its own

## Open questions

None. All questions were answered in the S12 consolidated design round, as recorded below.

## Decisions from the S12 design round

**A, A1. Brick edge is a function of bytes per voxel.** Eight-bit at 128, sixteen-bit at 64, four-byte float at 64. One match in `BrickGrid::new` is the only place the number lives.

**B, B1. Addressing and residency live in `ocelli-cache`** in a `brick` module over `[u32; 3]` and a sizing voxel kind defined there. `ocelli-volume` depends on `ocelli-cache` and converts.

**C, C1. Bricks sit in `CacheTier::Gpu`** in their own `Lru`, with `bytes` as the host buffer length today. F-040 removes the host brick in the step that uploads it, so one brick is never counted against two budgets.

**D, D1. No time field in the brick key now.** F-124 adds it in Part III.

**E, E1. No indirection slot mapping in this story.** Residency is a set of keys present or missing.

**F, F1. A small `BrickError` enum** with `VoxelOutOfRange` and `BrickOutOfRange`, each carrying axis and value.

## Recorded questions

**A. What edge does each voxel kind take.** Section 19 says 128 cubed and section 30 says 128 cubed, or 64 cubed for sixteen-bit data. At two bytes per voxel a 128 brick is 4,194,304 bytes and a 64 brick is 524,288 bytes, and at one byte a 128 brick is 2,097,152 bytes. Section 30 figure is the later and more specific statement, and it is the one Part III builds on.

- **A1, this plan recommendation.** The edge is a function of bytes per voxel. One and two byte kinds follow section 30 by name, with eight-bit at 128 and sixteen-bit at 64. Four byte float takes 64, on the reasoning section 30 is actually making, that a brick is a bounded byte count and not a bounded voxel count, which puts a float brick at 1,048,576 bytes. The function is one match in `BrickGrid::new` and is the only place the number lives
- **A2.** Edge 128 for every kind, following section 19 alone. Simpler, and it departs from section 30 sixteen-bit sentence, so it takes a `D-NN` row and quadruples the sixteen-bit brick the residency budget has to place
- **A3.** The caller passes the edge. Every consumer then chooses, two viewports can decompose one volume differently, and the brick key stops identifying a unique cut of the volume

**B. Where the brick types live, and which crate depends on which.** `ocelli-volume` is a scaffold with no dependencies and `ocelli-cache` is `no_std` over `alloc` with no dependencies. A brick is a cut of a volume and an entry in a cache, and F-033 writes the volume types in this same sprint, so the edge is drawn now or it is drawn by whichever story lands second.

- **B1, this plan recommendation.** Addressing and residency live in `ocelli-cache` in a `brick` module, over plain `[u32; 3]` dimensions and a voxel kind this crate defines for sizing. `ocelli-cache` gains no dependency. `ocelli-volume` depends on `ocelli-cache` and asks `BrickGrid` how its dimensions cut, which is the direction section 30 already states, that the cache is where the brick key lives. The cost is two voxel kind types, one in each crate, joined by a conversion in `ocelli-volume`. That cost is real and is named here rather than discovered
- **B2.** Put `BrickGrid` and `Brick` in `ocelli-volume` and let `ocelli-cache` stay generic, with `BrickCache` also in `ocelli-volume` over an `Lru` it imports. The volume types sit together and the cache crate stays untouched, and section 30 sentence about `ocelli-cache` with a brick key is then true only in the generic sense
- **B3.** `ocelli-cache` depends on `ocelli-volume` for `Volume`, `VoxelKind` and `VoxelStore`. One voxel kind type, and a cache crate that now understands volumes, which is the coupling `docs/lld/cache.md` says the crate exists to avoid when it states that it holds sizes and never contents it understands

**C. Which tier holds a brick, and what its bytes mean before any texture exists.** Section 8 gives three tiers. A brick in this story is host bytes cut for a GPU residency that F-040 has not built yet, so it is arguably decoded data with a GPU destination.

- **C1, this plan recommendation.** `CacheTier::Gpu`, in its own `Lru` with its own budget, and `bytes` is the host buffer length today. When F-040 uploads a brick, the uploaded texture entry reports its allocated footprint under the rule `docs/lld/cache.md` already states, and the host brick is removed in the same step, so one brick is never counted against two budgets at once. The tier names the resource the residency decision is about, which is what `Pressure` is read for
- **C2.** `CacheTier::Decoded`, alongside frames in a separate `Lru`. Honest about where the bytes are today, and it makes the GPU budget report nothing about the residency it is meant to bound, which is the claim decision D11 makes checkable

**D. Is time in the brick key now.** Section 30 says time is another chunk axis, and F-124 in S45 builds 4D on it. No Phase 1 volume type has a time dimension.

- **D1, this plan recommendation.** No time field. Adding one now means every key carries a zero nobody reads, and F-124 adding the field is a change to a type Part III owns, priced by section 38 as the multiscale change rather than as this hook. The hook this story is charged with is chunked residency existing at all
- **D2.** A `time: u32` field fixed at zero in Phase 1. The key never changes shape later, and every test and fixture in this story carries a field with one legal value, which is a case a reader has to rule out at every comparison

**E. Does the indirection mapping land now as data, without the texture.** Section 30 addresses bricks through a 3D indirection texture. The texture is GPU state and F-043 owns it, but the mapping from a brick key to a slot is host data this story could define.

- **E1, this plan recommendation.** No slot mapping in this story. Residency here is a set of keys present or missing, and a slot is an upload allocation decision that only means anything beside the texture it indexes. Defining it now invents a numbering F-043 then has to honour or break
- **E2.** A `slot: u32` assigned in `index_of` order at insert and returned from `missing` alongside the key. F-043 inherits a numbering, and this story gains a field whose only reader is four sprints away

**F. What error an out-of-range address returns.** Section 20 gives `ocelli-cache` no error type, and the crate today returns `Option` and `Admission` and panics nowhere. Addressing is the first fallible arithmetic in the crate.

- **F1, this plan recommendation.** A small `BrickError` enum in the `brick` module with one variant per failure, `VoxelOutOfRange` and `BrickOutOfRange`, each carrying the axis and the value. Checked arithmetic needs somewhere to go, and an `Option` would collapse two different caller mistakes into one silence
- **F2.** Return `Option` from the addressing functions, matching the crate existing shape. No new type, and a caller cannot tell a bad voxel from a bad brick, which matters when the caller is assembling from slice evidence it did not validate
