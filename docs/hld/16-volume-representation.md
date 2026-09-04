<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Volume representation

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 19. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 19. Volume representation

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

- **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n \* dims\[0\] \* dims\[1\] \* bytes_per_voxel. Do not introduce a per-slice Vec; the whole point is a single upload region.

- **Progressive assembly.** Clear the 3D texture at creation and upload slices as they land. present drives the loading indicator and tells the oracle which frames are comparable yet.

- **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick; §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford.
