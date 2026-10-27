# F-104 review, pass 1

**Reviewed**: HEAD `8b9eb65` (`S17, approve sprint designs`), plus the unstaged F-104 working-tree diff over 14 tracked paths and three untracked files. The tracked diff is 435 insertions and 252 deletions across `Cargo.lock`, `crates/ocelli-viewport/Cargo.toml`, `crates/ocelli-viewport/src/commands.rs`, `crates/ocelli-viewport/src/lib.rs`, `crates/ocelli-viewport/src/ring.rs`, `docs/lld/public-api.md`, `docs/lld/typescript-packaging.md`, `packages/core/src/boundary.wasm.test.ts`, `packages/core/src/commands.test.ts`, `packages/core/src/commands.ts`, `packages/core/src/index.test.ts`, `packages/core/src/index.ts`, `packages/core/src/ring.ts` and `packages/core/src/viewport.ts`. The untracked files are `crates/ocelli-viewport/src/boundary.rs` (`sha256 facf5ca3177bf583ff35096be193de9757ff125e1a127bd51d2c874a3f4668c1`), `crates/ocelli-viewport/tests/typegen.rs` (`sha256 f0f0e2981cb8c0f4f4e0416c99d7d51a51a4d36c825b071db55d82dfddd8768b`) and `packages/core/src/generated.ts` (`sha256 9810a82c1ca0d685a1959e9a64c3ab8d9cbedb5bd9601a97bb37e63771c71946`). The F-104 paths were not edited by this pass. The AC2 and generated-file mutations were temporary, reverted, and proven byte-identical to the pre-mutation copies.

**Author / reviewer**: The author is the F-104 implementer, whose handoff is `.claude/scratch/F-104-progress.md`. The reviewer is an independent subagent that did not author the change. The author's identity is not established from a commit trailer. The reviewer has no authorship in the changed material. A concurrent workflow-engineering change appeared during the pass and staged 14 paths under `.claude/`, `.agents/` and `.opencode/`, including an eight-line addition to `.claude/commands/microscope.md`. That change is not F-104 and is not reviewed here.

**Result**: 2 defects, 1 smell, 10 nitpicks.

## Acceptance challenged

- **AC2** was executed, not read. `payload_bytes: 17` for `SetWindow` was changed to `21` in `crates/ocelli-viewport/src/boundary.rs`. The typegen test went red at `line 48: committed "  SetWindow: 17,", generated "  SetWindow: 21,"`. `UPDATE_TYPES=1` rewrote the file and the test went green. Reverting to `17` and regenerating went green, and both files matched the pre-mutation copies byte for byte. This rejects a comparison that reads the committed file and restates it, and rejects a generator that never writes.
- **AC3** was challenged with the independent hand-built byte fixture in `packages/core/src/commands.test.ts`. It pins the envelope and payloads `1, 24, 17, 16, 8, 8, 1, 1, 1, 1, 4` from the HLD, not from `COMMAND_PAYLOAD_BYTES`. The generator reads `COMMAND_TABLE`, so a change to the Rust table moves both sides and the hand-built fixture goes red.
- **AC5** was challenged on both sides. `packages/core/src/boundary.wasm.test.ts` sends every generated `VOI_FUNCTION_BYTE`, `PRESET_BYTE`, `INVERT_BYTE` and `COLORMAP_BYTE` value through the built module and expects one applied command, then sends `9`, `9`, `2` and `2` and expects zero. `commands.test.ts` pins the exact maps `[0, 1, 2]`, `[0, 1, 2]`, `[0, 1]`, `[0, 1]`. Mutating `VOI_FUNCTION_BYTE.linear` to `7` went red in `commands.test.ts`. A permutation of the map would still be accepted by the decoder count assertion, so the exact-value assertion in `commands.test.ts` is the guard that catches a shift.
- **AC6** was challenged by mutating `STATE_VOI_CENTER_OFFSET` from `64` to `68` in the committed generated file. `packages/core/src/viewport.test.ts` went red with `expected 400 to be 40`, because the fixture writes the HLD literal `64` and the reader uses the generated offset. This rejects a reader that agrees with itself.
- **AC7 and AC8** were executed against a rebuilt dev module. The boundary suite applies all eleven kinds packed by `packCommands` and gets `11`, then reads `copy_state()` through `readViewportState` and gets centre `40`, width `400`, slice `0`, count `0`, flags `0`. The camera values `camera[0] = 512 / 600` and `camera[12] = -85.333` were derived independently from `Fit`: `fit_scale = 600 / 512`, `a = 1 / fit_scale = 512 / 600`, `c = 256 - 400 * a = -85.333`. This rejects a fixture copied from the implementation's output and rejects a state offset that reads the wrong field.
- **AC9** was challenged on its integrated half. The empty-header half is real and green. The presented-event half is not covered, and the recorded reason is false. `crates/ocelli-wasm/src/canvas.rs:680` exports `publishCpuCanvas`, which calls `publish_cpu_frame`, which calls `Session::present`, which calls `ring.push_presented` with kind 6. `packages/core/src/boundary.wasm.test.ts:495` already asserts that `publishCpuCanvas` is in the glue. The path is not exercisable in Node because Node 24 has no `OffscreenCanvas` or `ImageData`. The AC state is correctly recorded as unmet. The stated reason is not.
- **AC11** was challenged by reading the three hand-written modules for boundary literals. `viewport.ts` imports every state offset and the flag bits. `commands.ts` imports the ids, payload lengths and enum bytes. `ring.ts` imports the header size, stride, field offsets and payload size. The remaining literals are the sixteen camera indices in `cameraAt`, the `8` command envelope in the packer, and the `0` and `1` flip bytes. The camera count is tied to the `Camera` tuple and `CAMERA_COMPONENTS`, so a drift fails type-checking. The envelope and flip bytes are discussed under N7 and N8.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --porcelain`, `git diff --stat`, `git rev-parse HEAD` | Working tree at `8b9eb65` plus the F-104 diff and three untracked files | Executed now | 14 modified F-104 paths, 3 untracked F-104 files, 14 unrelated staged workflow paths that appeared mid-pass |
| `cargo test -p ocelli-viewport --test typegen` | `crates/ocelli-viewport/tests/typegen.rs` against `packages/core/src/generated.ts` | Executed now | 2 passed, exit 0 |
| `bin/ocelli.sh test ocelli-viewport` | The crate's unit and integration tests | Executed now | Green, exit 0 |
| `bin/ocelli.sh gate test` (`cargo test --workspace`) | The floor test gate | Executed now | Green, exit 0. The typegen test is included |
| `bin/ocelli.sh clippy ocelli-viewport` | The changed crate | Executed now | Green, exit 0 |
| `bin/ocelli.sh gate fmt nostd` | Workspace formatting and no_std dependency reach | Executed now | Green, exit 0, 7 no_std crates |
| `bin/ocelli.sh gate unsafe bindgen pins` | `unsafe` allow-list, wasm-bindgen confinement, pins | Executed now | Green, exit 0, 155 files checked, 2 permitted |
| `bin/ocelli.sh gate types` | `tsc --build` across the TypeScript workspaces | Executed now | Green, exit 0 |
| `npm run typecheck` | TypeScript build | Executed now | Green, exit 0 |
| `npm run lint` | ESLint including the wasm-view ban | Executed now | Green, exit 0 |
| `npm run test` | The default vitest suite | Executed now | 10 files, 107 tests, exit 0 |
| `npx vitest run packages/core/src/commands.test.ts packages/core/src/viewport.test.ts packages/core/src/ring.test.ts packages/core/src/index.test.ts` | The four named files | Executed now | 4 files, 29 tests, exit 0 |
| `python3 scripts/prose_check.py` | Voice rules over tracked operator-facing prose | Executed now | Green over 479 files, exit 0. The gate uses `git ls-files` and does not see the three untracked source files, which are outside its prose scope |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | A dev module at the current tree | Executed now | Green, exit 0. The pre-existing build was older than `boundary.rs`, so it was rebuilt |
| `npx vitest run --config vitest.boundary.config.ts` | The built module through `packages/core/src/boundary.wasm.test.ts` | Executed now | 1 file, 11 tests, exit 0 |
| AC2 mutation, `SetWindow` 17 to 21 | `crates/ocelli-viewport/src/boundary.rs` | Executed now | Red, exit 101, `line 48: committed "  SetWindow: 17,", generated "  SetWindow: 21,"` |
| AC2 regeneration | `UPDATE_TYPES=1 cargo test -p ocelli-viewport --test typegen` | Executed now | 2 passed, exit 0, generated `SetWindow: 21` |
| AC2 revert and regeneration | Restore 17, regenerate | Executed now | 2 passed, exit 0, `BOUNDARY_IDENTICAL` and `GENERATED_IDENTICAL` |
| Offset mutation, `STATE_VOI_CENTER_OFFSET` 64 to 68 | `packages/core/src/generated.ts` | Executed now | Red, exit 1, `viewport.test.ts` 1 failed and 6 passed, `expected 400 to be 40` |
| Enum mutation, `VOI_FUNCTION_BYTE.linear` 0 to 7 | `packages/core/src/generated.ts` | Executed now | Red, exit 1, `commands.test.ts` 1 failed and 8 passed, received `[7, 1, 2]` |
| Payload mutation, `COMMAND_PAYLOAD_BYTES.SetWindow` 17 to 21 | `packages/core/src/generated.ts` | Executed now | Red, exit 1, two tests failed, `RangeError: SetWindow payload is 17 bytes and the Rust table declares 21` at `commands.ts:183` |
| Restore and green re-run | Pre-mutation copies | Executed now | `GENERATED_IDENTICAL`, 4 files and 29 tests green, typegen 2 passed |
| `bin/ocelli.sh gate wasm` | Release size budget | Prior receipt only | Not re-run in this pass. The progress note records 2,052,821 bytes against 2,099,014 |
| `bin/ocelli.sh gate --sprint` | The sprint profile | Not run | The final profile is outside this working pass |

## Defects

### D1, the "one definition" claim is false for the wasm export wrappers

**Where**: `docs/lld/public-api.md:14`, `.claude/scratch/F-104-progress.md:11`, `crates/ocelli-wasm/src/session.rs:1041`, `crates/ocelli-wasm/src/session.rs:1043`, `crates/ocelli-wasm/src/session.rs:1081`.

**What**: The changed LLD says `crates/ocelli-viewport/src/boundary.rs` "is the one definition of the boundary's numbers". The progress note repeats the claim. The wasm export wrappers still restate the numbers. `bind::copy_state` allocates `vec![0_u8; 96]` and checks `copied == 96`. `drain_events` allocates `vec![0_u8; 16 + 256 * 48]`. The values 96, 16, 256 and 48 are the state size, the ring header size, the ring capacity and the ring stride.

**Why it is wrong**: The plan's invariant says "Every boundary number is declared once in Rust and appears in TypeScript only in `generated.ts`." `STATE_BYTES` is now derived from `size_of::<ViewportState>()` in `commands.rs`, and the ring copy uses `EVENT_HEADER_BYTES` and `EVENT_STRIDE` from `boundary.rs`. The wrappers did not move with them. The LLD sentence is a factual claim that the observed tree contradicts.

**Evidence**: `grep -n "96\|16 + 256\|STATE_BYTES" crates/ocelli-wasm/src/session.rs` shows the two wrapper literals and the derived `STATE_BYTES` import at line 15. The boundary suite's empty-header test reads 16 bytes, so the current values are correct. The claim is the part that is wrong.

**Scope and regression**: No current wrong behaviour. The smallest meaningful regression is a change to `ViewportState` or `EVENT_STRIDE` that moves the generated TypeScript and the derived Rust constants but leaves the wrappers at 96 and 16 + 256 * 48. The manual boundary suite would catch it, the floor `test` gate would not. Fix by narrowing the LLD sentence to the viewport crate and the TypeScript shell and recording a follow-up for the wrappers, or by importing the constants into `crates/ocelli-wasm/src/session.rs`. The plan says F-104 does not touch wasm-bindgen, so the narrowing is the smaller change. Owner: F-104 author.

### D2, the AC9 record names a false blocker

**Where**: `.claude/scratch/F-104-progress.md:42`.

**What**: The record says "`Session::present` exists in `crates/ocelli-wasm/src/session.rs` but has no `#[wasm_bindgen]` export, so the plan's 'after a present call' has no public path." The second clause is false. `crates/ocelli-wasm/src/canvas.rs:680` exports `publishCpuCanvas`, which calls `publish_cpu_frame` at `canvas.rs:926` and `canvas.rs:938`, which calls `Session::present` at `session.rs:861`, which calls `ring.push_presented`. The presented kind is 6. `packages/core/src/boundary.wasm.test.ts:495` asserts that the built glue contains `publishCpuCanvas`, so the public export is already observed by this story's own test.

**Why it is wrong**: The AC is correctly recorded as partial and unmet, but the reason and the first owner are wrong. A follow-up that adds a redundant `present` export would duplicate an existing public path. The real blocker is the Node environment. Node 24.13.0 reports `typeof globalThis.OffscreenCanvas` as `undefined` and `typeof globalThis.ImageData` as `undefined`, so `bindCpuCanvas` cannot reach the retained image without stubs. The path is a browser CPU-canvas path, which the record itself names as the alternative owner.

**Evidence**: `grep -n "publishCpuCanvas\|publish_cpu_frame\|fn present" crates/ocelli-wasm/src/canvas.rs crates/ocelli-wasm/src/session.rs` shows the chain. `node -e 'console.log(process.version, typeof globalThis.OffscreenCanvas, typeof globalThis.ImageData)'` prints `v24.13.0 undefined undefined`. The boundary test's name assertion is green.

**Scope and regression**: The AC9 presented-event half remains unmet. The smallest meaningful regression is a recorded reason that sends the next owner to add a duplicate export instead of a canvas-stubbed boundary case or a browser run. Fix the record to name `publishCpuCanvas` and the Node canvas limitation, or add a presented-event case with `OffscreenCanvas`, `OffscreenCanvasRenderingContext2D` and `ImageData` stubs. Owner: F-104 author.

## Smells

### S1, the wasm export wrappers no longer track the derived boundary sizes

**Where**: `crates/ocelli-wasm/src/session.rs:1041`, `crates/ocelli-wasm/src/session.rs:1043`, `crates/ocelli-wasm/src/session.rs:1081`.

**What**: `bind::copy_state` allocates 96 bytes and accepts only 96. `drain_events` allocates `16 + 256 * 48` bytes. Before F-104, `STATE_BYTES` was a literal 96 and `ring.rs` used literal 16 and 48, so the wrappers and the producer agreed by construction. After F-104, `STATE_BYTES` is `size_of::<ViewportState>()` and the ring copy uses `EVENT_HEADER_BYTES`, `EVENT_STRIDE` and `RING_CAPACITY`. The wrappers are now a second copy that does not move.

**Failure mechanism**: If `EVENT_STRIDE` grows, `drain_events` allocates the old maximum. `EventRing::copy_pending` sees `out.len() < EVENT_HEADER_BYTES + pending * EVENT_STRIDE` and returns 0. `drain_events` truncates to 0 and returns an empty `Vec`. `drainRing` sees a zero-byte buffer and reports no events and no dropped count, so the shell silently loses the ring. The `test` gate's native `ring.rs` test uses the same old literals and would fail, which forces the test fixture to move, but the `drain_events` wrapper is not covered by the floor. It is covered only by the manual boundary suite. The `copy_state` wrapper has the same shape, with `write_state` returning `StateLength` and the shell seeing `null`.

**Why it is a smell**: The values are correct today. The hazard is a future boundary change that the floor does not catch and the LLD says cannot exist. Import `STATE_BYTES`, `EVENT_HEADER_BYTES`, `EVENT_STRIDE` and `RING_CAPACITY` in the wrapper, or narrow D1 and record the follow-up. Owner: F-104 author.

## Nitpicks

### N1, `cameraAt` hard-codes sixteen reads

`packages/core/src/viewport.ts:225` unrolls sixteen `at(...)` calls while `STATE_CAMERA_COMPONENTS` is imported and used for `CAMERA_COMPONENTS`. The count is duplicated. A loop over `STATE_CAMERA_COMPONENTS` would remove the duplicate. The `Camera` tuple type is a sixteen-tuple and `viewport.test.ts` assigns `CAMERA_COMPONENTS` to `Camera["length"]`, so a drift to eight fails type-checking and the test. This is a preference, not a blocking duplicate.

### N2, the boundary test's `submit` helper duplicates `writeCommands`

`packages/core/src/boundary.wasm.test.ts:91` builds a `CommandWrite` that constructs `new Uint8Array(wasm.memory.buffer, ptr, payload.byteLength)`. `packages/core/src/bulk.ts:212` already provides `writeCommands(wasm, sink, bytes)` with the same body. `writeCommands` is imported at `boundary.wasm.test.ts:36` and unused by the new cases. Tests are in the ESLint wasm-view allow-list, so this is not a boundary violation. Using `writeCommands` would keep the view in `bulk.ts` and remove a wrapper that only forwards.

### N3, the typegen test has unreachable skip branches

`crates/ocelli-viewport/tests/typegen.rs:218` returns silently when `generated_path()` is `None`, and `:228` returns silently when reading the file fails after `assert!(actual.is_ok())`. The first branch is unreachable for an absolute `CARGO_MANIFEST_DIR`, and `generated_path` already asserts that `packages/core/src` is a directory. The second branch is unreachable after the assert. The plan says the test "refuses to run when the workspace root cannot be found, rather than skipping". The assert is the refusal. The two `else { return; }` arms read as skips to a later reader.

### N4, "field for field" is not literal for the field names

`crates/ocelli-viewport/src/boundary.rs:176` says "Section 17.4's readback, field for field" and `:196` says "Section 17.3's ring header, field for field". The byte layout is identical, but `RingHeader.capacity` is HLD `cap` and `ViewportState.padding` is HLD `_pad`. The `ViewportState` doc names the `_pad` rename at the field, the `RingHeader` doc does not. The layout is correct. The sentence is loose.

### N5, four generated exports have no consumer today

`STATE_PADDING_OFFSET`, `EVENT_RING_TAIL_OFFSET` and `EVENT_RING_CAPACITY_OFFSET` are emitted and published but no reader uses them. `RING_EVENT_KIND` is published for F-103 and has no consumer in this story. All four are listed in `packages/core/src/index.test.ts`, so they are checked for existence. The unused offsets document the full layouts. The plan's listing was illustrative, so the extra constants are a reasonable consequence of the one-definition invariant, not a defect.

### N6, `UPDATE_TYPES=0` regenerates

`crates/ocelli-viewport/tests/typegen.rs:222` tests `std::env::var_os("UPDATE_TYPES").is_some()`, so any value, including `0`, rewrites the committed file. The plan names `UPDATE_TYPES=1`. A value check would make the regeneration opt-in by value rather than by presence.

### N7, the shared `flag()` uses invert names for the flip bytes

`crates/ocelli-viewport/src/commands.rs:514` maps `INVERT_IDENTITY` and `INVERT_INVERSE` to `false` and `true`. `flag()` is used by `FlipHorizontal`, `FlipVertical` and `SetInvert`. The names describe a presentation shape, not a flip toggle. `packages/core/src/commands.ts:210` hard-codes `command.enabled ? 1 : 0` for the flips and does not use `INVERT_BYTE`. The values agree, so this is a naming preference. A `BOOL_BYTE` table, or a flip-specific constant, would keep the vocabulary honest.

### N8, the eight-byte command envelope is not generated

`crates/ocelli-viewport/src/commands.rs:432` uses `offset.checked_add(8)` for the envelope, and `packages/core/src/commands.ts:187` writes two `u16` values and one `u32`. The envelope size is a boundary number that `boundary.rs` does not own. The plan's "every boundary number" is broader than the command table. The HLD fixes the envelope at eight bytes, so this is a small gap rather than a defect.

### N9, the `RangeError` refusal is exercised but not asserted

`packages/core/src/commands.ts:183` throws a `RangeError` when `payloadOf`'s length disagrees with `COMMAND_PAYLOAD_BYTES`. It is a real refusal. Mutating `COMMAND_PAYLOAD_BYTES.SetWindow` to 21 made two `commands.test.ts` cases red with `RangeError: SetWindow payload is 17 bytes and the Rust table declares 21`, so the branch is reached when the table drifts. No test asserts the `RangeError` type or message. The function doc and the LLD do not mention the refusal. The drift failure mode changes from a Rust `BadLength` refusal to a synchronous shell throw. Documenting the refusal and asserting `toThrow(RangeError)` would make the guard's contract visible.

### N10, the plan's `bytemuck` justification names the wrong crate

`.claude/plans/F-104-design.md:314` says `bytemuck` "is already a workspace dependency in HLD section 15.2 and is used by `ocelli-core`'s record type". HLD section 15.2 does list `bytemuck`, and the workspace manifest declares it. The crates that use it are `ocelli-cache` (`RingEvent` is `Pod`) and `ocelli-render` (`cast_slice`), not `ocelli-core`. The dependency decision is correct and the implementation's `Cargo.toml` comment says only "Section 15.2 already lists `bytemuck`". The plan sentence is a false supporting clause in an approved plan.

## Verified clean

- **State layout**: `ViewportState` is `#[repr(C)]` and `Pod`, with `size_of` 96 and offsets 0, 64, 68, 72, 76, 80 and 84 for camera, centre, width, slice, count, flags and padding. The explicit `padding: [u32; 3]` leaves no implicit tail padding, so `bytemuck::bytes_of` writes exactly 96 bytes. `STATE_BYTES` is `size_of::<ViewportState>()`, and `write_state` refuses any other length with `StateLength`. The native `state_offsets_follow_section_17_4_and_bit_3_is_refining` test writes the HLD literals and reads the copied bytes. The boundary test reads the built module's copy through `readViewportState`.
- **Ring layout**: `RingHeader` is 16 bytes with head, tail, capacity and dropped at 0, 4, 8 and 12. `RingEvent` is 48 bytes with kind, viewport, seq and payload at 0, 4, 8 and 16. `EVENT_STRIDE` is `size_of::<RingEvent>()`, and `EVENT_PAYLOAD_BYTES` is `EVENT_STRIDE - EVENT_PAYLOAD_OFFSET`, which is 32. The derivation is safe because `payload` is the last field and the struct has no tail padding. The native `ring.rs` test pins 16 and 48 from the HLD and checks the header and the event bytes. `drainRing` handles a buffer shorter than the header and a declared count that exceeds the buffer, returning no events and the dropped count. `readEvent` adds the view's own `byteOffset` when it slices the payload.
- **Command layout**: `COMMAND_TABLE` is the eleven kinds in order with payloads 24, 16, 8, 8, 1, 1, 17, 1, 1, 1 and 4. `payload_len_of` is a lookup in `COMMAND_TABLE`, and the decoder reads it at `commands.rs:439`. The TS packer writes the generated id and the generated length. The independent hand-built byte fixture in `commands.test.ts` pins the HLD bytes. The built module applies all eleven kinds.
- **Enum bytes**: `VOI_FUNCTION_BYTES` is `0, 1, 2`, `PRESET_BYTES` is `0, 1, 2`, `COLORMAP_BYTES` is `0, 1` and `INVERT_BYTES` is `0, 1`. The decoder accepts each and refuses an outside byte. The generated maps agree with the hard-coded values in `commands.test.ts`. The boundary suite sends each generated byte through the built module.
- **Flags**: `FLAG_INVERT`, `FLAG_LOADING`, `FLAG_ERROR` and `FLAG_REFINING` are 1, 2, 4 and 8. `commands.rs` composes them, `commands.rs` test asserts the invert bit, and the session tests assert the refining bit.
- **Generated file**: `packages/core/src/generated.ts` is byte-identical to the generator output at the reviewed tree. The header is the two required lines, the "Do not edit" line is present, and there is no `SNAPSHOT_*` constant. The snapshot constants are correctly left to F-050.
- **Generated imports**: `viewport.ts`, `commands.ts` and `ring.ts` all import from `./generated.js`. `index.ts` re-exports the generated names and keeps the prior public names with the same values. `HEADER_BYTES` is `EVENT_HEADER_BYTES`, `EVENT_STRIDE` is the generated stride, `COMMAND_ID` is the generated object, `STACK_COMMANDS` is the generated tuple, and the flag and state constants are the generated values. `npm run test`'s published-surface assertion is green.
- **Test reality**: The typegen comparison is not self-referential. The AC2 mutation went red with the differing line, regeneration went green, and the revert returned both files to their pre-mutation bytes. The generated-offset mutation, the enum mutation and the payload mutation each went red in the reader that consumes the generated constant. The boundary suite is green over eleven tests against a module rebuilt at the current tree.
- **Gate coverage**: The `test` gate is `cargo test --workspace` and includes the typegen integration test. The `wasm` gate does not run the typegen test, and `test` is in the floor. The `types` gate runs `tsc --build`. The `unsafe`, `bindgen` and `pins` gates are green. The `nostd` gate is green with the new `bytemuck` dependency.
- **Boundary and tier**: No `wasm-bindgen` outside `ocelli-wasm`, confirmed by the `bindgen` gate. The new module is native and adds no boundary export. The generated constants describe state and command bytes, not pixels. The boundary test asserts the glue has no pixel getter. `readViewportState` and `drainRing` read copies, not wasm memory. The new test helper builds a view in a test file, which the ESLint rule allows. No render-loop allocation is added. The generator is a native test with no device or browser dependency, so it is tier-independent.
- **Structure**: No new trait, generic, `Box<dyn>` or `Arc<dyn>`. `CommandSpec`, `ViewportState` and `RingHeader` are data records, not extension points. The `boundary` module removes scattered literals and gives the reader one place to look. The `bytemuck` dependency is the existing workspace dependency, already used by `ocelli-cache` and `ocelli-render`, and the `nostd` gate confirms it does not pull `std` into `ocelli-viewport`. The generated constants beyond the plan's illustrative listing are a reasonable consequence of the one-definition invariant. The unused offsets and `RING_EVENT_KIND` are noted under N5.

## Limitations and handoff

- **AC9's presented-event half is unmet**. The public path is `publishCpuCanvas` at `crates/ocelli-wasm/src/canvas.rs:680`, not a missing `present` export. Node 24 has no `OffscreenCanvas` or `ImageData`, so the boundary test cannot exercise it without stubs. Owner: F-104 author. Either add a presented-event case with canvas stubs, or record the browser-only limitation and leave the criterion unmet for the browser CPU-canvas path. Do not add a redundant `present` export.
- **The three new files are untracked in this review tree**. The `prose` gate reads `git ls-files` and does not see them. They are source files outside the prose scope, so the prose rule is not the issue. The `unsafe`, `bindgen`, `content` and `provenance` gates also read `git ls-files`, so the author must stage `boundary.rs`, `typegen.rs` and `generated.ts` and re-run those gates before recording verification. The `test` gate did exercise the new `typegen.rs` because cargo tests the working tree, not the index.
- **The `wasm` gate and the sprint gate were not re-run**. The progress note's size receipt, 2,052,821 bytes against the 2,099,014 ceiling, is a prior receipt. The final `gate --sprint` profile is outside this working pass.
- **The boundary build was rebuilt for this pass**. The pre-existing `crates/ocelli-wasm/target/boundary-wasm` predated `boundary.rs` by 37 seconds, so it was rebuilt before the boundary suite ran. The boundary suite is green at the current tree, 11 tests.
- **The event copy is not covered end to end**. The generated event field offsets are checked against the HLD literals in `ring.test.ts` and the native `ring.rs` test, and the empty header is checked through the built module. No test copies a real presented event through the built module and reads it with `readEvent`. This is the same gap as AC9.
- **Endianness**: `write_state` and the ring copy use `bytemuck::bytes_of`, which writes native-endian bytes. The old code used `to_le_bytes`. On wasm32 and the native little-endian targets this is correct, and the boundary test reads the built module little-endian. A big-endian target would write bytes the TypeScript readers misread. No such target exists in the workspace, and the module doc claims the wire is little-endian on every target this ships to.
- **The concurrent workflow-engineering change is not reviewed**. It staged `.claude/WORKFLOW.md`, `.claude/commands/*.md`, `.agents/skills/*/SKILL.md`, `.opencode/commands/*.md` and `.claude/reviews/workflow-engineering-guidance-pass-1.md` during the pass. It is not part of F-104 and needs its own review.
- **The clean parts of this pass are bounded**. The arithmetic, the generated file, the three hand-written modules, the typegen guard and the built-module state and empty-ring cases are covered. The presented-event copy, the wasm size budget and the sprint profile are not. A clean bounded review is not completed acceptance and not sprint readiness.
