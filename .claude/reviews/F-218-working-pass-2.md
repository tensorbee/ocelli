# F-218 review, pass 2

**Reviewed**: worktree `ocelli-wt-f-218`, branch `work/f-218-xai`, base and HEAD `ab3d11cc7586069aa95bb02762b576acb4dc3fbb`. Staged tree `891e11f71ff0592855983021aba9e2fe46052d1d`, which is the same tree pass 1 reviewed. The pass 1 remediation is all unstaged (`MM` on `README.md`, `ci/guard-probe-budget.json`, `docs/lld/typescript-packaging.md`, `docs/runbooks/guard-verification.md`, `packages/core/README.md`, `scripts/advisory_check.py`, `scripts/guards/catalogue.py`, `scripts/package_check.py`, `scripts/tests/test_advisory_check.py`), plus the untracked `scripts/tests/test_unreachable_injections.py`. `tools/oracle/out` is untracked and out of scope. Pin: SHA-256 of `git diff ab3d11c` plus every untracked non-ignored file except this review, `88f170d4d7a68115db71b2e7dd67a9f2bd80af8c7c716554ae12cac924157216`, identical at the start and the end of the pass.
**Author / reviewer**: implementation and remediation by an external coding agent (its reports are in the session scratchpad, not tracked). Reviewer is a fresh independent Claude agent that wrote none of the diff, none of the plan and not pass 1. The only file this pass wrote in the worktree is this review. All experiments ran in `scratchpad/review-f218-2/`, including a full copy of the reviewed tree (`full/`, its own git repository) for the deletion and mutation runs.
**Result**: 1 defect, 2 smells, 2 nitpicks

## Acceptance challenged

- **Pass 1 D1 (CommonJS blind trace)**: replayed all 14 injections through the real `check_consumer` with a fresh template built from the current `package_check.py`, not by running the standing test. All 14 closed (table below).
- **Pass 1 D2 (padded scope)**: replayed with ten spellings. All closed.
- **Pass 1 S1 (vacuous proof)**: `trace_entry_problems` and the single `unreachable_problems` call each proved watched by deletion.
- **Pass 1 S2 (exact-name scan)**: the widened regex is watched by `test_advisory_check.py`, which `gate advisories` runs.
- **Twenty-four new evasion attempts** of this pass's own, each classified by channel. Plausible ones are caught, implausible misses are honest limits, and the docs overclaim against those limits (D1).
- **AC4 against an aliased runtime dependency**, derived from npm's own lock and audit shapes, run with a real `npm audit` (S1).
- **Whether the D1 fix itself is guarded**, by mutating it away and running every standing check (S2).

## Evidence

All executed now, node v24.13.0, npm 11.6.2, Python 3.14.5, macOS.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `classify.py` over pass 1's 14 variants | real `check_consumer`, tarballs packed from the worktree, template consumer from one real `npm install` | now | green: 0 problems, 73 trace URLs, `dcmjs.es.js` and `dcmjs.js` both recorded. Every other case red, channels in the table below |
| `evasions.py`, 24 new cases | same template | now | table below |
| `npm audit --omit=dev --package-lock-only --json` over a scratch lock with `node_modules/zipper` (`name: adm-zip`) and dcmjs depending on `zipper` | scratch copy of `scripts/`, `ci/`, manifests, lock | now | report names `adm-zip` with `nodes: ["node_modules/zipper"]`. `advisory_check.py --report`: with the row, exit 1 `stale advisory exception`. With the row removed, **exit 0 `OK: advisory check, no exception`** (S1) |
| D2 spellings, `advisory_check.py exceptions` and `_unreachable_rows()` | scratch tree, scope line substituted | now | `unreachable`: exit 0, 1 row. `Unreachable`, `UNREACHABLE`, trailing tab, leading tab, trailing space, Cyrillic `а`, trailing NBSP, trailing ZWSP, soft hyphen: each exit 1 `is not unreachable`, 0 rows |
| `deletions2.py`, 21 deletions over 19 probes | `full/` copy | now | every probe red under its deletion, including `return []` in `trace_entry_problems`, `pass` for the `unreachable_problems` call, and `value.strip() in SCOPES`. Restored: 19 probes OK, 12 controls green, `git status` clean |
| Smoke `createRequire(import.meta.url)("dcmjs")` line deleted | `full/` copy, harness pointed at it | now | `cjs-stringbuilt` 0 channel hits while adm-zip loads. The four `packages.unreachable-*` probes and `test_advisory_check.py` all still OK (S2) |
| Specifier regex reverted to the bare name, and separately backtick dropped | `full/` copy | now | `test_advisory_check.py` FAILED (5 and 3 failures) |
| Consumer `npm audit --omit=dev --json` | unmodified template consumer | now | high 4: `@ocelli/core`, `@ocelli/react`, `adm-zip`, `dcmjs` |
| `grep -i 'adm-zip\|admzip\|adm_zip'` and a `"adm"` fragment search over installed dcmjs builds, `dictionary.mjs`, `@ocelli/*/dist` | template consumer | now | no hit |
| `bin/ocelli.sh gate` advisories, packages, guards, guards-deep, prose, provenance, ci, lint, types, backlog | worktree as pinned | now | all exit 0. guards: 415 refusal probes, 79 controls. guards-deep: 465 refusal probes, 83 controls, `packages.unreachable-trace` and `advisories.exception-scope-padded` red. ci: 28 floor gates invoked |
| `guard_census.py`, `--check-runbook` | worktree | now | exit 0 both, 1052 refusals, 0 watched by nothing |
| `unittest -p test_advisory_check.py` | worktree | now | 45 tests OK |
| `unittest -p test_unreachable_injections.py` | worktree | now | 1 test OK, 22.9s |

### Pass 1 injections, replayed

| Case | Trace | Scan | Verdict |
|------|-------|------|---------|
| green | - | - | green, 0 problems |
| es-static-import, es-createRequire, es-dynamic-await, es-dynamic-noawait, es-subpath, es-template | caught | caught | red |
| es-require | not loaded, smoke throws | caught | red |
| es-stringbuilt-dynamic, es-stringbuilt-createRequire | caught | - | red, trace only (pass 1 D1 closed) |
| cjs-require, cjs-subpath, cjs-template | caught | caught | red (pass 1 S2 closed) |
| cjs-stringbuilt | caught | - | red, trace only |

### New evasion attempts

`SB` is `["adm","zip"].join("-")`.

| Case | Trace | Scan | Verdict and plausibility |
|------|-------|------|-------------------------|
| cjs `module.require(SB)` | caught | - | red |
| cjs `module.constructor._load(SB, module)` | caught | - | red |
| cjs `eval("req"+"uire")(SB)` | caught | - | red |
| cjs `process.mainModule.require(SB)`, unguarded | - | - | red, smoke throws (`mainModule` is undefined under an ESM entry) |
| cjs `if (process.mainModule) process.mainModule.require(SB)` | - | - | **green**. Loads only under a CommonJS entry. Implausible, honest limit |
| cjs `require.resolve(SB)` then read and `new Function` | - | - | red only because the smoke throws on the module's own relative require. A working variant is implausible, honest limit |
| es `globalThis["req"+"uire"]` | - | - | green, but `require` is not a global, so nothing loads. Not an evasion |
| es / cjs `package.json` `"imports": {"#zip": "adm-zip"}`, then `import("#zip")` / `require("#zip")` | caught | - | red |
| symlinked `node_modules/zipper -> adm-zip`, `require("zipper")` | caught (realpath) | - | red |
| npm alias directory `node_modules/zipper` (lock `name: adm-zip`), `require("zipper")` | - | - | **green**, 19 adm-zip modules loaded from `zipper/`. On the consumer side neither catches. On the repository side the row goes stale, so `gate advisories` is red while the row exists. See S1 for the case with no row |
| es `new Worker(..., {eval: true})` requiring `SB` | - | - | **green**. Measured: the worker inherits `--import ./trace.mjs` in `execArgv`, has 16 adm-zip modules in `require.cache`, and the trace records 0. Implausible at import time in a library, honest limit |
| es `child_process.execFileSync(node, ["-e", require(SB)])` | - | - | **green**. The child gets no hook. Implausible, honest limit |
| es `import("adm-zip/package.json", {with: {type: "json"}})` | caught | caught | red |
| es same, string-built | caught | - | red |
| es `export function f() { return import("adm-zip"); }`, never called | - | caught | red |
| es same, string-built | - | - | **green**. A lazy load in a function the smoke never calls. The most plausible miss, but a run-time-built specifier in shipped code is still unlikely, honest limit |
| es `import "../../adm-zip/adm-zip.js"` | caught | - | red |
| `dcmjs.min.js` `require("../../adm-zip/adm-zip.js")` | - | - | **green**. No condition selects `dcmjs.min.js`, and a sibling-relative path is implausible. Honest limit |
| `dcmjs.min.js` `require("adm-zip")` | - | caught | red |
| `generate/dictionary.mjs` (export `./dictionary`) string-built `await import(SB)` | - | - | **green**. The smoke never imports `dcmjs/dictionary`. Implausible, honest limit |
| `dictionary.mjs` literal `await import("adm-zip")` | - | caught | red |
| es `import.meta.resolve(SB)` | caught | - | red (resolve only, a conservative refusal) |
| es `setTimeout(() => import(SB), 50)` | caught | - | red |

Every plausible way for a shipped file to load adm-zip that I tried (any literal specifier anywhere, any load at import time on the main thread through either dcmjs entry) is caught. The misses share one property: the specifier is built at run time *and* the load runs in code the smoke run does not execute on its main thread, or under a package name other than the excepted one.

## Defects

### D1, three documents say the gate proves more than it checks

**Where**: `README.md:311`, "`gate packages` proves that on every run", where "that" is "Nothing in `@ocelli/core` or in dcmjs's published files loads adm-zip". `packages/core/README.md:18`, the same sentence, and line 19, "It records every ESM and CommonJS resolution". `docs/lld/typescript-packaging.md:179-180`, "The recorded answer is a proof that no shipped file loads `adm-zip`".
**What**: the gate checks a bounded thing: no main-thread resolution under `node_modules/adm-zip/` during an import of `@ocelli/core` plus a `require` of `dcmjs`, the consumer lock version, and no quoted specifier in a `.js`, `.mjs` or `.cjs` file. Six edits to published dcmjs files load adm-zip and leave `check_consumer` at 0 problems (evasion table: lazy string-built import, string-built import in `dictionary.mjs`, a worker, a child process, a guarded `mainModule.require`, and a sibling-relative require in `dcmjs.min.js`). The README's later sentence "names that package as a module specifier in any quote, with or without a subpath" is also wider than the regex, which does not match `"../../adm-zip/adm-zip.js"`. "Records every resolution" is false for a worker thread, measured.
**Why it is wrong**: microscope section 3, a claim in prose that is false is a defect. The brief for this pass asked that an honest limit not be overclaimed. The underlying fact, that no shipped file names or loads adm-zip today, is true (grep clean, trace clean). Only the word "proves" and the "every" are wrong.
**Evidence**: `evasions.py` rows marked green above, each with the trace and the scan read through the real `check_consumer`. The worker case was confirmed with `require.cache` inside the worker (16 entries) against 0 trace lines.
**Scope and regression**: the README, the package README a consumer reads on npm, and the LLD. `use.html` says only "nothing ... loads", which is true, and needs no change. Smallest fix: replace "proves that" and "a proof that" with "checks that" and "a check that", drop "every" in `packages/core/README.md:19`, and add one sentence to each: "A load whose specifier is built at run time, in code the smoke run does not execute on its main thread, is outside the check." No code change.

## Smells

### S1, an npm-aliased runtime dependency escapes the audit gate

**Where**: `scripts/advisory_check.py:256`, `if package not in closure or not isinstance(vuln, dict): continue`, where `closure` comes from `licence_check.npm_runtime_closure()`, which names packages by dependency key (the alias), while npm audit names the vulnerability by the real package name.
**What**: with a lock whose runtime closure installs adm-zip as `"zipper": "npm:adm-zip@0.5.17"`, the real `npm audit` reports `adm-zip` with `nodes: ["node_modules/zipper"]`. `closure` contains `zipper`, not `adm-zip`, so the high advisories are skipped. dcmjs's own entry carries only string `via`, which is skipped by design. With no exception row the gate prints `OK: advisory check, no exception` and exits 0.
**Failure mechanism**: AC4 says a high advisory on a closure package with no row is refused. No alias is in the lock today (0 aliased lock entries, measured), which is why this is a smell and not a defect. npm aliases are common in the wider ecosystem, for example `"string-width-cjs": "npm:string-width@^4.2.0"` from `@isaacs/cliui`, so one arriving in the 12-package runtime closure through a dependency bump is a realistic mechanism. The consumer-side `npm-alias-dir` evasion has the same root, and today it is caught only because the existing row turns stale.
**Smallest fix**: judge a vulnerability as in the closure when its package is in `closure` or any of its `nodes` is a key of `versions` (which `closure_facts` already builds from closure node paths). One unit case with the alias report shape above. Optionally have `_pair_present` accept a node whose lock `name` equals the row's package.

### S2, the D1 fix is watched only by a test no gate runs

**Where**: `scripts/tests/test_unreachable_injections.py` (untracked), against `bin/ocelli.sh`, whose every `unittest discover` names its `-p` pattern, none of them this file, and `.github/workflows/ci.yml`, which does not name it either.
**What**: deleting the smoke's `createRequire(import.meta.url)("dcmjs")` line (`scripts/package_check.py:383`) turns `cjs-stringbuilt` back into a green consumer that loads adm-zip, and the four `packages.unreachable-*` probes and `test_advisory_check.py` all stay green. Reverting `registerHooks` to `module.register` has the same exposure, because `trace_entry_problems` only needs the ESM entry, which the async hook still records. The injection test catches both, but it runs only when someone runs it by hand. The implementer's report calls it "the standing injection test".
**Failure mechanism**: microscope section 4. A later edit to the smoke script or the hook silently reopens pass 1 D1 with every gate green.
**Smallest fix**: run the file from `gate packages` after `package_check.py` (it takes about 23s and reuses one install), or add a level-1 probe or unit test that asserts the smoke text requires `dcmjs` and `TRACE_REGISTER` calls `registerHooks`. A trace-entry check on `node_modules/dcmjs/build/dcmjs.js` as well as `dcmjs.es.js` would also catch the first mutation, since the healthy trace records it.

## Nitpicks

- **N1**: `DCMJS_ES_ENTRY` is hard-coded to dcmjs while the rows are general. A future unreachable row under another dependency gets a trace-entry check that says nothing about that dependency. Fine for the one row today.
- **N2**: the `packages` guard's `limit` text says removing the call "leaves the probe green". It is consistent with this repository's convention (the guard accepts, so the probe fails), but it reads as the opposite to a newcomer. "leaves the guard green and the probe failing" would be unambiguous.

Pass 1's N1 to N3 were not re-examined beyond N3, which the remediation addressed (the docs now name `@ocelli/core` and `@ocelli/react`, confirmed by the consumer audit).

## Verified clean

- Pass 1 D1 closed: all 14 injections red or green as derived, through a fresh template, independently of the standing test.
- Pass 1 D2 closed: `scope_is_unreachable` is the single comparison both scripts call, exact match. Ten spellings refused with 0 rows proved. Other row fields are stripped identically by both readers, so they cannot separate.
- Pass 1 S1 closed: an empty or absent trace is refused, and the probe calls the real `check_consumer`, so deleting either the refusal or the call is seen.
- Pass 1 S2 closed for quotes, backticks, subpaths and `./node_modules/` prefixes, with the unit test proved by two mutations.
- All 17 pass 1 deletion cases and the 2 new refusals re-proved watched by deletion in a full scratch copy.
- The `--import ./trace.mjs` argument is watched by the trace probe's control, because the stub writes the trace only for that argument vector.
- CI pins node 24 at `.github/workflows/ci.yml:93` and `:301`, no `engines` field exists, and `registerHooks` absent would fail the smoke and so fail closed.
- Consumer audit and the README override recipe statement (high 4, and the `overrides` route) match measurement. `use.html` sentence is true. `more.html` has no dependency text.
- Every requested gate and test is green on the pinned tree (Evidence table).
- No product code, wasm, pixel or GPU path is touched.

## Limitations and handoff

- Injection variants reuse one real install by copying it, as pass 1 did. Typecheck ran in the replay harness (pass 1's harness does not stub it) and was stubbed in the `full/` mutation run, which lacks `node_modules`, so those runs show an extra `typescript is not installed` problem that is unrelated to the channels.
- Python 3.12, which CI pins, was not run. Other node majors were not tried.
- AC3's CI half and AC23 remain open as pass 1 recorded.
- The remediation is unstaged. Whoever stages it produces a new tree, and that tree needs its own verification receipt.
- Next owner: implementation, for D1, S1 and S2. Then a fresh independent pass 3 over the changed material.
