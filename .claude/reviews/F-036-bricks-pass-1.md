# F-036 review, pass 1

**Reviewed**: working tree on sprint/s12 at 5bed3e6
**Result**: 2 defects, 1 smell, 2 nitpicks

## Defects

### D1, An in-range address panics in debug and returns a wrapped `Ok` in release

**Where**: `crates/ocelli-cache/src/brick.rs:153` and `crates/ocelli-cache/src/brick.rs:174`
**What**: `index_of` and `byte_offset` multiply with the wrapping `u64` operator. `byte_offset` then scales by `saturating_mul`. Both doc comments say a result that does not fit in `usize` is `BrickError`. A release build does not do that. It returns `Ok` of a wrapped or saturated value. A debug build panics on the multiply at those two lines.
**Why it is wrong**: The approved plan requires checked arithmetic, widened before the products and narrowed once, and says an address is an error rather than a wrapped index. HLD section 19's layout is that product. HLD section 23 treats a panic as poisoning a wasm instance. Saturation to `u64::MAX` is not an error either. On a 64-bit host that value fits in `usize`, so the function reports success for an address that does not exist.
**Evidence**: Temporary probes, removed after the runs. `brick.rs` was not edited for these probes.

- Voxel `[0, 0, 2^23]` in dims `[2^20, 2^20, 2^23 + 1]` at 4 bytes. The linear index is `2^63`, which fits in `u64`. Times 4 is `2^65`, which does not. Debug and release both returned `Ok(18446744073709551615)`, which is `u64::MAX`, not `Err`.
- Voxel `[0, 0, 19999999]` in dims `[1000000, 1000000, 20000000]` at 1 byte. The true product is `19999999000000000000`. Debug panicked at `brick.rs:174` with `attempt to multiply with overflow`. Release returned `Ok(1553254926290448384)`, the product modulo `2^64`.
- Brick `[0, 0, 2^20]` in a `u32::MAX` cube at the sixteen-bit edge. The true index is `2^72`. Debug panicked at `brick.rs:153` with `attempt to multiply with overflow`. Release returned `Ok(0)`, the same index as brick `[0, 0, 0]`.

`bin/ocelli.sh test ocelli-cache --test bricks -- probe_linear_product_past_u64_is_not_a_successful_offset` exited 101 in debug. The same filter with `--release` exited 101 and printed the wrapped `Ok` values above. The probes were deleted. `git hash-object crates/ocelli-cache/src/brick.rs` returned `2f7c8a87a749af3816e46532b5d777263344a287`.

### D2, The missing-order test stays green when recency and order are both wrong

**Where**: `crates/ocelli-cache/tests/bricks.rs:186` and `crates/ocelli-cache/src/brick.rs:378`
**What**: `missing_follows_index_order_and_is_not_a_use` builds a grid of `[3, 1, 1]` and inserts brick 0 before brick 1. That is the same order `missing` scans. Calling `Lru::get` from `missing` refreshes brick 0 and then brick 1, so brick 0 is still the least recently used and the assertion still passes. A z-fastest walk of a one-row grid pushes the same three keys. The code under review does use `contains_key` and x-fastest loops. The test does not hold either choice in place.
**Why it is wrong**: The plan says `missing` returns keys in `index_of` order and does not touch recency, because a residency query must not change the next eviction. Microscope item 2. A test that passes both ways will be counted as that guard forever. `Lru::get` at `crates/ocelli-cache/src/lru.rs:185` ticks before the lookup, including on a miss. `contains_key` at `lru.rs:203` does not.
**Evidence**: `missing` was changed to `&mut self`, `is_resident` with it, and the `contains_key` test was replaced with `self.bricks.get(&key).is_none()`. `bin/ocelli.sh test ocelli-cache --test bricks` exited 0 with all 18 committed tests passing, including `missing_follows_index_order_and_is_not_a_use`. A temporary probe inserted brick 1 and then brick 0, called `missing`, and inserted an 80-byte stranger into the same 256-byte budget. Under `get` it failed with left `Some([0, 0, 0])` and right `Some([1, 0, 0])`. After restoring `contains_key`, that probe passed. The z-fastest loop swap, x outermost, was then applied on its own. `bin/ocelli.sh test ocelli-cache --test bricks -- missing_follows_index_order_and_is_not_a_use` exited 0. The full bricks binary also exited 0. Both edits were reverted. `git hash-object` on `brick.rs` and `tests/bricks.rs` matched the baseline hashes below.

## Smells

### S1, `Volume::brick_grid` has no caller in the tests

**Where**: `crates/ocelli-volume/src/lib.rs:375`
**What**: The method is the one line `BrickGrid::new(self.dims, BrickWidth::from(self.voxel))`. `crates/ocelli-volume/tests/brick_width.rs` calls `BrickWidth::from` itself and never builds a `Volume`. Replacing the body with `BrickGrid::new([1, 1, 1], BrickWidth::EightBit)` left the volume suite green.
**Why it matters**: The behaviour is correct today, which is why this is a smell. A later edit can ignore `self.dims` or `self.voxel` and `each_voxel_kind_selects_one_brick_edge` still passes, because that test never enters the method. `docs/lld/volumes.md` and `docs/lld/cache.md` both name `Volume::brick_grid` as the conversion.
**Evidence**: `bin/ocelli.sh test ocelli-volume` exited 0 with the scaffold test, 14 assembly tests, and the brick-width test all passing. The body was restored. `git hash-object crates/ocelli-volume/src/lib.rs` returned `91770c6ee83c920908d272b6ad283292d83394cb`.

## Nitpicks

### N1, The property never builds a second brick

**Where**: `crates/ocelli-cache/tests/bricks.rs:336`
**What**: Each dimension is drawn from `1..=24`. Every edge `BrickGrid::new` can select is 64 or 128. Every generated grid is `[1, 1, 1]`. The extent sum and the origin check still run, and they still reject a full-edge extent on a volume smaller than the edge. They never see a brick boundary. The hand fixtures cover that boundary.

### N2, Two fallbacks cannot run for an edge this constructor selects

**Where**: `crates/ocelli-cache/src/brick.rs:215` and `crates/ocelli-cache/src/brick.rs:218`
**What**: `ceil_div` turns a quotient that does not fit in `u32` into `u32::MAX`. `div_ceil` of a `u32` by 64 or 128 always fits. `div_edge` returns 0 when the edge is 0, which would report brick 0 for every voxel. `BrickGrid::new` only stores 64 or 128.

## Verified clean

- Baseline hashes matched the nine paths in the assignment before any edit. After every mutation was reverted they matched again. The commands and the results are at the end of this section.
- Hand arithmetic, not the comments. A 512 by 512 by 600 volume at edge 64 ceilings to grid 8, 8, 10. Voxel 200, 130, 599 floors to brick 3, 2, 9. Origin is 192, 128, 576. Section 19's product `((576 * 512 + 128) * 512 + 192) * 2` is 302121344. That value, the last voxel's offset 314572798, and the whole volume 314572800 all fit in `u32` (`u32::MAX` is 4294967295). The plan's sentence that this fixture wraps is false. The separate wide fixture is the pin, and the code does not wrap it. See the `u32` mutation below. The plan's sentence that every brick of the 512 volume is 524288 bytes is also false. The last z brick starts at 576 and has extent 24. `64 * 64 * 24 * 2` is 196608. `a_sixteen_bit_volume_decomposes_at_edge_64` asserts that partial extent and those two byte counts. It does not assert the false sentence. A 500 by 300 by 130 volume at edge 128 ceilings to 4, 3, 2. The last brick is extent 116, 44, 2 and `116 * 44 * 2` is 10208. A 64 by 64 by 40 volume at edge 128 is one brick. `64 * 64 * 64 * 2` is 524288. `512 * 512 * 600 * 2` is 314572800, which is greater than `256 * 1024 * 1024` (268435456). `(1 * 65536) * 65536` is 4294967296.
- Truncating division. `u64::from(dim).div_ceil(u64::from(edge))` was replaced with `/`. `bin/ocelli.sh test ocelli-cache --test bricks -- an_edge_brick_reports_its_true_extent_and_its_true_bytes the_brick_containing_a_voxel_is_the_floor_of_each_axis a_sixteen_bit_volume_decomposes_at_edge_64` exited 101. The edge fixture's grid was left `[3, 2, 1]` and right `[4, 3, 2]`. Both 512 assertions were left `[8, 8, 9]` and right `[8, 8, 10]`. The `div_ceil` call was restored.
- `index_of` order. The x and z terms were swapped. `bin/ocelli.sh test ocelli-cache --test bricks -- index_of_is_x_fastest_then_y_then_z` exited 101 at `bricks.rs:140`, left `Ok(64)` and right `Ok(1)`. The original product was restored. The formula is `(z * grid_y + y) * grid_x + x`, which is section 19 at brick scale.
- `u32` narrowing. The byte-offset product was rewritten with `wrapping_mul` and `wrapping_add` on `u32`, then widened. `a_brick_origin_and_its_byte_offset_agree` stayed green. `a_byte_offset_past_u32_max_does_not_wrap` failed with left `Ok(0)` and right `Ok(4294967296)`. Exit 101. The `u64::from` products were restored. The shipped function does not wrap that wide offset on this 64-bit host. On wasm32 the same `usize::try_from` fails for 4294967296 and returns `VoxelOutOfRange` instead of truncating. That is the documented unfit-address path, and it is a different hole from D1.
- Recency of the shipped `missing`. It calls `contains_key`, which does not tick. The defect is that the committed test does not notice the opposite. D2 records both mutations.
- Decision D11. `BrickGrid::new` is the only decomposition. There is no size comparison and no whole-volume constructor in `brick.rs` or in `Volume::brick_grid`. A one-brick volume and the 512 volume share that constructor. The plan already says a threshold test would stay green. No such branch was found, so there is no finding.
- Decision C1. `BrickCache::new` passes `CacheTier::Gpu` at `brick.rs:351`. Replacing it with `CacheTier::Decoded` made `an_oversized_brick_is_refused_and_evicts_nothing` fail, left `Decoded` and right `Gpu`. Exit 101. Restored. `Brick::bytes` returns `self.samples.len()` at `brick.rs:335`. Returning 0 failed the edge fixture at left 0 and right 10208, and the sixteen-bit fixture at left 0 and right 524288. Exit 101. Restored.
- Decision F1. `BrickError` has `VoxelOutOfRange` and `BrickOutOfRange`, each with `axis` and `value`, at `brick.rs:59`. Deleting the `voxel_in_range` call in `brick_of` made `an_out_of_range_voxel_or_brick_is_an_error_on_each_axis` fail, left `Ok([8, 0, 0])` and right `Err(VoxelOutOfRange { axis: 0, value: 512 })`. The coordinate was not wrapped to 0. The check was restored.
- Decision B1. `ocelli-volume` already depended on `ocelli-cache` before this story. `From<VoxelKind> for BrickWidth` maps `U8` to `EightBit`, both sixteen-bit kinds to `SixteenBit`, and `F32` to `Float32`. The edge numbers 128, 64 and 64 live only in the match inside `BrickGrid::new`. The untested method is smell S1, not a wrong conversion.
- Tiers. `docs/lld/cache.md` says brick addressing is full on A, B and C, that tier B's 256 floor holds a 128 edge, and that tier C runs the same code and holds host bricks. Those sentences match the plan and the code. No device is created. No `docs/hld/DEVIATIONS.md` row was added. Search of `brick.rs` found no `unsafe`, no `wasm-bindgen`, and no `as` cast. No new trait. `BrickCache` wrapping `Lru` is the type the plan specifies, and it fixes the tier.
- Other guards that do go red. Ceiling on an exact axis stays one brick and one past the edge adds a brick, with one test per axis. An oversized insert is `Admission::refused` with nothing evicted and `used == 0`. Eviction of the only resident brick clears `is_resident` for that volume. The row-copy fixture places 7 at volume byte 328 and reads it back at brick sample 72, which is `200 - 128`.
- Commands on the restored tree. `bin/ocelli.sh test ocelli-cache --test bricks` exited 0 with 18 passed. `bin/ocelli.sh test ocelli-volume --test brick_width` exited 0 with 1 passed. `bin/ocelli.sh clippy ocelli-cache` exited 0. `bin/ocelli.sh clippy ocelli-volume` exited 0. Exit codes were read from the commands themselves.
- Hashes after the last revert:

```
2f7c8a87a749af3816e46532b5d777263344a287  crates/ocelli-cache/src/brick.rs
be76b6533b95fbb33315d2d3382ceb7f3e2f76cd  crates/ocelli-cache/src/lib.rs
a9f4926b3d0566da045d2d8a3e4b9b4e8c5252ff  crates/ocelli-cache/tests/bricks.rs
91770c6ee83c920908d272b6ad283292d83394cb  crates/ocelli-volume/src/lib.rs
248325d953479cc2fd35441049f5acbfc243f28e  crates/ocelli-volume/tests/brick_width.rs
996cb497a408c0ae396b9d4a5746a1fcdbf955f8  docs/lld/cache.md
a7d271b31bd08393ff4cfcaf5df98d9e1b597863  docs/lld/volumes.md
81836f06fb4a9ca352318b97aa20ad15c4c406f5  docs/lld/README.md
daa09c1ed15d9727fe03aa1d50a9f00461d40076  docs/sprints/BACKLOG.md
```
