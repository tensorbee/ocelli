# F-057 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s19` at HEAD `870cb01` (`870cb0130d3418b7ac677748fdba49475fbb3061`). Not a staged tree. Same product paths as pass 1, after the three sentence edits. Unstaged `.claude/plans/F-057-design.md`, `Cargo.lock`, `crates/ocelli-render/Cargo.toml`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs` and `crates/ocelli-volume/src/lib.rs`. Untracked `crates/ocelli-render/src/progressive.rs`, `crates/ocelli-render/tests/progressive.rs` and `crates/ocelli-volume/tests/progressive.rs`. `.claude/reviews/F-057-progressive-pass-1.md` is also untracked and was not part of the remediation.
**Author / reviewer**: Author of the three sentences is the parent implementation agent (grok). Reviewer is the same microscope subagent that wrote pass 1 and did not write these sentences. This is not a human review and not a self-review of the remediation.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

Pass 1 D1, D2 and D3 were false sentences. This pass checked that each old sentence is gone and executed the replacement. No function body was edited. `crates/ocelli-render/src/progressive.rs` still hashes to `00ae8cdd892d63fc7fddbe5888890616c4a07dbcb5c2e0ad796874a5f0ff4166`, the pass 1 value. The signed decode is still `i16::from_le_bytes`. The stamp comparison is still `held == &brick.samples`.

D1 closed. `present_slice_f32` no longer says `SliceOutOfRange` when the index is outside the store. The new Errors text says `SliceNotPresent` when the presence bit is clear, including an index at or past the slice count, and `SliceOutOfRange` when that bit is set and the store has no such slice. `SlicePresence::test` returns false when `n >= slices` and does not read a bit. It also returns false when the bit is clear. `present_slice_f32` returns `SliceNotPresent` on that false result, before `data.slice`. `assemble_from` and the progressive fixture set `present.slices` from `dims[2]`, so an index at or past that count takes the first arm. `SliceOutOfRange` is only the `ok_or` after `test` has returned true and `VoxelStore::slice` has returned `None`. That is the second sentence. A search for the old "outside the store" wording in this function finds nothing.

D2 closed. `sync_volume` no longer says a missing texture means the slices have not landed. The new text says a brick `ready_bricks` omits is not uploaded and is not filled with zeros. The walk is `for coord in volume.ready_bricks()` and then `cut_ready_brick`. An omitted coordinate is not passed to `upload` and is not given a zero texture. Recovery is `install_recovered`: `self.bricks.clear()` then `self.brick_stamps.clear()`, with no return before those lines. The next `sync_volume` calls `sync_ready_bricks`. With the stamps gone, a ready brick fails the `held == &brick.samples` check and is uploaded again. Pass 1's GPU test `sync_uploads_a_ready_brick_once` already observed that re-upload, and `sync_skips_a_brick_with_an_absent_slice` already observed no texture and no upload for the omitted brick. Those bodies are unchanged, so those receipts still apply.

D3 closed. The render test module doc now says AC2's eight values, x fastest, slice 0 then slice 1. Plan AC2 is samples `[1, 2, 3, 4, 5, 6, 7, 8]` in that order. The string `AC1's eight` is absent. The only AC id in that test file is AC2. Pass 1 read those eight bytes back from the device.

N1 was not changed. It stays a nitpick.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Read of `present_slice_f32`, `SlicePresence::test` and `VoxelStore::slice` | `lib.rs` after the Errors edit. SHA-256 `4b2a0dd5263825b14d5838b0e0c9e939da9702726ea1f3a4096d6dfa28953f99` | executed now | `test` is false at or past `slices`. `SliceNotPresent` is that arm. `SliceOutOfRange` is only after a set bit and a missing store span |
| Read of `sync_volume` and `install_recovered` | `graph.rs` SHA-256 `b42a48ff16d0874694dbb58253ff491b102477605138354e69d02da679eeeb97` | executed now | Omitted bricks are outside the `ready_bricks` loop. Textures and stamps clear on consecutive lines. Next sync has no stamp |
| Read of the render progressive module doc | `tests/progressive.rs` SHA-256 `9081ecbd1b56fab6aaa199d69e85e74283aad0b3ab8163316ca9a8b3b89a6659` | executed now | Line 4 names AC2. No `AC1's eight` |
| `rg` for the three old phrases in `crates` | this tree | executed now | No match in the F-057 files. The only "outside the store" hit is an unrelated codec comment |
| SHA-256 of `crates/ocelli-render/src/progressive.rs` | untracked helper | executed now | Matches the pass 1 hash. The walk was not edited |
| `git rev-parse --short HEAD` and `git status --short` | working tree | executed now | HEAD `870cb01`. Product status matches pass 1 plus the sentence edits. No commit |
| GPU progressive suite, signed `u16` mutation, tier C arm removal | pass 1, then restored | prior receipt, not re-run | GPU exit 0, 9 passed, 0 ignored, 5.87s, no tier C early return. Both mutations exit 101 and were reverted before pass 1 closed |
| Volume suite, clippy on both crates, CPU progressive tests | pass 1 restored tree | prior receipt, not re-run | All exit 0. Not repeated, because this pass changed comments only |

No test was re-run. The executable bodies cited above match the bodies pass 1 ran.

## Defects

None.

Pass 1 D1, D2 and D3 are closed. The three replacements match the code they describe. No new false sentence was added.

## Smells

None.

## Nitpicks

### N1, `BrickNotReady` says every refused coordinate still has an absent slice

**Where**: `crates/ocelli-volume/src/lib.rs:230` and `lib.rs:681-682`
**What**: Unchanged from pass 1. The Display string is "brick {brick:?} still has an absent slice", and the `cut_ready_brick` Errors section repeats it. The check is `!ready_bricks().contains`. The variant doc on line 229 says the coordinate is not in that list. On the AC2 volume, `[1, 0, 0]` is outside the one-brick grid and still returns `BrickNotReady`.
**Why it does not block**: the refusal matches the plan, nothing is copied, and the accurate sentence is already on the variant. The in-grid AC1 case really does have an absent slice. The remediation left this wording on purpose.

## Verified clean

The remediation is three comments. `present_slice_f32`, `sync_volume`, `sync_ready_bricks` and `install_recovered` do the same work pass 1 reviewed. The helper file hash is unchanged. `i16::from_le_bytes` and `held == &brick.samples` are still the restored forms.

`SliceNotPresent` covers a clear bit and an index `test` rejects because it is past `present.slices`. `SliceOutOfRange` is not that case. It is the later arm, and only when the bit is set. The old outside-the-store claim is not in the function.

An omitted brick is not in the upload loop, so this path does not invent a zero texture for it. `install_recovered` drops the texture map and the stamp map together. The following sync treats a missing stamp as a brick that still needs upload. That is what the new `sync_volume` paragraph says, and it is what the pass 1 device test already measured.

AC2 is the eight-byte row. The test comment now cites that row. The x-fastest order in the comment is the order pass 1 read back.

No new `as` cast, tier branch, pass, WGSL, `unsafe` or `wasm-bindgen` arrived with the sentences. Tier C, grid overflow and the stamp-after-success rule were not retouched.

## Limitations and handoff

Behaviour was not executed again. The GPU readback, the `u16` mutation and the tier C arm deletion remain pass 1 receipts. They apply only because the bodies they ran are still the bodies on this tree.

N1 is still open and still does not block. No product edit is required for a clean behavioural pass. This is not sprint acceptance and not a `/verify` receipt. The next owner can leave N1 or tighten the Display string. Completion still belongs to a later step.
