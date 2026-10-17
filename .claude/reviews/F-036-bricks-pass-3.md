# F-036 review, pass 3

**Reviewed**: working tree on sprint/s12 at 5bed3e6, F-036 uncommitted
**Result**: 0 defects, 0 smells, 2 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

### N1, The property never builds a second brick

**Where**: `crates/ocelli-cache/tests/bricks.rs:410`
**What**: Each dimension is still drawn from `1..=24`. Every edge `BrickGrid::new` can select is 64 or 128, so every generated grid is `[1, 1, 1]`. The extent sum and the origin check still run. They never see a brick boundary. The hand fixtures cover that boundary. Not promoted. The range does not cause a wrong result. This is the same nitpick as pass 2. The new test did not change the generator.

### N2, Two fallbacks cannot run for an edge this constructor selects

**Where**: `crates/ocelli-cache/src/brick.rs:219` and `crates/ocelli-cache/src/brick.rs:224`
**What**: `ceil_div` turns a quotient that does not fit in `u32` into `u32::MAX`. `div_ceil` of a `u32` by 64 or 128 always fits. `u32::MAX` divided by 64 ceilings to 67108864, and divided by 128 ceilings to 33554432. `div_edge` returns 0 when the edge is 0. `BrickGrid::new` only stores 64 or 128. Not promoted. Neither fallback runs for a grid this constructor builds. This is pass 2 N2.

## Verified clean

- Baseline hashes matched all nine paths before any edit. `crates/ocelli-cache/src/brick.rs` was `7820fd3a9094ebca5409bdd72389c257cdb8ab25`, so the claim that this pass did not change `brick.rs` holds. The same nine hashes matched again after the last restore. The list is at the end of this section. `pub fn set_budget` is still at `crates/ocelli-cache/src/lru.rs:246`.
- Pass 2 S1 is held on all three axes. The shipped test is `an_empty_grid_is_not_resident`. Eight-bit edge 128. Ceiling division, computed here and not copied from the test, gives `[0, 4, 4] -> [0, 1, 1]`, `[4, 0, 4] -> [1, 0, 1]`, and `[4, 4, 0] -> [1, 1, 0]`. Each product is 0 bricks. The test expects those grids. They are not wrong.
- `bin/ocelli.sh test ocelli-cache --test bricks an_empty_grid_is_not_resident` on the shipped tree exited 0. One test passed and 21 were filtered out.
- The early return was replaced with `let _ = (gx, gy, gz)`. The same filter exited 101. The panic was at `crates/ocelli-cache/tests/bricks.rs:253`, `assertion failed: !cache.is_resident(volume, &grid)`. That is the first case, because a failing `assert!` stops the loop. The three-line check was restored. `git hash-object crates/ocelli-cache/src/brick.rs` was again `7820fd3a9094ebca5409bdd72389c257cdb8ab25`.
- The check was then narrowed to `if gx == 0`. The same filter on the unmodified test exited 101 at the same assertion. The x case returns false under that check, so the panic is the next case, dims `[4, 0, 4]`. The z case was not reached. Compile warnings named unused `gy` and unused `gz`, which is the narrowed check.
- A temporary collector, under that same `if gx == 0` check, ran every case and was then removed. It exited 101. The message was `resident empty grids [[4, 0, 4], [4, 4, 0], [0, 0, 0]] count 2`. The y grid and the z grid were resident. The trailing `[0, 0, 0]` is the unused slot, not a case. The x grid was not resident. Grid equality and an empty `missing` list had already passed for all three cases, or the collector would have stopped earlier. The shipped assertion is the negation of that residency check, so y and z both make it fail when reached. The test holds the full condition. This is not still a smell.
- The collector and the narrowed check were both removed. The restored filter exited 0, one passed, 21 filtered out. No unused-binding warning on that compile.
- `is_resident` is still false when any grid axis is 0, and otherwise `missing` is empty. That is the LLD sentence in `docs/lld/cache.md`: the list is empty for a grid that has at least one brick. A zero axis makes the brick count 0, so `missing` walks nothing and returns an empty list. Without the return, `is_resident` would be true. The shipped return rejects that.
- Pass 1 D1, D2, and S1 were not mutated again. `brick.rs`, `crates/ocelli-volume/src/lib.rs`, and `crates/ocelli-volume/tests/brick_width.rs` still hash to the pass 2 baseline. The bodies still match what pass 2 verified. `checked_linear` is `checked_mul` and `checked_add` in `u64`, and `byte_offset` then does `checked_mul` by the byte width and `usize::try_from`. `missing` takes `&self`, walks x fastest, and calls `contains_key`. `Lru::get` still ticks. `contains_key` does not. `Volume::brick_grid` is `BrickGrid::new(self.dims, BrickWidth::from(self.voxel))`. `From<VoxelKind> for BrickWidth` maps `U8` to `EightBit`, both sixteen-bit kinds to `SixteenBit`, and `F32` to `Float32`.
- No `as` cast in `brick.rs`. No `unwrap`, `expect`, or panic on the address path. `BrickError` still has two variants. `BrickGrid::new` is the only decomposition. There is no size threshold and no whole-volume constructor. `ocelli-cache` depends on `bytemuck` and `ocelli-pixel`. It does not depend on `ocelli-volume` or `wgpu`.
- Hashes after the last restore:

```
7820fd3a9094ebca5409bdd72389c257cdb8ab25  crates/ocelli-cache/src/brick.rs
be76b6533b95fbb33315d2d3382ceb7f3e2f76cd  crates/ocelli-cache/src/lib.rs
2dbe186cd3f0b5868c7d5c93a05c578e5e720ffc  crates/ocelli-cache/tests/bricks.rs
91770c6ee83c920908d272b6ad283292d83394cb  crates/ocelli-volume/src/lib.rs
139d0473123e86712f8e52ef99a0c9a3ad18518c  crates/ocelli-volume/tests/brick_width.rs
8f84813f43b57baf0c5809875a5c155e350581b8  docs/lld/cache.md
a7d271b31bd08393ff4cfcaf5df98d9e1b597863  docs/lld/volumes.md
81836f06fb4a9ca352318b97aa20ad15c4c406f5  docs/lld/README.md
daa09c1ed15d9727fe03aa1d50a9f00461d40076  docs/sprints/BACKLOG.md
```
