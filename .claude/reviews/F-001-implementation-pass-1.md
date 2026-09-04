# F-001, implementation review, pass 1

**Reviewer**: independent agent, did not write the code
**Diff reviewed**: uncommitted working tree of `ocelli-wt/f-001-claude` on
`work/f-001-claude`, base cd74768. Four tracked files modified (`Cargo.lock`,
`Cargo.toml`, `crates/ocelli-core/Cargo.toml`,
`crates/ocelli-core/src/lib.rs`) and seven untracked files added under
`crates/ocelli-core/`.
**Result**: 3 defects, 2 smells, 5 nitpicks

## Defects

### D1, the `NOTE` above `impl<S> Clone for Pt<S>` states something false about this crate

**Where**: `crates/ocelli-core/src/space.rs:55-56`

```rust
// NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
// types do not satisfy. Implement by hand.
```

**What**: the second half of the sentence is false. Under D-08 the three
marker enums derive `Clone` and `Copy` at lines 27, 31 and 39 of the same
file, so they do satisfy an `S: Clone` bound, and `#[derive(Clone, Copy)]` on
`Pt<S>` compiles and works for `Pt<Canvas>`, `Pt<World>` and `Pt<Index>`. The
comment is contradicted twenty five lines above itself.

**Why it is wrong**: the sentence is verbatim HLD section 16, where it was
true because section 16 writes the markers bare. D-08 changed exactly that
premise and neither the plan nor the D-08 row in `docs/hld/DEVIATIONS.md`
noticed the note had become false. `DEVIATIONS.md` line 25 in fact leans on
it: "section 16's own note identifies exactly this trap for `Clone` and
`Copy`". That trap no longer exists in this tree.

The hand-written impls are still the right code, but for a different reason
than the one stated. `impl<S> Copy for Pt<S>` is unconditional and stays
correct for a future marker that is not itself `Copy`, whereas the derive
would bound it. That reason is not what the file says, and the reason a
reader can check is the one that is wrong.

**Evidence**: compiled the derived form against rustc 1.97.1, edition 2024,
with the marker enum carrying this tree's exact derive list.

```
$ rustc --edition 2024 -o /tmp/derivetest /tmp/derivetest.rs   # exit 0
$ /tmp/derivetest
derive(Clone, Copy) on Pt<S> compiles fine with derived markers
```

The probe declares `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub
enum Canvas {}` and `#[derive(Debug, PartialEq, Clone, Copy)] pub struct
Pt<S>`, then binds, copies, clones and compares a `Pt<Canvas>`.

### D2, `HLD section 15.2, verbatim` now heads a block this diff made non-verbatim

**Where**: `Cargo.toml:33`, the comment immediately above
`[workspace.dependencies]` at line 41.

**What**: the diff rewrote the second paragraph of that comment and left the
first sentence. The block it heads is no longer verbatim section 15.2. `glam`
is changed per D-09, and `proptest` and `trybuild` are two entries section
15.2 does not contain.

**Why it is wrong**: `docs/hld/12-workspace-and-build.md` section 15.2 gives
`[workspace.dependencies]` as exactly five entries, with `glam = "0.30"`. The
tree has seven, one of them altered. The same phrase is used at `Cargo.toml:69`
for `[profile.release]`, where it is still true, so the phrase is this file's
idiom for "the following TOML block is copied from 15.2" rather than a loose
gesture at the section. One of the two blocks it labels no longer is.

The task asked specifically whether the replacement comment is true, and it
is. "Cargo does not resolve an unused workspace dependency, so an entry costs
nothing until a crate names it" was checked and holds, and "F-001 activates
glam" holds. The defect is the sentence directly above the replacement, which
this same diff falsified and did not touch.

**Evidence**:

```
$ grep -c 'name = "wgpu"' Cargo.lock      -> 0
$ grep -c 'name = "dicom"' Cargo.lock     -> 0
$ grep -c 'name = "bytemuck"' Cargo.lock  -> 0
$ grep -c 'name = "thiserror"' Cargo.lock -> 0
```

so the replacement paragraph is true, and

```
$ awk '/^### 15.2/,/^### 15.3/' docs/hld/12-workspace-and-build.md
[workspace.dependencies]
wgpu = "=30.0.1" # pin EXACTLY - breaking changes ~quarterly
dicom = { version = "0.10", default-features = false }
glam = "0.30"
bytemuck = { version = "1", features = ["derive"] }
thiserror = "2"
```

so the surviving sentence is not.

### D3, `roundtrip.rs` calls canvas coordinates millimetres, and cites the wrong row of 25.1

**Where**: `crates/ocelli-core/tests/roundtrip.rs:17-19`, `:28-29`, `:82-83`,
and every `prop_assert!` and `assert!` in the file.

**What**: every quantity bounded in this file is a canvas coordinate, not a
world one. `t` is a `Transform<Canvas, World>`, so `t.inverse().apply(t.apply(p))`
is a `Pt<Canvas>` and `back.x - p.x` is a difference in CSS pixels. The file
names its constant `TOLERANCE_MM`, documents it as "HLD section 25.1: world
coordinates within 1e-6 mm", and asserts at line 82 that

> A transform that ignored the `w` divide would come back tens of
> millimetres away from where it started.

The measured drift is 86.6, and the unit is CSS pixels of canvas coordinate,
not millimetres.

**Why it is wrong**: HLD section 22, 25.1 has two geometry clauses, "world
coordinates within 1e-6 mm" and "canvas coordinates within a quarter pixel".
The file quotes the first and applies it to a quantity governed by the second.
The bound itself is fine, 1e-6 is section 25's own listing and is far tighter
than a quarter pixel, so nothing is weakened today. What is wrong is the
label, and this is the project whose stated central risk is exactly this
confusion. The concrete failure mode is a later reader who checks the name
`TOLERANCE_MM` against 25.1, finds that the row governing canvas coordinates
says a quarter pixel, and "corrects" 1e-6 to 0.25. That reads as a fix rather
than as the tolerance change AGENTS.md forbids.

The same conflation, milder because the constant is not named `_MM`, is at
`crates/ocelli-core/src/space.rs:155-156`, where `TOLERANCE` is documented as
"HLD section 25.1, the geometry row" and is then applied to `Pt<Canvas>` at
line 199 and `Pt<Index>` at line 217. An index tolerance is in voxels and
25.1 has no row for it at all.

**Evidence**: mutation 3 below, reading the panic message rather than the
exit status alone.

```
thread 'the_perspective_divide_is_visible_at_this_tolerance' panicked at
crates/ocelli-core/tests/roundtrip.rs:109:5:
x drifted to 9912.435180867995
```

against an expected 9999.0, so 86.56 units of drift, in the canvas space the
round trip returns to.

## Smells

### S1, `identity()` is a free cross-space cast, and its doc comment sells it as one

**Where**: `crates/ocelli-core/src/space.rs:109-112`

```rust
/// The transform that changes nothing but the space.
pub const fn identity() -> Self {
    Self::from_mat4(DMat4::IDENTITY)
}
```

**What**: this sits in `impl<A, B> Transform<A, B>`, so
`Transform::<Canvas, World>::identity()` compiles. Applying it turns a
`Pt<Canvas>` into a `Pt<World>` with no arithmetic, no cast and nothing to
review. The doc comment describes that as the feature: it changes nothing but
the space.

**Why it will cause a defect**: HLD section 16's whole claim is that a canvas
point and a world point are not interchangeable. A constructor whose name
means "no conversion needed" and whose type is `Transform<A, B>` re-admits the
interchange in the terse, innocent-looking form an agent reaches for when it
does not yet have a real transform. The crate's own tests reach for it three
times that way already, at `space.rs:189`, `tests/ui/apply_refuses_a_point_from_another_space.rs:9`
and `tests/ui/then_refuses_a_transform_that_does_not_join.rs:8-9`. When F-023
builds a viewport, the tempting stub for a camera that does not exist yet is
`Transform::<World, Canvas>::identity()`, and that produces plausible wrong
numbers rather than an error.

Stating the counter-argument, because it is real: `from_mat4` is public and
takes any matrix, so `from_mat4(DMat4::IDENTITY)` reaches the same place and
constraining `identity()` closes no hole. The difference is that `from_mat4`
makes the caller name a matrix, which is a deliberate act a reviewer sees,
where `identity()` reads as an assertion that no conversion was needed.
Narrowing it to `impl<S> Transform<S, S>` keeps every meaningful use and
costs the three placeholder ones a slightly longer spelling.

The plan decided two constructors and exactly two exist, so this is not a
count violation. It is the signature of one of them.

### S2, the crate documents a glam behaviour F-023 is told to rely on, and nothing executes it

**Where**: `crates/ocelli-core/src/space.rs:127-137`

> glam's `inverse` returns non-finite components for a singular one rather
> than failing. [...] A caller must not assume this method checked.

**What**: the claim is true today, verified below. Nothing in the crate pins
it. No test constructs a singular `Transform` and asserts the result is
non-finite.

**Why it will cause a defect**: `glam = "0.30"` is a caret range, not the
exact pin `wgpu` gets, so `cargo update` can move it within 0.30.x. The plan
records this behaviour as a decision F-023 must not assume away, and
`docs/lld/core-types.md` is scheduled to carry it forward. A dependency whose
behaviour is load bearing for a future story, asserted in prose and executed
nowhere, is microscope section 4's case: it looks authoritative and a green
suite says nothing about it. It is also cheap to close, because the crate
already contains a singular matrix. `image_plane_transform()` in the geometry
fixture has a zero third column by construction, per PS3.3 C.7.6.2.1.1, and
its determinant is exactly 0.

**Evidence**: a standalone probe against glam 0.30.10 with this tree's
feature set, using the fixture's own matrix.

```
determinant = 0
inverse().project_point3 = DVec3(NaN, NaN, NaN)  is_finite=false
```

## Nitpicks

### N1, `a_point_is_copy_and_survives_being_used_twice` cannot distinguish right from wrong

`crates/ocelli-core/src/space.rs:186-193`. It applies the same transform to
the same point twice and asserts the two results are equal. That holds for any
deterministic `apply`, correct or not. The property in the name, `Copy`, is
checked by the compiler on line 190 and needs no assertion. It went red under
my identity-to-zero mutation only incidentally, because `NaN != NaN`.

It is also the crate's only exact `f64` comparison. `assert_eq!` on two
`Pt<Canvas>` runs the derived `PartialEq`, which compares the three `f64`
fields with `==`. It passes `float_cmp = "deny"` only because the comparison
is inside a derive expansion. I confirmed the lint is otherwise live in test
code: adding `assert!(once.x == twice.x)` on the next line fails clippy with
"strict comparison of `f32` or `f64`", exit 101. The comparison here is
defensible, both sides come from one deterministic call, but review criterion
8 exists and this is the one place the tree sits on the wrong side of it.

### N2, a millimetre tolerance applied to dimensionless dot products

`crates/ocelli-core/tests/geometry_ps3_3_c7_6_2.rs:180-196` bounds
`X.X - 1`, `Y.Y - 1` and `X.Y` with `TOLERANCE_MM`. Those are unitless. The
values are exact terminating decimals so nothing is at risk, but the constant
carries the wrong unit into a place it does not belong. Related to D3.

### N3, PS3.3 C.7.6.2.1.1 has no slice index in its column vector

`crates/ocelli-core/src/space.rs:36-38` says `z` is the slice index "which is
the order DICOM PS3.3 C.7.6.2.1.1 writes its column vector in". The
standard's column vector is `(i, j, 0, 1)`, with a literal zero in the third
position. The `i` before `j` part is right. The slice index is not in that
section at all, which the geometry fixture's own header states correctly at
lines 118-120.

### N4, the plan says the workspace manifest is edited in one place and it is edited in three

`.claude/plans/F-001-design.md:24` says "The workspace manifest is edited in
exactly one place, to make `glam` usable from a `no_std` crate", and the
Approach section at line 199 says "Two edits". The tree makes three changes:
`glam` altered, `proptest` added, `trybuild` added. `trybuild`'s workspace
entry appears in no section of the plan, though the plan's test table does name
the tool. Nothing here is wrong in the code. The plan is a tracked artefact and
is now inconsistent with itself and with the tree.

### N5, the geometry fixture builds its own matrix, so today it exercises `apply`

`crates/ocelli-core/tests/geometry_ps3_3_c7_6_2.rs:121-136`. There is no
library code yet that maps IPP, IOP and PixelSpacing to a `Transform`, so the
transposition the fixture hunts has no implementation it could be wrong in.
The file is a correct and valuable executable record of the PS3.3 derivation
and becomes a real regression test the moment the geometry story lands, and I
confirmed it goes red under a transposition. It should not be described in
AS_BUILT as covering a spacing decision that no shipped code makes yet.

## What I checked and found correct

**The geometry arithmetic.** Recomputed all four expected values independently
with `fractions.Fraction`, from PS3.3 C.7.6.2.1.1 and not from the Rust, using
`P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y`.

| Case | Exact rational | Decimal | Fixture claims |
|------|----------------|---------|----------------|
| P(0,0) | (-226/5, 1187/10, -65/2) | (-45.2, 118.7, -32.5) | matches |
| P(1,0) | (-901/20, 5927/50, -1619/50) | (-45.05, 118.54, -32.38) | matches |
| P(0,1) | (-224/5, 5947/50, -817/25) | (-44.8, 118.94, -32.68) | matches |
| P(255,191) | (1389/20, 6187/50, -907/25) | (69.45, 123.74, -36.28) | matches |

All four agree to the digit. The direction cosines are genuinely orthonormal
in exact arithmetic: `X.X = 1`, `Y.Y = 1`, `X.Y = 0`, no floating point
involved. `PixelSpacing[0] = 0.5` multiplies the column direction cosine and
`PixelSpacing[1] = 0.25` multiplies the row direction cosine, which is the
right way round and is the single most likely defect in this story. `i` is
carried in `x` and is the column index, `j` is carried in `y` and is the row
index. The frame is oblique and the pixel is non-square, so the transposition
moves the answer by 0.16 mm, 0.2 mm and 63.72 mm at the three interior cases,
against a 1e-6 mm tolerance. `ImagePositionPatient` is treated as the centre
of the first voxel with no half-pixel offset, correctly.

`roundtrip.rs`'s claim that `FRAME_Z` is the cross product of the other two is
also true in exact arithmetic: `X cross Y = (0, 3/5, 4/5)`, and `FRAME_Z` is
`(0.0, 0.6, 0.8)`.

**The plan's specific decisions.** `apply` uses `project_point3`
(`space.rs:123`), not `transform_point3`. I read glam 0.30.10's source rather
than trusting the name: `project_point3` ends `res = res.div(res.w)`, a real
per-component division, so for an affine matrix with `w` exactly 1.0 the
result is bit-identical to the affine form, and the doc comment's claim about
IEEE 754 holds. `then` composes as `next.m * self.m` (`space.rs:146`).
`Transform` has exactly two constructors, `from_mat4` and `identity`, and no
third crept in. `value.rs` contains no arithmetic, no `From`, no `Into` and no
conversion of any kind between the three newtypes, so the modality LUT is not
duplicated out of `ocelli-pixel`.

**The composition test uses a genuinely non-commuting pair.** A translation of
+10 in x against a scale of 2 in x, giving 22 one way and 12 the other. Not
two translations. Both orders are pinned, in `then_applies_the_receiver_first`
and `the_other_composition_order_gives_the_other_answer`, and both went red
when I reversed the operands.

**The mechanically enforced rules, checked by hand.** No `as` cast anywhere in
`space.rs`, `value.rs` or any of the five test files. No `#[allow]` of any
lint, inner or outer. No `unwrap`, `expect` or `panic!`, including in tests.
No `unsafe`. No `wasm_bindgen` reachable from `ocelli-core`, confirmed by the
bindgen gate as well as by grep. `#![cfg_attr(not(test), no_std)]` is intact at
`lib.rs:8` and `cargo check -p ocelli-core --target wasm32-unknown-unknown`
exits 0. D-09 resolves as intended: `cargo tree --target wasm32-unknown-unknown`
shows `glam feature "libm"` with no `std`, and `libm v0.2.16` in the lock.

**Prose claims executed rather than accepted.** `lib.rs:5` says section 28
puts `space` and `value` first, and `docs/hld/25-first-ten-files.md` section
28 lists them as entries 1 and 2. `from_mat4`'s doc comment describes the
first column as the row cosine scaled by `PixelSpacing[1]` and the second as
the column cosine scaled by `PixelSpacing[0]`, which is correct. The
`roundtrip.rs` bound on `w` is right: `w = 1 + z/2000` for `z` in
`[-500, 500]` gives `[0.75, 1.25]` as claimed. The geometry fixture's header
arithmetic derives the values it states, term by term, in exact decimals.
`Cargo.lock` gained only glam, libm, trybuild and the proptest tree, nothing
unexpected in the non-dev graph.

**The worker's report about the plan's third mutation is accurate.** The plan
at line 259 called for replacing `project_point3` with `transform_point3` and
confirming the round-trip property goes red. Run against the affine test the
plan's listing specifies, in isolation, it stays green:

```
$ cargo test -p ocelli-core --test roundtrip -- --exact canvas_world_roundtrip
test canvas_world_roundtrip ... ok
test result: ok. 1 passed; 0 failed        exit 0
```

The two cases the worker added instead do go red, and so does the unit test
`apply_divides_by_the_resulting_w`. The report is correct on both halves. The
worker's numeric claim in the progress note, that mutation 1 makes P(1, 0)
read -45.05 as -44.9, is also right: under a transposition P(1,0) becomes
IPP + 0.5 * X = (-44.9, 118.38, -32.26).

**The compile-fail suite is a live guard, not decoration.** See mutation 8.

**Commands, each exit code read from the command itself.**

```
bin/ocelli.sh check ocelli-core                              exit 0
bin/ocelli.sh test ocelli-core                               exit 0, 22 tests
bin/ocelli.sh clippy ocelli-core                             exit 0
cargo check -p ocelli-core --target wasm32-unknown-unknown   exit 0
bin/ocelli.sh gate fmt clippy test bindgen unsafe pins \
  deviations prose content provenance                        exit 0, ALL GREEN
```

`bin/ocelli.sh clippy` passes `--all-targets`, so the test files are linted
and not merely compiled.

**Tree state after review.** Every mutation was reverted and verified.
`git status --short` and `git diff --stat` are identical to their state at the
start of the review, the two mutated source files hash back to their
pre-mutation md5 values, and the `proptest-regressions` directory that
mutation 3 creates was removed. Nothing was fixed and nothing was committed.

## Mutations run, and what went red

| # | Mutation | Result |
|---|----------|--------|
| 1 | `PixelSpacing[0]` and `[1]` transposed in `image_plane_transform` | RED. 4 of 6 geometry tests, including `the_transposition_is_visible_at_this_tolerance`, which correctly notices the two transforms have become identical |
| 2 | `then` composed as `self.m * next.m` | RED. Both `then_applies_the_receiver_first` and `the_other_composition_order_gives_the_other_answer` |
| 3 | `project_point3` replaced with `transform_point3` | RED. `apply_divides_by_the_resulting_w`, `canvas_world_roundtrip_under_perspective`, `the_perspective_divide_is_visible_at_this_tolerance`. GREEN, correctly, for `canvas_world_roundtrip` and the whole affine geometry fixture |
| 4 | P(0,1) expected `y` changed from 118.94 to 118.95 | RED. Exactly one test, `voxel_0_1_steps_one_row_by_pixel_spacing_0_along_the_column_cosine`, and no other |
| 5 | `identity()` returns `DMat4::ZERO` | RED. All three `identity_leaves_*` tests, plus `a_point_is_copy_and_survives_being_used_twice` incidentally through NaN |
| 6 | `inverse()` returns `self.m` unchanged | RED. `inverse_returns_a_point_to_the_space_it_came_from` |
| 7 | `Pt::new` swaps `x` and `y` | RED. 5 unit tests including `new_stores_its_arguments_in_order` |
| 8 | A `tests/ui` case altered so it compiles | RED. `mixing_spaces_does_not_compile` fails with a trybuild error, so the compile-fail suite is a real guard |
| 9 | `ROW_COSINE` and `COLUMN_COSINE` swapped in `image_plane_transform` | RED. The three interior geometry cases. `voxel_0_0_is_image_position_patient` stays green, correctly, since `i = j = 0` |
| 10 | `assert!(once.x == twice.x)` added to a unit test | RED at clippy, "strict comparison of `f32` or `f64`", exit 101. `float_cmp = "deny"` is live in test code |

An earlier attempt at mutation 5, replacing `DMat4::IDENTITY` with
`DMat4::from_scale`, failed to compile rather than failing a test, because
`identity()` is `const fn` and `from_scale` is not. It was redone with
`DMat4::ZERO` and is reported above as the version that ran.
