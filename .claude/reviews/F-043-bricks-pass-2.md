# F-043 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s14` against `bd66269`
**Result**: 1 defect, 0 smells, 1 nitpick

## Defects

### D1, The length comment says one voxel is not enough

**Where**: `crates/ocelli-render/src/brick_upload.rs:138`
**What**: `brick_bytes_match` now says the length is the product of the three axes and the sample width, which is what `expected_len` computes. The next sentence says one voxel is not enough. A brick of extent `[1, 1, 1]` is one voxel. Its accepted length is the sample width, not more than one voxel.
**Why it is wrong**: A false sentence in a doc comment is a defect. The new cut test builds that extent for `I16` and for `U16`, two bytes each, and the restored test passes. The product rule in the sentence above already replaced the pass 1 claim. This sentence denies a legal extent.
**Evidence**: Read against `expected_len` at `brick_upload.rs:176` and the passing `cut_keeps_the_sample_kind_and_float_is_four_bytes`. No upload behaviour was changed for this finding. The comment is the wrong part.

## Smells

None.

## Nitpicks

### N1, The three LLD pages are still unchanged

**Where**: `.claude/plans/F-043-design.md` section "LLD impact", and `docs/lld/render-graph.md`, `docs/lld/cache.md`, `docs/lld/volumes.md`
**What**: None of those pages is in the diff. Carried from pass 1. Complete-feature step 9 owns the LLD, so this stays a nitpick and is not a defect.

## Verified clean

Pass 1's four product guards were re-run. The remediation did not move the walk or the loops. `git hash-object crates/ocelli-volume/src/lib.rs` is still `3bca7d1e7107d49d604e1656439c98df34d06c72`, the pass 1 hash. `ready_bricks` is still x innermost, then y, then z. `brick_is_ready` is still `slice < end` with `end = oz + ez`.

The new expectations match the specification, not a copied loop. Eight-bit edge is 128. `200 - 128 = 72`. Brick 0 covers 0 through 127. Brick 1 covers 128 through 199. `130` cubed ceilings to grid `[2, 2, 2]`. `index_of([1, 0, 0])` is 1. `index_of([0, 0, 1])` is 4, because `(1 * 2 + 0) * 2 + 0 = 4`. The listed keys are that order. Two float voxels are 8 bytes. Byte 4 is the first byte of the second voxel. `I16` and `U16` share a width of 2 and the test requires different stored kinds.

Mutations, each restored before the next:

- `while slice < end` became `while slice + 1 < end`. `a_brick_touching_an_absent_slice_is_not_ready` failed at `ready_bricks.rs:25`, which is `ready_except(127)`, left both bricks and right only `[0, 0, 1]`. Exit 101. The 50 and 127 asserts were then removed so only `ready_except(199)` remained. The same walk failed, left both bricks and right only `[0, 0, 0]`. Exit 101. Both files were restored.
- `ready_bricks` was rewritten with z innermost and each axis incremented in its own loop. `ready_bricks_are_in_index_order` failed at `ready_bricks.rs:57`. The left list places `[0, 0, 1]` before `[1, 0, 0]`. The right list does not. Exit 101. The x-fastest loops were restored.
- Every successful `cut` stored `BrickSample::U8`. `cut_keeps_the_sample_kind_and_float_is_four_bytes` failed at `bricks.rs:384`, left `U8` and right `I16`. Exit 101. The passed `sample` was restored.
- `BrickSample::F32` width was changed from 4 to 2. The same test failed at `bricks.rs:372`, left 2 and right 4. Exit 101. The width was restored to 4.

After those restores, `bin/ocelli.sh test ocelli-volume --test ready_bricks` exited 0 with 4 passed. `bin/ocelli.sh test ocelli-cache --test bricks -- cut_keeps_the_sample_kind_and_float_is_four_bytes a_sixteen_bit_volume_decomposes_at_edge_64 a_cut_copies_the_row_at_the_section_19_offset` exited 0 with 3 passed.

The `cut` comment now says `sample` is the kind and that sixteen-bit samples share a byte width, not a format. That matches the function, which stores the passed `sample`. The `Ok` arm was not given a second kind. `F32` is 4 again at `brick.rs:42`.

`brick.rs` relative to HEAD still only adds `BrickSample`, stores that argument, and adjusts the comments. The upload module's plan, format match, and `write_texture` path were not part of this remediation. The GPU readback was not repeated. No new `as` cast, `unwrap`, or `expect` was added in the fixtures. The 256 MiB line in `bricks.rs` is still the host-budget fixture, not an upload branch. No edge brick is padded. `set_voi` was not edited. The carried LINEAR path was not edited.

Hashes after the last restore:

```
cd9e4a714dccc20c1c7d26179a2c8847b2d613e6  crates/ocelli-cache/src/brick.rs
3bca7d1e7107d49d604e1656439c98df34d06c72  crates/ocelli-volume/src/lib.rs
321d01d7221461972e1ad1d2a1fb7b7f7d1f1e80  crates/ocelli-volume/tests/ready_bricks.rs
```
