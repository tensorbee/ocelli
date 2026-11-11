# F-228 review, pass 2

**Reviewed**: the staged index on `work/f-228-xai`, HEAD `4458d5fec5b2150ba06b5eb03cf8a192100e155e`, `git write-tree` `02bf90496c92ae2c36658692906b21b78cf6843a` before and after the pass. Nine staged paths: `.claude/plans/F-228-design.md` (the lead's amendments), `.claude/reviews/F-228-working-pass-1.md`, `README.md`, `docs/lld/typescript-packaging.md`, `eslint.config.js`, `examples/viewer-react/csp-probe.mjs`, `examples/viewer-react/src/csp-policy.ts`, `examples/viewer-react/src/csp-policy.test.ts`, `examples/viewer-react/src/node-test-modules.d.ts`. No unstaged tracked changes. Untracked `tools/oracle/out` ignored as instructed. Scope is the example-viewer half, AC1 to AC10, plus the lead amendments after pass 1. The folder-viewer half, AC11 to AC16, is not reviewed.
**Author / reviewer**: remediation by GLM-5.3 in opencode, per its report at the end of `glm-F-228-fix1.log`, after two attempts a watchdog killed. Plan amendments by the sprint lead. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed material.
**Result**: 5 defects, 0 smells, 7 nitpicks

## Leftovers from the killed attempts

None found. `git status --ignored` shows only the usual build outputs, `target/` holds no scratch directory from the implementer, no tracked file carries a duplicated section (`grep -c` of each new heading is 1), and no `TODO` or `FIXME` is in the staged paths. No shipped source changed: the staged names are the nine above, so `core-worker.ts`, `core-worker-client.ts` and every `packages/*/src` file are untouched, which agrees with the report.

## Acceptance challenged

Every mutation ran on copies under `target/f228-review2/`: a probe copy with absolute paths, and an export of the index (`git checkout-index`) for vitest and `cargo tree`. Nothing ran against the tracked files. The real `Cargo.lock` is unchanged.

- **Pass-1 D1, the cause of each failure.** Swap copy: `no-wasm` withdraws `connect-src`, `no-connect` withdraws `'wasm-unsafe-eval'`, the worker chunk in `worker-strict` gets `NO_CONNECT`, and `worker-fallback` drops `connect-src` instead of `worker-src`. Result: those four FAIL, 9 passed, each naming its own assertion ("the module bytes were fetched", "no glue or module request reached the route", "the worker response carried the header without 'wasm-unsafe-eval'", "worker-src absent falls back"). On the true removal, the unmodified probe passes 13 of 13. Adjacent case: the worker chunk given `script-src 'wasm-unsafe-eval'` without `'self'` turns `worker-strict` red (N8 on which assertion carries it). **Closed.**
- **Pass-1 D2, the strict case sees the worker.** `fetch("http://dicomweb.test/leak")` planted in the served worker chunk: `strict FAIL no securitypolicyviolation fires in the worker`. `console.error("planted")` planted separately: `strict FAIL no console message is logged in the worker`. **Closed.** Adjacent case: the same fetch behind `setTimeout(..., 1500)` leaves strict green (N7). Adjacent case: `console.warn("Content Security Policy planted")` in the worker turns strict red through the PAGE listener, which shows the worker's console API reaches `page.on("console")` (D4).
- **Pass-1 D3, the blob control's positive half.** Module version byte set to 2: `blob-control FAIL the same blob worker with the same module compiles when the policy allows it, got result: error: CompileError`. **Closed.**
- **Pass-1 D4, AC2 never skips.** `grep -rnE "\.skip\b|ctx\.skip|\.todo\(|skipIf|\.only\("` over `packages/*/src` and `examples/viewer-react/src` prints nothing, and vitest reports 497 passed with no skip. LLD quote removed: AC2 red. Policy grown by `base-uri 'none'`: AC2 red. The skip is closed. Adjacent cases that stay green are D1.
- **Pass-1 D5, the resolved graph.** Each with the copy's lock refreshed, then the guard run: inline table, dotted key, renamed dependency, `[dependencies.ocelli-codec]` subtable and a transitive edge through `ocelli-cache` all go red with the detection message, not a resolve error. The four spellings are closed. A `[dev-dependencies]` entry ALSO goes red, and an edge behind `decode-worker` stays green (D2).
- **Pass-1 D6 and S1.** `new Worker("data:text/javascript,1")` in `packages/core/src/pool.ts` red. A template-literal `` `data:...` `` in `packages/react/src/index.ts` red. `URL.createObjectURL` in `examples/viewer-react/src/core-worker-client.ts` red. A new `packages/extra/src/index.ts` with `new Function(` red. **Closed.**
- **Pass-1 D7.** `'unsafe-eval'` deleted from `FORBIDDEN_SOURCES`: "forbids exactly the sources the plan refuses" and "refuses a forbidden source" red. `data:` deleted: the same two red. `PLAN_FORBIDDEN` matches the plan's write-set line 141 character for character. **Closed.**
- **Pass-1 D8, the four comments.** `csp-policy.ts:5-7` now says the tests read two documents, true. `csp-policy.ts:7-8` no longer credits the probe's minimality cases, true. `csp-probe.mjs:12-15` replaced the "single named changes" sentence with one that is false in a new way (D4). `csp-probe.mjs:679` now names the worker's probe fetch of the glue, and the dumped worker violation is `connect-src .../crates/ocelli-wasm/pkg/ocelli_wasm.js`, which matches `core-worker.ts:130`. Three of four closed.
- **Pass-1 S2, the refusals.** Run through Node against the real module: `"https://a.example https://b.example"` refused (whitespace), `"'self'"` refused (quote), `"http://x/;"` refused (`;`), `"*"` refused (forbidden). Mutations: the `;` check removed, the whitespace check removed and the scheme-only check removed each turn one test red. Adjacent case: a leading or trailing whitespace character is accepted and copied into the header (D3).
- **Documents.** README line 178 equals `"Content-Security-Policy: " + policyHeader()`, README line 193 equals the same with `policyHeader(["https://pacs.example"])`, LLD line 283 equals the first. All three are true by string comparison in Node. The sentences around them are covered in D4 and D5.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | staged tree, existing `crates/ocelli-wasm/pkg` dated 2026-11-05 19:45, not rebuilt | now | 0, `core-worker-Cgmb5Hu4.js` |
| `npm run typecheck` | staged tree | now | 0 |
| `npm run lint` | staged tree | now | 0 |
| `npm run test` | staged tree | now | 0, 61 files, 497 passed (497), no skip |
| `vitest run csp-policy.test.ts --reporter=verbose` | staged tree | now | 11 passed, 0 skipped |
| `node examples/viewer-react/csp-probe.mjs` | Chromium 151.0.7922.34, Node 24.13.0, corpus present | now | 0, 13 passed, 0 failed, 0 not run |
| Eight probe mutations (swap, worker without `'self'`, planted fetch, planted error, planted warn, late fetch, broken module, console dump) | probe copies | now | as listed above |
| AC2, AC9, AC10, D7, S2 mutations | index export under `target/f228-review2/tree` | now | as listed above |
| `policyHeader` over sixteen inputs | real module through Node type stripping | now | see D3 and N9 |
| `bin/ocelli.sh gate packages` | staged tree | now | 0, 497 passed, ALL GREEN |
| `bin/ocelli.sh gate guards` | staged tree | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | staged tree | now | 0, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | staged tree | now | 0, 1102 refusals in 78 files, all claimed |
| `python3 scripts/prose_check.py` | staged tree | now | OK, 865 files |
| `python3 scripts/source_provenance_check.py` | staged tree | now | OK, 1711 files |
| `git write-tree`, `git status --short` | worktree | now, before and after | `02bf9049...` both times, same staged set |

## Defects

### D1, AC2 is a substring test, so a quote that drifts past the policy's end passes
**Where**: `examples/viewer-react/src/csp-policy.test.ts:120-137`, against `csp-policy.ts:5-7`, README "its test holds this section and `docs/lld/typescript-packaging.md` to the exact text", and the lead amendment "AC2 checks those two exactly".
**What**: the test asks `document.text.includes(policyHeader())`. The policy ends at `connect-src 'self'`, so any quote that extends it still contains it, and a policy that loses a trailing directive is still contained in the old quote.
**Why it is wrong**: plan AC2 rejects "A fourth copy in prose that drifts", and the module and the README both say a copy cannot drift.
**Evidence**: index export. README line 178 changed to `... connect-src 'self' 'unsafe-eval'` and LLD line 283 to `... connect-src 'self' https://evil.example`: 11 passed. The README's first block deleted, the DICOMweb block kept: 11 passed, because that block starts with the policy. `["connect-src", ["'self'"]]` deleted from `EMBEDDING_POLICY` with the documents untouched: AC2 green (only AC1 red).
**Scope and regression**: AC2, and the folder-viewer half when it adds the Use book. A document advertising a forbidden source next to the header passes every test. Compare each `Content-Security-Policy:` line in the two documents whole, against `policyHeader()` or a `policyHeader([...])` form, then replay the two extension plants and see red.

### D2, the AC9 guard counts a dev-dependency and misses the decode-worker edge
**Where**: `examples/viewer-react/src/csp-policy.test.ts:140-182`, against its comment at `:142-146`, the failure message at `:179` ("so the codec is linked into the module") and the LLD "must not name `ocelli-codec` by any spelling of the edge and through no intermediate crate".
**What**: `cargo tree` with no `-e` includes dev-dependency edges and resolves default features only. A `[dev-dependencies] ocelli-codec` entry links nothing into the module and turns the guard red. `ocelli-codec = { ..., optional = true }` with `decode-worker = ["dep:ocelli-codec"]` links the codec into the module `bin/ocelli.sh wasm-decode` builds (`--no-default-features --features decode-worker`, `bin/ocelli.sh:536-545`, run by `gate wasm`), and the guard stays green.
**Why it is wrong**: plan AC9 guards "A wasm decode landing under a policy nobody re-measured". `decode-worker` is the build named for decoding, so it is the likeliest place for that edge.
**Evidence**: index export, lock refreshed per plant. Dev-dependency: `Tests 1 failed`, with the detection message. Feature-gated edge: `Tests 1 passed`, while `cargo tree ... --no-default-features --features decode-worker -i ocelli-codec` prints `ocelli-codec v0.1.0 └── ocelli-wasm`. A candidate fix, `-e normal --all-features`, prints the edge for the feature plant and `warning: nothing to print.` for the dev plant. The guard would also need to treat that warning as clean.
**Scope and regression**: AC9. Resolve the normal edges under every feature set the shipped builds use, then replay both plants: feature edge red, dev-dependency green.

### D3, `policyHeader` checks the trimmed entry and appends the raw one
**Where**: `examples/viewer-react/src/csp-policy.ts:59-97`, against its docstring at `:52-56` ("refused when it carries whitespace") and the lead amendment ("any entry containing `;`, `,`, whitespace or a quote, is refused").
**What**: every check runs on `source.trim()`, and line 97 appends `extraConnect` untrimmed. An entry with leading or trailing whitespace passes, and the whitespace lands in the header.
**Evidence**: through Node, `policyHeader(["https://pacs.example\n"])` returns `"... connect-src 'self' https://pacs.example\n"`, and `[" https://pacs.example"]` and `["\thttps://pacs.example"]` are accepted with the whitespace kept.
**Why it is wrong**: the function exists to produce a header value, and a line break in a field value is not allowed (RFC 9110 section 5.5). The amended contract says such an entry is refused.
**Scope and regression**: today the only caller is the probe, which passes a constant. Refuse when `trimmed !== source`, or check `source` itself, and add a leading-space and a trailing-newline case to the structural test.

### D4, comments and printed details in the probe and the module that execution contradicts
**Where and what**:
- `csp-probe.mjs:17-22` says a dedicated worker's console text "never reach[es] the page" because "Chromium keeps the worker's console there", and `:263-267` says the collector records "a dedicated worker's CSP console text". Measured: the worker's console API reaches `page.on("console")`. The dumped `no-wasm` and `no-connect` page messages are `["error: CoreLoadFailed"]`, which is the worker's own `console.error` at `core-worker.ts:75`, and a `console.warn` planted in the worker turned strict red through the page listener. The browser's own CSP text from the worker ("Refused to ...") reaches neither the page listener nor the collector, whose `console` array holds only `CoreLoadFailed`. The collector's violation listener is what does the work. Its `console.error` wrapper sees nothing the page does not already see.
- `csp-probe.mjs:578-582`, "no console message is logged in the worker", is measured on `console.error` only.
- `csp-probe.mjs:13-15`, "the positive control varies them back". `BLOB_ALLOWED` (`:137`) keeps `blob:` and restores only `'wasm-unsafe-eval'`, so it varies one of the two back.
- `csp-probe.mjs:773`, page-strict prints "only the worker response carried 'wasm-unsafe-eval'". The case's header function (`:763-766`) withholds it from `/` and `/index.html` only, so the bundle, the glue and the module carried it too. This was present in the pass-1 tree and pass 1 missed it.
- `csp-policy.ts:50-51`, each `extraConnect` origin is appended to `connect-src` "for a DICOMweb service on another origin or a module hosted on a CDN". Plan item 1 says a CDN-hosted module goes in both `script-src` and `connect-src`, and under CSP Level 3 the glue's `import()` is a script request governed by `script-src`, so the header this function returns for a CDN refuses the glue. Also present in the pass-1 tree and missed.
**Why it is wrong**: a false sentence is a defect under section 3 of the review command, and the probe prints the page-strict detail as evidence.
**Scope and regression**: delete or correct each sentence. Drop the CDN clause, or have the function say it covers DICOMweb only.

### D5, LLD sentences that the tree contradicts
**Where**: `docs/lld/typescript-packaging.md`, the new section.
**What**:
- "whose test holds three copies of it to their sources: ... and the resolved dependency graph of `crates/ocelli-wasm`". The graph is not a copy of the policy. The test holds one policy against two documents, plus two unrelated guards.
- "which must not name `ocelli-codec` by any spelling of the edge and through no intermediate crate" and "When the codec becomes a dependency of the module, the test fails". False for the feature-gated edge, and the test also fails for a dev-dependency that is not a dependency of the module (D2).
- "The folder viewer's own policy is per page and hash based rather than this shared one: `default-src 'none'`, ...". `grep -rln "Content-Security-Policy" examples/folder-viewer crates` prints nothing, so the folder viewer sends no policy today. The closing sentence puts the work in the other half, but the paragraph says the policy exists now. S46's history shows halves committed on their own ("F-227 and F-228 library halves in progress"), so this text can land before the code it describes.
**Why it is wrong**: the LLD describes what exists and what proves it, and a false sentence there is a defect.
**Scope and regression**: rewrite the first sentence, let the second follow the D2 fix, and either mark the folder-viewer paragraph as the other half's pending work or move it into that half.

## Smells

None.

## Nitpicks

- N1, carried. `eslint.config.js`: `console`, `process` and `URL` are never used bare, because the probe writes `globalThis.*`.
- N2, carried and replayed. `new Function(` planted in `csp-policy.ts` passes AC10, because the whole file is exempt rather than its `FORBIDDEN_SOURCES` literal.
- N3, carried. `csp-probe.mjs:53-60` reports any import failure, a syntax error included, as a Node type-stripping problem.
- N6, carried. `node-test-modules.d.ts` also makes the `node:` modules resolve for the example's browser sources. Vite still refuses them at build.
- N7, new. `workerProbe` settles for 300 ms after the page settles. A refused fetch scheduled 1.5 s after load leaves strict green. `documents` and `dicomweb` drive the worker further under `STRICT` and never read the collector. A plausible concern, not reproduced against shipped code, which does nothing deferred in the worker today.
- N8, new. `worker-strict`'s `chunk[0].csp` and `documentResponse[0].csp` assertions compare the probe's own routing function with itself, so in the swap and the no-`'self'` plants the case went red on a tautology. The discriminating assertion is the module fetch, and reading the worker's `script-src wasm-eval` violation, as `no-wasm` does, would tie the case to its cause directly.
- N9, new. The forbidden-source match is case-sensitive while CSP scheme matching is ASCII case-insensitive, so `BLOB:https://x.example` is accepted, and so is `https://`. Neither is a valid source expression, so a browser ignores both and nothing widens today. `BLOB:` and `DATA:` alone are refused by the scheme-only rule.

Pass-1 N4 (`&apos;` and `&#x27;`) and N5 (`worker-src` documented as a pin) are closed.

## Verified clean

- Pass-1 D1, D2, D3, D6, D7 and S1 closed by replaying the original trigger, red now, and an adjacent case each.
- Pass-1 D4's skip, D5's four spellings plus the transitive edge, and S2's four named inputs closed. What stays open in those areas is new: D1, D2 and D3.
- Each failure case asserts its own cause, and passes on the true removal. `worker-fallback` asserts the fallback and says so, as the amendment asks.
- The probe still binds no port. It serves through `page.route` on invented origins, and the prelude is instrumentation on the worker chunk only, with the header unchanged.
- README and LLD policy quotes are byte-exact against `policyHeader()`, and the DICOMweb block is byte-exact against `policyHeader(["https://pacs.example"])`.
- README sentences on the every-response rule, the pin, `blob:` and `data:`, D5, the build commands and the routes agree with the probe's cases and pass 1's withheld-header receipts. `openDecodePool` is exported from `packages/core/src/index.ts:259`, and the only `new Worker` in shipped sources is the URL-constructed one at `core-worker-client.ts:51`.
- LLD sentences on `script-src` (the glue through `import()`, `core-worker.ts:134`), `connect-src` (`core-worker.ts:130`, and the module fetch in the route log), E1, E2, the tarball and "No gate was added" are true.
- `gate packages` runs `csp-policy.test.ts`: 497 passed there equals `npm run test`.
- The tree is unchanged: `git write-tree` `02bf9049...` before and after, the same staged set, no unstaged change. Scratch under `target/f228-review2/` was deleted.

## Limitations and handoff

- Not exercised: Firefox and WebKit, the corpus-absent `NOT RUN` branch of AC8, a fresh `bin/ocelli.sh wasm` (the existing pkg's source identity is not pinned), and anything in AC11 to AC16.
- The CDN half of D4 comes from CSP Level 3 and plan item 1, and was not measured with a CDN-hosted glue.
- One harness mishap of my own, recorded so it is not mistaken for a finding: my restore script first treated the tracked `vendor/*/Cargo.toml.orig` files as backups and overwrote the copy's `vendor/*/Cargo.toml`, which made `cargo tree` fail in the copy. The copy's `vendor/` was re-exported from the index and every cargo result above comes from after that. The real tree was never touched.
- Next owner: the implementer remediates D1 to D5, then a fresh independent pass reviews the changed material. The lead decides whether the LLD's folder-viewer paragraph belongs to this half.
