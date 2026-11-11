# F-228 review, pass 6

**Reviewed**: `git diff 20e9459 db427d9` on `work/f-228-xai`, HEAD `db427d9211fafdc6d660fd99003d206112d874d2`, clean working tree (`git diff` empty, `git write-tree` `922c43d5063fc512f70e20d862f1690521e022d4` before and after). Five paths: `.claude/plans/F-228-design.md` (lead, `f9638f7`), `README.md`, `docs/lld/typescript-packaging.md`, `examples/viewer-react/src/csp-policy.ts`, `examples/viewer-react/src/csp-policy.test.ts` (implementer, `db427d9`). Untracked earlier review reports read as context, `tools/oracle/out` ignored. Scope is the example-viewer half, AC1 to AC10, AC2 as amended by "Lead amendments after review pass 4". AC11 to AC16 are not reviewed.
**Author / reviewer**: plan lines by the sprint lead (Claude), remediation by GLM-5.3 in opencode per the report at the end of `glm-F-228-fix5.log`. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed material.
**Result**: 1 defect, 0 smells, 3 nitpicks

## Acceptance challenged

The plants ran on a scratch copy at `target/f228-review6` holding `README.md`, the LLD, `csp-policy.ts`, `csp-policy.test.ts` and `vitest.config.ts`. Each plant started from a fresh copy of a pristine snapshot and ran `npx vitest run --root . examples/viewer-react/src/csp-policy.test.ts -t "every marked block"`, so only the AC2 test executed. 51 cases, every one as expected. The scratch was deleted at the end and nothing ran against a tracked file.

**Pass 5's 35 cases, replayed.** All 30 red plants are still red with the named finding: header widened, directive dropped, trailing space and tab, CRLF throughout (`found 0 marked blocks`), header line alone ending CR, header name lower-cased and `-Report-Only`, entity-encoded quotes, each block deleted, each marker deleted, marker before prose, marker as last line, blank line after the marker, `~~~text` fence, an extra marked block in each document, indented marker, marker with a trailing space, a marked block inside a `~~~markdown` block in each document. The three by-design greens are still green: an indented marker before an extra `default-src *` block, unmarked prose and an unmarked fenced block carrying `'unsafe-eval'` and `default-src *`.

**Pass 5's D1 is closed.** R1 swapped to the pacs header is red (`README.md:177: marked block 1's header line is not the expected header`), R2 swapped to the base header is red (`README.md:193: marked block 2's ...`).

**Pass 5's D2 is closed.** A second CSP line in the LLD block, a `Report-Only` line in R1 and R1's closing fence removed are each red with `the marked block is not exactly the opening fence, one header line and the closing fence`.

**Pass 5's N1 is taken.** ```` ```bash ```` and ```` ```js ```` before the exact header are now red (`the marker is not followed by a ```text block`), which the docs now state.

**New cases, all red**:

| Plant | Finding |
|-------|---------|
| README blocks in swapped order | `:177` block 1 and `:193` block 2, header not expected |
| R1 with three header lines, LLD with three header lines | not exactly fence, one header line, fence |
| R2 with an extra blank line before the closing fence | not exactly ..., `: ` (the blank line) |
| LLD closing fence ```` ``` trailing ```` | not exactly ..., `` ``` trailing `` |
| R1 opening fence ```` ```text ```` plus a trailing space | not followed by a ```text block, count 1 of 2 |
| LLD block moved to end of file without closing fence, with and without a final newline | not exactly ..., `: ` and `(the document ends first)` |
| R1 opening fence CRLF, R2 closing fence CRLF, both LLD fences CRLF, LLD marker line CRLF | not followed / not exactly / count 0 of 1 |
| LLD closing fence of four backticks, LLD closing fence with two trailing spaces | not exactly ... (stricter than GFM, which accepts both, fails closed) |
| An extra marked base block inserted after R1 | block 2 header not expected, block 3 has no expected header, count 3 of 2 |

**Policy drift through the module**: `EMBEDDING_POLICY` grown by `["base-uri", ["'none'"]]` in the scratch turns AC1's two tests and AC2 red, with all three marked blocks named. The AC10 scan is also red in the scratch, and is red on the unplanted scratch too, because the scratch holds no `packages/*/src`. That is a scratch artefact and not a finding.

**Rendering.** `pandoc -f gfm -t html` renders the new ```` ```text ```` spans in README and the LLD as `<code>```text</code>`, and both markers still pass through as comments.

**Plan lines.** `F-228-design.md:172` now says "every marked block quotes it (AC2)" and the AC2 row at `:183` says "Superseded by the marker rule in the lead amendments after review pass 4". Neither names `more.html`, entity decoding or "byte for byte" any more, so pass 5's N4 is closed. The Trigger column matches the amendment. N1 below is about the row's last-but-one column.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build` | `db427d9`, existing `crates/ocelli-wasm/pkg` not rebuilt | now | 0, built in 970ms |
| `npm run typecheck` | `db427d9` | now | 0 |
| `npm run lint` | `db427d9` | now | 0 |
| `npm test` | `db427d9` | now | 0, 61 files, 499 passed (499), no `skipped` or `todo` in the log |
| `node examples/viewer-react/csp-probe.mjs` | Chromium 151.0.7922.34, corpus present | now | 0, 13 passed, 0 failed, 0 not run |
| AC2 plant matrix, 51 cases | scratch copy of `db427d9` | now | 51 as expected, 0 unexpected |
| `EMBEDDING_POLICY` grown by `base-uri`, whole file | scratch copy | now | AC1 two tests and AC2 red |
| `pandoc -f gfm -t html` of README and the LLD | `db427d9` | now | spans render as code, markers as comments |
| `bin/ocelli.sh gate packages` | `db427d9` | now | 0, 499 passed, ALL GREEN |
| `bin/ocelli.sh gate guards` | `db427d9` | now | 0, ALL GREEN, harness 10 properties |
| `python3 scripts/guard_census.py` | `db427d9` | now | 0, 1102 refusals in 78 files, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | `db427d9` | now | 0 |
| `python3 scripts/prose_check.py` | `db427d9` | now | 0, OK, 865 files |
| `python3 scripts/source_provenance_check.py` | `db427d9` | now | 0, OK, 1711 files |
| `git write-tree`, `git status --short`, `git diff \| shasum -a 256`, `git rev-parse HEAD` | worktree | now, before and after | unchanged, empty diff hash `e3b0c442...`, HEAD `db427d9` |

## Defects

### D1, README says the DICOMweb header is below the sentence, and it is above
**Where**: `README.md:219-221`, "holds the header line to the expected header for that block, the first marked block here the base header above and the second the DICOMweb header below".
**What**: the sentence sits at line 219 to 222. The base header block is at `:177-180`, above it, and the DICOMweb header block is at `:193-196`, also above it. Nothing below the sentence quotes a CSP header: `grep -n "Content-Security-Policy:" README.md` prints lines 179 and 195 only, and the next section is "Start the local series viewer". A reader sent "below" for the DICOMweb header finds the folder viewer instead.
**Why it is wrong**: a false sentence in `README.md` is a defect under section 3 of the review command. The part about what the test checks is true (the k-th block against the k-th expected header, plants above), only the locator is wrong.
**Evidence**: `grep -n "Content-Security-Policy:\|ocelli:csp" README.md` gives 177, 179, 193, 195 and 217, so both blocks precede the sentence.
**Scope and regression**: README only. The LLD's matching sentence says "here the base header" with no locator and is true. Smallest fix: "the first marked block here the base header and the second the DICOMweb header", or "both above". No test change, this is prose that review checks under the amendment's "Prose is not parsed".

## Smells

None.

## Nitpicks

- N1, new, for the lead. The AC2 row's "Wrong implementation rejected" column at `F-228-design.md:183` still reads "A fourth copy in prose that drifts", while the amendment it now points to says "Prose is not parsed". An unmarked drifting copy, in prose or in an unmarked fenced block, is green by design (replayed above). If "copy in prose" means a quote in a document, it is true of marked blocks. "A marked quote that drifts from `policyHeader`" would say what the test rejects.
- N2, new, for the lead. The amendment at `:300` says the marked block is "in Markdown a fenced block", and the test now requires exactly ```` ```text ````. The test is stricter, which fails closed, and README and the LLD state the stricter rule, so nothing is false. Only the amendment lags its implementation.
- N3, carried from pass 5's N2. The test comment at `csp-policy.test.ts:151-152` still lists "the Use book's block lands with the folder-viewer half" inside the sentence saying what the test holds.
- Pass 5's N3 (the short docstring line) is closed by the rewrap, and N4 is closed by `f9638f7`. Pass 4's N1, N2, N3, N6, N7, N8, N9, N11 (lead), N12, N13, N14, N15 and N17 are untouched by this diff and remain carried.

## Verified clean

- The matcher binds each marked block whole and in order: opening exactly ```` ```text ````, one header line equal to the k-th expected header, closing exactly ```` ``` ````, and the count taken from the expected list. Every branch of `csp-policy.test.ts:164-199` was driven red by at least one plant, including both `(the document ends first)` paths and the "no expected header" path.
- Every sentence in the diff about AC2 was checked against the plants. README `:216-222`: the marker on its own line, "exactly the opening ```text fence, one header line and the closing fence", "the expected header for that block", "exactly two" and "exactly one" are true, the "below" locator is D1. LLD `:313-319`: true. `csp-policy.ts:5-10`: true. The test docstring at `:135-140`, the comment at `:145-152` (apart from N3) and the assertion message at `:203` are true. The test name "every marked block is the exact policy text" is now true, since the block is held whole.
- The plan's line 172 and the AC2 row's trigger match the amendment (N1 and N2 aside).
- Pass 4's closed items still hold. `examples/viewer-react/csp-probe.mjs` is byte-identical to `23b341c` (`git diff --quiet`), and the probe runs green, 13 of 13. The CDN sentence at `csp-policy.ts:54-56` is unchanged. The old prose matcher is still gone: `git grep` outside the review reports finds none of `POLICY_TOKENS`, `policyValueSetBy`, `decodeEntities`, `HEADER_LINE`, `HEADER_NAME` or `includes(header)`. `use.html` and `more.html` are not touched by `23b341c..db427d9` and quote no policy yet.
- Every required command is green, as listed in Evidence. The suite is still 499 tests, which fits one AC2 test rewritten in place.
- The tree is unchanged: HEAD `db427d9`, empty `git diff`, `git write-tree` `922c43d5...` before and after, `git status` the same untracked set plus this report. `target/f228-review6` was deleted.

## Limitations and handoff

- Not exercised: the HTML `<pre>` shape for the Use book, since no HTML page quotes the policy yet. Also GitHub's own renderer (pandoc's GFM reader stood in for it), Firefox and WebKit, a fresh `bin/ocelli.sh wasm`, the corpus-absent `NOT RUN` branch of the probe, the probe's console plants (the file is unchanged since pass 4 verified them), and AC11 to AC16.
- GLM's report says "No commit", and `db427d9` is a commit. Whoever committed it, the reviewed content is the commit.
- AC2's test has no finding this pass. The one defect is a locator word in README prose. Next owner: the implementer fixes D1, the lead decides N1 and N2, then a fresh independent pass.
