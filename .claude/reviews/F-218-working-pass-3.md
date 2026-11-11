# F-218 review, pass 3

**Reviewed**: worktree `ocelli-wt-f-218`, branch `work/f-218-xai`, base and HEAD `ab3d11cc7586069aa95bb02762b576acb4dc3fbb`. Staged tree `891e11f71ff0592855983021aba9e2fe46052d1d`, unchanged since pass 1. The pass 1 and pass 2 remediation is unstaged (`MM` on `README.md`, `bin/ocelli.sh`, `ci/guard-probe-budget.json`, `docs/lld/typescript-packaging.md`, `docs/runbooks/guard-verification.md`, `packages/core/README.md`, `scripts/advisory_check.py`, `scripts/guards/catalogue.py`, `scripts/package_check.py`, `scripts/tests/test_advisory_check.py`) plus untracked `scripts/tests/test_unreachable_injections.py`. `tools/oracle/out` is untracked and out of scope. Pin: SHA-1 of `git diff ab3d11c` followed by the untracked injection test, `f1d89c4fee9e23173ea21bf93224f9f8aaa043db`, identical at the start and the end of the pass.
**Author / reviewer**: implementation and remediation by an external coding agent. Reviewer is a fresh independent Claude agent that wrote none of the diff, the plan, pass 1 or pass 2. The only worktree file this pass wrote is this review. Gate runs wrote ignored build output (`packages/*/dist`) as `gate packages` always does. Mutations ran in `scratchpad/f218p3/` (`base/`, a scripts-and-manifests git copy, and `full/`, a git copy of every tracked and untracked file minus `tools/oracle/out`).
**Result**: 3 defects, 0 smells, 4 nitpicks

## Acceptance challenged

- **Pass 2 D1 (docs overclaim)**: the three named sentences are fixed. The adjacent search found one more document making the same claim (D1 below).
- **Pass 2 S1 (npm alias escapes the audit)**: replayed with the real `npm audit --omit=dev --package-lock-only` over a lock where dcmjs depends on `zipper: npm:adm-zip@0.5.17`, then three adjacent forms: top-level `node_modules/zipper`, scoped `node_modules/@x/zipper`, and nested `node_modules/dcmjs/node_modules/zipper`. Closed in all three.
- **Pass 2 S2 (injection test in no gate)**: `gate packages` now runs it, and two level-1 wiring probes exist. Adjacent case: the line removed from `gate packages` (N1).
- **Plausible wrong behaviours this pass rejected by observation**: the alias fix admitting a node outside the closure (M2), the trace check silently dropped from the composition (M4), a hoisted static import skipping the CommonJS require (ordering mutation), and the two smoke wiring lines removed (M6, M7).
- **The consumer override recipe** in both READMEs, executed on a real consumer.

## Evidence

All executed now. node v24.13.0, npm 11.6.2, Python 3.14.5 (unit tests also under 3.13), macOS.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate packages` | worktree as pinned | now | exit 0, injection test `Ran 1 test in 24.5s OK` inside the gate |
| `bin/ocelli.sh gate guards` | worktree | now | exit 0, 417 refusal probes, 37 guards, 44 accept, 81 controls |
| `bin/ocelli.sh gate advisories`, `ci`, `prose` | worktree | now | exit 0 each |
| `guard_census.py`, `--check-runbook` | worktree | now | exit 0 both, 1054 refusals in 78 files, 0 watched by nothing |
| `prose_check.py`, `source_provenance_check.py` | worktree | now | exit 0 both |
| `unittest scripts.tests.test_advisory_check` and the `discover -p` form | worktree | now | 48 tests OK. Python 3.13: OK |
| Real `npm audit` plus `advisory_check.py --report`, unmodified lock | `base/` copy | now | exit 0, `OK: advisory check, exception adm-zip@0.5.17`, three moderate warnings |
| Same, alias `top`, `scoped`, `nested`, with the row | `base/` copy, lock rewritten | now | report names `adm-zip` with the alias node each time. exit 1 `npm adm-zip@0.5.17 is a stale advisory exception` |
| Same, alias forms, row removed | same | now | exit 1, five `adm-zip@0.5.17 GHSA-... has no exception` each time (pass 2 had exit 0 here) |
| M1: `_in_shipped_closure` node branch replaced by `return False` | `base/` copy | now | `test_an_npm_alias_is_in_the_closure_by_its_node` FAILED. Real caller on the `top` alias, no row: `OK: advisory check, no exception` (pass 2 S1 reopened, so the test is the watcher) |
| M2: `_closure_node` loses `_path_package(node) in closure` | `base/` copy | now | `test_a_versions_entry_outside_the_closure_is_not_shipped` FAILED, and the `advisories.outside-closure` snippet input yields `vite@7.0.0 ... has no exception` |
| M3: `unreachable_problems` call deleted from `check_consumer` | `full/` copy | now | `guard_probe.py --only packages.unreachable-trace`: FAIL, "exited 0. The guard it aims at did not fire" |
| M4: `*resolved_package_problems(...)` dropped from `unreachable_problems` | `full/` copy, then module patch through the real injection test | now | all six `packages.*` level-1 probes OK, `test_advisory_check` OK. Injection test FAILED on all 12 trace-channel cases |
| Ordering: `SMOKE_SCRIPT` switched to a hoisted static import | module patch through the real injection test | now | FAILED on `es-require`: the ES throw skips the require, `does not record dcmjs/build/dcmjs.js`. Unpatched control: OK |
| M5: injection test removed from the `packages` arm | `full/` copy | now | `ci_floor_check.py` 0, `guard_census.py` 0, `--check-runbook` 0 (N1) |
| M6: `createRequire(...)("dcmjs")` line removed from `SMOKE_SCRIPT` | `full/` copy | now | `packages.smoke-requires-dcmjs` control exits 1, harness FAIL |
| M7: `registerHooks` renamed away in `TRACE_REGISTER` | `full/` copy | now | `packages.hook-register-hooks` control exits 1, harness FAIL |
| Real consumer from `check_consumer` | tarballs packed from the worktree | now | `[]`. Trace 73 URLs, both dcmjs builds. `npm audit --omit=dev`: high 4, `@ocelli/core`, `@ocelli/react`, `adm-zip`, `dcmjs` |
| Consumer adds `"overrides": {"adm-zip": "0.6.1"}`, `npm install` | same consumer | now | adm-zip 0.6.1 installed, audit total 0, `node --import ./trace.mjs smoke.mjs` prints `ok 0.1.0` |
| `npm view adm-zip versions` | registry | now | 0.6.1 exists |
| dcmjs facts | installed `node_modules/dcmjs/package.json` | now | 0.52.2, `files` = `build`, `generate/dictionary.mjs`, `README.md`, `License.txt`. `adm-zip: 0.5.17` under dependencies. Only `package.json` names adm-zip |
| `_specifier_pattern("adm-zip")` over 12 strings | worktree | now | bare, subpath, backtick, `../`, `../../adm-zip/adm-zip.js`, `./node_modules/`, `../../node_modules/` match. `adm-zipper`, `my-adm-zip`, `adm-zip.js`, bare `node_modules/adm-zip`, absolute path do not. Matches every doc sentence about the scan |

## Defects

### D1, `docs/lld/guards.md` still says the exception is proved

**Where**: `docs/lld/guards.md:625-626`, "The one recorded exception is adm-zip 0.5.17, scope unreachable, proved by `gate packages` on a real consumer install."
**What**: this is the claim pass 2 D1 removed from `README.md`, `packages/core/README.md` and `docs/lld/typescript-packaging.md`, which now say "checks" and name the limit. The guards LLD was added in the same change and still says "proved".
**Why it is wrong**: microscope section 3. The gate checks a bounded thing. A string-built load in code the smoke run does not execute is outside it, and the other three documents now say so. Pass 2 measured six such edits that leave `check_consumer` at 0 problems.
**Evidence**: `git diff ab3d11c | grep '^+.*proved'` gives only this line. The `--report` replays and pass 2's evasion table give the bound.
**Scope and regression**: one LLD sentence, present on the pinned tree. Smallest fix: "checked by `gate packages` on a real consumer install, within the bound `typescript-packaging.md` states". Prose only.

### D2, the `PROOFS` comment counts four where there is one

**Where**: `scripts/guards/catalogue.py:13109`, "# F-218. PROOFS is recorded with the four the plan names."
**What**: `PROOFS = ("packages-consumer-trace",)` holds one name. The plan names one proof (`.claude/plans/F-218-design.md:233`, `PROOFS = ("packages-consumer-trace",)`), and the comment sits above five `Constant` rows, not four.
**Why it is wrong**: microscope section 3, a false factual sentence in a code comment. It is in the staged material that passes 1 and 2 reviewed and did not flag, so it is new to the record and not a remediation regression.
**Evidence**: `grep -n '^PROOFS' scripts/advisory_check.py`, plan line 233, `grep -c 'Constant("advisories"' scripts/guards/catalogue.py` gives 5.
**Scope and regression**: comment only. Smallest fix: delete "PROOFS is recorded with the four the plan names." and keep the widening sentence.

### D3, "leaves the probe green" says the opposite of what was measured

**Where**: `scripts/guards/catalogue.py:10690`, the `packages.unreachable-trace` note, "deleting unreachable_problems from it leaves this probe green", and `:10725`, the `packages` limit, "removing that call leaves the probe green".
**What**: in this file "left the probe green" means a mutation the probe did not detect. See `:4970` in the base ("dropping `bool(arm) and` left that probe green") and `:12421` ("left the probe green with three shapes unguarded"). Read that way, both F-218 sentences say the deletion is not detected. It is detected (M3: the harness reports FAIL, "The guard it aims at did not fire"). Four lines later the same limit says removing the dcmjs require "leaves those controls failing", which is the other vocabulary for the same direction.
**Why it is wrong**: microscope section 3. A reader following the file's own convention learns that this probe does not watch the call, which is false. Pass 2 recorded this as N2, a nitpick, on the reading that "green" meant the guard accepted. The base file's two uses settle it the other way, and the remediation then added a sentence in the opposite vocabulary right beside it, so I am raising it to a defect.
**Evidence**: M3 in the table, plus `git show ab3d11c:scripts/guards/catalogue.py | grep -n 'probe green'`.
**Scope and regression**: catalogue prose only. Smallest fix: "so removing that call fails this probe", the same wording in the limit, and "fail" in place of "leave ... failing" so both sentences use one vocabulary.

## Smells

None.

## Nitpicks

- **N1, unwiring the injection test goes unnoticed.** With the `test_unreachable_injections.py` line removed from the `packages` arm (M5), `ci_floor_check.py`, `guard_census.py` and `--check-runbook` all exit 0. Today the test is the only watcher of two things: the trace check's place in `unreachable_problems` (M4, every level-1 probe and unit test stays green) and the smoke ordering. Most gate-arm unittest lines in this repository have the same exposure, so this does not block. Cheapest hardening: one `test_advisory_check` case giving `unreachable_problems` a consumer directory whose trace holds a `node_modules/adm-zip/` URL. That closes M4 at level 1.
- **N2**: the `packages` guard's `refuses` text ends "A resolve trace that does not record the dcmjs ES entry is refused as well." The CommonJS entry is refused too (`trace_entry_problems`, unit-tested). The sentence is incomplete, not false.
- **N3**: `SECTIONS` and `REPORT_VERSION` in `scripts/advisory_check.py:59-60` follow `scope_is_unreachable` with no blank line, so they read as part of the function's block.
- **N4 (pass 2 N1, carried)**: `DCMJS_TRACE_ENTRIES` is hard-coded to dcmjs while the rows are general. That is fine for one row.

## Verified clean

- **Pass 2 D1 closed for its three named sites.** `README.md:309-325`, `packages/core/README.md:16-31` and `docs/lld/typescript-packaging.md:174-202` say "checks", drop "every resolution", say "on the main thread" and state the run-time-specifier bound. Every factual sentence in them was executed: the regex reach (12 strings), the trace contents (73 URLs, both builds), the consumer audit (high 4, the four names), the override recipe (0.6.1 exists, installs, audit 0, smoke ok), dcmjs's `files` and dependency declaration, and the injection test running in `gate packages`. `use.html:241` is true. `more.html` carries no dependency text.
- **Pass 2 S1 closed.** The fix enters a vulnerability through its node path. A real npm report over three alias forms is refused with no row and stale with the row. M1 shows the unit test watches the fix and that the real caller depends on it. M2 shows the outside-closure half is watched. Nodes entered through an alias are filtered to closure nodes, so an alias cannot pull a dev-only node in.
- **Pass 2 S2 closed.** `gate packages` runs the injection test, which CI runs at `.github/workflows/ci.yml:324`. The two named mutations are also caught at level 1 (M6, M7), and `trace_entry_problems` independently refuses a missing `dcmjs.js`. The injection test catches M4 and the ordering mutation, which nothing else does. The unpatched control is green.
- **The `advisories` gate**: in the floor (excluded by no name in `--floor`), invoked by CI at `ci.yml:313`, its unit test runs in the gate, and its five constants are recorded in the budget with `constants_count` 50 and `gates_declared` 34. Floor prose in `docs/lld/guards.md` other than D1's sentence matches the code.
- The catalogue's advisory snippets and level-3 probes ran green in `gate guards`. The runbook is regenerated (`--check-runbook` 0).
- No Rust, wasm, pixel, GPU or boundary code is touched, so sections 1 and 5 of the checklist do not apply.

## Limitations and handoff

- `gate guards-deep` was not re-run this pass. Pass 2 ran it green on the same staged tree, and the deep-only probes do not touch the remediated lines beyond what `gate guards` covers.
- M5 checked the census and the CI floor checker, not a full `gate guards` run on the mutated copy.
- The injection mutations patched the loaded module rather than the file, because the full copy has no `node_modules`. The patched names are the module globals that `check_consumer` and `unreachable_problems` resolve at call time.
- Python 3.12, which CI pins, is not installed here. 3.13 and 3.14 ran.
- The consumer-side npm alias (dcmjs requiring an alias directory) is still invisible to `gate packages` alone. It is caught only because the same alias appears in the root lock and turns the row stale under `gate advisories`, which pass 2 recorded and this pass confirmed through the real audit.
- The remediation is unstaged. Staging it produces a new tree, and that tree needs its own verification receipt.
- Next owner: implementation, for D1 to D3, which are three prose edits. Then a fresh independent pass 4 over the changed lines.
