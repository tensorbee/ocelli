# F-220 review, pass 9

F-220 pass 9: 0 defects, 0 smells, 1 nitpick. Clean.

**Reviewed**: worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-220`, branch `work/f-220-xai`, HEAD `c07ef382` (equal to `git merge-base HEAD sprint/s46`), staged and unstaged, `tools/oracle/out` and the earlier review files ignored. Index tree `57826a82ffd185d1e819928d061ef39cc987fc56`, unstaged diff `shasum` `e12b80865f939ff86776c87c656a670ec7851405`, the same at start and end.
**Change since pass 8**: pass 8's index was `85ae2bd9`. `git diff 85ae2bd9 57826a82` touches only `scripts/licence_check.py`. `licence_check.py` was staged and unstaged in pass 8 and is staged only now, so the index moved by that file's whole F-220 diff and the unstaged hash moved with it. The union content changed only in the docstring line. Evidence: every other modified path has an mtime before pass 8's report (17:30:17). `licence_check.py` (17:30:46) is the only path written after it. Every `licence_check.py` line number pass 8 cites (1123, 1150, 1161, 1239, 1746, 1779, 1896) still points at the same text. The old union cannot be rebuilt from hashes alone, so this rests on those two observations.
**Author / reviewer**: reviewer is an independent Claude subagent spawned by the sprint lead. The docstring edit is the sprint lead's (Claude). Implementation is an external coding agent per the operator's assignment. This reviewer wrote none of it.

## Pass 8 findings

- **D1, closed.** `scripts/licence_check.py:1158-1162` now reads "A missing registry cache is a failure. This call does not reach the network and it does not skip the closure." The call at 1163-1165 is `cargo metadata --format-version 1 --locked --offline`. Executed now: `_cargo_metadata([])` with an empty `CARGO_HOME` and the network up raised `ShippedClosureError` "cargo metadata --locked --offline failed. Run `cargo fetch --locked` and retry. error: no matching package named `bytemuck` found", and the directory stayed at 4.0K (`.package-cache` and an empty `registry`). Nothing was downloaded with the network available, so the call does not reach it. The closure is not skipped: the error propagates, and `_rust_exception_problems` (1239-1241) and `binaries_problems` both turn it into a problem. With the warm cache the call returned 232 packages.
- **N15, carried.** `ci.yml:137-138` and `docs/lld/guards.md:628-629` are unchanged. Not counted.

## Sweep: whole-gate network claims

Every added line in `git diff HEAD` (staged plus unstaged, 3266 added lines outside the pinned texts) was grepped for offline, network, fetch, download, online, internet and reach. Each hit was read in context:

| Location | Sentence | Scope |
|----------|----------|-------|
| `licence_check.py:15-16` | "verify-upstream re-fetches pins ... The binaries gate reads the pinned files and does not fetch." | binaries |
| `licence_check.py:152-153` | `_RAW_PIN_CACHE`: "verify-upstream fills this. The binaries gate does not." | binaries |
| `licence_check.py:1158-1162` | the closed D1 sentence | this call |
| `licence_check.py:1745`, `:1777-1779` | "read offline", "The check does not use the network." | `published_sha1` and `published_commit_problems` |
| `licence_check.py:1781-1786` | `fetch_raw_pin`: "The binaries gate does not call this." | binaries |
| `licence_check.py:1896` | "The gate calls this with no blob_at and does not fetch." | pin fetch, see N16 |
| `licence_check.py:1975-1981` | "The floor gate does not call this." | a claim about calls. True: `verify-upstream` is not in `FLOOR_SECTIONS` (line 84), and `main` refuses it beside another section |
| `licence_check.py:2288` | "Offline. Local bytes and the recorded commit. verify-upstream fetches." | one call |
| `ci/notices/sources.toml:3` | "Gate time reads these files. It does not fetch." | these files, see N16 |
| `ci/notices/sources.toml:38-40` | "The check reads that file and does not use the network." | the published-commit check |
| `docs/DEVELOPER_SETUP.md:77-79` | "Gate time reads those files and checks each sha256. It does not fetch. A crate pin's commit is checked offline ..." | those files, see N16 |
| `catalogue.py:8041-8043`, `:8124-8126` | "does not use the network", "Neither call uses the network." | one probe's calls |
| `docs/lld/guards.md:628-629`, `ci.yml:137-138` | "the binaries section reads the registry cache offline" | binaries (N15) |
| plan lines 250, 254, 261-263 | pin fetching at gate time, `binaries` offline, `verify-upstream`, the `rust` section's online `cargo deny` | pins and binaries, and the `rust` exception is stated |
| `test_licence_check.py` names and messages | `test_the_binaries_gate_does_not_fetch` and similar | binaries |

The runbook's added lines (`docs/runbooks/guard-verification.md`) have no network sentence about the licences gate. The two matches are an unrelated catalogue row and a `package_check.py` npm-install limitation. `README.md`, `docs/RELEASE.md`, `docs/lld/release-policy.md`, `docs/SOURCE-POLICY.md` and `scripts/package_check.py` have no network sentence in F-220's lines. No line says the whole licences gate, or the `rust`, `npm`, `python`, `notices` or `holder` section, is offline, never fetches, or works without the network.

## Sweep: counts

| Claim | Where | Tree |
|-------|-------|------|
| "The four MIT-only crates also reproduce the repository root LICENSE.txt" | `sources.toml:47`, `SOURCE-POLICY.md:150` | `[[earlier]]` has 4 rows: block2 0.6.2, objc2 0.6.4, objc2-encode 4.1.0, objc2-foundation 0.3.2. `cargo metadata --offline` gives each `MIT`. The other six objc2-family crates are `Zlib OR Apache-2.0 OR MIT`. Holds |
| "the two entry points" | `licence_check.py:1205`, test 792, release-policy, README | wasm and native, as `_rust_shipped` walks them. Holds |
| `len(third) == 129` | test 797 | passes now |
| `len(data["crate"]) == 16` | test 1239 | `tomllib` reads 16 `[[crate]]` rows. Holds |
| `"licences": 121`, `"packages": 29` | `ci/guard-probe-budget.json` | `guard_census.py` compares the budget for equality and exits 0 |
| "one marker", "one copyright line", "one byte", "one attribution paragraph" | sources.toml, SOURCE-POLICY, guards.md, licence_check.py | singular descriptions, not tallies. The header carries `_CC_BY_ATTRIBUTION` once |

No other crate, file or test count appears in F-220's prose. The added docs say "72 tests" nowhere, and no prose states the 239 carriers or the 17 directories under `ci/notices/crates/`.

## Executed

| Command | Input | Executed now / prior | Exit and observation |
|---------|-------|----------------------|----------------------|
| `git write-tree`, `git diff \| shasum`, `git status --short`, start and end | worktree | now | `57826a82…`, `e12b8086…` both times, status unchanged |
| `git diff 85ae2bd9 57826a82` | pass 8 index to now | now | one file, `scripts/licence_check.py` |
| `_cargo_metadata([])`, empty `CARGO_HOME` in the scratchpad, network up | worktree | now | `ShippedClosureError`, 4.0K, no download. Directory deleted |
| `_cargo_metadata([])`, warm cache | worktree | now | 232 packages |
| `python3 scripts/licence_check.py binaries` | `CARGO_TARGET_DIR=<wt>/target` | now | 0, "OK: licence check (binaries)" |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | now | 0, 72 tests OK |
| `bin/ocelli.sh gate licences` | worktree | now | 0, all six sections OK, 72 tests, ALL GREEN, 1 gate |
| `python3 scripts/guard_census.py` | worktree | now | 0, 1102 refusals in 78 files, 543 probes, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | worktree | now | 0 |
| `python3 scripts/prose_check.py` | worktree | now | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | 0, 1677 files |
| `rust` section with an empty `CARGO_HOME`, network up and denied | | prior, pass 8 | not re-run. Nothing on that route changed |
| `gate guards`, `guards-deep`, `ci` | | not run | only a docstring changed since pass 7's runs |

## Defects

None.

## Smells

None.

## Nitpicks

- **N16, new.** Three sentences use "fetch" with no object: `sources.toml:3` and `DEVELOPER_SETUP.md:78` ("Gate time reads these/those files ... It does not fetch.") and `licence_check.py:1896` ("The gate calls this with no blob_at and does not fetch."). Each sits beside the pinned files or the `blob_at` fetcher, so the object read in context is the pins, and of the pins the sentence is true. It is not D1's shape, which was "does not reach the network" with nothing narrowing it. A reader who takes "gate time" as all of `gate licences` would be wrong on a cold cache, because `cargo deny` in the `rust` section downloads the registry (pass 8's run). "It does not fetch them" would close that reading. Not counted.
- N1 to N15, carried and not re-examined except N15 above.

## Verified clean

D1 is closed and the new sentence holds when executed against the call it describes. After a full sweep, no added comment, docstring, message or document line claims that the whole licences gate or a non-binaries section is offline. Each count in F-220's prose matches the tree. Every requested check exits 0 and the tree is unchanged.

## Limitations and handoff

- The empty-`CARGO_HOME` run proved that no download happened, with the network available. It did not deny the network at the OS level. Pass 8 did that with `sandbox-exec` and got the same refusal.
- The claim that the union changed only in the docstring rests on mtimes and line anchors, not on a stored pass 8 union.
- No GPU, browser or corpus path was exercised. None is in F-220's scope.
- This is a clean bounded review of the F-220 working tree. It does not record verification receipts or sprint readiness. Next owner: the lead, for `/verify` and `/complete-feature`.
- Only this report was written in the worktree. The scratch `CARGO_HOME` was under the session scratchpad and is deleted. One census output file was written beside the worktree, in the projects directory, and deleted right away.
