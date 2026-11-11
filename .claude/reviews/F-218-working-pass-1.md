# F-218 review, pass 1

**Reviewed**: staged tree `891e11f71ff0592855983021aba9e2fe46052d1d` over base `ab3d11cc7586069aa95bb02762b576acb4dc3fbb`, worktree `ocelli-wt-f-218`, branch `work/f-218-xai`. No unstaged tracked change. `tools/oracle/out` is untracked and out of scope. Combined `git diff --cached` plus `git diff` SHA-256 `b3fd5ce6be840f02138fe6cbc24c6af339ba6c24f2211cb3999a33f91cf7d707`, identical at the start and the end of the pass.
**Author / reviewer**: implementation by an external coding agent (its report is in the session scratchpad, not tracked). Reviewer is an independent Claude agent that wrote none of the diff and none of the plan. The only file this pass wrote in the worktree is this review.
**Result**: 2 defects, 2 smells, 3 nitpicks

## Acceptance challenged

- **AC15 and AC16 together, the unreachability proof.** Independently derived expectation: a shipped file that loads adm-zip by any means on the node path must turn `check_consumer` red, through the trace if the specifier is not a literal. Fourteen injections into the installed dcmjs builds were run through the real `check_consumer` (harness below). The proof rejects ESM static, dynamic, subpath and template-literal imports, but it does not see a CommonJS resolution at all (D1), and it accepts an empty trace (S1).
- **The plan's invariant "the exception and its proof cannot be separated"** (State transitions section). Challenged by a row the two scripts read differently. They do (D2).
- **AC4 to AC9, AC13** through the `--report` seam over the real tree and real rows, with synthetic reports built from the live one: all behave as the plan states.
- **AC20.** Every one of the 17 fragment-table refusals was replaced with `pass` in a scratch copy and its probe rerun. Each probe went red (the harness reported the probe no longer caught). Separately, deleting the three call sites in `check_consumer` left every probe and the census green (S1).
- **AC21 and the user-facing claims**, by a real consumer `npm audit` and by applying the recommended consumer override.

## Evidence

All executed now, in this pass, on node v24.13.0, npm 11.6.2, Python 3.14.5, macOS. Scratch directory `scratchpad/review-f218/`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm audit --omit=dev --package-lock-only --json` at the root | staged tree lock | now | exit 1, SHA-256 `cd57216c...749e761b`, same digest as the implementer's. adm-zip 5 high plus 3 moderate, dcmjs `via: ["adm-zip"]`. The id set equals `ci/advisories.toml` (AC2) |
| `python3 scripts/advisory_check.py`, and `npm --report audit.json` | live and saved report | now | exit 0 both, three moderate `WARN` lines, `OK: advisory check, exception adm-zip@0.5.17` |
| `--report` with planted cases | saved report, mutated in code | now | new high on adm-zip, high on pako, critical on pako, dcmjs's own high: exit 1 `has no exception`. GHSA-8238 removed: exit 1 `no longer reports`. Clean report: all 8 `no longer reports`. Moderate on pako: exit 0 with `WARN`. High on vite: exit 0. Version 1 and the string `"2"`: `report version`. `{}`, `error` key, unparseable, absent file: `did not produce a report`. `severe`: `unknown severity` |
| `npm_config_registry=http://127.0.0.1:9/ bin/ocelli.sh gate advisories` | staged tree | now | npm prints `{"message": ..., "error": {...}}`, the gate prints `npm audit did not produce a report` and FAILED. npm absent from `PATH`: same refusal. Fails closed, which is plan decision 3 |
| Scratch copy of tracked files, `scope = "unreachable "` | staged tree copy | now | `advisory_check.py exceptions` exit 0, `package_check._unreachable_rows()` returns `[]` (D2) |
| `harness.py` plus `variants.py`, real `check_consumer` over the packed tarballs, npm install replaced by a copy of one real install | tarballs packed from the staged tree | now | see the injection table under D1 |
| Same harness with `TRACE_REGISTER` emptied | as above | now | `es-subpath` and `es-stringbuilt-dynamic`, each of which loads adm-zip, return 0 problems with a 0-line trace (S1) |
| `node --import ./trace-sync.mjs smoke.mjs` using `module.registerHooks` | the `es-stringbuilt-createRequire` consumer | now | 19 adm-zip URLs recorded, where the shipped hook records 0 |
| Consumer `npm audit --omit=dev --json` | unmodified consumer install | now | exit 1. high 4: `adm-zip` (8 advisories), `dcmjs`, `@ocelli/core`, `@ocelli/react` (AC21) |
| Consumer with `"overrides": {"adm-zip": "0.6.1"}`, `npm install`, `npm ls`, `npm audit --omit=dev`, `node smoke.mjs` | fresh consumer from the tarballs | now | `adm-zip@0.6.1 overridden`, `found 0 vulnerabilities`, `ok 0.1.0`. The README recipe is accurate |
| `deletions.py`, 17 refusals replaced with `pass`, one probe each | scratch copy | now | 17 of 17 probes red under deletion. Copy restored, `git diff --stat` empty |
| Three `problems +=` lines removed from `check_consumer` | scratch copy | now | census OK, all 19 new probes green (S1) |
| Constant mutations: `REFUSED` without critical, `KNOWN` without info, `FIELDS` without `decided_by`, `SCOPES` plus `dev-only` | scratch copy | now | unit tests red for 3 of 4 (the `FIELDS` one stays green, N1), census FAIL for all 4 |
| `bin/ocelli.sh gate advisories packages licences ci guards guards-deep prose provenance backlog deviations lint types` | staged tree | now | all exit 0. guards: 413 refusal probes, 37 guards, 44 accept, 78 controls. guards-deep: 463, 40, 59, 82. packages: 57 files, 468 tests, consumer OK. ci: 28 floor gates. backlog: 205 planned F-IDs. deviations: 27 |
| `python3 scripts/guard_census.py`, `--check-runbook` | staged tree | now | both `OK: 1051 refusal(s) in 78 file(s), all claimed`, 0 watched by nothing |
| `python3 -B -m unittest discover -s scripts/tests -p test_advisory_check.py` | staged tree | now | 41 tests OK |
| `gh run view 41951682234` | `main` at the S45 merge | prior CI receipt | TypeScript job success: 57 files, 468 tests, packages OK, with no wasm build step |

## Defects

### D1, the resolve trace is blind to CommonJS resolution, so a shipped load through `createRequire` passes

**Where**: `scripts/package_check.py`, `TRACE_REGISTER` and `TRACE_HOOK` (the `register("./trace-hook.mjs", ...)` hook), consumed at the `resolved_package_problems` call in `check_consumer`.
**What**: `module.register` installs asynchronous ESM loader hooks. They do not see a `require()` made through `createRequire` from an ES module. Appending `import { createRequire as __cr } from "node:module"; __cr(import.meta.url)(["adm","zip"].join("-"));` to the installed `dcmjs/build/dcmjs.es.js`, the file node and bundlers both pick through dcmjs's `import` condition, loads adm-zip during the smoke import (16 adm-zip modules in `require.cache`) and `check_consumer` returns **0 problems**, with a trace of 72 URLs and none under adm-zip. With a literal `"adm-zip"` in the same `createRequire` call, only the static scan catches it, and the trace still records nothing.

Injection results through the real `check_consumer`:

| Injection | Trace | Static scan | Verdict |
|-----------|-------|-------------|---------|
| none | 0 adm-zip | clean | green, as expected |
| es `import "adm-zip"` | caught | caught | red, AC17 reproduced |
| es `require("adm-zip")` | not seen, smoke import throws ReferenceError | caught | red |
| es `createRequire(...)("adm-zip")` | **not seen** | caught | red, static only |
| es `await import("adm-zip")`, and unawaited | caught | caught | red |
| es string-built `import(...)` | caught | not seen | red, trace only, as the brief expects |
| es string-built `createRequire(...)(...)` | **not seen** | not seen | **green while adm-zip loads** |
| es `import "adm-zip/adm-zip.js"` | caught | not seen | red, trace only |
| es ``import(`adm-zip`)`` | caught | not seen | red, trace only |
| cjs build `require("adm-zip")` | not exercised | caught | red |
| cjs build subpath, template literal, string-built | not exercised | not seen | green (S2) |

**Why it is wrong**: plan decision 8(b) says the trace "proves the node path", AC15 says it "records every resolved URL", and `README.md:311` says "`gate packages` proves that on every run". A CommonJS resolution on the shipped node path is part of that path and is not recorded.
**Evidence**: `variants.py es-stringbuilt-createRequire`, then `probe-loaded.mjs` printing `adm-zip in require.cache: 16`. Replacing the hook with `module.registerHooks({ resolve })` (synchronous, present in node 24, which CI pins) recorded 19 adm-zip URLs on the same consumer.
**Scope and regression**: affects `gate packages`, the `unreachable` exception's credibility, and the three user-facing sentences that cite the proof. Present on the staged tree. Smallest fix: write `TRACE_REGISTER` as a `registerHooks({ resolve(s, c, next) { const r = next(s, c); appendFileSync(trace, r.url + "\n"); return r; } })` module and drop `trace-hook.mjs`, then rerun AC17 adding the `createRequire` string-built injection as a recorded RED. Uncertainty: none for node 24.13. Not checked on other node majors.

### D2, a padded scope is accepted by the advisory gate and drops the row from the proof

**Where**: `scripts/advisory_check.py`, `exception_problems`, the scope check `scope.strip() not in SCOPES`, against `scripts/package_check.py`, `_unreachable_rows` and `consumer_version_problems`, which compare `str(row.get("scope", "")) == "unreachable"` unstripped.
**What**: with `scope = "unreachable "` in `ci/advisories.toml`, `advisory_check.py exceptions` exits 0 and accepts the exception, and `package_check._unreachable_rows()` returns `[]`, so the version check, the trace check and the static scan all run over no package and pass.
**Why it is wrong**: the plan's invariant, "an `unreachable` row is accepted by `advisory_check.py` only if its `proof` names the check `package_check.py` runs, and that check reads the same rows, so the exception and its proof cannot be separated". A one-character edit separates them, and no gate goes red.
**Evidence**: scratch copy of the tracked tree, `sed` on the scope line, `advisory_check.py exceptions` printed `OK: advisory check, exception adm-zip@0.5.17`, `_unreachable_rows()` printed `[]`.
**Scope and regression**: affects the exception and every consumer check. Present on the staged tree. Smallest fix: compare the scope exactly in `advisory_check.py` (`scope not in SCOPES`, no strip, so the padded value is refused with `is not unreachable`), plus a level-3 probe `advisories.exception-scope-padded` substituting `scope = "unreachable "`. Stripping in `package_check.py` instead also closes it, but leaves two readers that must keep agreeing.

## Smells

### S1, the proof can go vacuous with every gate green

**Where**: `scripts/package_check.py`, `check_consumer`, the block that reads `trace-urls.txt` and the three `problems +=` lines.
**What**: an absent or empty trace is accepted. With the hook registration emptied, consumers that do load adm-zip (`es-subpath`, `es-stringbuilt-dynamic`) return 0 problems. Separately, deleting the three `problems +=` lines leaves the census at `OK, 0 watched by nothing` and all 19 new probes green, because the level-1 probes call the pure functions directly and the healthy install has nothing to refuse. The implementer's report records "An empty trace is not a new refusal" as a decision.
**Failure mechanism**: a node change to how `--import` or `register` behaves, or an edit to the hook, silently turns the AC15 half of the proof off. The AC17 RED is a one-off manual record and does not rerun.
**Smallest fix**: refuse a trace that does not contain the resolved `node_modules/dcmjs/build/dcmjs.es.js` URL (it is resolved on every healthy run today), with its own fragment and a level-1 probe. Optionally fold the three calls into one `unreachable_problems(consumer, rows)` so a level-1 probe over a synthetic consumer directory exercises the read-trace, read-lock and walk-files path as well.

### S2, the static scan matches only the exact quoted name

**Where**: `scripts/package_check.py`, `specifier_problems`, the check `f'"{name}"' in text or f"'{name}'" in text`.
**What**: `require("adm-zip/adm-zip.js")` and ``require(`adm-zip`)`` in `dcmjs/build/dcmjs.js` evade both checks. That is the file dcmjs's `require` condition and its `main` field select, so a bundler or tool that resolves through `main` would bundle adm-zip while the smoke import, which takes the `import` condition, never runs that file. In `dcmjs.es.js` the same two forms are caught only by the trace.
**Failure mechanism**: plan decision 8(c) says a quoted name is "the only way a bundler or a resolver could reach it". Subpath and template-literal specifiers are two more ways, and both are statically resolvable by webpack. The implementation matches AC16 as written, which is why this is a smell rather than a defect.
**Smallest fix**: match ``["'`]adm-zip(?:/[^"'`]*)?["'`]`` (escaped per name), with one unit case for each of the subpath and template-literal forms.

## Nitpicks

- **N1**: `scripts/tests/test_advisory_check.py:94`, `test_each_required_field_emptied_is_missing` iterates `A.FIELDS`, the implementation's own tuple, so dropping a field leaves it green. The census's `FIELDS` constant catches that, so coverage holds, but the test should iterate the plan's literal nine-field list (HLD 27.2 R2).
- **N2**: `scripts/advisory_check.py`, the `not isinstance(row, dict)` branch of `exception_problems` is unreachable through `main`, because `read_rows` already filters out non-dict rows.
- **N3**: a consumer's own audit lists `@ocelli/core` and `@ocelli/react` themselves as high, as well as adm-zip and dcmjs. The README sentence "A project's own npm audit will still report it" is true. Saying that the consumer will also see `@ocelli/core` named would spare a reader a surprise.

## Verified clean

- AC1: no tracked `package.json` overrides a closure package, and the root lock still resolves adm-zip at 0.5.17. `override_problems` walks nested keys (planted nested override refused by its probe, with the deletion proved).
- AC2: the `ci/advisories.toml` GHSA set equals the live report's adm-zip set, by the same digest.
- AC3 local half: `gate advisories` exit 0, three `WARN` lines and the `OK` line.
- AC4 to AC9 through both the probes and the `--report` seam over real rows. Critical is refused by membership. A string `via` is skipped, so dcmjs is not refused for adm-zip's advisories, while a dcmjs advisory of its own is refused.
- AC9's network half: an unreachable registry and a missing `npm` both fail closed with `did not produce a report`, which is what plan decision 3 chose.
- AC10 to AC13 probes, and AC20: 17 of 17 refusals proven watched by deletion.
- AC14 to AC17 on the unmodified install: the consumer lock gives adm-zip 0.5.17, 71 trace URLs and none under adm-zip, no other installed script names it. The AC17 static-import RED reproduces with exactly the implementer's two problems.
- AC18: `advisories` is in `GATES`, falls in `--floor` (the arm excludes only named gates), and runs in the CI `frontend` job straight after `npm ci`. `gate ci` OK.
- AC19: the census and runbook are consistent, with 1051 refusals and 0 watched by nothing. The five recorded constants each turn the census red under mutation.
- AC21 and AC22: consumer audit measured. The README override recipe was applied in a real consumer, giving `overridden`, 0 vulnerabilities and a working smoke import. The `use.html` and `packages/core/README.md` sentences are true of today's files. `more.html` has no dependency text.
- The noted CI gap: the `frontend` job's steps on `ab3d11c` are the same as on the staged tree, apart from the new `gate advisories` step, which needs neither `dist` nor wasm. The latest `main` CI run's TypeScript job passed `gate packages` (468 tests, consumer OK) with no wasm build, because `npm run typecheck` builds `dist` first. The gap the implementer describes did not show up in CI. This story neither creates it nor makes it worse.
- Boundary and tier: no product code, wasm, pixel or GPU path touched.

## Limitations and handoff

- AC3's CI half is open until a pushed head runs the `frontend` job. AC23 (the Rust follow-up row) is owed at `/complete-feature`.
- The variants replaced `npm install` with a copy of one real install from the same tarballs, so every variant ran the real `check_consumer` code over identical files. The tsc steps ran each time.
- Python 3.12, which CI pins, was not run locally. 3.14.5 was used.
- The D1 fix was shown with a standalone `registerHooks` script, not as a patch to `package_check.py`.
- Next owner: implementation, for D1, D2, S1 and S2. Then a fresh independent pass 2 over the changed material, replaying the D1 injection table and the padded-scope trigger.
