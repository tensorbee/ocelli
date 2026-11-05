# F-230, Public options refuse unknown keys, one coded shell error, and loadCore keeps its cause

**Status**: approved
**Epic ref**: E16.6
**Sprint**: S46
**Estimate**: 2w

Proposed backlog row, to be filed by the lead under "M4, Public API, the boundary, and the stack viewport" in `docs/sprints/BACKLOG.md` and in `docs/sprints/allocation.json` with `epic_key` E16, `epic` "E16 Public API & TypeScript shell", milestone M4, sprint S46, `depends_eids` [], `depends_fids` []:

```text
| F-230 | E16.6 | S46 | Public options refuse unknown keys: every options and request object in @ocelli/core refuses an unknown key at run time with a coded OcelliError naming it, DicomwebError and the bulk, command and instance errors share that class, and loadCore rejects with the initialiser's cause instead of resolving false | TS | 2w | - | pending |
```

E16.6 is the next free number. `grep -oE '\| E16\.[0-9]+ \|' docs/sprints/BACKLOG.md` prints E16.1 to E16.5, and `allocation.json` holds no E16 entry above E16.5. Allocation note to carry: "From the legacy Cornerstone issue list the operator shared on 2026-11-05, items 5 and 9. Lands before F-224, which adds a DicomwebClient option, and before F-213 and F-215, which read or edit the same package files."

## Normative source, transcribed

### HLD section 23, `docs/hld/20-errors-and-panics.md`, the bullets this story applies

|  |
|----|
| Error codes are stable and versioned. The shell switches on the code; the message is for humans and may change. |
| A poisoned instance surfaces to the user as a viewport-level error state, never as a silent blank canvas. |

The section's opening sentence, verbatim:

|  |
|----|
| thiserror in the core crates; the boundary maps everything to a stable numeric code and a message. |

### HLD section 10, `docs/hld/07-concurrency-and-typescript.md`, "What stays TypeScript"

"Framework bindings, DICOMweb fetch and authentication" is a TypeScript responsibility. The option objects this story checks are all shell objects, so the check is TypeScript and nothing crosses the boundary.

### Decision D1, `docs/hld/11-decision-log.md`

| D1 | Feature parity with a new API | Drop-in compatibility; adapter shim | Clean core; a migration you own |

### Release rules, `docs/RELEASE.md`

| M2 to M12 | 0.2.0 to 0.12.0 | One minor per milestone. Anything may change. |

Verbatim: "**0.x carries no stability promise and says so.**" And: "The version bump itself lands earlier, through its own F-ID, so `/release` never edits a version."

`docs/lld/release-policy.md`, verbatim: "`0.x` carries no stability promise. The removal window is null."

`packages/core/package.json` is `"version": "0.1.0"` and `CHANGELOG.md` `## Unreleased` reads "Repository bootstrap. Nothing is published."

### The code this story changes, read from the tree at `8729656`

`packages/core/src/dicomweb.ts`, lines 10 to 30: `DicomwebErrorCode` is `"InvalidRequest" | "Network" | "Aborted" | "HttpStatus" | "ContentType"`, and `class DicomwebError extends Error` with `readonly code` and an optional `readonly status`, documented as "A safe transport failure that never retains a URL, header, or body."

`packages/core/src/dicomweb.ts`, lines 62 to 68, the constructor reads five fields and nothing else:

```ts
constructor(options: DicomwebClientOptions) {
  this.#studiesServiceUrl = safeUrl(options.studiesServiceUrl);
  this.#wadoUriUrl = safeUrl(options.wadoUriUrl);
  this.#fetch = options.fetch;
  this.#wasm = options.wasm;
  this.#sink = options.sink;
}
```

`packages/core/src/pointer.ts`, lines 198 to 201, `createPointerDispatch` reads `options.viewport`, `options.now ?? Date.now` and `options.basis ?? (() => null)`. A key spelt `nowFn` is ignored and `Date.now` is used.

`packages/core/src/load.ts`, lines 16 to 41, `loadCore(initialise: () => Promise<unknown>): Promise<boolean>`, whose chain ends `.then(() => { ready = true; return true; }).catch(() => false).finally(...)`. The thrown value is discarded.

`packages/core/src/bulk.ts` throws `new Error("allocation refused")` (lines 194, 341, 400), `new Error("commit refused")` (203, 409), `new Error("stack allocation refused")` (143), `new Error("stack configuration refused")` (148), `new Error("selected-frame export is unavailable")` (265), `new Error("handoff length ... disagrees with meta.bytes ...")` (384), `new RangeError("stack length is outside the bulk limit")` (133), `new RangeError("series is not a u64")` (137), `new TypeError("frame meta is not an object")` (432) and `` new TypeError(`${name} is not a u32`) `` (459). None carries a code. `frameCommitMeta` reads six named fields and ignores any other, and it takes the stack-selection path only when `stackIndex` or `stackEpoch` is present.

`packages/core/src/commands.ts` throws `RangeError` at lines 183, 234, 254 and 261. `packages/core/src/instance-bytes.ts` line 69 throws `new Error("the instance body was not committed")`.

`packages/core/src/errors.ts` is the ring record model: numeric `ERROR_CODE` (1, 700, 701) mirrored from `ci/error-codes.json` and checked by `scripts/error_code_check.py`, with `describeError(code)`.

## What the specification does not cover

The HLD names no TypeScript error class, no option validation and no load signature. Every item below is decided by the lead under the operator's delegation of 2026-11-05.

1. **Which objects are "public options objects".** The in-scope set is every exported interface or type alias in `packages/core/src` whose name ends in `Options` or `Request`, plus `FrameCommitMeta`. Today that is `DicomwebClientOptions`, `QidoSearchOptions`, `PointerDispatchOptions`, `CursorControllerOptions`, `WadoInstanceRequest`, `KeyObjectRequest` and `SecondaryCaptureRequest`, found by `grep -n 'export \(interface\|type\) [A-Za-z]*\(Options\|Request\)\b' packages/core/src/*.ts`. `FrameCommitMeta` joins because a misspelt `stackIndex` with no `stackEpoch` silently takes the generation path today, which commits a frame to the wrong target. Fact records such as `StableClaim`, `MatrixInput`, `HandoffMeta`, `StackFrameKey` and `PackedCommand` are out of scope, because a misspelt key there already refuses or answers false rather than configuring a different behaviour. A census test fixes the set so a new `*Options` or `*Request` type cannot ship without a key list.
2. **What "unknown" means.** An own property key, string or symbol, from `Reflect.ownKeys`, that is not a declared field of the type. Inherited keys are not checked, because the field reads do not see them as configuration either. A declared optional key whose value is `undefined` is accepted. A value that is not a non-null, non-array object is refused as `InvalidOptions`. `CursorControllerOptions.cursors` is a record keyed by stack command, and its keys are checked against `STACK_COMMANDS`, because an unknown command key is ignored today in the same way.
3. **One class, the existing pattern generalised.** `OcelliError extends Error` with a string `code`. `DicomwebError` becomes `class DicomwebError extends OcelliError`, keeps its name, its five codes, its `status`, its messages and its rule that it retains no URL, header or body. So `instanceof DicomwebError` and `error.code` keep working, and `instanceof OcelliError` catches every error the shell originates. The numeric ring codes in `errors.ts` stay separate. They come from Rust through `ci/error-codes.json`, and mixing a string code into that registry would make `scripts/error_code_check.py` compare two namespaces.
4. **The codes.** `OcelliErrorCode` is `DicomwebErrorCode` plus `UnknownOption`, `InvalidOptions`, `AllocationRefused`, `CommitRefused`, `LengthMismatch`, `InvalidValue`, `ExportUnavailable` and `CoreLoadFailed`. Section 23 says codes are stable and versioned, so the union is closed and a test pins it.
5. **Existing messages are kept verbatim.** Only the class and the code are new, so a caller matching a message today still matches and section 23's "may change" is not exercised by this story.
6. **Errors the caller raised are rethrown unchanged.** `submitCommands` rethrows the caller's `write` error, `openDecodePool` rethrows the caller's `start` error, and `writeStack` and `writeFrame` rethrow after releasing the ticket. Wrapping them would change the identity of an error the shell did not originate.
7. **Unknown keys throw even where the function returns null for bad content.** `writeKeyObject` and `writeSecondaryCapture` return `null` when the content cannot be written, inside a `try` that turns every throw into `null`. A misspelt key is a programming error and not content, so the key check runs before that `try` and throws `UnknownOption`.
8. **The message names the key.** `` `${owner} has no option ${JSON.stringify(key)}` ``, with the key cut to 64 UTF-16 units before quoting and a symbol key written as `String(symbol)`. The key also sits on `error.option`. A key is a caller identifier, not a value, so naming it keeps `DicomwebError`'s rule that no URL, header or body is retained.
9. **`loadCore` rejects.** The signature becomes `loadCore(initialise: () => Promise<unknown>): Promise<true>`. Success resolves `true`. Failure rejects with `OcelliError` code `CoreLoadFailed`, message "The imaging core did not load.", and `cause` the value the initialiser threw or rejected with. Rejected alternatives: keeping `Promise<boolean>` and adding a `coreLoadError()` getter, which leaves the false path silent for every caller who does not know the getter exists, and resolving a `{ ok, error }` object, which an untyped caller's existing `if (!loaded)` would read as success every time, the quietly wrong class. `Promise<true>` rather than `Promise<void>` because an untyped `if (!loaded)` on `undefined` would read success as failure. Retry, sharing of one in-flight load, and the synchronous-throw handling S43 pass 1 fixed are unchanged.
10. **Compatibility under 0.x.** `docs/RELEASE.md` gives 0.x no stability promise and nothing is published, so this changes the API in place with one `CHANGELOG.md` `## Unreleased` `### Changed` entry naming the three observable changes: the `loadCore` rejection, `RangeError` and `TypeError` from bulk and command calls becoming `OcelliError`, and unknown keys refused. No version bump, which `docs/RELEASE.md` gives to its own F-ID. No deprecation shim, because `docs/lld/release-policy.md` gives 0.x a null removal window and a shim would be a wrapper that only forwards.

## Problem and reproducer

Source identity: HEAD `8729656` on `sprint/s46`, `packages/core/src` unmodified.

1. `new DicomwebClient({ ...valid, cahce: "no-store" })` constructs, and the misspelt key has no effect. Nothing tells the caller.
2. `createPointerDispatch({ ...valid, nowFn: clock })` uses `Date.now`. A test written against the misspelt clock passes or fails for a reason nobody can see.
3. `writeFrame(wasm, sessionBulkSink(api), bytes, { viewport: 1, columns: 1, rows: 1, generation: 1, stackindex: 3 })` commits through the generation path, not the selected-frame path the caller meant.
4. `await loadCore(() => Promise.reject(new Error("pkg/ocelli_wasm.js 404")))` resolves `false`. The 404 is gone. `examples/viewer-react/src/core-worker.ts` then posts an empty tier and the page shows "core not built" for every failure cause.
5. `writeFrame` with a zero allocation throws `Error("allocation refused")`. A caller can match the message only, which section 23 says may change.

```bash
npx vitest run packages/core/src/options.test.ts packages/core/src/load.test.ts packages/core/src/errors-class.test.ts
```

All three files are new, so this fails today on missing modules, which is not the semantic RED. The semantic RED is recorded by running the AC1, AC5 and AC8 scenarios against the old source through a scratch harness: the misspelt option constructs, `loadCore` resolves `false`, and `allocation refused` carries no `code`.

## Scope, API and compatibility

In scope: `@ocelli/core` only. `@ocelli/react` props are React props and out of scope. Non-goals: validating the types or ranges of known options beyond what each function already does, any change to the numeric ring codes, and any Rust change.

New module `packages/core/src/ocelli-error.ts`:

```ts
export type OcelliErrorCode =
  | DicomwebErrorCode
  | "UnknownOption"
  | "InvalidOptions"
  | "AllocationRefused"
  | "CommitRefused"
  | "LengthMismatch"
  | "InvalidValue"
  | "ExportUnavailable"
  | "CoreLoadFailed";

export class OcelliError extends Error {
  readonly code: OcelliErrorCode;
  readonly option?: string;            // UnknownOption only
  constructor(code: OcelliErrorCode, message: string, details?: { option?: string; cause?: unknown });
}
```

`name` is `"OcelliError"`. `cause` goes through the ES2022 `Error` options, which `tsconfig.base.json`'s `"target": "ES2022"` and `"lib": ["ES2022", ...]` provide. `DicomwebErrorCode` moves to this module and `dicomweb.ts` re-exports it, so the public name is unchanged.

New module `packages/core/src/options.ts`, internal, not exported from `index.ts`:

```ts
export function refuseUnknownOptions(owner: string, value: unknown, keys: readonly string[]): void;
export const DICOMWEB_CLIENT_OPTION_KEYS = [
  "studiesServiceUrl", "wadoUriUrl", "fetch", "wasm", "sink",
] as const satisfies readonly (keyof DicomwebClientOptions)[];
// one list per in-scope type, each followed by a compile-time completeness check:
type MissingDicomwebClient = Exclude<keyof DicomwebClientOptions, (typeof DICOMWEB_CLIENT_OPTION_KEYS)[number]>;
const dicomwebClientComplete: [MissingDicomwebClient] extends [never] ? true : never = true;
```

`satisfies` refuses a listed key the type lacks, and the `Exclude` line refuses a type key the list lacks, so `npm run typecheck` goes red when an interface gains a field without its list. `options.ts` also exports `OPTION_KEY_LISTS: Readonly<Record<string, readonly string[]>>`, keyed by type name, for the census test.

Where each check runs, first thing in the call:

| Type | Entry point | Owner string in the message |
|---|---|---|
| `DicomwebClientOptions` | `new DicomwebClient(options)` | `DicomwebClientOptions` |
| `QidoSearchOptions` | `searchStudies`, `searchSeries`, `searchInstances`, before UID validation | `QidoSearchOptions` |
| `PointerDispatchOptions` | `createPointerDispatch` | `PointerDispatchOptions` |
| `CursorControllerOptions`, and its `cursors` record against `STACK_COMMANDS` | `createCursorController` | `CursorControllerOptions`, and `CursorControllerOptions.cursors` |
| `WadoInstanceRequest` | `postWadoInstance` | `WadoInstanceRequest` |
| `KeyObjectRequest` | `writeKeyObject`, before its `try` | `KeyObjectRequest` |
| `SecondaryCaptureRequest` | `writeSecondaryCapture`, before its `try` | `SecondaryCaptureRequest` |
| `FrameCommitMeta` | `writeFrame` when the sink is ticketed, and `acceptTransferredFrame`'s `target`, on the caller's object | `FrameCommitMeta` |

`acceptTransferredFrame` builds an internal object with a `handoff` field and passes it to the adapter's `commitAllocated`. The public check runs on the caller's `target` before that object exists, and the adapter's internal parse accepts the library's own `handoff` field. A caller cannot pass `handoff` through `writeFrame`, because the public list does not contain it.

Error mapping, messages unchanged:

| Site | Today | Code |
|---|---|---|
| `bulk.ts` allocation refused, all three, and stack allocation refused | `Error` | `AllocationRefused` |
| `bulk.ts` commit refused, both, and stack configuration refused | `Error` | `CommitRefused` |
| `bulk.ts` handoff length disagrees, and stack length outside the bulk limit | `Error`, `RangeError` | `LengthMismatch` |
| `bulk.ts` series is not a u64, `<name>` is not a u32, frame meta is not an object | `RangeError`, `TypeError` | `InvalidValue`, the last one `InvalidOptions` |
| `bulk.ts` selected-frame export is unavailable | `Error` | `ExportUnavailable` |
| `commands.ts` payload length, not a u16, not a u32, not an i32 | `RangeError` | `InvalidValue` |
| `instance-bytes.ts` instance body was not committed | `Error` | `CommitRefused` |
| `dicomweb.ts` every `DicomwebError` | `Error` subclass | unchanged codes, now an `OcelliError` subclass |
| `load.ts` initialiser failure | resolves `false` | rejects `CoreLoadFailed` with `cause` |

`index.ts` exports `OcelliError` and `type OcelliErrorCode`. `index.test.ts`'s `PUBLISHED` list gains `OcelliError`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Any caller of the eight entry points above | `refuseUnknownOptions` with that type's list | Runs before any side effect: no fetch, no alloc, no listener attached | `OcelliError` `UnknownOption`, `option` set |
| `examples/viewer-react/src/core-worker.ts` `start` | `loadCore` | Catches the rejection, keeps today's empty-tier status, and logs `error.code` | Same page text as today on failure |
| `examples/viewer-react/src/core-panel.tsx`, `measurement-report.ts`, `App.tsx` | `postWadoInstance`, `createPointerDispatch`, `writeKeyObject`, `writeSecondaryCapture` | Their literals carry declared keys only today | Unchanged, proved by their existing tests staying green |
| `@ocelli/react` | Imports from `@ocelli/core` | Does not construct these options | Unchanged |
| F-224, later | `DICOMWEB_CLIENT_OPTION_KEYS` and `WadoInstanceRequest`'s list | Adds `cache`, and the compile-time check forces the list edit | Its own ACs |
| F-213 `LIBRARY_FILES`, later | The relative import closure of `index.ts` | Two new modules join the closure | F-213 reads the tree that holds them |
| F-215, later | `KeyObjectRequest` list | F-215 changes the request's fields, and the compile-time check forces the list edit | Its own ACs |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No load attempted | `loadCore(init)`, init resolves | `ready` true, resolves `true` | none |
| No load attempted | `loadCore(init)`, init rejects or throws synchronously | rejects `CoreLoadFailed`, `cause` the thrown value | `ready` false, `pending` cleared, `coreAvailable()` false |
| A failed load | `loadCore(init2)`, init2 resolves | init2 called, resolves `true` | none |
| A load in flight | second `loadCore` before it settles | same promise, the initialiser is not called twice | both callers see the same rejection when it fails |
| Loaded | `loadCore(init3)` | resolves `true`, init3 not called | unchanged |
| A `DicomwebClient` call | options with an unknown key | throws before `fetch` | no request made, no sink write |
| `createPointerDispatch` | unknown key | throws before a dispatcher exists | no listener attached |
| `writeFrame` ticketed | meta with an unknown key | throws before `alloc` | no allocation, no ticket captured, the previous slot untouched |

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Bulk slot in `writeFrame`, `acceptTransferredFrame`, `writeStack` | `alloc`, ticket | ticket | non-zero commit code | Unknown key: refused before `alloc`. Coded refusal after `alloc`: the ticket is released, as today |
| `loadCore` pending promise | `loadCore` | the promise object | settle | Failure clears `pending` so the next call retries |
| `ready` flag | `loadCore` success | realm | `ready = true` | Never set on failure |

The key check is the first statement of each entry point, so no side effect precedes a refusal. An identical repeated bad call throws the same code again and changes nothing.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | For each of the eight types, a valid object plus `zzUnknown: 1` at its entry point throws `OcelliError`, `code` `UnknownOption`, `option` `"zzUnknown"`, message `<Owner> has no option "zzUnknown"`. The injected `fetch`, `alloc` and listener spies record zero calls | `options.test.ts`, `refuses an unknown key at every entry point`, one case per row of the table | A check on some entry points only. Mutation: delete the call in `createPointerDispatch` and see its case go red | implementation |
| AC2 | The same valid objects without the extra key behave exactly as today | every existing test in `dicomweb.test.ts`, `pointer.test.ts`, `cursor.test.ts`, `instance-bytes.test.ts`, `tid2010.test.ts`, `secondary-capture.test.ts`, `bulk.test.ts` green, unmodified except for the class assertions AC6 changes | A key list missing a real field, which refuses valid input | implementation |
| AC3 | A declared optional key set to `undefined` is accepted. A symbol key is refused with `option` `String(symbol)`. A key of 100 characters is quoted cut to 64. `null`, an array and `7` as options throw `InvalidOptions` | `options.test.ts`, `edge keys` | `Object.keys`, which misses a symbol key. Treating `undefined` as present and unknown | implementation |
| AC4 | `createCursorController({ ...valid, cursors: { scrol: pair } })` throws `UnknownOption` with owner `CursorControllerOptions.cursors` and `option` `"scrol"`. Every `STACK_COMMANDS` name is accepted | `cursor.test.ts`, one added case | Checking only the top level | implementation |
| AC5 | Census: the set of exported `*Options` and `*Request` names found by regex over `packages/core/src/*.ts`, tests excluded, plus `FrameCommitMeta`, equals `Object.keys(OPTION_KEY_LISTS)` | `options.test.ts`, `every options type has a key list` | A new `*Options` type with no list. Mutation: add `export interface ZzOptions {}` to a scratch copy and see it go red | implementation |
| AC6 | Every row of the error mapping table throws `OcelliError` with that code and the message it has today. `DicomwebError` is `instanceof OcelliError` and `instanceof Error`, keeps `name` `"DicomwebError"`, its `code` and `status`, and has no `cause` | `errors-class.test.ts`, one case per row, plus the existing `toThrow("allocation refused")` style assertions unchanged. The two `toThrow(RangeError)` assertions in `commands.test.ts` lines 210 and 213 and the one in `bulk.test.ts` line 250 become `OcelliError` with `InvalidValue`, recorded as the declared change | A new class whose message changed, which a caller matching text today would miss. A `DicomwebError` that keeps the fetch error as `cause`, which could retain a URL | implementation |
| AC7 | Errors the caller raised pass through by identity: `submitCommands` with a throwing `write`, `openDecodePool` with a throwing `start`, `writeStack` with a sink whose `configure_stack` throws | `errors-class.test.ts`, `caller errors are rethrown unchanged`, asserting `toBe` the same object | Wrapping the caller's error in `OcelliError` | implementation |
| AC8 | `loadCore(() => Promise.reject(e))` rejects with `OcelliError` `CoreLoadFailed` whose `cause` is `e` by identity. A synchronous `throw e` does the same. `coreAvailable()` stays false. The next `loadCore` with a resolving initialiser resolves `true` and calls it once | `load.test.ts` and `load-retry.test.ts`, rewritten for the rejection | `.catch(() => false)` kept. A rejection without `cause`. A `pending` that is not cleared, which S43 pass 1 S2 found once | implementation |
| AC9 | Two concurrent `loadCore` calls over one failing initialiser call it once and both reject with the same error object | `load.test.ts`, `concurrent callers share one failure` | A second initialiser call | implementation |
| AC10 | The return type is `Promise<true>`. A `.ts` fixture assigning the result to `boolean` compiles, and one assigning `false` to it fails | `npm run typecheck` over `load.types.test.ts` with a `@ts-expect-error` line, built through whichever test project F-221 leaves in place | `Promise<void>` or `Promise<boolean>` | implementation |
| AC11 | `examples/viewer-react` shows today's text on a failed load and logs the code | Review of the `start` diff in `core-worker.ts`, which has no test file today and runs `self.onmessage` at import, plus `examples/viewer-react`'s existing `core-panel` and `core-strict` tests green | An unhandled rejection from `void start(...)` that leaves the worker silent | implementation, then review |
| AC12 | `index.ts` exports `OcelliError`. `PUBLISHED` in `index.test.ts` lists it | `index.test.ts` | A class that is not reachable from the package root | implementation |
| AC13 | `npm run lint`, `npm run typecheck`, `npm test`, `bin/ocelli.sh gate packages` and `bin/ocelli.sh gate types` green | named gates | A shipped `.d.ts` that drops `cause` or `option` | implementation |
| AC14 | `CHANGELOG.md` `## Unreleased` has a `### Changed` entry naming the three changes, and the user-facing pages describe the new behaviour | review | A breaking change with no record | implementation, then review |

## Approach

1. `ocelli-error.ts` with the class and the code union. Move `DicomwebErrorCode` there and re-export it from `dicomweb.ts`. `DicomwebError` extends it and passes no `cause`.
2. `options.ts` with `refuseUnknownOptions`, the eight lists, their completeness checks and `OPTION_KEY_LISTS`.
3. One call at the top of each entry point in the table. In `tid2010.ts` and `secondary-capture.ts` it sits before the `try`.
4. Replace each `throw` in the mapping table. Messages are copied from the line being replaced, not re-typed.
5. `load.ts`: `.catch((cause: unknown) => { throw new OcelliError("CoreLoadFailed", "The imaging core did not load.", { cause }); })`, return type `Promise<true>`. The `Promise.resolve().then(...)` structure and the `finally` stay.
6. `examples/viewer-react/src/core-worker.ts`: `try { await loadCore(...) } catch (error) { console.error(...); published = false; postStatus(""); return; }`.
7. Tests, CHANGELOG, LLD and the three user-facing documents.

Rejected: a schema library such as zod, which is a new shipped dependency for eight key lists. A `Proxy` that throws on unknown reads, which catches reads and not writes, so a misspelt key the code never reads is still silent. Freezing option objects, which refuses nothing.

## Dependencies, write ownership and resources

No declared dependency. Write set:

- `packages/core/src/ocelli-error.ts` (new), `options.ts` (new), `options.test.ts` (new), `errors-class.test.ts` (new), `load.types.test.ts` (new)
- `packages/core/src/dicomweb.ts`, `bulk.ts`, `commands.ts`, `instance-bytes.ts`, `pointer.ts`, `cursor.ts`, `tid2010.ts`, `secondary-capture.ts`, `load.ts`, `index.ts`
- `packages/core/src/load.test.ts`, `load-retry.test.ts`, `index.test.ts`, `bulk.test.ts`, `commands.test.ts`, `cursor.test.ts`
- `examples/viewer-react/src/core-worker.ts`
- `CHANGELOG.md`, `README.md`, `examples/folder-viewer/src/use.html`
- `docs/lld/public-api.md`, `docs/lld/errors.md`, `docs/lld/dicomweb.md`, `docs/lld/typescript-packaging.md`, at `/complete-feature`

Shared files and order. **F-230 lands before F-224**, because F-224 adds `cache` to `DicomwebClientOptions` and `WadoInstanceRequest`. With this story first, F-224 adds one key to each list and the compile-time check makes that edit impossible to forget, and F-224's refusal of a bad cache mode is already a coded `DicomwebError`. With F-224 first, this story would have to re-derive F-224's key and rework its refusal. F-230 also lands before F-213, whose `LIBRARY_FILES` test compares a list with the import closure of `index.ts`, which gains `ocelli-error.ts` and `options.ts`. F-230 lands before F-215, which edits `tid2010.ts` and `index.ts`. It shares no file with F-216, F-221, F-208 to F-212 or F-214, so it may start now in parallel with F-216. `examples/folder-viewer/src/use.html` is shared with every viewer story and is the one file to merge with care. No GPU, browser, corpus or network resource.

## Verification and evidence reuse

```bash
npm run build
npm run lint
npm run typecheck
npx vitest run packages/core examples/viewer-react
bin/ocelli.sh gate packages
bin/ocelli.sh gate types
python3 scripts/prose_check.py
```

Final: `bin/ocelli.sh gate --sprint` at sprint verification. Mutations, each applied, observed red, reverted: remove the check from `createPointerDispatch` (AC1), restore `.catch(() => false)` (AC8), drop `cause` (AC8), wrap the caller's `write` error (AC7), add an undeclared field to `PointerDispatchOptions` without its list (AC13 typecheck). Evidence is invalidated by any change to the files in the write set. No performance claim. The check is one `Reflect.ownKeys` per call on a construction or write path, never per pointer sample, because `createPointerDispatch` checks once at creation.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The checks run at construction and at the start of a write, not in a frame
- unsafe: none
- Tier A (WebGPU): n/a. No rendering or compute changes
- Tier B (WebGL2): n/a, for the same reason
- Tier C (CPU): n/a, for the same reason

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1, AC3, AC4, AC5 key refusal and census | `packages/core/src/options.test.ts`, `cursor.test.ts` |
| unit | AC6, AC7 error mapping and pass-through | `packages/core/src/errors-class.test.ts` |
| unit | AC8, AC9 load rejection, retry and sharing | `packages/core/src/load.test.ts`, `load-retry.test.ts` |
| contract | AC10 return type | `packages/core/src/load.types.test.ts` under `npm run typecheck` |
| contract | AC12 package root exports | `packages/core/src/index.test.ts` |

No `fixture` row, because nothing here does pixel or geometry arithmetic.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` counts viewport types, tools, blend modes, VOI functions, transfer syntaxes, segmentation representations, events and adapters. Option validation and the error class are none of them.

## Deviations

None. The HLD specifies no TypeScript error class or option rule, and section 23's stable-code bullet is applied, not departed from.

## LLD impact

- `docs/lld/public-api.md`: a new section "Options and refusals" with the in-scope rule, the eight types and the census.
- `docs/lld/errors.md`: a section saying the shell's string codes on `OcelliError` are a separate namespace from the numeric ring codes, and why.
- `docs/lld/dicomweb.md`, "Transport validation and errors": `DicomwebError` extends `OcelliError`, and an unknown option is `UnknownOption`, not a transport class.
- `docs/lld/typescript-packaging.md`, "The absent-core seam": `loadCore` resolves `true` or rejects `CoreLoadFailed` with the cause, and the example page catches it.

## User-facing documentation impact

- `README.md`, the `DicomwebClient` paragraph under "Use the library": one sentence that an option the client does not know is refused with `OcelliError` code `UnknownOption`, which names it.
- `examples/folder-viewer/src/use.html`, 5.1 "Load the core in a worker": the sentence "A thrown initialiser resolves `false` and leaves the flag false" becomes the rejection with `CoreLoadFailed` and `error.cause`, and the sample uses `try` and `catch` instead of `if (loaded && ...)`. 5.6 "DICOMweb": the comment in the `catch` adds that `DicomwebError` is an `OcelliError` and that an unknown option is `UnknownOption`. 15 "Errors": the last sentence about `DicomwebError` gains one sentence naming `OcelliError`, its string codes, and that they are not the numeric ring codes.
- `examples/folder-viewer/src/more.html`: no change. It names library behaviour the viewer does not draw, and none of its sections describes options, the load call or shell errors.

## Open questions

None open. Every judgement call is a lead decision.

- Q1, which objects are in scope. Answered: lead decision under the operator's delegation of 2026-11-05. `*Options`, `*Request` and `FrameCommitMeta`, with a census, decision 1.
- Q2, one class or the pattern generalised. Answered: lead decision under the operator's delegation of 2026-11-05. `OcelliError` as the base and `DicomwebError` as a subclass, decision 3.
- Q3, the `loadCore` signature. Answered: lead decision under the operator's delegation of 2026-11-05. `Promise<true>` that rejects `CoreLoadFailed` with the cause, decision 9.
- Q4, compatibility. Answered: lead decision under the operator's delegation of 2026-11-05. In place under 0.x with a CHANGELOG entry, no version bump and no shim, decision 10.
- Q5, order against F-224. Answered: lead decision under the operator's delegation of 2026-11-05. F-230 first, recorded under Dependencies.
