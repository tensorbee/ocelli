# F-079 review, pass 1

**Reviewed**: HEAD `7e300f3ada45dbeb26b364e4162f486be7521f05` on `sprint/s29`. That commit is F-078 and was not amended. The F-079 product is uncommitted. Modified: `crates/ocelli-geom/src/lib.rs` and `.claude/plans/F-079-design.md`. Untracked: `crates/ocelli-geom/src/redact.rs` and `crates/ocelli-geom/tests/redact.rs`. After the length mutation was restored, `shasum` SHA-1 values were `42229e255f890ff0a09f7a10b659b9cd894db88f`, `d2258646b7ca2e92299bbeb071032b2e242931bc`, `f21a8e9c918163a99cbed209d8bf1f1551972d34`, and `871076d989541f1d72a98836a60dc8a995ee2395`. Those match the pins. `redact.rs` SHA-256 is `75a8d4ab4a4fc656772cbe3030c627edad5304154c007474230ce44089fbf58f`. `git diff` for `measure.rs`, `stats.rs`, and `packages/core/src/annotation.ts` is empty.
**Author / reviewer**: The F-079 change has no commit, so this pass does not name an author from a commit or a trailer. The reviewer did not write the change.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

The contract is the local plan in `.claude/plans/F-079-design.md`. The HLD does not define an ultrasound direction or a redaction rectangle. `docs/hld/B-parity-surface.md` only counts tool classes. A search of `docs/hld/` for these two function names found nothing.

The plan diff against HEAD is the arithmetic correction only. An earlier sentence said the span `(0, 0, 0)` to `(0, 2, 9)` has length 2 while also saying `z` is in the length. Those two claims cannot both be true. The corrected sentence says the length is the square root of `4 + 81`. That follows the existing rule. It does not replace it. The code uses `DVec3` length, so it follows the corrected number. The NaN row changed from `1, NaN, 3, 4` to `1, 2, NaN, 4`. Index 1 is not the row-major left column. Index 0 and index 2 are. The refusal decision is unchanged. The test and the code use the corrected buffer.

AC1. Positive `y` is `PI / 2` and the square root of `4 + 81`. `positive_y_is_half_pi` passed on the restored tree. The required mutation replaced `DVec3::new(dx, dy, dz).length()` with `DVec2::new(dx, dy).length()`. That drops `z` from the length. The same test then failed at `crates/ocelli-geom/tests/redact.rs:37`, the comparison with `squared.sqrt()`. Exit 101. The angle assertion on the line above did not fail, which is what a length-only edit should do. The `DVec3` line was restored. A length of 2 would also fail the explicit `is_ne` against `2.0` on the restored test. That assertion passed after the restore.

AC2. Identical points, including both signs of zero, are `Degenerate`. `a_zero_direction_is_degenerate` passed. An `Ok` angle of 0 would fail the `Err` comparison.

AC3. The row-major left column of `1, 2, 3, 4` becomes `0, 2, 0, 4`. `the_left_column_is_redacted` passed. Filling the whole frame, or the first two samples, would fail `same_bits`.

AC4. The same rectangle again reports written 0, already 2, and applied true. `a_second_pass_says_the_pixels_were_unchanged` passed. Reporting a write that did not happen would fail the `Ok` comparison.

AC5. A rectangle at `(5, 5)` reports applied false and leaves the buffer unchanged. `a_missed_rectangle_is_not_applied` passed. An applied miss would fail that comparison.

AC6. The buffer `1, 2, NaN, 4` with the left column returns `Frame`. The first sample stays 1 and the third stays `NaN`. `a_non_finite_sample_blocks_the_write` passed. Writing the finite neighbour and then refusing would fail the first-sample comparison.

The five extra tests also passed. A non-finite point is `Frame`. A non-finite fill writes nothing. A zero rectangle side is not applied. A short buffer is `Degenerate` and stays short. A negative-zero sample with a positive-zero fill counts as already filled, and the stored bits stay negative zero.

`distance` is the unchanged F-070 function. `measure.rs` has an empty diff. `redact.rs` does not define another `distance`. The length call is the private `span_length` shape, a `DVec3` length, written at the call site because `span_length` is private to `measure.rs`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom --test redact positive_y_is_half_pi` after `DVec2` length | Mutated `redact.rs` only. The other three files were untouched | Executed now | Exit 101. `positive_y_is_half_pi` failed at `tests/redact.rs:37`. 10 tests filtered out. The mutated library also warned that `DVec3` was unused |
| `bin/ocelli.sh test ocelli-geom --test redact` | Restored tree. SHA-1 pins matched before this run | Executed now | Exit 0. 11 passed and 0 failed |
| `bin/ocelli.sh clippy ocelli-geom` | Restored tree | Executed now | Exit 0. `cargo clippy -p ocelli-geom --all-targets -- -D warnings` |
| `cargo fmt -p ocelli-geom -- --check` | Restored tree. This check does not write | Executed now | Exit 1. Two diffs, recorded as the nitpick below |
| `shasum` and `shasum -a 256` | After the restore, and again after the tests, clippy, and the format check | Executed now | All four SHA-1 values match the pins. `redact.rs` SHA-256 matches the pin |
| `git diff` for `measure.rs`, `stats.rs`, and `annotation.ts` | HEAD `7e300f3ada45dbeb26b364e4162f486be7521f05` | Executed now | Empty |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14). One cargo process ran at a time. The length mutation was restored. wasm-pack was not run. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

`cargo fmt -p ocelli-geom -- --check` exits 1. `bin/ocelli.sh fmt` is that check across the workspace, so this crate would fail it. The behavior tests passed on the same bytes. The fix is mechanical and does not change a result.

`crates/ocelli-geom/src/lib.rs` places `pub use redact` before `pub use measure`. rustfmt moves the redact import to after the measure import and before `pub use stats`. The `mod` lines are already in order.

`crates/ocelli-geom/tests/redact.rs` line 36 keeps the radians `assert!` on one line. rustfmt wraps that call. `rustfmt.toml` sets `max_width` 100 and leaves the small-heuristic default in place.

## Verified clean

`ultrasound_direction` (`redact.rs` 86-108) returns `Frame` when a point is non-finite, when a component difference is non-finite, when the `DVec3` length is non-finite, or when `to_angle` is non-finite. It returns `Degenerate` when `exact_zero` matches the length bits, which covers both signs of zero. The angle is `DVec2::new(dx, dy).to_angle()` at line 103. glam 0.30.10 implements that as `atan2(y, x)`, so this call is `atan2(dy, dx)`. `z` is not an argument. The length at line 96 is `DVec3::new(dx, dy, dz).length()`, which is `sqrt` of the dot product. For `(0, 2, 9)` that dot product is `0 + 4 + 81`. The passing test shows that value matches `squared.sqrt()` and is not `2`.

`redact_rect` (`redact.rs` 119-189) returns `Degenerate` for a zero frame, for a `u32` product overflow, and for a buffer length other than `width * height`, all before a write. A non-finite fill returns `Frame` before the span is classified. `axis_span` returns `None` for a zero side or an origin outside the frame, and the function then returns `applied: false` with both counts at 0. A partial overlap clamps the end with `min` against the frame. An overflowing `origin + side` uses the frame limit, which is the intersection rather than a miss.

The scan loop at lines 149-170 counts writes and already-filled samples and returns `Frame` on a non-finite sample. It does not assign. The write loop at lines 172-182 runs only after that scan. It assigns `fill` only when `same_finite` is false. `same_finite` treats both zero bit patterns as equal and otherwise uses `total_cmp`, so it matches `==` for finite values and does not use an epsilon. A stored negative zero is therefore not replaced by a positive zero. `applied` is true whenever that function returns `Ok` after a non-empty span, including when every sample was already the fill.

A refusal in the scan cannot have written, because the write loop has not started. The write loop's own `Err` arms call the same `flat_index` and then `get_mut`. For a span that survived the scan, `row < height`, `column < width`, and `width * height` fits in `u32`, so the index is below the buffer length. Those `Err` arms are not a second write policy. The checked counter increments are the same kind of guard. The intersection count cannot exceed the product that already fits in `u32`.

There is no `as` cast, `unwrap`, `expect`, `unsafe`, or `wasm_bindgen` in `redact.rs`. Slice access uses `get` and `get_mut`. Clippy denies `float_cmp`, `unwrap_used`, `expect_used`, and `panic`, and `indexing_slicing` is a warning denied by `-D warnings`. The restored clippy run exited 0, including the integration test. The `Result` functions do not carry a bare `must_use` attribute. The eight-argument allow is the signature the plan names.

`lib.rs` adds `mod redact`, re-exports `Direction`, `Redaction`, `redact_rect`, and `ultrasound_direction`, and says the angle is the image plane, the length includes `z`, a miss sets `applied` false, and `distance` stays the F-070 function. `GeomError` is still the `measure` type. There is no new trait and no `Box<dyn>`. Tiers A, B, and C are not applicable. These functions do not open a device and do not render.

The tests build the expected length as `0 + 2 * 2 + 9 * 9` and call `sqrt` in the test crate. They do not copy a length out of a previous run. `same_bits` compares `to_bits`, so a negative zero written over by a positive zero would fail the dedicated test. That test passed.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. The plan adds `ultrasound_direction` and `redact_rect` to `docs/lld/geometry.md` and `docs/lld/public-api.md` at completion. Those files were not in this diff.

The angle is locked for one span, `(0, 0, 0)` to `(0, 2, 9)`. Dropping `z` from the length was executed and went red. A second edit that folds `z` into the positive `atan2` argument was not run. For this span `dx` is 0, so a positive y argument of any finite magnitude is still `PI / 2`. The shipped call does not do that. It passes `dx` and `dy` only.

The NaN test locks the first sample and the `NaN` at index 2. It does not compare the other two samples. The write loop was read. It was not given its own mutation. A zero frame, a product that overflows `u32`, a non-finite fill on a miss, and a `NaN` outside the intersection were read and not executed. A pure `z` span was not executed. The plan's zero-length rule includes `z`, so that span is not `Degenerate`, and the angle is `atan2(0, 0)`.

Only `ocelli-geom` `--test redact` was executed, plus the restored clippy run and the format check. The other geometry tests were not run. No wasm build and no oracle comparison.

The temporary length edit is gone. The four SHA-1 values match the pins. Do not attach this result to a later tree. The review file is untracked.
