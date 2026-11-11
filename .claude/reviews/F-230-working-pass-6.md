# F-230 review, pass 6

**Reviewed**: staged tree `40f842ca4d253f15d644623fd18019ff9fcc3e3e` on HEAD `46323bd`, branch `work/f-230-xai`. The unstaged diff is empty (`git diff | git hash-object --stdin` gives `e69de29…`, the empty blob). Untracked `tools/oracle/out` and the pass 1 to 5 reports ignored. Scope: the corrected frame-meta carve-out in `docs/lld/public-api.md` "Options and refusals", its agreement with `docs/lld/errors.md`, and the whole section read again on the staged tree.
**Author / reviewer**: the carve-out was restored by the sprint lead (Claude). Review by Claude Opus 5.5 as an independent `/microscope --working` subagent that wrote none of the implementation, the design, the LLD edits, the carve-out or the earlier passes. The carve-out's wording is the one pass 5 suggested, and this pass is independent of that pass too.
**Result**: 0 defects, 1 smell, 2 nitpicks

## Acceptance challenged

- The carve-out sentence, "`acceptTransferredFrame`, and `writeFrame` with a ticketed sink, throw `InvalidOptions` with `frame meta is not an object` for a meta that is not an object, which includes a function. An array gets the generic message.", against the built `dist` with `null`, `undefined`, a number, a string, a boolean, a symbol, a function and `[]`, through both entry points. A valid meta and a meta with an unknown key were run as controls.
- `writeFrame` with a plain sink (no `commitAllocated`), which the sentence leaves out. Expected: no refusal of any kind. Then a search of every LLD page, the README and `use.html` for a sentence claiming the plain sink is checked.
- Whether a test would go red if either new clause, the function clause or the array clause, became false. Two mutations, below.
- Every other sentence in "Options and refusals", by probe on the built `dist` where it makes a runtime claim.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --short`, `git diff \| git hash-object --stdin`, `git write-tree` | Worktree | Now | 30 staged paths, no unstaged path, tree `40f842ca…` |
| `npm run build` | Worktree, which equals the index because nothing is unstaged | Now | Exit 0. `packages/core/dist` is ignored (`.gitignore:34`) |
| `node probe6.mjs` against `packages/core/dist/index.js`, 10 metas times 3 routes | Built `dist` | Now | Probe table 1 |
| `node probe6b.mjs`, the other entry points | Built `dist` | Now | Probe table 2 |
| Mutation A: `bulk.ts:197` and `:397` guard gains `\|\| Array.isArray(…)`, so an array gets the specific message | Worktree copy, restored from a scratch backup, blob `f05dae31…` before and after | Now | `npx vitest run packages/core`: 48 files, 437 tests, all green |
| Mutation B: the same guards let a function through (`typeof … !== "object" && typeof … !== "function"`), so a function gets the generic message | Same | Now | 48 files, 437 tests, all green |
| `npm run typecheck` | Worktree | Now | Exit 0 |
| `npx vitest run` | Worktree | Now | Exit 0, 486 tests |
| `python3 scripts/prose_check.py --staged`, then `python3 scripts/prose_check.py` | Worktree | Now | Exit 0, 7 files. Exit 0, 844 files |
| `python3 scripts/source_provenance_check.py` | Worktree | Now | Exit 0, 1674 files |
| `awk` for lines over 80 columns in `public-api.md` 63 to 115 | Worktree | Now | Only line 82, a table row. The rewrapped paragraph is within 80 |
| `git write-tree` and the unstaged hash at the end | Worktree | Now | `40f842ca…` and `e69de29…`, unchanged |

Probe table 1, frame meta (`allocs` is calls to the sink's `alloc`):

| Meta | Ticketed `writeFrame` | `acceptTransferredFrame` | Plain-sink `writeFrame` |
|------|-----------------------|--------------------------|-------------------------|
| `null`, `undefined`, `3`, `"x"`, `true`, `Symbol("s")`, a function | `InvalidOptions`, `frame meta is not an object`, allocs 0 | Same | Accepted. allocs 1, `commit_frame` receives the value unchanged |
| `[]` | `InvalidOptions`, `FrameCommitMeta is not an options object`, allocs 0 | Same | Accepted, allocs 1 |
| Valid four-field object | Accepted, allocs 1 | Accepted, allocs 1 | Accepted, allocs 1 |
| Valid object plus `zz` | `UnknownOption`, `FrameCommitMeta has no option "zz"`, allocs 0 | Same | Accepted, `zz` reaches `commit_frame` |

Probe table 2, other entry points:

| Case | Observed |
|------|----------|
| `createPointerDispatch` with a 100-character key | `UnknownOption`, message and `error.option` hold the first 64 characters |
| `createPointerDispatch` with a symbol key | `UnknownOption`, `has no option "Symbol(zz)"`, `error.option` is the string `"Symbol(zz)"` |
| `createPointerDispatch` with a function, `[]`, `null`, `undefined` | `InvalidOptions`, `PointerDispatchOptions is not an options object`, no `option` |
| `createCursorController` with `cursors: { nope: 1 }` and `cursors: []` | `UnknownOption` and `InvalidOptions`, owner `CursorControllerOptions.cursors` |
| `writeKeyObject({})`, `writeSecondaryCapture({})` | Return `null` |
| Both with `{ zz: 1 }` | `UnknownOption` |
| `postWadoInstance({ zz: 1 })` | Rejects `UnknownOption` |
| `searchStudies(null)`, `searchStudies([])`, `searchStudies({ zz: 1 })` | Each rejects, `InvalidOptions` twice then `UnknownOption`, zero fetches, no synchronous throw |
| `searchStudies(undefined)` | Default parameter `{}` applies. One fetch, which this probe's fetch rejects as `Network` |

## Defects

None.

## Smells

### S1, the two clauses the carve-out adds are not pinned by any test

**Where**: `docs/lld/public-api.md:95-97` ("which includes a function. An array gets the generic message."), `packages/core/src/bulk.ts:197` and `:397`, `packages/core/src/errors-class.test.ts:251-286`.
**What**: the sentence is true today (probe table 1). But the test that guards this message, "keeps the non-object frame meta message at the public entry points", loops over `[null, 7]` only. Neither `options.test.ts` nor `errors-class.test.ts` passes a function or an array as frame meta. Mutation A makes an array take the specific message and mutation B makes a function take the generic one. Each makes one clause of the documented sentence false, and the `@ocelli/core` suite stays green under both.
**Why it is a smell**: the code calls this "the message callers match", and the page now documents its exact split. The two mutations are the natural tidy-ups of a guard that sits beside `refuseUnknownOptions`. Adding `Array.isArray` matches the generic guard, and widening to "not an object or function" reads as a fix. Either lands green and leaves the page false. This carve-out has already been wrong in its details once (pass 4 D3), which is the failure this would repeat.
**Evidence**: the two mutation rows above, 437 of 437 green each time, with the file restored and its blob hash checked.
**Scope and regression**: test coverage only. The behaviour and the prose are correct on the staged tree. Smallest fix: in that test, widen the loop to `[null, undefined, 7, "x", true, Symbol("s"), () => {}]`, and add one assertion that `[]` gives `InvalidOptions` with `FrameCommitMeta is not an options object` through both entry points with zero `alloc` calls. Then rerun mutations A and B and confirm each goes red.

## Nitpicks

- **N1.** `public-api.md:93-94`: "A value that is not a non-null, non-array object throws `InvalidOptions`". For the three QIDO searches, `undefined` is accepted, because `options` defaults to `{}` and JavaScript treats an `undefined` argument as an omitted one. The searches then fetch (probe table 2). The parameter is optional in the signature, so passing `undefined` reads as leaving it out rather than as a value. Writing "except an omitted optional parameter" would remove the ambiguity. Not blocking.
- **N2.** `docs/lld/errors.md:497`: "`error.option` holds the key." It holds the shown text, which is cut to 64 units, and a symbol key becomes the string `String(symbol)` (probe table 2). A caller comparing `error.option === key` gets `false` for a symbol or a long key. The row links to the public-api section, and that section says it exactly. "holds the key as the message shows it" would make the row exact. Not blocking.

## Verified clean

- **The carve-out sentence.** Every clause matches probe table 1 for all eight non-object metas through both entry points, and nothing is allocated before any refusal. The specific message comes from `bulk.ts:197-198` and `:397-398`, ahead of `refuseUnknownOptions`. The array message comes from `options.ts:176-177`.
- **Plain sink not claimed.** A plain-sink `writeFrame` does not refuse anything, including an unknown key. The only places that name `writeFrame` as a checker are the table row (line 82) and the carve-out, and both say "with a ticketed sink". `typescript-packaging.md:118-119`, `errors.md` lines 363, 396 and 499 to 510, `use.html:380-390` and the README make no refusal claim for a plain sink. The README and `more.html` do not mention `writeFrame` or `FrameCommitMeta`.
- **errors.md agrees with public-api.md.** The `InvalidOptions` row (`errors.md:498`) gives `refuseUnknownOptions` for a value that is not an options object, and `bulk.ts` for a frame meta that is not an object. The three `bulk.ts` raise sites (197, 397, 451) and the one in `options.ts` (177) are the only `InvalidOptions` raises in non-test source. 451 is `frameCommitMeta`, which `sessionBulkSink().commitAllocated` also reaches, and that is still `bulk.ts`.
- **The rest of "Options and refusals", read end to end.** The eight-type rule and its table, the not-checked records, the `Reflect.ownKeys` rule, the 64-unit cut, `String(symbol)`, `error.option`, inherited keys, declared `undefined` keys, the generic `InvalidOptions` rule (with N1), the `cursors` owner path, refusal before any fetch, allocation or write, rejection rather than throw for the QIDO searches, `null` from the two writers with `UnknownOption` still raised, and the census. Each runtime claim matches a probe row. The census scan reads `packages/core/src` without recursion, and that directory has no subdirectories, so "every non-test `.ts` file" holds.
- **Rewrap.** The paragraph is grammatical, nothing is duplicated, and it is within 80 columns. Pass 5 N1 is closed.
- **Checks.** Typecheck, the full vitest suite, both prose checks and the provenance check pass.
- **Worktree unchanged.** The same staged tree and empty unstaged diff at both ends. The mutated `bulk.ts` was restored from a backup and its blob hash matches the index. This report is the only new file.

## Limitations and handoff

- **Not re-reviewed.** Code paths outside frame meta and the options rules, other stories' LLD hunks, and the user docs except for the `writeFrame` mentions searched above.
- **Environment.** Node probes against the built `dist`, no browser. The example build used the worktree's ignored wasm `pkg`.
- **Prior findings.** Pass 5 D1 is closed by the restored carve-out. Pass 5 N1 is closed by the rewrap.
- **Next step.** The implementer adds the S1 cases to `errors-class.test.ts` and confirms mutations A and B go red. Then a fresh independent pass reviews that test change.
