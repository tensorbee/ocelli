# F-043 review, pass 3

**Reviewed**: uncommitted working tree on `sprint/s14` against `bd66269`
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1, The three LLD pages are still unchanged

**Where**: `.claude/plans/F-043-design.md` section "LLD impact", and `docs/lld/render-graph.md`, `docs/lld/cache.md`, `docs/lld/volumes.md`
**What**: None of those pages is in the diff. Carried from pass 1. Complete-feature step 9 owns the LLD, so this stays a nitpick.

## Verified clean

The pass 2 hashes still match, so the walk, the loop order, the stored sample, and the float width were not edited again.

```
cd9e4a714dccc20c1c7d26179a2c8847b2d613e6  crates/ocelli-cache/src/brick.rs
3bca7d1e7107d49d604e1656439c98df34d06c72  crates/ocelli-volume/src/lib.rs
321d01d7221461972e1ad1d2a1fb7b7f7d1f1e80  crates/ocelli-volume/tests/ready_bricks.rs
```

`crates/ocelli-cache/tests/bricks.rs` is still `8c9c4aa9b56114605952cb4d28a146937fb0fa0e`. `crates/ocelli-render/tests/brick_upload.rs` is still `82ab41a02f1b4ad1bfc3f90ee32aee0e80232787`. `git status --short -- docs/lld` prints nothing. The tracked diff is the same eight paths recorded for this feature.

`crates/ocelli-render/src/brick_upload.rs` is now `e2874823f249fc78bf27f1ad5138a2343b552fa8`. Putting the pass 2 comment back into a temporary copy of that file produces `be6f07a5d4b151d53f061279407ffbc7b913a6f4`, the pass 2 hash. The difference is 83 bytes, all inside the doc comment on `brick_bytes_match`. The four mutations from pass 2 were not repeated.

The comment at lines 136 to 138 says `brick.samples.len()` is the voxel count times the sample width, that this product is the only accepted length, and that the check does not write. `expected_len` at line 177 multiplies the three axes by `bytes_per_voxel` with checked arithmetic. Extent `[1, 1, 1]` is one voxel, and that product is `bytes_per_voxel`, which `expected_len` returns. `brick_bytes_match` returns `Ok` when `samples.len()` equals the returned length. A different length returns `WrongLength`. `None` from `expected_len` returns `ExtentUnavailable`, which the unchanged Errors section names. The function reads the length and does not call `write_texture`.
