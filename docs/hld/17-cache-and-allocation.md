<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Cache and allocation discipline

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 20. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 20. Cache and allocation discipline

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

- **No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter.

- **Decode into caller-provided buffers** — fn decode(&self, src: &\[u8\], out: &mut \[u8\]), never a Vec returned per frame.

- **Use bytemuck::cast_slice** for reinterpreting pixel buffers. Hand-written transmutes are unsafe code with no upside here.

- **Reuse staging buffers by size class.** Texture uploads should draw from a small pool, not allocate.
