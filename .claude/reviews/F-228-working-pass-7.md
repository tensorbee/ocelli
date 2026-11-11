# F-228 review, pass 7

**Reviewed**: `git diff db427d9 0490062` on `work/f-228-xai`, HEAD `0490062ef3dd80696ca3cecf4334a2b52a5b8fac`, clean working tree (`git diff` empty, `git write-tree` `8ad81e39bc20490b7e9290253fa23b39513306f2` before and after). Three paths: `.claude/plans/F-228-design.md` (AC2 row and the pass 4 amendment), `README.md:219-222`, `examples/viewer-react/src/csp-policy.test.ts:148-152`. Untracked earlier review reports read as context, `tools/oracle/out` ignored. The whole README CSP section, `README.md:172-226`, was re-read. Scope is the example-viewer half, AC1 to AC10, AC2 as amended. AC11 to AC16 are not reviewed.
**Author / reviewer**: remediation by the sprint lead (Claude), commit `0490062`. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed material.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

**Pass 6's D1 is closed.** `README.md:219-221` now reads "the first marked block here the base header and the second the DICOMweb header", with no locator. `grep -n "ocelli:csp\|Content-Security-Policy:" README.md` gives the markers at 177 and 193 and the headers at 179 (base) and 195 (DICOMweb), and the test's README list is `[policyHeader(), policyHeader(["https://pacs.example"])]` at `csp-policy.test.ts:153`, so first is base and second is DICOMweb. True.

**Pass 6's N1 is closed.** The AC2 row's rejected-implementation cell at `F-228-design.md:183` now says "A marked block that drifts from `policyHeader()`". Replayed in a scratch copy (README, the LLD, `csp-policy.ts`, `csp-policy.test.ts`, `vitest.config.ts`, under the scratchpad, `-t "every marked block"`): unplanted is green, 1 passed. With `https://pacs.example` removed from `README.md:195` it is red with `README.md:193: marked block 2's header line is not the expected header`. That is a marked block drifting from `policyHeader()`, rejected. The scratch was deleted.

**Pass 6's N2 is closed.** The amendment at `F-228-design.md:300` now says "in Markdown a fenced block opened by ```` ```text ````", which is `opening !== "```text"` at `csp-policy.test.ts:169`. The rest of the bullet (header line equal to the expected header, README exactly two, LLD exactly one, the folder-viewer half adds the Use book's block and count) is unchanged and still true against `:153-154` and `:194-198`. The HTML `<pre>` clause describes a shape no document uses yet and the test does not implement, and the bullet's last sentence assigns that to the folder-viewer half, so it is a plan for later work rather than a claim about this test.

**Pass 6's N3 is closed.** The comment at `csp-policy.test.ts:145-151` now ends "each document to exactly that number of marked blocks." Each sentence matches the code below it: marker on its own line (`:164`), ```` ```text ```` opening (`:169`), one header line (`:176`), closing fence (`:188`), k-th block to k-th expected header (`:177`), exact count (`:194`). The diff to this file is comment-only: every `+`/`-` line in `git diff db427d9 HEAD -- csp-policy.test.ts` is a `//` line.

**README CSP section re-read, `:172-226`.** No other locator or count is false. "two marked blocks" and "exactly one" in the LLD match the files (`docs/lld/typescript-packaging.md:282` is its only marker). "the four directives" style counts are not in this section. "decision D5 in the decision log" is the SharedArrayBuffer and single-threaded decision at `docs/hld/11-decision-log.md:26`. "binds no port and starts no server" matches `csp-probe.mjs:389`, `page.route("**/*", ...)`, with no `listen` or `createServer` in the file. `openDecodePool` exists and is exported (`packages/core/src/index.test.ts:191`). "The policy exists once in `examples/viewer-react/src/csp-policy.ts`" stands from pass 6. "the local series page below" at `:226` is true, the next section is "Start the local series viewer" at `:228`. The LLD's matching sentence at `:313-319` is unchanged and still true.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run typecheck` | `0490062` | now | 0 |
| `npm run lint` (`eslint .`) | `0490062` | now | 0 |
| `npm test` | `0490062` | now | 0, 61 files, 499 passed (499), no `skipped` or `todo` in the log |
| AC2 scratch, unplanted and README:195 drift | scratch copy of `0490062` | now | green 1 passed, then red naming `README.md:193` block 2 |
| `bin/ocelli.sh gate packages` | `0490062` | now | 0, 499 passed, ALL GREEN |
| `bin/ocelli.sh gate guards` | `0490062` | now | 0, ALL GREEN |
| `python3 scripts/guard_census.py` | `0490062` | now | 0, 0 watched by nothing |
| `python3 scripts/prose_check.py` | `0490062` | now | 0, OK, 865 files |
| `python3 scripts/source_provenance_check.py` | `0490062` | now | 0, OK, 1711 files |
| `npx prettier --check` on the test at HEAD and at `db427d9` | `git show db427d9:...` into scratch | now | both warn, the diff Prettier wants is at `:336-345` (`needsWeakerPolicy` expects), outside this commit's hunk `@@ -148,8 +148,7 @@` |
| `git grep -il prettier` | `0490062` | now | no tracked file names prettier, `node_modules/.bin/prettier` absent, `npx` fetched it |
| `git write-tree`, `git status --short`, `git diff \| shasum -a 256` | worktree | now, before and after | `8ad81e39...` unchanged, same untracked set plus this report, empty diff hash `e3b0c442...` |

## Defects

None.

## Smells

None.

## Nitpicks

None new. Pass 6's N1, N2 and N3 are closed above. Pass 4's carried nitpicks (N1, N2, N3, N6, N7, N8, N9, N11, N12, N13, N14, N15, N17) are untouched by this diff and remain carried as non-blocking.

## Verified clean

- Each of the four pass 6 findings, closed against the test and the documents as above, with the AC2 test driven green and red once in a scratch copy.
- Every changed sentence in the diff, the README sentence, the AC2 row cell, the amendment bullet and the test comment, checked against `csp-policy.test.ts:152-203`.
- The whole README CSP section for locators and counts, as above.
- Prettier: the formatting it reports is pre-existing at `db427d9` and sits outside the lines this commit touched. No gate runs Prettier. It is not a dependency, no tracked file names it, and `npm run lint` is `eslint .`, which is green. Not a finding.
- Every required command green, as in Evidence. The suite is 499 tests, unchanged from pass 6, which fits a comment-only test change.
- Tree unchanged, as in Evidence.

## Limitations and handoff

- Not exercised this pass: the probe `csp-probe.mjs` (byte-unchanged since `23b341c`, green 13 of 13 in pass 6), the HTML `<pre>` shape for the Use book, GitHub's own Markdown renderer, a fresh `bin/ocelli.sh wasm`, AC11 to AC16 and the oracle. Pass 6's 51-case plant matrix was not replayed, since the test's logic is unchanged.
- This is a clean bounded review of `db427d9..0490062` for the example half. It is not the folder-viewer half's review, nor completed acceptance or sprint readiness.
