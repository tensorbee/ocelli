# F-230 review, pass 3

**Reviewed**: the staged tree `83fa5e84caa83eb99036a8bb2f08e9c4db6c924c` plus the unstaged diff on `work/f-230-xai`, whose `git diff` hashes to `905911e30607a8ec761ba8e11a2c5c584dcd7f27`. Base `7a0cabb251c07a58c6c68ee7b0ea0b06034ad3c1` (HEAD). The pass 2 D1 remediation is staged, in `examples/folder-viewer/src/use.html`. Untracked `tools/oracle/out` and the pass 1 and pass 2 reports ignored.
**Author / reviewer**: implementation by an external coding agent. The D1 remediation by the sprint lead. Review by Claude Opus 5.5 as an independent `/microscope --working` subagent that wrote none of the implementation, the remediation, the design or the earlier passes.
**Result**: 0 defects, 0 smells, 5 nitpicks

## Prior findings

| Pass 2 | State | How it was replayed |
|--------|-------|---------------------|
| D1, use.html 5.6 comment in a branch that never sees `UnknownOption` | **Closed** | See below |
| N1 to N5 | Carried, unchanged | `shownKey` still slices at 64 units. `options.test.ts:156` and `:160` still construct the same client. use.html 5.5 still lists two messages. The ticketed `writeFrame` guard at `bulk.ts:197` still passes an array through to the generic refusal. The plan amendments are still appended, with the rows they override left standing |

### D1 replay

- **Import.** The sample imports `OcelliError` from `@ocelli/core`. `packages/core/package.json` exports `.` as `./dist/index.js` and `./dist/index.d.ts`, and after `npm run build` both re-export `OcelliError` from `./ocelli-error.js` (`dist/index.js:27`, `dist/index.d.ts:28`).
- **Branch order.** `DicomwebError extends OcelliError` (`dicomweb.ts:19`, and `Object.getPrototypeOf(DicomwebError.prototype) === OcelliError.prototype` is true on `dist`). The sample tests `DicomwebError` first and `OcelliError` in the `else if`, so a transport error keeps its own branch. Reversing them would be wrong: on `dist` a `DicomwebError` `InvalidRequest` is `instanceof OcelliError` true.
- **Comment claims.** "Any other shell error" matches more.html and use.html's own definition, "`OcelliError` is the shell error", and every non-transport `throw` in `dicomweb.ts`, `options.ts` and `bulk.ts` is an `OcelliError`. "error.code is UnknownOption and error.option names the key" matches `refuseUnknownOptions`, which passes `{ option }` only for `UnknownOption`, and `OcelliError`, which sets `option` only when given and documents it as set only for `UnknownOption`. The name is cut at 64 units, which is N1.
- **Executed.** A scratchpad module importing the bare specifier `@ocelli/core` (symlinked to this worktree's built package) ran the sample's `catch` verbatim with a fetch that counts calls:

| Call | Branch taken | Fetches |
|------|--------------|---------|
| `searchStudies({ filter: [...], limit: 10 })` | `OcelliError` `UnknownOption`, `option` `filter` | 0 |
| `searchStudies({ filters: [...], limt: 10 })` | `OcelliError` `UnknownOption`, `option` `limt` | 0 |
| `searchSeries("1.2.3", { limt: 1 })` | `OcelliError` `UnknownOption`, `option` `limt` | 0 |
| `searchStudies(null)` | `OcelliError` `InvalidOptions`, `option` undefined | 0 |
| `searchStudies` with valid keys, fetch rejects | `DicomwebError` `Network` | 1 |
| `searchSeries("x")` | `DicomwebError` `InvalidRequest` | 0 new |

## Sweep, every error-catching sample in the three documents

`grep -n "catch\|instanceof\|error\.code"` over `README.md`, `examples/folder-viewer/src/use.html` and `examples/folder-viewer/src/more.html` finds two `catch` blocks, both in use.html. README and more.html have none.

- **5.1, `loadCore`.** The `try` wraps only `loadCore`, whose only rejection is the `.catch` at `load.ts:36-37` building `CoreLoadFailed` with `cause`. The comment holds for every error its `try` can see.
- **5.6, DICOMweb.** Closed above. The constructor sits outside the `try` and its comment claims nothing about it.
- 5.5's `writeFrame` sample has a comment but no `catch`, so it is outside this class, and its incompleteness is N3.

## Evidence

| Command | Source identity | Executed now | Exit and observation |
|---------|-----------------|--------------|----------------------|
| `npm run build` | staged 83fa5e84 plus unstaged 905911e3 | now | 0 |
| `npm run test` | same | now | 0, 60 files, 486 tests |
| `CARGO_TARGET_DIR=<wt>/target cargo test -p ocelli-folder-viewer --test preview` | same | now | 0, 32 passed |
| `python3 scripts/prose_check.py` | same | now | 0, 844 files clean |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1674 files clean |
| Node probe against built `@ocelli/core` | same, after build | now | the table above |
| `git write-tree` and `git diff \| git hash-object --stdin` at the end | | now | `83fa5e84...` and `905911e3...`, unchanged |

Environment: macOS, Node v24.13.0. No GPU, browser or corpus used, and this story needs none.

## Defects

None.

## Smells

None.

## Nitpicks

- **N1 to N5**, carried from pass 2 unchanged, as tabled above.

## Verified clean

- The staged use.html diff against HEAD: 5.1's prose and sample, 5.6's import and new branch, and the new sentence in the error chapter. README line 349 to 350 against `options.ts` and the probe.
- No `wasm-bindgen`, `unsafe`, pixel arithmetic, render-loop or tier change. All tiers n/a, as declared.

## Limitations and handoff

- **Not exercised:** `core-worker.ts` with a failing load in a real browser (AC11), as in passes 1 and 2. The dual CJS and ESM `instanceof` hazard, which predates this story.
- **Carried, not in this diff:** `acceptTransferredFrame` drops `stackIndex` and `stackEpoch`. Still for the lead to file or document.
- **Still for `/complete-feature`:** the CHANGELOG entry worded as the AC6 amendment says (see N5), and the LLD edits.
- The probe ran from the scratchpad. This report is the only file written in the worktree.

F-230 pass 3: 0 defects, 0 smells, 5 nitpicks.
