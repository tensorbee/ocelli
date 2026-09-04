//! HLD section 16's claim, checked rather than asserted.
//!
//! > The payoff: `Transform<Canvas, World>` composes with
//! > `Transform<World, Index>` and will not compose with anything else. A
//! > whole class of tool bugs stops compiling.
//!
//! "Stops compiling" is not observable from a test that runs. Each case in
//! `tests/ui/` is a program that must fail to build, paired with the exact
//! diagnostic it must produce. A change that quietly erased the space
//! parameters would leave every other test in this crate green and would turn
//! these red, which is the only reason they are here.

#[test]
fn mixing_spaces_does_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
