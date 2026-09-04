//! A `GpuContext` must not yield an owned device.
//!
//! Section 31: "Two devices cannot share textures, which would defeat the
//! entire point." A second OWNED handle is how a second device arrives, so
//! `GpuContext` has no accessor that produces one and no `Clone`. Only
//! `device()` and `queue()`, both shared borrows.
//!
//! The context arrives as a PARAMETER rather than from `unimplemented!()`.
//! A diverging initialiser makes everything after it unreachable, and a
//! compile-fail case that only fails because of an error the compiler would
//! have raised anyway is not testing what it claims to.

use ocelli_render::GpuContext;

fn steal(context: GpuContext) {
    // There is no `into_device`. If somebody adds one, this case stops failing
    // and the suite says so.
    let _owned: wgpu::Device = context.into_device();
}

fn main() {}
