# F-104 review, pass 3

**Reviewed**: HEAD `8b9eb65` (`S17, approve sprint designs`) plus the F-104 working-tree diff. At the start of this pass the unstaged tracked diff was 542 insertions and 256 deletions over 15 paths, with three untracked F-104 files. During the pass a concurrent process staged the whole F-104 tree, so the reviewed bytes are now the index content: 18 F-104 paths, 1215 insertions and 256 deletions. The new material is `packages/core/src/boundary.wasm.test.ts` (`sha256 dbb87b24305818a9c24219fef33017a696ab69a113cbe10e4c47a3fd6eebea57`), the untracked `.claude/scratch/F-104-progress.md` (`sha256 15519f1e8eb039fe15517635e845b01cae3fdf3679313e41ded046f03f999df0`) and the unchanged `crates/ocelli-wasm/src/session.rs` (`sha256 30b873ad71aa1acfd39a9d369b38bbff277e13721e715cccba83d214d7511675`). The other F-104 paths are byte-identical to pass 2's reviewed tree: `crates/ocelli-viewport/src/boundary.rs` `sha256 facf5ca3177bf583ff35096be193de9757ff125e1a127bd51d2c874a3f4668c1`, `crates/ocelli-viewport/tests/typegen.rs` `sha256 f0f0e2981cb8c0f4f4e0416c99d7d51a51a4d36c825b071db55d82dfddd8768b` and `packages/core/src/generated.ts` `sha256 9810a82c1ca0d685a1959e9a64c3ab8d9cbedb5bd9601a97bb37e63771c71946`.

**Author / reviewer**: The author is the F-104 implementer, whose handoff is `.claude/scratch/F-104-progress.md`. The reviewer is an independent subagent that did not author the change, the remediation or the handoff. A prior pass-3 record was already present at this path when this pass began (mtime 02:25). This reviewer did not write it and cannot establish its author. It is preserved verbatim in the appendix so that the record is not lost. The reviewer has no authorship in the changed material.

**Result**: 0 defects, 0 smells, 3 nitpicks.

## Acceptance challenged

- **AC9's presented-event half was executed, not read.** `packages/core/src/boundary.wasm.test.ts` commits a 1 by 1 frame, binds a stub `OffscreenCanvas` at 64 by 64, calls `publishCpuCanvas`, and drains one event. The test asserts `ring.byteLength` equals `EVENT_HEADER_BYTES + EVENT_STRIDE` (64), one event, dropped 0, kind `RING_EVENT_KIND.presented` (6), viewport 1, seq `0n`, and payload words 64, 64 and 0. An independent probe through the glue wrapper, not the test's assertions, observed kind 6, viewport 1, seq 0, generation 0, drawn width 64, drawn height 64, flags 0 and a zero payload tail. The observation rejects a stride or header that reads a neighbour, a producer that writes another kind, and a settle that leaves the refining flag set.
- **AC9's full-ring half was executed, not read.** The test commits one frame, binds the stub canvas, calls `publishCpuInteraction` 300 times, and drains. It asserts `ring.byteLength` equals `EVENT_HEADER_BYTES + 256 * EVENT_STRIDE` (12304), 256 events, dropped 44, and `events[255].seq === 255n`. The probe observed pending 256, tail 0, capacity 256, dropped 44, first seq 0, last seq 255, last kind 6, last generation 256, last drawn extent 32 by 32 and last flags 8, the refining bit. The 300 pushes, 256 kept and 44 dropped match `EventRing::push`, which refuses a new event when `pending == RING_CAPACITY` and increments `dropped`. A ring that dropped the oldest event would keep seq 44 to 299 and fail the last-seq assertion.
- **The capacity term is load-bearing.** `drain_events` was changed to allocate `EVENT_HEADER_BYTES + 40 * EVENT_STRIDE`, that is 1936 bytes, instead of header plus 256 times stride. The dev module was rebuilt. `drains a full ring at the generated capacity and stride` went red with `AssertionError: expected +0 to be 12304` at line 577, exit 1. The presented-event case stayed green, because 1936 bytes is enough for one event, which shows that the full-ring case is the one that guards the capacity term. Reverting, rebuilding and re-running gave 12 passed, exit 0.
- **Sequence arithmetic, derived independently.** `EventRing::new` starts `seq` at 0. `push_presented` assigns the current `seq` before incrementing. Three hundred pushes with capacity 256 keep the first 256 events, so the last kept sequence is 255 and the dropped count is 44. The test's numbers are that derivation, not a copy of an implementation readback.
- **Pass 2 D1 is closed.** The progress record no longer says "unmet in Node" or "browser-only". The AC9 row is `evidenced`, names both new tests, and says the stubs are test-only and are not a browser blit observation. Every clause is true.
- **Pass 2 S1 is closed.** The full-ring case makes the `capacity * EVENT_STRIDE` allocation term observable, and the mutation above shows it goes red when the term is too small. Pass 1's D1 and S1 remain closed: `copy_state` and `drain_events` still derive their sizes from `STATE_BYTES`, `EVENT_HEADER_BYTES`, `RING_CAPACITY` and `EVENT_STRIDE` at `session.rs:1041` and `session.rs:1085`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --porcelain`, `git diff --stat`, `shasum -a 256` | HEAD `8b9eb65`, the F-104 diff and the untracked progress note | Executed now | 15 tracked paths, 542 insertions and 256 deletions at the start, 3 untracked F-104 files at pass 2's hashes, empty index. A concurrent process staged the tree mid-pass |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | A dev module at the reviewed tree | Executed now | Exit 0, rebuilt before the suite and after each mutation and revert |
| `npx vitest run --config vitest.boundary.config.ts` | `packages/core/src/boundary.wasm.test.ts` over the built module | Executed now | 12 passed, exit 0, before and after the mutation |
| `bin/ocelli.sh test ocelli-wasm` | The crate's native unit tests | Executed now | 31 passed, exit 0 |
| `bin/ocelli.sh clippy ocelli-wasm` | Host target only, so it does not compile the `cfg(target_arch = "wasm32")` module | Executed now | Exit 0 |
| `cargo clippy -p ocelli-wasm --target wasm32-unknown-unknown --all-targets --no-deps -- -D warnings` | The changed module, linted for wasm32 | Executed now | Exit 0 |
| `bin/ocelli.sh gate fmt clippy test nostd` | The four named gates | Executed now | ALL GREEN, 4 gates, exit 0. `nostd` reports 7 no_std crates. The viewport typegen and ring tests are in the `test` gate |
| `npm run typecheck` | `tsc --build` | Executed now | Exit 0 |
| `npm run lint` | ESLint, including the wasm-view ban | Executed now | Exit 0 |
| `python3 scripts/prose_check.py` | Voice rules over the index | Executed now | OK over 479 files before the concurrent staging and 482 after it, exit 0 |
| Probe `f104-pass3-probe.mjs`, 31 checks | The built module through the glue wrappers `bindCpuCanvas`, `publishCpuCanvas`, `publishCpuInteraction` and `drain_events` | Executed now, file outside the tree | PROBE GREEN, exit 0, one event at 64 bytes and a full ring at 12304 bytes |
| Mutation, `drain_events` capacity `40_usize` | `crates/ocelli-wasm/src/session.rs:1085` | Executed now | Boundary suite 1 failed and 11 passed, exit 1, `expected +0 to be 12304`. The presented-event case stayed green |
| Restore and green re-run | `sha256` of `session.rs` `30b873ad...`, rebuilt module | Executed now | 12 passed, exit 0, `git diff` empty over the staged tree |
| `bin/ocelli.sh gate wasm` | Release size budget | Prior receipt only | Not re-run. The progress note records 2,052,821 bytes against 2,099,014 |
| `bin/ocelli.sh gate --sprint` | The sprint profile | Not run | Outside a working pass |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, `copy_state`'s doc comment still restates 96

`crates/ocelli-wasm/src/session.rs:411` says "Copy the 96-byte readback. Returns 96". The wrapper itself uses `STATE_BYTES`. The sentence is true today and will drift when `ViewportState` changes. Carried from pass 2. It does not block.

### N2, `drain_events` uses qualified paths

`crates/ocelli-wasm/src/session.rs:1085` spells `ocelli_viewport::ring::RING_CAPACITY` and the boundary constants in full, while `STATE_BYTES` is imported by name. A `use` at the module top would read like the rest of the file. Carried from pass 2. Preference only.

### N3, the presented-event case commits before binding the canvas, so the event generation is 0

`packages/core/src/boundary.wasm.test.ts:532` calls `writeFrame` with `META.generation` 1 before `bindCpuCanvas` at line 540. `bindCpuCanvas` calls `Session::bind_cpu`, which opens the raster at generation 0 and `sync_cpu_buffer` sets the viewport generation to 0. The presented event therefore carries generation 0, not the committed 1. The test asserts drawn width, height and flags but not the generation word. The native `state_and_presented_events_are_copies` test asserts generation 9 in the no-raster path, and `crates/ocelli-viewport/tests/ring.rs` pins the payload layout, so this is a coverage preference rather than a wrong observation. Binding the canvas before the commit and asserting the generation word would exercise the raster-bound generation at the boundary. The full-ring case has the same order and its last event carries generation 256.

## Verified clean

- **The presented-event test produces kind 6 through the public path.** The test calls the wasm export named `publishCpuCanvas`, which reaches `publish_cpu_frame` and `Session::present`, and `Session::present` calls `ring.push_presented` with `KIND_PRESENTED`. The independent probe drove the glue wrapper `publishCpuCanvas`, which throws on a refused `Result`, and it returned without error. A refused blit returns before the push, so the presence of the event also proves the blit path returned `Ok`.
- **The stubs are test-only.** `StubImageData`, `StubContext` and `StubCanvas` are classes inside `packages/core/src/boundary.wasm.test.ts`. `installCanvasStubs` writes only `globalThis.ImageData`, `globalThis.OffscreenCanvas` and `globalThis.OffscreenCanvasRenderingContext2D`. No production export, no wasm-bindgen import and no runtime dependency is added. The stub `ImageData` carries a real `Uint8ClampedArray`, and `copy_presented` copies the real Rust presentation buffer into it. `putImageData` is a no-op, so only the DOM blit is unobserved, not the event or the copied bytes.
- **The assertions are not weakened.** The payload is the copy `readEvent` returns from `view.buffer.slice`, and the DataView is over that copy, not over wasm memory. The test's `ring.byteLength` assertion is the returned copy length, so a too-small allocation returns 0 and goes red. The full-ring test's `EVENT_HEADER_BYTES + 256 * EVENT_STRIDE` is 16 plus 12288, which matches `RING_CAPACITY` 256 and `EVENT_STRIDE` 48.
- **Boundary and tier.** No view over wasm memory is added outside `packages/core/src/bulk.ts`. The `submit` helper in the test builds a view, and `npm run lint` is green, so the test file is inside the existing wasm-view allow-list. No per-event callback, no allocation in a render loop, no new dependency and no `wasm-bindgen` outside `ocelli-wasm`. The canvas path retains pixels in the session and blits to the stub, and the ring carries metadata, so no pixels cross the boundary.
- **Structure.** The change adds test cases and test-only stub classes. No new trait, generic, `Box<dyn>`, wrapper or feature flag. The reader still finds the boundary numbers in `crates/ocelli-viewport/src/boundary.rs` and the generated file.
- **Progress note.** The AC9 row is `evidenced`, names `publishes a presented event and drains it at the generated stride` and `drains a full ring at the generated capacity and stride`, and says the stubs are test-only and not a browser blit observation. The "What landed" sentence and the "Commands run now" entry for 12 tests match the executed tree. The note does not claim a browser blit.
- **Prior findings.** Pass 2 D1 is closed by the corrected AC9 record and the two new cases. Pass 2 S1 is closed by the full-ring case and the capacity mutation. Pass 1 D1 and S1 are closed by the `session.rs` hash `30b873ad...` and the derived allocation. Pass 1 D2 is closed by the record and the tests.
- **Tree restored.** `crates/ocelli-wasm/src/session.rs` is back at `sha256 30b873ad71aa1acfd39a9d369b38bbff277e13721e715cccba83d214d7511675`. The working tree has no unstaged F-104 diff against the staged content. The temporary mutation was the only tracked-source edit, and it is reverted.

## Limitations and handoff

- **What this pass did not exercise.** The real browser canvas blit. `putImageData` was a no-op stub, so the DOM side of `publishCpuCanvas` is not observed. Neither are the WebGPU and WebGL2 tiers. The release size gate (`bin/ocelli.sh gate wasm`) and `bin/ocelli.sh gate --sprint` were not run. The progress note's size receipt is prior evidence. A clean bounded review is not completed acceptance and not sprint readiness.
- **A concurrent process staged the tree during the pass.** It staged the 18 F-104 paths, added `CHANGELOG.md`, `docs/sprints/AS_BUILT.md`, `docs/sprints/BACKLOG.md`, `docs/sprints/CURRENT_SPRINT.md` and `docs/sprints/SPRINT_TRACKER.md`, and staged a prior pass-3 record. Those sprint and changelog changes are not F-104 boundary material and are not reviewed here. The `unsafe`, `bindgen`, `content` and `provenance` gates read `git ls-files`, so the author must re-run them over the staged tree before recording verification. This pass did not run them.
- **The prior pass-3 record is not this reviewer's work.** It was present before this pass and is preserved in the appendix. Its result was reached without the floor gates. This review's result is independent of it.
- **AC9's generation word is not asserted at the boundary.** See N3. The native tests pin the payload layout and the no-raster generation value, so this is a coverage preference, not an unmet criterion.

## Appendix, prior pass-3 record found in the tree, preserved verbatim

This record was present at this path before this review began. It is not the work of this reviewer. It is reproduced unchanged so that the record is not lost.

# F-104 review, pass 3

**Reviewed**: HEAD `8b9eb65` plus the uncommitted F-104 diff. Tracked diff is 542 insertions and 256 deletions over 15 paths. Untracked files are `crates/ocelli-viewport/src/boundary.rs` (`sha256 facf5ca3177bf583ff35096be193de9757ff125e1a127bd51d2c874a3f4668c1`), `crates/ocelli-viewport/tests/typegen.rs` (`sha256 f0f0e2981cb8c0f4f4e0416c99d7d51a51a4d36c825b071db55d82dfddd8768b`) and `packages/core/src/generated.ts` (`sha256 9810a82c1ca0d685a1959e9a64c3ab8d9cbedb5bd9601a97bb37e63771c71946`). `crates/ocelli-wasm/src/session.rs` is `sha256 30b873ad71aa1acfd39a9d369b38bbff277e13721e715cccba83d214d7511675`. `packages/core/src/boundary.wasm.test.ts` is `sha256 dbb87b24305818a9c24219fef33017a696ab69a113cbe10e4c47a3fd6eebea57`. This pass reviews the remediation after pass 2. The new material is the presented-event case, the full-ring case, and the acceptance record that cites them.

**Author / reviewer**: The author is the F-104 implementer. The reviewer did not write the implementation. The reviewer's own check in this pass is the stride mutation below.

**Result**: 0 defects, 0 smells, 2 nitpicks.

## Acceptance challenged

- **Pass 2 D1 is closed.** The progress record no longer calls the presented event browser-only. `packages/core/src/boundary.wasm.test.ts` publishes through `publishCpuCanvas` with canvas stubs and drains one kind-6 event at `EVENT_HEADER_BYTES + EVENT_STRIDE`.
- **Pass 2 S1 is closed by execution.** `drain_events` was changed so the capacity term multiplied `40_usize` instead of `EVENT_STRIDE`. The dev module was rebuilt. `drains a full ring at the generated capacity and stride` went red with `expected 12304 to be 0`. The source was restored to `sha256 30b873ad...` and the module was rebuilt. The boundary suite then passed, 12 tests.
- **Sequence arithmetic.** `EventRing::new` starts `seq` at 0 and `push_presented` assigns the current value before incrementing. Three hundred interaction publishes keep 256 events and drop 44. The last kept sequence is 255. The full-ring test asserts that value, the length `16 + 256 * 48`, and dropped 44. The green suite matches that derivation.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run --config vitest.boundary.config.ts` | Rebuilt dev module after the restored `session.rs` | Executed now | 12 passed, exit 0 |
| Stride mutation, `EVENT_STRIDE` factor replaced by `40_usize` | `crates/ocelli-wasm/src/session.rs` `drain_events` | Executed now | Full-ring test red, `expected 12304 to be 0`, exit 1. Source restored byte for byte |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | Restored source | Executed now | Done, exit 0 |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, `copy_state`'s doc comment still restates 96

`crates/ocelli-wasm/src/session.rs:411` says "Copy the 96-byte readback. Returns 96". The wrapper itself uses `STATE_BYTES`. The sentence is true today. Carried from pass 2. It does not block.

### N2, `drain_events` uses qualified paths

`crates/ocelli-wasm/src/session.rs:1085` spells `ocelli_viewport::ring::RING_CAPACITY` and the boundary constants in full, while `STATE_BYTES` is imported by name. Carried from pass 2. Preference only.

## Verified clean

- Pass 2's blocking findings are closed on this tree. The presented event and the full ring are in the boundary suite, and the suite is green after the mutation was reverted.
- `push_presented` writes generation, width, height and flags as little-endian `u32` at payload offsets 0, 4, 8 and 12. The presented-event test reads width, height and flags at 4, 8 and 12.
- The three untracked generator files are the same bytes pass 2 reviewed. This pass did not reopen that material.

## Limitations and handoff

- The floor gates, the wasm size gate and `gate --sprint` were not re-run in this pass. The progress note's earlier receipts are prior evidence.
- The canvas stubs do not observe a browser blit. AC9 asks for the copied ring event, which this suite now runs.
- `boundary.rs`, `typegen.rs` and `generated.ts` are still untracked. Completion has to stage them before the content gates.
- A clean working pass is not the feature verify receipt.
