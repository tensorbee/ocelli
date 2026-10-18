# F-049 review, pass 2

**Reviewed**: working tree on sprint/s15 against 1e54993, after the opening-sentence correction
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The pass 1 nitpick was the opening claim that both `frame_shift` and `apply_placement` store spacing and the index shift. `docs/lld/viewport.md` lines 9 to 11 now say that `apply_placement` stores the displayed spacing and the measured index shift on that view, and that it does not draw. `frame_shift` is not named there.

That sentence matches `crates/ocelli-viewport/src/placement.rs`. `apply_placement` at lines 54 to 58 takes `&mut View`, calls `frame_shift`, then `set_spacing(column_mm, row_mm)` and `set_index_shift(column, row)`. It does not draw. `frame_shift` at line 27 returns `(f64, f64)` and does not take a `View`. The placement section at `docs/lld/viewport.md:125` still names the same two setters. A search of `docs/lld/viewport.md` found no prose semicolon, no em dash, and no en dash.

`placement.rs` is still the restored pass 1 function. `ImagePlane::new` receives `displayed.spacing()` at line 33. The column guard is `spacing.column_mm() > 0.0` at line 44. The row guard is `spacing.row_mm() > 0.0` at line 49. `apply_placement` still calls `set_index_shift` at line 58. `tests/placement.rs` is the same 8 tests, still using `to_bits` and the same expected values, including canvas `(1.5, 3)` and fit offset `(3, 0)`. Neither Rust file was edited to hide the nitpick, so the three mutations were not repeated.

`git status --short` shows tracked edits only in `crates/ocelli-viewport/src/lib.rs`, `docs/lld/README.md`, and `docs/lld/viewport.md`. The untracked product files are `placement.rs` and `tests/placement.rs`. `lib.rs` and the README viewport row are the same text read in pass 1. The viewport diff against `1e54993` is the F-049 placement section plus this opening correction. `bin/ocelli.sh test ocelli-viewport --test placement` exited 0. All 8 tests passed. None were filtered out.
