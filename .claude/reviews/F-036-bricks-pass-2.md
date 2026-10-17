# F-036 review, pass 2

**Reviewed**: working tree on sprint/s12 at 5bed3e6, F-036 uncommitted
**Result**: 0 defects, 1 smell, 2 nitpicks

## Defects

None.

## Smells

### S1, The empty-grid check in `is_resident` is not held by a test

**Where**: `crates/ocelli-cache/src/brick.rs:421`
**What**: `is_resident` returns false when any grid component is 0. Deleting that return left all 21 tests in the bricks binary green. With the check restored, a temporary probe of dims `[0, 4, 4]` built grid `[0, 1, 1]`, saw an empty `missing` list, and saw `is_resident` false. The probe was removed.
**Why it matters**: `docs/lld/cache.md` says `is_resident` is that list being empty for a grid that has at least one brick. Without the check, a zero component makes `missing` return nothing, so `is_resident` is true and that sentence is false. The suite does not notice. The behaviour today matches the sentence, which is why this is a smell and not a defect.
**Evidence**: The check was replaced with `let _ = (gx, gy, gz)`. `bin/ocelli.sh test ocelli-cache --test bricks` exited 0 with 21 passed. The check was restored. A temporary test `an_empty_grid_is_not_resident` was added and `bin/ocelli.sh test ocelli-cache --test bricks an_empty_grid_is_not_resident` exited 0. That test was then deleted. `git hash-object crates/ocelli-cache/src/brick.rs` and `crates/ocelli-cache/tests/bricks.rs` matched the baseline after the restore.

## Nitpicks

### N1, The property never builds a second brick

**Where**: `crates/ocelli-cache/tests/bricks.rs:391`
**What**: Each dimension is still drawn from `1..=24`. Every edge `BrickGrid::new` can select is 64 or 128, so every generated grid is `[1, 1, 1]`. The extent sum and the origin check still run. They never see a brick boundary. The hand fixtures cover that boundary. Not promoted. The range does not cause a wrong result.

### N2, Two fallbacks cannot run for an edge this constructor selects

**Where**: `crates/ocelli-cache/src/brick.rs:219` and `crates/ocelli-cache/src/brick.rs:224`
**What**: `ceil_div` turns a quotient that does not fit in `u32` into `u32::MAX`. `div_ceil` of a `u32` by 64 or 128 always fits. `div_edge` returns 0 when the edge is 0. `BrickGrid::new` only stores 64 or 128. Not promoted. Neither fallback runs for a grid this constructor builds.

## Verified clean

- Baseline hashes matched all nine paths before any edit. They matched again after the last restore. The list is at the end of this section. `pub fn set_budget` is still at `crates/ocelli-cache/src/lru.rs:246`.
- Pass 1 D1 stayed dead. `checked_linear` was put back to wrapping `u64` multiply and add, and the byte-width step was put back to `saturating_mul`. `bin/ocelli.sh test ocelli-cache --test bricks past_u64` exited 101. Debug panicked in the wrapping multiply for the linear-index and brick-index tests, at the mutated helper. The byte-width test failed the assertion with left `Ok(18446744073709551615)` and right `Err(VoxelOutOfRange { axis: 2, value: 8388608 })`. The same filter with `--release` exited 101 and did not panic. Release left `Ok(0)` for the brick index, `Ok(1553254926290448384)` for the linear index, and `Ok(18446744073709551615)` for the byte product. Those are the wrapped and saturated values, not `Err`. Both edits were restored. On the restored tree the same release filter exited 0 with 3 passed, so the shipped code does not return `Ok` for those products.
- The three products were computed independently before the runs. Voxel `[0, 0, 2^23]` in dims `[2^20, 2^20, 2^23 + 1]` has linear index `2^63`. Times 4 is `2^65`. Voxel `[0, 0, 19999999]` in dims `[1000000, 1000000, 20000000]` has product `19999999000000000000`, past `u64::MAX`. Brick `[0, 0, 2^20]` in a `u32::MAX` cube at edge 64 has grid axis `67108864`, so the brick is inside the grid, and the index is `2^72`. The release `Ok(0)` on the mutated brick index shows the test reached the multiply. An early range rejection would have stayed `Err`.
- Pass 1 D2 stayed dead, in both halves. `missing` and `is_resident` were changed to `&mut self`, and `contains_key` was replaced with `self.bricks.get(&key).is_none()`. `bin/ocelli.sh test ocelli-cache --test bricks missing_follows_index_order_and_is_not_a_use` exited 101 at `bricks.rs:228`, left `Some([1, 0, 0])` and right `Some([0, 1, 0])`. That is the eviction victim. Those three lines were restored. Separately, only the walk was changed to x outermost. The same filter exited 101 at `bricks.rs:196`, left `[0, 1, 0]` and right `[1, 0, 0]`. The x-fastest walk was restored. `Lru::get` at `lru.rs:185` still ticks before the lookup. `contains_key` at `lru.rs:203` does not. The shipped `missing` calls `contains_key`.
- Pass 1 S1 stayed dead. `Volume::brick_grid` was replaced with `BrickGrid::new([1, 1, 1], BrickWidth::EightBit)`. `bin/ocelli.sh test ocelli-volume --test brick_width` exited 101. The dims assertion was left `[1, 1, 1]` and right `[130, 70, 3]`. The edge assertion was left `128` and right `64`. The one-line body was restored.
- No `as` cast in `brick.rs`, `bricks.rs`, or `brick_width.rs`. The F-036 diff in `ocelli-volume/src/lib.rs` has none. No `unwrap`, `expect`, `panic!`, `todo!`, or `unimplemented!` in `brick.rs`. Overflow on the address path is `checked_mul`, `checked_add`, and `usize::try_from`.
- The pure width overflow still carries axis 2 and the z component. That payload is acceptable under the plan. Decision F1 gives `BrickError` exactly two variants, each with axis and value, and it does not name a third failure for a product. The doc comments and `docs/lld/cache.md` say a product that does not fit in `u64`, or an address that does not fit in `usize`, is `BrickError`, never a wrapped index and never a panic. The restored debug suite and the restored release filter both return `Err` for the three probes, so that sentence matches the code. It is not broader than `byte_offset` and `index_of`.
- Hand fixtures, computed here and asserted in the tests, not copied back from a wrong run. A 512 by 512 by 600 volume at edge 64 ceilings to 8, 8, 10. Voxel 200, 130, 599 floors to brick 3, 2, 9. Origin is 192, 128, 576. `((576 * 512 + 128) * 512 + 192) * 2` is 302121344. The last z brick is extent 64, 64, 24 and `64 * 64 * 24 * 2` is 196608. An interior 64 cube at two bytes is 524288. The plan's sentence that every brick of the 512 volume is 524288 bytes is false, and neither the test nor `docs/lld/cache.md` states it. The plan's sentence that this fixture wraps a `u32` is also false. `512 * 512 * 600 * 2` is 314572800, which fits in `u32` and is greater than 268435456. A 500 by 300 by 130 volume at edge 128 ceilings to 4, 3, 2. The last brick is extent 116, 44, 2 and `116 * 44 * 2` is 10208. The tests assert those numbers.
- `missing` does not allocate a use. The shipped signature is `&self` and the body calls `contains_key`. The D2 mutation is what a use would do, and that mutation went red. Hardcoding the key level to 0 left the 21 tests green. That is not filed. `BrickGrid::new` writes level 0, and `a_one_brick_volume_uses_the_same_constructor` compares the returned key to `grid.level`, so the two agree for every grid this constructor builds.
- Decision D11. `BrickGrid::new` is the only decomposition. There is no size comparison and no whole-volume constructor in `brick.rs` or in `Volume::brick_grid`.
- `BrickCache::insert` returns `self.bricks.insert(key, brick)` unchanged. `BrickCache::new` passes `CacheTier::Gpu`. `Brick::bytes` returns `self.samples.len()`. The edge fixture asserts 10208 and the sixteen-bit fixture asserts 524288 and 196608.
- `ocelli-cache` depends on `bytemuck` and `ocelli-pixel`. It does not depend on `ocelli-volume` and it does not depend on `wgpu`. `ocelli-volume` depends on `ocelli-cache`. `From<VoxelKind> for BrickWidth` maps `U8` to `EightBit`, both sixteen-bit kinds to `SixteenBit`, and `F32` to `Float32`.
- Tiers. `docs/lld/cache.md` says brick addressing is full on A, B and C, that it is host arithmetic, that tier B's 256 floor holds a 128 edge, and that tier C runs the same code. No device is created in `brick.rs`. No `docs/hld/DEVIATIONS.md` row was added.
- This host is 64-bit. The past-`u32` test expects `Ok(4294967296)`, which fits in this host's `usize`. These integration tests are not run for wasm32. `bin/ocelli.sh` says a wasm32 test run needs a browser runner and is not this gate. On a 32-bit `usize` the same call does not fit, and `usize::try_from` is the documented `BrickError` path. That was not executed here.
- Commands on the restored tree, with exit codes from the commands themselves. `bin/ocelli.sh test ocelli-cache --test bricks` exited 0 with 21 passed, before the mutations. `bin/ocelli.sh test ocelli-volume --test brick_width` exited 0 with 2 passed. `bin/ocelli.sh test ocelli-cache --test bricks past_u64 --release` exited 0 with 3 passed after the restore. `bin/ocelli.sh clippy ocelli-cache` exited 0. `bin/ocelli.sh clippy ocelli-volume` exited 0.
- Hashes after the last restore:

```
7820fd3a9094ebca5409bdd72389c257cdb8ab25  crates/ocelli-cache/src/brick.rs
be76b6533b95fbb33315d2d3382ceb7f3e2f76cd  crates/ocelli-cache/src/lib.rs
6791cd75cff293485c884de366642eaf7177172f  crates/ocelli-cache/tests/bricks.rs
91770c6ee83c920908d272b6ad283292d83394cb  crates/ocelli-volume/src/lib.rs
139d0473123e86712f8e52ef99a0c9a3ad18518c  crates/ocelli-volume/tests/brick_width.rs
8f84813f43b57baf0c5809875a5c155e350581b8  docs/lld/cache.md
a7d271b31bd08393ff4cfcaf5df98d9e1b597863  docs/lld/volumes.md
81836f06fb4a9ca352318b97aa20ad15c4c406f5  docs/lld/README.md
daa09c1ed15d9727fe03aa1d50a9f00461d40076  docs/sprints/BACKLOG.md
```
