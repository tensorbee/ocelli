# F-220 review, pass 8

F-220 pass 8: 1 defect, 0 smells, 1 nitpick. Not clean.

**Reviewed**: worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-220`, branch `work/f-220-xai`, staged and unstaged, `tools/oracle/out` ignored. Index tree `85ae2bd98c8b1446d092daf35bcafd06baecbe71`, unstaged diff `shasum` `2e7a828214fdadc34dd60ad516bcdc5fcd0e407b`, the same at start and end. Against pass 7's index `b020b15c`, `git diff b020b15c 85ae2bd9 --stat` is one file, `.claude/plans/F-220-design.md`, 2 insertions and 1 deletion. The unstaged diff hash is pass 7's.
**Author / reviewer**: reviewer is an independent Claude subagent spawned by the sprint lead. The change under review is the sprint lead's (Claude) rewrite of bullet 1 and the new "Outside this story" bullet in "Lead amendments after review pass 4". Implementation is an external coding agent per the operator's assignment. This reviewer wrote none of it.
**Scope**: the two sentences, executed. AC11 and every other AC and amendment read against them. Every offline, fetch and network sentence F-220 added to tracked prose and doc comments, including `scripts/licence_check.py`.

## Pass 7 findings

- **D1, closed in the plan.** Line 270 now reads "The `binaries` section stays offline. It reads no network and no pin is fetched at gate time." Line 272 records that the `rust` section's `cargo deny` runs without `--offline` and downloads the registry on an empty `CARGO_HOME`. Both execute as written (below). The same whole-gate claim survives in one F-220 doc comment, which is D1 below.

## Defects

### D1, `_cargo_metadata` says "The gate does not reach the network"

**Where**: `scripts/licence_check.py:1161`, docstring of `_cargo_metadata`, added by F-220 (absent from `git show HEAD:scripts/licence_check.py`).
**What**: "A missing registry cache is a failure. The gate does not reach the network and it does not skip the closure." The subject is "the gate", unqualified. `_cargo_metadata` is called through `_rust_shipped` by two sections: `binaries` and the `rust` section's `_rust_exception_problems` (`licence_check.py:1239`), which `rust_problems` calls at line 1150 right after its own `cargo deny --locked` (line 1123), which has no `--offline`.
**Trigger and result, executed now**: `CARGO_HOME=<empty> python3 scripts/licence_check.py rust` with the network up exited 0 and left 340M of `registry/{cache,index,src}`. The same with outbound IP denied by `sandbox-exec` exited 1, "cargo deny exited 1". So the `rust` section, which is one of this function's two callers and part of `gate licences`, reaches the network.
**Why it is a defect**: `/microscope` section 3, a false claim in a doc comment. It is pass 7's D1 in a second file: the lead narrowed the plan to the `binaries` section, and this sentence still makes the whole-gate claim, in the place a maintainer reads before touching the call. Read as "this call" it is true. Read as written it is false.
**Smallest fix**: say what the function does, for example "This call does not reach the network and it does not skip the closure." No code change.

## Smells

None.

## Nitpicks

- N15, new. `.github/workflows/ci.yml:137-139` says "A missing cache is a failure, so populate it before the licence gate", and `docs/lld/guards.md:628` gives the same reason. True of `binaries` alone. Through `gate licences` the `rust` section runs first (`FLOOR_SECTIONS`, line 84) and fills a cold cache itself, so today the CI step is redundant rather than required. It becomes required if the section order changes or `cargo deny` gains `--offline`. Not counted.
- N1 to N14, carried and not re-examined.

## AC11 and the amendments

- **AC11 is about the `binaries` section, not the whole gate.** Its requirement is "`binaries` fails naming `cargo fetch --locked` and exits non-zero", and its test is a unit test that runs `licence_check.py binaries` alone (`test_offline_empty_cargo_home_names_fetch`, `test_licence_check.py:1329`). "A silent network fetch at gate time" is the failure mode that test catches in `binaries`, and the new line 272 records the `rust` route that it does not. No finding.
- **Decision 9** ("The check runs `cargo metadata --locked --offline`, and a missing registry cache is a failure") is about the `binaries` check. Holds.
- **Decision 6, rejected alternative at line 164, pass-1 bullet 2, pass-2 bullet 1** are about pinned texts and pins, not the registry. They agree with the new bullets. Line 186 ("`cargo fetch --locked` populates the registry cache as any build does") is consistent with line 272.
- **Other tracked prose F-220 added.** `ci/notices/sources.toml:3` and `:40`, `docs/DEVELOPER_SETUP.md:80-82`, `docs/lld/guards.md:628-629`, `ci.yml:137-139`, the `licence_check.py` module docstring ("The binaries gate reads the pinned files and does not fetch"), lines 1746, 1779 and 1896, and the two catalogue notes at `catalogue.py:8043` and `:8126` are each scoped to the pinned files, the `binaries` section, or one call. None says the whole licences gate is offline. `docs/lld/release-policy.md`, `docs/RELEASE.md` and `README.md` say nothing about the network in F-220's lines. The only unscoped sentence is D1.

## Executed

| Command | Input | Exit and observation |
|---------|-------|----------------------|
| `git write-tree`, `git diff \| shasum`, start and end | worktree | `85ae2bd9…`, `2e7a8282…` both times |
| `binaries`, empty `CARGO_HOME`, Python sockets stubbed by `sitecustomize`, outbound IP denied by `sandbox-exec` | worktree | 1, "cargo metadata --locked --offline failed. Run `cargo fetch --locked` and retry." Directory still 4.0K |
| `binaries`, empty `CARGO_HOME`, sockets stubbed, no sandbox | worktree | 1, same message, still 4.0K |
| `binaries`, warm cache, sockets stubbed and network denied | worktree | 0, OK. Bullet 1 holds |
| `rust`, empty `CARGO_HOME`, network denied | worktree | 1, "cargo deny exited 1", 4.0K |
| `rust`, empty `CARGO_HOME`, network up | worktree | 0, OK, 340M downloaded. Line 272 holds |
| `binaries` on that cache, network denied | worktree | 0, OK |
| scratch copy, `--offline` removed from `_cargo_metadata`, unit tests | scratch, no git | AC11's test FAILS. Two other errors are `git ls-files` in a copy with no repository, also present unmutated |
| `python3 scripts/licence_check.py binaries` | worktree, `CARGO_TARGET_DIR=<wt>/target` | 0, OK |
| `python3 -B -m unittest discover -s scripts/tests -p test_licence_check.py` | worktree | 72 OK |
| `bin/ocelli.sh gate licences` | worktree | ALL GREEN |
| `python3 scripts/prose_check.py` | worktree | 0, 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | 0, 1677 files |
| `gate guards`, `guards-deep`, `ci` | | not re-run. Only the plan changed since pass 7 |

## Verified clean

Both lead sentences hold when executed. The plan diff since pass 7 is the two bullets and nothing else. AC11 is scoped to `binaries`. No other F-220 prose claims the whole gate is offline.

## Limitations and handoff

- The socket stub covers Python only. `sandbox-exec` denying outbound IP covers the `cargo` subprocesses. A warm-cache `cargo deny` with the network denied was not tried.
- Every cold `CARGO_HOME` and the scratch copies were under the session scratchpad and are deleted. Only this report was written in the worktree.
- Next owner: the lead, for D1, a one-sentence docstring change. Then an independent pass 9.
