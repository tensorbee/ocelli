# S35 review, pass 2

**Reviewed**: Range `c3f85e51a6c5e0a7e71ae9ed17b90faf2dcc6b19..HEAD` plus the uncommitted working tree on `sprint/s35`. HEAD is `2ac7844150c0f074d62321e2d9bd5bb7615f1f2f`. The feature tree before the doc fix is `d7d75e3f7d42d36496feb0125e4924b9c12a0597`. This pass did not change HEAD.

**Reviewer**: Independent subagent `01a19edd-5bc2-71e3-bd0b-3a5a59df532b`. This reviewer did not write the S35 code, the designs, the story reviews, or the sprint pass 1 review.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

D1 from pass 1 is the F-095 completion note. The working tree sentence in `docs/sprints/AS_BUILT.md` line 4477 is now: "A blank-id length of 2 stored the empty id. The one-character ids still passed. The bytes were restored."

That sentence no longer says a length of 2 refused a one-character id. `git grep` for `refused a one-character` found no match in tracked working-tree files. A workspace search finds the phrase only in the untracked pass 1 review, where it is the defect record. That file was not edited. The index and HEAD still hold the old sentence inside `docs/sprints/AS_BUILT.md`. That is the uncommitted fix under review, not a second claim.

The new sentence has no em-dash and no prose semicolon. A character scan of those three sentences found no semicolon character, no em-dash, no en-dash, and no code point above 127.

This pass saved `packages/core/src/annotation-state.ts`, then changed the one `id.length === 0` comparison to `id.length === 2`. No other product file was edited. `npm test -- packages/core/src/annotation-state.test.ts` exited 1. Vitest 4.1.11 reported 1 failed and 4 passed. The failure is `refuses a blank id and a tree the content tree refuses` at line 111. The assertion expected null. The received state stored one record whose id is empty. The four passing tests use the one-character ids `a`, `b`, and `c`. The runner stopped at line 111, so the later expects in that same test were not reached. Those later expects are not the red row. The red row is the empty id.

The saved blob was copied back. `cmp` exited 0. The annotation-state sha256 returned to `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`. During the mutation that hash was `f6594a8c367d5fe23d2f711b55bec4ab79337dc014b89c04e31c4c3b34f8974a`. The restored source again has `if (id.length === 0 || frameOfReference.length === 0)` and returns null. An empty id is refused. `id.length === 2` is gone.

The other three product hashes were the same before the edit and after the restore:

- `packages/core/src/sr.ts` `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8`
- `crates/ocelli-seg/src/seg_roundtrip.rs` `8dc88e3aed9b232509105b9c01aeb8a721d7a8c760fa8fa82fe01cd330848679`
- `crates/ocelli-seg/src/contour.rs` `997c27cb9a7ad6476ee60bd24acc744df42cf27191a500b7a7fe314889877ab3`

The corrected note matches this run. A length of 2 stored the empty id. The one-character ids still passed. The bytes were restored. D1 is fixed. No new defect or smell was found.

Closing S35 does not complete M11 and does not publish. F-096 and F-097 stay in S36.

## Evidence

| Command | Identity | Executed now | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git status --porcelain`, four `shasum -a 256` | Before the mutation | Yes | Exit 0. HEAD `2ac7844150c0f074d62321e2d9bd5bb7615f1f2f`. Status was the AS_BUILT edit and untracked pass 1. All four hashes matched the values above |
| `git diff -- docs/sprints/AS_BUILT.md` | The only intended product edit | Yes | One sentence at line 4477. The old refusal claim is gone |
| `git grep` for `refused a one-character` | Tracked working tree | Yes | No match |
| Blob copy, then `id.length === 0` changed to `id.length === 2` | Only `annotation-state.ts` | Yes | One occurrence. Diff is that comparison only. Mutated hash `f6594a8c367d5fe23d2f711b55bec4ab79337dc014b89c04e31c4c3b34f8974a` |
| `npm test -- packages/core/src/annotation-state.test.ts` | Mutated blank-id check | Yes | Exit 1. Vitest 4.1.11. Line 111 expected null and received a stored empty id. 4 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `annotation-state.ts` | Yes | `cmp` exit 0. Hash returned to `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`. The other three hashes still match |
| Restored `putAnnotation` source | Empty id | Yes | `id.length === 0` returns null. `id.length === 2` is absent |
| Sentence scan of the three new sentences | Em-dash and prose semicolon | Yes | None |
| `git rev-parse HEAD` and `git status --porcelain` | After the restore and after this file | Yes | HEAD unchanged. Status is the AS_BUILT edit plus the two untracked review files. No product file is modified |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The working-tree note states the outcome this pass measured. The empty id was stored under the length-2 comparison. The tests that use `a`, `b`, and `c` stayed green. The restored function refuses an empty id. The four product hashes match the required values. This pass does not block.

## Limitations and handoff

The other three pass 1 mutations were not repeated. The sprint gate was not run. No wasm-pack, oracle, or cargo command was started. This pass did not stage, commit, push, or record a sprint verification. The AS_BUILT sentence is left in the working tree. F-096 and F-097 stay in S36. Closing S35 does not complete M11 and does not publish.

This file is untracked until the parent stages it.

dwv, Horos, and the medical viewer Grok were not opened.
