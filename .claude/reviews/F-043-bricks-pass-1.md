# F-043 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s14` against `bd66269`, including the three untracked files
**Result**: 4 defects, 0 smells, 1 nitpick

## Defects

### D1, The absent-slice test does not lock the last slice of the brick

**Where**: `crates/ocelli-volume/src/lib.rs:613` and `crates/ocelli-volume/tests/ready_bricks.rs:14`
**What**: `brick_is_ready` walks `oz .. oz + ez`. That bound is right. The committed fixture omits slice 50 of a 128-deep brick and still passes if the walk stops one short of the end. Slice 50 stays inside the shortened range, so brick `[0, 0, 0]` is still excluded, and brick `[0, 0, 1]` stays ready because its own last slice was present.
**Why it is wrong**: The plan and HLD section 19 say a brick whose z range includes an absent slice is not ready. The last index of brick 0 is 127 and the last index of brick 1 is 199. Neither absence is in the fixture. A test that passes both ways is the guard that will be counted, which is microscope item 2.
**Evidence**: `while slice < end` was changed to `while slice + 1 < end`. `bin/ocelli.sh test ocelli-volume --test ready_bricks` exited 0, 4 passed. The bound was restored. A probe that is not in the tree then called the restored function. Omitting only slice 127 left `[[0, 0, 1]]`. Omitting only slice 199 left `[[0, 0, 0]]`. That probe passed and was deleted. `git hash-object crates/ocelli-volume/src/lib.rs crates/ocelli-volume/tests/ready_bricks.rs` is again `3bca7d1e7107d49d604e1656439c98df34d06c72` and `3fa69bb415c2860a3ca4849c9a44880d971df71b`.

### D2, The order test does not lock z as the slowest axis

**Where**: `crates/ocelli-volume/src/lib.rs:578` and `crates/ocelli-volume/tests/ready_bricks.rs:33`
**What**: The shipped loops are x, then y, then z, which matches `BrickGrid::index_of`. The order test builds a grid of `[2, 2, 1]`. A walk with z innermost emits those same four keys, because z never changes.
**Why it is wrong**: The plan requires `index_of` order, x fastest, then y, then z. `index_of` has its own test in `ocelli-cache`. `ready_bricks` has a second set of loops, and the test named for that order does not hold the z term. Microscope item 2.
**Evidence**: The loops were rewritten with y outermost, x in the middle, and z innermost. `bin/ocelli.sh test ocelli-volume --test ready_bricks` exited 0, 4 passed. The loops were restored. On the restored function, a deleted probe of a complete `[200, 4, 200]` volume returned `[[0, 0, 0], [1, 0, 0], [0, 0, 1], [1, 0, 1]]`. That is x, then z, with a single y. The same hash pair as D1 matches after the restore.

### D3, Nothing reads the sample `Brick::cut` stores, or the float width

**Where**: `crates/ocelli-cache/src/brick.rs:382` and `crates/ocelli-cache/src/brick.rs:42`
**What**: `cut` copies with `sample.bytes_per_voxel` and stores `sample` on the brick. No test reads `brick.sample` after `cut`. `BrickSample::F32` is 4 bytes, and no test reads that width either. `I16` and `U16` widths of 2 are pinned by the 524288-byte cut and by `bytes_per_row == 128`. `U8` is pinned by a row of 3. The float arm is not.
**Why it is wrong**: The upload format is `brick.sample`, and `cut` is the function that attaches the kind to the copied bytes. PS3.3 C.7.6.3 Pixel Representation is why `I16` and `U16` must stay distinct after the copy, not only in `From<VoxelKind>`. A float width of 2 would plan a 2-byte stride for a 4-byte voxel. The format tests construct `Brick` literals, so they never enter `cut`. Microscope item 2.
**Evidence**: The `Ok` arm stored `BrickSample::U8` for every cut. `bin/ocelli.sh test ocelli-cache --test bricks` exited 0, 22 passed. That edit was restored. `Self::F32 => 4` was then changed to 2. Cache bricks exited 0 with 22 passed, `ready_bricks` exited 0 with 4 passed, and `bin/ocelli.sh test ocelli-render --test brick_upload` exited 0 with 4 passed and the GPU test ignored. The 4 was restored. `git hash-object crates/ocelli-cache/src/brick.rs` is `a5249ce5e9dfedd9c7b6e7716bd7e31634c3c378`.

### D4, Two doc comments describe a different function from the one they sit on

**Where**: `crates/ocelli-render/src/brick_upload.rs:136` and `crates/ocelli-cache/src/brick.rs:302`
**What**: `brick_bytes_match` says `brick.samples` is exactly one voxel of `brick.extent`. The function accepts the buffer only when its length is the full extent times `bytes_per_voxel`. The `cut` comment says `` `sample` is the byte width ``. `BrickSample` is the kind. `I16` and `U16` have one width and two formats, which the next sentence of that comment already says.
**Why it is wrong**: A false doc comment is a defect. The length sentence tells a later edit to compare against one voxel, which would refuse every real brick. The width sentence collapses the distinction D3 is about.
**Evidence**: Read against `brick_bytes_match` at `brick_upload.rs:145` and `BrickSample::bytes_per_voxel` at `brick.rs:38`. No behaviour change was required. The comments are the wrong part.

## Smells

None.

## Nitpicks

### N1, The three LLD pages named in the plan are unchanged

**Where**: `.claude/plans/F-043-design.md` section "LLD impact", and `docs/lld/render-graph.md`, `docs/lld/cache.md`, `docs/lld/volumes.md`
**What**: None of those pages is in the diff. `volumes.md` still says texture upload is F-040 and does not mention `ready_bricks`. That sentence remains true of the volume crate, which still creates no texture. The gap is omission, not a contradicted behaviour claim.

## Verified clean

The pixel path was re-run, not taken from the author's note. On the restored tree, `bin/ocelli.sh test ocelli-render --test brick_upload -- --include-ignored` exited 0. All 5 tests passed, including `a_brick_upload_matches_the_ramp_and_a_window_does_not_reupload`. The ramp readback is the 27 stored bytes, 0 through 26. `set_voi` left `brick_uploads` at 2 and those texels unchanged. Replacement made the counter 3. `install_recovered` left it at 3 and dropped both textures. A later upload made it 4.

`I16` is `R16Sint` at `brick_upload.rs:80`. Changing that arm to `R16Uint` made `signed_and_unsigned_sixteen_bit_select_different_formats` fail, left `R16Uint` and right `R16Sint`, exit 101. The arm was restored before the clean GPU run. `U8` is `R8Uint`, `U16` is `R16Uint`, and `F32` is `R32Float`. The same test refuses `R32Float` for an `I16` brick, `R8Unorm` for `U8`, and compares both sixteen-bit formats with `R16Unorm`. There is one match and no second format.

Tier C is the first branch in `brick_plan`. The unit test passes `R32Float` for an `I16` brick on `Tier::Cpu` and expects `Unavailable`, so the format is not consulted. `upload` returns the same brick on every `brick_plan` error, before `create_texture`. `opens_a_device` is false for `Tier::Cpu`, and `resolve_adapter` returns `None`, so the graph method is not the tier C entry. The pure function is. A missing `TEXTURE_BINDING` or `COPY_DST` is `FormatUnavailable`. An axis of 0 is `EmptyExtent`.

There is no 256 MiB comparison and no whole-volume texture in the new upload. `brick_plan` copies `brick.extent`. A `[512, 512, 600]` sixteen-bit grid is `[8, 8, 10]`, and brick `[0, 0, 9]` plans `[64, 64, 24]` with a row of 128, not `[512, 512, 600]`. An `[8, 8, 8]` brick plans `[8, 8, 8]`, not a 128 cube. A `[130, 1, 1]` eight-bit grid's second brick is width 2. The GPU test uploaded that brick and read back `[9, 10]`. The cache file's 256 MiB assert is the pre-existing host-budget fixture, not an upload branch.

`bytes_per_row` is the extent width times the sample width. The 3 by 3 by 3 plan asserts 3 and asserts it is not 256. Pinned `wgpu-types` 30.0.1 says `COPY_BYTES_PER_ROW_ALIGNMENT` does not apply to `Queue::write_texture`. The readback helper is the only 256-byte stride, and it unpacks before the comparison. The GPU test's match is the evidence that the tight write landed.

`set_voi` writes the thirty-two-byte uniform and does not call `upload_brick`. That was read at `graph.rs:1318` and confirmed by the GPU counter. `install_recovered` calls `bricks.clear`, which drops the map and does not add to `uploads`. A short buffer returns `WrongLength` before `create_texture`. The GPU test used 26 bytes, saw counter 0, and saw no texture.

The host brick is taken by value and is not inserted into `BrickCache`. `ocelli-cache` still has no wgpu dependency. `Cargo.lock` adds one edge, `ocelli-render` on `ocelli-cache`. The map key is the existing `BrickKey`. Phase 1 does not evict textures to a byte budget. The new module has no indirection texture, no volume shader, and no staging pool.

`tick` does not name `bricks`. Both frame destructures end in `..`. Upload allocation is on `upload_brick`, not on the frame. `upload_stored` is not in the diff, and `stored_bytes_per_row` is still the 2D float helper.

Added product lines have no `unsafe`, no `wasm_bindgen`, no `as` cast, no `unwrap`, and no `expect`. A search of the added diff hit the word "as" only in two English sentences. There is no new trait, no new generic parameter, and no `Box<dyn>`.

Decision D11 is what the upload follows. A volume that fits is still a brick. Deviation D-11 was not involved. The carried LINEAR path was not edited. The ramp that came back is the container bytes, not a windowed value.

`Limits::downlevel_defaults().max_texture_dimension_3d` is asserted equal to 256, a `[128, 1, 1]` brick is accepted at that limit, and `[257, 1, 1]` is `ExtentUnavailable`. The comparison is `>`. An axis equal to the limit would also be accepted, and a grid edge of 64 or 128 cannot produce 256, so `>=` was not filed.

`ExtentUnavailable` is also returned when a product does not fit in `u32` or `usize`. The `Display` text mentions only the device limit. Device limits this crate records cannot reach that overflow for these four widths, so it was not filed.

Hashes after the last revert, matching the tree this pass started from:

```
a5249ce5e9dfedd9c7b6e7716bd7e31634c3c378  crates/ocelli-cache/src/brick.rs
3bca7d1e7107d49d604e1656439c98df34d06c72  crates/ocelli-volume/src/lib.rs
33b27b2eb0a3ceaa3f23f8a2200013595782d037  crates/ocelli-render/src/brick_upload.rs
3fa69bb415c2860a3ca4849c9a44880d971df71b  crates/ocelli-volume/tests/ready_bricks.rs
```
