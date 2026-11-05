# F-221 review, pass 3

**Reviewed**: staged tree `1266492e414766c13ea6efc800b3c17baded5ce6` on `work/f-221-xai`, diffed against the merge base `27db5728dc4ef78876bd27a48665fb36b97c8d8b` with `sprint/s46`. No unstaged tracked paths. The untracked `tools/oracle/out` and the pass 1 and pass 2 reports are outside the review.
**Author / reviewer**: implementation by an external coding agent. The pass 1 D1 repair and the pass 2 D1 and D2 prose rewrites are by the sprint lead (Claude). Review by Claude Opus 5.5 as an independent `/microscope --working` subagent. It wrote none of the plan, the implementation or the rewrites.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 2 D2 replay, closed.** `docs/lld/typescript-packaging.md:28-29` now says "`@ocelli/react` excludes the two test globs". The two globs named just before are `src/**/*.test.ts` and `src/**/*.test.tsx`. `packages/react/tsconfig.json` has `"exclude": ["src/**/*.test.ts", "src/**/*.test.tsx"]`. The sentence matches.
- **Pass 2 D1 replay, closed.** Lines 63-64 now say only "Each test project names its own `tsBuildInfoFile` there, so the two never write one build info file". `packages/core/tsconfig.test.json` names `../../node_modules/.cache/ocelli-tsc/core-test.tsbuildinfo`, and `packages/react/tsconfig.test.json` names `.../react-test.tsbuildinfo`. Both are under the directory the previous sentence names. After `npm run typecheck` the cache holds exactly `core-test/`, `core-test.tsbuildinfo`, `react-test/`, `react-test.tsbuildinfo`. The false causal clause is gone, and nothing replaced it.
- **The dcmjs sentence, an independently built case.** The LLD says "Test files import `dcmjs` directly, and that ambient module declaration does not cross a project reference." I built a scratch config outside the worktree. It extends the core test project with the same references and drops only `src/dcmjs.d.ts` from `include`. `tsc --build` gave exit 1 with TS7016 "Could not find a declaration file for module 'dcmjs'" in `secondary-capture.test.ts`, `tid1500.test.ts` and `tid2010.test.ts`. Those are the three test files that import `dcmjs`. So the sentence holds, and leaving out the include would fail.
- **The tsbuildinfo trap, executed.** `rm -rf packages/react/dist` then `tsc --build --verbose packages/react` printed "is up to date because newest input ... is older than output 'packages/react/tsconfig.tsbuildinfo'" and left `dist` absent. That confirms the LLD's line 68-70 sentence on TypeScript 5.9.3. After that, `npm run build -w packages/react` wrote 12 entries. A planted `dist/stale.test.js` was gone after the next `build`. `npm run clean` removed both `dist` and `tsconfig.tsbuildinfo`, and the build that followed wrote `dist/index.js`.
- **The name rule, beyond the probe's own inputs.** `_is_test_artifact` gave True for `dist/a.test.d.ts.map`, `dist/x.spec.d.ts` and `dist/__tests__/a.js`. It gave False for `dist/test.js`, `dist/latest.js`, `dist/contest.d.ts` and `dist/foo__tests__/a.js`. The last one shows that the `__tests__` rule matches a whole segment and not a substring, as the LLD says ("a path with a `__tests__` segment").

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` at start and end | staged index | now | `1266492e...` both times |
| `cat` of both `tsconfig.json`, both `tsconfig.test.json`, root `tsconfig.json`, `tsconfig.base.json` | staged tree | now | as quoted above. Base sets no `jsx` |
| `npm run typecheck` | staged tree | now | 0. 48 `.test.js` in the cache, matching 48 tracked `*.test.ts(x)`. 3 `boundary.wasm.test.*` outputs. 0 `.test.` entries in either `dist` |
| scratch config without `src/dcmjs.d.ts` | scratchpad, refs the real core project | now | 1, TS7016 in the 3 dcmjs-importing tests |
| `rm -rf packages/react/dist` then `tsc --build --verbose` | staged tree | now | up to date, `dist` absent |
| react `build`, planted `stale.test.js`, `clean`, `build` | staged tree | now | 12 entries, plant gone, both removed, `index.js` present |
| `npx eslint node_modules/.cache/ocelli-tsc/core-test/ring.test.js` | staged tree | now | "File ignored by default because it is located under the node_modules directory" |
| `_is_test_artifact` on 9 paths | `scripts/package_check.py` | now | as listed above |
| `npm pack --dry-run --json`, core and react | after `npm run build` | now | 140 with 0 test, tsbuildinfo or node_modules paths, and 17 with 0 |
| `bin/ocelli.sh gate packages` | staged tree | now | 0, ALL GREEN |
| `scripts/prose_check.py` | staged tree | now | 0, 837 files clean |
| `scripts/source_provenance_check.py` | staged tree | now | 0, 1662 files clean |
| refusal mutations, AC5 old-source RED, lint plants, upstream-invalidation and stale-green plants | pass 1 and pass 2 | prior receipt | stands. The code is unchanged since pass 2 and only LLD prose moved |

Environment: macOS Darwin 27.0.0, TypeScript 5.9.3. The only worktree writes were ignored build output (`packages/react/dist`, the build info, the cache). The scratch config and its output stayed in the session scratchpad.

## Defects

None.

## Smells

None.

## Nitpicks

- `docs/lld/typescript-packaging.md:29` and `:64` are over-long source lines left over from the in-place rewrites ("... A compiled test is not a library output. Public modules are" and "... never write one build info file. `dist` stays the library output"). They render correctly.
- `docs/lld/typescript-packaging.md:50-52` says the two test projects "include only the test sources", and the next sentence says core's also includes two `.d.ts` files. Read together the paragraph is accurate, and core's `include` lists exactly those four entries. "include the test sources" would avoid the momentary contradiction.
- `.claude/plans/F-221-design.md`, decision 1, gives every `packages/<pkg>/tsconfig.json` the three-entry exclude. React now has two, a deliberate change after the pass 1 nitpick, and the LLD describes the tree correctly. The plan's quoted `refuses` text is also the pre-F-218 wording. That text was accurate when the plan was approved, and the plan is not in this diff.

## Verified clean

- Every factual sentence in the F-221 hunks of `docs/lld/typescript-packaging.md` (lines 3, 26-29, 50-76 and the new forbidden-content table row), checked against the tsconfig files, `vitest.config.ts` (its own globs, no tsconfig reference), `.gitignore:33` (`node_modules/`), ESLint's default node_modules ignore, the `package.json` scripts and the observed builds.
- The catalogue's probe comment, probe note and guard `notes` addition. The probe calls `check_tarball` for both `core` and `react` on the four paths, with `latest.js` and `contest.d.ts` in the control. A `.test.js`-only suffix rule would miss `index.test.d.ts`, and a bare `test` substring rule would refuse the control. The `refuses` text names "a test file". The runbook row 422 and its prose match the catalogue.
- The plan's "today" facts against the merge base: both library `tsconfig.json` files, the root solution file, both `build`/`clean` scripts, `files`, the `vitest.config.ts` block quoted verbatim, ESLint `allowDefaultProject` with three entries, and no `node:` import in non-test core source.
- `README.md`, `use.html`, `more.html`: no sentence about tarball contents, tsconfig or build scripts went stale. The one hit, `README.md:312`, describes the consumer proof, and F-221 does not change it.

## Limitations and handoff

- As the caller instructed, this pass did not repeat the pass 1 and pass 2 matrix (refusal mutations, AC5 RED, lint plants, incremental-settling plants). Those receipts stand because the staged code and configuration are unchanged since pass 2. The new material is two LLD sentences.
- I did not run `gate --sprint`, `gate guards`, `gate lint`, `npm run test` or a separate publish dry run in this pass. `gate packages` includes the publish dry run.
- This is a clean bounded review of the staged tree `1266492e...`. It is not sprint readiness. Next owner: the sprint lead, for `/verify` and `/complete-feature` on this tree. The nitpicks do not block.
