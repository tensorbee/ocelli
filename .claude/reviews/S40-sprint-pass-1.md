# S40 review, pass 1

**Reviewed**: `f1154912fa68ea4d8f917fa519656e3889cdb9ce..05c7691e0343b862b0be9a1aa434b25175c84ec6`, tree `a0c22f39b318937db07cc5c5bee0dbe2f41bd003`. Working tree clean before the pass and after every temporary mutation was restored. This review file is not part of that tree.
**Author / reviewer**: sprint/s40 / 01a1a140-d410-7150-a74d-4ecbeac31101 / independent pass 1. This reviewer did not write the sprint. The six commits are authored by Atul Sharma. This reviewer did not write them.
**Result**: 1 defect, 0 smells, 0 nitpicks. A defect blocks completion. Closing S40 does not complete M13 and does not publish. The version stays 0.1.0. D-22 still blocks publication. This pass does not clear the sprint.

## Acceptance challenged

F-112 says `openDecodePool` accepts an integer count from 1 to 8 and that a count outside that range returns null and does not start. The restored function does accept 8 and does refuse 0 and 9 without starting. The committed suite does not hold the inclusive ceiling. Replacing `count > MAX_DECODE_WORKERS` with `count >= MAX_DECODE_WORKERS` refuses 8, and `packages/core/src/pool.test.ts` stayed green.

The other challenged claims held on this tree. The decode resolve has no `wgpu` and no `naga`, and the default feature remains `render-worker`. Admission is `budget > 0 && bytes <= budget` in one function. A 65536 by 65536 load is 4294967296 bytes here, and `fits_address_bits` returns nothing for that product at 32 bits. `certifyRow` returns null for SwiftShader with expected tier B when the exit code is 2 and the resolved tier is A. Tier C without that substring stays observed. The committed matrix has no observation and no certified row.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git status --porcelain` | `05c7691`, tree `a0c22f39` | executed now | HEAD matched. Porcelain empty at start and after restore. |
| `python3 scripts/decode_graph_check.py` | manifests and `cargo tree` for wasm32, decode-worker and the default render graph | executed now | exit 0. `OK: decode build has no wgpu or naga, render build keeps wgpu, wasm-opt stays on` |
| `npx vitest run` pool, matrix, and index tests | `pool.ts`, `matrix.ts`, `index.ts` on the restored tree | executed now | exit 0. 23 tests passed. |
| `bin/ocelli.sh test ocelli-cache profile` | `profile.rs` on the restored tree, 64-bit `usize` | executed now | exit 0. 7 profile tests passed. |
| Pool operator mutation, then the same pool vitest, then restore | `count >= MAX_DECODE_WORKERS` | executed now | exit 0. 8 tests passed while count 8 was refused. |
| Node strip-types call on the restored pool and matrix | count 8, counts 0 and 9, SwiftShader expected B with exit 2 and resolved tier A, llvmpipe expected C | executed now | count 8 started indexes 0 through 7. Bad counts did not start. SwiftShader case returned null. llvmpipe returned observed. |
| `bytes <= budget` changed to `bytes < budget`, two cache tests, then restore | equal budget on the large volume and `would_admit` | executed now | both exit 101. Large volume failed `decoded_admitted`. Boundary test got false against true. |
| `product < limit` changed to `product <= limit`, then restore | `fits_address_bits(4294967296, 32)` | executed now | exit 101. `is_none` failed. |
| SwiftShader null narrowed to exit 0 and equal tiers, then restore | `matrix.test.ts` | executed now | exit 1. The exit 1 case and the differing-tier case returned observed instead of null. |
| On-disk wasm sizes, not rebuilt by this pass | `pkg/ocelli_wasm_bg.wasm` and `pkg-decode/ocelli_wasm_bg.wasm`, mtime 11:49 UTC | inspected now | 2171133 and 16455 bytes. Decode bytes contain neither `wgpu`, `naga`, nor `attachOffscreenCanvas`. Render `.d.ts` exports `attachOffscreenCanvas`. |
| Feature profiles named in `AS_BUILT.md` | per-story floor plus corpus receipts | prior receipt | not re-run. No sprint profile exists for this HEAD. |

## Defects

### D1, count 8 is accepted by the function and not by the tests

**Where**: `packages/core/src/pool.ts:47` and `packages/core/src/pool.test.ts:36`
**What**: The function accepts an integer count from 1 to 8. The committed tests never open a pool of 8. Changing the ceiling comparison from `count > MAX_DECODE_WORKERS` to `count >= MAX_DECODE_WORKERS` refuses 8 and does not start a worker. The pool suite still passed, 8 tests, exit 0. `MAX_DECODE_WORKERS` stayed 8, and the refusal of 9 stayed covered, so neither assertion noticed.
**Why it is wrong**: `.claude/plans/F-112-design.md` says `count` must be an integer from 1 to 8. The sprint claim is the same. A suite that stays green when 8 is refused does not hold that claim. HLD section 9 leaves N to the caller and this plan is the local ceiling.
**Evidence**: The mutated comparison was run with `npx vitest run packages/core/src/pool.test.ts` and exited 0. After restore, a direct call with count 8 returned a pool of count 8 and starter indexes 0, 1, 2, 3, 4, 5, 6 and 7. Counts 0 and 9 returned null and did not start. `git diff` was empty after the restore.
**Scope and regression**: Present on this HEAD as a test gap. The function on this tree already accepts 8. Affected caller is any shell that asks for the documented maximum. Smallest regression is one assertion that count 8 returns a pool and that the starter is called once per index from 0 through 7, plus the same run after replacing `>` with `>=`, which must go red. The lower bound is already locked. Count 0 returns null in the committed test.

## Smells

None.

## Nitpicks

None.

## Verified clean

Closing S40 does not complete M13 and does not publish. `Cargo.toml` and `packages/core/src/index.ts` keep version 0.1.0. D-22 remains the publication block. The backlog keeps F-X003 `in-progress`. F-117 stays pending on S41 and is not in the S40 table. Tracker rows for F-110, F-112, F-113 and F-114 say `not measured`. They do not state a ratio and they do not claim a sprint receipt. Backlog and current-sprint status cells for those four stories say `done`. F-042, F-035 and F-034 are `done`.

The render budget file `ci/wasm-size-budget.json` is not in the sprint diff. It still records 2151196 bytes and tolerance 0.05. The ceiling used by the size check is 2258755. The on-disk render module is 2171133 bytes, inside that ceiling. `wasm-opt` is still a non-empty flag list. `default` is `render-worker` only. `decode-worker` enables nothing. `bin/ocelli.sh wasm-decode` passes `--no-default-features --features decode-worker`. The decode `.d.ts` exports `install_panic_hook`, `ocelli_version`, `panic_record_len` and `panic_record_ptr`, and it does not name `attachOffscreenCanvas`. The graph check on this tree agreed. The artefacts were built at 11:49 UTC, after the F-113 commit and before the F-114 commit. F-114 does not change the wasm graph.

`admits` is `budget > 0 && bytes <= budget`. `Lru::would_admit` and `profile_load` both call it. Encoded used is 0. The hand products 157286400, 262144, 4294967296 and 786432 match the dimensions. Tier C with a non-zero GPU figure returns nothing. An equal budget is admitted, and one byte under it is not. Both of those tests went red when the comparison lost the equal case. The 32-bit address test went red when the limit comparison became inclusive.

`certifyRow` ignores a caller status. A missing observation is `not-run`. Tier B is certified only when the renderer string does not contain `SwiftShader`, the exit code is 0, the tiers match, and the string is non-empty. Tier C is certified only when the tiers match, the exit code is 0, and the string does contain `SwiftShader`. SwiftShader with expected tier B returns null before the exit code and the resolved tier are consulted. Narrowing that arm left the two committed tests red. `ci/browser-matrix.json` has five rows, every status is `not-run`, and it has no observation and no `certified` text. The function does not launch a browser. No browser probe output was added in the diff.

`pool.ts` does not contain the identifier `SharedArrayBuffer`. `threadingEscalation` returns `stable`. `sharedMemory` is false. A non-object owner, a second open, and a non-integer count return null before `start`.

The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. The cache F-ID header equals the last cell of the cache row. The only new deviation row is D-27, which the approved F-110 plan adds. D-14 was updated to point at that row. D-07 was not edited. New D-07 citations are the F-114 plan, `matrix.ts` and the F-114 as-built entry. New D-27 citations belong to the F-110 split. The diff adds no `unsafe` and no `wasm-bindgen` outside `ocelli-wasm`. The new fixtures are synthetic dimensions and browser-row names. They are not patient data.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, `wasm-pack`, or a browser probe. It did not rebuild the wasm artefacts. It inspected the modules already on disk and resolved the graphs with `cargo tree`. Feature-profile sentences in `AS_BUILT.md` are prior receipts, not a sprint receipt, and they were not re-run. No sprint profile has been recorded for this HEAD. Closing S40 does not complete M13 and does not publish.

The pool mutation, the admission mutation, the address-limit mutation and the SwiftShader mutation were restored. `git status --porcelain` was empty afterwards. The next owner should add the count 8 assertion, then a fresh independent pass should review that test on the new tree. Do not mark F-X003 done. Do not treat these feature receipts as a sprint receipt.
