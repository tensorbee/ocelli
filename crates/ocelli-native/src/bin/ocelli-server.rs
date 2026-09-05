//! The server entry point. HLD section 13's render-target trait separates
//! surface from offscreen texture so that server-side rendering reuses
//! ocelli-render unchanged.
//!
//! Stub. It exists in Phase 1 so that Phase 2 and Phase 3 are new entry points
//! rather than new implementations, and so that F-007's cross-target proof has
//! something that actually LINKS rather than merely type-checks.

fn main() {
    println!("{}", ocelli_native::entry_point_banner("ocelli-server"));
}
