<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Render graph

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 22. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 22. Render graph

```rust
pub enum Pass {
    Stack(StackPass),
    VolumeRaycast(VolumePass),
    SegOverlay(SegPass),
}
pub struct Caps {
    pub compute: bool,
    pub max_tex_3d: u32,
    pub max_buffer: u64,
    pub tier: Tier, // A = WebGPU, B = WebGL2 downlevel
}
```

- **Pipelines compile at init**, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame.

- **Dirty tracking.** A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.

- **One queue.submit() per frame** across all viewports, from the render worker's requestAnimationFrame.

- **Device loss is a real state, not an error path.** Handle device_lost, rebuild the device and all resources, and restore viewport state from the shell's copy — the same recovery path §23 needs for panics.
