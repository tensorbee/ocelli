# F-114 review, pass 2

**Reviewed**: Staged index tree `7c0f7a307138d382a4fa4823c91451bdfb0f7650` on `sprint/s40`. HEAD is `39fd74d5b141e28f0c92decb45d9d2c9360f82ab`. The review is `git diff --cached HEAD` only. `git write-tree` matched that hash at the start and again after the worktree edits were restored. The worktree matches the index. This file is untracked.

**Author / reviewer**: staged implementation / 01a1a11e-fbc0-70a3-aa5b-7f88dc612f7d / independent pass 2. This reviewer did not write the change.

**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-114-design.md` and HLD section 7. Section 7 names tier A as WebGPU and tier B as WebGL2 through wgpu's downlevel profile. Deviation D-07 adds tier C. The row names, the expected tiers, and the not-run rule are the approved local plan. They are not an HLD formula. No staged file extends D-07. `docs/lld/migration.md` does not contain F-114.

The unit file locks these cases.

- SwiftShader, expected C, resolved C, and exit 0 is certified.
- A renderer without that substring, expected B, resolved B, and exit 0 is certified.
- SwiftShader with expected B and resolved B returns null for exit 0 and for exit 1, including when status says certified.
- No observation with status certified is not-run.
- Exit 1 with a non-SwiftShader renderer and status certified is observed.
- llvmpipe, expected C, resolved C, and exit 0 is observed.
- Expected B with resolved C and an ANGLE renderer is observed. Expected C with resolved B and SwiftShader is observed.
- A certifying observation with status not-run stays certified.
- An empty renderer is observed. A finished tier A row is observed.
- The committed JSON has the five named rows, no observation key, and neither the word certified nor the word SwiftShader. `certifyRow` returns not-run for each.

An independent probe of the restored function was run and was not added to the tree. It also checked cases the unit file does not lock.

- Expected B, resolved C, renderer `Google SwiftShader`, and exit 0 returns null. Exit 1 returns null. Resolved A returns null.
- Expected B, resolved C, and renderer `ANGLE (Apple, Metal)` returns observed.
- Expected C, resolved B, and renderer `Google SwiftShader` returns observed.
- A certifying observation with status observed stays certified.
- Exit 2 is observed. A non-numeric exit value `NaN` is observed. A one-space renderer is certified because the plan says non-empty, not trimmed.
- The lowercase string `swiftshader` does not contain the substring `SwiftShader`, so expected B is certified.
- A failed llvmpipe row and a failed SwiftShader row with expected C are observed.

Those observations reject a classifier that certifies SwiftShader as tier B, certifies tier C from llvmpipe, trusts a caller status, treats a missing run as certified, or certifies the two mismatched rows the new test names.

Deleting `observation.resolvedTier === row.expectedTier` made `does not certify a resolved tier that differs from the expected tier` fail. Expected observed. Received certified. Exit 1. Pass 1's equality gap is closed for that deletion.

Changing the expected-B SwiftShader arm from `return null` to `return "certified"` made `returns null when SwiftShader is recorded as tier B` fail. Expected null. Received certified. Exit 1.

Changing the tier C arm so any finished tier C row is certified, with no SwiftShader check, made `does not certify tier C without SwiftShader` fail. Expected observed. Received certified. Exit 1.

Adding `observation.resolvedTier === row.expectedTier` to the null arm left all 14 tests green. On that edit, expected B, resolved C, and renderer `Google SwiftShader` returned observed. The restored function returns null. The plan requires null for that pair with no resolved-tier exception. The suite does not lock it.

The published name was checked through `index.ts`. A temporary test imported `certifyRow` from `./index.js` and from `./matrix.js`, asserted the same function, and asserted null for SwiftShader with expected B and resolved C. The test passed and was deleted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before review | Executed now | Exit 0. Tree `7c0f7a307138d382a4fa4823c91451bdfb0f7650`. HEAD `39fd74d5b141e28f0c92decb45d9d2c9360f82ab`. |
| Node probe of restored `matrix.ts` and `ci/browser-matrix.json` | Staged function and JSON | Executed now | 23 cases matched the plan. Expected B with resolved C and SwiftShader returned null. The JSON had five not-run rows and no observation. |
| `npx vitest run` on the two test files after deleting the resolved-tier equality, filter `does not certify a resolved tier` | Mutated worktree, index not updated | Executed now | Exit 1. Expected observed and received certified on the ANGLE mismatch. |
| Same files after the SwiftShader arm returned certified, filter `returns null when SwiftShader` | Mutated worktree, index not updated | Executed now | Exit 1. Expected null and received certified. |
| Same files after tier C certified without a SwiftShader check, filter `without SwiftShader` | Mutated worktree, index not updated | Executed now | Exit 1. Expected observed and received certified on the llvmpipe row. |
| Full matrix and index tests after the null arm also required equal tiers, then a node probe of that edit | Mutated worktree, index not updated | Executed now | Exit 0. 14 tests passed. The SwiftShader mismatch then returned observed. |
| `npx vitest run` on `matrix.test.ts` and `index.test.ts` | Restored staged sources | Executed now | Exit 0. 14 tests passed. 10 in `matrix.test.ts` and 4 in `index.test.ts`. |
| Temporary published-binding test, then deleted | `index.ts` re-export of `certifyRow` | Executed now | Exit 0. The index binding was the module function and returned null. |
| `npx tsc -p packages/core --noEmit --pretty false` | Staged sources | Executed now | Exit 0. |
| `npx eslint` on `matrix.ts`, `matrix.test.ts`, `index.ts`, and `index.test.ts` | Those four files | Executed now | Exit 0. |
| `python3 scripts/prose_check.py --staged` | 8 staged markdown paths in scope. `CHANGELOG.md` is outside that checker and was read directly. | Executed now | Exit 0. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row | Staged text | Executed now | The strings are equal and both end with F-114. `migration.md` does not contain F-114. |
| `git restore --worktree` on `matrix.ts`, probe file removed, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `7c0f7a307138d382a4fa4823c91451bdfb0f7650`. |

Environment: Node v24.13.0. Vitest 4.1.11. No cargo, rustc, or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

### D1, a tier mismatch is not always observed

**Where**: `docs/sprints/AS_BUILT.md:4673` and `docs/sprints/AS_BUILT.md:4685`

**What**: The delivery paragraph says a resolved tier that differs from the expected tier is observed. Expected tier B, renderer `Google SwiftShader`, and resolved tier C returns null for exit 0 and for exit 1. The notes say an expected tier B row whose resolved tier is C stays observed. That claim is false when the renderer contains SwiftShader.

**Why it is wrong**: The approved plan says a SwiftShader string with expected tier B returns null. That sentence has no success qualifier and no resolved-tier exception. Null is not the observed status. HLD section 7 resolves one tier. A false sentence in the delivery record is the same class as the failed-probe overclaim from pass 1.

**Evidence**: The restored function returns null for expected B, resolved C, and renderer `Google SwiftShader` at exit 0, at exit 1, and for resolved A. Adding an equal-tier conjunct to the null arm left `npx vitest run packages/core/src/matrix.test.ts packages/core/src/index.test.ts` at exit 0 with 14 passed. A node probe of that edit returned observed for the same SwiftShader mismatch. The source was restored. `git write-tree` stayed `7c0f7a307138d382a4fa4823c91451bdfb0f7650`.

**Scope and regression**: The function matches the plan. The two sentences do not. A later reader can narrow the null return to equal tiers and call that a fix, and the current suite stays green. The smallest correction is to stop saying every tier mismatch is observed. SwiftShader with expected tier B stays null even when the resolved tier differs. A non-SwiftShader mismatch stays observed. The notes sentence needs the same limit. Present on the staged tree.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass 1's equality deletion now fails `does not certify a resolved tier that differs from the expected tier`. The first assertion expected observed and received certified.
- Pass 1's failed-probe overclaim is gone. The paragraph says a failed probe whose renderer does not contain SwiftShader is observed, and it says SwiftShader with expected tier B returns null when the exit code is not 0. Both match the function.
- The four type names in the `index.test.ts` parenthetical are `MatrixInput`, `MatrixObservation`, `MatrixStatus`, and `MatrixTier`. Those are the four type exports added from `matrix.ts`.
- `certifyRow` ignores `status`. A missing observation is not-run even when status says certified. A certifying observation stays certified when status says not-run or observed.
- Tier B is certified only for exit 0, equal tiers, a non-empty renderer, and no SwiftShader substring. The required mutation to return certified for that pair went red.
- Tier C is certified only for exit 0, equal tiers, a non-empty renderer, and a SwiftShader substring. The required mutation that dropped the substring check went red on llvmpipe.
- SwiftShader with expected B and resolved B returns null for exit 0 and for exit 1.
- Tier A with an observation is observed and is not certified. Tier A with no observation is not-run. The tier-coverage line is that observation answer, not a claim about a missing run.
- The committed matrix names chromium-webgl2 (B), chromium-swiftshader (C), firefox (B), safari (B), and enterprise-locked (B). Each status is not-run. No row has an observation. None is certified.
- The function does not launch a browser, does not write the JSON file, and does not call `addEventListener`. The diff does not touch `tools/tier-b/run.mjs` or `tools/swiftshader/run.mjs`. Those tools are not certification.
- No wasm-bindgen, no pixel path, no `unsafe`, and no new deviation. D-07 is cited and is not edited.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-114.
- The changelog bullet matches the function. A missing run stays not-run. SwiftShader with expected tier B returns null. Tier C is not certified unless that substring is present.
- The tracker row says the duration was not measured. It does not claim a browser was certified and it does not claim a feature receipt. The AS_BUILT verification paragraph is the house template. It was not read as a gate receipt.
- `tsc --noEmit` and eslint on the four TypeScript files exited 0. The staged prose checker exited 0. The restored vitest run exited 0.
- The matrix test count in the delivery record is ten. The restored vitest run reported ten tests in `matrix.test.ts`.

## Limitations and handoff

No browser was launched. No probe in `tools/tier-b` or `tools/swiftshader` was run. The matrix stays not-run, which is the commit rule, not a passed certification. The floor gate and the corpus gate were not run. `dist/` is ignored, so this pass did not rebuild the package output.

The unit file does not lock SwiftShader with expected B and a resolved tier other than B. The restored function returns null. Narrowing that null arm to equal tiers stayed green and returned observed. The published callable check in `index.test.ts` does not call `certifyRow`. A deleted temporary test did. The type-export parenthetical is not a census of every type export in `index.ts`. The older omissions predate this diff. This pass checked the four new names only.

This pass blocks. The next owner corrects the mismatch sentences. A new independent pass reviews that tree. Do not treat this pass as sprint readiness.
