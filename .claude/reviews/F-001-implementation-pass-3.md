# F-001, implementation review, pass 3

**Reviewer**: independent agent, wrote neither the code nor passes 1 and 2
**Diff reviewed**: working tree, base cd74768
**Result**: 0 defects, 1 smell, 7 nitpicks

Every round 2 remediation was treated as new code. Every factual claim in it
was executed rather than read. Every mutation was reverted and the revert
proved with `diff -r -q` and `md5 -q` against a full copy of
`crates/ocelli-core`, `Cargo.toml`, `Cargo.lock` and the plan taken before the
first mutation, not by eye.

## Earlier findings, and whether each is genuinely resolved

### Pass 1 D1, D2, D3, S1, S2 and N1 to N5. All still resolved.

Re-verified rather than inherited from pass 2. The `NOTE` correction at
`space.rs:63-77` is true, both halves independently reconfirmed with standalone
rustc probes against 1.97.1 edition 2024 (probes A and B below). `Cargo.toml`'s
`verbatim` label is corrected and the count in it is right, five entries in HLD
section 15.2 against seven here. The unit conflation is gone from every
coordinate assertion. `identity` is on `impl<S> Transform<S, S>` and mutation
M5 shows the trybuild case fires when it is widened back. The singular-inverse
claim is executed by a test that mutation M6 turns red.

### Pass 2 D1, the 25.1 row-count sentence. RESOLVED, and the new sentence is true.

I read `docs/hld/22-testing-and-tolerance.md` myself rather than taking pass 2's
extract. Section 25.1's geometry bullet is one line with two clauses:

```
- **Geometry:** world coordinates within 1e-6 mm; canvas coordinates within a quarter pixel.
```

`space.rs:216-220` now says the bullet "covers two of them, world and canvas,
with different numbers", and then names the world row as 1e-6 mm, the canvas
row as a quarter pixel, and the absence of an index row. All four statements
check out. The introducing sentence and the list agree.

### Pass 2 D2, the plan's `Transform` `Clone`/`Copy` paragraph. RESOLVED.

`.claude/plans/F-001-design.md:194-196` now reads "for the same reason they are
on `Pt`, which is that the impls are unconditional where a derive would bound
`A` and `B`". That agrees with `space.rs:110-113` and with the corrected D-08
note in the canonical `docs/hld/DEVIATIONS.md`, which says "the reason they
stay has changed and the source says so. It is no longer that a derive would
not compile. It is that `impl<S> Clone for Pt<S>` and `impl<S> Copy for Pt<S>`
are unconditional." Three artefacts, one story.

The correction note attached below it is also accurate. `#[derive(Debug)]` on
`Transform<A, B>` does compile whether or not the markers derive `Debug`, which
I checked by compiling it.

### Pass 2 S1, D-08 exercised by nothing. RESOLVED for the half the deviation record rests on, not for the rest. See the smell below.

`d_08_keeps_debug_and_partial_eq_usable_on_a_point` at `space.rs:281-307` is a
real guard, and I proved it three separate ways rather than reading it.

**1. It fails to compile if D-08 is reverted.** I removed all three
`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` lines, restoring
`pub enum Canvas {}` exactly as HLD section 16 writes it, and changed nothing
else. `cargo test -p ocelli-core` exits 101 with six top-level errors, all of
them inside this one test:

```
error[E0277]: `Canvas` doesn't implement `Debug`
   --> crates/ocelli-core/src/space.rs:288:33
288 |         let rendered = format!("{a:?}");

error[E0369]: binary operation `==` cannot be applied to type `Pt<Canvas>`
   --> crates/ocelli-core/src/space.rs:300:9
300 |         assert_eq!(a, b);
```

E0277 and E0369 are precisely the two codes the D-08 record names. Restored,
`md5 -q crates/ocelli-core/src/space.rs` back to
`6ff3960e000a6c50c60b88f5c75df1fe`.

**2. The `{:?}` assertion inspects the rendered text, it does not merely call
`format!`.** I replaced `#[derive(Debug, PartialEq)]` on `Pt<S>` with a
hand-written `Debug` that writes the string `"Pt"` and nothing else, so the
test still compiles:

```
thread 'space::tests::d_08_keeps_debug_and_partial_eq_usable_on_a_point' panicked at
crates/ocelli-core/src/space.rs:298:9:
Debug for Pt<Canvas> rendered Pt, which does not carry its own fields
```

**3. The `PartialEq` half cannot pass by accident either.** I replaced the
derived `PartialEq` with `fn eq(&self, _other: &Self) -> bool { true }`:

```
panicked at crates/ocelli-core/src/space.rs:312:9:
assertion `left != right` failed
```

so `assert_ne!(a, c)` is load bearing and a degenerate `PartialEq` does not
satisfy the test.

**4. The doc comment's counterfactual is true.** `space.rs:277-280` claims that
without this test "the crate compiles and all of its tests pass with the
derives removed". I removed the three derives **and** deleted the test
function. Every remaining test passed, 23 of 23 green, zero compile errors. The
sentence is measured, not asserted.

### The new trybuild case, and all four ui cases.

All four run and all four are compared against their `.stderr`. From the
pristine tree:

```
test tests/ui/apply_refuses_a_point_from_another_space.rs ... ok
test tests/ui/identity_refuses_to_cast_between_spaces.rs ... ok
test tests/ui/then_refuses_a_transform_that_does_not_join.rs ... ok
test tests/ui/value_spaces_do_not_interconvert.rs ... ok
```

`TRYBUILD` is unset in the environment and nothing in `bin/ocelli.sh` sets it,
so trybuild is in compare mode and not overwrite mode.

**The snapshots are compared, not merely present.** I edited one line of
`apply_refuses_a_point_from_another_space.stderr`, changing `expected
Pt<Canvas>` to `expected Pt<World>`, and trybuild reported `mismatch` with an
EXPECTED and ACTUAL diff, exit 101. Restored, `diff -q -r` silent over all
eight ui files.

**The new case is a live guard.** I collapsed `Modality` into
`pub type Modality = Stored;`, which is the shape of the defect it exists to
catch. The `Stored` to `Modality` error disappeared and a different one took
its place:

```
EXPECTED: error[E0308]: mismatched types ... expected `Display`, found `Modality`
ACTUAL:   error[E0423]: expected function, tuple struct or tuple variant,
                        found type alias `Modality`
test mixing_spaces_does_not_compile ... FAILED
```

### The seven nitpicks round 2 took.

| # | Claim | Verified |
|---|---|---|
| N1 | `DIMENSIONLESS_TOLERANCE` added in `space.rs` | Present at `:233`, used at `:470` on the determinant. The determinant no longer borrows a space unit. See nitpick N1 below for the sentence four lines above it |
| N2 | "twenty five lines above" replaced by something that cannot go stale | True. The text is now "where they are declared above", no line count anywhere in the file |
| N3 | the 86.6 figure moved onto the fixed case | True, and the number is exact. Measured under the `transform_point3` mutation: `x drifted to 9912.435180867995` against 9999.0, so 86.564819 CSS pixels. The comment says 86.5648 |
| N3 | "it has reported drifts of 51.1 and 71.4 on different runs" | **Confirmed.** Eight runs with the regressions file cleared each time: 55.07, 77.61, 18.71, 58.41, 6.49, 71.24, 24.32, 44.03. Run-dependent exactly as claimed, and 71.24 is within a rounding of the reported 71.4 |
| N4 | the plan's compile-fail row names three ui cases | True as far as it goes, but four now exist. Nitpick N5 below |
| N5 | `value.rs` is honest about being a shape check, and the section 16.1 claim is guarded | True on both halves |
| N6 | the progress note's "fourth trybuild case" | There are now genuinely four |
| N7 | `inverse`'s doc qualifies the glam behaviour | True in substance, slightly overbroad. Nitpick N4 below |

## New defects

None.

## New smells

### S1, the D-08 guard covers two of the deviation's six derives, and the two it does not cover are the ones this file's prose depends on

**Where**: `crates/ocelli-core/src/space.rs:33`, `:37`, `:47`, and the guard at
`:281-307`

**What**: D-08 gives the three markers the derive list
`Debug, Clone, Copy, PartialEq, Eq, Hash`. The guard exercises `Debug` and
`PartialEq`. It does not reach `Clone`, `Copy`, `Eq` or `Hash`. I trimmed all
three markers to `#[derive(Debug, PartialEq)]`, a partial revert of the
deviation, and changed nothing else:

```
$ cargo test -p ocelli-core
test result: ok. 14 passed  (lib, including d_08_keeps_debug_and_partial_eq_usable_on_a_point)
test result: ok.  1 passed  (compile_fail, all four ui cases)
test result: ok.  6 passed  (geometry)
test result: ok.  3 passed  (roundtrip)
exit 0
```

24 of 24 green with four of the six derives deleted.

**Why it will cause a defect**: this file states twice that the markers having
`Clone` and `Copy` is load bearing, and neither statement is reachable by
anything that runs.

- `space.rs:22-26`, the module's D-08 block: "giving the markers `Clone` and
  `Copy` also retires the trap section 16's own `NOTE` describes."
- `space.rs:66-71`, above `impl<S> Clone for Pt<S>`: "D-08 gives `Canvas`,
  `World` and `Index` `Clone` and `Copy` where they are declared above, so they
  do satisfy that bound now and the derived form compiles for all three."

Both become false the moment someone deletes `Clone, Copy` from the marker
list, and nothing goes red when they do. The canonical `docs/hld/DEVIATIONS.md`
row would also stop describing the tree, and the `deviations` gate does not
catch it, because that gate checks only that citations resolve and that D-01
matches `Cargo.toml`, which I confirmed by reading its output on the reverted
tree.

This is the shape pass 2 graded a smell, narrowed rather than closed. It is
also microscope section 4's case for the four derives it does not reach: a
thing that is present, looks authoritative, and is never reached. The failure
mode is concrete and it is the same one this story has already produced twice,
a comment describing the world before the deviation was applied.

It is cheap to close. One line in the test module, for instance

```rust
fn assert_marker_bounds<T: Copy + Eq + core::hash::Hash>() {}
```

called once per marker, turns all six derives into a guard. I am not fixing it.

**Evidence**: the partial-revert run above. Reverted, `diff -r -q` silent
against the baseline copy.

## Nitpicks

### N1, `SPACE_UNIT_TOLERANCE`'s doc still claims universality four lines above the constant that contradicts it

`space.rs:209-210` opens with "The bound every assertion below uses, in the
units of whichever space the point is in." `DIMENSIONLESS_TOLERANCE` at
`:230-233` then says it is "The bound for quantities that are not points and
have no space", and the determinant assertion at `:470` uses it. So not every
assertion below uses the first constant. The N1 remediation added the second
bound and did not update the sentence that claims there is only one. Nothing is
at risk today. The concrete cost is a reader adding a dimensionless assertion,
trusting "every assertion below", and reaching for the space unit, which is the
borrowing the second constant exists to prevent.

### N2, the guard calls `Debug` "a separate requirement from the one `assert_eq!` imposes", and `assert_eq!` imposes `Debug` too

`space.rs:288-290`. `assert_eq!` requires both `PartialEq` and `Debug`, which
the same test says correctly eleven lines later at `:297-299`. So the `{:?}`
call is not a separate **trait** requirement. What it genuinely adds is
behavioural rather than bound-related, and it is real: my mutation to a
field-free `Debug` turned the `contains` assertion red while `assert_eq!` stayed
green. The test is stronger than the sentence claims, and the sentence names
the wrong reason.

### N3, the quotation attributed to `docs/hld/DEVIATIONS.md` is not that file's wording

`space.rs:264-266` writes, inside quotation marks, that the record says
"verified against rustc rather than reasoned about: `assert_eq!` on two
`Pt<Canvas>` fails with E0369 and E0277". `docs/hld/DEVIATIONS.md` says
"`assert_eq!` on two `Pt<Canvas>` fails to compile with E0369 and E0277,
verified against rustc rather than reasoned about." The clauses are reordered
and "to compile" is dropped. The plan at `:138-139` is closer but is not the
cited source. The substance is right and I confirmed both error codes myself.

### N4, `inverse`'s new qualification is slightly overbroad on `debug-glam-assert`

`space.rs:161-164` says glam's `glam_assert!` "panics when the `glam-assert` or
`debug-glam-assert` feature is on". `glam-0.30.10/src/macros.rs:5-14` gates the
panicking form on `any(all(debug_assertions, feature = "debug-glam-assert"),
feature = "glam-assert")`, so `debug-glam-assert` alone in a release build does
not panic. Everything else in the paragraph is exact: the assert really is on
the determinant, at `glam-0.30.10/src/f64/dmat4.rs:714`, `glam_assert!(dot1 !=
0.0)`, and neither feature is on. `cargo tree -e features --workspace` resolves
exactly one glam feature across the whole workspace, `libm`.

### N5, the plan's compile-fail row names three ui cases where four now exist

`.claude/plans/F-001-design.md:266` names `apply`, `then` and `identity`. The
round 2 remediation raised that row from two to three and added
`value_spaces_do_not_interconvert.rs` in the same round without adding it to
the table. This is the third pass in which the plan's test table has been one
behind the tree. Related: the plan's Deviations section gives the `identity`
narrowing a marked correction note saying "A trybuild case holds the narrowing
in place", and gives the much larger D-08 guard no such note, though the
convention was established for a smaller change.

### N6, an exact `f64` comparison is back in the crate, deliberately

`space.rs:303` and `:306` are `assert_eq!(a, b)` and `assert_ne!(a, c)` on
`Pt<Canvas>`, which run the derived `PartialEq` and compare three `f64` fields
with `==`. Pass 1 graded the equivalent a nitpick and the author removed it.
Pass 2 then recorded "The only `assert_eq!` in the crate is on two `&str`" as
verified clean, and that sentence no longer describes the tree. This is not a
regression to fix: the D-08 record argues at length that the derive expansion
is exactly where the float comparison belongs, both operands are identical
literals with no arithmetic between construction and comparison, and the guard
cannot exercise `PartialEq` without an equality. It passes `float_cmp = "deny"`
only because the comparison is inside a derive expansion, which pass 1 measured.
Recorded so the review chain stays accurate, not as a change request.

### N7, the plan's two `verbatim` transcriptions are not verbatim, and cannot be

Present at base cd74768 and untouched by this diff, so it is not this work's
doing. `.claude/plans/F-001-design.md:106-107` labels the 25.1 geometry row
verbatim and writes a comma where the HLD writes a semicolon. Line 76 labels
section 16.1 verbatim and writes a hyphen where the HLD writes an em-dash. Both
substitutions are forced, because `scripts/prose_check.py` covers
`.claude/plans/` and bans both characters while exempting `docs/hld/`. The
honest fix is to drop the word verbatim from those two headings rather than to
change either file. For the integrator, not for this story.

## The D-09 guard question, measured

**Confirmed. D-09 is guarded by nothing, and the author's corrected statement is
right in every particular.**

I reverted the workspace entry to `glam = "0.30"` and changed nothing else.

**The revert took effect.** `cargo tree` on both targets:

```
$ cargo tree -p ocelli-core -e normal,features
ocelli-core v0.1.0
└── glam feature "default"
    ├── glam v0.30.10
    └── glam feature "std"
        └── glam v0.30.10

$ cargo tree -p ocelli-core --target wasm32-unknown-unknown -e normal,features
   ... identical, glam feature "std"

$ grep -c 'name = "libm"' Cargo.lock
0
```

`libm` left the lock file entirely and the `std` feature is on, so this is a
real revert and not a no-op.

**Nothing notices.** Each exit code read from the command itself:

```
cargo check -p ocelli-core                                    exit 0
cargo check -p ocelli-core --target wasm32-unknown-unknown    exit 0
cargo test  -p ocelli-core                                    exit 0
bin/ocelli.sh gate fmt clippy test bindgen unsafe pins \
  deviations prose content provenance                         exit 0, ALL GREEN
```

**Why the wasm check does not catch it**, verified rather than assumed.
`wasm32-unknown-unknown` ships a `std` implementation in the toolchain, and a
`no_std` crate may depend on a `std` crate without any error, so
`#![cfg_attr(not(test), no_std)]` on `ocelli-core` is satisfied while its
dependency graph is not `no_std` at all. `glam-0.30.10/src/lib.rs:269` is
`#![cfg_attr(not(feature = "std"), no_std)]`, so with `std` on, glam itself is a
`std` crate, and `:288-291` is the `compile_error!` that fires only when none of
`std`, `libm` or `nostd-libm` is selected. The D-09 justification text is
therefore accurate about glam. The gap is purely that nothing in this repository
asserts which of the two backends is chosen.

**The `deviations` gate does not close it.** Its own output on the reverted tree
is `OK: 9 deviations declared, every citation resolves, D-01 matches Cargo.toml`.
It verifies that D-09's citation of section 15.2 resolves, not that D-09 is
applied. D-01 is the only row checked against `Cargo.toml`.

Restored. `md5 -q Cargo.toml` back to `08c2e9e9c1d914b8fcddd8aad612ca5b` and
`diff -q` silent for `Cargo.toml` and `Cargo.lock`. No gate added, per the
brief. The cheapest close would be a target with no `std` in the toolchain, and
this machine has only `aarch64-apple-darwin` and `wasm32-unknown-unknown`
installed, so a `cargo tree` assertion on the resolved feature set is the
likelier shape. That is the integrator's call.

## What I checked and found correct

**The geometry, recomputed from DICOM PS3.3 C.7.6.2.1.1 with exact rationals
and never from the Rust.** `fractions.Fraction`, `P = IPP + i * PixelSpacing[1]
* X + j * PixelSpacing[0] * Y`, `X = IOP[0..3]` the row cosine, `Y = IOP[3..6]`
the column cosine, `i` the column index, `j` the row index.

| Case | Exact rational | Decimal | Fixture |
|---|---|---|---|
| P(0,0) | (-226/5, 1187/10, -65/2) | (-45.2, 118.7, -32.5) | matches |
| P(1,0) | (-901/20, 5927/50, -1619/50) | (-45.05, 118.54, -32.38) | matches |
| P(0,1) | (-224/5, 5947/50, -817/25) | (-44.8, 118.94, -32.68) | matches |
| P(255,191) | (1389/20, 6187/50, -907/25) | (69.45, 123.74, -36.28) | matches |

The step vectors are `di * X = (3/20, -4/25, 3/25)` per column and
`dj * Y = (2/5, 6/25, -9/50)` per row, which are the first and second columns of
`image_plane_transform` exactly. `SPACING_BETWEEN_ROWS`, which is
`PixelSpacing[0] = 0.5`, multiplies the **column** direction cosine. That is the
single most expensive thing in this story to get backwards and it is the right
way round. `X.X = 1`, `Y.Y = 1`, `X.Y = 0` in exact rationals, so the frame is
genuinely orthonormal. `X cross Y = (0, 3/5, 4/5)`, which is `roundtrip.rs`'s
`FRAME_Z` exactly, so that file's right-handed claim holds.
`ImagePositionPatient` is treated as the centre of the first voxel with no
half-pixel offset. The transposition moves the answer by 0.16, 0.2 and 63.72 mm
at the three interior cases against a 1e-6 mm bound, so the fixture's own claim
that it can see the defect it was built for is quantitatively true, and the
guard's threshold of `TOLERANCE_MM * 1000.0` is met with four orders of margin
at the worst case.

The fixture header's account of the standard is correct: the column vector is
`(i, j, 0, 1)` with a literal zero in the third position, `di` is the column
pixel resolution and is `PixelSpacing[1]`, `dj` is the row pixel resolution and
is `PixelSpacing[0]`, and the third matrix column is zero because the equation
is in-plane.

**`apply`, read from glam's source rather than from the method name.**
`glam-0.30.10/src/f64/dmat4.rs:1153-1160` ends `res = res.div(res.w)`, a real
per-component division, so the doc comment's IEEE 754 claim holds: for `w`
exactly 1.0 the affine answer is returned unchanged.
`transform_point3` at `:1176-1183` is the affine-only form with no divide, and
carries its own `glam_assert!` on the bottom row.

**The `w` divide unit test, recomputed by hand.**
`q = 4*c0 + 6*c1 + 8*c2 + c3 = (9, 14, 19, 5)`, `q.xyz / 5 = (1.8, 2.8, 3.8)`,
and `(9, 14, 19)` without the divide. The doc comment's arithmetic is right.

**Composition.** `then` is `next.m * self.m`. The worked pair is genuinely
non-commuting, translate x by +10 against scale x by 2, giving 22 one way and 12
the other, and both orders have their own test. The reversed form does type
check identically, which mutation M2 demonstrated by compiling.

**The perspective bound.** `w = 1 + z/2000` over `z` in `[-500, 500]` is
`[0.75, 1.25]`. The "quarter of a million times tighter" claim is `0.25 / 1e-6 =
250000`.

**Both counterexample guards are live.** `separation > 1.0` fires with "differ
by only 0 mm here" when `PERSPECTIVE_W_PER_MM` is zeroed. The singularity
assertion fires with "the fixture matrix is not singular, determinant is 0.1"
when the zero column is replaced.

**The mechanical rules, checked by hand and not only by gate.** No `as` cast in
`space.rs`, `value.rs` or any of the six test files. The twelve `\bas\b` hits are
all English prose. No `#[allow]`, inner or outer. No `unwrap`, `expect`,
`panic!`, `todo!` or `unimplemented!`. No `unsafe`. No `wasm-bindgen` or
`wasm_bindgen` anywhere under `crates/ocelli-core`, in source or in either
manifest. `#![cfg_attr(not(test), no_std)]` intact at `lib.rs:10` and
`cargo check -p ocelli-core --target wasm32-unknown-unknown` exits 0. No em-dash
in any new Rust or TOML file, and the only semicolons in comments are inside the
fenced transcription of HLD section 25's listing in `roundtrip.rs`.
`bin/ocelli.sh clippy` is `cargo clippy -p <crate> --all-targets -- -D warnings`,
so the test files are linted and not merely compiled.

**Structure, against AGENTS.md.** No new trait. The two generic constructs are
the ones AGENTS.md itself names as worth adding, and both are instantiated
several ways today. No `Box<dyn>`, no forwarding wrapper, no feature flag. Two
constructors on `Transform` and no third crept in. `value.rs` still carries no
arithmetic, no `From`, no `Into` and no conversion of any kind, so HLD section
18's single-implementation rule for the LUT chain is not pre-empted.

**Boundary and tier.** Nothing here renders. Tier A, B and C are all "not
applicable" and the plan declares each explicitly rather than omitting a row. No
allocation, no pixels crossing anything, no wasm memory view, no
`queue.submit()`.

**Prose executed rather than accepted.** `lib.rs:5` says HLD section 28 puts
`space` and `value` first, and `docs/hld/25-first-ten-files.md` section 28 lists
them as entries 1 and 2. HLD section 15.2's dependency block really is five
entries, `wgpu`, `dicom`, `glam`, `bytemuck`, `thiserror`, against seven here.
The `[profile.release]` block is still verbatim, all five keys and values
matching. `float_cmp = "deny"` is at `Cargo.toml:23`.

**Two standalone rustc probes**, compiled outside the crate against 1.97.1
edition 2024, because the claims are about what the compiler does and not about
what this tree does.

| Probe | Claim | Result |
|---|---|---|
| A | "the derived form compiles for all three", the corrected `NOTE` reasoning | exit 0. `#[derive(Debug, PartialEq, Clone, Copy)]` on a `PhantomData` struct binds, copies, clones and compares with a marker carrying this tree's derive list |
| B | a derive would make `Copy` contingent on the marker's, the hand impl does not | `error[E0277]: the trait bound PtDerived<Future>: Copy is not satisfied` for the derived form, accepted for the hand-implemented one |

**Commands, each exit code read from the command itself and never from a pipe.**

```
bin/ocelli.sh check ocelli-core                              exit 0
bin/ocelli.sh test ocelli-core                               exit 0, 24 tests
bin/ocelli.sh clippy ocelli-core                             exit 0
cargo check -p ocelli-core --target wasm32-unknown-unknown   exit 0
bin/ocelli.sh gate fmt clippy test bindgen unsafe pins \
  deviations prose content provenance                        exit 0, ALL GREEN
```

**Tree state after review.** Every mutation reverted and every revert proved.
`diff -r -q crates/ocelli-core` against the pre-review copy is silent, and so is
`diff -q` for `Cargo.toml`, `Cargo.lock` and `.claude/plans/F-001-design.md`.
`md5 -q` gives `6ff3960e000a6c50c60b88f5c75df1fe` for `space.rs`,
`40b59ccfc96de79901ef41883a78424b` for `value.rs`,
`e0474a78aa8ca167e0081dc5f3e21db0` for `roundtrip.rs`,
`2ab84363254b587fc170526716308699` for the geometry fixture and
`08c2e9e9c1d914b8fcddd8aad612ca5b` for `Cargo.toml`. `git status --short` is
identical to its state at the start. One
`crates/ocelli-core/tests/roundtrip.proptest-regressions` file was created by my
own mutation runs and removed, and I note in passing that
`.gitignore` does not cover that pattern, so a future proptest failure will
leave an untracked file behind. Nothing was fixed and nothing was committed.

## Mutations run, and what went red

| # | Mutation | Result |
|---|---|---|
| M1 | `PixelSpacing[0]` and `[1]` transposed in `image_plane_transform` | RED, 4 of 6 geometry tests including `the_transposition_is_visible_at_this_tolerance` |
| M2 | `then` composed as `self.m * next.m` | RED, both composition tests and nothing else. It compiles, which is the point of having the test |
| M3 | `project_point3` replaced with `transform_point3` | RED for `apply_divides_by_the_resulting_w`, `canvas_world_roundtrip_under_perspective` and the fixed perspective case. GREEN, correctly, for the affine round trip and all six geometry cases. Measured 9999.0 to 9912.435180867995, so 86.5648 |
| M4 | P(0,1) expected `y` moved from 118.94 to 118.95 | RED, exactly one test and no other |
| M5 | `identity` widened back to a two-parameter impl | RED, trybuild reports "Expected test case to fail to compile, but it succeeded" |
| M6 | `inverse` guards singular matrices and returns identity | RED for `inverse_does_not_check_invertibility` alone, with the full message about the doc comment and F-023 |
| M7 | `apply` returns `Pt::new(v.x, v.x, v.z)` | RED, 8 of 14 lib tests |
| M8 | `Pt::new` swaps `x` and `y` | RED, 6 of 14 lib tests including `new_stores_its_arguments_in_order` |
| M9 | the singular fixture's zero column replaced with `(0, 0, 1, 0)` | RED with "the fixture matrix is not singular, determinant is 0.1" |
| M10 | `PERSPECTIVE_W_PER_MM` set to zero | RED with "differ by only 0 mm here" |
| M11 | **D-08 reverted in full**, all three markers restored to bare `pub enum` | RED at compile time, E0277 and E0369, both inside the new guard and nowhere else |
| M12 | D-08 reverted **and** the guard test deleted | **GREEN, 23 of 23.** Confirms the guard's own counterfactual claim |
| M13 | `Debug` on `Pt<S>` hand-implemented to print no fields | RED, "Debug for Pt<Canvas> rendered Pt, which does not carry its own fields". The `{:?}` assertion inspects text |
| M14 | `PartialEq` on `Pt<S>` hand-implemented as always true | RED at `assert_ne!`. The guard is pinned from both sides |
| M15 | `Modality` collapsed into `pub type Modality = Stored` | RED, trybuild mismatch on `value_spaces_do_not_interconvert` |
| M16 | one line of `apply_..._another_space.stderr` altered | RED, trybuild mismatch. The snapshots are compared, not decorative |
| M17 | **D-08 reverted in part**, markers trimmed to `#[derive(Debug, PartialEq)]` | **GREEN, 24 of 24.** Smell S1 |
| M18 | the workspace `glam` entry reverted to `glam = "0.30"` | **GREEN everywhere**, including the ten gates and the wasm32 check. The D-09 section above |
