# F-117 review, pass 1

**Reviewed**: Uncommitted worktree on `sprint/s41`. HEAD is `2beddd02702caf43998ebd40b3c92a6157dbe17e`. `packages/core/src/release.ts`, `packages/core/src/release.test.ts` and `docs/lld/release-policy.md` are untracked. The other F-117 edits are unstaged. This file is untracked. Nothing was staged or committed.

**Author / reviewer**: uncommitted implementation / 01a1a19b-e469-7a32-9e2f-677f8395f9f2 / independent pass 1. This reviewer did not write the change.

**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

AC1 to AC9 were checked against `.claude/plans/F-117-design.md`. `docs/RELEASE.md` says 1.0.0 waits on this story and that `/release` is the only command that may publish. It does not say how many certified rows are enough, and it does not say how long a deprecated name stays. The full-matrix rule and the one-minor window are the approved local decision. They are not an HLD formula.

The unit file locks these cases.

- D-22 open with 5 certified rows of 5 returns false.
- 0 certified rows of 5 returns false.
- 4 certified rows of 5 returns false.
- D-22 closed with 5 of 5 returns true.
- Counts of 1.5 return false. A matrix of 1 with a certified count of 1.5 returns false.
- An empty matrix, 0 and 0, returns false.
- `0.x`, `patch`, `minor` and `major` return null, null, 1 and 0.
- `beta` and an empty string return null.
- `mayClaimStable` and `removalMinorsRemaining` are on the published runtime list.
- `VERSION` stays `0.1.0` in `release.test.ts` and in the existing index test.

An independent probe ran on the original function and was not left in the tree. The restore later matched those bytes. The probe also checked cases the unit file does not name.

- A certified count of 6 against 5 rows returns false. A certified count of 2 against 1 row returns false. Replacing equality with a greater-or-equal check would have accepted the surplus. The function does not.
- Equal negative counts return false. A negative row count returns false. A negative certified count against a positive row count returns false. Negative zero returns false.
- `NaN` returns false. Positive infinity against positive infinity returns false. Negative infinity returns false. In JavaScript, infinity compares equal to itself, so a missing integer check would have returned true. The function returns false.
- With equal counts of 1, `d22Open` values `0`, `1`, `"false"`, `""`, `null`, `undefined` and an object all return false. A truthiness check would have treated `0` and `""` as closed. The function requires exactly `false`.
- `Minor`, `MAJOR`, `Major`, `0.X` and `PATCH` return null. A channel with a leading space, a trailing space, both, a tab or a trailing newline returns null.
- A null channel, an undefined channel, a number and an object return null. None of those calls throw.
- String counts `"5"` and `"5"` return false. Boolean counts return false. A missing field object returns false.
- The committed matrix has 5 rows and no certified row. Passing that count with D-22 open returns false. The same counts with D-22 closed also return false.
- A frozen input of 4 certified rows of 4, with D-22 closed, returns true. The object is unchanged.
- `1.0` is an integer and returns true when D-22 is closed and the certified count is the same. `Number.MAX_SAFE_INTEGER` with an equal certified count returns true.
- None of the 50 probe calls threw.

A local Vite load showed that the index bindings are the same two functions. `StableClaim` is not a runtime key. `VERSION` read from the index module is `0.1.0`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git status --short` and `git branch --show-current` | Worktree before this review file | Executed now | HEAD `2beddd02702caf43998ebd40b3c92a6157dbe17e` on `sprint/s41`. Five modified tracked files and three untracked product files. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row | Worktree text | Executed now | The strings are equal. Both have 49 ids. Both end with F-117. Length 342. |
| `npx vitest run packages/core/src/release.test.ts` | Original `release.ts` sha256 `e6827453340bda5c0bb15e89067cb0af18365b58e338bd4cc78d556402e74a7b` | Executed now, before the mutation | Exit 0. 9 passed. |
| Node probe of `mayClaimStable` and `removalMinorsRemaining` | Same original bytes. The probe was a process, not a file. | Executed now, before the mutation | Exit 0. 50 calls. 0 throws. Surplus, negative, `NaN`, infinity, non-boolean `d22Open`, `Minor`, `MAJOR` and padded `minor` behaved as the plan states. |
| Vite load of `index.ts` and `release.ts` | Same original bytes | Executed now, before the mutation | Exit 0. The two index bindings are the module functions. `StableClaim` is absent at runtime. `VERSION` is `0.1.0`. |
| Same vitest command after the minor return changed from 1 to 2 | Mutated worktree, not staged | Executed now | Exit 1. 1 failed and 8 passed. `returns null, null, 1 and 0 for the four channels` expected 1 and received 2 at `release.test.ts:51`. |
| `cmp`, `shasum -a 256` and `wc -c` after copy-back | Restored bytes | Executed now | Compare empty. sha256 `e6827453340bda5c0bb15e89067cb0af18365b58e338bd4cc78d556402e74a7b`. Length 834. |
| `npx vitest run packages/core/src/release.test.ts packages/core/src/index.test.ts` | Restored tree | Executed now | Exit 0. 13 passed. 9 in `release.test.ts` and 4 in `index.test.ts`. |
| `npx tsc -p packages/core --noEmit --pretty false` | Worktree sources after the restore | Executed now | Exit 0. |
| `npx eslint` on `release.ts`, `release.test.ts`, `index.ts` and `index.test.ts` | Those four files, after the restore | Executed now | Exit 0. ESLint 9.39.5. |
| Python scan of the new prose, plus a read of `ci/browser-matrix.json`, `packages/core/package.json` and `Cargo.toml` | Worktree text. Neither version file is in the diff. | Executed now | No em dash and no prose semicolon in the new prose. The matrix has 5 rows, each `not-run`, and no observation. Both version fields are `0.1.0`. |

Environment: Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. No cargo, rustc or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the type-export list omits StableClaim

**Where**: `packages/core/src/index.test.ts:36` and `packages/core/src/index.ts:269`

**What**: The parenthetical names the type-only exports and stops at `MatrixTier`. This diff also exports `StableClaim`. `ColdPhases`, `StartupRole` and `KeyAction` were already unnamed. The runtime list does name `mayClaimStable` and `removalMinorsRemaining`.

**Why it is here**: The list is hand maintained. This diff publishes one type and does not name it. The runtime test cannot see a type export, so the omission does not fail a test. It does not change behaviour.

## Verified clean

- AC1. D-22 open returns false even when every row is certified. The unit test locks 5 of 5 with the flag open. The source accepts only `d22Open === false` by rejecting every other value with `!==`.
- AC2. Zero certified rows returns false. The unit test locks 0 of 5. The empty matrix, 0 and 0, also returns false, so an empty matrix is not treated as certified.
- AC3. A certified count short of the row count returns false. The unit test locks 4 of 5. The probe locks a count above the row count, which the unit file does not name.
- AC4. Closed D-22 and equal positive counts return true. The true result does not write a version file. `packages/core/package.json` and `[workspace.package]` stay `0.1.0`, and neither file is in the diff. `VERSION` in `index.ts` is unchanged.
- AC5. A non-integer count returns false. The unit test locks 1.5. The probe locks `NaN`, both infinities, numeric strings and booleans. `Number.isInteger` does not coerce them.
- AC6. The four channels return null, null, 1 and 0. Changing the minor return from 1 to 2 went red on `release.test.ts:51`. `0.x` was not given a window of 0.
- AC7. An unknown channel returns null. The unit test locks `beta` and `""`. The probe locks case changes, surrounding spaces, and non-strings. Those calls do not throw.
- AC8. `index.ts` exports `mayClaimStable`, `removalMinorsRemaining` and the type `StableClaim`. The published runtime list names the two functions. The restored index test matched `Object.keys` of the module. The Vite load saw the same functions. `tsc` accepted the type export.
- AC9. The existing index test still expects `0.1.0`, and that test passed. `release.test.ts` expects the same literal.
- The functions do not read a file, do not parse `ci/browser-matrix.json`, and do not bump a version. The module has no `addEventListener`, no `wasm-bindgen`, no `unsafe`, no fetch and no network call.
- The committed matrix names chromium-webgl2, chromium-swiftshader, firefox, safari and enterprise-locked. Each status is `not-run`. No row has an observation. None is certified. D-22 is still the open publication block. This diff does not edit `docs/hld/DEVIATIONS.md`. The vendor package `openjph-core` 0.1.0 still has no `LICENSE` file. A caller who passes those facts gets false.
- `docs/lld/release-policy.md` states the full-matrix rule, the null windows for `0.x`, patch and any other channel, the one further minor, and the major window of 0. It says this page does not publish and does not create a `v` tag. The workspace version in that page is `0.1.0`.
- The public-api section says true only when D-22 is closed, the row count is an integer of at least 1, and the certified count is that same integer. Every other input returns false. The channel table matches the function. It says the functions do not publish and do not change the workspace version.
- Tiers A, B and C are not applicable in the policy page and in the public-api section. The functions do not render and do not branch on a tier.
- The changelog bullet is under `Unreleased`. It does not add a version heading. It says nothing is published and that the workspace version stays `0.1.0`. No `v` tag points at HEAD. This story does not run `/release`.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-117. The release-policy row is F-117.
- No new deviation. The design records none. D-22 is cited and not edited. D-11 is not edited.
- The new prose has no em dash and no prose semicolon. That covers `docs/lld/release-policy.md`, the stable-claim section, the changelog bullet and the new README cells.
- `tsc --noEmit` and eslint on the four TypeScript files exited 0. The restored vitest run exited 0.

## Limitations and handoff

The floor gate and the corpus gate were not run. No browser was launched. The story was not committed. `git diff` cannot show a restore of `release.ts`, because that file is still untracked. The sha256 match and the empty byte compare are the restore evidence.

The probe ran before the mutation. The restored file is byte-identical to the probed file, and the restored suite locks the minor window at 1 again.

Behaviour matches the approved plan. The type-export parenthetical is the only open note.
