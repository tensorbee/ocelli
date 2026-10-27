# F-057 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s19` at HEAD `870cb01` (`870cb0130d3418b7ac677748fdba49475fbb3061`). Not a staged tree. Unstaged `.claude/plans/F-057-design.md`, `Cargo.lock`, `crates/ocelli-render/Cargo.toml`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs` and `crates/ocelli-volume/src/lib.rs`. Untracked `crates/ocelli-render/src/progressive.rs`, `crates/ocelli-render/tests/progressive.rs` and `crates/ocelli-volume/tests/progressive.rs`. This review file was written after those hashes were taken and is not part of the reviewed tree.
**Author / reviewer**: Author is the parent implementation agent (grok) on `sprint/s19`. Reviewer is an independent microscope subagent in a separate context and did not write the implementation, the plan or the tests. This is not a human review and not a self-review.
**Result**: 3 defects, 0 smells, 1 nitpick

## Acceptance challenged

AC1. `U8` dims `[4, 1, 2]`, edge 128, one brick. Slice 0 is `[1, 2, 3, 4]`. Slice 1 stays clear. `ready_bricks` is empty. `cut_ready_brick([0, 0, 0])` is `BrickNotReady`. `present_slice_f32(1)` is `SliceNotPresent`. `present_slice_f32(0)` compares `f32::to_bits` with `[1.0, 2.0, 3.0, 4.0]`. A cut of the cleared store would return zeros for slice 1 and fail that match. The test passed.

AC2. After slice 1 is `[5, 6, 7, 8]`, the cut extent is `[4, 1, 2]` and the samples are `[1, 2, 3, 4, 5, 6, 7, 8]`, x fastest, slice 0 then slice 1. That rejects an x/y swap and a one-slice pad. The test passed.

AC3. `I16` bytes `FD FF` are little-endian two's complement. PS3.3 C.7.6.3 Pixel Representation value `0001H` is a signed container, so the word is `-3`, not unsigned `65533`. Python `struct.unpack` gives `i16` `-3` and `u16` `65533`. The `f32` bit patterns are `3225419776` for `-3.0` and `1199570176` for `65533.0`. The plan says not to apply Bits Stored again. The `I16` arm is `i16::from_le_bytes` and then `f32::from`. Replacing that `i16` with `u16` made `signed_samples_stay_signed` fail. The brick samples stayed the raw bytes `FD FF 00 00` on the restored path. No second mask is in `decode_container`.

AC4, AC5 and AC6. The GPU tests call `RenderGraph::sync_volume`, not the free helper. This pass ran them with `--include-ignored --test-threads=1 --nocapture`. Nine tests passed and none were ignored. Stdout did not contain the tier C early return. AC4 read back `[1, 2, 3, 4, 5, 6, 7, 8]`, left `brick_uploads` unchanged on the second sync, then `install_recovered` dropped the texture and the next sync uploaded again. AC5 left the counter unchanged and `brick_texture` absent, with `skipped == 1`. AC6 read back `[9, 9, 9, 9, 5, 6, 7, 8]` and incremented the counter a second time. A no-adapter return would have printed the tier C line and skipped those assertions. That did not happen.

AC7. `sync_is_unavailable_on_tier_c` calls `sync_ready_bricks` with `Tier::Cpu` and a stamp that is not the ready brick. The result is `ProgressError::Upload` with `brick: None` and `BrickUploadError::Unavailable`. The upload closure is not called. The foreign stamp stays and the ready key is not inserted. Deleting the `Tier::Cpu` arm made that match fail. `sync_volume` is the production caller. Its body is the call at `graph.rs:1850`, passing `self.tier`, and the only `stamps.insert` is inside the helper after `upload` returns `Ok`. `RenderGraph::new` still returns `TierCUnavailable` before a graph exists, so the floor test cannot construct a tier C graph. That is the plan's recorded shape, and the call itself is present.

Also rejected. A grid of `ceil(u32::MAX / 128)` on two axes overflows `u32` in `grid_count` and returns `ProgressError::Grid` before `ready_bricks`. The overflow test finished in the same instant as the other CPU tests, so it did not walk that grid. `skipped` is `count.checked_sub(uploaded)`, not a saturating count and not `Unavailable`. Level keys are hardcoded `0`. `ready_bricks` still uses `brick_grid()`, which is level 0. `level0_prefix` is the same product as F-058 `byte_len` over `volume.dims` and `volume.voxel`. F-058 keeps level 0 at offset 0 and appends after that prefix. The levels suite still passed. Coarser bytes are not a second upload list.

The stamp comparison was not edited. It is still `held == &brick.samples`. `equal_brick_bytes_are_not_uploaded_twice` expects the second call to upload nothing. That test passed, which an inverted comparison would not do.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test progressive --test levels --test ready_bricks` | working tree before the temporary edits | executed now | exit 0. levels 5, progressive 3, ready_bricks 4 |
| `bin/ocelli.sh test ocelli-render --test progressive` | same tree, ignored GPU rows not run | executed now | exit 0. 6 passed, 3 ignored |
| `i16::from_le_bytes` replaced with `u16::from_le_bytes`, then `signed_samples_stay_signed` | temporary edit of `crates/ocelli-volume/src/lib.rs` only | executed now | exit 101. assertion left `1199570176`, right `3225419776` |
| Restore of that line | SHA-256 of `lib.rs` | executed now | `0cf839dbb969169b2d93075a6039725275d7bd43174802f946e3bdb437b26ef8`, same as before the edit. `signed_samples_stay_signed` then exit 0 |
| `Tier::Cpu` arm removed, then `sync_is_unavailable_on_tier_c` | temporary edit of untracked `crates/ocelli-render/src/progressive.rs` | executed now | exit 101. `Unavailable` match failed. Compiler warned that `tier` was unused |
| Restore of that arm | SHA-256 of `progressive.rs` | executed now | `00ae8cdd892d63fc7fddbe5888890616c4a07dbcb5c2e0ad796874a5f0ff4166`, same as before the edit. `sync_is_unavailable_on_tier_c` then exit 0 |
| `bin/ocelli.sh clippy ocelli-volume` and `bin/ocelli.sh clippy ocelli-render` | restored tree, `--all-targets`, warnings denied | executed now | both exit 0 |
| `bin/ocelli.sh test ocelli-render --test progressive -- --include-ignored --test-threads=1 --nocapture` | restored tree, alone, no other cargo | executed now | exit 0. 9 passed, 0 ignored, 5.87s. No tier C early-return line |
| `bin/ocelli.sh test ocelli-volume` | restored tree | executed now | exit 0. unit 1, assembly 14, brick_width 2, construct 10, levels 5, progressive 3, ready_bricks 4, doc-tests 0 |
| Python `struct` of `FD FF` | not a repo file | executed now | `i16` `-3`, `u16` `65533`. `f32` bits `3225419776` and `1199570176` |
| `rg` for `unsafe`, `wasm_bindgen` and `wasm-bindgen` | `progressive.rs` in both crates, both progressive tests, `volume` `lib.rs`, `render` `graph.rs` and `lib.rs` | executed now | no matches |
| `git diff --stat` on `ocelli-cache` and `brick_upload.rs` | working tree | executed now | empty. `upload_brick` and `Brick::cut` are not in the diff |
| `git status --short` and `git rev-parse HEAD` after restore | same product paths as the pin | executed now | HEAD still `870cb0130d3418b7ac677748fdba49475fbb3061`. Status matched the pin |

The parent receipt of 9 passed in 1.48s was not used. This pass ran the GPU command itself. `bin/ocelli.sh gate unsafe` was not used. The new Rust files are untracked, so `git ls-files` would not show them.

## Defects

### D1, `present_slice_f32` documents the wrong error outside the store

**Where**: `crates/ocelli-volume/src/lib.rs:700`
**What**: The Errors section says `SliceOutOfRange` when the index is outside the store. `SlicePresence::test` returns false when `n >= slices`, and `present_slice_f32` returns `SliceNotPresent` before `data.slice`. A volume from `assemble` or from the test helper sets `present.slices`, `dims[2]` and the store depth to the same count. `present_slice_f32(2)` on dims `[4, 1, 2]` returns `SliceNotPresent { index: 2 }`.
**Why it is wrong**: The sentence names a condition and an error. The condition produces the other error. AC1 requires `SliceNotPresent` for a slice that has not landed, and an index past the volume has not landed either. HLD section 19 wants presence to say which frames exist. A caller that matches `SliceOutOfRange` for an outside index will not see it.
**Evidence**: `lib.rs:295-298` and `lib.rs:701-708`. No device. The line was not mutated. AC1's in-range missing slice already passes as `SliceNotPresent`.
**Scope and regression**: present on this working tree. Consumers are callers of `present_slice_f32`, including a stack upload that must not receive a zero buffer. The smallest fix is to delete the `SliceOutOfRange` sentence, or to state the real predicate. That predicate is a presence bit that is set while `VoxelStore::slice` returns `None`. Do not change the AC1 refusal. A hand-built volume with a longer presence set than the store can still reach `SliceOutOfRange`. That is not the sentence as written.

### D2, a missing texture does not mean the slices have not landed

**Where**: `crates/ocelli-render/src/graph.rs:1836`
**What**: `sync_volume` says a missing texture means that brick's slices have not all landed. Both slices of the AC2 volume are present before the first sync, and there is no texture yet. `install_recovered` then drops the texture at line 1905 and the stamp at line 1906 while the slices stay present. The comment on that drop says the next sync uploads ready bricks again. A ready brick whose upload returns an error is also unstamped and has no new texture.
**Why it is wrong**: HLD section 19 says to upload as slices land and to let presence drive the loading indicator. The plan's decision is narrower and stands. An incomplete brick is not uploaded, and stored zero is not a stand-in. "No texture means the slices have not landed" is the other direction, and recovery falsifies it. F-059 is not supposed to sample a key this sync never uploaded. Treating every missing texture as an absent slice would keep a recovered complete volume on the loading path.
**Evidence**: the GPU test `sync_uploads_a_ready_brick_once` places both slices, reads the eight bytes back, calls `install_recovered`, asserts `brick_texture` is `None`, then syncs and observes `uploaded == 1`. That test passed in this pass with no tier C return. The slices were present across the gap where the texture was missing.
**Scope and regression**: the sentence on this working tree. Upload, stamp clear and the re-upload do not need a logic change. The smallest fix is to replace the sentence with the rule the code actually keeps. Sync does not create a texture for a brick `ready_bricks` omitted, and it does not upload zeros. After recovery a missing texture is a dropped device resource. The next `sync_volume` uploads ready bricks again.

### D3, the render test names AC1 for AC2's eight bytes

**Where**: `crates/ocelli-render/tests/progressive.rs:4`
**What**: The module doc says AC1's eight values are the hand-filled store, x fastest, slice 0 then slice 1. Plan AC1 places four bytes on slice 0 and leaves slice 1 absent. The eight bytes `[1, 2, 3, 4, 5, 6, 7, 8]` are AC2.
**Why it is wrong**: A false AC id sends the next reader to the wrong row. The byte-order claim matches AC2 and the GPU readback. The pixels are not wrong.
**Evidence**: plan AC1 and AC2. The GPU readback of those eight bytes passed in this pass.
**Scope and regression**: the test comment only. The smallest fix is to say AC2. No product change.

## Smells

None.

## Nitpicks

### N1, `BrickNotReady` says every refused coordinate still has an absent slice

**Where**: `crates/ocelli-volume/src/lib.rs:230` and `lib.rs:681-682`
**What**: The Display string is "brick {brick:?} still has an absent slice". The `cut_ready_brick` Errors section repeats that reason. The check is `!ready_bricks().contains`, and the variant doc on line 229 says the coordinate is not in that list. On the AC2 volume the grid is one brick. `cut_ready_brick([1, 0, 0])` is outside that grid and still returns `BrickNotReady`. That coordinate is not a brick with an absent slice.
**Why it does not block**: the refusal matches the plan, nothing is copied, and the accurate sentence is already on the variant. The in-grid AC1 case really does have an absent slice.

## Verified clean

`FD FF` stays `i16` `-3` on the restored line `i16::from_le_bytes`. The `u16` mutation went red and was reverted. `U16` uses `u16::from_le_bytes`. `F32` uses `f32::from_le_bytes`. `U8` uses `f32::from` of the byte. No `as` cast was added. Integers of 16 bits are exact in `f32`. No Bits Stored mask was added. The container bytes in the brick stay the bytes `place` copied.

Stamps are `BTreeMap<BrickKey, Vec<u8>>` beside `BrickResidency`. The key level is `0`. The value is a clone of `brick.samples`, compared by equality, and inserted only after `upload` returns `Ok`. A failed later brick keeps the earlier stamp and does not insert the refused key. `sync_volume` moves the map out and writes it back, including on error, so earlier successes stay. Tier C returns before the walk, so it does not cut and does not insert. `install_recovered` calls `self.brick_stamps.clear()` on the line after `self.bricks.clear()`. There is no earlier return in that function. `tick` does not call `sync_volume`. `write_texture` does not call `queue.submit`.

`skipped` is the grid product minus uploads in that call. Overflow of the product, of the upload counter, or of the subtraction is `ProgressError::Grid`. The overflow fixture returns before any upload. Tier A and tier B both pass the `Tier::Cpu` check and use `upload_brick`. Tier C is `BrickUploadError::Unavailable` with `brick: None`. No zero volume is built. No new `Pass`, no WGSL, no `wasm-bindgen` and no `unsafe` in the touched files.

`upload_brick` is unchanged and still takes `&ResolvedAdapter`. `get_texture_format_features` is called on `adapter.wgpu_adapter()`. `GpuContext` stores instance, device, queue, caps, graph resources and the loss slot. It does not store an adapter. `sync_volume` passes the same reference through and does not keep a second one. `RenderGraph::new` refuses tier C before the struct exists. `BrickError` is not `thiserror`. `ProgressError::Volume` carries `VolumeError`. `BrickResidency::clear` still clears textures and leaves `uploads` alone. `ready_bricks` and `Brick::cut` are not in the diff. `ocelli-volume` does not depend on `ocelli-render`. `Cargo.lock` only adds `ocelli-volume` and dev-dependency `glam` to `ocelli-render`.

Level 0 is the prefix of length `dims product * bytes_per_voxel`. That is the same arithmetic as `level_bytes(0)` when level 0 sits at offset 0, which `assemble_from` and `append_level` keep. The full volume suite, including levels, construct, assembly and ready bricks, passed after the restore.

The fact-correction sentences that were checked above match the tree. They are not the three defects.

## Limitations and handoff

No append-then-cut test was run. The prefix length was compared with `byte_len` by reading. `Brick::cut` indexes with level 0 dims, so a longer store is outside those offsets even before the prefix slice. The prefix still excludes the suffix from the buffer `cut` receives. That interaction is unread by a test.

`U16` and `F32` decode arms have no fixture. They were read. They were not mutated.

The resolved GPU tier was not printed. The run did not take the tier C return, and the readback assertions ran. Tier B is the same non-CPU branch as tier A. It was not a second adapter.

`sync_volume` itself cannot be called on a tier C graph, because `new` refuses that tier. The CPU arm was proved on `sync_ready_bricks`. The production function calls that helper. A real `upload_brick` refusal through `sync_volume` was not raised on the device. The closure maps `Err((brick, source))` to `ProgressError::Upload { brick: Some(brick), source }`.

Product source matches the pre-review hashes. The two mutations are not in the tree. The next owner fixes the three sentences. No logic change is required for the behaviour this pass executed. This is not sprint acceptance and not a `/verify` receipt.
