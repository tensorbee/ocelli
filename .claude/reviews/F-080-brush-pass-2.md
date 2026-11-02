# F-080 review, pass 2

**Reviewed**: branch `sprint/s30`, HEAD `ba6f3957770ddefaa029b9d82ef9d3fbec87ab8c`. The F-080 implementation is uncommitted. `crates/ocelli-seg/src/brush.rs` is untracked and its SHA-256 is `a8756735471dd7787d2538daf1e61e12529576ab0425571984be0a76d27f96dc`, the same pin as pass 1. `crates/ocelli-seg/tests/brush.rs` is untracked and its SHA-256 is `b95e96c182086722db3e229453d7c47bfac9a6a7251f1165fce8f2b5a7e33a3e`. Also uncommitted and not edited by this pass: `crates/ocelli-seg/src/lib.rs`, `CHANGELOG.md`, `docs/lld/public-api.md`, `docs/lld/README.md`. `git diff` is empty for `labelmap.rs`, `contour.rs`, `surface.rs`, and `crates/ocelli-seg/Cargo.toml`. The pass 1 report was not edited. After the mutation below was restored, `brush.rs` matched the pin again.
**Author / reviewer**: the assignment names the author as grok. This pass did not write the brush, the tests, or the docs, and it did not resume the pass 1 reviewer. There is no commit, so this pass does not take an author from a trailer. Independence is that separation. This is not a self-review of the implementation and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The HLD does not define a brush. Appendix B only counts tool classes. The approved plan `.claude/plans/F-080-design.md` is the contract. No deviation was filed.

A sample is inside when the squared index distance compares `<=` the squared radius through `total_cmp`. A circle uses `x` and `y` only, on the slice whose index is the same number as `center_z`, including both zero bit patterns. A non-integer `center_z` selects no slice. A sphere uses `x`, `y`, and `z` on every supplied slice. Both zero bit patterns are a zero radius. A negative radius is `Degenerate`. A non-finite centre or radius is `Frame`. Empty stack, zero side, bad length, and mismatched slices are `Degenerate`. A refusal writes nothing. Segment 0 erases. `covered` counts samples inside. `changed` counts those whose stored number differs.

Pass 1 ran these behaviours from a temporary test and then deleted it. The nine tests are now permanent. This pass re-derived the pixel results from the plan and checked them against the assertions.

Negative zero. Circle centre `(1, 1, -0.0)`, radius `-0.0`, segment 0, two slices of 1s. Both zero patterns are a zero radius, so this is not `Degenerate`. Only slice 0 matches, and radius 0 includes only index `1 * 3 + 1 = 4`. Covered 1, changed 1, that sample becomes 0, and slice 1 stays 1s. The green run asserts that, so it rejects painting slice 1. Removing the bit-pattern clause from `strictly_negative` makes `total_cmp` treat `-0.0` as below `+0.0`. The stamp then refuses. The test went red and the other seventeen stayed green. The panic is the refusal. It is not a pixel assertion. The green run is what locks the erased index.

Fractional circle. Centre `z` `0.5`, radius 10, two slices of 7s. No slice index equals `0.5`. Covered 0, changed 0, both slices stay 7s. A radius of 10 would paint every sample on a selected slice, so this rejects rounding `0.5` onto slice 0 or slice 1 and rejects painting every slice.

Empty stack and width 0. Both are `Degenerate`. The width 0 grid keeps an empty index buffer. Dropping the zero-side check would return `Ok` with covered 0, because the `x` loop would not run. The test expects `Degenerate`, so that guard is load-bearing. An empty slice cannot grow, because the argument is a slice rather than a `Vec`.

Non-finite centre and recovery. `+inf`, `-inf`, and `NaN` centre `x` are `Frame`. `+inf` radius is `Frame`. The nine 9s stay. A later radius 0 stamp writes segment 2 at index 4 with covered 1 and changed 1. Dropping the finite check would return `Ok` for a `NaN` or infinite centre, because a non-finite squared distance does not compare `<=` a finite squared radius, and an infinite radius would paint the 9s. The `Frame` matches and the 9s assertion reject that. The later stamp rejects a refusal that leaves the grid unable to take a valid write.

Long buffer. A 3 by 3 grid with 10 nines is `Degenerate` and all ten stay. The older length-8 test still rejects a short buffer. A check that rejected only one of those two lengths would fail one of the two tests.

Column then row. Radius 0 at `(x = 0, y = 1)` is linear index `1 * 3 + 0 = 3`. The test expects only that sample to become 4. A swapped axis would write index 1.

Outside corner. Start `[2, 0, 0, 0, 2, 0, 0, 0, 0]`, circle radius 1, segment 2. The plus covers five samples. Index 0 is outside and already 2. Index 4 is inside and already 2. Four inside samples change. Covered 5, changed 4, result `[2, 2, 0, 2, 2, 2, 0, 2, 0]`. Covered 5 rejects treating the outside corner as inside. Changed 4 rejects counting the already painted centre as a change.

Halfway sphere. Centre `(1, 1, 0.5)`, radius `0.5`, segment 5. Squared radius is `0.25`. Each slice centre has squared distance `0.25`, so the edge of the ball includes it. An orthogonal neighbour has squared distance `1.25` and is outside. Covered 2, changed 2, and each slice has only index 4 set to 5. That matches the derivation. Replacing `is_le` with `is_lt` would miss both centres, because the squared distance equals the squared radius. The test therefore locks that equality when the `z` term supplies it. It does not by itself reject a sphere that ignores `z`, because radius `0.5` also misses those edges in the plane. AC2 already rejects ignoring `z`. Slice 1 would become a plus and covered would not stay 6.

Radius just under 1. `1.0_f64.next_down()` is `1 - 2^-53`. Its square is strictly below 1. The edge squared distance is 1, so the edge is outside. The centre squared distance is 0, so the centre is inside. The test expects covered 1, changed 1, and only index 4 set. This test would stay green if `is_le` became `is_lt`, because the centre is strictly inside and the edge is strictly outside. AC1 remains the guard that an edge on radius 1 is inside. This test is the guard that a radius just below 1 does not include that edge.

AC9 still passes on the restored source. `present_labelmap` was not edited. Fill on and outline off keeps the LUT alpha at the centre. Index `(1, 1)` on a width of 3 is linear index 4, so byte 16. The corner is segment 0 and stays transparent. `stamp_brush` is not called from `graph.rs`, a shader, or TypeScript.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test brush` | `brush.rs` `a8756735...d27f96dc` and `tests/brush.rs` `b95e96c1...a7e33a3e`. 18 tests, 9 of them new. | Executed now, before the mutation. | Exit 0. 18 passed and 0 failed. |
| Same command after `strictly_negative` was reduced to `value.total_cmp(&0.0).is_lt()` | `brush.rs` lines 157-159 only. Tests file unchanged. | Executed now. | Exit 101. 17 passed and 1 failed. `a_negative_zero_radius_erases_only_the_matching_slice` panicked at `crates/ocelli-seg/tests/brush.rs:36:19`: `internal error: entered unreachable code: the plan's finite stamp`. |
| `shasum -a 256` of `brush.rs` and `tests/brush.rs` after the clause was restored | Restored `brush.rs`. The clause text occurs once. | Executed now. | Both hashes match the pins above. |
| `git diff` for `labelmap.rs`, `contour.rs`, `surface.rs`, and `Cargo.toml` | Working tree against HEAD. | Executed now. | Empty. |
| `rg stamp_brush` outside `crates/ocelli-seg` | Working tree, `target` excluded. | Executed now. | Only `docs/lld/public-api.md`. No TypeScript and no shader hit. |
| Clippy of `ocelli-seg` | Not this tree's command. | Prior receipt named by the assignment. Not executed now. | Reported exit 0 before this pass. |
| Pass 1 `is_le` to `is_lt` mutation | Pass 1 report. | Prior receipt. Not executed now. | Not used as evidence for this pass. |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14), date context 2026-11-02. The two cargo runs were sequential. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened. `/tmp/ocelli-s30-f084` was not read.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`brush.rs` was not edited except for the mutation that was restored. The restored hash is the pass 1 pin. The new tests call the same `stamp_brush`.

`strictly_negative` is `total_cmp` `is_lt` and then an exclusion of `0x8000_0000_0000_0000`. The green run shows radius `-0.0` stamps. The red run shows that the exclusion is why. `same_number` still treats both zero bit patterns as one slice, which is why the restored stamp erases slice 0 only. The finite check still runs before that, so `+inf`, `-inf`, and `NaN` stay `Frame`.

The distance helper squares `dx` and `dy`, adds a circle `dz` of 0 or a sphere `z` delta, and uses `total_cmp` `is_le`. There is no `sqrt`, no `as` cast, no `unsafe`, and no `wasm-bindgen` in `brush.rs`. `f64::from` on `u32` is exact. Indexing uses `get_mut`. Staged copies are assigned only after the slice loop returns. `Cargo.toml` does not name glam. The stamp does not open a device. Tiers A, B, and C are not a result split.

The new tests have no comments of their own. The file header says the expected indices are hand-computed in the plan and that the radius test is the plan rather than an HLD formula. The nine new expectations match the plan rules derived above.

The changelog bullet is true on the reading that a circle is the one-slice stamp and a sphere is the every-slice stamp. A sample on the squared radius is inside. A negative radius or a non-finite centre writes nothing. Segment 0 erases. The labelmap presenter diff is empty. A non-integer `center_z` paints no slice, which is the public-api equality sentence, and `a_fractional_circle_slice_paints_nothing` locks it. The public-api paragraph also matches the code. Both zero bit patterns select slice 0. Radius 0 paints only an exact integer centre. `present_labelmap` is unchanged. There is no TypeScript copy. The `README.md` row names the circle and sphere brush and lists F-080.

`lib.rs` exports `Brush`, `BrushCoverage`, `BrushError`, `BrushShape`, and `stamp_brush`. The integration test compiled that library and passed.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. It covers the uncommitted F-080 tree against the approved plan, with the nine new tests as the change since pass 1.

Clippy was not run. A wasm build was not run. The rest of `ocelli-seg` was not tested. No oracle and no corpus run. The pass 1 `is_le` to `is_lt` mutation was not repeated. Height 0 was not given its own call. It shares the zero-side check with the width 0 case that was executed. A negative infinity radius was not given its own call. It shares the finite check with the `+inf` radius case that was executed. A grid whose linear index does not fit in `u32` was not allocated. A negative `NaN` payload other than the infinities and the quiet `NaN` centre was not executed.

The halfway-sphere pixels do not by themselves prove that `z` was used. AC2 is the test that rejects ignoring `z`. The suite was not run again after the restore. The restored SHA-256 matches the file the green run executed.

The product bytes match the pins above. Do not attach this result to a later tree. The review file is untracked.
