# F-114 review, pass 1

**Reviewed**: Staged index tree `da5ced88f586d35f7e44509ab48fdcf0d25f569a` on `sprint/s40`. HEAD is `39fd74d5b141e28f0c92decb45d9d2c9360f82ab` (F-113). The review is `git diff --cached HEAD` only. `git write-tree` matched that hash at the start and again after the worktree edits were restored. The worktree matches the index. This file is unstaged.

**Author / reviewer**: staged implementation / 01a1a110-8ef6-7982-9345-fbf949606068 / independent pass 1. This reviewer did not write the change.

**Result**: 2 defects, 0 smells, 1 nitpick

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-114-design.md` and HLD section 7. Section 7 names tier A as WebGPU and tier B as WebGL2. Deviation D-07 adds tier C. The row names, the expected tiers and the not-run rule are the approved local plan. They are not an HLD formula. No staged file extends D-07. `docs/lld/migration.md` does not gain F-114.

The unit file locks these cases.

- SwiftShader, expected C, resolved C and exit 0 is certified.
- A renderer without that substring, expected B, resolved B and exit 0 is certified.
- SwiftShader with expected B and resolved B returns null for exit 0 and for exit 1, including when status says certified.
- No observation with status certified is not-run.
- Exit 1 with a non-SwiftShader renderer and status certified is observed.
- llvmpipe, expected C, resolved C and exit 0 is observed.
- An empty renderer is observed. A finished tier A row is observed.
- The committed JSON has the five named rows, no observation key, and neither the word certified nor the word SwiftShader. `certifyRow` returns not-run for each.

An independent probe of the restored function was run and was not added to the tree. It also checked cases the unit file does not lock.

- Expected B, resolved C, renderer `ANGLE (Apple, Metal)` and exit 0 is observed.
- Expected C, resolved B, renderer `Google SwiftShader` and exit 0 is observed.
- Expected B, resolved C and renderer `Google SwiftShader` is null.
- A certifying observation with status `not-run` or status `observed` is still certified.
- Exit 2 is observed. A non-numeric exit value `NaN` is observed. A one-space renderer is certified because the plan says non-empty, not trimmed.
- The lowercase string `swiftshader` does not contain the substring `SwiftShader`, so expected B is certified.
- llvmpipe with expected B, resolved B and exit 0 is certified. AC2 requires that. The spike names other software renderers. This plan does not. That is not an extension of D-07.

Those observations reject a classifier that certifies SwiftShader as tier B, certifies tier C from llvmpipe, trusts a caller status, or treats a missing run as certified.

Changing the expected-B SwiftShader arm from `return null` to `return "certified"` made `returns null when SwiftShader is recorded as tier B` fail. Expected null. Received certified. Exit 1.

Changing the tier C arm so any finished tier C row is certified, with no SwiftShader check, made `does not certify tier C without SwiftShader` fail. Expected observed. Received certified. Exit 1.

Deleting `observation.resolvedTier === row.expectedTier` left all 12 tests green. On that edit, expected B with resolved C and an ANGLE renderer returned certified, and expected C with resolved B and SwiftShader returned certified. Both are observed on the restored function. The comparison was restored.

The published name was checked through `index.ts`. A temporary test imported `certifyRow` from `./index.js` and from `./matrix.js`, asserted the same function, and asserted null for SwiftShader with expected B and resolved C. The test passed and was deleted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before review | Executed now | Exit 0. Tree `da5ced88f586d35f7e44509ab48fdcf0d25f569a`. HEAD `39fd74d5b141e28f0c92decb45d9d2c9360f82ab`. |
| `npx vitest run` on `matrix.test.ts` and `index.test.ts` | Restored staged sources, after the mutations | Executed now | Exit 0. 12 tests passed. 8 in `matrix.test.ts` and 4 in `index.test.ts`. |
| Same matrix file after the SwiftShader arm returned certified, filter `returns null when SwiftShader` | Mutated worktree, index not updated | Executed now | Exit 1. Expected null and received certified. |
| Same matrix file after tier C certified without a SwiftShader check, filter `without SwiftShader` | Mutated worktree, index not updated | Executed now | Exit 1. Expected observed and received certified on the llvmpipe row. |
| Full matrix and index tests after deleting the resolved-tier equality | Mutated worktree, index not updated | Executed now | Exit 0. 12 tests passed. |
| Node probe of that same edit | Mutated `matrix.ts` | Executed now | Expected B with resolved C returned certified. Expected C with resolved B and SwiftShader returned certified. |
| Node probe of restored `matrix.ts` | Staged `matrix.ts` and `ci/browser-matrix.json` | Executed now | 21 cases matched the plan, including the two mismatches as observed. The JSON had five rows and no observation. |
| Temporary published-binding test, then deleted | `index.ts` re-export of `certifyRow` | Executed now | Exit 0. The index binding was the module function and returned null. |
| `npx tsc -p packages/core --noEmit --pretty false` | Staged sources | Executed now | Exit 0. TypeScript 5.9.3. |
| `npx eslint` on `matrix.ts`, `matrix.test.ts`, `index.ts` and `index.test.ts` | Those four files | Executed now | Exit 0. |
| `python3 scripts/prose_check.py --staged` | 7 staged markdown paths in scope. `CHANGELOG.md` is outside that checker and was read directly. | Executed now | Exit 0. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row | Staged text | Executed now | The strings are equal and both end with F-114. `migration.md` does not contain F-114. |
| `git restore --worktree` on `matrix.ts`, probe file removed, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `da5ced88f586d35f7e44509ab48fdcf0d25f569a`. |

Environment: Node v24.13.0. Vitest 4.1.11. No cargo, rustc or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

### D1, resolved tier is not guarded

**Where**: `packages/core/src/matrix.ts:38` and `packages/core/src/matrix.test.ts:78`

**What**: Every unit input uses a resolved tier equal to the expected tier. Deleting `observation.resolvedTier === row.expectedTier` leaves `matrix.test.ts` and `index.test.ts` green. On that edit, a finished expected-B row whose resolved tier is C is certified, and a finished expected-C row whose resolved tier is B and whose renderer contains SwiftShader is certified.

**Why it is wrong**: The plan says a row is certified only when the exit code is 0, the resolved tier equals the expected tier, and the renderer string is non-empty. HLD section 7 resolves one tier. A resolved tier of C must not be recorded as certified tier B. A test that stays green when that comparison is removed is not a guard.

**Evidence**: After the equality was deleted, `npx vitest run packages/core/src/matrix.test.ts packages/core/src/index.test.ts` exited 0 with 12 passed. A node probe of the same edit returned certified for expected B with resolved C and renderer `ANGLE (Apple, Metal)`, and certified for expected C with resolved B and renderer `Google SwiftShader`. The restored function returns observed for both. The source was restored. `git write-tree` stayed `da5ced88f586d35f7e44509ab48fdcf0d25f569a`.

**Scope and regression**: `certifyRow` on the staged tree still compares the tiers, so today's function is not wrong. The suite will not notice if that comparison is dropped. The smallest regression is one expected-B row with resolved C and a non-SwiftShader renderer, expecting observed, plus one expected-C row with resolved B and SwiftShader, expecting observed. Present on the staged tree.

### D2, a failed SwiftShader probe is not observed

**Where**: `docs/sprints/AS_BUILT.md:4673`

**What**: The paragraph says a failed probe is observed. Expected tier B, a renderer that contains SwiftShader, and exit code 1 returns null. The same paragraph already says that pair returns null, and the unit test expects null for exit code 1.

**Why it is wrong**: The sentence is a false claim in the delivery record. The plan says a SwiftShader string with expected tier B returns null, with no success qualifier. The null result is not the observed status.

**Evidence**: The restored function returns null for that input. `returns null when SwiftShader is recorded as tier B` asserts null at `matrix.test.ts:129` when exit code is 1 and status says certified.

**Scope and regression**: The function matches the plan. The sentence does not. A later reader could change the null return into observed and call it a fix. The smallest correction is to stop saying every failed probe is observed. Present on the staged tree.

## Smells

None.

## Nitpicks

### N1, the type-export list omits the matrix types

**Where**: `packages/core/src/index.test.ts:20`

**What**: The parenthetical names the type-only exports and stops at `WorkerHandle`. `index.ts` also exports `MatrixInput`, `MatrixObservation`, `MatrixStatus` and `MatrixTier`.

**Why it is here**: The list is hand maintained. This diff publishes four types and does not name them. The runtime test cannot see type exports, so the omission does not fail a test. It does not change behaviour.

## Verified clean

- `certifyRow` ignores `status`. A missing observation is not-run even when status says certified. A certifying observation stays certified when status says not-run or observed.
- Tier B is certified only for exit 0, equal tiers, a non-empty renderer and no SwiftShader substring. The required mutation to return certified for that pair went red.
- Tier C is certified only for exit 0, equal tiers, a non-empty renderer and a SwiftShader substring. The required mutation that dropped the substring check went red on llvmpipe.
- SwiftShader with expected B returns null for exit 0, exit 1 and a resolved tier of C.
- Tier A with an observation is observed and is not certified. Tier A with no observation is not-run. The tier-coverage line is that finished-observation answer, not a claim about a missing run.
- The committed matrix names chromium-webgl2 (B), chromium-swiftshader (C), firefox (B), safari (B) and enterprise-locked (B). Each status is not-run. No row has an observation. None is certified.
- The function does not launch a browser, does not write the JSON file and does not call `addEventListener`. The diff does not touch `tools/tier-b/run.mjs` or `tools/swiftshader/run.mjs`. Those tools are not certification.
- No wasm-bindgen, no pixel path, no `unsafe` and no new deviation. D-07 is cited and is not edited.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-114.
- The changelog bullet matches the function. A missing run stays not-run. SwiftShader with expected tier B returns null. Tier C is not certified unless that substring is present.
- The tracker row says the duration was not measured. It does not claim a browser was certified. The AS_BUILT verification paragraph is the house template. It was not read as a gate receipt.
- `tsc --noEmit` and eslint on the four TypeScript files exited 0. The staged prose checker exited 0. The restored vitest run exited 0.

## Limitations and handoff

No browser was launched. No probe in `tools/tier-b` or `tools/swiftshader` was run. The matrix stays not-run, which is the commit rule, not a passed certification. The floor gate and the corpus gate were not run. `dist/` is ignored, so this pass did not rebuild the package output.

The unit file does not lock a resolved tier that differs from the expected tier. That is D1. It also does not lock status `not-run` beside a certifying observation. The restored function still certifies that input. The published callable check in `index.test.ts` does not include `certifyRow`. A deleted temporary test did.

This pass blocks. The next owner adds a mismatch test and corrects the failed-probe sentence. A new independent pass reviews that tree. Do not treat this pass as sprint readiness.
