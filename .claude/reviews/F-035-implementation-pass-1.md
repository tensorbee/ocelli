# F-035 review, pass 1

**Reviewed**: uncommitted working tree on `work/f-035-grok` against `2e8d033`. `git diff --stat` is 9 files, 542 insertions and 35 deletions. No untracked files.
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the wide-`usize` arm of `check_buffer` never runs here

**Where**: `crates/ocelli-cache/src/image.rs:166`
**What**: `usize::try_from(self.bytes)` takes the `Err` arm only when `bytes` does not fit in `usize`. The host test binary is `target_pointer_width="64"`, so every `u64` fits. `refuses_a_length_that_disagrees_with_the_buffer` uses lengths 4 and 0 only. The arm is the wasm32 refusal the LLD describes, and it is not wrong. This suite cannot see a change to it.
**Judgment**: Missing pin on a 32-bit edge, not a wrong result.

## Verified clean

The pass read the uncommitted diff, the approved plan, HLD sections 5.2, 6, 17.1, 17.2 and 24, and decision D5 as restated in `CLAUDE.md`. No source, test, plan or LLD was left changed by the pass.

`exportDecodedFrame` builds both typed arrays inside the call and returns a fresh `ArrayBuffer`. `acceptTransferredFrame` throws on `buffer.byteLength !== meta.bytes` before `alloc`, then builds the wasm view, then `set`, then `commit_frame`. `transferredFrameMessage` returns `{ message, transfer }` where `message` is `{ kind: "frame", frameId, buffer, meta }` and `transfer` is `[buffer]`, the same object. It does not call `postMessage` and it does not build a view. None of the three stores a `Uint8Array`, returns one, or hoists one. That is the plan's reading of decision D3. The returned value is the section 24 `ArrayBuffer`, not a view over `wasm.memory.buffer`.

The growth fixture is `returns the second call's bytes after wasm memory grows` in `packages/core/src/bulk.test.ts`. This pass hoisted the view, re-ran the test, and reverted the hoist. The mutation was `retainedView ??= new Uint8Array(wasm.memory.buffer)` and a copy through `new Uint8Array(retainedView.buffer, ptr, len)`. `npx vitest run packages/core/src/bulk.test.ts` reported `1 failed | 12 passed (13)` and the red test was that growth test, with `TypeError: Cannot perform Construct on a detached ArrayBuffer`. After revert, `git diff --numstat` for `packages/core/src/bulk.ts` was `92 0` again, and the same vitest command plus `packages/core/src/index.test.ts` reported 17 passed. The accept-before-`alloc` mutation and the Rust endian and inequality mutations were not re-run. The accept function's source order is the section 17.2 order, and its test uses `growingSink`, whose `alloc` calls `memory.grow`.

`HandoffMeta` is `#[repr(C)]` with `SeriesId` now `#[repr(transparent)]`. `bin/ocelli.sh test ocelli-cache --lib handoff_tests` passed both tests. The wire constant is the little-endian image of `0x0102_0304_0506_0708`, `0xA0B0_C0D0`, `0x1122_3344` and `524_288`, checked by hand against those values, and the test asserts `to_le_bytes` and `from_le_bytes` against that constant plus offsets 0, 8, 12 and 16, size 24 and alignment 8. The host is little-endian, so the passing test alone would not separate `to_le_bytes` from `to_ne_bytes`. The source calls `to_le_bytes` and `from_le_bytes`. `check_buffer` returns `Ok` only when the lengths match, returns `HandoffError::Length` otherwise, and does not read the sample bytes. The TypeScript refusal sets `series`, `instance` and `frame` equal to the buffer length and `bytes` unequal. The success path does the opposite, so the wrong field cannot satisfy both tests.

The plan correction matches the signatures. `acceptTransferredFrame` takes `BulkSink`. `exportDecodedFrame` takes `WasmMemory` only. `transferredFrameMessage` takes neither. All three are in `packages/core/src/bulk.ts`. `index.ts` re-exports the functions and the `HandoffMeta` and `FrameMessage` types. `eslint.config.js` still disables the wasm-memory view ban only for `bulk.ts`, `panic.ts` and `packages/core/src/*.test.ts`.

The diff adds no `unsafe`, no `wasm_bindgen`, no new trait, no `as` cast and no `wgpu` use. The plan's three tier rows stay n/a. `ocelli-render` is untouched. `bin/ocelli.sh clippy ocelli-cache` exited 0. `npx eslint` on the four touched TypeScript files exited 0. `python3 scripts/prose_check.py` reported voice rules clean over 370 files. Added LLD and plan lines have no em-dash and no prose semicolon outside inline code. The new LLD sentences that name the copy order, the length throw, the transfer list, the 24-byte layout and the two-file allowance match the code and the eslint list.
