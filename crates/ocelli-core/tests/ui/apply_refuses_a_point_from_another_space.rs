//! A `Transform<Canvas, World>` must refuse a `Pt<Index>`.
//!
//! In cornerstone both are `number[]` and this line compiles, runs, and
//! produces a plausible wrong number.
//!
//! Built with `from_mat4` rather than `identity`, because `identity` is
//! `Transform<S, S>` and a cross-space one cannot be spelled that way at all.
//! Naming the matrix is the cost that narrowing imposes, and it is the point
//! of it.

use glam::DMat4;
use ocelli_core::{Canvas, Index, Pt, Transform, World};

fn main() {
    let t = Transform::<Canvas, World>::from_mat4(DMat4::IDENTITY);
    let voxel = Pt::<Index>::new(255.0, 191.0, 0.0);
    let _ = t.apply(voxel);
}
