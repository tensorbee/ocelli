<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Codec registry

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 21. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 21. Codec registry

Explicit runtime registration, not the inventory crate — inventory does not work on WebAssembly, which is precisely why dicom-rs's own plugin registry is unavailable there. Explicit registration is also what lets a native build link C codecs the browser build cannot.

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
pub struct Registry { by_ts: HashMap<&'static str, Arc<dyn Decoder>> }
impl Registry {
    pub fn register(&mut self, d: Arc<dyn Decoder>) {
        for ts in d.transfer_syntaxes() { self.by_ts.insert(ts, d.clone()); }
    }
}
```

|  |
|----|
| **TWO OPEN GATES** HTJ2K through openjp2 is registered in dicom-rs but unverified under wasm32 — test it bit-exact against OpenJPH output in week one. JPEG-LS has no credible pure-Rust path; the registry design deliberately allows a JS-side bridge to @cornerstonejs/codec-charls as a registered decoder, so choosing that route costs an adapter rather than a redesign. |
