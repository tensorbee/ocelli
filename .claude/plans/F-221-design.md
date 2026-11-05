# F-221, Published package contents

**Status**: approved
**Epic ref**: E38.31
**Sprint**: S46
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "Published package contents: the @ocelli/core tarball stops shipping compiled test files, with a packages gate refusal for any test file in a published tarball". Layer TS, no declared dependency, milestone M20.

`allocation.json` notes: "Found while designing F-218 on 2026-11-05. npm pack lists 47 compiled *.test.js files under dist/ among 280 entries."

## Normative source, transcribed

The HLD says nothing about tarball contents. The normative sources are the packaging story and the existing gate.

### Story E1.3, as `scripts/package_check.py` states it

Module docstring, verbatim: "`tsc --build` proves the TypeScript compiles. It proves nothing about what a consumer receives, and the gap between those two is where packaging defects live. Every check here is about the second thing."

The forbidden-content rule, verbatim from the comment above it: "Never in the tarball. `src` and the build info are noise a consumer pays to download, and `.tsbuildinfo` additionally leaks absolute paths from the machine that built it."

```python
FORBIDDEN_PREFIXES = ["src/", "node_modules/"]
FORBIDDEN_SUFFIXES = [".tsbuildinfo"]
```

### The `packages` guard, `scripts/guards/catalogue.py`

`refuses`, verbatim: "A published package whose version skews from the workspace, an advertised exports path absent from the tarball, a tarball carrying sources or build state, and a consumer that cannot import or type-check what was published." Its probes are level 1 over the pure `check_tarball` and `check_manifest`, because the sandbox has no `node_modules`.

### The test runner, `vitest.config.ts`

```ts
include: [
  "packages/*/src/**/*.test.ts",
  "packages/*/src/**/*.test.tsx",
  "examples/*/src/**/*.test.ts",
  "examples/*/src/**/*.test.tsx",
],
exclude: ["packages/core/src/boundary.wasm.test.ts"],
passWithNoTests: false,
```

Vitest discovers tests by these globs and transforms them itself. It does not use a tsconfig `include` to find a test.

### The compiler configuration today

`packages/core/tsconfig.json`: `{"extends": "../../tsconfig.base.json", "compilerOptions": {"outDir": "./dist", "rootDir": "./src"}, "include": ["src/**/*"]}`. `packages/react/tsconfig.json` is the same with `"jsx": "react-jsx"` and `"references": [{"path": "../core"}]`. Both `package.json` files have `"build": "tsc --build"`, `"clean": "rm -rf dist"` and `"files": ["dist", "LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "README.md"]`. The root `tsconfig.json` is a solution file referencing `./packages/core`, `./packages/react` and `./examples/viewer-react`, and `npm run typecheck` is `tsc --build` over it. `tsconfig.base.json` sets `composite: true`. ESLint uses `parserOptions.projectService` with `allowDefaultProject` for three config files only, and the custom rule `ocelli/no-wasm-memory-view` refuses to run without project-service type information.

## Facts measured for this plan

Measured on 2026-11-05 on this branch with TypeScript 5.9.3, `npm pack --dry-run --json`, and a scratch project outside the repository.

1. `@ocelli/core` packs **281 entries**, of which **141 are test output**: 47 modules times `.js`, `.js.map` and `.d.ts`. The allocation note's 280 predates F-217's `NOTICE`. The 140 others are the 5 root files and 45 modules times 3.
2. **46 test sources are tracked.** The 47th module is `dist/cursor.probe.test.*`, dated 2 November, with no source anywhere in the tree. It is a stale output that `tsc --build` never removes.
3. `@ocelli/react` packs **23 entries, 6 of them test output**, `OcelliViewport.test` and `pump.test`. The story names `@ocelli/core`, and a refusal for "any test file in a published tarball" turns `@ocelli/react` red too, so both are fixed here.
4. **`tsc --build` does not re-emit a deleted `dist` while the `.tsbuildinfo` survives.** In the scratch project, `rm -rf dist` then `tsc -b --verbose` printed "is up to date because newest input ... is older than output ... tsconfig.tsbuildinfo" and left `dist` absent. So today's `clean` script followed by `build` yields an empty `dist`. A clean build must remove both.
5. **A split configuration works with every consumer.** In the scratch project, a library `tsconfig.json` that excludes `src/**/*.test.ts` plus a `tsconfig.test.json` that includes only tests and references the library, both referenced from the root solution file, gave: `tsc -b` exit 0 with no test output in `dist`, a type error planted in the test file reported by `tsc -b`, a dependent package referencing `../core` unchanged, and ESLint `projectService` applying a type-aware rule to the test file.
6. `packages/core/src/node-test-modules.d.ts` declares `node:fs`, `node:path` and `node:url` "for the package tests". No non-test source in `packages/core/src` imports a `node:` module.

## What the specification does not cover

Every item is this plan's decision, made by the lead under the operator's delegation of 2026-11-05.

1. **Exclude tests from the library project and typecheck them in a sibling project.** `packages/<pkg>/tsconfig.json` stays the library project that `dist` is built from, gains `"exclude": ["src/**/*.test.ts", "src/**/*.test.tsx", "src/node-test-modules.d.ts"]`, and keeps every other field. A new `packages/<pkg>/tsconfig.test.json` extends the base, includes the test files and `node-test-modules.d.ts` for core, references `./tsconfig.json` (and `../core/tsconfig.json` for react), and emits to `../../node_modules/.cache/ocelli-tsc/<pkg>-test`, which is ignored, never packed and never linted. The root solution file gains both test projects. The library project keeps the name `tsconfig.json`, so `packages/react`, `examples/viewer-react` and every reference to `../core` resolve unchanged.
2. **The build is clean.** Each `build` script becomes `rm -rf dist tsconfig.tsbuildinfo && tsc --build`, and each `clean` script removes both, which fixes fact 4's trap. `rm -rf` matches the existing `clean` script. A stale output cannot survive a build, so fact 2's file goes.
3. **The refusal is a name rule over the packed list**, added to `check_tarball` beside the existing prefix and suffix rules. A path is a test file when its basename contains `.test.` or `.spec.`, or a path segment is `__tests__`. The rule matches `.js`, `.js.map`, `.d.ts` and `.d.ts.map` alike, because the marker is in the stem. The message names the package and the path.
4. **No `files` negation.** A `"!dist/**/*.test.*"` entry would hide the output while still building it, and a stale or renamed test would ship the day the pattern stopped matching. The refusal is the backstop, the build exclusion is the fix.
5. **No orphan-output check.** A check that every `dist` module has a tracked source would also catch fact 2, but the clean build removes the cause and the test-name refusal catches the observed instance. Recorded as considered and declined.

## Problem and reproducer

```bash
cd packages/core && npm pack --dry-run --json | python3 -c "import json,sys;f=[x['path'] for x in json.load(sys.stdin)[0]['files']];print(len(f),sum('.test.' in p for p in f))"
# 281 141
cd packages/react && npm pack --dry-run --json | python3 -c "import json,sys;f=[x['path'] for x in json.load(sys.stdin)[0]['files']];print(len(f),sum('.test.' in p for p in f))"
# 23 6
bin/ocelli.sh gate packages      # green today, which is the defect
```

Expected after this story: zero test entries in either tarball, `gate packages` red if one appears, and tests still typechecked and linted.

## Scope, API and compatibility

In scope: the two package `tsconfig.json` files, two new `tsconfig.test.json` files, the root `tsconfig.json`, the `build` and `clean` scripts of both packages, the refusal in `scripts/package_check.py`, its guard probe, and the docs.

Non-goals:

- `examples/viewer-react`. It is private and never packed.
- The consumer proof, the publish dry run and F-218's advisory checks in `package_check.py`. Unchanged.
- What `@ocelli/core` should ship beyond `dist`. The wasm module is not in the tarball today, and that is F-220's concern and RELEASE.md step 6's, not this story's.

No public API changes. A consumer of `@ocelli/core` loses only files it never imported. `exports`, `main` and `types` are unchanged.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `npm run build`, both packages | `rm -rf dist tsconfig.tsbuildinfo && tsc --build` over `tsconfig.json` | `package_check.py` runs it before `npm pack` | `dist` with library modules only |
| `npm run typecheck`, `gate types`, CI TypeScript job | root solution file, now with both test projects | Every source and test file typechecked once | Exit status, and no test output in `dist` |
| `npm run lint`, `gate lint` | ESLint `projectService` | Test files found through the root solution's references | Type-aware rules apply to test files |
| `npm run test`, `gate packages` first half | `vitest.config.ts` globs | Unchanged | Same test files and test count |
| `vitest.boundary.config.ts` | Its own include | Unchanged | Unchanged |
| `gate packages`, `check_tarball` | New test-file name rule | Both packages | A refusal naming the path |
| `npm pack` and `npm publish` | `files: ["dist", ...]` | Unchanged `files` | Tarball without test output |

## State transitions and invariants

Not applicable. Builds and checks are functions of the tree. The invariant that spans consumers: `dist` is written only by the library project, whether `build` or `typecheck` ran last, because the test projects emit under `node_modules/.cache`.

## Ownership, commit and recovery

Not applicable. No runtime resource.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `@ocelli/core` tarball has 0 test entries and exactly the 140 non-test entries of fact 1 | `npm pack --dry-run --json` count before and after, in the handoff | Excluding `.test.js` only, which ships `.d.ts` and `.js.map` | Implementation |
| AC2 | `@ocelli/react` tarball has 0 test entries and 17 entries | same, react | Fixing core only | Implementation |
| AC3 | A clean build drops stale output. A file `dist/stale.test.js` planted by hand is absent after `npm run build`, and `dist/index.js` is present after `npm run clean && npm run build` | manual observation recorded, then `gate packages` | `rm -rf dist` alone, which leaves an empty `dist` (fact 4) and fails the exports check | Implementation |
| AC4 | Refusal. `check_tarball` given a name list containing `dist/index.test.js`, `dist/index.test.d.ts`, `dist/x.spec.js` and `dist/__tests__/a.js` returns a problem naming each of the four paths. A healthy list that also carries `dist/latest.js` and `dist/contest.d.ts` returns none | probe `packages.test-file-in-tarball`, level 1, its message asserted, and its control over the healthy list | A suffix rule on `.test.js` only. A rule applied to core only. A rule that matches `latest.js` or `contest.js` by substring `test` | guards |
| AC5 | Old-source RED. On today's tree with the refusal added and the build unchanged, `gate packages` fails naming `dist/alongside.test.js`. With the build fixed it passes | recorded in the handoff | A refusal that never ran against the real tarball | Implementation, recorded |
| AC6 | Tests are still typechecked. A type error planted in `packages/core/src/ring.test.ts` and in `packages/react/src/pump.test.ts` turns `npm run typecheck` red, reverted green. `boundary.wasm.test.ts` is in the core test project | recorded RED and GREEN | Dropping tests from every project, which `tsc` then never sees | Implementation, recorded |
| AC7 | Tests are still linted with type information. `npm run lint` passes with no "not found by the project service" error. A planted `new Uint8Array(wasm.memory.buffer)` in `packages/react/src/pump.test.ts` turns `gate lint` red with the HLD 17.2 message, reverted green | recorded RED and GREEN | A test project the project service cannot find, which silently disables the type-aware rule | Implementation, recorded |
| AC8 | `npm run typecheck` writes no test output to either `dist`. After it, `ls packages/core/dist packages/react/dist \| grep -c '\.test\.'` is 0 | observation | A `tsconfig.build.json` for the build while `tsconfig.json` still emits tests into `dist` on typecheck | Implementation |
| AC9 | Vitest unchanged. The number of test files and tests reported by `npm run test` is equal before and after | counts in the handoff | A tsconfig change that drops a test from discovery or breaks its JSX transform | Implementation |
| AC10 | `gate packages`, `gate types`, `gate lint` and `gate guards` pass on the final tree, including F-218's checks | gates | A fix that breaks F-218's consumer lock check | Implementation |
| AC11 | Guard registration. `packages` guard `refuses` text names a test file. `ci/guard-probe-budget.json` re-recorded, runbook regenerated with `--render-runbook`, `--check-runbook` clean | `gate guards` | A refusal with no probe | guards |

## Approach

1. Record the AC1, AC2 and AC9 baselines.
2. Add the test-file rule to `check_tarball` and its unit test. Run `gate packages` on the unchanged build and keep the red output (AC5).
3. Edit the two `tsconfig.json` files, add the two `tsconfig.test.json` files, add them to the root solution file.
4. Change the `build` and `clean` scripts.
5. Run AC3 and AC6 to AC9, then the gates.
6. Add the probe and its control, re-record the budget, regenerate the runbook. No `scripts/tests/test_package_check.py` exists, and the `packages` arm of `bin/ocelli.sh` runs no Python unit test, so the level-1 probe is the rule's permanent test, as it is for the guard's two existing refusals. `bin/ocelli.sh` is not edited.

Rejected alternatives:

- A `files` negation, decision 4.
- A `tsconfig.build.json` used by `build`, with `tsconfig.json` left including tests. Typecheck would still emit tests into `dist`, which AC8 rejects.
- A solution-style `packages/<pkg>/tsconfig.json` with `tsconfig.lib.json` beside it. It works (fact 5 tried both), and it renames the project that `packages/react` and `examples/viewer-react` reference, which is more change for the same result.
- `noEmit` on the test projects. A referenced composite project emits, and the scratch run did not establish that `tsc --build` accepts `noEmit` there. Emitting to an ignored cache directory is measured to work.

## Dependencies, write ownership and resources

No declared dependency. Write set:

```text
packages/core/tsconfig.json, packages/react/tsconfig.json
packages/core/tsconfig.test.json, packages/react/tsconfig.test.json   new
tsconfig.json
packages/core/package.json, packages/react/package.json               scripts only
scripts/package_check.py                                              check_tarball, one rule
scripts/guards/catalogue.py, ci/guard-probe-budget.json, docs/runbooks/guard-verification.md
docs/lld/typescript-packaging.md
```

**Shared with F-218**, in progress on `work/f-218-xai` in `../ocelli-wt-f-218`: `scripts/package_check.py`, `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`, `docs/runbooks/guard-verification.md` and `docs/lld/typescript-packaging.md`. F-218 adds consumer-lock and advisory functions to `package_check.py`, and F-221 adds one rule inside `check_tarball`. **Order: F-221 starts after F-218 is integrated into `sprint/s46`**, and is based on that tree. The budget and runbook are regenerated by F-221 after F-218's entries exist.

Shared with F-220: `scripts/package_check.py` (F-220 adds a notices rule to `check_tarball` and `REQUIRED`), `packages/core/package.json`, the catalogue, the budget and the runbook. **F-220 follows F-221.** Sprint order for these three files: F-218, then F-221, then F-220.

No GPU, browser or corpus. `npm pack` needs no network. `gate packages` runs `npm install` of local tarballs and `npm publish --dry-run`, as today.

## Verification and evidence reuse

```bash
bin/ocelli.sh gate lint types packages
bin/ocelli.sh gate guards prose
```

Final profile: `bin/ocelli.sh gate --sprint`. Identities: TypeScript 5.9.3, the vitest version in `package-lock.json`, the packed entry counts. Evidence is invalidated by any change to a tsconfig, a `build` script, `files`, or `check_tarball`.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, packaging and build configuration
- Tier B (WebGL2): n/a, packaging and build configuration
- Tier C (CPU): n/a, packaging and build configuration

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| guard probe | AC4, the test-file rule in the floor sandbox, and its non-matches in the control | `scripts/guards/catalogue.py` |
| integration | AC1 to AC3, AC5 to AC10 through the real build, pack, typecheck, lint and test | `gate packages`, `gate types`, `gate lint`, handoff records |

No `fixture` row is owed. No pixel or geometry arithmetic.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no packaging row.

## Deviations

None.

## LLD impact

`docs/lld/typescript-packaging.md`, "What is in the `@ocelli/core` tarball today": "Everything under `packages/core/src` compiles into `dist`" becomes library modules only, the test projects and their cache output directory, the clean build and why `.tsbuildinfo` goes with `dist`, and the refusal.

## User-facing documentation impact

No change to `README.md`, `examples/folder-viewer/src/use.html` or `examples/folder-viewer/src/more.html`. None of them lists tarball contents or the build scripts, and no public module, export or command changes.

## Open questions

None open.

- Q1, also fix `@ocelli/react`. Answered: lead decision under the operator's delegation of 2026-11-05. Yes, fact 3.
- Q2, build exclusion against `files` negation. Answered: lead decision under the operator's delegation of 2026-11-05. Build exclusion with a refusal behind it, decisions 1 and 4.
- Q3, clean build. Answered: lead decision under the operator's delegation of 2026-11-05. Yes, removing `.tsbuildinfo` with `dist`, decision 2 and fact 4.
