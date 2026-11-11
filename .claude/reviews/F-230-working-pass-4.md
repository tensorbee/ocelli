# F-230 review, pass 4

**Reviewed**: `git diff --cached 7099f0c09feb26278c3c1cafb7ae5c18ebfc145a -- docs/lld`, scoped to the F-230 hunks (the same five files as `git diff --cached HEAD -- docs/lld`, HEAD `7a0cabb251c07a58c6c68ee7b0ea0b06034ad3c1`). Staged tree `6189fe3d6a888327c5d5bfed276cff199e4ab894`. Every sentence was executed against the working tree, which is that staged tree plus the unstaged diff hashing to `905911e30607a8ec761ba8e11a2c5c584dcd7f27` (the same unstaged diff pass 3 reviewed). Untracked `tools/oracle/out` and the pass 1 to 3 reports ignored. The merge-base range also carries F-119, F-217, F-218, F-219, F-221 and F-222 hunks in `cache.md`, `codecs.md`, `corpus.md`, `guards.md`, `release-policy.md` and `volumes.md`. Those belong to other stories and were not reviewed.
**Author / reviewer**: the LLD edits were made by another agent at completion step 9. Review by Claude Opus 5.5 as an independent `/microscope --working` subagent that wrote none of the implementation, the design, the LLD edits or the earlier passes.
**Result**: 3 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- The eight type names and which function checks each, against `OPTION_KEY_LISTS` and every `refuseUnknownOptions` call site.
- The 13 codes and their order, against `ocelli-error.ts` and `errors-class.test.ts`.
- What raises each code, against every `throw` in non-test `packages/core/src/*.ts`. Every shell `throw` is an `OcelliError` or a `DicomwebError`, or a rethrow.
- When `option` and `cause` are set. Only `options.ts` passes `option` and only `load.ts` passes `cause`.
- The 64-unit cut, symbol formatting, inherited and non-enumerable own keys, declared keys set to `undefined`, ordering before fetch and allocation, throw against reject, the pass-through list, the engine-error claim, the `loadCore` retry and concurrency claims, the core-worker `catch`, and the `LengthMismatch` claim, all through `dist` by a probe.
- The census claim, by mutating a copy.

The discriminating cases that found the defects: a fetch that rejects with an `AbortError`, the injected callbacks the shell replaces or swallows, and a function passed as frame meta.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `node scratchpad/probe4.mjs` importing `packages/core/dist/index.js` | `dist` built 14:36, newer than every `src` mtime (latest 14:31). `dist/dicomweb.js:33` is `async searchStudies` and `dist/bulk.js:79` holds the ticketed guard, so it matches the working tree | Now | See the probe table below |
| `tsc --noEmit -p` on a scratch copy of `packages/core`, then the same with `readonly cache?: string` added to `QidoSearchOptions` | Copy of the working-tree `src` | Now | Base exit 0. Mutated exit 2, `src/options.ts(47,7): error TS2322: Type 'true' is not assignable to type 'never'` |
| `npx vitest run` on `options`, `errors-class`, `load`, `load-retry`, `bulk` and `dicomweb` tests | Working tree | Now | 6 files, 50 tests passed |
| `python3 scripts/error_code_check.py` | Working tree | Now | Exit 0, "3 error code(s) agree across Rust, TypeScript and the registry" |
| `python3 scripts/prose_check.py --staged`, then `python3 scripts/prose_check.py` | Index, then the tree | Now | Exit 0, 6 files. Exit 0, 844 files |
| `python3 scripts/source_provenance_check.py` | Tree | Now | Exit 0, 1674 files |
| `git write-tree`, `git diff \| git hash-object --stdin`, `git status --short` | Before and after | Now | `6189fe3d…` and `905911e3…` at both ends. Status unchanged apart from this report |

Probe observations:

| Case | Observed |
|------|----------|
| `new DicomwebClient` with an extra key | Throws `OcelliError` `UnknownOption`, message `DicomwebClientOptions has no option "extra"`, `option` `extra`, no `cause` |
| 100-character key | `option` and the quoted message text are cut to 64 units |
| Symbol key `Symbol("s")` | `option` is `Symbol(s)` |
| `null` or `[]` options | `InvalidOptions`, `DicomwebClientOptions is not an options object`, `option` undefined |
| Inherited extra key | Accepted |
| Non-enumerable own extra key | `UnknownOption` (`Reflect.ownKeys`) |
| `searchStudies`, `searchSeries`, `searchInstances` with `limt` | Each returns a rejected promise with `UnknownOption`. No fetch |
| `searchStudies({ limit: undefined })` | Accepted, reaches the fetch |
| Fetch rejects with `DOMException` `AbortError` | **`DicomwebError` `Aborted`**, not `Network` |
| Fetch rejects with a `TypeError` while the signal is aborted | **`DicomwebError` `Aborted`** |
| `postWadoInstance` with an extra key | Rejects with `UnknownOption`. No fetch |
| `createPointerDispatch`, `writeKeyObject`, `writeSecondaryCapture` with an extra key | Throw `UnknownOption` |
| `createCursorController` with `cursors: { Bogus: 1 }` | `CursorControllerOptions.cursors has no option "Bogus"` |
| `writeKeyObject` with empty strings | Returns `null`. `writeKeyObject(null)` throws `InvalidOptions` |
| Ticketed `writeFrame` with `null` | `frame meta is not an object` |
| Ticketed `writeFrame` with a function | **`frame meta is not an object`** |
| Ticketed `writeFrame` with `[]` | `FrameCommitMeta is not an options object` |
| Ticketed `writeFrame` with an extra key | `UnknownOption`, zero `alloc` calls |
| Plain-sink `writeFrame` (no `commitAllocated`) with `null` or an extra key | **Returns without throwing** |
| `acceptTransferredFrame` with an extra key and a length mismatch | `UnknownOption` first, then `LengthMismatch` `handoff length 3 disagrees with meta.bytes 4` with valid meta. Zero `alloc` calls |
| `acceptTransferredFrame` with a function as meta, plain sink | `frame meta is not an object` |
| `writeStack([])` and a zero epoch | `LengthMismatch`, then `CommitRefused` |
| `loadCore` with a synchronous throw | Rejects `CoreLoadFailed`, "The imaging core did not load.", `cause` present, flag false |
| `loadCore` rejecting with `42` | `cause` is `42`, flag false, the next call runs its initialiser |
| Two concurrent calls | The same promise object. The second initialiser never runs. Both resolve `true`, then the flag is true |
| A call after success | Initialiser not run |

## Defects

### D1, dicomweb.md says every rejected fetch becomes `Network`

**Where**: `docs/lld/dicomweb.md:55-56`, "A rejected fetch becomes `Network` without the fetch's error."
**What**: an aborted fetch is a rejected fetch, and it becomes `Aborted`. `#get` (`packages/core/src/dicomweb.ts:266-273`) maps a rejection to `Aborted` when `signal?.aborted` is true or the error is a `DOMException` named `AbortError`, and to `Network` only otherwise. `transportError` (`dicomweb.ts:364-372`) does the same for a failed body read.
**Why it is wrong**: a false sentence in the LLD. The section's own first paragraph lists `Aborted` as a class, and this sentence takes away the only route that produces it.
**Evidence**: probe rows "Fetch rejects with `DOMException` `AbortError`" and "Fetch rejects with a `TypeError` while the signal is aborted". Both give `DicomwebError` `Aborted`, "DICOMweb request was cancelled".
**Scope and regression**: prose only, present on the staged tree. A caller who writes a cancel path from this page would branch on `Network` and never see their own abort. Smallest fix: "A rejected fetch becomes `Aborted` when the signal is aborted or the rejection is an `AbortError`, and `Network` otherwise. Neither keeps the fetch's error."

### D2, errors.md's bold rule says no foreign error is replaced, and three paths replace or swallow one

**Where**: `docs/lld/errors.md:509`, "**Errors the shell did not raise pass through unchanged.**"
**What**: the shell did not raise any of these, and none passes through unchanged:
- The injected `fetch`'s rejection becomes a new `DicomwebError` with no `cause` (`dicomweb.ts:266-273`, `289-290`). The same LLD, `dicomweb.md:55-56`, says so.
- The `loadCore` initialiser's error becomes `CoreLoadFailed` and survives only as `cause` (`load.ts:36-37`). The table eight lines above says so.
- `writeKeyObject` and `writeSecondaryCapture` catch any error from dcmjs and return `null` (`tid2010.ts:186-190`, `secondary-capture.ts:187-191`).
- `setPointerCapture` and `releasePointerCapture` errors are swallowed (`pointer.ts:717-721`, `729-733`).
**Why it is wrong**: a universal rule in prose that the code contradicts, twice inside the same LLD set. The design's decision 6 (`.claude/plans/F-230-design.md:84`) is narrower: the caller's `write` and `start` errors, and the ticket-releasing rethrows.
**Evidence**: the source lines above, and the probe's `loadCore` rows, where the rejection is `CoreLoadFailed` and not the thrown `Error` or `42`.
**Scope and regression**: prose only, present on the staged tree. A reader who relies on the rule would expect `instanceof TypeError` from a failing fetch, or the initialiser's own error from `loadCore`. Smallest fix: make the bold sentence a list rather than a rule, for example "**These errors pass through unchanged.**", followed by the sentences already there. Or name the exceptions: `loadCore` wraps, the DICOMweb transport replaces, and the two writers return `null`.

### D3, public-api.md's carve-out for frame meta misses a function and the plain sink

**Where**: `docs/lld/public-api.md:92-96`, "A value that is not a non-null, non-array object throws `InvalidOptions`, message `` `${owner} is not an options object` ``. `writeFrame` and `acceptTransferredFrame` keep `frame meta is not an object` for a null or primitive meta."
**What**: two cases go against the text.
- **A function as meta.** The guards at `bulk.ts:197` and `bulk.ts:397` are `typeof meta !== "object" || meta === null`, so a function gets `frame meta is not an object`. A function is not null or a primitive, so the text predicts `FrameCommitMeta is not an options object`.
- **The plain sink.** `writeFrame` checks only when `session.commitAllocated` is defined (`bulk.ts:195`). With a `BulkSink` that has only `commit_frame`, a null meta or an unknown key returns without any throw. The sentence names `writeFrame` without that condition. The table above it does carry the condition.
**Why it is wrong**: a false sentence about an exported function's message and refusal.
**Evidence**: probe rows "Ticketed `writeFrame` with a function", "`acceptTransferredFrame` with a function as meta, plain sink" and "Plain-sink `writeFrame` … with `null` or an extra key".
**Scope and regression**: prose only. The sentence holds only with the unstaged `bulk.ts` (see the handoff below). Smallest fix: "`acceptTransferredFrame`, and `writeFrame` with a ticketed sink, keep `frame meta is not an object` for a meta that is not an object, which includes a function."

## Smells

None.

## Nitpicks

- **N1.** `errors.md:248-249` says `errors-class.test.ts` "pins it in this order". The test pins membership both ways, through `satisfies` and the `Exclude` check, and pins the order of its own `SHELL_CODES` literal. A union has no observable order, so reordering `ocelli-error.ts:15-24` stays green. "pins its members, listed here in declaration order" would be exact.
- **N2.** Several added lines exceed the files' wrap width: `typescript-packaging.md` (the `LengthMismatch` sentence at about line 120, and two lines of the absent-core seam paragraph), and the `errors.md` Area line. Prose checks pass. This is layout only.

Pass 3's N1 to N5 are about code and user docs and were not re-examined, except that its N4 (a ticketed `writeFrame` passes an array to the generic refusal) still holds. D3 is adjacent to it.

## Verified clean

- **Types and checkers.** `OPTION_KEY_LISTS` (`options.ts:150-159`) has exactly the eight names. The scanner regex over non-test top-level `src/*.ts` (`src` has no subdirectories) finds exactly seven `*Options` and `*Request` exports, plus `FrameCommitMeta`. Each table row matches its call site: `dicomweb.ts:64`, `73`, `81`, `93`, `pointer.ts:200`, `cursor.ts:161` and `163`, `instance-bytes.ts:30`, `tid2010.ts:185`, `secondary-capture.ts:186`, `bulk.ts:200` (ticketed only) and `bulk.ts:400`. Each runs before any field read. `HandoffMeta`, `StackFrameKey` and `PackedCommand` exist and are unchecked.
- **Codes.** The 13 codes are in the stated order in `ocelli-error.ts:8-24`, and the first five are `DicomwebErrorCode`. Each row of the "Raised by" table matches its sites: `UnknownOption` and `InvalidOptions` (`options.ts:177`, `184`, `bulk.ts:198`, `398`, `451`), `AllocationRefused` (`bulk.ts:149`, `205`, `354`, `419`, a complete list), `CommitRefused` (`bulk.ts:154`, `214`, `428`, `instance-bytes.ts:72`), `LengthMismatch` (`bulk.ts:136`, `402`), `InvalidValue` (`bulk.ts:142`, `478`, `commands.ts:184`, `236`, `256`, `263`), `ExportUnavailable` (`bulk.ts:277`) and `CoreLoadFailed` (`load.ts:37`).
- **`option` and `cause`.** `option` is set only for `UnknownOption` and `cause` only for `CoreLoadFailed`. `DicomwebError` passes no details, so it has no `cause`. Its `name` is `DicomwebError` and `messageFor` fixes one message per code.
- **Message, cut and symbol.** The message format, the 64-unit cut applied before both the message and `option`, `String(symbol)`, inherited keys skipped, declared `undefined` accepted, and arrays and `null` refused, all on `dist`.
- **Order and promise shape.** The refusal comes before fetch and `alloc` (zero fetches, zero allocs). The constructor and the sync factories throw. The three searches and `postWadoInstance` reject.
- **Pass-through and engine errors.** The listed rethrows exist (`commands.ts:126-128`, `pool.ts:59-62`, `bulk.ts:156`, `216-218`, `430-432`). `bulk.test.ts:250` expects `RangeError`.
- **Ring codes.** `describeError(code: number)` (`errors.ts:158`). `error_code_check.py` compares Rust, `errors.ts` and `ci/error-codes.json`, and does not read string codes.
- **Module tables.** The four shell modules are re-exported (`index.ts:51`, `59`, `121`, `132`, `135`). The tarball table rows for `ocelli-error.ts` and `options.ts` are correct.
- **`loadCore`.** It returns `Promise<true>`, sets the flag only after the initialiser resolves, wraps a synchronous throw, retries after failure, shares one pending promise across concurrent calls, and skips the initialiser after success.
- **The example worker.** `core-worker.ts:70-79` awaits inside `try`, logs `error.code` or the raw value, and posts `available: coreAvailable()`, false at that point, with tier `""`.
- **`LengthMismatch` claim.** It is thrown before the copy, and before `alloc`.
- **Census.** A new field fails `typecheck` (mutation above).
- **Index rows.** The `README.md` index rows match each file's header, including F-199, F-200 and F-201 on `public-api.md`.
- **Stale text.** Nothing else in the five files was made stale. A grep for `loadCore`, "resolves false", throw, null-return and refusal wording found no stale sentence.

## Limitations and handoff

- **Staging precondition.** Two staged LLD claims are true only with the unstaged code: "The three QIDO searches reject their promise", in `public-api.md:99-100` and `dicomweb.md:60-61`, and the `frame meta is not an object` carve-out. On the index alone, `searchStudies` is not `async` (`git show :packages/core/src/dicomweb.ts`, line 72), so it throws synchronously, and the ticketed `writeFrame` gives the generic message for `null`. The completion commit must include the unstaged `bulk.ts`, `dicomweb.ts`, `errors-class.test.ts`, `options.test.ts` and plan amendment, or these sentences ship false.
- **Not reviewed.** The other stories' LLD hunks in the merge-base range. The user docs and the code, beyond executing the LLD's claims (pass 3 covered them).
- **Not exercised.** No browser run of the example worker. The `catch` was read and the `loadCore` side was probed under node.
- **Next step.** The implementer fixes D1 to D3 in the LLD, then a fresh independent pass reviews the changed sentences.
