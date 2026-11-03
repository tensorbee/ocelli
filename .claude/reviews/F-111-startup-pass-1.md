# F-111 review, pass 1

**Reviewed**: Uncommitted worktree on `sprint/s41`. HEAD is `51934d8e4827ce4686a3d3156582d441b331de86`. `packages/core/src/startup.ts` and `packages/core/src/startup.test.ts` are untracked. The other F-111 edits are unstaged. This file is untracked. Nothing was staged or committed.

**Author / reviewer**: uncommitted implementation / 01a1a16b-2bec-7850-a215-b220ac88bcca / independent pass 1. This reviewer did not write the change.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC8 were checked against `.claude/plans/F-111-design.md` and HLD section 26 in `docs/hld/23-performance-rules.md`. Section 26 says to measure with the benchmark harness before optimising anything. It does not name a decode artefact, a release-only sum, or a 2.3 ms input. The path names, the release-only rule, and the choice not to replace the 2.3 ms baseline are approved local decisions. They are not HLD formulas. No file in this diff closes gate A4 or edits a baseline.

The unit file locks these cases.

- Role `render` returns `pkg/ocelli_wasm_bg.wasm`.
- Role `decode` returns `pkg-decode/ocelli_wasm_bg.wasm`.
- The render result does not contain `pkg-decode`.
- The decode result does not contain `pkg/ocelli_wasm_bg.wasm`.
- Role `native` and an empty role return null.
- Release phases 1, 2, 4, 8 and 16 sum to 31.
- A dev profile returns null for the sum and for the first frame.
- A negative phase returns null. `NaN` returns null.
- The source text does not contain `bench-baseline` or `2.3`.
- A release interval of 4.5 is returned unchanged. Zero is returned. Positive infinity and -1 return null.

An independent probe of the restored functions was run and was not added to the tree.

- `RENDER`, `render ` with a trailing space, and `undefined` return null. `Release` and `release ` return null.
- Positive infinity and negative infinity on one phase return null. A missing `glue` field returns null.
- Five zeros sum to 0. `glue` of negative zero is kept in the sum, so 2, 4, 8 and 16 with that `glue` total 30. A first-frame negative zero is returned as negative zero, not coerced to 0.
- `0.1` plus `0.2` is the raw sum. The five-phase fixture does not total 2.3.
- A record keyed as `glue_module_script` and `first_call` returns null. Those are the runner names. The plan's fields are `glue` and `firstCall`. This story does not read the runner.

`RENDER_ARTIFACT` was changed from `pkg/ocelli_wasm_bg.wasm` to `pkg/not-the-render-module.wasm`. `names the render module` failed. Expected `pkg/ocelli_wasm_bg.wasm`. Received `pkg/not-the-render-module.wasm`. Exit 1. The file was restored to the pre-mutation bytes. sha256 `74db04ecef0cea8b9316325b1209b5d70e00f64a2308511120b11b11c2572288` matched before the edit and after the restore. The restored suite exited 0.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git status --short` and `git rev-parse HEAD` | Worktree before review | Executed now | Exit 0. HEAD `51934d8e4827ce4686a3d3156582d441b331de86`. Six modified tracked files and two untracked module files. No `ci/bench-baseline.json` and no `ci/wasm-size-budget.json`. |
| Node probe of restored `startup.ts` | Worktree functions, 30 inputs after the two negative-zero observations | Executed now | Exit 0. Paths, refusals, the raw float sum, and a null result for runner keys matched the plan. |
| `npx vitest run packages/core/src/startup.test.ts` after `RENDER_ARTIFACT` named `pkg/not-the-render-module.wasm` | Mutated worktree, not staged | Executed now | Exit 1. 1 failed and 13 passed. Failure at `startup.test.ts:29`. |
| Same file restored, then `npx vitest run` on `startup.test.ts` and `index.test.ts` | Restored worktree | Executed now | Exit 0. 18 passed. 14 in `startup.test.ts` and 4 in `index.test.ts`. |
| `npx tsc -p packages/core --noEmit --pretty false` | Worktree sources before the mutation | Executed now | Exit 0. |
| `npx eslint` on `startup.ts`, `startup.test.ts`, `index.ts`, and `index.test.ts` | Those four files, before the mutation | Executed now | Exit 0. ESLint 9.39.5. |
| `python3 scripts/prose_check.py` | Tracked markdown in scope, worktree text | Executed now | Exit 0. 719 files. `CHANGELOG.md` is outside that checker and was read directly. |
| Python compare of F-ID headers with the last cell of each README row | Worktree `public-api.md`, `benchmarks.md`, and `docs/lld/README.md` | Executed now | Both pairs are equal. Both public-api strings end with F-111. Both benchmarks strings end with F-111. |
| `git status --short` after restore | Worktree | Executed now | Same six modified files and the same two untracked module files. The mutation string is absent. |

Environment: Node v24.13.0. Vitest 4.1.11. No cargo, rustc, or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- AC1. `startupArtifact("render")` is `pkg/ocelli_wasm_bg.wasm`. The render string does not contain `pkg-decode`.
- AC2. `startupArtifact("decode")` is `pkg-decode/ocelli_wasm_bg.wasm`. That string does not contain `pkg/ocelli_wasm_bg.wasm`. The substring check is not vacuous.
- AC3. An unknown role returns null. The probe also refused a different capitalisation, a trailing space, and `undefined`. None of those fell through to the render path.
- AC4. The five release phases are added in the order `glue`, `fetch`, `compile`, `instantiate`, `firstCall`. The fixture total is 31, so a dropped phase would miss that literal. The source does not add 2.3 and does not round the `0.1` plus `0.2` case.
- AC5. A dev profile, a negative phase, `NaN`, and either infinity return null. They are not coerced to 0. A negative zero is finite and compares as at least 0, so it stays in the sum.
- AC6. `summariseFirstFrame("release", 4.5)` is 4.5. Zero is 0. Negative zero is returned unchanged. No startup constant is added.
- AC7. A dev profile, positive infinity, negative infinity, `NaN`, and -1 return null for the first frame.
- AC8. `index.ts` exports `startupArtifact`, `summariseColdStart`, and `summariseFirstFrame`. The published list names those three. The restored index test matched `Object.keys` of the module.
- `startup.ts` does not contain `addEventListener`, `wasm-bindgen`, `unsafe`, `bench-baseline`, or `2.3`. It does not fetch a file and it does not read `ci/bench-baseline.json`.
- The functions allocate no wasm, open no worker, and write no baseline. The five-element array in the sum is not on a render loop. There is no pixel read-back and no tier argument. Tiers A, B and C stay not applicable.
- `docs/lld/public-api.md` states both paths, null for any other role, the five fields, release plus finite and at least 0, the same rule for the first frame, and that the functions do not read the 2.3 ms baseline and do not close gate A4.
- `docs/lld/benchmarks.md` says F-111 is not a replacement of the 2.3 ms figure, names both artefact paths, and says gate A4 stays open. The earlier sentence that 2.3 ms remains the F-006 regression baseline is still there.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-111. The benchmarks header equals the last cell of the benchmarks row. Both end with F-111.
- `docs/hld/A-spike-gates.md` is not in the diff. Its A4 row still says the question is unmeasured. `ci/bench-baseline.json` still holds `wasm.cold_start` at 2.3 and is not modified. `ci/wasm-size-budget.json` is not modified.
- The changelog bullet names the render or decode path, a release sum of five finite phases, an unchanged release interval, a dev profile that returns nothing, and the 2.3 ms baseline staying. The public-api section is the full predicate.
- `npx tsc -p packages/core --noEmit` exited 0. eslint on the four TypeScript files exited 0. The tracked prose checker exited 0.

## Limitations and handoff

No browser was launched. `tools/bench/src/runners/wasm_cold_start.mjs` was not run and was not edited. A caller who passes that runner's phase record gets null, because `glue_module_script` and `first_call` are not `glue` and `firstCall`. The plan leaves the runner unchanged and names the function fields itself. The null is a refusal, not a sum of the wrong keys.

The integer fixture does not lock addition order. Reordering those five integers would still total 31. The source adds them in the plan's order. The float probe checked the unrounded sum, not a reordered sum.

`index.test.ts` checks that the three names are published. It does not call them. `startup.test.ts` calls them from the module. The type-export parenthetical does not name `ColdPhases` or `StartupRole`. That parenthetical is not a census of every type export.

The floor gate and the corpus gate were not run. This pass did not stage the work, so an index hash would not see the untracked module. This pass does not block on the worktree. It is not a sprint-profile record. Do not treat the untouched 2.3 ms figure as a measurement of the current module, and do not treat an open A4 row as a closed gate.
