# F-116 review, pass 1

**Reviewed**: Uncommitted worktree on `sprint/s41`. HEAD is `8dc944f54ac73809bce0054ccde58fd644d13979`. `packages/core/src/playbook.ts`, `packages/core/src/playbook.test.ts` and `docs/lld/playbook.md` are untracked. The other F-116 edits are unstaged. This file is untracked. Nothing was staged or committed.

**Author / reviewer**: uncommitted implementation / 01a1a18b-566b-7e63-9154-380d74232b59 / independent pass 1. This reviewer did not write the change.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC6 were checked against `.claude/plans/F-116-design.md` and HLD section 12 in `docs/hld/09-migration.md`. Section 12 names the order and does not give a function. It does not say what a skipped step returns. The prefix rule is the approved local decision. Stack, then MPR, then volume is that order. The kind name for the third step is volume.

The unit file locks these cases.

- An empty list returns stack.
- stack returns mpr.
- stack then mpr returns volume.
- stack then mpr then volume returns null.
- mpr, volume, stack then volume, stack then stack, and oblique return null.
- `docs/lld/playbook.md` contains the sentence "The order is stack, then mpr, then volume."

`index.ts` exports `nextMigrationKind`. The published name list includes it. `index.test.ts` compares that list with the module keys.

An independent probe was run on the restored function and was not left in the tree. It also checked cases the unit file does not name.

- mpr then stack returns null. volume then mpr then stack returns null. mpr then volume returns null.
- A fourth entry after the finished list returns null. stack then mpr then mpr returns null.
- Stack, MPR and Volume with a capital letter return null. The string 3d returns null. An empty string and a leading space return null.
- Each input was frozen. The function returned the expected kind and did not write the array.
- The index binding is the same function.
- After those calls, the stack, mpr and volume flags were still off.
- The five worked lines in `docs/lld/playbook.md` are present and match the function.

The first element of `ORDER` was changed from stack to mpr, so the constant read mpr, stack, volume. `npx vitest run packages/core/src/playbook.test.ts` then failed three tests. Exit 1. `starts at stack` expected stack and received mpr at `playbook.test.ts:17`. `walks stack, then mpr, then volume` expected mpr and received null at `playbook.test.ts:21`. `returns null for a skip, a duplicate or an unknown name` expected null and received stack at `playbook.test.ts:27`. The doc test stayed green. The file was restored to the pre-mutation bytes. sha256 `e7d15807afecd936667e654c4bb1c959901d1ad51f00dcaf362b2205865052a1` matched before the edit and after the restore. A byte compare against the saved original was empty. Length 745. The restored suite exited 0.

## The second kind list

Acceptable. Not a defect and not a smell.

`packages/core/src/flag.ts` line 14 and `packages/core/src/playbook.ts` line 11 both read `["stack", "mpr", "volume"]`. The text is the same. playbook.ts imports `ViewportKind` as a type only. An esbuild emit of that module has a local `ORDER` and no import. The function does not load the flag module and cannot flip a flag. The probe left every kind flag off. The flag constant stays private. This story does not edit `flag.ts`, so it does not export that constant for reuse. The playbook array is typed as `ViewportKind`, so a literal outside those three names does not typecheck there. The function argument stays `readonly string[]`, which is what lets an unknown name reach the null return. The second list is the isolation between a pure prefix check and the flag module. The lists match today. Changing the playbook constant fails `playbook.test.ts`, which this pass showed. That is not wrong behaviour and it does not by itself produce a later defect.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git status --short` and `git rev-parse HEAD` | Worktree before this review file | Executed now | HEAD `8dc944f54ac73809bce0054ccde58fd644d13979` on `sprint/s41`. Five modified tracked files and three untracked product files. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row | Worktree text | Executed now | The strings are equal. Both have 48 ids. Both end with F-116. Length 335. |
| `npx vitest run packages/core/src/playbook.test.ts` | Original `playbook.ts` sha256 `e7d15807afecd936667e654c4bb1c959901d1ad51f00dcaf362b2205865052a1` | Executed now, before the mutation | Exit 0. 4 passed. |
| Same command after `ORDER` started with mpr | Mutated worktree, not staged | Executed now | Exit 1. 3 failed and 1 passed. Failures at `playbook.test.ts:17`, `playbook.test.ts:21` and `playbook.test.ts:27`. |
| `cmp` and sha256 after copy-back | Restored bytes, length 745 | Executed now | Compare empty. sha256 `e7d15807afecd936667e654c4bb1c959901d1ad51f00dcaf362b2205865052a1`. |
| Probe plus `playbook.test.ts` and `index.test.ts` | Restored tree. Probe deleted after the run. | Executed now | Exit 0. 11 passed. 4 in `playbook.test.ts`, 4 in `index.test.ts` and 3 in the probe. |
| esbuild of `playbook.ts` | Restored source | Executed now | Exit 0. The emit has a local `ORDER` and no import. |
| `npx tsc -p packages/core --noEmit --pretty false` | Worktree sources after the restore | Executed now | Exit 0. |
| `npx eslint` on `playbook.ts`, `playbook.test.ts`, `index.ts` and `index.test.ts` | Those four files, after the restore | Executed now | Exit 0. ESLint 9.39.5. |
| Voice scan of `playbook.md`, `public-api.md` and `docs/lld/README.md` | Including the untracked playbook page | Executed now | No em dash and no prose semicolon. |

Environment: Node v24.13.0. Vitest 4.1.11. No cargo, rustc or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- AC1. An empty list returns stack. Changing the first kind to mpr went red on that assertion.
- AC2. stack returns mpr. stack then mpr returns volume. The same mutation went red because stack was no longer a prefix.
- AC3. The finished list returns null. The green run locked it. The probe locked it again.
- AC4. A skip, a duplicate and an unknown name return null. The mutation made mpr return stack, and the skip assertion went red. The probe also refused a reorder, a list longer than three, a repeated mpr, a capitalised name, a blank string and a padded name. The function does not repair the list.
- AC5. The playbook names stack, then mpr, then volume. The worked calls match the function. A finished null is described as finished. A skip, a duplicate or an unknown name is also null. The page says the caller tells those two nulls apart by looking at the list.
- AC6. `nextMigrationKind` is exported from `index.ts`. The published list names it. The probe saw the same function.
- Frozen inputs stayed unchanged. The source does not assign into the done list.
- The function does not flip a feature flag. The source does not call `setKindEnabled`. After the probe calls, every kind flag was still off.
- The playbook page and the changelog do not claim that 1.0.0 is published. The changelog section is Unreleased and says nothing is published.
- `rewriteListenerNames` is not in the diff. No new package was added. No shadow renderer was added.
- Tiers A, B and C are not applicable in the playbook and in the public-api section. The function does not render.
- HLD section 12 supplies the order. The null on a skip is the approved local decision, not an HLD formula. The implementation follows that decision.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-116. The playbook row is F-116.
- No new deviation. The design records none.
