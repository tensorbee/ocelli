# F-218, Dependency advisories in shipped packages

**Status**: approved
**Epic ref**: E38.28
**Sprint**: S46
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "Dependency advisories in shipped packages: adm-zip below 0.6.1 reaches the published @ocelli/core runtime through dcmjs, resolved by a patched override or a proof that no shipped path reaches it, with an audit gate over shipped dependencies". Layer TS, depends on F-217, milestone M20.

`allocation.json` notes: "npm audit on 2026-11-05 reports GHSA-xcpc-8h2w-3j85 and GHSA-7q85-xj36-vmfc (high, memory exhaustion from a crafted ZIP) and GHSA-vwc7-r8mq-g2x9 (moderate) for adm-zip 0.5.17 under dcmjs 0.52.2. The audit's suggested fix downgrades dcmjs a major version."

The lead's brief adds that the audit's `fixAvailable` (dcmjs 0.29.6, semver-major downgrade) is not acceptable, and that F-217's patterns (`scripts/licence_check.py`, `ci/licences.toml`, the `bin/ocelli.sh` gate rows) are reused rather than duplicated. F-217 is read from branch `work/f-217-grok` at `c82f7e9`, which is "implementation by an external coding agent, before review", so every F-217 name this plan reuses is re-checked against the integrated tree before implementation starts.

## Normative source, transcribed

### HLD decision D10, `docs/hld/11-decision-log.md`

```text
| D10 | Keep dcmjs for SR generation | Reimplement TID 1500 in Rust | Standards conformance for near-zero cost |
```

D10 rules out the cheapest-looking fix, dropping dcmjs. This plan keeps it.

### HLD section 7, `docs/hld/07-concurrency-and-typescript.md`

The TypeScript shell owns, verbatim: "ONNX and SAM-backed AI tools, and the dcmjs bridge for TID 1500 structured reports". dcmjs is therefore a runtime dependency of the published shell by design, and is not a dev tool that could be moved out of `dependencies`.

### HLD 27.2, rule R2, `docs/hld/24-agent-code-standards.md`

"Tests derive from the spec or the oracle, never from reading the implementation". It governs the probes and unit tests below, whose expected refusals derive from this plan's policy table and npm's documented report shape, not from the checker's code.

### npm 11.6.2 documentation, shipped with the installed CLI

`docs/content/configuring-npm/package-json.md`, section "overrides", verbatim:

"Overrides are only considered in the root `package.json` file for a project. Overrides in installed dependencies (including [workspaces](/using-npm/workspaces)) are not considered in dependency tree resolution. Published packages may dictate their resolutions by pinning dependencies or using an [`npm-shrinkwrap.json`](/configuring-npm/npm-shrinkwrap-json) file."

`docs/content/configuring-npm/npm-shrinkwrap-json.md`, verbatim: "It's strongly discouraged for library authors to publish this file, since that would prevent end users from having control over transitive dependency updates."

`docs/content/commands/npm-audit.md`, verbatim: "The audit command submits a description of the dependencies configured in your project to your default registry and asks for a report of known vulnerabilities." And: "By default, the audit command will exit with a non-zero code if any vulnerability is found." And: "If any `--omit` options are specified ... then packages will be omitted from the submitted payload as appropriate."

The first paragraph settles the override question. A root `overrides` entry changes this repository's lockfile and never reaches a consumer of the published `@ocelli/core`.

## Facts measured for this plan

Measured on 2026-11-05 on `sprint/s46` with node v24.13.0 and npm 11.6.2.

1. **The dependency path.** `npm ls adm-zip --all` gives `@ocelli/core@0.1.0 -> dcmjs@0.52.2 -> adm-zip@0.5.17`. `node_modules/dcmjs/package.json` declares `adm-zip` under `dependencies`, pinned exactly at `0.5.17`, not under `optionalDependencies` or `peerDependencies`. `packages/core/package.json` pins `dcmjs` exactly at `0.52.2`. dcmjs `latest` on the registry is still `0.52.2` and still pins `adm-zip 0.5.17`.
2. **dcmjs's only use of adm-zip is its own test helper.** In the dcmjs source repository (MIT, in bounds), shallow clone of `main` at `aee06e4` (2026-10-19), `git grep adm-zip` finds `package.json` (the `dependencies` entry), `pnpm-lock.yaml`, and `test/testUtils.js`, whose `unzip` helper does `new AdmZip(zipFilePath)` then `zip.extractAllTo(targetPath, true)` to unpack downloaded test data. dcmjs's published `files` are `build`, `generate/dictionary.mjs`, `README.md` and `License.txt`, so `test/` never ships. The declaration under `dependencies` is an upstream packaging error.
3. **No shipped dcmjs file names adm-zip.** `grep -i 'adm-zip\|admzip'` over `build/dcmjs.es.js`, `build/dcmjs.js`, `build/dcmjs.min.js` and `generate/dictionary.mjs` finds nothing. The ES build carries no top-level `import` statement and one `export` line, and neither build contains `import(` or a live `require(` (the four `require('pako')` hits are inside doc comments). Both builds are self-contained bundles.
4. **Importing @ocelli/core never resolves adm-zip.** A `node:module` `register` resolve hook printing every resolved URL, run over `await import('./dist/index.js')` in `packages/core`, resolves only `packages/core/dist/*` and `node_modules/dcmjs/build/dcmjs.es.js`. `require('dcmjs')` leaves only `dcmjs/build/` in `require.cache`. `packages/core/src` and `packages/react/src` contain no dynamic `import(`.
5. **No install-time code.** The root lock entry for `node_modules/adm-zip` has no `hasInstallScript`, and adm-zip's `scripts` are `test` and formatting only.
6. **There is no bundler.** `@ocelli/core` builds with `tsc --build` (`docs/lld/typescript-packaging.md`, "There is no bundler"). `npm pack --dry-run` lists 280 entries, all under `dist/` plus `LICENSE-APACHE`, `LICENSE-MIT`, `README.md` and `package.json`, so dcmjs is a dependency the consumer resolves and not code this package carries.
7. **The advisory set is larger than the backlog row says.** `npm audit --omit=dev --package-lock-only --json` (exit 1, `auditReportVersion` 2) reports two vulnerable packages, `adm-zip` (eight advisories) and `dcmjs` (`via: ["adm-zip"]`, no advisory of its own). High: GHSA-xcpc-8h2w-3j85, GHSA-7q85-xj36-vmfc, GHSA-rcw4-f5rp-g42v, GHSA-j5f4-cc29-5x44, GHSA-8238-w5pm-2374. Moderate: GHSA-vwc7-r8mq-g2x9, GHSA-p634-w6r4-rjp2, GHSA-c6fg-446q-cg94. No critical. Every range ends at `<=0.6.0` or `<0.6.1`, so adm-zip 0.6.1 (MIT, `engines.node >=14.0`, published 2026-09-11) clears all eight. `fixAvailable` is `dcmjs@0.29.6` with `isSemVerMajor: true`.
8. **The full audit is noisier and out of scope.** Without `--omit=dev` the root audit reports 15 vulnerable packages (6 high, 9 moderate), and every one outside `adm-zip` and `dcmjs` is dev tooling (`vite`, `vitest`, `typescript-eslint`, `zod`, `source-map-js`, `brace-expansion`, `minimatch`).
9. **The published runtime closure** walked from `packages/core` and `packages/react` `dependencies` over the root lock, the walk `licence_check.npm_runtime_closure()` performs on the F-217 branch, is twelve names: `@babel/runtime-corejs3`, `@ocelli/core`, `adm-zip`, `core-js-pure`, `dcmjs`, `gl-matrix`, `iota-array`, `is-buffer`, `lodash.clonedeep`, `loglevel`, `ndarray`, `pako`. The existing root `overrides` (`react`, `react-dom`) name nothing in it, because `@ocelli/react` takes React as a peer.
10. **Rust is clean of vulnerabilities today.** `cargo deny --locked --format json check advisories` with cargo-deny 0.19.0 over the F-217 worktree reports one diagnostic, `RUSTSEC-2021-0153`, "`encoding` is unmaintained", reached through `dicom-encoding 0.10.0` from `ocelli-dicom`. It is an unmaintained notice, not a vulnerability. Ten of the twelve crates are publishable (`ocelli-native` and `ocelli-wasm` carry `publish = false`).
11. **`npm audit` needs the registry.** It submits the tree to the Bulk Advisory endpoint. `--package-lock-only` removes the need for `node_modules` and not the need for the network. The floor gate `packages` already installs tarballs from the registry, so a floor gate that needs the network has precedent.
12. **The guard sandbox copies `git ls-files` only** (`scripts/guards/sandbox.py`), so a probe sees the tracked lock and manifests, and has no `node_modules` and no audit report.

## What the specification does not cover

The HLD says nothing about advisories, audit gates or transitive dependency policy. Every decision below is this plan's.

1. **Resolution route: the proof, not an override.** No `overrides` entry is added for adm-zip. An override would change the repository's lockfile, turn the in-repo audit green, and leave every consumer of the published package with exactly what they get today (npm documentation above), so it is a green gate over a tree nobody ships. The resolution is a recorded, mechanically re-proved exception: adm-zip 0.5.17 is installed for consumers and is never loaded by any shipped entry point. (Q1, answered.)
2. **The gate audits the lock, and refuses an override that would make the lock lie.** `npm audit --omit=dev --package-lock-only --json` at the root, filtered to the published runtime closure. Because an override would make the lock disagree with what a consumer resolves, any root `overrides` key, at any nesting depth, naming a closure package is refused. The two existing keys name nothing in the closure (fact 9).
3. **Live audit in the floor, failing closed.** The audit runs in the floor gate `advisories`, with network, like `packages`. A missing, unparseable or erroring report, or an `auditReportVersion` other than 2, is a failure and never a skip. This is the one floor gate whose verdict can change without a tree change, because the advisory database moves. That is the property an advisory gate exists for, and it is written down here so nobody mistakes it for flakiness. (Q2, answered.)
4. **Threshold.** `high` and `critical` are refused unless excepted. `moderate`, `low` and `info` print a `WARN` line naming the advisory and do not fail. A severity outside those five fails closed.
5. **Exceptions live in a new `ci/advisories.toml`, not in `ci/licences.toml`.** F-217's file refuses any exception inside the published runtime closure, which is the exact place this story's exception sits, and its `licence` field and three scopes mean something else. The row shape follows F-217's (`reason`, `scope`, `decided`, `decided_by`, plus identity), and the validation pattern is the same: missing fields, unknown scope and stale rows are refused.
6. **One row per package and version, listing its advisories.** Fields: `ecosystem`, `package`, `version`, `advisories` (GHSA ids), `scope`, `proof`, `reason`, `decided`, `decided_by`. A high or critical advisory on that package that the row does not list is refused, so a new advisory forces a fresh decision. A listed advisory the report no longer carries is refused as stale, so the list cannot rot. A row whose `(package, version)` is not in the published closure of the lock is refused as stale, which is what a dcmjs release moving to adm-zip 0.6.1 produces. All eight current advisories are listed, so the decision record is complete. (Q3, answered.)
7. **One scope, `unreachable`, with a mechanical proof.** Its meaning: the package is installed for consumers and no shipped entry point loads it. The `proof` field must be `packages-consumer-trace`, the name of the check below, and anything else is refused. A row whose package carries `hasInstallScript` in the lock is refused, because install-time code runs whether or not anything imports the package.
8. **The proof runs against what a consumer actually installs.** `scripts/package_check.py` already installs the real tarballs into a project outside the workspace (`check_consumer`). This story adds three checks there, for each `unreachable` row. (a) The consumer's own lock resolves the package at the row's version, so the exception describes what consumers get. (b) The existing smoke import runs under a resolve hook and no resolved URL lies under `node_modules/<package>/`. (c) No `.js`, `.mjs` or `.cjs` file of any other installed package contains the package name as a quoted string literal, which is the only way a bundler or a resolver could reach it. (b) proves the node path and (c) covers bundlers, which pick dcmjs's `import` condition file without running anything.
9. **Reuse of F-217.** `scripts/advisory_check.py` imports `licence_check.npm_runtime_closure()` for the closure and reads the root lock itself for node-path versions, so no closure walk is written twice. It does not import F-217's private helpers. If integration renames `npm_runtime_closure`, this story follows the integrated name.
10. **Rust advisories are a follow-up, not this story.** The backlog row, its layer (TS) and its estimate are npm. Fact 10 shows no Rust vulnerability today and one unmaintained notice whose disposition (`encoding` through dicom-rs) is its own decision. A follow-up story, "Rust advisories over the shipped crate closure through cargo-deny 0.19.0", is filed in `BACKLOG.md` before this story closes, and the gate is named `advisories` with an `npm` section so Rust joins it rather than becoming a second gate. (Q4, answered.)
11. **The exception is decided by the lead.** The story row names "a proof that no shipped path reaches it" as an accepted resolution, so accepting adm-zip 0.5.17 on that proof is within the story's scope. The row records `decided_by = "lead, under the operator's delegation of 2026-11-05"`. (Q5, answered.)

## Problem and reproducer

```bash
npm ls adm-zip --all                                   # @ocelli/core -> dcmjs@0.52.2 -> adm-zip@0.5.17
npm audit --omit=dev --package-lock-only --json        # exit 1, adm-zip 5 high and 3 moderate
bin/ocelli.sh gate --list | grep -c advisor            # 0, no gate looks
```

Expected after this story: `bin/ocelli.sh gate advisories` exits 0 with a `WARN` line per moderate advisory and an `OK` line naming the one recorded exception. Adding a high advisory to any closure package, removing a GHSA id from the exception, overriding a closure package, or making adm-zip load from a shipped entry point each turns a floor gate red. Today a high advisory in the published runtime is invisible to every gate, which is the defect.

## Scope, API and compatibility

In scope:

1. `scripts/advisory_check.py` with sections `exceptions` (offline) and `npm` (network), and `--report PATH` to evaluate a saved audit report instead of running npm.
2. `ci/advisories.toml` with the one adm-zip row.
3. The three consumer checks in `scripts/package_check.py`.
4. Gate row and arm, CI step, guard catalogue entry and probes, census record, runbook render.
5. The user-facing note on what a consumer's own `npm audit` reports.
6. The follow-up Rust story row, filed at `/complete-feature`.

Non-goals, each with its reason:

- An `overrides` entry for adm-zip in the root `package.json` or in `packages/core/package.json`. The root one reaches no consumer and the package-level one is ignored by npm (documentation above). Decision 1.
- Publishing an `npm-shrinkwrap.json` in `@ocelli/core`. npm's own documentation strongly discourages it for libraries, it freezes every transitive version for consumers, and it would install adm-zip 0.6.1 against dcmjs's exact `0.5.17` pin, which `npm ls` reports as an invalid tree.
- Downgrading dcmjs to 0.29.6 (`fixAvailable`). A semver-major downgrade of the TID 1500 bridge that D10 keeps, for a package the shipped code never loads.
- Vendoring or bundling dcmjs into `@ocelli/core`. It would contradict the recorded no-bundler decision and change the published package's shape to remove a dependency that never executes.
- Dev-tooling advisories (fact 8) and the `tools/oracle` and `tools/bench` lockfiles. None ships. A dev-advisory policy is a separate story if the operator wants one, and none is filed by this plan.
- Rust advisories. Decision 10.
- Opening an upstream dcmjs issue or pull request to move `adm-zip` to `devDependencies`. It is the real fix, and it is an external publication under the operator's identity, so this plan records it in the exception's `reason` and leaves the action to the operator. When dcmjs ships it, the stale-row refusal removes the exception.
- Compiled `*.test.js` files under `packages/core/dist`, which the tarball carries (47 of 280 entries). Observed during this design, unrelated to advisories, and reported to the lead rather than fixed here.

Public signatures added:

```text
python3 scripts/advisory_check.py [exceptions|npm]...      # default: both
python3 scripts/advisory_check.py npm --report PATH        # evaluate a saved report, no network
bin/ocelli.sh gate advisories                              # floor
```

Pure functions, the units the level-1 probes call:

```text
exception_problems(rows: list[dict], closure: set[str], versions: dict[str, str], install_scripts: set[str]) -> list[str]
override_problems(overrides: dict, closure: set[str]) -> list[str]
audit_problems(report: dict, closure: set[str], versions: dict[str, str], rows: list[dict]) -> tuple[list[str], list[str]]
```

`versions` maps a lock node path (`node_modules/adm-zip`) to its version. `audit_problems` returns problems and warnings. In `scripts/package_check.py`:

```text
consumer_version_problems(rows: list[dict], consumer_lock: dict) -> list[str]
resolved_package_problems(resolved: list[str], names: set[str]) -> list[str]
specifier_problems(files: dict[str, str], names: set[str]) -> list[str]
```

The exception row, as it will be written:

```toml
[[exception]]
ecosystem = "npm"
package = "adm-zip"
version = "0.5.17"
advisories = [
  "GHSA-xcpc-8h2w-3j85", "GHSA-7q85-xj36-vmfc", "GHSA-rcw4-f5rp-g42v",
  "GHSA-j5f4-cc29-5x44", "GHSA-8238-w5pm-2374", "GHSA-vwc7-r8mq-g2x9",
  "GHSA-p634-w6r4-rjp2", "GHSA-c6fg-446q-cg94",
]
scope = "unreachable"
proof = "packages-consumer-trace"
reason = "dcmjs 0.52.2 declares adm-zip under dependencies and uses it only in test/testUtils.js, which it does not publish. No shipped dcmjs or @ocelli file names it. The real fix is upstream, moving adm-zip to devDependencies."
decided = "2026-11-05"
decided_by = "lead, under the operator's delegation of 2026-11-05"
```

The GHSA list is copied from the audit's JSON by the implementation, never re-typed (CLAUDE.md).

Refusal messages, each with a fragment unique to it, so every probe below names exactly one refusal:

| Refusal | Fragment |
|---------|----------|
| exception field absent or empty | `is missing` |
| scope other than `unreachable` | `is not unreachable` |
| `(package, version)` not in the published closure of the lock | `stale advisory exception` |
| excepted package has an install script | `has an install script` |
| `proof` is not a known proof | `names no proof the packages gate runs` |
| override on a closure package | `overrides do not reach a consumer` |
| high or critical advisory with no covering row | `has no exception` |
| listed advisory the report no longer carries | `no longer reports` |
| report absent, unparseable, erroring | `did not produce a report` |
| `auditReportVersion` other than 2 | `report version` |
| severity outside the five npm uses | `unknown severity` |
| consumer resolves a different version | `the consumer resolves` |
| excepted package resolved by the smoke import | `loads an unreachable package` |
| excepted package named by another package's file | `names an unreachable package` |

Compatibility: no product API, export, wire format, pixel path or wasm export changes. The published tarball's contents do not change.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `bin/ocelli.sh gate advisories` | `advisory_check.py`, both sections, then `test_advisory_check.py` | floor, network, fails closed | `OK`, `WARN` lines, or each problem |
| `advisory_check.py exceptions` | `ci/advisories.toml`, root `package-lock.json`, root `package.json` `overrides`, `licence_check.npm_runtime_closure()` | offline, sandbox-safe | problems |
| `advisory_check.py npm` | `npm audit --omit=dev --package-lock-only --json` at the root, or `--report` | network unless `--report` | problems and warnings |
| `bin/ocelli.sh gate packages`, `check_consumer` | consumer outside the workspace, `node --import` resolve hook, consumer `package-lock.json`, installed files | floor, needs `node_modules` and network as today | problems |
| CI `frontend` job | `bin/ocelli.sh gate advisories` after `npm ci` | every event the job runs on | job status |
| `bin/ocelli.sh gate ci`, `scripts/ci_floor_check.py` | the new floor gate must be invoked by a CI step | unchanged rule | pass |
| `bin/ocelli.sh gate guards`, census | the `advisories` entry, the new `packages` probes, recorded constants | floor | pass |
| `licence_check.py npm` (F-217) | unchanged, and unaffected because `ci/advisories.toml` is a different file | floor | unchanged |
| a consumer of the published `@ocelli/core` | npm's resolver, no overrides honoured | outside this repository | adm-zip 0.5.17 installed, never loaded, reported by their own `npm audit` |

Owner of the closure: `licence_check.npm_runtime_closure()`. Owner of the threshold and the severity set: `advisory_check.py` constants, recorded by the census. Owner of the decision: `ci/advisories.toml`, written only with a decision recorded in the row. Owner of the reachability proof: `package_check.py`, because it is the only check that sees a consumer's install.

## State transitions and invariants

Not applicable as a stateful feature. Every check is a function of tracked files, the audit report and, for the consumer checks, a temporary install that is deleted afterwards. The invariant across consumers: an `unreachable` row is accepted by `advisory_check.py` only if its `proof` names the check `package_check.py` runs, and that check reads the same rows, so the exception and its proof cannot be separated.

## Ownership, commit and recovery

Not applicable. No runtime resource is acquired. The consumer install lives in a temporary directory owned by `package_check.py`, as it does today.

## Numbered acceptance inventory

| ID | Trigger and expected behaviour | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | No `overrides` key for adm-zip or any closure package in any tracked `package.json`, and the root lock still resolves `node_modules/adm-zip` at 0.5.17 | `advisory_check.py exceptions`, grep | quietly adding the override that turns the in-repo audit green | implementation |
| AC2 | `ci/advisories.toml` carries the one row above, its GHSA list equal to the set of advisory ids the live `--omit=dev` report attributes to adm-zip at implementation time | `gate advisories`, plus the report saved in the handoff | a hand-typed list missing one id | implementation |
| AC3 | Real run. `bin/ocelli.sh gate advisories` exits 0, prints one `WARN` line per moderate adm-zip advisory and an `OK` line naming the exception | local run, CI `frontend` job | a gate green because it audited nothing | implementation, then external (CI on the pushed head) |
| AC4 | Unexcepted high. A report carrying a high advisory on a closure package with no row is refused, naming the GHSA id and `has no exception`. The same for `critical` | probes `advisories.unexcepted-high`, `advisories.unexcepted-critical`, level 1, unit tests | a severity comparison that drops `critical` or compares strings lexically | guards |
| AC5 | New advisory on an excepted package. A high advisory on adm-zip 0.5.17 absent from the row's list is refused | probe `advisories.new-advisory`, level 1 | a package-level waiver that admits every future advisory | guards |
| AC6 | Stale advisory. A listed GHSA id the report no longer carries is refused with `no longer reports` | probe `advisories.stale-advisory`, level 1 | a list that only grows | guards |
| AC7 | Dev package accepted. A high advisory on a package outside the closure (for example `vite`) passes with no row | accept probe `advisories.outside-closure`, level 1 | refusing every high in the report, which would also refuse the production dependencies of the private example workspaces that the root `--omit=dev` report includes | guards |
| AC8 | Moderate warns. An unexcepted moderate on a closure package exits 0 and prints a `WARN` line naming it | accept probe `advisories.moderate-warns`, level 1, unit test asserting the line | failing on moderate, or dropping it silently | guards |
| AC9 | Fail closed. An empty report, a report with an `error` key, unparseable text, `auditReportVersion: 1` and an unknown severity are each refused with their own fragment. A stub `npm` on `PATH` that exits 1 with no JSON is refused | probes `advisories.no-report`, `advisories.report-version`, unit tests with the stub | treating audit's non-zero exit as the verdict, or a network failure as a pass | guards, implementation |
| AC10 | Exception shape. Each required field emptied in turn is refused with `is missing <field>`. Scope `dev-only` is refused with `is not unreachable`. `proof` changed is refused | probes `advisories.exception-incomplete`, `advisories.exception-scope`, `advisories.exception-proof`, level 3 over the sandbox's real `ci/advisories.toml`, unit tests per field | reusing F-217's scopes, which would accept `dev-only` | guards |
| AC11 | Stale row. The row's version changed to `0.5.16`, or its package renamed, is refused with `stale advisory exception` | probe `advisories.exception-stale`, level 3 | a row that outlives the dcmjs release that fixes it | guards |
| AC12 | Install script. The sandbox lock's adm-zip entry given `"hasInstallScript": true` is refused | probe `advisories.install-script`, level 3 | an unreachability claim that ignores install-time code | guards |
| AC13 | Override. `"adm-zip": "0.6.1"` added to the root `overrides`, and separately nested as `"dcmjs": {"adm-zip": "0.6.1"}`, are each refused with `overrides do not reach a consumer`. The existing `react` and `react-dom` overrides pass | probes `advisories.shipped-override`, `advisories.nested-override`, level 3, and the unmutated control | a top-level-only key scan | guards |
| AC14 | Consumer version. In the consumer install, adm-zip resolves at the row's version. A consumer lock giving another version is refused with `the consumer resolves` | probe `packages.unreachable-version`, level 1, then the real `gate packages` | an exception that describes the repository lock rather than the consumer | guards, implementation |
| AC15 | Consumer trace. The smoke import under the resolve hook records every resolved URL, and none lies under `node_modules/adm-zip/`. A synthetic URL list containing one is refused with `loads an unreachable package` | probe `packages.unreachable-loaded`, level 1, then the real `gate packages` | a trace that records only the entry module | guards, implementation |
| AC16 | Static scan. No installed `.js`, `.mjs` or `.cjs` file outside adm-zip itself contains `"adm-zip"` or `'adm-zip'`. A synthetic file map with one is refused with `names an unreachable package` | probe `packages.unreachable-named`, level 1, then the real `gate packages` | a scan of `@ocelli/core` only, which would miss a dcmjs build that starts importing it | guards, implementation |
| AC17 | Red on the real path. In a scratch copy of the consumer, appending `import "adm-zip"` to the installed `dcmjs/build/dcmjs.es.js` makes `check_consumer` refuse through both AC15 and AC16, and the unmodified copy is green. Recorded as RED and GREEN in the handoff | manual run, scratch consumer | a proof that has never been seen to fail | implementation, recorded |
| AC18 | Gate wiring. `advisories` is in `GATES`, in the floor, run by the CI `frontend` job after `npm ci`, and `ci_floor_check.py` passes | `gate ci` | a floor gate CI never runs | guards |
| AC19 | Guard registration. An `advisories` entry with `claims=("*",)` over `scripts/advisory_check.py`. The `packages` entry gains the three probes and its `refuses` text the unreachability clause. Recorded constants: the refused severities, the known severities, the exception field set and the scope set. Census re-recorded, runbook re-rendered, `--check-runbook` clean, and `python3 scripts/guard_census.py` reports zero refusals watched by nothing | `gate guards`, `gate guards-deep` | a refusal added without a probe | guards |
| AC20 | Each new probe goes green when its refusal is deleted. For every refusal in the fragment table, replace its `append` with `pass` in a scratch copy and rerun that probe's fixture, which must then exit 0 | manual, recorded in the handoff | a probe red for an unrelated reason | implementation, recorded |
| AC21 | Consumer audit observed. In the consumer install, `npm audit --omit=dev --json` reports adm-zip and dcmjs as high, and that observation is quoted in the handoff, so the user-facing note describes measured behaviour | manual, recorded | documentation that promises consumers a clean audit | implementation, recorded |
| AC22 | Documentation, per the section below | `gate prose skills` | a README that implies the override fixes consumers | implementation |
| AC23 | Follow-up filed. A `BACKLOG.md`, `SPRINT_PLAN.md` and `allocation.json` row for Rust advisories through cargo-deny, sprint unassigned or the next open sprint, before this story closes | `gate backlog` | the Rust half silently dropped | `/complete-feature` |

## Approach

Order, each step green before the next.

1. **Integrate F-217 first** and re-read `npm_runtime_closure` on the integrated tree. If its name or return type changed, adapt the import here, nothing else.
2. **Save the live report.** `npm audit --omit=dev --package-lock-only --json > <scratch>/audit.json`, record its digest and the npm version in the handoff. Unit tests and level-1 probes use small synthetic reports built in code from npm's documented v2 shape (`vulnerabilities.<name>.via[]` with `source`, `url`, `severity`, `range`, and `nodes[]`), not this file, so a moving database never moves a test.
3. **`advisory_check.py`.** Constants first (`REFUSED = ("critical", "high")`, `KNOWN = ("critical", "high", "moderate", "low", "info")`, `FIELDS`, `SCOPES = ("unreachable",)`, `PROOFS = ("packages-consumer-trace",)`). Then the three pure functions with unit tests written from the AC table. The GHSA id is the last path segment of each `via` entry's `url`. `via` entries that are strings (dcmjs's `"adm-zip"`) are attribution, not advisories, and are skipped, so dcmjs is not refused twice for one cause. Every node path of a vulnerable package whose name is in the closure is evaluated, which is conservative when one name sits at two versions.
4. **The npm runner.** `subprocess.run(["npm", "audit", "--omit=dev", "--package-lock-only", "--json"], cwd=ROOT)`, ignore the exit status, parse stdout, fail closed on anything else.
5. **`ci/advisories.toml`.** Write the row, copying the GHSA list from step 2's report with a one-line `json` extraction.
6. **`package_check.py`.** After the existing install, read `ci/advisories.toml` with `tomllib`. Write a resolve hook module into the work directory and run the existing `smoke.mjs` as `node --import ./trace.mjs smoke.mjs` with the hook appending each resolved URL to a file, so the smoke check and the trace are one run. Walk `consumer/node_modules` for the static scan, skipping the excepted package's own directory. Read `consumer/package-lock.json` for the version.
7. **Wiring.** `GATES` row `"advisories|no|high and critical advisories in the published npm runtime closure"`, arm `advisories) python3 scripts/advisory_check.py && python3 -B -m unittest discover -s scripts/tests -p test_advisory_check.py ;;`, CI step after `npm ci` in `frontend`, catalogue entries and probes, `guard_census.py --record`, `--render-runbook`.
8. **AC17, AC20, AC21** by hand in the scratchpad, recorded.
9. **Documentation**, then the follow-up row at `/complete-feature`.

Rejected alternatives, with reasons beyond the non-goals above:

- **A tracked advisory snapshot checked offline.** Deterministic, and stale by construction: a new high advisory would never turn anything red until someone refreshed the snapshot, which is the defect this story exists for.
- **Auditing a fresh consumer install instead of the lock.** It sees exactly what a consumer resolves, and it doubles the registry traffic of `packages` for a difference decision 2 already closes, since every closure package is pinned exactly except three ranged leaves (`core-js-pure ^3.48.0`, `iota-array ^1.0.0`, `is-buffer ^1.0.2`), where a consumer resolves the newest match and the lock can only be older, so a lock audit errs towards red.
- **Extending `ci/licences.toml` and its validator.** Its shipped-closure refusal is the opposite rule, and a shared file would make one refusal's probe depend on the other's rows.
- **Per-advisory rows.** Eight rows carrying the same reason and proof. One row with a list keeps the reason in one place and still forces a decision per new id (AC5).

## Dependencies, write ownership and resources

Declared dependency F-217. Source-level dependency: `licence_check.npm_runtime_closure()` and F-217's CI ordering. F-217 must be integrated into `sprint/s46` before this story starts.

Write set:

```text
scripts/advisory_check.py                       new
scripts/tests/test_advisory_check.py            new
ci/advisories.toml                              new
scripts/package_check.py
bin/ocelli.sh, .github/workflows/ci.yml
scripts/guards/catalogue.py, ci/guard-probe-budget.json, docs/runbooks/guard-verification.md
docs/lld/guards.md, docs/lld/typescript-packaging.md
README.md, packages/core/README.md, examples/folder-viewer/src/use.html
docs/sprints/BACKLOG.md, docs/sprints/SPRINT_PLAN.md, docs/sprints/allocation.json   (AC23, at /complete-feature)
```

Exclusive files, no concurrent writer: `bin/ocelli.sh`, `.github/workflows/ci.yml`, `scripts/guards/catalogue.py`, `ci/guard-probe-budget.json`, `docs/runbooks/guard-verification.md`. F-219 writes the last three, so F-218 and F-219 run serially. Recommended order: F-217, then F-219 (smaller, no dependency), then F-218, which re-records the census over both.

Network: the registry, for `npm audit` and the existing consumer install. No GPU, no browser, no corpus.

## Verification and evidence reuse

Focused, in order:

```bash
python3 -B -m unittest discover -s scripts/tests -p test_advisory_check.py
python3 scripts/advisory_check.py
bin/ocelli.sh gate advisories packages licences
bin/ocelli.sh gate ci guards guards-deep backlog prose deviations
python3 scripts/guard_census.py --check-runbook
```

Final profile: `bin/ocelli.sh gate --sprint`, recorded by `/verify` against the staged tree.

Identities recorded in the handoff: node and npm versions, the saved report's SHA-256 and its GHSA list, dcmjs 0.52.2 and adm-zip 0.5.17 integrity strings from the lock, the consumer trace's resolved URL list.

What invalidates evidence: any lockfile change, a dcmjs or adm-zip version change, or an advisory published or withdrawn between the AC2 measurement and verification, which shows up as AC5 or AC6 and is a new decision rather than a re-run.

External observation: AC3's CI half is observed only by the `frontend` job on the pushed head. Until that run exists, AC3 is open for its CI half.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, repository tooling and packaging policy, no rendering or compute
- Tier B (WebGL2): n/a, repository tooling and packaging policy, no rendering or compute
- Tier C (CPU): n/a, repository tooling and packaging policy, no rendering or compute

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | threshold, severities, attribution entries skipped, closure filter, row coverage, new and stale advisories, fail-closed report handling, override walk at depth, exception fields, scope, proof, stale row, install script | `scripts/tests/test_advisory_check.py` |
| unit | stub `npm` on `PATH` with no JSON, and with an `error` key | `scripts/tests/test_advisory_check.py` |
| guard probe | every refusal in the fragment table driven red, two accept probes, level 1 for pure functions and level 3 over the sandbox's tracked files | `scripts/guards/catalogue.py`, `gate guards` |
| integration | the real consumer install, trace and scan | `gate packages` |
| mutation | AC17 and AC20, recorded RED and GREEN | handoff |

No `fixture` row is owed. No function doing pixel or geometry arithmetic is added or changed.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no packaging or dependency-policy row, and no rendered output changes.

## Deviations

None. The HLD makes no statement about dependency advisories for this plan to depart from.

## LLD impact

- `docs/lld/guards.md`: the `advisories` guard, its probes, its recorded constants, and the note that it is the one floor gate whose verdict follows an external database.
- `docs/lld/typescript-packaging.md`: under "What `scripts/package_check.py` proves", a row for the unreachability proof, and a short section on why no override is used, quoting npm's documentation.

## User-facing documentation impact

- `README.md`, section "Use the library": one paragraph after the sentence naming the workspace packages. `@ocelli/core` depends on dcmjs 0.52.2, which declares adm-zip 0.5.17 as a runtime dependency. Nothing in `@ocelli/core` or dcmjs's published files loads it, and `gate packages` proves that on every run. A project's own `npm audit` will still report it, because npm reports what is installed. A project that needs a clean audit can add `"overrides": { "adm-zip": "0.6.1" }` to its own root `package.json`, which is safe because nothing calls it.
- `packages/core/README.md`, the README a consumer reads inside the tarball: the same paragraph, shortened.
- `examples/folder-viewer/src/use.html`: the packages table row for `@ocelli/core` (currently "One runtime dependency, `dcmjs` 0.52.2, for the TID 1500 bridge.") gains one sentence after the table saying dcmjs brings adm-zip, that it is never loaded, and that a consumer's audit reports it.
- `examples/folder-viewer/src/more.html`: no change. It carries no dependency or packaging text.

## Open questions

**Q1, resolution route.** Affects AC1, AC13 to AC17. Options: (a) root override, (b) proof of unreachability with a recorded exception, (c) shrinkwrap, (d) vendor dcmjs, (e) downgrade dcmjs. Answered: lead decision under the operator's delegation of 2026-11-05, (b). (a) reaches no consumer, (c) is discouraged by npm for libraries, (d) contradicts the no-bundler decision, (e) is the semver-major downgrade the brief rules out.

**Q2, network in the floor.** Affects AC3, AC9, AC18. Options: (a) live audit in the floor, failing closed, (b) a tracked snapshot checked offline, (c) a non-floor gate. Answered: lead decision under the operator's delegation of 2026-11-05, (a). `packages` already needs the registry in the floor, (b) never turns red on a new advisory, and (c) would put the shipped-runtime check outside what CI runs on every push.

**Q3, exception granularity.** Affects AC5, AC6, AC10, AC11. Answered: lead decision under the operator's delegation of 2026-11-05, one row per package and version listing every advisory id, with new high ids refused and missing ids stale.

**Q4, Rust.** Affects AC23. Options: (a) include `cargo deny check advisories` over the shipped crate closure now, (b) npm now and a filed follow-up. Answered: lead decision under the operator's delegation of 2026-11-05, (b). The story row, layer and estimate are npm, Rust has no vulnerability today (fact 10), and the `encoding` unmaintained notice needs its own disposition.

**Q5, who accepts the adm-zip exception.** Affects AC2. Answered: lead decision under the operator's delegation of 2026-11-05. The story row itself accepts "a proof that no shipped path reaches it", so the acceptance is within scope, and the row records the lead under that delegation.

No question is left open.
