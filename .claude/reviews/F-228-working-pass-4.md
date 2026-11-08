# F-228 review, pass 4

**Reviewed**: the uncommitted working tree on `work/f-228-xai` against the pass-3 checkpoint `fb8d5ee`, five tracked paths: `README.md`, `docs/lld/typescript-packaging.md`, `examples/viewer-react/csp-probe.mjs`, `examples/viewer-react/src/csp-policy.ts`, `examples/viewer-react/src/csp-policy.test.ts`. `git diff fb8d5ee` is 161 insertions and 52 deletions, sha256 `c2cfac578d22cdec...`. Untracked `.claude/reviews/F-228-working-pass-2.md` and `-pass-3.md` read as context, `tools/oracle/out` ignored as instructed. `git write-tree` (the index) is `d0d556f8481dc71a19b81d695277076a8e181df4` before and after. Scope is the example-viewer half, AC1 to AC10. AC11 to AC16 are not reviewed.
**Author / reviewer**: remediation by GLM-5.3 in opencode, per its report at the end of `glm-F-228-fix3.log`. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed material.
**Result**: 3 defects, 1 smell, 14 nitpicks

## Acceptance challenged

Every mutation ran on a copy under `target/f228-review4/tree` (`git ls-files -co --exclude-standard` through rsync, `node_modules` and `crates/ocelli-wasm/pkg` symlinked), or on a probe copy at `target/f228-review4/probe-abs.mjs` whose `HERE`, policy import and Playwright require point at the real worktree. Each AC2 plant was appended to the end of the document copy, the test run, and the copy restored from a pristine backup, which was diffed against the real file afterwards. Nothing ran against a tracked file.

- **Pass-3 D1, AC2 reads the header in the spellings a reader recognises.** Each plant below red on README and on the LLD, with its own finding: a lower-case `content-security-policy: default-src *` line, an nginx `add_header Content-Security-Policy "...'unsafe-eval'" always;` line, a JS `headers: { "Content-Security-Policy": "script-src 'self' 'unsafe-eval'" }` line, a quoted `"Content-Security-Policy: <policy> 'unsafe-eval'"` string, a prose line naming the header with `'unsafe-eval'`, a `<meta http-equiv>` line carrying `'unsafe-eval'`, a `Content-Security-Policy-Report-Only: default-src *` line and a `Content-Security-Policy: img-src *` line. A prose line that only names the header stays green. **The four spellings pass 3 named are closed.** The new matcher has three failures of its own: deleting every quote is green (D1), a spelling that carries content outside the capture or outside the token list is green while the prose says it cannot be (D2), and the correct policy in a quoted string is refused (D3).
- **Over-broad matching, asked directly.** The current README and LLD are green: the only lines naming the header are `README.md:178`, `:193` and `:216`, and the LLD's `:283` and `:314`. No other README code sample names it. So nothing red today. Correct or legitimate content that goes red: the exact policy in a quoted string (`const header = "Content-Security-Policy: <policy>";`) and in `curl -H "..."`, the exact policy in a `<meta http-equiv>` tag, an Apache `Header set` line, a lower-case header line and an indented header line, the exact policy in the Use book's house code-block style `<pre><code>Content-Security-Policy: <policy></code></pre>`, and prose such as "A `Content-Security-Policy` without `worker-src` falls back to `script-src`". The last two are S1, because the plan requires the Use book and More page to quote the policy.
- **Pass-3 D2, strict asserts the full console set.** In the probe copy, each of `console.assert(false, ...)`, `console.trace`, `console.table`, `console.count`, `console.dir`, `console.group` with `groupEnd`, and `console.timeEnd` of an unknown timer was planted in the worker prelude. Each turned strict red twice: once on the page-message assertion, and again with that assertion disabled, on the worker-collector assertion. The dumps show both channels carried every plant (for example `page ["startGroup: planted group","endGroup: console.groupEnd"]` beside `console ["group: planted group","groupEnd: "]`). In Chromium 151 the prelude wraps all 24 own function properties of the worker's `console`. Without a plant, both sets are `[]`. **Closed.**
- **Pass-3 D3, the CDN sentence.** `csp-policy.ts:52-53` now says the worker's `import()` of the glue is the script request. `core-worker.ts:134-137` imports `__OCELLI_WASM_URL__` (the glue), and the glue fetches `ocelli_wasm_bg.wasm` through `new URL` and `fetch` (`ocelli_wasm.js:2894-2899`), which is `connect-src`. Under CSP Level 3, `import()` is a script-destination request, so `script-src` governs it. **Closed.**
- **N10, the control-character refusal.** `/\p{Cc}/u` is true for U+0000, U+0007, U+007F, U+0085 and U+009F and false for U+200B, and `/\s/` is false for all six, so the branch is the one that refuses them. Mutation: the branch disabled, "refuses a control character inside an entry" goes red, 1 failed and 12 passed. **Closed**, with N12 and N14 below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | working tree, existing `crates/ocelli-wasm/pkg` not rebuilt | now | 0, `core-worker-Cgmb5Hu4.js`, built in 970ms |
| `npm run typecheck` | working tree | now | 0 |
| `npm run lint` | working tree | now | 0 |
| `npm run test` | working tree | now | 0, 61 files, 499 passed (499), no skip and no todo in the log |
| skip grep over every tracked `.ts`, `.tsx`, `.mts`, `.mjs` | working tree | now | only four conditional `t.skip` in `tools/oracle/tests/args_test.mjs`, outside this diff and outside `npm run test` (N17) |
| `node examples/viewer-react/csp-probe.mjs` | Chromium 151.0.7922.34, Node 24, corpus present | now | 0, 13 passed, 0 failed, 0 not run |
| probe copy, 7 worker plants by 2 variants, strict only | probe copy | now | 14 of 14 red, see Acceptance |
| AC2 matrix, 30 plants on README and 29 on the LLD | scratch copy | now | see Acceptance, D1 to D3 and S1 |
| AC2 deletion matrix, 5 cases | scratch copy | now | all green, see D1 |
| `EMBEDDING_POLICY` grown by `base-uri 'none'` | scratch copy | now | 3 red, including AC2 |
| control-character branch disabled | scratch copy | now | 1 red |
| `bin/ocelli.sh gate packages` | working tree | now | 0, 499 passed, ALL GREEN |
| `bin/ocelli.sh gate guards` | working tree | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | working tree | now | 0, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | working tree | now | 0 |
| `python3 scripts/prose_check.py` | working tree | now | OK, 865 files |
| `python3 scripts/source_provenance_check.py` | working tree | now | OK, 1711 files |
| `git write-tree`, `git status --short`, `git diff fb8d5ee \| shasum -a 256` | worktree | now, before the report and after | unchanged, see Verified clean |

## Defects

### D1, AC2 no longer fails when a document stops quoting the policy
**Where**: `examples/viewer-react/src/csp-policy.test.ts:214-271`, the removed count loop (`fb8d5ee`'s `csp-policy.test.ts:161-170`), and the comment at `:224` ("No count of lines is kept").
**What**: the test only inspects lines that name the header, so a document with no such line has nothing to fail on. In the copy, each of these leaves the whole file green, 13 of 13: README's base quote at `:178` deleted, README's DICOMweb quote at `:193` deleted, the LLD's quote at `:283` deleted, both README blocks deleted, and a second identical quote added to the LLD.
**Why it is wrong**: plan AC2 (`.claude/plans/F-228-design.md:183`) requires README's fenced block to contain `policyHeader()`, and the lead amendment (`:291`) says AC2 "checks those two exactly, red or green". The test is still titled "every document quotes the one policy". This was closed before: pass 1 recorded "LLD quote removed: AC2 red", and pass 2 D1 listed "The README's first block deleted" as part of that defect's trigger. Pass 3 asked for the counts to be described as the test's own, not removed. The assertion message's old "neither drift nor vanish" became "neither drift nor hide", which drops that property without a word.
**Evidence**: the five-case deletion matrix above, each `Tests 13 passed (13)`. The implementer's demo replayed extension and trailing space as pass-2 regressions but not deletion.
**Scope and regression**: AC2 for both documents now, and for the Use book and More page when the folder-viewer half adds them. Smallest fix: restore a per-document requirement that each expected header appears exactly once as a whole line, and say in the comment that the test holds those counts. Replay the five deletion cases and see red.

### D2, README and the LLD say the check holds every setting line whole and keeps every directive and source out of prose, and it does neither
**Where**: `README.md:214-221`, `docs/lld/typescript-packaging.md:313-320`, the test comment at `csp-policy.test.ts:215-224`, the `POLICY_TOKENS` docstring at `:153-157`, and the assertion message at `:269`. The matcher is `policyValueSetBy` at `:182-211` and `POLICY_TOKENS` at `:158-172`.
**What**: three sentences are false by execution.
- "A line that sets the header in one of those must equal an expected header exactly, the whole line." For the nginx, object-key and setter shapes only the first quoted capture is compared, so anything after it is unchecked. Green on both documents: `headers: { "Content-Security-Policy": "<policy>" + " 'unsafe-eval'" }`, `res.setHeader("Content-Security-Policy", "<policy>" + " https://evil.example");`, and `add_header Content-Security-Policy "<policy>" always; add_header Content-Security-Policy "<policy> https://evil.example" always;`. The setter line is an example where the drift widens `connect-src`.
- "A line that only names the header in prose may carry no directive or source." Only the 13 strings in `POLICY_TOKENS` are checked. Green on both documents: "The `Content-Security-Policy` header may also allow https://evil.example.", "Set `Content-Security-Policy` with `style-src *` too.", and "Older Chromium wants `'wasm-eval'` in `Content-Security-Policy`." Spellings the matcher does not read fall into the same branch, so `headers: { 'Content-Security-Policy': 'img-src *' }` and Apache's `Header set Content-Security-Policy "img-src *"` are green too.
- "however the line spells it", followed by four shapes. Apache, Caddy, `<meta http-equiv>` and single-quoted values are not read as setting the header (see S1 and D3 for where they land). The `POLICY_TOKENS` docstring says it holds "the source expressions the policy and the forbidden list name", but the forbidden list's `*` is not among them, which is why the `style-src *` plant is green.
**Why it is wrong**: a false sentence in README and the LLD is a defect under section 3 of the review command. Plan AC2 rejects "A fourth copy in prose that drifts", and a header widened by concatenation is such a copy.
**Evidence**: the plant matrix, rows R6 to R13 and R15, green on both documents. A multi-line `res.setHeader(` call carrying `'unsafe-eval'` is green as well, which no per-line reading can see. That is a limit to state, not a promise to keep.
**Scope and regression**: README, LLD, the test comment, the docstring and the assertion message. Smallest fix: describe what the matcher does (the shapes it reads, a value compared up to its closing quote, and the named tokens) and delete "however the line spells it" and "no directive or source". If the stronger property is wanted, require a line that names the header to be either an exact expected line or free of every directive name and every `'...'`, scheme and host source. Then replay R6 to R13.

### D3, the quoted-string shape refuses the exact policy text and reports it as a different policy
**Where**: `csp-policy.test.ts:202-209`, the regex `(["'])content-security-policy[ \t]*:(.*?)\1`, and the finding at `:251-254`.
**What**: the capture starts straight after the colon, so it keeps the space a writer puts there. `allowed.includes(value)` then compares `" default-src 'none'; ..."` with `"default-src 'none'; ..."`. `const header = "Content-Security-Policy: <policy>";` and `curl -H "Content-Security-Policy: <policy>" https://app.example` are each refused on both documents with "a header set to something other than the policy text", which is false. Only a quoted string with no space after the colon, which nobody writes, can pass.
**Why it is wrong**: the README and LLD sentences say a line that sets the header in a quoted string "must equal an expected header exactly", and this line does. A check whose finding misreports a correct copy will be "fixed" by editing the document away from the policy, or by weakening the check.
**Evidence**: the G4 and G5 plants, with the test output line `README.md:458: a header set to something other than the policy text: const header = "Content-Security-Policy: default-src 'none'; ...";`.
**Scope and regression**: the quoted shape in both documents, and any future curl or fetch sample. Smallest fix: compare the whole quoted string with `Content-Security-Policy: ${policy}`, or trim exactly one leading space, then replay G4 and G5 green and R4 red.

## Smells

### S1, the planned Use book and More page quotes cannot pass this matcher
**Where**: `HEADER_LINE` at `csp-policy.test.ts:151`, the prose branch at `:259-265`, and the comment at `:224-225` ("The Use book quote lands with the folder-viewer half, which adds it here").
**What**: the plan requires `use.html`'s 5.1 paragraph and `more.html`'s `#worker` section to quote `policyHeader()` (`F-228-design.md:183`). The house style for a code block in `use.html` opens `<pre><code>` on the same line as the first line of code (`use.html:255`, `:259`, `:263`, `:269`). Planted in that style, `<pre><code>Content-Security-Policy: <policy></code></pre>` is refused as "a prose mention of the header carrying policy text (default-src, script-src, worker-src, connect-src, 'none', 'self', 'wasm-unsafe-eval')". A 5.1 sentence in that page's style, "A DICOMweb origin is added to `connect-src` of the `Content-Security-Policy` header", is refused too. So the folder-viewer half cannot add the documents the plan names without rewriting this matcher, and the easiest way to get green is to stop naming the header next to the policy.
**Mechanism**: the only shape compared as a whole line is a header at the start of a line, and HTML puts tags first. Every other line naming the header falls into the token branch.
**Smallest fix**: when D1 restores the counts, match an expected header as a whole line after stripping a leading `<pre><code>` and trailing `</code></pre>`, or decide now how the HTML pages will be read and write it in the comment.

## Nitpicks

- N1, carried. `eslint.config.js`: `console`, `process` and `URL` are never used bare.
- N2, carried. `new Function(` in `csp-policy.ts` passes AC10, because the whole file is exempt.
- N3, carried. `csp-probe.mjs:58-64` reports any import failure as a Node type-stripping problem.
- N6, carried. `node-test-modules.d.ts` also resolves `node:` modules for the example's browser sources.
- N7, carried. `workerProbe` settles 300 ms, so a refusal deferred past that leaves strict green.
- N8, carried. `worker-strict`'s header assertions compare the probe's routing function with itself.
- N9, carried. `BLOB:https://x.example` and `https://` are accepted. Neither is a valid source expression, so nothing widens.
- N11, carried, for the lead. `.claude/plans/F-228-design.md:269` still describes the LLD's folder-viewer paragraph, which pass 3 recorded as deleted.
- N12, new. `policyHeader`'s docstring at `csp-policy.ts:54-60` lists the refusals and leaves out the new control-character one, as it already left out the empty entry.
- N13, new. The probe's console wrapper (`csp-probe.mjs:290-294`) discards return values, so `console.createTask()` and `console.context()` return `undefined` inside the worker. Nothing in `dist/assets` calls either today (grep).
- N14, new. The control-character refusal prints the entry raw (`${source}`), so a BEL or NUL is invisible in the message. The trim refusal uses `JSON.stringify(source)`.
- N15, new. The two strict console assertion messages are phrased as the failure ("... is not empty under the strict header"), while every neighbour states the expectation ("the worker canvas pixel equals the control").
- N16, new. The prose says the header is read "case-insensitively", yet `content-security-policy: <policy>`, the exact policy in lower case, is refused, because the comparison is with the canonical spelling. That is consistent with "exactly", but the two words read together suggest otherwise.
- N17, new and outside this diff. Pass 3's Evidence says its skip grep over every tracked `.ts`, `.tsx`, `.mts` and `.mjs` printed nothing. The same pattern matches four conditional `t.skip` calls in `tools/oracle/tests/args_test.mjs`. They are not in `npm run test`, so the 499 count has no skip.

Pass-3 N10 is closed by the new refusal. Pass-2 N4 and N5 stay closed.

## Verified clean

- Pass-3 D1's four named spellings, plus prose with a listed token, the meta tag, Report-Only and an off-policy header line, are red on both documents. A prose mention with no policy content is green. `EMBEDDING_POLICY` grown by a directive turns AC2 red with the documents untouched.
- Pass-3 D2 is closed on both channels independently, across seven methods, including `assert`, `trace`, `table`, `count`, `dir` and `group`. The probe comments at `:20-27` and `:271-275` and the printed detail are true in Chromium 151: 24 of 24 own console functions are wrapped.
- Pass-3 D3 is closed against the glue's source and the worker's `import()`.
- The control-character refusal sits after the whitespace check, so tab, CR and LF still get the whitespace message, and its test goes red when the branch is removed.
- `use.html` and `more.html` do not mention the policy yet, and nothing in this diff describes them. The README and LLD text around the edits reads correctly apart from D2's sentences.
- Every required command is green, listed in Evidence. The suite is 499 tests, one more than pass 3's 498, which is the control-character test.
- The tree is unchanged. `git write-tree` is `d0d556f8...` before and after, the status is the same five modified paths plus the two earlier review reports and this one, and the `git diff fb8d5ee` hash matches before and after. `target/f228-review4` was deleted at the end. One accidental scratch copy at `/tmp/x` (README, from a typo in my command) was deleted straight away.

## Limitations and handoff

- Not exercised: Firefox and WebKit, where the console namespace's own properties and the worker console forwarding may differ, the corpus-absent `NOT RUN` branch, a fresh `bin/ocelli.sh wasm`, AC11 to AC16, and a CDN-hosted glue.
- The same area has now drawn a finding in four passes (AC2's reach). Each round closed the named trigger and opened an adjacent one, and this round's demo did not replay deletion, which pass 1 and pass 2 had already listed. Worth telling the operator, though the findings are changing rather than repeating.
- Next owner: the implementer fixes D1 to D3 and S1, then a fresh independent pass. The lead decides N11.
