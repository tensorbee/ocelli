# F-077 review, pass 1

**Reviewed**: HEAD `568ca61bf66e4ec1553ba86830486bf8df21dcd1` on `sprint/s29`. That commit is F-075 and was not amended. The F-077 product is uncommitted. Modified: `crates/ocelli-geom/src/lib.rs`. Untracked: `crates/ocelli-geom/src/livewire.rs` and `crates/ocelli-geom/tests/livewire.rs`. After the mutation and the probe were removed, `shasum` SHA-1 values were `a9042af2e8f0dc6540f67dd6b4111c5f12c40ec7`, `990622e61a7e2cf9712c4e4ab977de109c763ee7`, and `d205f9ac02915eaabb50a908ffa360c6e7bcae16`. Those match the pins. `git diff` for `measure.rs` and `stats.rs` is empty.
**Author / reviewer**: The F-077 change has no commit, so this pass does not name an author from a commit or a trailer. The reviewer did not write the change.
**Result**: 0 defects, 0 smells, 0 nitpicks. Pass.

## Acceptance challenged

AC1. Costs `1, 100, 1, 1` with start `(0, 0)` and goal `(1, 1)` produce `(0, 0, 0), (0, 1, 0), (1, 1, 0)`. The entered sum through `(0, 1)` is 2. The other 4-connected route enters the cost-100 pixel and sums to 101. `the_cheap_path_follows_the_gradient` passed. The assertion is that three-point list, so a straight route through `(1, 0)` would fail it, and a two-point diagonal would fail it. The call passes `z` 9 and `z` 4. The expected points store `z` 0.

AC2. `the_mask_is_the_visited_pixels` expects `1, 0, 1, 1`. Filling the interior would set the cost-100 pixel and fail that vector. The test passed.

AC3. Start equal to goal at `(1, 0)` returns one point, `(1, 0, 0)`. `a_seed_is_a_one_pixel_path` passed. Treating the seed as `Degenerate` would fail it.

AC4. Cost `-1` is `Degenerate`. Cost `NaN` is `Frame`. Both checks live in `a_negative_cost_is_degenerate` and passed. A negative cost reported as `Frame`, or a `NaN` cost reported as `Degenerate`, would fail its assertion.

AC5. An empty path is `Degenerate`. `an_empty_path_is_not_a_blank_mask` compares with `Err` and passed. A zero mask would fail it.

AC6. Start `x` of `0.5` is `Degenerate`. `a_fractional_index_is_degenerate` passed. Using the truncated pixel and returning a path would fail the `Err` comparison.

Tie. Uniform costs `1, 1, 1, 1` must take `(0, 0, 0), (1, 0, 0), (1, 1, 0)`, because index 1 is settled before index 2. The parent test in `relax` was changed from a strict less-than rejection to a greater-than rejection, so an equal cost replaced the parent. `a_tie_keeps_the_lower_index` then failed at `crates/ocelli-geom/tests/livewire.rs:113`. The actual path was `(0, 0, 0), (0, 1, 0), (1, 1, 0)`. The original condition was restored. The full livewire file passed again.

Independent grid, one temporary test, then deleted. Width 2 and height 3. Costs `1, 50, 1, 50, 1, 1`. Start `(0, 0)` with input `z` 7. Goal `(1, 2)` with input `z` 8. The path was `(0, 0, 0), (0, 1, 0), (0, 2, 0), (1, 2, 0)`. Entered costs were the pixels at `(0, 1)`, `(0, 2)`, and `(1, 2)`, which are 1, 1, and 1. The sum is 3. The right-hand route would enter 50, 50, and 1, summing to 101. The mask was `1, 0, 1, 0, 1, 1`. The probe passed. It is not in the tree. The test file hash matches the pin.

## Evidence

| Command | Exit | Observation |
|---------|------|-------------|
| `bin/ocelli.sh test ocelli-geom --test livewire` | 0 | 7 passed and 0 failed on the pinned tree |
| `bin/ocelli.sh test ocelli-geom --test livewire a_tie_keeps_the_lower_index` after editing `relax` so an equal cost replaces the parent | 101 | failed at `tests/livewire.rs:113`. Actual path went through `(0, 1)` |
| same full livewire file after restoring `if !candidate.total_cmp(&known).is_lt()` | 0 | 7 passed. All three SHA-1 values matched the pins |
| `bin/ocelli.sh test ocelli-geom --test livewire review_probe_tall_grid_avoids_the_cost_50_column` | 0 | temporary 2 by 3 probe passed. The test was deleted. The test file SHA-1 matched the pin again |
| `bin/ocelli.sh clippy ocelli-geom` | 0 | `cargo clippy -p ocelli-geom --all-targets -- -D warnings` on the restored tree |

Source hashes at the end of the pass, from `shasum`:

- `crates/ocelli-geom/src/lib.rs` `a9042af2e8f0dc6540f67dd6b4111c5f12c40ec7`
- `crates/ocelli-geom/src/livewire.rs` `990622e61a7e2cf9712c4e4ab977de109c763ee7`
- `crates/ocelli-geom/tests/livewire.rs` `d205f9ac02915eaabb50a908ffa360c6e7bcae16`

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14). One cargo process ran at a time. The parent mutation was restored. wasm-pack was not run. No patient data was used.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The contract is the local plan in `.claude/plans/F-077-design.md`. It is not an HLD formula. `docs/hld/26-differentiating-capabilities.md` line 82 names interactive segmentation in the initial kernel set. A search of `docs/hld/` for livewire, neighbourhood, neighborhood, fill rule, and cost grid found nothing. Section 32 is prompted segmentation and does not define this path or this mask. The new comments say the HLD names the idea and does not define the cost. They do not claim that the HLD specifies the mask.

`integer_index` (`livewire.rs` 21-51) returns `Frame` for a non-finite value at lines 22-24, before the negative check at lines 25-27 and the fraction check at lines 28-31. The fraction check is `DVec2::new(value, 0.0).trunc().x` compared with `total_cmp`. It does not cast. A mismatch is `Degenerate`, so the truncated value is not used as a pixel.

`livewire_mask` (`livewire.rs` 330-348) returns `Degenerate` for an empty path before it builds a buffer. The loop at lines 336-339 checks every point with `?` and only then allocates the mask at line 341. A later out-of-range point therefore returns `Err` and cannot return `Ok`.

Neighbours are left, right, above, and below (`livewire.rs` 182-196). There is no diagonal. The search seeds the start distance with `0.0` at line 248, so the start pixel cost is not added. An equal candidate returns at lines 153-155 without writing a parent. A strictly smaller candidate writes the parent at line 163. `cheapest_unsettled` replaces its best cost only on `is_lt` at line 117, so the lowest index wins a tie. `point_from_index` stores `z` 0 at line 89. The seed return stores `z` 0 at lines 309-313.

`extern crate alloc` is at `lib.rs` 21. The module is `mod livewire` at line 23. The crate root re-exports both functions at line 27. There is no new trait and no `Box<dyn>`. `livewire.rs` does not mention `wasm_bindgen`, `polygon_area`, or `spline_point`. A search for an `as` cast, `unwrap(`, `expect(`, and `unsafe` in that file found none. `unwrap_or` at line 111 is not `unwrap`. Pixel reads use `get`, not slice indexing. Workspace clippy warns on `indexing_slicing` and denies `unwrap_used`, `expect_used`, and `panic`. `bin/ocelli.sh clippy ocelli-geom` exited 0.

The plan marks tiers A, B, and C as not applicable. These functions do not open a device and do not render. Refusal returns `Err` and drops the scratch vectors. No TypeScript file was in the uncommitted set.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. The plan adds `livewire_path` and `livewire_mask` to `docs/lld/geometry.md` and `docs/lld/public-api.md` at completion. Those files were not in this diff.

The function returns points, not the numeric path cost. The shipped tests lock the pixels. They do not observe a constant added to every distance. The excluded start cost was read at the distance seed.

A mask whose first point is in range and whose later point is not was not executed. The early `?` was read. Width 0, a mismatched buffer length, a negative index, and an infinite cost were not given their own runs. They use `cell_count`, `validate_costs`, and `integer_index`, which the executed refusals already enter.

Only `ocelli-geom` `--test livewire` was executed, plus the deleted probe. The other geometry tests were not run. Clippy still checked the crate targets.

The temporary parent edit and the temporary test are gone. The three SHA-1 values match the pins. Do not attach this result to a later tree. The review file is untracked.
