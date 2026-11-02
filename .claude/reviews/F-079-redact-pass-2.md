# F-079 review, pass 2

**Reviewed**: HEAD `7e300f3ada45dbeb26b364e4162f486be7521f05` on `sprint/s29`. That commit is F-078 and was not amended. The F-079 product is uncommitted. This pass reviews the tree after `cargo fmt -p ocelli-geom`. `crates/ocelli-geom/src/redact.rs` and `.claude/plans/F-079-design.md` are the same bytes pass 1 pinned. `crates/ocelli-geom/src/lib.rs` and `crates/ocelli-geom/tests/redact.rs` are the formatted files. After the length mutation was restored, and again after clippy, `shasum` SHA-1 values were `42229e255f890ff0a09f7a10b659b9cd894db88f`, `65e3ef26b30a2517c161ddada6b5ee2b6e017b59`, `02846e5f7f8636552418137f4e17cd347395cb87`, and `871076d989541f1d72a98836a60dc8a995ee2395`. Those match the pins for this tree. `redact.rs` SHA-256 is `75a8d4ab4a4fc656772cbe3030c627edad5304154c007474230ce44089fbf58f`. `git diff` for `measure.rs`, `stats.rs`, and `packages/core/src/annotation.ts` is empty.
**Author / reviewer**: The F-079 change has no commit, so this pass does not name an author from a commit or a trailer. The reviewer did not write the change or the rustfmt edit and is not the pass 1 reviewer.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is unchanged. The HLD does not define these tools. The local plan is the contract. Length includes `z`. The angle does not. A missed rectangle sets `applied` false. A non-finite sample in the intersection writes nothing. This pass does not replace that contract and it does not ask for a DICOM formula.

The check is the rustfmt diff plus one replay of the length mutation on the formatted test.

`crates/ocelli-geom/src/lib.rs` now has `pub use redact` after `pub use measure` and before `pub use stats`. `mod redact` was already between `mod measure` and `mod stats`. The re-export is still `Direction`, `Redaction`, `redact_rect`, and `ultrasound_direction`. The redact tests import those names from `ocelli_geom` and passed on this tree, so the public names still resolve. `cargo fmt -p ocelli-geom -- --check` exited 0.

In `positive_y_is_half_pi`, the radians assertion is wrapped at `crates/ocelli-geom/tests/redact.rs` lines 36 to 41. It is still `found.radians.total_cmp` against `std::f64::consts::FRAC_PI_2`. The length assertion is line 42. It still compares `found.length` with `squared.sqrt()`. `squared` is still `0.0 + 2.0 * 2.0 + 9.0 * 9.0`. Line 43 still requires that length to differ from `2.0`. The wrap does not change either predicate. `redact.rs` was not part of the format. Its length call is still `DVec3` and its angle call is still `DVec2::to_angle`.

AC1. The formatted suite passed `positive_y_is_half_pi` before the mutation. The mutation replaced the line 96 call `DVec3::new(dx, dy, dz).length()` with `DVec2::new(dx, dy).length()`. `bin/ocelli.sh test ocelli-geom --test redact positive_y_is_half_pi` then failed at `crates/ocelli-geom/tests/redact.rs:42:5`. That is the comparison with `squared.sqrt()`. Exit 101. The radians assertion above that line did not fail. The build warned that `DVec3` was unused, which is this edit. The `DVec3` call was restored. The four SHA-1 pins matched again. The green run was on those same bytes.

AC2 through AC6 were not given a new mutation. The same pre-mutation run passed `a_zero_direction_is_degenerate`, `the_left_column_is_redacted`, `a_second_pass_says_the_pixels_were_unchanged`, `a_missed_rectangle_is_not_applied`, and `a_non_finite_sample_blocks_the_write`. Format did not edit `redact.rs`.

Pass 1 recorded one nitpick. `cargo fmt -p ocelli-geom -- --check` exited 1 on that tree. This pass runs the same check and it exits 0. That nitpick is closed. It is not carried and it is not a new finding.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom --test redact` | Formatted tree, before the mutation. The four SHA-1 pins matched | Executed now | Exit 0. 11 passed and 0 failed |
| `cargo fmt -p ocelli-geom -- --check` | Same tree. The check does not write | Executed now | Exit 0 |
| `bin/ocelli.sh test ocelli-geom --test redact positive_y_is_half_pi` after `DVec2` length | Mutated `redact.rs` line 96 only | Executed now | Exit 101. Failed at `tests/redact.rs:42`, the `squared.sqrt()` comparison. 10 tests filtered out. The build warned that `DVec3` was unused |
| `bin/ocelli.sh clippy ocelli-geom` | Restored tree. This is `cargo clippy -p ocelli-geom --all-targets -- -D warnings` | Executed now | Exit 0 |
| `shasum` and `shasum -a 256` | After the restore, and again after clippy | Executed now | All four SHA-1 values match the pins. `redact.rs` SHA-256 matches the pin |
| `git diff` for `measure.rs`, `stats.rs`, and `annotation.ts` | HEAD `7e300f3ada45dbeb26b364e4162f486be7521f05` | Executed now | Empty |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14). One cargo process ran at a time. The length mutation was restored. wasm-pack was not run. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The format result in `lib.rs` is the `pub use` order above. `mod redact` stays in alphabetical order with the other modules. The crate comment still says the angle is the image plane, the length includes `z`, a missed rectangle sets `applied` false, and `distance` stays the F-070 function. `GeomError` is still the `measure` type.

The formatted test still builds the expected length as `0 + 2 * 2 + 9 * 9` and calls `sqrt` in the test crate. It does not copy a length out of a previous run. The mutation that drops `z` fails that comparison and does not fail the angle comparison. Restored line 96 is `DVec3::new(dx, dy, dz).length()`. Line 103 is `DVec2::new(dx, dy).to_angle()`. There is no `DVec2` length call in the restored file.

`redact.rs` SHA-1 and SHA-256 match the pass 1 pins, so the function bodies pass 1 read are the bodies on this tree. The plan SHA matches `871076d989541f1d72a98836a60dc8a995ee2395`. The plan still says the length of `(0, 2, 9)` is the square root of `4 + 81`.

Clippy on the restored tree exited 0 with warnings denied. The unused-`DVec3` warning appeared only while the mutation was present.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. It covers the rustfmt diff and a confirmation that the length mutation still goes red on the formatted test.

The other wrong implementation named by AC1, folding `z` into the angle, was not executed. For this span `dx` is 0, so a positive `y` of any finite size is still `PI / 2`. The shipped call does not do that. It passes `dx` and `dy` only. Pass 1 recorded the same limit.

No redaction path was mutated on this pass. A zero frame, an overflowing product, a non-finite fill, and a `NaN` outside the intersection were not re-executed. The full redact suite did pass on the formatted bytes before the mutation.

Only `ocelli-geom` `--test redact` was executed, plus the format check and clippy. The other geometry tests were not run. No wasm build and no oracle comparison. The plan's LLD edits are not in this diff.

The temporary length edit is gone. The four SHA-1 values match the pins. Do not attach this result to a later tree. The review file is untracked.
