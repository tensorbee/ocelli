//! The device-ownership contract, asserted as compile errors.
//!
//! HLD section 31 says `ocelli-compute` never creates a `wgpu::Device` and
//! borrows the one `ocelli-render` owns. A comment saying so is not a
//! mechanism. These cases are, and they need no GPU and no adapter, so they
//! run in the CI floor where the real device never exists.
//!
//! The same trybuild harness F-001 used for coordinate-space mismatches.

#[test]
fn device_ownership_contract_is_a_compile_error_to_break() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
