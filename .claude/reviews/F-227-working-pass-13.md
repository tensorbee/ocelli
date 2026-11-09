# F-227 review, pass 13

**Reviewed**: library half. Worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `8b2e979` ("F-227, the constant bullet states no width, amendments take precedence"), `git write-tree` `71cdb64a351d93766733cc6aee883ed893efb69d`, no unstaged tracked changes. The remediation is `git diff 767dd54 8b2e979`, two lines of `.claude/plans/F-227-design.md` (line 144 gains the precedence sentence, line 155 loses "`width = 1`"). Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `ddc98990`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `882b72a2`, both unchanged since pass 7, plan sha1 `e2ceb71e`. Untracked `tools/oracle/out` and the pass 1 to 12 reports are not part of the review.
**Author / reviewer**: the two edited lines are the sprint lead's, per the caller's brief. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 12 were written by the same model, so this pass is independent of the correction but not of the findings it answers.
**Result**: 3 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 12 D1, line 155.** The Constant bullet now reads "`center = low`, grey 128". Through the real `auto_window`, constants 7, 50, 1000, 2047, 2048, 3000, 65535, -1024, 1e6 and 1.6e7 each give centre equal to the value and display 127.5, which `display_index`'s `display + 0.5` rounding (`crates/ocelli-render/src/colour.rs:490`) draws as 128, at widths 1, 1, 1, 1, 2, 2, 33, 1, 513 and 8193. The bullet states no width now, so none of those can falsify it. **Closed.**
- **The new precedence sentence, line 144.** It governs "these items", items 1 to 21. It does not reach the "Scope, API and compatibility" section, so D2 below is not covered by it whatever an amendment says. Under the brief's rule (covered only when a reader of the amendment would see it changes that item) I applied it item by item below.
- **The sweep the brief asks for.** Every exact value, count, threshold or exactness word in items 1 to 21 and the library section at lines 187 to 260, each checked against the source, the tests or a probe through the real library. The full list is under Verified clean, and the three that fail are D1 to D3.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff 767dd54 8b2e979`, `git diff --stat 4458d5f HEAD`, `git write-tree`, `shasum` | HEAD `8b2e979` | now, start and end | two plan lines. Branch diff touches only the plan, `auto_window.rs`, `lib.rs` and the test file. Tree `71cdb64a` at both ends |
| probe crate `target/pass13-probe`, path dependencies on `ocelli-pixel` and `ocelli-core`, `CARGO_TARGET_DIR` the worktree's `target`, release, bins `main` and `alloc`, deleted with their build outputs after the run | real `series_sample`, `AutoWindowPool`, `auto_window`, `VoiTransform`, `ModalityTransform` | now | rows below |
| `main`: `series_sample` | M = 17, 267, 16, 1, 0 and every M from 1 to 4999 | now | `[0..=14, 16]`, the 16 indices item 6 lists, length 16, `[0]`, `EmptySeries`. Every M: length `min(M, 16)`, first 0, last `M - 1`, strictly increasing |
| `main`: counted values | slope 1, no zeros, (N, K) = (16384, 16), (65536, 16), (262144, 16), (4194304, 1), (1455104, 16), and 16 frames of 1455104 | now | 16384, 65536, 65536, 1048576, 63266, total 1012256, never above `1 << 20` |
| `main`: order | three frames, three different rescales, all six add orders | now | bitwise identical centre, width, low, high and counted |
| `main`: Background | single left-out sample `m`, slope 1, walked by `next_up` from 1.0 and `next_down` from -1.0 to 1e9 in magnitude | now | first `m` whose window is not `(m + 0.5, 1)`: 1.5000001 (`0x3FC00001`), centre 2.0000002, width 1.0000001. None on the negative side. 4194303.75 and 8388608 draw 0.0. A mixed left-out frame (stored 0, 5, 9, padding 5 to 9, slope 3.5, intercept 100.25) draws every sample 0.0 |
| `main`: LINEAR against LINEAR_EXACT | 100,000 integer pairs and 100,000 slope-times-integer pairs, fixed LCG seed. LINEAR at `c = (low + high + 1) / 2`, `w = high - low + 1`, LINEAR_EXACT at item 8's `c = low + w / 2`, `w = high - low`, both through `VoiTransform::apply`, and LINEAR again in f64 | now | integer: 100000 and 100000 map both edges exactly. Non-integer: LINEAR 72906, LINEAR_EXACT 78648. f64 LINEAR: 0 edge misses |
| `alloc`: a tracking `GlobalAlloc` recording the largest allocation or reallocation during `new`, K `add_frame` calls and `finish` | (K, N) = (16, 262144), (16, 1455104), (1, 16777216), (2, 1048576), (3, 1048576), (5, 1455104), (7, 1048576), (12, 262144) | now | 4194304 bytes for K = 16, 1, 2. **5592400 (5.333 MiB) for K = 3, 6710880 (6.400 MiB) for K = 5, 4793472 (4.571 MiB) for K = 7, 5592384 (5.333 MiB) for K = 12** |
| `grep` of `cpu.rs`, `ocelli-viewport/src/presentation.rs` and every `.rs` under `crates`, `examples`, `tools` for `PixelError::` | HEAD and base `4458d5f` | now | the only `PixelError` match in `cpu.rs` is `color_error`, lines 966 to 974, ending `_ => RasterError::Unavailable`. Every viewport match ends `Err(_)`. No match lists every variant |
| `cargo test -p ocelli-pixel` | tree `71cdb64a` | now | exit 0, every binary ok, `auto_window` 32 passed in 5.74 s |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |

## Defects

### D1, item 8 says LINEAR is exact only for integer modality values

**Where**: `.claude/plans/F-227-design.md:153`, item 8: "LINEAR would need `c = (low + high + 1) / 2` and `w = high - low + 1`, exact only for integer modality values, which a PET slope does not give."
**What**: with that `c` and `w`, C.11.2.1.2's `c' = c - 0.5 = (low + high) / 2` and `w' = w - 1 = high - low`, so the edges `c' - w'/2` and `c' + w'/2` are `low` and `high` and the ramp gives `ymin` at `low` and `ymax` at `high` for any real `low < high`, integer or not. The integer restriction is false in the real arithmetic the sentence's own derivation uses. In f32 the contrast the sentence draws fails too: on slope-times-integer pairs LINEAR maps both edges exactly in 72,906 of 100,000 and item 8's LINEAR_EXACT formula in 78,648, so neither is exact and the difference is not the one stated.
**Why it is wrong**: microscope section 3, a false claim in prose, and it is the rationale given for choosing LINEAR_EXACT. The plan's own transcription at lines 51 to 66 is enough to check it. No amendment touches LINEAR. Pass 9 N1 treated the LINEAR_EXACT derivation before it as describing the superseded formula, which is the right reading of that half, but the LINEAR half is not a description of the formula the pass 4 rulings replace.
**Evidence**: the f64 and f32 rows of the probe above, and the two-line algebra.
**Scope and regression**: prose only, present since the plan was approved (`1087630`). The decision to use LINEAR_EXACT is not affected, since LINEAR_EXACT needs no `+1` and no `-0.5` and that is reason enough. A later story choosing a window function from this item would believe LINEAR cannot carry a PET window, which is false. Smallest fix: delete "exact only for integer modality values, which a PET slope does not give", or the whole LINEAR sentence.

### D2, the library section says `cpu.rs:970` matches `PixelError` exhaustively, and it has a wildcard arm

**Where**: `.claude/plans/F-227-design.md:260`: "`PixelError` is not extended, because `crates/ocelli-render/src/cpu.rs:970` matches it exhaustively."
**What**: line 970 is inside `color_error` (lines 966 to 974), which lists four variants and ends `_ => RasterError::Unavailable`. Every other `PixelError` match in the workspace, the four in `ocelli-viewport/src/presentation.rs`, also ends in `Err(_)`. A new `PixelError` variant therefore compiles everywhere and lands in `Unavailable`. The repository's own term for the no-wildcard kind is "a total match with no wildcard arm" (`ocelli-render/src/voi.rs:181`, `caps.rs:145`), and this is not one. The same text was at line 970 on the branch base `4458d5f`.
**Why it is wrong**: microscope section 3, a false claim about the tree. Line 144's precedence sentence covers items 1 to 21 only, so this section is not covered by it, and no amendment mentions `PixelError`.
**Evidence**: the `grep` row above. No mutation was applied, because the brief forbids editing the tree, and the `_` arm at line 972 is decisive without one.
**Scope and regression**: prose only. The decision not to extend `PixelError` stands on its own, since a separate `AutoWindowError` keeps window refusals out of every `PixelError` consumer. The stated reason would mislead a later story into believing a new `PixelError` variant breaks `ocelli-render`'s build, when what it would actually do is route silently to `Unavailable`. Smallest fix: delete the "because" clause, or say the wildcard would swallow a new variant.

### D3, item 7 says the scratch is at most 4 MiB, and the pool allocates up to 6.4 MiB

**Where**: `.claude/plans/F-227-design.md:152`, item 7: "The scratch is at most 4 MiB of `f32`." Source `crates/ocelli-pixel/src/auto_window.rs:221`, `values: Vec::with_capacity(sample_budget(frames))`, and the `push` at line 275.
**What**: the pool reserves one frame's budget, `2^20 / K`, and grows by `push`. The values held never exceed `2^20`, but `Vec` grows by doubling, so for a `K` that is not a power of two the capacity passes `2^20`. With 5 frames of 1421 by 1024 the budget is 209,715, the pool holds 1,039,360 values, and the buffer is reallocated to 1,677,720 `f32`, 6,710,880 bytes. Twelve 512 by 512 CT frames, an ordinary short series, reach 5,592,384 bytes. Three 1024 by 1024 frames reach 5,592,400 and seven reach 4,793,472. K = 1, 2 and 16 stay at exactly 4 MiB. The old buffer is also live during a reallocation unless the allocator grows in place, so the transient peak may be higher still. That is unconfirmed.
**Why it is wrong**: microscope section 3, an exact bound in prose that the code exceeds by measurement. The same item's "so the total never exceeds the cap" is true of values, and line 381's "at most `AUTO_WINDOW_SAMPLE_CAP` values" is true, but "4 MiB" is a memory figure and the memory is larger. No amendment changes item 7.
**Evidence**: the `alloc` probe row, a tracking global allocator around `new`, `add_frame` and `finish` through the public API.
**Scope and regression**: present on HEAD since the pool was written. Every series of 3, 5, 6, 7 or 9 to 15 images whose frames fill most of their budget is affected, which the viewer half's `/series-window` will reach on short series. Nothing is red, and no test bounds the allocation. Smallest fix, either: reserve the whole pool once, `Vec::with_capacity(sample_budget(frames) * frames)`, which is at most `2^20` so no `push` grows it (K = 1 already reserves that much), with a test that runs a counting allocator over K = 5 and goes red on today's line 221. Or delete the 4 MiB sentence. Choosing between them is the lead's call.

## Smells

None.

## Nitpicks

- **N1** `.claude/plans/F-227-design.md:166`, item 15, "up to 16 decodes and one selection over at most 2^20 values". `finish` makes two `select_nth_unstable_by` calls, the second inside the tail the first leaves (source lines 401 and 407), and also runs each sampled value through its modality stage. The sentence names the main costs, not all of them, and AC17 measures the real latency, so nothing depends on the count. "One selection" read as "the selection step" is fair. Not blocking.
- **N2** line 156, the Background bullet, "`center = m_max + 0.5`, `width = 1`". From `m_max` = 1.5000001 (`0x3FC00001`) the f32 sum `m + 0.5` ties down to 2.0, the window fails its check and widens to 1.0000001 with centre 2.0000002. So the formula is the search's first try, not always the result, and that holds well below the 4194303.75 example. Covered under the brief's rule: pass 1 bullet 2 ("A background frame draws black by the same check"), pass 1 bullet 3 (which withdraws this bullet's own "Exact while `|m_max| < 2^22`" by naming it) and pass 3 bullet 1 ("the widening search for a constant or background window") each visibly change this bullet, and the reader is left with no magnitude at which width 1 is promised. This is the same shape pass 12 D1 removed from the line above. Deleting "`width = 1`" and the withdrawn "Exact while" sentence would stop the next pass reopening it.
- **N3** line 203, the API block's `#[derive(Clone, Copy, Debug, PartialEq)]` on `StoredRange`. The source derives `Clone, Copy, Debug` and implements `PartialEq` by hand as bitwise equality (source lines 63 to 80), because `Stored` has no `PartialEq` and the derive cannot compile, as pass 1 D4 noted. The trait set is the same and the impl's doc states the relation. Not blocking.
- **Carried**: pass 12 N1, pass 11 N1 (pass 10 N2), pass 9 N1 to N3 and the pass 7 nitpicks pass 9 carried, unchanged because their text and code are unchanged.

## Verified clean

Every exact value or exactness claim checked, by item. "Probe" means the real library this pass. "Source" means `auto_window.rs` at the lines given.

- **Line 144, the precedence sentence**: true as a rule. Its reach is items 1 to 21, as noted above.
- **Item 1**: `auto_window.rs` beside `lut.rs`. `ocelli-geom/Cargo.toml` depends on `ocelli-core` only. `stats.rs` has `Histogram { min, max, width: f64 .. }` over `f64` samples. True.
- **Item 2**: each frame through its own stage, source line 263 `modality.apply`. AC3's test `fixture_series_pool_uses_each_frame_rescale` exists and passes. True.
- **Item 3**: stored-space predicates before rescale, source lines 268 to 269. `PaddingRange { value: i32, limit: Option<i32> }` in stored space (`frame_open.rs:143`). Stored 0 at intercept -1024 is -1024 by arithmetic. AC4's `fixture_stored_zero_is_left_out_not_modality_zero` exists. The pseudo-PHI survey facts were not re-read (outside git).
- **Item 4**: only stored 0 and padding are tested. True.
- **Item 5**: ranks `n.div_ceil(100)` and `(99 * n).div_ceil(100)`, source lines 398 and 399, both at least 1 for `n >= 1`. No `as` cast in the source, `grep` finds none. Order independence: the probe's six orders are bitwise identical.
- **Item 6**: 16 frames, `K = min(M, 16)`, the formula, `[0]` at K = 1, first and last always, strictly increasing, `M = 0` refused, the M = 17 and M = 267 examples. Probe, every M to 4999.
- **Item 7**: `1 << 20`, `budget = cap / K`, `stride = N.div_ceil(budget)`, independent of content, budget 65,536 at 16 frames, 128 by 128 and 256 by 256 read exactly, 512 by 512 at stride 4 (65,536 of 262,144 counted), one frame keeps the whole cap (1,048,576 of 4,194,304), exclusions after the stride (source lines 262 to 276), total never above the cap (1,012,256 for 16 MG frames). All true. "At most 4 MiB" is D3.
- **Item 8**: the formula is marked superseded, true for every pair at least three units apart (pass 9). The LINEAR_EXACT derivation describes the superseded formula, pass 9 N1. "A `width` or `center` that is not finite is refused", source lines 425 to 427, test `a_non_finite_window_is_refused`. The LINEAR clause is D1.
- **Item 9**: Constant, `center = low` and grey 128, true at every value probed, condition `high - low` not greater than zero, basis `Constant` (pass 12 N1 on the label carried). "Keeps the pinned one-sample fixtures": the one-stored-value fixtures are below 2047, where width is still 1. Background: `m_max` is the left-out maximum (source lines 270 to 273), every left-out sample draws 0.0, basis `Background`, and its width and "Exact while" are covered by the amendments, N2. Two values: ranks 1 and 2, stored 3 and 9 give low 3, high 9, counted 2. Signed: no signedness branch. Non-finite: refused per sampled value before exclusion, source line 264. Refusals: as listed, `auto_window_pool_refusals`. The "item 13" pointer leads to the accumulator, not the refusal list, which is at line 258. That is a cross-reference slip, not an exactness claim.
- **Items 10 to 12, 14, 16, 18 and 21**: viewer behaviour, not written yet. They contain no library number. Item 12's "contributes nothing ... not replaced" matches `add_frame` leaving the pool unchanged on refusal and `finish` keeping the declared budgets.
- **Item 11**: one pool per series, `AutoWindowPool`. True of the library.
- **Item 13**: `new`, `add_frame`, `finish`, and `auto_window` as `new(1)` plus one `add_frame` plus `finish`, source lines 649 to 657. True.
- **Item 15**: "up to 16 decodes" equals `AUTO_WINDOW_SERIES_FRAMES`, and "at most 2^20 values" is true. "One selection" is N1.
- **Item 17**: Rust's `f32` `Display` is the shortest round-trip string. True of std. Not library code.
- **Item 19**: the branch diff touches no `open_frame`, `ocelli-wasm` or `packages/` file. "Nothing is invented" is still at `frame_open.rs:101`. True.
- **Item 20**: `PaddingRange` is `i32`, `Stored` is `f32`, exact for integers up to `2^24` in magnitude. True.
- **Library section, lines 189 to 260**: both constants. The `series_sample` signature. `AutoWindowBasis` and `AutoWindow` fields and derives. `transform` is the stated `VoiTransform::new` call (source lines 149 to 159). The pool's fields. `new` accepts 1 to 16. The `add_frame` stride doc. `auto_window` as a pool of one. The error enum, apart from `UnverifiableWindow`, which pass 3 names and so supersedes (pass 9). `FrameCount` is `new(0)` or `new(17)`. A failed `add_frame` leaves the pool as it was: `InvalidPadding`, `TooManyFrames` and `EmptyFrame` return before any mutation, and `NonFiniteModality` truncates to `mark` without committing `left_out_max` or `added`. `finish` with fewer frames keeps the declared budgets. `lib.rs` has `pub mod auto_window`, re-exports the public items and keeps `no_std` with `alloc`. `select_nth_unstable_by` and `total_cmp` are in `core`. All true. The `StoredRange` derive is N3 and the `cpu.rs:970` reason is D2.
- `cargo test -p ocelli-pixel`, `prose_check.py` and `source_provenance_check.py` pass.

## Limitations and handoff

- Not exercised: a GPU, tier B, the viewer half (not written), the oracle, real bytes. No source mutation was run. The code is unchanged since pass 7's mutation table, and the brief forbids editing the tree. The D3 regression test is proposed, not written.
- The allocation figures are from the macOS system allocator through `GlobalAlloc`. The `Vec` doubling that causes them is std's growth policy, not a platform property.
- Next owner: the lead decides D3 (code or prose), and corrects D1 and D2, which are prose only. A fresh independent pass 14 then reviews the change. If D3 is fixed in code, that pass needs the mutation and the allocation test, not a prose read. This is a bounded review of the library half, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` `71cdb64a` and the three sha1s were re-checked at the end. The probe lived under the worktree's ignored `target/` and was deleted with its build outputs.
