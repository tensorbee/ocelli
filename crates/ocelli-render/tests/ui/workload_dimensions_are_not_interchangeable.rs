//! A target edge and a pass count must not be interchangeable.
//!
//! `probe::measure_with`'s documentation recorded this as an open residue for
//! nine sprint-review passes: `run` took `edge` and `passes` as two adjacent
//! `u32`s, `measure`'s closure forwarded them positionally, and transposing
//! them compiled and passed the whole suite.
//!
//! It passed because no test reaches that call site, **not** because the
//! numbers agree. `fragments` is `edge * edge * passes`, which is not symmetric
//! in its two arguments: the calibration goes from 65,536 fragments to 256 and
//! the full pass from 16,777,216 to 262,144. `fragments` and `run` are handed
//! the same transposed pair, so the reported numerator matches the trivial
//! workload actually shaded and nothing internal disagrees. The consequence is
//! deviation D-07's misdetection arriving from the direction the resolver
//! exists to catch: a demoted hardware adapter rendering nothing on tier C and
//! presenting as a slow viewer.
//!
//! F-037 took the newtypes `probe::measure_with` named it as the story to
//! argue. This case is what stops a later change from collapsing them back to
//! a pair of integers that happen to be spelled differently.
//!
//! The values arrive as PARAMETERS rather than from `unimplemented!()`, for
//! the reason `no_owned_device_out_of_context.rs` gives: a diverging
//! initialiser makes everything after it unreachable, and a compile-fail case
//! that only fails because of an error the compiler would have raised anyway
//! is not testing what it claims to.

use ocelli_render::probe::{Edge, Passes};

fn transpose(edge: Edge, passes: Passes) -> (Edge, Passes) {
    // The transposition. If `Edge` and `Passes` ever become the same type, or
    // gain a `From` between them, this case stops failing and the suite says
    // so.
    let wrong_edge: Edge = passes;
    let wrong_passes: Passes = edge;
    (wrong_edge, wrong_passes)
}

fn main() {}
