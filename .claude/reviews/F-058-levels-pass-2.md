# F-058 review, pass 2

**Reviewed**: working tree on `sprint/s19` at `0e35d6d3bbc77dd97a75da8d61f117aa41f8cd15` (designs only, not a committed range). Unstaged `crates/ocelli-volume/src/lib.rs`. Untracked `crates/ocelli-volume/src/levels.rs`, `crates/ocelli-volume/tests/levels.rs` and `.claude/reviews/F-058-levels-pass-1.md`. The pass 1 file was read for replay only. This pass 2 file was not in that pinned tree.
**Author / reviewer**: Author is the sprint implementation agent on `sprint/s19`. Reviewer is an independent microscope subagent in a separate context and did not write the code or the doc fix. This is not a human review and not a self-review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. Level 0 is `[4, 4, 4]` at spacing `[1, 2, 3]`, `U8`, filled with `0x11`. A child `[2, 2, 2]` is eight bytes. The hand ratio on each axis is `spacing_0 * 4 / 2`, so `[2, 4, 6]`. The old store length is `4 * 4 * 4 = 64`. The budget after one copy is `64 + 8 = 72`. Level 0's first byte stays `0x11`. The new index is 1 and `levels.len()` is 2.

`scaled` evaluates `step * numer / denom` with `numer` from level 0 and `denom` from the child. That is `1 * 4 / 2`, `2 * 4 / 2` and `3 * 4 / 2`. The offset is `data.len()` before `extend_bytes`. `Budgeted::bytes` is that same length. The level vector is not a second copy. Nothing writes the level 0 prefix. The permanent test compares spacing with `f64::to_bits`.

The inverted operator was executed, not only read. `*slot = step * denom / numer` made `append_level_keeps_level_zero_and_scales_spacing` fail. The assertion observed `[0.5, 1.0, 1.5]` against `[2.0, 4.0, 6.0]`. The other four level tests stayed green. The line was restored and the file hash matched the pre-mutation bytes.

Parenthesization was re-judged. Rust treats `step * numer / denom` as `(step * numer) / denom`. The plan's API sentence is that unparenthesized product. The plan's ratio paragraph writes `spacing_0 * (dims_0 / dims_level)`. On AC1 both associations are the same bits, `[2.0, 4.0, 6.0]`. They also agree on the non-uniform `[2, 1, 2]` ratio `[2, 8, 6]` and on `1 * 4 / 3`. An unchanged axis such as `0.7 * 3 / 3` differs by one `f64` ULP. The plan does not require that input or its bits. AC1 does not hit the split. This pass does not treat it as a defect.

`Volume::level_bytes` was re-judged. It is not in the plan's three-method list. It is still the right observation for AC1. `VoxelStore::slice` only addresses the level 0 prefix, so a zero-filled append of eight bytes would still satisfy offset 64, budget 72, spacing `[2, 4, 6]` and the first byte `0x11`. The span reader rejects that stand-in. It does not allocate, downsample or add a trait. This pass does not call it a defect, a smell or a nitpick.

AC2. `Volume::level` is `Vec::get`. It does not substitute `levels[0]`. `brick_grid_at` calls `level` before `BrickGrid::new`. On the AC1 volume, index 2 is `LevelAbsent { index: 2 }` for both callers. Index 0 matches `brick_grid` on dims, edge, grid and level. A one-entry volume's `level(1)` and `brick_grid_at(1)` were also `LevelAbsent` in a temporary test that was removed afterwards.

AC3. `[4, 4, 4]`, `[0, 2, 2]`, `[5, 2, 2]` and a 7-byte `[2, 2, 2]` each return `LevelShape` before `extend_bytes`. Length stays 1 and the store stays 64. A 9-byte `[2, 2, 2]` was the adjacent long buffer. It also returned `LevelShape`, left length 1 and left the first byte `0x11`. A legal append after that refusal still returned index 1.

AC4. `assemble_from` still pushes one `Level` at offset 0. `construct` does not call `append_level`. `construct_starts_with_one_level` and the existing construct and assembly assertions that `levels.len()` is 1 passed.

AC5. `brick_grid_at` calls `BrickGrid::new(level.dims, BrickWidth::from(self.voxel))` and then assigns `level`. `VoxelKind::I16` maps to `BrickWidth::SixteenBit`. The only edge match is in `BrickGrid::new`: eight-bit is 128, sixteen-bit is 64, float is 64. The permanent test observes edge 64 and level 1. The same constructor on a coarse `U8` grid observed edge 128 in the temporary test.

Also executed after a successful append, then removed with the temporary test. A second append of different bytes returned index 2 at offset 72, grew the budget from 72 to 80, and left both copies intact. `place` of a 16-byte slice rewrote level 0 only. `ready_bricks` stayed `[[0, 0, 0]]` and `brick_grid().level` stayed 0. A `[2, 1, 2]` child scaled to `[2, 8, 6]`. A `[4, 4, 2]` child scaled to `[1, 2, 6]`, so an equal axis is kept and one shorter axis is enough.

Rejected by reading as well as by those runs: a downsample or average, opacity correction, a tier branch, `wgpu`, `wasm-bindgen` and `unsafe`. No other crate calls `append_level`, `brick_grid_at` or `level_bytes`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test levels` | pinned unstaged `lib.rs`, untracked `levels.rs` and `tests/levels.rs` | executed now | exit 0. 5 passed, 0 failed |
| `bin/ocelli.sh test ocelli-volume` | same tree | executed now | exit 0. unit 1, assembly 14, brick_width 2, construct 10, levels 5, ready_bricks 4, doc-tests 0 |
| `bin/ocelli.sh clippy ocelli-volume` | same tree | executed now | exit 0. no warning printed |
| Hand ratio in Python for AC1, plus both associations | spacing `[1, 2, 3]`, dims `[4, 4, 4]` to `[2, 2, 2]` | executed now | `[2.0, 4.0, 6.0]` either way. Offset 64. Budget 72. `0.7 * 3 / 3` differs by one ULP |
| `rg` for `unsafe`, `wgpu`, `wasm_bindgen` and `wasm-bindgen` | `lib.rs`, `levels.rs`, `tests/levels.rs` | executed now | no matches in those three files |
| Inverted `*slot = step * denom / numer`, then restore | temporary edit of untracked `levels.rs` only | executed now | exit 101. AC1 observed `[0.5, 1.0, 1.5]` against `[2.0, 4.0, 6.0]`. Four other tests passed. SHA-256 of `levels.rs` matched `56a278af9829e83dd3404aba5013d4258d2a466390fa414d56a6b3c2245bc37d` after restore |
| Temporary `review_adjacent_refusal_then_second_append`, then restore | added to `tests/levels.rs` and removed | executed now | exit 0. 1 passed. Test file SHA-256 matched `93fb9cc27a185b563290eed314b48163d9ba579910be397e99a744cab6d60eb4`. The test is not in the tree |
| `git status --short` and `git rev-parse HEAD` after restore | same three product paths as the pin | executed now | HEAD still `0e35d6d3bbc77dd97a75da8d61f117aa41f8cd15`. Status matched the pin |

`bin/ocelli.sh gate unsafe` was not used. Both new Rust paths are untracked, so `git ls-files` would not show them.

## Defects

None.

Pass 1 D1 is closed. `crates/ocelli-volume/src/lib.rs:395` now says level 0 is full resolution and later caller-supplied levels are appended. The words "exactly one entry in this story" are absent from the crate. `append_level([2, 2, 2], &[1, 2, 3, 4, 5, 6, 7, 8])` on the AC1 volume still returns index 1 and leaves `levels.len() == 2`. Level 0 stays `[4, 4, 4]` at offset 0 with first byte `0x11`. A constructed volume still has length 1, so the sentence does not claim that every value already holds a second entry. The append behaviour matches the pass 1 account. The false length claim does not.

## Smells

None.

## Nitpicks

None.

## Verified clean

The diff in `lib.rs` is `mod levels`, `VolumeError::LevelAbsent`, `VolumeError::LevelShape`, `VoxelStore::as_bytes`, `VoxelStore::extend_bytes`, and the rewritten `levels` doc. `assemble_from`, `construct`, `place`, `brick_grid`, `ready_bricks` and `Budgeted::bytes` are not otherwise edited. `bytes` remains `data.len()`.

`append_level` copies with `extend_from_slice`. It does not average, skip or generate samples. There is no opacity correction and no renderer call. Level 0 stays at offset 0. The child offset is the store length before the copy. `VoxelStore` dims stay the level 0 dims, so `slice` and `place` stay inside that prefix. There is still no per-slice `Vec`.

Every `LevelShape` return in `append_level` is before `extend_bytes`. That includes a bad shape, a byte length that does not match, a `scaled` miss and a level count that does not fit in `u32`. A missing level 0 returns `LevelAbsent` on the first line and does not grow the store. `coarser` rejects a zero axis, an axis above level 0, and a grid that is not strictly smaller on at least one axis. Equal axes are allowed when another axis shrinks. The temporary `[4, 4, 2]` append observed that case.

Spacing is baked from the level 0 entry at append time. A second append pushes another entry and another copy at the new length. It does not deduplicate. The temporary run observed index 2, offset 72 and budget 80.

`brick_grid` still builds from `Volume.dims` and leaves `BrickGrid::level` at 0. `brick_grid_at` does not keep a second edge table. Sixteen-bit signed and unsigned share `BrickWidth::SixteenBit`. Float uses the same edge 64 in `BrickGrid::new`. The permanent test uses `I16`.

No `as` cast was added. `f64::from` of a `u32` is exact. No new trait or generic was added. The crate stays `no_std` outside tests. Tiers A, B and C stay not applicable. The bytes are host data. Append is not on a render loop.

`extend_bytes` itself does not re-check the length. The plan's failure path is the early return, and that is what the code does. The only caller is `append_level`.

## Limitations and handoff

The mutation and the adjacent test were reverted. Product source matches the pinned bytes. Those two runs are evidence about this tree, not extra tests left behind.

`docs/lld/volumes.md` line 21 still says this story holds exactly one level, at offset 0. The approved plan lists that file under completion and defers the sentence to `/complete-feature`. It does not require the LLD edit before this review. This pass does not count that sentence as a defect. It was not edited.

A volume with an empty `levels` vector was not constructed. The first line of `append_level` is `level(0)`, so that path returns `LevelAbsent` before a copy, but this pass did not call it. `level_bytes` returning `LevelShape` for a span that does not fit the store was not executed. A well-formed append does not hit it. `U16` and `F32` were not appended. They use the same byte-length and `BrickGrid::new` paths as `I16` and `U8`.

No wasm build, oracle, corpus, GPU, rustfmt or `gate --sprint` was run. No adapter was opened. Tier evidence is the absence of a tier branch, not three renderers. This pass does not close the story. The LLD sentence remains for completion. There is no code fix to hand back. Do not treat the pass 1 receipt as this review.
