# F-227 review, pass 15

**Reviewed**: `git diff 58e6db4 892fda6` on branch `work/f-227-xai`, HEAD `892fda6d841dfb35c0ac9645be2eaa5887102e44`, tracked tree clean. One file, `crates/ocelli-pixel/src/auto_window.rs`, 46 insertions and 0 deletions in one hunk at line 717. Library half only.
**Author / reviewer**: the change is by GLM-5.3 in opencode, per its report in the session scratchpad `glm-F-227-n2b.log`. Review by an independent Claude reviewer who authored none of the reviewed material.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 14 N2, no unit test fills the reserve exactly.** Plan item 7's stride rule, derived by hand. 2^20 = 1,048,576.
  - K = 7: budget 1,048,576 / 7 = 149,796 (7 * 149,796 = 1,048,572, remainder 4). A frame of 149,796 gives stride `149,796.div_ceil(149,796)` = 1 and pushed `149,796.div_ceil(1)` = 149,796, the budget. Seven frames push 1,048,572, which equals the reserve `sample_budget(7) * 7`.
  - K = 13: budget 1,048,576 / 13 = 80,659 (13 * 80,659 = 1,048,567, remainder 9). Stride 1, pushed 80,659, thirteen frames push 1,048,567, the reserve.
  - Every number in both doc comments matches this, and a Python recomputation printed the same values. The stored value is 100 with no padding, so no sample is excluded and the push count is the strided count.
- **The helper's comment claims** a reserve short by one trips the push's `debug_assert` in debug and the capacity comparison in release. Both were executed (below). The comment's "a reserve short by a few thousand values passes" the older tests is true: they leave 9,215 (K = 5) and 262,143 (K = 3) values unused.
- **Plausible wrong behaviour rejected**: a reserve one short (M2) and one over (M4). The over-reserve case is caught by the final `len == capacity` assertion, which the comment does not claim but which makes that line load-bearing rather than decorative.
- **Only the test module changed**: `mod tests` opens at line 672, the single hunk starts at 717 and inserts after the last existing test, `--numstat` reports 0 deletions, so no source line moved.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff --numstat 58e6db4 HEAD`, hunk count | worktree | now | `46 0 auto_window.rs`, 1 hunk at `@@ -717,4 +717,50 @@` |
| Python recomputation of budget, stride, pushed, reserve for K = 7 and 13 | the two fixture lengths | now | `7 149796 1048572 4 1 149796 1048572` and `13 80659 1048567 9 1 80659 1048567` |
| `cargo fmt --all --check` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-pixel` | same | now | 0. lib 18 (both new tests ok), auto_window 32, color 15, image_plane 4, lut_chain 16, modality 5, palette 7, stored_pixel 2, voi 6, doc 0 |
| `python3 scripts/prose_check.py` | worktree | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, clean over 1708 files |
| Mutant M2, reserve `sample_budget(frames) * frames - 1`, `cargo test --lib auto_window::tests` debug | `git archive HEAD` under `target/p15-scratch` | now | 101. Both new tests panic at `auto_window.rs:280`, `self.values.len() < self.values.capacity()`. Both older tests green |
| Mutant M2, `--release` | same | now | 101. Both new tests fail at line 741, `left: 2097142 right: 1048571` (K = 7) and `left: 2097132 right: 1048566` (K = 13), the doubled buffer. Older tests green |
| Mutant M4, reserve `+ 1`, debug and release | same | now | 101 both. Line 743, `len == capacity`: `1048572` vs `1048573` and `1048567` vs `1048568` |
| Unmutated scratch, `--release` | same, restored from `git show HEAD:` | now | 0, 4 passed |
| GLM's own runs | its log | prior receipt | agrees with every observation above |
| `git status --short`, `git diff --quiet`, `git diff --cached --quiet` | worktree after `rm -rf target/p15-scratch` | now | tracked tree unchanged, HEAD 892fda6, only the pre-existing untracked reviews and `tools/oracle/out` plus this report |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1** `auto_window.rs:728` to `744`, `assert_reserve_filled` repeats `assert_scratch_bounded` line for line except its last two assertions and the per-frame 4 MiB check. A change to the setup (modality, frame value, the `is_ok` pattern) has to be made twice. One helper with a `fills: bool` argument, or the exact-fill helper calling the bounded one's setup, would hold it once. Not blocking, test-only and both copies are currently identical where they overlap.
- **Carried**: pass 14 N1 (plan line 156, `center = m_max + 0.5`), and the earlier carried nitpicks pass 14 listed, unchanged because their text is unchanged in this diff. Pass 14 N2 is closed.

## Verified clean

- Both fixture lengths and both reserves, by hand and by recomputation, against item 7's rule, not against the comments.
- Both new tests go red under M2 in debug and in release, on the assertion each profile's comment names, while the two older tests stay green, which is exactly the gap N2 described. Under M4 both go red on the final equality.
- The release failure values are twice the mutated reserve, which confirms `debug_assert` was compiled out and the buffer reallocated.
- Allocation exactness: the tests assume `Vec::with_capacity` returns the requested capacity exactly. Std's `RawVec` records the requested capacity, and the unmutated and mutated runs both observed exact values.
- No production line changed. The `#[cfg(test)] capacity()` accessor and the `values` field access are test-module only.

## Limitations and handoff

- No oracle, GPU or wasm job, by instruction. `cargo test` was the default debug profile, plus release only for the scratch mutants.
- The viewer half is not in scope.
- With zero defects and zero smells this pass is clean for the library half's reviewed range `58e6db4..892fda6`. It does not on its own complete F-227's acceptance or the verify gate on the new tree.
