//! `identity` lives in `impl<S> Transform<S, S>`, so a cross-space identity
//! cannot be spelled.
//!
//! Without that narrowing `Transform::<Canvas, World>::identity()` compiles
//! and turns a canvas point into a world point with no arithmetic, no cast
//! and nothing in the diff for a reviewer to stop on. This case is here so
//! that widening the impl back to `Transform<A, B>` turns a test red instead
//! of passing silently.

use ocelli_core::{Canvas, Transform, World};

fn main() {
    let _ = Transform::<Canvas, World>::identity();
}
