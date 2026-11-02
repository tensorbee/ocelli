# F-080 review, pass 1

**Reviewed**: branch `sprint/s30`, HEAD `ba6f3957770ddefaa029b9d82ef9d3fbec87ab8c` (`S30, approve sprint designs`). The F-080 implementation is uncommitted. Paths: `crates/ocelli-seg/src/brush.rs`, `crates/ocelli-seg/tests/brush.rs`, `crates/ocelli-seg/src/lib.rs`, `CHANGELOG.md`, `docs/lld/public-api.md`, `docs/lld/README.md`. The approved plan `.claude/plans/F-080-design.md` is already in HEAD. After the mutation was restored and the temporary test was deleted, SHA-256 was `a8756735471dd7787d2538daf1e61e12529576ab0425571984be0a76d27f96dc` for `brush.rs` and `88c17a20b5cf7b9320cd3399f0b13a23e804496f180a0fe393bd2382490b78ef` for `tests/brush.rs`. Those match the hashes taken before any review edit. `git diff` for `labelmap.rs`, `contour.rs`, `surface.rs`, and `crates/ocelli-seg/Cargo.toml` is empty.
**Author / reviewer**: the assignment names the implementation author as grok. This reviewer did not write the brush, the tests, or the docs. The change has no commit, so this pass does not take an author from a trailer. Independence is limited to that separation. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The HLD Appendix B only counts tool classes. It does not define a brush. The approved plan is the contract. The squared-distance rule is that plan. It is not an HLD formula. No deviation was filed.

A sample is inside when the `f64` squared index distance compares `<=` the squared radius through `total_cmp`. There is no `sqrt` and no millimetre conversion. A circle uses `x` and `y` only, and only the slice whose index is the same number as `center_z`, including both zero bit patterns. A non-integer `center_z` selects no slice and is not an error. A sphere uses `x`, `y`, and `z` on every supplied slice. Radius 0 includes only an exact index centre. Both zero bit patterns are a zero radius. Negative radius is `Degenerate`. Non-finite centre or radius is `Frame`. Empty stack, zero side, bad length, and mismatched slices are `Degenerate`. A refusal writes nothing. Segment 0 erases. `covered` counts samples inside. `changed` counts those whose stored number differs.

AC1. The permanent test passed on the restored source. Replacing `is_le` with `is_lt` made `a_circle_of_radius_one_paints_the_plus_and_not_the_corners` fail at `crates/ocelli-seg/tests/brush.rs:44`. `coverage.covered` was 1 against 5. Radius 1 at `(1, 1)` keeps the centre strictly inside and puts the four edge samples on the radius, so the count 1 is the centre without those edges. The index assertion was not reached. This rejects a strict less-than stamp. The compare was restored.

AC2. The permanent two-slice sphere passed. A separate executed case, sphere centre `(1, 1, 0.5)` radius `0.5` segment 5, returned covered 2 and changed 2. Each 3x3 slice had only index 4 set. Squared distance at each centre is `0.25`, which equals the squared radius. An edge is `1.25` and is outside. That rejects ignoring `z`, and it rejects treating the sphere as a circle that matches no slice.

AC3 and the refusal paths. The permanent negative radius and `NaN` radius test passed, with the nines unchanged. The temporary case then sent `+inf` radius, `-inf` centre `x`, `NaN` centre `y`, and `-inf` radius. Each returned `Frame`. The nine 9s stayed. A following radius 0 stamp returned covered 1 and changed 1 and wrote segment 2 only at index 4. That rejects a non-finite stamp and a refusal that leaves the grid unable to accept a later valid stamp.

AC4. The permanent length 8 test passed. A length 10 buffer on a 3x3 grid returned `Degenerate`. All ten 9s stayed. That rejects stamping a long buffer.

AC5. The permanent radius 0 tests passed. Under `is_lt`, `radius_zero_paints_only_an_integer_centre` failed at line 114 with covered 0 against 1, because the exact centre has squared distance equal to a zero radius. Radius `-0.0` with centre `z` of `-0.0` on two slices of 1s returned `Ok`, covered 1, changed 1. Only slice 0 index 4 became 0. Slice 1 stayed 1s. That rejects classifying `-0` as `Degenerate`, and it rejects painting every slice for a negative-zero circle.

AC6. The permanent circle at `z = 1` and the one-slice circle at `z = -0.0` passed. A circle at `center_z` `0.5`, radius 10, on two slices of 7s returned `Ok`, covered 0, changed 0. Both slices stayed 7s. That rejects rounding `0.5` onto a slice and rejects painting every slice.

AC7. The permanent erase-then-repeat test passed. The `-0.0` radius case above also wrote segment 0 and counted the change. A second identical stamp is the permanent test, not a new execution.

AC8. The permanent 3x3 beside 2x2 test passed. Both slices stayed unchanged.

AC9. The permanent presenter test passed. Centre bytes at offset 16 were the LUT colour and corner offset 0 was transparent. `present_labelmap` in `labelmap.rs` writes segment 0 as `[0, 0, 0, 0]`, addresses colour from segment 1, and uses row-major order times 4. Fill with outline off keeps the LUT alpha. Index `(1, 1)` on a width of 3 is linear index 4, so byte 16. `git diff` for `labelmap.rs` is empty. The `is_lt` mutation left this test green, because the centre is strictly inside radius 1 and the corner is outside. AC1 is the boundary guard. AC9 does not replace it.

An axis case the permanent plus cannot see: radius 0 at `(x = 0, y = 1)` painted only index 3. Index 1 stayed 0. That rejects swapped column and row. A partial circle on `[2, 0, 0, 0, 2, 0, 0, 0, 0]` returned covered 5 and changed 4, and the outside corner that already held 2 stayed 2. That rejects counting an outside sample as a change. Width 0 and height 4 with an empty index buffer returned `Degenerate` rather than `Ok` with covered 0. An empty stack returned `Degenerate`. Radius `1.0_f64.next_down()` painted only the centre, so an edge just outside the squared radius stays out.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test brush` | Product `brush.rs` and `tests/brush.rs` at the SHA-256 pins above. Nine permanent tests. | Executed now. No prior receipt. | Exit 0. 9 passed, 0 failed. |
| Same command after `dist_sq.total_cmp(&radius_sq).is_le()` was changed to `is_lt` | `brush.rs` line 154 only. | Executed now. | Exit 101. 4 passed, 5 failed. AC1 failed at `tests/brush.rs:44`, covered left 1 right 5. AC2 failed at line 56, covered left 1 right 6. AC5 failed at line 114, covered left 0 right 1. AC6 failed at line 137, covered left 1 right 5. AC7 failed at line 162, covered left 0 right 1. AC3, AC4, AC8, and AC9 stayed green. |
| `shasum -a 256` and `rg -n "is_le\|is_lt"` after restore | Restored `brush.rs`. | Executed now. | Hash matches the pin. Line 154 is `is_le`. The only `is_lt` is line 158, `strictly_negative`, which the pinned file already had. `git diff -- brush.rs` is empty because the file is untracked and the bytes match the pin. |
| `bin/ocelli.sh test ocelli-seg --test brush review_pass_temporary_cases` | Temporary test appended to `tests/brush.rs`, then deleted. `brush.rs` was the restored pin. | Executed now. | Exit 0. 1 passed, 9 filtered out. The first draft of that test expected the wrong corner and failed at the pattern assertion. The expectation was corrected to the outside corner that already held segment 2. The rerun exited 0. The test was then deleted. |
| `shasum -a 256` of both product files, `git status --short`, `git diff` for `labelmap.rs`, `contour.rs`, `surface.rs`, `Cargo.toml` | After the temporary test was removed. | Executed now. | Both hashes match the pins. Labelmap, contour, surface, and the seg manifest have no diff. |
| `rg stamp_brush` over Rust, TypeScript, and shader names | Working tree. | Executed now. | Hits only `brush.rs`, `lib.rs`, and `tests/brush.rs`. No hit in `graph.rs`, shaders, or TypeScript. |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14), date context 2026-11-02. One cargo process ran at a time. The `is_lt` edit and the temporary test are gone. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened. F-084 was not read.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`stamp_brush` checks a finite centre and radius first, then a negative radius, then an empty stack, a zero side, a `width * height` product, and equal length on every slice. Copies are staged after that validation. The caller grids are assigned only after the slice loop returns. Early `Frame` and `Degenerate` returns were executed and left the bytes in place. The mid-loop offset `Err` path was read. It returns before that assignment.

The distance helper uses `dx`, `dy`, and a `dz` of 0 for a circle. A sphere uses `z_f - center_z`. The predicate is `total_cmp` `is_le`. `f64::from` on `u32` is exact. There is no `sqrt`, no `as` cast, no `unwrap`, no `expect`, and no `panic` in `brush.rs`. Indexing uses `get` and `get_mut`. `Brush` derives `Debug`, `Clone`, and `Copy`, and it does not derive `PartialEq`. `BrushShape`, `BrushCoverage`, and `BrushError` do not hold an `f64`. `strictly_negative` treats the `0x8000_0000_0000_0000` bit pattern as not negative. `same_number` treats both zero bit patterns as one slice. Those two paths were executed.

`lib.rs` is `no_std` outside the crate's own unit tests and declares `extern crate alloc`. `brush.rs` uses `alloc::vec::Vec`. The integration test compiles that library without `cfg(test)` and passed. `crates/ocelli-seg/Cargo.toml` was not edited and does not name glam. It already depends on `ocelli-core`. The brush module does not reference glam or wgpu. The stamp does not open a device. Tiers A, B, and C are not a result split.

The new changelog bullet and the new public-api paragraph match the code on the claims they make. A sample on the squared radius is inside. A circle paints only the equal `center_z` slice, and both zero bit patterns select slice 0. A sphere uses every slice. Radius 0 paints only the integer centre. A negative radius or a non-finite centre writes nothing. Segment 0 erases. `present_labelmap` is unchanged. There is no TypeScript copy. The function docs also name a non-finite radius as `Frame`. That refusal was executed. The changelog sentence "A circle paints one slice" is the contrast with a sphere across slices. A non-integer `center_z` paints none, which the public-api equality sentence states and the `0.5` case executed.

`present_labelmap` is called from the brush test and from `graph.rs`. `stamp_brush` is not called from `graph.rs`, a shader, or TypeScript. The presenter formula was not edited.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. It covers the uncommitted F-080 tree against the approved plan.

Clippy was not run. A wasm build was not run. The rest of `ocelli-seg` was not tested. No oracle and no corpus run. A grid whose linear index does not fit in `u32` was not allocated. A negative `NaN` bit pattern other than the infinities and the quiet `NaN` centre was not executed. The permanent suite does not name radius `-0.0`, a non-integer `center_z`, an empty stack, a zero side, a long buffer, or the off-centre radius 0 axis case. Those were executed from a temporary test that has been deleted. They are not locked by the permanent file.

The product bytes match the pins above. Do not attach this result to a later tree. The review file is untracked.
