# S41 review, pass 1

**Reviewed**: HEAD `1cda5b021c5339549015919604bca84b17694722` on `sprint/s41`. The merge base is `ce2722338b94308bf8112dc1ffbbeeaadd59a917`. `docs/sprints/CURRENT_SPRINT.md` is unstaged. No other path is unstaged and nothing is staged. `packages/core/src/release.ts` was mutated for one run and restored. It is not in the diff.
**Author / reviewer**: sprint/s41 / 01a1a1cc-390b-7452-b34c-3e83bcce84c7 / independent pass 1. This reviewer did not write the sprint.
**Result**: 0 defects, 0 smells, 1 nitpick. The nitpick does not block. A defect or a smell would block completion. Closing S41 completes M13 and the planned Phase 1 range and does not publish. The version stays 0.1.0. This pass does not mark F-X003 done. F-X011 stays pending. D-22 still blocks publication.

## Acceptance challenged

F-117 says `removalMinorsRemaining` returns null for `0.x`. A window of 0 would be a promise the plan refuses. On the restored function the channel test passed and `0.x` was null. The function was then changed so `0.x` returned 0. `npx vitest run packages/core/src/release.test.ts` failed one test and the other eight stayed green. The failure was `expected +0 to be null` at `packages/core/src/release.test.ts` line 49. Restoring the file made that suite pass again. The sha256 matched the pre-mutation bytes.

A separate probe was run on the restored modules and was not left in the tree. It checked the claims the unit files lock and a few neighbours.

- D-22 open with 5 certified rows of 5 returns false. 4 of 5 returns false. 6 of 5 returns false. Closed D-22 with 5 of 5 returns true, and `VERSION` stays `0.1.0`.
- An empty migration list returns stack. stack returns mpr. stack then mpr returns volume. The finished list returns null. A volume-first list returns null. After those calls the stack, mpr and volume flags are still off.
- Shifted ArrowUp is scroll-previous. Shifted ArrowLeft stays pan-left. Shifted equals stays zoom-in. `keys.ts` does not contain `addEventListener` or `pointer.ts`.
- A message with both trailer lines and the four verify fields is recorded. A message with neither trailer is missing. `provenance.ts` does not name `writeFile` or `appendFile`.
- A dev profile whose glue phase is 2.3 returns null. The same phases on release sum to 2.3. A release first-frame interval of 2.3 is returned unchanged. A dev interval of 2.3 returns null. `startup.ts` does not contain `2.3` or `bench-baseline`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git merge-base HEAD main` | branch `sprint/s41` | executed now | HEAD `1cda5b021c5339549015919604bca84b17694722`. Merge base `ce2722338b94308bf8112dc1ffbbeeaadd59a917`. |
| `git status --short` after the restore | worktree | executed now | Only `docs/sprints/CURRENT_SPRINT.md` is modified. The cached diff is empty. |
| `git diff ce27223...HEAD -- Cargo.toml packages/core/package.json package.json` | version files against the merge base | executed now | Empty. Exit 0. No version bump. `packages/react/package.json` is also `0.1.0` and is not in the sprint diff. |
| Sprint table counts and the backlog awk in `CURRENT_SPRINT.md` | working sprint note and `docs/sprints/BACKLOG.md` | executed now | Both table counts are 5. The five S41 rows print done. F-X003 is in-progress. F-X011 is pending. |
| Python compare of the public-api F-ID header with the README public-api cell | working tree | executed now | Equal. 50 ids. Both end with F-118. The benchmarks, playbook and release-policy pairs also match. |
| `npx vitest run` on the five story files and `index.test.ts` | restored tree, before the mutation | executed now | Exit 0. 44 passed. The counts are 14, 6, 4, 9, 7 and 4. |
| `npx tsc -p packages/core --noEmit --pretty false` | restored sources, before the mutation | executed now | Exit 0. TypeScript 5.9.3. |
| Deleted probe of the five modules and `kindEnabled` | restored tree, before the mutation | executed now | Exit 0. 5 passed. The probe file is absent. |
| `npx vitest run packages/core/src/release.test.ts` after `0.x` returned 0 | mutated `release.ts`, not staged | executed now | Exit 1. 1 failed and 8 passed. Failure at line 49. |
| `cmp`, `shasum -a 256` and the same vitest command | restored `release.ts` | executed now | Compare empty. sha256 `e6827453340bda5c0bb15e89067cb0af18365b58e338bd4cc78d556402e74a7b`. Length 834. Exit 0. 9 passed. |
| Sprint name diff for the hook, deviations, size budget, cold-start baseline and browser matrix | range `ce27223...HEAD` | executed now | None of those paths. Deviation rows stay 27 on both sides. `python3 scripts/deviation_check.py` printed OK. |
| Read of the budget, the baseline and the A4 row | files outside the sprint diff | executed now | Budget bytes are 2151196 with tolerance 0.05. `wasm.cold_start` is 2.3 and is the F-006 figure. The A4 row still says the question is unmeasured. |
| Text compare of the two kind lists | `packages/core/src/flag.ts` and `packages/core/src/playbook.ts` | executed now | Both constants are `["stack", "mpr", "volume"]`. |

Environment: Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. The sprint gate was not run.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the type-export parenthetical still stops at MatrixTier

**Where**: `packages/core/src/index.test.ts` lines 35 to 36. The new type exports are `packages/core/src/index.ts` lines 261, 264 and 269.

**What**: The comment lists type-only exports and stops at `MatrixTier`. This sprint also exports `ColdPhases`, `StartupRole`, `KeyAction` and `StableClaim`. The runtime list names the functions. The F-115 review and the F-117 review classified the same omission as a nitpick. This pass does not fix it.

**Why it is here**: The list is hand maintained. A deleted type export is not visible to the runtime key test. The omission does not change behaviour.

## Verified clean

The unstaged corrections in `docs/sprints/CURRENT_SPRINT.md` match the code. F-111 names `pkg/ocelli_wasm_bg.wasm` for render and `pkg-decode/ocelli_wasm_bg.wasm` for decode. `summariseColdStart` adds glue, fetch, compile, instantiate and firstCall only for a release profile whose phases are finite and at least 0. A dev profile returns null. The source does not read the 2.3 ms baseline and the baseline file is not in the sprint diff. Gate A4 stays open. `ci/wasm-size-budget.json` is not in the sprint diff. The dependency sentence says F-111 chooses the artefact from the split F-110 built. `bin/ocelli.sh` writes the decode module under `pkg-decode`. The function returns that relative path. It does not measure a wasm graph.

F-115 names the arrow keys, plus, equals and minus. In `packages/core/src/keys.ts`, shift changes only ArrowUp and ArrowDown. The module has no import, no `addEventListener` call and no camera write. The story does not edit `ci/browser-matrix.json`.

F-116 returns the next kind only when the done list is a prefix of stack, then mpr, then volume. `nextMigrationKind` imports `ViewportKind` as a type and does not call `setKindEnabled`. The probe left every kind flag off. `docs/lld/playbook.md` states the same order and says this page does not claim that 1.0.0 is published. It does not state a formula the HLD does not contain. The second kind list in `flag.ts` is the same text. The F-116 review accepted that duplicate. The lists still match, and this pass does not reopen it.

F-117 returns false while D-22 is open and while the certified count is not the full integer matrix count. `0.x` is null, not 0. The mutation that returned 0 went red and the restored suite is green. `CHANGELOG.md` has one heading, `Unreleased`, and says nothing is published. No version heading was added. No tag points at HEAD. `Cargo.toml`, `package.json` and `packages/core/package.json` stay `0.1.0`. `index.test.ts` expects that literal and the test passed. D-22 and D-11 are not in the sprint diff.

F-118 classifies one message as recorded, missing or forged. The function does not write a trailer. `.githooks/commit-msg` is not in the sprint diff, so the hook stays the only writer. A hand-written verify line is still a hook refusal, not a classifier success by itself. `recorded` is the required shape. The public-api section says that shape does not prove the named tree.

Each story states tiers A, B and C are not applicable. That is in each approved plan and in the cold-start, keyboard, migration, stable-claim and provenance sections of `docs/lld/public-api.md`. The playbook page and the release-policy page say the same. The functions do not branch on a tier.

The public-api F-ID header equals the README public-api cell. Both have 50 ids and both end with F-118. The benchmarks header ends with F-111. The playbook header is F-116. The release-policy header is F-117. Those README cells match.

The five backlog rows and the five sprint rows are done. F-X003 stays in-progress. F-X011 stays pending. The five S41 tracker rows say not measured. This pass does not demand a duration or a ratio. The committed matrix has five rows and every status is not-run. The release-policy sentence that the current matrix has no certified row matches that file.

`docs/lld/benchmarks.md` says F-111 is not the 2.3 ms measurement, names both artefact paths, and says gate A4 stays open. The cold-start, keyboard, migration, stable-claim and provenance sections match the functions exercised above. No false sentence was found in those sections or in the unstaged sprint note.

The size-check formula `int(baseline * (1 + tolerance))` on 2151196 and 0.05 is 2258755. That is the ceiling named in the carried sprint note. The decode size file still records 16455 bytes and is not in the sprint diff. Tag `s40` is annotated object `730fdcac` and peels to merge `ce27223`. This pass did not remeasure wasm.

No new deviation was added. The version was not bumped. The pool count operator, the F-113 admission rule and the F-114 SwiftShader rule were not reopened.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, the floor gate or the corpus gate. It did not start cargo, rustc or wasm-pack. It did not rebuild wasm and it did not launch a browser. The five story commits carry verify trailers from earlier feature profiles. Those trailers are prior receipts. This pass did not re-run them. It did not re-run the S40 pool ceiling, the cache admission check or the SwiftShader rule.

This pass did not stage this review, did not commit and did not push. The review file is outside the index. Closing S41 completes M13 and the planned Phase 1 range and does not publish. Do not mark F-X003 done. This pass does not open the next phase.
