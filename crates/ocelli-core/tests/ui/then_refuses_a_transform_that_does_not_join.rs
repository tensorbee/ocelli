//! `Transform<Canvas, World>` composes with `Transform<World, Index>` and
//! with nothing else. Chaining it onto a `Transform<Index, World>` is the
//! composition HLD section 16 says must stop compiling.

use glam::DMat4;
use ocelli_core::{Canvas, Index, Transform, World};

fn main() {
    let canvas_to_world = Transform::<Canvas, World>::from_mat4(DMat4::IDENTITY);
    let index_to_world = Transform::<Index, World>::from_mat4(DMat4::IDENTITY);
    let _ = canvas_to_world.then(&index_to_world);
}
