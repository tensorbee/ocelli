# F-114 review, pass 3

**Reviewed**: Staged index tree `de94fb6203bc03d18733c17d7692d9761747c7fa` on `sprint/s40`. HEAD is `39fd74d5b141e28f0c92decb45d9d2c9360f82ab`. The review is `git diff --cached HEAD` only. `git write-tree` matched that hash at the start and again after the worktree edits were restored. The worktree matches the index. This file is untracked.

**Author / reviewer**: staged implementation / 01a1a132-9143-70f0-909e-a29c179a96fb / independent pass 3. This reviewer did not write the change.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-114-design.md` and HLD section 7. Section 7 names tier A as WebGPU and tier B as WebGL2 through wgpu's downlevel profile. Deviation D-07 adds tier C. The row names, the expected tiers, and the not-run rule are the approved local plan. They are not an HLD formula. No staged file extends D-07. `docs/lld/migration.md` does not contain F-114.

The unit file locks these cases.

- SwiftShader, expected C, resolved C, and exit 0 is certified.
- A renderer without that substring, expected B, resolved B, and exit 0 is certified.
- SwiftShader with expected B and resolved B returns null for exit 0 and for exit 1, including when status says certified.
- No observation with status certified is not-run.
- Exit 1 with a non-SwiftShader renderer and status certified is observed.
- llvmpipe, expected C, resolved C, and exit 0 is observed.
- Expected B with resolved A or resolved C and renderer `Google SwiftShader` at exit 0 returns null.
- Expected B with resolved C and an ANGLE renderer is observed. Expected C with resolved B and SwiftShader is observed.
- A certifying observation with status not-run stays certified.
- An empty renderer is observed. A finished tier A row is observed.
- The committed JSON has the five named rows, no observation key, and neither the word certified nor the word SwiftShader. `certifyRow` returns not-run for each.

An independent probe of the restored function was run and was not added to the tree. It also checked cases the unit file does not lock in one assertion.

- Expected B, renderer `Google SwiftShader`, and exit 1 with resolved B returns null. Exit 0 with resolved C returns null. Exit 1 with resolved C returns null. Exit 2 with resolved A returns null.
- A failed probe whose renderer does not contain SwiftShader is observed. That held for ANGLE on expected B, llvmpipe on expected C, wgpu on expected A, and ANGLE on expected B with resolved C.
- A resolved-tier mismatch whose renderer does not contain SwiftShader is observed, including an empty renderer.
- Expected C with resolved B and SwiftShader is observed. Expected A with resolved B and SwiftShader is observed. Those are not null.
- A missing tier A observation is not-run. A certifying observation stays certified when status says observed.
- The lowercase string `swiftshader` does not contain `SwiftShader`, so expected B is certified. A one-space renderer is certified. A non-numeric exit value `NaN` is observed. A failed SwiftShader row with expected C is observed.

Those observations reject a classifier that certifies SwiftShader as tier B, certifies tier C from llvmpipe, trusts a caller status, treats a missing run as certified, certifies a resolved-tier mismatch, or narrows the expected-B SwiftShader null to equal tiers.

Adding `observation.resolvedTier === row.expectedTier` to the expected-B SwiftShader arm made `returns null for SwiftShader on expected tier B when the resolved tier differs` fail. Expected null. Received observed. Exit 1. The first loop value was resolved A. The arm was restored and does not require equal tiers. Pass 2's green narrowing is closed on this suite.

Adding `observation.exitCode === 0` to that same arm made `returns null when SwiftShader is recorded as tier B` fail at the exit 1 assertion. Expected null. Received observed. Exit 1. That conjunct was restored.

The published name was checked through `index.ts`. A temporary test imported `certifyRow` from `./index.js` and from `./matrix.js`, asserted the same function, and asserted null for SwiftShader with expected B, resolved C, and exit 1. The test passed and was deleted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before review | Executed now | Exit 0. Tree `de94fb6203bc03d18733c17d7692d9761747c7fa`. HEAD `39fd74d5b141e28f0c92decb45d9d2c9360f82ab`. |
| Node probe of restored `matrix.ts` | Staged function, 25 inputs | Executed now | Exit 0. 25 cases matched. Expected B with SwiftShader returned null for exit 1, for a different resolved tier, and for both together. Expected C with SwiftShader and resolved B returned observed. |
| `npx vitest run` on `matrix.test.ts`, filter `when the resolved tier differs`, after the null arm also required equal tiers | Mutated worktree, index not updated | Executed now | Exit 1. Expected null and received observed at `matrix.test.ts:183`. |
| Same file, filter `returns null when SwiftShader is recorded as tier B`, after the null arm also required exit code 0 | Mutated worktree, index not updated | Executed now | Exit 1. Expected null and received observed at `matrix.test.ts:129`. |
| `npx vitest run` on `matrix.test.ts` and `index.test.ts` | Restored staged sources | Executed now | Exit 0. 15 tests passed. 11 in `matrix.test.ts` and 4 in `index.test.ts`. |
| Temporary published-binding test, then deleted | `index.ts` re-export of `certifyRow` | Executed now | Exit 0. The index binding was the module function and returned null. |
| `npx tsc -p packages/core --noEmit --pretty false` | Staged sources | Executed now | Exit 0. TypeScript 5.9.3. |
| `npx eslint` on `matrix.ts`, `matrix.test.ts`, `index.ts`, and `index.test.ts` | Those four files | Executed now | Exit 0. ESLint 9.39.5. |
| `python3 scripts/prose_check.py --staged` | 9 staged markdown paths in scope. `CHANGELOG.md` is outside that checker and was read directly. | Executed now | Exit 0. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row | Staged text | Executed now | The strings are equal and both end with F-114. `migration.md` does not contain F-114. |
| Python read of `ci/browser-matrix.json` and the F-114 tracker row | Staged JSON and tracker | Executed now | Five not-run rows. No observation key. The words certified and SwiftShader are absent. The tracker cell says not measured and does not say receipt. |
| `git restore --worktree` on `matrix.ts`, probe file removed, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `de94fb6203bc03d18733c17d7692d9761747c7fa`. |

Environment: Node v24.13.0. Vitest 4.1.11. No cargo, rustc, or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass 2's overclaim is gone. `docs/sprints/AS_BUILT.md` says SwiftShader with expected tier B returns null, including when the exit code is not 0 and when the resolved tier differs. The restored function does that before it reads the exit code or the resolved tier. Both narrowings of that arm went red and were restored.
- A failed probe whose renderer does not contain SwiftShader is observed. The unit test locks ANGLE, expected B, resolved B, and exit 1. The probe returned observed for the other non-SwiftShader failures above.
- A resolved tier that differs from the expected tier is observed when the renderer does not contain SwiftShader. The sentence does not say only when. Expected C with SwiftShader and a different resolved tier stays observed. The notes say the same limit. An expected B row whose resolved tier is C stays observed when the renderer does not contain SwiftShader. An expected C row whose resolved tier is B stays observed, including when the renderer contains SwiftShader. SwiftShader with expected tier B stays null when the resolved tier differs.
- Tests added says eleven tests in `packages/core/src/matrix.test.ts`. The restored run reported 11 tests in that file.
- The note that narrowing the null arm to equal tiers left that suite green is the pass 2 history. It is not a claim about this suite. This suite failed that edit.
- `certifyRow` ignores `status`. A missing observation is not-run even when status says certified. A certifying observation stays certified when status says not-run or observed.
- Tier B is certified only for exit 0, equal tiers, a non-empty renderer, and no SwiftShader substring. Tier C is certified only for exit 0, equal tiers, a non-empty renderer, and a SwiftShader substring. Tier A with an observation is observed and is not certified. Tier A with no observation is not-run.
- The committed matrix names chromium-webgl2 (B), chromium-swiftshader (C), firefox (B), safari (B), and enterprise-locked (B). Each status is not-run. No row has an observation. None is certified. No browser was run.
- The function does not launch a browser, does not write the JSON file, and does not call `addEventListener`. The diff does not touch `tools/tier-b/run.mjs` or `tools/swiftshader/run.mjs`.
- No wasm-bindgen, no pixel path, no `unsafe`, and no new deviation. D-07 is cited and is not edited.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-114. `docs/lld/migration.md` does not gain F-114.
- The changelog bullet matches the function. A missing run stays not-run. SwiftShader with expected tier B returns null. Tier C is not certified unless that substring is present.
- The tracker row says the duration was not measured. It does not claim a browser was certified and it does not claim a feature receipt. The AS_BUILT verification paragraph is the house template. It was not read as a gate receipt.
- The four type names in the `index.test.ts` parenthetical are `MatrixInput`, `MatrixObservation`, `MatrixStatus`, and `MatrixTier`. `certifyRow` is on the published runtime list, and that list matches `Object.keys` of the index module.
- `tsc --noEmit` and eslint on the four TypeScript files exited 0. The staged prose checker exited 0. The restored vitest run exited 0.

## Limitations and handoff

No browser was launched. No probe in `tools/tier-b` or `tools/swiftshader` was run. The matrix stays not-run, which is the commit rule, not a passed certification. The floor gate and the corpus gate were not run. `dist/` is ignored, so this pass did not rebuild the package output.

The unit file does not put exit code not 0 and a different resolved tier in one assertion. A direct probe of that input returned null, and each half is locked by its own test. The published callable check in `index.test.ts` does not call `certifyRow`. A deleted temporary test did. The type-export parenthetical is not a census of every type export in `index.ts`. The older omissions predate this diff. The observation-success branch in `rowsFrom` was not executed, because the committed file has no observation.

This pass does not block on the staged tree. It is not sprint readiness. The sprint push still needs a later sprint-profile record. Do not treat a not-run matrix as a certified browser.
