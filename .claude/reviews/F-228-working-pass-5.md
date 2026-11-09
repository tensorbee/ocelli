# F-228 review, pass 5

**Reviewed**: the uncommitted working tree on `work/f-228-xai` against the pass-4 checkpoint `23b341c`, four tracked paths: `README.md`, `docs/lld/typescript-packaging.md`, `examples/viewer-react/src/csp-policy.ts`, `examples/viewer-react/src/csp-policy.test.ts`. `git diff 23b341c` is 56 insertions and 135 deletions, sha256 `fbc541f7abeb5b80...`. Untracked `.claude/reviews/F-228-working-pass-2.md` to `-pass-4.md` read as context, `tools/oracle/out` ignored as instructed. `git write-tree` (the index) is `815b9f16d7cb972eeac9d385919d6e0e80b1356b` before and after. `git diff fb8d5ee 23b341c -- README.md docs examples` hashes to `c2cfac578d22cdec...`, the hash pass 4 recorded for the tree it reviewed, so the checkpoint is that tree. Scope is the example-viewer half, AC1 to AC10, with AC2 as amended by "Lead amendments after review pass 4". AC11 to AC16 are not reviewed.
**Author / reviewer**: remediation by GLM-5.3 in opencode, per its report at the end of `glm-F-228-fix4.log`. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed material.
**Result**: 2 defects, 0 smells, 4 nitpicks

## Acceptance challenged

Every AC2 plant ran on a scratch copy at `target/f228-review5/tree` holding `README.md`, the LLD, `csp-policy.ts`, `csp-policy.test.ts` and `vitest.config.ts`, run with `npx vitest run --root . examples/viewer-react/src/csp-policy.test.ts -t "every marked block"` so only the AC2 test executed. Each plant started from a pristine copy, and after the matrix both the pristine and the scratch copies were compared byte for byte with the real files and were the same. Nothing ran against a tracked file. The scratch was deleted at the end.

**The marker rule, end to end.** Red, each with its own finding line:

| Plant | Finding |
|-------|---------|
| README first header gains ` https://evil.example` | `README.md:177`, header line not expected |
| LLD header loses `worker-src 'self';` | `typescript-packaging.md:282`, header line not expected |
| README second header gains a trailing space, first a trailing tab | `:193` and `:177`, header line not expected |
| README converted to CRLF throughout | `found 0 marked blocks, expected exactly 2` |
| LLD header line alone ends in CR | `:282`, header line not expected |
| LLD header name lower-cased, README name made `-Report-Only` | header line not expected |
| Entity-encoded quotes `&#39;` in the LLD header | header line not expected (no decoding now, correct for Markdown) |
| README first block deleted, second block deleted, LLD block deleted | count 1 of 2, 1 of 2, 0 of 1 |
| README first marker deleted, LLD marker deleted | count 1 of 2, 0 of 1 |
| Marker followed by prose, marker as the last line, marker then a blank line then the block | `the marker is not followed by a fenced block` (the blank-line case also drops the count) |
| Marker before a `~~~text` fence | not followed by a fenced block, count 1 of 2 |
| Extra identical marked block in the LLD, extra marked DICOMweb block in README | count 2 of 1, 3 of 2 |
| Marker indented two spaces, marker with a trailing space | count 1 of 2 |
| Marker plus fence plus exact header inside a `~~~markdown` block in README | count 3 of 2 |
| Same inside the LLD with `default-src *` | header line not expected, and count 2 of 1 |

Green: the baseline, a block in ```` ```bash ```` or ```` ```js ```` with the exact header (N1), an indented marker before an extra `default-src *` block (by design, see Verified clean), unmarked prose and an unmarked fenced block carrying `'unsafe-eval'` and `default-src *` (by design, prose is not parsed), and the four plants that are D1 and D2.

**Pass-4 D1 (deletion green) is closed**: the three deletions and both marker deletions are red. **Pass-4 D2 and D3 are closed** by removal: the matcher and the sentences describing it are gone. **Pass-4 S1 is closed by the amendment**, which fixes the HTML shape (`<pre>` with the code on the next line) for the folder-viewer half. That shape is not read today, correctly, since no HTML page quotes the policy yet.

**Policy drift reaches AC2 through the module**: `EMBEDDING_POLICY` grown by `["base-uri", ["'none'"]]` in the scratch copy turns "every marked block is the exact policy text" red beside "is exactly the four directives", with the documents untouched. Restored and compared afterwards.

**Rendering.** `pandoc -f gfm -t html` of both documents passes each marker through as a raw `<!-- ocelli:csp -->` comment, which a browser does not display, and each block still renders as `<pre class="text"><code>Content-Security-Policy: ...</code></pre>`. The inline mention in the prose at `README.md:217` and the LLD's `:315` renders as escaped code text, `<code>&lt;!-- ocelli:csp --&gt;</code>`, and the test does not count it, because that line is not the marker alone.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | working tree, existing `crates/ocelli-wasm/pkg` not rebuilt | now | 0, built in 973ms |
| `npm run typecheck` | working tree | now | 0 |
| `npm run lint` | working tree | now | 0 |
| `npm run test` | working tree | now | 0, 61 files, 499 passed (499), no skip or todo line in the log |
| `node examples/viewer-react/csp-probe.mjs` | Chromium 151.0.7922.34, corpus present | now | 0, 13 passed, 0 failed, 0 not run |
| AC2 plant matrix, 35 cases | scratch copy | now | as tabled in Acceptance |
| `EMBEDDING_POLICY` grown by `base-uri` | scratch copy | now | 2 red, AC1 and AC2 |
| `pandoc -f gfm -t html` of README and the LLD | working tree | now | markers pass through as comments, blocks render as code |
| `bin/ocelli.sh gate packages` | working tree | now | 0, 499 passed, ALL GREEN |
| `bin/ocelli.sh gate guards` | working tree | now | 0, ALL GREEN, harness 10 properties |
| `python3 scripts/guard_census.py` | working tree | now | 0, 1102 refusals in 78 files, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | working tree | now | 0 |
| `python3 scripts/prose_check.py` | working tree | now | OK, 865 files |
| `python3 scripts/source_provenance_check.py` | working tree | now | OK, 1711 files |
| `git write-tree`, `git status --short`, `git diff 23b341c \| shasum -a 256` | worktree | now, before and after | unchanged |

## Defects

### D1, a README block may quote the other README header, so README can stop quoting `policyHeader()` and stay green
**Where**: `examples/viewer-react/src/csp-policy.test.ts:150-164` and `:180`. Both README blocks are compared against the set `[policyHeader(), policyHeader(["https://pacs.example"])]` with `expectedHeaders.includes(header)`.
**What**: the first README block, at `README.md:177-180`, under "An embedding page ... needs one header on every response", replaced by the DICOMweb header with `https://pacs.example` is green. So is the second block, at `:193-196` under "A DICOMweb service on another origin is added to `connect-src`", replaced by the base header. In the first case README quotes no `policyHeader()` at all and tells every embedder they need a third-party origin in `connect-src`. In the second, README no longer shows the DICOMweb extension it describes.
**Why it is wrong**: the amendment says "AC2 checks that each marked block's header line equals the expected header exactly", one expected header per block. The plan's AC2 row (`F-228-design.md:183`) requires README to contain `policyHeader()`, and the wrong implementation it rejects is "A fourth copy in prose that drifts". The test comment at `:146-147` says the test holds "each marked block's header line to the exact expected header", which is false for the swap.
**Evidence**: plants "R1 header swapped to pacs header" and "R2 header swapped to base header", each `Tests 1 passed | 12 skipped (13)`.
**Scope and regression**: README now, and the Use book when the folder-viewer half adds its block. Smallest fix: give each document an ordered list of expected headers, compare the k-th marked block with the k-th entry, and take the count from the list length. Then replay both swaps and see red.

### D2, the prose says each marked block is held to the header text, and only its first line is
**Where**: `README.md:216-218` ("holds each fenced block that the `<!-- ocelli:csp -->` marker precedes on its own line to the exact header text"), `docs/lld/typescript-packaging.md:314-316` (the same words), and `examples/viewer-react/src/csp-policy.ts:6-7` ("compare each marked block against it byte for byte"). The check is `csp-policy.test.ts:179-180`, which reads `lines[index + 2]` and nothing after it.
**What**: green on the scratch copy: a second line `Content-Security-Policy: script-src 'self' 'unsafe-eval'` added inside the LLD's marked block, a second line `Content-Security-Policy-Report-Only: default-src *` added inside README's first block, and README's first block with its closing fence deleted. In GFM the last one turns the rest of the README section into code. A reader who copies the block copies every line in it.
**Why it is wrong**: a false sentence in README and the LLD is a defect under section 3 of the review command. The amendment's rule is narrower ("each marked block's header line"), and the docs and the module docstring claim the wider one.
**Evidence**: plants "LLD block gains a second CSP line", "R1 block gains a second line (Report-Only *)" and "R1 block closing fence removed", each `Tests 1 passed | 12 skipped (13)`.
**Scope and regression**: README, the LLD and the module docstring. The assertion message at `:194` ("must quote the policy header exactly") is true as worded. Smallest fix: require `lines[index + 3]` to be the closing fence, which makes "the block is the header" true and costs one comparison. Otherwise reword the three sentences to "the first line of each marked block". Then replay the three plants.

## Smells

None.

## Nitpicks

- N1, new. `opening.startsWith("```")` (`csp-policy.test.ts:172`) accepts any info string, so the exact header in a ```` ```bash ```` or ```` ```js ```` block is green. The content is exact, so nothing drifts. The house blocks are `text`, and requiring ```` ```text ```` would fix one spelling.
- N2, new. The test comment at `:144-149` lists "the Use book's block lands with the folder-viewer half" as a third thing the test holds. Two sentences would read correctly.
- N3, new. The rewrap at `csp-policy.ts:6-8` leaves a short middle line ("byte for byte. The four directives are F-228's design"). Cosmetic. D2 is about what the sentence claims.
- N4, new, for the lead. `F-228-design.md:172` and the AC2 row at `:183` still say "every document quotes it byte for byte", name `more.html`'s `#worker` section and entity decoding, while the amendment at `:300` names only the Use book's block. Whether the More page still quotes the policy is a lead call.
- Carried, untouched by this diff: N1, N2, N3, N6, N7, N8, N9, N11 (lead), N12, N13, N14, N15 and N17 from pass 4. N16 (the "case-insensitively" wording) is closed, since that prose is gone.

## Verified clean

- The old prose matcher is gone. `git grep` over the tree, review files excluded, finds none of `POLICY_TOKENS`, `policyValueSetBy`, `decodeEntities`, `HEADER_LINE`, `HEADER_NAME`, "however the line spells", "prose mention" or "no directive or source". The only `Content-Security-Policy` in the test is the expected-header template at `:163`. No rule reads a line that is not the marker, the line after it or the line after that. Unmarked prose and an unmarked fenced block carrying `'unsafe-eval'`, `*` and `default-src *` stay green, which is what "prose is not parsed" means.
- The marker must equal `<!-- ocelli:csp -->` exactly, alone on its line, directly before the fence. An indented marker, a trailing space, CRLF and a blank line in between all fail closed through the count or the "not followed" finding. An indented marker before an *extra* block is not a marker, so that block is unchecked, which is the same as any unmarked block under the amendment.
- A marker inside another code block is still counted, so it fails through the exact count, both with the exact header and with a wrong one.
- Every sentence in the diff was checked against execution. README `:216-218` and the LLD `:314-317`: the marker, "on its own line" and the counts (two and one) are true. "To the exact header text" is D2. The test comment and the `CSP_MARKER` docstring at `:135-139` are true apart from D1's "the exact expected header". The assertion message at `:194` is true. The module docstring is D2.
- README and the LLD read correctly around the markers. Each marker sits between the introducing sentence's blank line and the fence, and the pandoc GFM output shows no visible marker and unchanged code blocks.
- Pass 4's closed items still hold. `examples/viewer-react/csp-probe.mjs` is byte-identical to `23b341c` (`git diff --quiet`), and pass 4 verified that file's strict console capture on both channels. The probe runs green now, 13 of 13. The CDN sentence at `csp-policy.ts:52-53` (the worker's `import()` of the glue is a script request) is unchanged.
- Every required command is green, listed in Evidence. The suite is 499 tests, the same as pass 4, which fits one AC2 test replaced by one AC2 test.
- The tree is unchanged: `git write-tree` is `815b9f16...` before and after, the status is the same four modified paths plus the earlier review reports and this one, and the `git diff 23b341c` hash is the same before and after. `target/f228-review5` was deleted.

## Limitations and handoff

- Not exercised: the HTML `<pre>` shape the amendment names for the Use book, because no HTML page quotes the policy yet and the test does not read one. Also not exercised: GitHub's own renderer (pandoc's GFM reader stood in for it), Firefox and WebKit, a fresh `bin/ocelli.sh wasm`, the corpus-absent `NOT RUN` branch, and AC11 to AC16. The probe was run but its console plants were not repeated, because the file is unchanged since pass 4 verified them.
- AC2 has drawn a finding in five passes now. This round's findings are narrower than before: the marker rule is sound, and both defects are about what the marker rule binds (which header goes in which block, and how far the block runs). Each is a few lines to fix.
- Next owner: the implementer fixes D1 and D2, then a fresh independent pass. The lead decides N4 and pass 4's N11.
