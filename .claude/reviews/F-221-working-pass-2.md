# F-221 review, pass 2

**Reviewed**: staged tree `92f19a163d73312eb9ba5ee9415a74f5e7ce5afb` on `work/f-221-xai`. No unstaged tracked paths. The untracked `tools/oracle/out` and `.claude/reviews/F-221-working-pass-1.md` are outside the review.
**Author / reviewer**: implementation by an external coding agent, with the D1 repair by the sprint lead (Claude). Review by Claude Opus 5.5 as an independent `/microscope --working` subagent. It did not author the plan, the implementation or the repair.
**Result**: 2 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 1 D1 replay, closed.** I emptied `node_modules/.cache/ocelli-tsc` (which still held pass 1's orphan `tsconfig.test.tsbuildinfo`), both `dist` and every library `.tsbuildinfo`. Then I ran `npx tsc --build --verbose` twice. The first run built all five projects. The second printed "is up to date because newest input ... is older than output" for all five, with `core-test.tsbuildinfo` and `react-test.tsbuildinfo` as the two test outputs. The cache now holds exactly `core-test/`, `core-test.tsbuildinfo`, `react-test/`, `react-test.tsbuildinfo`.
- **Only the touched project responds.** `touch packages/core/src/ring.test.ts` changed only `packages/core/tsconfig.test.json` ("needs to update timestamps"). The other four stayed up to date. `touch packages/react/src/pump.test.ts` changed only `packages/react/tsconfig.test.json`.
- **Stale green, rejected.** From settled state, I appended `export const planted: number = "not a number";` to `ring.test.ts`, `boundary.wasm.test.ts`, `OcelliViewport.test.tsx` and `pump.test.ts` in turn. `npm run typecheck` gave exit 1 and exactly one TS2322 at the planted line each time. After each restore it gave exit 0. With the plant left in `pump.test.ts`, two consecutive `tsc --build` runs both gave exit 1, so the build info does not record an errored project as settled.
- **Upstream-only edit, rejected.** I settled green with `export const plantedLib = 1;` in `packages/core/src/index.ts`, consumed as `number` in `ring.test.ts` (relative import) and in `pump.test.ts` (through `@ocelli/core`). Five projects were up to date. Then I changed only the library line to `"s"`. `tsc --build` reported TS2322 in both test files. So a library change invalidates both downstream test projects through the references, and neither keeps a stale green.
- **Clean build from an empty cache.** With the cache, `dist` and the library `.tsbuildinfo` files removed, `npm run build` gave exit 0, created no cache directory and wrote 135 core `dist` entries. `npm run typecheck` then gave exit 0, with `core-test` holding 138 files and `react-test` 6. No `.tsbuildinfo` and no `.test.` file in either `dist`. `git check-ignore -v` attributes `core-test.tsbuildinfo` to `.gitignore:33:node_modules/`.
- **Tarball.** `npm pack --dry-run --json` gave core 140 entries, 0 test paths, 0 `tsbuildinfo` paths, and react 17, 0, 0.
- **The new LLD sentence, executed.** See D1. The repair holds for this repository, but the stated cause is false in general.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx tsc --build --verbose` x2, empty cache | staged tree | now | 0, second run: 5 of 5 up to date |
| `touch` one core test, then one react test, `--verbose` | staged tree | now | only the touched test project updates timestamps |
| `npm run typecheck`, 4 planted TS2322 | staged tree plus one line each | now | 1 each, one error at the plant, 0 after restore |
| `tsc --build` x2 with a plant present | staged tree plus one line | now | 1 and 1 |
| library-only type change after settling | staged tree plus 3 lines | now | TS2322 in `ring.test.ts` and `pump.test.ts` |
| scratch `tsc --build` experiment, 4 layouts | TypeScript 5.9.3, scratchpad | now | collision only in the repo's layout, see D1 |
| `npm run build`, `typecheck`, `lint`, `test` | staged tree | now | 0 each, 57 files / 468 tests |
| `npm pack --dry-run --json -w packages/core`, `-w packages/react` | after `npm run build` | now | 140/0 and 17/0 |
| `bin/ocelli.sh gate packages` | staged tree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | staged tree | now | 0, ALL GREEN |
| `guard_census.py`, `guard_census.py --check-runbook` | staged tree | now | 0 each, "0 watched by nothing" |
| `prose_check.py` | staged tree | now | 0, 837 files clean |
| `source_provenance_check.py` | staged tree | now | 0, 1662 files clean |

Environment: macOS Darwin 27.0.0, TypeScript 5.9.3. I restored every plant from a byte copy. `git status --short` at the end shows the same staged set, and `git write-tree` is `92f19a163d73312eb9ba5ee9415a74f5e7ce5afb`, the same as at the start. The scratch experiment directory was deleted.

## Defects

### D1, the new LLD sentence gives a false cause for the collision

**Where**: `docs/lld/typescript-packaging.md:63-66`

**What**: The sentence says each test project names its own `tsBuildInfoFile` "because two projects with the same `rootDir` would otherwise share one build info path". The two projects do not have the same `rootDir`, which is `packages/core/src` in one and `packages/react/src` in the other. Sharing a `rootDir` value is also not sufficient for a collision. Without `tsBuildInfoFile`, TypeScript writes the build info to `outDir` joined with the config path relative to `rootDir`. Here that is `<outDir>/../tsconfig.test.tsbuildinfo`, because `rootDir` is one level below the config. So the two sibling `outDir`s resolve to the same parent, and the two config files have the same name.

**Why it is wrong**: `/microscope` section 3 says a false claim in prose is a defect. An LLD reader who moves an `outDir` or renames a config would reason from this cause and get the wrong answer.

**Evidence**: Two scratch projects `a` and `b`, each with `composite`, `types: []` and `src/x.test.ts`, built twice with `tsc --build --verbose`:
- `rootDir ./src`, `outDir ../cache/{a,b}-test` (the repository's layout): one `cache/tsconfig.test.tsbuildinfo`, and both projects "out of date ... was root file of compilation but not any more". The collision reproduces.
- `rootDir ./src`, `outDir ../cache/{a,b}-test/out`: two files, both up to date. Same `rootDir`, no collision.
- `rootDir .`, `outDir ../cache/{a,b}-test`: two files, both up to date.

**Scope and regression**: This is prose only. The configuration is correct, and the replay above confirms it. Smallest fix: delete the "because" clause, or state the real mechanism ("by default the build info lands beside `outDir`'s parent when `rootDir` is `./src`, and both test projects' `outDir`s share that parent").

### D2, "`@ocelli/react` uses the same exclude" went stale with the pass 1 nitpick fix

**Where**: `docs/lld/typescript-packaging.md:26-29`

**What**: The paragraph lists core's exclude as `src/**/*.test.ts`, `src/**/*.test.tsx` and `src/node-test-modules.d.ts`, then says "`@ocelli/react` uses the same exclude". After the repair, `packages/react/tsconfig.json` excludes only `["src/**/*.test.ts", "src/**/*.test.tsx"]`. The repair took the pass 1 nitpick and left the sentence describing the old three-entry exclude.

**Why it is wrong**: This is a false claim in prose (`/microscope` section 3). It is also the remediation-introduced class that section 3 warns about.

**Evidence**: `cat packages/react/tsconfig.json` shows the two-entry exclude. `grep -n "same exclude" docs/lld/typescript-packaging.md` gives line 29.

**Scope and regression**: Prose only. Smallest fix: "`@ocelli/react` excludes the same two test globs."

## Smells

None.

## Nitpicks

- `docs/lld/typescript-packaging.md:66-67` breaks the paragraph mid-sentence ("`dist` stays the library output" / "whether `build` or ..."), which looks left over from an insertion. It is harmless in rendered Markdown.

## Verified clean

- Both `tsconfig.test.json` files: distinct `tsBuildInfoFile` paths under the ignored cache, each `outDir` unchanged, `rootDir` unchanged, the react `jsx` setting kept, and the references unchanged from pass 1.
- `packages/react/tsconfig.json`: dropping `src/node-test-modules.d.ts` from the exclude changes nothing in the build, because no such file exists in `packages/react/src`. The react `dist` holds no test file and the tarball has 17 entries as before.
- Incremental settling: up-to-date after one build, per-project invalidation on touch, errors persist across runs, and an upstream library change invalidates both test projects. None of these produced a stale green.
- Build info isolation: no `.tsbuildinfo` in either `dist` or either tarball. A cold `npm run build` does not depend on the cache.
- Every gate and check the caller listed is green, as shown in the evidence table.

## Limitations and handoff

- Pass 1's acceptance runs for AC1 to AC11 that the repair does not touch (the refusal mutations, the AC5 base-config regression, lint plants) were not repeated. The repair changes only tsconfig fields and one LLD sentence, and the gates that cover them are green now.
- `gate --sprint` and `npm publish --dry-run` outside `gate packages` were not run. No GPU, browser or corpus path is involved.
- Next owner: the implementer fixes D1 and D2, which are both LLD prose. A fresh independent pass then re-reads `docs/lld/typescript-packaging.md` lines 20 to 70 against both `tsconfig.json` and both `tsconfig.test.json` files.
