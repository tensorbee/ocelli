# F-001, implementation review, pass 2

**Reviewer**: independent agent, wrote neither the code nor pass 1
**Diff reviewed**: working tree, base cd74768
**Result**: 2 defects, 1 smell, 7 nitpicks

Every remediation was treated as new code. Every factual claim in it was
executed rather than read. Every mutation below was reverted and the revert
proved with `md5 -q` against a copy taken before the review started, not by
eye.

## Pass 1 findings, and whether each is genuinely resolved

### D1, the false `NOTE` about `S: Clone`. RESOLVED, and the new reasoning is true.

`space.rs:63-76` now quotes section 16's note as a quotation and corrects it
beside itself. Three separate checks:

1. **The quote is exact.** Byte-compared against
   `docs/hld/13-core-types.md` section 16. Same two sentences, same wording.
2. **"the derived form compiles for all three" is true.** Probe A, rustc
   1.97.1, edition 2024, with this tree's exact marker derive list and a
   `Copy` stand-in for `DMat4`. `#[derive(Debug, PartialEq, Clone, Copy)]` on
   `Pt<S>` and `#[derive(Debug, Clone, Copy)]` on `Transform<A, B>` both
   compile, bind, copy, clone and compare. Exit 0.
3. **The contingency claim is true**, and it is the load-bearing half of the
   new text. Probe B declares a marker `pub enum Future {}` with no derives
   and passes both forms to `fn takes_copy<T: Copy>`:

```
error[E0277]: the trait bound `PtDerived<Future>: Copy` is not satisfied
   |            ^^^^^^^^^ - type parameter would need to implement `Copy`
```

   The hand-implemented `PtHand<Future>` is accepted in the same file. So
   `impl<S> Copy for Pt<S>` really is unconditional and the derive really
   would make it contingent.

The `Transform` comment at `space.rs:109-112` is corrected the same way and is
also true. The module's D-08 block at `space.rs:22-26` names the consequence.
All of this agrees with the corrected D-08 note now in the canonical
`docs/hld/DEVIATIONS.md`.

### D2, the `verbatim` label. RESOLVED. Counted independently.

`Cargo.toml:33-37`. I counted HLD section 15.2's block myself with
`awk '/^### 15\.2/,/^### 15\.3/'`: `wgpu`, `dicom`, `glam`, `bytemuck`,
`thiserror`, which is **five**. The tree's `[workspace.dependencies]` is
`wgpu`, `dicom`, `glam`, `bytemuck`, `thiserror`, `proptest`, `trybuild`,
which is **seven**. Three departures, all named in the comment: `glam` under
D-09, `proptest` and `trybuild` added.

The same phrase over `[profile.release]` at `Cargo.toml:75` was left alone and
is still true. All five keys and values match section 15.2 exactly:
`opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`,
`strip = true`.

Every sub-claim in the replacement comment was executed:

- glam's `[features]` really is `default = ["std"]`, and
  `glam-0.30.10/src/lib.rs:284-291` really does `compile_error!` when none of
  `std`, `libm`, `nostd-libm` is on.
- `grep -c '^name = "<x>"$' Cargo.lock` gives 0 for `wgpu`, `dicom`,
  `bytemuck` and `thiserror`, and 1 for `glam`, `libm`, `proptest`,
  `trybuild`. The unused workspace entries really do cost nothing.
- `cargo tree -p ocelli-core --target wasm32-unknown-unknown -e normal,features`
  gives `glam feature "libm"` and `libm v0.2.16`, no `std`. D-09 resolves as
  written.

### D3, unit conflation. RESOLVED in the coordinate assertions. One residual false sentence, reported as new defect D1.

**No tolerance value moved.** Every one of the four constants is `1e-6`, the
same number the pre-remediation tree carried:

| Constant | File | Value |
|---|---|---|
| `ROUND_TRIP_TOLERANCE_PX` | `tests/roundtrip.rs:48` | `1e-6` |
| `SPACE_UNIT_TOLERANCE` | `src/space.rs:216` | `1e-6` |
| `TOLERANCE_MM` | `tests/geometry_ps3_3_c7_6_2.rs:96` | `1e-6` |
| `DIMENSIONLESS_TOLERANCE` | `tests/geometry_ps3_3_c7_6_2.rs:103` | `1e-6` |

I enumerated every assertion in the three files and checked the space each one
actually bounds:

- `roundtrip.rs`, nine `prop_assert!` and three `assert!` on round trips.
  All are `Pt<Canvas>` differences, all bounded by `ROUND_TRIP_TOLERANCE_PX`.
  Correct.
- `roundtrip.rs:158`, the one world-space assertion. Bounded by a literal
  `1.0` and labelled millimetres at `:149-151`. Both operands really are
  `Pt<World>` from forward transforms. Correct.
- `geometry_ps3_3_c7_6_2.rs`, twelve component assertions through
  `assert_world_within_tolerance`, all `Pt<World>`, all `TOLERANCE_MM`.
  Correct. The transposition guard at `:258` bounds a world-space distance
  with `TOLERANCE_MM * 1000.0`. Correct.
- `geometry_ps3_3_c7_6_2.rs:194-205`, three dot products, now on
  `DIMENSIONLESS_TOLERANCE`. N2 resolved.
- `space.rs`, all `assert_close` calls, deliberately unlabelled with the three
  spaces spelled out. Defensible. The sentence introducing that list is
  false, which is new defect D1.
- `space.rs:399`, the determinant assertion, on `SPACE_UNIT_TOLERANCE`. Not a
  point in any space. New nitpick N1.

### S1, `identity()` as a free cross-space cast. RESOLVED and pinned by an executing guard.

`identity` is now on `impl<S> Transform<S, S>` (`space.rs:188-193`).
`Transform::<Canvas, World>::identity()` does not compile, and that is
asserted rather than believed: `tests/ui/identity_refuses_to_cast_between_spaces.rs`
is a live trybuild case. Mutation M5 widened the impl back to
`impl<A, B> Transform<A, B>` and the guard fired:

```
test tests/ui/identity_refuses_to_cast_between_spaces.rs ... error
Expected test case to fail to compile, but it succeeded.
test mixing_spaces_does_not_compile ... FAILED
```

All three ui cases run. My first reading of the test output suggested only one
did, which was an artefact of grepping past ANSI escapes. The raw log shows
`apply_...`, `identity_...` and `then_...` each reported by trybuild.

**The three changed call sites keep their meaning.**
`apply_refuses_a_point_from_another_space.rs` still builds a
`Transform<Canvas, World>` and still feeds it a `Pt<Index>`, now via
`from_mat4(DMat4::IDENTITY)`. `then_refuses_a_transform_that_does_not_join.rs`
still chains `Transform<Canvas, World>` onto `Transform<Index, World>`. Both
`.stderr` files still match, which trybuild enforces. The third site, the
`Copy` unit test, changed from one identity to a scale and a translation, and
is strictly stronger for it (see N1 below).

The ui crate does resolve `glam`, as the author claims. `glam` is a normal
dependency of `ocelli-core`, trybuild's generated `ocelli-core-tests` project
inherits it, and the cases compile far enough to produce the expected E0308
and E0599 rather than an unresolved-crate error.

### S2, the unexecuted singular-inverse claim. RESOLVED with a real test.

`space.rs:388-416`. Three independent checks:

1. **The matrix is genuinely singular.** Third column is `DVec4::ZERO`, so the
   determinant is exactly zero. Not merely small: mutation M6 replaced
   `inverse`'s body with `if self.m.determinant() == 0.0 { return identity }`
   and the branch was taken, which only happens on an exact zero.
2. **It would fail if `inverse` started guarding.** M6 again:

```
test space::tests::inverse_does_not_check_invertibility ... FAILED
inverse of a singular transform returned finite components (-45.2, 118.7, -32.5)
```

   That test and no other. 12 passed, 1 failed.
3. **The singularity assertion is itself a guard, not decoration.** M8
   replaced the zero column with `(0, 0, 1, 0)` and the assertion fired with
   `the fixture matrix is not singular, determinant is 0.1`. So the case
   cannot stop being singular unnoticed.

The assertion is `!is_finite()` on all three components, which is the strong
form, not a weaker "differs from the input".

### N1, the near-vacuous `Copy` test. RESOLVED, and measured rather than argued.

Mutation M7 changed `apply` to `Pt::new(v.x, v.x, v.z)`, a deterministic
defect, and re-added pass 1's old test body alongside the new one:

```
test space::tests::a_point_is_copy_and_both_uses_see_the_same_point ... FAILED
test space::tests::old_form_a_point_is_copy_and_survives_being_used_twice ... ok
```

New form red, old form green, same defect. The author's claim holds. The new
test also carries two hand-worked answers rather than one self-comparison, and
it no longer contains the crate's only `assert_eq!` on a float.

It did cost something, which is new smell S1.

### N2, N3, N4. RESOLVED. N5 correctly left to the integrator.

N2: `DIMENSIONLESS_TOLERANCE` added and applied to all three dot products.
N3: `space.rs:40-46` now says the slice index is this crate's own and that
C.7.6.2.1.1's column vector is `(i, j, 0, 1)`. Checked against the standard.
N4: the plan now says three edits in both places, with a marked correction
note. Two smaller staleness spots survive, reported as nitpick N4.

## New defects

### D1, `space.rs:204` says section 25.1 has a geometry row for one of the three spaces, and it has two

**Where**: `crates/ocelli-core/src/space.rs:204`

```rust
/// It is deliberately not labelled with a unit, because these tests span
/// three spaces and HLD section 25.1 has a row for one of them:
```

**What**: HLD section 22, 25.1's geometry bullet gives `world coordinates
within 1e-6 mm` and then `canvas coordinates within a quarter pixel`. That is a
row for **two** of the three spaces. The doc comment's own next three bullets say so, naming
the world row, the canvas row, and the absence of an index row. The sentence
introducing the list contradicts the list.

**Why it is wrong**: a false claim about the normative tolerance table, in the
one doc block whose entire job is to stop a reader reaching for the wrong
25.1 row. It is new prose, added by the D3 remediation. A reader who trusts
the sentence concludes that only `Pt<World>` has a bound in 25.1 and that
canvas coordinates are unconstrained, which is exactly the confusion the block
was written to prevent, arrived at from the opposite direction.

**Evidence**: `awk` over `docs/hld/22-testing-and-tolerance.md`:

```
- **Geometry:** world coordinates within 1e-6 mm; canvas coordinates within a quarter pixel.
```

Two clauses, two spaces. And the file itself, three lines lower, at
`space.rs:206-210`, enumerates both of them.

### D2, the design plan still gives the reason pass 1 established is false

**Where**: `.claude/plans/F-001-design.md:185-186`

```
`Clone` and `Copy` are hand-implemented on `Transform` too, for the reason the
HLD's note gives for `Pt`. `Debug` is derived on both, which works because the
markers derive it.
```

**What**: this is the sentence pass 1's D1 falsified, in a different file. The
reason the HLD's note gives is that the markers do not satisfy an `S: Clone`
bound. Under D-08 they do, verified above. The source now says the opposite in
so many words at `space.rs:109-112`: "not for the reason section 16's note
gives". The corrected D-08 note in `docs/hld/DEVIATIONS.md` also says the
opposite.

**Why it is wrong**: `.claude/plans/` is a tracked, agent-neutral artefact per
AGENTS.md, reviewed like code. The remediation edited this exact file twice,
for nitpick N4, and added a marked "Corrected during implementation" note
while leaving the sentence that pass 1 raised as a blocking defect. The tree
now carries the corrected reasoning in two places and the falsified reasoning
in one, which is worse than carrying it consistently, because the next reader
has no way to tell which is current.

The second sentence is loose in the same direction. `#[derive(Debug)]` on
`Transform<A, B>` compiles whether or not the markers derive `Debug`, because
the derive bounds the parameters rather than requiring them. What the markers
deriving `Debug` buys is a usable `Pt<Canvas>: Debug` at a call site, and see
smell S1 for whether anything uses it.

**Evidence**: probes A and B above, plus the three-way disagreement between
`space.rs:109-112`, `docs/hld/DEVIATIONS.md`'s D-08 note, and this line.

## New smells

### S1, D-08 is exercised by nothing, and the N1 remediation is what removed the last thing that exercised it

**Where**: `crates/ocelli-core/src/space.rs:33`, `:37`, `:47`

**What**: D-08 is this story's headline deviation from the normative HLD. Its
justification, in the plan and in the DEVIATIONS row, is that
`#[derive(Debug, PartialEq)]` on `Pt<S>` is unusable at every call site while
the markers are bare, "verified against rustc rather than reasoned about:
`assert_eq!` on two `Pt<Canvas>` fails with E0369 and E0277".

Nothing in the tree performs that call any more. I grepped: the only
`assert_eq!` in the crate is `lib.rs:26`, on two `&str`. Nothing formats a
`Pt` with `{:?}`. Nothing compares two `Pt` with `==`.

**Why it will cause a defect**: this is microscope section 4's case exactly, a
thing that is present, looks authoritative, and is never reached. Reverting
D-08 in full, restoring the three markers to precisely what HLD section 16
writes, leaves the crate compiling and the whole suite green. A future
contributor who notices that the source deviates from the HLD listing can
delete the derives, run everything, see green, and land it. The failure then
surfaces in whichever downstream story first writes `assert_eq!` on two
points, with the deviation record pointing at a justification the tree no
longer demonstrates.

It is also a regression the remediation caused. Pass 1's `Copy` test contained
`assert_eq!(once, twice)` on two `Pt<Canvas>`, which required both
`Pt<Canvas>: Debug` and `Pt<Canvas>: PartialEq` and was therefore the one
place D-08 was load bearing. N1's replacement uses `assert_close`, which
compares `f64` fields and needs neither. Fixing the weak test removed the
coverage that mattered, and the weak test was where it lived.

**Evidence**: mutation M9, the markers reverted to bare `pub enum Canvas {}`,
`pub enum World {}`, `pub enum Index {}`, nothing else touched.

```
$ cargo test -p ocelli-core
M9 (D-08 reverted) exit=0
grep -cE "^error"  ->  0
test result: ok. 13 passed   (lib)
test result: ok.  1 passed   (compile_fail, all three ui cases)
test result: ok.  6 passed   (geometry)
test result: ok.  3 passed   (roundtrip)
```

Zero compile errors, zero failures, all 23 tests green with the deviation
removed. Reverted, `md5 -q` back to `2c3ca4095b7ecec0440784c375f0e1f1`.

The cheap close is one assertion that needs what D-08 provides. A single
`assert_eq!` on two `Pt<Canvas>` in `space.rs`'s test module, or a `{:?}`
format, turns the derives back into a guard. It is defensible for the
implementer to argue this belongs to a later story, but it is not defensible
for the tree to claim a deviation is load bearing and prove nothing.

## Nitpicks

### N1, `SPACE_UNIT_TOLERANCE` bounds a determinant at `space.rs:399`

The constant's own doc at `:200-201` says it is "the bound every assertion
below uses, in the units of whichever space the point is in". A determinant is
not a point and has no space. Nothing is at risk today, because M6 proved the
determinant is exactly `0.0`, so any positive bound passes. It is the same
unit-borrowing that pass 1's D3 named, in a test the remediation added, and
the geometry fixture solved the identical problem the other way by adding
`DIMENSIONLESS_TOLERANCE`.

### N2, "twenty five lines above" at `space.rs:69`

The three marker derives are at lines 33, 37 and 47, so they are 16, 26 and 30
lines above the comment. The phrase is inherited from pass 1's prose. A line
count in a comment goes stale on the next edit anyway.

### N3, the 86.6 figure sits on the test that did not measure it

`tests/roundtrip.rs:102-103` attaches "measured at 86.6 under the mutation
that drops the divide" to `canvas_world_roundtrip_under_perspective`. Under
that mutation the proptest's own reported drift is 51.1, not 86.6. 86.6 comes
from the fixed case below it, at `x = 9999, z = 499`, where I measured
`9999.0 - 9912.435180867995 = 86.5648`. The number is right and the
attribution is loose.

### N4, two stale spots survive in the plan beyond D2

`.claude/plans/F-001-design.md:244`, the compile-fail row, still names two ui
cases where three exist. The plan is also silent on S1's narrowing of
`identity` to `impl<S> Transform<S, S>`, which is a change to a public
signature the plan explicitly decides at `:144-146`, while carrying an
explicit correction note for the much smaller N4. If a correction note is the
convention, this change earned one.

### N5, `value.rs`'s `each_newtype_carries_its_field_unchanged` cannot fail

`crates/ocelli-core/src/value.rs:33-42`. `Stored(-1024.0).0` is the tuple
constructor and the field read, with nothing between them. No change to
`value.rs` that still compiles can make the assertion false, and the doc
comment's hypothesis, "a tuple struct that rounded or clamped on
construction", is not something a tuple struct with a public field can do.
`each_newtype_is_copy` is in the same position, though its `Copy` property is
genuinely checked by the compiler at `let again = (s, m, d)`. Pass 1 graded
the equivalent `space.rs` test a nitpick and the author fixed it, so this is
noted for consistency rather than as a blocker. There is genuinely nothing
behavioural in `value.rs` to test.

### N6, the progress note calls the new trybuild case the fourth

`.claude/scratch/F-001-progress.md`, the S1 row, says "A fourth trybuild case,
`identity_refuses_to_cast_between_spaces.rs`". There are three:
`apply_...`, `identity_...`, `then_...`. The file is gitignored, so this ships
nowhere, but it is the handoff record another agent reads. Everything else in
that note checks out, including both md5 values and the 23-test count.

### N7, `inverse`'s doc states a glam behaviour that is feature-conditional

`space.rs:147-149` says glam's `inverse` "returns non-finite components for a
singular one rather than failing". `glam-0.30.10/src/f64/dmat4.rs:714` is
`glam_assert!(dot1 != 0.0)`, which panics when the `glam-assert` or
`debug-glam-assert` feature is on. Neither is on in this workspace, so the
sentence is true as configured, and `inverse_does_not_check_invertibility`
would go red if feature unification ever turned one on, so the risk is closed.
The sentence is still unqualified about a dependency in a workspace that
denies `panic`, `unwrap` and `expect` precisely because a panic poisons a wasm
instance.

## What I checked and found correct

**The geometry arithmetic, recomputed from the standard rather than from the
Rust.** `fractions.Fraction`, `P = IPP + i * PixelSpacing[1] * X + j *
PixelSpacing[0] * Y`, with `X = IOP[0..3]` the row cosine, `Y = IOP[3..6]` the
column cosine, `i` the column index and `j` the row index.

| Case | Exact rational | Decimal | Fixture |
|---|---|---|---|
| P(0,0) | (-226/5, 1187/10, -65/2) | (-45.2, 118.7, -32.5) | matches |
| P(1,0) | (-901/20, 5927/50, -1619/50) | (-45.05, 118.54, -32.38) | matches |
| P(0,1) | (-224/5, 5947/50, -817/25) | (-44.8, 118.94, -32.68) | matches |
| P(255,191) | (1389/20, 6187/50, -907/25) | (69.45, 123.74, -36.28) | matches |

The step vectors match the header's hand computation exactly:
`di * X = (3/20, -4/25, 3/25) = (0.15, -0.16, 0.12)` per column and
`dj * Y = (2/5, 6/25, -9/50) = (0.4, 0.24, -0.18)` per row, and those are the
first and second columns of `image_plane_transform`. So `SPACING_BETWEEN_ROWS
= PixelSpacing[0] = 0.5` multiplies the **column** cosine, which is the single
most expensive thing in this story to get backwards, and it is the right way
round. `X.X = 1`, `Y.Y = 1`, `X.Y = 0` in exact rationals. `X cross Y =
(0, 3/5, 4/5)`, which is `roundtrip.rs`'s `FRAME_Z` exactly, so that file's
right-handed-frame claim holds. `ImagePositionPatient` is the centre of the
first voxel with no half-pixel offset, correctly.

A transposition moves the answer by 0.16, 0.2 and 63.72 mm at the three
interior cases, against a 1e-6 mm bound, so the fixture's claim that it can see
the defect it was built for is quantitatively true.

**`apply` and the `w` divide, read from glam's source rather than from the
method name.** `glam-0.30.10/src/f64/dmat4.rs:1153-1160`:
`res = res.div(res.w)`, and `DVec4`'s `Div<f64>` at `dvec4.rs:1253-1264` is
four per-component `f64` divisions, not a multiply by a reciprocal. So the doc
comment's IEEE 754 claim holds exactly: for `w == 1.0` each component is
divided by 1.0 and is unchanged. `transform_point3` at `:1176-1183` is the
affine-only form with no divide.

**Composition order.** `then` is `next.m * self.m`, and the doc's worked pair
is right: translate x by +10 then scale x by 2 gives 22, the other order gives
12. Both orders are pinned by their own test.

**The `w` divide unit test.** Recomputed by hand from the column-major matrix:
`q = 4*c0 + 6*c1 + 8*c2 + c3 = (9, 14, 19, 5)`, `q.xyz / 5 = (1.8, 2.8, 3.8)`,
and (9, 14, 19) without the divide. The doc comment's arithmetic is correct.

**The perspective bound.** `w = 1 + z/2000` over `z` in `[-500, 500]` is
`[0.75, 1.25]` as claimed. The "quarter of a million times tighter" claim is
`0.25 / 1e-6 = 250000`, correct.

**The `separation > 1.0` guard is a real guard.** Mutation M10 set
`PERSPECTIVE_W_PER_MM` to zero and it fired with "differ by only 0 mm here, so
this case does not exercise the w divide". Without it the perspective test
could silently degrade into a second affine case.

**Mechanical rules, checked by hand and not only by gate.** No `as` cast in
`space.rs`, `value.rs` or any of the five test files. The six `grep` hits for
`\bas\b` are all English prose. No `#[allow]` anywhere. No `unwrap`, `expect`,
`panic!`, `todo!` or `unimplemented!`. No `unsafe`. No `wasm-bindgen`, in
source or in either manifest. The only `assert_eq!` in the crate is on two
`&str`. `#![cfg_attr(not(test), no_std)]` intact at `lib.rs:10`, and
`cargo check -p ocelli-core --target wasm32-unknown-unknown` exits 0. No
em-dash anywhere in the new Rust prose, and the only semicolons in comments are
inside the fenced transcription of HLD section 25's listing.

**Structure, against AGENTS.md.** No new trait. The one generic construct,
`Pt<S>` and `Transform<A, B>`, is instantiated three and several ways today
and is the example AGENTS.md itself names as worth adding. No `Box<dyn>`, no
forwarding wrapper, no feature flag. Two constructors on `Transform`, as the
plan decided, and no third crept in.

**Boundary and tier.** Nothing here renders, so tier A, B and C are all "not
applicable" and the plan says so explicitly for each. No allocation: `Pt` and
`Transform` are `Copy` and live on the stack. No pixels, no wasm memory view,
no `queue.submit`.

**`value.rs` carries no arithmetic.** No `From`, no `Into`, no conversion of
any kind between `Stored`, `Modality` and `Display`, so HLD section 18's
single-implementation rule for the LUT chain is not pre-empted.

**Commands, each exit code read from the command itself and not from a pipe.**

```
bin/ocelli.sh check ocelli-core                              exit 0
bin/ocelli.sh test ocelli-core                               exit 0, 23 tests
bin/ocelli.sh clippy ocelli-core                             exit 0
cargo check -p ocelli-core --target wasm32-unknown-unknown   exit 0
bin/ocelli.sh gate fmt clippy test bindgen unsafe pins \
  deviations prose content provenance                        exit 0, ALL GREEN
```

**Tree state after review.** Every mutation reverted and every revert proved.
`space.rs` hashes back to `2c3ca4095b7ecec0440784c375f0e1f1`, the geometry
fixture to `2ab84363254b587fc170526716308699`, `roundtrip.rs` to
`fea26e5920eda34e99011b113b321d41`, and `diff -q` is silent for every other
file including `Cargo.toml`, `Cargo.lock` and all six `tests/ui` files.
`git status --short` is identical to its state at the start. No
`proptest-regressions` directory was left behind. Nothing was fixed and
nothing was committed.

## Mutations run, and what went red

| # | Mutation | Result |
|---|----------|--------|
| M1 | `PixelSpacing[0]` and `[1]` transposed in `image_plane_transform` | RED, 4 of 6 geometry tests including `the_transposition_is_visible_at_this_tolerance` |
| M2 | `then` composed as `self.m * next.m` | RED, both composition tests, nothing else |
| M3 | `project_point3` replaced with `transform_point3` | RED for `apply_divides_by_the_resulting_w`, `canvas_world_roundtrip_under_perspective` and `the_perspective_divide_is_visible_at_this_tolerance`. GREEN, correctly, for the affine round trip and the whole geometry fixture. Measured drift 9999.0 to 9912.435180867995, so 86.5648 |
| M4 | P(0,1) expected `y` moved from 118.94 to 118.95 | RED, exactly one test and no other |
| M5 | `identity` widened back to `impl<A, B> Transform<A, B>` | RED, trybuild reports "Expected test case to fail to compile, but it succeeded" |
| M6 | `inverse` guards singular matrices and returns identity | RED for `inverse_does_not_check_invertibility` alone. The `== 0.0` branch was taken, proving the determinant is exactly zero |
| M7 | `apply` returns `Pt::new(v.x, v.x, v.z)`, with pass 1's old `Copy` test body re-added alongside the new one | RED for the new test, GREEN for the old one. N1's remediation measured, not argued |
| M8 | The singular fixture's zero column replaced with `(0, 0, 1, 0)` | RED with "the fixture matrix is not singular, determinant is 0.1". The singularity assertion is a live guard |
| M9 | D-08 reverted in full, the three markers restored to bare `pub enum` | **GREEN. Zero compile errors, 23 of 23 tests pass.** Smell S1 |
| M10 | `PERSPECTIVE_W_PER_MM` set to zero | RED with "differ by only 0 mm here", the separation guard is live |

Two out-of-tree rustc probes, compiled standalone against 1.97.1 edition 2024
rather than run inside the crate:

| Probe | Claim under test | Result |
|---|---|---|
| A | "the derived form compiles for all three" | exit 0, runs, prints |
| B | `#[derive(Copy)]` makes `Pt`'s `Copy` contingent on the marker's | E0277 for the derived form, accepted for the hand-implemented one |
