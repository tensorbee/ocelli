# F-104 review, pass 4

**Reviewed**: staged index `3d7bde55a538fad8539a5dc62df37336e05c7e52` on `sprint/s17` at HEAD `8b9eb65`. This review file is not part of that tree. Source hashes at review: `boundary.rs` `facf5ca3177bf583ff35096be193de9757ff125e1a127bd51d2c874a3f4668c1`, `typegen.rs` `f0f0e2981cb8c0f4f4e0416c99d7d51a51a4d36c825b071db55d82dfddd8768b`, `ring.rs` `e6d7c50e72ee50d473fdb724b6a5e2b2dfa2565dd61d13eaf2c8991f32fe7826`, `commands.rs` `abc50fa516de9d7cf4564aab7694e3282e9f13de0504979bf6995fa8e693ffdb`, `session.rs` `30b873ad71aa1acfd39a9d369b38bbff277e13721e715cccba83d214d7511675`, `boundary.wasm.test.ts` `dbb87b24305818a9c24219fef33017a696ab69a113cbe10e4c47a3fd6eebea57`.

**Author / reviewer**: The implementation was already in the staged tree. This pass did not write it. The reviewer is the S17 host session. DeepSeek was not asked, on the operator's instruction.

**Result**: 0 defects, 3 smells, 2 nitpicks.

## Acceptance challenged

AC9. The presented-event test binds a 64 by 64 canvas and then reads payload offsets 4 and 8. Both expected values are 64. Swapping drawn width with drawn height in `push_presented` leaves that test green. The stride, kind, viewport and sequence checks do reject a wrong stride. They do not reject a swapped extent.

The generator contract. `generated_types_match_the_committed_file` returns success when `generated_path` is `None`. The approved plan says a missing workspace root is a refusal, and a mismatch is a failure, never a skip.

HLD 17.1 little-endian. `write_state` and `EventRing::copy_pending` used `to_le_bytes` before this diff. They now copy `bytemuck::bytes_of`. On this little-endian host the bytes match. A big-endian build would publish a byte-swapped window centre and a byte-swapped ring header, and the shell reads both as little-endian.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate --floor` then `bin/ocelli.sh gate corpus` | index `3d7bde55a538fad8539a5dc62df37336e05c7e52` | executed now, before this review file existed | `FLOOR_EXIT:0`, 26 gates, `CORPUS_EXIT:0`, 95 manifest rows |
| Read of `push_presented` and the presented-event test | `ring.rs` payload order and `boundary.wasm.test.ts` | executed now, by reading | width and height expectations are both 64 |
| Staged diff of the byte stores | `commands.rs` and `ring.rs` | executed now | `to_le_bytes` helpers deleted, `bytes_of` added |
| Independent camera derivation | Fit 800 by 600, 512 by 512, 1 mm spacing, no rotation | executed now, by calculation | element 0 is `512/600`, element 12 is `-256/3` |

## Defects

None. On this host the new copies match the previous little-endian stores. The command lengths, the 96-byte state and the 48-byte event match HLD 17.3 and 17.4.

## Smells

### S1, square presented extent

**Where**: `packages/core/src/boundary.wasm.test.ts` around the `bindCpuCanvas(..., 64, 64, 1)` call and the payload reads of offsets 4 and 8
**What**: drawn width and drawn height are both expected to be 64
**Why it is wrong**: the payload contract is generation, then drawn width, then drawn height, then flags. A swap is a wrong extent, and this is the test AC9 cites for that event
**Evidence**: both literals are 64, and `device_extent(64, 64, 1)` is `(64, 64)`. The assertion cannot go red on a swap
**Scope and regression**: the new AC9 test only. `push_presented` writes width then height. The smallest repair is a non-square device extent plus a flag value that is not zero, so a missing or swapped word fails

### S2, generator skip

**Where**: `crates/ocelli-viewport/tests/typegen.rs` `generated_types_match_the_committed_file`
**What**: `let Some(path) = generated_path() else { return }`
**Why it is wrong**: the approved plan says the test refuses when the workspace root cannot be found, rather than skipping. A `#[test]` that returns is a pass
**Evidence**: `generated_path` returns `Option` through `parent()?.parent()?`, and `None` takes the success return. The read path asserts and then returns on `Err` as well, which is dead after the assert
**Scope and regression**: only a manifest directory with a single path component reaches `None`. The missing `packages/core/src` path already asserts. The repair is to panic on both failures

### S3, host endian after the explicit little-endian stores were removed

**Where**: `StackViewport::write_state` and `EventRing::copy_pending`
**What**: `bytemuck::bytes_of` replaced `to_le_bytes` for the state, the ring header and the event
**Why it is wrong**: HLD 17.1 says the wire format is little-endian. The shell reads with a little-endian `DataView`. Host order matches only while the target is little-endian. A window centre swapped on a big-endian build is the quietly wrong value this project exists to catch
**Evidence**: the staged diff deletes `write_f32`, `write_u32` and the ring's `to_le_bytes` helpers
**Scope and regression**: camera, window centre, window width, slice, flags, and the ring header and event words. Payload bytes inside an event were already little-endian before the struct copy. The repair is a compile failure on a big-endian target, not a second hand-written layout

## Nitpicks

- N1. `Session::copy_state` still says it copies 96 bytes and returns 96. The wrapper returns `STATE_BYTES`. The number is 96 today and the layout test pins it
- N2. `drain_events` spells `EVENT_HEADER_BYTES` and `EVENT_STRIDE` as full paths, while `STATE_BYTES` in the same module is imported by name

## Verified clean

- `ViewportState` is 96 bytes. Offsets are 0, 64, 68, 72, 76, 80 and 84, which is HLD 17.4 including the three padding words
- The ring header is 16 bytes and the event stride is 48. The payload is 32 bytes at offset 16. That is HLD 17.3
- Payload lengths match the decoders. Fit 24, Pan 16, Zoom 8, Rotate 8, each flip 1, SetWindow 17, preset 1, invert 1, colormap 1, Scroll 4
- Enum bytes are the previous literals. Linear 0, linear exact 1, sigmoid 2. Presets 0, 1 and 2. Identity 0 and inverse 1. Grey 0 and stops 1
- `flag` still accepts only 0 and 1. Those values are the invert constants, so flips and invert keep the bytes they already had. Splitting them would add a second copy of the same match
- `FLAG_INVERT` is the explicit inverse. `MONOCHROME1` is not that bit
- Fit of an 800 by 600 canvas over a 512 by 512 grid at 1 mm spacing gives camera element 0 of `512/600` and element 12 of `-256/3`. The test's `-85.33333` is that translation at four decimal places
- No `wasm-bindgen` was added outside `ocelli-wasm`. No new `unsafe`. The drain path copies bytes out and does not return a view over linear memory
- The hand-written shell modules import the generated constants. The committed `generated.ts` is what `typegen` emits on this tree
- Floor and corpus were green on this index. That is not a receipt for a later edit

## Limitations and handoff

A big-endian compiler was not run. The boundary wasm suite was not repeated in this pass. Its earlier 12 passing tests and the stride mutation belong to the previous source hashes, which this pass did not change. The three smells are repaired next, then a fresh pass reviews that repair. Do not record a verify receipt for `3d7bde55a538` and then edit. That receipt would describe a tree the repair leaves behind.
