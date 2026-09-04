# F-007 implementation review, pass 1

**Scope**: the working diff for F-007 (E1.7), cross-target build proof, native
desktop and server binary.
**Result**: 0 defects, 0 smells, 1 nitpick.

The two defects this story could have shipped were caught before the code was
written, by checking the plan's factual claims against the tree rather than
implementing them. Both are recorded under `## Where the plan was wrong` below
rather than as review defects, because they never reached the diff.

## Where the plan was wrong, and what was done instead

`/implement-feature` says the plan is authoritative on decisions and
**verifiable on facts**. Two of its facts were false.

### 1. "`cargo tree -p ocelli-native --target wasm32-unknown-unknown` must not resolve"

It resolves. Nothing declared the crate native-only, so cargo has no reason to
refuse it, and `cargo check -p ocelli-native --target wasm32-unknown-unknown`
**succeeded** before this story. A guard asserting the crate does not build for
wasm32 would have been asserting something untrue, which is the worst kind of
guard: green, and green about nothing.

**Adapted with the plan's intent**, which was to make section 4's `wasm: no`
cell mean something. The crate now carries

```rust
#[cfg(target_arch = "wasm32")]
compile_error!("ocelli-native is native-only. HLD section 4's crate table ...");
```

so the cell is true rather than asserted. Observed: `cargo check -p
ocelli-native --target wasm32-unknown-unknown` now exits 101 with that message.

### 2. "compare the resolved feature sets across targets"

The plan assumed the two package sets are otherwise comparable. They are not.
The wasm32 tree legitimately carries eleven packages the host tree does not,
being `wasm-bindgen` and its proc-macro plumbing. A naive set comparison would
have reported eleven differences on day one and been re-baselined immediately,
which is how a guard stops meaning anything.

**Adapted**: the check reports three categories separately, a shared package
whose features differ, a package present on one target only, and a declaration
that no longer describes anything. Each fails, and each says something
different.

## Nitpick

### N1. Step 3 overlaps the `clippy` and `test` gates

`cargo check --workspace --all-targets` re-checks natively what those two gates
already compile. It is kept because it is the explicit assertion of section 4's
`native: yes` column, and because the `native` gate should be runnable and
meaningful on its own rather than only as part of a floor run. Cheap, since the
build is cached. Does not block.

## What was checked and found clean

- Every `as` cast: there are none in this diff.
- Arithmetic: none. No pixel and no coordinate, so HLD 27.2 R3 does not apply.
- `unsafe`: none added.
- `unwrap`, `expect`, `panic`: none. The binaries print and return.
- The extension point list matches `docs/hld/10-extension-points.md` section
  13 in the document's order: SeriesSource, render target, dynamic codec
  registry, calibrated display presentation. Checked against the file, not
  against memory.
- Would the tests fail if the code were wrong: yes, proved by mutation.

## The four mutations

| Mutation | Expected | Observed |
|----------|----------|----------|
| `entry_point_banner` emits only the first extension point | banner test red | `left: 1  right: 4` |
| A `std`-only dependency added to `ocelli-core` | step 2 red | `wait-timeout` fails to compile for wasm32, gate exits 101 |
| `glam` given `scalar-math` under a wasm32 target gate in `ocelli-core` | step 4 red | `host ['libm']  wasm32 ['libm', 'scalar-math']` |
| A package declared wasm32-only that is not present | step 4 red | `serde is declared as wasm32-only and no longer is` |

All reverted, and `bin/ocelli.sh native` is green after the reverts.

**One mutation attempt failed to prove anything and was replaced.** Adding
`rand = { default-features = false }` to the workspace and to `ocelli-core` was
meant to create a per-target feature difference. It did not, because the crate
resolved identically on both targets, and the gate correctly stayed green. That
is a mutation that proves nothing rather than a guard that missed something,
and it was replaced with the `glam` `scalar-math` case above, which does create
a real difference. Recording it because a mutation that quietly does not
mutate is exactly how a suite acquires false confidence.
