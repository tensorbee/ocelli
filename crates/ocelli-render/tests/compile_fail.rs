//! Invariants of this crate that are asserted as compile errors.
//!
//! The same trybuild harness F-001 used for coordinate-space mismatches and
//! F-008 used for the device-ownership contract in `ocelli-compute`. These
//! cases need no GPU and no adapter, so they run in the CI floor that
//! deviation D-04 leaves without one.

#[test]
fn workload_dimensions_are_a_compile_error_to_transpose() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
