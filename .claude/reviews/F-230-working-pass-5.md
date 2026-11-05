# F-230 review, pass 5

**Reviewed**: staged tree `6ad2095a88b254ebe240e4a77eb61d4f76d369fc` on HEAD `7a0cabb251c07a58c6c68ee7b0ea0b06034ad3c1`, branch `work/f-230-xai`. The unstaged diff is empty (`git diff | git hash-object --stdin` gives `e69de29…`, the empty blob). Untracked `tools/oracle/out` and the pass 1 to 4 reports ignored. Scope: the three LLD deletions pass 4 asked for, the staging precondition pass 4 named, and the build, test and typecheck on the index alone.
**Author / reviewer**: the deletions were made by the sprint lead (Claude). Review by Claude Opus 5.5 as an independent `/microscope --working` subagent that wrote none of the implementation, the design, the LLD edits, the deletions or the earlier passes.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- Each deleted sentence's paragraph, read for grammar, truth, and any remaining sentence that still implies the deleted claim or became false once the qualifier was gone.
- That the index now holds the pass 4 code fixes, by running the built `dist` of an exported index rather than the worktree.
- The discriminating cases: a ticketed `writeFrame` and `acceptTransferredFrame` given `null`, a number, a function and `[]` as frame meta, and the three QIDO searches given an unknown key, observed as reject against throw.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --short`, `git diff --stat`, `git diff \| git hash-object --stdin` | Worktree | Now | 30 staged paths, no unstaged path, empty-blob hash |
| `git diff --cached --stat` on `bulk.ts`, `dicomweb.ts`, `errors-class.test.ts`, `options.test.ts`, `.claude/plans/F-230-design.md` | Index | Now | All five present (45, 27, 476 new, 372 new, 7 lines). The plan gains "Lead amendments after review pass 1" |
| `git checkout-index -a --prefix=scratchpad/p5tree/`, `node_modules` built from per-entry symlinks to the worktree's with `@ocelli/*` relinked into the scratch tree, and the ignored `crates/ocelli-wasm/pkg` copied in because the example build imports it | Index `6ad2095a…` | Now | Scratch tree only |
| `npm run build` | Scratch tree | Now | Exit 0. The first attempt failed only on the missing ignored wasm `pkg` |
| `npm run typecheck` | Scratch tree | Now | Exit 0 |
| `npm run test` | Scratch tree | Now | Exit 0, 60 files, 486 tests |
| `node probe5.mjs` against scratch `packages/core/dist/index.js` | Scratch build of the index | Now | See the probe table |
| Mutation: `searchStudies` made non-`async` with `return this.#qido(...)` | Scratch copy, reverted and diffed | Now | Red: `options.test.ts > public options > rejects an unknown search key as a promise` |
| Mutation: `bulk.ts:198` message changed to `FrameCommitMeta is not an options object` | Scratch copy, reverted and diffed | Now | Red: `errors-class.test.ts > OcelliError > keeps the non-object frame meta message at the public entry points` |
| `python3 scripts/prose_check.py --staged`, then `python3 scripts/prose_check.py` | Worktree | Now | Exit 0, 7 files. Exit 0, 844 files |
| `python3 scripts/source_provenance_check.py` | Worktree | Now | Exit 0, 1674 files |
| `git write-tree` and the unstaged hash, before and after | Worktree | Now | `6ad2095a…` and `e69de29…` at both ends |

Probe observations:

| Case | Observed |
|------|----------|
| `searchStudies({ limt: 1 })` | Returns a promise that rejects `OcelliError` `UnknownOption`, `QidoSearchOptions has no option "limt"`. No synchronous throw |
| `searchSeries` and `searchInstances` with valid UIDs and `limt` | Each rejects `UnknownOption`. Zero fetches across all three |
| Fetch rejects with `DOMException` `AbortError` | `DicomwebError` `Aborted`, no `cause` |
| Fetch rejects with `TypeError` | `DicomwebError` `Network`, no `cause` |
| Ticketed `writeFrame` meta `null`, `3`, a function | `InvalidOptions`, `"frame meta is not an object"` |
| `acceptTransferredFrame` target `null`, `3`, a function | `InvalidOptions`, `"frame meta is not an object"` |
| Both with `[]` | `InvalidOptions`, `"FrameCommitMeta is not an options object"` |
| `alloc` calls across all eight frame-meta cases | 0 |

## Defects

### D1, public-api.md now gives `FrameCommitMeta` the generic message for a non-object meta

**Where**: `docs/lld/public-api.md:92-94`, "A value that is not a non-null, non-array object throws `InvalidOptions`, message `` `${owner} is not an options object` ``."
**What**: the sentence is the rule for all eight in-scope types, and `FrameCommitMeta` is one of them (the table row above it). For that owner, a `null`, primitive or function meta gives the message `frame meta is not an object`, not `FrameCommitMeta is not an options object`. Only an array gets the generic message. `bulk.ts:197-198` and `bulk.ts:397-398` raise it before `refuseUnknownOptions` runs, with the comment "This message is the one callers match", and `errors-class.test.ts` pins it (the second mutation above goes red). The deleted sentence was the carve-out. It was false in its details (pass 4 D3), but deleting it left the general rule false for this owner instead of fixing it.
**Why it is wrong**: a false sentence about an exported function's refusal message, and the message the code says callers match. Pass 4 D3's smallest fix was a corrected carve-out, not a deletion. `docs/lld/errors.md:498` still says `bulk.ts` raises `InvalidOptions` "for a frame meta that is not an object", so the two LLD pages now disagree on the message.
**Evidence**: probe rows "Ticketed `writeFrame` meta `null`, `3`, a function" and "`acceptTransferredFrame` target …". The code at `bulk.ts:197-198`, `397-398`.
**Scope and regression**: prose only, on the staged tree. A caller who matches the message from this page would match `FrameCommitMeta is not an options object` and miss every non-array refusal. Smallest fix, after the generic sentence: "`acceptTransferredFrame`, and `writeFrame` with a ticketed sink, throw `InvalidOptions` with the message `frame meta is not an object` for a meta that is not an object, which includes a function. An array gets the generic message." Every clause there is in the probe table.

## Smells

None.

## Nitpicks

- **N1.** The deletion joined two lines into `docs/lld/public-api.md:94` at 102 characters, against the paragraph's 80-column wrap. Prose checks pass. Layout only, and the D1 fix will rewrap it anyway.

## Verified clean

- **dicomweb.md deletion.** The paragraph is now "The injected fetch receives the caller's `AbortSignal` unchanged. Ocelli does not retry a rejected fetch." Grammatical, and nothing in "Transport validation and errors" now says a rejected fetch becomes `Network`. The class list still names `Aborted` and `Network`, and the probe shows both routes. The `DicomwebError` paragraph ("carries no `cause`") holds for both.
- **errors.md deletion.** The pass-through paragraph now starts with the specific list (`submitCommands`, `openDecodePool`, the three ticket-releasing rethrows) and the engine-error limit. Grammatical. No remaining sentence in "Shell errors are a second namespace" states a universal pass-through. "`OcelliError extends Error` is the class for every failure the shell raises" is about raised errors, not foreign ones, and stays true. The `CoreLoadFailed` row still says `loadCore` wraps.
- **Staging precondition.** All five paths pass 4 named are in `git diff --cached`. Nothing is unstaged. On the index, `searchStudies` is `async` (`dicomweb.ts:72`) and rejects, and the "three QIDO searches reject their promise" sentences in `public-api.md:97-98` and `dicomweb.md:59-60` are true.
- **Index build.** Build, typecheck and the full vitest suite pass on an exported copy of the index, independent of the worktree.
- **Tests guard the claims.** Both mutations went red in a test that names the behaviour.
- **Voice and provenance.** Both checks clean.
- **Worktree unchanged.** Same staged tree and empty unstaged diff at both ends. This report is the only new file.

## Limitations and handoff

- **Not re-reviewed.** Everything pass 4 marked clean outside the three deletions, the user docs and the code (pass 3), and the other stories' LLD hunks.
- **Environment.** The example build needed the ignored wasm `pkg` copied from the worktree, so that artefact is the worktree's build, not one made from the index. No browser run.
- **Next step.** The implementer fixes D1 in `public-api.md`, then a fresh independent pass reviews that sentence.
