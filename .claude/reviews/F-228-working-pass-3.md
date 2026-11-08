# F-228 review, pass 3

**Reviewed**: the uncommitted working tree on `work/f-228-xai` against the pass-2 checkpoint `6260ee2`, five tracked paths: `README.md`, `docs/lld/typescript-packaging.md`, `examples/viewer-react/csp-probe.mjs`, `examples/viewer-react/src/csp-policy.ts`, `examples/viewer-react/src/csp-policy.test.ts`. `git diff 6260ee2` is 147 insertions and 75 deletions, sha256 `d2516530a3ea6153...`. Untracked `.claude/reviews/F-228-working-pass-2.md` read as context, `tools/oracle/out` ignored as instructed. `git write-tree` (the index) is `02bf90496c92ae2c36658692906b21b78cf6843a` before and after. Scope is the example-viewer half, AC1 to AC10. AC11 to AC16 are not reviewed.
**Author / reviewer**: remediation by GLM-5.3 in opencode, per its report at the end of `glm-F-228-fix2.log`. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed material.
**Result**: 3 defects, 0 smells, 9 nitpicks

## Acceptance challenged

Every mutation ran on a copy of the working tree under `target/f228-review3/tree` (`git ls-files` through rsync, `node_modules` and `crates/ocelli-wasm/pkg` symlinked), or on a probe copy at `target/f228-review3/probe-abs.mjs` whose `HERE`, policy import and Playwright require point at the real worktree. Nothing ran against a tracked file. The "11 skipped" in the AC2 and AC9 matrices below is my `-t` filter, not a skip in the suite.

- **Pass-2 D1, AC2 compares each line whole.** In the copy, on README and on the LLD separately: the quote extended by `'unsafe-eval'`, a second identical quote line, a second differing quote line, the block deleted, `connect-src` upper-cased, a trailing space, a trailing `\r`, and the DICOMweb line extended by `https://evil.example`. Each red, 15 of 15 (the DICOMweb plant exists only in README). **The original trigger is closed.** Adjacent cases that stay green: a lower-case `content-security-policy:` line, an nginx `add_header Content-Security-Policy "..."` line, a JS `"Content-Security-Policy": "..."` line, and a prose line naming `` `Content-Security-Policy` `` with `'unsafe-eval'`, each added beside the correct quote. The prose that describes the check says it covers every one of those lines (D1).
- **Pass-2 D2, AC9 reads normal edges with every feature.** In the copy, lock refreshed per plant with `cargo metadata --offline`: a `[dev-dependencies]` codec edge green, a `[build-dependencies]` edge green, a dev-dependency of `ocelli-cache` green, a `cfg(not(target_arch = "wasm32"))` edge green. Red with the detection message: `decode-worker = ["dep:ocelli-codec"]`, an implicit optional feature, a plain normal edge, a rename (`codec = { package = "ocelli-codec" }`), a `cfg(target_arch = "wasm32")` edge, and `decode-worker = ["ocelli-cache/codec"]` turning on an optional codec edge in `ocelli-cache`. The baseline with no codec edge is clean. A stale lock under `--locked` is a hard error ("could not resolve the wasm graph"), not a pass. **Closed.**
- **Pass-2 D3, untrimmed entries refused.** Through Node against the real module: trailing `\n`, `\r\n`, leading space, leading tab, trailing U+00A0, leading U+2003, trailing U+3000, leading U+FEFF and trailing U+2028 all refused with "may not carry leading or trailing whitespace". An inner U+00A0 refused as inner whitespace, `"   "` refused, `""` refused as empty. Two valid origins appended in order. Mutation: the `trimmed !== source` branch disabled, "refuses an entry that differs from its trimmed form" goes red. **Closed.** Adjacent: U+0085, U+200B and U+0000 are not JS whitespace and are accepted (N10).
- **Pass-2 D4, every sentence executed.** The blob sentence is true (`BLOB_ALLOWED` is `STRICT` with `worker-src 'self' blob:`). The page-strict detail is true (`/` and `/index.html` alone get `WITHOUT_WASM_EVAL`, and `core ready` follows). The worker's own console calls reaching `page.on("console")` is true, and the browser's own CSP text from the worker reaching neither listener is true: under `no-wasm` the dumped page messages are `["error: CoreLoadFailed"]` and the collector holds `["error: CoreLoadFailed"]` beside the violation `script-src wasm-eval`. The new "at every level" and "at any level" sentences are false (D2). The rewritten CDN sentence names the wrong request (D3).
- **Pass-2 D5.** The folder-viewer paragraph is gone, and `grep "hash based"` over `docs` and `README.md` prints nothing. "Three copies" is gone. The `cargo tree` sentences agree with the AC9 matrix above. The document-check sentence carries D1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | working tree, existing `crates/ocelli-wasm/pkg` not rebuilt | now | 0, `core-worker-Cgmb5Hu4.js` |
| `npm run typecheck` | working tree | now | 0 |
| `npm run lint` | working tree | now | 0 |
| `npm run test` | working tree | now | 0, 61 files, 498 passed (498), no skip and no todo in the log |
| skip grep over every tracked `.ts`, `.tsx`, `.mts`, `.mjs` for `.skip`, `.todo`, `.only`, `skipIf`, `runIf`, `ctx.skip`, `xit(`, `xdescribe(` | working tree | now | nothing printed |
| `vitest run csp-policy.test.ts --reporter=verbose` | scratch copy | now | 12 passed |
| `node examples/viewer-react/csp-probe.mjs` | Chromium 151.0.7922.34, Node 24, corpus present | now | 0, 13 passed, 0 failed, 0 not run |
| probe copy, worker plants `console.assert(false, ...)`, `console.trace`, `console.table`, `console.warn`, and a dump of `messages` and the collector | probe copy | now | see D2 |
| AC2 matrix, 13 plants by 2 documents | scratch copy | now | as listed above |
| AC9 matrix, 10 plants plus a stale lock | scratch copy | now | as listed above |
| `policyHeader` over 17 inputs, plus N9's two | real module through Node | now | as listed above |
| `bin/ocelli.sh gate packages` | working tree | now | 0, 498 passed, ALL GREEN |
| `bin/ocelli.sh gate guards` | working tree | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | working tree | now | 0, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | working tree | now | 0 |
| `python3 scripts/prose_check.py` | working tree | now | OK, 865 files |
| `python3 scripts/source_provenance_check.py` | working tree | now | OK, 1711 files |
| `git write-tree`, `git status --short`, `git diff 6260ee2 \| sha256sum` | worktree | now, before the report and after | unchanged, see Verified clean |

## Defects

### D1, the document check is described as covering every line that names the header, and it covers one spelling
**Where**: `README.md:214-217`, `docs/lld/typescript-packaging.md:312-315`, the test comment at `examples/viewer-react/src/csp-policy.test.ts:139-142`, against the filter at `csp-policy.test.ts:153-155`.
**What**: the prose says every line of the two documents "that names `Content-Security-Policy`" must equal the header, the whole line. The test only looks at lines containing the case-sensitive string `Content-Security-Policy:`. Both sentences refute themselves: `README.md:216` and `docs/lld/typescript-packaging.md:314` each name `` `Content-Security-Policy` ``, neither is a header, and the test is green. The same gap admits drift the sentences promise to catch. Added beside the correct quote, a `content-security-policy: default-src *` line, an nginx `add_header Content-Security-Policy "default-src 'none'; script-src 'self' 'unsafe-eval'" always;` line, a JS `headers: { "Content-Security-Policy": "script-src 'self' 'unsafe-eval'" }` line, and a prose line carrying `'unsafe-eval'` each leave AC2 green, in both documents. A header field name is case-insensitive (RFC 9110 section 5.1), so the lower-case line is the same header. The closing clauses, "holds each document to the number of those lines it carries" and "the number of those lines it claims", are circular. The counts are written in the test (two for README, one for the LLD) and the documents claim nothing.
**Why it is wrong**: a false sentence in the README and the LLD is a defect under section 3 of the review command. Plan AC2 rejects "A fourth copy in prose that drifts", and an nginx or JS config sample is the usual way a README shows this header.
**Evidence**: the 13-plant matrix in the copy. Eight plants red on both documents, the four alternative spellings green on both. `grep -n Content-Security-Policy` lists `README.md:216` and the LLD's `:314` beside the quotes.
**Scope and regression**: README, LLD and the test comment, and the Use book when the folder-viewer half adds it here. Smallest fix: say what the test does, for example "every line containing `Content-Security-Policy:`", and state the counts as the test's own. If the alternative spellings are meant to be covered, match the field name case-insensitively and refuse any line that pairs it with a source list it does not equal. Then replay the lower-case and `add_header` plants.

### D2, "no console message at any level" and "console clean in the page and the worker" are printed while the worker logs at error level
**Where**: `examples/viewer-react/csp-probe.mjs:283-294` (the prelude wraps five methods), the comments at `:22-24` ("the collector records them too, at every level") and `:272-273` ("under every level"), the assertion message at `:592`, the printed detail at `:594`, and the page check at `:564-571`, which keeps only messages containing "Content Security Policy".
**What**: the prelude wraps `error`, `warn`, `info`, `debug` and `log`. In the worker, `console.assert(false, "planted assert")`, `console.trace("planted trace")` and `console.table([...])` each leave `strict` green, and the real probe would still print "console clean in the page and the worker". The dump shows the page listener did receive them (`page=["assert: planted assert"]`, `["trace: planted trace"]`, `["table: [planted table]"]`) while the collector stayed `[]`. The page-side check never sees them because it filters for the CSP text. A failed `console.assert` is an error-level message in Chromium. A planted `console.warn` does turn strict red, so the wrapped five work.
**Why it is wrong**: this round wrote "at any level" and "every level" as part of the D4 remediation, and running them shows they are false. The detail is printed as evidence.
**Evidence**: the probe copy with a `PLANT` env hook appended to the prelude and the strict and no-wasm details replaced by dumps. Strict PASS for assert, trace and table, FAIL for warn. Without a plant, strict's page messages are `[]`.
**Scope and regression**: the strict case and its printed detail. Because the worker's console reaches `page.on("console")` whatever the method, `assert.deepEqual(messages, [])` in strict covers every method on both sides. Its baseline is `[]`, measured. The alternative is to narrow the sentences and the detail to the five methods. Replay the `console.assert` plant and see red.

### D3, the CDN sentence in `policyHeader`'s docstring names the wrong request
**Where**: `examples/viewer-react/src/csp-policy.ts:52-54`, "because the glue's `import()` of the module is a script request".
**What**: the glue does not `import()` the module. It fetches `ocelli_wasm_bg.wasm` (`ocelli_wasm.js:2894-2899`, `new URL(...)` then `fetch`) and compiles it with `instantiateStreaming`, which is `connect-src` and `'wasm-unsafe-eval'`. The script request is the worker's `import()` of the glue (`core-worker.ts:134-137`), which the LLD states correctly ("the wasm-bindgen glue through `import()`"). The conclusion still holds, since a CDN origin would need `script-src` for the glue, but the stated reason is false.
**Why it is wrong**: under CSP Level 3, `import()` of the glue is a script-destination request governed by `script-src`. The module's fetch is governed by `connect-src`. A comment that gets this backwards is written by the same process as the code and will be copied into the next sentence about it.
**Evidence**: `grep -nE "import\(|fetch\(|instantiateStreaming" crates/ocelli-wasm/pkg/ocelli_wasm.js` and `core-worker.ts:126-137`.
**Scope and regression**: one sentence, new in this round. Write "because the worker's `import()` of the glue is a script request".

## Smells

None.

## Nitpicks

- N1, carried. `eslint.config.js`: `console`, `process` and `URL` are never used bare.
- N2, carried. `new Function(` in `csp-policy.ts` passes AC10, because the whole file is exempt.
- N3, carried. `csp-probe.mjs:58-64` reports any import failure as a Node type-stripping problem.
- N6, carried. `node-test-modules.d.ts` also resolves `node:` modules for the example's browser sources.
- N7, carried. `workerProbe` settles 300 ms, so a refusal deferred past that leaves strict green.
- N8, carried. `worker-strict`'s header assertions compare the probe's routing function with itself.
- N9, carried and replayed. `BLOB:https://x.example` and `https://` are accepted. Neither is a valid source expression, so nothing widens.
- N10, new. U+0000, U+0085 and U+200B are accepted inside an entry, because none is JS whitespace. NUL makes a field value RFC 9110 section 5.5 refuses, and `new Headers({a: "x\u0000"})` throws, so the failure is loud rather than a quietly wider policy.
- N11, new, for the lead. `.claude/plans/F-228-design.md:269` still says the LLD section describes the folder viewer's policy "in two sentences". The paragraph was deleted this round as pass 2 offered, so the folder-viewer half has to restore it or the plan's write-set line has to change.

Pass-2 N4 and N5 stay closed.

## Verified clean

- Pass-2 D1's trigger (extension past the end, a deleted block, a new trailing directive) is closed. Eight plants were red on each document, including case, trailing whitespace and CR. What is open is the new D1.
- Pass-2 D2 is closed across ten plants and the stale-lock path. Dev, build, dev-of-intermediate and native-target edges are green. Feature, implicit-feature, normal, rename, wasm-target and feature-through-intermediate edges are red, each with the detection message. The test comment's "`decode-worker` is the build `bin/ocelli.sh wasm-decode` names" matches `bin/ocelli.sh:544`.
- Pass-2 D3 is closed, including four Unicode spaces at either end. Only entries that pass every check are appended (`checked`), and the new test goes red when the branch is removed.
- Pass-2 D4: four of the five corrected sentences are true by execution. The fifth was rewritten into D3. The two new "level" sentences are D2.
- Pass-2 D5: the three LLD items are resolved as asked. Every other sentence in the rewritten LLD paragraph (the two ways, the two guards, `-e normal --all-features`, the spellings, dev-dependency green, feature edge red, the decode gap, the scan list) agrees with the matrices or with pass 2's carried receipts. The surrounding README and LLD text reads correctly around the edits, apart from D1's sentences.
- `use.html` and `more.html` do not mention the policy, and nothing in this diff describes them.
- Every required command is green, listed in Evidence. The suite is 498 tests, one more than pass 2's 497, which is the new trim test. No skip anywhere in the tracked TypeScript and JS tree.
- The tree is unchanged. `git write-tree` is `02bf9049...` before and after, the status is the same five modified paths plus the pass-2 report, and the `git diff 6260ee2` hash matches before and after. `target/f228-review3` was deleted at the end.

## Limitations and handoff

- Not exercised: Firefox and WebKit, the corpus-absent `NOT RUN` branch, a fresh `bin/ocelli.sh wasm` (the pkg's source identity is not pinned), AC11 to AC16, and a CDN-hosted glue. D3 is established from the glue's source and CSP Level 3, not from a CDN run.
- One mishap of my own, recorded so it is not mistaken for a finding: my first AC9 backup wrote both crate manifests to one file name, so the first matrix failed in `cargo metadata` with "two packages named `ocelli-cache`". Both manifests and the lock were re-copied from the real worktree, and every AC9 result above comes from the second run. The real tree was never touched.
- Next owner: the implementer fixes D1 to D3, then a fresh independent pass. The lead decides N11.
