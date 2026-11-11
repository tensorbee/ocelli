# F-221 review, pass 1

**Reviewed**: staged tree `9138d104af7f9b767aceefed1f3b778d77a56924` on `work/f-221-xai`, HEAD and merge-base `27db5728dc4ef78876bd27a48665fb36b97c8d8b`. No unstaged tracked paths. The untracked `tools/oracle/out` symlink is outside the review.
**Author / reviewer**: implementation by an external coding agent (per the caller). Review by Claude Opus 5.5 as an independent `/microscope --working` subagent, which did not author the plan or the implementation.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- **AC1, AC2.** Fresh `npm run build`, then `npm pack --dry-run --json` per package, counting basenames with `.test.`, `.spec.` or a `__tests__` segment: core 140 entries, 0 test, react 17 entries, 0 test. Top-level members are only `dist` and the five root files. Rejects "exclude `.test.js` only", which would leave `.d.ts` and `.js.map` in the count.
- **AC3.** Planted `dist/stale.test.js` and an orphan `dist/orphan.js` beside the stale `tsconfig.tsbuildinfo` left by an old-config build. `npm run build` in `packages/core` exit 0, both planted files gone, 0 test entries, 135 files. The trap: `rm -rf dist` with `tsconfig.tsbuildinfo` kept, then `npm run build`, gave 135 files and `dist/index.js`. `npm run clean` removed both `dist` and `tsconfig.tsbuildinfo`, and the build after it gave 135 files and `dist/index.js`. So `dist` is not empty after clean then build. Plain `npx tsc --build --verbose` after `rm -rf dist` printed "is up to date because newest input 'src/sr.ts' is older than output 'tsconfig.tsbuildinfo'" and left `dist` absent, which confirms the LLD's fact-4 sentence.
- **AC4.** The probe passes. Seven mutations of `scripts/package_check.py`, each restored from a byte copy: the rule disabled, a `.test.js` suffix rule, a substring `test` rule (only the control went red), core only, the `__tests__` branch dropped, `.spec.` dropped, and the package name dropped from the message. Each turned `guard_probe.py --only packages.test-file-in-tarball` FAIL. So every case the plan names discriminates.
- **AC5.** In the working tree only, I wrote the base versions of both `tsconfig.json`, the root `tsconfig.json` and both `package.json` files back and removed both `tsconfig.test.json`, with the new refusal kept. `package_check.py --no-publish-dry-run` printed FAIL with 144 "which is a test file" lines, core 46 x 3 plus react 2 x 3, the first being `dist/alongside.test.js`. Restored with `git checkout --` from the index.
- **AC6.** I appended `export const planted: number = "not a number";` to `ring.test.ts`, `boundary.wasm.test.ts`, `pump.test.ts` and `OcelliViewport.test.tsx` in turn. Each gave TS2322 at the planted line and `tsc --build` exit 2. Each was reverted, and the green run exit 0. Dropping `src/dcmjs.d.ts` from the core test project gave TS7016 in `secondary-capture.test.ts`, `tid1500.test.ts` and `tid2010.test.ts`, which confirms the LLD's claim that the ambient declaration does not cross the reference.
- **AC7.** `npm run lint` exit 0, with no project-service error. A function returning `new Uint8Array(wasm.memory.buffer)` planted in `pump.test.ts` and in `OcelliViewport.test.tsx` gave `ocelli/no-wasm-memory-view` with the HLD 17.2 message. The same plant in `ring.test.ts` raised nothing, because `eslint.config.js` turns the rule off for `packages/core/src/*.test.ts` on purpose. That is not a regression. Lint exit 0 with no "not found by the project service" error for any core test file shows the project service finds them. `gate lint` ALL GREEN after the revert.
- **AC8.** I removed both `dist` directories, both library `.tsbuildinfo` files and `node_modules/.cache/ocelli-tsc`, then ran `npm run typecheck` (exit 0). Result: 0 test entries in either `dist`, core 135 and react 12 files, `core-test` 138 files (46 x 3, so `boundary.wasm.test.ts` is in it) and `react-test` 6 files. `git check-ignore -v` attributes the cache to `.gitignore:33:node_modules/`.
- **AC9.** `npm run test` on the staged configuration gave 57 files and 468 tests. With the base configuration written back into the working tree it also gave 57 files and 468 tests.
- **AC10, AC11.** The gates are in the evidence table. The runbook row 422 and the guard limit sentence are present, and the budget `packages: 28` is accepted by `gate guards`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run test` | staged tree, then base config in the working tree | now | 0, 57 files / 468 tests both ways |
| `package_check.py --no-publish-dry-run` | base config plus the new refusal | now | 1, 144 test-file problems |
| `npm run build`, `npm run clean` with planted files, in `packages/core` | staged tree | now | 0, plants gone, 135 files, `dist/index.js` present |
| `npm run typecheck` from an empty cache | staged tree | now | 0, 0 test entries in `dist` |
| `tsc --build` with 4 planted type errors | staged tree plus one line each | now | 2 each, 0 after revert |
| `eslint` with 3 planted views | staged tree plus one function each | now | red in react tests, off by config in core tests |
| `guard_probe.py --only packages.test-file-in-tarball`, 7 mutations | staged tree | now | 0 unmutated, FAIL under every mutation |
| `npm pack --dry-run --json -w packages/core`, `-w packages/react` | after `npm run build` | now | 140/0 and 17/0 |
| `bin/ocelli.sh gate packages` | staged tree | now | 0, "OK: 2 package(s) at 0.1.0 ..." and unittest OK |
| `bin/ocelli.sh gate guards` | staged tree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate types`, `gate lint`, `gate prose` | staged tree | now | 0 each |
| `guard_census.py`, `guard_census.py --check-runbook` | staged tree | now | 0, "0 watched by nothing", runbook 0 |
| `prose_check.py`, `source_provenance_check.py` | staged tree | now | 0, 837 files and 1662 files clean |
| `tsc --build --verbose` twice | staged tree | now | both test projects "out of date" on every run, see D1 |

Environment: macOS Darwin 27.0.0, TypeScript 5.9.3, vitest 4.1.11. After every mutation `git status --short` showed only the staged set plus `?? tools/oracle/out`, and the final `git write-tree` is `9138d104af7f9b767aceefed1f3b778d77a56924`, the same as at the start.

## Defects

### D1, the two test projects share one `.tsbuildinfo`, so neither is ever up to date

**Where**: `packages/core/tsconfig.test.json:4-5` and `packages/react/tsconfig.test.json:4-5`

**What**: Both projects set `rootDir: ./src` and an `outDir` under `node_modules/.cache/ocelli-tsc/<pkg>-test`, with no `tsBuildInfoFile`. TypeScript puts the build info at `outDir` plus the config path relative to `rootDir`, which is `../tsconfig.test` here. So both projects write `node_modules/.cache/ocelli-tsc/tsconfig.test.tsbuildinfo`, a single file that is present in the cache now. Each build overwrites the other project's state. `npx tsc --build --verbose`, run twice in a row on an unchanged tree, prints the same two lines both times:

```text
Project 'packages/core/tsconfig.test.json' is out of date because buildinfo file 'node_modules/.cache/ocelli-tsc/tsconfig.test.tsbuildinfo' indicates that file 'packages/react/src/pump.test.ts' was root file of compilation but not any more.
Project 'packages/react/tsconfig.test.json' is out of date because buildinfo file 'node_modules/.cache/ocelli-tsc/tsconfig.test.tsbuildinfo' indicates that file 'packages/core/src/alongside.test.ts' was root file of compilation but not any more.
```

**Why it is wrong**: The plan's decision 1 gives each test project its own output location, `<pkg>-test`, and that is false for the build state. `tsc --build` is an incremental build, and two projects writing one state file break its up-to-date contract on every run. It does not hide a type error today, because both projects always rebuild. It is still two writers sharing one file, and the result depends on build order, which the LLD's "emit under `node_modules/.cache/ocelli-tsc/`" paragraph does not mention.

**Evidence**: I added `"tsBuildInfoFile": "../../node_modules/.cache/ocelli-tsc/<pkg>-test.tsbuildinfo"` to both files in the working tree only. With that, the second `--verbose` run reported both projects "up to date", and a no-op `tsc --build` went from 0.94s to 0.21s wall clock. I then reverted with `git checkout --` and deleted the two probe build-info files.

**Scope and regression**: This affects `npm run typecheck`, `gate types`, the CI TypeScript job and `tools/bench`'s `npx tsc --build`. It is present on the staged tree only, because the base has no test projects. The smallest regression is two consecutive `tsc --build --verbose` runs, with the second printing no "out of date" line for either test project. Nothing in the gates watches this today. Fix it with a per-project `tsBuildInfoFile`, or an `outDir` under which the computed path is distinct, and add one LLD sentence saying where each build info lives.

## Smells

None.

## Nitpicks

- `packages/react/tsconfig.json` excludes `src/node-test-modules.d.ts`, which does not exist in `packages/react/src`. It is harmless, and it follows the plan's "same exclude". It reads as if react had such a file.

## Verified clean

- The refusal rule in `_is_test_artifact` matches on a basename segment and a `__tests__` path segment, not on a substring. The probe and its control discriminate all seven mutations listed above.
- Consumers, by grep of `packages/(core|react)/dist`, `tsconfig`, `tsbuildinfo` and the build scripts outside `docs/hld`:
  - The CI order is lint, then typecheck, then `gate packages`, and all three are green.
  - `tools/bench/src/browser_session.mjs` hashes `packages/core/dist` and `packages/core/tsconfig.json`. The bench fingerprint changes because test files left `dist`, and the bench re-records it, so nothing breaks.
  - The `LIBRARY_FILES` allowlist of F-213 and F-215 serves non-test modules only.
  - `examples/viewer-react` references `../../packages/core` and `../../packages/react` unchanged.
  - `use.html:266` ("tsc for the packages") is still true.
  - `README.md`, `use.html` and `more.html` make no claim about tarball contents or `clean`.
- Every new sentence in `docs/lld/typescript-packaging.md` was executed: the exclude, the test-project includes, the dcmjs reference claim, the jsx claim, the root references, "Vitest ... does not read these projects", "ignored, never packed and never linted", "`dist` stays the library output", the stale-`.tsbuildinfo` trap and the refusal's wording. Each holds.
- The catalogue comment, the probe note, the `refuses` text and the `limit` sentence match the probe's observed behaviour. The runbook regenerates clean.

## Limitations and handoff

- `npm publish --dry-run` ran only inside `gate packages`, and that gate was green.
- `gate --sprint` was not run.
- No GPU, browser or corpus path is touched by this story, and none was exercised.
- I checked vitest's tsconfig resolution only through the unchanged counts and the green JSX test. I did not trace Vite's tsconfig lookup.
- Next owner: the implementer fixes D1. A fresh independent pass then reviews the two `tsconfig.test.json` edits, any LLD sentence added, and the AC6 and AC8 runs repeated on the new tree.
