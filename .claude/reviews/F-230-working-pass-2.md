# F-230 review, pass 2

**Reviewed**: the staged tree `1501c7b8f35a81c014d64622cf5999d2319e5a41` (unchanged since pass 1) plus the unstaged remediation on `work/f-230-xai`, whose `git diff` hashes to `0734c3ca63b35940553587e4f123e62e9af07b41`. Unstaged paths: `.claude/plans/F-230-design.md`, `examples/folder-viewer/src/use.html`, `packages/core/src/bulk.ts`, `packages/core/src/dicomweb.ts`, `packages/core/src/errors-class.test.ts`, `packages/core/src/options.test.ts`. Base `7a0cabb251c07a58c6c68ee7b0ea0b06034ad3c1` (HEAD). Untracked `tools/oracle/out` and `.claude/reviews/F-230-working-pass-1.md` ignored.
**Author / reviewer**: implementation and remediation by an external coding agent. Plan amendments by the sprint lead. Review by Claude Opus 5.5 as an independent `/microscope --working` subagent that wrote none of the implementation, the remediation, the design or pass 1.
**Result**: 1 defect, 0 smells, 5 nitpicks

## Prior findings

| Pass 1 | State | How it was replayed |
|--------|-------|---------------------|
| D1, `searchStudies` throws synchronously | **Closed** | Probe on rebuilt `dist`, each call made without `await` inside a `try`: `searchStudies`, `searchSeries` and `searchInstances` each returned a promise that rejected, for `{ zz: 1 }` (`UnknownOption`), `null`, `[]` and `7` (`InvalidOptions`). Zero fetches. Reverting `async` turned "rejects an unknown search key as a promise" red |
| D2, non-object frame meta lost its message | **Closed** | Probe: `writeFrame` (ticketed sink) and `acceptTransferredFrame` with `null`, `7`, `undefined`, `"s"` and a function each throw `InvalidOptions` "frame meta is not an object", and the sink's `alloc` was never called. Removing the `writeFrame` guard, then separately the `acceptTransferredFrame` guard, each turned "keeps the non-object frame meta message at the public entry points" red |
| D3, use.html 5.1 catch comment | **Closed** | The `try` now wraps only `loadCore`. `loadCore` rejects only from its `.catch`, which always builds `CoreLoadFailed` with `cause`, so the comment holds on every path its `try` covers. `resolvedTier()` sits outside, behind `coreAvailable()` |
| N1, N2, N3 | Carried, unchanged | `shownKey` still slices at 64 units. The duplicate `new DicomwebClient` is still at `options.test.ts:156` and `:160`. use.html line 383 still lists two messages |

## Acceptance challenged

- **Failure channel, every checked entry point, base against head.** Async on both: `searchStudies` (base returned `#qido`'s promise, head is `async`), `searchSeries`, `searchInstances`, `postWadoInstance`, `loadCore` (a `Promise.resolve().then` chain on both). Sync on both: the `DicomwebClient` constructor, `createPointerDispatch`, `createCursorController`, `writeFrame`, `acceptTransferredFrame`. `writeKeyObject` and `writeSecondaryCapture` are sync on both and could not throw on base, because a catch-all returned `null`. They now throw for an unknown key and for a non-object request. Design decisions 2 and 7 and AC3 authorise both, so neither is the D1 class. No other method moved from rejecting to throwing.
- **Every thrown message, base against head.** Every `throw` in the eleven changed source files was extracted from `git show HEAD:` and from the tree, multi-line ones included, and diffed. Every pre-existing message is byte-identical. The new texts are the three non-object guards, `options.ts`'s two and `CoreLoadFailed`. One input still changes text: an array frame meta. See N4.
- **Doc samples whose comment depends on what the `try` wraps.** use.html has two `try` samples, 5.1 (closed above) and 5.6. README and more.html have none. 5.6 is D1 below.
- **Plan amendments against the tree.** `bulk.test.ts` is untouched and line 250 still asserts `toThrow(RangeError)`. The sink at line 226 returns `memory.buffer.byteLength + 8`, so the error is the engine's, as the AC6 amendment says. `commands.test.ts` changed only the two `RangeError` assertions at the old lines 210 and 213. `dicomweb.test.ts` changed only the one call, from `...responseSink()` to `wasm` and `sink`. Both amendments match the tree. The original rows they override were left standing, which is N5.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | staged 1501c7b8 plus unstaged 0734c3ca | now | 0 |
| `npm run typecheck` | same | now | 0 |
| `npm run lint` | same | now | 0 |
| `npm test` | same | now | 0, 60 files, 486 tests |
| `npm run build --workspace @ocelli/example-viewer` | same, under `pipefail` | now | 0 |
| `bin/ocelli.sh gate packages` | same | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | same | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | same | now | 0, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | same | now | 0 |
| `python3 scripts/prose_check.py` | same | now | 0, 844 files clean |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1674 files clean |
| 3 mutations (`searchStudies` back to non-async, each D2 guard removed), `npx vitest run packages/core` | same | now | each red, one named test each, 436 others green. Restored by `cp` from a scratchpad backup, `shasum -c` OK |
| Node probe against `packages/core/dist/index.js` | same, after `npm run build` | now | outputs quoted above and in D1 |
| `git write-tree` and `git diff \| git hash-object --stdin` at the end | | now | `1501c7b8...`, `0734c3ca...`, unchanged |

Environment: macOS, Node v24.13.0, vitest 4.1.11. No GPU, browser or corpus used, and this story needs none.

## Defects

### D1, use.html 5.6 tells the reader an unknown option arrives in a branch it never reaches

**Where**: `examples/folder-viewer/src/use.html:436-440`
**What**: the sample's `catch` handles only `if (error instanceof DicomwebError)`, and inside that branch the new comment says "DicomwebError is an OcelliError. An unknown option is UnknownOption." An unknown search key is refused by `refuseUnknownOptions`, which throws a plain `OcelliError`, not a `DicomwebError`. `DicomwebErrorCode` holds five codes and `UnknownOption` is not one of them. So the branch the comment sits in never sees `UnknownOption`, and a reader who copies the sample swallows a misspelt option silently, because the `catch` has no other branch. That is the quietly ignored option this story exists to remove.
**Why it is wrong**: `.claude/commands/microscope.md` section 3 counts every behaviour sentence in use.html, and a sentence that is true alone but false where it is placed misleads exactly the caller the page is for. Design decision 4 makes `UnknownOption` an `OcelliError` code outside `DicomwebErrorCode`.
**Evidence**: probe on `dist`: `searchStudies({ zz: 1 })` rejected `UnknownOption`, `instanceof DicomwebError` false. The same for `searchSeries`, `searchInstances` and `postWadoInstance`. `grep -n DicomwebErrorCode -A6 packages/core/src/ocelli-error.ts` lists `InvalidRequest`, `Network`, `Aborted`, `HttpStatus`, `ContentType`.
**Scope and regression**: documentation only, present in the staged tree that pass 1 reviewed and missed by it. Smallest fix: move the sentence out of the `DicomwebError` branch, for example an `else if (error instanceof OcelliError)` branch commented "error.code is UnknownOption or InvalidOptions, and error.option names the key", with `OcelliError` added to the sample's import. Or delete the second sentence.

## Smells

None.

## Nitpicks

- **N1, carried.** `shownKey` cuts at 64 UTF-16 units and can leave a lone high surrogate.
- **N2, carried.** `options.test.ts:156` and `:160` construct the same refused client twice.
- **N3, carried.** use.html line 383 names two of the messages `writeFrame` can raise.
- **N4, new.** An array frame meta still changes text. On base `frameCommitMeta([])` passed the object check and threw `TypeError` "viewport is not a u32". On head the new guard passes an array, `refuseUnknownOptions` refuses it, and the text is "FrameCommitMeta is not an options object". Decision 2 and AC3 make an array `InvalidOptions`, and a field-bearing array that base accepted is now refused, so this is a new refusal rather than an old one reworded. It is recorded because decision 5's rationale, a caller matching today's text, covers it, and the lead may want the D2 guard to include `Array.isArray`.
- **N5, new.** The amendments are appended and the rows they override still read the old way: AC6 at plan line 222 still says `bulk.test.ts` line 250 becomes `OcelliError`, decision 10 at line 88 still says `RangeError` and `TypeError` from bulk calls become `OcelliError`, AC2 at line 218 lists `dicomweb.test.ts` as unmodified, and the write set at line 250 does not list `dicomweb.test.ts`. The CHANGELOG entry at `/complete-feature` is worded from decision 10 unless the writer also reads the amendment. Striking the four rows in place would leave one reading. Also "frame meta is not an object" is now written in three places in `bulk.ts`, guarded by tests on all three routes.

## Verified clean

- The remediation diff, read fresh. `searchStudies` awaits `#qido` and returns `undefined`, matching its `Promise<void>` type and its two siblings. The two guards are the first statement on the ticketed `writeFrame` path and in `acceptTransferredFrame`, before `alloc`, and the non-ticketed `writeFrame` path is unchanged. The guard's predicate is base `frameCommitMeta`'s predicate exactly, so every value base refused with that message still gets it, functions included. Ordering against the length check in `acceptTransferredFrame` changed (a `null` target with a length mismatch now reports the meta), which design line 211 requires.
- The two new tests call the public functions, not `sessionBulkSink` directly, assert the code and the message, and assert no allocation and no fetch. The D1 test calls each method without `await` and would catch a synchronous throw.
- The `dicomweb.test.ts` edit keeps the sanitisation assertions it was written for. Its comment is true.
- use.html 5.1's prose and comment, README lines 349 to 350, and section 15's two new sentences, against `load.ts`, `options.ts` and the probe.
- No `wasm-bindgen`, `unsafe`, pixel arithmetic, render-loop or tier change. All tiers n/a, as declared.

## Limitations and handoff

- **Not exercised:** `core-worker.ts` with a failing load in a real browser (AC11), as in pass 1. The dual CJS and ESM `instanceof` hazard, which predates this story.
- **Carried from pass 1, not in this diff:** `acceptTransferredFrame` drops `stackIndex` and `stackEpoch` from what it commits. Still for the lead to file or document.
- **Still for `/complete-feature`, confirmed absent:** the CHANGELOG entry, worded as the AC6 amendment says, and the four LLD edits.
- **Next:** the implementer fixes D1, then a fresh pass 3 reviews that change and any response to N4 and N5.
- Mutations were applied to the working tree and restored from scratchpad copies rather than `git checkout`, because the remediation is unstaged. This report is the only file written.

F-230 pass 2: 1 defect, 0 smells, 5 nitpicks.
