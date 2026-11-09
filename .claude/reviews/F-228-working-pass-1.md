# F-228 review, pass 1

**Reviewed**: the staged index on `work/f-228-xai`, HEAD `4458d5fec5b2150ba06b5eb03cf8a192100e155e`, `git write-tree` `d2133aaa64ed36c69bbc29af83aaff96364ae672` before and after the pass. Five staged paths: `eslint.config.js`, `examples/viewer-react/csp-probe.mjs`, `examples/viewer-react/src/csp-policy.ts`, `examples/viewer-react/src/csp-policy.test.ts`, `examples/viewer-react/src/node-test-modules.d.ts`. No unstaged tracked changes. Untracked `tools/oracle/out` ignored as instructed. Scope is the example-viewer half, AC1 to AC10. The folder-viewer half, AC11 to AC16, is not reviewed.
**Author / reviewer**: implementation by GLM-5.3 in opencode, per its report. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed material and none of the design plan.
**Result**: 8 defects, 2 smells, 6 nitpicks

## Acceptance challenged

Every mutation below ran on copies under `target/f228-review/` (a probe copy with absolute paths, a policy copy, and a source-tree copy for the vitest file), never on the tracked files.

- **AC3, AC4, priority 1.** Each of the four directives was removed from a copy of `EMBEDDING_POLICY` and the probe rerun. Removing `'wasm-unsafe-eval'` or `worker-src` stops the probe before any case runs, through its own derivation helpers ("could not remove", "could not find"). Removing `default-src` leaves the probe 12 of 12 green, and only AC1's unit test goes red. With a header override that keeps the derivations intact, the strict case was rerun under each removal: without `worker-src` it passes (the directive falls back to `script-src 'self'`), without `default-src` it passes, without `'wasm-unsafe-eval'` or `connect-src` it fails with "the strict header must not stop the worker and wasm path", and without `script-src` it times out. The policy is sufficient and grants nothing the path does not use. It is not directive-minimal, because `worker-src 'self'` adds no permission over the fallback (N5).
- **Named failure.** The probe header was swapped so that `no-wasm` and `worker-strict` withdraw `connect-src` instead of `'wasm-unsafe-eval'`. Both still pass (D1).
- **Every response.** The header was withheld from the worker chunk only: `no-wasm`, `no-connect` and `worker-strict` go red. It was withheld from the document only: `no-worker`, `no-script` and `dicomweb-unlisted` go red. A response the probe forgot to cover is caught for the two responses whose policy matters.
- **AC3 strict against control.** Tier `C` and pixel `255 255 255 255` in both. The pixel discriminates, since `no-wasm` reads `0 0 0 0` from the same 1 by 1 canvas. A fetch planted in the served worker chunk and refused under strict leaves strict reporting "console clean" (D2).
- **AC5.** The route log shows that `worker-strict` fetches and fulfils `ocelli_wasm_bg-*.wasm` before failing, so today's failure is the compile, as claimed. The probe does not assert this (D1). `page-strict` reaches `core ready` and discriminates the inheritance model.
- **AC6.** With the empty module's version byte changed to 2, the relaxed half still reports `CompileError` and passes (D3).
- **AC8.** The `connect-src` violation in `dicomweb-unlisted` is asserted and seen. Corpus present, both cases ran. The corpus-absent branch was not exercised.
- **AC2 ratchet, priority 3.** In a tree copy, all three documents quoting the policy pass, one dropped fails, and then a policy that grows `base-uri 'none'` with the documents untouched skips (D4).
- **AC9.** Three valid Cargo spellings of the edge stay green (D5).
- **AC10, priority 2.** Plants in a tree copy: `new Function(` in `packages/react/src/index.ts` red, `URL.createObjectURL` in `packages/core/src/pool.ts` red, `new Function(` in the glue red. `new Worker("data:text/javascript,1")` in `pool.ts` green (D6). `new Function(` in a new `packages/extra/src/index.ts` green (S1). `new Function(` in `csp-policy.ts` green (N2).
- **AC1.** Adding `'unsafe-eval'` to `script-src` turns three tests red. Deleting `'unsafe-eval'` from `FORBIDDEN_SOURCES` leaves all seven green (D7).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | staged tree, existing `crates/ocelli-wasm/pkg` dated 2026-11-05 19:45, not rebuilt | now | 0, `dist/assets/core-worker-Cgmb5Hu4.js` |
| `npm run typecheck` | staged tree | now | 0 |
| `npm run lint` | staged tree | now | 0 |
| `npm run test` | staged tree | now | 0, 61 files, 493 passed, 1 skipped (494) |
| `vitest run csp-policy.test.ts --reporter=verbose` | staged tree | now | 7 passed, 1 skipped (AC2) |
| `node examples/viewer-react/csp-probe.mjs` | Chromium 151.0.7922.34, Node 24.13.0 | now | 0, 12 passed, 0 failed, 0 not run |
| `lsof -iTCP -sTCP:LISTEN` sampled during a probe run | host | now | no new listening socket |
| Policy-removal and header-override probe runs | copies under `target/f228-review` | now | as listed under Acceptance |
| Probe with a per-case route log and worker console listener | copy | now | worker CSP refusals surface only as `CoreLoadFailed` |
| `bin/ocelli.sh gate packages` | staged tree | now | 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | staged tree | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | staged tree | now | 0, 0 watched by nothing. `examples/` is outside `SCAN_ROOTS` |
| `python3 scripts/guard_census.py --check-runbook` | staged tree | now | 0, 1102 refusals in 78 files, all claimed |
| `python3 scripts/prose_check.py` | staged tree | now | OK, 864 files |
| `python3 scripts/source_provenance_check.py` | staged tree | now | OK, 1710 files |
| ESLint API over the probe with the new block varied | `eslint.config.js` loaded, block replaced in memory | now | clean with only `document` and `window` |
| `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown -i ocelli-codec` | workspace | now | no path, so no transitive edge today |

## Defects

### D1, three minimality cases assert a generic failure, and pass for the wrong cause
**Where**: `examples/viewer-react/csp-probe.mjs:478-524` (`no-wasm`, `no-connect`) and `:544-555` (`worker-strict`), against the claim at `:6-8`.
**What**: each asserts only `core not built`, plus `Canvas not published` for two of them. Every worker-side failure produces exactly that state and the single console line `error: CoreLoadFailed`, so the three cases cannot tell a refused compile from a refused fetch or any other load failure.
**Why it is wrong**: plan AC4 says "Each case's named failure is asserted, not only the absence of success" and names "A minimality claim that passes because the page broke for another reason" as the wrong implementation it rejects. The file's own header repeats the claim.
**Evidence**: in a probe copy, `no-wasm` used `NO_CONNECT` and `worker-strict` gave the worker chunk `NO_CONNECT`. Result: "no-wasm PASS", "worker-strict PASS", 12 passed. A discriminator exists: a per-case log of routed paths shows `no-wasm` and `worker-strict` requesting `ocelli_wasm_bg-Cux6QOe9.wasm`, while `no-connect` requests nothing after `core-worker-Cgmb5Hu4.js`.
**Scope and regression**: AC4 and AC5 evidence. Record the routed paths per case, assert that the module bytes were fulfilled in `no-wasm` and `worker-strict` and that no request after the worker chunk reached the route in `no-connect`, then rerun the swapped-header copy and see both cases red.

### D2, the strict case's console and violation checks cannot see the worker
**Where**: `examples/viewer-react/csp-probe.mjs:307-315` and `:462-474`.
**What**: the `securitypolicyviolation` listener goes in through `page.addInitScript`, which does not run in a dedicated worker, and Chromium's CSP console text from a worker never reaches `page.on("console")`. Only document-context refusals are observed. The worker is where `'wasm-unsafe-eval'`, the glue import and both fetches are enforced.
**Why it is wrong**: AC3 requires that "no console message contains 'Content Security Policy'", and the probe reports "console clean" as that evidence.
**Evidence**: a probe copy prepended `fetch("http://dicomweb.test/leak").catch(() => {})` to the served worker chunk. The route log shows `leak` requested under control and absent under strict, so strict refused it, and strict still printed "PASS tier C, pixel 255 255 255 255, console clean". In the unmodified run, the strict and control route logs are identical, which is indirect evidence that nothing is refused in the worker today.
**Scope and regression**: AC3. A tolerated worker-side refusal, such as an optional fetch in a later worker story, would pass unseen. Smallest regression: assert that strict's set of routed requests equals control's, and rerun the planted fetch to see red. Attaching a CDP session to worker targets for `Log.entryAdded` is the fuller alternative, not tried here.

### D3, the blob control's CompileError half passes with an invalid module
**Where**: `examples/viewer-react/csp-probe.mjs:190-227` and `:590-597`.
**What**: the relaxed half accepts any `CompileError`, there is no positive control, and the worker posts `error.name` only. A compile failure for any reason passes.
**Why it is wrong**: AC6 exists to show that the blob worker inherits the page policy. Without a run where the same worker compiles under `'wasm-unsafe-eval'`, nothing attributes the error to the policy.
**Evidence**: in a copy, the module bytes changed from version 1 to version 2. Result: "blob-control PASS ... relaxed page: result: error: CompileError".
**Scope and regression**: AC6. Add the positive control (`worker-src 'self' blob:` with `'wasm-unsafe-eval'` kept, expect `compiled`), or post `error.message` and assert that it names the policy, then replay the version-byte plant and see red.

### D4, the AC2 skip also covers the drift it guards against, and nothing ever refuses it
**Where**: `examples/viewer-react/src/csp-policy.test.ts:92-101`.
**What**: the test skips whenever all three documents lack the current `policyHeader()`. That is both the state before the edits land and the state after the policy text changes with no document updated.
**Why it is wrong**: plan AC2 rejects "A fourth copy in prose that drifts". The implementer's report says "a partial edit or a drift is a red test rather than a silent skip", and that is false for a policy change. This is the first `ctx.skip` in the TypeScript tree, and no gate or script reads vitest's skip count, so the skip can stay a skip indefinitely. On honesty: the plan's Approach step 1 and write set put the three document edits in this half. The skip reason states the cause truthfully, but the check belongs to this half, and an unevidenced row should read red rather than skipped.
**Evidence**: tree copy. All three documents quoting the policy: "1 passed". `more.html` quote removed: "1 failed". Quote restored and `["base-uri", ["'none'"]]` added to `EMBEDDING_POLICY`: the AC2 test skipped.
**Scope and regression**: AC2 and the completion gate. Remove the skip so the row is red until the edits land, or tie the skip to an explicit pending constant that drift cannot satisfy. Once the documents land, rerun the `base-uri` mutation and see red.

### D5, the AC9 decode guard misses three valid spellings of the edge
**Where**: `examples/viewer-react/src/csp-policy.test.ts:105-125`.
**What**: the line regex `^\s*ocelli-codec\s*=` inside a table ending in `dependencies` misses a dotted key, a renamed dependency and a dependency subtable.
**Why it is wrong**: plan AC9 says the test fails "when any dependency table names `ocelli-codec`".
**Evidence**: tree copy, each added under `[dependencies]` or as a table. `ocelli-codec = { path = ... }` red. `ocelli-codec.path = "../ocelli-codec"` green. `codec = { package = "ocelli-codec", path = ... }` green. `[dependencies.ocelli-codec]` with `path = ...` green. A concern, not reproduced: a transitive edge, for example `ocelli-viewport` gaining `ocelli-dicom`, which depends on `ocelli-codec`, would link the codec into the module with the guard still green. `cargo tree` shows no such path today.
**Scope and regression**: AC9. Flag any non-comment line of the manifest that names `ocelli-codec`, or ask `cargo metadata`, then replay the three plants and see red.

### D6, the AC10 scan claims data workers and has no data needle
**Where**: `examples/viewer-react/src/csp-policy.test.ts:128` and `:135`.
**What**: the test is titled "no shipped source constructs a blob or data worker or evaluates strings", and `BANNED` has no `data:` entry.
**Why it is wrong**: AC10's trigger is "No source Ocelli ships constructs a blob or data worker", and plan item 3 forbids both schemes. The plan's own needle list omits `data:`, so the plan contradicts its trigger, and the lead should correct one or the other.
**Evidence**: tree copy, `export const w = () => new Worker("data:text/javascript,1");` appended to `packages/core/src/pool.ts`: 7 passed. A bare `data:` needle would false-positive on `packages/core/src/dcmjs.d.ts:17` (`data: {`), so it needs a quoted-prefix pattern.
**Scope and regression**: AC10. Add a quoted `data:` pattern and replay the plant.

### D7, the AC1 refusal test is derived from the module under test
**Where**: `examples/viewer-react/src/csp-policy.test.ts:55-75`, against its own comment at `:15-17`.
**What**: "contains no forbidden source" and "refuses a forbidden source" iterate the imported `FORBIDDEN_SOURCES`, so shrinking the list shrinks the test with it.
**Why it is wrong**: the file says "Expected values are written from the design plan, never from the module's output", and CLAUDE.md requires tests to come from the specification. The plan spells out the seven sources.
**Evidence**: tree copy, `"'unsafe-eval'"` deleted from `FORBIDDEN_SOURCES`: 7 passed, 1 skipped. `policyHeader(["'unsafe-eval'"])` would then be accepted.
**Scope and regression**: AC1. Spell the plan's seven sources in the test and replay the deletion.

### D8, prose claims that execution contradicts
**Where and what**:
- `csp-policy.ts:5-7`, "The tests and the probe read the documents that quote the policy". The probe reads no document. `grep -n "README\|use.html\|more.html" examples/viewer-react/csp-probe.mjs` prints nothing.
- `csp-policy.ts:8-9`, "kept by the probe's minimality cases". With `default-src` removed the probe is 12 of 12 green, and AC1's unit test is what keeps it. With `worker-src` removed, only a derivation helper refuses, not a case.
- `csp-probe.mjs:12-13`, "The variation headers are single named changes". `RELAXED_BLOB` is two changes, `blob:` added and `'wasm-unsafe-eval'` removed.
- `csp-probe.mjs:516`, "connect-src 'none' refuses the glue's fetch of the module". The route log shows the refusal stops at the worker's own probe fetch of `ocelli_wasm.js`, before the glue loads.
**Why it is wrong**: a false sentence is a defect under section 3 of the review command.
**Scope and regression**: deleting or correcting each sentence is enough.

## Smells

### S1, the AC10 directory list is chosen by hand
**Where**: `csp-policy.test.ts:137-141`.
**Mechanism**: a new workspace package's sources are never scanned. In a tree copy, `new Function("return 1")` in a new `packages/extra/src/index.ts` passed. Today the list equals `packages/*/src` plus the one TypeScript example, so the gap is latent. Derive it from `packages/*/src` instead, or from the root `workspaces`.

### S2, `policyHeader` leaves gaps in its refusal policy
**Where**: `csp-policy.ts:12-16` and `:41-60`.
**Mechanism**: run through Node, `policyHeader(["https:"])` and `["http:"]` are accepted. Those scheme sources allow any origin, which contradicts "so an embedding page lists explicit origins". `["https://pacs.example; img-src https://evil.example"]` returns a header with a new `img-src` directive, which contradicts "Each origin in `extraConnect` is appended to `connect-src`" and AC1's "connect-src only". A trailing `,` starts a second policy, and `'none'` is accepted. Today the only caller is the probe, with a constant. Refuse `;`, `,`, whitespace inside one entry, bare scheme sources and quoted keywords, and test each one.

## Nitpicks

- N1. `eslint.config.js`: `console`, `process` and `URL` are never referenced bare, because the probe writes `globalThis.*`. The ESLint API run with only `document` and `window` is clean. The block is scoped to the one file and widens nothing else. It mirrors the sibling blocks.
- N2. AC10 exempts the whole of `csp-policy.ts`, so a planted `new Function(` there passes. Exempting only the `FORBIDDEN_SOURCES` literal would be narrower.
- N3. `csp-probe.mjs:44-51` reports any import failure, a syntax error included, as a Node type-stripping problem.
- N4. `decodeEntities` handles `&#39;` but not `&apos;` or `&#x27;`, which matters once the pages quote the policy.
- N5. A plan-level note for the lead. `worker-src 'self'` adds no permission over its fallback to `script-src 'self'` (strict, documents and DICOMweb all pass without it), so the documentation should justify it as an explicit pin that a later `script-src` addition cannot widen, not as a requirement.
- N6. `node-test-modules.d.ts` makes `node:fs` resolve for the example's shipped browser sources too, so such an import would typecheck. Vite would still refuse it at build. Same pattern as `packages/core`. It hides no type error in the test: the declared `readdirSync` overload matches Node's real return type.

## Verified clean

- The probe binds no port and starts no server. No `listen`, `createServer` or `PORT` appears in the source, a sampled `lsof` during a run showed no new listener, and the invented origins mean an unrouted request could not succeed at all.
- The policy is applied to every routed response, 404s and DICOMweb replies included (`serveWith`). Withholding it from the worker chunk or the document turns named cases red.
- `page-strict` and `worker-strict` together measure E1's response model in Chromium 151, and the run prints the browser version.
- `no-worker` and `no-script` assert their specific violation and their specific state.
- The `/@fs` mapping refuses paths outside the repository root, sibling-prefix directories included, and `.wasm` is served as `application/wasm`.
- The AC10 scan goes red for `new Function(` and `createObjectURL` in the covered directories and in the glue. The positive control exercises the scanner.
- AC1's exact-string test goes red when `'unsafe-eval'` is added to `script-src`.
- AC7: under strict, `Pixels 0, 1, 2, 3` and `TID 1500 12.5` equal the control's.
- No `unsafe`, no `wasm-bindgen`, no dependency change, no pixel arithmetic and nothing on the boundary. The probe reads a canvas pixel in the harness only.
- Tier: C resolved and recorded. A and B not claimed, as the plan says.
- Tree unchanged: `git write-tree` `d2133aaa...` before and after, staged set identical.

## Limitations and handoff

- Not exercised: Firefox and WebKit, the corpus-absent `NOT RUN` branch of AC8, a fresh `bin/ocelli.sh wasm` (the existing pkg's source identity is not pinned), and anything in AC11 to AC16.
- `guard_census.py` does not scan `examples/`, so the probe's refusals are outside the census by its scope, not by this diff.
- The three document edits (AC2), the LLD section and the backlog row remain open. The implementer's report says its run scope forbade the documents.
- Next owner: the implementer remediates D1 to D8, S1 and S2, then a fresh independent pass reviews the changed material. The lead decides D6's plan wording and N5.
- Scratch copies under `target/f228-review/` were deleted at the end of the pass.
