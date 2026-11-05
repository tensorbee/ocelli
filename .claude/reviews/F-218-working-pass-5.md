# F-218 review, pass 5

**Reviewed**: worktree `ocelli-wt-f-218`, branch `work/f-218-xai`, base and HEAD `ab3d11cc7586069aa95bb02762b576acb4dc3fbb`. Staged tree `891e11f71ff0592855983021aba9e2fe46052d1d` (`git write-tree`), unchanged since pass 1. The remediation of passes 1 to 4 is unstaged, plus untracked `scripts/tests/test_unreachable_injections.py`. `tools/oracle/out` is untracked and out of scope. Pin: SHA-1 of `git diff` is `0fbd9e7ef5b2a52ffda2ffd7f0cac3dbf3658478` and of the untracked test is `49008b1dcdb1a0e6ee708d0568d53731058c87e9`, both identical at the start and the end of this pass.
**Author / reviewer**: implementation by an external coding agent, remediation of pass 4 by the sprint lead (Claude). Reviewer is a fresh independent Claude agent that wrote none of the diff, the plan, the earlier passes or the pass 4 fix. The only worktree file this pass wrote is this review. Reconstruction and a node probe ran in the session scratchpad. Gate runs wrote ignored build output (`packages/*/dist`) as `gate packages` always does.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **The pass 4 fix changed only what it says.** In scratch copies, "the rows it checks." was put back to "the rows it proves." in `scripts/advisory_check.py`, "a row it checks," back to "a row it proves," in `scripts/tests/test_advisory_check.py`, and the `docs/lld/guards.md:625-628` paragraph back to its pass 4 wrap (the 105-column line N1 named). Each replacement was asserted to match exactly once. The `git diff` rebuilt from those copies, with the blob `index` lines recomputed, hashes to `f658d99be7852cbf0820e006e6d438c6b93a3429`. That is pass 4's pin. The same diff followed by the untracked test hashes to `4a4435e30b8da112ddf332780e091c0ddf71fa80`, pass 4's second pin. So the tree differs from what pass 4 reviewed by exactly those two words and that line break, and nothing else moved. The rewrapped paragraph is now 75, 75, 71 and 44 columns.
- **A sweep for certainty about unreachability or safety, in every word form.** All 2183 added lines were extracted: `git diff -U0 ab3d11c` (minus the generated runbook, swept separately) plus the full untracked test. They were grepped case-insensitively for `prov`, `proof`, `guarantee`, `ensur`, `never reach`, `cannot reach`, `impossible`, `always`, `verif`, `demonstrat`, `establish`, `confirm`, `safe`, `certain`. A second grep covered `never`, `cannot`, `nothing`, `no shipped|entry|file|load`, `unreachab` and `every`. Each hit was read in context and judged against the code or an executed check (table below).

## Evidence

All of it was executed in this pass. node v24.13.0, Python 3.14.5, macOS.

| Command / check | Exit and observation |
|-----------------|----------------------|
| Pin reconstruction above | matches both pass 4 SHA-1 values |
| `grep -rlE 'adm-zip\|AdmZip\|admzip'` over installed `node_modules/dcmjs` 0.52.2 | only `package.json`. No `adm` substring in `build/*.js` or `generate/dictionary.mjs`. No `require(` or `import(` with a non-literal argument. The only `require` in `dcmjs.js` is `'pako'` |
| same grep over built `packages/core/dist` and `packages/react/dist` | no hit |
| `npm view adm-zip@0.6.1 version` | `0.6.1`, so the override the READMEs suggest names a real release |
| node probe: `module.register` resolve hook, then `createRequire(import.meta.url)("./dep.cjs")` | the hook logs the entry and `node:module` but not `dep.cjs`. "`module.register` does not see a `require` from `createRequire`" holds on node 24.13 |
| `bin/ocelli.sh gate packages` | exit 0, consumer OK line, injection test `Ran 1 test in 26.6s OK` |
| `bin/ocelli.sh gate guards` | exit 0, ALL GREEN |
| `guard_census.py` | exit 0, `1054 refusal(s) in 78 file(s), all claimed` |
| `guard_census.py --check-runbook` | exit 0 |
| `prose_check.py` | exit 0, 826 files |
| `source_provenance_check.py` | exit 0, 1648 files |
| `python3 -m unittest scripts.tests.test_advisory_check` | 48 tests OK |
| runbook added lines, same vocabulary | two hits, `census.impossible-wall-clock-pair` and a `safe-integer` row. Both are other stories' rows renumbered and not F-218 text |

## Sweep verdicts

Every hit was judged. The ones that carry a claim:

- **`README.md:309-311`, `packages/core/README.md:16-18`, `use.html:241`**: "Nothing in `@ocelli/core` or in dcmjs's published files loads adm-zip." It is stated as a fact apart from the gate, and inspection bears it out: no shipped JS file in either package contains `adm`, and dcmjs has no computed specifier. The next sentence, "`gate packages` checks that on every run", is followed by the run-time specifier bound, so the gate is not credited with more than it checks. True.
- **`README.md:325`, `packages/core/README.md:30-31`**: "safe because nothing calls it". This rests on the sentence above, which is true, and 0.6.1 exists. True.
- **`scripts/advisory_check.py:5-8`**: a root override "never reaches a consumer". That is npm's documented rule, quoted in `typescript-packaging.md`. "`gate packages` checks that no shipped entry point loads the package" is the pass 4 wording and is bounded. True.
- **`scripts/advisory_check.py:54-56`**: "the advisory gate and the consumer proof cannot accept different rows". The claim is scoped by "for the same decision". `exception_problems` (line 118) and `package_check._unreachable_rows`, `consumer_version_problems` and `unreachable_problems` (lines 179, 191, 379) all decide scope through `scope_is_unreachable`, and the padded-scope test pins both sides. True.
- **`docs/lld/guards.md:625-628`**: "checked by `gate packages` ... on the paths that gate runs". Bounded. True.
- **`docs/lld/typescript-packaging.md:179-181`**: "a check that no shipped file loads `adm-zip` on the paths this gate runs". Bounded. "Inside it, the test would prove nothing" at line 161 is base text (`git show ab3d11c` line 161).
- **`scripts/package_check.py:328-332`**: "`module.register` does not record a `require` from `createRequire`" was measured above. "a smoke script that never calls `createRequire` never loads the CommonJS build" agrees with dcmjs's `exports` map (`import` resolves to `dcmjs.es.js`) and with `@ocelli/core` being ESM. True.
- **`bin/ocelli.sh:291-293`, injection test docstring line 7**: "the static scan cannot see" a string-built load. That is true by construction, and the table asserts `{"trace"}` alone for the three `.join` cases, which `gate packages` ran green.
- **`ci/advisories.toml:2`**: "A row is accepted only when its proof is the packages consumer trace". This is the `proof` field check (`PROOFS`, probe `advisories.exception-proof`). True.
- **The `proof` vocabulary**: `PROOFS`, the `proof` field, "names no proof the packages gate runs", "the consumer proof" (base `package_check.py:28`), class `ConsumerProof`, "The consumer lock is the proof". These are the design's names for the row field and the version check, and none claims that unreachability is proved.
- **`package_check.py` OK line, "tarball contents and exports verified"**: base text (`ab3d11c` line 320), not F-218's.

No sentence claims that the gate proves, guarantees or establishes unreachability.

## Defects

None.

## Smells

None.

## Nitpicks

- **N1**: `ci/advisories.toml:20` reason says "No shipped dcmjs or @ocelli file names it." dcmjs's shipped `package.json` does name adm-zip, under `dependencies`. The sentence before it says exactly that, so a reader is not misled, and the scan covers `.js`, `.mjs` and `.cjs` only. "No shipped dcmjs or @ocelli script names it" would be exact.
- **N2**: the `packages` entry's `refuses` text at `scripts/guards/catalogue.py:10541-10542` says "A resolve trace that does not record the dcmjs ES entry is refused as well." `trace_entry_problems` refuses a missing CommonJS entry as well (`test_a_trace_without_the_dcmjs_entry_is_refused`). This sentence says less than the code does, not more, so it does not block.

## Verified clean

- **Pass 4 D1 closed.** Both comments now say "checks", and the pin reconstruction shows nothing else changed.
- **Pass 4 N1 closed.** The paragraph is rewrapped under 80 columns with its words unchanged.
- **Pass 4 N2** (the `smoke-requires-dcmjs` note says "leaves this control failing") was not changed and still does not block.

## Limitations and handoff

- "uses it only in test/testUtils.js" in the advisory reason cannot be checked from the published tarball, because that file is not in it. Only "does not publish" was checked (`files` in dcmjs `package.json` is `build`, `generate/dictionary.mjs`, `README.md`, `License.txt`).
- No mutation this pass. The pass 4 mutations (M3, M6, M7) cover the guard lines, and the pass 4 fix touched comments only.
- `gate guards-deep` was not run. Python 3.12, which CI pins, is not installed here.
- The remediation is still unstaged. The tree that gets staged needs its own verification receipt.
- Counts line: F-218 working pass 5, 0 defects, 0 smells, 2 nitpicks.
