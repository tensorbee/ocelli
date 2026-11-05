# F-230 review, pass 1

**Reviewed**: the staged tree `1501c7b8f35a81c014d64622cf5999d2319e5a41` on `work/f-230-xai`, over base `7a0cabb251c07a58c6c68ee7b0ea0b06034ad3c1` (`git merge-base HEAD sprint/s46`, which is also HEAD). 24 staged paths, no unstaged tracked changes. Untracked `tools/oracle/out` ignored as instructed. Design `.claude/plans/F-230-design.md` (approved).
**Author / reviewer**: implementation by an external coding agent (its report is in the session scratchpad, not tracked). Review by Claude Opus 5.5 as an independent `/microscope --working` subagent that wrote none of the implementation or the design.
**Result**: 3 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **AC1, eight entry points.** Every refusal call was mutated away in turn (cursor top level, cursors record, `searchSeries`, `postWadoInstance`, `acceptTransferredFrame`, `writeKeyObject` moved inside its `try`) and a named test went red each time. Zero `fetch` calls held under a probe against the built `dist`. **Not rejected by the test:** the shape of the refusal. `searchStudies` throws synchronously and its two siblings reject, which `raised()` cannot tell apart because it awaits inside its own `try`. That is D1.
- **AC2, existing tests unmodified.** One test was changed (`dicomweb.test.ts`, `responseSink()` spread). That is ruling 2 below. Every call site outside `packages/core/src` was read: `examples/viewer-react/src/App.tsx` (`writeKeyObject`, `writeSecondaryCapture`), `core-panel.tsx` (`postWadoInstance`), `core-worker.ts` (`writeFrame`), `measurement-report.ts` (`createPointerDispatch`), `tools/bench/page/session-worker.mjs` (`writeFrame` twice, conditional `stackIndex`/`stackEpoch` spread) and `scripts/package_check.py` (type-only use). Each passes declared keys only. `packages/react` calls none of the checked functions. `examples/folder-viewer` serves no `/lib/` library in this tree. Its `src/` holds Rust and three HTML pages, and `grep -rn 'lib/' examples/folder-viewer/src` prints nothing. F-213 adds that route later.
- **AC3, edge keys.** The test covers `undefined` values, symbols, the 64-unit cut, inherited keys and `null`, array and `7` options. Mutating `Reflect.ownKeys` to `Object.keys`, accepting symbols, or changing the limit to 65 each turned "edge keys" red. Probe: `cursors: []`, `cursors: null` and a symbol-keyed cursors record are refused with the right owner. A key whose 64th unit is a high surrogate is cut mid-pair. That is N1.
- **AC4, nested cursors.** Mutating the record check away turned "refuses an unknown cursor command and accepts every stack command" red.
- **AC5, census.** Adding `export interface ZzOptions` to `pointer.ts` turned "every options type has a key list" red. `packages/core/src` has no subdirectories, so the top-level `readdirSync` covers it.
- **AC6, mapping table.** Changing the `writeDicomwebResponse` code to `CommitRefused` turned "maps allocation refusals" red. Mutating `DicomwebError` to carry a `cause` turned its test red. **Rejected by probe:** decision 5's verbatim-message rule at the two public entry points for a non-object meta. That is D2.
- **AC7, caller errors by identity.** Wrapping the `writeStack` `configure_stack` error turned two tests red.
- **AC8 and AC9, loadCore.** `.catch(() => false)` and a dropped `cause` each turned three tests red. A probe on `dist` showed two concurrent callers getting the same promise and the same rejection object, `cause` matching by identity, `coreAvailable()` false, then a retry resolving `true` with one initialiser call, then no call once ready.
- **AC10, Promise<true>.** Changing the return type to `Promise<boolean>` failed typecheck at `load.types.test.ts(24,7)`. Changing it to `Promise<void>` failed at four sites.
- **AC13, compile-time drift.** Adding `zzExtra?` to `KeyObjectRequest` failed at `options.ts(106,7)`. Removing `"now"` from the pointer list failed at `options.ts(61,7)` and also turned 24 pointer tests red. The shipped `dist/ocelli-error.d.ts` declares `option` and `details.cause`.
- **AC11, worker.** `start` catches the rejection, logs `error.code`, posts an empty tier and returns. It was read in review only, because no browser runs it.
- **AC14, docs.** README.md and use.html were checked line by line (see Verified clean, D3 and N3). CHANGELOG and the LLD are deliberately left for completion, as the handoff below says.

## Rulings on the implementer's two contradictions

**Ruling 1, `bulk.test.ts` line 250: the implementer is right, and the plan's AC6 sentence is wrong.** The `RangeError` comes from the engine, raised by `new Uint8Array(memory.buffer, ptr, len)` when the caller's `BulkSink.alloc` returns a pointer past the end of memory. The shell has no `throw` there. The mapping table enumerates throw sites, and this one is not among them. Decision 6 passes collaborator failures through. The non-goals exclude validating anything beyond what each function already checks. The sibling engine errors at `bulk.test.ts` lines 517 and 562, the `TypeError`s on a detached buffer, would remain engine errors anyway, so coding only this one would make the class less uniform, not more. A shell bounds pre-check would be a new validation of a collaborator's return value, which belongs to a separate story if anyone wants it. **Required at completion:** amend AC6 to strike the line 250 reference. Word the CHANGELOG entry as "errors the shell raises from bulk and command calls" rather than every `RangeError` and `TypeError`, because engine errors from typed-array views still escape as `RangeError` and `TypeError`.

**Ruling 2, `commits` refused as `UnknownOption`: refusing it is right.** It is an own, undeclared, enumerable key. Spreading a larger object is exactly how TypeScript's excess-property check gets bypassed, and that is the hole this story's run-time check exists to close. Before the edit the test passed for the wrong reason: the spread key was silently ignored. The edited test still asserts the configured-URL sanitisation it was written for. No real consumer passes an extra key (call sites listed under AC2). **Required at completion:** record the `dicomweb.test.ts` edit as a declared change to AC2, and add `dicomweb.test.ts` to the write set. The plan lists neither.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | staged tree 1501c7b8 | now | 0 |
| `npm run typecheck` | same | now | 0 |
| `npm run lint` | same | now | 0 |
| `npm test` | same | now | 0, 60 files, 484 tests |
| `npm run build --workspace @ocelli/example-viewer` | same, wasm pkg present | now | 0 |
| `bin/ocelli.sh gate packages` | same | now | 0, ALL GREEN |
| `bin/ocelli.sh gate types` | same | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | same | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | same | now | 0, 1055 refusals, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | same | now | 0 |
| `python3 scripts/prose_check.py` | same | now | 0, 844 files clean |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1674 files clean |
| 17 source mutations, `npx vitest run packages/core` and/or `npm run typecheck`, each restored with `git checkout --` from the index and byte-compared | same | now | every one red, named tests above |
| 2 return-type mutations of `loadCore`, typecheck | same | now | both red |
| Node probe against rebuilt `packages/core/dist` | same, after `npm run build` restored dist | now | D1 and D2 outputs quoted below |
| `git write-tree` at the end | | now | `1501c7b8f35a81c014d64622cf5999d2319e5a41`, unchanged |

Environment: macOS, Node v24.13.0, vitest 4.1.11. No GPU, browser or corpus used, and this story needs none.

## Defects

### D1, `searchStudies` throws synchronously where its siblings reject

**Where**: `packages/core/src/dicomweb.ts:72`, `searchStudies` is not `async`, so `refuseUnknownOptions` at line 73 throws out of the call itself.
**What**: `client.searchStudies({ zz: 1 })` and `client.searchStudies(null)` throw synchronously. `searchSeries` and `searchInstances`, which share the same type, owner string and check, return a rejected promise. On base `searchStudies` could never throw synchronously, because it only returned `this.#qido(...)` and `#qido` is `async`. So a caller written as `client.searchStudies(opts).catch(report)`, or one that collects promises for `Promise.allSettled`, now gets an uncaught exception.
**Why it is wrong**: three sibling methods of one public type give two different failure channels for the same refusal. The change also moves a pre-existing rejection (`null` options) onto the synchronous channel. The design's state table promises "no request made" and "throws before fetch", which holds, but nothing in the design authorises changing a promise-returning method's failure channel.
**Evidence**: probe on the rebuilt `dist`: `searchStudies THREW SYNCHRONOUSLY UnknownOption`, `searchSeries returned promise -> rejected UnknownOption`, `searchInstances returned promise -> rejected UnknownOption`, `searchStudies(null) THREW SYNCHRONOUSLY InvalidOptions`. The AC1 test cannot see the difference because `raised()` awaits inside its own `try`.
**Scope and regression**: present only in this diff. Affects any promise-style caller of `searchStudies`. use.html 5.6 uses `await` inside `try`, so it is unaffected. Smallest fix: make `searchStudies` `async`. Smallest regression test: `const p = client.searchStudies(withKey({}, "zzUnknown", 1)); expect(p).toBeInstanceOf(Promise); await expect(p).rejects.toMatchObject({ code: "UnknownOption" })`, repeated for all three methods.

### D2, a non-object frame meta lost its message at the two public entry points

**Where**: `packages/core/src/bulk.ts:196` (`writeFrame`, ticketed) and `bulk.ts:392` (`acceptTransferredFrame`). `refuseUnknownOptions` now runs before `frameCommitMeta` and throws its own message.
**What**: before this change, `writeFrame(wasm, ticketedSink, bytes, null)` threw `TypeError("frame meta is not an object")`. It now throws `OcelliError` `InvalidOptions` with the message "FrameCommitMeta is not an options object". `acceptTransferredFrame` with a `null` target does the same. The code is right. The message is not the one it was.
**Why it is wrong**: design decision 5 says "Existing messages are kept verbatim ... so a caller matching a message today still matches". AC6 says every row of the mapping table throws "with the message it has today", and the table's row for "frame meta is not an object" gives `InvalidOptions` with the message unchanged. The AC6 test reaches that message only by calling `sessionBulkSink(...).commitAllocated(4, 4, null)` directly (`errors-class.test.ts:243`), a path no public `writeFrame` or `acceptTransferredFrame` caller takes. So the row passes through a route that hides the change.
**Evidence**: probe on `dist`: `writeFrame null meta: InvalidOptions "FrameCommitMeta is not an options object"`, `writeFrame 7 meta:` the same, `accept null target + length mismatch:` the same.
**Scope and regression**: present only in this diff. Affects text-matching callers of the two functions. Smallest fix: keep the existing message for the non-object case at these two sites, for example by a non-object check ahead of `refuseUnknownOptions` that throws `InvalidOptions` with "frame meta is not an object". Regression: `writeFrame(wasm, ticketed, bytes, null)` and `acceptTransferredFrame(..., null)` each `toThrow("frame meta is not an object")` with `code` `InvalidOptions`.

### D3, use.html 5.1's catch comment is false for one path of its own sample

**Where**: `examples/folder-viewer/src/use.html:284-295`
**What**: the `try` now wraps both `loadCore(...)` and `await wasm.resolvedTier()`. The `catch` comment reads "error.code is CoreLoadFailed. error.cause is the initialiser failure." When `resolvedTier()` rejects, the error lands in the same `catch`, has no `code` and no `cause`, and the comment is false.
**Why it is wrong**: `.claude/commands/microscope.md` section 3 counts every behaviour sentence in use.html, and a false one is a defect. The old sample had no such claim.
**Evidence**: read of the staged page against `load.ts`. `resolvedTier` is a wasm export outside `loadCore`.
**Scope and regression**: documentation only, in this diff. Smallest fix: close the `try` after `loadCore`, or reword the comment to cover only a load failure.

## Smells

None.

## Nitpicks

- **N1.** `options.ts` `shownKey` cuts at 64 UTF-16 units with `slice`, so a key whose 64th unit is a high surrogate leaves a lone surrogate. The probe printed `...\ud83d` in the message, and `error.option` holds the half pair. Cutting on a code-point boundary would avoid it.
- **N2.** `options.test.ts` constructs the same refused `DicomwebClient` twice in a row in "refuses an unknown key at every entry point". The second call adds no coverage.
- **N3.** use.html 5.5's comment "Throws "allocation refused" or "commit refused"" lists two messages out of the several `writeFrame` can now raise, including `UnknownOption` and `InvalidValue`. It was already incomplete before this diff. Naming `OcelliError` and its codes there would match 5.1 and 15.

## Verified clean

- `OcelliError`: `name`, `code`, `option` only for `UnknownOption`, and `cause` only when passed. `DicomwebError` extends it, keeps its name, five codes, messages and `status`, and passes no `cause`. A thrown `fetch` is not retained, which the probe and a mutation both confirm.
- `DicomwebErrorCode` moved to `ocelli-error.ts` and is re-exported from `dicomweb.ts` and `index.ts`. `OcelliError` and `OcelliErrorCode` are exported from the package root, `options.ts` is not, and `index.test.ts` `PUBLISHED` lists `OcelliError`.
- Each of the 22 mapped throws in `bulk.ts`, `commands.ts` and `instance-bytes.ts` was checked against the base text. Messages are byte-identical and codes match the design table.
- The eight key lists match their types field for field (`satisfies` one way, `Exclude` the other). `OPTION_KEY_LISTS` has exactly the eight names.
- `acceptTransferredFrame` checks the caller's `target` and never its own internal `handoff` object, and `bulk.test.ts:583` still sees `{ handoff, ...target }`.
- The refusal is the first statement at every entry point. `writeKeyObject` and `writeSecondaryCapture` check before their `try`. The non-ticketed `writeFrame` path does not check, as the design says.
- `loadCore`: the `Promise.resolve().then` structure and the `finally` clear are unchanged, and there is no cast.
- README.md line 349-350 is true for the constructor and the three search methods. use.html section 15's new sentences are true. more.html has no stale sentence, and its samples pass declared keys only.
- No `wasm-bindgen`, `unsafe`, pixel arithmetic, render-loop or tier change. All tiers n/a, as declared.

## Limitations and handoff

- **Not exercised:** `core-worker.ts` in a real browser with a failing load (AC11), which was read in review only. The dual CJS and ESM `instanceof OcelliError` hazard was not looked at. It predates this story through `DicomwebError`.
- **Carried concern, already on base, not in this diff:** `acceptTransferredFrame` builds `committed` without `stackIndex` or `stackEpoch` (base `bulk.ts:389-395`). F-230's key list now declares those two keys accepted there, and a correctly spelt selection is still committed down the generation path. The probe printed the committed keys as `handoff, viewport, columns, rows, generation`. This is the same quietly-ignored-option class at the declared-key level. The lead should file a story or document the handoff path's limit. It does not block F-230.
- **Left for `/complete-feature`, confirmed absent from the tree:** the AC14 `CHANGELOG.md` `### Changed` entry (`## Unreleased` still reads "Repository bootstrap"), worded as ruling 1 requires. The four LLD edits, including `docs/lld/typescript-packaging.md` lines 254-256, which still say the load "resolves false". The AC2 and AC6 plan amendments from rulings 1 and 2.
- **Next:** the implementer fixes D1 to D3. A fresh independent pass 2 then replays D1's three-method probe and D2's two null-meta calls through the public functions, and checks the regression tests go red against the current code.
- Mutations were applied to the working tree and restored from the index. Each restore was byte-compared, and `git write-tree` is unchanged at `1501c7b8`. `packages/core/dist` was rebuilt after the type mutations, because `tsc --build` had emitted a mutated `load.d.ts`. That rebuild produces untracked output. This report is the only file written.

F-230 pass 1: 3 defects, 0 smells, 3 nitpicks.
