//! The server entry point. HLD section 13's render-target trait separates
//! surface from offscreen texture so that server-side rendering reuses
//! ocelli-render unchanged.
//!
//! Stub. It exists in Phase 1 so that Phase 2 and Phase 3 are new entry points
//! rather than new implementations, and so that F-007's cross-target proof has
//! something that actually LINKS rather than merely type-checks.
//!
//! F-004 gives it one real job: read `OCELLI_TIER`, resolve a tier, and print
//! the evidence. A server deployment is the estate deviation D-07 is about, so
//! it is the one that most needs to be able to say which signal decided.

fn main() {
    println!("{}", ocelli_native::entry_point_banner("ocelli-server"));
    println!();
    match ocelli_native::resolve_tier() {
        Some(resolution) => println!("{}", ocelli_native::tier_report(&resolution)),
        None => println!("{}", ocelli_native::TIER_UNRESOLVED),
    }
}
