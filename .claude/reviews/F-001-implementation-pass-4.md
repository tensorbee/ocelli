# F-001, implementation review, pass 4

**Reviewer**: independent agent, wrote neither the code nor passes 1 to 3
**Diff reviewed**: working tree, base cd74768
**Result**: 0 defects, 0 smells, 3 nitpicks

This pass is clean on defects and smells. The evidence is below rather than
the assertion, because a clean pass and a shallow one produce the same
sentence otherwise.

## Round 3's changes, verified

### The D-08 guard now reaches all six derives. CONFIRMED, by all three mutations.

`assert_marker_bounds<T>() where T: Debug + Clone + Copy + PartialEq + Eq +
Hash` at `crates/ocelli-core/src/space.rs:268`, called once per marker at
lines 315 to 317.

**M1, full revert to bare markers.** Replaced all three
`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub enum X {}` with
`pub enum X {}`.

```
error: could not compile `ocelli-core` (lib test) due to 19 previous errors
TESTEXIT=101
```

The diagnostics include E0369 and E0277, which is exactly what the D-08 row in
`docs/hld/DEVIATIONS.md` claims rustc reports. That claim is therefore
executed, not quoted.

**M2, partial revert to `#[derive(Debug, PartialEq)]`.** This was green before
round 3.

```
error[E0277]: the trait bound `space::Index: Hash` is not satisfied
   --> crates/ocelli-core/src/space.rs:317:32
note: required by a bound in `assert_marker_bounds`
error: could not compile `ocelli-core` (lib test) due to 9 previous errors
TESTEXIT=101
```

Nine errors, three markers times Clone, Copy, Eq and Hash minus the
supertrait-implied duplicates, every one of them raised at the
`assert_marker_bounds::<T>()` call site. Round 3's claim holds.

**M3, removing only `Clone`.** `#[derive(Debug, Copy, PartialEq, Eq, Hash)]`.
Red twice over, and importantly not only at the derive site:

```
error[E0277]: the trait bound `Canvas: Clone` is not satisfied
    |                                ^^^^^^ the trait `Clone` is not implemented for `Canvas`
270 |         T: core::fmt::Debug + Clone + Copy + PartialEq + Eq + core::hash::Hash,
TESTEXIT=101
```

So the guard's own doc claim, that naming `Clone` and `PartialEq` explicitly
makes a single deletion fail "against a bound that names it", is true and not
merely satisfied by `derive(Copy)`'s own `Clone` requirement.

**The guard is also the only thing holding D-08.** I removed the guard and its
helper AND fully reverted the three markers to bare, then ran the whole crate
suite:

```
running 13 tests ... test result: ok. 13 passed
running 1 test  ... test result: ok. 1 passed      (trybuild)
running 6 tests ... test result: ok. 6 passed      (geometry fixture)
running 3 tests ... test result: ok. 3 passed      (roundtrip)
EXIT=0
```

Green. The sentence at `space.rs:306`, "Without this test the deviation is
exercised by nothing", is measured true.

### The glam `cfg` quotation. CONFIRMED exact.

`space.rs:163` quotes `any(all(debug_assertions, feature =
"debug-glam-assert"), feature = "glam-assert")` from
`glam-0.30.10/src/macros.rs`. Read from
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.30.10/src/macros.rs`
lines 1 to 14:

```rust
#[cfg(any(
    all(debug_assertions, feature = "debug-glam-assert"),
    feature = "glam-assert"
))]
macro_rules! glam_assert { ($($arg:tt)*) => ( assert!($($arg)*); ) }
#[cfg(not(any(
    all(debug_assertions, feature = "debug-glam-assert"),
    feature = "glam-assert"
)))]
macro_rules! glam_assert { ($($arg:tt)*) => {}; }
```

Byte for byte the same predicate, and the two-arm expansion the comment
describes. The comment's derived conclusion, that `glam-assert` panics in any
profile and `debug-glam-assert` only in a debug one, follows correctly from
that predicate.

Two supporting facts also check out. `DMat4::inverse` at
`glam-0.30.10/src/f64/dmat4.rs:718` does carry `glam_assert!(dot1 != 0.0)`, so
the comment is talking about a real assertion and not an imagined one. And
`cargo tree -p ocelli-core -e features` resolves `glam feature "libm"` and
nothing else, so "Neither feature is on here" is true of this tree. `glam` is
named by exactly one crate manifest in the workspace, so there is no other
crate to unify a feature in from.

### The plan's compile-fail row. CONFIRMED, and now matches the disk.

The row claims "Four cases" and names `crates/ocelli-core/tests/ui/`.

```
$ ls -1 crates/ocelli-core/tests/ui/*.rs | wc -l
       4
```

apply_refuses_a_point_from_another_space.rs,
identity_refuses_to_cast_between_spaces.rs,
then_refuses_a_transform_that_does_not_join.rs,
value_spaces_do_not_interconvert.rs. The four the row names, in that order.
The stale claim that survived three rounds is gone.

I also checked the row's neighbours, since this is the file with the record.
The `fixture` row says "four named voxel indices" and the fixture has four
(`voxel_0_0`, `voxel_1_0`, `voxel_0_1`, `voxel_255_191`). The Cargo.toml
section says "Three edits" and the workspace manifest gained exactly three
dependency edits (`glam` features, `proptest`, `trybuild`). The
`[workspace.dependencies]` comment says section 15.2 gives five entries and
the block has seven: `docs/hld/12-workspace-and-build.md` lines 78 to 82 list
five, and the block lists seven. The same comment says the `[profile.release]`
block below is still verbatim and still says so, and it is, matching HLD lines
83 to 88 exactly with the label at Cargo.toml:75.

### The exact `f64` comparison at `space.rs:344`. Sound, not merely present.

The justification is that both operands are constructed from identical
literals with no arithmetic in between, that the comparison itself lives
inside the derived `PartialEq` where `DEVIATIONS.md` argues it belongs, and
that the guard cannot exercise `PartialEq` without an equality.

All three legs hold. `a` and `b` are `Pt::<Canvas>::new(1.5, -2.25, 3.75)`
with no operation applied, so exact equality is deterministic and a tolerance
would be strictly weaker for no gain. `DEVIATIONS.md`'s D-08 essay does weigh
the two options and does prefer keeping the float comparison inside a derive
expansion. And `assert_eq!` on two `Pt<Canvas>` is precisely the call the
deviation record cites as the thing that fails to compile without D-08, so
replacing it with anything else would stop guarding the deviation. The
`assert_ne!(a, c)` on the next line pins the other side, so a `PartialEq` that
always returned true fails.

## New defects

None.

## New smells

None.

## Nitpicks

### N1, `space.rs:339` instructs future reviewers not to raise a finding

"This is an exact `f64` comparison and it is deliberate, so please do not
raise it again." The argument that follows is sound and is worth keeping. The
clause addressed to reviewers is review correspondence rather than a fact
about the code, and it asks the next independent pass to skip a check rather
than to make it quickly. Deleting seven words leaves the justification intact.
Preference, does not block.

### N2, `roundtrip.rs:103` cites two drift figures no one can check

"it has reported drifts of 51.1 and 71.4 on different runs". This is a report
of past generator output, and it is the one sentence in the diff a later
reader cannot falsify. My own `project_point3` to `transform_point3` mutation
reported `-77.11981745175126`, which is a third number, entirely consistent
with the sentence's own point that the value depends on what proptest shrinks
to. The checkable number is on the fixed case immediately below and is exact,
so nothing rests on this. Preference, does not block.

### N3, half of `lib.rs`'s scaffold test is a tautology, and it is base-commit code

`lib.rs:26`, `assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"))`, where
`CRATE_NAME` is defined four lines above as `env!("CARGO_PKG_NAME")` in the
same crate. Both sides expand to the same literal, so that assertion cannot
fail for any edit. The second assertion, `starts_with("ocelli")`, is real, so
the test as a whole can go red and this is not a vacuous test. `lib.rs` is in
this diff but these five lines are not, so this is the integrator's call
rather than a remediation for F-001. Recorded because it is the same defect
class this story produced three times and it sits in the file the story
touched. Preference, does not block.

## What I checked and found correct

### Geometry against DICOM PS3.3 C.7.6.2.1.1, recomputed independently

I recomputed every expected value from the standard with `fractions.Fraction`,
from `P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y`, without
running the Rust and without reading the fixture's arithmetic first:

```
P(0,0)     = ('-226/5', '1187/10', '-65/2')   = (-45.2,  118.7,  -32.5)
P(1,0)     = ('-901/20', '5927/50', '-1619/50') = (-45.05, 118.54, -32.38)
P(0,1)     = ('-224/5', '5947/50', '-817/25')   = (-44.8,  118.94, -32.68)
P(255,191) = ('1389/20', '6187/50', '-907/25')  = (69.45,  123.74, -36.28)
```

All four match the fixture's asserted values exactly. Every one is an exact
terminating decimal, so the 1e-6 mm tolerance is a floor and not a fudge.

The crossing is right where it matters. `SPACING_BETWEEN_COLUMNS` is
PixelSpacing[1] = 0.25 and multiplies `ROW_COSINE` = IOP[0..3], which is the
`i` term. `SPACING_BETWEEN_ROWS` is PixelSpacing[0] = 0.5 and multiplies
`COLUMN_COSINE` = IOP[3..6], which is the `j` term. That is C.7.6.2.1.1's
Delta-i as column pixel resolution and Delta-j as row pixel resolution, and it
is the transposition the microscope checklist names.

Orthonormality checked exactly: X.X = 1, Y.Y = 1, X.Y = 0. The matrix printed
in the file header, with `(i, j, 0, 1)` as the column vector and a zero third
column, is C.7.6.2.1.1's own layout, and the `Index` doc at `space.rs:42`
correctly says the literal zero is the standard's and that the `z` slot is
this crate's own addition.

`roundtrip.rs`'s `FRAME_Z` claims to be `FRAME_X` cross `FRAME_Y`. Computed
exactly: `(0, 3/5, 4/5)`, and the constant is `(0.0, 0.6, 0.8)`. True, and the
frame is right-handed.

`the_transposition_is_visible_at_this_tolerance` asserts the separation
exceeds `TOLERANCE_MM * 1000.0` = 1e-3. The exact separations are 0.16, 0.2
and 63.72 mm. The margin is real.

### Mechanical rules

- No `as` cast anywhere in `src/` or `tests/`. Grep over the cast forms
  returns nothing.
- No `#[allow]`, no `unwrap`, no `.expect(`, no `panic!`, no `unsafe`, no
  `wasm_bindgen`. Grep returns nothing across all nine new files.
- `#![cfg_attr(not(test), no_std)]` intact at `lib.rs:10`, and
  `cargo check -p ocelli-core --target wasm32-unknown-unknown` exits 0, which
  builds the lib only and therefore actually exercises the `no_std` arm.
- The one exact `f64` comparison is judged above and is sound.
- Boundary and tier: this is CPU-side type and arithmetic code. No pixels, no
  wasm-bindgen, no wgpu, no `queue.submit`, no allocation on any path. The
  plan's tier block declares n/a for A, B and C with a stated reason, which is
  the declared answer the rule asks for rather than an omitted row.
- Structure: no new trait, no generic with one implementer, no `Box<dyn>`, no
  forwarding wrapper. The generics here are the HLD's own phantom parameters.

### Prose claims executed rather than accepted

- `lib.rs` says `space` and `value` are the two modules HLD section 28 puts
  first. `docs/hld/25-first-ten-files.md` is section 28 and rows 1 and 2 are
  those two files. True.
- The D-08 module block's claim that `PhantomData` implements `Debug` and
  `PartialEq` for any `S` without a bound is true of `core`.
- The comment above `impl<S> Clone for Pt<S>` says the derived form now
  compiles in this tree, and the comment above `Transform`'s impls says the
  same of `A` and `B`. I executed both together: deleted all four hand impls,
  added `#[derive(Debug, PartialEq, Clone, Copy)]` to `Pt` and
  `#[derive(Debug, Clone, Copy)]` to `Transform`, and got
  `test result: ok. 14 passed`. Both claims are true, and the stated reason
  for keeping the hand impls anyway, that they are unconditional where a
  derive would bound the parameter, is the correct reason.
- `inverse`'s doc says `inverse_does_not_check_invertibility` executes the
  claim against a singular matrix. It does, the fixture matrix is the
  in-plane C.7.6.2.1.1 matrix whose third column is zero, the test asserts the
  determinant is zero rather than assuming it, and it asserts all three
  returned components are non-finite.
- `roundtrip.rs:125` claims that dropping the `w` divide returns
  `9912.435180867995` instead of `9999.0`, which is 86.5648 CSS pixels. My
  `transform_point3` mutation printed `x drifted to 9912.435180867995`. Exact
  match, and 9999 minus that is 86.564819132005.
- `roundtrip.rs`'s module block says 1e-6 is roughly a quarter of a million
  times tighter than 25.1's quarter-pixel canvas row. 0.25 / 1e-6 = 250000.
  True.
- HLD 25.1's geometry bullet does carry two numbers, world at 1e-6 mm and
  canvas at a quarter pixel, which is what `SPACE_UNIT_TOLERANCE`'s doc now
  says, and 25.1 does have no row for voxel index space, which it also says.
- No em-dash and no prose semicolon in any new file. The only semicolons in
  comments are inside `roundtrip.rs`'s verbatim transcription of HLD section
  25's Rust listing, which is code.

### Tests that pass and would not if the code were wrong

Every mutation below went red and was reverted. See the next section for the
list and the outputs.

### Tests examined for vacuity, and the verdict on each

- `d_08_keeps_the_marker_derives_load_bearing`. Not vacuous, by M1, M2 and M3.
  The three-line `assert_marker_bounds` block is the load-bearing part and it
  is a compile-time check, which is the only kind available for a trait bound.
- `each_newtype_is_copy` in `value.rs`. Not vacuous. It looks like three
  tautological `to_bits` comparisons, but the tuple construction moves the
  three values and the assertions read them afterwards, so `Copy` is required
  to compile. Dropping `Copy` from the three derives gives
  `error[E0382]: use of moved value: s`, `m`, `d`. Red.
- `each_newtype_carries_its_field_unchanged` in `value.rs`. This one genuinely
  cannot fail while the three remain tuple structs with public fields, and its
  own doc comment says exactly that in four sentences and points at the
  trybuild case that does guard the section 16.1 claim. Raised as pass 2 N5
  and dispositioned there. I re-checked the disposition rather than re-raising
  it: the honest label at the site is what stops it being counted as coverage,
  and it is present.
- `mixing_spaces_does_not_compile` and its four ui cases. Not vacuous. Two
  independent mutations turned it red, one that widened `identity` and one
  that collapsed `Modality` into a type alias.
- `the_fixture_frame_is_orthonormal` and
  `the_transposition_is_visible_at_this_tolerance`. Both are meta-assertions
  about the fixture rather than about the library, which is the right shape
  for them, and both are reachable: the transposition mutation turned the
  second one red with "moves the point by only 0 mm".
- `inverse_does_not_check_invertibility`. Not vacuous. It is the only thing in
  the crate that would notice a glam patch release changing the singular-matrix
  behaviour the `inverse` doc and F-023 are told to rely on.
- The `separation > 1.0` tail of
  `the_perspective_divide_is_visible_at_this_tolerance`. Reachable, and it is
  what proves that case is projective rather than accidentally affine.

### Commands run, with their own exit codes

Exit codes read from the command, never from a pipe.

```
bin/ocelli.sh check ocelli-core                                   EXIT=0
bin/ocelli.sh test ocelli-core                                    EXIT=0
   14 passed (lib), 1 passed (trybuild), 6 passed (fixture), 3 passed (roundtrip)
bin/ocelli.sh clippy ocelli-core                                  EXIT=0
bin/ocelli.sh gate fmt clippy test bindgen unsafe pins \
  deviations prose content provenance                             EXIT=0
   ALL GREEN  10 gate(s)
cargo check -p ocelli-core --target wasm32-unknown-unknown        EXIT=0
```

All five re-run after every mutation was reverted, all still green.

## Mutations run, and what went red

| # | Mutation | Result |
|---|----------|--------|
| M1 | All three markers reverted to bare `pub enum X {}` | Red. 19 compile errors, including E0369 and E0277 |
| M2 | All three markers to `#[derive(Debug, PartialEq)]` | Red. 9 errors, all at `assert_marker_bounds` |
| M3 | `Clone` removed from the three derives only | Red. Errors at the derive site AND at line 270's bound |
| M4 | `then` composes `self.m * next.m` | Red. `then_applies_the_receiver_first` and `the_other_composition_order_gives_the_other_answer` |
| M5 | `project_point3` to `transform_point3` | Red. `apply_divides_by_the_resulting_w`, `canvas_world_roundtrip_under_perspective`, `the_perspective_divide_is_visible_at_this_tolerance`. Printed `x drifted to 9912.435180867995` |
| M6 | `di` and `dj` transposed in `image_plane_transform` | Red. Four of six fixture tests, including the transposition meta-test |
| M7 | One digit, `123.74` to `123.75` in P(255, 191) | Red. `voxel_255_191_accumulates_both_terms` only |
| M8 | `Copy` dropped from the three value newtypes | Red. `E0382: use of moved value` three times |
| M9 | `Modality` collapsed to `pub type Modality = Stored` | Red. `value_spaces_do_not_interconvert` trybuild case, 1 of 4 failed |
| M10 | `identity` widened to `impl<A, B> Transform<A, B>` | Red. `identity_refuses_to_cast_between_spaces`, 1 of 4 failed |
| M11 | `Pt::new` stores `x: y, y: x` | Red. 6 of 14 lib tests |
| M12 | `identity()` returns a z-scaling matrix | Red. All three `identity_leaves_*` tests |
| M13 | D-08 guard and helper deleted AND markers reverted bare | GREEN, which is the point. The guard is the only thing holding D-08 |
| M14 | Hand impls deleted, `Clone, Copy` derived on `Pt` and `Transform` | GREEN, which confirms the two comments that say the derived form now compiles |

Every mutation was reverted and the revert proved with `md5 -q` against the
digests taken before the first mutation. All thirteen source and plan files
match:

```
SAME .claude/plans/F-001-design.md                111a4a... etc, all 13 SAME
SAME crates/ocelli-core/src/space.rs
SAME crates/ocelli-core/src/value.rs
...
```

One artefact my own run created, `crates/ocelli-core/tests/
roundtrip.proptest-regressions`, written when M5 made the perspective proptest
fail, was deleted. It was absent from the file listing taken at the start of
this pass and its absence is confirmed by re-listing. `git status --porcelain
-uall` is byte-identical to the status at the start of the pass. Nothing was
fixed and nothing was committed.

## Verdict

**PASS.** Zero defects, zero smells, three nitpicks, none of which blocks.

Round 3's four claims are all true and all executed rather than read. The
D-08 guard is now genuinely load bearing across all six derives, which the
partial-revert mutation proves and which the previous round could not claim.
The geometry is right against PS3.3 C.7.6.2.1.1 recomputed from the standard
in exact rational arithmetic. Every test in the story that names a behaviour
goes red when that behaviour is broken, with the single exception of
`each_newtype_carries_its_field_unchanged`, which cannot fail, says so at the
site in four sentences, and was already dispositioned in pass 2.

Two known items are excluded by instruction and are not counted above: D-09 is
guarded by nothing, and `.gitignore` does not cover `*.proptest-regressions`.
The worktree's `docs/hld/DEVIATIONS.md` is one paragraph behind the canonical
copy, also by instruction not reported.
