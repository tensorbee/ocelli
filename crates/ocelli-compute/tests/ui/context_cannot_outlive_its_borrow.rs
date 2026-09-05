//! A `ComputeCtx` must not outlive the `GpuContext` it borrows.
//!
//! The lifetime is what stops a kernel retaining a device beyond the dispatch
//! it was given, which is section 31's rule expressed as something the borrow
//! checker enforces rather than something a reviewer has to notice.
//!
//! Both values arrive as parameters. An earlier draft of this case built them
//! with `unimplemented!()` and **compiled**, because a diverging initialiser
//! makes the rest of the function unreachable and the borrow checker never
//! runs. It passed as a compile-fail case that did not fail, which is the
//! exact shape of a test that proves nothing.

use ocelli_compute::ComputeCtx;
use ocelli_render::GpuContext;

fn escape<'a>(gpu: GpuContext, encoder: &'a mut wgpu::CommandEncoder) -> ComputeCtx<'a> {
    // `gpu` is owned by this function and dropped at the end of it, so the
    // returned context would reference a dead local.
    ComputeCtx::new(&gpu, encoder)
}

fn main() {}
