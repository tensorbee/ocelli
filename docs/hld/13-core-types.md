<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# Core types, coordinate spaces and value spaces

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 16. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 16. Core types: coordinate spaces

Cornerstone represents canvas points, world points and voxel indices all as number\[\]. Mixing them is a silent, common and expensive bug. Rust can make the mistake impossible at compile time, and this is one of the clearest places the language actually earns its cost.

```rust
// ocelli-core/src/space.rs
use core::marker::PhantomData;
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

*The payoff: Transform\<Canvas, World\> composes with Transform\<World, Index\> and will not compose with anything else. A whole class of tool bugs stops compiling.*

### 16.1 Value spaces

The same trick applies to pixel values, and for the same reason — the LUT chain is a sequence of transformations whose stages are easy to apply out of order.

```rust
// ocelli-core/src/value.rs
#[derive(Clone, Copy, Debug)] pub struct Stored(pub f32); // raw from pixel data
#[derive(Clone, Copy, Debug)] pub struct Modality(pub f32); // after rescale; HU for CT
#[derive(Clone, Copy, Debug)] pub struct Display(pub f32); // after VOI, in [ymin, ymax]
```

*You cannot accidentally window a stored value, and a reviewer can see the stage from the type.*
