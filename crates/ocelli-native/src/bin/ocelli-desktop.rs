//! The desktop entry point. HLD section 13 calls calibrated display
//! presentation, DICOM PS3.14 greyscale calibration, "the strongest reason the
//! desktop target exists", and it is unreachable from a web page.
//!
//! Stub. It exists in Phase 1 so that Phase 2 and Phase 3 are new entry points
//! rather than new implementations, and so that F-007's cross-target proof has
//! something that actually LINKS rather than merely type-checks.
//!
//! F-004 gives it one real job: read `OCELLI_TIER`, resolve a tier, and print
//! the evidence. That is the operator override's named user.

fn main() {
    println!("{}", ocelli_native::entry_point_banner("ocelli-desktop"));
    println!();
    match ocelli_native::resolve_tier() {
        Some(resolution) => println!("{}", ocelli_native::tier_report(&resolution)),
        None => println!("{}", ocelli_native::TIER_UNRESOLVED),
    }
}
