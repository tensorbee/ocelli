# F-058 review, pass 1

**Reviewed**: working tree on `sprint/s19` at `0e35d6d3bbc77dd97a75da8d61f117aa41f8cd15` (designs only, not a committed range). Unstaged `crates/ocelli-volume/src/lib.rs`. Untracked `crates/ocelli-volume/src/levels.rs` and `crates/ocelli-volume/tests/levels.rs`.
**Author / reviewer**: Author is the sprint implementation agent on `sprint/s19`. Reviewer is an independent microscope subagent in a separate context and did not write the code. This is not a human review and not a self-review.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. Level 0 is `[4, 4, 4]` at spacing `[1, 2, 3]`, `U8`, filled with `0x11`. A child `[2, 2, 2]` is eight bytes. The hand ratio on each axis is `spacing_0 * 4 / 2`, so `[2, 4, 6]`. The old store length is `4 * 4 * 4 = 64`. The budget after one copy is `64 + 8 = 72`. Level 0's first byte stays `0x11`.

`scaled` evaluates `step * numer / denom` with `numer` taken from level 0's dims and `denom` from the child. That is `1 * 4 / 2`, `2 * 4 / 2` and `3 * 4 / 2`. The offset is `data.len()` before `extend_bytes`. `Budgeted::bytes` is that same length, so the level vector is not a second copy. Nothing writes the level 0 prefix. The inverted operator `step * denom / numer` is `[0.5, 1.0, 1.5]`. The source is not that expression. `append_level_keeps_level_zero_and_scales_spacing` compares spacing with `f64::to_bits`, so the inverted triple cannot pass. This pass did not edit the source to replay that mutation.

An independent non-uniform reading of the same loop, not a separate test, is child `[2, 1, 2]`: spacing `[2, 8, 6]` and `2 * 1 * 2 * 2 = 8` bytes. AC5 uses those dims and that byte count for `I16` and does not assert the spacing. The loop has no per-axis special case that would make only the uniform `2x` ratio correct.

AC2. `Volume::level` is `Vec::get` on the index. It does not substitute `levels[0]`. `brick_grid_at` calls `level` before `BrickGrid::new`, so a missing index builds no grid. On the AC1 volume, index 2 is `LevelAbsent { index: 2 }` for both callers. Index 0 still matches `brick_grid` on dims, edge, grid and level.

AC3. `[4, 4, 4]`, `[0, 2, 2]`, `[5, 2, 2]` and a 7-byte `[2, 2, 2]` all return `LevelShape` before `extend_bytes`. After all four calls the test still sees one level and 64 bytes. A zero axis is rejected in `coarser` even though a zero dimension would make `byte_len` 0 and match the empty buffer. An axis of 5 is rejected even though `5 * 2 * 2` equals the 20-byte buffer, so the refusal is the shape and not a short read.

AC4. `assemble_from` still pushes one `Level` at offset 0. `construct` places slices and does not call `append_level`. The new construct test and the existing construct tests agree.

AC5. `brick_grid_at` calls `BrickGrid::new(level.dims, BrickWidth::from(self.voxel))` and then assigns the public `level` field. `VoxelKind::I16` maps to `BrickWidth::SixteenBit` in `lib.rs`. The only edge match is `BrickGrid::new`: sixteen-bit is 64, eight-bit is 128, float is 64. `levels.rs` does not match on an edge. The test observes edge 64 and level 1.

Also rejected by reading, not by a second test: a downsample or average, a tier branch, `wgpu`, `wasm-bindgen`, and `unsafe`. `ready_bricks` still calls `brick_grid`, which is level 0. `place` still writes one level-0 slice. `SlicePresence` is not updated by append.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test levels` | unstaged `lib.rs`, untracked `levels.rs` and `tests/levels.rs` | executed now | exit 0. 5 passed, 0 failed |
| `bin/ocelli.sh test ocelli-volume` | same tree | executed now | exit 0. unit 1, assembly 14, brick_width 2, construct 10, levels 5, ready_bricks 4, doc-tests 0 |
| `bin/ocelli.sh clippy ocelli-volume` | same tree | executed now | exit 0. no warning printed |
| Hand ratio in Python for AC1 | spacing `[1, 2, 3]`, dims `[4, 4, 4]` to `[2, 2, 2]` | executed now | `[2.0, 4.0, 6.0]`, offset 64, budget 72 |
| Read of `levels.rs` for `unsafe`, `wgpu`, `wasm_bindgen` | untracked `levels.rs`, `tests/levels.rs`, and `lib.rs` | executed now | each count is 0 |
| Author stand-in that returned `LevelShape` for a legal `[2, 2, 2]` | not in this tree | author report, not re-run | reported exit 101, three tests failed |
| Author spacing mutation `step * denom / numer` | not in this tree | author report, not re-run | reported AC1 observed `[0.5, 1.0, 1.5]` against `[2, 4, 6]`, then reverted |

`bin/ocelli.sh gate unsafe` was not used as evidence for the new files. `scripts/unsafe_allowlist_check.py` lists `git ls-files`. Both new paths are untracked, so a green gate would not have read them.

## Defects

### D1, The level-axis field doc still says there is one entry

**Where**: `crates/ocelli-volume/src/lib.rs:395`
**What**: `Volume.levels` is documented as "exactly one entry in this story." `append_level([2, 2, 2], &[1, 2, 3, 4, 5, 6, 7, 8])` on the AC1 volume returns index 1 and leaves `levels.len() == 2`. The behaviour is what AC1 requires. The sentence on the field is not.
**Why it is wrong**: HLD section 19 puts a multiscale `levels` vector on `Volume`, with level 0 at full resolution, so later entries do not require a new type. The approved plan's AC1 requires the length to become 2. A doc comment is a claim about that field. This claim is false on the working tree, and it is the sentence that would teach a later reader to treat `len() == 1` as an invariant. The plan tells F-057, F-052 and F-059 not to panic on that assumption.
**Evidence**: the line is unchanged in `git diff` and still reads as one entry. `append_level_keeps_level_zero_and_scales_spacing` asserts length 2 and passed in the levels run above. No mutation was required. The passing test and the comment cannot both be true.
**Scope and regression**: present on this working tree, not only on historical HEAD. Consumers are readers of `Volume.levels`, including the later S19 stories that share this crate. The append behaviour itself does not need a code change. The smallest fix is to rewrite that one sentence so level 0 stays full resolution and later entries are appended. Uncertainty is limited to the old qualifier "in this story". It no longer names a story whose axis cannot grow.

## Smells

None.

## Nitpicks

None.

## Verified clean

The behaviour matches the plan. The diff in `lib.rs` is `mod levels`, `VolumeError::LevelAbsent` and `VolumeError::LevelShape`, and `VoxelStore::as_bytes` plus `extend_bytes`. `assemble_from`, `construct`, `place`, `brick_grid`, `ready_bricks` and `Budgeted::bytes` are not otherwise edited. `bytes` remains `data.len()`, so an append of 8 bytes grows the budget by 8.

`append_level` copies the caller buffer with `extend_from_slice` and does not average, skip or generate samples. There is no opacity correction and no renderer call. `ocelli-render` does not read `VoxelStore`.

Level 0 stays at offset 0. The child offset is the store length before the copy. Slice length still uses `VoxelStore.dims`, which append does not change, so `place` and `slice` stay inside the level 0 prefix. The extra bytes sit at the end of the same `Vec<u8>`. There is still no per-slice `Vec`.

Spacing is level 0's spacing times level 0's dims over the child dims, baked at append time from the level 0 entry rather than from a later level. A second append would push another entry and another copy at the new length. That path is unread by a test and is obvious in the function. It does not deduplicate.

Every `LevelShape` return, including a `u32` index that does not fit, is before `extend_bytes`. A missing level 0 returns `LevelAbsent` on that same first line and does not grow the store. `coarser` rejects a zero axis, an axis above level 0, and a grid that is not strictly smaller on at least one axis. Equal axes are allowed when another axis shrinks.

`brick_grid_at(0)` and `brick_grid()` agree when `volume.dims` and `levels[0].dims` agree, which is how `assemble_from` and the AC1 fixture build them. The coarse grid's edge is whatever `BrickGrid::new` selects for the volume's voxel kind. Sixteen-bit signed and unsigned share `BrickWidth::SixteenBit`. The test uses `I16`. There is no second edge table in this crate's new module.

No `as` cast was added. `f64::from` of a `u32` is exact. AC1's `[2, 4, 6]` is exact in `f64` for either parenthesization of the ratio. The plan's API sentence is the unparenthesized product. The parenthesized form in the plan's ratio paragraph can differ by one `f64` ULP on some non-dyadic inputs, including an unchanged axis such as `0.7 * 3 / 3`. AC1 does not hit that split. This pass does not treat it as a defect.

There is no `wgpu`, no `wasm-bindgen` and no `unsafe` in the three files. No tier path was added. Tiers A, B and C stay not applicable. The bytes are host data. Append is not on a render loop.

`construct_starts_with_one_level` and the existing `construct.rs` assertion that `levels.len()` is 1 both passed. Assembly still starts at one level. The levels module docs say it does not downsample, and the only store write is the checked copy.

## Limitations and handoff

This pass did not re-apply the author's LevelShape stand-in or the inverted spacing mutation. Product source was not edited. The operators were checked by reading, and the green AC1 test locks the bit pattern `[2, 4, 6]`.

`level(1)` on a volume that still has one entry is not its own test. It is the same `get` as index 2. A buffer longer than the child is refused by `samples.len() != expected` and is not one of AC3's four calls. AC3 does not re-read the level 0 byte after refusal. Those four paths return before any copy, so they cannot change that byte. The success path does assert `0x11`.

The eight-bit coarse edge is not asserted. It is the `EightBit` arm of the same `BrickGrid::new`, which is 128. `U16` is not given its own append test. It uses the same `SixteenBit` width as `I16`.

`docs/lld/volumes.md` still says this story holds exactly one level, at offset 0. The plan defers that sentence to completion. This pass does not count the missing LLD edit as a defect or a nitpick. The field doc in D1 is already on the type this diff ships, so it is not the same deferred page.

No wasm build, oracle, corpus, GPU or `gate --sprint` was run. No adapter was opened. Tier evidence is the absence of a tier branch, not three renderers. This pass does not close the story. The append behaviour can stay. D1 is a comment fix, and a later independent pass has to see that sentence after it changes. Do not attach this pass to a tree that still has the old sentence, and do not treat the author's earlier green receipt as this review.
