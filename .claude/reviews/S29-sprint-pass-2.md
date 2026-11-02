# S29 review, pass 2

**Reviewed**: HEAD `f7c842d96fb701ec08c1a78c3c2bd79d2e40e0d5` on `sprint/s29`. HEAD tree is `e61b6ec38144100ce5d961596413fd5801ab66d9`. `git write-tree` is that same tree. The repair is unstaged. Pass 1 at `.claude/reviews/S29-sprint-pass-1.md` was already untracked and was not edited. This pass is the D1 repair only. It is not sprint verification.
**Author / reviewer**: This reviewer did not write the repair, the plans, or pass 1. Pass 1 was not resumed. The story trailers were not re-read.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 D1 is closed. The HLD still does not define histogram bins. The local rule is the plan sentence at `.claude/plans/F-075-design.md` line 60. The maximum belongs to the last bin, including when adding the width steps past it. That is the same placement rule. It is not a new formula.

`histogram` sends a sample to the last index only when `sample.total_cmp(&max).is_eq()` at `crates/ocelli-geom/src/stats.rs` lines 217 to 218. Every other sample keeps the edge walk at lines 220 to 226. A sample equal to the maximum is the only one forced into the last bin. A lesser sample can still finish in the last bin when the walk reaches it.

Samples 0, 0.5 and 1 in two bins have width 0.5. The value 0.5 is not equal to the maximum. The walk starts at edge 0.5, the `is_ge` test steps once, and the index stops at 1 because that index is already the last bin. The value 0 stays in bin 0. The value 1 is the only sample that takes the equality branch, and the walk would also stop at index 1. The higher bin for 0.5 is the walk. The restored test records counts 1 and 2.

Minimum `10000000` and maximum `10000000 + 1e-8` in 8 bins are not an equal-value histogram. The width is not 0. The maximum compares equal to `max` and is written in the last bin. The minimum does not compare equal. The walk leaves it in the first bin. The restored test expects the first count and the last count to be 1, and the sum to be 2.

Removing the equality branch makes every sample use the walk. `the_maximum_of_a_narrow_range_is_the_last_bin` then fails. The panic is at `crates/ocelli-geom/tests/stats.rs:102`, so the first count had already compared equal to 1 and the last count was 0. The same additions in Python, with `>=` on these positive finite values, stop the maximum at index 5. That Python walk is not a Rust call. `>=` matches `is_ge` for these values, which is the same reason pass 1 gave.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `HEAD^{tree}`, and `git write-tree` | `sprint/s29` before the mutation | Executed now | HEAD `f7c842d96fb701ec08c1a78c3c2bd79d2e40e0d5`. Both trees `e61b6ec38144100ce5d961596413fd5801ab66d9` |
| `git diff --name-only` and `git diff --stat` | Unstaged repair against that HEAD | Executed now | Six paths. `stats.rs` is 20 lines. No `measure.rs`, no `annotation.ts`, no `DEVIATIONS.md`, and no wasm budget file |
| `shasum` and `shasum -a 256` before the mutation | The six repair paths. `stats.rs` bytes copied to `/tmp` first | Executed now | All six SHA-1 values match the supplied pins. `stats.rs` SHA-256 is `733c21702b1b5638509eefea55e552710a342888e297d947cd2af2358c3e29bd` |
| `bin/ocelli.sh test ocelli-geom --test stats the_maximum_of_a_narrow_range_is_the_last_bin` | Equality branch removed. Every sample used the edge walk. One cargo process | Executed now | Exit 101. The process status is 101. Compiled `ocelli-geom v0.1.0`. 0 passed, 1 failed, 10 filtered out. Panic at `tests/stats.rs:102`. Left 0. Right 1 |
| Python float64 model of that walk | Same adds and `>=` as the mutated loop. Not a Rust call | Executed now | The maximum stops at index 5. The minimum stays at index 0. Width is not 0. With the equality branch restored in the model, the maximum is index 7 |
| Python model of samples 0, 0.5 and 1 | Two bins, width 0.5. Not a Rust call | Executed now | 0.5 is not equal to the maximum and the walk index is 1. Only 1.0 compares equal to the maximum. Its walk index is also 1 |
| `shasum` after `cp` of the saved `stats.rs` | Restored bytes, before the green run | Executed now | The same six SHA-1 pins. The same `stats.rs` SHA-256. The equality branch is back at line 217 |
| `bin/ocelli.sh test ocelli-geom --test stats` | Restored tree. One later cargo process | Executed now | Exit 0. Recompiled `ocelli-geom v0.1.0`. 11 passed, 0 failed, 0 filtered out. `an_interior_edge_belongs_to_the_higher_bin` passed. `the_maximum_of_a_narrow_range_is_the_last_bin` passed |
| `shasum` after the green run | Same six paths | Executed now | The six SHA-1 pins still match. `stats.rs` SHA-256 is unchanged. HEAD is still `f7c842d96fb701ec08c1a78c3c2bd79d2e40e0d5` |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14). No pipe was used for either cargo status. wasm-pack was not run. The gate was not run. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None. Pass 1 D1 is closed.

## Smells

None.

## Nitpicks

None.

## Verified clean

The forced index is `last` only inside `sample.total_cmp(&max).is_eq()`. The `else` branch is the previous edge walk, including `index < last` and `is_ge`. Equal samples still return at the exact-zero width path before that loop. `equal_samples_fill_the_first_bin` passed on the restored tree.

The narrow fixture is outside the two-bin case. Its width check is `total_cmp` not equal to 0, and both end counts are 1 with a sum of 2. The red run shows those end counts are not produced by the walk alone. The first assertion passed while the last assertion failed, so the walk kept one sample in the first bin and put none in the last bin.

The interior edge is unchanged. 0.5 reaches bin 1 without comparing equal to the maximum. The green interior test is counts 1 and 2, minimum 0, maximum 1, and width 0.5.

The plan, the function comment at lines 160 to 162, and `docs/lld/geometry.md` lines 100 to 102 all carry the drift clause. `docs/lld/public-api.md` and `CHANGELOG.md` say the maximum belongs in the last bin and do not repeat the drift clause. Those shorter sentences are the same rule.

The SUV line is still `activity * inputs.patient_weight_kg * 1000.0 / decayed` at `stats.rs` line 274. The diff does not change it. `measure.rs` and `annotation.ts` are not in the diff. The repair adds no `unsafe` and no `wasm_bindgen`.

## Limitations and handoff

This pass reviews the uncommitted D1 repair. It does not re-open F-077, F-078, or F-079. It is not a sprint-profile record and it does not close M8. Nothing was staged, committed, pushed, or published.

The index still matches HEAD tree `e61b6ec38144100ce5d961596413fd5801ab66d9`. A receipt on that tree would miss the six unstaged paths. This file is a new untracked review and is not one of those six paths.

Two cargo processes ran, one after the other. The full `ocelli-geom` suite was not run. Clippy was not run. A sample strictly between the two narrow endpoints was not executed in Rust. Two samples equal to the maximum, a one-bin buffer, and a negative range were not executed in Rust. The two signs of zero were read and not run. They do not reach the new branch when the width path treats their difference as an exact zero.

The next owner can stage the six repair paths and record them against the new tree. Do not record the HEAD tree as if it contained the repair. Do not publish from this sprint.
